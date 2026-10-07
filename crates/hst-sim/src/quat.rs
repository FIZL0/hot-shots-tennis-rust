//! The game's rotation helpers, operation for operation: matrix → quaternion and quaternion → matrix on the
//! FPU (`ps2`), and quaternion slerp whose core is a VU0 micro program (`vu0`).
//! Matrices are 4 rows of 4 (row-major, rows are the basis vectors); quaternions are (x, y, z, w).

use crate::ps2;
use crate::vu0::{self, V4};

pub type M4 = [V4; 4];

/// Largest-diagonal matrix → quaternion.
pub fn from_matrix(m: &M4) -> V4 {
    use ps2::{add, div, mul, sqrt, sub};
    let (m00, m11, m22) = (m[0][0], m[1][1], m[2][2]);
    let s01 = m00;
    let d01 = add(s01, m11);
    let tr = add(m22, d01);
    if !(tr < 0.0) {
        let s = sqrt(add(1.0, tr));
        let f = div(0.5, s);
        return [mul(f, sub(m[2][1], m[1][2])), mul(f, sub(m[0][2], m[2][0])), mul(f, sub(m[1][0], m[0][1])), mul(0.5, s)];
    }
    let big = if m00 <= m11 { m11 } else { m00 };
    let big = if big <= m22 { m22 } else { big };
    if big == m00 {
        let s = sqrt(add(1.0, sub(m00, add(m11, m22))));
        let f = div(0.5, s);
        [mul(0.5, s), mul(f, add(m[0][1], m[1][0])), mul(f, add(m[2][0], m[0][2])), mul(f, sub(m[2][1], m[1][2]))]
    } else if big == m11 {
        let s = sqrt(add(1.0, sub(m11, add(m22, m00))));
        let f = div(0.5, s);
        [mul(f, add(m[0][1], m[1][0])), mul(0.5, s), mul(f, add(m[1][2], m[2][1])), mul(f, sub(m[0][2], m[2][0]))]
    } else {
        let s = sqrt(add(1.0, sub(m22, d01)));
        let f = div(0.5, s);
        [mul(f, add(m[2][0], m[0][2])), mul(f, add(m[1][2], m[2][1])), mul(0.5, s), mul(f, sub(m[1][0], m[0][1]))]
    }
}

/// FPU normalize of xyz, scaling all four components by the inverse length.
fn fpu_normalize(v: V4) -> V4 {
    use ps2::{div, madd, mul, sqrt};
    let s = sqrt(madd(madd(mul(v[1], v[1]), v[0], v[0]), v[2], v[2]));
    let inv = div(1.0, s);
    [mul(v[0], inv), mul(v[1], inv), mul(v[2], inv), mul(v[3], inv)]
}

fn fpu_cross(a: V4, b: V4) -> V4 {
    use ps2::{msub, mul};
    [msub(mul(a[1], b[2]), a[2], b[1]), msub(mul(a[2], b[0]), a[0], b[2]), msub(mul(a[0], b[1]), a[1], b[0]), 0.0]
}

/// Quaternion → rotation matrix, then re-orthonormalized (row 0, row 2 = row 0 × row 1, row 1 = row 2 × row 0).
pub fn to_matrix(q: V4) -> M4 {
    use ps2::{add, div, madd, mul, sub};
    let [x, y, z, w] = q;
    let n = madd(madd(madd(mul(y, y), x, x), z, z), w, w);
    let s = if n <= 0.0 { 0.0 } else { div(2.0, n) };
    let (sx, sy, sz) = (mul(x, s), mul(y, s), mul(z, s));
    let zz = mul(z, sz);
    let (wx, xx, wy, wz) = (mul(w, sx), mul(x, sx), mul(w, sy), mul(w, sz));
    let (xy, yy, xz, yz) = (mul(x, sy), mul(y, sy), mul(x, sz), mul(y, sz));
    let row0 = [sub(1.0, add(yy, zz)), sub(xy, wz), add(xz, wy), 0.0];
    let row1 = [add(xy, wz), sub(1.0, add(xx, zz)), sub(yz, wx), 0.0];
    let n0 = fpu_normalize(row0);
    let n2 = fpu_normalize(fpu_cross(n0, row1));
    let n1 = fpu_normalize(fpu_cross(n2, n0));
    [n0, n1, n2, [0.0, 0.0, 0.0, 1.0]]
}

