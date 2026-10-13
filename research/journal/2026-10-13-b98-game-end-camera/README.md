# B98 — game-end camera (Game + Server/Receiver banner)

Shot 0x61 (the game-end close-up) looks at frames 0, 29 and 45. Two port bugs made it look away from the winner:

1. **Predicted subject spot had w = 2.** For path reactions (gu_set 0x2e, team reactions 0x30..) `predicted()`
   took `motion::reaction_root` as-is. Its w adds up the path point's w, so the head frame built on it had
   w = 2, and 0x61 offset by it looked away from the player. Now `cutaway::reaction_spot` returns the root with
   w = 1. This also fixes doubles team-reaction rally cut-aways.
2. **Frame 0 had w = 1.** The game's table frame 0 is identity rotation with a zero last row (every recorded
   sample of `cutaway2_s05.bin`). `Frames::get(0)` returned `IDENTITY`. Forcing IDENTITY in the
   replay test raises its worst error from <1e-3 to 1.99.

## Proof
- `crates/hst-sim/tests/cutaway.rs`:
  - `prediction_s05` poses every predicted subject (reaction anims from PCANI / PCCG0 `_dummy` paths, every
    costume's skeleton) at each cut-away start. It checks them against the recorded table frames 29/45/61/77:
    24 subjects match within 1e-4, including the game-ending cut-away. With w = 2 it fails.
  - `cutaway_s05` now also asserts that the port's frame 0 equals the recording each sample.
  - Its per-tick replay of every shot, game end included, stays under 1e-3.
- By eye: `research/b98_game_end_shot.py 5 context/b98/orig/game_end1.png` (original, shot 0x61, eye
  (-2.38,-0.52,0.16) fwd (0.11,-0.13,0.99)). Port (slot-5 lineup, court 10, bots), `context/b98/port/game_end1.png`:
  shot 0x61, fwd (0.09,-0.17,0.98) toward the winner, the same full-body front close-up. Before the fix, fwd was
  (-0.93,-0.34,-0.14): sideways and up.

## Stall (user note)
Port windows sometimes exit with code 0 after 10–75 s (runs 7, 8 and 12; cause not found, no panic in the log).
The wait loop now breaks when the run's exit file appears and then retries, instead of polling a dead run.

## Not verified / not 1:1
- Port and original game ends are different moments: the bots' matches diverge, so the port showed player 3
  where the original showed player 1. The views are compared by eye, not frame-matched. Bit-exact checking comes
  from the fixture replay only.
- Only one recorded game end (slot 5, doubles, receiver wins). Server wins, set ends, change of ends, singles,
  human-driven slots and other characters/costumes are not recorded. Singles needs a human-driven save.
- The port screenshot ran with HST_BG=1 (no backdrop). The "Server/Receiver" word next to "Game" was missing
  in the port shot. That's the banner, not the camera; it is not chased here.
- Hand-back after the shot: per-tick only as covered by B48's tests, not re-recorded.
