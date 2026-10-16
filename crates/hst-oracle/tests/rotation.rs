//! The swept-sphere plane test, its contact point and the rotation helpers (matrix ↔ quaternion, slerp) against
//! `hst_sim::contact` and `hst_sim::quat`, through the oracle. Needs `context/iso/` (skips without it).
//!
//! The slerp's core is a VU0 micro program, run by the interpreter's micro mode (pipeline latencies and flag timing
//! included) from the micro memory the game's boot DMA chain fills; it is checked bare (as the motion sampler's squad
//! calls it) and inside the wrapper (t = 0/1 shortcuts, the shortest-arc flip and its dot).

use std::f32::consts::PI;

use hst_oracle::{Ee, HstEe, addr};
use hst_sim::contact::{Hit, contact_point, sweep};
use hst_sim::quat::{self, M4};

type V4 = [f32; 4];

fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}
fn uni(seed: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rng(seed) as f32 / u32::MAX as f32)
}

const A: u32 = 0x01e0_0000;
const B: u32 = 0x01e0_0100;
const C: u32 = 0x01e0_0200;
const D: u32 = 0x01e0_0300;
const E: u32 = 0x01e0_0400;

fn put(ee: &mut Ee, at: u32, v: &[f32]) {
    for (i, c) in v.iter().enumerate() {
        ee.write(at + 4 * i as u32, 4, c.to_bits() as u128);
    }
}
fn get<const N: usize>(ee: &Ee, at: u32) -> [f32; N] {
    std::array::from_fn(|i| ee.f32_at(at + 4 * i as u32))
}
fn bits<const N: usize>(v: [f32; N]) -> [u32; N] {
    v.map(f32::to_bits)
}

fn report(what: &str, n: usize, bad: &[String]) {
    assert!(bad.is_empty(), "{what}: {} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(8)].join("\n"));
}

/// A unit normal: axis-aligned (with the signed zeros the ball code passes) or any direction.
fn normal(s: &mut u64) -> V4 {
    match rng(s) % 4 {
        0 => [-0.0, -1.0, -0.0, -0.0],
        1 => [0.0, 0.0, if rng(s) & 1 == 0 { 1.0 } else { -1.0 }, 0.0],
        _ => {
            let v: [f32; 3] = std::array::from_fn(|_| uni(s, -1.0, 1.0));
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-3);
            [v[0] / l, v[1] / l, v[2] / l, 0.0]
        }
    }
}

/// A sweep near the plane: start at a signed distance around the radius, a short move mostly along the normal.
fn sweep_case(s: &mut u64) -> (V4, V4, f32, V4, V4, f32) {
    let r = [0.033, 0.0335, 0.1, uni(s, 0.01, 0.6)][rng(s) as usize % 4];
    let n = normal(s);
    let point = [uni(s, -12.0, 12.0), uni(s, -1.0, 3.0), uni(s, -25.0, 25.0), 1.0];
    // signed distance along the normal: a hit needs the start at 0.98r or more and the end within the skin
    let d0 = r * match rng(s) % 3 {
        0 => uni(s, 0.95, 1.02),
        1 => uni(s, 0.9, 2.0),
        _ => uni(s, -1.3, 3.0),
    };
    let tan: [f32; 3] = std::array::from_fn(|_| uni(s, -0.5, 0.5));
    let tn = tan[0] * n[0] + tan[1] * n[1] + tan[2] * n[2];
    let tan = [0, 1, 2].map(|k| tan[k] - n[k] * tn);
    let start = [0, 1, 2].map(|k| point[k] + n[k] * d0 + tan[k]);
    let along = match rng(s) % 4 {
        0 => r * uni(s, -0.01, 0.01),
        1 => r * uni(s, -3.0, 0.2),
        2 => 0.0,
        _ => uni(s, -0.5, 0.05),
    };
    // sideways moves (in the plane) or a little across it
    let side: [f32; 3] = std::array::from_fn(|_| uni(s, -0.2, 0.2) * (rng(s) & 1) as f32);
    let across = (side[0] * n[0] + side[1] * n[1] + side[2] * n[2]) * if rng(s) % 4 == 0 { 0.9 } else { 1.0 };
    let side = [0, 1, 2].map(|k| side[k] - n[k] * across);
    let end = [0, 1, 2].map(|k| start[k] + n[k] * along + side[k]);
    let best = match rng(s) % 4 {
        0 | 1 => f32::MAX,
        2 => uni(s, 0.0, 1.0),
        _ => [0.0, -1.0, 1.0, -0.5][rng(s) as usize % 4],
    };
    let w = |v: [f32; 3]| [v[0], v[1], v[2], 1.0];
    (w(start), w(end), r, point, n, best)
}

