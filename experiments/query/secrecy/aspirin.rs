/*
 * SELECT COUNT(DISTINCT pid)
 * FROM diagnosis d JOIN medication m
 * WHERE d.pid = m.pid AND d.time <= m.time
 *   AND d.diag = hd AND m.med = aspirin
*/

use std::path::PathBuf;
use clap::Parser;
use table::NetStateArgs;
use table::column_operator::PrefixSum;
use table::table_operator::Filter;
use std::time::Instant;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::secrecy_database_gen::{gen_diagnosis_table,gen_medication_table};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use protocols::protocols::rep3_ring::arithmetic::open;
use table::table_operator::{Groupby, OrderBy, Join, AggFunc, Open};
use table::column_operator::TransformBetweenArithAndBinary;
use table::predicate::Predicate;
use polars::prelude::*;

const HD:u64 = 3;
const ASPIRIN: u64 = 5;



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
    let (mut diag_table, rcd_polars) = gen_diagnosis_table(sf, &mut mpc_exec_args)?;
    let (mut med_table, med_polars) = gen_medication_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Diagnosis table generated with {} rows", diag_table.num_rows());
    tracing::info!("Medication table generated with {} rows", med_table.num_rows());

    tracing::info!("converting some columns to binary");
    let diag_diag_binary = diag_table["diag"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    diag_table.insert_column("[diag]".to_string(), diag_diag_binary);
    let med_med_binary = med_table["med"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    med_table.insert_column("[med]".to_string(), med_med_binary);


    tracing::info!("aspirin query start");
    let tot_start = Instant::now();

    tracing::info!("d.diag = hd AND m.med = aspirin");
    let _ = diag_table.filter_public("[diag]", Predicate::EqualBinary, &HD, &mut mpc_exec_args);
    let _ = med_table.filter_public("[med]", Predicate::EqualBinary, &ASPIRIN, &mut mpc_exec_args);
    diag_table.delete_column("[diag]");
    med_table.delete_column("[med]");

    
    // agg_min and agg_max require order by on the agg column, so we order by time first
    tracing::info!("rcd and med order by time compute agg min/max");
    let _ = diag_table.order_by("time", true, &mut mpc_exec_args);
    let _ = med_table.order_by("time", true, &mut mpc_exec_args);


    tracing::info!("min(d.time) as min_time, max(m.time) as max_time");
    let (e,perm, e_bit) = diag_table.group_by(vec!["pid"], &mut mpc_exec_args)?;
    let _ = diag_table.agg_min("time", "min_time", &e, &e_bit, &perm, &mut mpc_exec_args)?;
    diag_table.delete_column("time");

    let (e,perm, _e_bit) = med_table.group_by(vec!["pid"], &mut mpc_exec_args)?;
    let _ = med_table.agg_max("time", "max_time", &e, &perm, &mut mpc_exec_args)?;
    med_table.delete_column("time");

    
    tracing::info!("d.pid = m.pid AND d.time <= m.time");
    let mut final_table = diag_table.inner_join("pid", "pid", &med_table, &mut mpc_exec_args)?;
    let _ = final_table.filter_shared("min_time", "max_time", Predicate::LessOrEqual, &mut mpc_exec_args)?;
    

    tracing::info!("count distinct pid");
    //let _ = final_table.group_by(vec!["pid"], &mut mpc_exec_args)?;
    let count = final_table["valid"].prefix_sum();
    let open_count = open(count, mpc_exec_args.nets[0])?.0;


    tracing::info!("aspirin query execution completed");
    if party_id == PartyID::ID0 {
        tracing::info!("Total execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "aspirin");




//************* polars verification *************//
    let _ = final_table.order_by("valid", false, &mut mpc_exec_args);
    let mut result_table = final_table.open(&mut mpc_exec_args)?;
    result_table.head(open_count as usize);


    if party_id == PartyID::ID0 {
        tracing::info!("Aspirin polars verification:");

        let diagnosis = rcd_polars.unwrap();
        let medication = med_polars.unwrap();
        
        // Filter diagnosis where diag = HD
        let d_filtered = diagnosis.lazy()
            .filter(col("diag").eq(lit(HD)))
            .select([col("pid"), col("time").alias("d_time")]);
        
        // Filter medication where med = ASPIRIN
        let m_filtered = medication.lazy()
            .filter(col("med").eq(lit(ASPIRIN)))
            .select([col("pid"), col("time").alias("m_time")]);
        
        // Join: d.pid = m.pid AND d.time <= m.time
        let result = d_filtered
            .join(
                m_filtered,
                [col("pid")],
                [col("pid")],
                JoinArgs::new(JoinType::Inner)
            )
            .filter(col("d_time").lt_eq(col("m_time")))
            .sort(
                ["pid"],
                SortMultipleOptions::default()
                    .with_order_descending(false)
            )
            .select([col("pid")])
            .unique(None, UniqueKeepStrategy::First)  // DISTINCT
            .collect()?;

        let polars_count = result.height() as u64;
        tracing::info!("Polars distinct pid count: {}", polars_count);
        tracing::info!("MPC distinct pid count: {}", open_count);

        
        let _mpc_pids = result_table["pid"].get_data().to_vec();
        let _polars_pids = result.column("pid")?.u64()?.into_no_null_iter().collect::<Vec<u64>>();

        //tracing::info!("Polars pids: {:?}", polars_pids);
        //tracing::info!("MPC pids: {:?}", mpc_pids);

        assert_eq!(open_count, polars_count, "Distinct pid count does not match!");

        tracing::info!("aspirin verification passed!");
    }


    Ok(())
}
