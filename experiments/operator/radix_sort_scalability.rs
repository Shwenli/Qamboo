use std::path::PathBuf;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use operator::sort::radix_sort_multithreads;
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

    /// Test input number, 2^19(n=1), 2^20(n=2), ..., 2^25(n=6)
    #[clap(short = 'n', long, value_name = "NUMBER", default_value = "20")]
    number: u64,
}


fn main() -> Result<()> {
    let args = Args::parse();
    install_tracing();

    let partyid=args.party_id.clone();
    let default_states = rayon::current_num_threads() / 2;
    
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

    if states.len() <= default_states {
        let diff = default_states - states.len();
        for _i in 0..diff{
            let state = states[0].fork(0)?;
            states.push(state);
        }
    }
    
    let nets = nets.iter().collect::<Vec<&TcpNetwork>>();
    let mut states = states.iter_mut().collect::<Vec<&mut Rep3State>>();
    let party_id = states[0].id;

    tracing::info!("Network setup completed");


    for i in 0..args.number {
        let rows = 1 << (21 + i);
        let input = gen_rand_column_u64_ring(
            rows,
            "test".to_string(), 
            rows as u64, 
            1, 
            ShareType::Arithmetic, 
            &nets,  
            party_id
        ).0.get_data().to_vec();
        
        let input_clone = input.clone();

        let start = Instant::now();
        let _sorted_64bit = radix_sort_multithreads(input, true, 64, &nets, &mut states)?;
        let duration = start.elapsed();

        if party_id == PartyID::ID0 {
            tracing::info!("Radix Sort with 64-bit keys on {} rows took: {:?}", rows, duration);
        }
        print_communication_stats_operator(&nets, party_id, "Radix Sort 64-bit");

        let start = Instant::now();
        let _sorted_32bit = radix_sort_multithreads(input_clone, true, 32, &nets, &mut states)?;
        let duration = start.elapsed();

        if party_id == PartyID::ID0 {
            tracing::info!("Radix Sort with 32-bit keys on {} rows took: {:?}", rows, duration);
        }

        print_communication_stats_operator(&nets, party_id, "Radix Sort 32-bit");
    }
    
    Ok(())
}