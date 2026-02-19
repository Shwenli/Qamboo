/* 
 *  select
 *      p_brand,
 *      p_type,
 *      p_size,
 *   	count(distinct ps_suppkey) as supplier_cnt
 *  from
 *  	partsupp,
 *  	part
 *  where
 *  	p_partkey = ps_partkey
 *  	and p_brand <> '[BRAND]'
 *  	and p_type not like '[TYPE]%'
 *  	and p_size in ([SIZE1], [SIZE2], [SIZE3], [SIZE4], [SIZE5], [SIZE6], [SIZE7], [SIZE8])
 *  	and ps_suppkey not in (
 *  		select
 *  			s_suppkey
 *  		from
 *  			supplier
 *  		where
 *  			s_comment like '%Customer%Complaints%'
 *  	)
 *  	group by
 *  		p_brand,
 *  		p_type,
 *  		p_size
 *  	order by
 *  		supplier_cnt desc,
 *  		p_brand,
 *  		p_type,
 *  		p_size;
 */

use std::path::PathBuf;
use std::vec;
use clap::Parser;
use std::time::Instant;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use net::fast_tcp::{FastTcpNetwork, NetworkConfig};
use protocols::protocols::rep3_ring::arithmetic::{open};
//use primitives::utils::prefix_sum_sequential;
use experiments::tpch_database_gen;
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::column_operator::TransformBetweenArithAndBinary;
use table::column_operator::Distinct;
use table::predicate::Predicate;
use table::column_operator::PrefixSum;
use table::table_operator::Open;
use table::NetStateArgs;
use polars::prelude::*;


