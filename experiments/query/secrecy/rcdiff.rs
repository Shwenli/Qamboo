
// Rcdiff Query from the SMCQL and Secrecy paper
/*
 * SQL:
 * WITH rcd AS (
 *      SELECT pid, time, row_no
 *      FROM diagnosis
 *      WHERE diag=cdiff)
 *    SELECT DISTINCT pid
 *    FROM rcd r1 JOIN rcd r2
 *    WHERE r1.pid = r2.pid
 *       AND r2.time - r1.time >= 15 DAYS
 *       AND r2.time - r1.time <= 56 DAYS
 *       AND r2.row_no = r1.row_no + 1
 *
 * Note: row_no is not present in our schema; it is an implicit row generated
 * after sorting.
 *
 */

use std::path::PathBuf;

use clap::Parser;
use table::NetStateArgs;
use table::column_operator::ColumnBooleanOperator;
use table::column_operator::PrefixSum;
use table::share_column::ShareColumn;
use table::share_column::ShareType;
use table::table_operator::Filter;
use std::time::Instant;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use primitives::utils::get_data_share;
use experiments::secrecy_database_gen::gen_diagnosis_table;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use protocols::protocols::rep3_ring::arithmetic::open;
use table::table_operator::{Groupby, OrderBy, Project, Open};
use table::predicate::Predicate;
use polars::prelude::*;

const CDIFF_DIAG: u64 = 8;
const TIME_MIN: u64 = 15;
const TIME_MAX: u64 = 56;



#[derive(Parser)]
struct Args {
    /// The config file path
    #[clap(short = 'c', long, value_name = "CONFIGDIR")]
    config_dir: PathBuf,
    
    /// The party ID (0, 1, 2)
    #[clap(short = 'p', long, value_name = "PARTYID", default_value = "0")]
    party_id: String,

    /// The number of threads
    #[clap(short = 't', long, value_name = "THREADS", default_value = "6")]
    threads: usize,

    /// Scale factor
    #[clap(short = 's', long, value_name = "SF", default_value = "0.01")]
    sf: f32,
}


