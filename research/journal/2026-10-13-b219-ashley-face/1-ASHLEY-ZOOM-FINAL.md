# B219 — Ashley's face icon zoomed out

**State: FINAL.** Follows B176 (`../2026-10-13-b176-face-framing/`).

## Problem
B176 sizes the head square as `HEAD_FIT` (1.6) × the head's height (hair top to the head bone at the jaw). Ashley's
head is wide and short (visor/bow over a flat crown), so her square came out tighter than everyone else's: her face
filled the icon and the visor touched the top (`context/b219/before.png`, column 1).

## Fix
- `inspect::grow_square(sq, f, w, h)`: a square grown `f`× about its centre, kept inside the image.
- `previews.rs` bake: disc character 0 (Ashley) gets `ASHLEY_ZOOM` = 1.25 on her face square; the close-up is drawn
  from that square, so both the select grid icon and the match panel (B67 cuts its crop from the same close-up)
  follow. `RENDER` 7 → 8 so every cache bakes again.
- A crown-width term for everyone was already tried and dropped in B176 (changed almost nothing for the others), so
  this is per character, as the task asks.
- Test `inspect::tests::grow_square_stays_inside`.

## Checked
- `HST_BAKE=all` at r8: all 14 characters' costumes 0 and 3 (`context/b219/after.png`) and all 10 Ashley costumes
  (`ashley-all.png`): whole head with margin, visor and bows inside.
- `--shot`s: select grid (`context/b219/grid.png`) and a doubles match panel with Ashley costume 5 (`match.png`).

## Not verified / not 1:1
- Framing is ours (front-end, by eye); 1.25 tuned by eye against the rest of the cast, not measured.
- A mod using Ashley as donor keeps B176's framing (its own head shape), only disc Ashley is widened.
