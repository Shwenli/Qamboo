# Communication Module

## Overview

The `communication` module provides optimized communication primitives for the 3-party replicated secret sharing protocol (REP3). It implements zero-copy serialization and efficient network communication patterns tailored for MPC workloads.

## Naming Convention

The module follows a strict naming convention for different serialization strategies:

| Suffix | Method | Description | Performance |
|--------|--------|-------------|-------------|
| `_zc` | Zero-Copy | Transmute slices to raw bytes via pointer casting | ~0 overhead |
| `_fast` | Fixed-Size | Exact buffer allocation, 1-pass serialization | Fast |
| (none) | Default | Heuristic allocation via ark-serialize | Standard |

**Performance Hierarchy:** `_zc` > `_fast` > default

## Core Components

### 1. Zero-Copy Communication (`rep3/zc_comm.rs`)

Specialized zero-copy methods for `RingElement<T>` and `Rep3RingShare<T>` types.

**Key Insight:** On little-endian platforms, the in-memory layout of `RingElement<T>` matches the wire format, enabling simple pointer casts.

```rust
// Zero-copy send
pub fn send_next_many_zc<T, N>(
    net: &N,
    data: &[RingElement<T>],
) -> Result<()>

// Zero-copy receive
pub fn recv_prev_many_zc<T, N>(
    net: &N,
    len: usize,
) -> Result<Vec<RingElement<T>>>
```

**Safety Requirements:**
- Target must be little-endian (enforced at compile time)
- Types must be `#[repr(transparent)]` over primitive integers
- No padding bytes in the type layout

### 2. Network Implementation (`rep3/net_impl.rs`)

Standard serialized communication for generic types:

```rust
// Send with serialization
pub fn send_next_many<T, N>(
    net: &N,
    data: &[T],
) -> Result<()>
where T: CanonicalSerialize

// Receive with deserialization
pub fn recv_prev_many<T, N>(
    net: &N,
    len: usize,
) -> Result<Vec<T>>
where T: CanonicalDeserialize
```

### 3. Multi-Network Implementation (`rep3/multinet_impl.rs`)

Multi-threaded communication supporting concurrent operations across multiple network instances:

```rust
// Reshare with multi-threading support
pub fn reshare_many_multinet<T, N>(
    nets: &[&N],
    states: &mut [&mut Rep3State],
    data: &[Rep3RingShare<T>],
) -> Result<Vec<Rep3RingShare<T>>>

// Send to next party (multi-threaded)
pub fn send_next_many_multinet<T, N>(...)

// Receive from previous party (multi-threaded)
pub fn recv_prev_many_multinet<T, N>(...)
```

### 4. Zero-Copy Network Implementation (`rep3/zc_net_impl.rs`)

Combines zero-copy serialization with multi-network support:

```rust
pub fn send_next_many_zc_multinet<T, N>(...)
pub fn recv_prev_many_zc_multinet<T, N>(...)
```

### 5. Party Identification (`rep3/id.rs`)

Strongly-typed party identifiers:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyID {
    ID0,
    ID1,
    ID2,
}

impl PartyID {
    pub fn next(self) -> Self;  // Next party in ring
    pub fn prev(self) -> Self;  // Previous party in ring
}
```

## Architecture

```
communication/
├── src/
│   ├── lib.rs                   # Module exports and naming convention docs
│   ├── task.rs                  # Async task utilities
│   └── rep3/
│       ├── id.rs                # PartyID definition
│       ├── net_impl.rs          # Standard network implementation
│       ├── multinet_impl.rs     # Multi-threaded network operations
│       ├── zc_comm.rs           # Zero-copy communication primitives
│       └── zc_net_impl.rs       # Zero-copy with multi-network
```

## Communication Patterns

### Replicated Secret Sharing Reshare

The core communication primitive for REP3 multiplication:

```
Party i computes: z_i = x_i * y_i + randomness
Party i sends to Party i+1: share component
Party i receives from Party i-1: share component
```

```rust
pub fn reshare_many<T, N>(
    net: &N,
    state: &mut Rep3State,
    data: &[Rep3RingShare<T>],
) -> Result<Vec<Rep3RingShare<T>>> {
    // Generate random mask
    let mask = state.rngs.rand.random_elements_vec(...);
    
    // Compute masked values locally
    let masked: Vec<_> = data.iter()
        .zip(mask.iter())
        .map(|(d, m)| d.a + d.b - m)
        .collect();
    
    // Send to next party
    net.send(net.next_id(), &serialize(&masked))?;
    
    // Receive from previous party
    let received = net.recv(net.prev_id())?;
    
    // Reconstruct shares
    Ok(zip(mask, received).map(|(m, r)| Rep3RingShare::new(m, r)).collect())
}
```

## Performance Optimization

### Zero-Copy Serialization

```rust
// Before: Per-element serialization (slow)
for elem in data {
    let bytes = ark_serialize::serialize(elem)?;
    socket.send(&bytes)?;
}

// After: Single pointer cast (fast)
let bytes = RingElement::slice_as_bytes(data);  // O(1)
socket.send(bytes)?;  // Single syscall
```

### Vectored I/O

Multi-network implementation uses vectored sends to reduce system calls:

```rust
// Combines multiple sends into single operation
pub fn send_batch<T, N>(net: &N, batches: &[&[T]]) -> Result<()> {
    // Uses write_vectored for efficiency
}
```

## Usage Example

```rust
use communication::rep3::{
    net_impl::*,
    multinet_impl::*,
    zc_net_impl::*,
    id::PartyID,
};

// Zero-copy communication
let shares: Vec<Rep3RingShare<u64>> = ...;
send_next_many_zc(net, &shares)?;
let received = recv_prev_many_zc::<u64, _>(net, len)?;

// Multi-threaded resharing
let reshared = reshare_many_multinet(nets, states, &shares)?;

// Check party ID
if state.id == PartyID::ID0 {
    // Party 0 specific logic
}
```

## Implementation Notes

1. **Endianness Safety**: Zero-copy operations compile-fail on big-endian platforms to prevent silent corruption.

2. **Buffer Management**: Write buffering is handled at the network layer (`net` module), not here.

3. **Error Handling**: Uses `eyre` for ergonomic error propagation with context.

4. **Thread Safety**: All operations require `&N` where `N: Network + Send + Sync`.

## Integration with Other Modules

- **`algebra`**: Provides `RingElement` types for zero-copy operations
- **`protocols`**: Consumes communication primitives for MPC operations
- **`net`**: Underlying network trait implementation
- **`random`**: Provides correlated randomness for resharing
