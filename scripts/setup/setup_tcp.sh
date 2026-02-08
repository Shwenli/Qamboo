#!/bin/bash

# ==============================================================================
# Script Name: setup_tcp.sh
# Description:
#   Sets up TCP network configurations by running generation scripts on multiple hosts.
#   Supports generating for LAN (lan_net_gen) or Local (local_net_gen).
# Usage:
#   ./setup_tcp.sh -t <type> -h <host1>,<host2>...
#   Type: lan or local
# ==============================================================================

set -e

HOST_LIST=""
NET_TYPE=""

usage() {
    echo "Usage: $0 -t <type> -h <host1>,<host2>..."
    echo "  -t <type>   : Network type ('lan' or 'local')."
    echo "  -h <hosts>  : Comma-separated list of hosts (e.g., node0,node1,node2)."
    exit 1
}

while getopts "h:t:" opt; do
    case $opt in
        h) HOST_LIST="$OPTARG" ;;
        t) NET_TYPE="$OPTARG" ;;
        *) usage ;;
    esac
done

if [ -z "$HOST_LIST" ] || [ -z "$NET_TYPE" ]; then
    echo "Error: Host list (-h) and Net type (-t) are required."
    usage
fi

if [ "$NET_TYPE" != "lan" ] && [ "$NET_TYPE" != "local" ]; then
    echo "Error: Net type must be 'lan' or 'local'."
    usage
fi

# Determine project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Check relative path to project root. 
# Script is in scripts/setup/, so project root is ../../
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Project Root: $PROJECT_ROOT"
echo "Target Hosts: $HOST_LIST"
echo "Network Type: $NET_TYPE"
echo "============================================================"

# Convert comma-separated hosts to space-separated for python script args
IFS=',' read -r -a HOSTS <<< "$HOST_LIST"
HOSTS_SPACE="${HOSTS[*]}"

CURRENT_HOST=$(hostname)
echo "Current Host: $CURRENT_HOST"

for host in "${HOSTS[@]}"; do
    echo ">>> [$host] Setting up $NET_TYPE network..."

    # Construct command
    # We navigate to scripts/setup/net relative to project root
    CMD="cd \"$PROJECT_ROOT/scripts/setup/net\""
    
    if [ "$NET_TYPE" == "lan" ]; then
        # For LAN, pass all IPs
        CMD="$CMD && python3 lan_net_gen.py --ip $HOSTS_SPACE"
    else
        # For Local, just run script
        CMD="$CMD && python3 local_net_gen.py"
    fi

    
    IS_LOCAL=false
    if [ "$host" == "$CURRENT_HOST" ] || [ "$host" == "localhost" ] || [ "$host" == "127.0.0.1" ]; then
        IS_LOCAL=true
    fi
    

    if [ "$IS_LOCAL" = true ]; then
        echo "    Executing locally..."
        eval "$CMD"
    else
        echo "    Executing via SSH on $host..."
        ssh -o StrictHostKeyChecking=no "$host" "$CMD"
    fi
    
    echo "    Done."
done

echo "============================================================"
echo "Network Setup Completed."
echo "============================================================"
