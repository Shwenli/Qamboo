
use itertools::izip;
use itertools::Itertools;
use num_traits::{One};
use net::Network;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use random ::rep3::Rep3State;
use random::rep3::rep3rng_rayon::random_elements_vec_multithreads;
use communication::rep3::multinet_impl::reshare_many_multinet;
use rand::{distributions::Standard, prelude::Distribution};
use protocols::rep3_ring::{Rep3RingShare, binary, conversion};
use protocols::rep3_ring::arithmetic;
use algebra::ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement};
use crate::transform::a2b_many_multithreads;
use crate::kogge_stone_adder;

pub fn unsigned_ge_const_lhs_many<T: IntRing2k, N: Network>(
    x: &RingElement<T>,
    y: &[Rep3RingShare<T>],
    net: &N,    
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let b_bits = conversion::a2b_many(y, net, state)?;
    let len = y.len();
    let vec_x = vec![*x; len];
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_from_const_with_carry_many(&vec_x, &b_bits, net, state)?;

    Ok(r)
}

pub fn unsigned_ge_const_lhs_many_multithreads<T: IntRing2k, N: Network>(
    x: &RingElement<T>,
    y: &[Rep3RingShare<T>],
    nets: &[&N],    
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let b_bits = a2b_many_multithreads(y, nets, states)?;
    let len = y.len();
    let vec_x = vec![*x; len];
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_from_const_with_carry_many_multithreads(&vec_x, &b_bits, nets, states)?;

    Ok(r)
}

/// For input y is binary
pub (crate) fn unsigned_ge_const_lhs_many_binary<T: IntRing2k, N: Network>(
    x: &RingElement<T>,
    y: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let len = y.len();
    let vec_x = vec![*x; len];
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_from_const_with_carry_many(&vec_x, &y, net, state)?;

    Ok(r)
}

pub fn unsigned_ge_const_lhs_many_binary_multithreads<T: IntRing2k, N: Network>(
    x: &RingElement<T>,
    y: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let len = y.len();
    let vec_x = vec![*x; len];
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_from_const_with_carry_many_multithreads(&vec_x, &y, nets, states)?;

    Ok(r)
}

pub(crate) fn unsigned_ge_const_rhs_many<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    y: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let a_bits = conversion::a2b_many(x, net, state)?;
    let len = x.len();
    let vec_y = vec![*y; len];
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_by_const_with_carry_many(&a_bits, &vec_y, net, state)?;
    Ok(r)
}

pub fn unsigned_ge_const_rhs_many_multithreads<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    y: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let a_bits = a2b_many_multithreads(x, nets, states)?;
    let len = x.len();
    let vec_y = vec![*y; len];
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_by_const_with_carry_many_multithreads(&a_bits, &vec_y, nets, states)?;
    Ok(r)
}

pub (crate) fn unsigned_ge_const_rhs_many_binary<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    y: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let len = x.len();
    let vec_y = vec![*y; len];
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_by_const_with_carry_many(&x, &vec_y, net, state)?;

    Ok(r)
}

pub fn unsigned_ge_const_rhs_many_binary_multithreads<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    y: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let len = x.len();
    let vec_y = vec![*y; len];
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_by_const_with_carry_many_multithreads(&x, &vec_y, nets, states)?;

    Ok(r)
}

pub(crate) fn unsigned_ge_many<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    y: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let a_bits = conversion::a2b_many(x, net, state)?;
    let b_bits = conversion::a2b_many(y, net, state)?;
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_with_carry_many(&a_bits, &b_bits, net, state)?;
    Ok(r)
}

pub fn unsigned_ge_many_multithreads<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    y: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let a_bits = a2b_many_multithreads(x, nets, states)?;
    let b_bits = a2b_many_multithreads(y, nets, states)?;
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_with_carry_many_multithreads(&a_bits, &b_bits, nets, states)?;
    Ok(r)
}

/// x and y are binary shares
pub (crate) fn unsigned_ge_many_binary<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    y: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_with_carry_many(&x, &y, net, state)?;
    Ok(r)
}

