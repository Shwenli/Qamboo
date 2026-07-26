#!/bin/bash
#!run with: ./run_radix_sort_compare_ssh.sh 6 20

# Define remote host IPs and users (Modify according to actual situation) 
# Auto-detect project root from script location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel)"
BIN_PATH="./target/release/radix_sort"

NUM_COMMTHREADS=${1:-6} #number of threads (default: 6)
SHIFT=${2:-20} # Shift Factor (default: 20)
HOST1=${3:-node1}       # first remote host (default: node1)
HOST2=${4:-node2}       # second remote host (default: node2)

cd $PROJECT_PATH

# Build radix_sort binary and copy to remote hosts
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort --features tcp
scp target/release/radix_sort $HOST1:$PROJECT_PATH/target/release/
scp target/release/radix_sort $HOST2:$PROJECT_PATH/target/release/

# Run party 0 locally
$BIN_PATH -c experiments/net/multinode/ -p 0 -t $NUM_COMMTHREADS -s $SHIFT &
# SSH to HOST1 to run party 1
ssh $HOST1 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 1 -t $NUM_COMMTHREADS -s $SHIFT" &
# SSH to HOST2 to run party 2
ssh $HOST2 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 2 -t $NUM_COMMTHREADS -s $SHIFT" &

wait