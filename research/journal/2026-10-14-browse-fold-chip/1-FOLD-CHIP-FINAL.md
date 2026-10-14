# B337 — Mods Browse: fold an uploader with the mouse / a finger

B335 folded an uploader's mods with ← / → only; a tap on the heading is its Download all, so the pointer had no way.

## What changed
- `mods_screen.rs heading()` draws a grey "Fold" chip (or "Show" while folded) just left of the "Download all N"
  chip, and returns its rect; the title and its rule line end before it.
- `layout()` gives a heading row two hits: the whole row (`Row(r, ✕)`, Download all as before) and, drawn after so it
  wins, the chip (`Row(r, fold_press(folded))` = ← to fold, → to show). A tap selects the heading and runs the same
  `list_step` fold code as the keys, so the cursor lands on the heading exactly as with ←.
- Test `headings_and_side_by_side`: the chip's hit is found in the drawn hits, a tap at its centre folds bob's mods
  (8 → 6 rows), a tap on Show unfolds them, the heading's own Download all hit is still there.

## Checked
- `tools/check.sh -p hst --bin hst mods_screen`: 11 passed.
- `HST_MENU=browsemods --shot` at 1280×720 (`context/B337/browse.png`): Fold on open uploaders, Show on the folded
  one ("+ BY SOMEONE"), chips aligned, titles not squeezed.

## Not verified / not 1:1
- Mods Browse is our own front-end (no original to match).
- No portrait-window shot: `tools/shot.sh` (hyprctl launch) wrote no file in this session; the chip sits in the heading
  row inside the list box, which the existing layout already scales uniformly, and its width is fixed by its words.
