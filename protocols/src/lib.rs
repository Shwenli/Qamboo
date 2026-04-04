#![doc = include_str!("../README.md")]

#![warn(missing_docs)]



pub mod rep3_ring;

/// Re-exports
pub type RngType = rand_chacha::ChaCha12Rng;

/// The size of the seed for the random number generator used in the protocols.
pub const SEED_SIZE: usize = std::mem::size_of::<<RngType as rand::SeedableRng>::Seed>();



/// Trait for MPC protocol states
pub trait MpcState: Sized {
    /// The type of a party id
    type PartyID: Clone + Copy + Send + Sync;

    /// Get the id of the party
    fn id(&self) -> Self::PartyID;

    // TODO maybe use fork() and fork_with(n: usize)
    /// Crate a new state from self
    fn fork(&mut self, n: usize) -> eyre::Result<Self>;
}

// This implements fork for a dummy state that is used for plain variants of MPC protocols
impl MpcState for () {
    type PartyID = usize;

    fn id(&self) -> Self::PartyID {
        0
    }

    fn fork(&mut self, _n: usize) -> eyre::Result<Self> {
        Ok(())
    }
}
