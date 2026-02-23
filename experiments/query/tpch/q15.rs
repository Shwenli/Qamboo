/* 
 *   create view revenue[STREAM_ID] (supplier_no, total_revenue) as
 *      select
 *          l_suppkey,
 *          sum(l_extendedprice * (1 - l_discount))
 *      from
 *          lineitem
 *      where
 *          l_shipdate >= date '[DATE]'
 *          and l_shipdate < date '[DATE]' + interval '3' month
 *      group by
 *          l_suppkey;
 *
 *   select
 *      s_suppkey,
 *      s_name,
 *      s_address,
 *      s_phone,
 *      total_revenue
 *   from
 *      supplier,
 *      revenue[STREAM_ID]
 *   where
 *      s_suppkey = supplier_no
 *      and total_revenue = (
 *          select
 *              max(total_revenue)
 *          from
 *              revenue[STREAM_ID]
 *          )
 *   order by
 *          s_suppkey;
 *
 *   drop view revenue[STREAM_ID];
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
use protocols::protocols::rep3_ring::arithmetic::open;
use protocols::protocols::rep3_ring::Rep3RingShare;
use experiments::tpch_database_gen;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project, Open};
use table::column_operator::TransformBetweenArithAndBinary;
use table::column_operator::PrefixSum;
use table::share_column::ShareColumn;
use table::predicate::Predicate;
use table::NetStateArgs;
use polars::prelude::*;

const DATE: u64 = 80;
const DATE_PLUS_3_MONTH: u64 = 100;


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

    let (mut lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (supplier_table, supplier_table_polars) = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());


    tracing::info!("converting some columns to binary");

    let l_shipdate_binary = lineitem_table["l_shipdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column("[l_shipdate]".to_string(), l_shipdate_binary);


    tracing::info!("Projecting tables");

    let l_col_names = vec!["[l_shipdate]", "l_suppkey", "l_discount", "l_extendedprice", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let s_col_names = vec!["s_suppkey", "s_name", "s_address", "s_phone", "valid"];
    let supplier_table = supplier_table.project(s_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q15 start");
    let tot_start = Instant::now();

    tracing::info!("create view revenue[STREAM_ID] (supplier_no, total_revenue)");

    tracing::info!("l_shipdate >= date '[DATE]' and l_shipdate < date '[DATE]' + interval '1' month");

    let _ = lineitem_table.filter_public(
        "[l_shipdate]",
        Predicate::GreaterOrEqualBinary,
        &DATE,
        &mut mpc_exec_args,
    )?;

    let _ = lineitem_table.filter_public(
        "[l_shipdate]",
        Predicate::LessThanBinary,
        &DATE_PLUS_3_MONTH,
        &mut mpc_exec_args,
    )?;

    lineitem_table.delete_column("[l_shipdate]");


    tracing::info!("Group by l_suppkey");

    let group_by_col_names = vec!["l_suppkey"];

    let (e,perm,_) = lineitem_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;

    
    tracing::info!("sum(l_extendedprice * (1 - l_discount))");

    let const_element = RingElement(100u64);

    let mut revenue: ShareColumn<Rep3RingShare<u64>> = lineitem_table["l_extendedprice"].clone() * 
                                                (&(-lineitem_table["l_discount"].clone() + (const_element, &party_id)), &mut mpc_exec_args) 
                                                / (&const_element, &mut mpc_exec_args);

    revenue.update_name("revenue".to_string());
    lineitem_table.insert_column("revenue".to_string(), revenue);

    let _ = lineitem_table.agg_sum(
        "revenue",
        "total_revenue",
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;


    tracing::info!("select l _suppkey, total_revenue");

    let col_names = vec!["l_suppkey","total_revenue", "valid"];

    lineitem_table.project(col_names)?;

    tracing::info!("revenue[STREAM_ID] created");


    tracing::info!("select max(total_revenue) from revenue[STREAM_ID])");

    let _ = lineitem_table.order_by("valid", false, &mut mpc_exec_args)?;

    let _ = lineitem_table.order_by("total_revenue", false, &mut mpc_exec_args)?;

    let max_total_revenue = lineitem_table["total_revenue"].get_data() [0];


    tracing::info!("s_suppkey = supplier_no and total_revenue = max_total_revenue");

    let mut supplier_lineitem_table = supplier_table.inner_join(
        "s_suppkey",
        "l_suppkey",
        &lineitem_table,
        &mut mpc_exec_args,
    )?;
    let max_total_revenue_vec = vec![max_total_revenue; supplier_lineitem_table.num_rows()];

    let _ = supplier_lineitem_table.filter_shared_with_any_column("total_revenue",  &max_total_revenue_vec, Predicate::Equal, &mut mpc_exec_args)?;

    
    tracing::info!("order by s_suppkey");

    let _ = supplier_lineitem_table.order_by("l_suppkey", false, &mut mpc_exec_args)?;


    tracing::info!("Q15 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q15 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q15");




//************* polars verification *************//

    let _ = supplier_lineitem_table.order_by("valid", false, &mut mpc_exec_args)?;
    let mut result_table = supplier_lineitem_table.project(vec!["l_suppkey", "total_revenue", "valid"])?;
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q15 polars:");
        let lineitem = lineitem_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();

        let revenue_view = lineitem.lazy()
             .filter(
                col("l_shipdate").gt_eq(lit(DATE))
                .and(col("l_shipdate").lt(lit(DATE_PLUS_3_MONTH)))
             )
             .with_columns(vec![
                 ((col("l_extendedprice") * (lit(100) - col("l_discount"))) / (lit(100))).alias("revenue")
             ])
             .group_by(vec![col("l_suppkey")])
             .agg(vec![
                 col("revenue").sum().alias("total_revenue")
             ]);

        let max_total_revenue_df = revenue_view.clone()
            .select([col("total_revenue").max().alias("max_revenue")]);
        
        let max_revenue_val = max_total_revenue_df.collect()?.column("max_revenue")?.get(0)?.try_extract::<u64>()?;

        tracing::info!("Polars max_revenue: {}", max_revenue_val);

        let final_df = supplier.lazy()
            .join(
                revenue_view,
                [col("s_suppkey")],
                [col("l_suppkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .filter(col("total_revenue").eq(lit(max_revenue_val)))
            .select(vec![col("s_suppkey"), col("total_revenue")])
            .sort(["s_suppkey"], SortMultipleOptions::default())
            .collect()?;

        tracing::info!("Polars result: {:?}", final_df.height());
        
        let mpc_s_suppkey = mpc_result["l_suppkey"].get_data();
        let mpc_total_revenue = mpc_result["total_revenue"].get_data();

        let polars_s_suppkey = final_df.column("s_suppkey")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_total_revenue = final_df.column("total_revenue")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        assert_eq!(mpc_s_suppkey, polars_s_suppkey);
        assert_eq!(mpc_total_revenue, polars_total_revenue);
        
        tracing::info!("Verification passed!");
    }

    Ok(())
}

