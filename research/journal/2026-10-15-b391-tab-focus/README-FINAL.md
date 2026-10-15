# B391 — character select: focusable, scrolling category tabs (2026-10-15)

**Focus.** `Player::on_tabs` puts a player's cursor on their column's tab row.
- Keys / pad: ↑ from the grid's top row goes up onto the row, but only when there is more than one tab. On the row,
  ←→ switch straight to the previous / next tab. They wrap and pass over any tab with nothing open (`set_tab`, which
  keeps the tile when it's in the new tab and otherwise takes the tab's first open one). ↓ or ✕ go back down onto the
  tile; ✕ there doesn't ready. ↑ does nothing.
- Pointer: hovering a tab focuses it and switches to it, the same as hovering a tile moves onto it. Hovering a tile
  takes the cursor back to the grid.
- Drawing: the focused tab has a gold halo and ring, and the picked tile loses its ring (it keeps its fill), so only
  one spot shows the cursor. While on the row the legend reads Move "Category" and ✕ "Characters" (legend.rs). This
  keeps `legend_lists_what_each_screen_handles` in step.

**Scrolling.** `tabs::row` now draws every pill at its natural width; tabs are never squeezed or stretched. If the
row is too narrow, `shown` picks a run of whole tabs centred on the player's own tab (growing one tab each way in
turn while they fit). It keeps room at both ends for a « / », shown on a cut end; a tap or hover there goes to the
tab beyond it. A single tab wider than the row is squeezed into it. The run is worked out from the player's tab each
frame, so no scroll state is stored.

**Check.** New tests: `the_tab_row_takes_the_cursor` (keys, ✕, pointer) and `the_row_scrolls_instead_of_squeezing`.
All 114 main_menu tests pass. I also took shots with the new `HST_MENU=roster2-tabs` / `roster4-tabs` at 1280×720 and
900×900 (`context/b391/run1`). On 4 columns each shows one whole tab plus « »; on 2 columns, three or two tabs.

cursor.rs (on the task line) needed no change: the select has no 3D hand on its grid, and the tab focus follows
the grid's highlight style.

## Not verified / not 1:1
- Compared to the original: nothing to compare. The original has no category tabs; the front-end is ours to design (PLAN.md).
- Hovering a tab switches category at once (the request: "focusing one puts you on that category"). If you sweep the
  mouse across the row it switches tabs as it passes. This was checked in tests only, not by hand with a mouse.
