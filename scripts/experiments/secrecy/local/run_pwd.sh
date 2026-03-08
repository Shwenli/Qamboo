RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin pwd --features tcp

BIN_PATH=../../../../target/release/pwd

NUM_THREADS=${1:-4} #number of threads (default: 6)

SF=${2:-0.01} # scale factor (default: 0.01)


$BIN_PATH -c ../../../../experiments/net/local/ -p 0 -t $NUM_THREADS -s $SF &
$BIN_PATH -c ../../../../experiments/net/local/ -p 1 -t $NUM_THREADS -s $SF &
$BIN_PATH -c ../../../../experiments/net/local/ -p 2 -t $NUM_THREADS -s $SF &  

wait