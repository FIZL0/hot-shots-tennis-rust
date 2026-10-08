//! Sky clouds (singles only): flat quads scaled 40 scattered in a 2R square 200..400 m up (heavy rain: 200 of them
//! 80..200 m up, `weather::cloud_layout`), drifting with the court's wind and wrapping around; each fades out over
//! the outer tenth of the radius.

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

/// `count` clouds over `models` cloud models between heights `y` (game space, ×40 cloud units); `rand` gives 0..1.
pub fn spawn(count: usize, models: usize, y: [f32; 2], mut rand: impl FnMut() -> f32) -> Vec<Cloud> {
    let mut v: Vec<Cloud> = (0..count)
        .map(|_| {
            let y = y[0] + (y[1] - y[0]) * rand();
            let (x, z) = ((2.0 * rand() - 1.0) * R, (2.0 * rand() - 1.0) * R);
            Cloud { pos: [x, y, z], yaw: rand() * std::f32::consts::TAU, model: ((rand() * models as f32) as usize).min(models.saturating_sub(1)) }
        })
        .collect();
    v.sort_by(|a, b| a.pos[1].total_cmp(&b.pos[1]));
    v
}

/// One 60 Hz tick: drift towards `degrees` at `speed`, wrapping x and z into ±R.
pub fn tick(clouds: &mut [Cloud], degrees: f32, speed: f32) {
    let (s, c) = degrees.to_radians().sin_cos();
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
        let mut n = 0u32;
        let v = spawn(11, 3, [-200.0, -400.0], || { n += 1; (n % 7) as f32 / 7.0 });
        assert!(v.len() == 11 && v.windows(2).all(|w| w[0].pos[1] <= w[1].pos[1]) && v.iter().all(|k| k.model < 3));
    }
}
