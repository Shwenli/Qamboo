use ark_serialize::{CanonicalSerialize,CanonicalDeserialize};
use crate::rep3::id::PartyID;
use crate::rep3::net_impl::Rep3NetworkImpl;
use crate::task::get_task_chunks;
use net::Network;

// ══════════════════════════════════════════════════════════════
//  Multi-network wrappers for net_impl (serialized) operations
// ══════════════════════════════════════════════════════════════


#[inline(always)]
pub fn send_many_multinet<F, N>(
    nets: &[&N],
    to: PartyID,
    data: &[F],
) -> eyre::Result<()> 
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                n.send_many(to, chunk)
                    .unwrap_or_else(|e| panic!("send_many failed: {:?}", e))
            }
        })
    );

    Ok(())
}

#[inline(always)]
pub fn send_next_multinet<F, N>(
    nets: &[&N],
    data: &[F],
) -> eyre::Result<()> 
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    net::join_all(
        nets.iter().map(|&n| {
            move || {
                n.send_next(data)
                    .unwrap_or_else(|e| panic!("send_next failed: {:?}", e))
            }
        })
    );

    Ok(())
}


#[inline(always)]
pub fn recv_many_multinet<F, N>(
    nets: &[&N],
    from: PartyID,
) -> eyre::Result<Vec<F>> 
where
    F:CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let results = net::join_all(
        nets.iter().map(|&n| {
            move || {
                n.recv_many(from)
                    .unwrap_or_else(|e| panic!("recv_many failed: {:?}", e))
            }
        })
    );

    Ok(results.concat())
}


#[inline(always)]
pub fn recv_prev_multinet<F, N>(
    nets: &[&N],
) -> eyre::Result<Vec<F>> 
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let results = net::join_all(
        nets.iter().map(|&n| {
            move || {
                n.recv_prev::<F>()
                    .unwrap_or_else(|e| panic!("recv_prev failed: {:?}", e))
            }
        })
    );

    Ok(results)
}


#[inline(always)]
pub fn send_next_many_multinet<F, N>(
    nets: &[&N],
    data: &[F],
) -> eyre::Result<()> 
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                let id = PartyID::try_from(n.id()).unwrap();
                n.send_many(id.next(), chunk)
                    .unwrap_or_else(|e| panic!("send_next_many failed: {:?}", e))
            }
        })
    );

    Ok(())
}

#[inline(always)]
pub fn recv_next_many_multinet<F, N>(
    nets: &[&N],
) -> eyre::Result<Vec<F>> 
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let results = net::join_all(
        nets.iter().map(|&n| {
            move || {
                let id = PartyID::try_from(n.id()).unwrap();
                n.recv_many(id.next())
                    .unwrap_or_else(|e| panic!("recv_next_many failed: {:?}", e))
            }
        })
    );

    Ok(results.concat())
}

#[inline(always)]
pub fn send_prev_many_multinet<F, N>(
    nets: &[&N],
    data: &[F],
) -> eyre::Result<()> 
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                let id = PartyID::try_from(n.id()).unwrap();
                n.send_many(id.prev(), chunk)
                    .unwrap_or_else(|e| panic!("send_prev_many failed: {:?}", e))
            }
        })
    );

    Ok(())
}

#[inline(always)]
pub fn recv_prev_many_multinet<F, N>(
    nets: &[&N],
) -> eyre::Result<Vec<F>> 
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let results = net::join_all(
        nets.iter().map(|&n| {
            move || {
                let id = PartyID::try_from(n.id()).unwrap();
                n.recv_many(id.prev())
                    .unwrap_or_else(|e| panic!("recv_prev_many failed: {:?}", e))
            }
        })
    );

    Ok(results.concat())
}


#[inline(always)]
pub fn reshare_many_multinet<F: CanonicalSerialize + CanonicalDeserialize + Send + Clone, N: Network>(
    nets: &[&N],
    data: &[F],
    ) -> eyre::Result<Vec<F>> {

    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    let reshares = net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                n.reshare_many(chunk)
                    .unwrap_or_else(|e| panic!("Reshare failed: {:?}", e))
            }
        })
    );

    let result = reshares.concat();
    
    Ok(result)
}


/// Send-and-receive slices across multiple networks in parallel (serialized).
#[inline(always)]
pub fn send_and_recv_many_multinet<F, N>(
    nets: &[&N],
    to: PartyID,
    data: &[F],
    from: PartyID,
) -> eyre::Result<Vec<F>>
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    let results = net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                n.send_and_recv_many(to, chunk, from)
                    .unwrap_or_else(|e| panic!("send_and_recv_many failed: {:?}", e))
            }
        })
    );

    Ok(results.concat())
}

/// Broadcast slices to both parties across multiple networks in parallel (serialized).
#[inline(always)]
pub fn broadcast_many_multinet<F, N>(
    nets: &[&N],
    data: &[F],
) -> eyre::Result<(Vec<F>, Vec<F>)>
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    let results = net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                n.broadcast_many(chunk)
                    .unwrap_or_else(|e| panic!("broadcast_many failed: {:?}", e))
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

/// Reshare slices across multiple networks in parallel (fast path for fixed-size types).
#[inline(always)]
pub fn reshare_many_fast_multinet<F, N>(
    nets: &[&N],
    data: &[F],
) -> eyre::Result<Vec<F>>
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    let results = net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                n.reshare_many_fast(chunk)
                    .unwrap_or_else(|e| panic!("reshare_many_fast failed: {:?}", e))
            }
        })
    );

    Ok(results.concat())
}

/// Broadcast slices to both parties across multiple networks in parallel
#[inline(always)]
pub fn broadcast_many_fast_multinet<F, N>(
    nets: &[&N],
    data: &[F],
) -> eyre::Result<(Vec<F>, Vec<F>)>
where
    F: CanonicalSerialize + CanonicalDeserialize + Send + Clone,
    N: Network,
{
    let data_chunks = get_task_chunks(data, data.len(), nets.len())?;

    let results = net::join_all(
        data_chunks.into_iter().zip(nets.iter()).map(|(chunk, &n)| {
            move || {
                n.broadcast_many_fast(chunk)
                    .unwrap_or_else(|e| panic!("broadcast_many_fast failed: {:?}", e))
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