pub fn unsigned_ge_many_binary_multithreads<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    y: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let (_, r) = kogge_stone_adder::low_depth_binary_sub_with_carry_many_multithreads(&x, &y, nets, states)?;
    Ok(r)
}

/// Batched version of is_zero that checks multiple values at once.
/// This is more communication efficient as it batches the AND operations.
pub fn is_zero_many<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let mut x = izip!(x).map(|x| !x).collect::<Vec<_>>();

    let mut len = T::K;
    debug_assert!(len.is_power_of_two());
    while len > 1 {
        // if len % 2 == 1 // Does not happen, we are in a ring with 2^k
        len >>= 1;
        let mask = (RingElement::one() << len) - RingElement::one();
        
        // Split each element: upper half and lower half
        let y = izip!(&x).map(|x| (x >> len) & mask).collect::<Vec<_>>();
        let x_masked = izip!(x.iter()).map(|x| *x & mask).collect::<Vec<_>>();
        
        // AND the two halves together, now we only need `len` bits
        x = binary::and_vec(&x_masked, &y, net, state)?;
    }
    
    // extract LSB
    let res = izip!(x.iter()).map(|x| {
        Rep3RingShare {
            a: RingElement(Bit::new((x.a & RingElement::one()) == RingElement::one())),
            b: RingElement(Bit::new((x.b & RingElement::one()) == RingElement::one())),
        }
    }).collect::<Vec<_>>();
    
    Ok(res)
}

/// Batched version of is_zero that checks multiple values at once.
/// This is more communication efficient as it batches the AND operations.
pub fn is_zero_many_multithreads<T: IntRing2k, N: Network>(
    x: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let mut x = izip!(x).map(|x| !x).collect::<Vec<_>>();

    let mut len = T::K;
    debug_assert!(len.is_power_of_two());
    while len > 1 {
        // if len % 2 == 1 // Does not happen, we are in a ring with 2^k
        len >>= 1;
        let mask = (RingElement::one() << len) - RingElement::one();

        let (y, x_masked): (Vec<_>, Vec<_>) = x.par_iter()
            .with_min_len(1024)
            .map(|x| {
                let y_i = (x >> len) & mask;
                let x_masked = *x & mask;
                (y_i, x_masked)
            })
            .unzip();
        
        // AND the two halves together, now we only need `len` bits
        x = and_vec_multithreads(&x_masked, &y, nets, states)?;
    }
    
    // extract LSB
    let res: Vec<_> = x.par_iter()
    .with_min_len(1024)
    .map(|x| {
        Rep3RingShare {
            a: RingElement(Bit::new((x.a & RingElement::one()) == RingElement::one())),
            b: RingElement(Bit::new((x.b & RingElement::one()) == RingElement::one())),
        }
    }).collect();
    
    Ok(res)
}

/// Returns vector of 1 if lhs[i] >= rhs[i] and 0 otherwise. Checks if shared values in lhs are greater than or equal to corresponding shared values in rhs. The result is a vector of shared values that have value 1 if the corresponding lhs value is greater than or equal to the corresponding rhs value and 0 otherwise.
pub fn ge_many<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    unsigned_ge_many(lhs, rhs, net, state)
}

pub fn ge_many_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    unsigned_ge_many_multithreads(lhs, rhs, nets, states)
}

pub fn ge_many_binary<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    unsigned_ge_many_binary(lhs, rhs, net, state)
}

pub fn ge_many_binary_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    unsigned_ge_many_binary_multithreads(lhs, rhs, nets, states)
}

pub fn ge_public_many<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    unsigned_ge_const_rhs_many(lhs, rhs, net, state)
}

pub fn ge_public_many_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    unsigned_ge_const_rhs_many_multithreads(lhs, rhs, nets, states)
}

pub fn ge_public_many_binary<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    unsigned_ge_const_rhs_many_binary(lhs, rhs, net, state)
}

