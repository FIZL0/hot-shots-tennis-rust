# 1 — F / Vf types; ball, shot, player converted (FINAL)

## What landed

- `ps2::F(f32)`: `+ − × ÷` are `ps2::add/sub/mul/div` (chop, guard-bit aligned adds), for `F∘F`, `F∘f32`, `f32∘F`
  and the `*=`-style assigns; `madd/msub/sqrt/lerp/abs/min/max/clamp` methods; unary `-` is a sign flip; compares
  with `f32` on either side. Helpers `f()`, `fs()` (array → `[F; N]`), `f32s()` (back). Rust never reassociates or
  fuses, so `a + b + c` stays `(a + b) + c`; a test checks every operator against the function it stands for.
- `vu0::Vf(f32)`: the same over VU0's ops (no alignment, Q divide), `vf()`/`v4()`; `dot3/normalize/cross/
  transform/lerp` rewritten with it.
- `ball.rs`, `shot.rs`, `player.rs`: no `ps2::` calls left, `#![deny(clippy::float_arithmetic)]` on. Public types
  stay `f32` (storage/serde/snapshots); `F` is local to each function.
- The deny attribute also on `ps2.rs`/`vu0.rs` (the primitive functions `#[allow]`ed: they are the model) and on
  the modules with no float math (`court court_anim face mt noise replay score surprise`).
- Enforcement: `crates/hst-sim/tests/float_math.rs` fails when a module lacks the deny attribute or calls the
  `ps2`/`vu0` functions by hand, unless it's on the `PLAIN` / `CALLS` lists; a listed module that's already
  clean fails too, so the lists only shrink. `tools/done.sh` now runs `cargo clippy -p hst-sim --lib -- -A
  clippy::all` after the tests (only the deny lints report), so the deny attributes actually run at every wrap-up.
  Both checked by planting a violation (a `ps2::add(` call in shot.rs, an `a * 2.0` in player.rs): both caught.
- `hash_trace` (`crates/hst/src/play/netplay.rs`, `#[ignore]`): one `snapshot::state_hash` per tick of a mashed-
  pad match, to a file; the proof for this and the follow-ups.
- The rest of `hst-sim/src/` (31 modules, ~540 lint sites, mostly cutaway/shade/camera/effect) is split into C4–C8
  by area; `plan/C2.md` has the conversion rules and pitfalls.

## Proof

Baselines recorded from the unconverted code, then again after all three modules:

| trace | ticks | result |
|---|---|---|
| `HST_HASH_TRACE=… hash_trace` (1 match) | 10702 | identical (`cmp`) |
| `HST_HASH_GAMES=3` | 14712 | identical (`cmp`) |

`cargo test -p hst-sim`: 251 passed, 0 failed (250 before + `float_math`), including the shot table, flight,
ball-path, player and replay fixture tests against recordings.

## Pitfalls found

- `f32 op f32` compiles silently as plain IEEE (`atan2f(..) + pitch`): only the lint finds it.
- Clippy flags unary minus on `f32` variables and consts but not on literals, and skips `const` bodies.
- A function parameter named `f` (player `turn`'s `&mut Facing`) shadows the `f()` helper.

## Not verified / not 1:1

- **No C1 golden traces exist** (C1 is on hold), so "C1 hashes unchanged" was checked with this task's own
  `hash_trace` baselines instead: two mashed-pad matches, not every fixture or netplay run.
- **shot.rs frame lookup changed arithmetic**: the last interpolation step of a shot's flight frames was plain
  IEEE `fz·(f1 − f0) + f0 + 0.0`; it's now the FPU lerp idiom `(0 + f0) madd fz·(f1 − f0)` like the step before it,
  following the decompile's shape. The hash traces and the shot-table tests agree, but no test aims at the
  inputs where chop vs nearest would move the truncated frame count, and it wasn't checked with the oracle.
- **player.rs**: `Stats::new` (`spe / 10`) and `pad_dir`'s final `+ 0.0` were plain `f32` and are now FPU ops;
  the same result except for denormals (the FPU flushes them), which can't occur in these inputs.
- **No `V3`/`V4` vector type** (the task's "e.g."): arrays of `F` (`fs`/`f32s`) were used instead. The original
  computes each component with its own operation order (`(y·y + x·x) + z·z`, not x-first), so a vector type's
  `dot` would hide the order that has to be kept.
- **The other 31 sim modules aren't converted yet** (C4–C8), so plain `f32` arithmetic remains in sim code. The
  task was too big for one run; the lint and the test hold the converted ones and list the rest.
- PCSX2 wasn't needed (no game behaviour changed); not started.