const BRAND: u64 = 10;
const TYPE: u64 = 10;
const COMMENT: u64 = 0;
const SIZE1: u64 = 1;
const SIZE2: u64 = 2;
const SIZE3: u64 = 3;
const SIZE4: u64 = 4;
const SIZE5: u64 = 5;
const SIZE6: u64 = 6;
const SIZE7: u64 = 7;
const SIZE8: u64 = 8;


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
    let (partsupp_table, partsupp_table_polars) = tpch_database_gen::gen_partsupp_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Partsupp table generated with {} rows", partsupp_table.num_rows());

    let (mut part_table, part_table_polars) = tpch_database_gen::gen_part_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Part table generated with {} rows", part_table.num_rows());

    let (mut supplier_table, supplier_table_polars) = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());


    tracing::info!("converting some columns to binary");

    let p_brand_binary = part_table["p_brand"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    part_table.insert_column("[p_brand]".to_string(), p_brand_binary);

    let p_type_binary = part_table["p_type"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    part_table.insert_column("[p_type]".to_string(), p_type_binary);

    let p_size_binary = part_table["p_size"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    part_table.insert_column("[p_size]".to_string(), p_size_binary);

    let s_comment_binary = supplier_table["s_comment"].add_new_col_from_arithmetic_to_binary(&mut mpc_exec_args)?;
    supplier_table.insert_column("[s_comment]".to_string(), s_comment_binary);


    tracing::info!("Projecting tables");
    //Part.project({"[PartKey]", "[Brand]", "[Type]", "[Size]"});
    //PartSupp.project({"[SuppKey]", "[PartKey]"});
    //Supplier.project({"[SuppKey]", "[Comment]"});

    let p_col_names = vec!["p_partkey", "[p_brand]", "[p_type]", "[p_size]", "p_brand", "p_type", "p_size", "valid"];
    let mut part_table = part_table.project(p_col_names)?;

    let ps_col_names = vec!["ps_suppkey", "ps_partkey", "valid"];
    let mut partsupp_table = partsupp_table.project(ps_col_names)?;

    let s_col_names = vec!["s_suppkey", "[s_comment]", "valid"];
    let mut supplier_table = supplier_table.project(s_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q16 start");
    let tot_start = Instant::now();

    tracing::info!("p_brand <> '[BRAND]' and p_type not like '[TYPE]%'");

    let _ = part_table.filter_public(
        "[p_brand]",
        Predicate::NotEqualBinary,
        &BRAND,
        &mut mpc_exec_args,
    )?;

    let _ = part_table.filter_public(
        "[p_type]",
        Predicate::NotEqualBinary,
        &TYPE,
        &mut mpc_exec_args,
    )?;

    part_table.delete_column("[p_brand]");
    part_table.delete_column("[p_type]");


    tracing::info!("p_size in ([SIZE1], [SIZE2], [SIZE3], [SIZE4], [SIZE5], [SIZE6], [SIZE7], [SIZE8])");

    let size_values = vec![SIZE1, SIZE2, SIZE3, SIZE4, SIZE5, SIZE6, SIZE7, SIZE8];

    let _ = part_table.filter_in(
        "[p_size]",
        &size_values,
        Predicate::EqualBinary,
        &mut mpc_exec_args,
    )?;
    part_table.delete_column("[p_size]");


    tracing::info!("ps_suppkey not in (select s_suppkey from supplier where s_comment like '%Customer%Complaints%')");

    let _ = supplier_table.filter_public(
        "[s_comment]",
        Predicate::EqualBinary,
        &COMMENT,
        &mut mpc_exec_args,
    )?;
    supplier_table.delete_column("[s_comment]");

    let _ = partsupp_table.anti_join(
        "ps_suppkey",
        "s_suppkey",
        &supplier_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("p_partkey = ps_partkey");

    let mut part_partsupp_table = part_table.inner_join(
        "p_partkey",
        "ps_partkey",
        &partsupp_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("group by p_brand, p_type, p_size");

    let group_by_cols = vec!["p_brand", "p_type", "p_size"];

    let (e, perm,_, old_valid) = part_partsupp_table.group_by_retain_valid(
        group_by_cols,
        &mut mpc_exec_args,
    )?;


    tracing::info!("count(distinct ps_suppkey) as supplier_cnt");
    //let open_old_valid = prefix_sum_sequential(&old_valid)?;
    //tracing::info!("old valid cnt: {:?}", open(open_old_valid[open_old_valid.len() - 1], mpc_exec_args.nets[0]));

    let agg_valid = part_partsupp_table["ps_suppkey"].distinct(&old_valid, &e, &mut mpc_exec_args)?;

    //let result_distinct = prefix_sum_sequential(&agg_valid)?;
    //tracing::info!("distinct supplier cnt: {:?}", open(result_distinct[result_distinct.len() - 1], &net0));

    let _ = part_partsupp_table.agg_count_by_valid("supplier_cnt",&agg_valid, &perm, &mut mpc_exec_args)?;


    tracing::info!("supplier_cnt desc, order by p_brand, p_type, p_size");

    let _ = part_partsupp_table.order_by("supplier_cnt", false, &mut mpc_exec_args)?;


    tracing::info!("Q16 execution completed");

    if party_id == PartyID::ID0 {
        tracing::info!("Total Q16 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q16");





//************* polars verification *************//

    let mut result_table = part_partsupp_table.project(vec!["p_brand", "p_type", "p_size", "supplier_cnt", "valid"])?;

    // Sort valid to top
    let _ = result_table.order_by("valid", false, &mut mpc_exec_args)?;
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, mpc_exec_args.nets[0])?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if party_id == PartyID::ID0 {
        tracing::info!("Q16 polars:");
        let partsupp = partsupp_table_polars.unwrap();
        let part = part_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();

        // ps_suppkey not in (select s_suppkey from supplier where s_comment like '%Customer%Complaints%')
        let filtered_suppliers = supplier.lazy()
            .filter(col("s_comment").eq(lit(COMMENT))) 
            .select([col("s_suppkey")]);

        let filtered_partsupp = partsupp.lazy()
            .join(
                filtered_suppliers,
                [col("ps_suppkey")],
                [col("s_suppkey")],
                JoinArgs::new(JoinType::Anti)
            );

        let q16_result = part.lazy()
            .filter(
                col("p_brand").neq(lit(BRAND))
                .and(col("p_type").neq(lit(TYPE)))
                .and(
                    col("p_size").eq(lit(SIZE1))
                    .or(col("p_size").eq(lit(SIZE2)))
                    .or(col("p_size").eq(lit(SIZE3)))
                    .or(col("p_size").eq(lit(SIZE4)))
                    .or(col("p_size").eq(lit(SIZE5)))
                    .or(col("p_size").eq(lit(SIZE6)))
                    .or(col("p_size").eq(lit(SIZE7)))
                    .or(col("p_size").eq(lit(SIZE8)))
                )
            )
            .join(
                filtered_partsupp,
                [col("p_partkey")],
                [col("ps_partkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .group_by([col("p_brand"), col("p_type"), col("p_size")])
            .agg([
                col("ps_suppkey").n_unique().alias("supplier_cnt")
            ])
            .sort(
                ["supplier_cnt", "p_brand", "p_type", "p_size"],
                SortMultipleOptions::default().with_order_descending_multi([true, false, false, false])
            )
            .collect()?;

        let mpc_brand = mpc_result["p_brand"].get_data();
        let mpc_type = mpc_result["p_type"].get_data();
        let mpc_size = mpc_result["p_size"].get_data();
        let mpc_cnt = mpc_result["supplier_cnt"].get_data();
        

        let polars_brand = q16_result.column("p_brand")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_type = q16_result.column("p_type")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_size = q16_result.column("p_size")?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_cnt = q16_result.column("supplier_cnt")?.cast(&DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        tracing::info!("rows of polars: {:?}", polars_brand.len());

        assert_eq!(mpc_brand, polars_brand);
        assert_eq!(mpc_type, polars_type);
        assert_eq!(mpc_size, polars_size);
        assert_eq!(mpc_cnt, polars_cnt);

        // Check top 10
        //let len = std::cmp::min(10, polars_cnt.len());
        //eprintln!("Multiparty computation result: ");
        //eprintln!("mpc_brand: {:?}", &mpc_brand[0..len]);
        //eprintln!("mpc_supplier_cnt: {:?}", &mpc_cnt[0..len]);
        //eprintln!("Polars result: ");
        //eprintln!("polars_brand: {:?}", &polars_brand[0..len]);
        //eprintln!("polars_supplier_cnt: {:?}", &polars_cnt[0..len]);

        tracing::info!("Q16: MPC result matching checked!");
    }


    
    Ok(())
}
// 目前ps_suppkey是unique的，所以结果不变
