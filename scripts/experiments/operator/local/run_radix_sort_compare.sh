#./run_radix_sort.sh 6 20
cargo build --release --package experiments --bin radix_sort --features tcp

BIN_PATH=../../../../target/release/radix_sort

NUM_THREADS=${1:-6} #number of threads (default: 6)

SHIFT=${2:-20} # shift factor (default: 20)


$BIN_PATH -c ../../../../experiments/net/local/ -p 0 -t $NUM_THREADS -s $SHIFT &
$BIN_PATH -c ../../../../experiments/net/local/ -p 1 -t $NUM_THREADS -s $SHIFT &
$BIN_PATH -c ../../../../experiments/net/local/ -p 2 -t $NUM_THREADS -s $SHIFT &  

wait