#!/usr/bin/env bash
#
# fig11_a.sh — Fig 11a (Qamboo): effect of smallest-first join reordering.
# Runs the no_join_reorder variants (one optimization disabled) at SF=1 with
# 32 threads in LAN; the corresponding optimized numbers come from the Fig 7
# run at SF=1.
#
# Runs scripts/experiments/run_optimization.sh (variant: no_join_reorder).
#
# Usage:
#   ./fig11_a.sh [query-spec] [-h HOSTS]
#
# query-spec selects the queries to run (e.g. "2,5,7" or "2..10"); default:
# all queries available for this ablation: 2,5,7,8,9,10.
# HOSTS is a comma-separated list of 3 hosts, one per party (default:
# "node0,node1,node2"); the local host runs in-process.

set -euo pipefail

usage () {
    echo "Usage: $0 [query-spec] [-h HOSTS]"
    echo "  query-spec: queries to run, e.g. \"2,5,7\" or \"2..10\""
    echo "              (default: \"2,5,7,8,9,10\" — all available for this ablation)."
    echo "  Runs the no_join_reorder variants at SF=1 with 32 threads (LAN)."
    exit 1
}

# Optional query selection (default: all queries of this ablation).
QUERIES="2,5,7,8,9,10"
if [[ $# -gt 0 && "$1" != -* ]]; then
    QUERIES="$1"
    shift
fi

HOSTS="node0,node1,node2"
while [[ $# -gt 0 ]]; do
    case $1 in
        -h) HOSTS="$2"; shift 2 ;;
        *) echo "Error: Unknown option $1"; usage ;;
    esac
done

# Resolve paths from this script's own location, independent of the caller's
# working directory: nsdi27-ae/scripts/fig11 -> Qamboo repo root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QAMBOO_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
RUN_OPT="${QAMBOO_DIR}/scripts/experiments/run_optimization.sh"

echo "==== Fig 11a (Qamboo): no_join_reorder, queries ${QUERIES}, SF=1, 32 threads, LAN ===="
"${RUN_OPT}" no_join_reorder "${QUERIES}" -t 32 -s 1 -m tcp -h "${HOSTS}"

# Extract the result log into a CSV under nsdi27-ae/data/run/ (results of this
# run; the paper's published numbers live in nsdi27-ae/data/paper/).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
LOG="${QAMBOO_DIR}/experiments/result/query_optimization/no_join_reorder/multinode/stat_output.log"
python3 "${AE_DIR}/plotting_scripts/extract_log_data.py" \
    -i "${LOG}" -o "${RUN_DATA}/fig11a_qamboo.csv"

echo "==== Fig 11a finished. Results: ${RUN_DATA}/fig11a_qamboo.csv (log: ${LOG}) ===="
