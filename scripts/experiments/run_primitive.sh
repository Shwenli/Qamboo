#!/bin/bash

# ==============================================================================
# Script Name: run_primitive.sh
# Description: Primitive-level micro-benchmark runner.
#
# Usage:
#   ./run_primitive.sh <benches> [options]
#
#   <benches> : "mul_bench,compare_bench" or "all"
#               (mul_bench compare_bench)
#
# Options:
#   -m MODE    : local | tcp | rdma  (default: local)
#   -t N       : communication threads (default: 16)
#   -h HOSTS   : comma-separated host per party (default: node0,node1,node2;
#                tcp/rdma only; the local host runs in-process, others via SSH)
#   --rayon N  : RAYON_NUM_THREADS on all parties
#   --log PATH : override stat log
#   --no-log   : disable stat logging
#
# Each bench sweeps the input size internally from 2^16 to 2^25 rows.
#
# Examples:
#   ./run_primitive.sh mul_bench
#   ./run_primitive.sh compare_bench -t 6
#   ./run_primitive.sh all -t 6 -m tcp -h node0,node1,node2
# ==============================================================================

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/run_common.sh"

BENCHES="${1:-}"
if [ -z "$BENCHES" ]; then
    sed -n '4,25p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
fi
shift

parse_common_opts "$@" || exit 1
THREADS="${THREADS:-16}"

ALL_BENCHES=("mul_bench" "compare_bench")
read -r -a TARGETS <<< "$(expand_targets "$BENCHES" "${ALL_BENCHES[@]}")"
for t in "${TARGETS[@]}"; do
    contains "$t" "${ALL_BENCHES[@]}" || { echo "Error: invalid primitive bench '$t' (valid: ${ALL_BENCHES[*]})" >&2; exit 1; }
done

BINS=("${TARGETS[@]}")

setup_log "primitive"
run_all "Primitive"
