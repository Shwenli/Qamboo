//! Binary
//!
//! This module contains operations with binary shares

use super::{
    arithmetic::RingShare,
    ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement},
};
//use rayon::iter::{ParallelIterator,IndexedParallelIterator,IntoParallelRefIterator,IntoParallelRefMutIterator};
use crate::protocols::rep3_ring::{Rep3State, id::PartyID, network::Rep3NetworkExt};
use itertools::{Itertools, izip};
use net::Network;
use num_traits::{One, Zero};
use rand::{distributions::Standard, prelude::Distribution};
//use rayon::prelude::*;

mod ops;

/// Performs a bitwise XOR operation on two shared values.
pub fn xor<T: IntRing2k>(a: &RingShare<T>, b: &RingShare<T>) -> RingShare<T> {
    a ^ b
}

/// Performs a bitwise XOR operation on a shared value and a public value.
pub fn xor_public<T: IntRing2k>(
    shared: &RingShare<T>,
    public: &RingElement<T>,
    id: PartyID,
) -> RingShare<T> {
    let mut res = shared.to_owned();
    match id {
        PartyID::ID0 => res.a ^= public,
        PartyID::ID1 => res.b ^= public,
        PartyID::ID2 => {}
    }
    res
}

/// Performs a bitwise OR operation on two shared values.
pub fn or<T: IntRing2k, N: Network>(
    a: &RingShare<T>,
    b: &RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingShare<T>>
where
    Standard: Distribution<T>,
{
    let xor = a ^ b;
    let and = and(a, b, net, state)?;
    Ok(xor ^ and)
}

/// Performs a bitwise OR operation on two vector of shared values.
pub fn or_vec<T: IntRing2k, N: Network>(
    a: &[RingShare<T>],
    b: &[RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let xor = izip!(a, b)
        .map(|(a, b)| a ^ b)
        .collect_vec();
    let and = and_vec(a, b, net, state)?;
    Ok(izip!(xor.iter(), and.iter())
        .map(|(x, y)| *x ^ *y)
        .collect_vec())
}

/// Performs a bitwise OR operation on a shared value and a public value.
pub fn or_public<T: IntRing2k>(
    shared: &RingShare<T>,
    public: &RingElement<T>,
    id: PartyID,
) -> RingShare<T> {
    let tmp = shared & public;
    let xor = xor_public(shared, public, id);
    xor ^ tmp
}

/// Performs a bitwise AND operation on two shared values.
pub fn and<T: IntRing2k, N: Network>(
    a: &RingShare<T>,
    b: &RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingShare<T>>
where
    Standard: Distribution<T>,
{
    let (mut mask, mask_b) = state.rngs.rand.random_elements::<RingElement<T>>();
    mask ^= mask_b;
    let local_a = (a & b) ^ mask;
    let local_b = net.reshare(local_a)?;
    Ok(RingShare::new_ring(local_a, local_b))
}

/// Performs element-wise bitwise AND operation on the provided shared values.
pub fn and_vec<T: IntRing2k, N: Network>(
    a: &[RingShare<T>],
    b: &[RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<RingShare<T>>>
where
    Standard: Distribution<T>,
{
    /* 
    let (mut mask, mask_b) = state.rngs.rand.random_elements_vec::<RingElement<T>>(a.len());
    let local_a :Vec<RingElement<T>> = a.par_iter()
        .zip(b.par_iter())
        .zip(mask.par_iter_mut())
        .zip(mask_b.par_iter())
        .map(|(((a, b), mask), mask_b)| {
            *mask ^= *mask_b;
            (a & b) ^ *mask
        })
        .collect();
    */
    
    let local_a = izip!(a, b)
        .map(|(a, b)| {
            let (mut mask, mask_b) = state.rngs.rand.random_elements::<RingElement<T>>();
            mask ^= mask_b;
            (a & b) ^ mask
        })
        .collect_vec();
    
    let local_b = net.reshare(local_a.clone())?;
    Ok(izip!(local_a, local_b)
        .map(|(a, b)| RingShare::new_ring(a, b))
        .collect_vec())
}

/// bit and
pub fn and_vec_bit<N: Network>(
    a: &[RingShare<Bit>],
    b: &[RingShare<Bit>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<RingShare<Bit>>>
where
    Standard: Distribution<Bit>,
{
    let local_a = izip!(a, b)
        .map(|(a, b)| {
            let (mut mask, mask_b) = state.rngs.rand.random_elements::<RingElement<Bit>>();
            mask ^= mask_b;
            (a & b) ^ mask
        })
        .collect_vec();
    let local_b = net.reshare(local_a.clone())?;
    Ok(izip!(local_a, local_b)
        .map(|(a, b)| RingShare::new_ring(a, b))
        .collect_vec())
}


/// Performs a bitwise AND operation on a shared value and a public value.
pub fn and_with_public<T: IntRing2k>(
    shared: &RingShare<T>,
    public: &RingElement<T>,
) -> RingShare<T> {
    shared & public
}


/// Qamboo add : Performs a bitwise AND operation on a vector of shared values and a public value.
pub fn and_with_public_many<T: IntRing2k>(
    shared: &[RingShare<T>],
    public: &[RingElement<T>],
) -> Vec<RingShare<T>> {
    shared.iter().zip(public.iter()).map(|(s, p)| s & p).collect()
}

/// Shifts a share by a public value `F` to the right.
///
/// # Panics
/// This method panics if `public` is larger than the of bits of
/// the underlying `PrimeField`'s modulus'.
pub fn shift_r_public<T: IntRing2k>(shared: &RingShare<T>, public: RingElement<T>) -> RingShare<T> {
    // some special casing
    if public.is_zero() {
        return shared.to_owned();
    }
    let shift: usize = public
        .0
        .try_into()
        .expect("can cast shift operand to usize");
    shared >> shift
}

/// Shifts a share by a public value `F` to the left.
///
/// # Panics
/// This method panics if `public` is larger than the of bits of
/// the underlying `PrimeField`'s modulus'.
pub fn shift_l_public<T: IntRing2k>(shared: &RingShare<T>, public: RingElement<T>) -> RingShare<T> {
    // some special casing
    if public.is_zero() {
        return shared.to_owned();
    }
    let shift: usize = public
        .0
        .try_into()
        .expect("can cast shift operand to usize");
    shared << shift
}

/// Performs the opening of a shared value and returns the equivalent public value.
pub fn open<T: IntRing2k, N: Network>(a: &RingShare<T>, net: &N) -> eyre::Result<RingElement<T>> {
    let c = net.reshare(a.b)?;
    Ok(a.a ^ a.b ^ c)
}
/// Performs the opening of a vector of shared values and returns the equivalent public values.
pub fn open_vec<T: IntRing2k, N: Network>(
    a: &[RingShare<T>],
    net: &N,
) -> eyre::Result<Vec<RingElement<T>>> {
    let c = net.reshare(a.iter().map(|x| x.b.clone()).collect_vec())?;
    Ok(izip!(a, c)
        .map(|(x, c)| x.a ^ x.b ^ c)
        .collect()
    )
}

/// Transforms a public value into a shared value: \[a\] = a.
pub fn promote_to_trivial_share<T: IntRing2k>(
    id: PartyID,
    public_value: &RingElement<T>,
) -> RingShare<T> {
    match id {
        PartyID::ID0 => RingShare::new_ring(public_value.to_owned(), RingElement::zero()),
        PartyID::ID1 => RingShare::new_ring(RingElement::zero(), public_value.to_owned()),
        PartyID::ID2 => RingShare::zero_share(),
    }
}

/// Computes a CMUX: If `c` is `1`, returns `x_t`, otherwise returns `x_f`.
pub fn cmux<T: IntRing2k, N: Network>(
    c: &RingShare<T>,
    x_t: &RingShare<T>,
    x_f: &RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingShare<T>>
where
    Standard: Distribution<T>,
{
    let xor = x_f ^ x_t;
    let mut and = and(c, &xor, net, state)?;
    and ^= x_f;
    Ok(and)
}

/// Computes a CMUX for multiple inputs.
pub fn cmux_many<T: IntRing2k, N: Network>(
    c: &[RingShare<T>],
    x_t: &[RingShare<T>],
    x_f: &[RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<RingShare<T>>>
where
    Standard: Distribution<T>,
{
    let xor_vec = izip!(x_t, x_f)
        .map(|(x_t, x_f)| x_t ^ x_f)
        .collect::<Vec<_>>();

    let and = and_vec(c, &xor_vec, net, state)?;
    Ok(izip!(and.iter(), x_f.iter())
        .map(|(and, x_f)| *and ^ *x_f)
        .collect::<Vec<_>>())
}

//TODO most likely the inputs here are only one bit therefore we
//do not have to perform an or over the whole length of prime field
//but only one bit.
//Do we want that to be configurable? Semms like a waste?
/// Compute a OR tree of the input vec
pub fn or_tree<T: IntRing2k, N: Network>(
    mut inputs: Vec<RingShare<T>>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingShare<T>>
where
    Standard: Distribution<T>,
{
    let mut num = inputs.len();

    tracing::debug!("starting or tree over {} elements", inputs.len());
    while num > 1 {
        tracing::trace!("binary tree still has {} elements", num);
        let mod_ = num & 1;
        num >>= 1;

        let (a_vec, tmp) = inputs.split_at(num);
        let (b_vec, leftover) = tmp.split_at(num);

        let mut res = Vec::with_capacity(num);
        // TODO WE WANT THIS BATCHED!!!
        // THIS IS SUPER BAD
        for (a, b) in izip!(a_vec.iter(), b_vec.iter()) {
            res.push(or(a, b, net, state)?);
        }

        res.extend_from_slice(leftover);
        inputs = res;

        num += mod_;
    }
    let result = inputs[0];
    tracing::debug!("we did it!");
    Ok(result)
}

/// Computes a binary circuit to check whether the replicated binary-shared input x is zero or not. The output is a binary sharing of one bit.
pub fn is_zero<T: IntRing2k, N: Network>(
    x: &RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingShare<Bit>>
where
    Standard: Distribution<T>,
{
    // negate
    let mut x = !x;

    // do ands in a tree
    // TODO: Make and tree more communication efficient, ATM we send the full element for each level, even though they halve in size
    let mut len = T::K;
    debug_assert!(len.is_power_of_two());
    while len > 1 {
        // if len % 2 == 1 // Does not happen, we are in a ring with 2^k
        len >>= 1;
        let mask = (RingElement::one() << len) - RingElement::one();
        let y = x >> len;
        x = and(&(x & mask), &(y & mask), net, state)?;
    }
    // extract LSB
    Ok(RingShare {
        a: RingElement(Bit::new((x.a & RingElement::one()) == RingElement::one())),
        b: RingElement(Bit::new((x.b & RingElement::one()) == RingElement::one())),
    })
}




