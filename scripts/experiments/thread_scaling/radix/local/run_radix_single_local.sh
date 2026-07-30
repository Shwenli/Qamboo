#!/bin/bash

# Single-thread Radix Sort (Local)
# Usage: ./run_radix_single_local.sh [SHIFT]

# Get project root (find Cargo.toml location)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR/../../../../..")"
BIN_PATH="$PROJECT_PATH/target/release/radix_sort_single"

SHIFT=${1:-20}

cd "$PROJECT_PATH" || exit 1

# Build binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort_single --features tcp

# Run all 3 parties locally
"$BIN_PATH" -c experiments/net/local/ -p 0 -s "$SHIFT" &
"$BIN_PATH" -c experiments/net/local/ -p 1 -s "$SHIFT" &
"$BIN_PATH" -c experiments/net/local/ -p 2 -s "$SHIFT" &

wait
