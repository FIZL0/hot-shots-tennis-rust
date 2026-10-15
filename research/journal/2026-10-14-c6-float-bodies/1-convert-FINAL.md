# 1 — convert the eight body modules (FINAL)

## What landed
- All eight modules use `ps2::F` operators and `#![deny(clippy::float_arithmetic)]` (face.rs already had it). The
  lint is clean (`cargo clippy -p hst-sim --lib -- -A clippy::all`).
- `tests/float_math.rs`: foot, motion, npc, pose, position, stats and umpire are off PLAIN; face, foot, motion, npc,
  pose and position are off CALLS.
- Allowed host arithmetic, each marked with an `#[allow(clippy::float_arithmetic)]` and a comment:
  - `pose.rs` `mul` / `rotation`: the `first_frame` host approximation (it feeds the pelvis-turn table through
    `hst/src/character.rs` to within 1e-4) and the mods preview `model()`. This isn't game math, so it's left as it
    was to keep gameplay the same.
  - `foot.rs` throw lunge `r.lunge × 0.3` and `npc.rs` slide `−r − 0.1`: the game does these in soft-float double,
    so they stay as f64 arithmetic rounded to nearest.

## Rounding that changed (plain IEEE → EE FPU chop)
Each of these was plain f32 (nearest) before. The game does them on the FPU, so the port now uses `F`:
- `umpire.rs` `reset`: forward = (0 − pos)·1/√(x² + z²). The disassembly shows `sub.s`×3, `mula.s z·z`,
  `madd.s x·x`, `sqrt.s`, `div.s 1/…`, `mul.s`×2. The adder is symmetric, so `x*x + z*z` through `F` matches.
  The look step and eye easing are `(target − eye) × 0.1` in single precision.
- `stats.rs`: serve speed `kmh × 0.9` is single precision in the decompile. Also `rate`, game share, finish average
  and rating, and mph `× 0.62137`. 180 × 0.9 now chops to 161.99998 (the stats screen shows 161 km/h, not 162).
  The unit test `counts` was updated to match.
- `motion.rs` `soft_follow` |v|² (decompile: v[2]² + v[0]² + v[1]², float ops), `serve_walk` stick_x·forward (only
  the sign is used), `stroke_start` / `serve_swing` 8/frames (a division rounds to nearest anyway, so nothing
  changes). `position.rs` `across` p[0]·±1 is exact.

## Proof
| check | result |
|---|---|
| `tools/check.sh -p hst-sim --no-fail-fast` | TESTS PASSED, 251 tests (replay fixtures, umpire_s05 recording) |
| hash_trace 1 game (10702 ticks) vs clean HEAD | differs from tick 3 |
| hash_trace 3 games (14712 ticks) vs clean HEAD | differs from tick 3 |
| same, with only umpire.rs + stats.rs reverted to HEAD | **identical** to baseline (1 and 3 games) |
| umpire.rs alone reverted | first diff moves to line 368 (stats) |

So the hash changes come only from the umpire `reset` forward and from stats.rs. Both are rounding fixes toward
the FPU's chop. Neither touches the sim RNG, ball, players or score; the hash covers them because they're part of
the state. The motion, pose, foot, face, position and npc conversions leave the trace bit-identical.
(Traces: `context/c6/base*.txt`, `new*.txt`, `bis1.txt`, `bis2*.txt`.)

## Pitfalls
- `tools/check.sh -p hst-sim` without `--no-fail-fast` stops at the first failing lib unit test, before the
  integration tests run.
- A baseline build in a /tmp copy of the tree ran into "Disk quota exceeded". Use a tagged stash instead (apply it
  by SHA, then drop it).
- npc.rs was converted with a scratch script and then fixed by hand: a `NEG_PI` const, `-F(..)` for unary minus,
  and the `(F(k) + 1.0)` shapes.

## Not verified / not 1:1
- **hash_trace isn't unchanged** (the task asked for unchanged): umpire forward and stats now chop like the game's
  FPU instead of rounding to nearest. The decompile and disassembly were checked, but not a PINE recording of the
  umpire's forward vector or the stats screen's serve speed (e.g. 161 vs 162 km/h for a 180 km/h serve). PCSX2
  wasn't used for this task.
- `motion.rs` `soft_follow` threshold: chopped |v|² could flip a test exactly at the threshold. Not seen in the
  traces (motion alone leaves them identical), and not checked against a recording.
- `pose.rs` `mul` / `rotation` are still host arithmetic: a known approximation (`first_frame`), not the game's
  bone math.
- foot lunge / npc slide stay as f64 rounded to nearest. That's taken to match the game's soft-float double, but
  wasn't checked bit for bit against the game's double library.

## Wrap-up fix
- The first `tools/done.sh` run hit `attempt to add with overflow` in `stats.rs` `finish` on a result screen with
  no games played (netplay `the_result_screen_leads_back_to_the_lobby`). `ps2::div` gives ±max for x/0 where host
  floats gave NaN/inf, so both averages saturate to i32::MAX. The game's `addu`/`subu` wrap, so
  `avg[0] + avg[1]` and `100 − rating[0]` are now `wrapping_add`/`wrapping_sub`. The screen still ends up at
  51/49 for the winner, as before.
- `tools/check.sh --workspace --no-fail-fast`: 1 failure in the whole workspace,
  `hst play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally`. It lists the umpire mods in `mods/`,
  a symlink to the main checkout, which held only `hst_umpire_04_anna` at the time (changed 22:39 by another
  session). This is the environment, not C6: the test reads no sim float code. So `done.sh`'s gate couldn't pass,
  and its last steps (clippy clean → `ctx.py tick C6` → commit) were run by hand. Re-run that test once `mods/` has
  all five umpires again.
