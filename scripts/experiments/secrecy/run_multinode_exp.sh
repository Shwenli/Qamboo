#!/bin/bash

# ==============================================================================
# Usage: ./run_multinode_exp.sh [NUM_THREADS] [SF] [QUERIES]
# Examples:
#   ./run_multinode_exp.sh 4 0.01 "aspirin,credit"     -> Run aspirin and credit
#   ./run_multinode_exp.sh 4 0.1 "all"                 -> Run all queries
#   ./run_multinode_exp.sh 6 1 "comorbidity"           -> Run comorbidity only
# ==============================================================================

# This script is used to run Secrecy queries on multiple machines using SSH.

# Enable pipefail so that the exit status of the command in the pipeline is preserved
set -o pipefail

# Check if all required arguments are provided
if [ "$#" -lt 3 ]; then
    echo "Error: Missing required arguments."
    echo "Usage: $0 <NUM_THREADS> <SF> <QUERIES>"
    echo "  <NUM_THREADS> : Number of communication threads (e.g. 4)"
    echo "  <SF>          : Scale Factor (e.g. 0.01 or 0.1)"
    echo "  <QUERIES>     : Queries to run (e.g. \"aspirin,credit\" or \"all\")"
    echo "  Available queries: aspirin, comorbidity, credit, pwd, rcdiff"
    echo "Example: $0 4 0.01 \"aspirin,credit\""
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
# Current dir: .../scripts/experiments/secrecy
# Goal: .../experiments/result/secrecy_query/multinode/stat_output.log
LOG_FILE="$CURRENT_DIR/../../../experiments/result/secrecy_query/multinode/stat_output.log"

# Create the directory if it doesn't exist
mkdir -p "$(dirname "$LOG_FILE")"

# Check if multinode directory exists
if [ ! -d "$MULTINODE_SCRIPT_DIR" ]; then
    echo "Error: Directory '$MULTINODE_SCRIPT_DIR' not found."
    exit 1
fi

# Define all available queries
ALL_QUERIES=("aspirin" "comorbidity" "credit" "pwd" "rcdiff")

# Parse Query input
QUERY_LIST=()

if [ "$QUERY_INPUT" == "all" ]; then
    QUERY_LIST=("${ALL_QUERIES[@]}")
else
    # Split comma-separated values
    IFS=',' read -r -a QUERY_LIST <<< "$QUERY_INPUT"
fi

echo "============================================================"
echo "Starting Secrecy Multinode SSH Experiments"
echo "Comm Threads: $NUM_THREADS | Scale Factor: $SF"
echo "Queries: ${QUERY_LIST[*]}"
echo "============================================================"

# 2. Loop through queries
for query in "${QUERY_LIST[@]}"; do
    SCRIPT_NAME="run_${query}_ssh.sh"
    SCRIPT_PATH="$MULTINODE_SCRIPT_DIR/$SCRIPT_NAME"

    echo ""
    echo ">>> Running Secrecy Query '$query' (Multinode SSH) ..."

    if [ -f "$SCRIPT_PATH" ]; then
        # Grant execution permission (optional)
        chmod +x "$SCRIPT_PATH"
        
        # Execute script with Threads ($1) and SF ($2)
        (cd "$MULTINODE_SCRIPT_DIR" && ./$SCRIPT_NAME "$NUM_THREADS" "$SF") 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE")
        
        if [ $? -eq 0 ]; then
            echo ">>> Query '$query' Finished Successfully."
        else
            echo ">>> Query '$query' Failed."
        fi
    else
        echo ">>> Warning: Script $SCRIPT_PATH not found, skipping."
    fi
    
    # Optional: Cool down to ensure ports are released
    sleep 2
done

echo ""
echo "============================================================"
echo "All Secrecy Multinode Experiments Completed."
echo "============================================================"
