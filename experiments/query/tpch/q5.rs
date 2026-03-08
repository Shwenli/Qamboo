/*
 * select
 *     n_name,
 *     sum(l_extendedprice * (1 - l_discount)) as revenue
 * from
 *     customer,
 *     orders,
 *     lineitem,
 *     supplier,
 *     nation,
 *     region
 * where
 *     c_custkey = o_custkey
 *     and l_orderkey = o_orderkey
 *     and l_suppkey = s_suppkey
 *     and c_nationkey = s_nationkey
 *     and s_nationkey = n_nationkey
 *     and n_regionkey = r_regionkey
 *     and r_name = '[REGION]'
 *     and o_orderdate >= date '[DATE]'
 *     and o_orderdate < date '[DATE]' + interval '1' year
 * group by n_name
 * order by revenue desc;
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
use protocols::rep3_ring::arithmetic::{open};
use protocols::rep3_ring::Rep3RingShare;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen::{self, get_nation_table_size};
use table::column_operator::{ColumnBooleanOperator, PrefixSum};
use table::table_operator::{Filter, Groupby, AggFunc, Join, Open, OrderBy, Project};
use table::column_operator::TransformBetweenArithAndBinary;
use table::predicate::Predicate;
use table::share_column::ShareColumn;
use table::NetStateArgs;
use polars::prelude::*;

const DATE :u64 = 70;
const DATE_INTERVAL :u64 = 30;  // Arbitrary date interval to account for date format
const REGION :u64= 5;


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
    
    
    let (lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (customer_table, customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());

    let (mut orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());

    let (supplier_table, supplier_table_polars) = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());

    let (mut region_table, region_table_polars) = tpch_database_gen::gen_region_table(&mut mpc_exec_args)?;
    tracing::info!("Region table generated with {} rows", region_table.num_rows());

    let (nation_table, nation_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation_table.num_rows());

    tracing::info!("converting some columns to binary");

    let o_orderdate_binary = orders_table["o_orderdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    orders_table.insert_column("[o_orderdate]".to_string(), o_orderdate_binary);

    let r_name_binary = region_table["r_name"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    region_table.insert_column("[r_name]".to_string(), r_name_binary);

    
    tracing::info!("Projecting tables");
    /* 
    Customer.project({"[CustKey]", "[NationKey]"});
    Orders.project({"[OrderKey]", "[CustKey]", "[OrderDate]"});
    Lineitem.project({"[OrderKey]", "[SuppKey]", "ExtendedPrice", "Discount"});
    Supplier.project({"[SuppKey]", "[NationKey]"});
    Nation.project({"[NationKey]", "[RegionKey]", "[Name]"});
    Region.project({"[RegionKey]", "[Name]"});
    */

    let l_col_names = vec!["l_orderkey", "l_suppkey", "l_extendedprice", "l_discount", "valid"];
    let lineitem_table = lineitem_table.project(l_col_names)?;

    let o_col_names = vec!["o_orderkey", "o_custkey", "[o_orderdate]", "valid"];
    let mut orders_table = orders_table.project(o_col_names)?;

    let c_col_names = vec!["c_custkey", "c_nationkey", "valid"];
    let customer_table = customer_table.project(c_col_names)?;

    let s_col_names = vec!["s_suppkey", "s_nationkey", "valid"];
    let supplier_table = supplier_table.project(s_col_names)?;

    let n_col_names = vec!["n_nationkey", "n_regionkey", "n_name", "valid"];
    let nation_table = nation_table.project(n_col_names)?;

    let r_col_names = vec!["r_regionkey", "[r_name]", "valid"];
    let mut region_table = region_table.project(r_col_names)?;
    
    tracing::info!("Projection completed");


    tracing::info!("Q5 start");
    let tot_start = Instant::now();

    tracing::info!("o_orderdate >= date '[DATE]' and o_orderdate < date '[DATE]' + interval '1' year");

    let c1 = orders_table["[o_orderdate]"].ge_public_binary(&DATE, &mut mpc_exec_args)?;
    let c2 = orders_table["[o_orderdate]"].lt_public_binary(&(DATE + DATE_INTERVAL), &mut mpc_exec_args)?;
    let c = c1.and(&c2, &mut mpc_exec_args)?;

    let _ = orders_table.filter_directed_by_bool(c.get_data(), &mut mpc_exec_args)?;
    //let _ = orders_table.filter_public("[o_orderdate]", Predicate::GreaterOrEqualBinary, &DATE, &nets, &mut states)?;
    //let _ = orders_table.filter_public("[o_orderdate]", Predicate::LessThanBinary, &(DATE + DATE_INTERVAL), &nets, &mut states)?;

    orders_table.delete_column("[o_orderdate]");



    tracing::info!("r_name = '[REGION]'");

    let _ = region_table.filter_public("[r_name]", Predicate::EqualBinary, &REGION, &mut mpc_exec_args)?;

    region_table.delete_column("[r_name]");


    tracing::info!("sub query 1: s_nationkey = n_nationkey and n_regionkey = r_regionkey");

    let region_nation_table = region_table.inner_join(
        "r_regionkey",
        "n_regionkey",
        &nation_table,
        &mut mpc_exec_args,
    )?;

    let region_nation_supplier_table = region_nation_table.inner_join(
        "n_nationkey",
        "s_nationkey",
        &supplier_table,
        &mut mpc_exec_args,
    )?;

    
    let sub1_table = region_nation_supplier_table.inner_join(
        "s_suppkey",
        "l_suppkey",
        &lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("sub query 2: c_custkey = o_custkey");

    let sub2_table = customer_table.inner_join(
        "c_custkey",
        "o_custkey",
        &orders_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("merge tree: l_orderkey = o_orderkey");

    let mut final_table = sub2_table.inner_join(
        "o_orderkey",
        "l_orderkey",
        &sub1_table,
        &mut mpc_exec_args,
    )?;

    
    //tracing::info!("c_nationkey = s_nationkey where this is a filter condition");

    let _ = final_table.filter_shared("c_nationkey", "s_nationkey", Predicate::Equal, &mut mpc_exec_args);

    
    tracing::info!("Computing revenue");

    let const_element = RingElement(100u64);
    
    let mut revenue: ShareColumn<Rep3RingShare<u64>> = final_table["l_extendedprice"].clone() * 
                                                (&(-final_table["l_discount"].clone() + (const_element, &party_id)), &mut mpc_exec_args) 
                                                / (&const_element, &mut mpc_exec_args);

    revenue.update_name("l_revenue".to_string());
    final_table.insert_column("l_revenue".to_string(), revenue);

    final_table.delete_column("l_discount");
    final_table.delete_column("l_extendedprice");


     
    tracing::info!("group by n_name");

    let group_by_col_names = vec!["n_name"];

    let (e,perm,_) = final_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;


    tracing::info!("table aggregate");

    let _ = final_table.agg_sum(
        "l_revenue",
        "revenue",
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;


    tracing::info!("secure cut lineorder_table size to nation_table size");
    final_table.head(get_nation_table_size() as usize);


    tracing::info!("order by revenue desc");

    let _ = final_table.order_by(
        "revenue",
        false,
        &mut mpc_exec_args,
    )?;
    

    tracing::info!("Q5 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q5 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q5");



//************* polars verification *************//

    let _ = final_table.order_by("valid", false, &mut mpc_exec_args)?;
    
    let mut result_table = final_table.project(vec!["n_name", "revenue", "valid"])?;

    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q5 polars:");

        let region = region_table_polars.unwrap();
        let nation = nation_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();
        let lineitem = lineitem_table_polars.unwrap();
        let orders = orders_table_polars.unwrap();
        let customer = customer_table_polars.unwrap();

        let q5_sub1 = region.clone().lazy()
        .filter(col("r_name").eq(lit(REGION)))
        .join(
            nation.clone().lazy(),
            [col("r_regionkey")],
            [col("n_regionkey")],
            JoinArgs::new(JoinType::Inner)
        )
        .join(
            supplier.clone().lazy(),
            [col("n_nationkey")],
            [col("s_nationkey")],
            JoinArgs::new(JoinType::Inner)
        )
        .join(
            lineitem.clone().lazy(),
            [col("s_suppkey")],
            [col("l_suppkey")],
            JoinArgs::new(JoinType::Inner)
        );

        
        let q5_final = customer.clone().lazy()
        .join(
            orders.clone().lazy(),
            [col("c_custkey")],
            [col("o_custkey")],
            JoinArgs::new(JoinType::Inner)
        ).join(
            q5_sub1.lazy(),
            [col("o_orderkey")],
            [col("l_orderkey")],
            JoinArgs::new(JoinType::Inner)
        ).filter(
            col("c_nationkey").eq(col("n_nationkey"))
        ).filter(
            col("o_orderdate").gt_eq(lit(DATE)).and(col("o_orderdate").lt(lit(DATE + DATE_INTERVAL)))
        ).with_column(
            (((col("l_extendedprice") * (lit(100) - col("l_discount"))) / lit(100))).alias("revenue_contribution")
        )
        .group_by([col("n_name")])
        .agg([
            col("revenue_contribution").sum().alias("revenue")
        ])
        .sort(["revenue"], SortMultipleOptions::default().with_order_descending(true))
        .collect()
        .unwrap();


        let _q5_result = region.lazy()
        .filter(col("r_name").eq(lit(REGION)))
        .join(
            nation.lazy(),
            [col("r_regionkey")],
            [col("n_regionkey")],
            JoinArgs::new(JoinType::Inner)
        )
        .join(
            supplier.lazy(),
            [col("n_nationkey")],
            [col("s_nationkey")],
            JoinArgs::new(JoinType::Inner)
        )
        .join(
            lineitem.lazy(),
            [col("s_suppkey")],
            [col("l_suppkey")],
            JoinArgs::new(JoinType::Inner)
        )
        .join(
            orders.lazy(),
            [col("l_orderkey")],
            [col("o_orderkey")],
            JoinArgs::new(JoinType::Inner)
        )
        .filter(
            col("o_orderdate").gt_eq(lit(DATE)).and(col("o_orderdate").lt(lit(DATE + DATE_INTERVAL)))
        )
        .join(
            customer.lazy(),
            [col("o_custkey"), col("n_nationkey")], // s_nationkey might be dropped, use n_nationkey
            [col("c_custkey"), col("c_nationkey")],
            JoinArgs::new(JoinType::Inner)
        )
        .with_column(
            (col("l_extendedprice") * ((lit(100) - col("l_discount")) / lit(100))).alias("revenue_contribution")
        )
        .group_by([col("n_name")])
        .agg([
            col("revenue_contribution").sum().alias("revenue")
        ])
        .sort(["revenue"], SortMultipleOptions::default().with_order_descending(true))
        .collect()
        .unwrap();

        //let q5_result = q5_result.head(Some(100));
        //let q5_final = q5_final.head(Some(open_valid as usize));

        

        let mpc_n_name = mpc_result["n_name"].get_data();
        let mpc_revenue = mpc_result["revenue"].get_data();
        //let mpc_valid = mpc_result["valid"].get_data();

        //let polars_n_name_1 = q5_result.column("n_name")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_n_name = q5_final.column("n_name")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_revenue = q5_final.column("revenue")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        

        //println!("MPC n_name: {:?}", mpc_n_name);
        //println!("Polars n_name: {:?}", polars_n_name);
        //println!("MPC revenue: {:?}", mpc_revenue);
        //println!("Polars revenue: {:?}", polars_revenue);
        //println!("MPC valid: {:?}", mpc_valid);
        //assert_eq!(&polars_n_name_1, &polars_n_name);
        tracing::info!("rows of polars_n_name: {:?}", polars_n_name.len());
        assert_eq!(mpc_n_name, &polars_n_name);
        assert_eq!(mpc_revenue, &polars_revenue);

        tracing::info!("Q5 Passed: Qamboo result MATCHES Polars result !");
    }
    
    Ok(())
}
