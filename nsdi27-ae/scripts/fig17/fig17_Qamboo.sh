#!/usr/bin/env bash
#
# fig17_Qamboo.sh — Fig 17 (Qamboo side): secure multiplication and secure
# less-than comparison execution time at 2^16–2^25 secret-shared 64-bit
# elements.
#
# The MP-SPDZ baseline is run separately via fig17_mpspdz.sh (requires
# MP-SPDZ on the nodes, see nsdi27-ae/setup/setup_mpspdz.sh).
#
# Usage:
#   ./fig17_Qamboo.sh [-h HOSTS]
#
# HOSTS is a comma-separated list of 3 hosts, one per party (default:
# "node0,node1,node2"); the local host runs in-process.

set -euo pipefail

usage () {
    echo "Usage: $0 [-h HOSTS]"
    echo "  Runs the Qamboo multiplication and less-than comparison sweeps"
    echo "  (2^16–2^25 elements, 64-bit values)."
    exit 1
}

HOSTS="node0,node1,node2"
while [[ $# -gt 0 ]]; do
    case $1 in
        -h) HOSTS="$2"; shift 2 ;;
        *) echo "Error: Unknown option $1"; usage ;;
    esac
done

# Resolve paths from this script's own location, independent of the caller's
# working directory: nsdi27-ae/scripts/fig17 -> Qamboo repo root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QAMBOO_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
RUN_PRIMITIVE="${QAMBOO_DIR}/scripts/experiments/run_primitive.sh"

echo "==== Fig 17 (Qamboo): multiplication + less-than comparison, 2^16–2^25 elements ===="
"${RUN_PRIMITIVE}" all -m tcp -h "${HOSTS}"

# Extract the result log into a CSV under nsdi27-ae/data/run/ (results of this
# run; the paper's published numbers live in nsdi27-ae/data/paper/).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
LOG="${QAMBOO_DIR}/experiments/result/primitive/multinode/stat_output.log"
python3 "${AE_DIR}/plotting_scripts/extract_qamboo_log_data.py" \
    -i "${LOG}" -o "${RUN_DATA}/fig17_qamboo.csv"

echo "==== Fig 17 (Qamboo) finished. Results: ${RUN_DATA}/fig17_qamboo.csv (log: ${LOG}) ===="
