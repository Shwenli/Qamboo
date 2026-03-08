/*
 *   SELECT ID
 *   FROM R
 *   GROUP BY ID, PWD
 *   HAVING COUNT(*)>1
 */


use std::path::PathBuf;
use clap::Parser;
use table::NetStateArgs;
use table::column_operator::PrefixSum;
use table::table_operator::AggFunc;
use table::table_operator::Filter;
use std::time::Instant;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::secrecy_database_gen::gen_password_table;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use protocols::rep3_ring::arithmetic::open;
use table::table_operator::{Groupby, Project, Open, OrderBy};
use table::predicate::Predicate;
use polars::prelude::*;


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
    let (mut pwd_table, pwd_polars) = gen_password_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Password table generated with {} rows", pwd_table.num_rows());


    tracing::info!("pwd query start");
    let tot_start = Instant::now();

    let (e,perm, _) = pwd_table.group_by(vec!["id", "pwd"], &mut mpc_exec_args)?;
    let _ = pwd_table.agg_count("count", &e, &perm, &mut mpc_exec_args);
    let _ = pwd_table.filter_public("count", Predicate::GreaterThan, &1u64, &mut mpc_exec_args);
                    
    let mut final_table = pwd_table.project(vec!["id", "valid"])?;


    tracing::info!("pwd query execution completed");
    if party_id == PartyID::ID0 {
        tracing::info!("Total execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "pwd");




//************* polars verification *************//

    let _ = final_table.order_by("valid", false, &mut mpc_exec_args);

    let sum_valid = final_table["valid"].prefix_sum();
    let open_sum_valid = open(sum_valid, mpc_exec_args.nets[0])?;
    final_table.head(open_sum_valid.0 as usize);

    let mpc_result = final_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Pwd polars verification:");

        let pwd = pwd_polars.unwrap();
        
        let pwd_result = pwd.lazy()
            .group_by([col("id"), col("pwd")])
            .agg([col("id").count().alias("count")])
            .filter(col("count").gt(lit(1u64)))
            .sort(
                ["id", "pwd"],
                SortMultipleOptions::default()
                    .with_order_descending_multi([false, false])
            )
            .select([col("id")])
            .collect()?;

        tracing::info!("Polars distinct id count: {}", pwd_result.height());
        tracing::info!("MPC distinct id count: {}", open_sum_valid.0);
        
        let polars_pids = pwd_result.column("id")?.u64()?.into_no_null_iter().collect::<Vec<u64>>();
        let mpc_pids = mpc_result["id"].get_data().to_vec();

        assert_eq!(mpc_pids, polars_pids, "id does not match!");

        tracing::info!("pwd verification passed!");
    }


    Ok(())
}
