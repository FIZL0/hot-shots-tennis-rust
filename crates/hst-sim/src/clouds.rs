//! Sky clouds (drawn in singles only; made in doubles too): flat quads scaled 40 scattered in a 2R square 200..400 m up (heavy rain: 200 of them
//! 80..200 m up, `weather::cloud_layout`), drifting with the court's wind and wrapping around; each fades out over
//! the outer tenth of the radius.

use crate::ps2;

const R: f32 = 2000.0;

#[derive(Clone, Copy, Debug)]
pub struct Cloud {
    /// Game space (Y-down), relative to the camera.
    pub pos: [f32; 3],
    /// Turn about Y (the model is then flipped by a half-turn about X).
    pub yaw: f32,
    /// Which of the court's cloud models.
    pub model: usize,
}

/// Lays the clouds out from their `rand()` draws (`Rngs::clouds`: 5 each, in this order): the heading, z, y, x and
/// the model (`% models`). Each float is the draw's top 20 bits × 2⁻³¹ scaled into its range as the game does
/// (`lo + (hi − lo)·2⁻³¹·t`); `y` is the lowest and highest y in game units. Kept sorted by y (the game inserts each
/// before the first higher one).
pub fn spawn(draws: &[u32], models: usize, y: [f32; 2]) -> Vec<Cloud> {
    let unit = |t: u32| ps2::utof(t & 0xffff_f800);
    let at = |lo: f32, hi: f32, t: u32| ps2::madd(ps2::add(0.0, lo), ps2::mul(4.656_613e-10, ps2::sub(hi, lo)), unit(t));
    let mut v: Vec<Cloud> = draws
        .chunks_exact(5)
        .map(|d| Cloud {
            yaw: ps2::mul(6.283_185_5, ps2::mul(4.656_613e-10, unit(d[0]))),
            pos: [at(-R, R, d[3]), at(y[0], y[1], d[2]), at(-R, R, d[1])],
            model: d[4] as usize % models.max(1),
        })
        .collect();
    v.sort_by(|a, b| a.pos[1].total_cmp(&b.pos[1]));
    v
}

/// One 60 Hz tick: drift towards `degrees` at `speed`, wrapping x and z into ±R.
pub fn tick(clouds: &mut [Cloud], degrees: f32, speed: f32) {
    let (s, c) = musl::sincosf(degrees.to_radians());
    let step = speed / 600.0 * 40.0;
    let wrap = |v: f32| if v > R { v - 2.0 * R } else if v < -R { v + 2.0 * R } else { v };
    for k in clouds {
        k.pos[0] = wrap(k.pos[0] + step * s);
        k.pos[2] = wrap(k.pos[2] + step * c);
    }
}

/// Opacity 0..1: full inside 0.9R, none at R.
pub fn fade(k: &Cloud) -> f32 {
    let r2 = k.pos[0] * k.pos[0] + k.pos[2] * k.pos[2];
    ((R * R - r2) / (R * R - 0.81 * R * R)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drift_wrap_fade() {
        // slot 5 (court 10, wind 315°, speed 2): −0.0943 x, +0.0943 z per tick
        let mut k = [Cloud { pos: [0.0, -300.0, 0.0], yaw: 0.0, model: 0 }];
        tick(&mut k, 315.0, 2.0);
        assert!((k[0].pos[0] + 0.0943).abs() < 1e-4 && (k[0].pos[2] - 0.0943).abs() < 1e-4);
        k[0].pos[0] = -1999.95;
        tick(&mut k, 315.0, 2.0);
        assert!(k[0].pos[0] > 1999.0);
        // the game read 0.39 at r ≈ 1924
        k[0].pos = [1924.0, -300.0, 0.0];
        assert!((fade(&k[0]) - 0.39).abs() < 0.02);
        let draws: Vec<u32> = (0..55u32).map(|n| n.wrapping_mul(0x2f1a_9e37) >> 1).collect();
        let v = spawn(&draws, 3, [-200.0, -400.0]);
        assert!(v.len() == 11 && v.windows(2).all(|w| w[0].pos[1] <= w[1].pos[1]) && v.iter().all(|k| k.model < 3));
        assert!(v.iter().all(|k| (-400.0..=-200.0).contains(&k.pos[1]) && k.pos[0].abs() <= R && k.pos[2].abs() <= R));
    }
}
