/*
 *   select
 *      c_custkey,
 *      c_name,
 *      sum(l_extendedprice * (1 - l_discount)) as revenue,
 *      c_acctbal,
 *      n_name,
 *      c_address,
 *      c_phone,
 *      c_comment
 *  from
 *      customer,
 *      orders,
 *      lineitem,
 *      nation
 *  where
 *      c_custkey = o_custkey
 *      and l_orderkey = o_orderkey
 *      and o_orderdate >= date '[DATE]'
 *      and o_orderdate < date '[DATE]' + interval '3' month
 *      and l_returnflag = 'R'
 *      and c_nationkey = n_nationkey
 *  group by
 *      c_custkey,
 *      c_name,
 *      c_acctbal,
 *      c_phone,
 *      n_name,
 *      c_address,
 *      c_comment
 *  order by
 *      revenue desc;
 */

use std::path::PathBuf;
use std::vec;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use algebra::ring::ring_impl::RingElement;
use net::tcp::{TcpNetwork, NetworkConfig};
use protocols::protocols::rep3_ring::arithmetic::{open};
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::predicate::Predicate;
use table::column_operator::PrefixSum;
use table::table_operator::Open;
use table::NetStateArgs;
use experiments::tpch_database_gen;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::timer;
use polars::prelude::*;



const DATE : u64 = 100;
const DATE_INTERVAL : u64 = 10;  
const RETURNFLAG : u64 = 2;


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
    let default_threads = rayon::current_num_threads() / 2;

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
    
    let (lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (customer_table, customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());

    let (orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    let (nation_table, nation_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation_table.num_rows());


    tracing::info!("Projecting tables");

    let l_col_names = vec!["l_orderkey", "l_returnflag", "l_extendedprice", "l_discount", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let o_col_names = vec!["o_orderkey", "o_custkey", "o_orderdate", "valid"];
    let mut orders_table = orders_table.project(o_col_names)?;

    let c_col_names = vec!["c_custkey", "c_name", "c_acctbal", "c_phone", "c_address", "c_comment", "c_nationkey", "valid"];
    let customer_table = customer_table.project(c_col_names)?;
    
    tracing::info!("Projection completed");


    tracing::info!("Q10 start");
    let mut timer = timer::DebugTimer::new();
    
    let tot_start = Instant::now();
    tracing::info!("o_orderdate >= date '[DATE]' and o_orderdate < date '[DATE]' + interval '3' month");

    
    let o_filter_name = "o_orderdate";

    let _ = orders_table.filter_public(
        o_filter_name,
        Predicate::LessThan,
        &(DATE+DATE_INTERVAL),
        &mut mpc_exec_args,
    );

    let _ = orders_table.filter_public(
        o_filter_name,
        Predicate::GreaterOrEqual,
        &DATE,
        &mut mpc_exec_args,
    );


    orders_table.delete_column(o_filter_name);

    let l_filter_name = "l_returnflag";

    let _ = lineitem_table.filter_public(
        l_filter_name,
        Predicate::Equal,
        &RETURNFLAG,
        &mut mpc_exec_args,
    );

    lineitem_table.delete_column(l_filter_name);
    
    
    tracing::info!("Computing revenue");

    let const_element = RingElement(100u64);

    let mut revenue = lineitem_table["l_extendedprice"].clone() * 
                                                (&(-lineitem_table["l_discount"].clone() + (const_element, &party_id)), &mut mpc_exec_args) 
                                                / (&const_element, &mut mpc_exec_args);

    revenue.update_name("l_revenue".to_string());
    lineitem_table.insert_column("l_revenue".to_string(), revenue);

    lineitem_table.delete_column("l_discount");
    lineitem_table.delete_column("l_extendedprice");

    tracing::info!("Computing revenue completed");


    tracing::info!("join: customer, orders, lineitem, nation");

    let c_o_table = customer_table.inner_join(
        "c_custkey",
        "o_custkey",
        &orders_table,
        &mut mpc_exec_args,
    )?;

    let c_o_l_table = c_o_table.inner_join(
        "o_orderkey",
        "l_orderkey",
        &lineitem_table,
        &mut mpc_exec_args,
    )?;

    let mut final_table = nation_table.inner_join(
        "n_nationkey",
        "c_nationkey",
        &c_o_l_table,
        &mut mpc_exec_args,
    )?;

    timer.mark("join completed");

    
    tracing::info!("group by c_custkey, c_name, c_acctbal, c_phone, n_name, c_address, c_comment");
    let group_by_col_names = vec!["o_custkey", "n_name"];

    let (e,perm,_) = final_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;

    timer.mark("group by completed");


    tracing::info!("sum revenue");

    let to_agg_name = "l_revenue";
    let new_name = "revenue";

    final_table.agg_sum(
        to_agg_name,
        new_name,
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;


    tracing::info!("order by revenue desc");

    final_table.order_by(
        "revenue",
        false,
        &mut mpc_exec_args,
    )?;

    
    timer.mark("order by completed");


    tracing::info!("Q10 no-join-reorder execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q10 no-join-reorder execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q10 no-join-reorder");

    
    
//************* polars verification *************//

    let _ = final_table.order_by(
        "valid",
        false,
        &mut mpc_exec_args,
    )?;

    let mut result_table = final_table.project(vec!["o_custkey", "revenue", "valid"])?; 

    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q10 polars:");
        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        let customer = customer_table_polars.unwrap();
        let nation = nation_table_polars.unwrap();
        
        let q10_result = orders.lazy()
            .filter(
                col("o_orderdate").gt_eq(lit(DATE))
                .and(col("o_orderdate").lt(lit(DATE + DATE_INTERVAL)))
            )
            .join(
                customer.lazy().join(
                    nation.lazy(),
                    [col("c_nationkey")],
                    [col("n_nationkey")],
                    JoinArgs::new(JoinType::Inner)
                ),
                [col("o_custkey")],
                [col("c_custkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .join(
                lineitem.lazy().filter(
                    col("l_returnflag").eq(lit(RETURNFLAG))
                ),
                [col("o_orderkey")],
                [col("l_orderkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .group_by([col("o_custkey"), col("c_name"), col("c_acctbal"), col("c_phone"), col("n_name"), col("c_address"), col("c_comment")])
            .agg([
                (col("l_extendedprice") * (lit(100) - col("l_discount")) / lit(100)).sum().alias("revenue")
            ])
            .sort(["revenue", "o_custkey"], SortMultipleOptions::default().with_order_descending_multi([true, false]))
            .collect()
            .unwrap();
            
        let mpc_o_custkey = mpc_result["o_custkey"].get_data();
        let mpc_revenue = mpc_result["revenue"].get_data();

        let polars_o_custkey = q10_result.column("o_custkey")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_revenue = q10_result.column("revenue")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        tracing::info!("rows of polars: {:?}", polars_o_custkey.len());

        assert_eq!(mpc_o_custkey, &polars_o_custkey);
        assert_eq!(mpc_revenue, &polars_revenue);

        tracing::info!("Q10: MPC result matches polars result !");
    }
    

    Ok(())
}