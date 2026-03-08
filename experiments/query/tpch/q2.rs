/*
 select
 *     s_acctbal,
 *     s_name,
 *     n_name,
 *     p_partkey,
 *     p_mfgr,
 *     s_address,
 *     s_phone,
 *     s_comment
 * from
 *     part,
 *     supplier,
 *     partsupp,
 *     nation,
 *     region
 * where
 *     p_partkey = ps_partkey
 *     and s_suppkey = ps_suppkey
 *     and p_size = [SIZE]
 *     and p_type like '%[TYPE]'
 *     and s_nationkey = n_nationkey
 *     and n_regionkey = r_regionkey
 *     and r_name = '[REGION]'
 *     and ps_supplycost = (
 *         select
 *             min(ps_supplycost)
 *         from
 *             partsupp, supplier,
 *             nation, region
 *         where
 *             p_partkey = ps_partkey
 *             and s_suppkey = ps_suppkey
 *             and s_nationkey = n_nationkey
 *             and n_regionkey = r_regionkey
 *             and r_name = '[REGION]'
 *     )
 * order by
 *     s_acctbal desc,
 *     n_name,
 *     s_name,
 *     p_partkey;
 *
 * Ignores s_address, s_phone, s_comment, and p_mfgr because they aren't semantically interesting
 */
 

use std::path::PathBuf;
use std::vec;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use net::tcp::{TcpNetwork, NetworkConfig};
use std::time::Instant;
use experiments::tpch_database_gen::{self, get_part_table_size};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use protocols::rep3_ring::arithmetic::open;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Filter, Groupby, AggFunc, Join, Open, OrderBy, Project};
use table::column_operator::PrefixSum;
use table::predicate::Predicate;
use table::NetStateArgs;
use polars::prelude::*;



const SIZE: u64 = 15;
const REGION : u64 = 3; // ASIA
const TYPE : u64 = 3; // "BRASS"



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
    #[clap(short = 's', long, value_name = "SF", default_value = "0.1")]
    sf: f32,
}


