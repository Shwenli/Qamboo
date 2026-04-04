use communication::rep3::id::PartyID;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
};
use table::NetStateArgs;
use net::Network;

pub fn print_communication_stats<N: Network>(args: &NetStateArgs<N>, query: &str) {
    let mut total_sent = 0;
    let mut total_recv = 0;

    let mut collect_stats = |net: &N| {
        let stats = net.get_connection_stats();
        for (_, (sent, recv)) in stats.iter() {
            total_sent += sent;
            total_recv += recv;
        }
    };

    for net in args.nets {
        collect_stats(*net);
    }
    /* 
    if args.states[0].id == PartyID::ID0{
        tracing::info!("Total {query} Communication Sent {:.4} MB, Recv {:.4} MB", total_sent as f64 / 1024.0 / 1024.0, total_recv as f64 / 1024.0 / 1024.0);
    }
    */
    let party_id = args.states[0].id;
    tracing::info!("Total Party{party_id} {query} Communication Sent {:.4} MB, Recv {:.4} MB", total_sent as f64 / 1024.0 / 1024.0, total_recv as f64 / 1024.0 / 1024.0);
    
}


pub fn print_communication_stats_operator<N: Network>(nets:&[&N], id:PartyID, operator_name:&str) {
    let mut total_sent = 0;
    let mut total_recv = 0;

    let mut collect_stats = |net: &N| {
        let stats = net.get_connection_stats();
        for (_, (sent, recv)) in stats.iter() {
            total_sent += sent;
            total_recv += recv;
        }
    };

    for net in nets {
        collect_stats(*net);
    }
    /* 
    if id == PartyID::ID0{
        tracing::info!("Total {operator_name} Communication Sent {:.4} MB, Recv {:.4} MB", total_sent as f64 / 1024.0 / 1024.0, total_recv as f64 / 1024.0 / 1024.0);
    }
    */
    tracing::info!("Party{id} {operator_name} Communication Sent {:.4} MB, Recv {:.4} MB", total_sent as f64 / 1024.0 / 1024.0, total_recv as f64 / 1024.0 / 1024.0);
    
}

pub fn install_tracing() {
    
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
