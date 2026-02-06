#!/usr/bin/env bash
cargo run --release --example three_party_tcp --features tcp -- -c config_party1.toml &
cargo run --release --example three_party_tcp --features tcp -- -c config_party2.toml &
cargo run --release --example three_party_tcp --features tcp -- -c config_party3.toml 
