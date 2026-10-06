#!/usr/bin/env bash
# Screenshot a window by class/title substring (default: PCSX2) without focusing it.
#   tools/screenshot.sh out.png [pattern]
set -euo pipefail
out=${1:?usage: screenshot.sh out.png [window pattern]}
pat=${2:-pcsx2}
geom=$(hyprctl clients -j | jq -r --arg p "$pat" '
  [.[] | select((.class + " " + .title) | ascii_downcase | contains($p | ascii_downcase))][0]
  | select(. != null) | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"')
[ -n "$geom" ] || { echo "no window matching '$pat'" >&2; exit 1; }
grim -g "$geom" "$out"
echo "$out"
