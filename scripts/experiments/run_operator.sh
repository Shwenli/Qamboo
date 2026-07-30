#!/bin/bash

# ==============================================================================
# Script Name: run_operator.sh
# Description: Operator-level micro-benchmark runner.
#
# Usage:
#   ./run_operator.sh <benches> [options]
#
#   <benches> : "radix_sort,multi_keys_join" or "all"
#               (multi_keys_join radix_sort radix_sort_scalability
#                radix_sort_single radix_sort_multi)
#
# Options:
#   -m MODE    : local | tcp | rdma  (default: local)
#   -t N       : communication threads (default: 6)
#   -s X       : shift for radix_sort (default: 20)
#   -n N       : test number for radix_sort_scalability (default: 7)
#   -h HOSTS   : comma-separated host per party (default: node0,node1,node2;
#                tcp/rdma only; the local host runs in-process, others via SSH)
#   --rayon N  : RAYON_NUM_THREADS on all parties
#   --log PATH : override stat log
#   --no-log   : disable stat logging
#
# Examples:
#   ./run_operator.sh multi_keys_join
#   ./run_operator.sh radix_sort -t 6 -s 20 -m tcp -h node0,node1,node2
#   ./run_operator.sh radix_sort_scalability -t 6 -n 7
#
# Note: the MP-SPDZ radix-sort baseline is a standalone script:
#   ./run_radix_sort_mpspdz_ssh.sh
# ==============================================================================

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/run_common.sh"

BENCHES="${1:-}"
if [ -z "$BENCHES" ]; then
    sed -n '3,28p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
fi
shift

parse_common_opts "$@" || exit 1
THREADS="${THREADS:-6}"
SF="${SF:-20}"

ALL_BENCHES=("multi_keys_join" "radix_sort" "radix_sort_scalability" "radix_sort_single" "radix_sort_multi")
read -r -a TARGETS <<< "$(expand_targets "$BENCHES" "${ALL_BENCHES[@]}")"
for t in "${TARGETS[@]}"; do
    contains "$t" "${ALL_BENCHES[@]}" || { echo "Error: invalid operator bench '$t' (valid: ${ALL_BENCHES[*]})" >&2; exit 1; }
done

BINS=("${TARGETS[@]}")

setup_log "operator"
run_all "Operator"
