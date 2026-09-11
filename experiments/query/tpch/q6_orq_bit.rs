
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

/*
 * ORQ-style Q6 with a Bit valid column: like q6_orq, all five predicates are
 * comparisons against public constants, so the subtractions are moved to a
 * preprocessing phase and each predicate is a purely local sign-bit extraction
 * (ltz_bit) at query time.
 *
 * Unlike q6_orq, the predicate chain and the valid column never leave the Bit
 * domain: ltz_bit returns Rep3RingShare<Bit> columns, and_bit ANDs them at a
 * cost of 1 bit per row per gate (q6_orq widens to u64 and pays 8 bytes per
 * gate). The valid column is generated directly as Bit shares
 * (gen_valid_column_bit) and ANDed with the composed predicate in the Bit
 * domain as well. Since Bit share components are single bits by construction,
 * the result feeds straight into from_bit_to_arithmetic_t (bit_inject) to mask
 * the revenue — no bit_extract cleanup and no u64 widening of the filter is
 * needed.
 */
use std::time::Instant;
use std::path::PathBuf;
use std::vec;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use primitives::transform::from_bit_to_arithmetic_t_multithreads;
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use protocols::rep3_ring::arithmetic::open;
use algebra::ring::ring_impl::RingElement;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::tpch_database_gen;
use experiments::gen_valid_column_bit;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::column_operator::ColumnBooleanOperator;
use table::column_operator::PrefixSum;
use table::share_column::ShareColumn;
use table::share_column::ShareType;
use table::table_operator::Project;
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

    // the valid column as Bit shares: the whole filter chain stays in the Bit domain
    let valid_bit = gen_valid_column_bit(lineitem_table.num_rows(), "valid_bit".to_string(), mpc_exec_args.nets, party_id);


    //************* preprocessing: materialize difference columns (subtraction moved here) *************//
    let prep_start = Instant::now();
    tracing::info!("Preprocessing: materializing difference columns");

    // x >= c  <=>  ltz((c-1) - x); computed as (-x) + (c-1)
    let mut d_shipdate_ge_date = -lineitem_table["l_shipdate"].clone() + (RingElement(DATE - 1), &party_id);
    d_shipdate_ge_date.update_name("d_shipdate_ge_date".to_string());
    let d_shipdate_ge_date = d_shipdate_ge_date.add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column(d_shipdate_ge_date.get_name().to_string(), d_shipdate_ge_date);

    // x < c  <=>  ltz(x - c)
    let mut d_shipdate_lt_end = lineitem_table["l_shipdate"].clone() + (RingElement(DATEANDINTERVAL.wrapping_neg()), &party_id);
    d_shipdate_lt_end.update_name("d_shipdate_lt_end".to_string());
    let d_shipdate_lt_end = d_shipdate_lt_end.add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column(d_shipdate_lt_end.get_name().to_string(), d_shipdate_lt_end);

    let mut d_discount_ge_low = -lineitem_table["l_discount"].clone() + (RingElement(DISCOUNT_LOW - 1), &party_id);
    d_discount_ge_low.update_name("d_discount_ge_low".to_string());
    let d_discount_ge_low = d_discount_ge_low.add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column(d_discount_ge_low.get_name().to_string(), d_discount_ge_low);

    let mut d_discount_lt_high = lineitem_table["l_discount"].clone() + (RingElement(DISCOUNT_HIGH.wrapping_neg()), &party_id);
    d_discount_lt_high.update_name("d_discount_lt_high".to_string());
    let d_discount_lt_high = d_discount_lt_high.add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column(d_discount_lt_high.get_name().to_string(), d_discount_lt_high);

    let mut d_quantity_lt = lineitem_table["l_quantity"].clone() + (RingElement(QUANTITY_THRESHOLD.wrapping_neg()), &party_id);
    d_quantity_lt.update_name("d_quantity_lt".to_string());
    let d_quantity_lt = d_quantity_lt.add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    lineitem_table.insert_column(d_quantity_lt.get_name().to_string(), d_quantity_lt);

    if party_id == PartyID::ID0 {
        tracing::info!("Preprocessing time: {:?}", prep_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q6_ORQ_BIT preprocessing");


    tracing::info!("Projecting tables");

    let l_col_names = vec!["[d_shipdate_ge_date]", "[d_shipdate_lt_end]", "[d_discount_ge_low]", "[d_discount_lt_high]", "[d_quantity_lt]", "l_discount", "l_extendedprice"];
    let lineitem_table = lineitem_table.project(l_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q6_ORQ_BIT start");
    let tot_start = Instant::now();

    tracing::info!("Filtering tables: lineitem (all predicates are local ltz_bit, no communication)");

    let c1 = lineitem_table["[d_shipdate_ge_date]"].ltz_bit()?;
    let c2 = lineitem_table["[d_shipdate_lt_end]"].ltz_bit()?;
    let c3 = lineitem_table["[d_discount_ge_low]"].ltz_bit()?;
    let c4 = lineitem_table["[d_discount_lt_high]"].ltz_bit()?;
    let c5 = lineitem_table["[d_quantity_lt]"].ltz_bit()?;

    // the whole predicate chain stays in Bit shares: each AND communicates 1 bit per row
    let c = c1.and_bit(&c2, &mut mpc_exec_args)?
                  .and_bit(&c3, &mut mpc_exec_args)?
                  .and_bit(&c4, &mut mpc_exec_args)?
                  .and_bit(&c5, &mut mpc_exec_args)?;

    // AND with the valid column in the Bit domain as well (1 bit per row);
    // Bit share components stay single bits by construction, so no bit_extract is needed
    let new_valid_bit = valid_bit.and_bit(&c, &mut mpc_exec_args)?;

    tracing::info!("lineitem filter completed");

    tracing::info!("compute prefix sum");

    let revenue = lineitem_table["l_extendedprice"].clone() * (&lineitem_table["l_discount"] ,&mut mpc_exec_args);

    let valid_ari = ShareColumn::new(from_bit_to_arithmetic_t_multithreads::<u64, _>(new_valid_bit.get_data(), mpc_exec_args.nets, mpc_exec_args.states)?, ShareType::Arithmetic, "valid_ari".to_string());
    let revenue_after_valid = revenue * (&valid_ari ,&mut mpc_exec_args);

    let revenue = revenue_after_valid.prefix_sum();

    let open_revenue = open(revenue, mpc_exec_args.nets[0])?;


    tracing::info!("Q6_ORQ_BIT execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q6_ORQ_BIT execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q6_ORQ_BIT total (preprocessing + query)");



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

        tracing::info!("Q6_ORQ_BIT: MPC result matches polars result !");
    }


    Ok(())
}
