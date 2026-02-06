/**
 *
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
 *
 */

use std::time::Instant;
use std::path::PathBuf;
use std::vec;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use net::tcp::{TcpNetwork, NetworkConfig};
use protocols::protocols::rep3_ring::Rep3State;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen;
use table::column_operator::{ColumnBooleanOperator, PrefixSum};
use table::table_operator::{Filter, Groupby, AggFunc, Join, Open, OrderBy, Project};
use table::predicate::Predicate;
use table::NetStateArgs;
use protocols::protocols::rep3_ring::id::PartyID;
use protocols::protocols::rep3_ring::arithmetic::open;
use polars::prelude::*;


const DATE: u64 = 70;
const DATE_INTERVAL: u64 = 30;


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

    let mut mpc_exec_args = NetStateArgs::new(
        &nets,
        &mut state0,
        &mut state1,
        &mut states,
    );

    tracing::info!("Network setup completed");
    

    let (mut lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (mut orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    let l_commidate_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &lineitem_table["l_commitdate"],
        &mut mpc_exec_args,
    )?;
    lineitem_table.insert_column(l_commidate_binary.get_name().to_string(),l_commidate_binary);

    let l_receiptdate_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &lineitem_table["l_receiptdate"],
        &mut mpc_exec_args,
    )?;
    lineitem_table.insert_column(l_receiptdate_binary.get_name().to_string(),l_receiptdate_binary);


    let o_orderdate_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &orders_table["o_orderdate"],
        &mut mpc_exec_args,
    )?;
    orders_table.insert_column(o_orderdate_binary.get_name().to_string(),o_orderdate_binary);


    tracing::info!("Projecting tables");

    let l_col_names = vec!["l_orderkey", "[l_commitdate]", "[l_receiptdate]", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let o_col_names = vec!["o_orderkey", "o_orderpriority", "[o_orderdate]", "valid"];
    let mut orders_table = orders_table.project(o_col_names)?;
    
    tracing::info!("Projection completed");


    tracing::info!("Q4 start");
    let tot_start = Instant::now();

    tracing::info!("Filtering tables: lineitem");

    let l_lhs = "[l_commitdate]";
    let l_rhs = "[l_receiptdate]";

    let _ = lineitem_table.filter_shared(
        l_lhs,
        l_rhs,
        Predicate::LessThanBinary,
        &mut mpc_exec_args,
    )?;

    lineitem_table.delete_column("[l_commitdate]");
    lineitem_table.delete_column("[l_receiptdate]");

    tracing::info!("lineitem filter completed");


    tracing::info!("o_orderdate >= date '[DATE]' and o_orderdate < date '[DATE]' + interval '3' month");
    
    let c1 = orders_table["[o_orderdate]"].ge_public_binary(&DATE, &mut mpc_exec_args)?;
    let c2 = orders_table["[o_orderdate]"].lt_public_binary(&(DATE + DATE_INTERVAL), &mut mpc_exec_args)?;
    let c = c1.and(&c2, &mut mpc_exec_args)?;
    
    orders_table.filter_directed_by_bool(c.get_data(), &mut mpc_exec_args)?;

    orders_table.delete_column("[o_orderdate]");

    
    tracing::info!("semi join ");

    let k_l_name = "l_orderkey";
    let k_r_name = "o_orderkey";

    let _ = orders_table.semi_join(
        k_r_name,
        k_l_name,
        &lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("group by o_orderpriority");

    let group_by_col_names = vec!["o_orderpriority"];

    let (e,perm,_) = orders_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;


    tracing::info!("count(*) as order_count");

    let _ = orders_table.agg_count(
        "order_count",
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;

    tracing::info!("Aggregate completed");


    tracing::info!("order by o_orderpriority");

    let _ = orders_table.order_by(
        "o_orderpriority",
        true,
        &mut mpc_exec_args,
    )?;


    tracing::info!("Q4 execution completed");

    if mpc_exec_args.state0.id == PartyID::ID0 {
        tracing::info!("Total Q4 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q4");




//************* polars verification *************//

    let _ = orders_table.order_by("valid", false, &mut mpc_exec_args)?;
    let mut result_table = orders_table.project(vec!["o_orderpriority", "order_count", "valid"])?;

    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, &net0)?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if state0.id == PartyID::ID0 {
        tracing::info!("Q4 polars:");

        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        
        let q4_result = orders.lazy()
            .filter(
                col("o_orderdate").gt_eq(lit(DATE))
                .and(col("o_orderdate").lt(lit(DATE + DATE_INTERVAL)))
            )
            .join(
                lineitem.lazy().filter(
                    col("l_commitdate").lt(col("l_receiptdate"))
                ),
                [col("o_orderkey")],
                [col("l_orderkey")],
                JoinArgs::new(JoinType::Semi)
            )
            .group_by([col("o_orderpriority")])
            .agg([
                len().alias("order_count")
            ])
            .sort(["o_orderpriority"], SortMultipleOptions::default())
            .collect()
            .unwrap();

        let mpc_o_orderpriority = mpc_result["o_orderpriority"].get_data();
        let mpc_order_count = mpc_result["order_count"].get_data();

        let polars_o_orderpriority = q4_result.column("o_orderpriority")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_order_count = q4_result.column("order_count")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        tracing::info!("rows of polars: {:?}", polars_o_orderpriority.len());
        assert_eq!(mpc_o_orderpriority, &polars_o_orderpriority);
        assert_eq!(mpc_order_count, &polars_order_count);

        tracing::info!("Q4 Passed: Qamboo result MATCHES Polars result !");
    }

    Ok(())
}
