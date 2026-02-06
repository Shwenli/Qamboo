use itertools::izip;
use protocols::protocols::rep3_ring::{id::PartyID, network::Rep3NetworkExt};
use protocols::protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::{rep3_ring::Rep3RingShare};
use protocols::protocols::rep3_ring::arithmetic::promote_to_trivial_share;
use net::Network;

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

    let chunk_size = std::cmp::max(1, (data_len + net_num - 1) / net_num);
    let chunks: Vec<_> = data.chunks(chunk_size).map(|c| c).collect();

    Ok(chunks)
}

pub fn get_mut_task_chunks<T>(
    task: &mut [T],
    priv_len: usize,
    net_num: usize,
) -> eyre::Result<Vec<&mut [T]>> {

    let chunk_size = std::cmp::max(1, (priv_len + net_num - 1) / net_num);
    let chunks: Vec<_> = task.chunks_mut(chunk_size).map(|c| c).collect();
    Ok(chunks)
}

/// Performs a reshare on all shares in the vector.
pub fn reshare_vec_q<T: IntRing2k, N: Network>(
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

///顺序计算前缀和, 之后改为用rayon算
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

