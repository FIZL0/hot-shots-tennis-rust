//! The serve's contact IK against the original through the oracle: the overhead (serve and smash) and underhand arm
//! tables built from the disc's motions bit-exact against every player in the RAM images, then the solves (step,
//! jump scale, joint turns) over random contact points, positions and sides. Needs the ISO, `context/iso/` and
//! `context/ram/` (skips without them).

use hst_data::{ani, iso::Iso, mdl, xb::Archive};
use hst_oracle::{Ee, addr, root};
use hst_sim::pose::{Clip, Path, Skeleton};
use hst_sim::serve::{ServeArm, overhead_solve, serve_arm, underhand_solve};

fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}
fn uni(seed: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rng(seed) as f32 / u32::MAX as f32)
}

/// The player at `pl`'s smash, overhead serve and underhand serve tables, from the costume whose arm nodes have the
/// player's rest translations.
fn tables(iso: &mut Iso, ee: &Ee, pl: u32) -> [ServeArm; 3] {
    let u = |a: u32| ee.read(a, 4) as u32;
    let c = u(pl + 0x12bc) as usize;
    let rest_of = |k: u32| [0, 4, 8].map(|o| u(u(u(u(pl + 0x17f0 + 4 * k) + 0x108) + 0x10) + 0x70 + o));
    let want = [0, 9, 11, 12, 13].map(rest_of);
    let names = ["Racket", "Bip01RHand", "Bip01RForearm", "Bip01RUpperArm", "Bip01LUpperArm"];
    let sk = (0..10)
        .filter_map(|costume| {
            let mdata = iso.read(&format!("PC/PC{c:02}C{costume:02}.XB")).ok()?;
            let marc = Archive::parse(&mdata).unwrap();
            let e = marc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl"))?;
            let m = mdl::parse(&marc.read(e).unwrap()).unwrap();
            Some(Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() })
        })
        .find(|sk| names.iter().zip(&want).all(|(n, w)| sk.names.iter().position(|x| x == n).map(|i| [0, 1, 2].map(|j| sk.rest[i][3][j].to_bits())) == Some(*w)))
        .unwrap_or_else(|| panic!("no costume of character {c} matches"));
    let data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
    let arc = Archive::parse(&data).unwrap();
    let anim = |m: usize| {
        let stem = ani::motion_name(m, c).unwrap().to_ascii_lowercase();
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).unwrap();
        ani::parse(&arc.read(e).unwrap()).unwrap()
    };
    // the smash's and the serve's jump paths (`*_s_n_dummy`, `*_serve_t_dummy`)
    [(0x1f, Some(0x30)), (0x25, Some(0x31)), (0x26, None)].map(|(m, jump)| {
        let jump = jump.and_then(|j| Path::new(&anim(j)));
        serve_arm(&sk, &Clip::new(&sk, &anim(m)), jump.as_ref(), m == 0x26)
    })
}

fn check_table(ee: &Ee, pl: u32, a: &ServeArm, i: usize, ctx: &str) {
    let bits = |at: u32, n: u32| (0..n).map(|k| ee.read(at + 4 * k, 4) as u32).collect::<Vec<_>>();
    let mbits = |m: &[[f32; 4]; 4]| m.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>();
    let (base, sh, mats) = if i < 2 {
        let b = pl + 0x200 * i as u32;
        (b, pl + 0x38c0 + 0x10 * i as u32, [0x34c0, 0x3500, 0x3540, 0x3580, 0x3600, 0x3680])
    } else {
        (pl, pl + 0x3a40, [0x3900, 0x3940, 0x3980, 0x39c0, 0x3a00, 0x3a50])
    };
    for (name, m, off) in [("racket", &a.racket, mats[0]), ("hand", &a.hand, mats[1]), ("forearm", &a.forearm, mats[2]), ("upper", &a.upper, mats[3]), ("chain", &a.chain, mats[4]), ("aimed", &a.aimed, mats[5])] {
        assert_eq!(mbits(m), bits(base + off, 16), "{ctx} table {i} {name}");
    }
    assert_eq!(a.shoulder.map(f32::to_bits).to_vec(), bits(sh, 4), "{ctx} table {i} shoulder");
    if i < 2 {
        assert_eq!(a.radius.to_bits(), ee.read(pl + 0x38e0 + 4 * i as u32, 4) as u32, "{ctx} table {i} radius");
    }
}

