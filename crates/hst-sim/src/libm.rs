//! The game's `sinf` (fdlibm's float version as compiled into the original), on PS2 float arithmetic.

use crate::ps2::{add, mul, sub};

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

#[cfg(test)]
mod tests {
    #[test]
    fn close_to_host_sin() {
        for i in 0..=1000 {
            let x = i as f32 * std::f32::consts::PI / 1000.0;
            assert!((super::sinf(x) - x.sin()).abs() < 1e-6, "{x}");
        }
    }
}
