

use itertools::izip;
use num_traits::{One};
use rand::{distributions::Standard, prelude::Distribution};
use protocols::protocols::rep3_ring::{Rep3RingShare, binary::xor_public};
use algebra::ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement};
use random::rep3::Rep3State;
use net::Network;
use primitives::{compare::{and_vec_multithreads, or_vec_multithreads},transform};
use primitives::compare::{eq_many_multithreads,neq_many_multithreads};
use primitives::mul::mul_share_vec;
use primitives::utils::get_data_share;


/// Before using this function, the input must be sorted.
pub fn distinct_after_groupby_multithreads<T: IntRing2k, N: Network>(
    vals: &[Rep3RingShare<T>],
    e: &[Rep3RingShare<T>],
    valid: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
Standard: Distribution<T>,{
    //*Mux(valid, result_cond)
    //*result_cond = cond1 + (1-cond1) * cond2

    let e_1 = e[0..e.len()-1].to_vec();
    let e_2 = e[1..e.len()].to_vec();
    let cond_1 = eq_many_multithreads(&e_1, &e_2, nets, states)?; 
    let neq_cond_1 = izip!(&cond_1).map(|c| xor_public(c,&RingElement::<Bit>::one(), states[0].id)).collect::<Vec<Rep3RingShare<Bit>>>();

    let vals_1 = vals[0..vals.len()-1].to_vec();
    let vals_2 = vals[1..vals.len()].to_vec();
    let cond_2 = neq_many_multithreads(&vals_1, &vals_2, nets, states)?;

    let cond_tmp = and_vec_multithreads::<Bit,N>(&neq_cond_1, &cond_2, nets, states)?;

    let result_cond_tmp = or_vec_multithreads::<Bit,N>(&cond_1, &cond_tmp, nets, states)?;
    
    let result_cond_u64 = transform::from_bit_to_arithmetic_t_multithreads(&result_cond_tmp, nets, states)?;

    let mut result_cond = Vec::with_capacity(result_cond_tmp.len()+1);
    let one_share = get_data_share::<T>(&T::one(), states[0].id)?;
    result_cond.push(one_share);
    result_cond.extend(result_cond_u64);

    let result_valid = mul_share_vec(valid, &result_cond, nets, states)?;

    Ok(result_valid)
}