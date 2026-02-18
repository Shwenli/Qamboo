pub mod rep3rng;
pub mod rep3rng_rayon;

use std::marker::PhantomData;

use communication::rep3::id::PartyID;
use communication::rep3::net_impl::Rep3NetworkImpl;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use net::Network;
use rand::{CryptoRng, Rng, SeedableRng};
use rep3rng::{Rep3CorrelatedRng, Rep3Rand, Rep3RandBitComp};
use serde::{Deserialize, Serialize};
use crate::MpcState;
use crate::serde_compat::{ark_de, ark_se};

pub(crate) type RngType = rand_chacha::ChaCha12Rng;
pub(crate) const SEED_SIZE: usize = std::mem::size_of::<<RngType as rand::SeedableRng>::Seed>();

/// Trait for MPC protocol states


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

    pub fn setup_prf<N: Network, R: Rng + CryptoRng>(net: &N, rng: &mut R) -> eyre::Result<Rep3Rand> {
        let seed1: [u8; SEED_SIZE] = rng.r#gen();
        let seed2: [u8; SEED_SIZE] = net.reshare(seed1)?;

        Ok(Rep3Rand::new(seed1, seed2))
    }

    pub fn setup_bitcomp<N: Network>(
        net: &N,
        rands: &mut Rep3Rand,
    ) -> eyre::Result<(Rep3RandBitComp, Rep3RandBitComp)> {
        let id = PartyID::try_from(net.id())?;
        let (k1a, k1c) = rands.random_seeds();
        let (k2a, k2c) = rands.random_seeds();

        match id {
            PartyID::ID0 => {
                let k2b: [u8; SEED_SIZE] =
                    net.send_and_recv(PartyID::ID1, k1c, PartyID::ID2)?;
                let bitcomp1 = Rep3RandBitComp::new_2keys(k1a, k1c);
                let bitcomp2 = Rep3RandBitComp::new_3keys(k2a, k2b, k2c);
                Ok((bitcomp1, bitcomp2))
            }
            PartyID::ID1 => {
                net.send_next((k1c, k2c))?;
                let k1b: [u8; SEED_SIZE] = net.recv_prev()?;
                let bitcomp1 = Rep3RandBitComp::new_3keys(k1a, k1b, k1c);
                let bitcomp2 = Rep3RandBitComp::new_2keys(k2a, k2c);
                Ok((bitcomp1, bitcomp2))
            }
            PartyID::ID2 => {
                net.send_next(k2c)?;
                let (k1b, k2b): ([u8; SEED_SIZE], [u8; SEED_SIZE]) =
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
