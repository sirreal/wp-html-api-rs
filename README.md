Check this out, yes it has a git submodule.

```sh
submodule update --init --recursive
./build-and-run.sh
```

To build the wasm build:
- Install `wasm-pack` via https://rustwasm.github.io/wasm-pack/installer/
- Run:
  ```sh
  wasm-pack build crates/wp-html-api-wasm --release --no-pack --target=nodejs
  ./demo.js
  ```
