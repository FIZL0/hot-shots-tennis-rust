#!/bin/bash
# tools/select_stress.sh SECS DIR [hst env…] — B86's crash hunt: the menu's character select (`HST_MENU=chars`, four
# cards) with `HST_STRESS=SECS` jumping every card to a random character, costume, page and mod each frame, under
# tools/vram_watch.sh. DIR gets log.txt (the app's output; `panicked` = a crash) and vram.txt (time, card VRAM, own VRAM, RSS; MB).
set -u
cd "$(dirname "$0")/.."
secs=$1 dir=$2; shift 2
mkdir -p "$dir"
export WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-wayland-1} HST_BG=1 HST_MENU=chars HST_SELECT=0,1,2,3 HST_STRESS=$secs
bin=${HST_BIN:-${CARGO_TARGET_DIR:-target}/release/hst}
env "$@" "$bin" "Hot Shots Tennis (USA).iso" > "$dir/log.txt" 2>&1 &
pid=$!
tools/vram_watch.sh $pid "$dir/vram.txt" ${HST_VRAM_GB:-3} &
wait $pid; code=$?
echo "exit $code"
grep -m3 -A3 panicked "$dir/log.txt"
awk '{m=$3>m?$3:m; r=$4>r?$4:r} NR==5{a=$3} END{print "own VRAM at 5 s", a, "max", m, "last", $3, "MB; RSS max", r, "MB"}' "$dir/vram.txt"
