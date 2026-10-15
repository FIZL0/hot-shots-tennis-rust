# 1 — ai, rng, libm converted (FINAL)

## What landed

- `ai.rs`: every `ps2::add/sub/mul/div/madd/msub/sqrt` call is now an `F` operator in the same order (`atof`,
  `Choice::new`'s reach, ball `speed`/`pace`, the fast-ball km/h test, `Guess::target_x`/`verdict`,
  `run_round_width`, `stand_side`, `serve_spot`, `Runner::frames_to`, `reach_search`/`tier_search`/`scan`).
  Public fields and signatures stay `f32`. `target_x` was plain `side * ±5.485` and `abs` a plain `-v`: now FPU
  `mul.s` / `neg.s` — the same bits, since `side` is ±1 and a negation is a sign flip either way.
- `libm.rs`: the kernels and `sinf/cosf/tanf/atanf/atan2f/acosf/powf/table_sin` do their float ops on `F`; every
  bit trick (`to_bits` masks, `hi()`/`chop`, the exponent arithmetic, `trunc_i`) stays on the `f32` bits. Kernels
  return `F`. `acosf`'s `sqrtf` stays the host's correctly rounded sqrt (as before; the game's is the exact
  software one). The dead second `if y == 2.0` in `powf` was dropped. `table_sin`'s table is built by a separate
  `sin_table()` in host f64 with a scoped `#[allow]` (the game ships it as data; it isn't game arithmetic); the
  test module's host references carry an `#[allow]` too.
- `rng.rs`: `Mt::unit` on `F`. The lens flare (`setup_rand`, `flare_tick`) and `MenuFlame` keep their plain `f32`
  behind scoped `#[allow(clippy::float_arithmetic)]`s: they were already ponytailed as look-right plain f32, and the
  flare sits in `Game`, which `state_hash` prints, so moving it to FPU rounding would change the hash this task
  must keep.
- `tests/float_math.rs`: `ai`, `libm`, `rng` dropped from `PLAIN` and `CALLS`.
- The lint caught one slip while converting: `k_cos`'s `1.0 - qx` was `f32 − f32` (plain IEEE) after the
  rewrite; it's `F::ONE - qx` now.

## Proof

| check | result |
|---|---|
| `hash_trace` (1 match, 10702 ticks) vs baseline from the unconverted code | identical (`cmp`) |
| `HST_HASH_GAMES=3` (14712 ticks) | identical (`cmp`) |
| `tools/check.sh -p hst-sim` | 251 passed |
| `hst-oracle` `ai` (run frames, 20k) and `libm` (atan2f, atanf, powf) vs the game's code | passed (ran, no skip) |
| temporary test: HEAD's `libm.rs`/`ai.rs` compiled beside the new ones, 3M mixed inputs (raw bit patterns and ranges) through sinf/cosf/tanf/table_sin/atanf/atan2f/acosf/powf, 300k through atof/Choice/target_x/verdict/run_round_width/stand_side/serve_spot | bit-identical; a planted sign change in `k_cos` made it fail; test removed afterwards |
| `cargo clippy -p hst-sim --lib --tests -- -A clippy::all` | clean |

## Not verified / not 1:1

- **Flare and menu-flame stay plain `f32`** (scoped `#[allow]`), not FPU arithmetic like the original: they only
  have to look right (PLAN), and the flare is part of the hashed `Game`, so converting would break "hash_trace
  unchanged". A later task could move them to `F` together with a new hash baseline.
- **`Searcher::reach_search`/`tier_search`/`scan` and `AiParams::timing`** weren't in the old-vs-new test (they
  need a whole searcher set-up); they're covered by the two hash traces (bot AI over full matches) only.
- **`sinf`/`cosf` of ±inf/NaN** now go through `ps2::sub` (`x − x`); both give a NaN (the old/new test agreed on
  NaN-ness, not payload bits). Only a broken mod pose feeds one in.
- PCSX2 wasn't needed (no game behaviour changed); not started.
