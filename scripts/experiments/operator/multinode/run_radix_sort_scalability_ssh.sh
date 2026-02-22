#!/bin/bash
# Usage: ./run_radix_sort_scalability_ssh.sh <threads> <number>
# Example: ./run_radix_sort_scalability_ssh.sh 6 7 (6 threads, test from 2^19 to 2^25)

# Define remote host IPs and users (Modify according to actual situation)
HOST1="node1"
HOST2="node2"
PROJECT_PATH="/root/Qamboo"  # Replace with the absolute path of the project on remote machines
BIN_PATH="./target/release/radix_sort_scalability"

NUM_THREADS=${1:-6} # number of threads (default: 6)
NUM=${2:-7}         # test number, 2^19(n=1) to 2^25(n=7) (default: 7)

cd $PROJECT_PATH

# Build radix_sort_scalability binary and copy to remote hosts
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort_scalability --features tcp
scp target/release/radix_sort_scalability $HOST1:$PROJECT_PATH/target/release/
scp target/release/radix_sort_scalability $HOST2:$PROJECT_PATH/target/release/

# Run party 0 locally
$BIN_PATH -c experiments/net/multinode/ -p 0 -t $NUM_THREADS -n $NUM &
# SSH to HOST1 to run party 1
ssh $HOST1 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 1 -t $NUM_THREADS -n $NUM" &
# SSH to HOST2 to run party 2
ssh $HOST2 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 2 -t $NUM_THREADS -n $NUM" &

wait
