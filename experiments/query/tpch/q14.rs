/* 
 *  select
 *      100.00 * sum(case
 *                  when p_type like 'PROMO%'
 *                  then l_extendedprice*(1-l_discount)
 *                  else 0
 *              end) / sum(l_extendedprice * (1 - l_discount)) as promo_revenue
 *  from
 *      lineitem,
 *      part
 *  where
 *      l_partkey = p_partkey
 *      and l_shipdate >= date '[DATE]'
 *      and l_shipdate < date '[DATE]' + interval '1' month
 *
 */

use std::path::PathBuf;
use std::vec;
use clap::Parser;
use protocols::rep3_ring::arithmetic::{open};
use primitives::div::{div};
use std::time::Instant;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use algebra::ring::ring_impl::RingElement;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::tpch_database_gen;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use protocols::rep3_ring::Rep3RingShare;
use table::column_operator::TransformBetweenArithAndBinary;
use table::table_operator::{Filter, Join, Project};
use table::share_column::ShareColumn;
use table::column_operator::PrefixSum;
use table::predicate::Predicate;
use table::NetStateArgs;
use polars::prelude::*;

const DATE: u64 = 100;
const DATE_PLUS_1_MONTH: u64 = 110;
const P_TYPE :u64 = 1;


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

    let (part_table, part_table_polars) = tpch_database_gen::gen_part_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Part table generated with {} rows", part_table.num_rows());


    tracing::info!("converting some columns to binary");

    let l_shipdate_binary = lineitem_table["l_shipdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column("[l_shipdate]".to_string(), l_shipdate_binary);


    tracing::info!("Projecting tables");
    //LineItem.project({"[ShipDate]", "[PartKey]", "Discount", "ExtendedPrice"});
    //Part.project({"[PartKey]", "[Type]"});

    let l_col_names = vec!["[l_shipdate]", "l_partkey", "l_discount", "l_extendedprice", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    let p_col_names = vec!["p_partkey", "p_type", "valid"];
    let part_table = part_table.project(p_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q14 start");
    let tot_start = Instant::now();

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
        &DATE_PLUS_1_MONTH,
        &mut mpc_exec_args,
    )?;

    lineitem_table.delete_column("[l_shipdate]");


    tracing::info!("l_partkey = p_partkey");

    let mut part_lineitem_table = part_table.inner_join(
        
        "p_partkey",
        "l_partkey",
        &lineitem_table,
        &mut mpc_exec_args,
    )?;

    
    tracing::info!("compute disc_price");

    let const_element = RingElement(100u64);

    let mut revenue: ShareColumn<Rep3RingShare<u64>> = part_lineitem_table["l_extendedprice"].clone() * 
                                                (&(-part_lineitem_table["l_discount"].clone() + (const_element, &party_id)), &mut mpc_exec_args) 
                                                / (&const_element,&mut mpc_exec_args);

    revenue.update_name("revenue".to_string());
    part_lineitem_table.insert_column("revenue".to_string(), revenue);

    tracing::info!("compute sum revenue for promo");

    let revenue = part_lineitem_table["revenue"].clone()*(&part_lineitem_table["valid"], &mut mpc_exec_args);
    let sum_revenue = revenue.prefix_sum();
    //eprintln!("sum_revenue: {:?}", open(sum_revenue.clone(), &net0));

    tracing::info!("sum(case when p_type like 'PROMO%' then l_extendedprice*(1-l_discount) else 0 end)");

    let _ = part_lineitem_table.filter_public(
        "p_type",
        Predicate::Equal,
        &P_TYPE,
        &mut mpc_exec_args,
    )?;

    let new_revenue = part_lineitem_table["revenue"].clone() * (&part_lineitem_table["valid"], &mut mpc_exec_args);
    let mut sum_new_revenue = new_revenue.prefix_sum();
    sum_new_revenue *= RingElement(100u64);

    //eprintln!("sum_new_revenue: {:?}", open(sum_new_revenue.clone(), &net0));

    let result = div(&sum_new_revenue, &sum_revenue, 32, mpc_exec_args.nets[0], mpc_exec_args.states[0])?;
    
    let open_result = open(result.clone(), mpc_exec_args.nets[0])?;

    tracing::info!("Q14 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q14 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q14");

    

//************* polars verification *************//


    if party_id == PartyID::ID0 {
        tracing::info!("Q14 polars:");
        let lineitem = lineitem_table_polars.unwrap();
        let part = part_table_polars.unwrap();

        let final_df = lineitem.lazy()
             .filter(
                col("l_shipdate").gt_eq(lit(DATE))
                .and(col("l_shipdate").lt(lit(DATE_PLUS_1_MONTH)))
             )
            .join(
                part.lazy(),
                [col("l_partkey")],
                [col("p_partkey")],
                JoinArgs::new(JoinType::Inner)
            )
             .with_columns(vec![
                 ((col("l_extendedprice") * (lit(100) - col("l_discount"))) / (lit(100))).alias("revenue")
             ])
             .select(vec![
                 ((
                    (
                        col("revenue")
                        .filter(col("p_type").eq(lit(P_TYPE)))
                        .sum()
                    ) * lit(100)
                 )/(
                     col("revenue").sum()
                 )).alias("promo_revenue")
             ])
            .collect()?;

        //tracing::info!("Polars result: {:?}", final_df);
        
        let polars_val = final_df.column("promo_revenue")?.get(0)?.try_extract::<u64>()?;
        let mpc_val = open_result.0;
        tracing::info!("MPC result: {}", mpc_val);
        
        assert_eq!(mpc_val, polars_val, "promo_revenue not matched !");
        tracing::info!("Verification passed!");
    }

    Ok(())

}
