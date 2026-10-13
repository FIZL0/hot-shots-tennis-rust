# B158 — court easter eggs shown only when the original shows them

## Cause
The port already had every trigger creature's show/hide rule (P14c/B24, `hst_sim::npc::Trigger`): court 6's
bear (type 22) is on only when its reset (message 4, a new point) sees a tiebreak in the deciding set. But
`play/npcs.rs` sets a creature's `Visibility` only in `step`, and the court intro (B42) holds the match still, so
`step` doesn't run: every creature kept the visible state it spawned with, and the bear stood by the net post for
the whole flyover (`context/b158/port_c6_i5.png`). Fix: at setup, a creature whose spawn reset leaves it off
(`!(active && on && speed != 0)`, the same rule `step` draws by) starts hidden (`port_c6_i5_fix.png`: no bear).

## The eggs (decompile: trigger callbacks, P14 journal 3-TRIGGERS-PART)
Conditional creatures and their triggers; none draws from the sim stream (all from the court/creature MT, the
app's `rng.court`):
- court 1, type 2 (landing-spot creature): appears at the ball's landing spot after a game/set win (message 6).
- court 2, type 6: flyover after a game/set win (message 7).
- court 4, type 15 (passing ball): a 50 % roll at a change of ends picks one of 4 routes, once per match.
- court 5/8, types 19/33: a 50 % roll at serve placement arms a sound for the next point (sound only).
- court 6, type 22 (the bear, `trg_BA-01`): on at a new point iff the tiebreak flag is set and sets played + 1 ==
  2·sets − 1 (deciding set); the gallery manager pauses while it's on; head follows the ball.
- court 9, type 40: cheer when a team with no human wins a game/set in ≤ 3 shots (sound only).
- court 10, type 44 (crowd): same deciding-set tiebreak rule as 22, then random idle/call timers.
Among the creature callbacks only 22 and 44 read the tiebreak flag (scan of every function reading it).
Everything else on the courts (startled creatures, emitters, ball-hit obstacles) is ambient, not an egg.

## Checked against the original (PCSX2 copy 1)
`research/b158_bear.py`: from save slot 2 (main menu on these copies) a singles match (Ashley vs COM) with the
court index poked to 6, first serve saved to scratch slot 9; `… tb` loads it, pokes a 1-set match + tiebreak flag
and serves out points. `context/b158/run3_sheet.png`: no bear in the flyover frame or in play before the poke;
`run4_sheet.png`: no bear in the poked point, then the bear by the net post from the next point on (tiebreak
count 1, 2, 3 on the panel). Remaster: `--play --stage 6` shows no bear in play (`port_c6_t3.png`) and now none in
the intro. Its in-play show: with the deciding-tiebreak test forced on (a local, uncommitted edit of `npcs.rs`)
the bear stands by the net post from the first point's reset (`port_forced_25.png`), as in the original.

## Not verified / not 1:1
- No unforced remaster shot in a real tiebreak: `--sets 1 --games 1` bot matches (`HST_AUTOPLAY=1`) ended 2-0
  in all six tries, and there's no score-start hook; the deciding test itself (`npcs.rs`, from the decompile) is
  unchanged by this task.
- `tools/check.sh -p hst`: `net::tests::lan_lobby_by_mdns` failed by seeing two LAN lobbies (another parallel
  slot's); unrelated to this change.
- The original's intro flyover was caught in one frame only (the bear's net post not in view); that it is hidden
  there follows from the draw rule (the port's spawn reset is the original's, P14), not from a screenshot.
- Court 10's crowd (44) and the other conditional creatures weren't re-shot: the change is the same rule applied
  from spawn, and their in-play show/hide was checked in P14/B24.
