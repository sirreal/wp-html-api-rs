#!/usr/bin/env bash

set -e
set -u

cargo build --release
./target/release/cargo-php stubs --manifest crates/wp-html-api/Cargo.toml
php -d extension=target/debug/libwp_html_api.dylib demo.php
