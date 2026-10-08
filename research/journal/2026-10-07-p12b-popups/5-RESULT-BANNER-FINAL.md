# P12b5 — "Game / Set" + "Server / Receiver" banner over the result board

Ported to `crates/hst/src/play/popups.rs` (`Outcome`, `ResultBanner`, `tick_result`); `hst_sim::flow::PostPoint::waited`
exposes the scoreboard's wait counter (+0x184).

## What the original does

- Start `3899a0` from the scoreboard update `382810` when the wait counter reaches 45 (0x410ea0; the game/set wait
  is 90, 0x410e98), only when the result kind 0x4230b8 > 0 (1 game, 2 set; the match's last set never sets it):
  flag +0x571 = 1, t +0x574 = 14, stage +0x578 = 0. The update `389cb0` runs the same tick (via `383fa0`), so the
  first frame drawn has t = 13.
- Update `389cb0`: stage 0 t−1, below 0 → stage 1, t = game_rise + game_drop (0x410eb8 + 0x410ec0 = 16); stage 1
  t−1, below 0 → stage 2, t = 0 (held). Cleared by messages 0xd / 0x18 / 0xe (next point).
- Draw `3899d0` (from `382e80` after the result board): sprites from base +0x84 of the INPANE table at 0x4159f0:
  +0xf0 result_game (kind 1) / +0xf4 result_set, +0xf8 result_verBlue, +0xfc result_verRed (winner 0x4230a8 == 0).
  recv = (server 0x42304c's team) ≠ winner. Word 1 (0,0,144,64) at (124 recv / 148, 32) at full alpha; stage 0 its
  white copy (0,64) on top at t·128/14. Stage ≥ 1: word 2 (0, recv·64, 256, 64) at (276 / 300, 32), its white copy
  (0, (recv+2)·64) at max(t, 0x411130 = 0)·128/16. Flushed at 0x1b..0x1e, after the board's sheets: on top.
- Sheets: result_game/set 256×128 (orange "Game" / red "Set", white below), verRed/verBlue 256×256 (Server,
  Receiver, then white copies).

## Port

Starts when the flow's wait is at 45 for a Game / Set event and the match isn't over; stepped after each match tick,
dropped with the post-point phase. Quads appended to the result board's pool (drawn over it). Unit test
`result_banner` pins positions, UVs and the flash alphas. Forced-banner screenshot of ours:
`context/shots_p12b5/ours/dbg.png`.

## Verified

`research/result_banner_shots.py 5 30000 context/shots_p12b5 5 5` (slot 5 doubles, games poked to 5-5; polls
+0x571/+0x574/+0x578, F8 at stage 0 t 13/7/0, stage 1 t 16/8, held; stops at the first held banner): game 5-6,
server player 0, winner team 1 → "Game" at ~124 flashing white to orange over 14, then blue "Receiver" at ~276 with
its white copy fading over 16 (`context/shots_p12b5/orig/grid.png`). The log pairs t = 13 with counter 46, but the
counter is read a few PINE calls after t and races the next tick (a constant +1 over all six shots); the decompile
increments the counter, starts the banner and steps it in the one update call, so the port starts at 45 with t = 13
on the first frame. Not checked: a set banner, a server win, the red sheet (decompile only).
