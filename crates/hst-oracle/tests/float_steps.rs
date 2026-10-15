//! The three spots C2 moved from plain IEEE f32 to the EE FPU's ops, against the original through the oracle:
//! the character speed (TParam SPE / 10, `player::Stats::new`), the shot table lookup's flight frames
//! (`shot::lookup`, whose last interpolation step truncates to whole frames) and the human run direction
//! (`player::pad_dir`, ending in the camera matrix's `+ 0`). Needs `context/iso/` and `context/ram/s03.bin` (skips
//! without them).

use std::cell::Cell;
use std::rc::Rc;

use hst_oracle::{Ee, addr};
use hst_sim::player::{Stats, pad_dir, turned};
use hst_sim::shot::{Bounds, Table, lookup};

fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}
fn uni(seed: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rng(seed) as f32 / u32::MAX as f32)
}

fn sx(v: u32) -> u64 {
    v as i32 as i64 as u64
}

/// A row's character speed: the parser runs with its text helpers hooked, every number field reading 1 except the
/// SPE column (the parser's 42nd integer read), which reads `spe`.
#[test]
fn speed_is_spe_over_ten() {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    const SPE_READ: u32 = 42;
    let (reads, value) = (Rc::new(Cell::new(0u32)), Rc::new(Cell::new(0i32)));
    let token = 0x01e0_0000u32;
    ee.write(token, 4, b'1' as u128);
    for name in ["tparam_strtok", "tparam_strtok_r"] {
        ee.hooks.insert(addr(name), Box::new(move |e: &mut Ee| e.set(2, sx(token))));
    }
    ee.hooks.insert(addr("tparam_strcmp"), Box::new(|e: &mut Ee| e.set(2, 0)));
    ee.hooks.insert(addr("tparam_memset"), Box::new(|_: &mut Ee| {}));
    ee.hooks.insert(addr("tparam_atof"), Box::new(|e: &mut Ee| e.fpr[0] = 1f32.to_bits()));
    let (r, v) = (reads.clone(), value.clone());
    ee.hooks.insert(
        addr("tparam_atoi"),
        Box::new(move |e: &mut Ee| {
            r.set(r.get() + 1);
            e.set(2, if r.get() == SPE_READ { v.get() as i64 as u64 } else { 1 });
        }),
    );
    let at = addr("tparam_row");
    let mut spes: Vec<i32> = (-2000..2000).collect();
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    // large values too: past 2^24 the int-to-float conversion itself rounds (the FPU chops)
    spes.extend((0..8000).map(|k| match k % 4 {
        0 => (rng(&mut seed) % 1_000_000) as i32,
        1 => rng(&mut seed) as i32,
        2 => ((rng(&mut seed) % (1 << 26)) as i32) - (1 << 25),
        _ => (rng(&mut seed) % 200) as i32,
    }));
    let mut bad = vec![];
    for &spe in &spes {
        reads.set(0);
        value.set(spe);
        ee.call(at, &[0], &[], 200_000);
        assert!(reads.get() > SPE_READ, "the parser read only {} numbers", reads.get());
        let (want, got) = (ee.f32_at(0x2f_091c), Stats::new(spe, 1, 1, [1, 1, 1], 0).speed);
        if want.to_bits() != got.to_bits() {
            bad.push(format!("SPE {spe}: game {want:e} ({:08x}), port {got:e} ({:08x})", want.to_bits(), got.to_bits()));
        }
    }
    assert!(bad.is_empty(), "{} of {} differ:\n{}", bad.len(), spes.len(), bad[..bad.len().min(8)].join("\n"));
}

