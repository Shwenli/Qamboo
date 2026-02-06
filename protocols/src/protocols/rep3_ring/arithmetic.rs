//! Arithmetic
//!
//! This module contains operations with arithmetic shares

use crate::protocols::rep3_ring::{Rep3State, id::PartyID, network::Rep3NetworkExt};
use itertools::{Itertools, izip};
use net::Network;
use num_traits::{One, Zero};
use rand::{distributions::Standard, prelude::Distribution};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use types::Rep3RingShare;

use super::{
    ring::{int_ring::IntRing2k, ring_impl::RingElement},
};

mod ops;
pub(super) mod types;

/// Type alias for a [`Rep3RingShare`] which is used for both arithmetic and binary shares.
pub type RingShare<F> = Rep3RingShare<F>;

/// Performs addition between two shared values.
pub fn add<T: IntRing2k>(a: RingShare<T>, b: RingShare<T>) -> RingShare<T> {
    a + b
}

/// Performs addition between two shared values in place
pub fn add_assign<T: IntRing2k>(shared: &mut RingShare<T>, b: RingShare<T>) {
    *shared += b;
}

/// Performs addition between a shared value and a public value.
pub fn add_public<T: IntRing2k>(
    shared: RingShare<T>,
    public: RingElement<T>,
    id: PartyID,
) -> RingShare<T> {
    let mut res = shared;
    match id {
        PartyID::ID0 => res.a += public,
        PartyID::ID1 => res.b += public,
        PartyID::ID2 => {}
    }
    res
}

/// Performs addition between a shared value and a public value in place.
pub fn add_assign_public<T: IntRing2k>(
    shared: &mut RingShare<T>,
    public: RingElement<T>,
    id: PartyID,
) {
    match id {
        PartyID::ID0 => shared.a += public,
        PartyID::ID1 => shared.b += public,
        PartyID::ID2 => {}
    }
}
/// Performs addition between two shared values.
pub fn add_vec<T: IntRing2k>(lhs: &[RingShare<T>], rhs: &[RingShare<T>]) -> Vec<RingShare<T>> {
    izip!(lhs.iter(), rhs.iter()).map(|(a, b)| add(*a, *b)).collect()
}

/// Performs element-wise addition of two vectors of shared values in place.
pub fn add_vec_assign<T: IntRing2k>(lhs: &mut [RingShare<T>], rhs: &[RingShare<T>]) {
    for (a, b) in izip!(lhs.iter_mut(), rhs.iter()) {
        *a += b;
    }
}

/// Performs subtraction between two shared values, returning a - b.
pub fn sub<T: IntRing2k>(a: RingShare<T>, b: RingShare<T>) -> RingShare<T> {
    a - b
}

/// Performs subtraction between two shared values in place.
pub fn sub_assign<T: IntRing2k>(shared: &mut RingShare<T>, b: RingShare<T>) {
    *shared -= b;
}

/// Performs element-wise subtraction of two vectors of shared values in place.
pub fn sub_vec_assign<T: IntRing2k>(lhs: &mut [RingShare<T>], rhs: &[RingShare<T>]) {
    for (a, b) in izip!(lhs.iter_mut(), rhs.iter()) {
        *a -= *b;
    }
}

/// Performs subtraction between a shared value and a public value, returning shared - public.
pub fn sub_shared_by_public<T: IntRing2k>(
    shared: RingShare<T>,
    public: RingElement<T>,
    id: PartyID,
) -> RingShare<T> {
    add_public(shared, -public, id)
}

/// Performs subtraction between a shared value and a public value, returning public - shared.
pub fn sub_public_by_shared<T: IntRing2k>(
    public: RingElement<T>,
    shared: RingShare<T>,
    id: PartyID,
) -> RingShare<T> {
    add_public(-shared, public, id)
}

