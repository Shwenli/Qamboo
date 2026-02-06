#!/bin/bash

# 定义远程主机IP和用户（请根据实际情况修改）
HOST1="172.17.40.26"  # 替换为实际IP
HOST2="172.17.40.27"  # 替换为实际IP
USER1="root"   # 替换为SSH用户名
USER2="root"
PROJECT_PATH="/root/Qamboo"  # 替换为项目在远程机器上的绝对路径
BIN_PATH="./target/release/q4"

cd $PROJECT_PATH

cargo build --release --package experiments --bin q4 --features tcp
scp target/release/q4 $USER1@$HOST1:$PROJECT_PATH/target/release/
scp target/release/q4 $USER2@$HOST2:$PROJECT_PATH/target/release/

# 本地运行 party 0
$BIN_PATH -c experiments/net/ -p 0 &
# SSH到HOST1运行 party 1
ssh $USER1@$HOST1 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/ -p 1" &
# SSH到HOST2运行 party 2
ssh $USER2@$HOST2 "cd $PROJECT_PATH && $BIN_PATH -c experiments/net/ -p 2"