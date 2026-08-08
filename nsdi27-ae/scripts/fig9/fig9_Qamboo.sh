#!/usr/bin/env bash
#
# fig9_Qamboo.sh — Fig 9 (Qamboo side): execution time of the five Secrecy
# application queries (comorbidity, rcdiff, aspirin, credit score, password
# reuse) plus TPC-H Q4/Q6/Q13, LAN only.
#
# Runs the queries directly via scripts/experiments/run_secrecy.sh and
# run_tpch.sh, using the exact maximum input sizes reported in the Secrecy
# paper (the scale factors are hardcoded below).
#
# Usage:
#   ./fig9_Qamboo.sh [-h HOSTS]
#
# HOSTS is a comma-separated list of 3 hosts, one per party (default:
# "node0,node1,node2"); the local host runs in-process.

set -euo pipefail

usage () {
    echo "Usage: $0 [-h HOSTS]"
    echo "  Runs the five Secrecy application queries plus TPC-H Q4/Q6/Q13"
    echo "  at the Secrecy paper's input sizes (LAN only)."
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
# working directory: nsdi27-ae/scripts/fig9 -> Qamboo repo root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QAMBOO_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
RUN_SECRECY="${QAMBOO_DIR}/scripts/experiments/run_secrecy.sh"
RUN_TPCH="${QAMBOO_DIR}/scripts/experiments/run_tpch.sh"

THREADS=16
LOG="${QAMBOO_DIR}/experiments/result/secrecy_query/multinode/stat_output_paper.log"
mkdir -p "$(dirname "${LOG}")"

echo "==== Fig 9 (Qamboo): Secrecy application queries + TPC-H Q4/Q6/Q13, LAN ===="

# Scale factors reproduce the maximum input sizes reported in the Secrecy
# paper: comorbidity SF=1; aspirin SF=0.016384; rcdiff SF=0.69905067;
# credit/pwd SF=0.4194304; TPC-H Q13 SF=0.17476267; Q4 SF=0.02184533;
# Q6 SF=1.39810133.
"${RUN_TPCH}"    13          -t "${THREADS}" -s 0.17476267 -m tcp -h "${HOSTS}" --log "${LOG}"
"${RUN_TPCH}"    4           -t "${THREADS}" -s 0.02184533 -m tcp -h "${HOSTS}" --log "${LOG}"
"${RUN_TPCH}"    6           -t "${THREADS}" -s 1.39810133 -m tcp -h "${HOSTS}" --log "${LOG}"
"${RUN_SECRECY}" comorbidity -t "${THREADS}" -s 1          -m tcp -h "${HOSTS}" --log "${LOG}"
"${RUN_SECRECY}" aspirin     -t "${THREADS}" -s 0.016384   -m tcp -h "${HOSTS}" --log "${LOG}"
"${RUN_SECRECY}" rcdiff      -t "${THREADS}" -s 0.69905067 -m tcp -h "${HOSTS}" --log "${LOG}"
"${RUN_SECRECY}" credit      -t "${THREADS}" -s 0.4194304  -m tcp -h "${HOSTS}" --log "${LOG}"
"${RUN_SECRECY}" pwd         -t "${THREADS}" -s 0.4194304  -m tcp -h "${HOSTS}" --log "${LOG}"


# Extract the result log into a CSV under nsdi27-ae/data/run/ (results of this
# run; the paper's published numbers live in nsdi27-ae/data/paper/).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
python3 "${AE_DIR}/plotting_scripts/extract_qamboo_log_data.py" \
    -i "${LOG}" -o "${RUN_DATA}/fig9_qamboo.csv"

echo "==== Fig 9 (Qamboo) finished. Results: ${RUN_DATA}/fig9_qamboo.csv (log: ${LOG}) ===="
