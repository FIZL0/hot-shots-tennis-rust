# B232 — UI no longer stretches with the window

## What stretched

At 4:3 or wider every HUD/menu rect was already uniform: across is `x / 6.4 · share` (share = (4/3)/aspect), down is
`y / 4.48` of the window height, which is the same scale. Narrower than 4:3 (a tall window, a phone upright) `share`
is 1 and down was still % of the window height, so everything — score panel, pop-ups, result pages, menus, the
Screen43 roots — stretched vertically. The custom-character grid compensated with its own squash (`Grid::tall`).

## Fix

`widescreen.rs`: `vshare(aspect)` (share of the window's height the 4:3 screen covers) and `vplace(y, h, aspect,
anchor)`, the vertical twin of `place`: `Centre` rides the centred 4:3 screen (letterboxed top and bottom), `Sides`
and `Span` keep to the window's top or bottom edge by which half the rect's centre is in, a full-height rect (wall,
dim) fills the window. `fit` now also sets the Screen43 roots' top/height. Every `y / 4.48` site goes through
`vplace` (panel, speed readout, pause menu, main menus + ticker bar, previews, intro plate/banner); hit-testing
(`pointer::hit_at`, `pause::screen_point`) and the web file input's rect (`mods_screen::add_rect`) follow it.
Tiled walls and preview render targets size from `unit_px` (the 4:3 screen's height / 448) instead of the window
height. `Grid::tall` is gone: a tall window keeps the 4×4 grid (it no longer gains rows).

At 4:3 and wider nothing changes (vshare = 1).

## Checked

`--shot` (context/b232/run1): main menu and character select at 960×720, 1280×720, 1680×720 (21:9), 540×960; a
match HUD at 960×720, 1680×720, 540×960. Tall: menu content centred, title bar top, ticker bottom, player cards in
the corners, score panel halves in the top-right / bottom-left corners, all at the 4:3 shape. Test
`widescreen::tall_window_letterboxes`; `tools/check.sh -p hst` passes.

## Not verified / not 1:1

- Loading cover in a tall window: not shot (no `--shot` hook into the menu → match load). Its 4:3 node is
  `height 100%, aspect 4/3, max_width 100%`, which taffy keeps 4:3 by carrying the max width over to the height; left
  as is.
- Netplay's stall/end text sizes by window height (`FontSize::Vh`): text keeps its shape, only grows in a tall
  window. Left.
- Horizontal `Span` pieces (bars' last pieces reaching the right edge) stretch across by design (solid/gradient bars,
  as before B232).
- Pause menu and result pages in a tall window not shot: they ride Screen43 / the same `vplace` path as the shot menus.
