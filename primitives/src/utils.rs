use communication::rep3::multinet_impl::reshare_many_multinet;
use itertools::izip;
use itertools::Itertools;
use communication::rep3::id::PartyID;
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};
use protocols::{rep3_ring::Rep3RingShare};
use protocols::rep3_ring::arithmetic::promote_to_trivial_share;
use net::Network;

/// Performs the opening of a shared value and returns the equivalent public value.
pub fn open_vec_multinet<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    nets: &[&N],
) -> eyre::Result<Vec<RingElement<T>>> {
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

