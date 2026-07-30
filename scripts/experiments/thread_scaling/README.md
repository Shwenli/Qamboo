# Thread Scaling Experiments for Qamboo

Thread-scaling sweeps evaluate the performance impact of different thread configurations:
- **RAYON_NUM_THREADS**: Number of Rayon compute threads (data parallelism)
- **NUM_COMMTHREADS**: Number of communication threads (I/O parallelism)

The sweep is implemented as the `thread_scaling` variant of
`scripts/experiments/run_optimization.sh`. This folder keeps the result
analysis script (`analyze_results.py`) and the radix-sort thread-scaling
scripts (`radix/`).

## Usage

```bash
cd scripts/experiments

# Local sweep (single machine, 3 parties)
./run_optimization.sh thread_scaling "1,3,4" -s 1

# Multinode sweep (party 0 local, parties 1 & 2 via SSH)
./run_optimization.sh thread_scaling "1..8" -s 0.1 -m tcp -h node0,node1,node2
```

### Thread Configurations

| Mode | RAYON_NUM_THREADS | NUM_COMMTHREADS |
|:---|:---|:---|
| local | 1, 2, 4, 8, 10 | 1, 2, 4, 8 |
| tcp / rdma | 1, 2, 4, 8, 16, 32 | 1, 2, 4, 8, 16 |

To modify these, edit `RAYON_LIST` / `COMM_LIST` in `run_optimization.sh`.

### Results

- Local: `experiments/result/thread_scaling_local/thread_scaling_local_sf<SF>.log`
- Multinode: `experiments/result/thread_scaling/thread_scaling_sf<SF>.log`

The log file contains timing results for each (RAYON_NUM_THREADS, NUM_COMMTHREADS)
combination and can be post-processed with `analyze_results.py`.

## Understanding the Parameters

### RAYON_NUM_THREADS (Compute Parallelism)
- Controls data parallelism within each MPC party
- Used by Rayon for parallel iterators
- Affects CPU utilization of compute-intensive operations

### NUM_COMMTHREADS (I/O Parallelism)
- Controls the number of communication threads
- Each thread manages connections to other parties
- Affects network throughput, especially in WAN settings

## Notes

- Ensure all remote hosts have the same CPU core count for consistent results
- The runner waits 1–2 seconds between runs to ensure port release
- Each query binary is built once per invocation, then copied to remote hosts
