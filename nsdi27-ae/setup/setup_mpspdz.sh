#!/usr/bin/env bash
#
# setup_mpspdz.sh — Install MP-SPDZ (data61/MP-SPDZ) as the RadixSort (Fig 10)
# and basic-primitive (Fig 17: multiplication, less-than) baseline,
# 3PC replicated-ring only.
#
# What it does:
#   1. Installs the build dependencies via apt (only needed on node0: the
#      runner ships a STATIC binary to the workers, so node1/node2 only need
#      SSH access).
#   2. Builds and installs Boost 1.75 from source (Ubuntu 22.04 ships 1.74,
#      which is too old for the pinned MP-SPDZ commit).
#   3. Clones the pinned MP-SPDZ commit into nsdi27-ae/baselines/mpspdz and
#      runs `make setup`.
#   4. Installs the sort.mpc / mul.mpc / lt.mpc benchmark programs into
#      Programs/Source.
#   5. Generates the HOSTS hostfile used by `compile-run.py -H`.
#
# Usage:
#   ./setup_mpspdz.sh [-h HOSTS]
#
# HOSTS is a comma-separated list of the 3 party hosts (default:
# "node0,node1,node2"); this script runs on party 0 and must have SSH access
# to the others.

set -euo pipefail

usage () {
    echo "Usage: $0 [-h HOSTS]"
    echo "  Installs MP-SPDZ into nsdi27-ae/baselines/mpspdz (3PC replicated-ring)."
    echo "  HOSTS: comma-separated list of the 3 party hosts (default: node0,node1,node2)."
    exit 1
}

HOSTS="node0,node1,node2"
while [[ $# -gt 0 ]]; do
    case $1 in
        -h) HOSTS="$2"; shift 2 ;;
        *) echo "Error: Unknown option $1"; usage ;;
    esac
done

IFS=',' read -r -a PARTIES <<< "${HOSTS}"
if [[ ${#PARTIES[@]} -ne 3 ]]; then
    echo "Error: exactly 3 party hosts are required (got: ${HOSTS})." >&2
    exit 1
fi

REPO_URL="https://github.com/data61/MP-SPDZ.git"
REPO_COMMIT="27220fc954490bdc55516383b7c9ac9eeb33d951"
BOOST_VERSION="1.75.0"
BOOST_TARBALL="boost_1_75_0.tar.gz"

# Resolve the install location from this script's own location so it never
# depends on the caller's working directory: nsdi27-ae/setup ->
# nsdi27-ae/baselines/mpspdz.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASELINES_DIR="${SCRIPT_DIR}/../baselines"
INSTALL_DIR="${BASELINES_DIR}/mpspdz"

echo "==> Installing build dependencies via apt..."
sudo apt update
sudo apt install -y automake autotools-dev build-essential clang cmake git \
    g++ libboost-dev libboost-filesystem-dev libboost-iostreams-dev \
    libboost-thread-dev libbz2-dev libgmp-dev libicu-dev libntl-dev \
    libsodium-dev libssl-dev libtool libstdc++-11-dev python3 python3-dev \
    python3-pip python3-fabric wget

echo "==> Building Boost ${BOOST_VERSION} from source..."
if [[ -e "/usr/local/lib/libboost_filesystem.so.${BOOST_VERSION}" ]]; then
    echo "    Boost ${BOOST_VERSION} already installed, skipping."
else
    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "${TMP_DIR}"' EXIT
    wget -P "${TMP_DIR}" "https://downloads.sourceforge.net/project/boost/boost/${BOOST_VERSION}/${BOOST_TARBALL}"
    tar -xf "${TMP_DIR}/${BOOST_TARBALL}" -C "${TMP_DIR}"
    cd "${TMP_DIR}/boost_1_75_0"
    ./bootstrap.sh
    ./b2 -j"$(nproc)"
    sudo ./b2 install
    sudo ldconfig
    cd "${SCRIPT_DIR}"
fi

echo "==> Cloning MP-SPDZ into ${INSTALL_DIR} (commit ${REPO_COMMIT})..."
mkdir -p "${BASELINES_DIR}"
if [[ ! -d "${INSTALL_DIR}/.git" ]]; then
    git clone "${REPO_URL}" "${INSTALL_DIR}"
fi
cd "${INSTALL_DIR}"
git fetch --all --tags
git checkout "${REPO_COMMIT}"

echo "==> Running MP-SPDZ setup (make setup)..."
make setup

# MP-SPDZ compiles against the source-built Boost 1.75 headers in
# /usr/local/include, but /usr/local/lib is not in the linker's default
# search path on Ubuntu, so static linking would silently pick up the older
# apt Boost (1.74) and fail with undefined boost::filesystem references.
# CONFIG.mine's MY_LDLIBS is expanded before MP-SPDZ's own -L flags, which
# puts /usr/local/lib first in the search order. Guarded so re-running this
# script doesn't append the line twice.
grep -qxF 'MY_LDLIBS = -L/usr/local/lib' CONFIG.mine 2>/dev/null || \
    echo 'MY_LDLIBS = -L/usr/local/lib' >> CONFIG.mine

echo "==> Installing the sort.mpc benchmark program..."
cat > Programs/Source/sort.mpc <<'EOF'
from Compiler.library import *
import Compiler

# Usage example
n = int(program.args[1])

v = sint.Array(n)

v.sort()
EOF

echo "==> Installing the mul.mpc benchmark program (Fig 17)..."
cat > Programs/Source/mul.mpc <<'EOF'
from Compiler.library import *
import Compiler

# Usage: mul <n> — element-wise multiplication of two n-element vectors of
# secret-shared 64-bit integers; the product is written back to memory so it
# cannot be optimized away.
n = int(program.args[1])

a = sint.Array(n)
b = sint.Array(n)
c = sint.Array(n)

c[:] = a[:] * b[:]
EOF

echo "==> Installing the lt.mpc benchmark program (Fig 17)..."
cat > Programs/Source/lt.mpc <<'EOF'
from Compiler.library import *
import Compiler

# Usage: lt <n> — element-wise less-than comparison of two n-element vectors
# of secret-shared 64-bit integers; the mask is written back to memory so it
# cannot be optimized away.
n = int(program.args[1])

a = sint.Array(n)
b = sint.Array(n)
c = sint.Array(n)

c[:] = a[:] < b[:]
EOF

echo "==> Generating the HOSTS hostfile (${PARTIES[*]})..."
printf '%s\n' "${PARTIES[@]}" > HOSTS

echo "==> Generating SSL certificates for the 3 parties..."
Scripts/setup-ssl.sh 3

echo
echo "======================================================================"
echo "SUCCESS: MP-SPDZ installed in ${INSTALL_DIR} (parties: ${PARTIES[*]})"
echo "Run the Fig 10 baseline with: ${SCRIPT_DIR}/../scripts/fig10/fig10_mpspdz.sh [first] [last]"
echo "Run the Fig 17 baseline with: ${SCRIPT_DIR}/../scripts/fig17/fig17_mpspdz.sh [first] [last]"
echo "======================================================================"
