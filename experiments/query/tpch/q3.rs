
/*
 *
 * Equivalent SQL:
 * select
 *     l_orderkey,
 *     sum(l_extendedprice*(1-l_discount)) as revenue,
 *     o_orderdate,
 *     o_shippriority
 * from
 *     customer,
 *     orders,
 *     lineitem
 * where
 *     c_mktsegment = '[SEGMENT]'
 *     and c_custkey = o_custkey
 *     and l_orderkey = o_orderkey
 *     and o_orderdate < date '[DATE]'
 *     and l_shipdate > date '[DATE]'
 * group by
 *     l_orderkey,
 *     o_orderdate,
 *     o_shippriority
 * order by
 *     revenue desc,
 *     o_orderdate;
 *
 * Ignores o_shippriority because it is always 0 as per TCPH spec
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
use protocols::protocols::rep3_ring::Rep3RingShare;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::net_statistics::{install_tracing, print_communication_stats};
use experiments::tpch_database_gen::{self, get_orders_table_size};
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::predicate::Predicate;
use table::share_column::ShareColumn;
use table::column_operator::PrefixSum;
use table::column_operator::TransformBetweenArithAndBinary;
use protocols::protocols::rep3_ring::arithmetic::open;
use table::table_operator::Open;
use table::NetStateArgs;
use polars::prelude::*;

const DATE : u64 = 80;


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

    tracing::info!("Q3 with SF: {}", sf);
    
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
    
 
    let (mut lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (mut customer_table, customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());

    let (mut orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());


    tracing::info!("converting some columns to binary");

    let l_shipdate_binary = lineitem_table["l_shipdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column("[l_shipdate]".to_string(), l_shipdate_binary);

    let o_orderdate_binary = orders_table["o_orderdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    orders_table.insert_column("[o_orderdate]".to_string(), o_orderdate_binary);

    let c_mktsegment_binary = customer_table["c_mktsegment"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    customer_table.insert_column("[c_mktsegment]".to_string(), c_mktsegment_binary);

    

    tracing::info!("Projecting tables");

    //*LineItem.project({"[OrderKey]", "[ShipDate]", "ExtendedPrice", "Discount"});
    //*Orders.project({"[OrderKey]", "[CustKey]", "[OrderDate]"});
    //*Customers.project({"[CustKey]", "[MktSegment]"});

    let l_col_names = vec!["l_orderkey", "[l_shipdate]", "l_extendedprice", "l_discount", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let o_col_names = vec!["o_orderkey", "o_custkey", "o_orderdate", "[o_orderdate]", "valid"];
    let mut orders_table = orders_table.project(o_col_names)?;

    let c_col_names = vec!["c_custkey", "[c_mktsegment]", "valid"];
    let mut customer_table = customer_table.project(c_col_names)?;
    
    tracing::info!("Projection completed");


    tracing::info!("Q3 start");
    let tot_start = Instant::now();

    tracing::info!("Filtering tables");

    let _ = customer_table.filter_public(
        "[c_mktsegment]",
        Predicate::EqualBinary,
        &1u64,
        &mut mpc_exec_args,
    )?;
    customer_table.delete_column("[c_mktsegment]");

    let _ = orders_table.filter_public(
        "[o_orderdate]",
        Predicate::LessThanBinary,
        &DATE,
        &mut mpc_exec_args,
    )?;

    let _ = lineitem_table.filter_public(
        "[l_shipdate]",
        Predicate::GreaterThanBinary,
        &DATE,
        &mut mpc_exec_args,
    )?;
    lineitem_table.delete_column("[l_shipdate]");
    orders_table.delete_column("[o_orderdate]");

    
    tracing::info!("Computing revenue");

    let const_element = RingElement(100u64);
    
    let mut revenue: ShareColumn<Rep3RingShare<u64>> = lineitem_table["l_extendedprice"].clone() * 
                                                (&(-lineitem_table["l_discount"].clone() + (const_element, &party_id)), &mut mpc_exec_args)
                                                / (&const_element,&mut mpc_exec_args);

    revenue.update_name("l_revenue".to_string());
    lineitem_table.insert_column("l_revenue".to_string(), revenue);

    lineitem_table.delete_column("l_discount");
    lineitem_table.delete_column("l_extendedprice");

    tracing::info!("Computing revenue completed");


    tracing::info!("Joining tables");
    let start = Instant::now();

    let custorder_table = customer_table.inner_join(
        "c_custkey",
        "o_custkey",
        &orders_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("First join time: {:?}", start.elapsed());


    let start = Instant::now();

    let mut custorderline_table = custorder_table.inner_join(
        "o_orderkey",
        "l_orderkey",
        &lineitem_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Second join time: {:?}", start.elapsed());
    tracing::info!("Joining completed");


    tracing::info!("table group by");
    let start = Instant::now();

    //group key reduction
    let group_by_col_names = vec!["l_orderkey"];
    
    let (e,perm,_) = custorderline_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Group by time: {:?}", start.elapsed());


    tracing::info!("table aggregate");
    let start = Instant::now();

    let to_agg_name = "l_revenue";
    let new_name = "revenue";

    custorderline_table.agg_sum(
        to_agg_name,
        new_name,
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Aggregate time: {:?}", start.elapsed());
    tracing::info!("Aggregate completed");

    let mut final_table = custorderline_table.project(vec!["l_orderkey", "o_orderdate", "revenue", "valid"])?;


    tracing::info!("safe cut custorderline_table size to order_table size");
    final_table.head(get_orders_table_size(sf) as usize);


    tracing::info!("table order by");

    final_table.order_by(
        "o_orderdate",
        true,
        &mut mpc_exec_args,
    )?;

    final_table.order_by(
        "revenue",
        false,
        &mut mpc_exec_args,
    )?;
    
    final_table.order_by(
        "valid",
        false,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Order by completed");


    tracing::info!("Q3 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q3 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q3");
    
    
//************* polars verification *************//

    let sum_valid = final_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    final_table.head(open_valid as usize);

    let mpc_result = final_table.open(&mut mpc_exec_args)?;

    
    if party_id == PartyID::ID0 {
        tracing::info!("Q3 polars:");

        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        let customer = customer_table_polars.unwrap();

        let q3_polars = customer.clone().lazy()
            .filter(col("c_mktsegment").eq(lit(1u64)))
            .join(
                orders.clone().lazy(),
                [col("c_custkey")],
                [col("o_custkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .filter(col("o_orderdate").lt(lit(DATE)))
            .join(
                lineitem.clone().lazy(),
                [col("o_orderkey")],
                [col("l_orderkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .filter(col("l_shipdate").gt(lit(DATE)))
            .with_column(
                (col("l_extendedprice") * (lit(100) - col("l_discount")) / lit(100)).alias("revenue_contribution")
            )
            .group_by([col("o_orderkey"), col("o_orderdate")])
            .agg([
                col("revenue_contribution").sum().alias("revenue")
            ])
            .sort(
                ["revenue", "o_orderdate", "o_orderkey"], 
                SortMultipleOptions::default()
                    .with_order_descending_multi([true, false, false]) // revenue desc, date asc, key asc
                    .with_maintain_order(true) // 保持稳定
            )
            .collect()
            .unwrap();

        let mpc_l_orderkey = mpc_result["l_orderkey"].get_data();
        let mpc_o_orderdate = mpc_result["o_orderdate"].get_data();
        let mpc_revenue = mpc_result["revenue"].get_data();

        let polars_l_orderkey = q3_polars.column("o_orderkey")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_o_orderdate = q3_polars.column("o_orderdate")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_revenue = q3_polars.column("revenue")?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        tracing::info!("rows of polars: {:?}", polars_l_orderkey.len());
        //tracing::info!("MPC l_orderkey: {:?}", mpc_l_orderkey);
        //tracing::info!("Polars l_orderkey: {:?}", polars_l_orderkey);
        //tracing::info!("MPC revenue: {:?}", mpc_revenue);
        //tracing::info!("Polars revenue: {:?}", polars_revenue);
        //tracing::info!("MPC o_orderdate: {:?}", mpc_o_orderdate);
        //tracing::info!("Polars o_orderdate: {:?}", polars_o_orderdate);
        
        assert_eq!(mpc_l_orderkey, &polars_l_orderkey);
        assert_eq!(mpc_o_orderdate, &polars_o_orderdate);
        assert_eq!(mpc_revenue, &polars_revenue);

        tracing::info!("Q3 Passed: Qamboo result MATCHES Polars result !");
    }
    
    Ok(())
}