#!/bin/bash
# tools/build.sh [linux|windows|all] — optimised (release) builds of the game into dist/:
#   dist/linux/hst        run: ./hst "Hot Shots Tennis (USA).iso"
#   dist/windows/hst.exe  run: drop the ISO on hst.exe, or `hst.exe "Hot Shots Tennis (USA).iso"`
# mods/ and settings.txt live beside the ISO, not the binary. No game data goes into dist/.
# Windows is cross-compiled with MinGW. One-time setup (Arch):
#   sudo pacman -S rustup mingw-w64-gcc     # rustup replaces the `rust` package
#   rustup default stable && rustup target add x86_64-pc-windows-gnu
set -e
cd "$(dirname "$0")/.."
what=${1:-linux}

linux() {
    cargo build --release -p hst
    mkdir -p dist/linux
    strip -o dist/linux/hst target/release/hst
    echo "dist/linux/hst"
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
