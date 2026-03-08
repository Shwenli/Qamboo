//! Arithmetic
//!
//! This module contains operations with arithmetic shares

use communication::rep3::id::PartyID;
use communication::rep3::net_impl::Rep3NetworkImpl;
use random::rep3::Rep3State;
use itertools::{Itertools, izip};
use net::Network;
use num_traits::{Zero};
use rand::{distributions::Standard, prelude::Distribution};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use types::Rep3RingShare;
use algebra::ring::{int_ring::IntRing2k, ring_impl::RingElement};

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