pub fn ge_public_many_binary_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    unsigned_ge_const_rhs_many_binary_multithreads(lhs, rhs, nets, states)
}

/// Returns 1 if lhs < rhs and 0 otherwise. Checks if one shared value is less than another shared value. The result is a shared value that has value 1 if the first shared value is less than the second shared value and 0 otherwise.
pub fn lt_many<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a < b is equivalent to !(a >= b)
    let tmp = ge_many(lhs, rhs, net, state)?;
    let res = izip!(tmp).map(|x| 
    arithmetic::sub_public_by_shared(RingElement::one(), x, state.id)).collect();
    Ok(res)
}

pub fn lt_many_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a < b is equivalent to !(a >= b)
    let tmp = ge_many_multithreads(lhs, rhs, nets, states)?;
    let res = izip!(tmp).map(|x| 
    arithmetic::sub_public_by_shared(RingElement::one(), x, states[0].id)).collect();
    Ok(res)
}

pub fn lt_many_binary<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a < b is equivalent to !(a >= b)
    let tmp = ge_many_binary(lhs, rhs, net, state)?;
    let res = izip!(tmp).map(|x| 
    arithmetic::sub_public_by_shared(RingElement::one(), x, state.id)).collect();
    Ok(res)

}

pub fn lt_many_binary_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a < b is equivalent to !(a >= b)
    let tmp = ge_many_binary_multithreads(lhs, rhs, nets, states)?;
    let res = izip!(tmp).map(|x| 
    arithmetic::sub_public_by_shared(RingElement::one(), x, states[0].id)).collect();
    Ok(res)
}

pub fn lt_public_many<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a < b is equivalent to !(a >= b)
    let tmp = ge_public_many(lhs, rhs, net, state)?;
    let res = izip!(tmp).map(|x| 
    arithmetic::sub_public_by_shared(RingElement::one(), x, state.id)).collect();
    Ok(res)
}

pub fn lt_public_many_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a < b is equivalent to !(a >= b)
    let id = states[0].id;
    let tmp = ge_public_many_multithreads(lhs, rhs, nets, states)?;
    let res = izip!(tmp).map(|x| 
    arithmetic::sub_public_by_shared(RingElement::one(), x, id)).collect();
    Ok(res)
}

pub fn lt_public_many_binary<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let tmp = ge_public_many_binary(lhs, rhs, net, state)?;
    let res = izip!(tmp).map(|x| 
    arithmetic::sub_public_by_shared(RingElement::one(), x, state.id)).collect();
    Ok(res)
}

pub fn lt_public_many_binary_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let id = states[0].id;
    let tmp = ge_public_many_binary_multithreads(lhs, rhs, nets, states)?;
    let res = izip!(tmp).map(|x| 
    arithmetic::sub_public_by_shared(RingElement::one(), x, id)).collect();
    Ok(res)
}

///Returns vector of 1 if lhs[i] <= rhs[i] and 0 otherwise. Checks if shared values in lhs are less than or equal to corresponding shared values in rhs. The result is a vector of shared values that have value 1 if the corresponding lhs value is less than or equal to the corresponding rhs value and 0 otherwise.
pub fn le_many<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a <= b is equivalent to b >= a
    ge_many(rhs, lhs, net, state)
}

pub fn le_many_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a <= b is equivalent to b >= a
    ge_many_multithreads(rhs, lhs, nets, states)
}


pub fn le_many_binary<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a <= b is equivalent to b >= a
    ge_many_binary(rhs, lhs, net, state)
}

pub fn le_many_binary_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a <= b is equivalent to b >= a
    ge_many_binary_multithreads(rhs, lhs, nets, states)
}

pub fn le_public_many<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a <= b is equivalent to b >= a
    unsigned_ge_const_lhs_many(rhs, lhs, net, state)
}

pub fn le_public_many_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a <= b is equivalent to b >= a
    unsigned_ge_const_lhs_many_multithreads(rhs, lhs, nets, states)
}

pub fn le_public_many_binary<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a <= b is equivalent to b >= a
    unsigned_ge_const_lhs_many_binary(rhs, lhs, net, state)
}

