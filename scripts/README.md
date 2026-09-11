# Scripts

This directory contains automation scripts for deploying the Qamboo framework and running all benchmark experiments, both locally (single-machine, multi-process) and on distributed multi-node clusters.

## Directory Structure

```text
scripts/
├── setup/                  # Environment setup and deployment automation
│   ├── net/               # Network configuration generators
│   ├── deploy.sh          # One-click multi-machine deployment pipeline
│   ├── setup_connection.sh # Generate network configs across hosts
│   ├── setup_delay.sh     # Network delay emulation
│   ├── setup_host.sh      # Update /etc/hosts across nodes
│   ├── setup_rdma.sh      # RDMA environment setup
│   └── setup_ssh.sh       # SSH trust establishment
│
└── experiments/           # Benchmark execution
    ├── run_common.sh     # Shared engine (sourced by the runners)
    ├── run_tpch.sh       # TPC-H Q1–Q22
    ├── run_secrecy.sh    # Secrecy application benchmarks
    ├── run_operator.sh   # Operator micro-benchmarks
    ├── run_primitive.sh  # Primitive micro-benchmarks
    ├── run_optimization.sh # Ablations + thread-scaling sweeps
    ├── run_radix_sort_mpspdz_ssh.sh # MP-SPDZ baseline
    └── thread_scaling/   # Analysis tooling + radix thread-scaling scripts
```

---

## Setup & Deployment

### Quick Deployment

For fresh multi-node deployments, run the one-click deployment pipeline:

```bash
cd scripts/setup
./deploy.sh -i 192.168.1.10,192.168.1.11 -x node
```

This pipeline executes the following steps automatically:
1. **`setup_ssh.sh`** — Establish password-less SSH trust between nodes (also usable standalone).
2. **`setup_host.sh`** — Synchronize `/etc/hosts` entries.
3. **`setup_rdma.sh`** — Configure RDMA environments.
4. **Rust check** — Verify/install the Rust toolchain on remote hosts.
5. **Build & distribute** — Compile the release build and sync the entire project to remote hosts.

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
| `net/local_net_gen.py` | Generate localhost port mappings |
| `net/lan_net_gen.py` | Generate LAN IP/port mappings |

---

## Running Benchmarks

Each benchmark suite has its own runner under `scripts/experiments/`, all built on the shared engine `run_common.sh`:

| Script | Suite | Targets |
|:---|:---|:---|
| `run_tpch.sh` | TPC-H Q1–Q22 | `"1..22"`, `"1,3,5"`, `all` |
| `run_secrecy.sh` | Secrecy apps | `aspirin comorbidity credit pwd rcdiff`, `all` |
| `run_operator.sh` | Operator micro-benchmarks | `multi_keys_join radix_sort radix_sort_scalability radix_sort_single radix_sort_multi`, `all` |
| `run_primitive.sh` | Primitive micro-benchmarks | `mul_bench compare_bench`, `all` |
| `run_optimization.sh` | Ablations + thread scaling | `no_secure_cut`, `no_join_reorder`, `no_semi`, `thread_scaling` |

Each runner builds the binary (`cargo build --release --features tcp`), launches the 3 parties, and appends `INFO Total` lines to the suite's stat log. Targets support comma lists (`"1,3,5"`), ranges (`"1..8"`), and `all`.

### Modes

```text
local  3 parties as local background processes (experiments/net/local/); default
tcp    party i runs on the i-th host of -h, remote parties via SSH
       (experiments/net/multinode/)
rdma   like tcp, but binaries run under smc_run (SMC-R RDMA)
```

### Options

```text
<targets>  What to run: comma list ("1,3,5"), range ("1..8"), or "all".
           run_optimization.sh takes a variant name first
           (no_secure_cut / no_join_reorder / no_semi / thread_scaling)
-m         Execution mode (local/tcp/rdma); default: local
-t         Number of communication threads per party; default: 6 (4 for run_secrecy.sh)
-s         Scale factor / shift (log2 input size for radix_sort);
           default: 0.01 (20 for radix_sort)
--start    radix_sort_scalability sweep start (first size: 2^start rows); default: 20
--end      radix_sort_scalability sweep end, inclusive (last size: 2^end rows); default: 26
-h         Comma-separated list of 3 hosts, one per party; a host matching this
           machine runs in-process, others via SSH (tcp/rdma modes only);
           default: node0,node1,node2
--rayon    RAYON_NUM_THREADS, compute threads per party; default: unset
--log      Override the stat log path; default: per-suite path
--no-log   Disable stat logging
```

### Examples

