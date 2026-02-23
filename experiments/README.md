# Experiments Benchmark

This directory contains benchmark experiments for the Qamboo framework, primarily used to evaluate the performance of MPC (Secure Multi-Party Computation) queries.

## Directory Structure

```
experiments/
├── query/
│   ├── tpch/              # TPC-H benchmarks (Q1-Q22)
│   ├── optimization/      # Optimization comparison experiments
│   └── secrecy/           # Privacy-preserving application experiments
├── operator/              # Operator-level benchmarks
├── net/                   # Network configuration files
│   ├── local/             # Local test configurations
│   └── multinode/         # Multi-node test configurations
├── result/                # Experiment results
└── src/                   # Shared library code
    ├── tpch_database_gen.rs    # TPC-H data generator
    ├── secrecy_database_gen.rs # Privacy data generator
    ├── net_statistics.rs       # Network statistics utilities
    └── timer.rs                # Timer utilities
```

## TPC-H Benchmark

### Dataset Scale

Dataset size calculation based on Scale Factor (SF):

| Table Name | Row Count Formula | Rows (SF=0.1) | Rows (SF=1) |
|:---|:---|:---|:---|
| **lineitem** | sf × 6,000,000 | 600,000 | 6,000,000 |
| **orders** | sf × 1,500,000 | 150,000 | 1,500,000 |
| **customer** | sf × 150,000 | 15,000 | 150,000 |
| **part** | sf × 200,000 | 20,000 | 200,000 |
| **supplier** | sf × 10,000 | 1,000 | 10,000 |
| **partsupp** | sf × 800,000 | 80,000 | 800,000 |
| **nation** | 25 | 25 | 25 |
| **region** | 5 | 5 | 5 |

### TPC-H Query Table Statistics

The following table lists the initial tables used by each TPC-H query and their total row counts (SF=1):

| Query | Tables Involved | Total Initial Rows (SF=1) |
|:---|:---|:---|
| **Q1** | lineitem | 6,000,000 |
| **Q2** | part, supplier, partsupp, nation, region | 1,010,030 |
| **Q3** | customer, orders, lineitem | 7,650,000 |
| **Q4** | orders, lineitem | 7,500,000 |
| **Q5** | customer, orders, lineitem, supplier, nation, region | 7,660,030 |
| **Q6** | lineitem | 6,000,000 |
| **Q7** | supplier, lineitem, orders, customer, nation(n1), nation(n2) | 7,660,025 |
| **Q8** | part, supplier, lineitem, orders, customer, nation(n1), nation(n2), region | 7,860,030 |
| **Q9** | part, supplier, lineitem, partsupp, orders, nation | 8,510,025 |
| **Q10** | customer, orders, lineitem, nation | 7,650,025 |
| **Q11** | partsupp, supplier, nation | 810,025 |
| **Q12** | orders, lineitem | 7,500,000 |
| **Q13** | customer, orders | 1,650,000 |
| **Q14** | lineitem, part | 6,200,000 |
| **Q15** | lineitem, supplier | 6,010,000 |
| **Q16** | partsupp, part, supplier | 1,010,000 |
| **Q17** | lineitem, part | 6,200,000 |
| **Q18** | customer, orders, lineitem | 7,650,000 |
| **Q19** | lineitem, part | 6,200,000 |
| **Q20** | supplier, nation, partsupp, part, lineitem | 7,010,025 |
| **Q21** | supplier, lineitem(l1,l2,l3), orders, nation | 7,510,025 |
| **Q22** | customer, orders | 1,650,000 |

> **Notes**:
> - In Q7 and Q8, the nation table appears as two aliases (n1 and n2), but represents the same logical table, so counted only once (25 rows)
> - In Q21, lineitem appears as three aliases (l1, l2, l3), but represents the same logical table, so counted only once (6,000,000 rows)
> - Total row counts do not include intermediate result tables generated during query execution

### Running TPC-H Queries

```bash
# Build specific query
cargo build --release --bin q1

# Run locally (3-party computation)
./target/release/q1 -c experiments/net/local/0/ -p 0 -t 6 -s 0.1 &
./target/release/q1 -c experiments/net/local/0/ -p 1 -t 6 -s 0.1 &
./target/release/q1 -c experiments/net/local/0/ -p 2 -t 6 -s 0.1
```

Parameters:

- `-c`: Configuration file directory
- `-p`: Party ID (0, 1, 2)
- `-t`: Number of threads
- `-s`: Scale Factor (dataset size factor)

## Optimization Comparison Experiments

### No secure cut Optimization Comparison

Compare performance impact of enabling/disabling secure cut optimization:

| Query | Optimized Version | Unoptimized Version |
|:---|:---|:---|
| Q2 | `q2` | `q2_no_secure_cut` |
| Q3 | `q3` | `q3_no_secure_cut` |
| Q5 | `q5` | `q5_no_secure_cut` |
| Q8 | `q8` | `q8_no_secure_cut` |
| Q13 | `q13` | `q13_no_secure_cut` |
| Q17 | `q17` | `q17_no_secure_cut` |
| Q18 | `q18` | `q18_no_secure_cut` |
| Q20 | `q20` | `q20_no_secure_cut` |
| Q21 | `q21` | `q21_no_secure_cut` |

### Join Reorder Optimization Comparison

- `q5`: With join reorder optimization
- `q5_no_join_reorder`: Without join reorder optimization

### Semi Join Optimization Comparison

- `q4`: With semi join optimization
- `q4_no_semi`: Without semi join optimization (using regular join + filter)

## Operator-Level Benchmarks

| Test Name | Description |
|:---|:---|
| `multi_keys_join` | Multi-key join performance test |
| `radix_sort` | Radix sort performance test |
| `radix_sort_scalability` | Radix sort scalability test |

## Privacy-Preserving Application Experiments

| Test Name | Description |
|:---|:---|
| `comorbidity` | Comorbidity analysis (medical data) |
| `rcdiff` | Differential privacy related experiments |

## Network Configuration

- `net/local/`: Local single-machine multi-process test configurations (10 groups: 0-9)
- `net/multinode/`: Multi-machine distributed test configurations (16 groups: 0-15)

Each configuration group contains TOML config files for 3 parties:
- `config_party0.toml`
- `config_party1.toml`
- `config_party2.toml`

## Data Generation

TPC-H data is dynamically generated by `src/tpch_database_gen.rs`, supporting the following tables:
- lineitem, orders, customer
- part, supplier, partsupp
- nation, region

Data is generated in secret-shared form, with verification mode supported (ID0 generates plaintext comparison data).
