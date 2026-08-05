#!/usr/bin/env bash
#
# fig10_mpspdz.sh — Fig 10 (MP-SPDZ side): 3PC replicated-ring RadixSort,
# sweeping 2^first to 2^last rows, 1 iteration per size.
#
# Must be run after nsdi27-ae/setup/setup_mpspdz.sh. Output is tee'd to
# nsdi27-ae/data/run/fig10_mpspdz.log and then extracted to
# nsdi27-ae/data/run/fig10_mpspdz.csv.
#
# Usage:
#   ./fig10_mpspdz.sh [first] [last]     # default: 16 24 (i.e., 2^16..2^24 rows)

set -euo pipefail

FIRST="${1:-16}"
LAST="${2:-24}"

if ! [[ "${FIRST}" =~ ^[0-9]+$ && "${LAST}" =~ ^[0-9]+$ ]]; then
    echo "Usage: $0 [first] [last]" >&2
    echo "  Sweeps RadixSort from 2^first to 2^last rows (3PC replicated-ring)." >&2
    exit 1
fi

# Resolve the MP-SPDZ install location from this script's own location:
# nsdi27-ae/scripts/fig10 -> nsdi27-ae/baselines/mpspdz.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SPDZ_DIR="${SCRIPT_DIR}/../../baselines/mpspdz"

if [[ ! -d "${SPDZ_DIR}" ]]; then
    echo "Error: ${SPDZ_DIR} not found; run ${SCRIPT_DIR}/../../setup/setup_mpspdz.sh first." >&2
    exit 1
fi
cd "${SPDZ_DIR}"

Scripts/setup-ssl.sh 3

# The sweep is tee'd into a log under nsdi27-ae/data/run/ and then extracted
# into a CSV (results of this run; the paper's published numbers live in
# nsdi27-ae/data/paper/).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
LOG="${RUN_DATA}/fig10_mpspdz.log"

for i in $(seq "${FIRST}" "${LAST}"); do
    input_size=$((2 ** i))

    sleep 5
    echo "Exponent $i"
    Scripts/compile-run.py -H HOSTS -E ring -v -t sort "${input_size}"

    echo "---"
done 2>&1 | tee "${LOG}"

python3 "${AE_DIR}/plotting_scripts/extract_mpspdz_log.py" \
    -i "${LOG}" -o "${RUN_DATA}/fig10_mpspdz.csv"

echo "==== Fig 10 (MP-SPDZ) finished. Results: ${RUN_DATA}/fig10_mpspdz.csv (log: ${LOG}) ===="
