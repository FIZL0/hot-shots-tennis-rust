# P14c3 proximity-startled creatures — FINAL

## Done
- `hst_sim::npc::Near` holds the positions the creatures look at (players by index, then the ball) and the per-type startled flags. `Trigger::step_near` and `reset_near` run the generic tick or reset, then the type callback (msg 2 or 4). The old `step` and `reset` call these with an empty `Near`.
- Distance is `dist2` then `ps2::sqrt` against the creature's world position.
- **Types 0, 1, 5, 31, 32, 39** (reach 2.0):
  - Startle needs: not latched, the type's flag clear, and anything within 2.0. It then:
    - sets the latch;
    - sets moving (except 31);
    - sets the sound timer to 1 (types 1, 31);
    - turns the creature on and animating, and restarts the animation;
    - sets the flag.
  - While latched, the animation holds at the type's frame once it reaches its end: 28 for type 0, 36 for type 31, 32 for the others, clamped to the length. Type 31 starts moving at frame 35.
  - Msg 4/0: on = (type 32), not animating or moving, back home, frame 0, flag cleared.
- **Types 27–29** (reach 4.0):
  - Within 4.0, the latch is set and the animation plays once.
  - The ball's ±0.2 box overlapping the creature's sets the latch and `struck`, which is the original's message 0x14 to the match.
  - Msg 4: on, home, still.
- **Type 48**, a scrub pair (state, frame):
  - Something within 1.0 turns it forward (or backward when the frame ≠ 0).
  - Each tick it sets the frame and steps by ±1.
  - At the end, or at 0, with nothing near, it stops.
  - Msg 4 keeps the frame.
- Play (`play/npcs.rs`) passes the players' and ball's positions each tick and keeps the flags in `Npcs`.

## Recording
- `tools/record_npc.py … prox` has these extras per sample: ball, players, the 64 type flags and the heap pair at +0xb8.
- `HST_POKE=at:k:who:dx:dz;…` moves creature k to a player's position (or the ball's, `b`) plus (dx, 0, dz) at sample `at`. This was used because driving a player there by pad (`HST_PAD`, vpad commands sent at load) faulted the serve. A poke on a standing creature takes effect before that tick, so the test starts that tick from the poked position.
- Fixtures (in `context/fixtures`, not in git):
  - `prox_c01.bin`: court 1, save slot 7, types 0 and 1, 1500 samples.
  - `prox_c02.bin`: court 2, slot 10, type 5 ×6, 1200 samples.
  - `prox_c11.bin`: court 11, slot 8, type 48 ×2, 1200 samples.
- Court menu moves from slot 9: R2 → 1, R4 → 6, L2 → 10, L4 → 9, L1 → 3, R1 → 5, L3 → 11, L5 → 8, L6 → 2. Court 7 was not found.

## Test
`startled_creatures_match_the_game` is bit-exact per tick, including the Mt draws (it tries offsets and 1/0/2 steps as in the engine test):
- court 1: 1944 ticks, 3 startles, 66 ticks held back by a set flag
- court 2: 6729 ticks, 1 startle, 518 held back
- court 11: 2384 ticks, 6 scrub turns

## Open
- **Types 27–29 (court 7) are unrecorded.** Their callback, including the ball box and message 0x14, is ported from code only. The court's menu move was not found.
- **(Resolved in 7 and 8.) Something outside the callback clears the type flags** 1–230 ticks after a startle; this was not found. The test reads the flags from the recording, and play keeps them set until the next point.
- **Steering (row `steer`) is still not ported**, so a startled type 1 walks with it. The test skips moving ticks of steering rows. That code is in the generic tick's `steer ≠ 0` branch: per-axis angle easing at +0x184/+0x188 toward the target, then a rotation matrix and a step of speed × forward.
- Sounds, +0xbc, controller speed (type 32 runs at 1) and facing angles are not kept (`ponytail:` in npc.rs).
