#!/bin/sh
# Builds the browser version into web/pkg (WebGL2) and web/pkg-webgpu (WebGPU; the page picks): serve web/ over http
# on localhost, or https (OPFS needs a secure context). Needs wasm-bindgen-cli at Cargo.lock's wasm-bindgen version and
# wasm-opt (binaryen). WEB_FAST=1 skips wasm-opt.
set -e
cd "$(dirname "$0")/.."
build() {
  CARGO_TARGET_DIR=$PWD/target-wasm cargo build -p hst --target wasm32-unknown-unknown --release $2
  wasm-bindgen --target web --no-typescript --out-dir "web/$1" target-wasm/wasm32-unknown-unknown/release/hst.wasm
  [ -n "$WEB_FAST" ] || wasm-opt -O2 --strip-debug --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals --enable-reference-types --enable-multivalue "web/$1/hst_bg.wasm" -o "web/$1/hst_bg.wasm"
}
build pkg
build pkg-webgpu "--features webgpu"
