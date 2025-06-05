#!/bin/bash
cargo build --release
scp ./target/release/acme_test_server acme:/opt/acme
ssh acme systemctl restart acme
