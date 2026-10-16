//! The court's lines, in metres from the centre mark (game space: x across, z along the court, the net at
//! z = 0), and the degree-to-radian factor the shot code multiplies by. The values are the game's f32 literals;
//! derived distances stay literals too (an f32 subtraction can round differently from the game's constant).
#![deny(clippy::float_arithmetic)]

/// The baseline's distance from the net (half the court's length).
pub const BASELINE: f32 = 11.885;
/// The court's length, baseline to baseline.
pub const LENGTH: f32 = 23.77;
/// The service line's distance from the net.
pub const SERVICE_LINE: f32 = 6.4;
/// The singles and doubles sidelines' distance from the centre line.
pub const SINGLES_SIDELINE: f32 = 4.115;
pub const DOUBLES_SIDELINE: f32 = 5.485;
/// The middle of a service box across (half the singles width).
pub const BOX_MIDDLE: f32 = 2.0575;

/// π/180 as the game's f32 (0x3c8e_fa35).
pub const DEG: f32 = 0.017453292;

/// The sideline in play: the doubles alleys count with `doubles`.
pub fn sideline(doubles: bool) -> f32 {
    if doubles { DOUBLES_SIDELINE } else { SINGLES_SIDELINE }
}
