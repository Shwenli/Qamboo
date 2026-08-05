#!/usr/bin/env bash
#
# fig7_Qamboo.sh — Fig 7 (Qamboo side): execution time of all 22 TPC-H
# queries at SF=1, with 32 compute threads and 16 network connections.
#
# The 16 network connections come from the TOML network configs generated
# during setup (scripts/setup/setup_connection.sh -t lan -n 16); the run
# script itself only takes the thread count, scale factor, and query list.
#
# In WAN mode, the 6 Gbps / 20 ms RTT emulation is applied via Qamboo's
# scripts/setup/setup_delay.sh (tc netem) before the run and removed
# afterwards (also on failure, via trap).
#
# Usage:
#   ./fig7_Qamboo.sh <lan|wan> [query-spec] [-h HOSTS]
#
# query-spec selects the queries to run (e.g. "1,3,4" or "1..8"; default:
# all 22). HOSTS is a comma-separated list of 3 hosts, one per party
# (default: "node0,node1,node2"); the local host runs in-process.

set -euo pipefail

usage () {
    echo "Usage: $0 <lan|wan> [query-spec] [-h HOSTS]"
    echo "  query-spec: queries to run, e.g. \"1,3,4\" or \"1..8\" (default: \"1..22\")."
    echo "  Runs the selected TPC-H queries at SF=1 with 32 threads / 16 connections."
    echo "  In WAN, we emulate a 20 ms RTT, 6 Gbps connection via"
    echo "  Qamboo's scripts/setup/setup_delay.sh (tc netem)."
    exit 1
}

if [[ $# -lt 1 ]]; then
    usage
fi

NETWORK="$1"
shift

if [[ "$NETWORK" != "lan" && "$NETWORK" != "wan" ]]; then
    echo 'Error: NETWORK must be `lan` or `wan`.'
    usage
fi

# Optional query selection, e.g. "1,3,4" or "1..8" (default: all 22 queries).
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
# working directory: nsdi27-ae/scripts/fig7 -> Qamboo repo root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QAMBOO_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
RUN_TPCH="${QAMBOO_DIR}/scripts/experiments/run_tpch.sh"
SETUP_DELAY="${QAMBOO_DIR}/scripts/setup/setup_delay.sh"

# node0 is the local node the script runs on.
DELAY_NODES="${HOSTS}"

cleanup () {
    if [[ "$NETWORK" == "wan" ]]; then
        echo "==== Removing tc WAN emulation (setup_delay.sh -d) ===="
        "${SETUP_DELAY}" -d -H "${DELAY_NODES}" || true
    fi
}
trap cleanup EXIT

# Load the WAN emulation (target: 6 Gbps, 20 ms RTT) on all nodes.
if [[ "$NETWORK" == "wan" ]]; then
    echo "==== Applying tc WAN emulation (setup_delay.sh -c, 6GBit/20ms target) ===="
    "${SETUP_DELAY}" -c -H "${DELAY_NODES}" 6GBit 20ms
fi

echo "==== Fig 7 (Qamboo): TPC-H queries ${QUERIES}, SF=1, 32 threads, ${NETWORK} ===="
"${RUN_TPCH}" "${QUERIES}" -t 32 -s 1 -m tcp -h "${HOSTS}"

# Extract the result log into a CSV under nsdi27-ae/data/run/ (results of this
# run; the paper's published numbers live in nsdi27-ae/data/paper/).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
LOG="${QAMBOO_DIR}/experiments/result/tpch_query/multinode/stat_output.log"
python3 "${AE_DIR}/plotting_scripts/extract_log_data.py" \
    -i "${LOG}" -o "${RUN_DATA}/fig7_qamboo_${NETWORK}.csv"

echo "==== Fig 7 (Qamboo) finished. Results: ${RUN_DATA}/fig7_qamboo_${NETWORK}.csv (log: ${LOG}) ===="
