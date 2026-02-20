# Algebra Module

## Overview

The `algebra` module provides the foundational algebraic structures for the Qamboo MPC framework. It implements ring theory abstractions optimized for secure multi-party computation, with a focus on efficient arithmetic operations over Z_{2^k} (rings of integers modulo 2^k).

## Core Components

### 1. RingElement<T> (`ring/ring_impl.rs`)

A transparent wrapper type for all datatypes implementing the `IntRing2k` trait, enabling explicit wrapping arithmetic operations.

**Key Features:**
- `#[repr(transparent)]` layout for zero-cost abstraction
- Zero-copy slice conversion via `slice_as_bytes()` and `bytes_to_slice()`
- Bit manipulation operations: `get_bit()`, `bits()`
- Safe transmutation between `Vec<T>` and `Vec<RingElement<T>>`

```rust
use algebra::ring::{ring_impl::RingElement, int_ring::IntRing2k};

// Zero-copy conversion for network transmission
let elements: Vec<RingElement<u64>> = vec![...];
let bytes = RingElement::slice_as_bytes(&elements);  // No allocation
```

### 2. IntRing2k Trait (`ring/int_ring.rs`)

Defines the interface for types that can be elements of a ring Z_{2^k}.

**Supported Types:**
- `u8`, `u16`, `u32`, `u64`, `u128` - Standard integer types
- `Bit` - Single-bit type for binary operations
- Custom `Uint` types via `ruint` crate

**Trait Requirements:**
```rust
pub trait IntRing2k: 
    WrappingAdd + WrappingSub + WrappingMul + WrappingNeg +
    BitXor + BitAnd + BitOr + Shl<usize> + Shr<usize> +
    Zero + One + Copy + Send + Sync + 'static
{
    const K: usize;      // Number of bits
    const BYTES: usize;  // Storage size in bytes
    
    fn from_reader<R: Read>(reader: R) -> io::Result<Self>;
    fn write<W: Write>(&self, writer: W) -> io::Result<()>;
    fn bits(&self) -> usize;
    fn cast_to_biguint(&self) -> BigUint;
    fn cast_from_biguint(biguint: &BigUint) -> Self;
}
```

### 3. Bit Type (`ring/bit.rs`)

A specialized single-bit type optimized for binary secret sharing and boolean circuits.

## Architecture

```
algebra/
├── src/
│   ├── lib.rs              # Module exports
│   ├── ring.rs             # Ring module interface
│   └── ring/
│       ├── ring_impl.rs    # RingElement implementation
│       ├── int_ring.rs     # IntRing2k trait definition
│       └── bit.rs          # Single-bit type
```

## Performance Considerations

### Zero-Copy Serialization

The `RingElement` type leverages Rust's type system for zero-copy serialization:

```rust
// On little-endian platforms, this is a simple pointer cast
#[inline]
pub fn slice_as_bytes(slice: &[Self]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            std::mem::size_of_val(slice),
        )
    }
}
```

### Wrapping Arithmetic

All operations explicitly use wrapping arithmetic to match the mathematical semantics of Z_{2^k} rings, avoiding undefined behavior from overflow.

## Usage Examples

```rust
use algebra::ring::{ring_impl::RingElement, int_ring::IntRing2k};

// Create ring elements
let a = RingElement(42u64);
let b = RingElement(100u64);

// Wrapping arithmetic
let c = a + b;  // Automatic wrapping at 2^64

// Bit manipulation
let bit_3 = a.get_bit(3);

// Zero-copy network transmission
let data: Vec<RingElement<u64>> = vec![a, b, c];
let bytes = RingElement::slice_as_bytes(&data);
socket.send(bytes)?;
```

## Dependencies

- `ark-serialize` - Canonical serialization for algebraic types
- `num-traits` - Numeric trait abstractions
- `num-bigint` - Big integer support for casting
- `ruint` - Large unsigned integer types
- `rand` - Random number generation
- `serde` - Serialization framework

## Design Rationale

1. **Type Safety**: Using `RingElement<T>` wrapper prevents accidental mixing of ring and non-ring arithmetic.

2. **Zero-Copy Performance**: The transparent representation enables efficient network communication without per-element serialization overhead.

3. **Generic Design**: The `IntRing2k` trait allows the MPC protocols to work with any suitable ring element type.

4. **Cloud-Native**: Optimized for scenarios where data is frequently serialized to the network.
