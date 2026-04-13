#!/bin/bash

PROJECT_PATH="/root/Qamboo"
BIN_PATH="./target/release/q10"

NUM_COMMTHREADS=${1:-6}
SF=${2:-0.01}
HOST1=${3:-node1}
HOST2=${4:-node2}

RAYON_T=${RAYON_NUM_THREADS:-$(nproc)}

cd $PROJECT_PATH

# Build binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin q10 --features tcp

# Copy to remote hosts
scp target/release/q10 $HOST1:$PROJECT_PATH/target/release/
scp target/release/q10 $HOST2:$PROJECT_PATH/target/release/

# Run with RAYON_NUM_THREADS
export RAYON_NUM_THREADS=$RAYON_T

RAYON_NUM_THREADS=$RAYON_T $BIN_PATH -c experiments/net/multinode/ -p 0 -t $NUM_COMMTHREADS -s $SF &
ssh $HOST1 "cd $PROJECT_PATH && export RAYON_NUM_THREADS=$RAYON_T && $BIN_PATH -c experiments/net/multinode/ -p 1 -t $NUM_COMMTHREADS -s $SF" &
ssh $HOST2 "cd $PROJECT_PATH && export RAYON_NUM_THREADS=$RAYON_T && $BIN_PATH -c experiments/net/multinode/ -p 2 -t $NUM_COMMTHREADS -s $SF" &

wait
