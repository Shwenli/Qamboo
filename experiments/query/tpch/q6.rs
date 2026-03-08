
/* 
 *  select
 *      sum(l_extendedprice*l_discount) as revenue
 *  from
 *      lineitem
 *where
 *      l_shipdate >= date '[DATE]'
 *      and l_shipdate < date '[DATE]' + interval '1' year
 *      and l_discount between [DISCOUNT] - 0.01 and [DISCOUNT] + 0.01
 *      and l_quantity < [QUANTITY];
 *
 */
use std::time::Instant;
use std::path::PathBuf;
use std::vec;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use protocols::rep3_ring::arithmetic::open;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::tpch_database_gen;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::column_operator::ColumnBooleanOperator;
use table::column_operator::PrefixSum;
use table::table_operator::{Filter, Project};
use table::NetStateArgs;
use table::column_operator::TransformBetweenArithAndBinary;
use polars::prelude::*;


const DISCOUNT_LOW: u64 = 5;
const DISCOUNT_HIGH: u64 = 7;
const QUANTITY_THRESHOLD: u64 = 40;
const DATE: u64 = 30;
const DATEANDINTERVAL: u64 = 50;


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
    

    let (mut lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;

    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());


    tracing::info!("converting some columns to binary");

    let l_shipdate_binary = lineitem_table["l_shipdate"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column(l_shipdate_binary.get_name().to_string(),l_shipdate_binary);

    let l_discount_binary = lineitem_table["l_discount"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column(l_discount_binary.get_name().to_string(),l_discount_binary);

    let l_quantity_binary = lineitem_table["l_quantity"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column(l_quantity_binary.get_name().to_string(),l_quantity_binary);


    tracing::info!("Projecting tables");

    let l_col_names = vec!["[l_shipdate]", "l_discount", "[l_discount]", "[l_quantity]", "l_extendedprice", "valid"];
    let mut lineitem_table = lineitem_table.project(l_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q6 start");
    let tot_start = Instant::now();

    tracing::info!("Filtering tables: lineitem");

    let c1 = lineitem_table["[l_shipdate]"].ge_public_binary(&DATE, &mut mpc_exec_args)?;
    let c2 = lineitem_table["[l_shipdate]"].lt_public_binary(&DATEANDINTERVAL, &mut mpc_exec_args)?;
    let c3 = lineitem_table["[l_discount]"].ge_public_binary(&DISCOUNT_LOW, &mut mpc_exec_args)?;
    let c4 = lineitem_table["[l_discount]"].lt_public_binary(&DISCOUNT_HIGH, &mut mpc_exec_args)?;
    let c5 = lineitem_table["[l_quantity]"].lt_public_binary(&QUANTITY_THRESHOLD, &mut mpc_exec_args)?;

    let c = c1.and(&c2, &mut mpc_exec_args)?
                  .and(&c3, &mut mpc_exec_args)?
                  .and(&c4, &mut mpc_exec_args)?
                  .and(&c5, &mut mpc_exec_args)?;
    let _ = lineitem_table.filter_directed_by_bool(c.get_data(), &mut mpc_exec_args)?;

    tracing::info!("lineitem filter completed");
    
    tracing::info!("compute prefix sum");

    let revenue = lineitem_table["l_extendedprice"].clone() * (&lineitem_table["l_discount"] ,&mut mpc_exec_args);
    let revenue_after_valid = revenue * (&lineitem_table["valid"] ,&mut mpc_exec_args);

    let revenue = revenue_after_valid.prefix_sum();

    let open_revenue = open(revenue, mpc_exec_args.nets[0])?;
    

    tracing::info!("Q6 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q6 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q6");


    
//************* polars verification *************//

    if party_id == PartyID::ID0 {
        tracing::info!("Q6 polars:");
        let lineitem = lineitem_table_polars.unwrap();

        let q6_result = lineitem.lazy()
            .filter(
                col("l_shipdate").gt_eq(lit(DATE))
                .and(col("l_shipdate").lt(lit(DATEANDINTERVAL)))
            )
            .filter(
                col("l_discount").gt_eq(lit(DISCOUNT_LOW))
                .and(col("l_discount").lt(lit(DISCOUNT_HIGH)))
            )
            .filter(
                col("l_quantity").lt(lit(QUANTITY_THRESHOLD))
            )
            .select([
                (col("l_extendedprice") * col("l_discount")).sum().alias("revenue")
            ])
            .collect()
            .unwrap();


        let polars_revenue = q6_result.column("revenue")?.u64()?.get(0).unwrap();
        
        //tracing::info!("Polars revenue: {}", polars_revenue);
        let mpc_revenue = open_revenue.0;
        assert_eq!(mpc_revenue, polars_revenue);
        
        tracing::info!("Q6: MPC result matches polars result !");
    }

    
    Ok(())
}

// count(*) 的算法是通过 count(valid) 实现的
// 比较是直接基于binary，不需要转换