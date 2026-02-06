
use core::panic;
use itertools::izip;
use num_traits::{One};
use rand::{distributions::Standard, prelude::Distribution};
use protocols::protocols::rep3_ring::{
    Rep3State,
    Rep3RingShare, binary, conversion,
    ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement}, arithmetic
};
use net::Network;
use crate::utils::get_task_chunks;
use crate::kogge_stone_adder;

pub(crate) fn unsigned_ge_const_lhs_many<T: IntRing2k, N: Network>(
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

    // do ands in a tree with progressive masking for efficiency
    // As we go down the tree, we only need to keep the lower bits
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

///Qamboo: Returns vector of 1 if lhs[i] <= rhs[i] and 0 otherwise. Checks if shared values in lhs are less than or equal to corresponding shared values in rhs. The result is a vector of shared values that have value 1 if the corresponding lhs value is less than or equal to the corresponding rhs value and 0 otherwise.
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
    //let t_t = std::time::Instant::now();

    let bits = conversion::a2b_many(&diff, net, state)?;

    //let t_e = t_t.elapsed();
    //eprintln!("Time for a2b in eq_many: {:?}", t_e);
    
    let is_zero = is_zero_many(&bits, net, state)?;

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
    // 在二进制环中, XOR 是加法, NOT 是 1 - x
    let xor_res = izip!(a, b)
        .map(|(a_i, b_i)| a_i ^ b_i)
        .collect::<Vec<_>>();
    let is_zero = is_zero_many(&xor_res, net, state)?;

    Ok(is_zero)
}

pub fn eq_many_binary_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net:&[&N],
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let xor_res = izip!(a, b)
        .map(|(a_i, b_i)| a_i ^ b_i)
        .collect::<Vec<_>>();

    let xor_res_chunks = get_task_chunks(&xor_res, xor_res.len(), net.len())?;
    let is_zero = net::join_all(
        xor_res_chunks.into_iter().zip(net.iter()).zip(state.iter_mut()).map(|((xor_res_chunk, &net_i), state_i)| {
            move || {
                let bits = is_zero_many(&xor_res_chunk, net_i, state_i).unwrap_or_else(|e|panic!("eq many multithreads error: {:?}",e));
                bits
            }
        })
    );
    
    let is_zero = is_zero.concat();

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

pub fn eq_public_many_multithreads<T: IntRing2k, N: Network>(
    shared: &[Rep3RingShare<T>],
    public: &RingElement<T>,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<Bit>>>
where
    Standard: Distribution<T>,
{
    let public = binary::promote_to_trivial_share(states[0].id, public);
    let public = vec![public; shared.len()];
    eq_many_multithreads(shared, &public, nets, states)
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
    let public = binary::promote_to_trivial_share(states[0].id, public);
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


// must binary share
pub fn and_vec_multithreads<T: IntRing2k, N: Network>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    net: &[&N],
    state: &mut [&mut Rep3State],
) -> eyre::Result<Vec<Rep3RingShare<T>>>
where
    Standard: Distribution<T>,
{ 
    let a_chunks = get_task_chunks(a, a.len(), net.len())?;
    let b_chunks = get_task_chunks(b, b.len(), net.len())?;

    let res = net::join_all(
        a_chunks.into_iter().zip(b_chunks.into_iter()).zip(net.iter()).zip(state.iter_mut())
        .map(|(((a_chunk, b_chunk), &n), state)| {
            move || {
                binary::and_vec(a_chunk, b_chunk, n, state).unwrap_or_else(|e|panic!("and_vec_multithreads error: {}", e))
            }
        })
    );

    let res = res.concat();

    Ok(res)
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




