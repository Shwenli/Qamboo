#!/usr/bin/env bash
#
# setup_orq.sh — Install ORQ (CASP-Systems-BU/orq) on an MPC cluster.
#
# ORQ ships its own multi-node deployment script,
# scripts/orchestration/deploy.sh, which (per ORQ's README):
#   1. generates/copies an SSH key to every node,
#   2. copies the repository to each worker node,
#   3. installs dependencies (via setup.sh) and builds test_primitives,
#   4. runs an MPI smoke test across all nodes.
#
# This wrapper therefore needs to:
#   1. Clone the pinned ORQ commit into nsdi27-ae/baselines/orq.
#   2. Patch ORQ's deploy.sh so it copies the repo to the SAME absolute path on
#      every worker instead of each worker's $HOME. Upstream deploy.sh does
#      `scp -r $REPO_NAME/ $W:~/`, which lands the files in ~/orq while the
#      rest of the script references the absolute $REPO_NAME; the two only
#      agree when the repo lives directly under $HOME. We want ORQ under
#      baselines/orq on all nodes, so we redirect that copy.
#   3. Generate the hostfile (nodeN -> IP) and invoke ORQ's deploy.sh.
#
# Usage:
#   ./setup_orq.sh <ip-0> <ip-1> <ip-2> [ip-3 ...]
#
# Each <ip-N> is the IP address of nodeN. node0 (the first argument) is the
# main node and must have SSH access to the others.

set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <ip-0> <ip-1> <ip-2> [ip-3 ...]" >&2
    echo "  node0 (first argument) is the main node and must have SSH access to the others." >&2
    exit 1
fi

IPS=("$@")
IP_LIST="$(IFS=,; echo "${IPS[*]}")"

# Build the node name list (node0 node1 ...) matching the IP count.
NODES=()
for i in "${!IPS[@]}"; do
    NODES+=("node${i}")
done

REPO_URL="https://github.com/CASP-Systems-BU/orq"
REPO_COMMIT="2d7946a95f6d1d49e020789b70a6cfbdc1198a46"

# Resolve the install location from this script's own location so it never
# depends on the caller's working directory. The script lives in
# nsdi27-ae/setup/, so the baselines dir is one level up. ORQ is installed
# into baselines/orq.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASELINES_DIR="${SCRIPT_DIR}/../baselines"
INSTALL_DIR="${BASELINES_DIR}/orq"

echo "==> Cloning ORQ into ${INSTALL_DIR} (commit ${REPO_COMMIT})..."
mkdir -p "${BASELINES_DIR}"
if [[ ! -d "${INSTALL_DIR}/.git" ]]; then
    git clone "${REPO_URL}" "${INSTALL_DIR}"
fi
cd "${INSTALL_DIR}"
git fetch --all --tags
git checkout "${REPO_COMMIT}"
git submodule update --init --recursive

# Patch ORQ's deploy.sh so worker copies land at ${INSTALL_DIR} (the absolute
# baselines path) on every node, instead of each worker's $HOME. Start from a
# pristine copy so the patch is deterministic across re-runs, then rewrite
# only the copy destination in the per-worker setup loop.
DEPLOY_SCRIPT="${INSTALL_DIR}/scripts/orchestration/deploy.sh"
echo "==> Patching ${DEPLOY_SCRIPT} to deploy ORQ under ${INSTALL_DIR} on all nodes..."
git checkout -- scripts/orchestration/deploy.sh
if grep -qF 'scp -r $REPO_NAME/ $W:~/' "${DEPLOY_SCRIPT}"; then
    sed -i 's|scp -r $REPO_NAME/ $W:~/|ssh $W mkdir -p "$(dirname "$REPO_NAME")"; scp -r $REPO_NAME $W:$REPO_NAME|' "${DEPLOY_SCRIPT}"
fi
if ! grep -qF 'scp -r $REPO_NAME $W:$REPO_NAME' "${DEPLOY_SCRIPT}"; then
    echo "!! Failed to patch ${DEPLOY_SCRIPT}: expected copy line not found." >&2
    echo "!! Upstream deploy.sh may have changed; aborting before deploy." >&2
    exit 1
fi

echo "==> Generating hostfile (nodeN -> IP)..."
./scripts/_update_hostfile.sh -x node -i "${IP_LIST}"

echo "==> Deploying ORQ to the cluster via ORQ's deploy.sh..."
echo "    ${DEPLOY_SCRIPT} ${INSTALL_DIR} ${NODES[*]}"
"${DEPLOY_SCRIPT}" "${INSTALL_DIR}" "${NODES[@]}"

echo
echo "======================================================================"
echo "SUCCESS: ORQ cloned into ${INSTALL_DIR} and deployed to: ${NODES[*]}"
echo "======================================================================"
