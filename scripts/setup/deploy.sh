#!/bin/bash

# ==============================================================================
# Script Name: deploy.sh
# Description: One-click multi-machine deployment for Qamboo.
#   Pipeline: SSH trust -> /etc/hosts -> RDMA -> Rust check -> Build -> Distribute
#
#   Note: setup_ssh.sh is invoked as Step 1, but also remains available as a
#   standalone script (e.g. ./setup_ssh.sh -h <host0>,<host1>...).
#
# Usage: ./deploy.sh -i <ip0>,<ip1>... [-x <prefix>]
# Example: ./deploy.sh -i 192.168.1.10,192.168.1.11 -x node
# ==============================================================================

set -e

IP_LIST=""
PREFIX="node"

usage() {
    echo "Usage: $0 -i <ip0>,<ip1>... [-x <prefix>]"
    echo "  -i  Comma-separated list of IPs"
    echo "  -x  Hostname prefix (default: node)"
    exit 1
}

while getopts "i:x:" opt; do
    case $opt in
        i) IP_LIST="$OPTARG" ;;
        x) PREFIX="$OPTARG" ;;
        *) usage ;;
    esac
done

if [ -z "$IP_LIST" ]; then
    echo "Error: IP list is required."
    usage
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

IFS=',' read -r -a IPS <<< "$IP_LIST"

# ------------------------------------------------------------------------------
# 1. Setup SSH Keys (Establish Trust)
# ------------------------------------------------------------------------------
echo "============================================================"
echo "Step 1: Setting up SSH Trust"
echo "============================================================"
# NOTE: setup_ssh.sh only accepts -h (comma-separated hosts); IPs work fine.
"$SCRIPT_DIR/setup_ssh.sh" -h "$IP_LIST"


# ------------------------------------------------------------------------------
# 2. Setup Hostfile (DNS)
# ------------------------------------------------------------------------------
echo ""
echo "============================================================"
echo "Step 2: Setting up /etc/hosts (Requires Sudo)"
echo "============================================================"
sudo "$SCRIPT_DIR/setup_host.sh" -x "$PREFIX" -i "$IP_LIST"


# ------------------------------------------------------------------------------
# Derive Hostname List ("node0,node1,...")
# ------------------------------------------------------------------------------
HOST_LIST=""
count=0
for _ in "${IPS[@]}"; do
    if [ -z "$HOST_LIST" ]; then
        HOST_LIST="${PREFIX}${count}"
    else
        HOST_LIST="${HOST_LIST},${PREFIX}${count}"
    fi
    count=$((count + 1))
done
IFS=',' read -r -a HOSTS <<< "$HOST_LIST"


# ------------------------------------------------------------------------------
# 3. Setup RDMA
# ------------------------------------------------------------------------------
echo ""
echo "============================================================"
echo "Step 3: Setting up RDMA on: $HOST_LIST"
echo "============================================================"
"$SCRIPT_DIR/setup_rdma.sh" -h "$HOST_LIST"


# ------------------------------------------------------------------------------
# 4. Check Remote Environments (Rust)
# ------------------------------------------------------------------------------
echo ""
echo "============================================================"
echo "Step 4: Checking Remote Environments (Rust)"
echo "============================================================"
for host in "${HOSTS[@]}"; do
    echo ">>> [Check] Checking $host..."

    # Try to source the env file first in case cargo is installed but not in non-interactive PATH
    if ssh -o StrictHostKeyChecking=no "$host" "[ -f \"\$HOME/.cargo/env\" ] && . \"\$HOME/.cargo/env\"; command -v cargo >/dev/null 2>&1"; then
        echo "    -> Rust is already installed on $host."
    else
        echo "    -> Rust NOT found on $host. Installing..."
        ssh -o StrictHostKeyChecking=no "$host" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && . \"\$HOME/.cargo/env\" && cargo --version"
        echo "    -> Rust installed successfully on $host."
    fi
done


# ------------------------------------------------------------------------------
# 5. Build (Release)
# ------------------------------------------------------------------------------
echo ""
echo "============================================================"
echo "Step 5: Building"
echo "============================================================"
echo ">>> [Build] Moving to project root: $PROJECT_ROOT"
cd "$PROJECT_ROOT"

# cargo may be installed via rustup but not in the non-interactive PATH
if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

echo ">>> [Build] Compiling workspace (Release)..."
RUSTFLAGS="-C target-cpu=native" cargo build --workspace --exclude experiments --release


# ------------------------------------------------------------------------------
# 6. Distribute
# ------------------------------------------------------------------------------
echo ""
echo "============================================================"
echo "Step 6: Distributing to: $HOST_LIST"
echo "============================================================"
for host in "${HOSTS[@]}"; do
    echo ">>> [Dist] Syncing to $host..."

    # Assuming path symmetry
    DEST_PARENT="$(dirname "$PROJECT_ROOT")"

    # Ensure parent dir exists
    ssh -o StrictHostKeyChecking=no "$host" "mkdir -p $DEST_PARENT"

    # Copy project (scp -r; note: this includes target/)
    scp -r -o StrictHostKeyChecking=no "$PROJECT_ROOT" "$host:$DEST_PARENT"

    echo ">>> [Dist] Synced to $host."
done
echo ">>> [Dist] Completed."

echo ""
echo "### Deployment Pipeline Finished Successfully! ###"
