#!/bin/bash

# Multi-thread Radix Sort (Local)
# Usage: ./run_radix_multi_local.sh [NUM_COMMTHREADS] [SHIFT]

# Get project root (find Cargo.toml location)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR/../../../../..")"
BIN_PATH="$PROJECT_PATH/target/release/radix_sort_multi"

NUM_COMMTHREADS=${1:-6}
SHIFT=${2:-20}

RAYON_T=${RAYON_NUM_THREADS:-$(nproc)}

cd "$PROJECT_PATH" || exit 1

# Build binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort_multi --features tcp

# Run all 3 parties locally with RAYON_NUM_THREADS
export RAYON_NUM_THREADS=$RAYON_T

"$BIN_PATH" -c experiments/net/local/ -p 0 -t "$NUM_COMMTHREADS" -s "$SHIFT" &
"$BIN_PATH" -c experiments/net/local/ -p 1 -t "$NUM_COMMTHREADS" -s "$SHIFT" &
"$BIN_PATH" -c experiments/net/local/ -p 2 -t "$NUM_COMMTHREADS" -s "$SHIFT" &

wait
