/// Q7 apply reorder join optimization version
/* 
select
 *      supp_nation,
 *      cust_nation,
 *      l_year, sum(volume) as revenue
 *  from (
 *          select
 *              n1.n_name as supp_nation,
 *              n2.n_name as cust_nation,
 *              extract(year from l_shipdate) as l_year,
 *              l_extendedprice * (1 - l_discount) as volume
 *          from
 *              supplier,
 *              lineitem,
 *              orders,
 *              customer,
 *              nation n1,
 *              nation n2
 *          where
 *              s_suppkey = l_suppkey
 *              and o_orderkey = l_orderkey
 *              and c_custkey = o_custkey
 *              and s_nationkey = n1.n_nationkey
 *              and c_nationkey = n2.n_nationkey
 *              and (
 *                  (n1.n_name = '[NATION1]' and n2.n_name = '[NATION2]')
 *                  or (n1.n_name = '[NATION2]' and n2.n_name = '[NATION1]')
 *              )
 *              and l_shipdate between date '1995-01-01' and date '1996-12-31'
 *      ) as shipping
 *  group by
 *      supp_nation,
 *      cust_nation,
 *      l_year
 *  order by
 *      supp_nation,
 *      cust_nation,
 *      l_year;
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
use protocols::protocols::rep3_ring::arithmetic::open;
use net::fast_tcp::{FastTcpNetwork, NetworkConfig};
use experiments::tpch_database_gen::{self};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::column_operator::ColumnBooleanOperator;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::column_operator::PrefixSum;
use table::table_operator::Open;
use table::column_operator::TransformBetweenArithAndBinary;
use table::NetStateArgs;
use polars::prelude::*;


const DATE_LOW : u64 = 90;
const DATE_HIGH : u64 = 110;
const NATION1 : u64 = 3;
const NATION2 : u64 = 5;
//const DATE_START: u64 = 105;



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

    let (mut lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    let (customer_table, customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());
    
    let (supplier_table, supplier_table_polars) = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());

    let (nation1_table, nation1_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation1_table.num_rows());

    let (nation2_table, nation2_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation2_table.num_rows());


    tracing::info!("converting some columns to binary");
    let l_shipdate_binary = lineitem_table["l_shipdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column("[l_shipdate]".to_string(), l_shipdate_binary);


    tracing::info!("Projecting tables");
    /*
    Supplier.project({"[SuppKey]", "[NationKey]"});
    LineItem.project({"[ShipDate]", "ExtendedPrice", "Discount", "[SuppKey]", "[OrderKey]"});
    Orders.project({"[OrderKey]", "[CustKey]"});
    Customer.project({"[CustKey]", "[NationKey]"});
    Nation1.project({"[NationKey]", "[Name]"});
    Nation2.project({"[NationKey]", "[Name]"});
    */
    let lineitem_col_names = vec!["l_shipdate", "[l_shipdate]", "l_extendedprice", "l_discount", "l_suppkey", "l_orderkey", "valid"];
    let mut lineitem_table = lineitem_table.project(lineitem_col_names)?;

    let orders_col_names = vec!["o_orderkey", "o_custkey", "valid"];
    let orders_table = orders_table.project(orders_col_names)?;

    let customer_col_names = vec!["c_custkey", "c_nationkey", "valid"];
    let customer_table = customer_table.project(customer_col_names)?;

    let supplier_col_names = vec!["s_suppkey", "s_nationkey", "valid"];
    let supplier_table = supplier_table.project(supplier_col_names)?;

    let nation1_col_names = vec!["n_nationkey", "n_name", "valid"];
    let nation2_col_names = vec!["n_nationkey", "n_name", "valid"];
    let mut nation1_table = nation1_table.project(nation1_col_names)?;
    let mut nation2_table = nation2_table.project(nation2_col_names)?;

    nation1_table.update_column_name("n_name", "n1_name");
    nation2_table.update_column_name("n_name", "n2_name");
    nation1_table.update_column_name("n_nationkey", "n1_nationkey");
    nation2_table.update_column_name("n_nationkey", "n2_nationkey");

    tracing::info!("Projection completed");


    tracing::info!("Q7 start");
    let tot_start = Instant::now();

    tracing::info!("l_shipdate between date '1995-01-01' and date '1996-12-31'");

    let c1 = lineitem_table["[l_shipdate]"].lt_public_binary(&DATE_HIGH, &mut mpc_exec_args)?;
    let c2 = lineitem_table["[l_shipdate]"].gt_public_binary(&DATE_LOW, &mut mpc_exec_args)?;
    let c = c1.and(&c2, &mut mpc_exec_args)?;

    let _ = lineitem_table.filter_directed_by_bool(c.get_data(), &mut mpc_exec_args)?;
    lineitem_table.delete_column("[l_shipdate]");


    tracing::info!("compute volume");

    let const1 = RingElement(100u64);
    
    let mut volume = lineitem_table["l_extendedprice"].clone() * 
                                                (&(-lineitem_table["l_discount"].clone() + (const1, &party_id)), &mut mpc_exec_args) 
                                                / (&const1, &mut mpc_exec_args);

    volume.update_name("l_volume".to_string());
    lineitem_table.insert_column("l_volume".to_string(), volume);

    lineitem_table.delete_column("l_discount");
    lineitem_table.delete_column("l_extendedprice");


    tracing::info!("join tree 1");

    tracing::info!("c_nationkey = n2.n_nationkey");

    let k_l_name = "n2_nationkey";
    let k_r_name = "c_nationkey";

    let mut nation2_customer_table = nation2_table.inner_join(
            k_l_name,
            k_r_name,
        &customer_table,
        &mut mpc_exec_args,
    )?;

    nation2_customer_table.delete_column("c_nationkey");


    tracing::info!("c_custkey = o_custkey");

    let k_l_name = "c_custkey";
    let k_r_name = "o_custkey";

    let mut nation2_customer_orders_table = nation2_customer_table.inner_join(
        k_l_name,
        k_r_name,
        &orders_table,
        &mut mpc_exec_args,
    )?;

    nation2_customer_orders_table.delete_column("o_custkey");
    
    
    tracing::info!("join tree 2");
    tracing::info!("s_nationkey = n1.n_nationkey");

    let k_l_name = "n1_nationkey";
    let k_r_name = "s_nationkey";

    let mut nation1_supplier_table = nation1_table.inner_join(
        k_l_name,
        k_r_name,
        &supplier_table,
        &mut mpc_exec_args,
    )?;

    nation1_supplier_table.delete_column("s_nationkey");


    tracing::info!("s_suppkey = l_suppkey");

    let k_l_name = "s_suppkey";
    let k_r_name = "l_suppkey";

    let mut nation1_supplier_lineitem_table = nation1_supplier_table.inner_join(
        k_l_name,
        k_r_name,
        &lineitem_table,
        &mut mpc_exec_args,
    )?;

    nation1_supplier_lineitem_table.delete_column("l_suppkey");


    tracing::info!("final join merge join tree 1 2");

    tracing::info!("l_orderkey = o_orderkey");

    
    //nation2_customer_orders_table.print_schema();
    //nation1_supplier_lineitem_table.print_schema();

    let k_l_name = "o_orderkey";
    let k_r_name = "l_orderkey";

    let mut final_table = nation2_customer_orders_table.inner_join(
        k_l_name,
        k_r_name,
        &nation1_supplier_lineitem_table,
        &mut mpc_exec_args,
    )?;

    //tracing::info!("final table created");
    //final_table.print_schema();


    tracing::info!("n1.n_name = '[NATION1]' and n2.n_name = '[NATION2]')
                   or (n1.n_name = '[NATION2]' and n2.n_name = '[NATION1]'");

    let eq_n1_nation1_c1 = final_table["n1_name"].eq_public(&NATION1, &mut mpc_exec_args)?;
    let eq_n2_nation2_c1 = final_table["n2_name"].eq_public(&NATION2, &mut mpc_exec_args)?;
    let condition_1 = eq_n1_nation1_c1.and(&eq_n2_nation2_c1, &mut mpc_exec_args)?;

    let eq_n1_nation2_c2 = final_table["n1_name"].eq_public(&NATION2, &mut mpc_exec_args)?;
    let eq_n2_nation1_c2 = final_table["n2_name"].eq_public(&NATION1, &mut mpc_exec_args)?;
    let condition_2 = eq_n1_nation2_c2.and(&eq_n2_nation1_c2, &mut mpc_exec_args)?;
    let condition = condition_1.or(&condition_2, &mut mpc_exec_args)?;
    
    let _ = final_table.filter_directed_by_bool(condition.get_data(), &mut mpc_exec_args)?;

    tracing::info!("final table project");
    
    let mut final_table = final_table.project(vec!["l_shipdate", "l_volume", "n1_name", "n2_name", "valid"])?;

    final_table.update_column_name("n1_name", "supp_nation");
    final_table.update_column_name("n2_name", "cust_nation");
    final_table.update_column_name("l_shipdate", "l_year");

    tracing::info!("group by supp_nation, cust_nation, l_year");

    let group_key_names = vec!["supp_nation", "cust_nation", "l_year"];

    let (e,perm,_) = final_table.group_by(group_key_names, &mut mpc_exec_args)?;

    tracing::info!("sum(volume) as revenue");

    let to_agg_name = "l_volume";
    let new_agg_name = "revenue";

    let _ = final_table.agg_sum(to_agg_name, new_agg_name, &e, &perm, &mut mpc_exec_args);


    tracing::info!("Q7 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q7 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q7");


//************* polars verification *************//

    let _ = final_table.order_by("valid", false, &mut mpc_exec_args)?;
    
    let mut result_table = final_table.project(vec![
        "supp_nation", 
        "cust_nation", 
        "l_year", 
        "revenue", 
        "valid"
    ])?;

    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q7 Polars Validation:");

        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        let customer = customer_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();
        let n1 = nation1_table_polars.unwrap();
        let n2 = nation2_table_polars.unwrap();

        // Q7 Logic as described
        // q1: (n1 = supp_nation, n2 = cust_nation) => NATION1 -> NATION2 
        // Note: MPC logic uses (n1_name=NATION1 and n2_name=NATION2) OR (n1_name=NATION2 and n2_name=NATION1)
        // Wait, the logic for join tree 1 is: c_nation -> n2 (cust_nation)
        // logic for join tree 2 is: s_nation -> n1 (supp_nation)
        // Then join together.
        // So n1 is supplier nation, n2 is customer nation.
        
        let q7_base = customer.lazy()
            .join(n2.lazy(), [col("c_nationkey")], [col("n_nationkey")], JoinArgs::new(JoinType::Inner))
            .join(orders.lazy(), [col("c_custkey")], [col("o_custkey")], JoinArgs::new(JoinType::Inner))
            .rename(["n_name"], ["cust_nation"], Default::default())
            .join(lineitem.lazy(), [col("o_orderkey")], [col("l_orderkey")], JoinArgs::new(JoinType::Inner))
            .join(supplier.lazy(), [col("l_suppkey")], [col("s_suppkey")], JoinArgs::new(JoinType::Inner))
            .join(n1.lazy(), [col("s_nationkey")], [col("n_nationkey")], JoinArgs::new(JoinType::Inner))
            .rename(["n_name"], ["supp_nation"], Default::default());
        let q7_result = q7_base
            .filter(
                (col("supp_nation").eq(lit(NATION1)).and(col("cust_nation").eq(lit(NATION2))))
                .or(col("supp_nation").eq(lit(NATION2)).and(col("cust_nation").eq(lit(NATION1))))
            )
            .filter(
                col("l_shipdate").gt(lit(DATE_LOW))
                .and(col("l_shipdate").lt(lit(DATE_HIGH)))
            )
            .with_columns([
                 (col("l_extendedprice") * (lit(100) - col("l_discount")) / lit(100)).alias("volume"),
                 col("l_shipdate").alias("l_year") // In this test gen, year is just shipdate? Or logic implies extraction. MPC logic just renamed shipdate to l_year.
            ])
            .group_by([col("supp_nation"), col("cust_nation"), col("l_year")])
            .agg([
                col("volume").sum().alias("revenue")
            ])
            .sort(
                ["supp_nation", "cust_nation", "l_year"],
                SortMultipleOptions::default()
            )
            .collect()
            .unwrap();

        let mpc_supp_nation = mpc_result["supp_nation"].get_data();
        let mpc_cust_nation = mpc_result["cust_nation"].get_data();
        let mpc_l_year = mpc_result["l_year"].get_data();
        let mpc_revenue = mpc_result["revenue"].get_data();

        let polars_supp_nation = q7_result.column("supp_nation")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_cust_nation = q7_result.column("cust_nation")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_l_year = q7_result.column("l_year")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_revenue = q7_result.column("revenue")?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        tracing::info!("Polars rows: {}", polars_revenue.len());
        
        assert_eq!(mpc_supp_nation, &polars_supp_nation, "Supp Nation mismatch");
        assert_eq!(mpc_cust_nation, &polars_cust_nation, "Cust Nation mismatch");
        assert_eq!(mpc_l_year, &polars_l_year, "Year mismatch");
        assert_eq!(mpc_revenue, &polars_revenue, "Revenue mismatch");

        tracing::info!("Q7: MPC result matches polars result!");
    }

    
    Ok(())
}