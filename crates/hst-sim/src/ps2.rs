//! Single-precision arithmetic as the reference emulator (PCSX2) performs the PS2 EE FPU's, so ported game
//! math reproduces recorded values bit for bit: chop rounding, the adder's operand alignment (one guard bit),
//! round-to-nearest divide and square root, denormals as zero. The model lives in `ps2-float`
//! (FIZL0/ps2-float, `fpu`); this is its name in the game crates. Verified bit-exact against the game's runtime
//! shot-parameter table (2912 madd/add/sub/mul results) and every replay.
#![deny(clippy::float_arithmetic)]

pub use ps2_float::fpu::{F, add, chop, div, f, f32s, fs, lerp, madd, msub, mul, sqrt, sub, from_u32 as utof};
