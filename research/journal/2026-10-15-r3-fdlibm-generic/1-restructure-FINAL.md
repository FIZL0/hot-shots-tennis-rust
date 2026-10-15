# 1 — restructure (FINAL)

## What changed
- `libm.rs`: module doc now says it is the SDK's newlib build of Sun's fdlibm, float flavour (Cygnus `ef_/kf_/sf_`
  sources), with fdlibm's Sun notice and the Cygnus float-conversion line kept verbatim. Laid out like the kit:
  `b(bits)` constant helper (was `f`), `// --- sin / cos / tan`, `atan / atan2`, `acos`, `pow` sections in the
  kit's order (kernels, `TAN_T`/`k_tan`, `NPIO2_HW`/`rem_pio2`, `sinf cosf tanf`), the kit's doc lines per
  function, `hi` → `hi12`. Game references in comments dropped. No arithmetic changed (every hex constant of the
  old file is still there; only renames and moves).
- `table_sin` (the game's 256-step table sine, a data table, not libm) moved to its one caller,
  `crates/hst-sim/src/effect.rs` (swing trail width). `state_hash`'s line for it is renamed `effect::table_sin`.
- Oracle: found the game's `sinf` 115fb8, `cosf` 115d18, `tanf` 1160a0 (the three callers of `__ieee754_rem_pio2f`
  1129d0; sin/cos told apart by which kernel the small-|x| branch calls) and `__ieee754_acosf` 111398 (thunk
  116148); added to `research/oracle.txt` and `crates/hst-oracle/tests/libm.rs`, 20k inputs each (sin/cos/tan
  within the ported |x| ≤ 201, acos in [−1, 1]). Before only atanf/atan2f/powf were oracle-checked.

## Results
| check | result |
|---|---|
| `tools/check.sh -p hst-oracle --test libm` (7 functions × 20k inputs vs the game's code) | all bit-identical |
| planted 1-ulp change in a `k_sin` constant | sinf/cosf tests fail (21 and 3 of 20000 differ); reverted |
| `tools/check.sh -p hst-sim` | 251 passed |

## Differences to the kit left in place
- The kit flushes Inf/NaN patterns to ±max (`fpu::operand`) in `sinf`/`cosf`; ours returns NaN for NaN/±inf (M16,
  a mod's degenerate pose). The kit has `asinf expf logf sqrtf` and a large-argument reduction; ours doesn't need
  them. Constants differ where this game's SDK build differs (acosf/powf one ulp low, older powf `t_h` seed): that is
  the per-game data the kit doc tells users to patch.
- `trunc_i` stays local (the kit uses `fpu::to_i32`; `hst_sim::ps2` has no such helper yet — R-dedupe can fold it).

## Not verified / not 1:1
- `rem_pio2` above |x| = 2^7·π/2 is not ported (panics), as before; the oracle tests stay inside that range.
- `sinf`/`cosf` NaN/inf inputs: not oracle-checked (the generator only makes finite values; the EE has no NaN).
- `acosf` outside [−1, 1]: port returns the host NaN, not checked against the game.
