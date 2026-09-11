#!/usr/bin/env bash
#
# fig16_Qamboo.sh — Fig 16 / Table "Ablation the Q6 gap" (Qamboo side):
# ablation of the Q6 gap in §eval:compare at SF=1. Runs four Q6 variants,
# each adding one ORQ-style optimization on top of the previous:
#
#   q6               Baseline (as evaluated in §eval:compare)      1x
#   q6_orq_ari_valid Variant 1.1: + precomputed comparisons       ~2.2x
#   q6_orq           Variant 1.2: + boolean validity              ~4.9x
#   q6_orq_bit       Variant 2:   + single-bit validity           ~6.8x
#
# Speedups are computed relative to the baseline (q6) from the extracted CSV.
#
# In WAN mode, the 6 Gbps / 20 ms RTT emulation is applied via Qamboo's
# scripts/setup/setup_delay.sh (tc netem) before the run and removed
# afterwards (also on failure, via trap).
#
# Usage:
#   ./fig16_Qamboo.sh <lan|wan> [-h HOSTS]
#
# HOSTS is a comma-separated list of 3 hosts, one per party (default:
# "node0,node1,node2"); the local host runs in-process.

set -euo pipefail

usage () {
    echo "Usage: $0 <lan|wan> [-h HOSTS]"
    echo "  Runs the Q6 ablation (q6, q6_orq_ari_valid, q6_orq, q6_orq_bit)"
    echo "  at SF=1 with 16 network connections in LAN, 32 in WAN."
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
# working directory: nsdi27-ae/scripts/fig16 -> Qamboo repo root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QAMBOO_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
RUN_TPCH="${QAMBOO_DIR}/scripts/experiments/run_tpch.sh"
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

# Network connections: 16 in LAN, 32 in WAN. Compute threads are auto-sized
# by Rayon (32 vCPUs -> 32 threads), so no --rayon flag is needed.
CONNS=16
[[ "$NETWORK" == "wan" ]] && CONNS=32

# Ablation order: baseline first, then one optimization stacked per variant.
QUERIES="6,6_orq_ari_valid,6_orq,6_orq_bit"
echo "==== Fig 16 (Qamboo): Q6 ablation [${QUERIES}], SF=1, ${CONNS} connections, ${NETWORK} ===="
"${RUN_TPCH}" "${QUERIES}" -t "${CONNS}" -s 1 -m tcp -h "${HOSTS}"

# Extract the result log into a CSV under nsdi27-ae/data/run/ (results of this
# run; the paper's published numbers live in nsdi27-ae/data/paper/).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
LOG="${QAMBOO_DIR}/experiments/result/tpch_query/multinode/stat_output.log"
CSV="${RUN_DATA}/fig16_qamboo_${NETWORK}.csv"
python3 "${AE_DIR}/plotting_scripts/extract_qamboo_log_data.py" --paper-time \
    -i "${LOG}" -o "${CSV}"

# Report per-variant speedups relative to the baseline (q6). The --paper-time
# CSV has the fig7 layout "Query,Qamboo,Orq,,Speed up"; the binaries log task
# names Q6 / Q6_ORQ_ARI_VALID / Q6_ORQ / Q6_ORQ_BIT.
echo "==== Fig 16 (Qamboo) speedups relative to baseline (q6) ===="
python3 - "${CSV}" <<'EOF'
import csv, sys

times = {}
with open(sys.argv[1]) as f:
    for row in csv.DictReader(f):
        try:
            times[row['Query'].strip()] = float(row['Qamboo'])
        except (KeyError, TypeError, ValueError):
            pass

labels = [
    ('Q6',               'Baseline (q6)'),
    ('Q6_ORQ_ARI_VALID', 'Variant 1.1 (+ precomputed comparisons)'),
    ('Q6_ORQ',           'Variant 1.2 (+ boolean validity)'),
    ('Q6_ORQ_BIT',       'Variant 2   (+ single-bit validity)'),
]
base = times.get('Q6')
if base is None:
    print(f"  (baseline 'Q6' not found in {sys.argv[1]}; skipping speedup table)")
    sys.exit(0)
for key, label in labels:
    if key in times:
        print(f"  {label:<45} {times[key]:>10.3f}s   {base / times[key]:.2f}x")
EOF

echo "==== Fig 16 (Qamboo) finished. Results: ${CSV} (log: ${LOG}) ===="