pub fn le_public_many_binary_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a <= b is equivalent to b >= a
    unsigned_ge_const_lhs_many_binary_multithreads(rhs, lhs, nets, states)
}

/// Returns vector of 1 if lhs[i] > rhs[i] and 0 otherwise. Checks if shared values in lhs are greater than corresponding shared values in rhs. The result is a vector of shared values that have value 1 if the corresponding lhs value is greater than the corresponding rhs value and 0 otherwise.
pub fn gt_many<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a > b is equivalent to !(a <= b)
    let tmp = le_many(lhs, rhs, net, state)?;
    let not_tmp = izip!(tmp).map(|x| !x).collect();
    Ok(not_tmp)
}

pub fn gt_many_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a > b is equivalent to !(a <= b)
    let tmp = le_many_multithreads(lhs, rhs, nets, states)?;
    let not_tmp = izip!(tmp).map(|x| !x).collect();
    Ok(not_tmp)
}

pub fn gt_many_binary<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a > b is equivalent to !(a <= b)
    let tmp = le_many_binary(lhs, rhs, net, state)?;
    let not_tmp = izip!(tmp).map(|x| !x).collect();
    Ok(not_tmp)
}

pub fn gt_many_binary_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a > b is equivalent to !(a <= b)
    let tmp = le_many_binary_multithreads(lhs, rhs, nets, states)?;
    let not_tmp = izip!(tmp).map(|x| !x).collect();
    Ok(not_tmp)
}

pub fn gt_public_many<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a > b is equivalent to !(a <= b)
    let tmp = le_public_many(lhs, rhs, net, state)?;
    let not_tmp = izip!(tmp).map(|x| !x).collect();
    Ok(not_tmp)
}

pub fn gt_public_many_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a > b is equivalent to !(a <= b)
    let tmp = le_public_many_multithreads(lhs, rhs, nets, states)?;
    let not_tmp = izip!(tmp).map(|x| !x).collect();
    Ok(not_tmp)
}

pub fn gt_public_many_binary<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a > b is equivalent to !(a <= b)
    let tmp = le_public_many_binary(lhs, rhs, net, state)?;
    let not_tmp = izip!(tmp).map(|x| !x).collect();
    Ok(not_tmp)
}

pub fn gt_public_many_binary_multithreads<T: IntRing2k, N: Network>(
    lhs: &[Rep3RingShare<T>],
    rhs: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a > b is equivalent to !(a <= b)
    let tmp = le_public_many_binary_multithreads(lhs, rhs, nets, states)?;
    let not_tmp = izip!(tmp).map(|x| !x).collect();
    Ok(not_tmp)
}


/// ORQ-style less-than-zero: extracts the sign bit (MSB) of a *binary* (XOR) shared value
/// and returns it as a shared bit. I.e., returns 1 iff the plaintext, interpreted in two's
/// complement, is negative. Purely local, no communication: for binary sharings,
/// MSB(x) = MSB(x1) ^ MSB(x2) ^ MSB(x3) since XOR is bitwise.
///
/// The input MUST be a binary sharing. For arithmetic sharings the MSB of the sum is not
/// the XOR of the per-share MSBs (carries propagate upwards); convert with a2b first.
pub fn ltz_orq<T: IntRing2k>(x: &Rep3RingShare<T>) -> Rep3RingShare<Bit> {
    x.get_bit(T::K - 1)
}

/// Batched variant of [ltz_orq].
pub fn ltz_orq_many<T: IntRing2k>(x: &[Rep3RingShare<T>]) -> Vec<Rep3RingShare<Bit>> {
    x.iter().map(ltz_orq).collect()
}

/// Multithreaded variant of [ltz_orq_many]. The operation is purely local,
/// so this only parallelizes the map, no network is involved.
pub fn ltz_orq_many_multithreads<T: IntRing2k>(x: &[Rep3RingShare<T>]) -> Vec<Rep3RingShare<Bit>> {
    x.par_iter().with_min_len(1024).map(ltz_orq).collect()
}




