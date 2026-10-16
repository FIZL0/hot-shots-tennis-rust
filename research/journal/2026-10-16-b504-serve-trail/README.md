# B504 — ball trail off the ball on serves

User (2026-10-16): on Anya's, Miranda's and Lola's ○ slice serves, the flight ribbon (and the other ball fx) doesn't
follow the ball.

## The original (PCSX2 copy 4)

How: `pick.py` 10 (Lola) and 8 (Miranda) to slot 8, then serve with ○ ×3. Each frame was recorded the way
`record_flight.py` does it (the effects object, ribbon + 50 samples, glow, ball, court marker 8). Captures:
`context/b504/lola1.bin` (topspin into the net), `lola2.bin` (slice), `miranda1.bin` (slice). The scripts are
`context/b504/serve_rec.py` and `an.py`.

- **The ribbon's newest sample is the ball's position (ball +0xe0 = marker 8) on every live frame**, in all three
  captures. Lola's slice serve curves (x 2.95 → −1.95, steps 0.64 → 0.38 m a frame); Miranda's runs 0.43 m a frame.
  The game draws one picture per frame, so its ribbon always starts at the ball.
- **The port's `effect::Flight` sample logic is bit-exact on these curving serves.** `tests/flight.rs`, pointed at
  each capture via `HST_FIXTURES`, matched every sample, the ring indices and the speed: Lola 3 ribbons over
  171 frames, Miranda 1 over 76. It then failed only its "≥ 20 ribbons" coverage count. So the sample timing along
  the curve, and which samples are kept, are not the problem.

## The cause

Drawing in the port. `play.rs draw` draws the ball at `lerp(prev_ball, pos, overstep)` between two ticks.
`draw_flight` put the ribbon's head and the glow disc on the tick's own `pos`. So on every display frame they sat
(1 − overstep) of a tick's flight *ahead* of the ball, then snapped back at the next tick: a sawtooth.

Measured in the port (temporary log, Lola vs Lola, stage 10): the gap ran from 0.95 m down to 0.01 m inside each
tick, on a display faster than 60 Hz. Serves are the fastest shots (0.6–1.0 m a tick), which is why they show it
most. The ribbon itself stays on the path; it only sticks out past the ball and jitters.

## Fix

`hst-base/src/effects.rs`:
- `BallFlight` keeps the match's `prev_ball` (passed in by `start_effects`).
- `draw_flight` puts the glow disc at the interpolated ball, the same lerp and overstep as `draw`.
- `ribbon_points` starts the ribbon at the interpolated ball, in place of its newest sample. The drawn ball always
  lies on the newest kept segment: the last tick's position is either the second-newest sample, or within the
  0.05 collinear test of the line through it.

The sim (`effect::Flight`) is unchanged, so gameplay, the RNG and match timing are untouched.

Test: `effects::tests::ribbon_starts_at_the_drawn_ball` uses Lola's recorded serve positions. `hit_fx` and the
netplay suite (`HST_NETPLAY=1`, 27) pass. Port shots: `context/b504/port/old8.png` (old placement, hit+8) and
`new8.png`. They are different serves, because the bot match isn't seeded the same way, so the numbers above are
the evidence, not the pictures.

## Not verified / not 1:1

- **Anya:** not checked against the original. She is an Out of Bounds mod character, and there is nothing of hers
  in PCSX2. A mod character runs the same flight and draw code, so the fix applies to her too.
- **No side-by-side picture of the same serve, PCSX2 vs port.** PCSX2 F8 shots land about 38 frames late (B60), and
  the port's bot serves weren't reproducible between runs. The comparison was done on numbers instead: the original's
  ribbon head is on the ball, and the port's gap went from up to 0.95 m to 0. B60's `context/b60/cmp.png` still
  serves as the look reference.
- **Ribbon length is not interpolated.** The length (`speed`·20) and the kept samples behind the head stay at the
  tick's values while the head moves between ticks. That is under a tick's worth of change, and nothing to match:
  the original has no in-between frames.
- **Pad mapping on copy 4:** the virtual pad's ○/✕ act swapped in the menus (✕ confirms), but in the match `circle`
  serves a slice. Worked around in the script; not chased.
