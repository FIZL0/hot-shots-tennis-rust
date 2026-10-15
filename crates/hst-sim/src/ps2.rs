//! Single-precision arithmetic as the reference emulator (PCSX2) performs the PS2 EE FPU's, so ported game
//! math reproduces recorded values bit for bit:
//! - results are rounded toward zero (FPU round mode "chop");
//! - before an add/sub the smaller operand loses the mantissa bits the EE would shift out, keeping one guard
//!   bit, and vanishes (sign only) when the exponents differ by 25 or more;
//! - divide and square root round to nearest (the emulator's separate divider round mode);
//! - denormals read and write as zero.
//! Verified bit-exact against the game's runtime shot-parameter table (2912 madd/add/sub/mul results).
#![deny(clippy::float_arithmetic)]

/// Denormals read as zero (keeping the sign), on the EE FPU and VU0 alike.
pub(crate) fn daz(x: f32) -> f32 {
    if x.is_subnormal() { 0.0f32.copysign(x) } else { x }
}

/// Round an f64 toward zero to f32 (also how VU0 rounds, and how the game's soft-float double → float conversion rounds).
pub fn chop(x: f64) -> f32 {
    let r = x as f32;
    let r = if (r as f64).abs() > x.abs() { f32::from_bits(r.to_bits() - 1) } else { r };
    daz(r)
}

/// Mask the smaller operand like the EE's alignment step.
fn align(a: f32, b: f32) -> (f32, f32) {
    let (ba, bb) = (a.to_bits(), b.to_bits());
    let (ea, eb) = (((ba >> 23) & 0xff) as i32, ((bb >> 23) & 0xff) as i32);
    let mask = |bits: u32, d: i32| if d >= 25 { bits & 0x8000_0000 } else { bits & !((1u32 << (d - 1)) - 1) };
    match ea - eb {
        d if d > 0 => (a, f32::from_bits(mask(bb, d))),
        d if d < 0 => (f32::from_bits(mask(ba, -d)), b),
        _ => (a, b),
    }
}

#[allow(clippy::float_arithmetic)] // the model itself, in host arithmetic
pub fn add(a: f32, b: f32) -> f32 {
    let (a, b) = align(daz(a), daz(b));
    // the masked operands differ by at most 25 bits of exponent, so the f64 sum is exact
    chop(a as f64 + b as f64)
}

#[allow(clippy::float_arithmetic)] // the model itself, in host arithmetic
pub fn sub(a: f32, b: f32) -> f32 {
    add(a, -b)
}

#[allow(clippy::float_arithmetic)] // the model itself, in host arithmetic
pub fn mul(a: f32, b: f32) -> f32 {
    chop(daz(a) as f64 * daz(b) as f64) // a 24×24-bit product is exact in f64
}

/// `madd.s`: accumulator + a·b, the product rounded first, then added through the aligning adder.
pub fn madd(acc: f32, a: f32, b: f32) -> f32 {
    add(acc, mul(a, b))
}

/// `msub.s`: accumulator − a·b.
pub fn msub(acc: f32, a: f32, b: f32) -> f32 {
    sub(acc, mul(a, b))
}

/// `sqrt.s` of |x|: square root shares the divider's round-to-nearest mode in the reference emulator.
pub fn sqrt(x: f32) -> f32 {
    daz(daz(x).abs().sqrt())
}

/// `div.s`: the divider is configured round-to-nearest in the reference emulator; x/0 gives ±max.
#[allow(clippy::float_arithmetic)] // the model itself, in host arithmetic
pub fn div(a: f32, b: f32) -> f32 {
    let (a, b) = (daz(a), daz(b));
    if b == 0.0 {
        return f32::MAX.copysign(if a.is_sign_negative() != b.is_sign_negative() { -1.0 } else { 1.0 });
    }
    daz(a / b)
}

/// An unsigned int to float as the game converts one (`cvt.s.w`, halved and doubled when the top bit is set):
/// chopped like every other FPU result.
pub fn utof(u: u32) -> f32 {
    chop(u as f64)
}

/// The game's lerp idiom (`adda 0, from` ; `sub to, from` ; `madd w`): from + w·(to − from).
pub fn lerp(from: f32, to: f32, w: f32) -> f32 {
    madd(add(0.0, from), w, sub(to, from))
}

/// An EE FPU single: `+ − × ÷` are the FPU's (`add`, `sub`, `mul`, `div` above), so a ported formula reads as
/// written. Rust evaluates `a + b + c` as `(a + b) + c` and never reassociates or fuses, so writing the
/// original's operations in its order keeps every intermediate rounding. `madd`/`msub` name the instruction where
/// the original has one (bit for bit the same as `acc + a * b`). An `f32` on either side is taken as an FPU
/// operand (constants, struct fields); unary minus is `neg.s`, a sign flip. Compares are the FPU's (`c.lt.s` …:
/// plain IEEE order, denormals aside).
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct F(pub f32);