/// The table lookup over random tables, axis bounds and hit/target pairs (both court halves). The table pointer and
/// bounds helpers are hooked to hand the game the same table and bounds as the port; the last step's truncation is
/// hooked to read its operands, so the run counts the inputs where plain IEEE arithmetic (the pre-C2 port) would
/// truncate to a different frame count, and requires some.
#[test]
fn lookup_frames() {
    let Some(mut ee) = Ee::from_ram("s03.bin") else { return eprintln!("skip: no context/iso or context/ram/s03.bin") };
    let (at, table, hit_at, tgt_at, out) = (addr("shot_lookup"), 0x01e0_0000u32, 0x01e1_0000u32, 0x01e1_0010u32, 0x01e1_0100u32);
    let bounds = Rc::new(Cell::new(Bounds::smash(0)));
    ee.hooks.insert(addr("shot_table"), Box::new(move |e: &mut Ee| e.set(2, sx(table))));
    let b = bounds.clone();
    ee.hooks.insert(
        addr("shot_bounds"),
        Box::new(move |e: &mut Ee| {
            let (b, sp) = (b.get(), e.r(29) as u32);
            let ptrs = [e.r(9) as u32, e.r(10) as u32, e.r(11) as u32, e.read(sp, 4) as u32, e.read(sp + 8, 4) as u32, e.read(sp + 16, 4) as u32];
            for (p, v) in ptrs.into_iter().zip([b.near, b.far, b.low, b.high, b.short, b.long]) {
                e.write(p, 4, v.to_bits() as u128);
            }
        }),
    );
    // (0 + f0) madd fz·(f1 − f0) in f0, fz in f27, f1 − f0 in f1, 0 + f0 in ACC; the hook does the cvt.w.s
    let last = Rc::new(Cell::new((0f32, 0f32, 0f32)));
    let l = last.clone();
    let trunc = addr("shot_frames_trunc");
    ee.hooks.insert(
        trunc,
        Box::new(move |e: &mut Ee| {
            let (f0, fz, d, acc) = (e.fpr[0], e.fpr[27], e.fpr[1], e.acc);
            l.set((f32::from_bits(acc), f32::from_bits(fz), f32::from_bits(d)));
            e.fpr[0] = (f32::from_bits(f0) as i32) as u32;
            e.set(31, sx(trunc + 4));
        }),
    );
    let presets = [
        Bounds::stroke(1, 0.0),
        Bounds::stroke(4, 0.0),
        Bounds { sloped: false, ..Bounds::stroke(0, -11.0) },
        Bounds { sloped: false, ..Bounds::stroke(2, -3.0) },
        Bounds::volley(1, 0.0),
        Bounds::serve(false, 0.064),
        Bounds::serve(true, 0.064),
        Bounds::smash(0),
        Bounds::smash(1),
    ];
    let (mut seed, mut bad, mut moved, mut n) = (0x5851_f42d_4c95_7f2du64, vec![], 0, 0);
    let mut cells = vec![0u32; 16 * 16 * 16];
    let mut t = Table::parse(&[0; 16 * 16 * 16 * 4]).unwrap();
    // one input through both; true when IEEE arithmetic would truncate the frames differently
    let mut probe = |ee: &mut Ee, t: &Table, b: Bounds, hit: [f32; 4], tgt: [f32; 4]| -> i32 {
        bounds.set(b);
        for (i, (h, t)) in hit.iter().zip(tgt).enumerate() {
            ee.write(hit_at + 4 * i as u32, 4, h.to_bits() as u128);
            ee.write(tgt_at + 4 * i as u32, 4, t.to_bits() as u128);
        }
        ee.write(0x01ff_c000, 4, hit_at as u128);
        ee.write(0x01ff_c008, 4, tgt_at as u128);
        ee.call(at, &[0x01e2_0000u64, sx(out), sx(out + 4), sx(out + 8), 1, 0, 0, 0], &[], 200_000);
        let want = (ee.f32_at(out), ee.f32_at(out + 4), ee.read(out + 8, 4) as i32);
        let l = lookup(t, &b, [hit[0], hit[1], hit[2]], [tgt[0], tgt[1], tgt[2]]);
        let (a, fz, d) = last.get();
        n += 1;
        moved += ((fz * d + a + 0.0) as i32 != want.2) as usize;
        if (l.elevation.to_bits(), l.speed.to_bits(), l.frames) != (want.0.to_bits(), want.1.to_bits(), want.2) {
            bad.push(format!("{b:?} hit {hit:?} target {tgt:?}: game {want:?}, port {:?}", (l.elevation, l.speed, l.frames)));
        }
        want.2
    };
    for k in 0..12_150 {
        let s = &mut seed;
        if k % 50 == 0 {
            // every other table has its frames rise by a fixed step per target plane (so the last step's operands
            // are exact small integers and only its rounding decides); the rest are random throughout
            let step = 1 + rng(s) % 17;
            for (i, c) in cells.iter_mut().enumerate() {
                let frames = if k % 100 == 0 { rng(s) & 0xff } else { step * (i / 256) as u32 };
                *c = frames << 24 | (rng(s) & 0xff_ffff);
                ee.write(table + 4 * i as u32, 4, *c as u128);
            }
            t = Table::parse(&cells.iter().flat_map(|c| c.to_le_bytes()).collect::<Vec<_>>()).unwrap();
        }
        let b = presets[rng(s) as usize % presets.len()];
        let side = if rng(s) & 1 == 0 { -1.0 } else { 1.0 };
        let hit = [uni(s, -6.0, 6.0), uni(s, -3.5, -0.07), side * uni(s, 0.5, 20.0), 1.0];
        let tgt = [uni(s, -5.0, 5.0), 0.0, -side * uni(s, 0.5, 13.0), 1.0];
        let frames = probe(&mut ee, &t, b, hit, tgt);
        if k < 12_000 || k % 100 < 50 {
            continue;
        }
        // Past the random inputs: on the stepped tables, the target x (in f32 bits, sideways moves the net crossing
        // smoothly) where the frame count turns over, then targets ever further below it. Just below a whole frame
        // count, IEEE rounding of the last step would reach it where the FPU's chop doesn't.
        let at_x = |x: f32| [x, tgt[1], tgt[2], 1.0];
        let (mut lo, mut hi) = (tgt[0].to_bits(), (tgt[0] + 12.0 * tgt[0].signum()).to_bits());
        let goal = probe(&mut ee, &t, b, hit, at_x(f32::from_bits(hi)));
        if goal == frames || lo >> 31 != hi >> 31 {
            continue;
        }
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if probe(&mut ee, &t, b, hit, at_x(f32::from_bits(mid))) == frames { lo = mid } else { hi = mid }
        }
        for e in 0..24 {
            probe(&mut ee, &t, b, hit, at_x(f32::from_bits(hi - (1 << e))));
        }
    }
    eprintln!("{moved} of {n} would truncate differently in IEEE arithmetic");
    assert!(n >= 10_000);
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(5)].join("\n"));
    assert!(moved > 0, "no input where the rounding moves the frame count");
}

