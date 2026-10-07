# N3d2: whiff shout, doubles call-out, reaction voices (FINAL)

## Paths
- Whiff: the swing countdown (+0x3ec4) reaching 0 with the miss flag +0x3ec8 (`0x349410` strokes, `0x351dd0` serves)
  calls `0x350840`: a swing motion (0x10..0x19, 0x1f, 0x25, 0x26) turns into its miss motion (0x27..0x2a),
  +0x3f96 = 1, and unless +0x4088 or +0x3f04 the player shouts program 4 keys 0..(players == 2).
- Call-out: the CPU object (`0x3cebf0` → `0x3d02c0`/`0x3d1320`, player at +4) gets +0xdc = 1 on each new plan
  (`0x3ce4c0`, `0x3ce7c0`). When it hands the ball to the partner (side ^ 2: `0x3d51e0`, +0xd4 = 1) it calls
  `0x3d5b20`: with player +0x3fa5 ≤ 1 and a 25% roll (`0x3640b0(…, 0x19)`, RNG 0x427130) program 6 key 3 + random
  bit through `0x355350` (which also logs it for the replay when gm+0x55 == 3); +0xdc cleared either way.
- Reaction voices (`0x354050`, player +0x3fa4 == 2, post-point): on the reaction's 5th frame with more than one
  player, if `0x369530(1, 1000, gm+0xbc, +0x1808+0x30)` is 0 and the player stands in the close view (cos ≥ 0.643 to
  the fixed axis at 0x1e7d30, projected size over 125/80): by reaction motion +0x3db0 — singles 0x2e → 9 keys 0..1
  (slot 4), 0x2f → 10 (slot 5), 0x2c → 7 keys 1..2 (0 with gm+0x35c), 0x2d → 8 keys 0..1 (2), other on the losing
  side 30% → 8 keys 3..4 (slot 3); doubles 0x2e → 9 key 0, 0x2f → 10 key 0, 0x2c → 7 keys 0..1, 0x2d → 8 key 0, else
  losing side → 8 key 1. Not ported (camera check).
- Program 6 keys 0..2 (`0x348a00` events 0x10/0xe, `0x3522b0`) need singles-only or partner conditions not hit here.

## Proof
`shouts_match_the_game`: the one missed swing (p3, 3044) shouts program 4 on its bank 9 frames later; the three
program 6 plays (1809 p3, 2740 p0, 3012 p1) are key 3/4 by a player of the side that did not hit last whose partner
takes the next ball. Frame 1029 (p1 program 7 key 1) is a reaction voice, left out.

## App
`play.rs` shouts the whiff at `WHIFF_POSE` and has the stand-in AI call once per incoming shot it leaves to its
partner (25%, at once rather than a few frames in).
