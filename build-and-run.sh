#!/usr/bin/env bash

set -e
set -u

cargo build --release --quiet -p wp-html-api-php-ext
echo "Running demo.php"
php -d extension=target/release/libwp_html_api_php_ext.dylib demo.php

wasm-pack build --release --no-pack --target=nodejs --out-dir="../../pkg-node"  crates/wp-html-api-wasm --quiet
echo "Running demo.js"
node ./demo.js
