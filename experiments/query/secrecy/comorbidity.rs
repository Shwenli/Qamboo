
/*
 *   SELECT
 *       diag, COUNT(*) cnt
 *   FROM
 *       diagnosis
 *   WHERE
 *       pid IN cohort
 *   GROUP BY
 *       diag
 *   ORDER BY
 *       cnt DESC
 *
 * SCHEMA:
 *   - cohort: [pid]
 *   - diagnosis: [pid, diag]
 */

use std::path::PathBuf;
use clap::Parser;
use table::NetStateArgs;
use std::time::Instant;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use protocols::rep3_ring::arithmetic::open;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::secrecy_database_gen::{gen_cohort_table, gen_diagnosis_comorbidity_table};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Join, Groupby, AggFunc, OrderBy, Open, Project};
use table::column_operator::PrefixSum;
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
    let (cohort_table, cohort_polars) = gen_cohort_table(sf, &mut mpc_exec_args)?;
    let (diagnosis_table, diagnosis_polars) = gen_diagnosis_comorbidity_table(sf, &mut mpc_exec_args)?;

    tracing::info!("Cohort table generated with {} rows", cohort_table.num_rows());
    tracing::info!("Diagnosis table generated with {} rows", diagnosis_table.num_rows());


    tracing::info!("Comorbidity query start");
    let tot_start = Instant::now();


    tracing::info!("Semi-join: diagnosis WHERE pid IN cohort");
    let mut final_table = cohort_table.inner_join("pid", "pid", &diagnosis_table, &mut mpc_exec_args)?;


    tracing::info!("GROUP BY diag, COUNT(*)");
    let (e, perm, _) = final_table.group_by(vec!["diag"], &mut mpc_exec_args)?;
    let _ = final_table.agg_count("cnt", &e, &perm, &mut mpc_exec_args)?;


    tracing::info!("ORDER BY cnt DESC");
    let _ = final_table.order_by("cnt", false, &mut mpc_exec_args)?;


    tracing::info!("Comorbidity query execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Comorbidity");




//************* polars verification *************//

    
    let mut result_table = final_table.project(vec!["diag", "cnt", "valid"])?;
    let _ = result_table.order_by("valid", false, &mut mpc_exec_args);
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    // Verification with Polars
    if party_id == PartyID::ID0 {
        tracing::info!("Comorbidity polars verification:");
        let cohort = cohort_polars.unwrap();
        let diagnosis = diagnosis_polars.unwrap();

        // SQL: SELECT diag, COUNT(*) cnt FROM diagnosis WHERE pid IN cohort GROUP BY diag ORDER BY cnt DESC LIMIT 10
        let comorbidity_result = diagnosis.lazy()
            .join(
                cohort.lazy(),
                [col("pid")],
                [col("pid")],
                JoinType::Inner.into()
            )
            .group_by([col("diag")])
            .agg([len().alias("cnt")])
            .sort(
                ["cnt", "diag"],
                SortMultipleOptions::default().with_maintain_order(true).with_order_descending_multi([true, false])
            )
            .collect()?;

        let mpc_diag = mpc_result["diag"].get_data();
        let mpc_cnt = mpc_result["cnt"].get_data();

        let polars_diag = comorbidity_result.column("diag")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_cnt = comorbidity_result.column("cnt")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();


        assert_eq!(open_valid as usize, comorbidity_result.height(), "Result row count mismatch");
        assert_eq!(mpc_cnt, &polars_cnt, "cnt mismatch");
        assert_eq!(mpc_diag, &polars_diag, "diag mismatch");

        tracing::info!("Verification passed!");
    }

    Ok(())
}