/* 
pub fn eq_many_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net:&[&N],
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let diff = izip!(a, b).map(|(a, b)| a - b).collect::<Vec<_>>();

    let diff_chunks = get_task_chunks(&diff, diff.len(), net.len())?;

    let is_zero = net::join_all(
        diff_chunks.into_iter().zip(net.iter()).zip(state.iter_mut()).map(|((diff_chunk, &net_i), state_i)| {
            move || {
                let bits = conversion::a2b_many(&diff_chunk, net_i, state_i).unwrap_or_else(|e|panic!("eq many multithreads a2b error: {:?}",e));
                is_zero_many(&bits, net_i, state_i).unwrap_or_else(|e|panic!("eq many multithreads error: {:?}",e))
            }
        })
    );

    let is_zero = is_zero.concat();
    
    Ok(is_zero)
}
*/

/// Returns vector of 1 if lhs[i] == rhs[i] and 0 otherwise. Checks if shared values in lhs are equal to corresponding shared values in rhs. The result is a vector of shared values that have value 1 if the corresponding lhs value is equal to the corresponding rhs value and 0 otherwise.
pub fn eq_many<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let diff = izip!(a, b).map(|(a, b)| a - b).collect::<Vec<_>>();
    let bits = conversion::a2b_many(&diff, net, state)?;
    let is_zero = is_zero_many(&bits, net, state)?;

    Ok(is_zero)
}

pub fn eq_many_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    nets:&[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let diff = izip!(a, b).map(|(a, b)| a - b).collect::<Vec<_>>();

    let diff_bits = a2b_many_multithreads(&diff, nets, states)?;
    let is_zero = is_zero_many_multithreads(&diff_bits, nets, states)?;
    
    Ok(is_zero)
}

pub fn eq_many_binary<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    // a == b  <=>  NOT (a XOR b)
    let xor_res = izip!(a, b)
        .map(|(a_i, b_i)| a_i ^ b_i)
        .collect::<Vec<_>>();
    let is_zero = is_zero_many(&xor_res, net, state)?;

    Ok(is_zero)
}

pub fn eq_many_binary_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net: &[&N],
    state: &mut[&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let xor_res = izip!(a, b)
        .map(|(a_i, b_i)| a_i ^ b_i)
        .collect::<Vec<_>>();

    let is_zero = is_zero_many_multithreads(&xor_res, net, state)?;

    Ok(is_zero)
}

pub fn eq_public_many<T: IntRing2k, N: Network>(
    shared: &[Rep3RingShare<T>],
    public: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let public = arithmetic::promote_to_trivial_share(state.id, *public);
    let public = vec![public; shared.len()];
    eq_many(shared, &public, net, state)
}

pub fn eq_public_many_multithreads<T: IntRing2k, N: Network>(
    shared: &[Rep3RingShare<T>],
    public: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let id = states[0].id;
    let public = arithmetic::promote_to_trivial_share(id, *public);
    let public = vec![public; shared.len()];
    eq_many_multithreads(shared, &public, nets, states)
}

pub fn eq_public_many_binary<T: IntRing2k, N: Network>(
    shared: &[Rep3RingShare<T>],
    public: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let public = binary::promote_to_trivial_share(state.id, public);
    let public = vec![public; shared.len()];
    eq_many_binary(shared, &public, net, state)
}

pub fn eq_public_many_binary_multithreads<T: IntRing2k, N: Network>(
    shared: &[Rep3RingShare<T>],
    public: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let id = states[0].id;
    let public = binary::promote_to_trivial_share(id, public);
    let public = vec![public; shared.len()];
    eq_many_binary_multithreads(shared, &public, nets, states)
}

/// Returns vector of 1 if lhs[i] != rhs[i] and 0 otherwise. Checks if shared values in lhs are not equal to corresponding shared values in rhs. The result is a vector of shared values that have value 1 if the corresponding lhs value is not equal to the corresponding rhs value and 0 otherwise.
pub fn neq_many<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let eq = eq_many(a, b, net, state)?;//所有权被拿走，不用保留eq
    let not_eq = izip!(eq).map(|x| !x).collect();
    Ok(not_eq)
}

