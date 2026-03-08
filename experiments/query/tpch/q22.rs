/*
 *  select
 *  	cntrycode,
 *  	count(*) as numcust,
 *  	sum(c_acctbal) as totacctbal
 *  from (
 *  	select
 *  		substring(c_phone from 1 for 2) as cntrycode,
 *  		c_acctbal
 *  	from
 *  		customer
 *  	where
 *  		substring(c_phone from 1 for 2) in
 *  		('[I1]','[I2]','[I3]','[I4]','[I5]','[I6]','[I7]')
 *  		and c_acctbal > (
 *  			select
 *  				avg(c_acctbal)
 *  			from
 *  				customer
 *  			where
 *  				c_acctbal > 0.00
 *  				and substring (c_phone from 1 for 2) in
 *  					('[I1]','[I2]','[I3]','[I4]','[I5]','[I6]','[I7]')
 *  		)
 *  		and not exists (
 *  			select
 *  				*
 *  			from
 *  				orders
 *  			where
 *  				o_custkey = c_custkey
 *  		)
 *  	) as custsale
 *  group by
 *  	cntrycode
 *  order by
 *  	cntrycode;
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
use primitives::div::div;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen;
use table::table_operator::{Filter, Groupby, AggFunc, Join, Project};
use table::predicate::Predicate;
use table::share_column::{ShareColumn, ShareType};
use table::column_operator::{ColumnBooleanOperator, PrefixSum, TransformBetweenArithAndBinary};
use table::table_operator::Open;
use table::NetStateArgs;
use polars::prelude::*;


const I1: u64 = 13;
const I2: u64 = 31;
const I3: u64 = 23;
const I4: u64 = 29;
const I5: u64 = 30;
const I6: u64 = 18;
const I7: u64 = 17;


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

    let (mut customer_table, customer_table_polars) = tpch_database_gen::gen_customer_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Customer table generated with {} rows", customer_table.num_rows());

    let (orders_table, orders_table_polars) = tpch_database_gen::gen_orders_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Orders table generated with {} rows", orders_table.num_rows());


    tracing::info!("converting some columns to binary");

    let c_cntrycode_binary = customer_table["c_cntrycode"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    customer_table.insert_column("[c_cntrycode]".to_string(), c_cntrycode_binary);

    let c_acctbal_binary = customer_table["c_acctbal"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    customer_table.insert_column("[c_acctbal]".to_string(), c_acctbal_binary);
    

    tracing::info!("Projecting tables");

    let o_col_names = vec!["o_custkey", "valid"];
    let orders_table = orders_table.project(o_col_names)?;

    let c_col_names = vec!["c_custkey", "c_acctbal", "[c_cntrycode]", "c_cntrycode", "[c_acctbal]", "valid"];
    let mut customer_table = customer_table.project(c_col_names)?;


    tracing::info!("Q22 start");
    let tot_start = Instant::now();

    tracing::info!("substring(c_phone from 1 for 2) in ('[I1]','[I2]','[I3]','[I4]','[I5]','[I6]','[I7]')");

    let pub_elements = vec![I1, I2, I3, I4, I5, I6, I7];
    let res_in = customer_table["[c_cntrycode]"].in_public_binary(&pub_elements, &mut mpc_exec_args)?;
    let _ = customer_table.filter_directed_by_bool(res_in.get_data(), &mut mpc_exec_args)?;
    customer_table.delete_column("[c_cntrycode]");

    let _ = customer_table.filter_public("[c_acctbal]",  Predicate::GreaterThanBinary, &0, &mut mpc_exec_args)?;
    customer_table.delete_column("[c_acctbal]");
    
    /* 
    let sum_valid = customer_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, &net0)?.0;
    tracing::info!("Rows after filter: {}", open_valid);
    */

    tracing::info!("select avg(c_acctbal)");
    let valid_clone = customer_table["valid"].clone();
    customer_table["c_acctbal"] *= (&valid_clone, &mut mpc_exec_args);
    let sum_acctbal = customer_table["c_acctbal"].prefix_sum();
    let sum_valid = customer_table["valid"].prefix_sum();
    let avg_acctbal = div(&sum_acctbal, &sum_valid, 32 as usize, mpc_exec_args.nets[0], mpc_exec_args.states[0])?;

    let avg_acctbal_col_data = vec![avg_acctbal; customer_table.num_rows()];
    let avg_acctbal_col = ShareColumn::new(
        avg_acctbal_col_data,
        ShareType::Arithmetic,
        "avg_acctbal".to_string(),
    );
    customer_table.insert_column("avg_acctbal".to_string(), avg_acctbal_col);


    tracing::info!("c_acctbal > avg_acctbal");

    let _ = customer_table.filter_shared("c_acctbal",  "avg_acctbal", Predicate::GreaterThan,  &mut mpc_exec_args)?;
    customer_table.delete_column("avg_acctbal");

    /* 
    let sum_valid = customer_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, &net0)?.0;
    tracing::info!("Rows after filter: {}", open_valid);
    */


    tracing::info!("not exists o_custkey = c_custkey");

    let _ = customer_table.anti_join(
        "c_custkey",
        "o_custkey",
        &orders_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("group by cntrycode");
    
    let (e,perm,_) = customer_table.group_by(
            vec!["c_cntrycode"],
        &mut mpc_exec_args,
    )?;


    tracing::info!("count(*) as numcust, sum(c_acctbal) as totacctbal");

    customer_table.agg_count(
        "numcust",
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;
   
    customer_table.agg_sum(
        "c_acctbal",
        "totacctbal",
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;


    tracing::info!("Q22 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q22 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q22");
    
    
    
//************* polars verification *************//

    let mut final_table = customer_table;

    let sum_valid = final_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    final_table.head(open_valid as usize);

    let mpc_result = final_table.open(&mut mpc_exec_args)?;

    
    if party_id == PartyID::ID0 {
        tracing::info!("Q22 polars:");

        let orders = orders_table_polars.unwrap();
        let customer = customer_table_polars.unwrap();

        // q1 = customer.filter(substring(c_phone, 1, 2) in country_codes)
        let q1 = customer.clone().lazy()
            .filter(
                col("c_cntrycode").eq(lit(I1))
                .or(col("c_cntrycode").eq(lit(I2)))
                .or(col("c_cntrycode").eq(lit(I3)))
                .or(col("c_cntrycode").eq(lit(I4)))
                .or(col("c_cntrycode").eq(lit(I5)))
                .or(col("c_cntrycode").eq(lit(I6)))
                .or(col("c_cntrycode").eq(lit(I7)))
            )
            .filter(col("c_acctbal").gt(lit(0)))
            .select([col("c_acctbal"), col("c_custkey"), col("c_cntrycode")]);
        
        tracing::info!("q1 rows: {}", q1.clone().collect()?.height());

        // q2 = avg(c_acctbal) where c_acctbal > 0
        let q2 = q1.clone().lazy()
            .with_columns([
                col("c_acctbal").sum().alias("sum_acctbal"),
                col("c_acctbal").count().alias("count_acctbal")
            ])
            .with_column(
                (col("sum_acctbal") / col("count_acctbal")).alias("avg_acctbal")
            )
            .filter(col("c_acctbal").gt(col("avg_acctbal")))
            .select([col("c_cntrycode"), col("c_acctbal"), col("c_custkey")]);        
        
        tracing::info!("q2 rows: {}", q2.clone().collect()?.height());

        let q22_polars = q2.join(
                orders.lazy(),
                [col("c_custkey")],
                [col("o_custkey")],
                JoinArgs::new(JoinType::Anti)
            )
            .group_by([col("c_cntrycode")])
            .agg([
                col("c_cntrycode").count().alias("numcust"),
                col("c_acctbal").sum().alias("totacctbal")
            ])
            .sort(
                ["c_cntrycode"],
                SortMultipleOptions::default().with_maintain_order(true)
            )
            .collect()?;
        

        let mpc_cntrycode = mpc_result["c_cntrycode"].get_data();
        let mpc_numcust = mpc_result["numcust"].get_data();
        let mpc_totacctbal = mpc_result["totacctbal"].get_data();

        let polars_cntrycode = q22_polars.column("c_cntrycode")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_numcust = q22_polars.column("numcust")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_totacctbal = q22_polars.column("totacctbal")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        tracing::info!("rows of polars: {:?}", q22_polars.height());
        // tracing::info!("MPC cntrycode: {:?}", mpc_cntrycode);
        // tracing::info!("Polars cntrycode: {:?}", polars_cntrycode);
        
        assert_eq!(mpc_cntrycode, &polars_cntrycode, "cntrycode mismatch");
        assert_eq!(mpc_numcust, &polars_numcust, "numcust mismatch");
        assert_eq!(mpc_totacctbal, &polars_totacctbal, "totacctbal mismatch");
        tracing::info!("Passed! Q22 result matched with Polars.");
    }
    
    Ok(())
}