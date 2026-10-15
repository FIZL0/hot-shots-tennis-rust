# C8 — presentation modules on `ps2::F` / `vu0::Vf`

C2 follow-up: `camera.rs`, `cutaway.rs`, `shade.rs`, `effect.rs`, `sound.rs` now carry
`#![deny(clippy::float_arithmetic)]`, use the `F`/`Vf` operators instead of `ps2::add(…)` calls, and are off both
lists in `tests/float_math.rs`. Public fields and signatures stay `f32`. Details: `1-convert-FINAL.md`.
