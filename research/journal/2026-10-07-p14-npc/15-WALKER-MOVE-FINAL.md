# P14e — walking spectators move (FINAL)

Port: `hst_sim::npc::Wander` + `Walker::wander` / `Walker::step_around` (the walker tick's moves; `step` is now
split into `stagger` → `wander` → `animate`). Wired in `crates/hst/src/play/npcs.rs`.

## What the tick does (wander mode)
- **Detect**: players, then the ball, then the other walkers (their drawn positions from the gallery manager).
  A source counts when it moved since last tick (players/walkers: position ≠ last tick's; ball: velocity ≠ 0),
  is within 3 m, nearer than the best so far and nearer than the threshold (player/walker 1.0 m, 0.5 m while
  dodging; ball 2.5 m, 1.5 m). Then: dodging = !frozen, ticks = 0, and the dodge direction is the source's move
  turned ±90° (whichever side step ends farther from it).
- **After detect**: not dodging → animation 3 stops looping; turning back (10 steps of the yaw to the court
  centre) unless frozen; frozen → unfreezes (a near source instead stops the dodge). Dodging → animation 3, rows
  facing the dodge direction, hit counter on a near source.
- **Dodge timers**: velocity = dir · 0.1; 30 ticks → animation 0; every 5th tick of the window three or more
  near hits → frozen + animation 4.
- **Step**: the court rectangle (±5.485 × ±11.885 grown by r = 0.3) gives a plane to slide on (side or corner,
  with a corner-point check along the step); out of it the court's collision world is swept instead. Up to 30
  hits, contact point, velocity minus its normal part, until a tenth of the step remains. Start height is
  −r − 0.1 (done in double, the test caught a ×0.1 misreading). On a hit the walker turns to face the residual
  velocity. Ends on the ground, kept off the net line (|z| ≥ 1).
- Reacting (mode 1): faces the winners' half; no movement.

## Verification
`crates/hst-oracle/tests/walker.rs`: 12 000 random states of one walker in the s04 court-4 match (position
inside/on the edges/corners/outside, any facing, random dodge/turn/freeze state, players/ball/other walkers near
or far, moving or still) — the port's `Walker::wander` against the game's walker tick, compared bit for bit:
drawn matrix, dodge/turn/freeze fields, push, direction, last-seen arrays, the drawn position the gallery
manager keeps, the animation chosen, loop/advance flags. All pass. 6072 cases moved, 5022 dodging, 100 blocked
(43 by the world mesh, the rest by the rectangle), 2411 frozen.

In play: `HST_AUTOPLAY=1 hst <iso> --stage 4 --play --shot-at 45` — walkers stand upright on the ground at their
spots, no drift (`context/p14e/`).

## Not verified / not 1:1
- **World sweep inside the oracle**: the game's world sweep runs VU0 microcode the oracle can't execute; the test
  hooks it with the port's `mesh::World::sweep`, so the rest of the tick is checked around the port's mesh sweep
  (which has its own tests), not against the game's.
- **Animation calls stubbed** in the oracle (set-animation only notes the id; the animation tick does nothing), so
  the loop/advance flags are only compared on cases where no animation was set. The set-animation port itself is
  P14b's.
- **No PCSX2 recording of a walker moving** (none exists; the task asked for one). The tick is proven function
  for function with the oracle, but when walkers get near a ball/player in real play (and how often) is checked by
  eye only.
- **Tick order** of the walkers relative to the players' and ball's moves isn't recorded; play steps them with the
  court generator after the sim (positions from this tick).
- **Ball velocity** in play is the port's per-tick `flight.ball.vel`; the game reads the ball object's velocity
  through an accessor not checked against it.
- **Ground shade**: the tick's ground lookup only feeds the model's lighting colour — rendering, not ported.
- **Dead branch**: the "step straight away" dodge (+0xc4 ≠ 0) is never set by any creator; not ported.
- **Rain**: the original skips the whole walker tick in rain; play skips the moves but still runs the
  animation (as before this task).
- **Mode-1 facing and the rain skip** aren't in the oracle test (mode 0 only).
- **Prev positions at spawn** are zero (the game's creator values not read); first tick may count everyone as
  moving. Presentation only.
