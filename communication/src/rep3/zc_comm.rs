
use crate::rep3::zc_net_impl::{Rep3ZeroCopyNetworkImpl, ZeroCopy};
use crate::rep3::id::PartyID;
use crate::task::get_task_chunks;
use net::Network;

// ══════════════════════════════════════════════════════════════
//  Multi-network wrappers for zc_net_impl (zero-copy) operations
// ══════════════════════════════════════════════════════════════

/// Reshare slices across multiple networks in parallel (zero-copy).
pub fn reshare_many_zc_multinet<Z: ZeroCopy + Clone, N: Network>(
    nets: &[&N],
    data: &[Z],
) -> eyre::Result<Vec<Z>> {
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    let results = net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                n.reshare_many_zc(chunk)
                    .unwrap_or_else(|e| panic!("reshare_many_zc failed: {:?}", e))
            }
        })
    );

    Ok(results.concat())
}

/// Send-and-receive slices across multiple networks in parallel (zero-copy).
pub fn send_and_recv_many_zc_multinet<Z: ZeroCopy + Clone, N: Network>(
    nets: &[&N],
    to: PartyID,
    data: &[Z],
    from: PartyID,
) -> eyre::Result<Vec<Z>> {
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    let results = net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                n.send_and_recv_many_zc(to, chunk, from)
                    .unwrap_or_else(|e| panic!("send_and_recv_many_zc failed: {:?}", e))
            }
        })
    );

    Ok(results.concat())
}

/// Broadcast slices to both parties across multiple networks in parallel (zero-copy).
pub fn broadcast_many_zc_multinet<Z: ZeroCopy + Clone, N: Network>(
    nets: &[&N],
    data: &[Z],
) -> eyre::Result<(Vec<Z>, Vec<Z>)> {
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    let results = net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                n.broadcast_many_zc(chunk)
                    .unwrap_or_else(|e| panic!("broadcast_many_zc failed: {:?}", e))
            }
        })
    );

    let mut prev_all = Vec::with_capacity(data.len());
    let mut next_all = Vec::with_capacity(data.len());
    for (prev, next) in results {
        prev_all.extend(prev);
        next_all.extend(next);
    }
    Ok((prev_all, next_all))
}




