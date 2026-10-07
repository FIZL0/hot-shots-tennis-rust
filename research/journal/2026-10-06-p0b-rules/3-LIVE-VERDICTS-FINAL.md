# P0b3 — live-ball verdicts (rally block per frame)

## Recording
- `tools/record_p2m2.py 5 context/fixtures/match_s05.bin 40000` (slot mode: loads the state itself, arms at its
  vsync 7522). Sample = round1 layout + live ball `*(gm+0x88)` 0x290 + rally block 0x3165f0 0x50
  (`replay::frames_live`, `Frame::{live_ball, rally, rally_u8}`). PCSX2 at NominalScalar 0.25.

## Findings
- Point-over check `0x325be0` runs in phase gm+0x55 = 3 while gm+0x56 != 4, **before the ball steps**: the
  decision frame sees the ball as it was at the end of the previous frame (decision at vsync 8501 on the second
  contact recorded at 8500).
- Hit check `0x325c60` (message 0x15) only in phase 3, sub != 4; clears 0x316619 (+0x29, an "illegal" marker
  not read by the judge — not modelled), then `0x1a92c0`, then +8 = hitter.
- Rally block +0x10 = the previous hitter (written by the hit routine); not modelled (judge doesn't read it).
- `0x327d30` clears let / +0x39 / +0x20 and faults unless second serve or let — matches `Rally::next_point`.

## Test
`crates/hst-sim/tests/score.rs::match_s05_rally_block`: every frame, hit check + point-over check on the frames
the game runs them, rally block compared field by field after each frame; resets (new_point, next_point, faults
cleared on a scored point) applied where the recording shows them (their timing = P0b4).
- The body hit (0x42305c) is read the same way: from the previous frame (body hit recorded at 23761, decided 23762).
- **Instant replay** (vsync 33630): the point-start save 0x316640.. (and score 0x4231xx) is copied back and the
  point is re-run at its own pace — gm+0x50 is the sim tick counter: ~24 ticks/vsync fast-forward, then slow
  motion, then 1/vsync. The test resyncs through the replay (it re-decides the same point at 33818).
- Match over: team 1 won the set 4–0; phase 5 at 33916; the match object is replaced right after (vsync 33922),
  which hung the recorder's stable-read loop — it now exits with "match object gone".

## Result
26400 frames (vsync 7522–33921, no missing frame), **36 decisions** — 26 points, 8 faults, 2 out after the net
(call 5) — all at the recorded frame with the recorded winner, rally block equal on every frame; 54 resets,
1 instant replay. Not in this match: lets, double faults, illegal hits, rally outs (call 1) — need another
recording (lets: P6).
