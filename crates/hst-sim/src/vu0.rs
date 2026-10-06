//! VU0 arithmetic as the reference emulator performs it (macro mode and micro programs): every operation is
//! rounded toward zero (VU round mode "chop"), with no add/sub operand alignment step (that is an FPU quirk),
//! and multiply-add is a rounded product followed by a rounded add.

pub type V4 = [f32; 4];

fn daz(x: f32) -> f32 {
    if x.is_subnormal() { 0.0f32.copysign(x) } else { x }
}

/// Round toward zero to f32. `x` must be the exact result or close enough that the f32 neighbourhood is right.
fn chop(x: f64) -> f32 {
    let r = x as f32;
    let r = if (r as f64).abs() > x.abs() { f32::from_bits(r.to_bits() - 1) } else { r };
    daz(r)
}

pub fn add(a: f32, b: f32) -> f32 {
    chop(daz(a) as f64 + daz(b) as f64)
}
pub fn sub(a: f32, b: f32) -> f32 {
    add(a, -b)
}
pub fn mul(a: f32, b: f32) -> f32 {
    chop(daz(a) as f64 * daz(b) as f64)
}
pub fn madd(acc: f32, a: f32, b: f32) -> f32 {
    add(acc, mul(a, b))
}
pub fn msub(acc: f32, a: f32, b: f32) -> f32 {
    sub(acc, mul(a, b))
}
pub fn sqrt(x: f32) -> f32 {
    chop((daz(x).abs() as f64).sqrt())
}
/// Q-register divide, chopped; x/0 gives ±max.
pub fn div(a: f32, b: f32) -> f32 {
    let (a, b) = (daz(a), daz(b));
    if b == 0.0 {
        return f32::MAX.copysign(if a.is_sign_negative() != b.is_sign_negative() { -1.0 } else { 1.0 });
    }
    let r = a / b; // nearest; step toward zero if it rounded away
    let over = ((r as f64) * (b as f64)).abs() > (a as f64).abs();
    daz(if over && r != 0.0 { f32::from_bits(r.to_bits() - 1) } else { r })
}

/// `vmul.xyz` + `vaddy.x` + `vaddz.x`: (a.x·b.x + a.y·b.y) + a.z·b.z.
pub fn dot3(a: V4, b: V4) -> f32 {
    add(add(mul(a[0], b[0]), mul(a[1], b[1])), mul(a[2], b[2]))
}

/// Normalize xyz (w becomes 0) as the game's VU0 routine: Q = sqrt(len²), Q = 1/Q, v·Q.
pub fn normalize(v: V4) -> V4 {
    let l = add(0.0, sqrt(dot3(v, v)));
    let q = div(1.0, l);
    [mul(v[0], q), mul(v[1], q), mul(v[2], q), 0.0]
}

/// `vopmula` / `vopmsub` cross product a × b (w = 0).
pub fn cross(a: V4, b: V4) -> V4 {
    [msub(mul(a[1], b[2]), b[1], a[2]), msub(mul(a[2], b[0]), b[2], a[0]), msub(mul(a[0], b[1]), b[0], a[1]), 0.0]
}

/// Row-vector transform: ((r0·v.x + r1·v.y) + r2·v.z) + r3·v.w per component (`vmulax`/`vmadda`/`vmaddw`).
pub fn transform(m: &[V4; 4], v: V4) -> V4 {
    std::array::from_fn(|k| madd(madd(madd(mul(m[0][k], v[0]), m[1][k], v[1]), m[2][k], v[2]), m[3][k], v[3]))
}

/// `a·t + b·(1 − t)` per component (`vmulax` then `vmaddx`).
pub fn lerp(a: V4, b: V4, t: f32) -> V4 {
    let u = sub(add(0.0, 1.0), t);
    std::array::from_fn(|k| madd(mul(a[k], t), b[k], u))
}