#[test]
fn plane_sweep() {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    let at = addr("plane_sweep");
    let (mut seed, mut bad, n) = (0x9e37_79b9_7f4a_7c15u64, vec![], 40_000);
    let mut kinds = [0usize; 3]; // miss, t > 0, inside the skin (t = 0, penetration < 0)
    for _ in 0..n {
        let (start, end, r, point, nrm, best) = sweep_case(&mut seed);
        // the hit record: t = the best so far, penetration 0 (as the callers clear it), the rest stale
        let rec: Vec<f32> = [best, 0.0].into_iter().chain((2..17).map(|k| k as f32 * 0.25)).collect();
        put(&mut ee, A, &rec);
        put(&mut ee, B, &[start[0], start[1], start[2], start[3], r]);
        put(&mut ee, C, &end);
        put(&mut ee, D, &point);
        put(&mut ee, E, &nrm);
        ee.call(at, &[A as u64, B as u64, C as u64, D as u64, E as u64], &[], 100_000);
        let got = sweep(start, end, r, point, nrm, best);
        let after: [f32; 17] = get(&ee, A);
        let case = format!("start {start:?} end {end:?} r {r} point {point:?} n {nrm:?} best {best}");
        match (ee.v0() as u32 != 0, got) {
            (false, None) => {
                kinds[0] += 1;
                if bits(after)[..] != rec.iter().map(|c| c.to_bits()).collect::<Vec<_>>()[..] {
                    bad.push(format!("{case}: miss wrote the record"));
                }
            }
            (true, Some(h)) => {
                kinds[if h.t > 0.0 { 1 } else { 2 }] += 1;
                let want = (after[0], after[1], [after[4], after[5], after[6], after[7]], [after[12], after[13], after[14], after[15]]);
                if (bits([want.0, want.1]), bits(want.2), bits(want.3)) != (bits([h.t, h.penetration]), bits(h.centre), bits(h.normal)) {
                    bad.push(format!("{case}: game {want:?} port {h:?}"));
                }
            }
            (g, p) => bad.push(format!("{case}: game hit {g}, port {p:?}")),
        }
    }
    eprintln!("miss / t>0 / inside the skin: {kinds:?}");
    assert!(kinds.iter().all(|&k| k > n / 50), "a branch is barely reached: {kinds:?}");
    report("plane sweep", n, &bad);
}

#[test]
fn contact_points() {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    let at = addr("contact_point");
    let (mut seed, mut bad, n) = (0x2545_f491_4f6c_dd1du64, vec![], 40_000);
    let mut kinds = [0usize; 5]; // no hit, pushed out, t == 0, held back to start, lerped
    for _ in 0..n {
        let s = &mut seed;
        let (start, mut end, r, point, nrm, _) = sweep_case(s);
        // hits from the sweep itself, plus made-up ones for the branches it rarely gives
        let hit = match rng(s) % 6 {
            0 => None,
            1 => Some(Hit { t: uni(s, 0.0, 1.0), penetration: 0.0, centre: start, normal: nrm }),
            2 => Some(Hit { t: uni(s, 0.0, 1e-3) * (rng(s) & 1) as f32, penetration: -uni(s, 0.0, 0.5) * (rng(s) & 1) as f32, centre: start, normal: nrm }),
            3 => {
                // a step that does not move on some axes (copied through), or barely moves (held back ≤ 0)
                for k in 0..3 {
                    if rng(s) & 1 == 0 {
                        end[k] = start[k];
                    }
                }
                Some(Hit { t: uni(s, 0.0, 1.0) * [1.0, 1e-4][rng(s) as usize % 2], penetration: 0.0, centre: start, normal: nrm })
            }
            _ => sweep(start, end, r, point, nrm, f32::MAX),
        };
        let mut rec = [0.0f32; 17];
        match &hit {
            None => rec[0] = f32::MAX,
            Some(h) => {
                (rec[0], rec[1]) = (h.t, h.penetration);
                rec[4..8].copy_from_slice(&h.centre);
                rec[12..16].copy_from_slice(&h.normal);
            }
        }
        put(&mut ee, A, &[0.5; 4]);
        put(&mut ee, B, &rec);
        put(&mut ee, C, &[start[0], start[1], start[2], start[3], r]);
        put(&mut ee, D, &end);
        ee.call(at, &[A as u64, B as u64, C as u64, D as u64], &[], 100_000);
        let (want, got): (V4, V4) = (get(&ee, A), contact_point(hit.as_ref(), start, end, r));
        kinds[match &hit {
            None => 0,
            Some(h) if h.penetration < 0.0 => 1,
            Some(h) if h.t == 0.0 => 2,
            Some(_) if got[..3] == start[..3] => 3,
            _ => 4,
        }] += 1;
        if bits(want) != bits(got) {
            bad.push(format!("{hit:?} start {start:?} end {end:?} r {r}: game {want:?} port {got:?}"));
        }
    }
    eprintln!("no hit / pushed out / t=0 / held to start / lerped: {kinds:?}");
    assert!(kinds.iter().all(|&k| k > n / 200), "a branch is barely reached: {kinds:?}");
    report("contact point", n, &bad);
}

