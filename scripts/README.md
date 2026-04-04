# Scripts

This directory contains automation scripts for deploying the Qamboo framework and running all benchmark experiments, both locally (single-machine, multi-process) and on distributed multi-node clusters.

## Directory Structure

```text
scripts/
├── setup/                  # Environment setup and deployment automation
│   ├── net/               # Network configuration generators
│   ├── build_dist.sh      # Build and distribute code to remote hosts
│   ├── deploy.sh          # One-click full deployment pipeline
│   ├── setup_connection.sh # Generate network configs across hosts
│   ├── setup_delay.sh     # Network delay emulation
│   ├── setup_host.sh      # Update /etc/hosts across nodes
│   ├── setup_rdma.sh      # RDMA environment setup
│   └── setup_ssh.sh       # SSH trust establishment
│
└── experiments/           # Benchmark execution scripts
    ├── tpch/             # TPC-H Q1–Q22 benchmarks
    ├── operator/         # Operator-level micro-benchmarks
    ├── optimization/     # Optimization ablation studies
    │   ├── no_secure_cut/
    │   ├── no_join_reorder/
    │   └── no_semi/
    └── secrecy/          # Privacy-preserving application benchmarks
```

---

## Setup & Deployment

### Quick Deployment

For fresh multi-node deployments, use the top-level orchestrator:

```bash
cd scripts/setup
./deploy.sh -i 192.168.1.10,192.168.1.11 -x node
```

This pipeline executes the following steps automatically:
1. **`setup_ssh.sh`** — Establish password-less SSH trust between nodes.
2. **`setup_host.sh`** — Synchronize `/etc/hosts` entries.
3. **`setup_rdma.sh`** — Configure RDMA environments.
4. **`build_dist.sh`** — Compile the release build and sync the entire project to remote hosts.

### Network Configuration

After deployment, generate TOML network configs for the 3-party protocol on all hosts:

```bash
./setup_connection.sh -t lan   -h node0,node1,node2 -n 16  # Multinode: 16 groups
./setup_connection.sh -t local -h localhost          -n 10  # Local: 10 groups
```

Generated configs are written to:
- `experiments/net/local/`      — 10 groups (`0`–`9`) for local execution
- `experiments/net/multinode/`  — 16 groups (`0`–`15`) for distributed execution

Each group contains `config_party0.toml`, `config_party1.toml`, and `config_party2.toml`.

### Setup Utilities

| Script | Purpose |
|:---|:---|
| `setup_ssh.sh` | Generate and exchange SSH keys across nodes |
| `setup_host.sh` | Write node aliases to `/etc/hosts` |
| `setup_rdma.sh` | Install and verify RDMA drivers/libraries |
| `setup_delay.sh` | Emulate WAN latency using `tc` (traffic control) |
| `build_dist.sh` | Compile workspace (`--exclude experiments`) and `scp -r` to all targets |
| `net/local_net_gen.py` | Generate localhost port mappings |
| `net/lan_net_gen.py` | Generate LAN IP/port mappings |

---

## Running Benchmarks

All experiment scripts follow a consistent layout:

- **`local/`** — Launch 3 parties as local background processes (`&`) and `wait`.
- **`multinode/`** — Build the binary locally, `scp` it to remote hosts, then launch party 0 locally and parties 1 & 2 via `ssh`.
- **`*_rdma*.sh`** — RDMA-enabled variants of multinode scripts.
- **Top-level `run_*.sh`** — Batch runners that iterate over a set of experiments, parse query ranges, and aggregate results.

### Common Arguments

| Argument | Description | Example |
|:---|:---|:---|
| `NUM_COMMTHREADS` | Number of communication threads | `6`, `12` |
| `SF` | Scale factor for data generation | `0.01`, `0.1`, `1` |
| `QUERIES` | Query list or range | `"1,3,5"` or `"1..8"` |
| `HOST1` / `HOST2` | Remote hosts for multinode SSH | `node1`, `192.168.1.11` |

---

## TPC-H Benchmarks (`experiments/tpch/`)

### Local Execution

Run individual queries or batch ranges:

```bash
cd scripts/experiments/tpch

# Single query
./local/run_q1.sh 6 0.1

# Batch runner: queries 1-22 at SF=1 with 6 threads
./run_local_exp.sh 6 1 "1..22"

# Batch runner: selective queries
./run_local_exp.sh 6 0.1 "1,3,5,8"
```

### Multinode Execution

