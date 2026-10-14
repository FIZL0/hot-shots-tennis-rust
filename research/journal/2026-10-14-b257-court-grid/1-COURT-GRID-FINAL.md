# B257 — confirm screen's court picker as the online pill grid

The confirm screen's Select Court no longer opens the original's tile strip (B64) along the screen's foot: it opens a
card over the seats column with the eleven courts and Random as pills, three to a row in the original strip's order
(`STRIP`), Random last — the online screens' grid (B214): `side_button` pills (the hovered one gold), the hand
cursor beside it, and `online::pill_step` (now `pub(super)`) for ←→↑↓ (nearest pill that way, no wrap, ←→ along the
row). The side card follows the hovered court live (band, surface, meters). ✕ keeps, ○ puts the old court back;
pointer/touch: a pill hovers it, the hovered pill again keeps it, a tap off the card cancels (B221's popup).
The card covers both teams' groups, so seat rows don't show under it. The legend shows Move "Court" instead of ←→.
The umpire picker is unchanged (still B64's face cards).

Code: `main_menu.rs` `court_at`, `court_rects`, `court_grid`; the Confirm picking step; tests
`court_and_umpire_pickers`, `random_court_and_umpire`, `pointer::taps_from_main_menu_to_match` updated.

Shots (`HST_MENU=courts`, via an `HST_BIN` wrapper since hyprctl exec drops env): `context/b257/courts-{43,169,219,portrait}.png`
next to `context/b257/lobby-169.png` (online lobby). Same pills/cursor at 4:3, 16:9, 21:9 and portrait.

## Not verified / not 1:1
- Not like the original's court strip on purpose: the task asks for the online grid instead (menus are ours, PLAN).
- A tap on the card's background between pills counts as "off the picker" and cancels (the popup has no
  swallow-only hit); harmless, the court goes back to the old one.
- Pre-existing, not this task: the side card's bounce/speed meters run past its right edge at 16:9/21:9.
