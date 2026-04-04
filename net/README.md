# Net Module

## Overview

The `net` module provides high-performance network abstractions for MPC protocols. It implements optimized TCP networking with features like connection pooling, vectored I/O, and userspace buffering, specifically designed for the communication patterns of multi-party computation.

## Network Implementations

### 1. FastTcpNetwork (`fast_tcp.rs`) — Recommended

A high-performance TCP implementation optimized for MPC workloads:

**Key Optimizations:**
- **Write buffering**: 2MB `BufWriter` per connection to coalesce small writes
- **Vectored I/O**: `write_vectored` merges length prefix + payload in one syscall
- **Read buffering**: 2MB `BufReader` to reduce read syscalls
- **Large socket buffers**: 8MB kernel buffers for high-bandwidth links
- **High-capacity channels**: 4096-slot crossbeam channels

```rust
use net::fast_tcp::{FastTcpNetwork, NetworkConfig, NetworkParty};

let config = NetworkConfig::new(
    my_id,
    bind_addr,
    vec![party0, party1, party2],
    Some(Duration::from_secs(60)),
    Some(128 * 1024 * 1024), // 128MB max frame
);

let network = FastTcpNetwork::new(config)?;
```

### 2. TcpNetwork (`tcp.rs`) — Basic

Standard TCP implementation without userspace buffering. Suitable for:
- Simple deployments
- Low-latency LAN environments
- Debugging and testing

## Core Network Trait

```rust
/// The core network abstraction for MPC protocols
pub trait Network: Send + Sync {
    /// Party ID (0-based)
    fn id(&self) -> usize;
    
    /// Send data to another party
    fn send(&self, to: usize, data: &[u8]) -> Result<()>;
    
    /// Receive data from another party
    fn recv(&self, from: usize) -> Result<Vec<u8>>;
    
    /// Get connection statistics
    fn get_connection_stats(&self) -> ConnectionStats;
    
    /// Buffered send (for implementations with buffering)
    fn send_buffered(&self, to: usize, data: &[u8]) -> Result<()>;
    
    /// Flush buffered data to specific party
    fn flush_to(&self, to: usize) -> Result<()>;
    
    /// Flush all buffered data
    fn flush_all(&self) -> Result<()>;
}
```

## Configuration

### NetworkConfig

```rust
pub struct NetworkConfig {
    pub parties: Vec<NetworkParty>,    // All parties in the network
    pub my_id: usize,                   // This party's ID
    pub bind_addr: SocketAddr,          // Local bind address
    pub timeout: Option<Duration>,      // Connection timeout
    pub max_frame_length: Option<usize>, // Max message size
}

pub struct NetworkParty {
    pub id: usize,
    pub dns_name: Address,  // Supports IP or hostname
}
```

### Address Resolution

```rust
pub enum Address {
    IpAddr(SocketAddr),
    DnsName { host: String, port: u16 },
}
```

Supports both static IP addresses and DNS hostnames for cloud deployments.

## Connection Management

### Topology

For 3-party protocols, the network maintains 2 connections per party:

```text
Party 0:  <---> Party 1
    ↑         ↓
    └----> Party 2
```

Each party connects to all other parties in a full mesh topology.

### Connection Establishment

```rust
// All parties bind to their addresses simultaneously
let listener = TcpListener::bind(bind_addr)?;

// Connect to parties with lower IDs
for party in parties_with_lower_id {
    let stream = TcpStream::connect(party.dns_name)?;
}

// Accept connections from parties with higher IDs
for party in parties_with_higher_id {
    let (stream, _) = listener.accept()?;
}
```

This ensures deterministic connection establishment without deadlocks.

## Performance Features

### Socket Tuning

```rust
// Large kernel buffers for high-bandwidth networks
socket.set_send_buffer_size(8 * 1024 * 1024)?;
socket.set_recv_buffer_size(8 * 1024 * 1024)?;

// TCP_NODELAY disabled for batching
socket.set_nodelay(false)?;

// Keep-alive for long-running protocols
socket.set_keepalive(Some(Duration::from_secs(60)))?;
```

### Vectored I/O

```rust
// Write length prefix and payload in single syscall
let len_bytes = (data.len() as u64).to_be_bytes();
let iovec = [
    IoSlice::new(&len_bytes),
    IoSlice::new(data),
];
writer.write_vectored(&iovec)?;
```

### Userspace Buffering

```rust
// FastTcpNetwork uses BufWriter for coalescing
pub struct FastTcpNetwork {
    writers: Vec<Mutex<BufWriter<TcpStream>>>, // 2MB buffers
    readers: Vec<Mutex<BufReader<TcpStream>>>, // 2MB buffers
}
```

## Connection Statistics

Monitor network usage for performance analysis:

```rust
let stats = network.get_connection_stats();
println!("{}", stats);
// Output:
// Party 0 <-> 1: SENT 1048576 bytes, RECV 524288 bytes
// Party 0 <-> 2: SENT 524288 bytes, RECV 1048576 bytes
```

## Parallel Network Operations

The module provides utilities for concurrent network operations:

```rust
use net::{join, join_all};

// Run two network operations in parallel
let (result1, result2) = join(
    || network.send(1, &data1),
    || network.recv(2),
);

// Run multiple operations in parallel
let results: Vec<_> = join_all([
    || network.send(1, &data1),
    || network.send(2, &data2),
    || network.recv(1),
    || network.recv(2),
]);
```

## Usage Example

```rust
use net::fast_tcp::{FastTcpNetwork, NetworkConfig, NetworkParty, Address};
use std::net::SocketAddr;

// Define parties
let parties = vec![
    NetworkParty::new(0, Address::IpAddr("10.0.0.1:9000".parse()?)),
    NetworkParty::new(1, Address::IpAddr("10.0.0.2:9000".parse()?)),
    NetworkParty::new(2, Address::IpAddr("10.0.0.3:9000".parse()?)),
];

// Create network
let config = NetworkConfig::new(
    0,  // My ID
    "0.0.0.0:9000".parse()?,  // Bind address
    parties,
    Some(Duration::from_secs(60)),
    None,
);
let network = FastTcpNetwork::new(config)?;

// Send/receive data
network.send(1, b"Hello Party 1")?;
let msg = network.recv(2)?;
```

## Architecture

```text
net/
├── src/
│   ├── lib.rs           # Network trait and utilities
│   ├── config.rs        # Configuration types
│   ├── fast_tcp.rs      # High-performance TCP implementation
│   └── tcp.rs           # Basic TCP implementation
└── examples/
    └── three_party_tcp.rs  # Example 3-party network
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `fast_tcp` | High-performance TCP with buffering | Yes |
| `tcp` | Basic TCP implementation | No |

## Performance Tuning Guide

### For High-Latency Networks (Cloud/WAN)

1. Increase socket buffers: 8MB+ recommended
2. Enable write buffering: Critical for small messages
3. Batch operations: Send multiple shares in one message
4. Use vectored I/O: Reduces syscalls

### For Low-Latency Networks (LAN/RDMA)

1. Smaller buffers may be sufficient (1-2MB)
2. Consider `tcp` implementation for simplicity
3. Reduce max frame size if messages are small

## Troubleshooting

### Connection Timeouts

- Check firewall rules between parties
- Verify all parties start within timeout window
- Increase `timeout` in NetworkConfig

### High Latency

- Enable `fast_tcp` feature for buffering
- Check socket buffer sizes (use `ss -tmnp`)
- Consider network topology (avoid NAT when possible)
