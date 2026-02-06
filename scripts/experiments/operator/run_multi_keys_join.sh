cargo build --release --package experiments --bin multi_keys_join --features tcp

BIN_PATH=../../../target/release/multi_keys_join

$BIN_PATH -c ../../net/ -p 0 &
$BIN_PATH -c ../../net/ -p 1 &
$BIN_PATH -c ../../net/ -p 2
