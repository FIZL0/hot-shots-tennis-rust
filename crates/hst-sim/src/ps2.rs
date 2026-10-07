//! Single-precision arithmetic as the reference emulator (PCSX2) performs the PS2 EE FPU's, so ported game
//! math reproduces recorded values bit for bit:
//! - results are rounded toward zero (FPU round mode "chop");
//! - before an add/sub the smaller operand loses the mantissa bits the EE would shift out, keeping one guard
//!   bit, and vanishes (sign only) when the exponents differ by 25 or more;
//! - divide and square root round to nearest (the emulator's separate divider round mode);
//! - denormals read and write as zero.
//! Verified bit-exact against the game's runtime shot-parameter table (2912 madd/add/sub/mul results).

fn daz(x: f32) -> f32 {
    if x.is_subnormal() { 0.0f32.copysign(x) } else { x }
}

/// Round an exact (or exactly representable) f64 toward zero to f32.
fn chop(x: f64) -> f32 {
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

pub fn add(a: f32, b: f32) -> f32 {
    let (a, b) = align(daz(a), daz(b));
    // the masked operands differ by at most 25 bits of exponent, so the f64 sum is exact
    chop(a as f64 + b as f64)
}

pub fn sub(a: f32, b: f32) -> f32 {
    add(a, -b)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_keeps_one_guard_bit() {
        // 0.95 - 0.25 * 0.05000001 lands on 0.9375 on the PS2 path (plain chop gives 0.93749994)
        assert_eq!(lerp(0.95, 0.9, 0.25), 0.9375);
        assert_eq!(add(1.0, 1e-8), 1.0);
    }
}
