#!/bin/bash

# Single-thread Radix Sort (Multinode)
# Usage: ./run_radix_single_ssh.sh [SHIFT] [HOSTS]
#   HOSTS: comma-separated list of 3 hosts, one per party
#          (default: node0,node1,node2); the local host runs in-process,
#          others via SSH.

# Auto-detect project root from script location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel)"
BIN_PATH="$PROJECT_PATH/target/release/radix_sort_single"

SHIFT=${1:-20}
HOSTS=${2:-node0,node1,node2}

is_local() { [ "$1" == "$(hostname)" ] || [ "$1" == "localhost" ] || [ "$1" == "127.0.0.1" ]; }
IFS=',' read -r -a HOSTS_ARR <<< "$HOSTS"

cd $PROJECT_PATH

# Build binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort_single --features tcp

# Copy to remote hosts
for h in "${HOSTS_ARR[@]}"; do
    is_local "$h" || scp target/release/radix_sort_single "$h:$PROJECT_PATH/target/release/"
done

# Party i runs on the i-th host
for p in 0 1 2; do
    h="${HOSTS_ARR[$p]}"
    if is_local "$h"; then
        $BIN_PATH -c experiments/net/multinode/ -p $p -s $SHIFT &
    else
        ssh "$h" "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p $p -s $SHIFT" &
    fi
done

wait
