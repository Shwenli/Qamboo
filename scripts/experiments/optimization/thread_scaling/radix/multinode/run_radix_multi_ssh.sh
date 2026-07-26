#!/bin/bash

# Multi-thread Radix Sort (Multinode SSH)
# Usage: ./run_radix_multi_ssh.sh [NUM_COMMTHREADS] [SHIFT] [HOST1] [HOST2]

PROJECT_PATH="/root/Qamboo"
BIN_PATH="$PROJECT_PATH/target/release/radix_sort_multi"

NUM_COMMTHREADS=${1:-6}
SHIFT=${2:-20}
HOST1=${3:-node1}
HOST2=${4:-node2}

RAYON_T=${RAYON_NUM_THREADS:-$(nproc)}

cd $PROJECT_PATH

# Build binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort_multi --features tcp

# Copy to remote hosts
scp target/release/radix_sort_multi $HOST1:$PROJECT_PATH/target/release/
scp target/release/radix_sort_multi $HOST2:$PROJECT_PATH/target/release/

# Run with RAYON_NUM_THREADS
export RAYON_NUM_THREADS=$RAYON_T

# Run party 0 locally
RAYON_NUM_THREADS=$RAYON_T $BIN_PATH -c experiments/net/multinode/ -p 0 -t $NUM_COMMTHREADS -s $SHIFT &
# Run party 1 on HOST1
ssh $HOST1 "cd $PROJECT_PATH && export RAYON_NUM_THREADS=$RAYON_T && $BIN_PATH -c experiments/net/multinode/ -p 1 -t $NUM_COMMTHREADS -s $SHIFT" &
# Run party 2 on HOST2
ssh $HOST2 "cd $PROJECT_PATH && export RAYON_NUM_THREADS=$RAYON_T && $BIN_PATH -c experiments/net/multinode/ -p 2 -t $NUM_COMMTHREADS -s $SHIFT" &

wait
