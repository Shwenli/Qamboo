//* 使用了安全的行数裁剪优化。
/*
  *
 * Equivalent SQL:
 *  select
 *      c_name,
 *      c_custkey,
 *      o_orderkey,
 *      o_orderdate,
 *      o_totalprice,
 *      sum(l_quantity)
 *  from
 *      customer,
 *      orders,
 *      lineitem
 *  where
 *      o_orderkey in (
 *          select
 *              l_orderkey
 *          from
 *              lineitem
 *          group by
 *              l_orderkey
 *          having
 *              sum(l_quantity) > [QUANTITY]
 *      )
 *      and c_custkey = o_custkey
 *      and o_orderkey = l_orderkey
 *  group by
 *      c_name,
 *      c_custkey,
 *      o_orderkey,
 *      o_orderdate,
 *      o_totalprice
 *  order by
 *      o_totalprice desc,
 *      o_orderdate;
 *
 */

use std::path::PathBuf;
use std::vec;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use net::fast_tcp::{FastTcpNetwork, NetworkConfig};
use protocols::protocols::rep3_ring::arithmetic::open;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen::{self, get_orders_table_size};
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project, Open};
use table::column_operator::PrefixSum;
use table::predicate::Predicate;
use table::NetStateArgs;
use polars::prelude::*;


const QUANTITY: u64 = 300;


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


    tracing::info!("Generating tables");
    let (lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (customer_table, customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());

    let (orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    //*Customer.project({"[CustKey]", "[C_Name]"});
    //*Order.project({"[OrderKey]", "[CustKey]", "[OrderDate]", "[TotalPrice]"});
    //*LineItem.project({"[OrderKey]", "Quantity"});

    tracing::info!("Projecting tables");

    let l_col_names = vec!["l_orderkey", "l_quantity", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let o_col_names = vec!["o_orderkey", "o_custkey", "o_orderdate", "o_totalprice", "valid"];
    let orders_table = orders_table.project(o_col_names)?;

    let c_col_names = vec!["c_custkey", "c_name", "valid"];
    let customer_table = customer_table.project(c_col_names)?;
    
    tracing::info!("Projection completed");


    tracing::info!("Q18 start");
    let tot_start = Instant::now();

    tracing::info!("sub query lineitem");

    tracing::info!("group by l_orderkey");
    let group_by_col_names = vec!["l_orderkey"];

    let (e,perm,_) = lineitem_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;


    tracing::info!("sum(l_quantity)");
    let to_agg_name = "l_quantity";
    let new_name = "sum_quantity";

    let _ = lineitem_table.agg_sum(
        to_agg_name,
        new_name,
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;

    lineitem_table.head(get_orders_table_size(sf) as usize);


    tracing::info!("sum_quantity > [QUANTITY]");
    let _ = lineitem_table.filter_public(
        "sum_quantity",
        Predicate::GreaterThan,
        &QUANTITY,
        &mut mpc_exec_args,
    )?;


    tracing::info!("main query");

    tracing::info!("c_custkey = o_custkey");
    let k_l_name = "c_custkey";
    let k_r_name = "o_custkey";

    let custorder_table = customer_table.inner_join(
        k_l_name,
        k_r_name,
        &orders_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("o_orderkey = l_orderkey");
    let k_l_name = "o_orderkey";
    let k_r_name = "l_orderkey";

    let mut final_table = custorder_table.inner_join(
        k_l_name,
        k_r_name,
        &lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("group by o_orderkey, c_custkey");

    let group_by_col_names = vec!["l_orderkey", "o_custkey"]; // only keep l_orderkey and o_custkey

    let (_e,_perm,_) = final_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;


    tracing::info!("order by o_totalprice desc, o_orderdate");

    let _ = final_table.order_by("o_orderdate", true, &mut mpc_exec_args)?;

    let _ = final_table.order_by("o_totalprice", false, &mut mpc_exec_args)?;


    tracing::info!("Q18 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q18 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q18");



//************* polars verification *************//
    
    let mut result_table = final_table.project(vec!["o_totalprice", "sum_quantity", "o_orderdate", "valid"])?;
    // sort valid to top
    let _ = result_table.order_by("valid", false, &mut mpc_exec_args)?;

    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q18 polars:");
        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        let customer = customer_table_polars.unwrap();

        // Subquery: l_orderkey in (select l_orderkey from lineitem group by l_orderkey having sum(l_quantity) > QUANTITY)
        let q1 = lineitem.clone().lazy()
            .group_by([col("l_orderkey")])
            .agg([col("l_quantity").sum().alias("sum_quantity")])
            .filter(col("sum_quantity").gt(lit(QUANTITY)));

        let final_df = orders.lazy()
            .join(q1, [col("o_orderkey")], [col("l_orderkey")], JoinArgs::new(JoinType::Semi))
            .join(lineitem.lazy(), [col("o_orderkey")], [col("l_orderkey")], JoinArgs::new(JoinType::Inner))
            .join(customer.lazy(), [col("o_custkey")], [col("c_custkey")], JoinArgs::new(JoinType::Inner))
            .group_by([col("c_name"), col("o_custkey"), col("o_orderkey"), col("o_orderdate"), col("o_totalprice")])
            .agg([col("l_quantity").sum().alias("col6")])
            .select([
                col("c_name"),
                col("o_custkey").alias("c_custkey"),
                col("o_orderkey"),
                col("o_orderdate"),
                col("o_totalprice"),
                col("col6").alias("sum_quantity"),
            ])
            .sort(
                ["o_totalprice", "o_orderdate", "o_orderkey"],// added o_orderkey to ensure deterministic order
                SortMultipleOptions::default().with_maintain_order(true).with_order_descending_multi([true, false, false])
            )
            .collect()?;

        tracing::info!("Polars result: {:?}", final_df.height());
        
        let mpc_o_totalprice = mpc_result["o_totalprice"].get_data();
        let mpc_sum_quantity = mpc_result["sum_quantity"].get_data();
        let mpc_o_orderdate = mpc_result["o_orderdate"].get_data();

        let polars_o_totalprice = final_df.column("o_totalprice")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_sum_quantity = final_df.column("sum_quantity")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_o_orderdate = final_df.column("o_orderdate")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        assert_eq!(mpc_o_totalprice, polars_o_totalprice, "o_totalprice verification failed!");
        assert_eq!(mpc_sum_quantity, polars_sum_quantity, "sum_quantity verification failed!");
        assert_eq!(mpc_o_orderdate, polars_o_orderdate, "o_orderdate verification failed!");
        
        tracing::info!("Verification passed!");
    }
    
    Ok(())
}