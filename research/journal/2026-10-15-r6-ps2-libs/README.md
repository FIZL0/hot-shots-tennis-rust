# R6 — the PS2 libraries move to their own repositories

The PS2 arithmetic and the R5900 interpreter existed three times: ours (after R2–R5) and the decomp kits'
`crates/ps2-float` / `crates/ps2-oracle` (FIZL0/ps2-decomp-kit-linux and -windows). Now there is one copy of
each, in its own public MIT repository, used through tag-pinned git dependencies:

| Repository | Holds | Releases |
|---|---|---|
| FIZL0/ps2-float (`../ps2-float`) | EE FPU (`fpu`, `F`, `Acc`), VU0 (`vu`), fdlibm (`libm`, `libm::sce` = the HST SDK's acosf/atan2f/powf), newlib `rand()` / MT19937 (`rand`, optional `serde`) | v0.1.0, v0.2.0 |
| FIZL0/ps2-oracle (`../ps2-oracle`) | R5900 interpreter (`ee`), ISO/ELF loader (`disc`: `Disc`, `Ee::from_program`, `from_ram_file`, `holds`) | v0.1.0, v0.1.1 (on ps2-float v0.2.0) |

Dependents now: opentennis (`hst-sim` → ps2-float v0.2.0 with serde; `hst-oracle` → ps2-oracle v0.1.1), both kits
(`crates/game-sim` → ps2-float v0.2.0; new `crates/kit-oracle` → ps2-oracle v0.1.1 + ps2-float v0.2.0, still
nothing from crates.io). Every AGENT.md (opentennis, both kits, both libraries) says the libraries are ours:
fix/extend there with a test, tag, bump the dependents, never vendor a copy.

## Reconciling

- **libm**: the kit's generic fdlibm port is the base; HST's SDK variants are `libm::sce` (acosf, atan2f, powf
  differ from stock fdlibm; sinf/cosf/tanf/atanf/sqrtf re-exported). C-function arguments go through `arg()`:
  Inf/NaN read as ±max (kit behaviour), denormals stay as they are — the oracle showed the game's sinf/tanf/atanf
  return a denormal argument unchanged (54/20000 mismatches with flushing; test
  `denormal_arguments_come_back_unchanged`). The kit's version flushed them.
- **VU0**: kit's API (`Lane`, `Lanes`, `Vf`) with HST's rounding: `vsqrt` chops (kit's rounded to nearest).
- **rand/MT**: from R4 (`hst_sim::ps2_rand`), new in the kit's world. `hst_sim::rng::Mt` is now a serde newtype
  over `ps2_float::rand::Mt` (`Mt::from_words`, Deref) so play.rs and snapshots are unchanged.
- **Interpreter**: the kit's (any RAM size, scratchpad, more COP2 specials, `Hook`, `written_ranges`) plus
  R5's MMI `madd`/`maddu` and the partial-`lwr` fix. `pcpyud` follows the manual (rd.lo = rs.hi, rd.hi =
  rt.hi): HST's old interpreter had rs/rt swapped; no oracle test depended on it.
- **Game layer stays home**: `hst-oracle/src/hst.rs` has `HstEe` (a trait: `Ee::game()`, `Ee::from_ram(name)`)
  over `ps2_oracle::Ee`; the kits' `kit-oracle` has `KitEe` (`game`, `from_ram`, `check_overlay`), `Kit`
  (kit.toml), `addr`, `skip`, `game_or_skip`, `open_disc` and the `call` binary behind `./kit oracle`.
- **Late upstream change**: while this ran, FIZL0/ps2-decomp-kit-linux got 17041c4 (from openmx F1/F2: rsqrt.s
  chops its root and divide and takes the dividend's sign on a zero divisor, max.s/min.s compare bit patterns,
  `mula`/`suba`/`dot`/`Acc`, 600 lines of integer-model tests). Merged into ps2-float as v0.2.0, ps2-oracle
  bumped to v0.1.1, kit commit rebased on it with the copy deleted.

## Checks

- opentennis: `tools/check.sh -p hst-sim` 243 pass (incl. `round1_fixture_is_frame_exact`, platforms
  x86_64/wasm32 state hash), `-p hst-oracle` 19 pass against the disc (libm, rng, ai, walker, …) — on both v0.1.0
  and v0.2.0/v0.1.1; `cargo check -p hst --tests` clean (only pre-existing warnings).
- ps2-float / ps2-oracle: `cargo test`, clippy `-D warnings`, wasm32 build (ps2-float); both test suites also
  pass as x86_64-pc-windows-gnu under Wine.
- Kit (Linux): `cargo test --workspace`, clippy, `./kit check` (131 self-tests), `./kit demo --check` 12/12
  (oracle bit-exact over 10k inputs on the generated demo disc), `./kit oracle overlay_mix`.
- Kit (Windows): `cargo test --workspace` natively and as x86_64-pc-windows-gnu under Wine, `kit demo --check`
  12/12, Python suites.
