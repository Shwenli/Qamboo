
/* 
* select
 *      l_shipmode,
 *      sum(case
 *          when o_orderpriority ='1-URGENT'
 *          or o_orderpriority ='2-HIGH'
 *          then 1
 *          else 0
 *      end) as high_line_count,
 *      sum(case
 *          when o_orderpriority <> '1-URGENT'
 *          and o_orderpriority <> '2-HIGH'
 *          then 1
 *          else 0
 *      end) as low_line_count
 *  from
 *      orders,
 *      lineitem
 *  where
 *      o_orderkey = l_orderkey
 *      and l_shipmode in ('[SHIPMODE1]', '[SHIPMODE2]')
 *      and l_commitdate < l_receiptdate
 *      and l_shipdate < l_commitdate
 *      and l_receiptdate >= date '[DATE]'
 *      and l_receiptdate < date '[DATE]' + interval '1' year
 *  group by
 *      l_shipmode
 *  order by
 *      l_shipmode;
 */
 

use std::path::PathBuf;
use std::vec;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use protocols::protocols::rep3_ring::Rep3RingShare;
use protocols::protocols::rep3_ring::Rep3State;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen;
use table::table_operator::{Filter, Project, Open, OrderBy, Groupby, AggFunc, Join};
use table::column_operator::{ColumnBooleanOperator, PrefixSum};
use table::share_column::ShareColumn;
use table::share_column::ShareType;
use table::NetStateArgs;
use protocols::protocols::rep3_ring::id::PartyID;
use protocols::protocols::rep3_ring::arithmetic::open;
use primitives::utils::{get_one_share_vec};
use polars::prelude::*;


const HIGH: u64 = 2;
const URGENT: u64 = 1;

