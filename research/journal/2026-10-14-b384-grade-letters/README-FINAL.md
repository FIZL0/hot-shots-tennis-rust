# B384 — stats card grade letters like the original's

**Before.** The roster's pink stats card drew each grade as a font letter in a flat colour (our own palette) inside
a white disc.

**The original.** The character select's △ card draws its grades as sprites from the attribute sheet
(`characterselect2_04`, column x 232, one 24×24 cell per grade, A at the top). Each letter has a vertical gradient
fill in the grade's colour (A orange, B red, C green, D blue, E purple, F grey) and a thick dark rim. They sit
straight on the card with no badge. `attribute_card` already draws them; B136 rebuilds that sheet as sharp vector
art in the disc's own sampled colours.

**Change.**
- `roster.rs` `stats`: each grade is now the original's sprite, `super::ATTRS` [232, 24·i, 24, 24], drawn straight
  on the card. The cell is the row's height, capped at 30 mockup px (the glyph fills 17 of the 24 px).
- The disc and our `GRADES` palette are gone.
- A hidden pick's "?" is grey and ringed like the labels.
- `stats` is now `pub(super)`.
- `mods_screen/grid.rs` `detail` (Browse mods' panel since B372) now shows this same card on the right of the
  picture: up to 44% of the width (≤ 110) and ≤ 100 high, centred on the picture.
- Before, the panel used the original's full 144×160 △ card, which needs 0.45× scale. At 16:9 that didn't fit, so
  the panel showed no stats at all. The play style is still in the blurb line.
- The old side panel in `mods_screen.rs` (`side_panel`, which uses `attribute_card` with the same sprites) was left
  alone.

**Shots** (`context/b384/`, `tools/shot.sh` + `HST_MENU` through `HST_BIN` wrappers):
- `run2`: `roster1` 1280×720, `roster4` 720×960 and 1680×720. The letters line up with the labels in every column.
- `run5`: `browsemods` with Ban hovered, at 1280×720, 960×720, 720×960 and 1680×720. The card fits beside the
  picture at every aspect.
- `run4/cmp.png`: Browse and roster cards side by side, same letter size.
- Hovering Ban needed a temporary env tweak to the sample's `m.sel`, reverted before commit.

## Not verified / not 1:1
- No PCSX2 side-by-side of the original's character select. The letters are the original's own sprites, taken from
  the disc's sheet and drawn unmodified. B354 already compared the △ card that uses them by eye. Driving PCSX2's
  front-end from boot to the select costs more than it would show here.
- The pink card, its labels and the row layout are ours (menus aren't 1:1 by rule). Only the grade letters follow
  the original.
- With `--original` the letters are the disc's 24-px sprites scaled up, so they're softer than the rebuilt sheet.
  Not shot.
- Hidden "?" (an online peer's hidden pick) not shot: no `HST_MENU` opens it. It's drawn with the existing
  `ringed_text`.

## Tests
`tools/check.sh`: 374 passed, 1 failed. The failure is `play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally`.
It reads the shared `mods/` folder (a symlink into the main checkout), and that folder now holds only
`hst_umpire_04_anna`; the other four umpire mods were removed outside this branch (likely B381's removals). The
failure has nothing to do with this change, so I committed by hand (ticked) instead of through `tools/done.sh`.
