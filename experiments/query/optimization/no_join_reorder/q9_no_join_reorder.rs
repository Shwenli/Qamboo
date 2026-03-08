/* 
* select
 *     nation,
 *     o_year,
 *     sum(amount) as sum_profit
 * from (
 *     select
 *         n_name as nation,
 *         extract(year from o_orderdate) as o_year,
 *         l_extendedprice * (1 - l_discount) - ps_supplycost * l_quantity as amount
 *     from
 *         part,
 *         supplier,
 *         lineitem,
 *         partsupp,
 *         orders,
 *         nation
 *     where
 *         s_suppkey = l_suppkey
 *         and ps_suppkey = l_suppkey
 *         and ps_partkey = l_partkey
 *         and p_partkey = l_partkey
 *         and o_orderkey = l_orderkey
 *         and s_nationkey = n_nationkey
 *         and p_name like '%[COLOR]%'
 *     ) as profit
 * group by
 *     nation,
 *     o_year
 * order by
 *     nation,
 *     o_year desc;
 *
 */
 
use std::path::PathBuf;
use std::vec;
use std::time::Instant;
use clap::{Parser};
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use algebra::ring::ring_impl::RingElement;
use net::tcp::{TcpNetwork, NetworkConfig};
use protocols::rep3_ring::arithmetic::open;
use experiments::tpch_database_gen::{self};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::predicate::Predicate;
use table::column_operator::PrefixSum;
use table::table_operator::Open;
use table::NetStateArgs;
use polars::prelude::*;


const COLOR : u64 = 2; // "forest green"


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

    tracing::info!("Generating tables");

    let (lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    let (part_table, part_table_polars) = tpch_database_gen::gen_part_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Part table generated with {} rows", part_table.num_rows());
    
    let (supplier_table, supplier_table_polars) = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());

    let (partsupp_table, partsupp_table_polars) = tpch_database_gen::gen_partsupp_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Partsupp table generated with {} rows", partsupp_table.num_rows());

    let (nation_table, nation_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation_table.num_rows());


    tracing::info!("Projecting tables");

    let lineitem_col_names = vec!["l_suppkey", "l_partkey", "l_orderkey", "l_extendedprice", "l_discount", "l_quantity", "valid"];
    let mut lineitem_table = lineitem_table.project(lineitem_col_names)?;

    let orders_col_names = vec!["o_orderkey", "o_orderdate", "valid"];
    let orders_table = orders_table.project(orders_col_names)?;

    let part_col_names = vec!["p_partkey", "p_name", "valid"];
    let mut part_table = part_table.project(part_col_names)?;

    let supplier_col_names = vec!["s_suppkey", "s_nationkey", "valid"];
    let supplier_table = supplier_table.project(supplier_col_names)?;

    let partsupp_col_names = vec!["ps_partkey", "ps_suppkey", "ps_supplycost", "valid"];
    let partsupp_table = partsupp_table.project(partsupp_col_names)?;

    let nation_col_names = vec!["n_nationkey", "n_name", "valid"];
    let mut nation_table = nation_table.project(nation_col_names)?;
    nation_table.update_column_name("n_name", "nation");

    tracing::info!("Projection completed");


    tracing::info!("Q9 start");
    let tot_start = Instant::now();

    tracing::info!("p_name = '[COLOR]'");

    let p_filter_name = "p_name";
    let _ = part_table.filter_public(
        p_filter_name,
        Predicate::Equal,
        &COLOR,
        &mut mpc_exec_args,
    )?;

    part_table.delete_column("p_name");


    tracing::info!("compute volume");
    let const_element = RingElement(100u64);
    let mut volume = lineitem_table["l_extendedprice"].clone() * 
                                                (&(-lineitem_table["l_discount"].clone() + (const_element, &party_id)), &mut mpc_exec_args) 
                                                / (&const_element, &mut mpc_exec_args);

    volume.update_name("l_volume".to_string());
    lineitem_table.insert_column("l_volume".to_string(), volume);

    lineitem_table.delete_column("l_discount");
    lineitem_table.delete_column("l_extendedprice");


    /*
        s_suppkey = l_suppkey
        and ps_suppkey = l_suppkey
        and ps_partkey = l_partkey
        and p_partkey = l_partkey
        and o_orderkey = l_orderkey
        and s_nationkey = n_nationkey
    */

    tracing::info!("join: supplier, lineitem, partsupp, part, orders, nation");

    let s_l_table = supplier_table.inner_join(
        "s_suppkey",
        "l_suppkey",
        &lineitem_table,
        &mut mpc_exec_args,
    )?;

    let s_l_ps_table = partsupp_table.inner_join_with_multi_keys(
        vec!["ps_suppkey", "ps_partkey"],
         vec!["l_suppkey", "l_partkey"],
         &s_l_table,
         &mut mpc_exec_args,
    )?;

    let s_l_ps_p_table = part_table.inner_join(
        "p_partkey",
        "l_partkey",
        &s_l_ps_table,
        &mut mpc_exec_args,
    )?;

    let s_l_ps_p_o_table = orders_table.inner_join(
        "o_orderkey",
        "l_orderkey",
        &s_l_ps_p_table,
        &mut mpc_exec_args,
    )?;

    let mut final_table = nation_table.inner_join(
        "n_nationkey",
        "s_nationkey",
        &s_l_ps_p_o_table,
        &mut mpc_exec_args,
    )?;
    

    tracing::info!("l_extendedprice * (1 - l_discount) - ps_supplycost * l_quantity as amount");
    let mut amount = final_table["l_volume"].clone() - (final_table["ps_supplycost"].clone() * (&final_table["l_quantity"], &mut mpc_exec_args));
    amount.update_name("amount".to_string());
    final_table.insert_column("amount".to_string(), amount);

    let final_col_names = vec!["nation", "o_orderdate", "amount", "valid"];
    let mut final_table = final_table.project(final_col_names)?;

    
    tracing::info!("group by nation, o_year");

    let group_key_names = vec!["nation", "o_orderdate"];
    let (e,perm, _) = final_table.group_by(group_key_names, &mut mpc_exec_args)?;


    tracing::info!("sum(amount) as sum_profit");

    let to_agg_name = "amount";
    let new_agg_name = "sum_profit";

    let _ = final_table.agg_sum(to_agg_name, new_agg_name, &e, &perm, &mut mpc_exec_args);
    final_table.delete_column(to_agg_name);
    

    tracing::info!("order by nation, o_year desc");
    let _ = final_table.order_by("o_orderdate", false, &mut mpc_exec_args)?;
    let _ = final_table.order_by("nation", true, &mut mpc_exec_args);
    

    tracing::info!("Q9 no-join-reorder execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q9 no-join-reorder execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q9 no-join-reorder");


