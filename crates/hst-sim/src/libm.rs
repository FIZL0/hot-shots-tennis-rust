//! The C library's `sinf`, `cosf`, `tanf`, `atanf`, `atan2f`, `acosf` and `powf` as Hot Shots Tennis links them:
//! the SDK's newlib build of fdlibm's float flavour, on EE FPU arithmetic. The port lives in `ps2-float`
//! (FIZL0/ps2-float, `libm::sce`: that SDK's constants and `powf` seed); each function is checked against the
//! game's own over 20k inputs (`crates/hst-oracle/tests/libm.rs`).
#![deny(clippy::float_arithmetic)]

pub use ps2_float::libm::sce::{acosf, atan2f, atanf, cosf, powf, sinf, tanf};
