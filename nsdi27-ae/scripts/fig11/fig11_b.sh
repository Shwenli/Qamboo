#!/usr/bin/env bash
#
# fig11_b.sh — Fig 11b (Qamboo): effect of secure group cutting.
# Runs the no_secure_cut variants (one optimization disabled) at SF=1 with
# 32 threads in LAN; the corresponding optimized numbers come from the Fig 7
# run at SF=1.
#
# Runs scripts/experiments/run_optimization.sh (variant: no_secure_cut).
#
# Usage:
#   ./fig11_b.sh [query-spec] [-h HOSTS]
#
# query-spec selects the queries to run (e.g. "2,3,5" or "2..21"); default:
# all queries available for this ablation: 2,3,5,8,13,17,18,20,21.
# HOSTS is a comma-separated list of 3 hosts, one per party (default:
# "node0,node1,node2"); the local host runs in-process.

set -euo pipefail

usage () {
    echo "Usage: $0 [query-spec] [-h HOSTS]"
    echo "  query-spec: queries to run, e.g. \"2,3,5\" or \"2..21\""
    echo "              (default: \"2,3,5,8,13,17,18,20,21\" — all available for this ablation)."
    echo "  Runs the no_secure_cut variants at SF=1 with 32 threads (LAN)."
    exit 1
}

# Optional query selection (default: all queries of this ablation).
QUERIES="2,3,5,8,13,17,18,20,21"
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

echo "==== Fig 11b (Qamboo): no_secure_cut, queries ${QUERIES}, SF=1, 32 threads, LAN ===="
"${RUN_OPT}" no_secure_cut "${QUERIES}" -t 32 -s 1 -m tcp -h "${HOSTS}"

# Extract the result log into a CSV under nsdi27-ae/data/run/ (results of this
# run; the paper's published numbers live in nsdi27-ae/data/paper/).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
LOG="${QAMBOO_DIR}/experiments/result/query_optimization/no_secure_cut/multinode/stat_output.log"
python3 "${AE_DIR}/plotting_scripts/extract_qamboo_log_data.py" \
    -i "${LOG}" -o "${RUN_DATA}/fig11b_qamboo.csv"

echo "==== Fig 11b finished. Results: ${RUN_DATA}/fig11b_qamboo.csv (log: ${LOG}) ===="
