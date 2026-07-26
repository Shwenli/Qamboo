#!/bin/bash

# Thread scaling version for LOCAL execution (single machine, 3 parties)

# Auto-detect project root (5 levels up from script location)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"
BIN_PATH="$PROJECT_PATH/target/release/q9"

NUM_COMMTHREADS=${1:-6}
SF=${2:-0.01}

RAYON_T=${RAYON_NUM_THREADS:-$(nproc)}

cd $PROJECT_PATH

# Build binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin q9 --features tcp

# Run all 3 parties locally with RAYON_NUM_THREADS
export RAYON_NUM_THREADS=$RAYON_T

$BIN_PATH -c experiments/net/local/ -p 0 -t $NUM_COMMTHREADS -s $SF &
$BIN_PATH -c experiments/net/local/ -p 1 -t $NUM_COMMTHREADS -s $SF &
$BIN_PATH -c experiments/net/local/ -p 2 -t $NUM_COMMTHREADS -s $SF &

wait
