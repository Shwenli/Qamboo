# Thread Scaling Experiments for Qamboo

This folder contains scripts to evaluate the performance impact of different thread configurations:
- **RAYON_NUM_THREADS**: Number of Rayon compute threads (data parallelism)
- **NUM_COMMTHREADS**: Number of communication threads (I/O parallelism)

Supports both **multinode** (distributed) and **local** (single machine) execution.

## Directory Structure

```
thread_scaling/
├── run_thread_scaling_exp.sh           # Main experiment runner (multinode)
├── run_thread_scaling_local_exp.sh     # Main experiment runner (local)
├── generate_query_scripts.sh           # Generate multinode query scripts
├── generate_local_scripts.sh           # Generate local query scripts
├── analyze_results.py                  # Result analysis script
├── multinode/
│   ├── run_q1_thread_scaling.sh        # Distributed query scripts
│   └── ... (Q1-Q22)
├── local/
│   ├── run_q1_thread_scaling.sh        # Local query scripts (3 parties on 1 machine)
│   └── ... (Q1-Q22)
└── README.md                           # This file
```

## Usage

### 1. Generate Query Scripts (if needed)

Scripts for Q1-Q22 are already generated for both multinode and local. If you need to regenerate:

```bash
cd scripts/experiments/optimization/thread_scaling

# Generate multinode scripts
./generate_query_scripts.sh

# Generate local scripts
./generate_local_scripts.sh
```

### 2. Run Thread Scaling Experiments

#### Multinode (Distributed)

```bash
cd scripts/experiments/optimization/thread_scaling
./run_thread_scaling_exp.sh <SF> <QUERIES> [-h1 HOST1] [-h2 HOST2]
```

**Examples:**

```bash
# Run Q1, Q3, Q4 at SF=1
./run_thread_scaling_exp.sh 1 "1,3,4"

# Run Q1-Q8 at SF=0.1
./run_thread_scaling_exp.sh 0.1 "1..8"

# Run with custom hosts
./run_thread_scaling_exp.sh 1 "3" -h1 192.168.1.11 -h2 192.168.1.12
```

#### Local (Single Machine)

```bash
cd scripts/experiments/optimization/thread_scaling
./run_thread_scaling_local_exp.sh <SF> <QUERIES>
```

**Examples:**

```bash
# Run Q1, Q3, Q4 at SF=1
./run_thread_scaling_local_exp.sh 1 "1,3,4"

# Run Q1-Q8 at SF=0.1
./run_thread_scaling_local_exp.sh 0.1 "1..8"

# Run all queries at SF=1
./run_thread_scaling_local_exp.sh 1 "1..22"
```

### 3. Default Thread Configurations

The script tests the following combinations:

| RAYON_NUM_THREADS | NUM_COMMTHREADS |
|-------------------|-----------------|
| 1, 2, 4, 8, 16, 32 | 1, 2, 4, 8, 16 |

**Total combinations**: 6 × 5 = 30 per query

To modify these configurations, edit the arrays in `run_thread_scaling_exp.sh`:
```bash
RAYON_THREADS_LIST=(1 2 4 8 16 32)
COMM_THREADS_LIST=(1 2 4 8 16)
```

### 4. Results

Results are saved to:

**Multinode:**
```
experiments/result/thread_scaling/thread_scaling_sf<SF>.log
```

**Local:**
```
experiments/result/thread_scaling_local/thread_scaling_local_sf<SF>.log
```

The log file contains timing results for each (RAYON_NUM_THREADS, NUM_COMMTHREADS) combination.

## Understanding the Parameters

### RAYON_NUM_THREADS (Compute Parallelism)
- Controls data parallelism within each MPC party
- Used by Rayon for parallel iterators
- Affects CPU utilization of compute-intensive operations

### NUM_COMMTHREADS (I/O Parallelism)
- Controls the number of communication threads
- Each thread manages connections to other parties
- Affects network throughput, especially in WAN settings

## Example Output

```
============================================================
Thread Scaling Results - SF=1, Queries=1,3,4
Started at: Mon Apr 13 10:00:00 CST 2026
============================================================
>>> [1/30] RAYON_NUM_THREADS=1, NUM_COMMTHREADS=1
    Q1:
    INFO Total time: 45.2s
    Q1 OK
    Q3:
    INFO Total time: 78.5s
    Q3 OK
    ...
>>> [2/30] RAYON_NUM_THREADS=1, NUM_COMMTHREADS=2
    ...
```

## Notes

- Ensure all remote hosts have the same CPU core count for consistent results
- The script waits 2 seconds between configurations to ensure port release
- Build is performed once per query, then binary is copied to remote hosts
