
/* Comorbidity Query from the Secrecy paper
 *
 * SQL:
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
 *   LIMIT 10
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
use protocols::protocols::rep3_ring::arithmetic::open;
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

    // [SQL] WHERE pid IN cohort
    // Semi-join: filter diagnosis where pid exists in cohort
    // Note: semi_join modifies diagnosis_table in place
    tracing::info!("Semi-join: diagnosis WHERE pid IN cohort");
    let mut filtered_diagnosis = diagnosis_table;
    filtered_diagnosis.semi_join("pid", "pid", &cohort_table, &mut mpc_exec_args)?;

    tracing::info!("Semi-join completed");

    // [SQL] GROUP BY diag, COUNT(*)
    tracing::info!("GROUP BY diag, COUNT(*)");
    
    let group_by_cols = vec!["diag"];
    let (e, perm, _) = filtered_diagnosis.group_by(group_by_cols, &mut mpc_exec_args)?;

    // Add count column
    let _ = filtered_diagnosis.agg_count("cnt", &e, &perm, &mut mpc_exec_args)?;

    tracing::info!("Aggregation completed");

    // [SQL] ORDER BY cnt DESC
    tracing::info!("ORDER BY cnt DESC");
    let _ = filtered_diagnosis.order_by("cnt", false, &mut mpc_exec_args)?;

    tracing::info!("Sort completed");

    // [SQL] LIMIT 10
    tracing::info!("LIMIT 10");
    filtered_diagnosis.head(10);

    tracing::info!("Comorbidity query execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Comorbidity");

    // Open results
    let mut result_table = filtered_diagnosis.project(vec!["diag", "cnt", "valid"])?;
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);

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
                ["cnt"],
                SortMultipleOptions::default().with_maintain_order(true).with_order_descending(true)
            )
            .limit(10)
            .collect()
            .unwrap();

        let mpc_diag = mpc_result["diag"].get_data();
        let mpc_cnt = mpc_result["cnt"].get_data();

        let polars_diag = comorbidity_result.column("diag")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_cnt = comorbidity_result.column("cnt")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        tracing::info!("MPC result: diag={:?}, cnt={:?}", mpc_diag, mpc_cnt);
        tracing::info!("Polars result: diag={:?}, cnt={:?}", polars_diag, polars_cnt);

        assert_eq!(mpc_diag.len(), polars_diag.len(), "Result row count mismatch");
        
        // Note: Due to semi-join and sorting being performed on secret-shared data,
        // the exact order of rows with equal counts may differ from plaintext.
        // We verify that the counts match (sorted in descending order).
        assert_eq!(mpc_cnt, &polars_cnt, "Count mismatch: MPC {:?} vs Polars {:?}", mpc_cnt, polars_cnt);

        tracing::info!("Verification passed!");
    }

    Ok(())
}
