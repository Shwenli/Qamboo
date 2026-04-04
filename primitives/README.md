# Primitives Module

## Overview

The `primitives` module provides high-level cryptographic primitives built on top of the MPC protocols. These primitives form the building blocks for secure database operations, enabling complex computations like comparisons, permutations, shuffling, and arithmetic operations on secretly shared data.

## Core Primitives

### 1. Comparison (`compare.rs`)

Implements secure comparison operations using the Kogge-Stone adder for low-depth boolean circuits.

**Operations:**
```rust
// Unsigned greater-than-or-equal (constant on LHS)
pub fn unsigned_ge_const_lhs_many<T, N>(
    x: &RingElement<T>,      // Public constant
    y: &[Rep3RingShare<T>],  // Shared values
    net: &N,
    state: &mut Rep3State,
) -> Result<Vec<Rep3RingShare<Bit>>>  // Returns comparison bits

// Unsigned greater-than-or-equal (constant on RHS)
pub fn unsigned_ge_const_rhs_many<T, N>(...)

// Equality test
pub fn eq_many<T, N>(...)

// Bitwise AND for bit vectors
pub fn and_vec_bit_multithreads<T, N>(...)
```

**Algorithm:** Uses A2B conversion followed by Kogge-Stone subtraction circuit for O(log k) depth comparisons.

### 2. Permutation (`permute.rs`)

Generates and applies secret permutations for sorting and reordering operations.

**Key Functions:**
```rust
// Generate permutation from key bits
pub fn gen_perm_multithreads<T, N>(
    bits: &[Rep3RingShare<T>],
    order: bool,           // true = ASC, false = DESC
    bitsize: usize,
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<Vec<Rep3RingShare<PermRing>>>

// Apply permutation to data
pub fn apply_inv_multithreads<T, N>(
    perm: &[Rep3RingShare<PermRing>],
    input: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<Vec<Rep3RingShare<T>>>

// Apply inverse permutation
pub fn apply_perm_multithreads<T, N>(...)

// Compose two permutations
pub fn compose_perm_multithreads<T, N>(...)
```

**Algorithm:** Bitwise radix sort using Waksman networks for permutation generation.

### 3. Shuffling (`shuffle.rs`)

Implements secure shuffling of shared arrays using permutations.

```rust
pub fn shuffle_multithreads<T, N>(
    pi: &[Rep3RingShare<PermRing>],      // Permutation
    input: &[Rep3RingShare<T>],           // Data to shuffle
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<Vec<Rep3RingShare<T>>>
```

**Algorithm:** Party-local permutation application with correlated randomness for re-sharing.

### 4. Multiplexer (MUX) (`mux.rs`)

Conditional selection operations:

```rust
// If-then-else with public values
pub fn mux_if_then_public<T>(
    cond: &Rep3RingShare<Bit>,  // Condition bit
    then_val: &RingElement<T>,  // Value if true
    else_val: &RingElement<T>,  // Value if false
    state: &Rep3State,
) -> Result<Rep3RingShare<T>>

// If-then-else with shared values (vectorized)
pub fn mux_if_then_share_vec_multithreads<T, N>(...)
```

### 5. Transform (`transform.rs`)

Data transformation utilities:

```rust
// Bit decomposition
pub fn bit_decompose_many_multithreads<T, N>(...)

// Inject bit into larger type
pub fn inject_bit_multithreads<T, N>(...)

// Arithmetic-to-binary conversion (optimized)
pub fn a2b_many_multithreads<T, N>(...)
```

### 6. Division (`div.rs`)

Secure division operations using Goldschmidt iteration.

```rust
// Secret/shared division
pub fn div_many<T, N>(
    numerators: &[Rep3RingShare<T>],
    denominators: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<Vec<Rep3RingShare<T>>>

// Public denominator division
pub fn div_many_public_den<T, N>(...)
```

### 7. Multiplication (`mul.rs`)

Vectorized multiplication with optimized communication patterns.

```rust
pub fn mul_share_vec<T, N>(
    a: &[Rep3RingShare<T>],
    b: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<Vec<Rep3RingShare<T>>>
```

### 8. Kogge-Stone Adder (`kogge_stone_adder.rs`)

Low-depth binary adder/subtractor for comparison circuits:

```rust
// Subtraction with carry output
pub fn low_depth_binary_sub_from_const_with_carry_many<T, N>(...)
pub fn low_depth_binary_sub_by_const_with_carry_many<T, N>(...)
```

**Depth:** O(log k) for k-bit values, crucial for efficient comparison.

### 9. Utilities (`utils.rs`)

Helper functions:

```rust
// Prefix sum (sequential)
pub fn prefix_sum_sequential<T>(input: &[Rep3RingShare<T>]) -> Result<Vec<Rep3RingShare<T>>>
```

## Architecture

```text
primitives/
├── src/
│   ├── lib.rs                   # Module exports
│   ├── compare.rs               # Secure comparison
│   ├── permute.rs               # Permutation generation/application
│   ├── shuffle.rs               # Secure shuffling
│   ├── mux.rs                   # Multiplexer operations
│   ├── transform.rs             # Data transformations
│   ├── div.rs                   # Secure division
│   ├── mul.rs                   # Vectorized multiplication
│   ├── kogge_stone_adder.rs     # Low-depth adder
│   └── utils.rs                 # Utility functions
```

## Performance Characteristics

| Primitive | Communication | Rounds | Parallelizable |
|-----------|--------------|--------|----------------|
| Comparison (GE) | O(k) bits | O(log k) | Yes |
| Permutation Gen | O(n log n) | O(log n) | Yes |
| Shuffle | O(n) | 2 | Yes |
| MUX | O(1) | 1 | Yes |
| Division | O(iterations × k) | O(log k) | Yes |

## Usage Example

```rust
use primitives::{
    compare::unsigned_ge_const_lhs_many_multithreads,
    permute::{gen_perm_multithreads, apply_inv_multithreads},
    shuffle::shuffle_multithreads,
};

// Secure comparison: check if values >= threshold
let threshold = RingElement(100u64);
let comparison_bits = unsigned_ge_const_lhs_many_multithreads(
    &threshold, &shared_values, nets, states
)?;

// Generate sorting permutation
let perm = gen_perm_multithreads(&key_bits, true, 64, nets, states)?;

// Apply permutation to sort data
let sorted_data = apply_inv_multithreads(&perm, &data, nets, states)?;

// Shuffle data
let shuffled = shuffle_multithreads(&perm, &sorted_data, nets, states)?;
```

## Implementation Notes

1. **Multi-threading**: All primitives support multi-threaded execution using Rayon for data-parallel operations.

2. **Communication Optimization**: Batched operations reduce round complexity for vectorized inputs.

3. **Memory Efficiency**: In-place operations where possible to minimize allocations.

4. **Zero-Copy Integration**: Leverages `algebra` module's zero-copy serialization for network operations.
