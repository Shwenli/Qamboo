#!/bin/bash

# ==============================================================================
# Usage: ./run_local_exp.sh [NUM_THREADS] [SF] [QUERIES]
# Examples:
#   ./run_local_exp.sh 6 1 "1,3,4"        -> Run Q1, Q3, Q4
#   ./run_local_exp.sh 6 0.1 "1..8"       -> Run Q1 to Q8
#   ./run_local_exp.sh 12 1 "3"           -> Run Q3 only
# ==============================================================================

# This script is used to run TPC-H queries locally on a single machine.

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

LOCAL_SCRIPT_DIR="./local"

# Calculate absolute path for LOG_FILE to avoid "cd" confusion
# Current dir: .../scripts/experiments/tpch
# Goal: .../experiments/result/tpch_query/local/stat_output.log
LOG_FILE="$CURRENT_DIR/../../../experiments/result/tpch_query/local/stat_output.log"

# Create the directory if it doesn't exist
mkdir -p "$(dirname "$LOG_FILE")"

# Check if local directory exists
if [ ! -d "$LOCAL_SCRIPT_DIR" ]; then
    echo "Error: Directory '$LOCAL_SCRIPT_DIR' not found."
    exit 1
fi

echo "============================================================"
echo "Starting Local Experiments"
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
    SCRIPT_NAME="run_q${q}.sh"
    SCRIPT_PATH="$LOCAL_SCRIPT_DIR/$SCRIPT_NAME"

    echo ""
    echo ">>> Running TPC-H Query $q ..."

    if [ -f "$SCRIPT_PATH" ]; then
        # Grant execution permission (optional)
        chmod +x "$SCRIPT_PATH"
        
        # Execute script with Threads ($1) and SF ($2)
        # Note: The local/run_q*.sh scripts expect $1=NUM_THREADS and $2=SF
        (cd "$LOCAL_SCRIPT_DIR" && ./$SCRIPT_NAME "$NUM_THREADS" "$SF") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
        
        if [ $? -eq 0 ]; then
            echo ">>> Query $q Finished Successfully."
        else
            echo ">>> Query $q Failed."
            # Uncomment the line below to stop on error
            # exit 1 
        fi
    else
        echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
    fi
    
    # Optional: Cool down to ensure ports are released
    sleep 1
done

echo ""
echo "============================================================"
echo "All Experiments Completed."
echo "============================================================"