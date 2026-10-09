#!/bin/bash
# tools/slow.sh [hst args…] — run the game as a weak device would: 2 CPU cores at 60 % of one each (≈ a Steam Deck
# under load, worse), and with `HST_SLOW_IO=1` disk reads/writes throttled like a cheap microSD (10 MB/s read, 5 MB/s
# write, 300/150 IOPS). CPU limits use the user's own cgroup; the io controller isn't delegated to user cgroups, so
# the I/O limits run the game in a system scope through sudo (it still runs as you).
# HST_CPUS / HST_QUOTA / HST_READ / HST_WRITE override the limits. HST_BIN the binary, HST_ISO the ISO.
set -e
cd "$(dirname "$0")/.."
iso=$(realpath -s "${HST_ISO:-Hot Shots Tennis (USA).iso}")
bin=$(realpath "${HST_BIN:-target/release/hst}")
cpu=(-p "AllowedCPUs=${HST_CPUS:-0-1}" -p "CPUQuota=${HST_QUOTA:-120%}")
if [ "${HST_SLOW_IO:-0}" = 1 ]; then
    io=(-p "IOReadBandwidthMax=$PWD ${HST_READ:-10M}" -p "IOWriteBandwidthMax=$PWD ${HST_WRITE:-5M}"
        -p "IOReadIOPSMax=$PWD 300" -p "IOWriteIOPSMax=$PWD 150")
    # drop the page cache first, so reads really come off the (throttled) disk
    sync && echo 1 | sudo tee /proc/sys/vm/drop_caches >/dev/null
    exec sudo systemd-run --scope --quiet --uid="$(id -u)" --gid="$(id -g)" \
        --setenv=WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}" --setenv=XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
        --setenv=HOME="$HOME" --setenv=HST_FPS="${HST_FPS:-1}" "${cpu[@]}" "${io[@]}" "$bin" "$iso" "$@"
fi
HST_FPS=${HST_FPS:-1} exec systemd-run --user --scope --quiet "${cpu[@]}" "$bin" "$iso" "$@"
