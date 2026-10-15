# 2 — built (FINAL)

The concept board was approved (user 2026-10-15). It keeps L1/R1 on the costume and puts the section jumps on L2/R2
(keyboard 1/3, the same keys as the Mods pages' kind tabs). The TODO line still says L1/R1, but that wording comes
from before the board.

## What changed
- `tabs.rs` now holds the roster's sections instead of B369's tabs. In order: ★ Favorites (only while there are
  any), Hot Shots Tennis (Random, the disc cast, then any mod whose `source` game is "Hot Shots Tennis", so the name
  never shows up twice), then each mod game by name, with "Mods" (untagged) last. Each section starts on a new row
  under its heading. `Player::tab` stores the section the cursor is in, because a favorite appears both in
  Favorites and in its own section. `on_tabs`, the tab row, `Act::Category` and the `rosterN-tabs` samples are gone.
- Moves: ←→ go through every tile in order across rows and sections and stop at the ends. ↑↓ move one row and keep
  the column (or take the row's last tile if the row is shorter), skipping headings; past the grid's ends they land
  on its first or last tile (as B306 did). L2 goes to the first tile of the current section, and from there to the
  previous section; R2 goes to the next section. Both wrap. Full tiles are skipped (B66). A tap on a heading
  (`Act::Section`) jumps to that section; the wheel and drags scroll by lines.
- Scrolling (`main_menu.rs` follow/scroll_by, `roster.rs` View/fit/last_top/laid) counts in lines (a heading or a
  row of tiles). The view scrolls as little as possible to keep the cursor's row on screen, plus its section's
  heading when the cursor is on the section's first row. Toggling a favorite grows or shrinks Favorites above every
  column; each column's view shifts by the same number of lines, so it keeps its place.
- `roster.rs`: the tab row is gone, and the grid has room for one heading plus its rows of tiles. A heading is the
  name (with a gold `*` before Favorites), a count pill, and a divider line. When the cursor's character also
  appears in another section, that tile is filled but has no ring. The scroll bar is measured in lines. The footer
  reads `Section   n of N`.
- Legend: `1 3 / L2 R2  Section` when there is more than one section.
- `HST_MENU=roster2-favs` / `roster4-favs`: four favorites, with 1P in Favorites and 2P on the last section.

## Checks
Unit tests (`tabs.rs` + `main_menu` + `pointer`) cover the section order and counts, mods from the game's own
source, moves across rows and sections, the ends, L2/R2 with wrapping, full tiles, favorites persisting and leaving
Favorites, follow and per-column views, heading taps, the select fitting at 0.4–4 aspect, and the legend matching
the handled presses.
Shots (`context/b382/run2/`): roster1, roster2-favs and roster4-favs at 1280×720, 700×1000 (tall) and 1600×560
(wide). The layout is right in every case: tiles stay square and headings are never squeezed.

## Not verified / not 1:1
- The front-end is ours (PLAN), so there is no comparison with the original.
- I could only check against the board's description in the TODO and the earlier journal, not the board itself
  (artifact 6kpFeK6ya6YUJ66BN9Bjzk is not reachable from here). Colours and sizes follow the B368 mockup's palette.
- Only whole lines are drawn. On some sizes a heading is the last line on view with its tiles below the fold. I
  left it that way because it shows there is more below.
