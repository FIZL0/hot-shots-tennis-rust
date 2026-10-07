//! The game's `sinf`, `acosf`, `atanf` and `atan2f` (fdlibm's float versions as compiled into the original, with its own
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

#[cfg(test)]
mod tests {
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
