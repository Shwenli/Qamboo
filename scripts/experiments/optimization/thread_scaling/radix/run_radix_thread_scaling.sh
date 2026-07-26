#!/bin/bash

# ==============================================================================
# Radix Sort Thread Scaling Experiments (Multinode)
# Usage: ./run_radix_thread_scaling.sh [SHIFT] [-h1 HOST1] [-h2 HOST2]
#
# Tests:
#   - Single-thread: radix_sort (1 config)
#   - Multi-thread: radix_sort_multithreads with varying RAYON_NUM_THREADS
#
# Thread configs:
#   - RAYON_NUM_THREADS: 2, 4, 8, 16, 32
#   - NUM_COMMTHREADS: 4, 8, 16, 32
# ==============================================================================

set -o pipefail

# Default values (2^23 = 8,388,608 rows for multinode)
HOST1="node1"
HOST2="node2"
SHIFT=${1:-23}

# Thread configurations
RAYON_THREADS_LIST=(2 4 8 16 32)
COMM_THREADS_LIST=(4 8 16 32)

# Parse arguments
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <SHIFT> [-h1 HOST1] [-h2 HOST2]"
    echo "  <SHIFT>       : 2^SHIFT rows (default: 20, i.e., 1M rows)"
    echo "  -h1 HOST1     : First remote host (default: node1)"
    echo "  -h2 HOST2     : Second remote host (default: node2)"
    exit 1
fi

shift 1

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
MULTINODE_DIR="./multinode"
RESULT_DIR="$CURRENT_DIR/../../../../../../experiments/result/radix_thread_scaling"
mkdir -p "$RESULT_DIR"

LOG_FILE="$RESULT_DIR/radix_multinode_shift${SHIFT}.log"
ROWS=$((1 << SHIFT))

echo "============================================================"
echo "Radix Sort Thread Scaling (Multinode)"
echo "Rows: 2^${SHIFT} = ${ROWS}"
echo "Host1: $HOST1 | Host2: $HOST2"
echo "============================================================"
echo "Single-thread: 1 config"
echo "Multi-thread: RAYON=${RAYON_THREADS_LIST[*]}, COMM=${COMM_THREADS_LIST[*]}"
echo "============================================================"
echo ""

# Initialize log
echo "Radix Sort Thread Scaling (Multinode) - 2^${SHIFT} rows" > "$LOG_FILE"
echo "Started at: $(date)" >> "$LOG_FILE"
echo "============================================================" >> "$LOG_FILE"

# ==================== Single-thread Test ====================
echo ">>> Testing Single-thread Radix Sort"
echo ">>> Single-thread Radix Sort" >> "$LOG_FILE"

SCRIPT_PATH="$MULTINODE_DIR/run_radix_single_ssh.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    echo "    Running single-thread version..."
    (cd "$MULTINODE_DIR" && ./run_radix_single_ssh.sh "$SHIFT" "$HOST1" "$HOST2") 2>&1 | tee >(grep --line-buffered "INFO Total" >> "$LOG_FILE")
    if [ $? -eq 0 ]; then
        echo "    Single-thread OK" >> "$LOG_FILE"
    else
        echo "    Single-thread FAILED" >> "$LOG_FILE"
    fi
else
    echo "    Warning: $SCRIPT_PATH not found"
fi

echo "--------------------------------------------------------" >> "$LOG_FILE"
sleep 2

# ==================== Multi-thread Tests ====================
total_configs=$((${#RAYON_THREADS_LIST[@]} * ${#COMM_THREADS_LIST[@]}))
current=0

for RAYON_T in "${RAYON_THREADS_LIST[@]}"; do
    for COMM_T in "${COMM_THREADS_LIST[@]}"; do
        ((current++))
        echo ""
        echo ">>> [${current}/${total_configs}] Multi-thread: RAYON_NUM_THREADS=$RAYON_T, NUM_COMMTHREADS=$COMM_T"
        echo ">>> [${current}/${total_configs}] RAYON_NUM_THREADS=$RAYON_T, NUM_COMMTHREADS=$COMM_T" >> "$LOG_FILE"
        
        SCRIPT_PATH="$MULTINODE_DIR/run_radix_multi_ssh.sh"
        if [ -f "$SCRIPT_PATH" ]; then
            chmod +x "$SCRIPT_PATH"
            echo "    Running multi-thread version..."
            (cd "$MULTINODE_DIR" && RAYON_NUM_THREADS=$RAYON_T ./run_radix_multi_ssh.sh "$COMM_T" "$SHIFT" "$HOST1" "$HOST2") 2>&1 | tee >(grep --line-buffered "INFO Total" >> "$LOG_FILE")
            if [ $? -eq 0 ]; then
                echo "    Multi-thread OK" >> "$LOG_FILE"
            else
                echo "    Multi-thread FAILED" >> "$LOG_FILE"
            fi
        else
            echo "    Warning: $SCRIPT_PATH not found"
        fi
        
        echo "--------------------------------------------------------" >> "$LOG_FILE"
        sleep 2
    done
done

echo ""
echo "============================================================"
echo "All Radix Sort Thread Scaling Experiments Completed."
echo "Results saved to: $LOG_FILE"
echo "============================================================"
