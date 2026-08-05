#!/usr/bin/env bash
#
# fig9_secrecy.sh — Fig 9 (Secrecy baseline): run the five Secrecy
# application queries (comorbidity, rcdiff, aspirin, credit score, password
# reuse) plus TPC-H Q4/Q6/Q13 via MPI across 3 nodes, LAN only.
#
# Paths are resolved from this script's own location, independent of the
# caller's working directory: the Secrecy build installed by
# ../../setup/setup_secrecy.sh at nsdi27-ae/baselines/secrecy/build,
# and logs under nsdi27-ae/data/logs/fig9/secrecy/.
#
# Usage:
#   ./fig9_secrecy.sh [NODE_PREFIX]
#
# Arguments:
#   NODE_PREFIX    Optional prefix for node hostnames (default: 'node')
#                  Nodes will be named as: {PREFIX}0, {PREFIX}1, {PREFIX}2
#
# Note: Run from node0. Secrecy must already be built on all nodes
# (../../setup/setup_secrecy.sh) at the same absolute path.

set -euo pipefail

usage() {
    echo "Usage: $0 [NODE_PREFIX]"
    echo ""
    echo "Run Secrecy benchmarks across multiple nodes using MPI (LAN only)."
    echo ""
    echo "Arguments:"
    echo "  NODE_PREFIX    Optional prefix for node hostnames (default: 'node')"
    echo "                 Nodes will be named as: {PREFIX}0, {PREFIX}1, {PREFIX}2"
    echo ""
    echo "Results will be saved to: nsdi27-ae/data/logs/fig9/secrecy/"
}

# Parse command line arguments
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

# Defaults
NODE_PREFIX="node"
if [[ $# -gt 0 ]]; then
    NODE_PREFIX="$1"
fi

# Resolve paths from this script's own location: nsdi27-ae/scripts/fig9.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

EXP_HOSTS="-n 3 -host ${NODE_PREFIX}0,${NODE_PREFIX}1,${NODE_PREFIX}2"
SECRECY_BUILD_DIR="${AE_DIR}/baselines/secrecy/build"
LOG_DIR="${AE_DIR}/data/logs/fig9/secrecy"

if [[ ! -d "${SECRECY_BUILD_DIR}" ]]; then
    echo "Error: ${SECRECY_BUILD_DIR} not found." >&2
    echo "Run ../../setup/setup_secrecy.sh first to install and build Secrecy." >&2
    exit 1
fi

# Create results directory
mkdir -p "${LOG_DIR}"

# Change to the build directory
cd "${SECRECY_BUILD_DIR}"

# Setting up the SUBNET variable
iface=""
# figure out which interface to use
# for now, just check node1
_node=${NODE_PREFIX}1
_iface=$(ip route get $(dig +short ${_node}) | grep -Po "((?<=dev )\S*)")
if [[ -n "$iface" && "$_iface" != "$iface" ]]; then
    echo "interface error: ${_node} routable via $_iface but previous interface(s) were routable via $iface".
    exit 1
fi
iface=$_iface
SUBNET=$(ip -o -f inet addr show ${iface} | awk '{print $4}')
[[ -n $iface ]] && echo "Common interface: ${iface}; subnet ${SUBNET}"

RUN_CMD="mpirun --mca btl_tcp_if_include $SUBNET --mca oob_tcp_if_include $SUBNET"
EXP_PREFIX="$RUN_CMD $EXP_HOSTS"

# Q4 (exp_tpch_q4): Lineintems 128k, oders 32k, batch size 8k
date
$EXP_PREFIX ./exp_tpch_q4 32768 131072 8192 | tee "${LOG_DIR}/exp_tpch_q4.txt"

# Q6 (exp_tpch_q6): 8m
date
$EXP_PREFIX ./exp_tpch_q6 8388608 | tee "${LOG_DIR}/exp_tpch_q6.txt"

# Q13 (exp_tpch_q13): Orders 256k, customers 32k, 4k batch size
date
$EXP_PREFIX ./exp_tpch_q13 32768 262144 4096 | tee "${LOG_DIR}/exp_tpch_q13.txt"

# Comorbidity (exp_q1): first table 2m, second table 256, TODO: check on second input
date
$EXP_PREFIX ./exp_q1 2097152 256 | tee "${LOG_DIR}/exp_q1.txt"

# Rec. cdiff (exp_q2) : first table 2m
date
$EXP_PREFIX ./exp_q2 2097152 | tee "${LOG_DIR}/exp_q2.txt"

# Aspirin count (exp_q3): input tables 32k , batch size 32k
date
$EXP_PREFIX ./exp_q3 32768 32768 16384 | tee "${LOG_DIR}/exp_q3.txt"

# Credit Score (exp_qcredit): input table 2m, batch size 256k
date
$EXP_PREFIX ./exp_qcredit 2097152 262144 | tee "${LOG_DIR}/exp_qcredit.txt"

# Password Reuse (exp_qpwd): input table 2m, batch size 256k
date
$EXP_PREFIX ./exp_qpwd 2097152 262144 | tee "${LOG_DIR}/exp_qpwd.txt"

# Extract the Secrecy logs into a CSV under nsdi27-ae/data/run/ (results of
# this run; the paper's published numbers live in nsdi27-ae/data/paper/).
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
python3 "${AE_DIR}/plotting_scripts/extract_orq_log.py" \
    -i "${LOG_DIR}" --median -o "${RUN_DATA}/fig9_secrecy.csv"

echo "==== Fig 9 (Secrecy) finished. Results: ${RUN_DATA}/fig9_secrecy.csv (logs: ${LOG_DIR}) ===="
