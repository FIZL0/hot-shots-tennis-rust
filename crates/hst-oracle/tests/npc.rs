//! The ball-hit obstacles' box test and the ball speed against `hst_sim::npc`, through the oracle. Needs
//! `context/iso/` and `context/ram/s03.bin` (the box extents are set at run time; skips without them).

use hst_oracle::{Ee, HstEe, addr};
use hst_sim::npc::{ball_kmh, boxes_overlap, obstacle_extents};

fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}
fn uni(seed: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rng(seed) as f32 / u32::MAX as f32)
}

/// A matrix about `at`: a rotation about Y (as the players' and obstacles' are), a random rotation, or random rows.
fn matrix(s: &mut u64, at: [f32; 3]) -> [[f32; 4]; 4] {
    let a = uni(s, -3.2, 3.2);
    let (sn, cs) = (a.sin(), a.cos());
    let mut m = match rng(s) % 3 {
        0 => [[cs, 0.0, -sn, 0.0], [0.0, 1.0, 0.0, 0.0], [sn, 0.0, cs, 0.0], [0.0; 4]],
        1 => {
            let (b, c) = (uni(s, -3.2, 3.2), uni(s, -3.2, 3.2));
            let (sb, cb, sc, cc) = (b.sin(), b.cos(), c.sin(), c.cos());
            let rx = [[1.0, 0.0, 0.0], [0.0, cb, sb], [0.0, -sb, cb]];
            let rz = [[cc, sc, 0.0], [-sc, cc, 0.0], [0.0, 0.0, 1.0]];
            let ry = [[cs, 0.0, -sn], [0.0, 1.0, 0.0], [sn, 0.0, cs]];
            let mul = |p: [[f32; 3]; 3], q: [[f32; 3]; 3]| -> [[f32; 3]; 3] {
                std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| p[i][k] * q[k][j]).sum()))
            };
            let r = mul(mul(rx, ry), rz);
            std::array::from_fn(|i| if i < 3 { [r[i][0], r[i][1], r[i][2], 0.0] } else { [0.0; 4] })
        }
        _ => std::array::from_fn(|i| if i < 3 { std::array::from_fn(|_| uni(s, -1.5, 1.5)) } else { [0.0; 4] }),
    };
    m[3] = [at[0], at[1], at[2], 1.0];
    m
}

#[test]
fn obstacle_boxes() {
    let Some(mut ee) = Ee::from_ram("s03.bin") else { return eprintln!("skip: no context/iso or context/ram/s03.bin") };
    let (at, a, b) = (addr("obstacle_box"), 0x01e0_0000u32, 0x01e0_0100u32);
    let (mut seed, mut bad, mut hits, n) = (0x9e37_79b9_7f4a_7c15u64, vec![], 0, 30_000);
    for k in 0..n {
        let s = &mut seed;
        let ty = [14u8, 21, 38][k % 3];
        let e = obstacle_extents(ty);
        let o = [uni(s, -10.0, 10.0), uni(s, -3.0, 0.0), uni(s, -20.0, 20.0)];
        let r = uni(s, 0.0, 1.0) * 2.0;
        let p = [o[0] + uni(s, -1.3, 1.3) * r, o[1] + uni(s, -2.8, 2.8) * r, o[2] + uni(s, -0.5, 0.5) * r];
        let (ma, mb) = (matrix(s, p), matrix(s, o));
        for (base, m) in [(a, &ma), (b, &mb)] {
            for (i, v) in m.iter().flatten().enumerate() {
                ee.write(base + 4 * i as u32, 4, v.to_bits() as u128);
            }
        }
        ee.call(at, &[a as u64, b as u64, ty as u64], &[], 100_000);
        let (want, got) = (ee.v0() != 0, boxes_overlap(&ma, &mb, e));
        hits += want as usize;
        if want != got {
            bad.push(format!("type {ty} {ma:?} vs {mb:?}: game {want}, port {got}"));
        }
    }
    eprintln!("{hits} of {n} overlap");
    assert!(hits > n / 5 && hits < n * 4 / 5, "unbalanced sample ({hits} overlaps)");
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(5)].join("\n"));
}

#[test]
fn ball_speed() {
    let Some(mut ee) = Ee::from_ram("s03.bin") else { return eprintln!("skip: no context/iso or context/ram/s03.bin") };
    let (at, ball) = (addr("ball_kmh"), 0x01e0_0000u32);
    let (mut seed, mut bad, n) = (0x2545_f491_4f6c_dd1du64, vec![], 20_000);
    for _ in 0..n {
        let s = &mut seed;
        let v: [f32; 3] = std::array::from_fn(|_| uni(s, -1.2, 1.2) * [1.0, 0.01, 0.3][rng(s) as usize % 3]);
        for (i, c) in v.iter().enumerate() {
            ee.write(ball + 0x140 + 4 * i as u32, 4, c.to_bits() as u128);
        }
        ee.call(at, &[ball as u64], &[], 100_000);
        let (want, got) = (ee.f0(), ball_kmh(v));
        if want.to_bits() != got.to_bits() {
            bad.push(format!("{v:?}: game {want}, port {got}"));
        }
    }
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(5)].join("\n"));
}