#[test]
fn serve_ik() {
    let Ok(mut iso) = Iso::open(root().join("Hot Shots Tennis (USA).iso")) else { return eprintln!("skip: no ISO") };
    let (over, under) = (addr("overhead_arm_solve"), addr("underhand_arm_solve"));
    let (mut seed, mut n, mut bad, mut moved, mut jumps) = (0x9e37_79b9_7f4a_7c15u64, 0, vec![], 0, 0);
    for s in ["s03", "s04", "s05", "s08", "s09"] {
        let Some(mut ee) = Ee::from_ram(&format!("{s}.bin")) else { return eprintln!("skip: no context/iso or {s}.bin") };
        let gm = ee.read(0x422f80, 4) as u32;
        let pls: Vec<u32> = (0..4).map(|p| ee.read(gm + 0xa8 + 4 * p, 4) as u32).collect();
        let tabs: Vec<_> = pls.iter().map(|&pl| tables(&mut iso, &ee, pl)).collect();
        for (p, (&pl, t)) in pls.iter().zip(&tabs).enumerate() {
            for (i, a) in t.iter().enumerate() {
                check_table(&ee, pl, a, i, &format!("{s} p{p}"));
            }
        }
        let f = |ee: &Ee, a: u32| f32::from_bits(ee.read(a, 4) as u32);
        for _ in 0..2400 {
            let sd = &mut seed;
            let p = rng(sd) as usize % 4;
            let (pl, kind) = (pls[p], rng(sd) % 3);
            let side = if rng(sd) % 2 == 0 { 1.0f32 } else { -1.0 };
            let pos = [uni(sd, -5.0, 5.0), uni(sd, -0.2, 0.2), uni(sd, -13.0, 13.0), 1.0];
            // heights are −y: from knee height to well over the head (a jump)
            let off = [uni(sd, -1.8, 1.8), uni(sd, -3.8, -0.3), uni(sd, -1.8, 1.8)];
            let contact = [pos[0] + off[0], pos[1] + off[1], pos[2] + off[2], 1.0];
            for (k, v) in pos.iter().enumerate() {
                ee.write(pl + 0x3d70 + 4 * k as u32, 4, v.to_bits() as u128);
            }
            for (k, v) in contact.iter().enumerate() {
                ee.write(pl + 0x3fb0 + 4 * k as u32, 4, v.to_bits() as u128);
            }
            ee.write(pl + 0x12b0, 4, side.to_bits() as u128);
            let scale = [side, f(&ee, pl + 0x12b4)];
            let (want_scale, got) = match kind {
                0 | 1 => {
                    ee.call(over, &[pl as u64, kind as u64], &[], 5_000_000);
                    let (s, j) = overhead_solve(&tabs[p][kind as usize], contact, pos, scale);
                    jumps += (j != 0.0) as usize;
                    (Some(j), s)
                }
                _ => {
                    ee.call(under, &[pl as u64], &[], 5_000_000);
                    (None, underhand_solve(&tabs[p][2], contact, pos, scale))
                }
            };
            let step: [f32; 4] = std::array::from_fn(|k| f(&ee, pl + 0x3fd0 + 4 * k as u32));
            let quats: [[f32; 4]; 4] = std::array::from_fn(|j| std::array::from_fn(|k| f(&ee, pl + 0x4010 + 0x10 * j as u32 + 4 * k as u32)));
            moved += (step[0] != 0.0 || step[2] != 0.0) as usize;
            let b = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            let jump_ok = want_scale.is_none_or(|j| j.to_bits() == (ee.read(pl + 0x3fcc, 4) as u32));
            if b(&step) != b(&got.step) || b(quats.as_flattened()) != b(got.quats.as_flattened()) || !jump_ok {
                bad.push(format!(
                    "{s} p{p} kind {kind} pos {pos:?} contact {contact:?} side {side}: game step {step:?} jump {} quats {quats:?}\n  port step {:?} jump {want_scale:?} quats {:?}",
                    f(&ee, pl + 0x3fcc),
                    got.step,
                    got.quats
                ));
            }
            n += 1;
        }
    }
    eprintln!("{n} solves, {moved} with a step, {jumps} with a jump, {} differ", bad.len());
    assert!(n >= 10_000 && moved > n / 10 && jumps > n / 10);
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(4)].join("\n"));
}

