#!/usr/bin/env bash

set -e
set -u

cargo build --release --quiet -p wp-html-api-php-ext -p cargo-php
./target/release/cargo-php stubs --manifest crates/wp-html-api-php-ext/Cargo.toml
echo "Running demo.php"
php -d extension=target/release/libwp_html_api_php_ext.dylib demo.php

wasm-pack build --release --no-pack --target=nodejs --out-dir="../../pkg-node"  crates/wp-html-api-wasm --quiet
echo "Running demo.js"
node ./demo.js
