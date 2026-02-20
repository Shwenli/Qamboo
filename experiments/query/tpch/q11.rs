
/*
 *  select
 *      ps_partkey,
 *      sum(ps_supplycost * ps_availqty) as value
 *  from
 *      partsupp,
 *      supplier,
 *      nation
 *  where
 *      ps_suppkey = s_suppkey
 *      and s_nationkey = n_nationkey
 *      and n_name = '[NATION]'
 *  group by
 *      ps_partkey
 *  having
 *      sum(ps_supplycost * ps_availqty) > (
 *          select
 *              sum(ps_supplycost * ps_availqty) * [FRACTION]
 *          from
 *              partsupp,
 *              supplier,
 *              nation
 *          where
 *              ps_suppkey = s_suppkey
 *              and s_nationkey = n_nationkey
 *              and n_name = '[NATION]'
 *      )
 *  order by
 *      value desc;
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
use protocols::protocols::rep3_ring::arithmetic::{open};
use primitives::utils::prefix_sum_sequential;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen::{self};
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::column_operator::PrefixSum;
use table::share_column::{ShareColumn,ShareType};
use table::NetStateArgs;
use table::predicate::Predicate;
use table::table_operator::Open;
use polars::prelude::*;


const NATION : u64 = 1; // "INDIA"
const FRACTION: f64 = 0.01;
const DIVIDE: u64 = (FRACTION * 1000000.0) as u64;



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

    let (partsupp_table, partsupp_table_polars) = tpch_database_gen::gen_partsupp_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Partsupp table generated with {} rows", partsupp_table.num_rows());
    
    let (supplier_table, supplier_table_polars) = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());

    let (nation_table, nation_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation_table.num_rows());


    tracing::info!("Projecting tables");
    /* 
    PartSupp.project({"[SuppKey]", "[PartKey]", "[SupplyCost]", "SupplyCost", "AvailQty"});
    Supplier.project({"[SuppKey]", "[NationKey]"});
    Nation.project({"[NationKey]", "[Name]"});
    */

    let partsupp_col_names = vec!["ps_partkey", "ps_suppkey", "ps_supplycost", "ps_availqty", "valid"];
    let partsupp_table = partsupp_table.project(partsupp_col_names)?;

    let supplier_col_names = vec!["s_suppkey", "s_nationkey", "valid"];
    let supplier_table = supplier_table.project(supplier_col_names)?;

    let nation_col_names = vec!["n_nationkey", "n_name", "valid"];
    let mut nation_table = nation_table.project(nation_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q11 start");
    let tot_start = Instant::now();

    tracing::info!("The main-query: ");

    tracing::info!("n_name = '[NATION]' ");

    let _ = nation_table.filter_public("n_name", Predicate::Equal, &NATION, &mut mpc_exec_args);


    tracing::info!("s_nationkey = n_nationkey");

    let k_l_name = "n_nationkey";
    let k_r_name = "s_nationkey";

    let nation_supplier_table = nation_table.inner_join(
        k_l_name,
        k_r_name,
        &supplier_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("s_suppkey = ps_suppkey");

    let k_l_name = "s_suppkey";
    let k_r_name = "ps_suppkey";

    let mut main_table = nation_supplier_table.inner_join(
        k_l_name,
        k_r_name,
        &partsupp_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("compute value = ps_supplycost * ps_availqty");

    let mut value_column = main_table["ps_supplycost"].clone() * (&main_table["ps_availqty"], &mut mpc_exec_args);
    value_column.update_name("value".to_string());
    main_table.insert_column("value".to_string(), value_column);


    tracing::info!("sub query");

    tracing::info!("compute FRACTION * sub_query_sum");

    let sub_query_table = main_table.clone();

    let sub_query_to_sum_valid = sub_query_table["valid"].clone() * (&sub_query_table["value"], &mut mpc_exec_args);

    let sub_query_sum = prefix_sum_sequential(sub_query_to_sum_valid.get_data())?;
    let sub_query_sum = vec![sub_query_sum[sub_query_sum.len()-1].clone();1];
    let sub_query_sum_cloumn = ShareColumn::new(sub_query_sum, ShareType::Arithmetic, "sub_sumvalue".to_string());

    let sub_query_sum_fraction_column = sub_query_sum_cloumn / (&RingElement(DIVIDE), &mut mpc_exec_args);
    let sub_query_sum_fraction = sub_query_sum_fraction_column.get_data()[0];


    tracing::info!("group by ps_partkey");

    let group_by_col_names = vec!["ps_partkey"];
    let (e,perm,_) = main_table.group_by(
        group_by_col_names,
        &mut mpc_exec_args,
    )?;


    tracing::info!("sum(ps_supplycost * ps_availqty) as value");

    let new_agg_name = "sumvalue";
    let to_agg_name = "value";

    let _ = main_table.agg_sum(
        to_agg_name,
        new_agg_name,
        &e,
        &perm,
        &mut mpc_exec_args,
    )?;

    
    tracing::info!("having sumvalue > sub_query_sum * FRACTION");

    let sub_query_sum_fraction_vec = vec![sub_query_sum_fraction; main_table.num_rows()];

    let _ = main_table.filter_shared_with_any_column("sumvalue", &sub_query_sum_fraction_vec, Predicate::GreaterThan, &mut mpc_exec_args)?;

    
    tracing::info!("order by sumvalue desc");

    let _ = main_table.order_by(
        "sumvalue",
        false,
        &mut mpc_exec_args,
    )?;


    tracing::info!("Q11 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q11 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q11");

    


//************* polars verification *************//

    let mut result_table = main_table.project(vec!["ps_partkey", "sumvalue", "valid"])?;
    // sort valid to top
    let _ = result_table.order_by("valid", false, &mut mpc_exec_args)?;
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q11 polars:");
        let partsupp = partsupp_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();
        let nation = nation_table_polars.unwrap();

        let joined = nation.lazy()
            .filter(col("n_name").eq(lit(NATION)))
            .join(
                supplier.lazy(),
                [col("n_nationkey")],
                [col("s_nationkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .join(
                partsupp.lazy(),
                [col("s_suppkey")],
                [col("ps_suppkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .with_column(
                (col("ps_supplycost") * col("ps_availqty")).alias("value")
            );

        // Calculate total sum for having clause
        let total_sum_df = joined.clone()
            .select([
                col("value").sum().alias("total_value")
            ])
            .collect()?;
        
        let total_value = total_sum_df.column("total_value")?.u64()?.get(0).unwrap();
        let threshold = total_value / (DIVIDE as u64); // Matches MPC division

        //tracing::info!("Polars total value: {}, threshold: {}", total_value, threshold);

        let q11_result = joined
            .group_by([col("ps_partkey")])
            .agg([
                col("value").sum().alias("sumvalue")
            ])
            .filter(
                col("sumvalue").gt(lit(threshold))
            )
            .sort(["sumvalue"], SortMultipleOptions::default().with_order_descending_multi([true]))
            .collect()?;

        let mpc_ps_partkey = mpc_result["ps_partkey"].get_data();
        let mpc_sumvalue = mpc_result["sumvalue"].get_data();

        let polars_ps_partkey = q11_result.column("ps_partkey")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_sumvalue = q11_result.column("sumvalue")?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        assert_eq!(mpc_ps_partkey, &polars_ps_partkey, "ps_partkey does not match");
        assert_eq!(mpc_sumvalue, &polars_sumvalue, "sumvalue does not match");
        

        tracing::info!("Q11: MPC result matching checked!");
    }

    Ok(())
}