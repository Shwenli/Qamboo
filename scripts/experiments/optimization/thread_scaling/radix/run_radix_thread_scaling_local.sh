#!/bin/bash

# ==============================================================================
# Radix Sort Thread Scaling Experiments (Local)
# Usage: ./run_radix_thread_scaling_local.sh [SHIFT]
#
# Tests:
#   - Single-thread: radix_sort (1 config)
#   - Multi-thread: radix_sort_multithreads with varying RAYON_NUM_THREADS
#
# Thread configs:
#   - RAYON_NUM_THREADS: 2, 4, 8, 10
#   - NUM_COMMTHREADS: 2, 4, 8
# ==============================================================================

set -o pipefail

# Default values (2^20 = 1,048,576 rows for local)
SHIFT=${1:-20}

# Thread configurations (local optimized)
RAYON_THREADS_LIST=(2 4 8 10)
COMM_THREADS_LIST=(2 4 8)

# Parse arguments
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <SHIFT>"
    echo "  <SHIFT>       : 2^SHIFT rows (default: 20, i.e., 1M rows)"
    exit 1
fi

CURRENT_DIR=$(pwd)
LOCAL_DIR="./local"
RESULT_DIR="$CURRENT_DIR/../../../../../../experiments/result/radix_thread_scaling_local"
mkdir -p "$RESULT_DIR"

LOG_FILE="$RESULT_DIR/radix_local_shift${SHIFT}.log"
ROWS=$((1 << SHIFT))

echo "============================================================"
echo "Radix Sort Thread Scaling (Local)"
echo "Rows: 2^${SHIFT} = ${ROWS}"
echo "============================================================"
echo "Single-thread: 1 config"
echo "Multi-thread: RAYON=${RAYON_THREADS_LIST[*]}, COMM=${COMM_THREADS_LIST[*]}"
echo "============================================================"
echo ""

# Initialize log
echo "Radix Sort Thread Scaling (Local) - 2^${SHIFT} rows" > "$LOG_FILE"
echo "Started at: $(date)" >> "$LOG_FILE"
echo "============================================================" >> "$LOG_FILE"

# ==================== Single-thread Test ====================
echo ">>> Testing Single-thread Radix Sort"
echo ">>> Single-thread Radix Sort" >> "$LOG_FILE"

SCRIPT_PATH="$LOCAL_DIR/run_radix_single_local.sh"
if [ -f "$SCRIPT_PATH" ]; then
    chmod +x "$SCRIPT_PATH"
    echo "    Running single-thread version..."
    (cd "$LOCAL_DIR" && ./run_radix_single_local.sh "$SHIFT") 2>&1 | tee >(grep --line-buffered "INFO Total" >> "$LOG_FILE")
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
        
        SCRIPT_PATH="$LOCAL_DIR/run_radix_multi_local.sh"
        if [ -f "$SCRIPT_PATH" ]; then
            chmod +x "$SCRIPT_PATH"
            echo "    Running multi-thread version..."
            (cd "$LOCAL_DIR" && RAYON_NUM_THREADS=$RAYON_T ./run_radix_multi_local.sh "$COMM_T" "$SHIFT") 2>&1 | tee >(grep --line-buffered "INFO Total" >> "$LOG_FILE")
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
echo "All Radix Sort Thread Scaling Experiments Completed (Local)."
echo "Results saved to: $LOG_FILE"
echo "============================================================"
