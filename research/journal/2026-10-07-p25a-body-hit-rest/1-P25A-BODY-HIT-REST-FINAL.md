# P25a — body hit, the rest (FINAL)

## Capture
`research/bodyhit_rec.py 5 context/p25a/real.bin 4.0 200` (slot 5, bot doubles on court 10): after the rally
starts it pokes every player's collision size (+0x13ec) to 4 m so a real body hit comes soon, then records per
frame the game object 0x60, the live ball 0x290 and per player +0x3b40 (0x80), +0x3f80 (0x40) and its anim object
(0x80). Parser: `research/bodyhit_parse.py`. Copied to `context/fixtures/bodyhit_s05.bin` for the test.

Player 1 was hit at vsync 7844 (tick 63): the ball's position is the ordinary step's, its velocity turned from
(−0.0586, −0.0222, 0.1959) to (0.0343, 0.0000, −0.1147), bounces 1 → 2; phase 3 → 4 two frames later.

## The hit (mode-3 handler, called from the player update's body test)
In order:
1. Facing: +0x3d60 and +0x3dc0 = −(d̂x, 0·inv, d̂z, 0·inv) with d the ball's last move (+0x140) and
   inv = 1/sqrt((0·0 + dx·dx) + dz·dz) (that op order). +0x3d40 = cross(+0x3d50, +0x3d60)·hand; +0x3e10/20 copy
   +0x3d70, +0x3e30 copies +0x3d60.
2. Motion 0x2b at speed 1, not looping, no hold.
3. Broadcast message 0x14 with the player. The ball's handler builds a plane normal
   n = (−vx·inv, −0, −vz·inv, −0), inv = 1/sqrt(vz·vz + vx·vx) from its velocity, and runs its step with it.
4. Remembers the player (the pop-up's flag); voice program 5 keys 0..0 (one RNG draw), at the player, 0x80.

### The ball's plane step (ball step with a non-zero plane argument)
- Velocity only scaled by the slow-motion factor: no drag, Magnus, gravity, wind or curve.
- One sub-step: start = position, end = start + v, swept against the plane through the end point with normal n
  (radius = ball radius). Contact response with material 0, as a first special touch (slide ×0.1, restitution
  from the material table or 0.05, special kick) — the same three branches the first net touch takes. Bounces
  and contacts counted as usual.
- The position stays where it was (not the contact point); +0x140 is not updated; frame counter +0xac still
  advances. No line calls.
- In the frame: the ball's ordinary step runs first, then the player update finds the hit and the plane step
  turns the velocity. The port's `detect` runs before `simulate` on last tick's ball, so reflecting there gives
  the same course.

`crates/hst-sim/tests/bodyhit.rs`: the frame before the hit stepped against court 10's world, then
`Flight::step_plane` with material 0, equals the hit frame bit for bit (position, velocity, spin, spin frame,
frame, bounces, contacts); the frame after follows by the ordinary step. Without the plane step the velocity
assert fails.

### The hit player afterwards
- In a match (≥ 2 players) a mode-3 player has no mover: stands still. The capture's hit player plays 0x2b from 0
  on the hit frame to its end (36) and holds it through tick 135.
- At the point's reaction the hit player's reaction motion is 0x2b again: set, then snapped to the clip's end
  (normally a no-op since it has finished), reaction state 2 (done): no root motion.
- Practice (< 2 players): once 0x2b ends the player goes back to standing.

## Message 0x15
Sent by the ball-spawn routine (it copies a whole ball object and draws from the RNG) whenever a new ball is put out
with the game object's phase non-zero. The game object only acts on it in phase 3 (rally) with sub-state ≠ 4: it
clears the pop-up's "made" flag and re-seeds the pop-up list. A new ball during a rally only happens in practice's
ball feed; in a match the next ball spawns at the serve. The port has no practice mode (2 or 4 players only), so
there is nothing to re-arm.

## Port
- `hst_sim::ball::Flight::step_plane`; `FrameFlags::plane` makes `respond` take the first-special branches.
- `hst_sim::sound::hit_cry` (program 5 key 0).
- `play/bodyhit.rs`:
  - `struck` on a real hit: plane step, facing from the ball's last move (the previous tick's position is kept in
    `Hits::last`), motion 0x2b, the cry.
  - `standing`: the hit player stays put (`locomote` returns early); `react` keeps its 0x2b with no root.
  - `posed`: the bone test's pose is the crossfade mix over the first 23 tracks, as `character::animate` draws it.
  - Alpha drawn between the last two ticks' values by the fixed-step overstep.
  - `HST_BODY_SIZE=<m>`: every player's collision size, like the capture's poke.
- `HST_AUTOPLAY=1 HST_BODY_SIZE=4 hst <iso> --play --shot context/p25a/port_3.6.png --shot-at 3.6`: the serve hits
  player 2, the ball comes back off her ((−0.156, 0.038, 0.575) → (0.088, 0, −0.326)), SMACK pops; she stays at
  her spot in 0x2b facing back along the ball's path until the next serve (a fault: no reaction).

## Left out (ponytail)
- The unfinished 0x2b isn't snapped to its end at the reaction (in a match it has always ended by then).
- The plane step's extra spin turn of the ball model; whether the plane contact plays a bounce sound (the port's
  bounce sounds read the count change in `simulate`, after the plane step, so it doesn't).
- In the crossfade mix a node the base clip doesn't key mixes from the other clip's value.
- The voice's RNG draw (the port's random numbers aren't the game's anyway).
- The crossfaded pose isn't checked against a capture mid-fade.
