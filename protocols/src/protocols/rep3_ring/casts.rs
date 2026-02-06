//! Casts
//!
//! Implements casts for sharings of different datatypes

use super::{Rep3RingShare, conversion, ring::int_ring::IntRing2k};
use crate::protocols::{
    rep3_ring::Rep3State,
    rep3_ring::ring::{bit::Bit, ring_impl::RingElement},
};
use net::Network;
use num_traits::AsPrimitive;
use rand::{distributions::Standard, prelude::Distribution};
use std::any::TypeId;


/// A downcast of a Rep3RingShare from a larger ring to a smaller ring, truncating the excess bits.
/// Does not require network interaction
pub fn downcast<T, U>(share: Rep3RingShare<T>) -> Rep3RingShare<U>
where
    T: IntRing2k + AsPrimitive<U>,
    U: IntRing2k,
{
    assert!(T::K >= U::K);

    Rep3RingShare {
        a: RingElement(share.a.0.as_()),
        b: RingElement(share.b.0.as_()),
    }
}

/// An upcast of a Rep3RingShare from a smaller ring to a larger ring
/// Does require network interaction
pub fn upcast_a2b<T, U, N>(
    share: Rep3RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<U>>
where
    T: IntRing2k + AsPrimitive<U>,
    U: IntRing2k,
    N: Network,
    Standard: Distribution<T> + Distribution<U>,
{
    assert!(T::K < U::K);

    // A special case for Bit
    if TypeId::of::<T>() == TypeId::of::<Bit>() {
        let share = crate::downcast(&share).expect("We already checked types");
        return conversion::bit_inject_from_bit(share, net, state);
    }

    let binary = conversion::a2b(share, net, state)?;
    let binary = Rep3RingShare {
        a: RingElement(binary.a.0.as_()),
        b: RingElement(binary.b.0.as_()),
    };
    conversion::b2a(&binary, net, state)
}


/// A cast of a Rep3RingShare from a ring to another ring. In case of a downcast, the excess bits are just truncated.
pub fn cast_a2b<T, U, N>(
    share: Rep3RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> eyre::Result<Rep3RingShare<U>>
where
    T: IntRing2k + AsPrimitive<U>,
    U: IntRing2k,
    N: Network,
    Standard: Distribution<T> + Distribution<U>,
{
    if T::K >= U::K {
        Ok(downcast(share))
    } else {
        upcast_a2b(share, net, state)
    }
}






