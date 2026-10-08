# 8 — Type flags computed (P14c3b)

- P14c3a already ported the clear: the draw's stage countdown (+0x27a) clears the type flag at 0 (`step_near`). Play
  uses `step_near`, so it no longer holds a flag until the next point.
- `startled_creatures_match_the_game` now steps sample by sample over all creatures, carrying the flags from the first
  sample. Only the creatures' ticks set or clear them; at an unstepped message tick, a reset message (0xc, 0xe, 0x1a)
  runs `reset_near`, which clears them. After every tick the computed flags must equal the recorded ones.
- The only clear at an unstepped tick is court 2's vsync 32060, message 0x19 → 0xe (a point's end).
- Check: removing the countdown's clear fails court 1 at vsync 5858 (type 1).
- Bit-exact on courts 1, 2 and 11 (5992 / 7056 / 2384 ticks).
