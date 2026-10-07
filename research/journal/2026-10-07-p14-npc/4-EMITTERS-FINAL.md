# P14c2 ambient sound emitters — FINAL

## Where things stand
- `hst_data::exe::Game::emitter(ty)` gives a trigger type's sound (callback row +4), mean gap (param row short +0x54) and jitter (float +0x58).
- `hst_sim::npc::EMITTERS` lists the 19 types whose callback is the plain emitter (0x3f6990): 4 7 11 12 13 16 17 23 24 25 26 35 36 41 42 45 46 50 51. None of them draw RNG in the motion reset (row +0xc, +0x44 and +0x70 are all 0).
- `npc::Emitter` ports these steps:
  - the generic tick's countdown: on reaching 0 it plays program 7 / sound id with volume 0x40, positional; sweep = 240; direction = (MT >> 16) & 1;
  - the re-arm when timer < 1: `cvt.w.s(mul(itof(base), msub(1.0, jitter, mul(2⁻³², utof(r)))))` (asm 0x3f6a94: `adda.s` 0+1, then `msub.s`);
  - type 36's ±0.375 pan sweep, wrapped to [0, 359];
  - msg 4's replay save/restore and its sweep = 0.

## Result
Test `emitters_count_down_like_the_game` (crates/hst-sim/tests/npc.rs) is bit-exact on four courts (needs at least 3):
- court 10 (trig_s05): 4 emitters, 11884 ticks, 9 plays;
- court 1 (trig_s07): 2, 5998, 4;
- court 2 (trig_s10): 2, 5842, 4;
- court 4 (trig_s03): 2, 5982, 3. The truncated recording works as is.

Court 2's failure at vsync 33654 was a read artefact, not a double tick: both emitters' timers drop by 2 into that sample, and the next sample (33655) has every object byte and the MT unchanged (only one manager byte differs). The 33654 sample was read after the game ticked again. Fix: the test's still-filter now compares the object region (`wo(0)..`) instead of the whole sample.

Replay save/restore (msg 4 with 0xe) is ported straight from the decompile, but no recording exercises it (0 resets survived the filter).

## Start pan (not ported)
The start pan (+0xd4) is copied from the sound library's last bearing (`*(lib)+0xc`), written at the end of the library's positional play call from its listener solve. The listener depends on the camera mode byte (match object +0xbc → +0x51, or +0x52 for some modes) and the library's listener mode:
- mode 6 / mode 0: a table position;
- camera modes 7–10: the live camera's matrix, its eye pulled a third of the way toward the target and clamped to ±8.685 (x) / ±17.885 (z);
- mode 2: a fixed matrix plus a per-mode offset;
- otherwise the fixed listener (what `sound::place` ports).

So the start pan differs from `sound::place` while a non-default camera mode is active. This is not ported (ponytail comment on `Emitter::pan`); the test still copies the recording's pan on a play. Only type 36 uses the pan audibly. It belongs with audio (P20) if ever needed; the recordings have no camera matrix to verify it.

## App wiring
`play.rs` builds `Game::emitters` at setup from `npc::spawn` trigger kinds in `npc::EMITTERS` (court = --stage or --court; layout loaded by the new `main.rs court_layout`, also used by `load`). It ticks them in `simulate` after the gallery, pushes program 7 / key = sound id / volume 0x40 at the emitter's position into the slot-0 (court bank) sound queue, and resets their sweep at `next_point`. They draw from the app's rng, not the game's shared MT (not ported to the app). Checked live: court 10 spawns 4 emitters (45, 46 ×2) and they fire (sounds 5 and 27) within 50 s.
