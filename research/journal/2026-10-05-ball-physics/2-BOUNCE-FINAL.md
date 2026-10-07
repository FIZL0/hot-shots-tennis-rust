# Ground bounce, curve, rolling — DONE (commit 9018eed, crates/hst-sim/src/ball.rs)

Verified: 39 whole shots from slot 5 (court 10) replayed from frame 3 to path end, 4524 frames,
velocity within 1e-5 (tests/flights.rs, fixture context/fixtures/flights_s05.csv via tools/fixture_flights.py).

Rules learned (all in code comments too):
- Displacement per frame = vel + wind + curve profile (curve=+0x254 vertical, bend=+0x250 along +0x90),
  profile sin(π·(i+1)/T) − sin(π·i/T), T = max(+0x260, i+1), only before first bounce. Stored vel excludes it.
- Sweep vs plane y=0 at radius 0.064 (+0x8d0). After contact the frame continues with the new velocity for the
  remaining share; if remaining ≤ 0.1 the frame ends at the contact point. Up to 30 sub-steps.
- Bounce: friction 0.3, spin↔speed via contact frame (spin_to_speed 0.2), spin relax per court, frame turn by
  slide/(slide+|spin|), restitution 0.7 × first-bounce scale, spin kick per court (×0.5 kind 1, ×0.1 kind 4,
  classes 1–2, first bounce), spin ×(1−relax) on bounces only. Rolling: ≥2 contacts and |vn| ≤ 0.02.
- PS2 FPU has no NaN: guard zero tangential speed (keep previous contact basis).

Not covered yet: net/walls/other materials (only the court plane), special-shot hooks (gated off in this match),
slow-motion dt, game-mode restitution 0.85. Court tables for courts ≠ 10 copied from RAM but only court 10 verified.
