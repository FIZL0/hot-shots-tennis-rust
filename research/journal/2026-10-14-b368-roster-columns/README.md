# B368 — per-player roster columns (FINAL)

Spec: `plan/B368.md` (mockup boards at 1280×720). UI only; no PCSX2 needed (the front-end is ours to design).

## What changed
- `crates/hst/src/play/main_menu/roster.rs` (new): `Screen::Chars` drawn as one column per seat in use
  (`Menu::players()`: 2 singles, 4 doubles). `columns(view, n)` lays out the top bar, 24 px margins, 12 px gaps and,
  per column, header / pills / preview + stats / tabs / grid + scroll bar / footer in mockup pixels
  (`PY = 448/720` units down, `PX = PY·15/14` across, so tiles are square on screen). Tiles are at least 84 px and
  stretch to fill the width; rows are as many as fit; height left under the last whole row goes to the preview
  (up to twice its size). The pills go on two rows when a column is narrower than 290 px inside.
- Seat colours, grade colours, mode pill, gold play style, pink stats card with outlined labels and A–F discs, all
  as the spec. The stats are always up (`grades_of` / `mod_grades`); the △ card toggle (`Player.card`) is gone.
- `main_menu.rs`: `Grid` is now just the view (+ the lobby/confirm rects); per-player `scroll[4]` / `followed[4]`,
  `page(p)`, `follow(p)`, `scroll_by(p, d)`, `columns()`. Arrows move by the column's `cols`; the moved column
  follows its cursor; a window resize re-follows every column.
- `pointer.rs`: `Act::Cell(p, i)` (only the device driving column `p`), `Act::Grid(p)`, `Act::Scroll(p, d)`;
  a touch drag or the wheel over a column scrolls that column only (for its own device).
- `legend.rs`: `button_names(dev, act, bind)` gives a device's L1/R1, Select names for the pill chips (keyboard Q/E,
  C); Chars no longer lists △.
- `previews.rs`: face icons read for every column's tiles on view (`Menu::on_view`).
- Shots: `HST_MENU=roster1..roster4` (1–4 humans, rest COM), `roster2-deep` (2P scrolled into the mods).

## Checks
- Shots in `context/b368/`: `r{1,2,3,4}-{169,43,219}.png` (1280×720, 960×720, 1680×720), `r2-deep.png`
  (2P on mod 40 of 92, its column scrolled, 1P's at the top; `--shot-at 20` so the mod faces have baked).
  No clipping, uniform scaling, at 16:9 13/6/4/3 tiles a row for 1–4 columns (tested).
- Tests: `select_fits_the_window` (1–4 columns × 7 aspects: in view, above the legend, parts in order, square tiles
  filling the width, 13/6/4/3 at 16:9), `roster_scrolls_to_the_cursor` (per-column follow, wheel scroll for its own
  device only, clamps, tap on a scrolled tile), `triangle_does_nothing_on_the_select`, `select_fits_the_pool`,
  pointer `taps_from_main_menu_to_match` / `wide_window_hits` on column cells, legend "handled vs legend".
  `tools/check.sh -p hst` passes.

## Not verified / not 1:1
- Tabs: one "All N" tab only; the category tabs are B369's.
- Footer reads "All   k of N": the menu font has no "·", "…", ◂ or ▸, so the mockup's arrows/dots are left out and
  names are cut with "...".
- Device names come from `dev_name` ("Controller N", "Keyboard", "COM"), not the mockup's "Pad 1".
- Hand icon: the menu's pointing-hand cursor art, not the mockup's hand glyph (no such art in the menu atlas).
- On a narrow column (4 players at 4:3) a long name is cut ("Glo...") to keep the device on the header.
- Online room look (names, lobby flow) is B370/B371's; this keeps the old online picking rules inside the columns.
- Real gamepad input and a touch screen weren't tried live; covered by `Menu::step` / `point` tests and the legend
  probe only.
- Mod face tiles are blank until their icons bake (the existing icon reader); early `--shot`s show empty tiles.
