//! The C library's `sinf`, `cosf`, `tanf`, `atanf`, `atan2f`, `acosf` and `powf`, on EE FPU arithmetic.
//!
//! On the PS2 these are not instructions but C functions compiled into the executable from the SDK's
//! `libm.a`: the newlib build of Sun's fdlibm in its `float` flavour (Cygnus' `ef_*.c` / `kf_*.c` /
//! `sf_*.c`). This is a port of that code, step for step, each operation rounded by [`crate::ps2::F`]
//! (chopping, short adder, no denormals) so the results match the original bit for bit, and identically on
//! every host: integer ops and the four basic IEEE operations only, never the host's `std` functions.
//!
//! The constants are written as bit patterns, so a one-ulp difference between toolchains is a
//! one-character edit. These are the values in the SDK build that Hot Shots Tennis (SCUS-97610) links,
//! each function checked against the game's own over 20k inputs (`crates/hst-oracle/tests/libm.rs`):
//! `acosf` and `powf` have every polynomial constant one ulp below the correctly rounded decimal (that
//! compiler converted literals toward zero), and `powf` has the older fdlibm `t_h` seed. Shaped like
//! `ps2-float`'s `libm.rs` (FIZL0/ps2-decomp-kit-linux), so the two can merge.
//!
//! Argument reduction for `sin`/`cos`/`tan` is fdlibm's three-step one, exact up to `|x| <= 2^7·π/2`;
//! `__kernel_rem_pio2f` (larger arguments) is not ported.
//!
//! The fdlibm notice, as the float sources carry it:
//!
//! ```text
//! Conversion to float by Ian Lance Taylor, Cygnus Support, ian@cygnus.com.
//!
//! ====================================================
//! Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//!
//! Developed at SunPro, a Sun Microsystems, Inc. business.
//! Permission to use, copy, modify, and distribute this
//! software is freely granted, provided that this notice
//! is preserved.
//! ====================================================
//! ```
#![deny(clippy::float_arithmetic)]

use crate::ps2::F;

