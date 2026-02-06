

/*
 * select
 *   s_name,
 * 	 s_address
 * from
 * 	 supplier, nation
 * where
 * 	 s_suppkey in (
 *     select
 * 	     ps_suppkey
 * 	   from
 *       partsupp
 * 	   where
 *       ps_partkey in (
 *         select
 *           p_partkey
 *         from
 *           part
 *         where
 *           p_name like '[COLOR]%'
 *       )
 *       and ps_availqty > (
 *         select
 *           0.5 * sum(l_quantity)
 *         from
 *           lineitem
 *         where
 *           l_partkey = ps_partkey
 *           and l_suppkey = ps_suppkey
 *           and l_shipdate >= date('[DATE]')
 *           and l_shipdate < date('[DATE]') + interval '1' year
 *     )
 * 	 )
 * 	 and s_nationkey = n_nationkey
 * 	 and n_name = '[NATION]'
 * order by
 * 	 s_name;
 */
 
 
use std::path::PathBuf;
use std::vec;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use protocols::protocols::rep3_ring::Rep3State;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::rep3_ring::id::PartyID;
use protocols::protocols::rep3_ring::arithmetic::open;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::net_statistics::install_tracing;
use experiments::net_statistics::print_communication_stats;
use experiments::tpch_database_gen::{self, get_partsupp_table_size};
use table::column_operator::{ColumnBooleanOperator, PrefixSum};
use table::table_operator::{Filter, Groupby, AggFunc, Join, OrderBy, Project};
use table::predicate::Predicate;
use table::table_operator::Open;
use table::NetStateArgs;
use polars::prelude::*;


