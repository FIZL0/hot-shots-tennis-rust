#!/usr/bin/env bash
# Screenshot a window by class/title substring (default: PCSX2) without focusing it.
#   tools/screenshot.sh out.png [pattern]
# With HST_PCSX2=N (a parallel copy, usually on a hidden workspace where grim sees nothing) it sends that copy's own
# Screenshot hotkey (F8) instead and copies the PNG it writes (full size; fails while the copy is paused).
set -euo pipefail
out=${1:?usage: screenshot.sh out.png [window pattern]}
pat=${2:-pcsx2}
if [ -n "${HST_PCSX2:-}" ]; then
    T=$(cd "$(dirname "$0")" && pwd)
    d="$(dirname "$(git -C "$T" rev-parse --path-format=absolute --git-common-dir)")-slots/pcsx2/s$HST_PCSX2/PCSX2/snaps"
    pid=$(cat "${XDG_RUNTIME_DIR:-/tmp}/hst-pcsx2$HST_PCSX2.pid")
    mkdir -p "$d"; before=$(ls "$d" | wc -l)
    # no Hyprland window (compositor without a monitor, stale signature): press the copy's pad-bound Screenshot
    # hotkey instead (L3 + R3; set "Screenshot = SDL-0/LeftStick & SDL-0/RightStick" in the copy's PCSX2.ini)
    hyprctl dispatch "hl.dsp.send_shortcut({ mods = \"\", key = \"F8\", window = \"pid:$pid\" })" >/dev/null 2>&1 ||
        "$T/vpad.py" send "down l3" "down r3" "sleep 120" "up r3" "up l3"
    for _ in $(seq 10); do sleep 0.3; [ "$(ls "$d" | wc -l)" != "$before" ] && break; done
    # a copy whose ini binds Screenshot to the pad only ignores F8: press L3 + R3 too
    [ "$(ls "$d" | wc -l)" != "$before" ] || "$T/vpad.py" send "down l3" "down r3" "sleep 120" "up r3" "up l3"
    for _ in $(seq 20); do sleep 0.3; [ "$(ls "$d" | wc -l)" != "$before" ] && break; done
    [ "$(ls "$d" | wc -l)" != "$before" ] || { echo "PCSX2 copy $HST_PCSX2 wrote no screenshot (paused?)" >&2; exit 1; }
    f=$d/$(ls "$d" | sort -V | tail -1)
    # PCSX2 writes the PNG after creating the file: wait until its size settles
    for _ in $(seq 30); do s=$(stat -c %s "$f"); sleep 0.3; [ "$s" -gt 0 ] && [ "$s" = "$(stat -c %s "$f")" ] && break; done
    cp "$f" "$out"
    echo "$out"
    exit
fi
geom=$(hyprctl clients -j | jq -r --arg p "$pat" '
  [.[] | select((.class + " " + .title) | ascii_downcase | contains($p | ascii_downcase))][0]
  | select(. != null) | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"')
[ -n "$geom" ] || { echo "no window matching '$pat'" >&2; exit 1; }
grim -g "$geom" "$out"
echo "$out"
