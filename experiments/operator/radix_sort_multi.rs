use std::path::PathBuf;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use operator::sort::radix_sort_multithreads;
use random::rep3::Rep3State;
use communication::rep3::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use experiments::net_statistics::{install_tracing, print_communication_stats_operator};
use experiments::gen_rand_column_u64_ring;
use table::share_column::ShareType;

#[derive(Parser)]
struct Args {
    /// The config file path
    #[clap(short = 'c', long, value_name = "CONFIGDIR")]
    config_dir: PathBuf,

    /// The party ID (0, 1, 2)
    #[clap(short = 'p', long, value_name = "PARTYID", default_value = "0")]
    party_id: String,

    /// The number of communication threads
    #[clap(short = 't', long, value_name = "THREADS", default_value = "6")]
    threads: usize,

    /// Shift factor (2^shift rows)
    #[clap(short = 's', long, value_name = "SHIFT", default_value = "20")]
    shift: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    install_tracing();

    let shift = args.shift;
    let rows = 1 << shift as usize;
    let partyid = args.party_id.clone();
    
    let rayon_threads = rayon::current_num_threads();
    tracing::info!("Multi-thread Radix Sort with rows: {}, rayon_threads: {}", rows, rayon_threads);
    tracing::info!("setting up network");

    let mut nets: Vec<TcpNetwork> = Vec::new();
    let mut states: Vec<Rep3State> = Vec::new();

    let file_path0 = PathBuf::from(format!("{}0/config_party{}.toml", args.config_dir.display(), partyid));
    let config: NetworkConfig = toml::from_str(&std::fs::read_to_string(file_path0).context("opening config file")?).context("parsing config file")?;
    let net0 = TcpNetwork::new(config)?;
    let state0 = Rep3State::new(&net0)?;

    for i in 2..(2+args.threads) {
        let file_path = PathBuf::from(format!("{}{}/config_party{}.toml", args.config_dir.display(), i, partyid));
        let config: NetworkConfig = toml::from_str(&std::fs::read_to_string(file_path).context("opening config file")?).context("parsing config file")?;
        let net = TcpNetwork::new(config)?;
        let state = Rep3State::new(&net)?;
        nets.push(net);
        states.push(state);
    }

    let nets_ref = nets.iter().collect::<Vec<&TcpNetwork>>();
    let mut states_ref = states.iter_mut().collect::<Vec<&mut Rep3State>>();

    tracing::info!("Network setup completed");

    tracing::info!("Generating input");
    let input = gen_rand_column_u64_ring(
        rows,
        "test".to_string(), 
        rows as u64, 
        1, 
        ShareType::Arithmetic, 
        &nets_ref,  
        state0.id
    ).0.get_data().to_vec();

    tracing::info!("Starting Multi-thread Radix Sort");
    let start = Instant::now();

    let _sorted = radix_sort_multithreads(input, true, 64, &nets_ref, &mut states_ref)?;

    if state0.id == PartyID::ID0 {
        tracing::info!("INFO Total multi-thread radix sort execution time: {:?}", start.elapsed());
    }
    print_communication_stats_operator(&nets_ref, state0.id, "Multi-thread Radix Sort");
    
    Ok(())
}
