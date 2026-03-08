#!/bin/bash

# ==============================================================================
# Script Name: setup_connection.sh
# Description:
#   Sets up TCP network configurations by running generation scripts on multiple hosts.
#   Supports generating for LAN (lan_net_gen) or Local (local_net_gen).
# Usage:
#   ./setup_connection.sh -t <type> -h <host1>,<host2>... -n <num_groups>
#   Type: lan or local
#   Num: number of groups (default: 10)
# ==============================================================================

set -e

HOST_LIST=""
NET_TYPE=""
NUM_GROUPS=10

usage() {
    echo "Usage: $0 -t <type> -h <host1>,<host2>... [-n <num_groups>]"
    echo "  -t <type>       : Network type ('lan' or 'local')."
    echo "  -h <hosts>      : Comma-separated list of hosts (e.g., node0,node1,node2)."
    echo "  -n <num_groups> : Number of groups to generate (default: 10)."
    exit 1
}

while getopts "h:t:n:" opt; do
    case $opt in
        h) HOST_LIST="$OPTARG" ;;
        t) NET_TYPE="$OPTARG" ;;
        n) NUM_GROUPS="$OPTARG" ;;
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
echo "Num Groups: $NUM_GROUPS"
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
        # For LAN, pass all IPs and num groups
        CMD="$CMD && python3 lan_net_gen.py --num $NUM_GROUPS --ip $HOSTS_SPACE"
    else
        # For Local, pass num groups
        CMD="$CMD && python3 local_net_gen.py --num $NUM_GROUPS"
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
