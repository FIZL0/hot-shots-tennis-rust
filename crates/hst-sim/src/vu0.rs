//! VU0 arithmetic as the reference emulator performs it (macro mode and micro programs): every operation is
//! rounded toward zero, with no add/sub operand alignment step (that is an FPU quirk), and multiply-add is a
//! rounded product followed by a rounded add. The model lives in `ps2-float` (FIZL0/ps2-float, `vu`); `Vf` here
//! is its one-lane `Lane`.
#![deny(clippy::float_arithmetic)]

pub use ps2_float::vu::{
    Lane as Vf, Lanes as Vfs, V4, add, cross, div, dot3, lanes as vf, lerp, madd, msub, mul, normalize, sqrt, sub,
    transform, v4,
};
