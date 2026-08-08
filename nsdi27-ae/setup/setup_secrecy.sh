#!/usr/bin/env bash
#
# setup_secrecy.sh — Set up Secrecy on node0 (local) and node1/node2 (scp).
#
# Secrecy is cloned and built on the local node only, then copied to
# node1/node2 with scp into the SAME absolute path
# (nsdi27-ae/baselines/secrecy). The remote nodes therefore need no
# internet access and no build — matching the scp-based distribution
# used by the main deploy script. Because the binaries are built on
# node0, all nodes should run the same OS/architecture.
#
# Prerequisites on node0: a C++ toolchain, cmake and MPI; on node1/node2:
# the MPI runtime libraries.
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

    # Skip everything (clone, deps, build) when a previous run already
    # produced the benchmark binaries.
    if compgen -G "${install_dir}/build/exp_*" > /dev/null; then
        echo "==> Secrecy already built at ${install_dir}, skipping setup."
        return 0
    fi

    # Clone Secrecy
    if [[ ! -d "${install_dir}/.git" ]]; then
        echo "==> Cloning Secrecy into ${install_dir}..."
        mkdir -p "$(dirname "${install_dir}")"
        git clone https://github.com/CASP-Systems-BU/Secrecy.git "${install_dir}"
    else
        echo "==> Secrecy already cloned at ${install_dir}."
    fi
    # Run the cd-dependent steps in a subshell so the function does not
    # change the caller's working directory (the orchestrator still needs
    # it for the stdin redirect below).
    (
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
    )

    echo "==> Secrecy installed at ${install_dir}."
}

# ----------------------------- orchestrator -----------------------------

usage() {
    echo "Usage: $0 [NODE_PREFIX]"
    echo ""
    echo "Setup Secrecy on node0 (local), then copy it to node1 and node2 (via scp)."
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

# Normalize to a canonical absolute path (resolves the '..' above) so the
# remote side receives the exact same path.
INSTALL_DIR="$(cd "${INSTALL_DIR}" && pwd)"

# Remote nodes: copy the locally built tree to the same absolute path. The
# remote nodes need no internet access and no build toolchain for this.
# The source is given as "${INSTALL_DIR}/." so re-runs overwrite the remote
# contents in place instead of nesting a new directory inside it.
for n in 1 2; do
    echo "==> Copying Secrecy to ${NODE_PREFIX}${n} (${INSTALL_DIR})..."
    ssh "${NODE_PREFIX}${n}" "mkdir -p '${INSTALL_DIR}'"
    scp -rq "${INSTALL_DIR}/." "${NODE_PREFIX}${n}:${INSTALL_DIR}"
done

echo
echo "======================================================================"
echo "SUCCESS: Secrecy set up on ${NODE_PREFIX}0, ${NODE_PREFIX}1, ${NODE_PREFIX}2"
echo "======================================================================"
