#!/usr/bin/env bash
#
# setup_secrecy.sh — Set up Secrecy on node0 (local) and node1/node2 (SSH).
#
# All nodes install Secrecy into the SAME absolute path
# (nsdi27-ae/baselines/secrecy). This single script plays two roles:
#   * orchestrator (default): installs on the local node, then re-invokes
#     itself via stdin (`bash -s`) on node1/node2;
#   * worker (SECRECY_WORKER=1): just clones and builds Secrecy on the
#     current node. The install path is passed through SECRECY_INSTALL_DIR
#     because the script's own location cannot be resolved via stdin.
#
# Prerequisites on every node: a C++ toolchain, cmake and MPI.
#
# Usage:
#   ./setup_secrecy.sh [NODE_PREFIX]
#
# Arguments:
#   NODE_PREFIX    Optional prefix for node hostnames (default: 'node')
#                  Nodes will be named as: {PREFIX}0, {PREFIX}1, {PREFIX}2
#
# Examples:
#   ./setup_secrecy.sh            # -> node0, node1, node2
#   ./setup_secrecy.sh machine-   # -> machine-0, machine-1, machine-2

set -euo pipefail

install_secrecy() {
    local install_dir="$1"

    # Clone Secrecy
    if [[ ! -d "${install_dir}/.git" ]]; then
        echo "==> Cloning Secrecy into ${install_dir}..."
        mkdir -p "$(dirname "${install_dir}")"
        git clone https://github.com/CASP-Systems-BU/Secrecy.git "${install_dir}"
    else
        echo "==> Secrecy already cloned at ${install_dir}."
    fi
    cd "${install_dir}"

    # Clone the sql-parser dependency
    if [[ ! -d include/external-lib/sql-parser/.git ]]; then
        echo "==> Cloning sql-parser..."
        mkdir -p include/external-lib
        git clone https://github.com/mfaisal97/sql-parser.git include/external-lib/sql-parser
    else
        echo "==> sql-parser already present."
    fi

    # Build
    echo "==> Building Secrecy..."
    mkdir -p build
    cd build
    cmake ..
    make -j "$(( $(nproc) / 2 ))"

    echo "==> Secrecy installed at ${install_dir}."
}

# Worker mode (used for the stdin re-invocation on remote nodes): install
# and exit. SECRECY_INSTALL_DIR is mandatory here because the script's own
# path cannot be resolved when read from stdin.
if [[ "${SECRECY_WORKER:-}" == "1" ]]; then
    install_secrecy "${SECRECY_INSTALL_DIR:?SECRECY_INSTALL_DIR must be set in worker mode}"
    exit 0
fi

# ----------------------------- orchestrator -----------------------------

usage() {
    echo "Usage: $0 [NODE_PREFIX]"
    echo ""
    echo "Setup Secrecy on node0 (local), node1 and node2 (via SSH)."
    echo ""
    echo "Arguments:"
    echo "  NODE_PREFIX    Optional prefix for node hostnames (default: 'node')"
    echo "                 Nodes will be named as: {PREFIX}0, {PREFIX}1, {PREFIX}2"
    echo ""
    echo "Examples:"
    echo "  $0              # Uses default prefix 'node' -> node0, node1, node2"
    echo "  $0 machine-     # Uses prefix 'machine-' -> machine-0, machine-1, machine-2"
    echo ""
    echo "Note: Make sure SSH access is configured for the remote nodes."
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

NODE_PREFIX="node"
if [[ $# -gt 0 ]]; then
    NODE_PREFIX="$1"
fi

# Resolve paths from this script's own location, independent of the caller's
# working directory: nsdi27-ae/setup -> nsdi27-ae/baselines/secrecy.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${SCRIPT_DIR}/../baselines/secrecy"

# Local node
echo "==> Setting up Secrecy on the local node (${INSTALL_DIR})..."
install_secrecy "${INSTALL_DIR}"

# Remote nodes, at the same absolute path (script re-invokes itself via stdin)
for n in 1 2; do
    echo "==> Setting up Secrecy on ${NODE_PREFIX}${n} (${INSTALL_DIR})..."
    ssh "${NODE_PREFIX}${n}" \
        "SECRECY_WORKER=1 SECRECY_INSTALL_DIR='${INSTALL_DIR}' bash -s" < "${BASH_SOURCE[0]}"
done

echo
echo "======================================================================"
echo "SUCCESS: Secrecy set up on ${NODE_PREFIX}0, ${NODE_PREFIX}1, ${NODE_PREFIX}2"
echo "======================================================================"
