//! The game's `sinf`, `cosf`, `acosf`, `atanf` and `atan2f` (fdlibm's float versions as compiled into the original, with its own
//! rounding of the constants), on PS2 float arithmetic.

use crate::ps2::{add, div, mul, sub};

const fn f(b: u32) -> f32 {
    f32::from_bits(b)
}

/// `cvt.w.s` on the R5900 truncates.
fn trunc_i(x: f32) -> i32 {
    x as i32
}

/// __kernel_sinf(x, y, iy).
fn k_sin(x: f32, y: f32, iy: bool) -> f32 {
    if x.to_bits() & 0x7fff_ffff <= 0x31ff_ffff && trunc_i(x) == 0 {
        return x;
    }
    let z = mul(x, x);
    let v = mul(z, x);
    let mut r = add(mul(z, f(0x2f2e_c9d2)), f(0xb2d7_2f34));
    r = add(mul(z, r), f(0x3638_ef1a));
    r = add(mul(z, r), f(0xb950_0d01));
    r = add(mul(z, r), f(0x3c08_8889));
    if !iy {
        add(x, mul(v, add(mul(z, r), f(0xbe2a_aaaa))))
    } else {
        sub(x, sub(sub(mul(z, sub(mul(y, 0.5), mul(v, r))), y), mul(v, f(0xbe2a_aaaa))))
    }
}

/// __kernel_cosf(x, y).
fn k_cos(x: f32, y: f32) -> f32 {
    let ix = x.to_bits() & 0x7fff_ffff;
    if ix <= 0x31ff_ffff && trunc_i(x) == 0 {
        return 1.0;
    }
    let z = mul(x, x);
    let mut r = add(mul(z, f(0xad47_d74e)), f(0x310f_74f5));
    r = add(mul(z, r), f(0xb493_f27b));
    r = add(mul(z, r), f(0x37d0_0d00));
    r = add(mul(z, r), f(0xbab6_0b60));
    r = add(mul(z, r), f(0x3d2a_aaaa));
    let zr = mul(z, r);
    if ix <= 0x3e99_9999 {
        sub(1.0, sub(mul(z, 0.5), sub(mul(z, zr), mul(x, y))))
    } else {
        let qx = if ix > 0x3f48_0000 { f(0x3e90_0000) } else { f(ix - 0x0100_0000) };
        sub(sub(1.0, qx), sub(sub(mul(z, 0.5), qx), sub(mul(z, zr), mul(x, y))))
    }
}

const NPIO2_HW: [u32; 32] = [
    0x3fc90f00, 0x40490f00, 0x4096cb00, 0x40c90f00, 0x40fb5300, 0x4116cb00, 0x412fed00, 0x41490f00, 0x41623100,
    0x417b5300, 0x418a3a00, 0x4196cb00, 0x41a35c00, 0x41afed00, 0x41bc7e00, 0x41c90f00, 0x41d5a000, 0x41e23100,
    0x41eec200, 0x41fb5300, 0x4203f200, 0x420a3a00, 0x42108300, 0x4216cb00, 0x421d1400, 0x42235c00, 0x4229a500,
    0x422fed00, 0x42363600, 0x423c7e00, 0x4242c700, 0x42490f00,
];

