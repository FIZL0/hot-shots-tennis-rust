#!/bin/bash
# tools/build.sh [linux|windows|all] — optimised (release) builds of the game into dist/:
#   dist/linux/hst        run: ./hst "Hot Shots Tennis (USA).iso"
#   dist/windows/hst.exe  run: drop the ISO on hst.exe, or `hst.exe "Hot Shots Tennis (USA).iso"`
# mods/ and settings.txt live beside the ISO, not the binary. No game data goes into dist/.
# Windows is cross-compiled with MinGW. One-time setup (Arch):
#   sudo pacman -S rustup mingw-w64-gcc     # rustup replaces the `rust` package
#   rustup default stable && rustup target add x86_64-pc-windows-gnu
# Linux links against glibc 2.31 with cargo-zigbuild so it runs on SteamOS / Steam Deck (SteamOS 3.5 ships 2.37,
# 3.7 2.41; 2.31 is Valve's Steam Runtime "sniper", so it also runs inside it). One-time setup:
#   cargo install --locked cargo-zigbuild && pip install --user ziglang   # or: sudo pacman -S zig
set -e
cd "$(dirname "$0")/.."
what=${1:-linux}

glibc=2.31

linux() {
    if ! command -v cargo-zigbuild >/dev/null; then
        echo "linux build needs (once): cargo install --locked cargo-zigbuild && pip install --user ziglang" >&2
        exit 1
    fi
    # allow-shlib-undefined: the dev machine's libasound/libudev reference its newer glibc; the player's own copies
    # are loaded at run time, so only the binary's own symbol versions matter (checked below)
    RUSTFLAGS="-C link-arg=-Wl,--allow-shlib-undefined" \
        cargo zigbuild --release -p hst --target x86_64-unknown-linux-gnu.$glibc
    mkdir -p dist/linux
    strip -o dist/linux/hst target/x86_64-unknown-linux-gnu/release/hst
    local need
    need=$(objdump -T dist/linux/hst | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)
    echo "dist/linux/hst needs $need (target GLIBC_$glibc)"
    if [ "$(printf '%s\n' "${need#GLIBC_}" $glibc | sort -V | tail -1)" != $glibc ]; then
        echo "dist/linux/hst needs $need, newer than GLIBC_$glibc: won't start on SteamOS" >&2
        exit 1
    fi
}

windows() {
    local t=x86_64-pc-windows-gnu
    if ! command -v x86_64-w64-mingw32-gcc >/dev/null || ! rustc --print target-libdir --target $t 2>/dev/null | xargs test -d; then
        echo "windows build needs (once): sudo pacman -S rustup mingw-w64-gcc; rustup default stable; rustup target add $t" >&2
        exit 1
    fi
    # crt-static: no MinGW runtime DLLs to ship beside the .exe
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc RUSTFLAGS="-C target-feature=+crt-static" \
        cargo build --release -p hst --target $t
    mkdir -p dist/windows
    x86_64-w64-mingw32-strip -o dist/windows/hst.exe target/$t/release/hst.exe
    echo "dist/windows/hst.exe"
}

case $what in
    linux) linux ;;
    windows) windows ;;
    all) linux; windows ;;
    *) echo "usage: tools/build.sh [linux|windows|all]" >&2; exit 2 ;;
esac
