#!/usr/bin/env bash
# This script is adapted from ORQ's repository
# (scripts/sosp25-replication/artifact-tpch.sh, commit 2d7946a).
#
# Differences from upstream:
#   * 3PC (SH-HM) only; the 2PC/4PC protocols and the plot branch are removed.
#   * All of ORQ's relative paths only resolve inside the ORQ repo, so we cd
#     into the ORQ checkout at nsdi27-ae/baselines/orq (installed by
#     ../../setup/setup_orq.sh) and run from there.
#   * WAN delay/bandwidth emulation uses Qamboo's scripts/setup/setup_delay.sh
#     (tc netem, 6 Gbps / 20 ms RTT) instead of ORQ's cluster-wan-sim.sh. The
#     tc rules are loaded before the experiments and removed afterwards (also
#     on failure, via trap). ORQ's own cluster-wan-sim.sh is neutralized for
#     the duration of the run so the delay is not applied twice; the "wan"
#     setting is still passed through so ORQ builds with -DWAN_CONFIGURATION.

set -e

SCALE_FACTOR=1
PROTOCOL=3

# Resolve paths from this script's own location, independent of the caller's
# working directory: nsdi27-ae/scripts/fig7 -> nsdi27-ae.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
ORQ_DIR="${AE_DIR}/baselines/orq"

# Qamboo's tc-based WAN emulation script (Qamboo/scripts/setup/setup_delay.sh).
SETUP_DELAY="${AE_DIR}/../scripts/setup/setup_delay.sh"
DELAY_NODES="node0,node1,node2"

usage () {
    echo "Usage: $0 <lan|wan> [query-spec]"
    echo "  Protocol is fixed to 3PC (SH-HM)."
    echo "  We expect node0, node1, ... to be routable"
    echo "  In WAN, we emulate a 20 ms RTT, 6 Gbps connection via"
    echo "  Qamboo's scripts/setup/setup_delay.sh (tc netem)."
    echo "  Queries will run at scale factor $SCALE_FACTOR."
    exit 1
}

if [[ $# -lt 1 || $# -gt 2 ]]; then
    usage;
fi

NETWORK="$1"

if [[ "$NETWORK" != "lan" && "$NETWORK" != "wan" ]]; then
    echo 'Error: NETWORK must be `lan` or `wan`.'
    usage;
fi

# This script is a copy of ORQ's scripts/sosp25-replication/artifact-tpch.sh;
# run from that directory so the relative paths below (../comm/, ../../build,
# ../query-experiments.sh) resolve correctly.
cd "${ORQ_DIR}/scripts/sosp25-replication"

# Neutralize ORQ's own WAN simulation for the duration of this run; WAN
# emulation is handled by Qamboo's setup_delay.sh instead. cluster-wan-sim.sh
# is invoked by ../query-experiments.sh on node0 only, so replacing the local
# file is enough. It is restored on exit.
WAN_SIM="${ORQ_DIR}/scripts/comm/cluster-wan-sim.sh"
cat > "${WAN_SIM}" <<'EOF'
#!/usr/bin/env bash
# Neutralized by fig7_Orq.sh: WAN emulation is handled externally by
# Qamboo's scripts/setup/setup_delay.sh. Restored on exit.
exit 0
EOF
chmod +x "${WAN_SIM}"

cleanup () {
    if [[ "$NETWORK" == "wan" ]]; then
        echo "==== Removing tc WAN emulation (setup_delay.sh -d) ===="
        "${SETUP_DELAY}" -d -H "${DELAY_NODES}" || true
    fi
    git -C "${ORQ_DIR}" checkout -- scripts/comm/cluster-wan-sim.sh || true
}
trap cleanup EXIT

ping -qc 1 node1 && \
ping -qc 1 node2 && \
echo "==== Connectivity check OK! ====" || exit 1

(
    echo "==== Test nocopy... ==="
    cd ../../build
    ../scripts/run_experiment.sh -s $NETWORK -x node -p $PROTOCOL -c nocopy -T 1 -r 20 test_primitives
    echo "==== Test OK? Cancel if not. ===="
    sleep 1
)

# Load the WAN emulation (6 Gbps, 20 ms RTT) on all nodes via Qamboo's tc
# script. Removed again by the EXIT trap after the experiments.
if [[ "$NETWORK" == "wan" ]]; then
    echo "==== Applying tc WAN emulation (setup_delay.sh -c, 6GBit/20ms) ===="
    "${SETUP_DELAY}" -c -H "${DELAY_NODES}" 6GBit 20ms
fi

# If not specified, this arg will be empty
QUERY_SELECT=$2

# Run queries with 16 threads.
echo "==== Start TPCH ===="
../query-experiments.sh tpch $SCALE_FACTOR $PROTOCOL 16 $NETWORK $QUERY_SELECT
echo "==== Finished TPCH ===="

# Extract the ORQ results into a CSV under nsdi27-ae/data/run/ (results of
# this run; the paper's published numbers live in nsdi27-ae/data/paper/).
# query-experiments.sh writes one log per query to
# results/query-benchmark/tpch/<timestamp>-3PC-<env>-SF1/raw_data/q*.txt;
# pick the folder created by this run (the newest one).
AE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_DATA="${AE_DIR}/data/run"
mkdir -p "${RUN_DATA}"
LATEST_RAW="$(ls -td "${ORQ_DIR}"/results/query-benchmark/tpch/*/raw_data 2>/dev/null | head -1)"
if [[ -n "${LATEST_RAW}" ]]; then
    python3 "${AE_DIR}/plotting_scripts/extract_orq_log.py" \
        -i "${LATEST_RAW}" --median -o "${RUN_DATA}/fig7_orq_${NETWORK}.csv"
    echo "==== Fig 7 (ORQ) finished. Results: ${RUN_DATA}/fig7_orq_${NETWORK}.csv (logs: ${LATEST_RAW}) ===="
else
    echo "WARNING: no ORQ results found under ${ORQ_DIR}/results/query-benchmark/tpch" >&2
fi


