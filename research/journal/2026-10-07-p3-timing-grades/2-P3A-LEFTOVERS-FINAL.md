# P3a — timing-grade leftovers, bit for bit

## Launch residuals (all 234 recorded strokes/volleys now exact: velocity, frames, target)

- **Scattered ~1e-4**: the lookup's sloped low bound (37c9a0) takes the *scattered* hit z (hit − scatter), not the
  ball's. `Bounds` now carries `sloped` and `lookup` recomputes the low bound from its own hit point.
- **Strokes a few ulps off**: the stroke high bound is `msub(0 + (−1.163835883), 1.3, 1.3) − 0.5` (bits 0x3f94f893),
  not −1.1638.
- **Slices**: stroke kind 1's low bound is −radius (no slope); kind 4 low = high = −0.3; volley kind 1 likewise
  −radius; volley high −1.6·1.3 − 0.5.
- Lookup: the far side is turned with the VU0 `rot_y(π)` transform; an index at the axis max steps to N−2 with
  fraction 1.0. Scatter add/sub in `play/timing.rs` go through `ps2`.

## Mis-hits (3467b0, after the swing lock, grade 4 only)

- Applies to ground strokes (branch 1) of kind 0/1/2, volleys (2) and dives (3). Chance: base 0; dive 20 (and the
  framed threshold 20); a slice (kind 1, branch 1/2) −15. Not a dive: stamina +0x3df4 < 10 adds 20 (10 for a slice,
  framed threshold 20); the current motion (player anim +0x20) 0x1a/0x1b or odd adds 10.
- Height +0x3f44 ≤ 0.2: roll%100 < chance + 20 → mis-hit; ≤ 0.4: chance + 35. A slice, or a second roll%100 ≥
  threshold + 50, is dull (+0x3f0c = 1, scale = {0.9, 0.85, 0.8} or {0.95, 0.9, 0.85}[roll%3]); else framed
  (+0x3f06 = 1). No mis-hit on a flat/topspin ground stroke → 0.95. A volley's scale < 1 → 1 − (1 − s)/2.2.
  Rolls are `(MT >> 16 & 0x7fff) % n` from the shared RNG (gm+0x80).
- Framed: a lob (kind 3, blend 0, no counter) to a random spot with its own side/depth error (37b0d0 args), op order
  from the disassembly (`swing::wild_aim`): 15% sideline (by bit 16) 3 + 3.4u deep; 50% anywhere across
  (−line + 2·line·u) 3 + 3.4u deep; else roll%100 < 33 baseline 11.885 anywhere across with depth error
  (−0.66 + 1.66u)/1.5, < 66 the +x sideline (+end) / else −x (−end), 3 + 8.885u deep with that side error × end.
  Depth × end (+0x12b0). u = 2^−32 · utof(MT).
- The stamina cost lands 2 frames before the launch, so the roll sees it (the app subtracts it for the roll).
- Reactions: the mis-hit shout (program 2) and framed/dull hit sounds were ported in N3 (`sound::Hit` framed/dull);
  they now fire. The framed lob also whistles (kind 3).

## Verification (`tests/timing.rs`)

- Dull hits: both recorded (match_s05 14984 slice 0.85; 1p3goodcpus 21486 volley 0.9 → 0.9545) launch exactly with
  one of `mis_hit`'s scales.
- Framed hits: all 6 (5 distinct) launch exactly from one of `wild_aim`'s shapes: 4 without error, match_s05 16669
  the baseline with depth −0.224, new_recording 7389 the −x sideline with side 0.258 (solved per shape, then floats
  around the root; target and launch exact).
- `shot_tables` ground strokes are asserted exact too (were only reported).

## Left (PLAN P3b, P3c)

- The app draws from its own xorshift, not the game's MT19937 in the original order.
- Dives take no timing error in the app; the awkward input uses the swing animation, not the recorded motion id;
  what the launch does with the other-hand flag (37b0d0's 8th argument) is unread (all recorded launches match).
