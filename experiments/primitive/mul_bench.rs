use std::path::PathBuf;
use std::time::Instant;
use clap::Parser;
use color_eyre::{Result, eyre::Context};
use random::rep3::Rep3State;
use random::MpcState;
use communication::rep3::id::PartyID;
use net::tcp::{TcpNetwork, NetworkConfig};
use primitives::mul::mul_share_vec;
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
}


fn main() -> Result<()> {
    let args = Args::parse();
    install_tracing();

    let partyid = args.party_id.clone();
    let default_states = rayon::current_num_threads() / 2;

    tracing::info!("setting up network");
    let mut nets: Vec<TcpNetwork> = Vec::new();
    let mut states: Vec<Rep3State> = Vec::new();

    for i in 0..args.threads {
        let file_path = PathBuf::from(format!("{}{}/config_party{}.toml", args.config_dir.display(), i, partyid));
        let config: NetworkConfig = toml::from_str(&std::fs::read_to_string(file_path).context("opening config file")?).context("parsing config file")?;
        let net = TcpNetwork::new(config)?;
        let state = Rep3State::new(&net)?;
        nets.push(net);
        states.push(state);
    }

    if states.len() <= default_states {
        let diff = default_states - states.len();
        for _i in 0..diff {
            let state = states[0].fork(0)?;
            states.push(state);
        }
    }

    let nets = nets.iter().collect::<Vec<&TcpNetwork>>();
    let mut states = states.iter_mut().collect::<Vec<&mut Rep3State>>();
    let party_id = states[0].id;

    tracing::info!("Network setup completed");

    for i in 0..10 {
        let rows = 1usize << (16 + i);

        tracing::info!("Generating inputs with {} elements", rows);
        let lhs = gen_rand_column_u64_ring(
            rows,
            "lhs".to_string(),
            rows as u64,
            1,
            ShareType::Arithmetic,
            &nets,
            party_id,
        ).0.get_data().to_vec();

        let rhs = gen_rand_column_u64_ring(
            rows,
            "rhs".to_string(),
            rows as u64,
            1,
            ShareType::Arithmetic,
            &nets,
            party_id,
        ).0.get_data().to_vec();

        tracing::info!("Starting multiplication on {} elements", rows);
        let start = Instant::now();

        let _product = mul_share_vec(&lhs, &rhs, &nets, &mut states)?;

        let duration = start.elapsed();

        if party_id == PartyID::ID0 {
            tracing::info!("Total multiplication on {} elements execution time: {:?}", rows, duration);
        }
        print_communication_stats_operator(&nets, party_id, "Multiplication");
    }

    Ok(())
}