/// `F(x)`, shorter in formulas.
pub const fn f(x: f32) -> F {
    F(x)
}

/// Every element of an `f32` array as `F`.
pub fn fs<const N: usize>(a: [f32; N]) -> [F; N] {
    a.map(F)
}

/// Every element of an `F` array back as `f32`.
pub fn f32s<const N: usize>(a: [F; N]) -> [f32; N] {
    a.map(|x| x.0)
}

impl F {
    pub const ZERO: F = F(0.0);
    pub const ONE: F = F(1.0);

    /// `madd.s`: self + a·b.
    pub fn madd(self, a: impl Into<F>, b: impl Into<F>) -> F {
        F(madd(self.0, a.into().0, b.into().0))
    }
    /// `msub.s`: self − a·b.
    pub fn msub(self, a: impl Into<F>, b: impl Into<F>) -> F {
        F(msub(self.0, a.into().0, b.into().0))
    }
    /// `sqrt.s` of |self|.
    pub fn sqrt(self) -> F {
        F(sqrt(self.0))
    }
    /// The game's lerp idiom from `self` to `to` at `w` (see [`lerp`]).
    pub fn lerp(self, to: impl Into<F>, w: impl Into<F>) -> F {
        F(lerp(self.0, to.into().0, w.into().0))
    }
    /// `abs.s`.
    pub fn abs(self) -> F {
        F(self.0.abs())
    }
    /// `min.s` / `max.s`, and both: compares only, nothing rounds.
    pub fn min(self, o: impl Into<F>) -> F {
        F(self.0.min(o.into().0))
    }
    pub fn max(self, o: impl Into<F>) -> F {
        F(self.0.max(o.into().0))
    }
    pub fn clamp(self, lo: impl Into<F>, hi: impl Into<F>) -> F {
        F(self.0.clamp(lo.into().0, hi.into().0))
    }
}

impl From<f32> for F {
    fn from(x: f32) -> F {
        F(x)
    }
}
impl From<F> for f32 {
    fn from(x: F) -> f32 {
        x.0
    }
}
impl PartialEq<f32> for F {
    fn eq(&self, o: &f32) -> bool {
        self.0 == *o
    }
}
impl PartialOrd<f32> for F {
    fn partial_cmp(&self, o: &f32) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(o)
    }
}
impl PartialEq<F> for f32 {
    fn eq(&self, o: &F) -> bool {
        *self == o.0
    }
}
impl PartialOrd<F> for f32 {
    fn partial_cmp(&self, o: &F) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&o.0)
    }
}

impl std::ops::Neg for F {
    type Output = F;
    #[allow(clippy::float_arithmetic)]
    fn neg(self) -> F {
        F(-self.0)
    }
}

macro_rules! fpu_op {
    ($tr:ident $m:ident $as_tr:ident $as_m:ident $op:ident) => {
        impl std::ops::$tr<F> for F {
            type Output = F;
            fn $m(self, o: F) -> F {
                F($op(self.0, o.0))
            }
        }
        impl std::ops::$tr<f32> for F {
            type Output = F;
            fn $m(self, o: f32) -> F {
                F($op(self.0, o))
            }
        }
        impl std::ops::$tr<F> for f32 {
            type Output = F;
            fn $m(self, o: F) -> F {
                F($op(self, o.0))
            }
        }
        impl<T: Into<F>> std::ops::$as_tr<T> for F {
            fn $as_m(&mut self, o: T) {
                *self = F($op(self.0, o.into().0));
            }
        }
    };
}
fpu_op!(Add add AddAssign add_assign add);
fpu_op!(Sub sub SubAssign sub_assign sub);
fpu_op!(Mul mul MulAssign mul_assign mul);
fpu_op!(Div div DivAssign div_assign div);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_keeps_one_guard_bit() {
        // 0.95 - 0.25 * 0.05000001 lands on 0.9375 on the PS2 path (plain chop gives 0.93749994)
        assert_eq!(lerp(0.95, 0.9, 0.25), 0.9375);
        assert_eq!(add(1.0, 1e-8), 1.0);
    }

    #[test]
    fn operators_are_the_fpu_ops_in_written_order() {
        let (a, b, c) = (F(0.95), F(0.05000001), F(1e-8));
        assert_eq!(a - 0.25 * b, F(msub(0.95, 0.25, 0.05000001)));
        assert_eq!((a + b + c).0, add(add(0.95, 0.05000001), 1e-8));
        assert_eq!(a.madd(b, c), a + b * c);
        assert_eq!(F(0.95).lerp(0.9, 0.25), 0.9375);
        let mut x = a;
        x -= b;
        assert_eq!(x.0, sub(0.95, 0.05000001));
    }
}