/// Performs multiplication of two shared values.
pub fn mul<T: IntRing2k, N: Network>(
    a: RingShare<T>,
    b: RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingShare<T>>
where
    Standard: Distribution<T>,
{
    let local_a = a * b + state.rngs.rand.masking_element::<RingElement<T>>();
    let local_b = net.reshare(local_a)?;
    Ok(RingShare {
        a: local_a,
        b: local_b,
    })
}

/// Performs multiplication of a shared value and a public value.
pub fn mul_public<T: IntRing2k>(shared: RingShare<T>, public: RingElement<T>) -> RingShare<T> {
    shared * public
}

/// Performs multiplication of a shared value and a public value.
pub fn mul_assign_public<T: IntRing2k>(shared: &mut RingShare<T>, public: RingElement<T>) {
    *shared *= public;
}



/// Performs element-wise multiplication of two vectors of shared values. *DOES NOT PERFORM RESHARE*
///
/// # Security
/// If you want to perform additional non-linear operations on the result of this function,
/// you *MUST* call [`reshare_vec`] first. Only then, a reshare is performed.
pub fn local_mul_vec<T: IntRing2k>(
    lhs: &[RingShare<T>],
    rhs: &[RingShare<T>],
    state: &mut Rep3State,
) -> Vec<RingElement<T>>
where
    Standard: Distribution<T>,
{
    //squeeze all random elements at once in beginning for determinismus
    let masking_fes = state
        .rngs
        .rand
        .masking_elements_vec::<RingElement<T>>(lhs.len());

    lhs.par_iter()
        .zip_eq(rhs.par_iter())
        .zip_eq(masking_fes.par_iter())
        .with_min_len(1024)
        .map(|((lhs, rhs), masking)| lhs * rhs + masking)
        .collect()
}

/// Performs a reshare on all shares in the vector.
pub fn reshare_vec<T: IntRing2k, N: Network>(
    local_a: Vec<RingElement<T>>,
    net: &N,
) -> eyre::Result<Vec<RingShare<T>>> {
    let local_b = net.reshare_many(&local_a)?;
    if local_b.len() != local_a.len() {
        eyre::bail!("Invalid number of elements received");
    }
    Ok(izip!(local_a, local_b)
        .map(|(a, b)| RingShare::new_ring(a, b))
        .collect())
}


/// Performs element-wise multiplication of two vectors of shared values.
/// Use this function for small vecs. For large vecs see [`local_mul_vec`] and [`reshare_vec`]
pub fn mul_vec<T: IntRing2k, N: Network>(
    lhs: &[RingShare<T>],
    rhs: &[RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<RingShare<T>>>
where
    Standard: Distribution<T>,
{
    // do not use local_mul_vec here!!! We are , this means we
    // run on a tokio runtime. local_mul_vec uses rayon and starves the
    // runtime. This method is for small multiplications of vecs.
    // If you want a larger one use local_mul_vec and then reshare_vec.
    debug_assert_eq!(lhs.len(), rhs.len());
    let local_a = izip!(lhs.iter(), rhs.iter())
        .map(|(lhs, rhs)| lhs * rhs + state.rngs.rand.masking_element::<RingElement<T>>())
        .collect_vec();
    reshare_vec(local_a, net)
}

/// Negates a shared value.
pub fn neg<T: IntRing2k>(a: RingShare<T>) -> RingShare<T> {
    -a
}

/// Performs the opening of a shared value and returns the equivalent public value.
pub fn open<T: IntRing2k, N: Network>(a: RingShare<T>, net: &N) -> eyre::Result<RingElement<T>> {
    let c = net.reshare(a.b)?;
    Ok(a.a + a.b + c)
}

/// Performs the opening of a shared value and returns the equivalent public value.
pub fn open_bit<T: IntRing2k, N: Network>(
    a: RingShare<T>,
    net: &N,
) -> eyre::Result<RingElement<T>> {
    let c = net.reshare(a.b.to_owned())?;
    Ok(a.a ^ a.b ^ c)
}
///Qamboo: create
pub fn open_vec_bit<T: IntRing2k, N: Network>(
    a: &[RingShare<T>],
    net: &N,
) -> eyre::Result<Vec<RingElement<T>>> {
    // TODO think about something better... it is not so bad
    // because we use it exactly once in PLONK where we do it for 4
    // shares..;
    let b: Vec<_> = a.iter().map(|share| share.b).collect();
    let c = net.reshare_many(&b)?;
    Ok(izip!(a.iter(), c.iter())
        .map(|(share, c)| share.a ^ share.b ^ c)
        .collect_vec())
}

/// Performs the opening of a shared value and returns the equivalent public value.
pub fn open_vec<T: IntRing2k, N: Network>(
    a: &[RingShare<T>],
    net: &N,
) -> eyre::Result<Vec<RingElement<T>>> {
    // TODO think about something better... it is not so bad
    // because we use it exactly once in PLONK where we do it for 4
    // shares..
    let (a, b) = a
        .iter()
        .map(|share| (share.a, share.b))
        .collect::<(Vec<RingElement<T>>, Vec<RingElement<T>>)>();
    let c = net.reshare_many(&b)?;
    Ok(izip!(a, b, c).map(|(a, b, c)| a + b + c).collect_vec())
}



/// Convenience method for \[a\] + \[b\] * c
pub fn add_mul_public<T: IntRing2k>(
    a: RingShare<T>,
    b: RingShare<T>,
    c: RingElement<T>,
) -> RingShare<T> {
    add(a, mul_public(b, c))
}

/// Convenience method for \[a\] + \[b\] * \[c\]
pub fn add_mul<T: IntRing2k, N: Network>(
    a: RingShare<T>,
    b: RingShare<T>,
    c: RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingShare<T>>
where
    Standard: Distribution<T>,
{
    let mul = mul(c, b, net, state)?;
    Ok(add(a, mul))
}

/// Transforms a public value into a shared value: \[a\] = a.
pub fn promote_to_trivial_share<T: IntRing2k>(
    id: PartyID,
    public_value: RingElement<T>,
) -> RingShare<T> {
    match id {
        PartyID::ID0 => Rep3RingShare::new_ring(public_value, RingElement::zero()),
        PartyID::ID1 => Rep3RingShare::new_ring(RingElement::zero(), public_value),
        PartyID::ID2 => Rep3RingShare::zero_share(),
    }
}

/// This function performs a multiplication directly followed by an opening. This safes one round of communication in some MPC protocols compared to calling `mul` and `open` separately.
pub fn mul_open<T: IntRing2k, N: Network>(
    a: RingShare<T>,
    b: RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingElement<T>>
where
    Standard: Distribution<T>,
{
    let a = a * b + state.rngs.rand.masking_element::<RingElement<T>>();
    let (b, c) = net.broadcast(a)?;
    Ok(a + b + c)
}

/// This function performs a multiplication directly followed by an opening. This safes one round of communication in some MPC protocols compared to calling `mul` and `open` separately.
pub fn mul_open_vec<T: IntRing2k, N: Network>(
    a: &[RingShare<T>],
    b: &[RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<RingElement<T>>>
where
    Standard: Distribution<T>,
{
    let mut a = izip!(a, b)
        .map(|(a, b)| a * b + state.rngs.rand.masking_element::<RingElement<T>>())
        .collect_vec();
    let (b, c) = net.broadcast_many(&a)?;
    izip!(a.iter_mut(), b, c).for_each(|(a, b, c)| *a += b + c);
    Ok(a)
}



/// Performs a pow operation using a shared value as base and a public value as exponent.
pub fn pow_public<T: IntRing2k, N: Network>(
    shared: RingShare<T>,
    mut public: RingElement<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingShare<T>>
where
    Standard: Distribution<T>,
{
    // TODO: are negative exponents allowed in circom?
    let mut res = promote_to_trivial_share(state.id, RingElement::one());
    let mut shared: RingShare<T> = shared;
    while !public.is_zero() {
        if public.get_bit(0) == RingElement::one() {
            public -= RingElement::one();
            res = mul(res, shared, net, state)?;
        }
        shared = mul(shared, shared, net, state)?;
        public >>= 1;
    }
    Ok(res)
}

/// computes XOR using arithmetic operations, only valid when x and y are known to be 0 or 1.
pub fn arithmetic_xor<T: IntRing2k, N: Network>(
    x: RingShare<T>,
    y: RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<RingShare<T>>
where
    Standard: Distribution<T>,
{
    let mut d = x * y + state.rngs.rand.masking_element::<RingElement<T>>();
    d <<= 1;
    let e = x.a + y.a;
    let res_a = e - d;

    let res_b = net.reshare(res_a)?;
    Ok(RingShare { a: res_a, b: res_b })
}

/// computes XOR on many inputs using arithmetic operations, only valid when x and y are known to be 0 or 1.
pub fn arithmetic_xor_many<T: IntRing2k, N: Network>(
    x: &[RingShare<T>],
    y: &[RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Vec<RingShare<T>>>
where
    Standard: Distribution<T>,
{
    debug_assert_eq!(x.len(), y.len());

    let mut a = Vec::with_capacity(x.len());
    for (x, y) in x.iter().zip(y.iter()) {
        let mut d = x * y + state.rngs.rand.masking_element::<RingElement<T>>();
        d <<= 1;
        let e = x.a + y.a;
        let res_a = e - d;
        a.push(res_a);
    }

    let b = net.reshare_many(&a)?;
    let res = a
        .into_iter()
        .zip(b)
        .map(|(a, b)| RingShare { a, b })
        .collect();
    Ok(res)
}




