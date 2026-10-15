#!/bin/bash
# tools/bench.sh LOG SECONDS [hst args…] — frame-rate bench (P23): a 4-bot doubles match (court 1 unless args say, as
# `--stage N --court N`: the stage is what is drawn, the court its physics)
# in a floating 1280×800 window (the Steam Deck's size; HST_WINDOW overrides) on workspace 1, unfocused,
# with `HST_FPS=1`'s log in LOG; killed after SECONDS. HST_GPU=igpu draws on the CPU's integrated Radeon (weaker than the
# Deck's 8 CUs: a stand-in for its GPU class); HST_SLOW=1 adds tools/slow.sh's 2-core CPU limit; HST_ENV adds environment (e.g. `HST_PASSES=1`: per-pass GPU ms in LOG); HST_WRAP prefixes the command (e.g. `samply record --save-only -o P --`).
set -e
cd "$(dirname "$0")/.."
log=$(realpath -m "$1"); secs=$2; shift 2
iso=$(realpath -s "${HST_ISO:-Hot Shots Tennis (USA).iso}")
bin=$(realpath "${HST_BIN:-${CARGO_TARGET_DIR:-target}/release/hst}")
args=$([ $# = 0 ] || printf ' %q' "$@")
env="${HST_ENV:-} HST_BG=1 HST_FPS=1 HST_AUTOPLAY=1 HST_WINDOW='${HST_WINDOW:-1280 800}'"
[ "${HST_GPU:-}" = igpu ] && env+=" WGPU_ADAPTER_NAME=raphael"
wrap="${HST_WRAP:-}"
[ "${HST_SLOW:-0}" = 1 ] && wrap="systemd-run --user --scope --quiet -p AllowedCPUs=0-1 -p CPUQuota=120%"
rm -f "$log"
hyprctl dispatch "hl.dsp.exec_cmd([[cd $(printf %q "$PWD") && env $env timeout $secs $wrap $bin $(printf %q "$iso") --play --stage 1 --court 1 --chars 0,1,2,3$args > $(printf %q "$log") 2>&1]], { float = true, size = '${HST_WINDOW:-1280 800}', no_initial_focus = true })" >/dev/null
sleep $((secs + 2))
grep -E "AdapterInfo|^fps|^cpu" "$log" | sed 's/.*name: "\([^"]*\)".*/adapter: \1/'
