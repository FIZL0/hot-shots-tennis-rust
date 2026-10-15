//! The game's rotation helpers, operation for operation: matrix → quaternion and quaternion → matrix on the
//! FPU (`ps2`), and quaternion slerp whose core is a VU0 micro program (`vu0`).
//! Matrices are 4 rows of 4 (row-major, rows are the basis vectors); quaternions are (x, y, z, w).

#![deny(clippy::float_arithmetic)]

use crate::ps2::{F, f32s, fs};
use crate::vu0::{V4, Vf, v4, vf};

pub type M4 = [V4; 4];

/// Largest-diagonal matrix → quaternion.
pub fn from_matrix(m: &M4) -> V4 {
    let m = m.map(fs);
    let (m00, m11, m22) = (m[0][0], m[1][1], m[2][2]);
    let s01 = m00;
    let d01 = s01 + m11;
    let tr = m22 + d01;
    if !(tr < 0.0) {
        let s = (F::ONE + tr).sqrt();
        let f = 0.5 / s;
        return f32s([f * (m[2][1] - m[1][2]), f * (m[0][2] - m[2][0]), f * (m[1][0] - m[0][1]), 0.5 * s]);
    }
    let big = if m00 <= m11 { m11 } else { m00 };
    let big = if big <= m22 { m22 } else { big };
    f32s(if big == m00 {
        let s = (F::ONE + (m00 - (m11 + m22))).sqrt();
        let f = 0.5 / s;
        [0.5 * s, f * (m[0][1] + m[1][0]), f * (m[2][0] + m[0][2]), f * (m[2][1] - m[1][2])]
    } else if big == m11 {
        let s = (F::ONE + (m11 - (m22 + m00))).sqrt();
        let f = 0.5 / s;
        [f * (m[0][1] + m[1][0]), 0.5 * s, f * (m[1][2] + m[2][1]), f * (m[0][2] - m[2][0])]
    } else {
        let s = (F::ONE + (m22 - d01)).sqrt();
        let f = 0.5 / s;
        [f * (m[2][0] + m[0][2]), f * (m[1][2] + m[2][1]), 0.5 * s, f * (m[1][0] - m[0][1])]
    })
}

/// FPU normalize of xyz, scaling all four components by the inverse length.
fn fpu_normalize(v: [F; 4]) -> [F; 4] {
    let s = (v[1] * v[1]).madd(v[0], v[0]).madd(v[2], v[2]).sqrt();
    let inv = 1.0 / s;
    [v[0] * inv, v[1] * inv, v[2] * inv, v[3] * inv]
}

fn fpu_cross(a: [F; 4], b: [F; 4]) -> [F; 4] {
    [(a[1] * b[2]).msub(a[2], b[1]), (a[2] * b[0]).msub(a[0], b[2]), (a[0] * b[1]).msub(a[1], b[0]), F::ZERO]
}

/// Quaternion → rotation matrix, then re-orthonormalized (row 0, row 2 = row 0 × row 1, row 1 = row 2 × row 0).
pub fn to_matrix(q: V4) -> M4 {
    let [x, y, z, w] = fs(q);
    let n = (y * y).madd(x, x).madd(z, z).madd(w, w);
    let s = if n <= 0.0 { F::ZERO } else { 2.0 / n };
    let (sx, sy, sz) = (x * s, y * s, z * s);
    let zz = z * sz;
    let (wx, xx, wy, wz) = (w * sx, x * sx, w * sy, w * sz);
    let (xy, yy, xz, yz) = (x * sy, y * sy, x * sz, y * sz);
    let row0 = [1.0 - (yy + zz), xy - wz, xz + wy, F::ZERO];
    let row1 = [xy + wz, 1.0 - (xx + zz), yz - wx, F::ZERO];
    let n0 = fpu_normalize(row0);
    let n2 = fpu_normalize(fpu_cross(n0, row1));
    let n1 = fpu_normalize(fpu_cross(n2, n0));
    [f32s(n0), f32s(n1), f32s(n2), [0.0, 0.0, 0.0, 1.0]]
}

