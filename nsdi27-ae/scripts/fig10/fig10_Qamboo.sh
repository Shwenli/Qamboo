#!/usr/bin/env bash
#
# fig10_Qamboo.sh — Fig 10 (Qamboo side): oblivious RadixSort execution time
# at 2^16–2^24 rows for 64-bit and 32-bit keys.
#
# The MP-SPDZ baseline is run separately via
# scripts/experiments/run_radix_sort_mpspdz_ssh.sh (requires MP-SPDZ on the
# nodes).
#
# Usage:
#   ./fig10_Qamboo.sh [-h HOSTS]
#
# HOSTS is a comma-separated list of 3 hosts, one per party (default:
# "node0,node1,node2"); the local host runs in-process.

set -euo pipefail

usage () {
    echo "Usage: $0 [-h HOSTS]"
    echo "  Runs the Qamboo oblivious RadixSort comparison (2^16–2^24 rows,"
    echo "  64-bit and 32-bit keys)."
    exit 1
}

HOSTS="node0,node1,node2"
while [[ $# -gt 0 ]]; do
    case $1 in
        -h) HOSTS="$2"; shift 2 ;;
        *) echo "Error: Unknown option $1"; usage ;;
    esac
done

# Resolve paths from this script's own location, independent of the caller's
# working directory: nsdi27-ae/scripts/fig10 -> Qamboo repo root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QAMBOO_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
RUN_OPERATOR="${QAMBOO_DIR}/scripts/experiments/run_operator.sh"

echo "==== Fig 10 (Qamboo): oblivious RadixSort, 2^16–2^24 rows, 64/32-bit keys ===="
"${RUN_OPERATOR}" radix_sort -m tcp -h "${HOSTS}"
echo "==== Fig 10 (Qamboo) finished. Results: experiments/result/operator/multinode/stat_output.log ===="