/// A rotation of `angle` about a random axis (angles near π reach the largest-diagonal branches), random rows,
/// or a rotation scaled/perturbed like an accumulated bone matrix.
fn matrix(s: &mut u64) -> M4 {
    let rot = |s: &mut u64, angle: f32| -> M4 {
        let v: [f32; 3] = std::array::from_fn(|_| uni(s, -1.0, 1.0));
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-3);
        let (x, y, z) = (v[0] / l, v[1] / l, v[2] / l);
        let (sn, cs) = angle.sin_cos();
        let t = 1.0 - cs;
        [
            [t * x * x + cs, t * x * y + sn * z, t * x * z - sn * y, 0.0],
            [t * x * y - sn * z, t * y * y + cs, t * y * z + sn * x, 0.0],
            [t * x * z + sn * y, t * y * z - sn * x, t * z * z + cs, 0.0],
            [uni(s, -5.0, 5.0), uni(s, 0.0, 2.0), uni(s, -5.0, 5.0), 1.0],
        ]
    };
    match rng(s) % 5 {
        0 => rot(s, uni(&mut s.clone(), -0.5, 0.5)),
        1 => rot(s, uni(&mut s.clone(), 2.6, PI)),
        2 => rot(s, uni(&mut s.clone(), -PI, PI)),
        3 => {
            let mut m = rot(s, uni(&mut s.clone(), -PI, PI));
            let k = uni(s, 0.98, 1.02);
            for row in &mut m[..3] {
                for c in &mut row[..3] {
                    *c = *c * k + uni(s, -1e-3, 1e-3);
                }
            }
            m
        }
        _ => std::array::from_fn(|i| if i < 3 { std::array::from_fn(|j| if j < 3 { uni(s, -1.5, 1.5) } else { 0.0 }) } else { [0.0, 0.0, 0.0, 1.0] }),
    }
}

#[test]
fn matrix_to_quat() {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    let at = addr("quat_from_matrix");
    let (mut seed, mut bad, n) = (0x51_7cc1_b727_220au64, vec![], 30_000);
    let mut kinds = [0usize; 4]; // trace ≥ 0, largest m00, m11, m22
    for _ in 0..n {
        let m = matrix(&mut seed);
        put(&mut ee, B, m.as_flattened());
        ee.call(at, &[A as u64, B as u64], &[], 100_000);
        let (want, got): (V4, V4) = (get(&ee, A), quat::from_matrix(&m));
        let (d, tr) = ([m[0][0], m[1][1], m[2][2]], m[0][0] + m[1][1] + m[2][2]);
        kinds[if tr >= 0.0 { 0 } else if d[0] >= d[1] && d[0] >= d[2] { 1 } else if d[1] >= d[2] { 2 } else { 3 }] += 1;
        if bits(want) != bits(got) {
            bad.push(format!("{m:?}: game {want:?} port {got:?}"));
        }
    }
    eprintln!("trace / m00 / m11 / m22: {kinds:?}");
    assert!(kinds.iter().all(|&k| k > n / 50), "a branch is barely reached: {kinds:?}");
    report("matrix → quat", n, &bad);
}

fn unit_quat(s: &mut u64) -> V4 {
    let v: V4 = std::array::from_fn(|_| uni(s, -1.0, 1.0));
    let l = v.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-3);
    v.map(|c| c / l)
}

#[test]
fn quat_to_matrix() {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    let at = addr("quat_to_matrix");
    let (mut seed, mut bad, n) = (0x6a09_e667_f3bc_c908u64, vec![], 30_000);
    for k in 0..n {
        let s = &mut seed;
        let q = match k % 5 {
            0 | 1 => unit_quat(s),
            2 => unit_quat(s).map(|c| c * uni(s, 0.5, 2.0)),
            3 => [0.0, 0.0, 0.0, if k % 2 == 0 { 1.0 } else { -1.0 }].map(|c: f32| c + uni(s, -1e-3, 1e-3)),
            _ => std::array::from_fn(|_| uni(s, -3.0, 3.0)),
        };
        put(&mut ee, B, &q);
        ee.call(at, &[A as u64, B as u64], &[], 100_000);
        let (want, got): ([f32; 16], M4) = (get(&ee, A), quat::to_matrix(q));
        if bits(want) != bits(*got.as_flattened().first_chunk::<16>().unwrap()) {
            bad.push(format!("{q:?}: game {want:?} port {got:?}"));
        }
    }
    report("quat → matrix", n, &bad);
}

