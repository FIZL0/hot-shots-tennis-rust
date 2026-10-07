//! Where a positional sound plays from: the sound library places every game sound against a fixed listener behind
//! the near baseline, attenuates it with distance on the floor plane and splits it into the sequence's left and
//! right volume by the bearing.

use crate::libm::atan2f;
use crate::ps2::{add, div, madd, msub, mul, sqrt, sub};

/// The listener: identity orientation at (0, 0, −10) in game space (the library's fixed listener, the only mode
/// the game uses).
const LISTENER: [f32; 3] = [0.0, 0.0, -10.0];
const PI: f32 = f32::from_bits(0x4049_0fdb);
const DEGREES: f32 = f32::from_bits(0x4265_2ee1); // 57.29578
/// Distances (whole metres) are clamped to this; it is also where a sound falls silent.
const FAR: i32 = 128;
/// Full volume up to here.
const NEAR: f32 = 10.0;

/// Bearing from the listener in whole degrees (0..359, 0 straight ahead along +z) and floor distance in whole
/// metres (0..128) of a sound at `pos`.
pub fn place(pos: [f32; 3]) -> (i32, i32) {
    let (dx, dz) = (sub(pos[0], LISTENER[0]), sub(pos[2], LISTENER[2]));
    let yaw = 0.0; // atan2 of the listener's identity forward axis
    let a = msub(mul(add(PI, atan2f(dx, dz)), DEGREES), add(PI, yaw), DEGREES) as i32;
    let a = if a < 0 { a + 360 } else { a };
    let d = sqrt(madd(mul(dz, dz), dx, dx)) as i32;
    (a.clamp(0, 359), d.clamp(0, FAR))
}

/// `volume` (0..128) at `dist` metres: full up to 10 m, then linearly down to nothing at 128 m.
pub fn falloff(volume: i32, dist: i32) -> i32 {
    let v = volume as f32;
    let v = if dist as f32 <= NEAR {
        v
    } else if dist > FAR {
        0.0
    } else {
        let span = sub(FAR as f32, NEAR);
        let step = if span == 0.0 { 0.0 } else { div(v, span) };
        msub(add(v, 0.0), step, sub(dist as f32, NEAR))
    };
    (v as i32).clamp(0, 128)
}

/// The sequence's left and right volume for `volume` at bearing `angle`, from `exe::stereo_tables`. The tables'
/// signs (phase inversion) only apply with the library's surround mode, which the game leaves off.
pub fn stereo(volume: i32, angle: i32, tables: &[Vec<i32>; 2]) -> [i32; 2] {
    let i = ((angle + 90) % 720) as usize;
    tables.each_ref().map(|t| {
        let g = if t[i] == -4096 { 0 } else { t[i].abs() };
        (volume * g) >> 11
    })
}
