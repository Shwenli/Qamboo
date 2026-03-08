/* 
*  select
 *  o_year,
 *      sum(case
 *          when nation = '[NATION]'
 *          then volume
 *          else 0
 *      end) / sum(volume) as mkt_share
 *  from (
 *          select
 *              extract(year from o_orderdate) as o_year,
 *              l_extendedprice * (1-l_discount) as volume,
 *              n2.n_name as nation
 *          from
 *              part,
 *              supplier,
 *              lineitem,
 *              orders,
 *              customer,
 *              nation n1,
 *              nation n2,
 *              region
 *          where
 *              p_partkey = l_partkey
 *              and s_suppkey = l_suppkey
 *              and l_orderkey = o_orderkey
 *              and o_custkey = c_custkey
 *              and c_nationkey = n1.n_nationkey
 *              and n1.n_regionkey = r_regionkey
 *              and r_name = '[REGION]'
 *              and s_nationkey = n2.n_nationkey
 *              and o_orderdate between date '1995-01-01' and date '1996-12-31'
 *              and p_type = '[TYPE]'
 *      ) as all_nations
 *  group by
 *      o_year
 *  order by
 *      o_year;
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
use algebra::ring::ring_impl::RingElement;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::tpch_database_gen::{self, get_orders_table_size};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::predicate::Predicate;
use table::column_operator::{ColumnBooleanOperator, PrefixSum, TransformBetweenArithAndBinary};
use table::NetStateArgs;
use table::table_operator::Open;
use protocols::rep3_ring::arithmetic::open;

use polars::prelude::*;

const REGION : u64 = 3; // ASIA
const TYPE : u64 = 3; // "ECONOMY ANODIZED STEEL"
const DATE_LOW : u64 = 60;
const DATE_HIGH : u64 = 100;
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
    
    let (lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    let (customer_table, customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());

    let (part_table, part_table_polars) = tpch_database_gen::gen_part_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Part table generated with {} rows", part_table.num_rows());
    
    let (supplier_table, supplier_table_polars)  = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());

    let (nation1_table, nation1_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation1_table.num_rows());

    let (nation2_table, nation2_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation2_table.num_rows());

    let (region_table, region_table_polars) = tpch_database_gen::gen_region_table(&mut mpc_exec_args)?;
    tracing::info!("Region table generated with {} rows", region_table.num_rows());


    tracing::info!("Projecting tables");

    let lineitem_col_names = vec!["l_extendedprice", "l_discount", "l_partkey", "l_suppkey", "l_orderkey", "valid"];
    let mut lineitem_table = lineitem_table.project(lineitem_col_names)?;

    let orders_col_names = vec!["o_orderkey", "o_custkey", "o_orderdate", "valid"];
    let mut orders_table = orders_table.project(orders_col_names)?;

    let customer_col_names = vec!["c_custkey", "c_nationkey", "valid"];
    let customer_table = customer_table.project(customer_col_names)?;

    let part_col_names = vec!["p_partkey", "p_type", "valid"];
    let mut part_table = part_table.project(part_col_names)?;

    let supplier_col_names = vec!["s_suppkey", "s_nationkey", "valid"];
    let supplier_table = supplier_table.project(supplier_col_names)?;

    let nation1_col_names = vec!["n_nationkey", "n_regionkey", "valid"];
    let nation2_col_names = vec!["n_nationkey", "n_regionkey", "n_name", "valid"];
    let nation1_table = nation1_table.project(nation1_col_names)?;
    let mut nation2_table = nation2_table.project(nation2_col_names)?;
    nation2_table.update_column_name("n_name", "nation");

    let region_col_names = vec!["r_regionkey", "r_name", "valid"];
    let mut region_table = region_table.project(region_col_names)?;
    tracing::info!("Projection completed");


    tracing::info!("Q8 start");
    let tot_start = Instant::now();

    tracing::info!("r_name = '[REGION]'");

    let r_filter_name = "r_name";
    let _ = region_table.filter_public(
        r_filter_name,
        Predicate::Equal,
        &REGION,
        &mut mpc_exec_args,
    )?;


    tracing::info!("p_type = '[TYPE]' ");

    let p_filter_name = "p_type";
    let _ = part_table.filter_public(
        p_filter_name,
        Predicate::Equal,
        &TYPE,
        &mut mpc_exec_args,
    )?;


    tracing::info!("o_orderdate between date '1995-01-01' and date '1996-12-31'");

    let o_filter_name = "o_orderdate";
    let _ = orders_table.filter_public(
        o_filter_name,
        Predicate::LessThan,
        &DATE_HIGH,
        &mut mpc_exec_args,
    )?;

    let _ = orders_table.filter_public(
        o_filter_name,
        Predicate::GreaterThan,
        &DATE_LOW,
        &mut mpc_exec_args,
    )?;


    tracing::info!("compute volume");

    let const1 = RingElement(100u64);
    let mut volume = lineitem_table["l_extendedprice"].clone() * 
                                                (&(-lineitem_table["l_discount"].clone() + (const1, &party_id)), &mut mpc_exec_args)
                                                 /(&const1, &mut mpc_exec_args);

    volume.update_name("l_volume".to_string());
    lineitem_table.insert_column("l_volume".to_string(), volume);

    lineitem_table.delete_column("l_discount");
    lineitem_table.delete_column("l_extendedprice");


    tracing::info!("join tree 1");
    tracing::info!("n1.n_regionkey = r_regionkey");

    let k_l_name = "r_regionkey";
    let k_r_name = "n_regionkey";

    let nation1_region_table = region_table.inner_join(
        k_l_name,
        k_r_name,
        &nation1_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("c_nationkey = n1.n_nationkey");

    let k_l_name = "n_nationkey";
    let k_r_name = "c_nationkey";

    let nation1_region_customer_table = nation1_region_table.inner_join(
        k_l_name,
        k_r_name,
        &customer_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("c_custkey = o_custkey");

    let k_l_name = "c_custkey";
    let k_r_name = "o_custkey";

    let nation1_region_customer_orders_table = nation1_region_customer_table.inner_join(
        k_l_name,
        k_r_name,
        &orders_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("join tree 2");
    tracing::info!("s_nationkey = n2.n_nationkey");

    let k_l_name = "n_nationkey";
    let k_r_name = "s_nationkey";

    let natio2_supplier_table = nation2_table.inner_join(
        k_l_name,
        k_r_name,
        &supplier_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("p_partkey = l_partkey");

    let k_l_name = "p_partkey";
    let k_r_name = "l_partkey";

    let part_lineitem_table =part_table.inner_join(
        k_l_name,
        k_r_name,
        &lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("s_suppkey = l_suppkey");

    let k_l_name = "s_suppkey";
    let k_r_name = "l_suppkey";

    let nation2_supplier_part_lineitem_table = natio2_supplier_table.inner_join(
        k_l_name,
        k_r_name,
        &part_lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("final join merge join tree 1 2");
    tracing::info!("l_orderkey = o_orderkey");

    let nation1_region_customer_orders_table= nation1_region_customer_orders_table.project(vec!["o_orderdate", "o_orderkey", "valid"])?;
    let nation2_supplier_part_lineitem_table= nation2_supplier_part_lineitem_table.project(vec!["l_orderkey", "l_volume", "nation", "valid"])?;

    let k_l_name = "o_orderkey";
    let k_r_name = "l_orderkey";

    let final_table = nation1_region_customer_orders_table.inner_join(
        k_l_name,
        k_r_name,
        &nation2_supplier_part_lineitem_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("final table project");
    
    let mut final_table = final_table.project(vec!["o_orderdate", "l_volume", "nation", "valid"])?;


    tracing::info!("group by o_year");

    let group_key_names = vec!["o_orderdate"];

    let (e,perm, _) = final_table.group_by(group_key_names, &mut mpc_exec_args)?;

    tracing::info!("sum(volume) as mkt_share");

    let to_agg_name = "l_volume";
    let new_agg_name = "sum_volume";

    let _ = final_table.agg_sum(to_agg_name, new_agg_name, &e, &perm, &mut mpc_exec_args);


    tracing::info!("sum(case when nation = '[NATION]' then volume else 0 end)");
    // mutiply volume and eq_nation

    let mut condition = final_table["nation"].eq_public(&NATION, &mut mpc_exec_args)?;
    condition.from_binary_to_arithmetic(&mut mpc_exec_args)?;
    let mut filter_volume = final_table["l_volume"].clone() * (&condition, &mut mpc_exec_args) * RingElement(100u64);

    filter_volume.update_name("filter_volume".to_string());
    final_table.insert_column("filter_volume".to_string(), filter_volume);

    let _ = final_table.agg_sum(
        "filter_volume",
        "sum_filter_volume",
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;

    final_table.delete_column("l_volume");
    final_table.delete_column("nation");
    final_table.delete_column("filter_volume");
    
    tracing::info!("secure cut final table to orders size");
    final_table.head(get_orders_table_size(sf) as usize);


    let _ = final_table.order_by("valid", false, &mut mpc_exec_args)?;

    
    let div = final_table["sum_volume"].clone() + (RingElement(1u64), &party_id); // avoid divide by zero
    let mut mkt_share = final_table["sum_filter_volume"].clone() / (&div, &mut mpc_exec_args);
    mkt_share.update_name("mkt_share".to_string());
    final_table.insert_column("mkt_share".to_string(), mkt_share);

    final_table.delete_column("sum_filter_volume");
    final_table.delete_column("sum_volume");
    

    tracing::info!("Q8 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q8 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q8");



//************* polars verification *************//
    
    let mut result_table = final_table.project(vec![
        "o_orderdate", 
        "mkt_share", 
        "valid"
    ])?;

    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q8 Polars Validation:");

        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        let customer = customer_table_polars.unwrap();
        let part = part_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();
        let n1 = nation1_table_polars.unwrap(); // customer nation
        let n2 = nation2_table_polars.unwrap(); // supplier nation
        let region = region_table_polars.unwrap();

        let q8_result = part.lazy()
            .join(lineitem.lazy(), [col("p_partkey")], [col("l_partkey")], JoinArgs::new(JoinType::Inner))
            .join(supplier.lazy(), [col("l_suppkey")], [col("s_suppkey")], JoinArgs::new(JoinType::Inner))
            .join(orders.lazy(), [col("l_orderkey")], [col("o_orderkey")], JoinArgs::new(JoinType::Inner))
            .join(customer.lazy(), [col("o_custkey")], [col("c_custkey")], JoinArgs::new(JoinType::Inner))
            .join(n1.lazy(), [col("c_nationkey")], [col("n_nationkey")], JoinArgs::new(JoinType::Inner))
            .join(region.lazy(), [col("n_regionkey")], [col("r_regionkey")], JoinArgs::new(JoinType::Inner))
            .filter(col("r_name").eq(lit(REGION)))
            .join(n2.lazy(), [col("s_nationkey")], [col("n_nationkey")], JoinArgs::new(JoinType::Inner))
            .filter(
                col("o_orderdate").gt(lit(DATE_LOW))
                .and(col("o_orderdate").lt(lit(DATE_HIGH)))
            )
            .filter(col("p_type").eq(lit(TYPE)))
            .select([
                col("o_orderdate").alias("o_year"), // Simplify: year extraction if needed, here just date
                (col("l_extendedprice") * (lit(100) - col("l_discount")) / lit(100)).alias("volume"),
                col("n_name_right").alias("nation"), // n2 name (supplier nation). verify suffix if needed.
                                                     // Actually, schema: n1 (left) usually 'n_name', n2 (right) 'n_name_right' or similar. 
                                                     // But Polars join rename logic: n1 joined first.
                                                     // n1 join region => n_name (from n1)
                                                     // then join n2 => n_name duplicated. 
                                                     // Let's check generated table names or assume standard polars handling.
            ])
            .with_columns([
                 when(col("nation").eq(lit(NATION)))
                 .then(col("volume"))
                 .otherwise(lit(0))
                 .alias("_tmp")
            ])
            .group_by(["o_year"])
            .agg([
                (col("_tmp").sum() * lit(100) / col("volume").sum()).alias("mkt_share") // Division logic might differ (integer div in MPC?)
            ])
            .sort(["o_year"], SortMultipleOptions::default().with_maintain_order(true))
            .collect()
            .unwrap();
            
        

        let mpc_o_year = mpc_result["o_orderdate"].get_data();
        let mpc_mkt_share = mpc_result["mkt_share"].get_data();

        let polars_o_year = q8_result.column("o_year")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_mkt_share = q8_result.column("mkt_share")?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        tracing::info!("Polars rows: {}", polars_mkt_share.len());
        
        assert_eq!(mpc_o_year, &polars_o_year, "Year mismatch");
        assert_eq!(mpc_mkt_share, &polars_mkt_share, "Mkt Share mismatch");
        
        tracing::info!("Q8: MPC result matches polars result!");
    }
    
    Ok(())
}