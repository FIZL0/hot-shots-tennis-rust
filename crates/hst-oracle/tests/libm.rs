//! The game's libm against `hst_sim::libm`, through the oracle. Needs `context/iso/` (skips without it).

use hst_oracle::{Ee, HstEe, addr};
use hst_sim::libm;

/// xorshift: deterministic inputs without a dependency.
fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}

/// Half the draws are any finite bit pattern, half land in [-8, 8] where the game's callers live.
fn input(seed: &mut u64) -> f32 {
    let b = rng(seed);
    if b & 1 == 0 {
        let f = f32::from_bits(rng(seed));
        if f.is_finite() { f } else { 0.5 }
    } else {
        (rng(seed) as f32 / u32::MAX as f32 - 0.5) * 16.0
    }
}

fn check(name: &str, n: usize, args: impl Fn(&mut u64) -> Vec<f32>, port: impl Fn(&[f32]) -> f32) {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    let (at, mut seed, mut bad) = (addr(name), 0x9e37_79b9_7f4a_7c15u64, vec![]);
    for _ in 0..n {
        let a = args(&mut seed);
        ee.call(at, &[], &a, 100_000);
        let (want, got) = (ee.f0(), port(&a));
        if want.to_bits() != got.to_bits() {
            bad.push(format!("{name}{a:?}: game {want:e} ({:08x}) port {got:e} ({:08x})", want.to_bits(), got.to_bits()));
        }
    }
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
}

#[test]
fn atan2f() {
    check("atan2f", 20_000, |s| vec![input(s), input(s)], |a| libm::atan2f(a[0], a[1]));
}

#[test]
fn atanf() {
    check("atanf", 20_000, |s| vec![input(s)], |a| libm::atanf(a[0]));
}

/// The port covers x > 0 and moderate y (the game raises speeds to 1.1).
#[test]
fn powf() {
    check("powf", 20_000, |s| vec![input(s).abs().clamp(1e-6, 1e6), input(s).clamp(-4.0, 4.0)], |a| libm::powf(a[0], a[1]));
}

/// Any argument: past 2^7·π/2 ≈ 201 (nearly half the bit patterns) the reduction is `__kernel_rem_pio2f`, which
/// reads only the bits, so an Inf/NaN pattern is a number to it. Draws: a quarter any 32 bits, a quarter any finite
/// pattern, a quarter [-8, 8], a quarter up to ±2^24 where the reduction cancels most.
fn trig(seed: &mut u64) -> f32 {
    match rng(seed) & 3 {
        0 => f32::from_bits(rng(seed)),
        1 => Some(f32::from_bits(rng(seed))).filter(|f| f.is_finite()).unwrap_or(0.5),
        2 => (rng(seed) as f32 / u32::MAX as f32 - 0.5) * 16.0,
        _ => (rng(seed) as f32 / u32::MAX as f32 - 0.5) * 33554432.0,
    }
}

#[test]
fn sinf() {
    check("sinf", 20_000, |s| vec![trig(s)], |a| libm::sinf(a[0]));
}

#[test]
fn cosf() {
    check("cosf", 20_000, |s| vec![trig(s)], |a| libm::cosf(a[0]));
}

#[test]
fn tanf() {
    check("tanf", 20_000, |s| vec![trig(s)], |a| libm::tanf(a[0]));
}

/// Inside [-1, 1] (outside it the port answers the host's NaN; the EE has none).
#[test]
fn acosf() {
    let arg = |s: &mut u64| {
        let x = input(s);
        vec![if x.abs() <= 1.0 { x } else { x / 8.0 % 1.0 }]
    };
    check("acosf", 20_000, arg, |a| libm::acosf(a[0]));
}