```bash
cd scripts/experiments/tpch

# SSH-based distributed run
./run_multinode_exp.sh 6 1 "1..22" -h1 node1 -h2 node2

# RDMA-based distributed run
./run_multinode_rdma_exp.sh 6 1 "1..22" -h1 node1 -h2 node2
```

Results are appended to:
- `experiments/result/tpch_query/local/stat_output.log`
- `experiments/result/tpch_query/multinode/stat_output.log`

---

## Operator-Level Benchmarks (`experiments/operator/`)

| Script | Description |
|:---|:---|
| `local/run_multi_keys_join.sh` | Multi-key join performance test |
| `local/run_radix_sort_compare.sh` | Radix sort performance test |
| `local/run_radix_sort_scalability.sh` | Radix sort scalability sweep |
| `multinode/run_radix_sort_compare_ssh.sh` | Multinode radix sort (SSH) |
| `multinode/run_radix_sort_compare_ssh_rdma.sh` | Multinode radix sort (RDMA) |
| `multinode/run_radix_sort_mpspdz_ssh.sh` | MP-SPDZ radix-sort baseline |

Example:

```bash
cd scripts/experiments/operator/local
./run_multi_keys_join.sh
./run_radix_sort_compare.sh
```

---

## Optimization Comparison Experiments (`experiments/optimization/`)

These scripts run ablation studies that disable specific query-planning optimizations.

### Secure Group Cutting (`no_secure_cut/`)

Compares the full optimized query against a version with secure-cut disabled.

Supported queries: **Q2, Q3, Q5, Q8, Q13, Q17, Q18, Q20, Q21**

```bash
cd scripts/experiments/optimization/no_secure_cut
./local/run_q5_no_secure_cut.sh 6 0.1
./run_multinode_exp.sh 6 1 "2,3,5,8,13,17,18,20,21"
```

### Join Reorder (`no_join_reorder/`)

Compares the full optimized query against a version with join-reorder disabled.

Supported queries: **Q2, Q5, Q7, Q8, Q9, Q10**

```bash
cd scripts/experiments/optimization/no_join_reorder
./local/run_q5_no_join_reorder.sh 6 0.1
./run_multinode_exp.sh 6 1 "2,5,7,8,9,10"
```

### Semi-Join (`no_semi/`)

Compares the semi-join optimized version of **Q4** against the unoptimized fallback.

```bash
cd scripts/experiments/optimization/no_semi
./run_q4_no_semi.sh 6 0.1
```

---

## Privacy-Preserving Application Benchmarks (`experiments/secrecy/`)

| Script | Experiment | Description |
|:---|:---|:---|
| `run_comorbidity.sh` | `comorbidity` | Medical comorbidity analysis |
| `run_aspirin.sh` | `aspirin` | Aspirin treatment analysis |
| `run_credit.sh` | `credit` | Credit score change analysis |
| `run_pwd.sh` | `pwd` | Duplicate password detection |
| `run_rcdiff.sh` | `rcdiff` | Differential-privacy related experiments |

### Local Execution

```bash
cd scripts/experiments/secrecy/local
./run_comorbidity.sh 4 0.01
./run_aspirin.sh 4 0.01
./run_credit.sh 4 0.01
./run_pwd.sh 4 0.01
./run_rcdiff.sh 4 0.01
```

### Multinode Execution

```bash
cd scripts/experiments/secrecy
./run_multinode_exp.sh 4 0.1 -h1 node1 -h2 node2
```

There are also dedicated paper/RDMA batch runners:
- `run_multinode_exp_paper.sh` — Paper-result reproduction batch
- `run_multinode_rdma_exp.sh` — RDMA variant batch

---

## Result Collection

All batch runners pipe `INFO Total` lines from the experiment binaries into structured log files under `experiments/result/`:

| Experiment Category | Local Log Path | Multinode Log Path |
|:---|:---|:---|
| TPC-H | `experiments/result/tpch_query/local/stat_output.log` | `experiments/result/tpch_query/multinode/stat_output.log` |
| Secrecy | `experiments/result/secrecy_query/local/stat_output.log` | `experiments/result/secrecy_query/multinode/stat_output.log` |
| Operator | `experiments/result/operator/local/stat_output.log` | `experiments/result/operator/multinode/stat_output.log` |

> **Tip:** The batch scripts strip ANSI color codes before appending to logs, so they are safe for direct parsing with `awk`, `grep`, or Python.