fn main() -> Result<()> {
    let args = Args::parse();
    install_tracing();

    let sf = args.sf;
    let partyid = args.party_id.clone();
    let default_threads = rayon::current_num_threads() / 2;

    tracing::info!("setting up network");
    let mut nets: Vec<TcpNetwork> = Vec::new();
    let mut states: Vec<Rep3State> = Vec::new();

    for i in 0..args.threads {
        let file_path = PathBuf::from(format!("{}{}/config_party{}.toml", args.config_dir.display(), i, partyid));
        let config: NetworkConfig = toml::from_str(&std::fs::read_to_string(file_path).context("opening config file")?).context("parsing config file")?;
        let net = TcpNetwork::new(config)?;
        let state = Rep3State::new(&net)?;
        nets.push(net);
        states.push(state);
    }

    if states.len() <= default_threads {
        let diff = default_threads - states.len();
        for _i in 0..diff {
            let state = states[0].fork(0)?;
            states.push(state);
        }
    }

    let nets = nets.iter().collect::<Vec<&TcpNetwork>>();
    let mut states = states.iter_mut().collect::<Vec<&mut Rep3State>>();
    let party_id = states[0].id;

    let mut mpc_exec_args = NetStateArgs::new(&nets, &mut states);
    
    tracing::info!("Network setup completed");


    tracing::info!("Generating tables");
    let (mut rcd_table, rcd_polars) = gen_diagnosis_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Diagnosis table generated with {} rows", rcd_table.num_rows());


    tracing::info!("Rcdiff query start");
    let tot_start = Instant::now();

    tracing::info!("Filtering rcd table for CDIFF diagnosis");
    let _ = rcd_table.filter_public("diag", Predicate::EqualBinary, &CDIFF_DIAG, &mut mpc_exec_args);


    tracing::info!("Sorting rcd table by pid and time");
    let _ = rcd_table.order_by("time", true, &mut mpc_exec_args);
    let _ = rcd_table.order_by("pid", true, &mut mpc_exec_args);
    let _ = rcd_table.order_by("valid", false, &mut mpc_exec_args);


    tracing::info!("WHERE r1.pid = r2.pid AND r2.time - r1.time >= 15 DAYS AND r2.time - r1.time <= 56 DAYS AND r2.row_no = r1.row_no + 1");
    let mut pid2_data = rcd_table["pid"].get_data()[1..rcd_table.num_rows()].to_vec();
    pid2_data.push(get_data_share(&(1u64<<63u64), party_id)?);
    rcd_table.insert_column("pid2".to_string(), ShareColumn::new(pid2_data,ShareType::Arithmetic, "pid2".to_string()));
    
    let mut time2 = rcd_table["time"].get_data()[1..rcd_table.num_rows()].to_vec();
    time2.push(get_data_share(&(1u64<<63u64), party_id)?);
    rcd_table.insert_column("time2".to_string(), ShareColumn::new(time2, ShareType::Arithmetic, "time2".to_string()));

    let time_diff = rcd_table["time2"].clone() - rcd_table["time"].clone();
    rcd_table.insert_column("time_diff".to_string(), time_diff);

    let c1 = rcd_table["time_diff"].ge_public(&TIME_MIN, &mut mpc_exec_args)?;
    let c2 = rcd_table["time_diff"].le_public(&TIME_MAX, &mut mpc_exec_args)?;
    let c3 = rcd_table["pid"].equal(&rcd_table["pid2"], &mut mpc_exec_args)?;
    let condition = c1.and(&c2, &mut mpc_exec_args)?.and(&c3, &mut mpc_exec_args)?;
    let _ = rcd_table.filter_directed_by_bool(condition.get_data(), &mut mpc_exec_args)?;


    tracing::info!("Selecting distinct pid");
    let _ = rcd_table.group_by(vec!["pid"], &mut mpc_exec_args)?;
    let mut final_table = rcd_table.project(vec!["pid", "valid"])?;


    tracing::info!("rcdiff query execution completed");
    if party_id == PartyID::ID0 {
        tracing::info!("Total execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "rcdiff");




//************* polars verification *************//

    let sum_valid = final_table["valid"].prefix_sum();
    let open_sum_valid = open(sum_valid, mpc_exec_args.nets[0])?;
    final_table.head(open_sum_valid.0 as usize);

    let mpc_result = final_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Rcdiff polars verification:");

        let diagnosis = rcd_polars.unwrap();
        
        // WITH rcd AS (SELECT pid, time, row_no FROM diagnosis WHERE diag=cdiff)
        // Add row number (1-indexed like SQL)
        let rcd = diagnosis.lazy()
            .filter(col("diag").eq(lit(CDIFF_DIAG)))
            .with_column(col("pid").alias("r1_pid"))
            .with_column(col("time").alias("r1_time"))
            .with_row_index("r1_row_no", Some(0u32))  // 0-indexed
            .with_column((col("r1_row_no") + lit(1u32)).cast(DataType::UInt64).alias("r1_row_no"))  // convert to 1-indexed
            .select([col("r1_pid"), col("r1_time"), col("r1_row_no")])
            .collect()?;
        
        // Self join: r1 JOIN r2 ON r1.pid = r2.pid
        // Then apply conditions: r2.time - r1.time BETWEEN 15 AND 56
        // AND r2.row_no = r1.row_no + 1
        let rcd_result = rcd.clone().lazy()
            .join(
                rcd.clone().lazy().rename(
                    ["r1_pid".to_string(), "r1_time".to_string(), "r1_row_no".to_string()],
                    ["r2_pid".to_string(), "r2_time".to_string(), "r2_row_no".to_string()],
                    true
                ),
                [col("r1_pid")],
                [col("r2_pid")],
                JoinArgs::new(JoinType::Inner)
            )
            .filter(
                (col("r2_time") - col("r1_time")).gt_eq(lit(TIME_MIN))
                    .and((col("r2_time") - col("r1_time")).lt_eq(lit(TIME_MAX)))
                    .and(col("r2_row_no").eq(col("r1_row_no") + lit(1u64)))
            )
            .select([col("r1_pid").alias("pid")])  // SELECT r1.pid (equivalent to r2.pid)
            .unique(None, UniqueKeepStrategy::First)  // DISTINCT
            .sort(["pid"], SortMultipleOptions::default())
            .collect()?;

        tracing::info!("Polars distinct pid count: {}", rcd_result.height());
        
        let polars_pids = rcd_result.column("pid")?.u64()?.into_no_null_iter().collect::<Vec<u64>>();
        let mpc_pids = mpc_result["pid"].get_data().to_vec();

        assert_eq!(mpc_pids, polars_pids, "Distinct pid count does not match!");

        tracing::info!("rcdiff verification passed!");
    }


    Ok(())
}
