
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
use protocols::protocols::rep3_ring::Rep3State;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::rep3_ring::Rep3RingShare;
use protocols::protocols::rep3_ring::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::predicate::Predicate;
use table::share_column::ShareColumn;
use table::NetStateArgs;




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
    let partyid= args.party_id.clone();

    tracing::info!("setting up network");

    let mut nets: Vec<TcpNetwork> = Vec::new();
    let mut states: Vec<Rep3State> = Vec::new();

    let file_path0 = PathBuf::from(format!("{}0/config_party{}.toml", args.config_dir.display(), partyid));
    let config: NetworkConfig =toml::from_str(&std::fs::read_to_string(file_path0).context("opening config file")?).context("parsing config file")?;
    let net0 = TcpNetwork::new(config)?;
    let mut state0 = Rep3State::new(&net0)?;

    let file_path1 = PathBuf::from(format!("{}1/config_party{}.toml", args.config_dir.display(), partyid));
    let config: NetworkConfig =toml::from_str(&std::fs::read_to_string(file_path1).context("opening config file")?).context("parsing config file")?;
    let net1 = TcpNetwork::new(config)?;
    let mut state1 = Rep3State::new(&net1)?;

    for i in 2..(2+args.threads){
        let file_path = PathBuf::from(format!("{}{}/config_party{}.toml", args.config_dir.display(), i, partyid));
        let config: NetworkConfig =toml::from_str(&std::fs::read_to_string(file_path).context("opening config file")?).context("parsing config file")?;
        let net = TcpNetwork::new(config)?;
        let state = Rep3State::new(&net)?;
        nets.push(net);
        states.push(state);
    }
    let nets = nets.iter().collect::<Vec<&TcpNetwork>>();
    let mut states = states.iter_mut().collect::<Vec<&mut Rep3State>>();
    let party_id = state0.id;

    let mut mpc_exec_args = NetStateArgs::new(
        &nets,
        &mut state0,
        &mut state1,
        &mut states,
    );

    tracing::info!("Network setup completed");
    

    let (lineitem_table, _lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (customer_table, _customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());

    let (orders_table, _orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    // TODO: Implement Q3 query logic using lineitem_table

    //*LineItem.project({"[OrderKey]", "[ShipDate]", "ExtendedPrice", "Discount"});
    //*Orders.project({"[OrderKey]", "[CustKey]", "[OrderDate]"});
    //*Customers.project({"[CustKey]", "[MktSegment]"});

    tracing::info!("Projecting tables");

    let l_col_names = vec!["l_orderkey", "l_shipdate", "l_extendedprice", "l_discount", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let o_col_names = vec!["o_orderkey", "o_custkey", "o_orderdate", "valid"];
    let mut orders_table = orders_table.project(o_col_names)?;

    let c_col_names = vec!["c_custkey", "c_mktsegment", "valid"];
    let mut customer_table = customer_table.project(c_col_names)?;
    
    tracing::info!("Projection completed");

    tracing::info!("Q3 start");
    
    let tot_start = Instant::now();
    tracing::info!("Filtering tables");

    let c_filter_name = "c_mktsegment";

    let _ = customer_table.filter_public(
        c_filter_name,
        Predicate::Equal,
        &1u64,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Filtering1 completed");

    let o_filter_name = "o_orderdate";
    let _ = orders_table.filter_public(
        o_filter_name,
        Predicate::LessThan,
        &1995u64,
        &mut mpc_exec_args,
    )?;

    let l_filter_name = "l_shipdate";
    let _ = lineitem_table.filter_public(
        l_filter_name,
        Predicate::GreaterThan,
        &1995u64,
        &mut mpc_exec_args,
    )?;

    customer_table.delete_column(c_filter_name);
    
    tracing::info!("Filtering completed");

    
    tracing::info!("Computing revenue");

    let const_element = RingElement(100u64);
    //创造一个bigint类型的100
    
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

    let k_l_name = "c_custkey";
    let k_r_name = "o_custkey";

    let custorder_table = customer_table.inner_join(
        k_l_name,
        k_r_name,
        &orders_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("First join time: {:?}", start.elapsed());
    tracing::info!("now columns: {}", custorder_table.schema.len());
    //检查一下新表的每一列是否有值
    /* 
    for i in 0..custorder_table.schema.len() {
        println!("Column {}: name: {} {}", i, custorder_table.schema[i].get_name(), custorder_table.schema[i].get_data().len());
    }
    */

    let start = Instant::now();

    let k_l_name = "o_orderkey";
    let k_r_name = "l_orderkey";

    let mut lineorder_table = custorder_table.inner_join(
        k_l_name,
        k_r_name,
        &lineitem_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Second join time: {:?}", start.elapsed());
    tracing::info!("Joining completed");


    tracing::info!("table group by");
    let start = Instant::now();

    // 可以只按照l_orderkey分组，因为o_orderdate和l_orderkey是一一对应的
    let group_by_col_names = vec!["l_orderkey"];
    

    let (e,perm,_) = lineorder_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Group by time: {:?}", start.elapsed());


    tracing::info!("table aggregate");
    let start = Instant::now();

    let to_agg_name = "l_revenue";
    let new_name = "revenue";

    lineorder_table.agg_sum(
        to_agg_name,
        new_name,
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Aggregate time: {:?}", start.elapsed());
    tracing::info!("Aggregate completed");


    tracing::info!("table order by");
    let start = Instant::now();

    lineorder_table.order_by(
        "o_orderdate",
        false,
        &mut mpc_exec_args,
    )?;

    lineorder_table.order_by(
        "revenue",
        false,
        &mut mpc_exec_args,
    )?;
    
    lineorder_table.order_by(
        "valid",
        false,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Order by time: {:?}", start.elapsed());
    tracing::info!("Order by completed");


    tracing::info!("Q3 execution completed");

    if mpc_exec_args.state0.id == PartyID::ID0 {
        tracing::info!("Total Q3_no_safe_cut execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q3_no_safe_cut");
    
    Ok(())
}