```bash
cd scripts/experiments

# TPC-H
./run_tpch.sh 1 -t 6 -s 0.1                                     # single query, local
./run_tpch.sh "1..22" -t 6 -s 1                                 # batch, local
./run_tpch.sh "1..22" -t 32 -s 1 -m tcp  -h node0,node1,node2    # multinode
./run_tpch.sh "1..22" -t 32 -s 1 -m rdma -h node0,node1,node2    # multinode, RDMA

# Secrecy applications
./run_secrecy.sh comorbidity -t 4 -s 0.01
./run_secrecy.sh all -t 16 -s 0.1 -m tcp -h node0,node1,node2

# Operator micro-benchmarks
./run_operator.sh multi_keys_join
./run_operator.sh radix_sort -t 6 -s 20 -m tcp                  # shift=20, multinode
./run_operator.sh radix_sort_scalability -t 6 --start 16 --end 24   # sweep 2^16..2^24 rows

# Primitive micro-benchmarks (each sweeps 2^16..2^25 rows internally)
./run_primitive.sh mul_bench -t 6                               # multiplication sweep
./run_primitive.sh all -t 6 -m tcp -h node0,node1,node2         # mul + less-than sweeps

# Optimization ablations
./run_optimization.sh no_secure_cut "2,3,5" -t 6 -s 1 -m tcp -h node0,node1,node2
./run_optimization.sh no_semi 4 -t 6 -s 0.1

# Thread scaling (RAYON_NUM_THREADS x NUM_COMMTHREADS sweep)
./run_optimization.sh thread_scaling "1,3,4" -s 1               # local sweep
./run_optimization.sh thread_scaling "1..8" -s 1 -m tcp         # multinode sweep
```

Ablation variants and their supported queries:

| Variant | Supported queries |
|:---|:---|
| `no_secure_cut` | Q2, Q3, Q5, Q8, Q13, Q17, Q18, Q20, Q21 |
| `no_join_reorder` | Q2, Q5, Q7, Q8, Q9, Q10 |
| `no_semi` | Q4 |
| `thread_scaling` | Q1–Q22 (sweep; requires `-s`) |

Also under `scripts/experiments/`:

- `run_radix_sort_mpspdz_ssh.sh` — MP-SPDZ radix-sort baseline (requires MP-SPDZ on the nodes).
- `thread_scaling/` — result analysis (`analyze_results.py`) and radix-sort thread-scaling scripts (`radix/`).

### Shared engine (`run_common.sh`)

`run_common.sh` is sourced (not executed) by all five runners. To add a new suite runner: parse the suite's targets, fill `TARGETS`/`BINS`, call `setup_log`, then `run_all`.

```text
Options parsed by parse_common_opts (same as the runner CLIs above):
-m / -t / -s / -n / -h / --rayon / --log / --no-log

Globals consumed by the engine:
TARGETS       Display names to run (parallel to BINS); required by run_all
BINS          Cargo binary name per target (e.g. q5, comorbidity); required
MODE          local | tcp | rdma; default: local
THREADS       Communication threads; runner fills its suite default when unset
SF            Scale factor / shift; runner fills its suite default when unset
START         radix_sort_scalability sweep start (2^start rows); default: 20
END           radix_sort_scalability sweep end (2^end rows, inclusive); default: 26
HOST_LIST     From -h; party i runs on the i-th host, local hosts in-process;
              default: node0,node1,node2
RAYON         From --rayon; exported as RAYON_NUM_THREADS on every party
LOG_OVERRIDE  From --log; default: ""
NO_LOG        From --no-log; default: false
PROJECT_ROOT  Auto-detected via git rev-parse (fallback: ../.. from the script)
LOG_FILE      Resolved by setup_log; rdma mode uses stat_output_rdma.log

Functions provided:
parse_common_opts "$@"            Parse the options above into the globals
expand_targets <input> [all...]   Expand "all", comma lists and "1..8" ranges
contains <x> [values...]          Membership test
is_local_host <host>              True for $(hostname) / localhost / 127.0.0.1
setup_log <result-subdir>         Resolve LOG_FILE and mkdir -p its directory
run_one <bin>                     Build the binary (--features tcp), distribute
                                  it to remote hosts, and run the 3 parties
run_all <label>                   Loop TARGETS/BINS through run_one, tee
                                  "INFO Total" lines into LOG_FILE, and
                                  report per-target success/failure
```

---

## Result Collection

All batch runners pipe `INFO Total` lines from the experiment binaries into structured log files under `experiments/result/`:

| Experiment Category | Local Log Path | Multinode Log Path |
|:---|:---|:---|
| TPC-H | `experiments/result/tpch_query/local/stat_output.log` | `experiments/result/tpch_query/multinode/stat_output.log` |
| Secrecy | `experiments/result/secrecy_query/local/stat_output.log` | `experiments/result/secrecy_query/multinode/stat_output.log` |
| Operator | `experiments/result/operator/local/stat_output.log` | `experiments/result/operator/multinode/stat_output.log` |
| Primitive | `experiments/result/primitive/local/stat_output.log` | `experiments/result/primitive/multinode/stat_output.log` |
| Optimization | `experiments/result/query_optimization/<variant>/local/stat_output.log` | `experiments/result/query_optimization/<variant>/multinode/stat_output.log` |
| Thread scaling | `experiments/result/thread_scaling_local/` | `experiments/result/thread_scaling/` |

> RDMA runs (`-m rdma`) log to `stat_output_rdma.log` next to the multinode log.

> **Tip:** The batch scripts strip ANSI color codes before appending to logs, so they are safe for direct parsing with `awk`, `grep`, or Python.
