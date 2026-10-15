//! The game's swept-sphere vs plane test and its contact-point placement, operation for operation.
#![deny(clippy::float_arithmetic)]

use crate::ps2::{F, f};
use crate::vu0::{self, V4};

/// Result of a sweep: fraction along the move (0..1), penetration (negative = start was inside the skin),
/// the contact centre and the plane normal.
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub t: f32,
    pub penetration: f32,
    pub centre: V4,
    pub normal: V4,
}

/// Sweep a sphere of `radius` from `start` to `end` against the plane through `point` with unit `normal`
/// (pointing into the solid side, as the game passes it). `best` is the closest hit so far.
pub fn sweep(start: V4, end: V4, radius: f32, point: V4, normal: V4, best: f32) -> Option<Hit> {
    let r = F(radius);
    let skin = f(f32::from_bits(0x3f80_a3d7)) * r; // 1.005
    let de: V4 = std::array::from_fn(|k| (F(end[k]) - point[k]).0);
    let d_end = F(vu0::dot3(normal, de));
    if !(d_end <= skin) {
        return None;
    }
    let ds: V4 = std::array::from_fn(|k| (F(start[k]) - point[k]).0);
    let d_start = F(vu0::dot3(normal, ds));
    if d_start < -skin {
        return None;
    }
    let travel = d_end - d_start;
    if !(travel <= f(f32::from_bits(0x3ba3_d70a)) * r) {
        return None;
    }
    if r < d_end && r < d_start {
        return None;
    }
    if !(-r <= d_end) && !(-r <= d_start) {
        return None;
    }
    if d_start < f(f32::from_bits(0x3f7a_e148)) * r {
        return None;
    }
    let mut t = if r < d_start {
        let t = (r - d_start) / travel;
        if t <= 0.0 { F::ZERO } else if t <= 1.0 { t } else { F::ONE }
    } else {
        F(-1.0)
    };
    if best <= t {
        return None;
    }
    let mut centre = if t > 0.0 {
        vu0::lerp(end, start, t.0)
    } else if t < 0.0 {
        let push = r - d_start;
        let s = normal.map(|c| F(c) * push);
        [(start[0] + s[0]).0, (start[1] + s[1]).0, (start[2] + s[2]).0, start[3]]
    } else {
        start
    };
    let mut penetration = F::ZERO;
    if t < 0.0 || !(skin <= d_start) {
        penetration = -(skin - d_start) / r;
        t = F::ZERO;
    }
    centre[3] = 1.0;
    Some(Hit { t: t.0, penetration: penetration.0, centre, normal })
}

/// Where the ball ends up for a hit: pushed out along the normal when it started inside the skin, otherwise
/// moved to the hit fraction along the step; axes that did not move are copied. The original computes a
/// held-back fraction max(t − 0.005r/|step|, 0.95t) but only uses it to test for ≤ 0 (the lerp gets t).
pub fn contact_point(hit: Option<&Hit>, start: V4, end: V4, radius: f32) -> V4 {
    let Some(hit) = hit else { return end };
    let mut out = if hit.penetration < 0.0 {
        let d = F(hit.penetration) * radius;
        let v = [F(hit.normal[0]) * d, F(hit.normal[1]) * d, F(hit.normal[2]) * d, F::ONE];
        std::array::from_fn(|k| (start[k] - v[k]).0)
    } else if hit.t == 0.0 {
        start
    } else {
        let dy = F(end[1]) - start[1];
        let dx = F(end[0]) - start[0];
        let dz = F(end[2]) - start[2];
        let len = (F::ZERO + (F::ZERO + dy * dy).madd(dx, dx)).madd(dz, dz).sqrt();
        let back = f(f32::from_bits(0x3ba3_d70a)) * radius / len; // 0.005·r
        let floor = f(f32::from_bits(0x3f73_3333)) * hit.t; // 0.95
        let mut held = F(hit.t) - back;
        if held <= floor {
            held = floor;
        }
        if held <= 0.0 {
            start
        } else {
            let mut p = vu0::lerp(end, start, hit.t);
            for k in 0..3 {
                if start[k] == end[k] {
                    p[k] = start[k];
                }
            }
            p
        }
    };
    out[3] = 1.0;
    out
}
