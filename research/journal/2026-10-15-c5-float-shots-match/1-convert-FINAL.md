# C5 — conversion (FINAL)

## What was done

- All eight modules carry `#![deny(clippy::float_arithmetic)]` and use `ps2::F` (`F`, `f32s`, `utof`); no
  `ps2::add/…` calls left. Public fields and signatures stay `f32`.
- The ~340 call sites were rewritten by `research/c2_convert.py` (new): it parses each `add/sub/mul/div/madd/msub/
  sqrt/lerp(..)` call tree and prints it as infix with the same tree (left-assoc, parentheses only where the
  tree needs them), wrapping one f32 leaf in `F(..)` and ending each top-level call in `.0`. The leftovers
  clippy flagged were done by hand (unary minus on `f32` → `-F(x)`; `let m = mul;` closures; closure arg types).
- `aim.rs` reduces angles with `fmodf`; Rust `%` on f32 is the exact fmod, kept in a one-line
  `#[allow(clippy::float_arithmetic)] fn fmod(F, f32) -> F` (fmodf's result is exact, so no rounding question).
- Converter pitfall found on the way (fixed in the script): a paren clean-up regex let `[^()]*` span
  `1] - apex[1`, turning `(hand[1] - apex[1]).abs()` into `hand[1] - apex[1].abs()`. Caught by
  `match_s05_serves`; a re-run of the old and fixed regex over all four files showed that was the only hit.

## Plain-f32 spots that are now FPU ops (rounding changed)

Where the old code used IEEE f32 (round to nearest) for an op the EE does on its FPU (chop):

| where | before | now | check |
|---|---|---|---|
| `serve::launch` table lookup point `hit − scatter` | IEEE sub | `sub.s` | disasm of the shot launch: four `sub.s` (x, y, z, w) at 0x37e60c–0x37e648; the smash path already used `ps2::sub` |
| `serve::toss_velocity` | IEEE ops | FPU ops | `match_s05_serves`: plain f32 bit-exact on 33/41 tosses, `F` on 41/41 → the test's `< 1e-5` became a bit-exact `assert_eq` |
| `swing::smash_whiff` 2 m check | `dx*dx + dz*dz` IEEE | `(dz*dz).madd(dx, dx)` | disasm 0x34ea40…0x34ea98: `sub.s` ×4, `mula.s z,z`, `madd.s x,x`, `c.olt.s` against 4.0 |
| `serve::search` `h − ideal`, `instant` line distances, `flow` hold·60, `serve::dw1`, balloon alpha `/128` | IEEE | FPU | same op as `ai_pick`'s (`ps2::sub` already); dw1/alpha/hold are exact or compare-only at the values used |

Unary minus (`-x` → `-F(x)`) is a sign flip either way; `judge` and `flow::serve_placement` were exact already
(×±1, consts).

## Proof

- Baseline from pristine sources (this worktree, patch set aside): `context/c5/base.txt` (1 game, 10702 ticks),
  `base3.txt` (3 games).
- Control: everything converted except the first two rows above put back to plain f32 → `ctl.txt`/`ctl3.txt`
  **identical** to the baseline (`cmp`). The conversion itself is hash-neutral.
- Final (`final.txt` = `new.txt`, `new3.txt`): differs from the baseline from tick 100 (the first mistimed serve's
  `hit − scatter`) and, with only that fixed, from tick 6607 (a toss velocity). Both changes are the ones verified
  above against the disassembly / the recorded tosses.
- `tools/check.sh -p hst-sim` 251 passed (incl. `float_math`, `serve` with the stricter toss check, replay
  fixtures); `-p hst-oracle` passed, `instant` (100k generated points vs the original) included; clippy float lint clean.

## Not verified / not 1:1

- `serve::launch` `hit − scatter`: the instruction is verified in the disassembly of the shared launch path
  (0x37e420); no recorded serve tells round-to-nearest from chop there (`match_s05_serves` passes both ways),
  so the change is proven by the instruction, not by a capture.
- `smash_whiff`'s `mula`/`madd` order is from the disassembly; no recording has a smash whiff right at 2 m, and
  the hash trace doesn't hit a case where the two orders differ.
- `serve::search` / `instant::past_lines` / `flow` hold: switched to the FPU on the evidence that the game does
  these as plain `sub.s`/`mul.s` (as the already-ported neighbours do); not separately disassembled. The hash
  control shows they change nothing in the traced matches.
- Readability: the converter's output keeps `(F(a) - b).0` per binding instead of carrying `F` through whole
  functions; correct and lint-clean, but less tidy than hand-written `F` code in places.

## Wrap-up note

`tools/done.sh` stopped on `play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally`: `mods/` is a
symlink to the main checkout's `mods/`, which has only `hst_umpire_04_anna` installed (the test wants all five
or none). Not touched by C5 and not fixed here (the user's mods folder). Everything else in
`tools/check.sh --no-fail-fast` passed; ticked and committed by hand after the clippy float lint (done.sh's
other steps).
