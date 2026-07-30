#!/bin/bash
#!run with: ./run_radix_sort_mpspdz_ssh.sh [NUM_COMMTHREADS] [HOSTS]
#
# MP-SPDZ radix-sort baseline (requires MP-SPDZ on the nodes).
#   HOSTS: comma-separated list of 3 hosts, one per party
#          (default: node0,node1,node2); the local host runs in-process,
#          others via SSH.

# Auto-detect project root from script location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel)"
BIN_PATH="./target/release/radix_sort_mpspdz"

NUM_COMMTHREADS=${1:-6}                # number of threads (default: 6)
HOSTS=${2:-node0,node1,node2}          # host per party

is_local() { [ "$1" == "$(hostname)" ] || [ "$1" == "localhost" ] || [ "$1" == "127.0.0.1" ]; }
IFS=',' read -r -a HOSTS_ARR <<< "$HOSTS"

cd $PROJECT_PATH

# Build radix_sort_mpspdz binary and copy to remote hosts
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort_mpspdz --features tcp
for h in "${HOSTS_ARR[@]}"; do
    is_local "$h" || scp target/release/radix_sort_mpspdz "$h:$PROJECT_PATH/target/release/"
done

# Party i runs on the i-th host
for p in 0 1 2; do
    h="${HOSTS_ARR[$p]}"
    if is_local "$h"; then
        $BIN_PATH -c experiments/net/multinode/ -p $p -t $NUM_COMMTHREADS &
    else
        ssh "$h" "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p $p -t $NUM_COMMTHREADS" &
    fi
done

wait
