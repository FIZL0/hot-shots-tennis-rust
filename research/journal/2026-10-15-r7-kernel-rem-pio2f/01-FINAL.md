# R7 — 01 (FINAL)

## What the game does
`__ieee754_rem_pio2f` sends |x| > 0x43490f80 to `__kernel_rem_pio2f(tx, y, e0, nx, prec = 2, two_over_pi)` with
x cut into three 8-bit mantissa pieces. The kernel is newlib's `kf_rem_pio2.c` exactly (decompile + disasm read):
no `madd.s`, plain `mul.s`/`add.s` in fdlibm's order, `init_jk = {4, 7, 9}`, a 198-entry `two_over_pi` of bytes
(matches the true 2/π digits), helpers `scalbnf` and `floorf` (newlib's, `huge = 1e30`, `tiny = 1e-30`).
The one build difference: the `PIo2` float table has six of its eleven pieces one ulp **below** fdlibm's exact
values (0x37d9ffff, 0x2e83ffff, 0x2b4fffff, 0x27c1ffff, 0x22cfffff, 0x1bc5ffff) — the same toward-zero literal
conversion already seen in that build's `acosf`/`powf`.

## What changed
- ps2-float v0.3.0 (`src/libm.rs`): `rem_pio2_large` is now fdlibm's (pieces → `kernel_rem_pio2` on `F`
  arithmetic, `scalbnf`, `floorf`); the old exact fixed-point reduction and its host-f64 step are gone. Generic
  `sinf/cosf/tanf` use fdlibm's exact `PIo2`; `libm::sce` gets its own `sinf/cosf/tanf` with the SDK table
  (HST already re-exports those from `sce`). The reduction reads only bits, so an Inf/NaN pattern is the number
  2^128·1.m there (the game's `sinf(+Inf bits)` = 0x3f7990fc); the large path now takes the raw argument.
- ps2-float tests: the accuracy sweep above 201 is absolute (the chopped reduction is ~2^-22 absolute off near
  multiples of π/2: `sinf(9978003)` is 217 ulp off the true value, same bits as the game's), `libm_sce.rs` pins
  game values incl. three where the two tables differ. Exhaustive run (not committed, 6 min on 16 cores): no
  panic and no out-of-range index over every float above 201, both signs.
- ps2-oracle v0.1.2 (ps2-float bump), both decomp kits bumped (tests pass, pushed), opentennis pins both.
- `crates/hst-oracle/tests/libm.rs`: `sinf/cosf/tanf` now draw any 32-bit pattern / any finite / [-8, 8] /
  up to ±2^24 instead of folding into |x| ≤ 201: 20k each, **bit-exact** (before: 461/10000 one ulp off).
  Sensitivity check: with the exact `PIo2` in `sce` it fails (3/5/1 of 20k differ).

## Not verified / not 1:1
- none for sinf/cosf/tanf: every 32-bit argument class is in the oracle run and matches.
- `prec` 0/1/3 branches of the kernel are not ported (sin/cos/tan only use `prec = 2`).
