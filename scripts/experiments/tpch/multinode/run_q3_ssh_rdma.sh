#!/bin/bash

# Define remote host IPs and users (Modify according to actual situation)
PROJECT_PATH="/root/Qamboo"  # Replace with the absolute path of the project on remote machines
BIN_PATH="smc_run ./target/release/q3"

NUM_COMMTHREADS=${1:-6} #number of threads (default: 6)
SF=${2:-0.01} # scale factor (default: 0.01)
HOST1=${3:-node1}       # first remote host (default: node1)
HOST2=${4:-node2}       # second remote host (default: node2)

cd $PROJECT_PATH

# Build q3 binary and copy to remote hosts
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin q3 --features tcp
scp target/release/q3 $HOST1:$PROJECT_PATH/target/release/
scp target/release/q3 $HOST2:$PROJECT_PATH/target/release/

# Run party 0 locally
$BIN_PATH -c experiments/net/multinode/ -p 0 -t $NUM_COMMTHREADS -s $SF &
# SSH to HOST1 to run party 1
ssh $HOST1 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 1 -t $NUM_COMMTHREADS -s $SF" &
# SSH to HOST2 to run party 2
ssh $HOST2 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 2 -t $NUM_COMMTHREADS -s $SF" &

wait