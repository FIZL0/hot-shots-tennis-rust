# C4 — readable float math: ball and world modules

C2 follow-up (rules in `plan/C2.md`). Converted to `ps2::F` / `vu0::Vf` operators with
`#![deny(clippy::float_arithmetic)]`, dropped from both lists in `crates/hst-sim/tests/float_math.rs`:
`contact mesh world quat bodyhit tornado clouds weather noise court_anim` (the last two already carried the deny
attribute and only lose their hand calls). Iteration: `1-convert-FINAL.md`.
