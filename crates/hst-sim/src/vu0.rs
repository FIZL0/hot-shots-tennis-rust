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

/// IEEE add rounded toward zero. The f64 sum alone can round away an addend that is far smaller, so the
/// rounding error (TwoSum) decides the direction: x + tiny of opposite sign truncates to the next f32 down.
pub fn add(a: f32, b: f32) -> f32 {
    let (a, b) = (daz(a) as f64, daz(b) as f64);
    let s = a + b;
    let bb = s - a;
    let e = (a - (s - bb)) + (b - bb);
    let r = s as f32;
    let over = (r as f64 - s) - e; // r − exact sum
    daz(if over != 0.0 && r != 0.0 && (over > 0.0) == (r > 0.0) { f32::from_bits(r.to_bits() - 1) } else { r })
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

/// A VU0 lane: `+ − ×` are VU0's (`add`, `sub`, `mul` above, chopped, no alignment), `/` the Q divide, so a
/// ported micro program reads as written and keeps its order (`a + b + c` is `(a + b) + c`). `madd`/`msub` name
/// the instruction where the original has one (the same as `acc + a * b`). An `f32` on either side is taken as a
/// VU operand; unary minus is a sign flip.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Vf(pub f32);

/// A VU0 register's four lanes.
pub type Vfs = [Vf; 4];

/// Every lane of `v` as `Vf`.
pub fn vf(v: V4) -> Vfs {
    v.map(Vf)
}

/// Every lane back as `f32`.
pub fn v4(v: Vfs) -> V4 {
    v.map(|x| x.0)
}

impl Vf {
    /// `vmadd`: self + a·b.
    pub fn madd(self, a: impl Into<Vf>, b: impl Into<Vf>) -> Vf {
        Vf(madd(self.0, a.into().0, b.into().0))
    }
    /// `vmsub`: self − a·b.
    pub fn msub(self, a: impl Into<Vf>, b: impl Into<Vf>) -> Vf {
        Vf(msub(self.0, a.into().0, b.into().0))
    }
    /// `vsqrt` of |self|.
    pub fn sqrt(self) -> Vf {
        Vf(sqrt(self.0))
    }
}

impl From<f32> for Vf {
    fn from(x: f32) -> Vf {
        Vf(x)
    }
}
impl From<Vf> for f32 {
    fn from(x: Vf) -> f32 {
        x.0
    }
}
impl PartialEq<f32> for Vf {
    fn eq(&self, o: &f32) -> bool {
        self.0 == *o
    }
}
impl PartialOrd<f32> for Vf {
    fn partial_cmp(&self, o: &f32) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(o)
    }
}

impl std::ops::Neg for Vf {
    type Output = Vf;
    #[allow(clippy::float_arithmetic)]
    fn neg(self) -> Vf {
        Vf(-self.0)
    }
}

macro_rules! vu_op {
    ($tr:ident $m:ident $as_tr:ident $as_m:ident $op:ident) => {
        impl std::ops::$tr<Vf> for Vf {
            type Output = Vf;
            fn $m(self, o: Vf) -> Vf {
                Vf($op(self.0, o.0))
            }
        }
        impl std::ops::$tr<f32> for Vf {
            type Output = Vf;
            fn $m(self, o: f32) -> Vf {
                Vf($op(self.0, o))
            }
        }
        impl std::ops::$tr<Vf> for f32 {
            type Output = Vf;
            fn $m(self, o: Vf) -> Vf {
                Vf($op(self, o.0))
            }
        }
        impl<T: Into<Vf>> std::ops::$as_tr<T> for Vf {
            fn $as_m(&mut self, o: T) {
                *self = Vf($op(self.0, o.into().0));
            }
        }
    };
}
vu_op!(Add add AddAssign add_assign add);
vu_op!(Sub sub SubAssign sub_assign sub);
vu_op!(Mul mul MulAssign mul_assign mul);
vu_op!(Div div DivAssign div_assign div);

/// `vmul.xyz` + `vaddy.x` + `vaddz.x`: (a.x·b.x + a.y·b.y) + a.z·b.z.
pub fn dot3(a: V4, b: V4) -> f32 {
    let (a, b) = (vf(a), vf(b));
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).0
}

/// Normalize xyz (w becomes 0) as the game's VU0 routine: Q = sqrt(len²), Q = 1/Q, v·Q.
pub fn normalize(v: V4) -> V4 {
    let l = Vf(0.0) + Vf(dot3(v, v)).sqrt();
    let q = 1.0 / l;
    let v = vf(v);
    v4([v[0] * q, v[1] * q, v[2] * q, Vf(0.0)])
}

/// `vopmula` / `vopmsub` cross product a × b (w = 0).
pub fn cross(a: V4, b: V4) -> V4 {
    let (a, b) = (vf(a), vf(b));
    v4([(a[1] * b[2]).msub(b[1], a[2]), (a[2] * b[0]).msub(b[2], a[0]), (a[0] * b[1]).msub(b[0], a[1]), Vf(0.0)])
}

/// Row-vector transform: ((r0·v.x + r1·v.y) + r2·v.z) + r3·v.w per component (`vmulax`/`vmadda`/`vmaddw`).
pub fn transform(m: &[V4; 4], v: V4) -> V4 {
    let (m, v) = (m.map(vf), vf(v));
    v4(std::array::from_fn(|k| (m[0][k] * v[0]).madd(m[1][k], v[1]).madd(m[2][k], v[2]).madd(m[3][k], v[3])))
}

/// `a·t + b·(1 − t)` per component (`vmulax` then `vmaddx`).
pub fn lerp(a: V4, b: V4, t: f32) -> V4 {
    let (a, b, t) = (vf(a), vf(b), Vf(t));
    let u = Vf(0.0) + 1.0 - t;
    v4(std::array::from_fn(|k| (a[k] * t).madd(b[k], u)))
}
