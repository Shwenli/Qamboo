#!/bin/bash

# ==============================================================================
# Script Name: setup_delay.sh
# Description:
#   Configure network latency and bandwidth using tc (traffic control).
#   Supports executing on multiple hosts via SSH.
#
# Usage:
#   ./setup_delay.sh -c [-H <hosts>] [bandwidth] [latency]
#   ./setup_delay.sh -d [-H <hosts>]
#
# Options:
#   -c           Create tc configuration
#   -d           Delete tc configuration
#   -H <hosts>   Comma-separated list of hosts (e.g., node0,node1,node2)
#   -h           Show help information
#
# Parameters:
#   bandwidth:   Network bandwidth, e.g., 12GBit, 10Gbit, 1000Mbit (default: 12GBit)
#   latency:     Total latency between two ends, e.g., 20ms, 100ms (default: 0.1ms)
#                Note: Actual latency applied to each machine is half of total latency
#
# Examples:
#   ./setup_delay.sh -c -H node0,node1 12GBit 20ms  # Set 12GBit, 20ms total on node0,node1
#   ./setup_delay.sh -c 12GBit 20ms                 # Set locally
#   ./setup_delay.sh -d -H node0,node1              # Delete tc on node0,node1
#   ./setup_delay.sh -d                             # Delete tc locally
# ==============================================================================

set -e

# Default configuration
DEFAULT_RATE="12GBit"
DEFAULT_DELAY_TOTAL="0.1ms"
IFACE="eth0"

# Global variables
HOST_LIST=""
MODE=""
RATE=""
LATENCY=""

usage() {
    echo "Usage: $0 {-c|-d} [-H <hosts>] [bandwidth] [latency]"
    echo ""
    echo "Options:"
    echo "  -c              Create tc network latency configuration"
    echo "  -d              Delete existing tc configuration"
    echo "  -H <hosts>      Comma-separated list of hosts (e.g., node0,node1,node2)"
    echo "  -h              Show help information"
    echo ""
    echo "Parameters (for create mode):"
    echo "  bandwidth:      Network bandwidth, e.g., 12GBit, 10Gbit, 1000Mbit (default: $DEFAULT_RATE)"
    echo "  latency:        Total latency between two ends, e.g., 20ms, 100ms (default: $DEFAULT_DELAY_TOTAL)"
    echo "                  Note: Actual latency applied to each machine is half of total latency"
    echo ""
    echo "Examples:"
    echo "  $0 -c -H node0,node1 12GBit 20ms   # 12GBit, 20ms total on node0,node1 (10ms each)"
    echo "  $0 -c 10Gbit 50ms                   # 10Gbit, 50ms total locally (25ms)"
    echo "  $0 -c                               # Use defaults locally (12GBit, 0.1ms)"
    echo "  $0 -d -H node0,node1                # Delete tc on node0,node1"
    echo "  $0 -d                               # Delete tc locally"
    exit 1
}

# Parse latency parameter (supports units like ms, s, etc.)
# Divide total latency by 2 and return single machine latency
parse_delay() {
    local total_delay="$1"
    
    # Extract number and unit
    if [[ "$total_delay" =~ ^([0-9]+\.?[0-9]*)([a-zA-Z]+)$ ]]; then
        local num="${BASH_REMATCH[1]}"
        local unit="${BASH_REMATCH[2]}"
        
        # Calculate half latency
        local half_num=$(echo "scale=4; $num / 2" | bc 2>/dev/null || echo "0")
        
        # Remove trailing zeros
        half_num=$(echo "$half_num" | sed 's/0*$//;s/\.$//')
        
        echo "${half_num}${unit}"
    else
        # If parsing fails, assume unit is ms
        local num="$total_delay"
        local half_num=$(echo "scale=4; $num / 2" | bc 2>/dev/null || echo "0")
        half_num=$(echo "$half_num" | sed 's/0*$//;s/\.$//')
        echo "${half_num}ms"
    fi
}

# Delete existing tc configuration
delete_tc_local() {
    echo "Deleting tc configuration on $IFACE..."
    sudo tc qdisc del dev "$IFACE" root 2>/dev/null || true
    echo "tc configuration deleted"
}

# Create tc configuration
create_tc_local() {
    local rate="${1:-$DEFAULT_RATE}"
    local total_delay="${2:-$DEFAULT_DELAY_TOTAL}"
    
    # Calculate single machine latency (half of total latency)
    local single_delay=$(parse_delay "$total_delay")
    
    echo "============================================"
    echo "Configuring network parameters:"
    echo "  Interface:      $IFACE"
    echo "  Bandwidth:      $rate"
    echo "  Total latency:  $total_delay"
    echo "  Single machine: $single_delay (total/2)"
    echo "============================================"
    
    # Delete existing configuration first
    sudo tc qdisc del dev "$IFACE" root 2>/dev/null || true
    
    # Add new tc configuration
    echo "Executing: sudo tc qdisc add dev $IFACE root netem rate $rate delay $single_delay"
    sudo tc qdisc add dev "$IFACE" root netem rate "$rate" delay "$single_delay"
    
    echo ""
    echo "tc configuration created, current status:"
    tc qdisc show dev "$IFACE"
}

