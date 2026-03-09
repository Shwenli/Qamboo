RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin q9_no_join_reorder --features tcp

BIN_PATH=../../../../../target/release/q9_no_join_reorder

NUM_COMMTHREADS=${1:-6} #number of threads (default: 6)

SF=${2:-0.01} # scale factor (default: 0.01)


$BIN_PATH -c ../../../../../experiments/net/local/ -p 0 -t $NUM_COMMTHREADS -s $SF &
$BIN_PATH -c ../../../../../experiments/net/local/ -p 1 -t $NUM_COMMTHREADS -s $SF &
$BIN_PATH -c ../../../../../experiments/net/local/ -p 2 -t $NUM_COMMTHREADS -s $SF &  

wait
