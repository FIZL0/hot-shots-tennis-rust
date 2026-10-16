#!/bin/bash
# tools/shot.sh OUT.png [hst args…] — (HST_SIZE='W H' for another window size) `hst <iso> … --shot OUT.png` in a floating 1280×720 window (2048×1152 PNG at
# the desktop's scale) on workspace 1, unfocused and lowered under the user's window, so every shot has the
# same size whatever else is tiled. The window is titled "Hot Shots Tennis (shot)" (any `--shot` run): the user's
# ~/.config/hypr/hyprland.lua sends that title and class pcsx2-qt to workspace 1, unfocused, under every other window.
# HST_ISO overrides the ISO (a symlink in a folder without mods/texture-replacements shows the disc textures: the
# symlink is kept, not resolved); HST_BIN the binary. Other HST_* variables are passed on (hyprctl's exec drops the
# environment). HST_WAIT: seconds to wait for OUT (60; raise it with a late --shot-at). Promo shots (M98): `--no-hud --hud-key` (F10 toggles it),
# more frames from one run with HST_KEYS='20:shot=a.png 21:shot=b.png' (real-time seconds), any aspect with HST_SIZE.
# A/B shots: pin the match seed (HST_WEATHER_SEED=1234, or HST_WEATHER=0): a clock-picked one rolls the
# weather, and a cloudy roll draws the open court ~7.5 % darker than a clear one (B404).
set -e
cd "$(dirname "$0")/.."
out=$(realpath -m "$1"); shift
iso=$(realpath -s "${HST_ISO:-Hot Shots Tennis (USA).iso}")
bin=$(realpath "${HST_BIN:-${CARGO_TARGET_DIR:-target}/release/hst}")
args=$([ $# = 0 ] || printf ' %q' "$@")
envs=$(env | grep -E '^HST_[A-Z0-9_]+=' | grep -vE '^HST_(ISO|BIN|SIZE|PCSX2)=' | while IFS= read -r kv; do printf '%q ' "$kv"; done)
rm -f "$out"
hyprctl dispatch "hl.dsp.exec_cmd([[cd $(printf %q "$PWD") && ${envs:+env $envs}$bin $(printf %q "$iso")$args --shot $(printf %q "$out")]], { float = true, size = '${HST_SIZE:-1280 720}', no_initial_focus = true })" >/dev/null
for _ in $(seq $((${HST_WAIT:-60} * 2))); do
    [ -s "$out" ] && sleep 0.5 && exit 0
    sleep 0.5
done
echo "tools/shot.sh: no $out after ${HST_WAIT:-60} s" >&2
exit 1