/// __ieee754_rem_pio2f for |x| ≤ 2^7·π/2 (the large-argument path is never reached by the game's callers).
fn rem_pio2(x: f32) -> (i32, f32, f32) {
    let hx = x.to_bits() as i32;
    let ix = hx & 0x7fff_ffff;
    let (p1, p1t, p2, p2t, p3, p3t) =
        (f(0x3fc9_0f80), f(0x3735_4442), f(0x3735_43ff), f(0x2e85_a308), f(0x2e85_a300), f(0x248d_3131));
    if ix <= 0x4016_cbe3 {
        // |x| ~<= 3π/4: one step
        let s = if hx > 0 { 1.0 } else { -1.0 };
        let mut z = add(x, -s * p1);
        let (y0, y1) = if ix & !0xf != 0x3fc9_0fd0 {
            let y0 = add(z, -s * p1t);
            (y0, add(sub(z, y0), -s * p1t))
        } else {
            z = add(z, -s * p2);
            let y0 = add(z, -s * p2t);
            (y0, add(sub(z, y0), -s * p2t))
        };
        return (if hx > 0 { 1 } else { -1 }, y0, y1);
    }
    assert!(ix <= 0x4349_0f80, "sinf argument out of the ported range: {x}");
    let t = x.abs();
    let n = trunc_i(add(mul(t, f(0x3f22_f984)), 0.5));
    let fn_ = n as f32;
    let mut r = sub(t, mul(fn_, p1));
    let mut w = mul(fn_, p1t);
    let mut y0 = sub(r, w);
    if !(n < 32 && (ix as u32 & 0xffff_ff00) != NPIO2_HW[n as usize - 1]) {
        let j = ix >> 23;
        let i = j - ((y0.to_bits() >> 23) & 0xff) as i32;
        if i > 8 {
            let t0 = r;
            w = mul(fn_, p2);
            r = sub(t0, w);
            w = sub(mul(fn_, p2t), sub(sub(t0, r), w));
            y0 = sub(r, w);
            let i = j - ((y0.to_bits() >> 23) & 0xff) as i32;
            if i > 25 {
                let t0 = r;
                w = mul(fn_, p3);
                r = sub(t0, w);
                w = sub(mul(fn_, p3t), sub(sub(t0, r), w));
                y0 = sub(r, w);
            }
        }
    }
    let y1 = sub(sub(r, y0), w);
    if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) }
}

pub fn sinf(x: f32) -> f32 {
    if x.to_bits() & 0x7fff_ffff <= 0x3f49_0fd8 {
        return k_sin(x, 0.0, false);
    }
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => k_sin(y0, y1, true),
        1 => k_cos(y0, y1),
        2 => -k_sin(y0, y1, true),
        _ => -k_cos(y0, y1),
    }
}

pub fn cosf(x: f32) -> f32 {
    if x.to_bits() & 0x7fff_ffff <= 0x3f49_0fd8 {
        return k_cos(x, 0.0);
    }
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => k_cos(y0, y1),
        1 => -k_sin(y0, y1, true),
        2 => -k_cos(y0, y1),
        _ => k_sin(y0, y1, true),
    }
}

const ATAN_HI: [u32; 4] = [0x3eed6338, 0x3f490fda, 0x3f7b985d, 0x3fc90fda];
const ATAN_LO: [u32; 4] = [0x31ac3769, 0x33222167, 0x33140fb4, 0x33a22168];
const AT: [u32; 11] = [
    0x3eaaaaab, 0xbe4ccccc, 0x3e124924, 0xbde38e38, 0x3dba2e6e, 0xbd9d8795, 0x3d886b34, 0xbd6ef16a, 0x3d4bda59, 0xbd15a221,
    0x3c8569d7,
];

pub fn atanf(x0: f32) -> f32 {
    let hx = x0.to_bits() as i32;
    let ix = hx & 0x7fff_ffff;
    if ix > 0x507f_ffff {
        let (hi, lo) = (f(ATAN_HI[3]), f(ATAN_LO[3]));
        return if hx > 0 { add(hi, lo) } else { sub(-hi, lo) };
    }
    let (x, id) = if ix <= 0x3edf_ffff {
        if ix <= 0x30ff_ffff && 1.0 < add(x0, f(0x7149_f2c9)) {
            return x0;
        }
        (x0, -1)
    } else {
        let x = x0.abs();
        if ix <= 0x3f97_ffff {
            if ix <= 0x3f2f_ffff { (div(sub(add(x, x), 1.0), add(x, 2.0)), 0) } else { (div(sub(x, 1.0), add(x, 1.0)), 1) }
        } else if ix <= 0x401b_ffff {
            (div(sub(x, 1.5), add(mul(x, 1.5), 1.0)), 2)
        } else {
            (div(-1.0, x), 3)
        }
    };
    let a = AT.map(f);
    let z = mul(x, x);
    let w = mul(z, z);
    let s1 = mul(z, add(a[0], mul(w, add(a[2], mul(w, add(a[4], mul(w, add(a[6], mul(w, add(a[8], mul(w, a[10])))))))))));
    let s2 = mul(w, add(a[1], mul(w, add(a[3], mul(w, add(a[5], mul(w, add(a[7], mul(w, a[9])))))))));
    if id < 0 {
        return sub(x, mul(x, add(s1, s2)));
    }
    let id = id as usize;
    let z = sub(f(ATAN_HI[id]), sub(sub(mul(x, add(s1, s2)), f(ATAN_LO[id])), x));
    if hx < 0 { -z } else { z }
}

