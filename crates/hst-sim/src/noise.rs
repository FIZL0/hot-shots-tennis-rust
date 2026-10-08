//! The costume noise deformer (`.NOI`): hair, skirts and ties sway in the wind. Each deformer drives one node's
//! packets: per frame its phase advances by the wind-scaled rate, and the VU1 adds a value-noise offset to every
//! position entry of those packets (in the bone's space, before skinning), scaled by the entry's own weight.
//! EE side in FPU arithmetic (`ps2`), the micro program's in VU arithmetic (`vu0`).

use crate::{ps2, vu0};

/// The noise phase wraps at the table's length.
const SPAN: f32 = 32.0;

/// The global wind rate the deformers scale by: speed·0.0889 + 0.2, set at every game from its wind (`adda 0, 0.2;
/// madd speed, k`). Before the first game it is 1.0.
pub fn wind(speed: f32) -> f32 {
    ps2::madd(ps2::add(0.0, 0.2), f32::from_bits(0x3db6_0b61), speed)
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
        Noise { freq: ps2::mul(500.0, ps2::div(1.0, period)), phase: 0.0, prev: -1.0, amp: 0.0, rate: 0.0 }
    }

    /// The per-frame update: amplitude and rate from the wind, the previous phase kept, the phase advanced by
    /// `dt`·rate and wrapped into [0, 32). `amp` is the entry's first amplitude (the other two go unread).
    pub fn step(&mut self, dt: f32, rate: f32, amp: f32, wind: f32) {
        self.amp = ps2::mul(wind, ps2::mul(f32::from_bits(0x3b17_3ca7), amp)); // amp / 433.3
        self.rate = ps2::mul(wind, ps2::mul(f32::from_bits(0x3dcc_cccd), rate));
        self.prev = self.phase;
        self.phase = ps2::madd(ps2::add(0.0, self.phase), dt, self.rate);
        if self.phase < SPAN {
            while self.phase < 0.0 {
                self.phase = ps2::add(self.phase, SPAN);
            }
        } else {
            while self.phase >= SPAN {
                self.phase = ps2::sub(self.phase, SPAN);
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
    let mut r = 0x4049_0fd0u32 & 0x7f_ffff | 0x3f80_0000; // RINIT with the game's π, 3.14159012
    std::array::from_fn(|k| {
        r = (r << 1 ^ (r >> 4 & 1) ^ (r >> 22 & 1)) & 0x7f_ffff | 0x3f80_0000; // RNEXT
        vu0::mul(vu0::add(f32::from_bits(r), 2.0), if k % 2 == 0 { 0.25 } else { -0.25 })
    })
}

/// One position entry `p` moved by the noise at phase `prev` (the deformer's previous phase), scaled by its
/// weight. Each lane reads the noise of another: x of (z + y), y of (x + z), z of (y + x).
pub fn deform(t: &[f32; 32], p: [f32; 3], weight: f32, freq: f32, prev: f32, amp: f32) -> [f32; 3] {
    let trunc = |x: f32| x.trunc(); // FTOI0 then ITOF0
    let s = [vu0::add(p[0], p[2]), vu0::add(p[1], p[0]), vu0::add(p[2], p[1])];
    let mut floor = [0i32; 3];
    let mut frac = [0f32; 3];
    for k in 0..3 {
        let x = vu0::madd(vu0::add(0.0, prev), s[k], freq);
        let whole = trunc(x);
        let f = vu0::sub(x, whole);
        let g = trunc(vu0::sub(f, 1.0)); // −1 for f ≤ 0 (so a whole x lands on frac 1), else 0
        floor[k] = vu0::add(whole, g) as i32;
        frac[k] = vu0::sub(f, g);
    }
    std::array::from_fn(|k| {
        let j = (k + 2) % 3; // lane x reads z, y reads x, z reads y
        let a = t[(floor[j] & 31) as usize];
        let b = t[(vu0::add(floor[j] as f32, 1.0) as i32 & 31) as usize];
        let n = vu0::madd(vu0::mul(a, 1.0), frac[j], vu0::sub(b, a));
        vu0::madd(vu0::mul(p[k], 1.0), vu0::mul(amp, n), weight)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_seeded_with_pi() {
        let t = table();
        assert!(t.iter().step_by(2).all(|&v| (0.75..1.0).contains(&v)));
        assert!(t.iter().skip(1).step_by(2).all(|&v| (-1.0..=-0.75).contains(&v)));
        // RINIT 0x40490fd0 → R 0x3fc90fd0; the first RNEXT shifts in bit 4 (1) ^ bit 22 (1) = 0
        assert_eq!(t[0], vu0::mul(vu0::add(f32::from_bits(0x3f92_1fa0), 2.0), 0.25));
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
