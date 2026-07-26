#!/bin/bash

# ==============================================================================
# Generate LOCAL thread scaling scripts for all TPC-H queries (Q1-Q22)
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCAL_DIR="$SCRIPT_DIR/local"

mkdir -p "$LOCAL_DIR"

# Template for generating local query scripts
generate_local_script() {
    local q=$1
    cat > "$LOCAL_DIR/run_q${q}_thread_scaling.sh" << 'EOF'
#!/bin/bash

# Thread scaling version for LOCAL execution (single machine, 3 parties)

# Auto-detect project root (5 levels up from script location)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"
BIN_PATH="$PROJECT_PATH/target/release/qQUERY"

NUM_COMMTHREADS=${1:-6}
SF=${2:-0.01}

RAYON_T=${RAYON_NUM_THREADS:-$(nproc)}

cd $PROJECT_PATH

# Build binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin qQUERY --features tcp

# Run all 3 parties locally with RAYON_NUM_THREADS
export RAYON_NUM_THREADS=$RAYON_T

$BIN_PATH -c experiments/net/local/ -p 0 -t $NUM_COMMTHREADS -s $SF &
$BIN_PATH -c experiments/net/local/ -p 1 -t $NUM_COMMTHREADS -s $SF &
$BIN_PATH -c experiments/net/local/ -p 2 -t $NUM_COMMTHREADS -s $SF &

wait
EOF

    # Replace QUERY placeholder with actual query number
    sed -i.bak "s/QUERY/$q/g" "$LOCAL_DIR/run_q${q}_thread_scaling.sh"
    rm -f "$LOCAL_DIR/run_q${q}_thread_scaling.sh.bak"
    chmod +x "$LOCAL_DIR/run_q${q}_thread_scaling.sh"
}

# Generate scripts for Q1-Q22
echo "Generating LOCAL thread scaling scripts for TPC-H queries..."
for q in $(seq 1 22); do
    generate_local_script $q
    echo "  Created: local/run_q${q}_thread_scaling.sh"
done

echo ""
echo "All local scripts generated in: $LOCAL_DIR"
echo "Done!"
