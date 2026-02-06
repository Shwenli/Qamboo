#!/bin/bash

# ==============================================================================
# Script Name: setup_rdma.sh
# Description:
#   Loads SMC-R kernel modules (smc, smc_diag) on the specified list of hosts.
#   Runs locally on the current machine and via SSH on remote machines.
#
# Usage:
#   ./setup_rdma.sh -h <host1>,<host2>,...
#
# Example:
#   ./setup_rdma.sh -h machine0,machine1,machine2
# ==============================================================================

set -e

HOST_LIST=""

# Function to display usage
usage() {
    echo "Usage: $0 -h <host1>,<host2>,..."
    echo "  -h  Comma-separated list of remote hosts (IPs or hostnames)"
    exit 1
}

# Parse arguments
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

echo "run setup_ssh.sh"
echo ""

echo "============================================================"
echo "Starting SMC/RDMA Setup"
echo "Target Hosts: $HOST_LIST"
echo "============================================================"

IFS=',' read -r -a HOSTS <<< "$HOST_LIST"

# Function to run setup commands
setup_rdma_on_node() {
    local node=$1
    echo ">>> Setting up RDMA/SMC on $node ..."
    
    # Check if we are running on the current node (localhost check could be complex, 
    # so we simply try SSH for everyone or run directly if it matches hostname. 
    # For simplicity in cluster scripts, using SSH for all including self is often easiest 
    # if allowed, OR we check against hostname.)
    
    # Here we use SSH for all nodes. 
    # NOTE: This requires sudo access without password or user input.
    # We use 'sudo modprobe' which might ask for password unless configured in sudoers.
    
    ssh -o StrictHostKeyChecking=no "$node" "sudo modprobe smc && sudo modprobe smc_diag && echo 'Modules loaded successfully on \$(hostname)'" 2>&1 | sed "s/^/[$node] /"
    
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo ">>> $node: Success"
    else
        echo ">>> $node: Failed"
    fi
}

for host in "${HOSTS[@]}"; do
    setup_rdma_on_node "$host"
done

echo ""
echo "============================================================"
echo "RDMA Setup Completed."
echo "============================================================"
