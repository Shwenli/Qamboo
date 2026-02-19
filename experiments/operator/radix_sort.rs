use std::path::PathBuf;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use operator::sort::{radix_sort,radix_sort_multithreads};
use random::rep3::Rep3State;
use communication::rep3::id::PartyID;
use protocols::protocols::rep3_ring::arithmetic::open_vec;
use net::fast_tcp::{FastTcpNetwork, NetworkConfig};
use experiments::net_statistics::{install_tracing, print_communication_stats_operator};
use experiments::gen_rand_column_u64_ring;
use table::share_column::{ShareType};




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

    /// Shiften factor
    #[clap(short = 's', long, value_name = "SHIFT", default_value = "20")]
    shift: u64,
}


fn main() -> Result<()> {
    let args = Args::parse();
    install_tracing();

    let shift = args.shift;// shift factor for testing
    let rows = 1<<shift as usize;
    let partyid=args.party_id.clone();

    tracing::info!("Radix Sort with rows: {}", rows);
    
    tracing::info!("setting up network");
    
    let mut nets: Vec<FastTcpNetwork> = Vec::new();
    let mut states: Vec<Rep3State> = Vec::new();

    let file_path0 = PathBuf::from(format!("{}0/config_party{}.toml", args.config_dir.display(), partyid));
    let config: NetworkConfig = toml::from_str(&std::fs::read_to_string(file_path0).context("opening config file")?).context("parsing config file")?;
    let net0 = FastTcpNetwork::new(config)?;
    let mut state0 = Rep3State::new(&net0)?;
    

    for i in 2..(2+args.threads){
        let file_path = PathBuf::from(format!("{}{}/config_party{}.toml", args.config_dir.display(), i, partyid));
        let config: NetworkConfig = toml::from_str(&std::fs::read_to_string(file_path).context("opening config file")?).context("parsing config file")?;
        let net = FastTcpNetwork::new(config)?;
        let state = Rep3State::new(&net)?;
        nets.push(net);
        states.push(state);
    }

    let nets = nets.iter().collect::<Vec<&FastTcpNetwork>>();
    let mut states = states.iter_mut().collect::<Vec<&mut Rep3State>>();


    tracing::info!("Network setup completed");


    tracing::info!("Generating input");
    let input = gen_rand_column_u64_ring(
        rows,
        "test".to_string(), 
        rows as u64, 
        1, 
        ShareType::Arithmetic, 
        &nets,  
        state0.id).0.get_data().to_vec();
    
    let input1 = input.clone();


    tracing::info!("Starting Multithreads Radix Sort");
    let multithreads_start = Instant::now();

    let sorted = radix_sort_multithreads(input, true, 64, &nets, &mut states)?;

    if state0.id == PartyID::ID0 {
        tracing::info!("Total Multithreads Radix Sort execution time: {:?}", multithreads_start.elapsed());
    }
    print_communication_stats_operator(&nets, state0.id, "Multithreads Radix Sort");


    tracing::info!("Starting Single Thread Radix Sort");
    let singlethread_start = Instant::now();

    let sorted1 = radix_sort(input1, true, 64, &net0, &mut state0)?;

    if state0.id == PartyID::ID0 {
        tracing::info!("Single Thread Radix Sort execution time: {:?}", singlethread_start.elapsed());
    }
    print_communication_stats_operator(&nets, state0.id, "Single Thread Radix Sort");
    
    let sorted_open = open_vec(&sorted, &net0)?;
    let sorted1_open = open_vec(&sorted1, &net0)?;
    
    assert_eq!(sorted_open, sorted1_open);
    
    Ok(())
}