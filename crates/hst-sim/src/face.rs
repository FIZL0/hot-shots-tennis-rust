//! Face tracks: per motion, a `.MOR` file of morph-target weights and a `.UVA` file of texture UV offsets. The
//! motion setter selects the face of the same index (team reactions 0x30.. add the player's co offset; 0x1c/0x1d
//! keep the previous face) and starts its clock with the motion's speed and looping; each frame the face clock
//! wraps (looping) or clamps into its own length — the last key over all tracks, in frames — samples every track
//! at time × ticks per frame, then adds the speed, as [`crate::motion::Clock`] with that length.

use crate::ps2::{add, div, mul, sub};

/// A track's value at tick `t`: linear between keys, ramping from 0 to the first key, held after the last.
/// `cursor` is the key the last sample used (the search walks from it, so a tick that lands on a key takes the
/// segment it comes from); `uva` picks the UV sampler's ramp (t·(1/t₀) instead of v·t/t₀).
pub fn sample(ticks: &[i32], values: &[[f32; 4]], t: f32, cursor: &mut usize, uva: bool) -> [f32; 4] {
    let n = ticks.len().min(values.len());
    if n == 0 {
        *cursor = 0;
        return [0.0; 4];
    }
    if ticks[n - 1] as f32 <= t {
        *cursor = n - 1;
        return values[n - 1];
    }
    let t0 = ticks[0] as f32;
    if t < t0 {
        *cursor = 0;
        return if uva { values[0].map(|v| mul(mul(v, t), div(1.0, t0))) } else { values[0].map(|v| div(mul(v, t), t0)) };
    }
    let mut k = (*cursor).min(n - 1) as isize;
    while k >= 0 && t < ticks[k as usize] as f32 {
        k -= 1;
    }
    let mut k = k.max(0) as usize;
    while k < n - 1 && t > ticks[k + 1] as f32 {
        k += 1;
    }
    *cursor = k;
    let (a, b) = (values[k], values[k + 1]);
    let f = div(sub(t, ticks[k] as f32), (ticks[k + 1] - ticks[k]) as f32);
    std::array::from_fn(|c| add(a[c], mul(f, sub(b[c], a[c]))))
}

/// The face clock's length in frames: the last key over all tracks over the ticks per frame.
pub fn length<'a>(last_ticks: impl IntoIterator<Item = &'a [i32]>, ticks_per_frame: i32) -> f32 {
    let last = last_ticks.into_iter().filter_map(|t| t.last().copied()).fold(0, i32::max);
    div(last as f32, ticks_per_frame as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blink() {
        // blink_eye of a standing motion: shut at frame 3, open at 6, still until 120
        let (ticks, values) = ([0, 240, 480, 9600], [[0.0; 4], [1.0, 0.0, 0.0, 0.0], [0.0; 4], [0.0; 4]]);
        let mut cur = 0;
        let w: Vec<f32> = (0..8).map(|f| sample(&ticks, &values, f as f32 * 80.0, &mut cur, false)[0]).collect();
        // on the way down 1 + (2/3)·(0 − 1) rounds below 1/3
        assert_eq!(w, [0.0, 0.33333334, 0.6666667, 1.0, 0.6666667, 0.3333333, 0.0, 0.0]);
        assert_eq!(length([&ticks[..]], 80), 120.0);
    }
}
