#!/bin/sh
# Builds the browser version into web/pkg (an installable app: manifest, sw.js, touch.js; serve web/ over http on localhost, or https: OPFS needs a secure context).
# Needs wasm-bindgen-cli at Cargo.lock's wasm-bindgen version.
set -e
cd "$(dirname "$0")/.."
CARGO_TARGET_DIR=$PWD/target-wasm cargo build -p hst --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target-wasm/wasm32-unknown-unknown/release/hst.wasm
# ≈ 126 MB → 59 MB (15 MB gzipped) for phones; ~25 s. Skipped when binaryen isn't installed
if command -v wasm-opt >/dev/null; then
  wasm-opt -O2 --strip-debug --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals web/pkg/hst_bg.wasm -o web/pkg/hst_bg.opt.wasm
  mv web/pkg/hst_bg.opt.wasm web/pkg/hst_bg.wasm
fi
