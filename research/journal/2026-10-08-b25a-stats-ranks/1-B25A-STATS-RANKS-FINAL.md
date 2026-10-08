# B25a — stats page rank labels (FINAL)
- Gap from B25: the result's stats page drew rank row 0 ("Lv 5") for every COM player.
- Original: `context/shots/b25/orig/page1b.png` (slot 5, all bots) shows Lv 5, Lv 5, Lv 5, Lv 4 left to right —
  exactly the per-player rank ints B35 read from RAM for slot 5 (AIParam rows 84,86,85,89 → ranks 0,0,0,1), so the
  stats page uses the same per-player rank as the serve panel.
- Port: `play/match_stats.rs` takes `g.players[i].ai.rank` for COM players (humans none), as `play/panel.rs` does.
  No panel.rs change needed.
- Tests: `tools/check.sh -p hst` passes (55).

## Not verified / not 1:1
- The stats page's own draw code wasn't traced to the rank int (the header draw isn't in the result-page functions
  checked; the global is read through a base pointer the decompile doesn't name). Matched on the screenshot's labels
  instead, which equal B35's RAM-verified ints.
- No side-by-side port screenshot: getting the port to the stats page needs a whole match; the value is the same
  field B35 verified and the layout was checked in B25.
- Humans: the original's 17 (= no label) for humans on the stats page is assumed from the panel; page1b has no human.
