use std::path::PathBuf;

use clap::Parser;
use color_eyre::{Result, eyre::Context};
use net::{
    Network as _,
    tcp::{NetworkConfig, TcpNetwork},
};
use std::time::Instant;

#[derive(Parser)]
struct Args {
    /// The config file path
    #[clap(short, long, value_name = "CONFIG")]
    config: PathBuf,
}

fn install_tracing() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{
        EnvFilter,
        fmt::{self, format::FmtSpan},
    };

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_line_number(false)
        .with_span_events(FmtSpan::CLOSE | FmtSpan::ENTER);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

fn main() -> Result<()> {
    let args = Args::parse();
    install_tracing();

    let config: NetworkConfig =
        toml::from_str(&std::fs::read_to_string(args.config).context("opening config file")?)
            .context("parsing config file")?;
    let my_id = config.my_id;

    let network = TcpNetwork::new(config)?;

     let send_start = Instant::now();
    // send to all parties
    for id in 0..3 {
        if id != my_id {
            tracing::info!("party {my_id} sending to {id}");
            let send_time = Instant::now();
            let buf = vec![id as u8; 8000000];
            network.send(id, &buf)?;
            let send_elapsed = send_time.elapsed();
            tracing::info!("party {my_id} sent to {id} in {:?}", send_elapsed);
        }
    }
    let total_send_time = send_start.elapsed();
    tracing::info!("party {my_id} total send time: {:?}", total_send_time);

    // 记录接收时间
    let recv_start = Instant::now();
    // recv from all parties
    for id in 0..3 {
        if id != my_id {
            let recv_time = Instant::now();
            let buf = network.recv(id)?;
            let recv_elapsed = recv_time.elapsed();
            assert!(buf.iter().all(|&x| x == my_id as u8));
            tracing::info!("party {my_id} received from {id} in {:?}", recv_elapsed);
        }
    }
    let total_recv_time = recv_start.elapsed();
    tracing::info!("party {my_id} total recv time: {:?}", total_recv_time);

    // 总传输时间
    let total_communication_time = send_start.elapsed();
    tracing::info!("party {my_id} total communication time: {:?}", total_communication_time);

    println!(
        "Network connection stats:\n{}",
        network.get_connection_stats()
    );

    println!("\n=== Timing Statistics ===");
    println!("Total send time: {:?}", total_send_time);
    println!("Total recv time: {:?}", total_recv_time);
    println!("Total communication time: {:?}", total_communication_time);

    Ok(())
}
