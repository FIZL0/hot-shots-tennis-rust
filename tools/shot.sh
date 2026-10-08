#!/bin/bash
# tools/shot.sh OUT.png [hst args…] — `hst <iso> … --shot OUT.png` in a floating 1280×720 window (2048×1152 PNG at
# the desktop's scale) on a hidden special workspace, so every shot has the same size whatever else is tiled.
# HST_ISO overrides the ISO (a symlink in a folder without mods/texture-replacements shows the disc textures: the
# symlink is kept, not resolved); HST_BIN the binary.
set -e
cd "$(dirname "$0")/.."
out=$(realpath -m "$1"); shift
iso=$(realpath -s "${HST_ISO:-Hot Shots Tennis (USA).iso}")
bin=$(realpath "${HST_BIN:-target/release/hst}")
args=$(printf ' %q' "$@")
rm -f "$out"
hyprctl dispatch "hl.dsp.exec_cmd([[cd $(printf %q "$PWD") && $bin $(printf %q "$iso")$args --shot $(printf %q "$out")]], { float = true, size = '1280 720', no_initial_focus = true, workspace = 'special:hstshot silent' })" >/dev/null
for _ in $(seq 120); do
    [ -s "$out" ] && sleep 0.5 && exit 0
    sleep 0.5
done
echo "tools/shot.sh: no $out after 60 s" >&2
exit 1
