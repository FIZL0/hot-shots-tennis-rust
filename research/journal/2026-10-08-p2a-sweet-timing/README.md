# P2a — Sweet spots by timing, not by memory writes

Status: FINAL.

P2's aim recorder forced sweet hits (P1 +0x3fa0 = 0) and the opponents' slices (+0x3ee4 / +0x3fa0) by memory
writes, because its swings seemed to lock by themselves. The original has no auto-swing: the lock comes only from a
press. The recorder's stray presses locked early, and its straight-line timing didn't allow for the bounce. It now
presses by the game's own predicted path, and the fixtures are plain play.

## Files
- `tools/record_aim.py`: the recorder (no game writes left except the singles player count).
- `crates/hst-sim/tests/aim.rs` `human_aims` (unchanged). Fixtures `context/fixtures/aim_singles.bin`,
  `aim_doubles.bin` re-recorded.

## Journal
- [1-TIMING-FINAL.md](1-TIMING-FINAL.md): why the swings locked early, the fix and the recordings.
