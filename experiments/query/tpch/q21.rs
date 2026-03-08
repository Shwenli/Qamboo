/*
 * select
 *   s_name,
 *   count(*) as numwait
 * from
 *   supplier,
 *   lineitem l1,
 *   orders,
 *   nation
 * where
 *   s_suppkey = l1.l_suppkey
 *   and o_orderkey = l1.l_orderkey
 *   and o_orderstatus = 'F'
 *   and l1.l_receiptdate > l1.l_commitdate
 *   and exists (
 *     select
 *       *
 *     from
 *       lineitem l2
 *     where
 *       l2.l_orderkey = l1.l_orderkey
 *       and l2.l_suppkey <> l1.l_suppkey
 *      )
 *   and not exists (
 *     select
 *       *
 *     from
 *       lineitem l3
 *     where
 *       l3.l_orderkey = l1.l_orderkey
 *       and l3.l_suppkey <> l1.l_suppkey
 *       and l3.l_receiptdate > l3.l_commitdate
 *     )
 *   and s_nationkey = n_nationkey
 *   and n_name = '[NATION]'
 * group by
 *   s_name
 * order by
 *   numwait desc,
 *   s_name;
 */


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
use experiments::tpch_database_gen::{self, get_orders_table_size, get_supplier_table_size};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::predicate::Predicate;
use table::column_operator::{ColumnBooleanOperator, PrefixSum, TransformBetweenArithAndBinary};
use table::NetStateArgs;
use table::table_operator::Open;
use polars::prelude::*;