pub fn neq_many_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let eq = eq_many_multithreads(a, b, nets, states)?;//所有权被拿走，不用保留eq
    let not_eq = izip!(eq).map(|x| !x).collect();
    Ok(not_eq)
}

pub fn neq_many_binary<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let eq = eq_many_binary(a, b, net, state)?;
    let not_eq = izip!(eq).map(|x| !x).collect();
    Ok(not_eq)
}

pub fn neq_many_binary_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let eq = eq_many_binary_multithreads(a, b, nets, states)?;
    let not_eq = izip!(eq).map(|x| !x).collect();
    Ok(not_eq)
}

pub fn neq_public_many<T: IntRing2k, N: Network>(
    shared: &[Rep3RingShare<T>],
    public: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let public_share = binary::promote_to_trivial_share(state.id, public);
    let public_shares = vec![public_share; shared.len()];
    neq_many(shared, &public_shares, net, state)
} 

pub fn neq_public_many_multithreads<T: IntRing2k, N: Network>(
    shared: &[Rep3RingShare<T>],
    public: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let public_share = binary::promote_to_trivial_share(states[0].id, public);
    let public_shares = vec![public_share; shared.len()];
    neq_many_multithreads(shared, &public_shares, nets, states)
}

pub fn neq_public_many_binary<T: IntRing2k, N: Network>(
    shared: &[Rep3RingShare<T>],
    public: &RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let public_share = binary::promote_to_trivial_share(state.id, public);
    let public_shares = vec![public_share; shared.len()];
    neq_many_binary(shared, &public_shares, net, state)
}

pub fn neq_public_many_binary_multithreads<T: IntRing2k, N: Network>(
    shared: &[Rep3RingShare<T>],
    public: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let public_share = binary::promote_to_trivial_share(states[0].id, public);
    let public_shares = vec![public_share; shared.len()];
    neq_many_binary_multithreads(shared, &public_shares, nets, states)
}

/* 
// must binary share
pub fn and_vec_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{ 
    let a_chunks = get_task_chunks(a, a.len(), nets.len())?;
    let b_chunks = get_task_chunks(b, b.len(), nets.len())?;

    let res = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut())
        .map(|(((a_chunk, b_chunk), &n), state)| {
            move || {
                binary::and_vec(a_chunk, b_chunk, n, state).unwrap_or_else(|e|panic!("and_vec_multithreads error: {}", e))
            }
        })
    );

    let res = res.concat();

    Ok(res)
}
*/
// a and b must be binary shares
pub fn and_vec_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let len = a.len();
    let (mut mask,mask_b) = random_elements_vec_multithreads::<RingElement<T>>(states, len);
    
    let local_a :Vec<RingElement<T>> = a.par_iter()
        .zip(b.par_iter())
        .zip(mask.par_iter_mut())
        .zip(mask_b.par_iter())
        .with_min_len(1024)
        .map(|(((a, b), mask), mask_b)| {
            *mask ^= *mask_b;
            (a & b) ^ *mask
        })
        .collect();
    
    let local_b = reshare_many_multinet(nets, &local_a.clone())?;
    
    Ok(izip!(local_a, local_b)
        .map(|(a, b)| Rep3RingShare::new_ring(a, b))
        .collect_vec())
}

