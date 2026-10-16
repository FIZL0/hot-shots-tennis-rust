//! The costume noise deformer (`.NOI`): hair, skirts and ties sway in the wind. Each deformer drives one node's
//! packets: per frame its phase advances by the wind-scaled rate, and the VU1 adds a value-noise offset to every
//! position entry of those packets (in the bone's space, before skinning), scaled by the entry's own weight.
//! EE side in FPU arithmetic (`ps2`), the micro program's in VU arithmetic (`vu0`).
#![deny(clippy::float_arithmetic)]

use crate::ps2::F;
use crate::vu0::Vf;

/// The noise phase wraps at the table's length.
const SPAN: f32 = 32.0;
/// The wind rate: `WIND_BASE` + speed·`WIND_GAIN` (0.8/9).
const WIND_BASE: f32 = 0.2;
const WIND_GAIN: f32 = 0.08888889;
/// A deformer's amplitude and rate scales (0.03/13 ≈ 1/433.3, and 0.1).
const AMP_SCALE: f32 = 0.0023076923;
const RATE_SCALE: f32 = 0.1;
/// The VU random register's seed: the game's π (3.14159012, not π rounded), as RINIT takes it.
const SEED: u32 = 0x4049_0fd0;

/// The global wind rate the deformers scale by: speed·0.0889 + 0.2, set at every game from its wind (`adda 0, 0.2;
/// madd speed, k`). Before the first game it is 1.0.
pub fn wind(speed: f32) -> f32 {
    (F::ZERO + WIND_BASE).madd(WIND_GAIN, speed).0
}

/// One deformer's EE state: the noise frequency, and the phase now and a frame ago (the VU draws with the latter).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Noise {
    pub freq: f32,
    pub phase: f32,
    pub prev: f32,
    pub amp: f32,
    rate: f32,
}

impl Noise {
    /// From the `.NOI` entry's period: 500/period.
    pub fn new(period: f32) -> Self {
        Noise { freq: (500.0 * (1.0 / F(period))).0, phase: 0.0, prev: -1.0, amp: 0.0, rate: 0.0 }
    }

    /// The per-frame update: amplitude and rate from the wind, the previous phase kept, the phase advanced by
    /// `dt`·rate and wrapped into [0, 32). `amp` is the entry's first amplitude (the other two go unread).
    pub fn step(&mut self, dt: f32, rate: f32, amp: f32, wind: f32) {
        self.amp = (wind * (F(AMP_SCALE) * amp)).0;
        self.rate = (wind * (F(RATE_SCALE) * rate)).0;
        self.prev = self.phase;
        self.phase = (F::ZERO + self.phase).madd(dt, self.rate).0;
        if self.phase < SPAN {
            while self.phase < 0.0 {
                self.phase = (F(self.phase) + SPAN).0;
            }
        } else {
            while self.phase >= SPAN {
                self.phase = (F(self.phase) - SPAN).0;
            }
        }
    }

    /// A motion change restarts the deformer: phase 0, one step of `dt`, and no previous phase to lag behind.
    pub fn reset(&mut self, dt: f32, rate: f32, amp: f32, wind: f32) {
        self.phase = 0.0;
        self.step(dt, rate, amp, wind);
        self.prev = self.phase;
    }
}

/// The micro program's noise table, rebuilt the same at every call: the VU random register seeded with π, each
/// value r in [1, 2) mapped to (r + 2)/4, alternately negated.
pub fn table() -> [f32; 32] {
    // R is a float in [1, 2): exponent 0x3f8 over 23 random mantissa bits
    const MANTISSA: u32 = 0x7f_ffff;
    const ONE: u32 = 0x3f80_0000;
    let mut r = SEED & MANTISSA | ONE; // RINIT
    std::array::from_fn(|k| {
        r = (r << 1 ^ (r >> 4 & 1) ^ (r >> 22 & 1)) & MANTISSA | ONE; // RNEXT
        ((Vf(f32::from_bits(r)) + 2.0) * if k % 2 == 0 { 0.25 } else { -0.25 }).0
    })
}

/// One position entry `p` moved by the noise at phase `prev` (the deformer's previous phase), scaled by its
/// weight. Each lane reads the noise of another: x of (z + y), y of (x + z), z of (y + x).
pub fn deform(t: &[f32; 32], p: [f32; 3], weight: f32, freq: f32, prev: f32, amp: f32) -> [f32; 3] {
    let trunc = |x: f32| x.trunc(); // FTOI0 then ITOF0
    let trunc = |x: Vf| Vf(trunc(x.0));
    let s = [Vf(p[0]) + p[2], Vf(p[1]) + p[0], Vf(p[2]) + p[1]];
    let mut floor = [0i32; 3];
    let mut frac = [Vf(0.0); 3];
    for k in 0..3 {
        let x = (0.0 + Vf(prev)).madd(s[k], freq);
        let whole = trunc(x);
        let f = x - whole;
        let g = trunc(f - 1.0); // −1 for f ≤ 0 (so a whole x lands on frac 1), else 0
        floor[k] = (whole + g).0 as i32;
        frac[k] = f - g;
    }
    std::array::from_fn(|k| {
        let j = (k + 2) % 3; // lane x reads z, y reads x, z reads y
        let a = Vf(t[(floor[j] & 31) as usize]);
        let b = t[((Vf(floor[j] as f32) + 1.0).0 as i32 & 31) as usize];
        let n = (a * 1.0).madd(frac[j], b - a);
        (Vf(p[k]) * 1.0).madd(amp * n, weight).0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_are_the_games() {
        assert_eq!([WIND_GAIN, AMP_SCALE, RATE_SCALE].map(f32::to_bits), [0x3db6_0b61, 0x3b17_3ca7, 0x3dcc_cccd]);
    }

    #[test]
    fn table_seeded_with_pi() {
        let t = table();
        assert!(t.iter().step_by(2).all(|&v| (0.75..1.0).contains(&v)));
        assert!(t.iter().skip(1).step_by(2).all(|&v| (-1.0..=-0.75).contains(&v)));
        // RINIT 0x40490fd0 → R 0x3fc90fd0; the first RNEXT shifts in bit 4 (1) ^ bit 22 (1) = 0
        assert_eq!(t[0], ((Vf(f32::from_bits(0x3f92_1fa0)) + 2.0) * 0.25).0);
    }

    #[test]
    fn phase_wraps_and_resets() {
        // slot 5's capture at wind speed 2: wind 0x3ec16c17, rate 1 steps the phase by 0x3d1abcdf
        let w = wind(2.0);
        assert_eq!(w.to_bits(), 0x3ec1_6c17);
        let mut n = Noise::new(30.0);
        n.reset(0.0, 1.0, 2.0, w);
        assert_eq!((n.phase, n.prev), (0.0, 0.0));
        n.step(1.0, 1.0, 2.0, w);
        assert_eq!((n.prev, n.phase.to_bits()), (0.0, 0x3d1a_bcdf));
        assert_eq!(n.amp.to_bits(), 0x3ae4_892a);
        for _ in 0..2000 {
            n.step(1.0, 1.0, 2.0, w);
            assert!((0.0..SPAN).contains(&n.phase));
        }
    }

    #[test]
    fn weight_zero_leaves_the_entry() {
        let t = table();
        let p = [1.5, -2.25, 0.125];
        assert_eq!(deform(&t, p, 0.0, 16.6, 3.0, 0.004), p);
        let q = deform(&t, p, 1.0, 16.6, 3.0, 0.004);
        assert!(q.iter().zip(p).all(|(a, b)| (a - b).abs() <= 0.004 && a != &b));
    }
}
