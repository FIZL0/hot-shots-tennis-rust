# B508 — stats screen: clipped units, heavy decimal point

The layout (`match_stats.rs`) was already the original's: the decompiled page draw cuts "%" from
`result_status01` at (72,24,24,24), "pts" at (96,24,24,24), the point at (160,0,8,24), at the same screen x as ours.
The bug was in our 4× rebuild of that sheet (`menu_art.rs`, B136's `STATS` labels):

- "pts"'s label box started at x 92 (the disc ink is 97..114), so our "p" was set inside the "%" cell: it showed as
  a sliver after every "%" and was missing from "pts". "%" box nudged to its ink too (76,29,17,16); "pts" (97,29,18,17).
- The "." box is 7 px tall; `Style::sample` treats anything under 10 px as having no outline (the hand tags), so
  the font's full stop was stretched over the whole box in white: a solid square. The point is now drawn as the
  disc's is: a dark round ring with a small white middle (3 px in a 7-px box), colours sampled from the disc; its
  box moved 1 px left so the 8-px cell holds the whole ring.

Test `menu_art::tests::stats_units_fit_their_cells`: each unit's box inside its cell, the rebuilt point is a small
white dot in a dark ring with clear corners.

Checked: port shot at 930×1029 (a portrait window, doubles) `context/b508/run3/a.png` (before: `run1/a.png`)
against the original's stats page `context/shots/b25/orig/page1b.png` (B25, PCSX2): small round point, whole "%"
and "pts". Layout is unchanged, so other aspect ratios behave as before (widescreen::put on the 640×448 frame).

## Not verified / not 1:1
- No fresh PCSX2 screenshot: reaching the stats page needs a whole match; compared against the earlier capture of
  the same screen (B25's page1b.png) instead.
- Glyphs are our font (B136), so the units' shapes and the gap between number and "pts" differ a little from the
  disc's; positions are the original's.
- Singles not shot (same sheet and cells as doubles).
