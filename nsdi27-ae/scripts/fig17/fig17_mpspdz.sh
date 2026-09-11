#!/usr/bin/env bash
#
# fig17_mpspdz.sh — Fig 17 (MP-SPDZ side): 3PC replicated-ring vector
# multiplication and less-than comparison, sweeping 2^first to 2^last
# elements, 1 iteration per size.
#
# Must be run after nsdi27-ae/setup/setup_mpspdz.sh (which installs the
# mul.mpc and lt.mpc programs). Output is tee'd to
# nsdi27-ae/data/run/fig17_mpspdz_{mul,lt}.log and then extracted to
# nsdi27-ae/data/run/fig17_mpspdz_{mul,lt}.csv.
#
# Usage:
#   ./fig17_mpspdz.sh [first] [last]     # default: 16 25 (i.e., 2^16..2^25 elements)

set -euo pipefail

FIRST="${1:-16}"
LAST="${2:-25}"

if ! [[ "${FIRST}" =~ ^[0-9]+$ && "${LAST}" =~ ^[0-9]+$ ]]; then
    echo "Usage: $0 [first] [last]" >&2
    echo "  Sweeps multiplication and less-than from 2^first to 2^last elements" >&2
    echo "  (3PC replicated-ring)." >&2
    exit 1
fi

# Resolve the MP-SPDZ install location from this script's own location:
# nsdi27-ae/scripts/fig17 -> nsdi27-ae/baselines/mpspdz.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SPDZ_DIR="${SCRIPT_DIR}/../../baselines/mpspdz"

cd "${SPDZ_DIR}"

# Each sweep is tee'd into a per-program log under nsdi27-ae/data/run/ and then
# extracted into a CSV (results of this run; the paper's published numbers
# live in nsdi27-ae/data/paper/).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"

for prog in mul lt; do
    LOG="${RUN_DATA}/fig17_mpspdz_${prog}.log"

    for i in $(seq "${FIRST}" "${LAST}"); do
        input_size=$((2 ** i))

        sleep 5
        echo "Exponent $i"
        Scripts/compile-run.py -H HOSTS -E ring -v -t "${prog}" "${input_size}"

        echo "---"
    done 2>&1 | tee "${LOG}"

    python3 "${AE_DIR}/plotting_scripts/extract_mpspdz_log.py" \
        -i "${LOG}" -o "${RUN_DATA}/fig17_mpspdz_${prog}.csv"
done

echo "==== Fig 17 (MP-SPDZ) finished. Results: ${RUN_DATA}/fig17_mpspdz_{mul,lt}.csv ===="