# Execute on a single host
execute_on_host() {
    local host="$1"
    local cmd="$2"
    
    echo ">>> [$host] Setting up tc..."
    
    # Check if host is local
    local is_local=false
    local current_host
    current_host=$(hostname)
    
    if [ "$host" == "$current_host" ] || [ "$host" == "localhost" ] || [ "$host" == "127.0.0.1" ]; then
        is_local=true
    fi
    
    if [ "$is_local" = true ]; then
        echo "    Executing locally..."
        eval "$cmd"
    else
        echo "    Executing via SSH on $host..."
        ssh -o StrictHostKeyChecking=no "$host" "$cmd"
    fi
    
    echo "    Done."
}

# Main execution logic
main() {
    # Parse options
    while getopts "cdH:h" opt; do
        case $opt in
            c) MODE="create" ;;
            d) MODE="delete" ;;
            H) HOST_LIST="$OPTARG" ;;
            h) usage ;;
            *) usage ;;
        esac
    done
    
    shift $((OPTIND - 1))
    
    # Validate mode
    if [ -z "$MODE" ]; then
        echo "Error: Must specify either -c (create) or -d (delete)"
        usage
    fi
    
    # Get bandwidth and latency for create mode
    if [ "$MODE" == "create" ]; then
        RATE="${1:-$DEFAULT_RATE}"
        LATENCY="${2:-$DEFAULT_DELAY_TOTAL}"
    fi
    
    echo "============================================================"
    echo "Mode: $MODE"
    [ "$MODE" == "create" ] && echo "Bandwidth: $RATE, Latency: $LATENCY"
    echo "============================================================"
    
    # Build the command to execute
    local cmd
    if [ "$MODE" == "create" ]; then
        # For create mode, define a function and call it
        cmd="RATE='$RATE'; LATENCY='$LATENCY'; DEFAULT_RATE='$DEFAULT_RATE'; DEFAULT_DELAY_TOTAL='$DEFAULT_DELAY_TOTAL'; IFACE='$IFACE';"
        cmd+="
parse_delay() {
    local total_delay=\"\$1\"
    if [[ \"\$total_delay\" =~ ^([0-9]+\.?[0-9]*)([a-zA-Z]+)$ ]]; then
        local num=\"\${BASH_REMATCH[1]}\"
        local unit=\"\${BASH_REMATCH[2]}\"
        local half_num=\$(echo \"scale=4; \$num / 2\" | bc 2>/dev/null || echo \"0\")
        half_num=\$(echo \"\$half_num\" | sed 's/0*$//;s/\\.\$//')
        echo \"\${half_num}\${unit}\"
    else
        local num=\"\$total_delay\"
        local half_num=\$(echo \"scale=4; \$num / 2\" | bc 2>/dev/null || echo \"0\")
        half_num=\$(echo \"\$half_num\" | sed 's/0*$//;s/\\.\$//')
        echo \"\${half_num}ms\"
    fi
};
"
        cmd+="echo \"============================================\";"
        cmd+="echo \"Configuring network parameters:\";"
        cmd+="echo \"  Interface:      \$IFACE\";"
        cmd+="echo \"  Bandwidth:      \$RATE\";"
        cmd+="echo \"  Total latency:  \$LATENCY\";"
        cmd+="single_delay=\$(parse_delay \"\$LATENCY\");"
        cmd+="echo \"  Single machine: \$single_delay (total/2)\";"
        cmd+="echo \"============================================\";"
        cmd+="sudo tc qdisc del dev \$IFACE root 2>/dev/null || true;"
        cmd+="echo \"Executing: sudo tc qdisc add dev \$IFACE root netem rate \$RATE delay \$single_delay\";"
        cmd+="sudo tc qdisc add dev \$IFACE root netem rate \"\$RATE\" delay \"\$single_delay\";"
        cmd+="echo \"\";"
        cmd+="echo \"tc configuration created, current status:\";"
        cmd+="tc qdisc show dev \$IFACE"
    else
        # For delete mode
        cmd="IFACE='$IFACE';"
        cmd+="echo \"Deleting tc configuration on \$IFACE...\";"
        cmd+="sudo tc qdisc del dev \$IFACE root 2>/dev/null || true;"
        cmd+="echo \"tc configuration deleted\""
    fi
    
    # Execute on hosts
    if [ -n "$HOST_LIST" ]; then
        echo "Target Hosts: $HOST_LIST"
        echo "============================================================"
        
        IFS=',' read -r -a HOSTS <<< "$HOST_LIST"
        for host in "${HOSTS[@]}"; do
            execute_on_host "$host" "$cmd"
        done
    else
        # Local execution
        echo "Executing locally..."
        if [ "$MODE" == "create" ]; then
            create_tc_local "$RATE" "$LATENCY"
        else
            delete_tc_local
        fi
    fi
    
    echo "============================================================"
    echo "Setup Completed."
    echo "============================================================"
}

main "$@"
