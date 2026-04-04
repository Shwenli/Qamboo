# Random Module

## Overview

The `random` module provides secure correlated randomness generation for the Qamboo MPC framework. It implements PRF (Pseudo-Random Function) based randomness expansion that enables parties to generate shared random values without communication, dramatically reducing the communication overhead of MPC protocols.

## Core Concept: Correlated Randomness

In replicated secret sharing, parties can generate correlated randomness by sharing PRF keys during setup. After initialization, parties can locally expand these keys to produce:

- **Additive shares**: Values that sum to a random secret
- **Binary shares**: Values that XOR to a random secret
- **Bit decomposition shares**: Pre-processed random bits for A2B conversion

This eliminates the need for online communication to generate random values.

## Architecture

```text
random/
├── src/
│   ├── lib.rs              # Module exports and MpcState trait
│   ├── serde_compat.rs     # Serialization utilities
│   └── rep3/
│       ├── rep3rng.rs      # Core REP3 randomness generation
│       └── rep3rng_rayon.rs # Multi-threaded randomness
```

## Core Components

### 1. Rep3State (`rep3.rs`)

The main protocol state that holds all correlated randomness generators:

```rust
pub struct Rep3State {
    pub id: PartyID,                    // This party's ID
    pub rngs: Rep3CorrelatedRng,        // Correlated randomness generators
    pub rng: ChaCha12Rng,               // Local CSPRNG
}

impl Rep3State {
    /// Initialize from network (performs key exchange)
    pub fn new<N: Network>(net: &N) -> Result<Self>
    
    /// Fork state for multi-threading
    pub fn fork(&mut self, n: usize) -> Result<Vec<Self>>
}
```

### 2. Rep3CorrelatedRng (`rep3rng.rs`)

Container for all correlated randomness generators:

```rust
pub struct Rep3CorrelatedRng {
    pub rand: Rep3Rand,                      // Basic additive randomness
    pub bit_comp1: Rep3RandBitComp,          // Bit compression generator 1
    pub bit_comp2: Rep3RandBitComp,          // Bit compression generator 2
}
```

### 3. Rep3Rand (`rep3rng.rs`)

Generates additive and binary correlated randomness:

```rust
pub struct Rep3Rand {
    seed1: [u8; SEED_SIZE],  // Shared with party i+1
    seed2: [u8; SEED_SIZE],  // Shared with party i-1
}

impl Rep3Rand {
    /// Generate random elements for this party's share
    pub fn random_elements_vec<T>(&self, len: usize) -> Vec<RingElement<T>>
    
    /// Generate random binary shares
    pub fn random_binary_shares_vec<T>(&self, len: usize) -> Vec<Rep3RingShare<T>>
    
    /// Generate random seeds for sub-protocols
    pub fn random_seeds(&mut self) -> ([u8; SEED_SIZE], [u8; SEED_SIZE])
}
```

### 4. Rep3RandBitComp (`rep3rng.rs`)

Pre-computed randomness for efficient bit decomposition (A2B conversion):

```rust
pub struct Rep3RandBitComp {
    key1: [u8; SEED_SIZE],
    key2: [u8; SEED_SIZE],
    key3: Option<[u8; SEED_SIZE]>,  // Some parties have 3 keys
}
```

This generates the correlated randomness needed for the bit-decomposition protocol, which is the most expensive step in comparison operations.

## Key Setup Protocol

During initialization, parties perform a key exchange to establish shared PRF keys:

```
Party 0:                    Party 1:                  Party 2:
   |                          |                         |
   |  gen seed_a, seed_c      |  gen seed_b, seed_a     |  gen seed_c, seed_b
   |                          |                         |
   |<---------seed_a---------|--------seed_a---------> |
   |--------seed_c---------> |                         |
   |                          |<--------seed_c---------|
   |                          |                         |
   |  keys: (a,c)             |  keys: (b,a)            |  keys: (c,b)
```

```rust
impl Rep3State {
    pub fn setup_prf<N: Network, R: Rng + CryptoRng>(
        net: &N,
        rng: &mut R,
    ) -> Result<Rep3Rand> {
        let seed1: [u8; SEED_SIZE] = rng.gen();
        let seed2: [u8; SEED_SIZE] = net.reshare(seed1)?;
        Ok(Rep3Rand::new(seed1, seed2))
    }
}
```

## Multi-Threading Support (`rep3rng_rayon.rs`)

The module provides multi-threaded randomness generation using Rayon:

```rust
pub fn random_elements_vec_multithreads<T>(
    states: &mut [&mut Rep3State],
    len: usize,
) -> Vec<RingElement<T>>

pub fn random_binary_shares_vec_multithreads<T>(
    states: &mut [&mut Rep3State],
    len: usize,
) -> Vec<Rep3RingShare<T>>
```

These functions distribute work across multiple threads, each with its own forked state, for parallel randomness generation.

## Usage Examples

### Basic Randomness Generation

```rust
use random::rep3::Rep3State;

// Initialize state (performs key exchange)
let mut state = Rep3State::new(&net)?;

// Generate random shares (no network communication!)
let random_values: Vec<RingElement<u64>> = 
    state.rngs.rand.random_elements_vec(1000);

// Generate random binary shares
let random_bits: Vec<Rep3RingShare<u64>> = 
    state.rngs.rand.random_binary_shares_vec(1000);
```

### Multi-Threaded Randomness

```rust
use random::rep3::rep3rng_rayon::*;

// Fork state for 4 threads
let mut forked_states = state.fork(4)?;

// Generate randomness in parallel
let refs: Vec<&mut Rep3State> = forked_states.iter_mut().collect();
let random_values = random_elements_vec_multithreads::<u64>(
    &refs, 
    10000
);
```

### Integration with Protocols

```rust
use protocols::rep3_ring::arithmetic;

// Secure multiplication uses correlated randomness for re-sharing
let product = arithmetic::mul(&x, &y, net, &mut state)?;
// The 'mul' function internally uses state.rngs.rand for random masks
```

## Performance Characteristics

| Operation | Communication | Computation | Notes |
|-----------|--------------|-------------|-------|
| Setup | O(1) seeds | Key exchange | One-time cost |
| Random element | 0 | PRF expansion | ~ns per element |
| Binary share | 0 | PRF expansion | ~ns per element |
| Bit comp random | 0 | PRF expansion | Pre-computed for A2B |

## Security Considerations

1. **PRF Security**: Based on ChaCha12 stream cipher, providing 128-bit security
2. **Key Confidentiality**: Each key is known to exactly 2 parties, maintaining the sharing invariant
3. **Forward Secrecy**: New seeds generated for each protocol execution
4. **Fork Safety**: State forking uses independent key derivation to prevent key reuse

## Implementation Details

- **ChaCha12Rng**: Uses ChaCha12 for cryptographically secure pseudo-random generation
- **Zero Allocation**: Randomness expansion writes directly to pre-allocated vectors
- **Deterministic**: Same seed sequence produces identical randomness across parties (required for correctness)
