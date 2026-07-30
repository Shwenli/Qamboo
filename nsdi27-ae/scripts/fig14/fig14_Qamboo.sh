#!/usr/bin/env bash
#
# fig14_Qamboo.sh — Fig 14 (Qamboo side): oblivious RadixSort scaling from
# 2^20 to 2^27 rows for 64-bit and 32-bit keys, in LAN and WAN.
#
# In WAN mode, the 6 Gbps / 20 ms RTT emulation is applied via Qamboo's
# scripts/setup/setup_delay.sh (tc netem) before the run and removed
# afterwards (also on failure, via trap).
#
# Usage:
#   ./fig14_Qamboo.sh <lan|wan> [-h HOSTS]
#
# HOSTS is a comma-separated list of 3 hosts, one per party (default:
# "node0,node1,node2"); the local host runs in-process.

set -euo pipefail

usage () {
    echo "Usage: $0 <lan|wan> [-h HOSTS]"
    echo "  Sweeps oblivious RadixSort from 2^20 to 2^27 rows (64/32-bit keys)."
    echo "  In WAN, we emulate a 20 ms RTT, 6 Gbps connection via"
    echo "  Qamboo's scripts/setup/setup_delay.sh (tc netem)."
    exit 1
}

if [[ $# -lt 1 ]]; then
    usage
fi

NETWORK="$1"
shift

if [[ "$NETWORK" != "lan" && "$NETWORK" != "wan" ]]; then
    echo 'Error: NETWORK must be `lan` or `wan`.'
    usage
fi

HOSTS="node0,node1,node2"
while [[ $# -gt 0 ]]; do
    case $1 in
        -h) HOSTS="$2"; shift 2 ;;
        *) echo "Error: Unknown option $1"; usage ;;
    esac
done

# Resolve paths from this script's own location, independent of the caller's
# working directory: nsdi27-ae/scripts/fig14 -> Qamboo repo root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QAMBOO_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
RUN_OPERATOR="${QAMBOO_DIR}/scripts/experiments/run_operator.sh"
SETUP_DELAY="${QAMBOO_DIR}/scripts/setup/setup_delay.sh"

cleanup () {
    if [[ "$NETWORK" == "wan" ]]; then
        echo "==== Removing tc WAN emulation (setup_delay.sh -d) ===="
        "${SETUP_DELAY}" -d -H "${HOSTS}" || true
    fi
}
trap cleanup EXIT

# Load the WAN emulation (target: 6 Gbps, 20 ms RTT) on all nodes.
if [[ "$NETWORK" == "wan" ]]; then
    echo "==== Applying tc WAN emulation (setup_delay.sh -c, 6GBit/20ms target) ===="
    "${SETUP_DELAY}" -c -H "${HOSTS}" 6GBit 20ms
fi

echo "==== Fig 14 (Qamboo): RadixSort scaling, 2^20–2^27 rows, 64/32-bit keys, ${NETWORK} ===="
"${RUN_OPERATOR}" radix_sort_scalability -m tcp -h "${HOSTS}"
echo "==== Fig 14 (Qamboo) finished. Results: experiments/result/operator/multinode/stat_output.log ===="
