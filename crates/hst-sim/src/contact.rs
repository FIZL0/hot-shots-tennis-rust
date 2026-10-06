//! The game's swept-sphere vs plane test and its contact-point placement, operation for operation.

use crate::ps2;
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
    use ps2::{div, mul, sub};
    let skin = mul(f32::from_bits(0x3f80_a3d7), radius); // 1.005
    let de: V4 = std::array::from_fn(|k| sub(end[k], point[k]));
    let d_end = vu0::dot3(normal, de);
    if !(d_end <= skin) {
        return None;
    }
    let ds: V4 = std::array::from_fn(|k| sub(start[k], point[k]));
    let d_start = vu0::dot3(normal, ds);
    if d_start < -skin {
        return None;
    }
    let travel = sub(d_end, d_start);
    if !(travel <= mul(f32::from_bits(0x3ba3_d70a), radius)) {
        return None;
    }
    if radius < d_end && radius < d_start {
        return None;
    }
    if !(-radius <= d_end) && !(-radius <= d_start) {
        return None;
    }
    if d_start < mul(f32::from_bits(0x3f7a_e148), radius) {
        return None;
    }
    let mut t = if radius < d_start {
        let t = div(sub(radius, d_start), travel);
        if t <= 0.0 { 0.0 } else if t <= 1.0 { t } else { 1.0 }
    } else {
        -1.0
    };
    if best <= t {
        return None;
    }
    let mut centre = if t > 0.0 {
        vu0::lerp(end, start, t)
    } else if t < 0.0 {
        let push = sub(radius, d_start);
        let s = normal.map(|c| mul(c, push));
        [ps2::add(start[0], s[0]), ps2::add(start[1], s[1]), ps2::add(start[2], s[2]), start[3]]
    } else {
        start
    };
    let mut penetration = 0.0;
    if t < 0.0 || !(skin <= d_start) {
        penetration = div(-sub(skin, d_start), radius);
        t = 0.0;
    }
    centre[3] = 1.0;
    Some(Hit { t, penetration, centre, normal })
}

/// Where the ball ends up for a hit: pushed out along the normal when it started inside the skin, otherwise
/// moved to the hit fraction along the step; axes that did not move are copied. The original computes a
/// held-back fraction max(t − 0.005r/|step|, 0.95t) but only uses it to test for ≤ 0 (the lerp gets t).
pub fn contact_point(hit: Option<&Hit>, start: V4, end: V4, radius: f32) -> V4 {
    use ps2::{div, madd, mul, sqrt, sub};
    let Some(hit) = hit else { return end };
    let mut out = if hit.penetration < 0.0 {
        let d = mul(hit.penetration, radius);
        let v = [mul(hit.normal[0], d), mul(hit.normal[1], d), mul(hit.normal[2], d), 1.0];
        std::array::from_fn(|k| sub(start[k], v[k]))
    } else if hit.t == 0.0 {
        start
    } else {
        let dy = sub(end[1], start[1]);
        let dx = sub(end[0], start[0]);
        let dz = sub(end[2], start[2]);
        let len = sqrt(madd(ps2::add(0.0, madd(ps2::add(0.0, mul(dy, dy)), dx, dx)), dz, dz));
        let back = div(mul(f32::from_bits(0x3ba3_d70a), radius), len); // 0.005·r
        let floor = mul(f32::from_bits(0x3f73_3333), hit.t); // 0.95
        let mut f = sub(hit.t, back);
        if f <= floor {
            f = floor;
        }
        if f <= 0.0 {
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
