cargo build --release --package experiments --bin q3_no_secure_cut --features tcp

BIN_PATH=../../../../target/release/q3_no_secure_cut

NUM_THREADS=${1:-6} #number of threads (default: 6)

SF=${2:-0.01} # scale factor (default: 0.01)


$BIN_PATH -c ../../../../experiments/net/local/ -p 0 -t $NUM_THREADS -s $SF &
$BIN_PATH -c ../../../../experiments/net/local/ -p 1 -t $NUM_THREADS -s $SF &
$BIN_PATH -c ../../../../experiments/net/local/ -p 2 -t $NUM_THREADS -s $SF & 

wait