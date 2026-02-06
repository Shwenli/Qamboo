#!/bin/bash

# ==============================================================================
# Script Name: deploy.sh
# Description: Orchestrates the full setup pipeline: SSH -> Hosts -> RDMA -> Build/Dist
# Usage: ./deploy.sh -i <ip0>,<ip1>... [-x <prefix>]
# Example: ./deploy.sh -i 192.168.1.10,192.168.1.11 -x machine
# ==============================================================================

set -e

IP_LIST=""
PREFIX="machine"

usage() {
    echo "Usage: $0 -i <ip0>,<ip1>... [-x <prefix>]"
    echo "  -i  Comma-separated list of IPs"
    echo "  -x  Hostname prefix (default: machine)"
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


# ------------------------------------------------------------------------------
# 1. Setup SSH Keys (Establish Trust)
# ------------------------------------------------------------------------------
echo "### Step 1: Setting up SSH Trust ###"
"$SCRIPT_DIR/setup_ssh.sh" -i "$IP_LIST"


# ------------------------------------------------------------------------------
# 2. Setup Hostfile (DNS)
# ------------------------------------------------------------------------------
echo ""
echo "### Step 2: Setting up /etc/hosts ###"
# This requires sudo. We invoke the script using sudo or rely on user having permissions?
# setup_host.sh checks for root.
echo ">>> Running setup_host.sh (Requires Sudo)..."
sudo "$SCRIPT_DIR/setup_host.sh" -x "$PREFIX" -i "$IP_LIST"


# ------------------------------------------------------------------------------
# 3. Derive Hostname List
# ------------------------------------------------------------------------------
# We need to construct the list "machine0,machine1,..." from the IP list count
IFS=',' read -r -a IPS <<< "$IP_LIST"
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


# ------------------------------------------------------------------------------
# 4. Setup RDMA
# ------------------------------------------------------------------------------
echo ""
echo "### Step 3: Setting up RDMA on: $HOST_LIST ###"
"$SCRIPT_DIR/setup_rdma.sh" -h "$HOST_LIST"


# ------------------------------------------------------------------------------
# 5. Build and Distribute
# ------------------------------------------------------------------------------
echo ""
echo "### Step 4: Build and Distribute ###"
# We exclude machine0 (localhost) from remote distribution loop inside build_dist if we want?
# But build_dist.sh copies to input hosts.
# Usually we don't need to scp to ourselves (machine0), but build_dist logic handles remote scp.
# Let's filter the HOST_LIST passed to build_dist to exclude the local machine if possible, 
# or just let scp overwrite (wasteful but safe).
# For now, passing full list.

"$SCRIPT_DIR/build_dist.sh" -h "$HOST_LIST"

echo ""
echo "### Deployment Pipeline Finished Successfully! ###"
