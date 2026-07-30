# Radix Sort Thread Scaling Experiments

This folder contains scripts to evaluate the performance impact of different thread configurations on Radix Sort.

## Overview

- **Single-thread**: Uses `radix_sort` (no Rayon parallelism)
- **Multi-thread**: Uses `radix_sort_multithreads` (with Rayon parallelism)

## Directory Structure

```
radix/
├── run_radix_thread_scaling.sh         # Main experiment runner (multinode)
├── run_radix_thread_scaling_local.sh   # Main experiment runner (local)
├── multinode/
│   ├── run_radix_single_ssh.sh         # Single-thread distributed script
│   └── run_radix_multi_ssh.sh          # Multi-thread distributed script
└── local/
    ├── run_radix_single_local.sh       # Single-thread local script
    └── run_radix_multi_local.sh        # Multi-thread local script
```

## Usage

### Multinode (Distributed)

```bash
cd scripts/experiments/thread_scaling/radix
./run_radix_thread_scaling.sh <SHIFT> [-h HOSTS]
```

**Parameters:**
- `SHIFT`: Number of rows = 2^SHIFT (default: 23 = 8,388,608 rows for multinode)
- `HOSTS`: Comma-separated list of 3 hosts, one per party (default: `node0,node1,node2`); the local host runs in-process, others via SSH

**Examples:**
```bash
# Test with default 2^23 rows (8M rows)
./run_radix_thread_scaling.sh

# Test with custom rows (2^24 = 16M rows) on custom hosts
./run_radix_thread_scaling.sh 24 -h 192.168.1.10,192.168.1.11,192.168.1.12
```

**Thread Configurations (Multinode):**
| RAYON_NUM_THREADS | NUM_COMMTHREADS |
|-------------------|-----------------|
| 2, 4, 8, 16, 32   | 4, 8, 16, 32    |

- Single-thread: 1 config (no Rayon)
- Multi-thread: 5 × 4 = 20 configs
- **Total runs**: 21 per experiment

### Local (Single Machine)

```bash
cd scripts/experiments/thread_scaling/radix
./run_radix_thread_scaling_local.sh <SHIFT>
```

**Examples:**
```bash
# Test with default 2^20 rows (1M rows)
./run_radix_thread_scaling_local.sh

# Test with custom rows (2^18 = 256K rows)
./run_radix_thread_scaling_local.sh 18
```

**Thread Configurations (Local):**
| RAYON_NUM_THREADS | NUM_COMMTHREADS |
|-------------------|-----------------|
| 2, 4, 8, 10       | 2, 4, 8         |

- Single-thread: 1 config (no Rayon)
- Multi-thread: 4 × 3 = 12 configs
- **Total runs**: 13 per experiment

## Results

Results are saved to:

**Multinode:**
```
experiments/result/radix_thread_scaling/radix_multinode_shift<SHIFT>.log
```

**Local:**
```
experiments/result/radix_thread_scaling_local/radix_local_shift<SHIFT>.log
```

## Default Row Counts

- **Local (Single Machine)**: Default 2^20 rows (1,048,576 rows)
  - Suitable for single machine testing with limited memory
  - Faster iteration for debugging and parameter tuning
  
- **Multinode (Distributed)**: Default 2^23 rows (8,388,608 rows)
  - Larger dataset to better utilize distributed resources
  - More representative of production workloads

## Understanding the Implementations

### Single-thread (`radix_sort`)
- Uses sequential `bit_decompose_many`, `gen_perm`, `apply_perm`
- No Rayon parallelism
- One network connection per party

### Multi-thread (`radix_sort_multithreads`)
- Uses parallel `bit_decompose_many_multithreads`, `gen_perm_multithreads`, `apply_perm_multithreads`
- Rayon data parallelism for CPU-intensive operations
- Multiple network connections (NUM_COMMTHREADS) for I/O parallelism