/// atan2f(y, x); denormals count as zero.
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
            2 => return f(PI),
            3 => return -f(PI),
            _ => return y,
        }
    }
    if ix <= 0x007f_ffff {
        return if hy < 0 { -f(HALF_PI) } else { f(HALF_PI) };
    }
    let k = (iy - ix) >> 23;
    let z = if k >= 61 {
        f(HALF_PI)
    } else if hx < 0 && k < -60 {
        0.0
    } else {
        atanf(div(y, x).abs())
    };
    match m {
        0 => z,
        1 => -z,
        2 => sub(f(PI), sub(z, f(PI_LO))),
        _ => sub(sub(z, f(PI_LO)), f(PI)),
    }
}

/// acosf(x): fdlibm's float version as the original compiled it (its sqrtf is the exact bit-by-bit one).
pub fn acosf(x: f32) -> f32 {
    const PI: f32 = f(0x4049_0fda);
    const PIO2_HI: f32 = f(0x3fc9_0fda);
    const PIO2_LO: f32 = f(0x33a2_2168);
    // p(z)/q(z) with the game's constant order: p = z·(pS0 + z·(pS1 + … z·pS5)), q = 1 + z·(qS1 + … z·qS4)
    let pq = |z: f32| {
        let mut p = add(mul(z, f(0x3811_ef08)), f(0x3a4f_7f04));
        for c in [0xbd24_1146, 0x3e4e_0aa8, 0xbea6_b08f, 0x3e2a_aaaa] {
            p = add(mul(z, p), f(c));
        }
        let mut q = add(mul(z, f(0x3d9d_c62d)), f(0xbf30_3360));
        for c in [0x4001_572c, 0xc019_d138] {
            q = add(mul(z, q), f(c));
        }
        div(mul(z, p), add(mul(z, q), 1.0))
    };
    let hx = x.to_bits() as i32;
    let ix = hx & 0x7fff_ffff;
    if ix == 0x3f80_0000 {
        return if hx > 0 { 0.0 } else { PI };
    }
    if ix > 0x3f80_0000 {
        return f32::NAN;
    }
    if ix < 0x3f00_0000 {
        if ix <= 0x2300_0000 {
            return PIO2_HI;
        }
        let r = pq(mul(x, x));
        return sub(PIO2_HI, sub(x, sub(PIO2_LO, mul(x, r))));
    }
    if hx < 0 {
        let z = mul(add(x, 1.0), 0.5);
        let r = pq(z);
        let s = z.sqrt();
        let w = sub(mul(r, s), PIO2_LO);
        let t = add(s, w);
        return sub(PI, add(t, t));
    }
    let z = mul(sub(1.0, x), 0.5);
    let s = z.sqrt();
    let df = f(s.to_bits() & 0xffff_f000);
    let c = div(sub(z, mul(df, df)), add(s, df));
    let r = pq(z);
    let t = add(df, add(mul(r, s), c));
    add(t, t)
}

fn hi(x: f32) -> f32 {
    f(x.to_bits() & 0xffff_f000)
}

