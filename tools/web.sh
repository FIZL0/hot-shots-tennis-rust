#!/bin/sh
# Builds the browser version into web/pkg (WebGL2) and web/pkg-webgpu (WebGPU; the page picks), an installable app (manifest, sw.js, touch.js): serve web/ over http
# on localhost, or https (OPFS needs a secure context). Needs wasm-bindgen-cli at Cargo.lock's wasm-bindgen version and
# wasm-opt (binaryen). WEB_FAST=1 skips wasm-opt.
set -e
cd "$(dirname "$0")/.."
build() {
  CARGO_TARGET_DIR=$PWD/target-wasm cargo build -p hst --target wasm32-unknown-unknown --release --features ship $2
  wasm-bindgen --target web --no-typescript --out-dir "web/$1" target-wasm/wasm32-unknown-unknown/release/hst.wasm
  # B121: WebKit's getMappedRange hands back a 0-byte ArrayBuffer for a mapped-at-creation buffer on a lost device (or one
  # it couldn't allocate), and wgpu's view over it threw "Length out of range of buffer" through the game before Bevy
  # saw the loss. A view that doesn't fit its buffer is a scratch array instead; the device loss then reaches
  # index.html, which reloads.
  sed -i 's|const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);|const ret = (arg1 >>> 0) + (arg2 >>> 0) <= arg0.byteLength ? new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0) : new Uint8Array(arg2 >>> 0);|' "web/$1/hst.js"
  ! grep -q 'const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);' "web/$1/hst.js"
  [ -n "$WEB_FAST" ] || wasm-opt -O2 --strip-debug --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals --enable-reference-types --enable-multivalue "web/$1/hst_bg.wasm" -o "web/$1/hst_bg.wasm"
}
build pkg
build pkg-webgpu "--features webgpu"
# M96: the site's pictures, our own renderer's promo shots (docs/screenshots, AGENT.md), copied in at build time
mkdir -p web/shots && cp docs/screenshots/*.jpg web/shots/
# B465: iOS asks for /apple-touch-icon.png(-precomposed) when it misses the page's link (home-screen icon)
cp web/icon-180.png web/apple-touch-icon.png && cp web/icon-180.png web/apple-touch-icon-precomposed.png
# B317: the build's commit, which an open page polls to offer a Refresh once a newer one is deployed (index.html)
git rev-parse --short=7 HEAD 2>/dev/null > web/version.txt || date +%s > web/version.txt
