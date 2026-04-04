
use std::path::PathBuf;
use std::vec;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use protocols::rep3_ring::arithmetic::{promote_to_trivial_share,open_vec};
use algebra::ring::ring_impl::RingElement;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::net_statistics::install_tracing;
use operator::join::inner_join_table_multi_keys_multithreads;


#[derive(Parser)]
struct Args {
    /// The config file path
    #[clap(short = 'c', long, value_name = "CONFIGDIR")]
    config_dir: PathBuf,
    /// The party ID (0, 1, 2)
    #[clap(short = 'p', long, value_name = "PARTYID", default_value = "0")]
    party_id: String,
}


fn main() -> Result<()> {
    let args = Args::parse();
    install_tracing();

    tracing::info!("setting up network");
    let partyid=args.party_id.clone();

    let mut nets: Vec<TcpNetwork> = Vec::new();
    let mut states: Vec<Rep3State> = Vec::new();

    for i in 3..9{
        let file_path = PathBuf::from(format!("{}{}/config_party{}.toml", args.config_dir.display(), i, partyid));
        let config: NetworkConfig =toml::from_str(&std::fs::read_to_string(file_path).context("opening config file")?).context("parsing config file")?;
        let net = TcpNetwork::new(config)?;
        let state = Rep3State::new(&net)?;
        nets.push(net);
        states.push(state);
    }
    let nets = nets.iter().collect::<Vec<&TcpNetwork>>();
    let mut states = states.iter_mut().collect::<Vec<&mut Rep3State>>();
    tracing::info!("Network setup completed");

    /*



    let sf = 0.0005; // scale factor for testing

    let (lineitem_table, lineitem_table_polars) = tpch_database_gen::gen_lineitem_table(sf, &net0, &net1, &mut state0 , &mut state1)?;
    tracing::info!("Lineitem table generated with {} rows", lineitem_table.num_rows());

    let (partsupp_table, partsupp_table_polars) = tpch_database_gen::gen_partsupp_table(sf, &net0, &net1, &mut state0)?;
    tracing::info!("Partsupp table generated with {} rows", partsupp_table.num_rows());


    tracing::info!("Projecting tables");

    let lineitem_col_names = vec!["l_suppkey", "l_partkey", "l_orderkey", "l_extendedprice", "l_discount", "l_quantity", "valid"];
    let mut lineitem_table = lineitem_table.project(lineitem_col_names)?;

    let partsupp_col_names = vec!["ps_partkey", "ps_suppkey", "ps_supplycost", "valid"];
    let partsupp_table = partsupp_table.project(partsupp_col_names)?;

    tracing::info!("Projection completed");


    tracing::info!("start");
    let tot_start = Instant::now();

    let open_lineitem_table = lineitem_table.open(&net0)?;
    tracing::info!("Opened lineitem table {:?}", open_lineitem_table.print_first_rows(10, &mut state0));

    tracing::info!("l_partkey = ps_partkey and l_suppkey = ps_suppkey");

    let mut final_table = partsupp_table.inner_join_multi_keys(
        vec!["ps_partkey", "ps_suppkey"],
        vec!["l_partkey", "l_suppkey"],
        &lineitem_table,
        &nets,
        &mut state0,
        &mut state1,
        &mut states,
    )?;
    
    
    tracing::info!("Total execution time: {:?}", tot_start.elapsed());
    */

    let id = states[0].id;

    let kl1: Vec<u64> = vec![3,5,9];
    let kl1_share = kl1.iter().map(|x| promote_to_trivial_share(id, RingElement(*x))).collect::<Vec<_>>();
    let kr1: Vec<u64> = vec![3,7,9,9];
    let kr1_share = kr1.iter().map(|x| promote_to_trivial_share(id, RingElement(*x))).collect::<Vec<_>>();
    let val_l: Vec<u64> = vec![1,2,3];
    let val_l_share = val_l.iter().map(|x| promote_to_trivial_share(id, RingElement(*x))).collect::<Vec<_>>();
    let valid_l: Vec<u64> = vec![1,1,1];
    let valid_l_share = valid_l.iter().map(|x| promote_to_trivial_share(id, RingElement(*x))).collect::<Vec<_>>();
    let kl2: Vec<u64> = vec![6,7,8];
    let kl2_share = kl2.iter().map(|x| promote_to_trivial_share(id, RingElement(*x))).collect::<Vec<_>>();
    let kr2: Vec<u64> = vec![8,3,7,8];
    let kr2_share = kr2.iter().map(|x| promote_to_trivial_share(id, RingElement(*x))).collect::<Vec<_>>();
    let val_r: Vec<u64> = vec![1,2,3,4];
    let val_r_share = val_r.iter().map(|x| promote_to_trivial_share(id, RingElement(*x))).collect::<Vec<_>>();
    let valid_r: Vec<u64> = vec![1,1,1,1];
    let valid_r_share = valid_r.iter().map(|x| promote_to_trivial_share(id, RingElement(*x))).collect::<Vec<_>>();

    let result = inner_join_table_multi_keys_multithreads(vec![kl2_share,kl1_share], vec![kr2_share, kr1_share], vec![val_l_share.as_slice()], vec![val_r_share.as_slice()], valid_l_share, valid_r_share, 64, &nets, &mut states)?;

    for i in 0..result.len() {
        let open_result = open_vec(&result[i], nets[0])?;
        tracing::info!("result: {:?}", open_result);
    }







    // sort valid to top
    //let _ = final_table.order_by("valid", false, &nets, &mut state0, &mut state1, &mut states);
    /*
    let mut result_table = final_table.project(vec![
        "l_partkey",
        "l_suppkey",
        //"sum_profit", 
        "valid"
    ])?;
    */
    /*

    let sum_valid = final_table["valid"].prefix_sum();
    let open_valid = open(sum_valid, &net0)?.0;
    tracing::info!("open valid: {}", open_valid);

    let mpc_result = final_table.open(&net0)?;
    tracing::info!("mpc result: {:?}", mpc_result.print_first_rows(10, &mut state0));


    if state0.id == PartyID::ID0 {
        tracing::info!("Q9 Polars Validation:");

        let lineitem = lineitem_table_polars.unwrap();
        let partsupp = partsupp_table_polars.unwrap();
        
        // Polars implementation:
        let result = partsupp.lazy()
            .join(
                lineitem.lazy(),
                [col("ps_partkey"), col("ps_suppkey")],
                [col("l_partkey"), col("l_suppkey")],
                JoinArgs::new(JoinType::Inner),
            ).collect()
            .unwrap();
        
        tracing::info!("Polars rows: {}", result.height());
        eprintln!("Polars result:\n{}", result);

    }
    */
    
    Ok(())
}