#!/bin/bash
# tools/build.sh [linux|windows|ios|all] — optimised (release) builds of the game into dist/:
#   dist/linux/opentennis        run: ./opentennis "Hot Shots Tennis (USA).iso"
#   dist/windows/opentennis.exe  run: drop the ISO on opentennis.exe, or `opentennis.exe "Hot Shots Tennis (USA).iso"`
#   dist/ios/opentennis-ios.ipa  unsigned (M91): the player signs it (AltStore, Sideloadly…); the ISO goes in its Files folder.
#                         Needs the iOS SDK once: tools/ios-sdk.py Xcode_*.xip (see tools/ios-build.sh)
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
        -e HST_VERSION -e HOME=/cache -e CARGO_HOME=/cache/cargo -e RUSTUP_HOME=/cache/rustup -e CARGO_TARGET_DIR=/src/target/steamrt \
        $sdk sh -ec '
            [ -x /cache/cargo/bin/cargo ] || curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal --no-modify-path
            /cache/cargo/bin/cargo build --release -p hst'
    rm -rf dist/linux && mkdir -p dist/linux  # the tarball is the whole folder: nothing stale in it
    strip -o dist/linux/opentennis target/steamrt/release/hst
    # the icon on Wayland/X11 docks (the window's app id is "opentennis"): opentennis.desktop into ~/.local/share/applications, opentennis.png into
    # ~/.local/share/icons, opentennis on the PATH
    cp crates/hst/icon/hst.png dist/linux/opentennis.png
    printf '[Desktop Entry]\nType=Application\nName=OpenTennis\nExec=opentennis\nIcon=opentennis\nStartupWMClass=opentennis\nCategories=Game;\n' > dist/linux/opentennis.desktop
    local need glibc=2.31
    need=$(objdump -T dist/linux/opentennis | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)
    echo "dist/linux/opentennis needs $need (sniper has GLIBC_$glibc)"
    if [ "$(printf '%s\n' "${need#GLIBC_}" $glibc | sort -V | tail -1)" != $glibc ]; then
        echo "dist/linux/opentennis needs $need, newer than GLIBC_$glibc: won't start on SteamOS" >&2
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
    rm -rf dist/windows && mkdir -p dist/windows
    x86_64-w64-mingw32-strip -o dist/windows/opentennis.exe target/$t/release/hst.exe
    echo "dist/windows/opentennis.exe"
}

case $what in
    linux) linux ;;
    windows) windows ;;
    ios) mkdir -p dist/ios && tools/ios-ipa.sh dist/ios/opentennis-ios.ipa ;;
    all) linux; windows; mkdir -p dist/ios && tools/ios-ipa.sh dist/ios/opentennis-ios.ipa ;;
    *) echo "usage: tools/build.sh [linux|windows|ios|all]" >&2; exit 2 ;;
esac
