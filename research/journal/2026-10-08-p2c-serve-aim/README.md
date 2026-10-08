# P2c — The serve aim (0x353000), exact

Status: FINAL.

The "second aim writer" is the serve aim: 0x353000 runs at the serve swing's countdown end (0x351dd0, player
state 1). It is ported bit for bit: `serve::target` now builds 0x353000's base and goes through the rally aim's
core (`shot::aim_from`, split out of `shot::aim`), and the strong toss's nudges follow the asm's madd order with
the per-skill-level table. The 4 dropped doubles aims were instant-replay re-aims (+0x4088), now skipped.

## Files
- `crates/hst-sim/src/shot.rs`: `aim_from` (the core from a base, width and half); `aim` calls it.
- `crates/hst-sim/src/serve.rs`: `target` exact; `miss_of` (skill level by character).
- `crates/hst-data/src/exe.rs`: `serve_miss` (the 3-row table).
- `crates/hst/src/play.rs`: `serve_character` sets each character's miss row and serve angle.
- `tools/record_aim.py`: `AIM_SERVE=1`; replays skipped.
- `crates/hst-sim/tests/aim.rs`: `human_serve_aims` (fixture `aim_serve.bin`).

## Journal
- [1-SERVE-AIM-FINAL.md](1-SERVE-AIM-FINAL.md): what 0x353000 is, the exact port, recording, the replay aims.
