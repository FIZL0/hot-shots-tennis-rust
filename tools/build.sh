#!/bin/bash
# tools/build.sh [linux|windows|all] — optimised (release) builds of the game into dist/:
#   dist/linux/hst        run: ./hst "Hot Shots Tennis (USA).iso"
#   dist/windows/hst.exe  run: drop the ISO on hst.exe, or `hst.exe "Hot Shots Tennis (USA).iso"`
# mods/ and settings.txt live beside the ISO, not the binary. No game data goes into dist/.
# Windows is cross-compiled with MinGW. One-time setup (Arch):
#   sudo pacman -S rustup mingw-w64-gcc     # rustup replaces the `rust` package
#   rustup default stable && rustup target add x86_64-pc-windows-gnu
# Linux is built inside Valve's Steam Runtime "sniper" SDK container (glibc 2.31, Debian 11), so it runs on every
# SteamOS 3.x / Steam Deck (3.5 ships glibc 2.37, 3.7 2.41) and inside the Steam Runtime. One-time setup:
#   sudo pacman -S docker && sudo systemctl enable --now docker && sudo usermod -aG docker $USER   # then log in again
# The container's Rust toolchain and cargo cache live in ~/.cache/hst-steamrt; its build in target/steamrt.
set -e
cd "$(dirname "$0")/.."
what=${1:-linux}

sdk=registry.gitlab.steamos.cloud/steamrt/sniper/sdk:latest

linux() {
    if ! docker info >/dev/null 2>&1; then
        echo "linux build needs docker (once): sudo pacman -S docker; sudo systemctl enable --now docker; sudo usermod -aG docker \$USER; log in again" >&2
        exit 1
    fi
    mkdir -p ~/.cache/hst-steamrt
    docker run --rm -u "$(id -u):$(id -g)" -v "$PWD:/src" -v ~/.cache/hst-steamrt:/cache -w /src \
        -e HOME=/cache -e CARGO_HOME=/cache/cargo -e RUSTUP_HOME=/cache/rustup -e CARGO_TARGET_DIR=/src/target/steamrt \
        $sdk sh -ec '
            [ -x /cache/cargo/bin/cargo ] || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal --no-modify-path
            /cache/cargo/bin/cargo build --release -p hst'
    mkdir -p dist/linux
    strip -o dist/linux/hst target/steamrt/release/hst
    local need glibc=2.31
    need=$(objdump -T dist/linux/hst | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)
    echo "dist/linux/hst needs $need (sniper has GLIBC_$glibc)"
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
