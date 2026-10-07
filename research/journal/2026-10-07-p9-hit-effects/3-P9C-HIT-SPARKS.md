# P9c — hit sparks

The spark object sits at fx+0xa8 (vtable `0x1d18d0`). On a hit (event 1, after the impact starts), the game calls start `0x342970`(obj, 1, player). Every frame it runs update `0x342170`, then draw `0x3424b0`.

## Object

| Offset | Contents |
|---|---|
| +0x50 | state |
| +0x54 | texture |
| +0x58 | draw model |
| +0x5c | 25 particles × 0x40: +0 active, +0x10 pos, +0x20 vel, +0x30 half-size, +0x34 life (int) |
| +0x64 | 25 rolls × 0x50: a 4×4 turn about z, rows (c,s,0,0) (−s,c,0,0) (0,0,1,0) (0,0,0,1), then u1, u2, u3 uniforms at +0x40 |

- Textures are fx+0x50 list entries kind+6 (`toptubu00` … `droptubu00`), or entry 11 (`smashtubu00`) on a smash (branch 4). The `impactef_*` entries (1–5) are not used by the sparks; they belong to fx+0xa4 (P9e).

## Per-kind table

The table is at `0x3fc410` + kind·0x28, with these fields:
- speed flag;
- spawn flag;
- forward offset;
- spread;
- speed lo/hi;
- exponent;
- size lo/range;
- life lo/hi.

| Kind | Speed flag | Speed lo/hi | Size lo/range | All kinds |
|---|---|---|---|---|
| top, flat | 1 | 0.1/0.5 | 0.1/0.3 | offset 2.0, spread 3.5, exponent 1.1, life 10–40 |
| slice, lob | 0 | 0.3/0.3 | 0.5/0.5 | same |
| drop | 0 | 0.3/0.3 | 0.4/0.4 | same |

## Start

The speed is s = |ball velocity|.

**Smash or speed flag set** (computed from s):

| Quantity | Value |
|---|---|
| n | min(25, trunc(5 + 40·s)) |
| spread | 6·s |
| speed lo | 0.1 + 0.16·s |
| speed hi | 0.3 + 0.16·s |
| offset | 0.5 + s |
| size lo | speed lo |
| size hi | 0.3 + 0.5·s |
| pace base | s |

**Otherwise:** n = 25, the values come from the table, and the base is 0.33.

The pace is powf(base, 1.1). The game's powf is fdlibm `e_powf` with its own constant set, ported as `libm::powf`.

Basis M is the impact matrix (P9a) at the ball. Its translation is replaced by pos + z·offset. Then, for each spark i < n:
1. M = Rᵢ·M (VU row transforms). M keeps turning from one spark to the next.
2. dir = normalize(M.t + M.x·u1·spread − pos).
3. vel = dir·(lo + u2·range)·pace.
4. half-size = (size lo + (1−u1)·size range)/2.
5. life = trunc(10 + u3·30).

Sparks past n keep their old state.

## Update, draw and re-roll

**Update.** Each active spark:
- loses one frame of life and goes off at 0;
- otherwise moves pos += vel, then vel ×= 0.9 and size ×= 0.95.

The update also runs on the frame the burst starts.

**Re-roll.** When no spark is left, the game re-rolls every fourth roll, starting from a random one of the first four (MT19937 at gm+0x80), and sets state to 0. Over the capture, roll angles fall in [0, π) and the uniforms in [0, 1).

**Draw.** Each spark is a camera-facing quad: p ± R·s ± U·s, using the view's right and down rows. Colour is 128 grey. Alpha is life·128/20 below 20 frames, otherwise 128. Blending is normal alpha (0x101).

## Port

**`hst_sim::effect`:**
- `Sparks::start(kind, smash, pos, vel)` and `tick(reroll)`;
- `Roll::new(angle, u)`.

**`hst/effects.rs`:**
- `HitSparks` keeps its own xorshift rolls with the game's ranges; the game uses MT19937.
- `start_effects` starts a burst on a hit and ticks it every frame, start first.
- `draw_sparks` rebuilds one mesh of up to 25 quads with vertex-colour alpha.

## Check

**Recording.** `tools/record_sparks.py 5 2400` → `context/fixtures/sparks_s05.bin`. Each frame holds:
- the effect manager;
- the spark object;
- the 25 sparks;
- the 25 rolls;
- the ball;
- court marker 8.

**Test.** `crates/hst-sim/tests/sparks.rs` uses the game's own rolls. It covers 22 bursts (kinds top 10, slice 3, flat 6, lob 3), and every spark's pos/vel/size/life matched bit for bit on every one of the 2400 frames. The ball position is marker 8's translation; the velocity is ball+0x140.

**Screenshots.** `context/shots/p9/p9c_4.45.png`, `p9c_4.55.png`, `p9c_17.82.png`, beside `game_k0.png`:
- The port throws small blue stars around the flat impact and fading out within about half a second.
- In the game, sparkles surround the purple topspin starburst in the same way.

**Not checked:**
- No smash or drop burst appears in the capture. Smash takes the speed path that top/flat already exercise.
- Pixel colours were not compared.
