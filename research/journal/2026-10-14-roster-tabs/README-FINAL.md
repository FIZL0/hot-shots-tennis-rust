# B369 — roster category tabs (2026-10-14)

Spec: plan/B368.md § Tabs and overflow. Ran in parallel with B368 (per-player columns), so the tabs are built as a
self-contained module on the current shared-grid select, ready for B368's columns to draw per player.

## What landed
- `play/main_menu/tabs.rs`: the tab model (Favourites while there are any · Hot Shots with Random first · one per mod
  game, `Mod::game()` = `source` or "Mods", sorted by name, "Mods" last), each player's own tab (`Player::tab`, Hot
  Shots by default so a fresh select still opens on Random, B346).
- Overflow (`Menu::tab_move`): → past a row's last tile → next tab, ← past its first → previous, wrapping; same row
  clamped to the new tab's last, landing on its first/last open tile (B66 full tiles and tabs passed over); ↑↓ stay
  in the tab (B306's edge rule kept). A pick put off its tab (carried COM pick, random draw) is moved onto its home tab
  on the next move.
- □ / Tab toggles a favourite (not on Random, not when ready); saved to `favourites.txt` beside `settings.txt`
  (keys `hst N` / `mod <folder>`), loaded at startup; star (gold `*`) on the tile. Unfavouriting while on Favourites
  keeps the cursor on the tab (next favourite), or its home tab once Favourites is empty.
- Tab row over the tiles (shown player's tabs, counts, theirs filled in their colour) with the footer
  `« prev   n of N   next »`; tap a tab → `pointer::Act::Category`. Tile taps index the shown player's tab.
- Legend: "□ Favourite" (between Hand and Ready). The B296 □ filter / disc–custom page was already gone (B306);
  the old one-roster movement code is replaced by the tab moves.
- Tests: `tabs::tests` (order and counts, overflow wrap + row clamp, full tiles, favourites persisting, tab tap),
  updated `locked_costumes_and_match_order`, `roster_scrolls_to_the_cursor`, `triangle_shows_the_attribute_card`.

Shots (16:9, 4:3, 4 players, with 2 favourites): `context/b369/`.

## For B368's merge
`tabs::row(d, m, p, rect)` already takes the player: draw it in each column above its grid, and make
`Menu::cols(p)` (tabs.rs) return that column's tiles per row. `Menu::page()` uses `tab_cells(shown)`; per-column
scroll should use `tab_cells(p)`.

## Not verified / not 1:1
- The 4-player `--shot` with each column on a different tab: B368's columns aren't merged yet; this select shows
  one grid (the shown player's tab). Each player's tab and cursor are kept and tested, only the drawing waits.
- Glyphs: the menu font has no ◂ ▸ · ★, so the footer uses « » and the star is the font's `*`.
- With 8 tabs at 4:3 the pill labels get small/squeezed; B368's per-column row will need a scrolling tab strip or
  shorter names (mockup shows few tabs).
- "Favourites per name" waits for B371.
