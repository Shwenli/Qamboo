use std::mem::ManuallyDrop;
use communication::rep3::id::PartyID;
use algebra::ring::{bit::Bit, int_ring::IntRing2k, ring_impl::RingElement};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use num_traits::Zero;
use serde::{Deserialize, Serialize};

/// This type represents a replicated shared value. Since a replicated share of a ring element contains additive shares of two parties, this type contains two ring elements.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    CanonicalSerialize,
    CanonicalDeserialize,
)]
#[serde(bound = "")]
//#[repr(C)]
pub struct Rep3RingShare<T: IntRing2k> {
    /// Share of this party
    pub a: RingElement<T>,
    /// Share of the prev party
    pub b: RingElement<T>,
}

impl<T: IntRing2k> Default for Rep3RingShare<T> {
    fn default() -> Self {
        Self::zero_share()
    }
}

impl<T: IntRing2k> Rep3RingShare<T> {
    /// Constructs the type from two additive shares.
    pub fn new(a: T, b: T) -> Self {
        Self {
            a: RingElement(a),
            b: RingElement(b),
        }
    }

    /// Constructs a new share from two ring elements
    pub fn new_ring(a: RingElement<T>, b: RingElement<T>) -> Self {
        Self { a, b }
    }

    /// Constructs a zero share.
    pub fn zero_share() -> Self {
        Self {
            a: RingElement::zero(),
            b: RingElement::zero(),
        }
    }

    /// Unwraps the type into two additive shares.
    pub fn ab(self) -> (RingElement<T>, RingElement<T>) {
        (self.a, self.b)
    }

    /// Double the share in place
    pub fn double(&mut self) {
        self.a <<= 1;
        self.b <<= 1;
    }
    
    ///这个就是运行中生成share
    /// Promotes a public ring element to a replicated share by setting the additive share of the party with id=0 and leaving all other shares to be 0. Thus, the replicated shares of party 0 and party 1 are set.
    pub fn promote_from_trivial(val: &RingElement<T>, id: PartyID) -> Self {
        match id {
            PartyID::ID0 => Self::new_ring(*val, RingElement::zero()),
            PartyID::ID1 => Self::new_ring(RingElement::zero(), *val),
            PartyID::ID2 => Self::zero_share(),
        }
    }

    /// Return the bit at position `index`.
    pub fn get_bit(&self, index: usize) -> Rep3RingShare<Bit> {
        Rep3RingShare {
            a: RingElement(Bit::new(self.a.get_bit(index).0 == T::one())),
            b: RingElement(Bit::new(self.b.get_bit(index).0 == T::one())),
        }
    }

    /// View a slice of `Rep3RingShare<T>` as raw bytes, **zero-copy**.
    ///
    /// # Safety rationale
    /// - `Rep3RingShare<T>` is `#[repr(C)]` with two `RingElement<T>` fields.
    /// - `RingElement<T>` is `#[repr(transparent)]` over `T`.
    /// - On little-endian platforms the in-memory layout matches the wire format.
    #[inline]
    pub fn slice_as_bytes(slice: &[Self]) -> &[u8] {
        #[cfg(not(target_endian = "little"))]
        compile_error!("zero-copy share ↔ byte conversion requires a little-endian target");

        // SAFETY: Rep3RingShare is repr(C) with two repr(transparent) RingElement<T> fields.
        // The entire slice is contiguous [a0, b0, a1, b1, ...] in memory.
        unsafe {
            std::slice::from_raw_parts(
                slice.as_ptr() as *const u8,
                std::mem::size_of_val(slice),
            )
        }
    }

    /// Convert a `Vec<u8>` received from the network into a
    /// `Vec<Rep3RingShare<T>>`, **zero-copy** (no per-element deserialization).
    ///
    /// Returns `Err` if the byte length is not a multiple of `size_of::<Rep3RingShare<T>>()`.
    #[inline]
    pub fn vec_from_bytes(mut bytes: Vec<u8>) -> eyre::Result<Vec<Self>> {
        #[cfg(not(target_endian = "little"))]
        compile_error!("zero-copy share ↔ byte conversion requires a little-endian target");

        let share_size = std::mem::size_of::<Self>();
        if bytes.len() % share_size != 0 {
            eyre::bail!(
                "byte length {} is not a multiple of share size {}",
                bytes.len(),
                share_size
            );
        }
        let new_len = bytes.len() / share_size;
        let new_cap = bytes.capacity() / share_size;
        let ptr = bytes.as_mut_ptr() as *mut Self;
        std::mem::forget(bytes);
        // SAFETY: same layout guarantees as slice_as_bytes, in reverse.
        Ok(unsafe { Vec::from_raw_parts(ptr, new_len, new_cap) })
    }

    /// Convert a `Vec<Rep3RingShare<T>>` into a `Vec<RingElement<T>>`
    /// containing `[a0, b0, a1, b1, ...]`, **zero-copy**.
    #[inline]
    pub fn vec_to_ring_elements(mut shares: Vec<Self>) -> Vec<RingElement<T>> {
        let new_len = shares.len() * 2;
        let new_cap = shares.capacity() * 2;
        let ptr = shares.as_mut_ptr() as *mut RingElement<T>;
        let _ = ManuallyDrop::new(shares);
        // SAFETY: Rep3RingShare is repr(C) with two RingElement<T> fields.
        unsafe { Vec::from_raw_parts(ptr, new_len, new_cap) }
    }

    /// Convert a `Vec<RingElement<T>>` with layout `[a0, b0, a1, b1, ...]`
    /// into a `Vec<Rep3RingShare<T>>`, **zero-copy**.
    ///
    /// Returns `Err` if the length is not even.
    #[inline]
    pub fn vec_from_ring_elements(mut elems: Vec<RingElement<T>>) -> eyre::Result<Vec<Self>> {
        if elems.len() % 2 != 0 {
            eyre::bail!("element count {} is not even", elems.len());
        }
        let new_len = elems.len() / 2;
        let new_cap = elems.capacity() / 2;
        let ptr = elems.as_mut_ptr() as *mut Self;
        let _ = ManuallyDrop::new(elems);
        // SAFETY: Rep3RingShare is repr(C) with two RingElement<T> fields.
        Ok(unsafe { Vec::from_raw_parts(ptr, new_len, new_cap) })
    }
}
