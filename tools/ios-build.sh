#!/usr/bin/env bash
# Cross-build hst for iPhone on Linux: clang + rustup's ld64.lld against the SDK from tools/ios-sdk.py.
# Usage: tools/ios-build.sh [cargo args…]   (default: build --release -p hst)
set -euo pipefail
sdk=${HST_IOS_SDK:-$HOME/.local/share/hst-ios/iPhoneOS.sdk}
[ -d "$sdk" ] || { echo "no SDK at $sdk: run tools/ios-sdk.py Xcode.xip" >&2; exit 1; }
lld=$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/ld64.lld
min=15.0  # iPhone 13 Pro shipped with 15
export SDKROOT=$sdk IPHONEOS_DEPLOYMENT_TARGET=$min
export CC_aarch64_apple_ios=clang CXX_aarch64_apple_ios=clang++
export CFLAGS_aarch64_apple_ios="-target arm64-apple-ios$min -isysroot $sdk"
export CXXFLAGS_aarch64_apple_ios=$CFLAGS_aarch64_apple_ios
export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER=clang
export CARGO_TARGET_AARCH64_APPLE_IOS_RUSTFLAGS="-C link-arg=-target -C link-arg=arm64-apple-ios$min -C link-arg=-isysroot -C link-arg=$sdk -C link-arg=-fuse-ld=lld -C link-arg=--ld-path=$lld"
[ $# -gt 0 ] || set -- build --release -p hst
exec cargo "$1" --target aarch64-apple-ios "${@:2}"
