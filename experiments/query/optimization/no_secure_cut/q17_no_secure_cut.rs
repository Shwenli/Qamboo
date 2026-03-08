/* 
 *  select
 *      sum(l_extendedprice) / 7.0 as avg_yearly
 *  from
 *      lineitem,
 *      part
 *  where
 *      p_partkey = l_partkey
 *      and p_brand = '[BRAND]'
 *      and p_container = '[CONTAINER]'
 *      and l_quantity < (
 *          select
 *              0.2 * avg(l_quantity)
 *          from
 *              lineitem
 *          where
 *              l_partkey = p_partkey
 *      );
 */

use std::path::PathBuf;
use std::vec;
use clap::Parser;
use std::time::Instant;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use algebra::ring::ring_impl::RingElement;
use net::tcp::{TcpNetwork, NetworkConfig};
use protocols::rep3_ring::arithmetic::open;
use primitives::div::{div_share_by_public};
use experiments::tpch_database_gen::{self};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::column_operator::TransformBetweenArithAndBinary;
use table::predicate::Predicate;
use table::NetStateArgs;
use table::column_operator::PrefixSum;
use polars::prelude::*;

const BRAND: u64 = 10;
const CONTAINER: u64 = 10;


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

    let sf = args.sf; // scale factor for testing
    let partyid=args.party_id.clone();
    let default_threads = rayon::current_num_threads();

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


    tracing::info!("Generating tables");

    let (lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (mut part_table, part_table_polars) = tpch_database_gen::gen_part_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Part table generated with {} rows", part_table.num_rows());


    tracing::info!("converting some columns to binary");

    let p_brand_binary = part_table["p_brand"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    part_table.insert_column("[p_brand]".to_string(), p_brand_binary);

    let p_container_binary = part_table["p_container"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    part_table.insert_column("[p_container]".to_string(), p_container_binary);


    tracing::info!("Projecting tables");
    /*
    LineItem.project({"[PartKey]", "Quantity", "ExtendedPrice"});
    Part.project({"[PartKey]", "[Brand]", "[Container]"});
    */

    let l_col_names = vec!["l_partkey", "l_quantity", "l_extendedprice", "valid"];
    let lineitem_table = lineitem_table.project(l_col_names)?;

    let p_col_names = vec!["p_partkey", "[p_brand]", "[p_container]", "valid"];
    let mut part_table = part_table.project(p_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q17 start");
    let tot_start = Instant::now();

    tracing::info!("p_brand = '[BRAND]' and p_container = '[CONTAINER]'");

    let _ = part_table.filter_public(
        "[p_brand]",
        Predicate::EqualBinary,
        &BRAND,
        &mut mpc_exec_args,
    )?;

    let _ = part_table.filter_public(
        "[p_container]",
        Predicate::EqualBinary,
        &CONTAINER,
        &mut mpc_exec_args,
    )?;
    part_table.delete_column("[p_brand]");
    part_table.delete_column("[p_container]");


    tracing::info!("l_partkey = p_partkey");

    let part_lineitem_table = part_table.inner_join(
        
        "p_partkey",
        "l_partkey",
        &lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("sub query");

    tracing::info!("group by l_partkey and compute avg_l_quantity");

    let mut sub_query_table = part_lineitem_table.clone();
    
    let group_by_col_names = vec!["l_partkey"];
    let (e,perm,_) = sub_query_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;

    let _ = sub_query_table.agg_sum(
        "l_quantity",
        "sum_l_quantity",
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;

    let _ = sub_query_table.agg_count(
        "count_l_quantity",
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;

    /*
    //secure cut to part_table size, because group by l_partkey, then the size is at most part_table size
    tracing::info!("secure cut sub query table to part table size");
    sub_query_table.head(get_part_table_size(sf) as usize);
    */

    let mut avg_l_quantity = sub_query_table["sum_l_quantity"].clone() / (&sub_query_table["count_l_quantity"], &mut mpc_exec_args);
    avg_l_quantity /= (&RingElement(5u64), &mut mpc_exec_args);

    avg_l_quantity.update_name("avg_l_quantity".to_string());
    sub_query_table.insert_column("avg_l_quantity".to_string(), avg_l_quantity);

    let col_names = vec!["l_partkey", "avg_l_quantity", "valid"];
    let sub_query_table = sub_query_table.project(col_names)?;
    

    tracing::info!("join with sub table");

    let mut final_table = sub_query_table.inner_join(
        "l_partkey",
        "l_partkey",
        &part_lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("l_quantity < avg_l_quantity");

    let _ = final_table.filter_shared("l_quantity", "avg_l_quantity", Predicate::LessThan, &mut mpc_exec_args)?;


    tracing::info!("sum(l_extendedprice) / 7.0 as avg_yearly");
    let valid = final_table["valid"].clone();

    final_table["l_extendedprice"] *= (&valid, &mut mpc_exec_args);

    let sum_l_extendedprice = final_table["l_extendedprice"].prefix_sum();
    let avg_yearly = div_share_by_public(sum_l_extendedprice, &RingElement(7u64), mpc_exec_args.nets[0], mpc_exec_args.states[0])?;

    let open_avg_yearly = open(avg_yearly, mpc_exec_args.nets[0])?;

    
    tracing::info!("Q17 no-secure-cut execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q17 no-secure-cut execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q17 no-secure-cut");


    
//************* polars verification *************//

    let mut result_table = final_table.clone();

    let _ = result_table.order_by("valid", false, &mut mpc_exec_args)?;
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    
    if party_id == PartyID::ID0 {
        tracing::info!("Q17 polars:");
        let lineitem = lineitem_table_polars.unwrap();
        let part = part_table_polars.unwrap();

        let q1 = part.lazy()
            .filter(col("p_brand").eq(lit(BRAND)))
            .filter(col("p_container").eq(lit(CONTAINER)))
            .join(lineitem.lazy(), [col("p_partkey")], [col("l_partkey")], JoinArgs::new(JoinType::Inner));

        let q17_result = q1
            .clone()
            .group_by([col("p_partkey")])
            .agg([((col("l_quantity").sum() / col("l_quantity").count()) / lit(5)).alias("avg_quantity")])
            .select([col("p_partkey").alias("key"), col("avg_quantity")])
            .join(q1, [col("key")], [col("p_partkey")], JoinArgs::new(JoinType::Inner))
            .filter(col("l_quantity").lt(col("avg_quantity")))
            .select([(col("l_extendedprice").sum() / lit(7)).alias("avg_yearly")])
            .collect()?;

        let polars_avg_yearly = q17_result.column("avg_yearly")?.u64()?.get(0).unwrap_or(0);
        let mpc_result = open_avg_yearly.0;

        //tracing::info!("rows of polars: {:?}", q17_result.height());
        assert_eq!(mpc_result, polars_avg_yearly);
        
        //eprintln!("Multiparty computation result: ");
        //eprintln!("mpc_avg_yearly: {:?}", mpc_result);
        //eprintln!("Polars result: ");
        //eprintln!("polars_avg_yearly: {:?}", polars_avg_yearly);

        tracing::info!("Q17: MPC result verification done.");
    }


    Ok(())
}

