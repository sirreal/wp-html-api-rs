#!/usr/bin/env bash

set -e
set -u

cargo build --release
./target/release/cargo-php stubs --manifest crates/wp-html-api-php-ext/Cargo.toml
php -d extension=target/release/libwp_html_api_php_ext.dylib demo.php -i data/html-standard.html