//************* polars verification *************//

    let _ = final_table.order_by("valid", false, &mut mpc_exec_args);

    let mut result_table = final_table.project(vec![
        "nation", 
        "o_orderdate", 
        "sum_profit", 
        "valid"
    ])?;


    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;


    if party_id == PartyID::ID0 {
        tracing::info!("Q9 Polars Validation:");

        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        let part = part_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();
        let partsupp = partsupp_table_polars.unwrap();
        let nation = nation_table_polars.unwrap();
        
        // Polars implementation:
        let q9_result = part.lazy()
            .filter(col("p_name").eq(lit(COLOR))) // p_name is just COLOR code in this gen
            .join(partsupp.lazy(), [col("p_partkey")], [col("ps_partkey")], JoinArgs::new(JoinType::Inner))
            .join(supplier.lazy(), [col("ps_suppkey")], [col("s_suppkey")], JoinArgs::new(JoinType::Inner))
            .join(
                lineitem.lazy(),
                [col("p_partkey"), col("ps_suppkey")],
                [col("l_partkey"), col("l_suppkey")],
                JoinArgs::new(JoinType::Inner),
            )
            .join(orders.lazy(), [col("l_orderkey")], [col("o_orderkey")], JoinArgs::new(JoinType::Inner))
            .join(nation.lazy(), [col("s_nationkey")], [col("n_nationkey")], JoinArgs::new(JoinType::Inner))
            
            .select([
                col("n_name").alias("nation"),
                col("o_orderdate").alias("o_year"),
                (
                    (col("l_extendedprice") * (lit(100) - col("l_discount")) / lit(100))
                    - (col("ps_supplycost") * col("l_quantity"))
                ).alias("amount"),
            ])
             
            .group_by(["nation", "o_year"])
            .agg([
                col("amount").sum().alias("sum_profit")
            ])
            .sort(
                ["nation", "o_year"],
                SortMultipleOptions::default().with_order_descending_multi([false, true])
            )
            
            .collect()
            .unwrap();
        
        tracing::info!("Polars rows: {}", q9_result.height());

        let mpc_nation = mpc_result["nation"].get_data();
        let mpc_o_year = mpc_result["o_orderdate"].get_data();
        let mpc_sum_profit = mpc_result["sum_profit"].get_data(); // Signed int actually? 
        // In MPC code: amount = l_volume - ps_supplycost * l_quantity
        // It's using RingElement(u64) but subtraction might wrap if using Rep3RingShare.
        // Assuming result fits and no negative profit (or handled modulary). 
        // sum_profit usually fits in u64 if positive.
        // If profit is negative, standard u64 representation will be large.
        // But for assertion, bit patterns should match.
        // However, Polars u64 subtraction might underflow differently or check overflow.
        // TPCH data usually ensures revenue > cost? No guarantee.
        // Let's assume u64 interpretation is consistent.

        let polars_nation = q9_result.column("nation")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_o_year = q9_result.column("o_year")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_sum_profit = q9_result.column("sum_profit")?.u64()?.into_no_null_iter().collect::<Vec<_>>(); // Cast if needed

        
        
        assert_eq!(mpc_nation, &polars_nation, "Nation mismatch");
        assert_eq!(mpc_o_year, &polars_o_year, "Year mismatch");
        assert_eq!(mpc_sum_profit, &polars_sum_profit, "Profit mismatch");

        tracing::info!("Q9: MPC result matches polars result!");
    }
    
    Ok(())
}