/// The serve IK over the swing countdown against `tools/record_serve.py` captures (`context/b29/run3/`, from the
/// slot whose RAM image built the tables): the solve at the press from the recorded contact point and position
/// gives the recorded step and jump scale, and the IK's frames (the press tick at weight 0, one a tick to contact,
/// then the ramp-down) move the server's x/z through the mover exactly as recorded.
#[test]
fn serve_ik_captures() {
    use hst_sim::{player::mover, pose::ArmIk};
    let Ok(mut iso) = Iso::open(root().join("Hot Shots Tennis (USA).iso")) else { return eprintln!("skip: no ISO") };
    const S: usize = 4 + 0x18 + 0x40 + 0x50 + 0x2c0 + 0x20 + 0xe0 + 8 + 0x10;
    let (mut serves, mut ticks) = (0, 0);
    for (file, slot) in [("A_strong", "s03"), ("C_under", "s03"), ("D_early", "s03"), ("E1_late", "s03"), ("E2_late", "s03"), ("E3_late", "s03"), ("S5_bots", "s05")] {
        let Ok(d) = std::fs::read(root().join(format!("context/b29/run3/{file}.bin"))) else { return eprintln!("skip: no {file}") };
        let Some(ee) = Ee::from_ram(&format!("{slot}.bin")) else { return eprintln!("skip: no {slot}.bin") };
        let gm = ee.read(0x422f80, 4) as u32;
        let rows: Vec<&[u8]> = d.chunks_exact(S).collect();
        let w = |r: &[u8], o: usize| u32::from_le_bytes(r[o..o + 4].try_into().unwrap());
        let pf = |r: &[u8], o: u32| f32::from_bits(w(r, 0xac + (o - 0x3d40) as usize));
        let pi = |r: &[u8], o: u32| w(r, 0xac + (o - 0x3d40) as usize) as i32;
        let mut k = 1;
        while k < rows.len() {
            // the swing press: the countdown starts (the sample after the solve's tick, n = 1)
            let (r, cd) = (rows[k], pi(rows[k], 0x3ec4));
            let motion = w(r, 0xac + 0x2c0);
            let swing = motion == 0x25 || motion == 0x26;
            if !(swing && cd > 0 && pi(r, 0x3fc4) == 1 && pi(r, 0x3fc0) == cd && pi(rows[k - 1], 0x3ec4) != cd + 1) {
                k += 1;
                continue;
            }
            let server = w(r, 0x1c + 0xc);
            let pl = ee.read(gm + 0xa8 + 4 * server, 4) as u32;
            let t = tables(&mut iso, &ee, pl);
            let side = f32::from_bits(ee.read(pl + 0x12b0, 4) as u32);
            let scale = [side, f32::from_bits(ee.read(pl + 0x12b4, 4) as u32)];
            let contact: [f32; 4] = std::array::from_fn(|j| pf(r, 0x3fb0 + 4 * j as u32));
            let pos: [f32; 4] = std::array::from_fn(|j| pf(rows[k - 1], 0x3d70 + 4 * j as u32));
            let under = motion == 0x26;
            let solve = if under {
                underhand_solve(&t[2], contact, pos, scale)
            } else {
                let (s, j) = overhead_solve(&t[1], contact, pos, scale);
                assert_eq!(j.to_bits(), w(r, 0xac + (0x3fcc - 0x3d40)), "{file} @{}: jump scale", w(r, 0));
                s
            };
            let step: Vec<u32> = (0..4).map(|j| w(r, 0xac + (0x3fd0 - 0x3d40) + 4 * j)).collect();
            assert_eq!(solve.step.map(f32::to_bits).to_vec(), step, "{file} @{}: step", w(r, 0));
            let mut ik = ArmIk { solve, frames: cd, n: 0, on: true, free: !under, pos0: pos, seen: pos };
            let mut at = pos;
            for (j, r) in rows[k..].iter().enumerate() {
                ik.tick(&mut at, |a, d| {
                    let to = mover([a[0], a[1], a[2]], [d[0], d[1], d[2]], side, None, false);
                    let free = to[0] == hst_sim::ps2::add(a[0], d[0]) && to[2] == hst_sim::ps2::add(a[2], d[2]);
                    ([to[0], to[1], to[2], a[3]], free)
                });
                let got = [pf(r, 0x3d70), pf(r, 0x3d78)];
                let ctx = format!("{file} @{} (press +{j})", w(r, 0));
                assert_eq!([at[0].to_bits(), at[2].to_bits()], got.map(f32::to_bits), "{ctx}: x/z {at:?} vs {got:?}");
                assert_eq!((ik.n, ik.frames), (pi(r, 0x3fc4), pi(r, 0x3fc0)), "{ctx}: n, frames");
                ticks += 1;
                if !ik.on {
                    break;
                }
            }
            serves += 1;
            k += cd as usize;
        }
    }
    eprintln!("{serves} serves, {ticks} ticks");
    assert!(serves >= 4);
}

