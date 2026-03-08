/*
 * SELECT S.ID FROM (
 *   SELECT ID, MIN(CS) as cs1, MAX(CS) as cs2
 *   FROM R
 *   WHERE R.year=YEAR
 *   GROUP-BY ID ) as S
 * WHERE S.cs2 - S.cs1 > THRESHOLD
*/


use clap::Parser;

use random::MpcState;
use table::NetStateArgs;
use table::column_operator::ColumnBooleanOperator;
use std::path::PathBuf;
use std::time::Instant;
use random::rep3::Rep3State;
use table::column_operator::PrefixSum;
use table::table_operator::AggFunc;
use table::table_operator::Filter;
use color_eyre::{Result, eyre::Context};
use communication::rep3::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::secrecy_database_gen::gen_credit_score_table;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use protocols::protocols::rep3_ring::arithmetic::open;
use table::table_operator::{Groupby, Project, Open, OrderBy};
use table::column_operator::TransformBetweenArithAndBinary;
use table::predicate::Predicate;
use polars::prelude::*;


const TARGET_YEAR :u64 =  2026;
const THRESHOLD :u64 = 150;



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
    let (mut credit_table, credit_polars) = gen_credit_score_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Credit table generated with {} rows", credit_table.num_rows());


    tracing::info!("converting some columns to binary");
    let credit_year_binary = credit_table["year"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    credit_table.insert_column("[year]".to_string(), credit_year_binary);


    tracing::info!("credit query start");
    let tot_start = Instant::now();

     
    tracing::info!("R.year = YEAR");
    let _ = credit_table.filter_public("[year]", Predicate::EqualBinary, &TARGET_YEAR, &mut mpc_exec_args);
    credit_table.delete_column("[year]");
    

    tracing::info!("Group by ID and aggregate min and max of CS");
    let _ = credit_table.order_by("cs", true, &mut mpc_exec_args)?;
    let (e,perm, e_bit) = credit_table.group_by(vec!["uid"], &mut mpc_exec_args)?;
    let _ = credit_table.agg_min("cs", "cs1", &e, &e_bit, &perm, &mut mpc_exec_args)?;
    let _ = credit_table.agg_max("cs", "cs2", &e, &perm, &mut mpc_exec_args)?;

    
    tracing::info!("Filter S.cs2 - S.cs1 > THRESHOLD");
    let cs_diff = credit_table["cs2"].clone() - credit_table["cs1"].clone();
    let _ = credit_table.filter_directed_by_bool(cs_diff.gt_public(&THRESHOLD, &mut mpc_exec_args)?.get_data(), &mut mpc_exec_args)?;
    
                    
    let mut final_table = credit_table.project(vec!["uid", "valid"])?;


    tracing::info!("credit query execution completed");
    if party_id == PartyID::ID0 {
        tracing::info!("Total execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "credit");




//************* polars verification *************//

    let _ = final_table.order_by("valid", false, &mut mpc_exec_args);

    let sum_valid = final_table["valid"].prefix_sum();
    let open_sum_valid = open(sum_valid, mpc_exec_args.nets[0])?;
    final_table.head(open_sum_valid.0 as usize);

    let mpc_result = final_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Credit polars verification:");

        let credit = credit_polars.unwrap();
        
        let credit_result= credit.lazy()
            .filter(col("year").eq(TARGET_YEAR))
            .group_by([col("uid")])
            .agg([col("cs").min().alias("cs1"), col("cs").max().alias("cs2")])
            .with_column((col("cs2") - col("cs1")).alias("cs_diff"))
            .filter(col("cs_diff").gt(THRESHOLD))
            .sort(
                ["uid"],
                SortMultipleOptions::default()
                    .with_order_descending(false)
            )
            .select([col("uid")])
            .collect()?;

        tracing::info!("Polars distinct uid count: {}", credit_result.height());
        tracing::info!("MPC distinct uid count: {}", open_sum_valid.0);
        
        let polars_pids = credit_result.column("uid")?.u64()?.into_no_null_iter().collect::<Vec<u64>>();
        let mpc_pids = mpc_result["uid"].get_data().to_vec();

        assert_eq!(mpc_pids, polars_pids, "uid does not match!");

        tracing::info!("credit verification passed!");
    }


    Ok(())
}