const NATION : u64 = 5; // "INDIA"
const DATE: u64 = 60;
const DATE_PLUS_INTERVAL: u64 = 100;
const COLOR: u64 = 15;



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

    let mut mpc_exec_args = NetStateArgs::new(
        &nets,
        &mut state0,
        &mut state1,
        &mut states,
    );

    tracing::info!("Network setup completed");
    

    tracing::info!("Generating tables");

    let (mut lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (mut part_table, part_table_polars) = tpch_database_gen::gen_part_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Part table generated with {} rows", part_table.num_rows());

    let (partsupp_table, partsupp_table_polars) = tpch_database_gen::gen_partsupp_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Partsupp table generated with {} rows", partsupp_table.num_rows());
    
    let (supplier_table, supplier_table_polars) = tpch_database_gen::gen_supplier_table(sf, &mut mpc_exec_args)?;
    tracing::info!("Supplier table generated with {} rows", supplier_table.num_rows());

    let (mut nation_table, nation_table_polars) = tpch_database_gen::gen_nation_table(&mut mpc_exec_args)?;
    tracing::info!("Nation table generated with {} rows", nation_table.num_rows());


    tracing::info!("converting some columns to binary");

    let p_name_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &part_table["p_name"],
        &mut mpc_exec_args,
    )?;
    part_table.insert_column("[p_name]".to_string(), p_name_binary);

    let l_shipdate_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &lineitem_table["l_shipdate"],
        &mut mpc_exec_args,
    )?;
    lineitem_table.insert_column("[l_shipdate]".to_string(), l_shipdate_binary);

    let n_name_binary = tpch_database_gen::convert_binary_from_arithmetic(
        &nation_table["n_name"],
        &mut mpc_exec_args,
    )?;
    nation_table.insert_column("[n_name]".to_string(), n_name_binary);
    

    tracing::info!("Projecting tables");

    /* 
    S.project({"[SuppKey]", "[Name]", "[NationKey]", "[Name]", "[Address]"});
    PS.project({"AvailQty", "[PartKey]", "[SuppKey]"});
    L.project({"[ShipDate]", "[PartKey]", "[SuppKey]", "Quantity"});
    P.project({"[Name]", "[PartKey]"});
    N.project({"[NationKey]", "[Name]"});
    */

    let lineitem_col_names = vec!["[l_shipdate]", "l_partkey", "l_suppkey", "l_quantity", "valid"]; 
    let mut lineitem_table = lineitem_table.project(lineitem_col_names)?;

    let part_col_names = vec!["[p_name]", "p_partkey", "valid"];
    let mut part_table = part_table.project(part_col_names)?;

    let partsupp_col_names = vec!["ps_availqty", "ps_partkey", "ps_suppkey", "valid"];
    let partsupp_table = partsupp_table.project(partsupp_col_names)?;

    let supplier_col_names = vec!["s_suppkey", "s_nationkey", "s_name", "s_address", "valid"];
    let supplier_table = supplier_table.project(supplier_col_names)?;

    let nation_col_names = vec!["n_nationkey", "[n_name]", "valid"];
    let mut nation_table = nation_table.project(nation_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("Q20 start");
    let tot_start = Instant::now();

    tracing::info!("sub query: p_name like '[COLOR]%...'");

    tracing::info!("p_name like '[COLOR]%'");
    let _ = part_table.filter_public(
        "[p_name]",
        Predicate::EqualBinary,
        &COLOR,
        &mut mpc_exec_args,
    )?;
    part_table.delete_column("[p_name]");

    tracing::info!("ps_partkey in p_partkey");
    let part_partsupp_table = part_table.inner_join(
        "p_partkey",
        "ps_partkey",
        &partsupp_table,
        &mut mpc_exec_args,
    )?;


    tracing::info!("sub query: ps_availqty > ()");

    tracing::info!("l_shipdate >= date '[DATE]' and l_shipdate < date '[DATE]' + interval '1' year");

    let c1 = lineitem_table["[l_shipdate]"].ge_public_binary(
        &DATE,
        &mut mpc_exec_args,
    )?;
    let c2 = lineitem_table["[l_shipdate]"].lt_public_binary(
        &DATE_PLUS_INTERVAL,
        &mut mpc_exec_args,
    )?;

    let c = c1.and(&c2, &mut mpc_exec_args)?;

    lineitem_table.filter_directed_by_bool(c.get_data(), &mut mpc_exec_args)?;

    lineitem_table.delete_column("[l_shipdate]");


    tracing::info!("compute 0.5 * sum(l_quantity)");

    let group_key_names = vec!["l_partkey","l_suppkey"];
    let (e,perm,_)  = lineitem_table.group_by(group_key_names, &mut mpc_exec_args)?;

    let _ = lineitem_table.agg_sum("l_quantity", "sum_quantity", &e, &perm, &mut mpc_exec_args)?;

    lineitem_table.head(get_partsupp_table_size(sf) as usize);

    lineitem_table["sum_quantity"] /= (&RingElement(2u64), &mut mpc_exec_args);

    tracing::info!("l_partkey = ps_partkey and l_suppkey = ps_suppkey");

    let mut part_partsupp_lineitem_table = part_partsupp_table.inner_join_multi_keys(
        vec!["ps_partkey", "ps_suppkey"],
        vec!["l_partkey", "l_suppkey"],
        &lineitem_table,
        &mut mpc_exec_args,
    )?;

    tracing::info!("ps_availqty > 0.5 * sum(l_quantity)");
    let filter_values = part_partsupp_lineitem_table["sum_quantity"].get_data().to_vec();
    let _ = part_partsupp_lineitem_table.filter_shared_with_any_column("ps_availqty", &filter_values, Predicate::GreaterThanBinary, &mut mpc_exec_args);

    tracing::info!("select ps_suppkey from partsupp"); // ps_suppkey has been replaced by l_suppkey because join keep right table key
    let col_names = vec!["l_suppkey", "valid"];
    let sub_table1 = part_partsupp_lineitem_table.project(col_names)?;

    tracing::info!("sub query 1 completed");


    tracing::info!("sub query2: s_nationkey = n_nationkey ...");

    tracing::info!("n_name = '[NATION]'");
    let _ = nation_table.filter_public(
        "[n_name]",
        Predicate::EqualBinary,
        &NATION,
        &mut mpc_exec_args,
    )?;
    nation_table.delete_column("[n_name]");

    tracing::info!("s_nationkey = n_nationkey");
    let mut sub_table2 = nation_table.inner_join(
        "n_nationkey",
        "s_nationkey",
        &supplier_table,
        &mut mpc_exec_args,
    )?;
    

    tracing::info!("final: s_suppkey in ps_suppkey"); // ps_suppkey has been replaced by l_suppkey

    let _ = sub_table2.semi_join(
        "s_suppkey",
        "l_suppkey",
        &sub_table1,
        &mut mpc_exec_args,
    )?;
    let mut final_table = sub_table2;


    tracing::info!("order by s_name");
    let _ = final_table.order_by(
        "s_name",
        true,
        &mut mpc_exec_args,
    )?;


    tracing::info!("Q20 execution completed");

    if mpc_exec_args.state0.id == PartyID::ID0 {
        tracing::info!("Total Q20 execution time: {:?}", tot_start.elapsed());
    }
    print_communication_stats(&mpc_exec_args, "Q20");
    




//************* polars verification *************//

    let mut result_table = final_table.project(vec!["s_name", "s_address", "valid"])?;
    // sort valid to top
    let _ = result_table.order_by("valid", false, &mut mpc_exec_args)?;
    
    let sum_valid = result_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, &net0)?.0;
    tracing::info!("open valid: {}", open_valid);
    result_table.head(open_valid as usize);

    let mpc_result = result_table.open(&mut mpc_exec_args)?;

    if state0.id == PartyID::ID0 {
        tracing::info!("Q20 polars:");
        let lineitem = lineitem_table_polars.unwrap();
        let part = part_table_polars.unwrap();
        let partsupp = partsupp_table_polars.unwrap();
        let supplier = supplier_table_polars.unwrap();
        let nation = nation_table_polars.unwrap();

        // Q1
        let q1 = lineitem.lazy()
            .filter(
                col("l_shipdate").gt_eq(lit(DATE))
                .and(col("l_shipdate").lt(lit(DATE_PLUS_INTERVAL)))
            )
            .group_by([col("l_partkey"), col("l_suppkey")])
            .agg([
                (col("l_quantity").sum() / lit(2)).cast(DataType::Int64).alias("sum_quantity")
            ]);
        tracing::info!("q1 rows: {}", q1.clone().collect()?.height());

        // Q2
        let q2 = nation.lazy()
            .filter(col("n_name").eq(lit(NATION)));
        tracing::info!("q2 rows: {}", q2.clone().collect()?.height());

        // Q3
        let q3 = supplier.lazy()
            .join(
                q2,
                [col("s_nationkey")],
                [col("n_nationkey")],
                JoinArgs::new(JoinType::Inner)
            );
        tracing::info!("q3 rows: {}", q3.clone().collect()?.height());

        let test_part = part.clone().lazy()
            .filter(col("p_name").eq(lit(COLOR)));
        tracing::info!("test_part rows: {}", test_part.clone().collect()?.height());

        // Main Query
        let final_df = part.lazy()
            .filter(col("p_name").eq(lit(COLOR)))
            .select([col("p_partkey")])
            .unique(None, UniqueKeepStrategy::First)
            .join(
                partsupp.lazy(),
                [col("p_partkey")],
                [col("ps_partkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .join(
                q1,
                [col("ps_suppkey"), col("p_partkey")],
                [col("l_suppkey"), col("l_partkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .filter(col("ps_availqty").gt(col("sum_quantity")))
            .select([col("ps_suppkey")])
            .unique(None, UniqueKeepStrategy::First)
            .join(
                q3,
                [col("ps_suppkey")],
                [col("s_suppkey")],
                JoinArgs::new(JoinType::Inner)
            )
            .select([col("s_name"), col("s_address")])
            .sort(["s_name"], SortMultipleOptions::default());
        
        let df_result = final_df.collect()?;
        tracing::info!("Polars result: {:?}", df_result.height());

        let mpc_s_name = mpc_result["s_name"].get_data();
        let mpc_s_address = mpc_result["s_address"].get_data();

        let polars_s_name = df_result.column("s_name")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();
        let polars_s_address = df_result.column("s_address")?.cast(&polars::datatypes::DataType::UInt64)?.u64()?.into_no_null_iter().collect::<Vec<_>>();

        assert_eq!(mpc_s_name, polars_s_name, "s_name mismatch");
        assert_eq!(mpc_s_address, polars_s_address, "s_address mismatch");

        tracing::info!("Verification passed!");
    }


    Ok(())
}
//* safe cut and semi-join
