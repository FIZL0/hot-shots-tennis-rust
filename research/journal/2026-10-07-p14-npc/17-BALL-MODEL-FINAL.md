# 17 — P14c4a: the ball's drawn rotation and kept last move

Ball-hit obstacles (types 14/21) test the ball's box with the ball object's model matrix (+0xb0) and read its speed
from +0x140. Both are now carried by `hst_sim::ball::Flight` (`model`, `last`) and fed to the obstacles by
`play/npcs.rs`.

## What the original does
- End of every ball step (the normal step and the obstacle's plane step): if spin ≠ 0 and the spin frame's forward
  row has length, θ = spin·dt wrapped to ±π; R (translation zeroed) → R·Fᵀ·Rx(−θ)·F, re-orthonormalised from its
  forward row (z = n(r2), x = n(r1 × z), y = n(z × x)); row 3 = position. Ported as `ball::spin_model`.
- +0x140 = new − old position, written by the normal step only. A stopped ball (state 3) skips the step, so the
  model and the move are kept.
- Launch (the setter all launches go through) writes +0x130 = +0x140 = +0x150 = velocity. The hit path additionally
  copies a matrix it builds into +0xb0 (and +0xf0) before launching; in the serve hits it equals the new spin frame's
  rows → `Flight::struck`. Toss and new point leave the rotation identity → `Flight::new` (identity at pos, last = vel).
- An obstacle tick has two steps (+0xac +2): two turns.
- The ball held on the server's hand is placed each tick but steps by nothing (last stays 0).

## Port
- `ball.rs`: `spin_model`, `Flight.model/last`, `Flight::struck`; `advance` and `step_plane` turn the model;
  `advance` writes `last`.
- `play.rs`: the shot launch uses `Flight::struck` (one call changed).
- `play/npcs.rs`: ball box = the flight's model rotation at the ball; kmh = `ball_kmh(flight.last)`.

## Proof
`crates/hst-sim/tests/npc.rs` `ball_model_and_last_move_match_the_game` over `hit_c04.bin`/`hit_c09.bin`, every
consecutive tick bit-exact: 368 one-step ticks (turn + move), 8 two-step ticks, 277 standing ticks (kept),
5 launches (3 new: toss/new point, 2 struck: serve hits).

## Not verified / not 1:1
- Non-serve hits' model set: the fixtures only launch serve hits; that the hit's matrix is the spin frame for
  groundstrokes/volleys too is assumed from the same setter path (the hit builds the matrix from the shot's
  rotations), not recorded.
- Two-step ticks: the plane step's own spin frame isn't recorded; the test accepts the turn by the tick's start or
  end frame (which one per tick isn't pinned).
- dt ≠ 1 (slow motion): the port always turns with dt = 1; no recording at another dt.
- The app's own stepping of a standing ball: play steps the flight as before; whether it skips exactly where the
  original's state 3 does is P14's existing ball-state work, not re-checked here.
- Type 21 obstacles are still unrecorded (as before).
