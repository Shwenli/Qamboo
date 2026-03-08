
/*
 *  select
 *      c_count, count(*) as custdist
 *  from (
 *      select
 *          c_custkey,
 *          count(o_orderkey)
 *      from
 *          customer left outer join orders on
 *              c_custkey = o_custkey
 *              and o_comment not like ‘%[WORD1]%[WORD2]%’
 *      group by
 *          c_custkey
 *      ) as c_orders (c_custkey, c_count)
 *  group by
 *      c_count
 *  order by
 *      custdist desc,
 *      c_count desc;
 */
//* name is arithmetic share, [name] is binary share

use std::path::PathBuf;
use std::vec;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use protocols::rep3_ring::arithmetic::open;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project, Open};
use table::column_operator::PrefixSum;
use table::predicate::Predicate;
use table::NetStateArgs;
use table::column_operator::TransformBetweenArithAndBinary;
use polars::prelude::*;


const WORD: u64 = 123537;



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
    
    
    let (customer_table, customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());

    let (mut orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    let o_comment_binary = orders_table["o_comment"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    orders_table.insert_column(o_comment_binary.get_name().to_string(), o_comment_binary);


    tracing::info!("Projecting tables");

    let o_col_names = vec!["o_custkey", "[o_comment]", "valid"];
    let mut orders_table = orders_table.project(o_col_names)?;

    let c_col_names = vec!["c_custkey", "valid"];
    let customer_table = customer_table.project(c_col_names)?;
    
    tracing::info!("Projection completed");


    tracing::info!("Q13 start");
    let tot_start = Instant::now();

    tracing::info!("o_comment not like %[WORD1]%[WORD2]%"); 
    let _ = orders_table.filter_public(
        "[o_comment]",
        Predicate::NotEqualBinary,
        &WORD,
        &mut mpc_exec_args,
    )?;

    orders_table.delete_column("[o_comment]");
    

    tracing::info!("orders table group by o_custkey");
    let group_by_col_names = vec!["o_custkey"];
    let (e,perm,_) = orders_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;


    tracing::info!("agg count(o_orderkey)");
    let new_agg_name = "c_count";
    let _ = orders_table.agg_count(
        new_agg_name,
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;


    tracing::info!("Reduce orders table to customer table size");
    let customers_size = tpch_database_gen::get_customer_table_size(sf);
    orders_table.head(customers_size as usize);

    tracing::info!("orders table reduced to {} rows", orders_table.num_rows());


    tracing::info!("c_custkey = o_custkey");
    let mut final_table = customer_table.inner_join(
        "c_custkey",
        "o_custkey",
        &orders_table,
        &mut mpc_exec_args,
    )?;

    final_table.delete_column("c_custkey");


    tracing::info!("group by c_count");
    let group_by_col_names = vec!["c_count"];

    let (e,perm,_) = final_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;


    tracing::info!("count(*) as custdist");
    let new_agg_name = "custdist";

    let _ = final_table.agg_count(
        new_agg_name,
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;


    tracing::info!("sort custdist desc, c_count desc");
    
    let _ = final_table.order_by("c_count", false, &mut mpc_exec_args)?;
    let _ = final_table.order_by("custdist", false, &mut mpc_exec_args)?;


    tracing::info!("Q13 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q13 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q13");




//************* polars verification *************//

    let mut result_table = final_table.project(vec!["c_count", "custdist", "valid"])?;
    // sort valid to top
    let _ = result_table.order_by("valid", false, &mut mpc_exec_args)?;
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q13 polars:");
        let customer = customer_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();

        let o_filtered = orders.lazy()
            .filter(col("o_comment").neq(lit(WORD)));
        let final_df = customer.lazy()
            .join(
                o_filtered,
                [col("c_custkey")],
                [col("o_custkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .group_by(vec![col("c_custkey")])
            .agg(vec![
                len().alias("c_count")
            ])
            .group_by(vec![col("c_count")])
            .agg(vec![
                len().alias("custdist")
            ])
            .sort(
                ["custdist", "c_count"],
                SortMultipleOptions::default()
                    .with_order_descending(true)
            )
            .collect()?;

        
        tracing::info!("Polars result: {:?}", final_df.height());
        
        let mpc_c_count = mpc_result["c_count"].get_data();
        let mpc_custdist = mpc_result["custdist"].get_data();

        let polars_c_count = final_df.column("c_count")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_custdist = final_df.column("custdist")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        assert_eq!(mpc_c_count, polars_c_count);
        assert_eq!(mpc_custdist, polars_custdist);
        
        tracing::info!("Verification passed!");
    }
    
    Ok(())
}
//* q13是不是采用了预聚合