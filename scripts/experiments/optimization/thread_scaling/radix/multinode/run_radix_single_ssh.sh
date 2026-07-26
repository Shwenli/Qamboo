#!/bin/bash

# Single-thread Radix Sort (Multinode SSH)
# Usage: ./run_radix_single_ssh.sh [SHIFT] [HOST1] [HOST2]

# Auto-detect project root from script location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel)"
BIN_PATH="$PROJECT_PATH/target/release/radix_sort_single"

SHIFT=${1:-20}
HOST1=${2:-node1}
HOST2=${3:-node2}

cd $PROJECT_PATH

# Build binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort_single --features tcp

# Copy to remote hosts
scp target/release/radix_sort_single $HOST1:$PROJECT_PATH/target/release/
scp target/release/radix_sort_single $HOST2:$PROJECT_PATH/target/release/

# Run party 0 locally
$BIN_PATH -c experiments/net/multinode/ -p 0 -s $SHIFT &
# Run party 1 on HOST1
ssh $HOST1 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 1 -s $SHIFT" &
# Run party 2 on HOST2
ssh $HOST2 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 2 -s $SHIFT" &

wait