/// The human run direction over stick byte pairs (plus d-pad buttons and play phases): the pad read turns the
/// driver's bytes into the stick floats and d-pad bits, then the run direction does the rest. Player 1's pad in the
/// slot-3 dump; its controller-mode and vibration calls are stubbed. Every other input has the camera on the +z side
/// (the game turns the direction by π; the port's `turned`).
#[test]
fn run_direction() {
    let Some(mut ee) = Ee::from_ram("s03.bin") else { return eprintln!("skip: no context/iso or context/ram/s03.bin") };
    let gm = ee.read(0x42_2f80, 4) as u32;
    let player = ee.read(gm + 0xa8, 4) as u32;
    let pad = ee.read(gm + 0x60 + 4 * ee.read(player + 0x12b8, 4) as u32, 4) as u32;
    let raw = 0x2e_fb30 + 0x48 * ee.read(pad + 4, 4) as u32 + 0x12 * ee.read(pad + 8, 4) as u32;
    for name in ["pad_mode", "pad_motor"] {
        ee.hooks.insert(addr(name), Box::new(|_: &mut Ee| {}));
    }
    let flip = Rc::new(Cell::new(false));
    let f = flip.clone();
    ee.hooks.insert(addr("pad_dir_flip"), Box::new(move |e: &mut Ee| e.set(2, f.get() as u64)));
    let (read, dir, out) = (addr("pad_read"), addr("pad_dir"), 0x01e0_0000u32);
    let (mut seed, mut bad, mut n) = (0x1405_7b7e_f767_814fu64, vec![], 0);
    // every x byte against the bytes around the centre, the |48| edges and the ends, and random ones
    const EDGES: [u8; 16] = [0x00, 0x01, 0x30, 0x4e, 0x4f, 0x50, 0x51, 0x7f, 0x80, 0x81, 0xae, 0xaf, 0xb0, 0xb1, 0xfe, 0xff];
    for k in 0..0x4000u32 {
        let s = &mut seed;
        let (lx, ly) = ((k >> 6) as u8, if k & 63 < 16 { EDGES[k as usize & 15] } else { rng(s) as u8 });
        // the four d-pad bits (active high) plus noise in the other buttons
        let buttons = (rng(s) & 0xf0 | rng(s) & 0xff0f & if k % 3 == 0 { !0 } else { 0 }) as u16;
        let phase = [2u8, 3, 4, 0, 1, 5][if k % 7 == 0 { rng(s) as usize % 6 } else { rng(s) as usize % 3 }];
        ee.write(raw, 2, buttons as u128);
        ee.write(raw + 2, 2, 0x8080);
        ee.write(raw + 4, 1, lx as u128);
        ee.write(raw + 5, 1, ly as u128);
        ee.write(pad + 0x38, 8, 0);
        flip.set(k & 1 != 0);
        ee.write(gm + 0x55, 1, phase as u128);
        ee.call(read, &[sx(pad)], &[], 100_000);
        ee.call(dir, &[sx(player), sx(out), sx(out + 0x10), sx(out + 0x14)], &[], 100_000);
        let want = [ee.f32_at(out), ee.f32_at(out + 8)];
        let got = if flip.get() { turned(pad_dir(buttons, lx, ly, phase)) } else { pad_dir(buttons, lx, ly, phase) };
        n += 1;
        if want.map(f32::to_bits) != got.map(f32::to_bits) {
            bad.push(format!("flip {} buttons {buttons:04x} stick {lx:02x},{ly:02x} phase {phase}: game {want:?}, port {got:?}", flip.get()));
        }
    }
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(8)].join("\n"));
}
