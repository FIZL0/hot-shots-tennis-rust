# B423 — roster more-below peek

Concept: `plan/B422.md` item 2. Front-end, ours to design (no 1:1 proof).

## What changed
- `roster.rs` `peek` (called from `column` after the tiles): when a column's grid has lines past its view, its foot
  fades into the card's tint (10 strips, 30 px) and a pill sits centred over the fade: a down triangle and
  `84 more: Hot Shots Golf Fore!, +3`. The count is every tile not wholly on view (B426's cut row counts). The names
  are the sections whose headings are still below. They are not the section going on, so "11 more" alone means "the
  rest of this one". As many names fit as the pill's room allows (at most ¾ of the grid's width), then `+n`. If none
  fit, it shows just `84 more`.
- Scrolled down: an up-triangle pill `▲ Favorites, Hot Shots Tennis, +2` at the top right, which keeps it clear of a
  heading's name on the left. The top edge fades only when it cuts tiles; a heading on top is a clean edge and stays
  undimmed.
- The font has no ▲/▼ or `·`, so the triangle is drawn as thin solid rows (`triangle`) and the separator is `:`.
- `off_view` (pure, `pub(super)`) works out the sections above, the count below and the sections below.
- `pointer.rs`: new `Act::Page(p, ±1)`. A tap (not a hover) scrolls that column's grid by the lines on view, so the
  first line not wholly shown comes to the top. Only the column's own device can do it, like `Act::Scroll`. The wheel and drags
  over the pill scroll the column, the same as over its tiles (`column` maps `Page` too).

## Checks
- `tabs::tests::the_more_pills_page_the_grid`: count + shown = all tiles, last section named below, only a down pill
  at the top, another device and a hover don't page, a tap pages to `end`, an up pill then names Favorites, paging to
  the end leaves only the up pill, and paging back returns to 0.
- Shots `context/b423/run1/`: roster2-favs / roster4-favs / roster2-deep at 1280×720, roster4-favs portrait
  700×1000, roster3 at 1680×720 (21:9), roster1 at 960×720 (4:3).

## Not verified / not 1:1
- Front-end only (ours to design, PLAN.md). There's no original to compare it to.
- The mockup canvas isn't reachable from here. The pill's sizes, colours and placement (bottom centred, top right)
  are mine.
- The pills weren't tried by touch on a real phone or with a mouse click in a live window. Taps are covered by the
  unit test through `Menu::point`, and the hit rects come from the drawn list.
- The top pill can cover a seat tag (`1P`) on a top-row tile's corner, as seen in roster4-favs portrait (4P's column).
  It doesn't cover the tile's own cursor fill. I left it.
- When no row is cut at the foot (less than B426's 12 px left), the pill sits over the last whole row's bottom edge.
