use communication::rep3::multinet_impl::reshare_many_multinet;
use itertools::izip;
use itertools::Itertools;
use communication::rep3::id::PartyID;
use communication::rep3::net_impl::Rep3NetworkImpl;
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};
use protocols::protocols::{rep3_ring::Rep3RingShare};
use protocols::protocols::rep3_ring::arithmetic::promote_to_trivial_share;
use net::Network;

/// Performs the opening of a shared value and returns the equivalent public value.
pub fn open_vec_multinet<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    nets: &[&N],
) -> eyre::Result<Vec<RingElement<T>>> {
    // TODO think about something better... it is not so bad
    // because we use it exactly once in PLONK where we do it for 4
    // shares..
    let (a, b) = a
        .iter()
        .map(|share| (share.a, share.b))
        .collect::<(Vec<RingElement<T>>, Vec<RingElement<T>>)>();
    let c = reshare_many_multinet(nets, &b)?;
    Ok(izip!(a, b, c).map(|(a, b, c)| a + b + c).collect_vec())
}

pub fn get_data_share<T:IntRing2k>(
    data: &T,
    id: PartyID,
)-> eyre::Result<Rep3RingShare<T>> {

   let data_share =promote_to_trivial_share(id, RingElement(*data));

   Ok(data_share)
}

pub fn get_data_share_vec<T:IntRing2k>(
    data: &Vec<T>,
    id: PartyID,
)-> eyre::Result<Vec<Rep3RingShare<T>>> {

   let data_share =data.iter()
        .map(|d| promote_to_trivial_share(id, RingElement(*d)))
        .collect::<Vec<_>>();

   Ok(data_share)
}

pub fn get_one_share_vec<T:IntRing2k>(
    len:usize,
    id: PartyID,
)-> eyre::Result<Vec<Rep3RingShare<T>>> {

   let one_share =vec![promote_to_trivial_share(id, RingElement(T::one()));len];

   Ok(one_share)
}

pub fn get_task_chunks<T>(
    data: &[T],
    data_len: usize,
    net_num: usize,
) -> eyre::Result<Vec<&[T]>> {

    let mut chunks = Vec::with_capacity(net_num);
    let base_chunk_size = data_len / net_num;
    let remainder = data_len % net_num;
    let mut start = 0;

    for i in 0..net_num {
        let length = base_chunk_size + if i < remainder { 1 } else { 0 };
        let end = start + length;
        chunks.push(&data[start..end]);
        start = end;
    }

    Ok(chunks)
}

pub fn get_mut_task_chunks<T>(
    task: &mut [T],
    data_len: usize,
    net_num: usize,
) -> eyre::Result<Vec<&mut [T]>> {

    let mut chunks = Vec::with_capacity(net_num);
    let base_chunk_size = data_len / net_num;
    let remainder = data_len % net_num;
    
    let mut remaining_slice = task;

    for i in 0..net_num {
        let length = base_chunk_size + if i < remainder { 1 } else { 0 };
        let (chunk, rest) = remaining_slice.split_at_mut(length);
        chunks.push(chunk);
        remaining_slice = rest;
    }

    Ok(chunks)
}

/// Performs a reshare on all shares in the vector.
pub fn reshare_mulslice_vec<T: IntRing2k, N: Network>(
    local_a: &[RingElement<T>],
    net: &N,
) -> eyre::Result<Vec<Rep3RingShare<T>>> {
    let local_b = net.reshare_many(local_a)?;
    if local_b.len() != local_a.len() {
        eyre::bail!("Invalid number of elements received");
    }
    Ok(izip!(local_a, local_b)
        .map(|(a, b)| Rep3RingShare::new_ring(*a, b))
        .collect())
}

pub fn reshare_vec_q_multithreads<T: IntRing2k, N: Network>(
    data: &[RingElement<T>],
    net: &[&N],
) -> eyre::Result<Vec<Rep3RingShare<T>>> {

    let local_a_tasks = get_task_chunks(data, data.len(), net.len())?;
    
    let local_b = net::join_all(
        local_a_tasks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
            move || {
                n.reshare_many(chunk)
                    .unwrap_or_else(|e| panic!("Reshare failed: {:?}", e))
            }
        })
    );

    let local_b = local_b.concat();

    if local_b.len() != data.len() {
        eyre::bail!("Invalid number of elements received");
    }
    Ok(izip!(data, local_b)
        .map(|(a, b)| Rep3RingShare::new_ring(*a, b))
        .collect())
}

pub fn reshare_vec_multithreads<T: IntRing2k, N: Network>(
    data: Vec<RingElement<T>>,
    net: &[&N],
) -> eyre::Result<Vec<Rep3RingShare<T>>> {

    let local_a_tasks = get_task_chunks(&data, data.len(), net.len())?;
    
    let local_b = net::join_all(
        local_a_tasks.into_iter().zip(net.iter()).map(|(chunk, &n)| {
            move || {
                n.reshare_many(chunk)
                    .unwrap_or_else(|e| panic!("Reshare failed: {:?}", e))
            }
        })
    );

    let local_b = local_b.concat();

    if local_b.len() != data.len() {
        eyre::bail!("Invalid number of elements received");
    }
    Ok(izip!(data, local_b)
        .map(|(a, b)| Rep3RingShare::new_ring(a, b))
        .collect())
}

pub fn prefix_sum_sequential<T: IntRing2k>(
    inputs: &[Rep3RingShare<T>],
) -> eyre::Result<Vec<Rep3RingShare<T>>> {
    
    let n = inputs.len();
    let mut results = Vec::with_capacity(n);
    let mut current_sum = Rep3RingShare::zero_share();
    for input in inputs {
        current_sum += *input;
        results.push(current_sum);
    }
    
    Ok(results)
}

