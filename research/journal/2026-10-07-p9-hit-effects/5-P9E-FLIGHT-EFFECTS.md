# P9e — flight effects

There are two objects:
- the ball's ribbon, fx+0x9c (vtable `0x1d18a0`);
- the hit glow disc, fx+0xa4 (vtable `0x1d1900`).

Both are driven by the effect manager's event handler `0x33e5d0`:

| Event | Effect |
|---|---|
| 0 | ribbon reset |
| 1 (hit) | sets the kind (fx+0xd0); glow on unless the hitter's shot is a smash (fx+0xdd+8p == 4); sparks; ribbon reset |
| 3 | kind; glow on; ribbon reset |
| 4 (dead ball) | ribbon off (+0x70 = 0); glow off |

## Ribbon

The parameters are in table `0x3fc150` (stride 0x4c, row 0):
- 50 samples;
- collinear threshold 0.05: |d × e| with e normalized;
- minimum step 0.01;
- width = max(0.08, 0.005·tan(fov/2)·camera distance);
- v step 1.0 per unit of length;
- length = 20 × ball speed (10 under one of the game's mode counts > 2);
- y thresholds 0.5/1.5 for a colour gradient. The gradient is moot, because reset writes the same colour to all three stops.

**Reset `0x33ea10`.** Clears the ring and picks the colour by kind. The colours are RGBA bytes, where 128 = 1:

| Kind | Colour |
|---|---|
| 0 | `0x3e0e7f46` |
| 1 | `0x80300846` |
| 2 | `0x33338046` |
| 3 | `0x27802746` |
| 4 | `0x800c8046` |
| smash / other | `0x757d4146` |

With a gm+0x344 mode flag, smash/other is `0x80742280` instead.

**Update `0x33f3e0`.** Runs every frame:
1. The position is court marker 8's translation (+0x30).
2. The new point is compared with the samples at head−2 and head−3 (collinear and minimum-distance tests). A kept point is written at head, and head advances. Otherwise it overwrites head−1.
3. speed = sqrtf(x²+y²+z²) of the ball's +0x140, in that madd order. The other orders are 1 ulp off.

The whoosh sound call (`0x1a1b60`) is skipped.

**Draw `0x33f750`.**
- It walks the samples newest first until the run length reaches the length limit.
- The strip is camera-facing, with the width above.
- Alpha falls linearly to 0 at the tail.
- Texture: `ballrolling.tm2` (fx+0x50 entry 0).

## Glow

- Life is 30, and it decrements each frame.
- Textures are `impactef_{top,slice,flat,lob,drop}`, with flat → top and drop → slice.
- Toggle `0x341e90` starts it. Draw `0x3417c0`:
  - draws a disc of half-size 0.8/2 at the ball, perpendicular to the velocity;
  - spins it 90°·life/30 about the flight direction;
  - sets alpha to life·128/15 once life < 15.
- Not ported: the tilt applied when the ball flies along the court (60° rule), and the texture flip by facing (0x1a/0x1b).

## Check

**Recording.** `tools/record_flight.py 5 2400 context/fixtures/flight_s05.bin` (parser: `research/flight_rec.py`). Each frame holds:
- the clock;
- fx;
- the ribbon object and its 50 samples;
- the glow;
- the ball;
- marker 8.

**Test.** `crates/hst-sim/tests/flight.rs` feeds the marker position and ball velocity, and starts the ribbon/glow where the game's do. Over 22 ribbons, 20 glows and 1653 live frames, these are bit-exact:
- live;
- glow life and texture;
- colour;
- ring count, tail and head;
- every sample's bits;
- speed.

**App.**
- `effects::BallFlight` (a resource) runs from `start_effects`. A hit starts the ribbon, plus the glow unless it is a smash. Leaving Serve/Rally stops both.
- `draw_flight` builds the ribbon mesh and places the glow disc.
- `look()` now loads every `yumoto/*.tm2` effect material for sparks, trails and flight.

**Screenshots.**
- `context/shots/p9/p9e_4.45.png`: the orange topspin glow ring at the ball, just after the serve.
- `p9e_4.7.png`: the ring at the ball, with the violet ribbon trailing back to the server.
- `p9e_17.9.png`: mid-rally.

No game frame of the ring or ribbon was captured: `game_k0.png` is the hit frame, which shows only the sparks. Grabbing the PCSX2 window is unreliable here. The check is therefore the bit-exact sim; the drawing is by reading `0x33f750`/`0x3417c0`, and the game's blend modes are not matched.
