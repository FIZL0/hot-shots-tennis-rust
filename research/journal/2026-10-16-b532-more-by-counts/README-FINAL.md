# B532: More by <user> keeps every kind tab's count

**Cause.** `View::narrow` (mods_screen.rs) built the author's list from the store filtered to the *current kind*
as well as the author. `kind_counts` counts Browse's tabs from `entries()`, which is that list while narrowed, so
every other kind came out as 0.

**Fix.**
- `narrow` keeps the author's mods of every kind. The tiles already filter to the chosen kind (`grid::keeps`), so
  what you see doesn't change, and each kind tab now shows that author's count for its kind.
- A kind switch (`mods_kind`) no longer drops the author list. L2/R2 move between that author's characters,
  courts and so on; ○ goes back to the top of the whole list for that kind (`from` = 0, because the old row
  belonged to another kind's list).
- The author's Download all still installs only the chosen kind's mods, the same as before. It now filters
  `b.list` by kind itself.
- The "N mods by X." status after △ now counts the author's mods of every kind.

**Test.** `kind_tabs_filter_every_page` (extended): the tab counts while narrowed equal the author's per-kind
counts, not zeros; a court by another uploader is left out; switching kind keeps the author; Download all
queues only the characters.

## Not verified / not 1:1
- No comparison with the original: the mods browser is our own front end and doesn't exist in the original game.
- No screenshot taken. The change only affects the numbers on the tabs, and the test checks those.
