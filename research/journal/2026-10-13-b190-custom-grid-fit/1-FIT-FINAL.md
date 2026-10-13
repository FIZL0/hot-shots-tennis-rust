# B190 — custom character grid fits the window

## What changed (`play/main_menu.rs`)
- `Grid` replaces the fixed `ICON_COLS`/`ICON_ROWS`/`ICON_X`: `Grid::fit(aspect, widescreen)` gives the columns
  that fit between the side-pinned player cards (frame from 40 past the left cards — room for a hand — to 13 short
  of the right ones, centred in what's left) and the rows the frame (y 86..384, over the page count) holds.
- A tall window (narrower than 4:3) stretches the 448 space vertically; the grid is drawn at its 4:3 shape and its
  quads squashed by that stretch (`Grid::tall`), so faces stay square and more rows fit.
- `fit_grid` (Update, before `step`) refits `Menu::grid` every frame from the primary window: live on resize and on
  the widescreen setting. ←→/↑↓ step by the live column count; paging = cols × rows a page.
- At 4:3 the grid is B133's 4×4 (frame 200..467, same centres ±1 px).

## Checked
- Unit test `custom_grid_fits_the_window`: 4:3 → 4×4, 16:9 → 7×4, 21:9 → 11×4 (4×4 with widescreen off),
  9:16 → 4×9; frame clear of the cards and above the page count from 0.4 to 4.0 aspect.
- `--shot`s with 40 mods (`HST_MENU=mods`, doubles, context/b190/): 43.png 4×4 (1/3 pages), 169.png 7×4 (1/2),
  219.png 11×4 (all 40 on one page), portrait.png 450×800 4×9 (1/2), faces square, frame between the cards.

## Not verified / not 1:1
- The custom page is remaster-only (no original to match); layout by eye.
- Singles leaves free room under the side cards (below y 224) that the grid doesn't use: it keeps clear of the card
  columns at every height, for one rule in singles and doubles.
- Some cells in the shots are blank: icons stream in 2 a frame (B133), not a layout issue.
- Resize was checked as separate window sizes, not by dragging one window (the fit runs every frame either way).
