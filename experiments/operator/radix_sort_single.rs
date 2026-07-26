use std::path::PathBuf;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use operator::sort::radix_sort;
use random::rep3::Rep3State;
use communication::rep3::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
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

    /// Shiften factor (2^shift rows)
    #[clap(short = 's', long, value_name = "SHIFT", default_value = "20")]
    shift: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    install_tracing();

    let shift = args.shift;
    let rows = 1 << shift as usize;
    let partyid = args.party_id.clone();

    tracing::info!("Single-thread Radix Sort with rows: {}", rows);
    tracing::info!("setting up network");

    let file_path = PathBuf::from(format!("{}0/config_party{}.toml", args.config_dir.display(), partyid));
    let config: NetworkConfig = toml::from_str(&std::fs::read_to_string(file_path).context("opening config file")?).context("parsing config file")?;
    let net = TcpNetwork::new(config)?;
    let mut state = Rep3State::new(&net)?;

    tracing::info!("Network setup completed");

    tracing::info!("Generating input");
    let input = gen_rand_column_u64_ring(
        rows,
        "test".to_string(), 
        rows as u64, 
        1, 
        ShareType::Arithmetic, 
        &[&net],  
        state.id
    ).0.get_data().to_vec();

    tracing::info!("Starting Single-thread Radix Sort");
    let start = Instant::now();

    let _sorted = radix_sort(input, true, 64, &net, &mut state)?;

    if state.id == PartyID::ID0 {
        tracing::info!("INFO Total single-thread radix sort execution time: {:?}", start.elapsed());
    }
    print_communication_stats_operator(&[&net], state.id, "Single-thread Radix Sort");
    
    Ok(())
}
