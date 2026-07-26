#!/bin/bash

# ==============================================================================
# Usage: ./run_thread_scaling_local_exp.sh [SF] [QUERIES]
# 
# This script runs TPC-H queries LOCALLY with different combinations of:
#   - RAYON_NUM_THREADS: Number of Rayon compute threads (data parallelism)
#   - NUM_COMMTHREADS: Number of communication threads (I/O parallelism)
#
# Examples:
#   ./run_thread_scaling_local_exp.sh 1 "1,3,4"      -> Run Q1, Q3, Q4 at SF=1
#   ./run_thread_scaling_local_exp.sh 0.1 "1..8"     -> Run Q1 to Q8 at SF=0.1
# ==============================================================================

set -o pipefail

# Default values
SF=${1:-1}
QUERY_INPUT=${2:-"1,3,5,6,10"}

# Thread configurations to test (optimized for local execution)
RAYON_THREADS_LIST=(1 2 4 8 10)
COMM_THREADS_LIST=(1 2 4 8)

# Parse arguments
if [ "$#" -lt 2 ]; then
    echo "Error: Missing required arguments."
    echo "Usage: $0 <SF> <QUERIES>"
    echo "  <SF>          : Scale Factor (e.g. 0.1 or 1)"
    echo "  <QUERIES>     : Queries to run (e.g. \"1,3,4\" or \"1..8\")"
    echo ""
    echo "Thread configurations:"
    echo "  RAYON_NUM_THREADS: ${RAYON_THREADS_LIST[@]}"
    echo "  NUM_COMMTHREADS  : ${COMM_THREADS_LIST[@]}"
    exit 1
fi

CURRENT_DIR=$(pwd)
LOCAL_SCRIPT_DIR="./local"

# Result directory for thread scaling experiments
RESULT_DIR="$CURRENT_DIR/../../../../experiments/result/thread_scaling_local"
mkdir -p "$RESULT_DIR"

LOG_FILE="$RESULT_DIR/thread_scaling_local_sf${SF}.log"

echo "============================================================"
echo "Thread Scaling Experiments (LOCAL)"
echo "Scale Factor: $SF | Queries: $QUERY_INPUT"
echo "============================================================"
echo "RAYON_NUM_THREADS: ${RAYON_THREADS_LIST[@]}"
echo "NUM_COMMTHREADS:   ${COMM_THREADS_LIST[@]}"
echo "============================================================"
echo ""

# Clear previous log
echo "Thread Scaling Results (LOCAL) - SF=$SF, Queries=$QUERY_INPUT" > "$LOG_FILE"
echo "Started at: $(date)" >> "$LOG_FILE"
echo "============================================================" >> "$LOG_FILE"

# Parse Query input
QUERY_LIST=()
IFS=',' read -r -a RAW_PARTS <<< "$QUERY_INPUT"
for part in "${RAW_PARTS[@]}"; do
    if [[ "$part" == *".."* ]]; then
        start=$(echo $part | cut -d'.' -f1)
        end=$(echo $part | cut -d'.' -f3)
        for ((i=start; i<=end; i++)); do
            QUERY_LIST+=($i)
        done
    else
        QUERY_LIST+=($part)
    fi
done

# Main experiment loop
total_configs=$((${#RAYON_THREADS_LIST[@]} * ${#COMM_THREADS_LIST[@]}))
current=0

for RAYON_T in "${RAYON_THREADS_LIST[@]}"; do
    for COMM_T in "${COMM_THREADS_LIST[@]}"; do
        ((current++))
        echo ""
        echo ">>> [${current}/${total_configs}] Testing RAYON_NUM_THREADS=$RAYON_T, NUM_COMMTHREADS=$COMM_T"
        echo ">>> [${current}/${total_configs}] RAYON_NUM_THREADS=$RAYON_T, NUM_COMMTHREADS=$COMM_T" >> "$LOG_FILE"
        
        # Run all queries for this configuration
        for q in "${QUERY_LIST[@]}"; do
            SCRIPT_NAME="run_q${q}_thread_scaling.sh"
            SCRIPT_PATH="$LOCAL_SCRIPT_DIR/$SCRIPT_NAME"
            
            if [ -f "$SCRIPT_PATH" ]; then
                chmod +x "$SCRIPT_PATH"
                
                echo "    Running Q${q}..."
                echo "    Q${q}:" >> "$LOG_FILE"
                
                # Run with specific thread configuration
                (cd "$LOCAL_SCRIPT_DIR" && RAYON_NUM_THREADS=$RAYON_T ./$SCRIPT_NAME "$COMM_T" "$SF") 2>&1 | tee >(grep --line-buffered "INFO Total" >> "$LOG_FILE")
                
                if [ $? -eq 0 ]; then
                    echo "    Q${q} OK" >> "$LOG_FILE"
                else
                    echo "    Q${q} FAILED" >> "$LOG_FILE"
                fi
                
                sleep 1
            else
                echo "    Warning: Script $SCRIPT_PATH not found, skipping."
                echo "    Q${q}: SCRIPT NOT FOUND" >> "$LOG_FILE"
            fi
        done
        
        echo "--------------------------------------------------------" >> "$LOG_FILE"
        sleep 2
    done
done

echo ""
echo "============================================================"
echo "All Thread Scaling Experiments Completed (LOCAL)."
echo "Results saved to: $LOG_FILE"
echo "============================================================"
