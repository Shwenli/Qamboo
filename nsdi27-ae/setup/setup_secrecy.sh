#!/usr/bin/env bash
#
# setup_secrecy_cluster.sh — Set up Secrecy on all 3 nodes.
#
# Runs setup_secrecy.sh on the local node (node0) and on node1/node2 via
# SSH. All nodes install Secrecy into the SAME absolute path
# (nsdi27-ae/baselines/secrecy); because remote nodes receive the script via
# stdin (`bash -s`), the install path is passed explicitly through the
# SECRECY_INSTALL_DIR environment variable.
#
# Usage:
#   ./setup_secrecy_cluster.sh [NODE_PREFIX]
#
# Arguments:
#   NODE_PREFIX    Optional prefix for node hostnames (default: 'node')
#                  Nodes will be named as: {PREFIX}0, {PREFIX}1, {PREFIX}2
#
# Examples:
#   ./setup_secrecy_cluster.sh            # -> node0, node1, node2
#   ./setup_secrecy_cluster.sh machine-   # -> machine-0, machine-1, machine-2
#
# Note: Make sure SSH access is configured for the remote nodes, and that
# the remote absolute path matching this repo's location is writable.

set -euo pipefail

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

# Parse command line arguments
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

# defaults
NODE_PREFIX="node"
if [[ $# -gt 0 ]]; then
    NODE_PREFIX="$1"
fi

# Resolve paths from this script's own location, independent of the caller's
# working directory: nsdi27-ae/setup -> nsdi27-ae/baselines/secrecy.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${SCRIPT_DIR}/../baselines/secrecy"

# Local node setup
echo "==> Setting up Secrecy on the local node (${INSTALL_DIR})..."
SECRECY_INSTALL_DIR="${INSTALL_DIR}" "${SCRIPT_DIR}/setup_secrecy.sh"

# Setup Secrecy on all other nodes, at the same absolute path
for n in 1 2; do
    echo "==> Setting up Secrecy on ${NODE_PREFIX}${n} (${INSTALL_DIR})..."
    ssh "${NODE_PREFIX}${n}" "SECRECY_INSTALL_DIR='${INSTALL_DIR}' bash -s" < "${SCRIPT_DIR}/setup_secrecy.sh"
done

echo
echo "======================================================================"
echo "SUCCESS: Secrecy set up on ${NODE_PREFIX}0, ${NODE_PREFIX}1, ${NODE_PREFIX}2"
echo "======================================================================"
