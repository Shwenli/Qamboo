cargo build --release --package experiments --bin q3 --features tcp

BIN_PATH=../../../../target/release/q3

NUM_THREADS=${1:-6} #number of threads (default: 6)

SF=${2:-0.1} # scale factor (default: 0.1)


$BIN_PATH -c ../../../net/local/ -p 0 -t $NUM_THREADS -s $SF &
$BIN_PATH -c ../../../net/local/ -p 1 -t $NUM_THREADS -s $SF &
$BIN_PATH -c ../../../net/local/ -p 2 -t $NUM_THREADS -s $SF & 

wait