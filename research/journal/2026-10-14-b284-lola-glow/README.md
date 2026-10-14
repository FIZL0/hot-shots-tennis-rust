# B284 — Lola's spin draws wrong

Lola's special is the bent slice serve (her shot-effect row: bend only on the slice serve). She has no motions of her
own, so "her spin" was taken to be the glow disc (`impactef_*`) that turns at the ball after a special shot. B245
(same day, a few hours before the report) had changed that disc's turn; that change is what broke it.

## Findings
- The game builds the glow's frame from the ball's last move: side = up × flight, then flight × side, flight. The quad
  spans the first two rows, so the disc **faces the flight**.
- A flight within 60° of the court's length is turned about world up (the game's Y rotation, by
  sign(x)·sign(z)·min(60° − a, a)).
- The turn (90° · life / 30) is a Z rotation multiplied in front of the frame (rotation · frame in row form): each new
  row mixes rows 0 and 1, row 2 (the flight) is unchanged — an in-plane spin of a disc that keeps facing the flight.
  B245 had it as a turn about the side axis (flat along the flight → facing it), which made it a wide flat ellipse.
- Rotation helpers identified by the frame builder's degenerate case (it maps the flight to ±Y with the X rotation),
  giving the usual Z, X, Y order of the three.

## Fix
`effects.rs glow_turn`: disc faces the flight, tilt about up, then spin about the flight axis.

## Check
Port (`context/b284/port1` before, `port2` after, Lola 10 vs 2/1/5, stage 11, screenshots at hit +2…+28) vs the
original's Lola special slice serve (`context/b60/run4/s0s.png`, crop `context/b284/o_crop.png`). Before: wide flat
ellipse; after: tall narrow upright ellipse like the original (`context/b284/port2/m.png`, last tile = original).

## Not verified / not 1:1
- "Spin" assumed to mean the glow disc (unattended run, couldn't ask); no Lola-specific motion turned up.
- By eye only, one moment of the original (B60's screenshot, ~38 frames late, matched by ball position); not at each
  frame of the turn. Spin direction (sign) and the tilt's sign not checked against the original frame by frame.
- Different court/costume from the original's shot.
- The game's slowdown term in the turn angle and its flag switch by flight facing the camera are left out (as before).
