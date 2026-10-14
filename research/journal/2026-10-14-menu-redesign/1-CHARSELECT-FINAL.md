# B296 — character select redesign (FINAL)

Built after `mockups/CharSelect.dc.html` in `play/main_menu.rs` (`chars`, `Grid`, `Menu::roster`/`filters`/`set_filter`):

- **Seat strip** across the top: one card per player (face, "NP · device / COM · rank / online name", name, style and
  costume or Waiting / Ready to go! / Not selectable), teams split by a gold "VS" (1 v 1 or 2 v 2); the B238 ✕ on
  device seats. A tap on a card readies it (B221), as before. Cards cap at 240 wide and centre in a very wide window.
- **One roster grid**: Random first, the 14 disc characters, then every mod. Filter pills: All N / Hot Shots Tennis 14 /
  Mods N / one "From <game> n" per mod source (only the HST pill without mods). □ / Tab cycles them, a tap picks one.
  Arrows walk the roster index (±1, ±columns), skipping full cells (B66 costume locks via `full`/`skip_taken`);
  the page scrolls a row at a time ("Rows a–b of N", the bar's halves step up/down). Player tags and hand cursors
  on cells as before.
- **Detail panel** for the player who pressed last: 3D figure (`Tex::Preview`) or the "?" tile for Random, hand pill
  (tap toggles), name, style · source, five stat bars with F–A letters (the △ grades), costume chip (halves =
  prev / next). Landscape: a 200-wide column on the right; portrait: under the roster, the figure taking spare height.
- Layout from `Grid::fit(aspect)`: columns from a 54-unit minimum cell, rows from a 60-unit pitch, at most 64 cells
  a page (≤16 columns) so a page fits the quad pool (raised 1400 → 3200; `select_fits_the_pool` checks the worst case
  at aspects 4…0.4 with 4 players and 100 mods).
- Rules kept: B66 costume locks, Random (drawn on ready), COM picks (`coms` shot), B238 open seats, online editing of
  a bot (filter reset to All).

Shots (`context/b296/`): `chars-1280x720`, `mods-1280x720`, `coms-960x720-b` (4:3), `chars-1680x720-b` (21:9),
`chars4-540x960-b` (portrait, four humans). Tests: `select_fits_the_window`, `select_fits_the_pool`,
`roster_filters`, updated cursor/pointer tests; `tools/check.sh -p hst` 285 passed.

## Not verified / not 1:1

- Not compared with the original: the front-end is ours (PLAN 2026-10-12); compared with the mockup by eye only.
- The △ attribute-card toggle is gone: the stats are always in the detail panel, drawn as our own bars and letters,
  not the disc's ATTRS art.
- The mockup's L2/R2 filter hint became □ / Tab (L2/R2 were free in the mockup, but □ was already the page key).
- The character description stays in the bottom ticker, not inside the detail panel (panel too small at 4:3).
- One detail panel shows the last player to press, not one per human.
- Cells are square boxes, not rounded (quad pool budget); small faces drop the white ring.
- Mod face icons are blank until baked (they bake in the background, as before; the shots were taken early).
- Very wide windows: the seat cards stop at 240 units and centre, so the strip is narrower than the roster.
- Portrait seat cards clip "Keyboard" under the ✕ at 540 px wide.
- Touch/mouse paging moves a row per tap on the scroll bar (no drag).
