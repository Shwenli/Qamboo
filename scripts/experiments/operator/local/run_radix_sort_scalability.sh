#!/bin/bash
# Usage: ./run_radix_sort_scalability.sh <threads> <number>
# Example: ./run_radix_sort_scalability.sh 6 7 (6 threads, test from 2^19 to 2^25)

RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin radix_sort_scalability --features tcp

BIN_PATH=../../../../target/release/radix_sort_scalability

NUM_THREADS=${1:-6} # number of threads (default: 6)
NUM=${2:-7}         # test number, 2^19(n=1) to 2^25(n=7) (default: 7)

$BIN_PATH -c ../../../../experiments/net/local/ -p 0 -t $NUM_THREADS -n $NUM &
$BIN_PATH -c ../../../../experiments/net/local/ -p 1 -t $NUM_THREADS -n $NUM &
$BIN_PATH -c ../../../../experiments/net/local/ -p 2 -t $NUM_THREADS -n $NUM &

wait
