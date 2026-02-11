#!/bin/bash

# ==============================================================================
# Script Name: build_dist.sh
# Description:
#   1. Builds the project (release).
#   2. Distributes the code to remote hosts.
# Usage:
#   ./build_dist.sh -h <host1>,<host2>...
# ==============================================================================

set -e

HOST_LIST=""

usage() {
    echo "Usage: $0 -h <host1>,<host2>..."
    exit 1
}

while getopts "h:" opt; do
    case $opt in
        h) HOST_LIST="$OPTARG" ;;
        *) usage ;;
    esac
done

if [ -z "$HOST_LIST" ]; then
    echo "Error: Host list is required."
    usage
fi

echo "Run build_dist.sh"
echo ""

# 0. requirements check
echo "============================================================"
echo "Step 0: Checking Remote Environments (Rust)..."
echo "============================================================"

IFS=',' read -r -a HOSTS <<< "$HOST_LIST"

for host in "${HOSTS[@]}"; do
    echo ">>> [Check] Checking $host..."
    
    # Check if cargo is available. 
    # We try to source the env file first in case it's installed but not in non-interactive PATH
    if ssh -o StrictHostKeyChecking=no "$host" "[ -f \"\$HOME/.cargo/env\" ] && . \"\$HOME/.cargo/env\"; command -v cargo >/dev/null 2>&1"; then
        echo "    -> Rust is already installed on $host."
    else
        echo "    -> Rust NOT found on $host. Installing..."
        
        # Install Rust (passed -y to disable interactive prompts), source env, and verify
        ssh -o StrictHostKeyChecking=no "$host" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && . \"\$HOME/.cargo/env\" && cargo --version"
        
        echo "    -> Rust installed successfully on $host."
    fi
done



# 1. Build
echo "============================================================"
echo "Step 1: Building..."
echo "============================================================"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo ">>> [Build] Moving to project root: $PROJECT_ROOT"
cd "$PROJECT_ROOT"

echo ">>> [Build] Compiling workspace (Release)..."
RUSTFLAGS="-C target-cpu=native" cargo build --workspace --exclude experiments --release


# 2. Distribute
echo "============================================================"
echo "Step 2: Distributing..."
echo "Target Hosts: $HOST_LIST"
echo "============================================================"

echo ">>> [Dist] Syncing to hosts: $HOST_LIST"
IFS=',' read -r -a HOSTS <<< "$HOST_LIST"

for host in "${HOSTS[@]}"; do
    echo ">>> [Dist] Syncing to $host..."
    
    # Assuming path symmetry
    DEST_PARENT="$(dirname "$PROJECT_ROOT")"
    
    # Ensure parent dir exists
    ssh -o StrictHostKeyChecking=no "$host" "mkdir -p $DEST_PARENT"
    
    # Copy project
    # Using scp -r as requested. Excluding target usually not possible with basic scp -r without hacks.
    
    scp -r -o StrictHostKeyChecking=no "$PROJECT_ROOT" "$host:$DEST_PARENT"
    
    echo ">>> [Dist] Synced to $host."
done
echo ">>> [Dist] Completed."

echo "============================================================"
echo "Build and Dist Completed."
echo "============================================================"
