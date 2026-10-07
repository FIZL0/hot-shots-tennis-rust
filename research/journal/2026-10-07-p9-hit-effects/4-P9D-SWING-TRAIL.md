# P9d — swing trail

The plan calls it a "ball trail", but it is the **racket's** swing trail: one object per player at fx+0xac+4p (vtable `0x1d1960`).

## Start (event 2)

Event 2 = `kind | player<<4` is sent when a swing motion starts. That covers strokes, the pending swing at 8 frames, and the serve 0x25/0x26.

On event 2 the game:
- sets trail+0x54 to the player object (gm+0xa8+4p);
- calls start `0x3441c0`(trail, 1).

Start sets:
- length = life = trunc(clock `0x423f88` (=0) + (clipLen − time) / max(0.5, speed));
- the motion id (+0x74);
- stored = 0.

## Update (slot 3, `0x343610`)

It runs every frame, the start frame included.

1. life −= 1. The trail ends at 0, or when the motion changes to anything other than the follow-throughs 0x1c/0x1d.
2. While stored < 120, it samples two points on the racket (matrix `*(player+0x54)+0x88`): T + Y·0.47 (inner) and T + Y·0.93 (outer). T is row 3 and Y is row 1, in the game's madd order.
3. If the new outer point is within 0.01 of the last stored one, it overwrites that sample. Otherwise stored++. The point count is +0x84.
4. With 2 or more points:
   - **Alpha.** Integer ramps: up over trunc(len·0.3) frames, down over trunc(len·0.4) frames, 128 in between. Then + 64 × the distance between the last two outer points, capped at 255.
   - **Reach.** reach = life/len.
   - **Width.** width = 0.5·sin_table(π·powf(1−reach, 0.5)).
   - Then reach = powf(reach, 0.5).

### Helpers

- **sin_table `0x12c910`.** A 256-entry LUT at `0x1bb090` with linear interpolation, folded at π/2.
  - The entries are sin rounded to 6 decimals, plus 1e-6 corrections: entry 59 is −1e-6, and entries 79, 93, 110, 145, 209, 214, 226 and 248 are +1e-6.
  - It is ported as `libm::table_sin`.
- **powf `0x1121f0`.** It special-cases y == 2 (x·x) and y == 0.5 (exact sqrtf). The width was 1 ulp off until the 0.5 case was added.

### Not ported

Slot 4 (`0x343aa0`) and messages 4/5 (save/restore) are the replay path, which the port does not have.

## Draw (`0x343d80`, not bit-exact in the port)

- The ribbon has 2n−1 rows, from the tail to the racket.
- For row t = i/(rows−1):
  - points = spline `0x328a40` at (1−t)·reach·(−(n−1)) + (n−1). The last row is the newest sample.
  - The edge is widened by (1−t)·width along the normalized outer−inner direction.
  - UVs: inner (1, 0.98t), outer (0, 0.98t).
  - Alpha: alpha·t.
- Texture: `yumoto/zanzou.tm2`. Colour: 128. Blend: 0x101.
- The spline is a uniform cubic B-spline with its own weights for spans 0 and 1, and clamped indices.

## Check

**Recording.** `tools/record_trails.py 5 2400 context/fixtures/trails_s05.bin` (parser: `research/trail_rec.py`) records 24 swings. Each frame holds:
- the clock;
- fx;
- per player: the trail, its samples, the motion state and the racket matrix.

PINE rounds the 4-byte clock read up to 8. trail+0x54 is still 0 before a player's first swing, so the player object is read from gm instead.

**Test.** `crates/hst-sim/tests/trails.rs` feeds the game's racket matrix and motion to the port. The results were bit-exact for:
- live/end;
- every sample's 8 floats;
- the point count;
- reach, width and alpha;

on all 878 drawn frames of the 24 swings (motions 0x10–0x1f and serve 0x25).

**App.** `effects::SwingTrail` is a component on each figure:
- A swing motion starting (a new id/serial in 0x10–0x1b, 0x1e, 0x1f, 0x25 or 0x26) starts it, with the frames the clip has left.
- `tick_trails` runs after `character::tick` and samples the Racket joint in game space.
- `draw_trails` builds one ribbon mesh with alpha blend.

The racket joint is from the last drawn pose, so it lags one frame. The comment in the code marks this.

**Screenshots.**
- Port: `context/shots/p9/p9d_4.35.png` and `p9d_17.75.png` show faint white sweeps following the swinging racket.
- Game: `game_k0.png` shows the faint white streak behind Carol's serve swing.

Pixel colours were not compared. The game's blend is 0x101; the port's is plain alpha.
