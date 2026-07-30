#!/bin/bash

# ==============================================================================
# Script Name: setup_delay.sh
# Description:
#   Configure network latency and bandwidth using tc (traffic control).
#   Supports executing on multiple hosts via SSH.
#
#   The latency parameter is the TARGET total RTT between two nodes, not the
#   delay to add. The script measures the current average RTT once (by pinging
#   the first remote host in -H from the local node) and adds only the missing
#   difference: each node gets (target - measured) / 2 on its NIC, so every
#   node pair ends up at the target RTT. Since the delay is applied per NIC,
#   configuring each node once covers all pairs.
#
# Usage:
#   ./setup_delay.sh -c [-H <hosts>] [bandwidth] [target-latency]
#   ./setup_delay.sh -d [-H <hosts>]
#
# Options:
#   -c           Create tc configuration
#   -d           Delete tc configuration
#   -H <hosts>   Comma-separated list of hosts (e.g., node0,node1,node2)
#   -h           Show help information
#
# Parameters:
#   bandwidth:      Network bandwidth, e.g., 12GBit, 10Gbit, 1000Mbit (default: 12GBit)
#   target-latency: Target total RTT between two ends, e.g., 20ms, 100ms (default: 20ms)
#                   The script pings a peer to measure the current RTT and only
#                   adds the missing difference: (target - measured) / 2 per node.
#
# Examples:
#   ./setup_delay.sh -c -H node0,node1,node2 6GBit 20ms  # Target 6GBit, 20ms RTT on all nodes
#   ./setup_delay.sh -d -H node0,node1                  # Delete tc on node0,node1
#   ./setup_delay.sh -d                                 # Delete tc locally
# ==============================================================================

set -e

# Default configuration
DEFAULT_RATE="12GBit"
DEFAULT_DELAY_TOTAL="20ms"
IFACE="eth0"
PING_COUNT=5

# Global variables
HOST_LIST=""
MODE=""
RATE=""
LATENCY=""

usage() {
    echo "Usage: $0 {-c|-d} [-H <hosts>] [bandwidth] [target-latency]"
    echo ""
    echo "Options:"
    echo "  -c              Create tc network latency configuration"
    echo "  -d              Delete existing tc configuration"
    echo "  -H <hosts>      Comma-separated list of hosts (e.g., node0,node1,node2)"
    echo "  -h              Show help information"
    echo ""
    echo "Parameters (for create mode):"
    echo "  bandwidth:      Network bandwidth, e.g., 12GBit, 10Gbit, 1000Mbit (default: $DEFAULT_RATE)"
    echo "  target-latency: Target total RTT between two ends, e.g., 20ms, 100ms (default: $DEFAULT_DELAY_TOTAL)"
    echo "                  The script pings the first remote host in -H once, measures the"
    echo "                  current average RTT, and adds only the missing difference:"
    echo "                  (target - measured) / 2 per node."
    echo ""
    echo "Examples:"
    echo "  $0 -c -H node0,node1,node2 6GBit 20ms   # Target 6GBit, 20ms RTT on all nodes"
    echo "  $0 -d -H node0,node1                    # Delete tc on node0,node1"
    echo "  $0 -d                                   # Delete tc locally"
    exit 1
}

# Extract the numeric part (in ms) from a latency value like "20ms" or "20"
parse_ms() {
    local value="$1"
    if [[ "$value" =~ ^([0-9]+\.?[0-9]*)(ms)?$ ]]; then
        echo "${BASH_REMATCH[1]}"
    else
        echo "Error: latency must be given in ms, e.g., 20ms (got: $value)" >&2
        exit 1
    fi
}

# Measure the current average RTT (in ms) to a host via ping
measure_rtt() {
    local host="$1"
    local avg
    avg=$(ping -c "$PING_COUNT" -W 2 "$host" 2>/dev/null | awk -F'/' '/^(rtt|round-trip)/ {print $5}')
    if [[ -z "$avg" ]]; then
        echo "Error: failed to measure RTT to $host (is it reachable via ping?)" >&2
        exit 1
    fi
    echo "$avg"
}

# Trim trailing zeros from a decimal number (e.g., 0.5000 -> 0.5)
trim_zeros() {
    echo "$1" | sed 's/0*$//;s/\.$//'
}

# Delete existing tc configuration
delete_tc_local() {
    echo "Deleting tc configuration on $IFACE..."
    sudo tc qdisc del dev "$IFACE" root 2>/dev/null || true
    echo "tc configuration deleted"
}