const DATE: u64 = 30;
const DATEANDINTERVAL: u64 = 80;
const SHIPMODE1: u64 = 3;
const SHIPMODE2: u64 = 4;


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
    let party_id = state0.id;

    let mut mpc_exec_args = NetStateArgs::new(
        &nets,
        &mut state0,
        &mut state1,
        &mut states,
    );

    tracing::info!("Network setup completed");
    

    tracing::info!("Generating tables");

    let (mut lineitem_table, _lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (mut orders_table, _orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    tracing::info!("Generate tables completed");

    tracing::info!("converting some columns to binary");

    let l_shipmode_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &lineitem_table["l_shipmode"],
        &mut mpc_exec_args,
    )?;
    lineitem_table.insert_column(l_shipmode_binary.get_name().to_string(),l_shipmode_binary);

    let l_shipdate_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &lineitem_table["l_shipdate"],
        &mut mpc_exec_args,
    )?;
    lineitem_table.insert_column(l_shipdate_binary.get_name().to_string(),l_shipdate_binary);

    let l_commitdate_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &lineitem_table["l_commitdate"],
        &mut mpc_exec_args,
    )?;
    lineitem_table.insert_column(l_commitdate_binary.get_name().to_string(),l_commitdate_binary);

    let l_receiptdate_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &lineitem_table["l_receiptdate"],
        &mut mpc_exec_args,
    )?;
    lineitem_table.insert_column(l_receiptdate_binary.get_name().to_string(),l_receiptdate_binary);

    let o_orderpriority_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &orders_table["o_orderpriority"],
        &mut mpc_exec_args,
    )?;
    orders_table.insert_column(o_orderpriority_binary.get_name().to_string(),o_orderpriority_binary);


    tracing::info!("Projecting tables");
    /*
    L.project({"[ShipMode]", "[CommitDate]", "[ReceiptDate]", "[ShipDate]", "[OrderKey]"});
    O.project({"[OrderPriority]", "[OrderKey]"});
    */
    let l_col_names = vec!["l_shipmode", "[l_shipmode]", "[l_shipdate]", "[l_commitdate]", "l_orderkey", "[l_receiptdate]", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let o_col_names = vec!["[o_orderpriority]", "o_orderkey", "valid"];
    let mut orders_table = orders_table.project(o_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q12 start");
    let tot_start = Instant::now();

    tracing::info!("case when o_orderpriority ='1-URGENT or o_orderpriority ='2-HIGH' then 1 else 0");

    let f_values_high = vec![URGENT, HIGH];

    let high_line_data = get_one_share_vec::<u64>(orders_table["[o_orderpriority]"].len(), party_id)?;
    let mut high_line_column = ShareColumn::<Rep3RingShare<u64>>::new(
        high_line_data,
        ShareType::Arithmetic,
        "highline".to_string(),
    );

    let eq_f = orders_table["[o_orderpriority]"].in_public_binary(&f_values_high, &mut mpc_exec_args)?;
    high_line_column *= (&eq_f, &mut mpc_exec_args);

    orders_table.insert_column(high_line_column.get_name().to_string(), high_line_column);
    orders_table.delete_column("[o_orderpriority]");


    tracing::info!("l_shipmode in ('[SHIPMODE1]', '[SHIPMODE2]')");

    let c1 = lineitem_table["[l_shipmode]"].eq_public_binary(&SHIPMODE1, &mut mpc_exec_args)?;
    let c2 = lineitem_table["[l_shipmode]"].eq_public_binary(&SHIPMODE2, &mut mpc_exec_args)?;
    let s1 = c1.or(&c2, &mut mpc_exec_args)?;

    tracing::info!("l_commitdate < l_receiptdate and l_shipdate < l_commitdate");

    let c1 = lineitem_table["[l_commitdate]"].lt_binary(&lineitem_table["[l_receiptdate]"], &mut mpc_exec_args)?;
    let c2 = lineitem_table["[l_shipdate]"].lt_binary(&lineitem_table["[l_commitdate]"], &mut mpc_exec_args)?;

    let s2 = c1.and(&c2, &mut mpc_exec_args)?;

    tracing::info!("l_receiptdate >= date '[DATE]' and l_receiptdate < date '[DATE]' + interval '1' year");

    let c1 = lineitem_table["[l_receiptdate]"].ge_public_binary(&DATE, &mut mpc_exec_args)?;
    let c2 = lineitem_table["[l_receiptdate]"].lt_public_binary(&DATEANDINTERVAL, &mut mpc_exec_args)?;

    let s3 = c1.and(&c2, &mut mpc_exec_args)?;

    let s = s1.and(&s2, &mut mpc_exec_args)?.and(&s3, &mut mpc_exec_args)?;
    let _ = lineitem_table.filter_directed_by_bool(s.get_data(), &mut mpc_exec_args)?;


    let lineitem_table = lineitem_table.project(vec!["l_shipmode", "l_orderkey", "valid"])?;


    tracing::info!("o_orderkey = l_orderkey");

    let mut final_table = orders_table.inner_join(
        "o_orderkey", 
        "l_orderkey",
        &lineitem_table, 
        &mut mpc_exec_args
    )?;


    tracing::info!("group by l_shipmode");

    let group_by_cols = vec!["l_shipmode"];
    
    let (e,perm,_) = final_table.group_by(
        group_by_cols,
        &mut mpc_exec_args,
    )?;


    tracing::info!("sum(highline)");

    let to_agg_name = "highline";
    let new_agg_name = "high_line_count";

    let _ = final_table.agg_sum(
        to_agg_name,
        new_agg_name,
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;


    tracing::info!("Q12 execution completed");

    if mpc_exec_args.state0.id == PartyID::ID0 {
        tracing::info!("Total Q12 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q12");
    


//************* polars verification *************//

    let mut result_table = final_table.project(vec!["l_shipmode", "high_line_count", "valid"])?;
    // sort valid to top
    let _ = result_table.order_by("valid", false, &mut mpc_exec_args)?;
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, &net0)?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if state0.id == PartyID::ID0 {
        tracing::info!("Q12 polars:");
        let lineitem = _lineitem_table_polars.unwrap();
        let orders = _orders_table_polars.unwrap();

        let final_df = orders.lazy()
            .join(
                lineitem.lazy(),
                [col("o_orderkey")],
                [col("l_orderkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .filter(col("l_shipmode").eq(lit(SHIPMODE1)).or(col("l_shipmode").eq(lit(SHIPMODE2))))
            .filter(col("l_commitdate").lt(col("l_receiptdate")))
            .filter(col("l_shipdate").lt(col("l_commitdate")))
            .filter(
                col("l_receiptdate").gt_eq(lit(DATE))
                .and(col("l_receiptdate").lt(lit(DATEANDINTERVAL)))
            )
            .with_columns(vec![
                col("o_orderpriority").eq(lit(URGENT)).or(col("o_orderpriority").eq(lit(HIGH))).alias("line_count")
            ])
            .group_by(vec![col("l_shipmode")])
            .agg(vec![
                col("line_count").sum().alias("high_line_count")
            ])
            .sort(["l_shipmode"], SortMultipleOptions::default().with_maintain_order(true))
            .collect()?;
        
        tracing::info!("Polars result: {:?}", final_df.height());
        
        let mpc_high_line_count = mpc_result["high_line_count"].get_data();
        let mpc_l_shipmode = mpc_result["l_shipmode"].get_data();

        let polars_high_line_count = final_df.column("high_line_count")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_l_shipmode = final_df.column("l_shipmode")?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        assert_eq!(mpc_high_line_count, polars_high_line_count);
        assert_eq!(mpc_l_shipmode, polars_l_shipmode);
        
        
        tracing::info!("Verification passed!");
    }
    
    Ok(())
}
