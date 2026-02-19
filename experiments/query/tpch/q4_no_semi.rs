/*
 *   select
 *       o_orderpriority,
 *       count(*) as order_count
 *   from
 *       orders
 *   where
 *       o_orderdate >= date '[DATE]'
 *       and o_orderdate < date '[DATE]' + interval '3' month
 *       and exists (
 *           select
 *               *
 *           from
 *               lineitem
 *           where
 *               l_orderkey = o_orderkey
 *               and l_commitdate < l_receiptdate
 *       )
 *       group by
 *           o_orderpriority
 *       order by
 *           o_orderpriority;
 */


use std::path::PathBuf;
use std::vec;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use net::fast_tcp::{FastTcpNetwork, NetworkConfig};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use std::time::Instant;
use experiments::tpch_database_gen;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Filter, Groupby, AggFunc, Join, Project};
use table::predicate::Predicate;
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
    let default_threads = rayon::current_num_threads() / 2;
    
    tracing::info!("setting up network");
    let mut nets: Vec<FastTcpNetwork> = Vec::new();
    let mut states: Vec<Rep3State> = Vec::new();

    for i in 0..args.threads {
        let file_path = PathBuf::from(format!("{}{}/config_party{}.toml", args.config_dir.display(), i, partyid));
        let config: NetworkConfig =toml::from_str(&std::fs::read_to_string(file_path).context("opening config file")?).context("parsing config file")?;
        let net = FastTcpNetwork::new(config)?;
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
    
    let nets = nets.iter().collect::<Vec<&FastTcpNetwork>>();
    let mut states = states.iter_mut().collect::<Vec<&mut Rep3State>>();
    let party_id = states[0].id;

    let mut mpc_exec_args = NetStateArgs::new(
        &nets,
        &mut states,
    );

    tracing::info!("Network setup completed");
    

    let (lineitem_table, _lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (orders_table, _orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    // TODO: Implement Q4 query logic using lineitem_table

    //*LineItem.project({"[OrderKey]", "[ShipDate]", "ExtendedPrice", "Discount"});
    //*Orders.project({"[OrderKey]", "[CustKey]", "[OrderDate]"});
    //*Customers.project({"[CustKey]", "[MktSegment]"});

    tracing::info!("Projecting tables");

    let l_col_names = vec!["l_orderkey", "l_commitdate", "l_receiptdate", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let o_col_names = vec!["o_orderkey", "o_orderdate", "o_orderpriority", "valid"];
    let mut orders_table = orders_table.project(o_col_names)?;
    
    tracing::info!("Projection completed");


    tracing::info!("Q4 start");
    let tot_start = Instant::now();

    tracing::info!("Filtering tables: lineitem");

    let l_lhs = "l_commitdate";
    let l_rhs = "l_receiptdate";

    let _ = lineitem_table.filter_shared(
        l_lhs,
        l_rhs,
        Predicate::LessThan,
        &mut mpc_exec_args,
    )?;

    tracing::info!("lineitem filter completed");


    tracing::info!("Filtering tables: orders");
    let start = Instant::now();

    let o_date = "o_orderdate";
    let _ = orders_table.filter_public(
        o_date,
        Predicate::GreaterOrEqual,
        &88u64,
        &mut mpc_exec_args,
    )?;

    let o_date = "o_orderdate";
    let _ = orders_table.filter_public(
        o_date,
        Predicate::LessOrEqual,
        &94u64,
        &mut mpc_exec_args,
    )?;

    tracing::info!("orderstable filter time: {:?}", start.elapsed());

    
    tracing::info!("inner join start");
    let start = Instant::now();

    let k_l_name = "l_orderkey";
    let k_r_name = "o_orderkey";

    let mut orderslineitem_table = orders_table.inner_join(
        k_r_name,
        k_l_name,
        &lineitem_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("inner join time: {:?}", start.elapsed());


    tracing::info!("table group by");
    let start = Instant::now();

    let group_by_col_names = vec!["o_orderpriority"];

    let (e,perm,_) = orderslineitem_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Group by time: {:?}", start.elapsed());
    

    tracing::info!("table aggregate");
    let start = Instant::now();

    let new_agg_name = "order_count";

    let _ = orderslineitem_table.agg_count(
        new_agg_name,
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Aggregate time: {:?}", start.elapsed());
    tracing::info!("Aggregate completed");


    tracing::info!("Q4 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q4_no_semi execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q4_no_semi");

    Ok(())
}