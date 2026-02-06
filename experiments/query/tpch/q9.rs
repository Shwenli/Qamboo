/// Q2 apply reorder join optimization version
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
use protocols::protocols::rep3_ring::Rep3State;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::rep3_ring::id::PartyID;
use protocols::protocols::rep3_ring::arithmetic::open;
use net::tcp::{TcpNetwork, NetworkConfig};
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


    tracing::info!("s_nationkey = n_nationkey");
    
    let k_l_name = "n_nationkey";
    let k_r_name = "s_nationkey";
    
    let mut nation_supplier_table = nation_table.inner_join(
        k_l_name,
        k_r_name,
        &supplier_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("p_partkey = ps_partkey");

    let part_partsupp_table = part_table.inner_join(
        "p_partkey",
        "ps_partkey",
        &partsupp_table,
        &mut mpc_exec_args,
    )?;


    nation_supplier_table.delete_column("s_nationkey");


    tracing::info!("s_suppkey = ps_suppkey");

    let k_l_name = "s_suppkey";
    let k_r_name = "ps_suppkey";

    let nation_supplier_part_partsupp_table = nation_supplier_table.inner_join(
        k_l_name,
        k_r_name,
        &part_partsupp_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("o_orderkey = l_orderkey");

    let k_l_name = "o_orderkey";
    let k_r_name = "l_orderkey";

    let orders_lineitem_table = orders_table.inner_join(
        k_l_name,
        k_r_name,
        &lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("l_partkey = ps_partkey and l_suppkey = ps_suppkey");

    let mut final_table = nation_supplier_part_partsupp_table.inner_join_multi_keys(
        vec!["ps_partkey", "ps_suppkey"],
        vec!["l_partkey", "l_suppkey"],
        &orders_lineitem_table,
        &mut mpc_exec_args,
    )?;


    //tracing::info!("final table created");
    //final_table.print_schema();


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
    

    tracing::info!("Q9 execution completed");

    if mpc_exec_args.state0.id == PartyID::ID0 {
        tracing::info!("Total Q9 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q9");


//************* polars verification *************//

    let _ = final_table.order_by("valid", false, &mut mpc_exec_args);

    let mut result_table = final_table.project(vec![
        "nation", 
        "o_orderdate", 
        "sum_profit", 
        "valid"
    ])?;


    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, &net0)?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;


    if state0.id == PartyID::ID0 {
        tracing::info!("Q9 Polars Validation:");

        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        let part = part_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();
        let partsupp = partsupp_table_polars.unwrap();
        let nation = nation_table_polars.unwrap();

        // Q9 Logic:
        // Join graph: 
        // part -- partsupp -- supplier -- nation
        //            | \
        //            |  \ lineitem -- orders
        //            |
        
        // Note on joins in MPC code:
        // 1. nation join supplier (s_nationkey)
        // 2. + join lineitem (s_suppkey)
        // 3. + join partsupp (l_suppkey=ps_suppkey)
        // 4. + join partsupp (l_partkey=ps_partkey) ? Wait, double join on partsupp?
        //    Re-reading MPC:
        //    nation_supplier_lineitem_table JOIN partsupp ON (ps_suppkey = l_suppkey)
        //    THEN result JOIN partsupp ON (ps_partkey = l_partkey) 
        //    This effectively implies l_suppkey=ps_suppkey AND l_partkey=ps_partkey.
        //    Standard SQL q9 uses: s_suppkey=l_suppkey AND ps_suppkey=l_suppkey AND ps_partkey=l_partkey ...
        // 5. + join part (p_partkey = l_partkey)
        // 6. + join orders (o_orderkey = l_orderkey)
        
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