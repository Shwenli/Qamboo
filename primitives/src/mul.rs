use communication::rep3::multinet_impl::reshare_many_multinet;
use protocols::protocols::rep3_ring::Rep3RingShare;
use random::rep3::{Rep3State, rep3rng_rayon::masking_elements_vec_multithreads};
use itertools::izip;
use net::Network;
use rand::{distributions::Standard, prelude::Distribution};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};

pub fn mul_share_vec<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let local_mul_vec = local_mul_vec_multithreads(lhs, rhs, states);
    reshare_vec_multinet(local_mul_vec, nets)
}

pub fn local_mul_vec_multithreads<T: IntRing2k>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    states: &mut [&mut Rep3State],
) -> Vec<RingElement<T>>
where
    Standard: Distribution<T>,
{
    let masking_fes = masking_elements_vec_multithreads::<RingElement<T>>(states, lhs.len());

    lhs.par_iter()
        .zip_eq(rhs.par_iter())
        .zip_eq(masking_fes.par_iter())
        .with_min_len(1024)
        .map(|((lhs, rhs), masking)| lhs * rhs + masking)
        .collect()
}

/// Performs a reshare on all shares in the vector.
pub fn reshare_vec_multinet<T: IntRing2k, N: Network>(
    local_a: Vec<RingElement<T>>,
    nets: &[&N],
) -> eyre::Result<Vec<Rep3RingShare<T>>> {
    let local_b = reshare_many_multinet(nets,&local_a)?;
    
    if local_b.len() != local_a.len() {
        eyre::bail!("Invalid number of elements received");
    }

    Ok(izip!(local_a, local_b)
        .map(|(a, b)| Rep3RingShare::new_ring(a, b))
        .collect())
}