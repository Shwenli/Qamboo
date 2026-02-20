# Protocols Module

## Overview

The `protocols` module implements the core Multi-Party Computation (MPC) protocols for the Qamboo framework. Currently, it provides semi-honest secure 3-party replicated secret sharing (REP3) over rings Z_{2^k}.

## Replicated Secret Sharing (REP3)

REP3 is a 3-party secret sharing scheme where each party holds two additive shares of the secret. This design enables efficient computation with minimal communication overhead.

### Share Structure

For a secret value `x`, the sharing is:

```
x = a + b + c  (arithmetic sharing)
x = a ⊕ b ⊕ c  (binary sharing)
```

Each party holds:
- **Party 0**: (a, c)
- **Party 1**: (b, a)  
- **Party 2**: (c, b)

```rust
pub struct Rep3RingShare<T: IntRing2k> {
    pub a: RingElement<T>,  // First share component
    pub b: RingElement<T>,  // Second share component
}
```

## Module Structure

```
protocols/
├── src/
│   ├── lib.rs                 # Module exports and MpcState trait
│   ├── serde_compat.rs        # Serialization compatibility
│   ├── protocols.rs           # Protocol module interface
│   └── protocols/
│       └── rep3_ring/
│           ├── mod.rs         # Share/combine operations
│           ├── arithmetic.rs  # Arithmetic operations (+, -, *, public)
│           ├── binary.rs      # Binary operations (⊕, ∧, NOT)
│           ├── conversion.rs  # A2B/B2A conversions
│           └── detail.rs      # Implementation details
```

## Core Operations

### 1. Arithmetic Operations (`arithmetic.rs`)

Implements secure arithmetic over shared values:

```rust
// Addition (local operation)
pub fn add<T>(x: Rep3RingShare<T>, y: Rep3RingShare<T>) -> Rep3RingShare<T>

// Subtraction (local operation)
pub fn sub<T>(x: Rep3RingShare<T>, y: Rep3RingShare<T>) -> Rep3RingShare<T>

// Multiplication (requires 1 round of communication)
pub fn mul<T, N: Network>(
    x: &Rep3RingShare<T>,
    y: &Rep3RingShare<T>,
    net: &N,
    state: &mut Rep3State,
) -> Result<Rep3RingShare<T>>

// Public operations (mixed secret/public)
pub fn add_public<T>(x: Rep3RingShare<T>, public: RingElement<T>, id: PartyID) -> Rep3RingShare<T>
pub fn mul_public<T>(x: Rep3RingShare<T>, public: RingElement<T>) -> Rep3RingShare<T>
```

### 2. Binary Operations (`binary.rs`)

Implements secure boolean circuits:

```rust
// XOR (local operation)
pub fn xor<T>(x: Rep3RingShare<T>, y: Rep3RingShare<T>) -> Rep3RingShare<T>

// AND (requires 1 round of communication)
pub fn and<T, N: Network>(...)

// NOT (local operation)
pub fn not<T>(x: Rep3RingShare<T>) -> Rep3RingShare<T>
```

### 3. Arithmetic-to-Binary Conversion (`conversion.rs`)

Converts between arithmetic and binary representations:

```rust
// A2B: Arithmetic sharing → Binary sharing
pub fn a2b_many<T, N>(
    x: &[Rep3RingShare<T>],
    net: &N,
    state: &mut Rep3State,
) -> Result<Vec<Rep3RingShare<T>>>

// B2A: Binary sharing → Arithmetic sharing
pub fn b2a_many<T, N>(...)
```

These conversions are essential for comparison operations and mixed-mode circuits.

## Security Properties

### Semi-Honest Security

The REP3 protocol is secure against a single semi-honest adversary:

- **Privacy**: No single party can learn the secret value
- **Correctness**: Protocol output is correct if all parties follow the protocol
- **Collusion Resistance**: Tolerates collusion of up to 1 party

### Communication Complexity

| Operation | Communication (per element) | Rounds |
|-----------|---------------------------|--------|
| Add/Sub   | 0                         | 0      |
| Mul       | 2 field elements          | 1      |
| A2B/B2A   | O(k) bits                 | O(log k) |

## Usage Example

```rust
use protocols::protocols::rep3_ring::{
    arithmetic, binary, conversion,
    Rep3RingShare, share_ring_element, combine_ring_element,
};
use random::rep3::Rep3State;
use net::Network;

// Share a secret value
let secret = RingElement(42u64);
let [share0, share1, share2] = share_ring_element(secret, &mut rng);

// Party 0 computes:
let sum = arithmetic::add(share0, other_share0);

// Secure multiplication (requires network)
let product = arithmetic::mul(&share0, &other_share0, net, state)?;

// Reconstruct the result
let result = combine_ring_element(share0, share1, share2);
```

## Multi-Threading Support

All operations support multi-threaded execution for improved performance:

```rust
// Multi-threaded vector operations
pub fn add_vec_many_multithreads<T>(...)
pub fn mul_vec_many_multithreads<T, N>(...)
```

## References

- [Astra: High Throughput 3PC over Rings](https://eprint.iacr.org/2018/403.pdf) - Original REP3 protocol
- [Shamir Secret Sharing](https://www.iacr.org/archive/crypto2007/46220565/46220565.pdf) - Alternative sharing scheme