const NATION : u64 = 5;


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

    let (mut orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    let (supplier_table, supplier_table_polars)  = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());

    let (nation_table, nation_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation_table.num_rows());


    tracing::info!("converting some columns to binary");

    let l_receiptdate_binary = lineitem_table["l_receiptdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column("[l_receiptdate]".to_string(), l_receiptdate_binary);

    let l_commitdate_binary = lineitem_table["l_commitdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column("[l_commitdate]".to_string(), l_commitdate_binary);

    let o_orderstatus_binary = orders_table["o_orderstatus"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    orders_table.insert_column("[o_orderstatus]".to_string(), o_orderstatus_binary);


    tracing::info!("Projecting tables");

    let lineitem_col_names = vec!["l_suppkey", "l_orderkey", "[l_receiptdate]", "[l_commitdate]", "valid"];
    let mut lineitem_table = lineitem_table.project(lineitem_col_names)?;

    let orders_col_names = vec!["o_orderkey", "[o_orderstatus]", "valid"];
    let mut orders_table = orders_table.project(orders_col_names)?;

    let supplier_col_names = vec!["s_suppkey", "s_name", "s_nationkey", "valid"];
    let supplier_table = supplier_table.project(supplier_col_names)?;

    let nation_col_names = vec!["n_nationkey", "n_name", "valid"];
    let mut nation_table = nation_table.project(nation_col_names)?;

    let mut l_nofilter_table = lineitem_table.clone();
    l_nofilter_table.delete_column("[l_receiptdate]");
    l_nofilter_table.delete_column("[l_commitdate]");

    tracing::info!("Q21 start");
    let tot_start = Instant::now();

    tracing::info!("n_name = '[NATION]' ");
    let _ = nation_table.filter_public(
        "n_name",
        Predicate::Equal,
        &NATION,
        &mut mpc_exec_args,
    )?;
    nation_table.delete_column("n_name");

    tracing::info!("o_orderstatus = 'F'");
    let _ = orders_table.filter_public(
        "[o_orderstatus]",
        Predicate::EqualBinary,
        &0u64,
        &mut mpc_exec_args,
    )?;
    orders_table.delete_column("[o_orderstatus]");

    tracing::info!("l1.l_receiptdate > l1.l_commitdate");
    let _ = lineitem_table.filter_shared(
        "[l_receiptdate]",
        "[l_commitdate]",
        Predicate::GreaterThanBinary,
        &mut mpc_exec_args,
    )?;
    lineitem_table.delete_column("[l_receiptdate]");
    lineitem_table.delete_column("[l_commitdate]");


    tracing::info!("Tree1: Exists");

    let mut lineitem2_table = lineitem_table.clone();
    lineitem2_table.update_column_name("l_orderkey", "l2_orderkey");
    lineitem2_table.update_column_name("l_suppkey", "l2_suppkey");

    tracing::info!("Group by (l2_orderkey, l2_suppkey)");
    let group_key_names = vec!["l2_orderkey", "l2_suppkey"];
    let (e,perm, _) = lineitem2_table.group_by(group_key_names, &mut mpc_exec_args)?;

    tracing::info!("sum(valid) as sum_valid");
    let _ = lineitem2_table.agg_sum("valid", "sum_valid", &e, &perm, &mut mpc_exec_args)?;

    tracing::info!("compute sum_valid > 0 and make the result into a  exist_late column");
    let mut is_late = lineitem2_table["sum_valid"].gt_public(&0u64, &mut mpc_exec_args)?;
    is_late.from_binary_to_arithmetic(&mut mpc_exec_args)?;
    is_late.update_name("is_late".to_string());
    lineitem2_table.insert_column("is_late".to_string(), is_late);
    lineitem2_table.delete_column("sum_valid");

    tracing::info!("Group by l2_orderkey and sum(is_late) as num_late");
    let (e, perm, _) = lineitem2_table.group_by(vec!["l2_orderkey"], &mut mpc_exec_args)?;
    let _ = lineitem2_table.agg_sum("is_late", "num_late", &e, &perm, &mut mpc_exec_args)?;
    lineitem2_table.delete_column("is_late");

    tracing::info!("Secure Cutting Rows to Ordertable Cardinality");
    lineitem2_table.head(get_orders_table_size(sf) as usize);


    tracing::info!("Tree2: Count");
    let (e, perm, _) = l_nofilter_table.group_by(vec!["l_orderkey"], &mut mpc_exec_args)?;
    let _ = l_nofilter_table.agg_count("cnt_suppkey", &e, &perm, &mut mpc_exec_args)?;
    l_nofilter_table.head(get_orders_table_size(sf) as usize);
    l_nofilter_table.delete_column("l_suppkey");


    tracing::info!("Merge Tree1, Tree2 and lineitem_table");
    let lineitem_table = lineitem2_table.inner_join("l2_orderkey", "l_orderkey", &lineitem_table, &mut mpc_exec_args)?;
    let mut lineitem_table = l_nofilter_table.inner_join("l_orderkey", "l_orderkey", &lineitem_table, &mut mpc_exec_args)?;
    let c1 = lineitem_table["num_late"].eq_public(&1u64, &mut mpc_exec_args)?;
    let c2 = lineitem_table["cnt_suppkey"].gt_public(&1u64, &mut mpc_exec_args)?;
    let condition = c1.and(&c2, &mut mpc_exec_args)?;
    let _ = lineitem_table.filter_directed_by_bool(&condition.get_data(), &mut mpc_exec_args)?;
    

    tracing::info!("s_nationkey = n_nationkey and s_suppkey = l1.l_suppkey and o_orderkey = l1.l_orderkey");
    let nation_supplier_table = nation_table.inner_join(
        "n_nationkey",
        "s_nationkey",
        &supplier_table,
        &mut mpc_exec_args,
    )?;

    let nation_supplier_lineitem_table = nation_supplier_table.inner_join(
        "s_suppkey",
        "l_suppkey",
        &lineitem_table,
        &mut mpc_exec_args,
    )?;

    let mut final_table = orders_table.inner_join(
        "o_orderkey",
        "l_orderkey",
        &nation_supplier_lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("Group by s_name  and count(*) as numwait");
    let (e, perm, _) = final_table.group_by(vec!["s_name"], &mut mpc_exec_args)?;
    let _ = final_table.agg_count( "numwait", &e, &perm, &mut mpc_exec_args)?;

    tracing::info!("secure cut rows to supplier table size");
    final_table.head(get_supplier_table_size(sf) as usize);


    tracing::info!("Order by numwait desc, s_name");
    let _ = final_table.order_by("numwait", false, &mut mpc_exec_args)?;


    tracing::info!("Q21 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q21 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q21");
    


//************* polars verification *************//
    
    let mut result_table = final_table.project(vec!["numwait", "s_name", "valid"])?;
    // sort valid to top
    let _ = result_table.order_by("valid", false, &mut mpc_exec_args)?;

    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q21 polars verification:");

        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();
        let nation = nation_table_polars.unwrap();

        let q1 = lineitem.clone().lazy()
            .group_by([col("l_orderkey")])
            .agg([col("l_suppkey").count().alias("n_supp_by_order")])
            .filter(col("n_supp_by_order").gt(lit(1)))
            .join(
                lineitem.clone().lazy().filter(col("l_receiptdate").gt(col("l_commitdate"))),
                [col("l_orderkey")],
                [col("l_orderkey")],
                JoinArgs::new(JoinType::Inner)
            );
        
        let q21_polars = q1.clone()
            .group_by([col("l_orderkey")])
            .agg([col("l_suppkey").n_unique().alias("n_late_items_per_order")])
            .join(
                q1.clone(), 
                [col("l_orderkey")], 
                [col("l_orderkey")], 
                JoinArgs::new(JoinType::Inner)
            )
            .join(
                supplier.lazy(), 
                [col("l_suppkey")], 
                [col("s_suppkey")], 
                JoinArgs::new(JoinType::Inner)
            )
            .join(
                nation.lazy(), 
                [col("s_nationkey")], 
                [col("n_nationkey")], 
                JoinArgs::new(JoinType::Inner)
            )
            .join(
                orders.lazy(), 
                [col("l_orderkey")], 
                [col("o_orderkey")], 
                JoinArgs::new(JoinType::Inner)
            )
            .filter(col("n_late_items_per_order").eq(lit(1)))
            .filter(col("n_name").eq(lit(NATION))) 
            .filter(col("o_orderstatus").eq(lit(0)))
            .group_by([col("s_name")])
            .agg([len().alias("numwait")])
            .sort(
                ["numwait", "s_name"], 
                SortMultipleOptions::default().with_order_descending_multi(vec![true, false]).with_maintain_order(true)
            )
            .collect()?;

        let mpc_numwait = mpc_result["numwait"].get_data();
        let mpc_sname_vec = mpc_result["s_name"].get_data();
    
        let polars_numwait = q21_polars.column("numwait")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_sname = q21_polars.column("s_name")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        tracing::info!("MPC s_name rows: {}", mpc_sname_vec.len());
        tracing::info!("Polars s_name rows: {}", polars_sname.len());

        assert_eq!(mpc_sname_vec, polars_sname, "s_name mismatch");
        assert_eq!(mpc_numwait, polars_numwait, "numwait mismatch");
        
        
        tracing::info!("Passed! Q21 result matched with Polars.");
    }
    Ok(())
}