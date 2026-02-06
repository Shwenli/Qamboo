#!/bin/bash

# ==============================================================================
# Script Name: setup_ssh.sh
# Description:
#   1. Generates SSH keys on the host machine0.
#   2. Distributes SSH public keys to specified remote hosts to enable passwordless login.
#
# Usage:
#   ./setup_ssh.sh -h <host0>,<host1>,...
#
# Example:
#   ./setup_ssh.sh -h machine0,machine1
#   ./setup_ssh.sh -h 192.168.1.11,192.168.1.12
# ==============================================================================


# 1. SSH Key Generation
echo "run setup_ssh.sh"
echo ""
echo "============================================================"
echo "Step 1: SSH Key Generation"
echo "============================================================"

if [ ! -f ~/.ssh/id_rsa ]; then
    echo ">>> SSH key not found. Generating new SSH key pair (RSA 4096)..."
    # -t rsa: RSA algorithm
    # -b 4096: 4096 bit key size
    # -N "": No passphrase
    ssh-keygen -t rsa -b 4096 -N "" 
    echo ">>> SSH key generated."
else
    echo ">>> SSH key already exists at ~/.ssh/id_rsa. Skipping generation."
fi


# 2. Distribute Keys
echo ""
echo "============================================================"
echo "Step 2: Distribute SSH Keys"
echo "Target Hosts: $HOST_LIST"
echo "============================================================"

IFS=',' read -r -a HOSTS <<< "$HOST_LIST"

for host in "${HOSTS[@]}"; do
    echo ">>> Copying SSH ID to $host ..."
    echo "    (You may be asked to enter the password for $host if not already authorized)"
    
    # ssh-copy-id appends the public key to the remote ~/.ssh/authorized_keys
    # -o StrictHostKeyChecking=no avoids the "Are you sure..." confirmation prompt
    ssh-copy-id -o StrictHostKeyChecking=no "$host" || {
        echo ">>> Error: Failed to copy ID to $host"
        exit 1
    }
done