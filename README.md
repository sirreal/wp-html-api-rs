Check this out and run the demo. It has a submodule.

```sh
submodule update --init --recursive
./build-and-run.sh
```

To build the wasm build, you'll need [`wasm-pack`][wasm-pack].

## Building

### PHP Extension

```sh
cargo build --release --quiet -p wp-html-api-php-ext -p cargo-php
./target/release/cargo-php stubs --manifest crates/wp-html-api-php-ext/Cargo.toml
```

### JavaScript (Node.js)

```sh
wasm-pack build --release --no-pack --target=nodejs --out-dir="../../pkg-node"  crates/wp-html-api-wasm
```

### JavaScript (web)

```sh
RUSTFLAGS="-C opt-level=s" wasm-pack build --release --no-pack --target=web --out-dir="../../pkg-web" crates/wp-html-api-wasm
```

[wasm-pack]: https://rustwasm.github.io/wasm-pack/installer/
