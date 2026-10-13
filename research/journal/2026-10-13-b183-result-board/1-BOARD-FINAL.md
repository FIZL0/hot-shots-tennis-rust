# B183 — the match-result board was empty

**Cause.** `play.rs` (`Next::MatchOver`) calls `match_stats::match_over(g)` and then resets `g.score = Score::new()`
straight away, for the next match. The result page's board (`popups::final_board`) read `g.score` live, so it always
drew a fresh score: 0 sets each, `set` 0 (so no set columns, only the dashes), no games.

**Fix.** `hst_sim::stats::Table` (the finished match's table, already kept past the reset) now carries the match's
`Score` too (`#[serde(default)]`, so old snapshots still load). `match_over` fills it in before the reset, and
`final_board` takes that score rather than `g.score`. The board's layout code (already ported earlier) is unchanged, so
it still covers any aspect ratio like before. A tiebreak set already shows as 7-6 through `set_games`.

**Check.** `popups::tests::final_board_played` plays 6-2 3-6 6-4 through `Score::point` and checks the final board's
games per set column and the sets won (2-1). In the port, a bot singles match (`--games 1 --sets 1`) that finished
on its own: the result page shows "1 Set" / "0 Set" and the set as 2-0, the loser's 0 at half alpha
(`context/b183/run3/a280.png`). Before the fix the same page showed 0 / 0 and only dashes (`context/b183/run2/a195.png`
was taken with `HST_STATS`, which ends the match mid-set, so it still shows no columns; that is expected for that
debug path).

## Not verified / not 1:1

- No new PCSX2 screenshot of the original's finished-match board for this task: the board's layout was ported
  before, and this fix only changes where its numbers come from. A full match on PCSX2 takes longer than one
  20-minute tool run.
- A finished tiebreak set and a best-of-5 board weren't seen in the port, only in the unit test's logic (no fixture
  reaches a tiebreak; see *Needs the human*).
- `HST_STATS=<s>` (the debug early end) still ends the match mid-set, so its board shows no set columns. That is the
  debug path working as written, not a bug.
