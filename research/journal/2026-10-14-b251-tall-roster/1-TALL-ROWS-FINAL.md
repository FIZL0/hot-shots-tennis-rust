# B251 — custom roster fills tall windows

A tall window (aspect < 4:3) letterboxes the 4:3 screen top and bottom (B232), which left the custom characters
grid at 4×4 with empty space under it. The extra space in a tall window is height, not width (its width already
is the 4:3 screen's), so the grid now grows *rows*: `Grid::fit` puts the frame's bottom (`Grid::foot`) at 384 plus
the letterbox below the 4:3 screen (`(448 / vshare − 448) / 2`), i.e. down to where the ticker, pinned to the
window's bottom, begins. The page count and its hit rect follow `foot`. Wide windows keep B190's extra columns.

1:1 → 4×5, 9:16 → 4×8; 4:3 and wider unchanged (4×4 / 7×4 / 11×4). Hit-testing (`pointer::hit_at`) is linear
in `vplace`, so cells below the 4:3 screen take clicks.

Test: `custom_grid_fits_the_window` (rows per aspect; frame over the page count, page count clear of the ticker).

## Not verified / not 1:1
- No screenshot of a tall window's custom page: `--shot` can't open the character select's custom page without
  driving input, so it's checked by the layout test only. Front-end, so no original to match (PLAN.md).
- The grid only grows downward; the letterbox above the 4:3 screen (under the mode bar) stays empty, keeping the
  frame's top under the Singles/Doubles bar.
