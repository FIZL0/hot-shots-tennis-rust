# P0 input replay — fixture reader, CLI, round1 replays (FINAL)

## Capture
- `context/fixtures/round1.bin`: 20791 samples, vsync 10018..=30808, no gaps — complete: the playback ended
  there (the .p2m2's 21666 includes non-game frames; per the human's TODO note, commit d4ecfac).
  PCSX2 is still at NominalScalar 0.25 (PCSX2.ini) — set back to 1 when convenient.
- round1 is doubles (gm player count 4): P1 human on port 0 + 3 bots.

## Layout findings
- Pad word at pad manager +0x30 is stored **active-high** (idle 0, 0x2000 Circle); the earlier "^0xffff"
  note was wrong. Then rx, ry, lx, ly bytes.
- Player position: model matrix at player +0x3d40 (rows; translation x +0x3d70, y +0x3d74, z +0x3d78),
  updated every frame (follows the stick). The hit routine copies the model's matrix there.

## Shipped
- `hst_sim::replay` (Frame accessors: vsync, pad, globals, gm, ball, player fields/position, score).
- `cargo run -p hst-sim --bin replay -- <fixture> [from] [to]` → CSV per frame, gaps on stderr.
- Tests on round1 (all pass): fixture frame-exact (`tests/replay.rs`); 29 points score/rotation replay;
  30 umpire verdicts (6 net points skipped — no live ball); 810 predictor frames / 158 line calls bit-exact.
- Fix found by the longer capture: the verdict replay must clear faults when the scoreboard shows a scored
  point (as play.rs does) — vsync 28997 was misjudged as a double fault otherwise.

## Not possible yet
- Input → port per-frame diff of player positions/contact frames: the port has no ported player movement;
  P7 builds it on `replay::Frame::{pad, player_pos}` and owns that acceptance test.
- Next capture should add: live ball `*(gm+0x88)`, rally struct 0x3165f0..0x316630, and the end of the round.
