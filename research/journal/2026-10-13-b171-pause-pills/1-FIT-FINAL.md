# B171 — pause menu labels fit their pills

## Problem
`play/menu.rs` drew each option's 256×32 label row (`main_menu/pause::label`, the disc `word.tm2` glyphs, set again
by `menu_art::relabel` in the TTF) at 256×32 over a 288×32 pill: the ink (glyphs + outline) is ~30 px of the 32,
so the words covered the pill's rims top and bottom. With `--original` (no relabel) the disc glyph row was also
clipped at 256 px: "Resume the Matc", "Return to the Mer".

## Fix
- `menu.rs`: rows drawn at `FIT` = 0.625 (160×20 of panel px), centred on their pill (x 240, y pill + 6). Same 640×448
  panel coordinates as the pills, so `widescreen::place`'s Centre anchor keeps the ratio at any aspect/resolution.
- `main_menu/pause.rs::label`: words wider than the 256-px row are set on a wider one and shrunk uniformly
  (box filter, `shrink`) to fit it, centred; so any string (mod/other-language words) fits its row, and the row
  always fits the pill. Unit test `shrink_fits`; `slot3_pause` checks the new rects.

## Checked (`--shot`, `HST_PAUSE=6 --play --shot-at 8`, files in `context/b171/run1/`)
16:9 1280×720 (before/after), 16:9 854×480, 4:3 640×480, 21:9 1720×720, `--original` 1920×1080: every label inside
its pill with margin, none clipped.

## Not verified / not 1:1
- Not compared with the original's dialog: B141 replaced its pills and words with ours; the size is our choice
  (front-end, PLAN: ours to design).
- Only the three English strings we show exist; other languages / mod strings were checked through the code path
  (wider rows shrink to fit), not with a shot of a real translated string — the port has none yet.
