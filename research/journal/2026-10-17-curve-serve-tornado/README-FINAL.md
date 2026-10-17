# Curve-serve tornado off the ball's path (2026-10-17)

User: on Lola's slice (curve) serve the white effect isn't in line with the ball's path (it had been "fixed" before:
B60 checked the colours, B504 the ribbon head). The white effect is the ball's wind tornado (`wind2/tatumakiball`).

## Cause
The original's tornado update aims with the ball's +0x140, its **last move** (new − old position; P14 journal 17),
through a 4-word getter. On a curve serve the ball moves by vel + wind + bend, so +0x140 ≠ velocity.
`play/tornado.rs` fed `ball.vel`, so the tornado pointed along the yawed launch line, sideways off the path.
The sim test (`tests/tornado.rs`) already feeds +0x140, and foot_s05 has no curve serves, so it never showed.
The ribbon/glow per-frame feed (`flight.frame` in `start_effects`) had the same mix-up; `tests/flight.rs` also feeds
+0x140 (P9e: ribbon speed is |+0x140|). Both are now on `Flight::last`.

## Check
Port, Lola vs Lola, stage 10, human slice serve via `HST_KEYS` (KeyK added to the scripted keys):
`context/lola-fx/before_after.png`: old wiring (left) has the tornado sideways off the ball, fixed (right) has it
trailing along the ribbon. Tests: hst-play hit_fx, hst-sim tornado + flight pass.

## Not verified
No PCSX2 side-by-side of the same serve (the decompile + the fixture offsets are the evidence).
