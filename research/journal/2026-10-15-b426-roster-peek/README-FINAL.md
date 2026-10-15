# B426 — roster mod tiles missing under a section heading (phone portrait)

**Cause: the lines on view, not the previews.** `Columns::fit` shows only whole lines, and the grid's height is
`rows` tile rows plus one heading. If a section break falls so the next section's heading is the last whole line,
there's room left for most of a row under it (e.g. 9:13 at Menu Size 1.2 with two players: 3 a row, 6 rows,
Hot Shots Tennis' 16 = heading + 5 rows + the next heading, ~38 of a 56-unit tile left). That row didn't fit whole,
so nothing was drawn under the heading. Mod faces draw normally once they're on view (the shot below shows them).
A brute-force scan over portrait aspects 0.40–0.98 × Menu Size 0.6–2.0 × 1/2/4 players found 42 such layouts.

**Fix** (`roster.rs`): `Columns::laid` adds the next tile row's top whenever at least `PEEK` (12 px) is left under
the last whole line. Its boxes are cut at the grid's foot. `column` draws each tile whole, then `cut` trims its quads
(fill, rim, face, star) at the foot: the picture's source is cropped to match and the bottom corners are made square.
The hit is the cut box, so a tap there picks the tile. Then the cursor's follow scrolls it into view. `fit`,
`last_top`, the scroll bar and keyboard/pad movement are unchanged. The same peek also shows under a tiles row,
whenever a scrolled grid has the room, so it's always clear there's more below.

Test: `tabs::tests::a_heading_at_the_foot_shows_its_rows_top` (the 9:13 / 1.2 / two-player layout: the heading is
the last whole line, the Golf section's first row is laid and drawn cut exactly at the grid's foot, and its hit is
there). `select_fits_the_window` still holds (every laid box inside the grid).

Shots: `context/b426/run1/chars2-700x1030.png`, `chars2-540x1170.png` (`HST_SIZE`, `HST_MENU=chars2`): the second
Golf row is cut cleanly at the grid's foot in all four columns.

## Not verified / not 1:1
- Front-end only (ours to design, PLAN.md); there's no original to compare it to.
- Not checked on a real phone; checked with portrait desktop windows (shots above) and the layout test.
