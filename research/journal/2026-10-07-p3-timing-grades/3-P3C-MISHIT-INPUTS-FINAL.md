# P3c — mis-hit inputs exactly

## The launch's motion flags (3467b0, branch 1/2 only)

- The player's current motion number (anim +0x20) is mirrored at player +0x3df0 (checked: all 36000 player-frames of
  anim_s05.bin), so every recording has it. On all 259 recorded strokes/volleys it equals the locked swing +0x3e44
  at the launch; the app now reads its own current motion (`Player::cmd.id`).
- odd = motion & 1 (the other hand's side); awkward = odd or motion 0x1a/0x1b (body shot): +10 mis-hit chance.
  `swing::launch_motion`.
- The odd flag is 37b0d0's 8th argument ($7 at the call → $22 in 37b110 → stack arg to 37e420 → stack arg 0 of
  37e950, `lbu 896(sp)`): it multiplies the bend (37e240's first output, after its 23.77 m distance scale) by −1,
  before the left-hander's own −1. Nothing else reads it. The app had this as "backhand" (`!forehand`), which is
  the odd bit only for right-handers; now the motion's odd bit. `rally_effects_like_the_game` (serve.rs) now
  checks round1's two bent slices bit-exact with the flip from +0x3df0 (was "up to the sign").

## Dives

- The dive lock (in the search, after the dive set-up): +0x3ec4 = path index k, offset +0x3fa0 = bias +0x3f98 =
  k − 10 (its own sweet frame, not the tables'), grade 4, 2 when |k − 10| < 2 (`swing::dive_lock`). The app used
  k − 8.
- The launch takes the timing error like a volley, its side/depth along the dive direction (+0x3e60/+0x3e68) —
  `swing::timing_error` already had it; the app now sets it (`play/timing.rs dive_error`, contact height = the ball's
  at the launch = +0x3f44, checked).
- Recorded: p7e_c03.bin vsync 6610 (character 3, k 15, grade 4, error 0/−5/−10): launch velocity, frames and target
  exact. tests/timing.rs now also reads p7e_c03/c07, p5_presses_s04, p7_carol/kaito_s04: 260 launches exact, 134
  awkward, every launch's motion asserted equal to its swing.
