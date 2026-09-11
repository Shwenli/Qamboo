#!/bin/bash

# ==============================================================================
# Script Name: run_tpch.sh
# Description: TPC-H benchmark runner (Q1-Q22).
#
# Usage:
#   ./run_tpch.sh <queries> [options]
#
#   <queries>  : "1..22", "1,3,5", "all", or suffixed variants like "6_orq", "6_orq_bit", "6_orq_ari_valid"
#
# Options:
#   -m MODE    : local | tcp | rdma  (default: local)
#   -t N       : communication threads (default: 6)
#   -s X       : scale factor (default: 0.01)
#   -h HOSTS   : comma-separated host per party (default: node0,node1,node2;
#                tcp/rdma only; the local host runs in-process, others via SSH)
#   --rayon N  : RAYON_NUM_THREADS on all parties
#   --log PATH : override stat log
#   --no-log   : disable stat logging
#
# Examples:
#   ./run_tpch.sh "1..8" -t 6 -s 0.1
#   ./run_tpch.sh "6_orq" -s 1
#   ./run_tpch.sh "1..22" -t 32 -s 1 -m tcp -h node0,node1,node2
#   ./run_tpch.sh "1..22" -t 32 -s 1 -m rdma
# ==============================================================================

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/run_common.sh"

QUERIES="${1:-}"
if [ -z "$QUERIES" ]; then
    sed -n '3,25p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
fi
shift

parse_common_opts "$@" || exit 1
THREADS="${THREADS:-6}"
SF="${SF:-0.01}"

read -r -a TARGETS <<< "$(expand_targets "$QUERIES" $(seq 1 22))"
for t in "${TARGETS[@]}"; do
    contains "$t" $(seq 1 22) && continue
    # allow suffixed variants like "6_orq", "6_orq_bit", "6_orq_ari_valid": the numeric prefix must be a valid query
    if [[ "$t" =~ ^([0-9]+)_orq(_bit|_ari_valid)?$ ]] && contains "${BASH_REMATCH[1]}" $(seq 1 22); then
        continue
    fi
    echo "Error: invalid TPC-H query '$t' (valid: 1..22, optionally suffixed with _orq, _orq_bit or _orq_ari_valid)" >&2
    exit 1
done

BINS=()
for t in "${TARGETS[@]}"; do BINS+=("q$t"); done

setup_log "tpch_query"
run_all "TPC-H"