/// Quaternion slerp from `a` (t = 0) to `b` (t = 1).
pub fn slerp(a: V4, b: V4, t: f32) -> V4 {
    if t == 1.0 {
        return b;
    }
    if t == 0.0 {
        return a;
    }
    let (fa, fb) = (fs(a), fs(b));
    let d = (F::ZERO + (F::ZERO + (F::ZERO + fa[1] * fb[1]).madd(fa[0], fb[0])).madd(fa[2], fb[2])).madd(fa[3], fb[3]);
    let a = if d < 0.0 { f32s(fa.map(|c| -c)) } else { a };
    micro_slerp(a, b, t)
}

/// The VU0 micro program the game calls for slerp (θ via an arctan polynomial around π/4, sines via a
/// polynomial), with the Q-register latencies of the original instruction schedule. Called bare (no shortest-arc
/// flip, no end shortcuts) by the motion sampler's squad.
pub fn micro_slerp(q1: V4, q2: V4, t: f32) -> V4 {
    let (q1, q2, t) = (vf(q1), vf(q2), Vf(t));
    let p = [q1[0] * q2[0], q1[1] * q2[1], q1[2] * q2[2], q1[3] * q2[3]];
    let dot = (p[3] + p[2]).madd(1.0, p[1]).madd(1.0, p[0]);
    let one_minus_t = 1.0 - t;
    let sin2 = msub_w(Vf(1.0) + 0.0, dot);
    // flags: negative dot, sin²θ == 0 or dot > 0.999 fall back to a plain lerp
    let lerp_only = dot < 0.0 || sin2 == 0.0 || dot - 0.999_000_013 > 0.0;
    let (w1, w2, s1, s2) = if lerp_only {
        (one_minus_t, t, q1, q2)
    } else {
        let c = Vf(dot.0.abs());
        let sin = sin2.sqrt();
        let num = 0.0 - c + sin; // sinθ − cosθ
        let den = c + sin; // cosθ + sinθ
        let u = num * (1.0 / den); // tan(θ − π/4)
        let inv_sin = 1.0 / (0.0 + sin);
        let theta = atan_poly(u);
        let ang = [t * theta, one_minus_t * theta];
        let sines = ang.map(sin_poly);
        let s1 = q1.map(|c| c * inv_sin);
        let s2 = q2.map(|c| c * inv_sin);
        (sines[1], sines[0], s1, s2)
    };
    v4(std::array::from_fn(|k| (s1[k] * w1).madd(s2[k], w2)))
}

/// `msub.w`: 1 − dot·dot with the accumulator preloaded to 1.
fn msub_w(acc: Vf, dot: Vf) -> Vf {
    acc.msub(dot, dot)
}

/// θ = π/4 + atan(u), the micro program's odd polynomial in u (terms u¹ … u¹⁵).
fn atan_poly(u: Vf) -> Vf {
    let f = |b: u32| f32::from_bits(b);
    let u2 = u * u;
    let u3 = u2 * u;
    let u4 = u2 * u2;
    let u5 = u3 * u2;
    let u7 = u4 * u3;
    let u9 = u5 * u4;
    let u11 = u7 * u4;
    let u13 = u9 * u4;
    let u15 = u11 * u4;
    let acc = add0(f(0x3f49_0fdb)).madd(u, f(0x3f7f_fff5));
    let acc = acc.madd(u3, f(0xbeaa_a61c));
    let acc = acc.madd(u5, f(0x3e4c_40a6));
    let acc = acc.madd(u7, f(0xbe0e_6c63));
    let acc = acc.madd(u9, f(0x3dc5_77df));
    let acc = acc.madd(u11, f(0xbd65_01c4));
    let acc = acc.madd(u13, f(0x3cb3_1652));
    acc.madd(u15, f(0xbb84_d7e7))
}

fn add0(x: f32) -> Vf {
    0.0 + Vf(x)
}

/// sin(x) as the micro program's odd polynomial (x¹ … x⁹).
fn sin_poly(x: Vf) -> Vf {
    let f = |b: u32| f32::from_bits(b);
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x2 * x2;
    let x5 = x3 * x2;
    let x7 = x4 * x3;
    let x9 = x5 * x4;
    let acc = x * 1.0;
    let acc = acc.madd(x3, f(0xbe2a_aaa4));
    let acc = acc.madd(x5, f(0x3c08_873e));
    let acc = acc.madd(x7, f(0xb94f_b21f));
    acc.madd(x9, f(0x362e_9c14))
}
