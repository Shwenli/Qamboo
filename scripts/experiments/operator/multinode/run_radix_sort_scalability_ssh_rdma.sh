#!/bin/bash
# Usage: ./run_radix_sort_scalability_ssh_rdma.sh <threads> <number>
# Example: ./run_radix_sort_scalability_ssh_rdma.sh 6 7 (6 threads, test from 2^19 to 2^25)

# Define remote host IPs and users (Modify according to actual situation)
# Auto-detect project root from script location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel)"
BIN_PATH="smc_run ./target/release/radix_sort_scalability"

NUM_COMMTHREADS=${1:-6} # number of threads (default: 6)
NUM=${2:-6}         # test number, 2^20(n=1) to 2^25(n=6) (default: 6)
HOST1=${3:-node1}       # first remote host (default: node1)
HOST2=${4:-node2}       # second remote host (default: node2)

cd $PROJECT_PATH

# Build radix_sort_scalability binary and copy to remote hosts
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort_scalability --features tcp
scp target/release/radix_sort_scalability $HOST1:$PROJECT_PATH/target/release/
scp target/release/radix_sort_scalability $HOST2:$PROJECT_PATH/target/release/

# Run party 0 locally
$BIN_PATH -c experiments/net/multinode/ -p 0 -t $NUM_COMMTHREADS -n $NUM &
# SSH to HOST1 to run party 1
ssh $HOST1 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 1 -t $NUM_COMMTHREADS -n $NUM" &
# SSH to HOST2 to run party 2
ssh $HOST2 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 2 -t $NUM_COMMTHREADS -n $NUM" &

wait