/// Quaternion slerp from `a` (t = 0) to `b` (t = 1).
pub fn slerp(a: V4, b: V4, t: f32) -> V4 {
    use ps2::{add, madd, mul};
    if t == 1.0 {
        return b;
    }
    if t == 0.0 {
        return a;
    }
    let d = madd(add(0.0, madd(add(0.0, madd(add(0.0, mul(a[1], b[1])), a[0], b[0])), a[2], b[2])), a[3], b[3]);
    let a = if d < 0.0 { a.map(|c| -c) } else { a };
    micro_slerp(a, b, t)
}

/// The VU0 micro program the game calls for slerp (θ via an arctan polynomial around π/4, sines via a
/// polynomial), with the Q-register latencies of the original instruction schedule. Called bare (no shortest-arc
/// flip, no end shortcuts) by the motion sampler's squad.
pub fn micro_slerp(q1: V4, q2: V4, t: f32) -> V4 {
    use vu0::{add, div, madd, mul, sqrt, sub};
    let p = [mul(q1[0], q2[0]), mul(q1[1], q2[1]), mul(q1[2], q2[2]), mul(q1[3], q2[3])];
    let dot = madd(madd(add(p[3], p[2]), 1.0, p[1]), 1.0, p[0]);
    let one_minus_t = sub(1.0, t);
    let sin2 = msub_w(add(1.0, 0.0), dot);
    // flags: negative dot, sin²θ == 0 or dot > 0.999 fall back to a plain lerp
    let lerp_only = dot < 0.0 || sin2 == 0.0 || sub(dot, 0.999_000_013) > 0.0;
    let (w1, w2, s1, s2) = if lerp_only {
        (one_minus_t, t, q1, q2)
    } else {
        let c = dot.abs();
        let sin = sqrt(sin2);
        let num = add(sub(0.0, c), sin); // sinθ − cosθ
        let den = add(c, sin); // cosθ + sinθ
        let u = mul(num, div(1.0, den)); // tan(θ − π/4)
        let inv_sin = div(1.0, add(0.0, sin));
        let theta = atan_poly(u);
        let ang = [mul(t, theta), mul(one_minus_t, theta)];
        let sines = ang.map(sin_poly);
        let s1 = q1.map(|c| mul(c, inv_sin));
        let s2 = q2.map(|c| mul(c, inv_sin));
        (sines[1], sines[0], s1, s2)
    };
    std::array::from_fn(|k| madd(mul(s1[k], w1), s2[k], w2))
}

/// `msub.w`: 1 − dot·dot with the accumulator preloaded to 1.
fn msub_w(acc: f32, dot: f32) -> f32 {
    vu0::msub(acc, dot, dot)
}

/// θ = π/4 + atan(u), the micro program's odd polynomial in u (terms u¹ … u¹⁵).
fn atan_poly(u: f32) -> f32 {
    use vu0::{madd, mul};
    let f = |b: u32| f32::from_bits(b);
    let u2 = mul(u, u);
    let u3 = mul(u2, u);
    let u4 = mul(u2, u2);
    let u5 = mul(u3, u2);
    let u7 = mul(u4, u3);
    let u9 = mul(u5, u4);
    let u11 = mul(u7, u4);
    let u13 = mul(u9, u4);
    let u15 = mul(u11, u4);
    let acc = madd(add0(f(0x3f49_0fdb)), u, f(0x3f7f_fff5));
    let acc = madd(acc, u3, f(0xbeaa_a61c));
    let acc = madd(acc, u5, f(0x3e4c_40a6));
    let acc = madd(acc, u7, f(0xbe0e_6c63));
    let acc = madd(acc, u9, f(0x3dc5_77df));
    let acc = madd(acc, u11, f(0xbd65_01c4));
    let acc = madd(acc, u13, f(0x3cb3_1652));
    madd(acc, u15, f(0xbb84_d7e7))
}

fn add0(x: f32) -> f32 {
    vu0::add(0.0, x)
}

/// sin(x) as the micro program's odd polynomial (x¹ … x⁹).
fn sin_poly(x: f32) -> f32 {
    use vu0::{madd, mul};
    let f = |b: u32| f32::from_bits(b);
    let x2 = mul(x, x);
    let x3 = mul(x2, x);
    let x4 = mul(x2, x2);
    let x5 = mul(x3, x2);
    let x7 = mul(x4, x3);
    let x9 = mul(x5, x4);
    let acc = mul(x, 1.0);
    let acc = madd(acc, x3, f(0xbe2a_aaa4));
    let acc = madd(acc, x5, f(0x3c08_873e));
    let acc = madd(acc, x7, f(0xb94f_b21f));
    madd(acc, x9, f(0x362e_9c14))
}
