#!/bin/bash

# ==============================================================================
# Usage: ./run_thread_scaling_exp.sh [SF] [QUERIES] [-h1 HOST1] [-h2 HOST2]
# 
# This script runs TPC-H queries with different combinations of:
#   - RAYON_NUM_THREADS: Number of Rayon compute threads (data parallelism)
#   - NUM_COMMTHREADS: Number of communication threads (I/O parallelism)
#
# Examples:
#   ./run_thread_scaling_exp.sh 1 "1,3,4"                -> Run Q1, Q3, Q4 at SF=1
#   ./run_thread_scaling_exp.sh 0.1 "1..8"               -> Run Q1 to Q8 at SF=0.1
#   ./run_thread_scaling_exp.sh 1 "3" -h1 nodeA -h2 nodeB  -> Custom hosts
# ==============================================================================

set -o pipefail

# Default values
HOST1="node1"
HOST2="node2"

# Thread configurations to test
RAYON_THREADS_LIST=(1 2 4 8 16 32)
COMM_THREADS_LIST=(1 2 4 8 16)

# Parse arguments
if [ "$#" -lt 2 ]; then
    echo "Error: Missing required arguments."
    echo "Usage: $0 <SF> <QUERIES> [-h1 HOST1] [-h2 HOST2]"
    echo "  <SF>          : Scale Factor (e.g. 0.1 or 1)"
    echo "  <QUERIES>     : Queries to run (e.g. \"1,3,4\" or \"1..8\")"
    echo "  -h1 HOST1     : First remote host (default: node1)"
    echo "  -h2 HOST2     : Second remote host (default: node2)"
    echo ""
    echo "Thread configurations:"
    echo "  RAYON_NUM_THREADS: ${RAYON_THREADS_LIST[@]}"
    echo "  NUM_COMMTHREADS  : ${COMM_THREADS_LIST[@]}"
    exit 1
fi

SF=$1
QUERY_INPUT=$2
shift 2

# Parse optional -h1 and -h2 arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h1)
            HOST1="$2"
            shift 2
            ;;
        -h2)
            HOST2="$2"
            shift 2
            ;;
        *)
            echo "Warning: Unknown option $1"
            shift
            ;;
    esac
done

CURRENT_DIR=$(pwd)
MULTINODE_SCRIPT_DIR="./multinode"

# Result directory for thread scaling experiments
RESULT_DIR="$CURRENT_DIR/../../../../experiments/result/thread_scaling"
mkdir -p "$RESULT_DIR"

LOG_FILE="$RESULT_DIR/thread_scaling_sf${SF}.log"

echo "============================================================"
echo "Thread Scaling Experiments"
echo "Scale Factor: $SF | Queries: $QUERY_INPUT"
echo "Host1: $HOST1 | Host2: $HOST2"
echo "============================================================"
echo "RAYON_NUM_THREADS: ${RAYON_THREADS_LIST[@]}"
echo "NUM_COMMTHREADS:   ${COMM_THREADS_LIST[@]}"
echo "============================================================"
echo ""

# Clear previous log
echo "Thread Scaling Results - SF=$SF, Queries=$QUERY_INPUT" > "$LOG_FILE"
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
            SCRIPT_PATH="$MULTINODE_SCRIPT_DIR/$SCRIPT_NAME"
            
            if [ -f "$SCRIPT_PATH" ]; then
                chmod +x "$SCRIPT_PATH"
                
                echo "    Running Q${q}..."
                echo "    Q${q}:" >> "$LOG_FILE"
                
                # Run with specific thread configuration
                (cd "$MULTINODE_SCRIPT_DIR" && RAYON_NUM_THREADS=$RAYON_T ./$SCRIPT_NAME "$COMM_T" "$SF" "$HOST1" "$HOST2") 2>&1 | tee >(grep --line-buffered "INFO Total" >> "$LOG_FILE")
                
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
echo "All Thread Scaling Experiments Completed."
echo "Results saved to: $LOG_FILE"
echo "============================================================"
