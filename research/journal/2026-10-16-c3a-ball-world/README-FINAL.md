# C3a — readable ball and world modules

Task: C3's "no RAM mirroring" pass for `hst-sim/src/` contact, mesh, world, quat, bodyhit, tornado, clouds, weather,
noise, court_anim, ball, court. Behaviour-preserving only.

## What changed

- **contact.rs**: the sweep's tolerances are named (`SKIN` 1.005, `APPROACH` 0.005, `SUNK` 0.98, `HOLD_BACK`,
  `HOLD_SHARE`); a test pins their bits to the game's (`tolerances_are_the_games`). mesh.rs shares the first three.
- **mesh.rs**: `EDGE_SLACK`, `PUSH_OUT` (+ its penetration), `CORNER_CLEAR` (stays `from_bits`: the game's value
  is one ulp above the nearest f32 to 0.9604), `LOOK_PAST`, `NO_MATERIAL`; the wrap-mode byte is a `Wrap` enum
  (`Wrap::from_mode` from the model file's number; court.rs converts); the winding flag hidden in the third
  texture coordinate's w sign is `Tri::reversed()`; the nibble lookup is `AttributeMap::texel`.
- **court.rs / hst-data**: the attribute-map header offsets (2, 4, 0x14) moved into `mtl::AttributeMap::width/height/stride`;
  plant categories are `layout::category::{TREE, PROP, …}`; the umpire's creature index and the collision category
  list are named.
- **world.rs**: category ranges through `layout::category`, the code-digit decode reads `b'0'`/`b'A'`, `STEPS`
  (36) and `NET_POST_REACH`.
- **quat.rs**: the slerp micro program's polynomial coefficients are the `ATAN` / `SIN` tables.
- **ball.rs**: shot class/kind numbers the bounce reads (`STROKE`, `VOLLEY`, `SLICE`, `DROP`; `k::KIND1_*` →
  `SLICE_*`, `KIND4_*` → `DROP_*`), the court plane (`ORIGIN`, `INTO_GROUND`), `MAX_SUBSTEPS`, the predicted
  path's step counts (`14 + 15t` written as `PATH_FIRST_STEPS + PATH_STEPS·t`), `EFFECT_NET` / `EFFECT_GRASS`,
  and `turn` uses `world::{HALF_PI, PI, TWO_PI}` instead of bit patterns.
- **bodyhit.rs**: head target, CONK limit, rise/swell, bounce-word table row, hold/fade, contact/gap/max limits named;
  effect kinds via `ball::EFFECT_*`. (sound.rs has the same `effect == 2 / 5` rule: left for C3e.)
- **tornado.rs**: `START_KMH`, `GROWTH`, `UV_EXTRA`, `OPAQUE` (`fade << 7` → `fade * 128`).
- **clouds.rs**: `DRAWS`, `DRAW_BITS`, `DRAW_SCALE` (2⁻³¹), `world::TWO_PI`, drift constants.
- **weather.rs**: weather numbers are named consts (`CLEAR`, `CLOUDY`, `RAIN`, `HEAVY_RAIN`, `RAIN_2`, `HEAVY_RAIN_2`;
  `Game::weather` stays `u8` — it is serialized in snapshots and read by ~30 files outside this task), `heavy()`,
  `chance()`, gust constants, `DEGREE`/`PER_FRAME`, and `wind` folds with `world::{PI, TWO_PI}`.
- **noise.rs**: `WIND_BASE`/`WIND_GAIN`, `AMP_SCALE`, `RATE_SCALE`, `SEED`, the VU R register's mantissa/one bits; a
  test pins the scales' bits.
- **court_anim.rs**: `MATERIAL_TRACK` prefix.

Ball.rs field docs keep their `+0x…` ball-object offsets: they are documentation for reading recordings, not code
that reaches into memory.

## Proof

- `tools/check.sh -p hst-sim`: 253 passed (oracle, replay fixtures, recordings, the new bit-pin tests).
- `hash_trace` before (a detached worktree at the task's base commit) and after, 1 game and `HST_HASH_GAMES=3`, in
  `context/c3a/{base,after}{,3}.txt`: identical (`cmp`), 10702 ticks (1 game) and 14712 ticks (3 games).
- Note for C3b–f: `hash_trace` lives in hst-play now and sits under `netplay::tests::matches`, so plan/C2.md's
  old command matched 0 tests (fixed there now); use `HST_NETPLAY=1 tools/check.sh -p hst-play hash_trace -- --ignored`.

## Not verified / not 1:1

none — no behaviour changed; every rename is checked by the bit-exact tests and the unchanged hash trace.
