#!/bin/bash
# tools/court_previews.sh MODDIR[:SECONDS][@EYE]… — B401: each court mod's `previews/court.png` (640×360, the court select's
# picture): a `--shot` of the match intro's flyover SECONDS (default 2) in on the mod (HST_INTRO=1, no 2D:
# HST_COURT_PREVIEW=1), scaled down. The flyover is the layout court's, so the moment depends on the layout: the ones
# in use (2026-10-15) were Get a Grip 2 (c16, c17: 4, their layout's flyover starts outside the set), Fore/OT2 4,
# layouts 7 and 9 3 (later is the match camera under the "Start the Match!" banner), Out of Bounds c06 2.
# A closed set the flyover never enters (Get a Grip 2's c16 TV studio and c17 training court, layout 3: by the time it
# reaches the court it has handed over to the match camera) takes a fixed camera instead: @EYE is an HST_EYE view
# (game space eye,forward,down,tan), e.g. gag_c16_tvstudio@9,-7,17,-9,6,-17,0,1,0,0.75 (high corner inside the room).
# Needs `cargo build --release -p hst` and a Wayland/Hyprland session (tools/shot.sh); writes into the mod folders.
set -eo pipefail
cd "$(dirname "$0")/.."
[ -x "${HST_BIN:-target/release/hst}" ]
tmp=$(mktemp -d)
trap 'rm -r "$tmp"' EXIT
for arg in "$@"; do
    eye=
    [[ $arg == *@* ]] && eye=${arg#*@} && arg=${arg%%@*}
    dir=${arg%:*}
    at=2
    [ "$dir" = "$arg" ] || at=${arg##*:}
    name=$(basename "$dir")
    HST_EYE=$eye HST_INTRO=1 HST_COURT_PREVIEW=1 tools/shot.sh "$tmp/$name.png" --play --singles --court-mod "$dir" --chars 0,1 --shot-at "$at"
    mkdir -p "$dir/previews"
    python3 - "$tmp/$name.png" "$dir/previews/court.png" <<'PY'
import sys
from PIL import Image
Image.open(sys.argv[1]).convert("RGBA").resize((640, 360), Image.LANCZOS).save(sys.argv[2], optimize=True)
PY
    echo "$dir/previews/court.png"
done
