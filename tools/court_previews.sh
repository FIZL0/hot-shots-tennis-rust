#!/bin/bash
# tools/court_previews.sh MODDIR[:SECONDS]… — B401: each court mod's `previews/court.png` (640×360, the court select's
# picture): a `--shot` of the match intro's flyover SECONDS (default 2) in on the mod (HST_INTRO=1, no 2D:
# HST_COURT_PREVIEW=1), scaled down. An enclosed set the layout court's flyover starts outside of (Get a Grip's TV
# studio and Love Tennis courts) takes 4: the match camera, after the flyover.
# Needs `cargo build --release -p hst` and a Wayland/Hyprland session (tools/shot.sh); writes into the mod folders.
set -eo pipefail
cd "$(dirname "$0")/.."
[ -x "${HST_BIN:-target/release/hst}" ]
bin=$(realpath "${HST_BIN:-target/release/hst}")
tmp=$(mktemp -d)
trap 'rm -r "$tmp"' EXIT
# tools/shot.sh starts the game through hyprctl, which drops the environment: a wrapper carries it
printf '#!/bin/bash\nexport HST_INTRO=1 HST_COURT_PREVIEW=1\nexec %q "$@"\n' "$bin" > "$tmp/hst"
chmod +x "$tmp/hst"
for arg in "$@"; do
    dir=${arg%:*}
    at=2
    [ "$dir" = "$arg" ] || at=${arg##*:}
    name=$(basename "$dir")
    HST_BIN="$tmp/hst" tools/shot.sh "$tmp/$name.png" --play --singles --court-mod "$dir" --chars 0,1 --shot-at "$at"
    mkdir -p "$dir/previews"
    python3 - "$tmp/$name.png" "$dir/previews/court.png" <<'PY'
import sys
from PIL import Image
Image.open(sys.argv[1]).convert("RGBA").resize((640, 360), Image.LANCZOS).save(sys.argv[2], optimize=True)
PY
    echo "$dir/previews/court.png"
done
