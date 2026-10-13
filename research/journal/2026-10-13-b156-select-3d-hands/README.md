# B156: character select's per-player 3D hands (FINAL)

The select's pointers (disc grid and the custom page) drew the flat `Tex::Hand` sprite tinted `COLOURS[p]`. They now
draw `Tex::Cursor`, the menus' 3D hand image (`main_menu/cursor.rs`, 768×384, so sharp at any size), with the same
rects, offsets and `COLOURS[p]` tint (the quad colour multiplies the lit grey hand). All four share the one hand
camera/image: same pose, so one render covers them. The draw's `PAD` growth keeps each at its old anchor.

Four human cards plus hands need ~760 quads, over the 700-slot pool: 4P's card and hand vanished. `POOL` is now 1000.

`HST_MENU=chars4` opens the select with four humans on one portrait (for `--shot`). Shots: `context/b156/ours.png`
(16:9), `ours_wide.png` (1600×600), `crop.png` (the four hands): pink, orange, blue, green 3D hands, apart as the
flat ones were.

## Not verified / not 1:1
- Not compared with the original: the 4-player select is a menu (ours to design, PLAN 2026-10-12) and the original
  draws flat hands there; the task asked for the 3D one.
- The tint multiplies a lit mid-grey hand, so the colours are darker than the flat hands' (still clearly each
  player's colour, as in crop.png).
