#!/usr/bin/env bash
# Pull iPhoneOS.sdk out of an Xcode .xip on Linux (no Mac, no sudo): bsdtar opens the xar, a pbzx decoder streams
# the payload into cpio, which keeps only the SDK. Usage: tools/ios-sdk.sh ~/Downloads/Xcode_X.xip [OUT_DIR]
# OUT_DIR defaults to ~/.local/share/hst-ios; the SDK lands at OUT_DIR/iPhoneOS.sdk.
set -euo pipefail
xip=$(realpath "$1"); out=${2:-$HOME/.local/share/hst-ios}
mkdir -p "$out/tmp"; cd "$out/tmp"
bsdtar -xOf "$xip" Content | python3 -I -c '
import lzma, struct, sys
r, w = sys.stdin.buffer, sys.stdout.buffer
assert r.read(4) == b"pbzx", "not a pbzx stream"
r.read(8)  # flags
while (h := r.read(16)):
    size, csize = struct.unpack(">QQ", h)
    chunk = r.read(csize)
    w.write(chunk if csize == size else lzma.decompress(chunk))
' | cpio -idm --quiet '*/iPhoneOS.platform/Developer/SDKs/*'
sdk=$(find . -maxdepth 10 -type d -name 'iPhoneOS[0-9]*.sdk' | head -1)
[ -n "$sdk" ] || { echo "no iPhoneOS SDK in $xip" >&2; exit 1; }
rm -rf "$out/iPhoneOS.sdk"; mv "$sdk" "$out/iPhoneOS.sdk"
cd "$out"; rm -rf tmp
echo "$out/iPhoneOS.sdk"