# Create tc configuration with an already-computed per-node delay
create_tc_local() {
    local rate="$1"
    local delay="$2"

    echo "============================================"
    echo "Configuring network parameters:"
    echo "  Interface:      $IFACE"
    echo "  Bandwidth:      $rate"
    echo "  Added delay:    $delay (per node)"
    echo "============================================"

    # Delete existing configuration first
    sudo tc qdisc del dev "$IFACE" root 2>/dev/null || true

    # Add new tc configuration
    echo "Executing: sudo tc qdisc add dev $IFACE root netem rate $rate delay $delay"
    sudo tc qdisc add dev "$IFACE" root netem rate "$rate" delay "$delay"

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

    # Get bandwidth and target latency for create mode
    if [ "$MODE" == "create" ]; then
        RATE="${1:-$DEFAULT_RATE}"
        LATENCY="${2:-$DEFAULT_DELAY_TOTAL}"
    fi

    echo "============================================================"
    echo "Mode: $MODE"
    [ "$MODE" == "create" ] && echo "Bandwidth: $RATE, Target RTT: $LATENCY"
    echo "============================================================"

    # Build the command to execute
    local cmd
    if [ "$MODE" == "create" ]; then
        # Target-latency mode requires at least one remote host to ping.
        if [ -z "$HOST_LIST" ]; then
            echo "Error: create mode requires -H <hosts> so the current RTT to a" >&2
            echo "remote peer can be measured (the latency parameter is the target" >&2
            echo "total RTT, not the delay to add)." >&2
            exit 1
        fi

        # Measure the current RTT once. Prefer the second host in the list
        # (by convention the first is node0, the local node), so we ping a
        # real peer even when the local hostname differs from the node0
        # alias. Fall back to the first non-local host otherwise. The same
        # per-node delay is then applied to all nodes; since tc works per
        # NIC, one setting per node covers all pairs.
        local current_host peer=""
        current_host=$(hostname)
        IFS=',' read -r -a HOSTS <<< "$HOST_LIST"
        if [ ${#HOSTS[@]} -ge 2 ]; then
            peer="${HOSTS[1]}"
        else
            for host in "${HOSTS[@]}"; do
                if [ "$host" != "$current_host" ] && [ "$host" != "localhost" ] && [ "$host" != "127.0.0.1" ]; then
                    peer="$host"
                    break
                fi
            done
        fi
        if [ -z "$peer" ]; then
            echo "Error: no remote host found in -H list to ping for RTT measurement." >&2
            exit 1
        fi

        local target_ms measured_ms extra_ms per_node_ms
        target_ms=$(parse_ms "$LATENCY")
        echo "Measuring current RTT to $peer ($PING_COUNT pings)..."
        measured_ms=$(measure_rtt "$peer")
        extra_ms=$(awk "BEGIN{printf \"%.4f\", $target_ms - $measured_ms}")
        if awk "BEGIN{exit !($extra_ms <= 0)}"; then
            echo "WARNING: current RTT (${measured_ms}ms) already meets/exceeds the target" >&2
            echo "         (${target_ms}ms). No extra delay will be added (bandwidth cap only)." >&2
            per_node_ms="0"
        else
            per_node_ms=$(trim_zeros "$(awk "BEGIN{printf \"%.4f\", $extra_ms / 2}")")
        fi

        echo "------------------------------------------------------------"
        echo "  Target RTT:        ${target_ms}ms"
        echo "  Measured RTT:      ${measured_ms}ms (to $peer)"
        echo "  Extra delay:       $(trim_zeros "$extra_ms")ms total -> ${per_node_ms}ms per node"
        echo "------------------------------------------------------------"

        # Per-node delay is already computed; hosts just apply it.
        cmd="IFACE='$IFACE'; RATE='$RATE'; DELAY='${per_node_ms}ms';"
        cmd+="echo \"============================================\";"
        cmd+="echo \"Configuring network parameters:\";"
        cmd+="echo \"  Interface:      \$IFACE\";"
        cmd+="echo \"  Bandwidth:      \$RATE\";"
        cmd+="echo \"  Added delay:    \$DELAY (per node)\";"
        cmd+="echo \"============================================\";"
        cmd+="sudo tc qdisc del dev \$IFACE root 2>/dev/null || true;"
        cmd+="echo \"Executing: sudo tc qdisc add dev \$IFACE root netem rate \$RATE delay \$DELAY\";"
        cmd+="sudo tc qdisc add dev \$IFACE root netem rate \"\$RATE\" delay \"\$DELAY\";"
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

        # HOSTS may already be set from the measurement step above
        if [ ${#HOSTS[@]} -eq 0 ]; then
            IFS=',' read -r -a HOSTS <<< "$HOST_LIST"
        fi
        for host in "${HOSTS[@]}"; do
            execute_on_host "$host" "$cmd"
        done
    else
        # Local execution
        echo "Executing locally..."
        if [ "$MODE" == "create" ]; then
            create_tc_local "$RATE" "${per_node_ms}ms"
        else
            delete_tc_local
        fi
    fi

    echo "============================================================"
    echo "Setup Completed."
    echo "============================================================"
}

main "$@"
