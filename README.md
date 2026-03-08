# Qamboo 🎋

**Scalable Secure Collaborative Analytics in Cloud with Low-Overhead Implementation**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Paper](https://img.shields.io/badge/Paper-PDF-red.svg)](#)

> An efficient MPC-based framework for privacy-preserving collaborative data analytics in cloud environments.

---

## Abstract

Qamboo is a high-performance Secure Multi-Party Computation (MPC) framework designed for **scalable secure collaborative analytics** in cloud environments. By leveraging replicated secret sharing and novel low-overhead cryptographic protocols, Qamboo enables multiple distrusting parties to jointly analyze their combined datasets without revealing sensitive information. Our implementation supports the full TPC-H benchmark suite with significantly reduced communication overhead compared to prior approaches, making secure analytics practical for real-world cloud deployments.

**Key Contributions:**
- ☁️ **Cloud-Native Design**: Optimized for high-latency cloud networks with efficient batching and parallelization
- 📈 **Scalable Architecture**: Linear scaling with data size and number of parties
- ⚡ **Low Overhead**: Zero-copy serialization, minimized round trips, and streaming execution
- 🗃️ **Full SQL Support**: Complete TPC-H query suite (Q1-Q22) with Joins, GroupBy, Sort, and Aggregation

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Qamboo Framework                          │
├─────────────────────────────────────────────────────────────────┤
│  Application Layer │  TPC-H Queries  │  Custom Analytics        │
├─────────────────────────────────────────────────────────────────┤
│   Table Operators  │  Secure Join │ GroupBy │ Sort │ Filter     │
├─────────────────────────────────────────────────────────────────┤
│  Crypto Primitives │  Compare │ Permute │ Shuffle │ Multiplex   │
├─────────────────────────────────────────────────────────────────┤
│    MPC Protocols   │  Replicated Secret Sharing (3-Party)       │
│                    │  Shamir Secret Sharing                     │
├─────────────────────────────────────────────────────────────────┤
│   Network Layer    │  Fast TCP │ Zero-Copy Comm │ RDMA-ready   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Quick Start

### Prerequisites

- Rust 1.85 or later
- Linux/macOS environment
- (Optional) RDMA-capable network for high-performance deployments

### Building

```bash
# Clone the repository
git clone <repository-url>
cd qamboo

# Build release version
cargo build --release

# Run tests
cargo test --workspace
```

### Running TPC-H Benchmarks

Qamboo implements all 22 TPC-H queries for secure multi-party analytics:

```bash
# Run Query 1 (Pricing Summary Report)
cargo run --bin q1 --release -- --config configs/3party.toml

# Run Query 5 (Local Supplier Volume)
cargo run --bin q5 --release -- --config configs/3party.toml

# Run all TPC-H queries
./scripts/experiments/tpch/run_local_exp.sh
```

---

## Architecture

Qamboo adopts a modular workspace architecture with 9 specialized crates:

| Crate | Description | Key Features |
|-------|-------------|--------------|
| `protocols/` | Core MPC protocols | Replicated secret sharing, arithmetic operations |
| `primitives/` | Cryptographic primitives | Comparison, permutation, shuffling, MUX |
| `operator/` | Database operators | Join, GroupBy, Sort, Distinct, Aggregation |
| `table/` | Secure table operations | Columnar processing, query planning |
| `net/` | Network layer | Fast TCP, connection pooling, bandwidth optimization |
| `communication/` | Zero-copy communication | Optimized serialization for ring elements |
| `algebra/` | Algebraic foundations | Ring theory, finite field arithmetic |
| `random/` | Secure randomness | Correlated randomness generation |
| `experiments/` | Benchmarks & evaluation | TPC-H Q1-Q22, operator micro-benchmarks |

---

## Performance Highlights

### Low-Overhead Optimizations

| Technique | Benefit |
|-----------|---------|
| **Zero-Copy Serialization** | Eliminates memory copies for ring elements via pointer casting |
| **Streaming Execution** | Memory-efficient processing of large datasets |
| **Parallel Network I/O** | Concurrent sends/receives across party pairs |
| **Rayon Parallelism** | Data-parallel local computations |
| **Connection Pooling** | Reusable TCP connections with buffering |

### Scalability

- **Data Size**: Linear scaling with dataset size
- **Parties**: Optimized for 3-party setting (semi-honest)
- **Network**: Efficient bandwidth utilization for cloud deployments

---

## Security Model

Qamboo implements protocols secure in the **semi-honest model** with **3 parties**, where:

- **Privacy**: No single party learns others' private inputs
- **Robustness**: Tolerates collusion of up to 1 party
- **Assumptions**: Based on standard cryptographic hardness assumptions
- **Cloud-Ready**: Designed for honest-majority scenarios in cloud environments

---

## Example: Secure Collaborative Query

```rust
use table::{
    table_operator::{Filter, Groupby, AggFunc, OrderBy},
    share_table::SharedTable,
    NetStateArgs,
};
use protocols::rep3_ring::Rep3RingShare;
use random::rep3::Rep3State;
use net::fast_tcp::FastTcpNetwork;

// Initialize cloud deployment
let network = FastTcpNetwork::new(cloud_config)?;
let state = Rep3State::new(party_id, shared_seed);
let mut args = NetStateArgs::new(&[&network], &mut [&mut state]);

// Parties jointly analyze combined data without revealing inputs
let result = SharedTable::from_plain(&args, party_data)?
    .filter(&args, predicate)?              // Secure filtering
    .group_by(&args, keys, aggregations)?   // Secure aggregation
    .order_by(&args, sort_keys)?;           // Secure sorting

// Only final result is revealed
let output = result.open(&args)?;
```

---

## Repository Structure

```
qamboo/
├── Cargo.toml           # Workspace configuration
├── LICENSE              # Dual MIT/Apache-2.0 license
├── README.md            # This file
├── algebra/             # Ring and field arithmetic
├── communication/       # Zero-copy communication primitives
├── experiments/         # Evaluation suite
│   ├── query/tpch/      # TPC-H Q1-Q22 implementations
│   └── operator/        # Operator micro-benchmarks
├── net/                 # Cloud-optimized network layer
├── operator/            # Privacy-preserving relational operators
├── primitives/          # MPC building blocks (compare, permute, shuffle)
├── protocols/           # Replicated & Shamir secret sharing
├── random/              # Correlated randomness generation
├── scripts/             # Deployment automation
├── table/               # Secure table abstractions
└── tests/               # Integration tests
```

---

## Evaluation

Qamboo has been evaluated on:

- **TPC-H Benchmark**: All 22 queries at scale factors SF1-SF100
- **Cloud Deployment**: AWS/Azure multi-region setups
- **Network Conditions**: High-latency (50-200ms) WAN environments
- **Comparison**: Baseline against plain-text DB and prior MPC systems

See `experiments/` for reproducible benchmark scripts.

---

## Citation

If you use Qamboo in your research, please cite:

```bibtex
@inproceedings{qamboo2026,
  author    = {Shang, Qingxu},
  title     = {Qamboo: Scalable Secure Collaborative Analytics in Cloud 
               with Low-Overhead Implementation},
  booktitle = {...},
  year      = {2026},
  url       = {<repository-url>}
}
```

---

## Acknowledgments

Qamboo builds upon the following open-source libraries:

- [arkworks](https://github.com/arkworks-rs) — Elliptic curve and finite field arithmetic
- [tokio](https://tokio.rs/) — Async runtime for network operations
- [polars](https://pola.rs/) — DataFrame library for benchmark data generation
- [rayon](https://github.com/rayon-rs/rayon) — Data-parallelism for local computations

---

## License

This project is licensed under either of:

- **MIT License** — See [LICENSE](LICENSE) file for details
- **Apache License, Version 2.0** — See [LICENSE](LICENSE) file for details

at your option.

---

**Contact**: For questions or collaboration inquiries, please open an issue or contact the authors.

Made with 🎋 for secure cloud analytics.
