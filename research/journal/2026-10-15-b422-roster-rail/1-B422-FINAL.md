# B422: built

## What changed
- **Category rail** (`roster.rs` `rail`, `rail_offset`, `clip_x`): one row of chips between the preview and the
  grid. It never scrolls with the grid. Chips: `★ 4`, `Hot Shots Tennis 16`, one per mod game, `Mods n`. A single
  section still gets its chip and count. The cursor's section is filled in the seat colour; the other chips are
  outlined, and so is the current one when the seat isn't picking. Chips keep their natural width. When they don't fit,
  the rail shifts sideways only as far as it needs to keep the current chip clear of the fade (`rail_offset`, no fade
  after the last chip). At a cut edge the chips are clipped (glyph sources too, never squeezed), a fade in the card's
  tint goes over them, and a `«` / `»` marks the cut. A tap on a chip is `Act::Section`, the same jump as the headings
  and L2 / R2.
- **Header band** (`band`): a solid seat-colour strip across the card's top (rounded like the card's top corners). It
  shows a big `1P`–`4P`, the character's name over `player name - device` (the B371 name, when set), and a
  Near / Far pill on the right (left of the ✕). A tap on the left 60 % opens the name picker, as the old tag did. A
  computer player's band is dark, with words in the seat colour (it goes with the dashed rim). The pill is dropped in
  columns too narrow for it and a name.
- **Seat colours** (`seat`): the four now differ in lightness as well as hue (yellow ≈ L 87, green ≈ 68, red ≈ 50,
  blue ≈ 41), and each has its own ink for words on the band (dark on yellow / green, white on red / blue). The
  card's tint, rim, wells and tiles follow the new colours.
- **Cursor tile**: filled in the seat colour. Every active seat whose cursor is on a tile shows a `1P`–`4P` tag at
  the tile's foot (`tags`), in its own colour, in every column.
- **Faint seat number**: a big `1P`–`4P` in the seat colour, alpha ≈ 20 %, behind the preview figure. It is sized
  to the preview box's height and width.
- **Preview animation** (`previews.rs` `buttons`): moved off R2 / L2 (now the section jumps) to the right stick
  (flick up: win, down: loss; a flick fires again only after the stick comes back), keyboard R / F (the old 1 / 2
  collided with the section key 1). R / F are ignored while a name picker is open. The legend has a new
  `Act::Preview` entry, "Win / Loss", with a right-stick glyph (`Glyph::Stick`) and keycaps R F. It shows for the
  device's own seat when it isn't on Random.
- `pointer.rs`: `taps_from_main_menu_to_match` now taps 2P's tile 4 instead of 10. The rail costs a row of tiles on
  the default 4:3 grid, so tile 10 is now below the fold.

## Checks
- New unit tests in `roster.rs`: `the_rail_scrolls_to_the_cursors_chip` (offset rules) and `band_and_rail_in_the_card`
  (band across the card's top, rail between preview and grid, at 0.4–4 aspect, 1–4 columns). The existing
  `select_fits_the_window` and legend-vs-handled tests pass.
- Shots in `context/b422/run1/`: roster2-favs and roster4-favs at 16:9, roster4-favs portrait (700×1000), roster3 at
  21:9, roster2 at 4:3. In all of them the band, rail fade / scroll, cursor tags and faint number look right.
- `tools/check.sh -p hst`: everything passes except `update::tests::newer_release_swaps_and_bad_ones_keep_the_old_game`
  (signature verification): already failing on the base commit; done.sh refuses on it, so I ran audit + full check.sh by hand (only that test fails), ticked and committed directly, and filed a task for it. That test is in `update.rs`, which this task doesn't touch. It fails on the base commit's
  rename (fbaef6ff "Ship as OpenTennis"), not on this change.

## Not verified / not 1:1
- The front-end is ours (PLAN), so there is no comparison with the original.
- The mockup canvas (claude.ai artifact) isn't reachable from here. Sizes and colours are mine, following the
  description in `plan/B422.md`.
- Near / Far: seats in the first half (singles 1P, doubles 1P–2P) are shown as Near. I took this from the select's
  team order and did not check it against the match's camera side.
- The right-stick flick wasn't tried on a real pad (no pad in an unattended run). The keyboard R / F and the
  legend were checked by build and shot only.
- The rail's horizontal scroll isn't animated (it jumps to the new offset).
- Devices show as "Controller 0" in `HST_MENU` shots (the shot seats fake pads). This predates B422.
