#!/bin/bash

# ==============================================================================
# Generate thread scaling scripts for all TPC-H queries (Q1-Q22)
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MULTINODE_DIR="$SCRIPT_DIR/multinode"

mkdir -p "$MULTINODE_DIR"

# Template for generating query scripts
generate_script() {
    local q=$1
    cat > "$MULTINODE_DIR/run_q${q}_thread_scaling.sh" << 'EOF'
#!/bin/bash

PROJECT_PATH="/root/Qamboo"
BIN_PATH="./target/release/qQUERY"

NUM_COMMTHREADS=${1:-6}
SF=${2:-0.01}
HOST1=${3:-node1}
HOST2=${4:-node2}

RAYON_T=${RAYON_NUM_THREADS:-$(nproc)}

cd $PROJECT_PATH

# Build binary
RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin qQUERY --features tcp

# Copy to remote hosts
scp target/release/qQUERY $HOST1:$PROJECT_PATH/target/release/
scp target/release/qQUERY $HOST2:$PROJECT_PATH/target/release/

# Run with RAYON_NUM_THREADS
export RAYON_NUM_THREADS=$RAYON_T

RAYON_NUM_THREADS=$RAYON_T $BIN_PATH -c experiments/net/multinode/ -p 0 -t $NUM_COMMTHREADS -s $SF &
ssh $HOST1 "cd $PROJECT_PATH && export RAYON_NUM_THREADS=$RAYON_T && $BIN_PATH -c experiments/net/multinode/ -p 1 -t $NUM_COMMTHREADS -s $SF" &
ssh $HOST2 "cd $PROJECT_PATH && export RAYON_NUM_THREADS=$RAYON_T && $BIN_PATH -c experiments/net/multinode/ -p 2 -t $NUM_COMMTHREADS -s $SF" &

wait
EOF

    # Replace QUERY placeholder with actual query number
    sed -i.bak "s/QUERY/$q/g" "$MULTINODE_DIR/run_q${q}_thread_scaling.sh"
    rm -f "$MULTINODE_DIR/run_q${q}_thread_scaling.sh.bak"
    chmod +x "$MULTINODE_DIR/run_q${q}_thread_scaling.sh"
}

# Generate scripts for Q1-Q22
echo "Generating thread scaling scripts for TPC-H queries..."
for q in $(seq 1 22); do
    generate_script $q
    echo "  Created: run_q${q}_thread_scaling.sh"
done

echo ""
echo "All scripts generated in: $MULTINODE_DIR"
echo "Done!"