fn main() -> Result<()> {
    let args = Args::parse();
    install_tracing();

    let sf = args.sf;// scale factor for testing
    let partyid=args.party_id.clone();
    let default_threads = rayon::current_num_threads() / 2;

    tracing::info!("Q2 with SF: {}", sf);
    
    tracing::info!("setting up network");
    let mut nets: Vec<TcpNetwork> = Vec::new();
    let mut states: Vec<Rep3State> = Vec::new();

    for i in 0..args.threads {
        let file_path = PathBuf::from(format!("{}{}/config_party{}.toml", args.config_dir.display(), i, partyid));
        let config: NetworkConfig =toml::from_str(&std::fs::read_to_string(file_path).context("opening config file")?).context("parsing config file")?;
        let net = TcpNetwork::new(config)?;
        let state = Rep3State::new(&net)?;
        nets.push(net);
        states.push(state);
    }

    if states.len() <= default_threads {
        let diff = default_threads - states.len();
        for _i in 0..diff{
            let state = states[0].fork(0)?;
            states.push(state);
        }
    }
    
    let nets = nets.iter().collect::<Vec<&TcpNetwork>>();
    let mut states = states.iter_mut().collect::<Vec<&mut Rep3State>>();
    let party_id = states[0].id;

    let mut mpc_exec_args = NetStateArgs::new(
        &nets,
        &mut states,
    );

    tracing::info!("Network setup completed");

    
    let (part_table, part_table_polars) = tpch_database_gen::gen_part_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Part table generated with {} rows", part_table.num_rows());
    
    let (supplier_table, supplier_table_polars) = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());

    let (partsupp_table, partsupp_table_polars) = tpch_database_gen::gen_partsupp_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Partsupp table generated with {} rows", partsupp_table.num_rows());

    let (nation_table, nation_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation_table.num_rows());

    let (region_table, region_table_polars) = tpch_database_gen::gen_region_table(&mut mpc_exec_args)?;
    tracing::info!("Region table generated with {} rows", region_table.num_rows());


    tracing::info!("Projecting tables");

    let part_col_names = vec!["p_partkey", "p_size", "p_type", "valid"];
    let mut part_table = part_table.project(part_col_names)?;

    let supplier_col_names = vec!["s_suppkey", "s_nationkey", "s_acctbal", "s_name", "valid"];
    let supplier_table = supplier_table.project(supplier_col_names)?;

    let partsupp_col_names = vec!["ps_partkey", "ps_suppkey", "ps_supplycost", "valid"];
    let partsupp_table = partsupp_table.project(partsupp_col_names)?;

    let nation_col_names = vec!["n_nationkey", "n_regionkey", "n_name", "valid"];
    let nation_table = nation_table.project(nation_col_names)?;

    let region_col_names = vec!["r_regionkey", "r_name", "valid"];
    let mut region_table = region_table.project(region_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q2 start");
    let tot_start = Instant::now();

    tracing::info!("p_size = '[SIZE]' ");
    let filter_name = "p_size";
    let _ = part_table.filter_public(
        filter_name,
        Predicate::Equal,
        &SIZE,
        &mut mpc_exec_args,
    )?;

    tracing::info!("p_type like '%[TYPE]' ");
    let filter_name = "p_type";
    let _ = part_table.filter_public(
        filter_name,
        Predicate::Equal,
        &TYPE,
        &mut mpc_exec_args,
    )?;


    tracing::info!("The sub-query: ");
    
    tracing::info!("r_name = '[REGION]");

    let r_filter_name = "r_name";

    let _ = region_table.filter_public(
        r_filter_name,
        Predicate::Equal,
        &REGION,
        &mut mpc_exec_args,
    )?;


    tracing::info!("n_regionkey = r_regionkey");

    let k_l_name = "r_regionkey";
    let k_r_name = "n_regionkey";

    let nation_region_table = region_table.inner_join(
        k_l_name,
        k_r_name,
        &nation_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("s_nationkey = n_nationkey");

    let k_l_name = "n_nationkey";
    let k_r_name = "s_nationkey";

    let nation_region_supplier_table = nation_region_table.inner_join(
        k_l_name,
        k_r_name,
        &supplier_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("s_suppkey = ps_suppkey");

    let k_l_name = "s_suppkey";
    let k_r_name = "ps_suppkey";

    let nation_region_supplier_partsupp_table = nation_region_supplier_table.inner_join(
        k_l_name,
        k_r_name,
        &partsupp_table,
        &mut mpc_exec_args
    )?;

    tracing::info!("p_partkey = ps_partkey");

    let k_l_name = "p_partkey";
    let k_r_name = "ps_partkey";

    let sub_table = part_table.inner_join(
        k_l_name,
        k_r_name,
        &nation_region_supplier_partsupp_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("select min(ps_supplycost)");

    let mut sub_table_clone = sub_table.clone();

    let (e, perm, e_bit) = sub_table_clone.group_by(vec!["ps_partkey"], &mut mpc_exec_args)?;

    let _ = sub_table_clone.agg_min(
        "ps_supplycost",
        "min_ps_supplycost",
        &e,
        &e_bit,
        &perm,
        &mut mpc_exec_args
    )?;

    tracing::info!("secure cut sub_table_clone to partsupp size");
    sub_table_clone.head(get_part_table_size(sf) as usize);

    let sub_table_clone = sub_table_clone.project(vec!["ps_partkey", "min_ps_supplycost", "valid"])?;


    tracing::info!("sub_table_clone join with sub_table on ps_partkey");

    let mut final_table = sub_table_clone.inner_join(
        "ps_partkey",
        "ps_partkey",
        &sub_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("filter on ps_supplycost = min_ps_supplycost");

    let l_name = "ps_supplycost";
    let r_name = "min_ps_supplycost";

    let _ = final_table.filter_shared(
        l_name,
        r_name,
        Predicate::Equal,
        &mut mpc_exec_args,
    )?;

    final_table.delete_column("min_ps_supplycost");
    tracing::info!("order by");

    let sort_key_name = "ps_partkey";
    let _ = final_table.order_by(sort_key_name, true, &mut mpc_exec_args);

    let sort_key_name = "s_name";
    let _ = final_table.order_by(sort_key_name, true, &mut mpc_exec_args);

    let sort_key_name = "n_name";
    let _ = final_table.order_by(sort_key_name, true, &mut mpc_exec_args);

    let sort_key_name = "s_acctbal";
    let _ = final_table.order_by(sort_key_name, false, &mut mpc_exec_args);
    

    tracing::info!("Q2 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q2 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q2");




//************* polars verification *************//

    let mut result_table = final_table.project(vec!["s_acctbal", "s_name", "n_name", "ps_partkey", "valid"])?;
    // sort valid to top
    let _ = result_table.order_by("valid", false, &mut mpc_exec_args)?;
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q2 polars:");
        let part = part_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();
        let partsupp = partsupp_table_polars.unwrap();
        let nation = nation_table_polars.unwrap();
        let region = region_table_polars.unwrap();

        let part_filtered = part.lazy().filter(col("p_size").eq(lit(SIZE)).and(col("p_type").eq(lit(TYPE))));
        let region_filtered = region.lazy().filter(col("r_name").eq(lit(REGION)));
        
        tracing::info!("Filtered Part rows: {}", part_filtered.clone().collect()?.height());
        tracing::info!("Filtered Region rows: {}", region_filtered.clone().collect()?.height());
        // r_regionkey = n_regionkey
        // n_nationkey = s_nationkey
        // s_suppkey = ps_suppkey
        // p_partkey = ps_partkey

        let joined_df = region_filtered
            .join(nation.lazy(), [col("r_regionkey")], [col("n_regionkey")], JoinArgs::new(JoinType::Inner))
            .join(supplier.lazy(), [col("n_nationkey")], [col("s_nationkey")], JoinArgs::new(JoinType::Inner))
            .join(partsupp.lazy(), [col("s_suppkey")], [col("ps_suppkey")], JoinArgs::new(JoinType::Inner))
            .join(part_filtered, [col("ps_partkey")], [col("p_partkey")], JoinArgs::new(JoinType::Inner));

        tracing::info!("Joined DataFrame rows: {}", joined_df.clone().collect()?.height());
        // Note: The MPC implementation computes the global minimum supplycost across all valid joined rows
        // (after filters), rather than a per-part correlated subquery minimum.
        // We replicate the MPC logic here.
        // let min_value_df = joined_df.clone()
        //     .select([col("ps_supplycost").min().alias("min_val")])
        //     .collect()?;
        
        // let min_val = min_value_df.column("min_val")?.get(0)?.try_extract::<u64>()?;
        // tracing::info!("Polars min_supplycost: {}", min_val);

        // Standard TPCH Q2: Find minimum supplycost per part in the target region (correlated subquery)
        let final_df = joined_df
            .with_columns(vec![
                col("ps_supplycost").min().over([col("ps_partkey")]).alias("min_supplycost")
            ])
            .filter(col("ps_supplycost").eq(col("min_supplycost")))
            .sort(
                ["s_acctbal", "n_name", "s_name", "ps_partkey"],
                SortMultipleOptions::default()
                    .with_order_descending_multi([true, false, false, false])
            )
            .select([col("s_acctbal"), col("s_name"), col("n_name"), col("ps_partkey")])
            .collect()?;


        tracing::info!("Polars result: {:?}", final_df.height());
        
        let mpc_s_acctbal = mpc_result["s_acctbal"].get_data();
        let mpc_s_name = mpc_result["s_name"].get_data();
        let mpc_p_partkey = mpc_result["ps_partkey"].get_data();

        let polars_s_acctbal = final_df.column("s_acctbal")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_s_name = final_df.column("s_name")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_p_partkey = final_df.column("ps_partkey")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        assert_eq!(mpc_s_acctbal, polars_s_acctbal);
        assert_eq!(mpc_s_name, polars_s_name);
        assert_eq!(mpc_p_partkey, polars_p_partkey);
        
        tracing::info!("Q2 Passed: Qamboo result MATCHES Polars result !");
    }
    
    Ok(())
}