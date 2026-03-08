#!/bin/bash
#! run with: ./run_comorbidity_ssh.sh 4 0.01

# Define remote host IPs and users (Modify according to actual situation)
HOST1="node1"         
HOST2="node2"  
PROJECT_PATH="/root/Qamboo"  # Replace with the absolute path of the project on remote machines
BIN_PATH="./target/release/comorbidity"

NUM_THREADS=${1:-4} #number of threads (default: 4)
SF=${2:-0.01} # scale factor (default: 0.01)

cd $PROJECT_PATH

RUSTFLAGS="-C target-cpu=native" cargo build --release --package experiments --bin comorbidity --features tcp
scp target/release/comorbidity $HOST1:$PROJECT_PATH/target/release/
scp target/release/comorbidity $HOST2:$PROJECT_PATH/target/release/

# Run party 0 locally
$BIN_PATH -c experiments/net/multinode/ -p 0 -t $NUM_THREADS -s $SF &
# SSH to HOST1 to run party 1
ssh $HOST1 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 1 -t $NUM_THREADS -s $SF" &
# SSH to HOST2 to run party 2
ssh $HOST2 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/multinode/ -p 2 -t $NUM_THREADS -s $SF" &

wait
