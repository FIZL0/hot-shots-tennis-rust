#!/bin/sh
# Builds the browser version into web/pkg (serve web/ over http on localhost, or https: OPFS needs a secure context).
# Needs wasm-bindgen-cli at Cargo.lock's wasm-bindgen version.
set -e
cd "$(dirname "$0")/.."
CARGO_TARGET_DIR=$PWD/target-wasm cargo build -p hst --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target-wasm/wasm32-unknown-unknown/release/hst.wasm