#[test]
fn quat_slerp() {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    let at = addr("quat_slerp");
    let (mut seed, mut bad, n) = (0xbb67_ae85_84ca_a73bu64, vec![], 30_000);
    let mut kinds = [0usize; 4]; // t = 0, t = 1, flipped (dot < 0), as is
    for k in 0..n {
        let s = &mut seed;
        let a = unit_quat(s);
        let b = match k % 5 {
            0 => a.map(|c| -c),                                  // opposite
            1 => a.map(|c| c + uni(s, -1e-3, 1e-3)),             // nearly equal (the lerp fallback)
            2 => a.map(|c| -c + uni(s, -0.05, 0.05)),            // nearly opposite
            _ => unit_quat(s),
        };
        let t = match rng(s) % 6 {
            0 => 0.0,
            1 => 1.0,
            2 => [-0.0, 0.5, f32::from_bits(1), 1.0 - f32::EPSILON][rng(s) as usize % 4],
            _ => uni(s, 0.0, 1.0),
        };
        put(&mut ee, B, &a);
        put(&mut ee, C, &b);
        ee.call(at, &[A as u64, B as u64, C as u64], &[t], 100_000);
        let (want, got): (V4, V4) = (get(&ee, A), quat::slerp(a, b, t));
        let dot: f32 = (0..4).map(|i| a[i] * b[i]).sum();
        kinds[if t == 0.0 { 0 } else if t == 1.0 { 1 } else if dot < 0.0 { 2 } else { 3 }] += 1;
        if t == 0.0 && bits(got) != bits(a) || t == 1.0 && bits(got) != bits(b) {
            bad.push(format!("{a:?} → {b:?} at {t}: the end shortcut gave {got:?}"));
        }
        if bits(want) != bits(got) {
            bad.push(format!("{a:?} → {b:?} at {t}: game {want:?} port {got:?}"));
        }
    }
    eprintln!("t=0 / t=1 / flipped / as is: {kinds:?}");
    assert!(kinds.iter().all(|&k| k > n / 20), "a branch is barely reached: {kinds:?}");
    report("slerp", n, &bad);
}

/// The micro program called bare: its three lerp fallbacks (dot negative — a −0 dot included, the S flag is the
/// sign bit —, sin²θ = 0, dot > 0.999) and the arctan/sine polynomial path, on unit and off-unit quaternions.
#[test]
fn quat_micro_slerp() {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    let at = addr("quat_micro_slerp");
    let (mut seed, mut bad, n) = (0x3c6e_f372_fe94_f82bu64, vec![], 30_000);
    let mut kinds = [0usize; 3]; // negative dot, nearly equal (dot ≥ 0.999), the polynomial path
    for k in 0..n {
        let s = &mut seed;
        let a = unit_quat(s);
        let b = match k % 8 {
            0 => a.map(|c| -c + uni(s, -0.3, 0.3)),
            1 => a.map(|c| c + uni(s, -1e-3, 1e-3)),
            2 => a,
            // products that are all −0 (or +0): the dot's sign bit alone decides
            3 => std::array::from_fn(|i| if a[i] == 0.0 { 1.0 } else { [-0.0, 0.0][rng(s) as usize % 2] }),
            4 => unit_quat(s).map(|c| c * uni(s, 0.5, 1.5)),
            _ => unit_quat(s),
        };
        let a = if k % 8 == 3 { [a[0], 0.0, a[2], -0.0] } else { a };
        let t = match rng(s) % 8 {
            0 => [0.0, 1.0, -0.0, 0.5][rng(s) as usize % 4],
            1 => uni(s, -0.5, 1.5),
            _ => uni(s, 0.0, 1.0),
        };
        put(&mut ee, B, &a);
        put(&mut ee, C, &b);
        ee.call(at, &[A as u64, B as u64, C as u64], &[t], 1_000);
        let (want, got): (V4, V4) = (get(&ee, A), quat::micro_slerp(a, b, t));
        let dot: f32 = (0..4).map(|i| a[i] * b[i]).sum();
        kinds[if dot < 0.0 { 0 } else if dot >= 0.999 { 1 } else { 2 }] += 1;
        if bits(want) != bits(got) {
            bad.push(format!("{a:?} → {b:?} at {t}: game {want:?} port {got:?}"));
        }
    }
    eprintln!("negative / nearly equal / polynomial: {kinds:?}");
    assert!(kinds.iter().all(|&k| k > n / 20), "a branch is barely reached: {kinds:?}");
    report("micro slerp", n, &bad);
}
