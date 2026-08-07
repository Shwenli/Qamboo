#!/usr/bin/env bash
#
# fig12_tcp.sh — Fig 12 (Qamboo, TCP baseline): execution time of TPC-H
# queries at SF=1 with 32 threads over plain TCP, on eRDMA-capable instances
# (Alibaba Cloud Linux 3.2104 LTS). The RDMA counterpart is fig12_rdma.sh.
#
# Runs scripts/experiments/run_tpch.sh (mode: tcp).
#
# Usage:
#   ./fig12_tcp.sh [query-spec] [-h HOSTS]
#
# query-spec selects the queries to run (e.g. "1,3,4" or "1..8"); default:
# all 22 queries. HOSTS is a comma-separated list of 3 hosts, one per party
# (default: "node0,node1,node2"); the local host runs in-process.

set -euo pipefail

usage () {
    echo "Usage: $0 [query-spec] [-h HOSTS]"
    echo "  query-spec: queries to run, e.g. \"1,3,4\" or \"1..8\" (default: \"1..22\")."
    echo "  Runs the selected TPC-H queries at SF=1 with 32 threads over TCP."
    exit 1
}

# Optional query selection (default: all 22 queries).
QUERIES="1..22"
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
# working directory: nsdi27-ae/scripts/fig12 -> Qamboo repo root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QAMBOO_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
RUN_TPCH="${QAMBOO_DIR}/scripts/experiments/run_tpch.sh"

echo "==== Fig 12 (Qamboo, TCP): TPC-H queries ${QUERIES}, SF=1, 32 threads ===="
"${RUN_TPCH}" "${QUERIES}" -t 32 -s 1 -m tcp -h "${HOSTS}"

# Extract the result log into a CSV under nsdi27-ae/data/run/ (results of this
# run; the paper's published numbers live in nsdi27-ae/data/paper/).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
LOG="${QAMBOO_DIR}/experiments/result/tpch_query/multinode/stat_output.log"
python3 "${AE_DIR}/plotting_scripts/extract_qamboo_log_data.py" \
    -i "${LOG}" -o "${RUN_DATA}/fig12_tcp.csv"

echo "==== Fig 12 (TCP) finished. Results: ${RUN_DATA}/fig12_tcp.csv (log: ${LOG}) ===="
