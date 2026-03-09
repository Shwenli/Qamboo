#!/bin/bash

# ==============================================================================
# Usage: ./run_multinode_exp.sh [NUM_COMMTHREADS] [SF] [QUERIES]
# Examples:
#   ./run_multinode_exp.sh 6 1 "2,3,5"        -> Run Q2, Q3, Q5
#   ./run_multinode_exp.sh 6 0.1 "2..21"      -> Run all available queries
#   ./run_multinode_exp.sh 12 1 "3"           -> Run Q3 only
# ==============================================================================

# This script is used to run No Secure Cut queries on multiple machines using SSH.

# Enable pipefail so that the exit status of the command in the pipeline is preserved
set -o pipefail

# Check if all required arguments are provided
if [ "$#" -lt 3 ]; then
    echo "Error: Missing required arguments."
    echo "Usage: $0 <NUM_COMMTHREADS> <SF> <QUERIES>"
    echo "  <NUM_COMMTHREADS> : Number of communication threads (e.g. 6)"
    echo "  <SF>          : Scale Factor (e.g. 0.1 or 1)"
    echo "  <QUERIES>     : Queries to run (e.g. \"2,3,5\" or \"2..21\")"
    echo "  Available queries: 2, 3, 5, 8, 13, 17, 18, 20, 21"
    echo "Example: $0 6 1 \"2,3,5\""
    exit 1
fi

# 1. Set values from arguments
NUM_COMMTHREADS=$1
SF=$2
QUERY_INPUT=$3

# Use absolute path for safety
CURRENT_DIR=$(pwd)

# Script directory for multinode SSH scripts (without RDMA)
MULTINODE_SCRIPT_DIR="./multinode"

# Calculate absolute path for LOG_FILE
# Current dir: .../scripts/experiments/optimization/no_secure_cut
# Goal: .../experiments/result/query_optimization/no_secure_cut/multinode/stat_output.log
LOG_FILE="$CURRENT_DIR/../../../../experiments/result/query_optimization/no_secure_cut/multinode/stat_output.log"

# Create the directory if it doesn't exist
mkdir -p "$(dirname "$LOG_FILE")"

# Check if multinode directory exists
if [ ! -d "$MULTINODE_SCRIPT_DIR" ]; then
    echo "Error: Directory '$MULTINODE_SCRIPT_DIR' not found."
    exit 1
fi

echo "============================================================"
echo "Starting No Secure Cut Multinode SSH Experiments"
echo "Comm Threads: $NUM_COMMTHREADS | Scale Factor: $SF | Queries: $QUERY_INPUT"
echo "============================================================"

# 2. Parse Query input (supports comma: "2,3,5" and range: "2..21")
QUERY_LIST=()

# Split comma-separated values
IFS=',' read -r -a RAW_PARTS <<< "$QUERY_INPUT"

for part in "${RAW_PARTS[@]}"; do
    if [[ "$part" == *".."* ]]; then
        # Handle range format, e.g. "2..21"
        start=$(echo $part | cut -d'.' -f1)
        end=$(echo $part | cut -d'.' -f3)
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
    SCRIPT_NAME="run_q${q}_no_secure_cut_ssh.sh"
    SCRIPT_PATH="$MULTINODE_SCRIPT_DIR/$SCRIPT_NAME"

    echo ""
    echo ">>> Running No Secure Cut Query $q (Multinode SSH) ..."

    if [ -f "$SCRIPT_PATH" ]; then
        # Grant execution permission (optional)
        chmod +x "$SCRIPT_PATH"
        
        # Execute script with Threads ($1) and SF ($2)
        (cd "$MULTINODE_SCRIPT_DIR" && ./$SCRIPT_NAME "$NUM_COMMTHREADS" "$SF") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
        
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
echo "All No Secure Cut Multinode Experiments Completed."
echo "============================================================"