/// A constant from its bit pattern: no decimal-literal rounding to argue about.
const fn b(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `cvt.w.s` on the R5900 truncates.
fn trunc_i(x: f32) -> i32 {
    x as i32
}

// --- sin / cos / tan ----------------------------------------------------------------------------------------------

/// `__kernel_sinf(x, y, iy)`: sin on `|x| <= π/4`, `y` the tail of a reduced argument.
fn k_sin(x: f32, y: f32, iy: bool) -> F {
    if x.to_bits() & 0x7fff_ffff <= 0x31ff_ffff && trunc_i(x) == 0 {
        return F(x);
    }
    let (x, y) = (F(x), F(y));
    let z = x * x;
    let v = z * x;
    let mut r = z * b(0x2f2e_c9d2) + b(0xb2d7_2f34);
    r = z * r + b(0x3638_ef1a);
    r = z * r + b(0xb950_0d01);
    r = z * r + b(0x3c08_8889);
    if !iy {
        x + v * (z * r + b(0xbe2a_aaaa))
    } else {
        x - (z * (y * 0.5 - v * r) - y - v * b(0xbe2a_aaaa))
    }
}

/// `__kernel_cosf(x, y)`: cos on `|x| <= π/4`.
fn k_cos(x: f32, y: f32) -> F {
    let ix = x.to_bits() & 0x7fff_ffff;
    if ix <= 0x31ff_ffff && trunc_i(x) == 0 {
        return F::ONE;
    }
    let (x, y) = (F(x), F(y));
    let z = x * x;
    let mut r = z * b(0xad47_d74e) + b(0x310f_74f5);
    r = z * r + b(0xb493_f27b);
    r = z * r + b(0x37d0_0d00);
    r = z * r + b(0xbab6_0b60);
    r = z * r + b(0x3d2a_aaaa);
    let zr = z * r;
    if ix <= 0x3e99_9999 {
        1.0 - (z * 0.5 - (z * zr - x * y))
    } else {
        let qx = if ix > 0x3f48_0000 { b(0x3e90_0000) } else { b(ix - 0x0100_0000) };
        F::ONE - qx - (z * 0.5 - qx - (z * zr - x * y))
    }
}

const TAN_T: [u32; 13] = [
    0x3eaaaaab, 0x3e088889, 0x3d5d0dd1, 0x3cb327a3, 0x3c11371e, 0x3b6b6916, 0x3abede47, 0x3a1a26c7, 0x398137b8, 0x38a3f445,
    0x3895c07a, 0xb79bae5e, 0x37d95384,
];

/// `__kernel_tanf(x, y, iy)`: `tan(x + y)` for `iy == 1`, `-1/tan(x + y)` for `iy == -1`.
fn k_tan(x: f32, y: f32, iy: i32) -> F {
    let hx = x.to_bits() as i32;
    let ix = hx & 0x7fff_ffff;
    if ix <= 0x317f_ffff && trunc_i(x) == 0 {
        return if ix | (iy + 1) == 0 { 1.0 / F(x.abs()) } else if iy == 1 { F(x) } else { -1.0 / F(x) };
    }
    let big = ix > 0x3f2c_a13f;
    let (x, y) = if big {
        let (x, y) = if hx < 0 { (-F(x), -F(y)) } else { (F(x), F(y)) };
        (b(0x3f49_0fda) - x + (b(0x3322_2167) - y), F::ZERO)
    } else {
        (F(x), F(y))
    };
    let t = TAN_T.map(b);
    let z = x * x;
    let w = z * z;
    let s = z * x;
    let r = t[1] + w * (t[3] + w * (t[5] + w * (t[7] + w * (t[9] + w * t[11]))));
    let v = z * (t[2] + w * (t[4] + w * (t[6] + w * (t[8] + w * (t[10] + w * t[12])))));
    let r = y + z * (s * (r + v) + y) + t[0] * s;
    let w = x + r;
    if big {
        let v = iy as f32;
        let sign = (1 - ((hx >> 30) & 2)) as f32;
        return sign * (v - { let d = x - (w * w / (w + v) - r); d + d });
    }
    if iy == 1 {
        return w;
    }
    // −1/w, the reciprocal refined from its high half
    let chop = |a: F| F(b(a.0.to_bits() & 0xffff_f000));
    let z = chop(w);
    let v = r - (z - x);
    let a = -1.0 / w;
    let t = chop(a);
    t + a * (t * z + 1.0 + t * v)
}

const NPIO2_HW: [u32; 32] = [
    0x3fc90f00, 0x40490f00, 0x4096cb00, 0x40c90f00, 0x40fb5300, 0x4116cb00, 0x412fed00, 0x41490f00, 0x41623100,
    0x417b5300, 0x418a3a00, 0x4196cb00, 0x41a35c00, 0x41afed00, 0x41bc7e00, 0x41c90f00, 0x41d5a000, 0x41e23100,
    0x41eec200, 0x41fb5300, 0x4203f200, 0x420a3a00, 0x42108300, 0x4216cb00, 0x421d1400, 0x42235c00, 0x4229a500,
    0x422fed00, 0x42363600, 0x423c7e00, 0x4242c700, 0x42490f00,
];

/// `__ieee754_rem_pio2f`: `x = n·π/2 + (y0 + y1)` with `|y0 + y1| <= π/4`. Returns `(n, y0, y1)`.
///
/// Only up to `|x| <= 2^7·π/2`, where fdlibm's three-step reduction is exact; the `__kernel_rem_pio2f` path
/// above that is not ported (it panics).
fn rem_pio2(x: f32) -> (i32, f32, f32) {
    let hx = x.to_bits() as i32;
    let ix = hx & 0x7fff_ffff;
    let (p1, p1t, p2, p2t, p3, p3t) =
        (b(0x3fc9_0f80), b(0x3735_4442), b(0x3735_43ff), b(0x2e85_a308), b(0x2e85_a300), b(0x248d_3131));
    if ix <= 0x4016_cbe3 {
        // |x| ~<= 3π/4: one step
        let s = if hx > 0 { F::ONE } else { F(-1.0) };
        let mut z = x + -s * p1;
        let (y0, y1) = if ix & !0xf != 0x3fc9_0fd0 {
            let y0 = z + -s * p1t;
            (y0, z - y0 + -s * p1t)
        } else {
            z += -s * p2;
            let y0 = z + -s * p2t;
            (y0, z - y0 + -s * p2t)
        };
        return (if hx > 0 { 1 } else { -1 }, y0.0, y1.0);
    }
    assert!(ix <= 0x4349_0f80, "sinf argument out of the ported range: {x}");
    let t = F(x.abs());
    let n = trunc_i((t * b(0x3f22_f984) + 0.5).0);
    let fn_ = F(n as f32);
    let mut r = t - fn_ * p1;
    let mut w = fn_ * p1t;
    let mut y0 = r - w;
    if !(n < 32 && (ix as u32 & 0xffff_ff00) != NPIO2_HW[n as usize - 1]) {
        let j = ix >> 23;
        let i = j - ((y0.0.to_bits() >> 23) & 0xff) as i32;
        if i > 8 {
            let t0 = r;
            w = fn_ * p2;
            r = t0 - w;
            w = fn_ * p2t - (t0 - r - w);
            y0 = r - w;
            let i = j - ((y0.0.to_bits() >> 23) & 0xff) as i32;
            if i > 25 {
                let t0 = r;
                w = fn_ * p3;
                r = t0 - w;
                w = fn_ * p3t - (t0 - r - w);
                y0 = r - w;
            }
        }
    }
    let y1 = r - y0 - w;
    if hx < 0 { (-n, (-y0).0, (-y1).0) } else { (n, y0.0, y1.0) }
}

/// `sinf`.
pub fn sinf(x: f32) -> f32 {
    if !x.is_finite() {
        // NaN for NaN and ±inf, as fdlibm
        return (F(x) - x).0;
    }
    if x.to_bits() & 0x7fff_ffff <= 0x3f49_0fd8 {
        return k_sin(x, 0.0, false).0;
    }
    let (n, y0, y1) = rem_pio2(x);
    let r = match n & 3 {
        0 => k_sin(y0, y1, true),
        1 => k_cos(y0, y1),
        2 => -k_sin(y0, y1, true),
        _ => -k_cos(y0, y1),
    };
    r.0
}

/// `cosf`.
pub fn cosf(x: f32) -> f32 {
    if !x.is_finite() {
        // NaN for NaN and ±inf, as fdlibm
        return (F(x) - x).0;
    }
    if x.to_bits() & 0x7fff_ffff <= 0x3f49_0fd8 {
        return k_cos(x, 0.0).0;
    }
    let (n, y0, y1) = rem_pio2(x);
    let r = match n & 3 {
        0 => k_cos(y0, y1),
        1 => -k_sin(y0, y1, true),
        2 => -k_cos(y0, y1),
        _ => k_sin(y0, y1, true),
    };
    r.0
}

/// `tanf`.
pub fn tanf(x: f32) -> f32 {
    if x.to_bits() & 0x7fff_ffff <= 0x3f49_0fda {
        return k_tan(x, 0.0, 1).0;
    }
    let (n, y0, y1) = rem_pio2(x);
    k_tan(y0, y1, 1 - ((n & 1) << 1)).0
}

// --- atan / atan2 -------------------------------------------------------------------------------------------------

const ATAN_HI: [u32; 4] = [0x3eed6338, 0x3f490fda, 0x3f7b985d, 0x3fc90fda];
const ATAN_LO: [u32; 4] = [0x31ac3769, 0x33222167, 0x33140fb4, 0x33a22168];
const AT: [u32; 11] = [
    0x3eaaaaab, 0xbe4ccccc, 0x3e124924, 0xbde38e38, 0x3dba2e6e, 0xbd9d8795, 0x3d886b34, 0xbd6ef16a, 0x3d4bda59, 0xbd15a221,
    0x3c8569d7,
];

/// `atanf`.
pub fn atanf(x0: f32) -> f32 {
    let hx = x0.to_bits() as i32;
    let ix = hx & 0x7fff_ffff;
    if ix > 0x507f_ffff {
        let (hi, lo) = (F(b(ATAN_HI[3])), b(ATAN_LO[3]));
        return if hx > 0 { (hi + lo).0 } else { (-hi - lo).0 };
    }
    let (x, id) = if ix <= 0x3edf_ffff {
        if ix <= 0x30ff_ffff && 1.0 < F(x0) + b(0x7149_f2c9) {
            return x0;
        }
        (F(x0), -1)
    } else {
        let x = F(x0.abs());
        if ix <= 0x3f97_ffff {
            if ix <= 0x3f2f_ffff { ((x + x - 1.0) / (x + 2.0), 0) } else { ((x - 1.0) / (x + 1.0), 1) }
        } else if ix <= 0x401b_ffff {
            ((x - 1.5) / (x * 1.5 + 1.0), 2)
        } else {
            (-1.0 / x, 3)
        }
    };
    let a = AT.map(b);
    let z = x * x;
    let w = z * z;
    let s1 = z * (a[0] + w * (a[2] + w * (a[4] + w * (a[6] + w * (a[8] + w * a[10])))));
    let s2 = w * (a[1] + w * (a[3] + w * (a[5] + w * (a[7] + w * a[9]))));
    if id < 0 {
        return (x - x * (s1 + s2)).0;
    }
    let id = id as usize;
    let z = b(ATAN_HI[id]) - (x * (s1 + s2) - b(ATAN_LO[id]) - x);
    if hx < 0 { (-z).0 } else { z.0 }
}

/// `atan2f(y, x)`. Denormal arguments count as zero, as everywhere on the EE.
pub fn atan2f(y: f32, x: f32) -> f32 {
    const PI: u32 = 0x4049_0fda;
    const HALF_PI: u32 = 0x3fc9_0fda;
    const PI_LO: u32 = 0x3422_2168;
    let (hy, hx) = (y.to_bits() as i32, x.to_bits() as i32);
    if hx == 0x3f80_0000 {
        return atanf(y);
    }
    let (iy, ix) = (hy & 0x7fff_ffff, hx & 0x7fff_ffff);
    let m = ((hy as u32) >> 31) | ((hx >> 30) & 2) as u32;
    if iy <= 0x007f_ffff {
        match m {
            2 => return b(PI),
            3 => return (-F(b(PI))).0,
            _ => return y,
        }
    }
    if ix <= 0x007f_ffff {
        return if hy < 0 { (-F(b(HALF_PI))).0 } else { b(HALF_PI) };
    }
    let k = (iy - ix) >> 23;
    let z = if k >= 61 {
        F(b(HALF_PI))
    } else if hx < 0 && k < -60 {
        F::ZERO
    } else {
        F(atanf((F(y) / x).0.abs()))
    };
    let r = match m {
        0 => z,
        1 => -z,
        2 => b(PI) - (z - b(PI_LO)),
        _ => z - b(PI_LO) - b(PI),
    };
    r.0
}

// --- acos ---------------------------------------------------------------------------------------------------------

/// `acosf`. Its `sqrtf` is fdlibm's bit-by-bit one, correctly rounded.
pub fn acosf(x: f32) -> f32 {
    const PI: f32 = b(0x4049_0fda);
    const PIO2_HI: f32 = b(0x3fc9_0fda);
    const PIO2_LO: f32 = b(0x33a2_2168);
    // p(z)/q(z) in this build's term order: p = z·(pS0 + z·(pS1 + … z·pS5)), q = 1 + z·(qS1 + … z·qS4)
    let pq = |z: F| {
        let mut p = z * b(0x3811_ef08) + b(0x3a4f_7f04);
        for c in [0xbd24_1146, 0x3e4e_0aa8, 0xbea6_b08f, 0x3e2a_aaaa] {
            p = z * p + b(c);
        }
        let mut q = z * b(0x3d9d_c62d) + b(0xbf30_3360);
        for c in [0x4001_572c, 0xc019_d138] {
            q = z * q + b(c);
        }
        z * p / (z * q + 1.0)
    };
    // its sqrtf: correctly rounded, like the host's
    let sqrtf = |z: F| F(z.0.sqrt());
    let hx = x.to_bits() as i32;
    let ix = hx & 0x7fff_ffff;
    if ix == 0x3f80_0000 {
        return if hx > 0 { 0.0 } else { PI };
    }
    if ix > 0x3f80_0000 {
        return f32::NAN;
    }
    let x = F(x);
    if ix < 0x3f00_0000 {
        if ix <= 0x2300_0000 {
            return PIO2_HI;
        }
        let r = pq(x * x);
        return (PIO2_HI - (x - (PIO2_LO - x * r))).0;
    }
    if hx < 0 {
        let z = (x + 1.0) * 0.5;
        let r = pq(z);
        let s = sqrtf(z);
        let w = r * s - PIO2_LO;
        let t = s + w;
        return (PI - (t + t)).0;
    }
    let z = (1.0 - x) * 0.5;
    let s = sqrtf(z);
    let df = F(b(s.0.to_bits() & 0xffff_f000));
    let c = (z - df * df) / (s + df);
    let r = pq(z);
    let t = df + (r * s + c);
    (t + t).0
}

// --- pow ----------------------------------------------------------------------------------------------------------

/// The 12-bit-mantissa head of a float: fdlibm's way of splitting one value into two exactly
/// multipliable halves.
fn hi12(x: F) -> F {
    F(b(x.0.to_bits() & 0xffff_f000))
}

/// `__ieee754_powf(x, y)` for `x > 0` and `|y| < 2^27`, plus fdlibm's answer for a negative `x` to a
/// fractional power. This build's constant roundings and the older `t_h` seed (`+ 0x40000`, unmasked).
pub fn powf(x: f32, y: f32) -> f32 {
    if y == 0.5 && x.is_sign_positive() && x.is_finite() {
        return x.sqrt(); // its sqrtf: the exact bit-by-bit one
    }
    // a negative x to a fractional power is fdlibm's (x − x) / (x − x): 0/0, +max on the EE
    if x < 0.0 && x.to_bits() & 0x7fff_ffff >= 0x80_0000 && x != -1.0 && y.fract() != 0.0 && y.abs() < 1.0e8 {
        return ((F(x) - x) / (F(x) - x)).0;
    }
    // not ported: other x ≤ 0, |y| ≥ 2^27, |x| near 1 with huge y, over/underflow
    assert!(x > 0.0 && x.is_finite() && y.abs() < 1.0e8, "powf({x}, {y})");
    if y.to_bits() & 0x7fff_ffff < 0x80_0000 {
        return 1.0;
    }
    if y == 1.0 {
        return x;
    }
    if y == 2.0 {
        return (F(x) * x).0;
    }
    let (bp, dp_h, dp_l) = ([1.0, 1.5], [0.0, b(0x3f15_c000)], [0.0, b(0x35d1_cfdc)]);
    let ix = x.to_bits();
    let mut n = (ix >> 23) as i32 - 0x7f;
    let j = ix & 0x7f_ffff;
    let mut ix = j | 0x3f80_0000;
    let k = if j <= 0x1c_c471 {
        0
    } else if j < 0x5d_b3d7 {
        1
    } else {
        n += 1;
        ix -= 0x80_0000;
        0
    };
    let ax = F(b(ix));
    // log2(ax) as t1 + t2
    let u = ax - bp[k];
    let v = 1.0 / (ax + bp[k]);
    let ss = u * v;
    let s_h = hi12(ss);
    let t_h = F(b(((ix >> 1) | 0x2000_0000) + ((k as u32) << 21) + 0x4_0000));
    let t_l = ax - (t_h - bp[k]);
    let s_l = v * (u - s_h * t_h - s_h * t_l);
    let s2 = ss * ss;
    let mut r = F(b(0x3e53_f142));
    for c in [0x3e6c_3254, 0x3e8b_a304, 0x3eaa_aaab, 0x3edb_6db7, 0x3f19_9999] {
        r = s2 * r + b(c);
    }
    let r = s2 * s2 * r + s_l * (s_h + ss);
    let s2 = s_h * s_h;
    let t_h = hi12(s2 + 3.0 + r);
    let t_l = r - (t_h - 3.0 - s2);
    let u = s_h * t_h;
    let v = s_l * t_h + t_l * ss;
    let p_h = hi12(u + v);
    let p_l = v - (p_h - u);
    let z_h = p_h * b(0x3f76_3800);
    let z_l = p_h * b(0x369d_c39f) + p_l * b(0x3f76_384e) + dp_l[k];
    let t = n as f32;
    let t1 = hi12(z_h + z_l + dp_h[k] + t);
    let t2 = z_l - (t1 - t - dp_h[k] - z_h);
    // y·log2(x) as p_h + p_l, then 2^that
    let y1 = hi12(F(y));
    let p_l = (y - y1) * t1 + y * t2;
    let mut p_h = y1 * t1;
    let z = p_l + p_h;
    let jz = z.0.to_bits() as i32;
    let i = jz & 0x7fff_ffff;
    assert!(i < 0x42fc_0000, "powf({x}, {y}) out of range");
    let mut n = 0;
    if i > 0x3f00_0000 {
        let m = jz + (0x80_0000 >> ((i >> 23) - 0x7e));
        let e = ((m >> 23) & 0xff) - 0x7f;
        p_h -= b((m & !(0x7f_ffff >> e)) as u32);
        n = ((m & 0x7f_ffff) | 0x80_0000) >> (23 - e);
        if jz < 0 {
            n = -n;
        }
    }
    let t = hi12(p_l + p_h);
    let v = (p_l - (t - p_h)) * b(0x3f31_7217) + t * b(0x35bf_be8c);
    let z = t * b(0x3f31_7200) + v;
    let w = v - (z - t * b(0x3f31_7200));
    let t = z * z;
    let mut r = F(b(0x3331_bb4b));
    for c in [0xb5dd_ea0e, 0x388a_b354, 0xbb36_0b60, 0x3e2a_aaaa] {
        r = t * r + b(c);
    }
    let t1 = z - t * r;
    let r = z * t1 / (t1 - 2.0) - (w + z * w);
    let z = 1.0 - (r - z);
    let j = z.0.to_bits() as i32 + (n << 23);
    assert!(j >> 23 > 0, "powf({x}, {y}) underflows");
    b(j as u32)
}

#[cfg(test)]
#[allow(clippy::float_arithmetic)] // host references
mod tests {
    #[test]
    fn close_to_host_pow() {
        for i in 1..=1000 {
            let x = i as f32 / 100.0;
            assert!((super::powf(x, 1.1) / x.powf(1.1) - 1.0).abs() < 1e-6, "{x}");
        }
    }

    #[test]
    fn close_to_host_sin() {
        for i in 0..=1000 {
            let x = i as f32 * std::f32::consts::PI / 1000.0;
            assert!((super::sinf(x) - x.sin()).abs() < 1e-6, "{x}");
        }
    }

    #[test]
    fn close_to_host_atan2() {
        for i in 0..=1000 {
            let t = i as f32 * 2.0 * std::f32::consts::PI / 1000.0 - std::f32::consts::PI;
            let (y, x) = (3.0 * t.sin(), 3.0 * t.cos());
            assert!((super::atan2f(y, x) - y.atan2(x)).abs() < 2e-6, "{t}");
        }
    }

    #[test]
    fn close_to_host_acos() {
        for i in -1000..=1000 {
            let x = i as f32 / 1000.0;
            assert!((super::acosf(x) - x.acos()).abs() < 1e-6, "{x}");
        }
    }
}
