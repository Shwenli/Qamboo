/* 
 * select
 *  sum(l_extendedprice * (1 - l_discount) ) as revenue
 * from
 * 	lineitem,
 * 	part
 * where
 * 	(
 * 		p_partkey = l_partkey
 * 		and p_brand = ‘[BRAND1]’
 * 		and p_container in ( ‘SM CASE’, ‘SM BOX’, ‘SM PACK’, ‘SM PKG’)
 * 		and l_quantity >= [QUANTITY1] and l_quantity <= [QUANTITY1] + 10
 * 		and p_size between 1 and 5
 * 		and l_shipmode in (‘AIR’, ‘AIR REG’)
 * 		and l_shipinstruct = ‘DELIVER IN PERSON’
 * 	)
 * 	or
 * 	(
 * 		p_partkey = l_partkey
 * 		and p_brand = ‘[BRAND2]’
 * 		and p_container in (‘MED BAG’, ‘MED BOX’, ‘MED PKG’, ‘MED PACK’)
 * 		and l_quantity >= [QUANTITY2] and l_quantity <= [QUANTITY2] + 10
 * 		and p_size between 1 and 10
 * 		and l_shipmode in (‘AIR’, ‘AIR REG’)
 * 		and l_shipinstruct = ‘DELIVER IN PERSON’
 * 	)
 * 	or
 * 	(
 * 		p_partkey = l_partkey
 * 		and p_brand = ‘[BRAND3]’
 * 		and p_container in ( ‘LG CASE’, ‘LG BOX’, ‘LG PACK’, ‘LG PKG’)
 * 		and l_quantity >= [QUANTITY3] and l_quantity <= [QUANTITY3] + 10
 * 		and p_size between 1 and 15
 * 		and l_shipmode in (‘AIR’, ‘AIR REG’)
 * 		and l_shipinstruct = ‘DELIVER IN PERSON’
 * 	);
 */

use std::path::PathBuf;
use std::vec;
use clap::Parser;
use table::predicate::Predicate;
use std::time::Instant;
use color_eyre::{Result, eyre::Context};
use protocols::protocols::rep3_ring::Rep3State;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::rep3_ring::arithmetic::open;
use protocols::protocols::rep3_ring::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::tpch_database_gen::{self};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Filter, Join, Project};
use table::column_operator::{ColumnBooleanOperator, PrefixSum};
use table::NetStateArgs;
use polars::prelude::*;


const AIR: u64 = 1;
const DELIVER_IN_PERSON: u64 = 1;

