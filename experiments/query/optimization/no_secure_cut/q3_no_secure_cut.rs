
/*
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
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::column_operator::TransformBetweenArithAndBinary;
use table::predicate::Predicate;
use table::share_column::ShareColumn;
use table::NetStateArgs;

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

    let sf = args.sf; // scale factor for testing
    let partyid= args.party_id.clone();
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
    

    let (mut lineitem_table, _lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (mut customer_table, _customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());

    let (mut orders_table, _orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());


    tracing::info!("converting some columns to binary");

    let l_shipdate_binary = lineitem_table["l_shipdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column("[l_shipdate]".to_string(), l_shipdate_binary);

    let o_orderdate_binary = orders_table["o_orderdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    orders_table.insert_column("[o_orderdate]".to_string(), o_orderdate_binary);

    let c_mktsegment_binary = customer_table["c_mktsegment"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    customer_table.insert_column("[c_mktsegment]".to_string(), c_mktsegment_binary);


    tracing::info!("Projecting tables");

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
    
    tracing::info!("Filtering completed");

    
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

    let k_l_name = "c_custkey";
    let k_r_name = "o_custkey";

    let custorder_table = customer_table.inner_join(
        k_l_name,
        k_r_name,
        &orders_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("First join time: {:?}", start.elapsed());
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


    tracing::info!("Q3 no-secure-cut execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q3 no-secure-cut execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q3 no-secure-cut");
    
    Ok(())
}