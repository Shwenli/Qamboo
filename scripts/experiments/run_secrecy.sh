#!/bin/bash

# ==============================================================================
# Script Name: run_secrecy.sh
# Description: Secrecy application benchmark runner.
#
# Usage:
#   ./run_secrecy.sh <apps> [options]
#
#   <apps> : "aspirin,credit" or "all"
#            (aspirin comorbidity credit pwd rcdiff)
#
# Options:
#   -m MODE    : local | tcp | rdma  (default: local)
#   -t N       : communication threads (default: 4)
#   -s X       : scale factor (default: 0.01)
#   -h HOSTS   : comma-separated host per party (default: node0,node1,node2;
#                tcp/rdma only; the local host runs in-process, others via SSH)
#   --rayon N  : RAYON_NUM_THREADS on all parties
#   --log PATH : override stat log
#   --no-log   : disable stat logging
#
# Examples:
#   ./run_secrecy.sh comorbidity -t 4 -s 0.01
#   ./run_secrecy.sh all -t 16 -s 0.1 -m tcp -h node0,node1,node2
# ==============================================================================

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/run_common.sh"

APPS="${1:-}"
if [ -z "$APPS" ]; then
    sed -n '3,24p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
fi
shift

parse_common_opts "$@" || exit 1
THREADS="${THREADS:-4}"
SF="${SF:-0.01}"

ALL_APPS=("aspirin" "comorbidity" "credit" "pwd" "rcdiff")
read -r -a TARGETS <<< "$(expand_targets "$APPS" "${ALL_APPS[@]}")"
for t in "${TARGETS[@]}"; do
    contains "$t" "${ALL_APPS[@]}" || { echo "Error: invalid secrecy app '$t' (valid: ${ALL_APPS[*]})" >&2; exit 1; }
done

BINS=("${TARGETS[@]}")

setup_log "secrecy_query"
run_all "Secrecy"