pub fn and_vec_bit_multithreads<N: Network>(
    a: &[Rep3RingShare<Bit>],
    b: &[Rep3RingShare<Bit>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<Bit>,
{
    let (mut mask, mask_b) = random_elements_vec_multithreads::<RingElement<Bit>>(states, a.len());
    let local_a:Vec<_> = a.par_iter()
        .zip(b.par_iter())
        .zip(mask.par_iter_mut())
        .zip(mask_b.par_iter())
        .with_min_len(1024)
        .map(|(((a, b), mask), mask_b)| {
            *mask ^= *mask_b;
            (a & b) ^ *mask
        })
        .collect();
    /* 
    let local_a = izip!(a, b)
        .map(|(a, b)| {
            let (mut mask, mask_b) = state.rngs.rand.random_elements::<RingElement<Bit>>();
            mask ^= mask_b;
            (a & b) ^ mask
        })
        .collect_vec();
    */
    let local_b = reshare_many_multinet(nets, &local_a)?;
    Ok(izip!(local_a, local_b)
        .map(|(a, b)| Rep3RingShare::new_ring(a, b))
        .collect_vec())
}

pub fn or_vec_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{ 
    let xor = a.par_iter()
        .zip(b.par_iter())
        .with_min_len(1024)
        .map(|(a, b)| a ^ b)
        .collect::<Vec<_>>();

    let and = and_vec_multithreads(a, b, nets, states)?;

    let res = xor.par_iter()
        .zip(and.par_iter())
        .with_min_len(1024)
        .map(|(x, y)| *x ^ *y)
        .collect::<Vec<_>>();

    Ok(res)
}

/* 
pub fn or_vec_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{ 
    let a_chunks = get_task_chunks(a, a.len(), nets.len())?;
    let b_chunks = get_task_chunks(b, b.len(), nets.len())?;

    let res = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(nets.iter()).zip(states.iter_mut())
        .map(|(((a_chunk, b_chunk), &n), state)| {
            move || {
                binary::or_vec(a_chunk, b_chunk, n, state).unwrap_or_else(|e|panic!("or_vec_multithreads error: {}", e))
            }
        })
    );

    let res = res.concat();

    Ok(res)
}
*/








#[cfg(test)]
mod ltz_orq_test {
    use super::*;
    use rand::Rng;

    // Builds the three party views of a replicated XOR sharing of x:
    // x = x1 ^ x2 ^ x3, party i holds (x_i, x_{i+1}).
    fn share_binary(x: u64, rng: &mut impl Rng) -> [Rep3RingShare<u64>; 3] {
        let x1: u64 = rng.r#gen();
        let x2: u64 = rng.r#gen();
        let x3 = x ^ x1 ^ x2;
        [
            Rep3RingShare::new(x1, x2),
            Rep3RingShare::new(x2, x3),
            Rep3RingShare::new(x3, x1),
        ]
    }

    fn open_bit(shares: &[Rep3RingShare<Bit>; 3]) -> bool {
        // party i holds (s_i, s_{i+1}); collect s1, s2, s3 and XOR
        shares[0].a.0.convert() ^ shares[0].b.0.convert() ^ shares[1].b.0.convert()
    }

    #[test]
    fn ltz_orq_matches_two_complement_sign() {
        let mut rng = rand::thread_rng();
        let mut cases = vec![
            0u64,
            1,
            42,
            (1u64 << 63) - 1,
            1u64 << 63,
            (1u64 << 63) + 1,
            u64::MAX - 1,
            u64::MAX,
        ];
        for _ in 0..100 {
            cases.push(rng.r#gen());
        }

        for x in cases {
            let shares = share_binary(x, &mut rng);
            let bits = [
                ltz_orq(&shares[0]),
                ltz_orq(&shares[1]),
                ltz_orq(&shares[2]),
            ];
            assert_eq!(open_bit(&bits), x >> 63 == 1, "ltz_orq wrong for {x:#x}");
        }
    }

    #[test]
    fn ltz_orq_many_matches_single() {
        let mut rng = rand::thread_rng();
        let cases: Vec<u64> = (0..1000).map(|_| rng.r#gen()).collect();
        let party0: Vec<_> = cases.iter().map(|&x| share_binary(x, &mut rng)[0]).collect();

        let single: Vec<_> = party0.iter().map(ltz_orq).collect();
        let batch = ltz_orq_many(&party0);
        let batch_mt = ltz_orq_many_multithreads(&party0);

        assert_eq!(single, batch);
        assert_eq!(single, batch_mt);
    }
}