const BRAND1 :u64 = 5; 
const BRAND2 :u64 = 12; 
const BRAND3 :u64 = 24;
const QUANTITY1 :u64 = 8; 
const QUANTITY2 :u64 = 19; 
const QUANTITY3 :u64 = 22;


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

    let (part_table, _part_table_polars) = tpch_database_gen::gen_part_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Part table generated with {} rows", part_table.num_rows());


    tracing::info!("converting some columns to binary");

    let l_shipmode_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &lineitem_table["l_shipmode"],
        &mut mpc_exec_args,
    )?;
    lineitem_table.insert_column("[l_shipmode]".to_string(), l_shipmode_binary);

    let l_shipinstruct_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &lineitem_table["l_shipinstruct"],
        &mut mpc_exec_args,
    )?;
    lineitem_table.insert_column("[l_shipinstruct]".to_string(), l_shipinstruct_binary);


    tracing::info!("Projecting tables");
    /*
    L.project(
        {"[ShipMode]", "[ShipInstruct]", "[PartKey]", "ExtendedPrice", "Discount", "Quantity"});
    P.project({"[Size]", "[PartKey]", "[Brand]", "[Container]"});
    */

    let l_col_names = vec!["[l_shipmode]", "[l_shipinstruct]", "l_partkey", "l_extendedprice", "l_discount", "l_quantity", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let p_col_names = vec!["p_size", "p_brand", "p_container", "p_partkey", "valid"];
    let mut part_table = part_table.project(p_col_names)?;

    tracing::info!("Projection completed");

    
    //************* MPC Query Execution ************ *//

    tracing::info!("Q19 start");
    let tot_start = Instant::now();

    tracing::info!("l_shipmode in (AIR, AIR REG) and l_shipinstruct = DELIVER IN PERSON");

    let b1 = lineitem_table["[l_shipmode]"].eq_public_binary(&AIR, &mut mpc_exec_args)?;
    let b2 = lineitem_table["[l_shipinstruct]"].eq_public_binary(&DELIVER_IN_PERSON, &mut mpc_exec_args)?;
    let bool_result = b1.and(&b2, &mut mpc_exec_args)?;

    let _ = lineitem_table.filter_directed_by_bool(bool_result.get_data(), &mut mpc_exec_args)?;
    lineitem_table.delete_column("[l_shipmode]");
    lineitem_table.delete_column("[l_shipinstruct]");

    tracing::info!("p_size > 0");

    let _ = part_table.filter_public("p_size", Predicate::GreaterThan, &0u64, &mut mpc_exec_args)?;


    tracing::info!("p_partkey = l_partkey");

    let mut part_lineitem_table = part_table.inner_join(
        "p_partkey",
        "l_partkey",
        &lineitem_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("sub query");

    /*
     PL["[OrFilter]"] =
        (((PL["[Brand]"] == BRAND1) & (PL["[Container]"] == 0) & (PL["[Quantity]"] >= QUANTITY1) &
          (PL["[Quantity]"] <= QUANTITY1 + 10) & (PL["[Size]"] <= 5)) |
         ((PL["[Brand]"] == BRAND2) & (PL["[Container]"] == 1) & (PL["[Quantity]"] >= QUANTITY2) &
          (PL["[Quantity]"] <= QUANTITY2 + 10) & (PL["[Size]"] <= 10)) |
         ((PL["[Brand]"] == BRAND3) & (PL["[Container]"] == 2) & (PL["[Quantity]"] >= QUANTITY3) &
          (PL["[Quantity]"] <= QUANTITY3 + 10) & (PL["[Size]"] <= 15)));
    */

    let pl_brand_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &part_lineitem_table["p_brand"],
        &mut mpc_exec_args,
    )?;
    part_lineitem_table.insert_column("[p_brand]".to_string(), pl_brand_binary);

    let pl_container_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &part_lineitem_table["p_container"],
        &mut mpc_exec_args,
    )?;
    part_lineitem_table.insert_column("[p_container]".to_string(), pl_container_binary);

    let pl_size_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &part_lineitem_table["p_size"],
        &mut mpc_exec_args,
    )?;
    part_lineitem_table.insert_column("[p_size]".to_string(), pl_size_binary);

    let pl_quantity_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &part_lineitem_table["l_quantity"],
        &mut mpc_exec_args,
    )?;
    part_lineitem_table.insert_column("[l_quantity]".to_string(), pl_quantity_binary);

    let c1_b1 = part_lineitem_table["[p_brand]"].eq_public_binary(&BRAND1, &mut mpc_exec_args)?;
    let c1_b2 = part_lineitem_table["[p_container]"].eq_public_binary(&0u64, &mut mpc_exec_args)?;
    let c1_b3 = part_lineitem_table["[l_quantity]"].ge_public_binary(&QUANTITY1, &mut mpc_exec_args)?;
    let c1_b4 = part_lineitem_table["[l_quantity]"].le_public_binary(&(QUANTITY1 + 10), &mut mpc_exec_args)?;
    let c1_b5 = part_lineitem_table["[p_size]"].le_public_binary(&5, &mut mpc_exec_args)?;
    let c1 = c1_b1.and(&c1_b2, &mut mpc_exec_args)?.and(&c1_b3, &mut mpc_exec_args)?.and(&c1_b4, &mut mpc_exec_args)?.and(&c1_b5, &mut mpc_exec_args)?;
    let c2_b1 = part_lineitem_table["[p_brand]"].eq_public_binary(&BRAND2, &mut mpc_exec_args)?;
    let c2_b2 = part_lineitem_table["[p_container]"].eq_public_binary(&1u64, &mut mpc_exec_args)?;
    let c2_b3 = part_lineitem_table["[l_quantity]"].ge_public_binary(&QUANTITY2, &mut mpc_exec_args)?;
    let c2_b4 = part_lineitem_table["[l_quantity]"].le_public_binary(&(QUANTITY2 + 10), &mut mpc_exec_args)?;
    let c2_b5 = part_lineitem_table["[p_size]"].le_public_binary(&10, &mut mpc_exec_args)?;
    let c2 = c2_b1.and(&c2_b2, &mut mpc_exec_args)?.and(&c2_b3, &mut mpc_exec_args)?.and(&c2_b4, &mut mpc_exec_args)?.and(&c2_b5, &mut mpc_exec_args)?;

    let c3_b1 = part_lineitem_table["[p_brand]"].eq_public_binary(&BRAND3, &mut mpc_exec_args)?;
    let c3_b2 = part_lineitem_table["[p_container]"].eq_public_binary(&2u64, &mut mpc_exec_args)?;
    let c3_b3 = part_lineitem_table["[l_quantity]"].ge_public_binary(&QUANTITY3, &mut mpc_exec_args)?;
    let c3_b4 = part_lineitem_table["[l_quantity]"].le_public_binary(&(QUANTITY3 + 10), &mut mpc_exec_args)?;
    let c3_b5 = part_lineitem_table["[p_size]"].le_public_binary(&15, &mut mpc_exec_args)?;
    let c3 = c3_b1.and(&c3_b2, &mut mpc_exec_args)?.and(&c3_b3, &mut mpc_exec_args)?.and(&c3_b4, &mut mpc_exec_args)?.and(&c3_b5, &mut mpc_exec_args)?;

    let bool_result = c1.or(&c2, &mut mpc_exec_args)?.or(&c3, &mut mpc_exec_args)?;

    let _ = part_lineitem_table.filter_directed_by_bool(bool_result.get_data(), &mut mpc_exec_args)?;

    let final_table = part_lineitem_table.project(vec!["l_extendedprice", "l_discount", "valid"])?;


    tracing::info!("Computing sum revenue");

    let const_element = RingElement(100u64);

    let mut revenue = final_table["l_extendedprice"].clone() * 
                                                (&(-final_table["l_discount"].clone() + (const_element, &party_id)), &mut mpc_exec_args) 
                                                / (&const_element, &mut mpc_exec_args);

    revenue*= (&final_table["valid"], &mut mpc_exec_args);

    //let sum_valid = part_lineitem_table["valid"].prefix_sum();
    let sum_revenue = revenue.prefix_sum();

    let open_sum_revenue = open(sum_revenue, &net0)?;


    tracing::info!("Q19 execution completed");

    if mpc_exec_args.state0.id == PartyID::ID0 {
        tracing::info!("Total Q19 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q19");




    
//************* polars verification *************//

    if state0.id == PartyID::ID0 {
        tracing::info!("Q19 polars:");
        let lineitem = _lineitem_table_polars.unwrap();
        let part = _part_table_polars.unwrap();

        let q_final = part.lazy()
            .join(
                lineitem.lazy(),
                [col("p_partkey")],
                [col("l_partkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .filter(col("l_shipmode").eq(lit(AIR)))
            .filter(col("l_shipinstruct").eq(lit(DELIVER_IN_PERSON)))
            .filter(
                (
                    col("p_brand").eq(lit(BRAND1))
                    .and(col("p_container").eq(lit(0)))
                    .and(col("l_quantity").gt_eq(lit(QUANTITY1)))
                    .and(col("l_quantity").lt_eq(lit(QUANTITY1 + 10)))
                    .and(col("p_size").lt_eq(lit(5)))
                )
                .or(
                    col("p_brand").eq(lit(BRAND2))
                    .and(col("p_container").eq(lit(1)))
                    .and(col("l_quantity").gt_eq(lit(QUANTITY2)))
                    .and(col("l_quantity").lt_eq(lit(QUANTITY2 + 10)))
                    .and(col("p_size").lt_eq(lit(10)))
                )
                .or(
                    col("p_brand").eq(lit(BRAND3))
                    .and(col("p_container").eq(lit(2)))
                    .and(col("l_quantity").gt_eq(lit(QUANTITY3)))
                    .and(col("l_quantity").lt_eq(lit(QUANTITY3 + 10)))
                    .and(col("p_size").lt_eq(lit(15)))
                )
            )
            .select(vec![
                ((col("l_extendedprice") * (lit(100) - col("l_discount"))) / (lit(100))).sum().alias("revenue")
            ])
            .collect()?;

        tracing::info!("Polars result revenue: {:?}", q_final.column("revenue")?.u64()?.get(0).unwrap());
        
        // mpc_revenue_vec is inferred to be Vec<RingElement>
        let mpc_revenue = open_sum_revenue.0;
        let polars_revenue = q_final.column("revenue")?.u64()?.get(0).unwrap();

        assert_eq!(mpc_revenue, polars_revenue, "MPC and Polars results do not match!");
        tracing::info!("Verification passed!");
    }

    Ok(())
}