/// __ieee754_powf(x, y) for x > 0 and y of moderate size (the game raises speeds to 1.1). It is fdlibm's,
/// as compiled into the original: its own constant roundings and the old `t_h` seed (`+ 0x40000`, unmasked).
pub fn powf(x: f32, y: f32) -> f32 {
    // ponytail: x ≤ 0, |y| ≥ 2^27, |x| near 1 with huge y and over/underflow aren't ported; the game never asks
    assert!(x > 0.0 && x.is_finite() && y.abs() < 1.0e8, "powf({x}, {y})");
    if y.to_bits() & 0x7fff_ffff < 0x80_0000 {
        return 1.0;
    }
    if y == 1.0 {
        return x;
    }
    if y == 2.0 {
        return mul(x, x);
    }
    let (bp, dp_h, dp_l) = ([1.0, 1.5], [0.0, f(0x3f15_c000)], [0.0, f(0x35d1_cfdc)]);
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
    let ax = f(ix);
    // log2(ax) as t1 + t2
    let u = sub(ax, bp[k]);
    let v = div(1.0, add(ax, bp[k]));
    let ss = mul(u, v);
    let s_h = hi(ss);
    let t_h = f(((ix >> 1) | 0x2000_0000) + ((k as u32) << 21) + 0x4_0000);
    let t_l = sub(ax, sub(t_h, bp[k]));
    let s_l = mul(v, sub(sub(u, mul(s_h, t_h)), mul(s_h, t_l)));
    let s2 = mul(ss, ss);
    let mut r = f(0x3e53_f142);
    for c in [0x3e6c_3254, 0x3e8b_a304, 0x3eaa_aaab, 0x3edb_6db7, 0x3f19_9999] {
        r = add(mul(s2, r), f(c));
    }
    let r = add(mul(mul(s2, s2), r), mul(s_l, add(s_h, ss)));
    let s2 = mul(s_h, s_h);
    let t_h = hi(add(add(s2, 3.0), r));
    let t_l = sub(r, sub(sub(t_h, 3.0), s2));
    let u = mul(s_h, t_h);
    let v = add(mul(s_l, t_h), mul(t_l, ss));
    let p_h = hi(add(u, v));
    let p_l = sub(v, sub(p_h, u));
    let z_h = mul(p_h, f(0x3f76_3800));
    let z_l = add(add(mul(p_h, f(0x369d_c39f)), mul(p_l, f(0x3f76_384e))), dp_l[k]);
    let t = n as f32;
    let t1 = hi(add(add(add(z_h, z_l), dp_h[k]), t));
    let t2 = sub(z_l, sub(sub(sub(t1, t), dp_h[k]), z_h));
    // y·log2(x) as p_h + p_l, then 2^that
    let y1 = hi(y);
    let p_l = add(mul(sub(y, y1), t1), mul(y, t2));
    let mut p_h = mul(y1, t1);
    let z = add(p_l, p_h);
    let jz = z.to_bits() as i32;
    let i = jz & 0x7fff_ffff;
    assert!(i < 0x42fc_0000, "powf({x}, {y}) out of range");
    let mut n = 0;
    if i > 0x3f00_0000 {
        let m = jz + (0x80_0000 >> ((i >> 23) - 0x7e));
        let e = ((m >> 23) & 0xff) - 0x7f;
        p_h = sub(p_h, f((m & !(0x7f_ffff >> e)) as u32));
        n = ((m & 0x7f_ffff) | 0x80_0000) >> (23 - e);
        if jz < 0 {
            n = -n;
        }
    }
    let t = hi(add(p_l, p_h));
    let v = add(mul(sub(p_l, sub(t, p_h)), f(0x3f31_7217)), mul(t, f(0x35bf_be8c)));
    let z = add(mul(t, f(0x3f31_7200)), v);
    let w = sub(v, sub(z, mul(t, f(0x3f31_7200))));
    let t = mul(z, z);
    let mut r = f(0x3331_bb4b);
    for c in [0xb5dd_ea0e, 0x388a_b354, 0xbb36_0b60, 0x3e2a_aaaa] {
        r = add(mul(t, r), f(c));
    }
    let t1 = sub(z, mul(t, r));
    let r = sub(div(mul(z, t1), sub(t1, 2.0)), add(w, mul(z, w)));
    let z = sub(1.0, sub(r, z));
    let j = z.to_bits() as i32 + (n << 23);
    assert!(j >> 23 > 0, "powf({x}, {y}) underflows");
    f(j as u32)
}

#[cfg(test)]
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
