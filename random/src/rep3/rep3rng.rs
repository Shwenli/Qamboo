//! Rep3 RNGs
//!
//! This module contains implementations of rep3 rngs
use itertools::Itertools;
use communication::rep3::id::PartyID;
use std::mem::MaybeUninit;
use rand::{
    Rng, SeedableRng, distributions::Standard, prelude::Distribution, seq::SliceRandom,
};
use crate::rep3::{RngType, SEED_SIZE};


#[derive(Debug)]
/// A correlated rng for rep3
pub struct Rep3CorrelatedRng {
    /// Rep3 rng with this party's rng and the prev party's rng
    pub rand: Rep3Rand,
    pub bitcomp1: Rep3RandBitComp,
    pub bitcomp2: Rep3RandBitComp,
}

impl Rep3CorrelatedRng {
    /// Construct a new [`Rep3CorrelatedRng`]
    pub fn new(rand: Rep3Rand, bitcomp1: Rep3RandBitComp, bitcomp2: Rep3RandBitComp) -> Self {
        Self {
            rand,
            bitcomp1,
            bitcomp2,
        }
    }

    /// Create a fork of the current rng
    pub fn fork(&mut self) -> Self {
        let rand = self.rand.fork();
        let bitcomp1 = self.bitcomp1.fork();
        let bitcomp2 = self.bitcomp2.fork();
        Self {
            rand,
            bitcomp1,
            bitcomp2,
        }
    }

    /// Generate a value that is equal on all three parties
    pub fn generate_shared<T>(&mut self, id: PartyID) -> T
    where
        Standard: Distribution<T>,
    {
        match id {
            PartyID::ID0 => self.bitcomp1.rng2.r#gen(),
            PartyID::ID1 => self.bitcomp1.rng2.r#gen(),
            PartyID::ID2 => self.bitcomp1.rng1.r#gen(),
        }
    }


    
}

#[derive(Debug)]
/// Rep3 rng with this party's rng and the prev party's rng
pub struct Rep3Rand {
    rng1: RngType,
    rng2: RngType,
}

impl Rep3Rand {
    /// Construct a new [`Rep3Rand`]
    pub fn new(seed1: [u8; SEED_SIZE], seed2: [u8; SEED_SIZE]) -> Self {
        let rng1 = RngType::from_seed(seed1);
        let rng2 = RngType::from_seed(seed2);
        Self { rng1, rng2 }
    }

    /// Create a fork of this rng
    pub fn fork(&mut self) -> Self {
        let (seed1, seed2) = self.random_seeds();
        Self::new(seed1, seed2)
    }

    /// Generate a masking element
    pub fn masking_element<T>(&mut self) -> T
    where
        Standard: Distribution<T>,
        T: std::ops::Sub<Output = T>,
    {
        let (a, b) = self.random_elements::<T>();
        a - b
    }

