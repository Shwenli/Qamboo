#!/bin/bash

# ==============================================================================
# Usage: ./run_multinode_exp.sh [NUM_THREADS] [SF] [QUERIES]
# Examples:
#   ./run_multinode_exp.sh 6 1 "1,3,4"        -> Run Q1, Q3, Q4
#   ./run_multinode_exp.sh 6 0.1 "1..8"       -> Run Q1 to Q8
#   ./run_multinode_exp.sh 12 1 "3"           -> Run Q3 only
# ==============================================================================

# This script is used to run TPC-H queries on multiple machines using SSH.

# Enable pipefail so that the exit status of the command in the pipeline is preserved
set -o pipefail

# Check if all required arguments are provided
if [ "$#" -lt 3 ]; then
    echo "Error: Missing required arguments."
    echo "Usage: $0 <NUM_THREADS> <SF> <QUERIES>"
    echo "  <NUM_THREADS> : Number of threads (e.g. 6)"
    echo "  <SF>          : Scale Factor (e.g. 0.1 or 1)"
    echo "  <QUERIES>     : Queries to run (e.g. \"1,3,4\" or \"1..8\")"
    echo "Example: $0 6 1 \"1,3,4\""
    exit 1
fi

# 1. Set values from arguments
NUM_THREADS=$1
SF=$2
QUERY_INPUT=$3

# Use absolute path for safety
CURRENT_DIR=$(pwd)

# Script directory for multinode SSH scripts (without RDMA)
MULTINODE_SCRIPT_DIR="./multinode"

# Calculate absolute path for LOG_FILE
# Current dir: .../scripts/experiments/tpch
# Goal: .../experiments/result/tpch_query/multinode/stat_output.log
LOG_FILE="$CURRENT_DIR/../../../experiments/result/tpch_query/multinode/stat_output.log"

# Create the directory if it doesn't exist
mkdir -p "$(dirname "$LOG_FILE")"

# Check if multinode directory exists
if [ ! -d "$MULTINODE_SCRIPT_DIR" ]; then
    echo "Error: Directory '$MULTINODE_SCRIPT_DIR' not found."
    exit 1
fi

echo "============================================================"
echo "Starting Multinode SSH Experiments"
echo "Threads: $NUM_THREADS | Scale Factor: $SF | Queries: $QUERY_INPUT"
echo "============================================================"

# 2. Parse Query input (supports comma: "1,3,4" and range: "1..5")
QUERY_LIST=()

# Split comma-separated values
IFS=',' read -r -a RAW_PARTS <<< "$QUERY_INPUT"

for part in "${RAW_PARTS[@]}"; do
    if [[ "$part" == *".."* ]]; then
        # Handle range format, e.g. "1..5"
        start=$(echo $part | cut -d'.' -f1)
        end=$(echo $part | cut -d'.' -f3) # f3 because ".." creates an empty field between dots
        for ((i=start; i<=end; i++)); do
            QUERY_LIST+=($i)
        done
    else
        # Handle single number
        QUERY_LIST+=($part)
    fi
done

# 3. Loop through queries
for q in "${QUERY_LIST[@]}"; do
    SCRIPT_NAME="run_q${q}_ssh.sh"
    SCRIPT_PATH="$MULTINODE_SCRIPT_DIR/$SCRIPT_NAME"

    echo ""
    echo ">>> Running TPC-H Query $q (Multinode SSH) ..."

    if [ -f "$SCRIPT_PATH" ]; then
        # Grant execution permission (optional)
        chmod +x "$SCRIPT_PATH"
        
        # Execute script with Threads ($1) and SF ($2)
        # Note: The multinode scripts now also accept arguments
        (cd "$MULTINODE_SCRIPT_DIR" && ./$SCRIPT_NAME "$NUM_THREADS" "$SF") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
        
        if [ $? -eq 0 ]; then
            echo ">>> Query $q Finished Successfully."
        else
            echo ">>> Query $q Failed."
        fi
    else
        echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
    fi
    
    # Optional: Cool down to ensure ports are released
    sleep 2
done

echo ""
echo "============================================================"
echo "All Multinode Experiments Completed."
echo "============================================================"
