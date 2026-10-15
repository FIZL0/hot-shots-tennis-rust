# 1 — convert the ten modules (FINAL)

## What changed

- Every `ps2::add/sub/mul/div/madd/msub/sqrt` and `vu0::…` lane call is now an `F` / `Vf` operator or method in
  the same order (`add(add(a, b), c)` → `a + b + c`, `madd(add(0.0, x), a, b)` → `(F::ZERO + x).madd(a, b)`).
  Public signatures and fields stay `f32` / `V4`.
- `mesh.rs` helpers: `push` takes the scale as `F`; the corner/edge/face sweeps keep their distances as `F`.
  `texel`'s fold is the game's libm `fmod` in double precision, so that one closure is
  `#[allow(clippy::float_arithmetic)]` (f64 math, not the FPU).
- `quat.rs`: `micro_slerp`, `atan_poly`, `sin_poly` run on `Vf`; `from_matrix`/`to_matrix`/`fpu_*` on `F`.
- Unary minus on consts goes through `const NEG_PI` (world) / `NEG_R` (clouds); `world::bounds` starts from
  `f32::MIN` instead of `-f32::MAX` (the same value).
- `tests/float_math.rs`: `vu0::lerp` no longer counts as a hand call. It's the four-lane blend on `V4`, a helper
  like `vu0::dot3`, and there's no `Vf` equivalent; only the scalar `ps2::lerp` (`F::lerp`) is in scope. Rule
  noted in `plan/C2.md`.

## Proof

| check | result |
|---|---|
| `HST_HASH_TRACE` `hash_trace` (1 match, 10702 ticks) | identical to the baseline from the unconverted tree (`cmp`) |
| `HST_HASH_GAMES=3` (14712 ticks) | identical (`cmp`) |
| `tools/check.sh -p hst-sim` | 251 passed (world, mesh/live, bodyhit, tornado, weather, clouds, replay fixtures) |
| `cargo clippy -p hst-sim --lib -- -A clippy::all` | clean |

Baselines in `context/c4/base*.txt`, after-traces in `context/c4/after*.txt`.

## Not verified / not 1:1

- **Rounding changed where the old code used plain IEEE `f32`** (now the EE FPU's chop):
  - `clouds::tick` (drift step `speed/600·40`, the ±2R wrap) and `clouds::fade`. These are sky rendering only, not in
    the sim or the state hash. There's no decompile reference for these two (P17's journal measured them in
    RAM), so they were only checked by the module test's tolerance (±1e-4 drift, ±0.02 fade), not bit for bit.
  - `weather::look`'s fog `(1 − v)·255` and `weather::shadow`'s `s·0.75/0.5/0.25`. `look_matches_game` (PINE values,
    including far = 0x42cbffff) still passes bit-exact. The shadow products are exact for integer S ≤ 255 either way.
  - `tornado::opacity` `alpha/128`: dividing by a power of two is exact on both paths (denormals aside).
  - `bodyhit` `Popup::new`'s `-(0.6/30)`: was already `ps2::div`; only the negation moved onto `F` (a sign flip
    both ways).
- The hash traces only cover what a mashed-pad bot match reaches. Tornado, clouds, bounce words and the court
  animation don't feed `state_hash`, so for those the module tests are the only check.
- PCSX2 wasn't used: no game behaviour changed, so there was nothing to record.
- **Wrap-up wasn't through `tools/done.sh`**: `tools/check.sh --no-fail-fast` passed every target except
  `play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally`. That test expects the five `hst_umpire_*`
  mods under `mods/`, which is a symlink to the main checkout's shared `mods/`, and only `hst_umpire_04_anna` is
  there right now (environment state, unrelated to this float change). The clippy step, the tick and the commit
  were run by hand, as done.sh would.