    // TODO do not collect the values
    /// Generate a vector of masking elements
    pub fn masking_elements_vec<T>(&mut self, len: usize) -> Vec<T>
    where
        Standard: Distribution<T>,
        T: Send + Sync + std::ops::Sub<Output = T>,
    {
        let (a, b) = rayon::join(
            || (0..len).map(|_| self.rng1.r#gen()).collect::<Vec<_>>(),
            || (0..len).map(|_| self.rng2.r#gen()).collect::<Vec<_>>(),
        );
        a.into_iter()
        .zip_eq(b.into_iter())
        .map(|(a, b)| a - b)
        .collect()
    }

    /// Generate two random elements
    pub fn random_elements<T>(&mut self) -> (T, T)
    where
        Standard: Distribution<T>,
    {
        let a = self.rng1.r#gen();
        let b = self.rng2.r#gen();
        (a, b)
    }
    
    /// Generate a vector of random elements
    pub fn random_elements_vec<T>(&mut self, len: usize) -> (Vec<T>, Vec<T>)
    where
        Standard: Distribution<T>,
        T: Send + Sync,
    {
        let (a, b) = rayon::join(
            || (0..len).map(|_| self.rng1.r#gen()).collect::<Vec<_>>(),
            || (0..len).map(|_| self.rng2.r#gen()).collect::<Vec<_>>(),
        );
        (a, b)
    }

    pub fn random_elements_vec_test<T>(&mut self, len: usize) -> (Vec<T>, Vec<T>)
    where
        Standard: Distribution<T>,
        T: Send + Sync,
    {
        let a = (0..len).map(|_| self.rng1.r#gen()).collect::<Vec<_>>();
        let b = (0..len).map(|_| self.rng2.r#gen()).collect::<Vec<_>>();
        (a, b)
    }

    pub fn random_elements_into_uninit<T>(
        &mut self,
        a: &mut [MaybeUninit<T>],
        b: &mut [MaybeUninit<T>],
    ) where
        Standard: Distribution<T>,
    {
        assert_eq!(a.len(), b.len());
        for slot in a.iter_mut() {
            slot.write(self.rng1.r#gen());
        }
        for slot in b.iter_mut() {
            slot.write(self.rng2.r#gen());
        }
    }

    

    /// Generate a random `T` from rng1
    pub fn random_element_rng1<T>(&mut self) -> T
    where
        Standard: Distribution<T>,
    {
        self.rng1.r#gen()
    }

    /// Generate a vector of random `T` from rng1
    pub fn random_elements_rng1<T>(&mut self, len: usize) -> Vec<T>
    where
        Standard: Distribution<T>,
    {
        (0..len).map(|_| self.rng1.r#gen()).collect()
    }

    /// Generate a random `T` from rng1
    pub fn random_element_rng2<T>(&mut self) -> T
    where
        Standard: Distribution<T>,
    {
        self.rng2.r#gen()
    }

    /// Generate a vector of random `T` from rng2
    pub fn random_elements_rng2<T>(&mut self, len: usize) -> Vec<T>
    where
        Standard: Distribution<T>,
    {
        (0..len).map(|_| self.rng2.r#gen()).collect()
    }

    /// Generate a seed from each rng
    pub fn random_seeds(&mut self) -> ([u8; SEED_SIZE], [u8; SEED_SIZE]) {
        let seed1 = self.rng1.r#gen();
        let seed2 = self.rng2.r#gen();
        (seed1, seed2)
    }

    /// Generate a random shared permutation
    pub fn random_perm<T: Clone + Send>(&mut self, input: Vec<T>) -> (Vec<T>, Vec<T>) {
        let mut a = input.to_owned();
        let mut b = input;
        rayon::join(
            || a.shuffle(&mut self.rng1),
            || b.shuffle(&mut self.rng2),
        );
        (a, b)
    }
}

/// This struct is responsible for creating random shares for the Binary to Arithmetic conversion. The approach is the following: for a final sharing x = x1 + x2 + x3, we want to have random values x2, x3 and subtract these from the original value x using a binary circuit to get the share x1. Hence, we need to sample random x2 and x3 and share them amongst the parties. One RandBitComp struct is responsible for either sampling x2 or x3. For sampling x2, parties 1 and 2 will get x2 in plain (since this is the final share of x), so they need to have a PRF key from all parties. party 3, however, will not get x2 in plain and must thus only be able to sample its shares of x2, requiring two PRF keys.
#[derive(Debug)]
pub struct Rep3RandBitComp {
    rng1: RngType,
    rng2: RngType,
    rng3: Option<RngType>,
}

impl Rep3RandBitComp {
    /// Contruct a new [`Rep3RandBitComp`] w rngs
    pub fn new_2keys(rng1: [u8; SEED_SIZE], rng2: [u8; SEED_SIZE]) -> Self {
        Self {
            rng1: RngType::from_seed(rng1),
            rng2: RngType::from_seed(rng2),
            rng3: None,
        }
    }

    /// Contruct a new [`Rep3RandBitComp`] with 3 rngs
    pub fn new_3keys(
        rng1: [u8; SEED_SIZE],
        rng2: [u8; SEED_SIZE],
        rng3: [u8; SEED_SIZE],
    ) -> Self {
        Self {
            rng1: RngType::from_seed(rng1),
            rng2: RngType::from_seed(rng2),
            rng3: Some(RngType::from_seed(rng3)),
        }
    }

    /// Generate three random field elements
    pub fn random_elements_3keys<T>(&mut self) -> (T, T, T)
    where
        Standard: Distribution<T>,
    {
        let a = self.rng1.r#gen();
        let b = self.rng2.r#gen();
        let c = if let Some(rng3) = &mut self.rng3 {
            rng3.r#gen()
        } else {
            unreachable!()
        };
        (a, b, c)
    }

    pub fn random_elements_3keys_vec<T>(&mut self, len: usize) -> (Vec<T>, Vec<T>, Vec<T>)
    where
        Standard: Distribution<T>,
        T: Send + Sync,
    {
        let (a, b) = rayon::join(
            || (0..len).map(|_| self.rng1.r#gen()).collect::<Vec<_>>(),
            || (0..len).map(|_| self.rng2.r#gen()).collect::<Vec<_>>(),
        );
        let c = if let Some(rng3) = &mut self.rng3 {
            (0..len).map(|_| rng3.r#gen()).collect::<Vec<_>>()
        } else {
            unreachable!()
        };
        (a, b, c)
    }



    /// Create a fork of this rng
    pub fn fork(&mut self) -> Self {
        let rng1 = RngType::from_seed(self.rng1.r#gen());
        let rng2 = RngType::from_seed(self.rng2.r#gen());
        let rng3 = self
            .rng3
            .as_mut()
            .map(|rng| RngType::from_seed(rng.r#gen()));
        Self { rng1, rng2, rng3 }
    }
}
