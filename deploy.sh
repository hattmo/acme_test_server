#!/bin/bash
cargo build --release
ssh acme systemctl stop acme
scp ./target/release/acme_test_server acme:/opt/acme
ssh acme chmod +x /opt/acme
ssh acme systemctl start acme
