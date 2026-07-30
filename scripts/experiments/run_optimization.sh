#!/bin/bash

# ==============================================================================
# Script Name: run_optimization.sh
# Description: Optimization ablation runner + thread-scaling sweeps.
#
# Usage:
#   ./run_optimization.sh <variant> <queries> [options]
#   ./run_optimization.sh thread_scaling <queries> -s <SF> [options]
#
# Variants (one optimization disabled):
#   no_secure_cut    queries: 2,3,5,8,13,17,18,20,21
#   no_join_reorder  queries: 2,5,7,8,9,10
#   no_semi          query:   4
#   thread_scaling   sweep RAYON_NUM_THREADS x NUM_COMMTHREADS over TPC-H
#                    queries (1..22); SF is required via -s
#
# Options:
#   -m MODE    : local | tcp | rdma  (default: local)
#   -t N       : communication threads (default: 6; ignored by thread_scaling)
#   -s X       : scale factor (default: 0.01)
#   -h HOSTS   : comma-separated host per party (default: node0,node1,node2;
#                tcp/rdma only; the local host runs in-process, others via SSH)
#   --rayon N  : RAYON_NUM_THREADS on all parties (fixed value, no sweep)
#   --log PATH : override stat log
#   --no-log   : disable stat logging
#
# Examples:
#   ./run_optimization.sh no_secure_cut "2,3,5" -t 6 -s 1 -m tcp -h node0,node1,node2
#   ./run_optimization.sh no_join_reorder "2..10" -t 32 -s 1 -m tcp
#   ./run_optimization.sh no_semi 4 -t 6 -s 0.1
#   ./run_optimization.sh thread_scaling "1,3,4" -s 1                 # local sweep
#   ./run_optimization.sh thread_scaling "1..8" -s 1 -m tcp           # multinode sweep
# ==============================================================================

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/run_common.sh"

VARIANT="${1:-}"
QUERIES="${2:-}"
if [ -z "$VARIANT" ] || [ -z "$QUERIES" ]; then
    sed -n '3,32p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
fi
shift 2

case "$VARIANT" in
    no_secure_cut)   VALID="2 3 5 8 13 17 18 20 21" ;;
    no_join_reorder) VALID="2 5 7 8 9 10" ;;
    no_semi)         VALID="4" ;;
    thread_scaling)  VALID="$(seq 1 22 | tr '\n' ' ')" ;;
    *) echo "Error: unknown variant '$VARIANT'" >&2; exit 1 ;;
esac

parse_common_opts "$@" || exit 1

read -r -a TARGETS <<< "$(expand_targets "$QUERIES" $VALID)"
for t in "${TARGETS[@]}"; do
    contains "$t" $VALID || { echo "Error: query '$t' not available for '$VARIANT' (valid: $VALID)" >&2; exit 1; }
done

if [ "$VARIANT" == "thread_scaling" ]; then
    # --------------------------------------------------------------------------
    # Thread scaling: sweep RAYON_NUM_THREADS x NUM_COMMTHREADS
    # --------------------------------------------------------------------------
    if [ "$MODE" == "local" ]; then
        RAYON_LIST=(1 2 4 8 10)
        COMM_LIST=(1 2 4 8)
        RESULT_SUBDIR="thread_scaling_local"
        LOG_PREFIX="thread_scaling_local"
    else
        RAYON_LIST=(1 2 4 8 16 32)
        COMM_LIST=(1 2 4 8 16)
        RESULT_SUBDIR="thread_scaling"
        LOG_PREFIX="thread_scaling"
    fi
    SF="${SF:?Error: thread_scaling requires -s <SF>}"

    if [ -n "$LOG_OVERRIDE" ]; then
        LOG_FILE="$LOG_OVERRIDE"
    else
        LOG_FILE="$PROJECT_ROOT/experiments/result/$RESULT_SUBDIR/${LOG_PREFIX}_sf${SF}.log"
    fi
    mkdir -p "$(dirname "$LOG_FILE")"

    BINS=()
    for t in "${TARGETS[@]}"; do BINS+=("q$t"); done

    echo "============================================================"
    echo "Thread Scaling Experiments (mode: $MODE)"
    echo "Scale Factor: $SF | Queries: ${TARGETS[*]}"
    echo "RAYON_NUM_THREADS: ${RAYON_LIST[@]}"
    echo "NUM_COMMTHREADS:   ${COMM_LIST[@]}"
    echo "Log: $LOG_FILE"
    echo "============================================================"

    echo "Thread Scaling Results ($MODE) - SF=$SF, Queries=${TARGETS[*]}" > "$LOG_FILE"
    echo "Started at: $(date)" >> "$LOG_FILE"
    echo "============================================================" >> "$LOG_FILE"

    total=$((${#RAYON_LIST[@]} * ${#COMM_LIST[@]}))
    current=0
    for RAYON in "${RAYON_LIST[@]}"; do
        for THREADS in "${COMM_LIST[@]}"; do
            ((current++))
            echo ""
            echo ">>> [${current}/${total}] RAYON_NUM_THREADS=$RAYON, NUM_COMMTHREADS=$THREADS"
            echo ">>> [${current}/${total}] RAYON_NUM_THREADS=$RAYON, NUM_COMMTHREADS=$THREADS" >> "$LOG_FILE"

            for i in "${!TARGETS[@]}"; do
                q="${TARGETS[$i]}"
                bin="${BINS[$i]}"
                echo "    Running Q${q}..."
                echo "    Q${q}:" >> "$LOG_FILE"

                if run_one "$bin" 2>&1 | tee >(perl -pe 's/\x1b\[[0-9;]*m//g' | grep --line-buffered "INFO Total" >> "$LOG_FILE"); then
                    echo "    Q${q} OK" >> "$LOG_FILE"
                else
                    echo "    Q${q} FAILED" >> "$LOG_FILE"
                fi

                sleep 1
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
    exit 0
fi

# ------------------------------------------------------------------------------
# Standard ablation run
# ------------------------------------------------------------------------------
THREADS="${THREADS:-6}"
SF="${SF:-0.01}"

BINS=()
for t in "${TARGETS[@]}"; do BINS+=("q${t}_${VARIANT}"); done

setup_log "query_optimization/$VARIANT"
run_all "Optimization: $VARIANT"
