//! # REP3 Ring
//!
//! This module implements the rep3 share and combine operations for rings

pub mod arithmetic;
pub mod binary;
pub mod casts;
pub mod conversion;
pub mod detail;
pub mod ring;
pub mod id;
pub mod network;
pub mod rngs;



/// Shorthand type for a secret shared bit.
pub type Rep3BitShare = Rep3RingShare<ring::bit::Bit>;
pub use arithmetic::types::Rep3RingShare;
use rand::{CryptoRng, Rng, SeedableRng, distributions::Standard, prelude::Distribution};
use ring::{int_ring::IntRing2k, ring_impl::RingElement};

use std::marker::PhantomData;

use crate::serde_compat::{ark_de, ark_se};
use crate::{MpcState, RngType};
use ark_ff::{PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use id::PartyID;
use net::Network;
use network::Rep3NetworkExt;
use rngs::{Rep3CorrelatedRng, Rep3Rand, Rep3RandBitComp};
use serde::{Deserialize, Serialize};


/// The Rng used for expanding compressed Shares
pub type SeedRng = rand_chacha::ChaCha12Rng;

/// The internal state of the REP3 protocol
pub struct Rep3State {
    /// The id of the party
    pub id: PartyID,
    /// The correlated rng for rep3
    pub rngs: Rep3CorrelatedRng,
    /// The rng type
    pub rng: RngType,

}

impl Rep3State {
    /// Create a new [Rep3State] with the given [A2BType]
    pub fn new<N: Network>(net: &N) -> eyre::Result<Self> {
        let id = PartyID::try_from(net.id())?;
        let mut rng = rand_chacha::ChaCha12Rng::from_entropy();
        let mut rand = Self::setup_prf(net, &mut rng)?;
        let bitcomps = Self::setup_bitcomp(net, &mut rand)?;
        let rngs = Rep3CorrelatedRng::new(rand, bitcomps.0, bitcomps.1);

        Ok(Rep3State {
            id,
            rngs,
            rng,
        })
    }

    fn setup_prf<N: Network, R: Rng + CryptoRng>(net: &N, rng: &mut R) -> eyre::Result<Rep3Rand> {
        let seed1: [u8; crate::SEED_SIZE] = rng.r#gen();
        let seed2: [u8; crate::SEED_SIZE] = net.reshare(seed1)?;

        Ok(Rep3Rand::new(seed1, seed2))
    }

    fn setup_bitcomp<N: Network>(
        net: &N,
        rands: &mut Rep3Rand,
    ) -> eyre::Result<(Rep3RandBitComp, Rep3RandBitComp)> {
        let id = PartyID::try_from(net.id())?;
        let (k1a, k1c) = rands.random_seeds();
        let (k2a, k2c) = rands.random_seeds();

        match id {
            PartyID::ID0 => {
                let k2b: [u8; crate::SEED_SIZE] =
                    net.send_and_recv(PartyID::ID1, k1c, PartyID::ID2)?;
                let bitcomp1 = Rep3RandBitComp::new_2keys(k1a, k1c);
                let bitcomp2 = Rep3RandBitComp::new_3keys(k2a, k2b, k2c);
                Ok((bitcomp1, bitcomp2))
            }
            PartyID::ID1 => {
                net.send_next((k1c, k2c))?;
                let k1b: [u8; crate::SEED_SIZE] = net.recv_prev()?;
                let bitcomp1 = Rep3RandBitComp::new_3keys(k1a, k1b, k1c);
                let bitcomp2 = Rep3RandBitComp::new_2keys(k2a, k2c);
                Ok((bitcomp1, bitcomp2))
            }
            PartyID::ID2 => {
                net.send_next(k2c)?;
                let (k1b, k2b): ([u8; crate::SEED_SIZE], [u8; crate::SEED_SIZE]) =
                    net.recv_prev()?;
                let bitcomp1 = Rep3RandBitComp::new_3keys(k1a, k1b, k1c);
                let bitcomp2 = Rep3RandBitComp::new_3keys(k2a, k2b, k2c);
                Ok((bitcomp1, bitcomp2))
            }
        }
    }
}

impl MpcState for Rep3State {
    type PartyID = PartyID;

    fn id(&self) -> Self::PartyID {
        self.id
    }

    fn fork(&mut self, _: usize) -> eyre::Result<Self> {
        let id = self.id;
        let rngs = self.rngs.fork();
        let rng = RngType::from_seed(self.rng.r#gen());

        Ok(Self {
            id,
            rngs,
            rng,
        })
    }
}



/// A type that represents a compressed additive share. It can either be a seed (with length) or the actual share.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum SeededType<
    T: Clone + CanonicalSerialize + CanonicalDeserialize,
    U: Rng + SeedableRng + CryptoRng,
> where
    U::Seed: std::fmt::Debug + Clone + Serialize + for<'a> Deserialize<'a>,
{
    /// The actual additive share
    Shares(#[serde(serialize_with = "ark_se", deserialize_with = "ark_de")] T),
    /// A compressed additive share
    Seed(U::Seed, usize, PhantomData<U>),
}

impl<T: Clone + CanonicalSerialize + CanonicalDeserialize, U: Rng + SeedableRng + CryptoRng> Clone
    for SeededType<T, U>
where
    U::Seed: std::fmt::Debug + Clone + Serialize + for<'a> Deserialize<'a>,
{
    fn clone(&self) -> Self {
        match self {
            SeededType::Shares(val) => SeededType::Shares(val.clone()),
            SeededType::Seed(seed, len, _) => SeededType::Seed(seed.clone(), *len, PhantomData),
        }
    }
}

impl<F: PrimeField, U: Rng + SeedableRng + CryptoRng> SeededType<Vec<F>, U>
where
    U::Seed: std::fmt::Debug + Clone + Serialize + for<'a> Deserialize<'a>,
{
    /// Expands the compressed share.
    pub fn expand_vec(self) -> Vec<F> {
        match self {
            SeededType::Shares(val) => val,
            SeededType::Seed(seed, len, _) => {
                let mut rng = U::from_seed(seed);
                let mut shares = Vec::with_capacity(len);
                for _ in 0..len {
                    shares.push(F::rand(&mut rng));
                }
                shares
            }
        }
    }

    /// Returns the length of the share
    pub fn length(&self) -> usize {
        match self {
            SeededType::Shares(val) => val.len(),
            SeededType::Seed(_, len, _) => *len,
        }
    }
}

impl<F: PrimeField, U: Rng + SeedableRng + CryptoRng> SeededType<F, U>
where
    U::Seed: std::fmt::Debug + Clone + Serialize + for<'a> Deserialize<'a>,
{
    /// Expands the compressed share.
    pub fn expand(self) -> F {
        match self {
            SeededType::Shares(val) => val,
            SeededType::Seed(seed, len, _) => {
                assert_eq!(len, 1);
                let mut rng = U::from_seed(seed);
                F::rand(&mut rng)
            }
        }
    }
}

/// A type that represents a compressed replicated share. It consists of two compressed additive shares.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct ReplicatedSeedType<
    T: Clone + CanonicalSerialize + CanonicalDeserialize,
    U: Rng + SeedableRng + CryptoRng,
> where
    U::Seed: std::fmt::Debug + Clone + Serialize + for<'a> Deserialize<'a>,
{
    /// The first compressed additive share
    pub a: SeededType<T, U>,
    /// The second compressed additive share
    pub b: SeededType<T, U>,
}


/// Secret shares a ring element using replicated secret sharing and the provided random number generator. The ring element is split into three additive shares, where each party holds two. The outputs are of type [`Rep3RingShare`].
pub fn share_ring_element<T: IntRing2k, R: Rng + CryptoRng>(
    val: RingElement<T>,
    rng: &mut R,
) -> [Rep3RingShare<T>; 3]
where
    Standard: Distribution<T>,
{
    let a = rng.r#gen::<RingElement<T>>();
    let b = rng.r#gen::<RingElement<T>>();

    let c = val - a - b;
    let share1 = Rep3RingShare::new_ring(a, c);
    let share2 = Rep3RingShare::new_ring(b, a);
    let share3 = Rep3RingShare::new_ring(c, b);
    [share1, share2, share3]
}

/// Secret shares a vector of ring elements using replicated secret sharing and the provided random number generator. The ring elements are split into three additive shares each, where each party holds two. The outputs are of type [`Rep3RingShare`].
pub fn share_ring_elements<T: IntRing2k, R: Rng + CryptoRng>(
    vals: &[RingElement<T>],
    rng: &mut R,
) -> [Vec<Rep3RingShare<T>>; 3]
where
    Standard: Distribution<T>,
{
    let mut shares1 = Vec::with_capacity(vals.len());
    let mut shares2 = Vec::with_capacity(vals.len());
    let mut shares3 = Vec::with_capacity(vals.len());
    for val in vals {
        let [share1, share2, share3] = share_ring_element(val.to_owned(), rng);
        shares1.push(share1);
        shares2.push(share2);
        shares3.push(share3);
    }
    [shares1, shares2, shares3]
}

/// Secret shares a ring element using replicated secret sharing and the provided random number generator. The ring element is split into three binary shares, where each party holds two. The outputs are of type [`Rep3RingShare`].
pub fn share_ring_element_binary<T: IntRing2k, R: Rng + CryptoRng>(
    val: RingElement<T>,
    rng: &mut R,
) -> [Rep3RingShare<T>; 3]
where
    Standard: Distribution<T>,
{
    let a = rng.r#gen::<RingElement<T>>();
    let b = rng.r#gen::<RingElement<T>>();
    let c = val ^ a ^ b;
    let share1 = Rep3RingShare::new_ring(a, c);
    let share2 = Rep3RingShare::new_ring(b, a);
    let share3 = Rep3RingShare::new_ring(c, b);
    [share1, share2, share3]
}

/// Secret shares a vector of ring elements using replicated secret sharing and the provided random number generator. The ring elements are split into three binary shares each, where each party holds two. The outputs are of type [`Rep3RingShare`].
pub fn share_ring_elements_binary<T: IntRing2k, R: Rng + CryptoRng>(
    vals: &[RingElement<T>],
    rng: &mut R,
) -> [Vec<Rep3RingShare<T>>; 3]
where
    Standard: Distribution<T>,
{
    let mut shares1 = Vec::with_capacity(vals.len());
    let mut shares2 = Vec::with_capacity(vals.len());
    let mut shares3 = Vec::with_capacity(vals.len());
    for val in vals {
        let [share1, share2, share3] = share_ring_element_binary(val.to_owned(), rng);
        shares1.push(share1);
        shares2.push(share2);
        shares3.push(share3);
    }
    [shares1, shares2, shares3]
}

/// Reconstructs a ring element from its arithmetic replicated shares.
pub fn combine_ring_element<T: IntRing2k>(
    share1: Rep3RingShare<T>,
    share2: Rep3RingShare<T>,
    share3: Rep3RingShare<T>,
) -> RingElement<T> {
    share1.a + share2.a + share3.a
}

/// Reconstructs a vector of ring elements from its arithmetic replicated shares.
/// # Panics
/// Panics if the provided `Vec` sizes do not match.
pub fn combine_ring_elements<T: IntRing2k>(
    share1: &[Rep3RingShare<T>],
    share2: &[Rep3RingShare<T>],
    share3: &[Rep3RingShare<T>],
) -> Vec<RingElement<T>> {
    assert_eq!(share1.len(), share2.len());
    assert_eq!(share2.len(), share3.len());

    itertools::multizip((share1, share2, share3))
        .map(|(x1, x2, x3)| x1.a + x2.a + x3.a)
        .collect::<Vec<_>>()
}

/// Reconstructs a ring element from its binary replicated shares.
pub fn combine_ring_element_binary<T: IntRing2k>(
    share1: Rep3RingShare<T>,
    share2: Rep3RingShare<T>,
    share3: Rep3RingShare<T>,
) -> RingElement<T> {
    share1.a ^ share2.a ^ share3.a
}
/// Reconstructs a vector of ring elements from its binary replicated shares.
pub fn combine_ring_elements_binary<T: IntRing2k>(
    share1: &[Rep3RingShare<T>],
    share2: &[Rep3RingShare<T>],
    share3: &[Rep3RingShare<T>],
) -> Vec<RingElement<T>> {

    itertools::multizip((share1, share2, share3))
        .map(|(x1, x2, x3)| x1.a ^ x2.a ^ x3.a)
        .collect::<Vec<_>>()
}
