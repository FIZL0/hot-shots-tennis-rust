//! Ball bounce effects against a recorded bot match on court 10 (`context/fixtures/bounce_s05.bin`,
//! `tools/record_bounce.py 5`): from the ball's bounce count and contact records alone, every frame's marks (matrix,
//! life, length, alpha), dust puffs, the ring model's matrix, clocks, weights and life match the game bit for bit.

use hst_data::{ani, exe, iso::Iso, mdl, mor, mtl, xb::Archive};
use hst_sim::effect::{Bounce, Contact, Effect};

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v4(b: &[u8], o: usize) -> [f32; 4] {
    std::array::from_fn(|k| f(b, o + 4 * k))
}
fn bits<const N: usize>(v: [f32; N]) -> [u32; N] {
    v.map(f32::to_bits)
}

#[test]
fn bounces_s05() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Ok(mut iso), Ok(cnf), Ok(bin)) = (
        std::fs::read(format!("{root}/context/fixtures/bounce_s05.bin")),
        Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")),
        std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")),
        std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")),
    ) else {
        return eprintln!("bounce_s05.bin or disc absent, skipped");
    };
    let game = exe::Game::new(&cnf, &bin).unwrap();
    let materials = hst_sim::court::materials(&game);
    let arc_data = iso.read("AZUMA/C_EFF/EFFCT.XB0").unwrap();
    let arc = Archive::parse(&arc_data).unwrap();
    let get = |name: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(name)).map(|e| arc.read(e).unwrap());

    // header: court, per model (ring, crater) MOR/MTA entry counts; per frame vsync 8, bounce object 0xfc0, effects
    // 0x100, ball 0x290, 3 contact records 0x50, per model {model 0x80, mesh 0x60, MOR/MTA players 0x30, entries}
    let court = i(&data, 0) as usize;
    let counts: Vec<usize> = (0..4).map(|k| i(&data, 4 + 4 * k) as usize).collect();
    let msz = |k: usize| 0x140 + counts[2 * k] * 0x28 + counts[2 * k + 1] * 0x20;
    let (bo, ball, rec, m0) = (8, 8 + 0xfc0 + 0x100, 8 + 0xfc0 + 0x100 + 0x290, 8 + 0xfc0 + 0x100 + 0x290 + 0xf0);
    let size = m0 + msz(0) + msz(1);
    let frames: Vec<&[u8]> = data[20..].chunks_exact(size).collect();

    let load = |s: &str| {
        let m = mdl::parse(&get(&format!("{s}.mdl")).unwrap()).unwrap();
        let a = ani::parse(&get(&format!("{s}.ani")).unwrap()).unwrap();
        let mo = mor::parse(&get(&format!("{s}.mor")).unwrap(), 1).unwrap();
        let mt = mor::parse(&get(&format!("{s}.mta")).unwrap(), 1).unwrap();
        let mats = mtl::parse(&get(&format!("{s}.mtl")).unwrap(), get(&format!("{s}.mti")).as_deref()).unwrap().materials;
        let wt: Vec<usize> = mo.tracks.iter().filter_map(|t| m.morph_names.iter().position(|n| *n == t.name)).collect();
        (Effect::new(&m, &a, &mo, &mt, &mats), wt)
    };
    let (ring, wt) = load("bnd/ballbound_00");
    let (crater, _) = load(&format!("smash_bnd/c{court:02}/c{court:02}_chakudan"));
    assert_eq!(wt.len(), counts[0]);
    let look = game.bounce_looks()[court];
    let mut b = Bounce::new(ring, crater, look.puffs);

    let (mut marks, mut rings) = (0, 0);
    for (n, s) in frames.iter().enumerate() {
        // the whole game sometimes stands still for a few frames (ball, effects, models all unchanged): no update ran
        if n > 0 && s[8..] == frames[n - 1][8..] {
            continue;
        }
        let (o, r) = (&s[bo..bo + 0xfc0], &s[rec..rec + 0xf0]);
        let contacts: Vec<Contact> = if r.iter().all(|&x| x == 0) {
            vec![]
        } else {
            (0..3).map(|k| Contact { point: v4(r, 0x50 * k + 0x10), vel: v4(r, 0x50 * k + 0x20), normal: v4(r, 0x50 * k + 0x30), court: materials[r[0x50 * k + 0x40] as usize].court }).collect()
        };
        b.fading = o[0xd08] != 0;
        let bounces = i(&s[ball..], 0x224);
        if bounces == 1 && b.marks.len() < i(o, 0xd00) as usize {
            marks += 1;
        }
        b.tick(bounces, &contacts, false);

        assert_eq!(b.marks.len(), i(o, 0xd00) as usize, "frame {n} marks");
        for (k, m) in b.marks.iter().enumerate() {
            let g = 0x80 + 0x50 * k;
            let want: Vec<u32> = (0..16).map(|j| f(o, g + 4 * j).to_bits()).collect();
            assert_eq!(m.m.as_flattened().iter().map(|v| v.to_bits()).collect::<Vec<_>>(), want, "frame {n} mark {k} matrix");
            assert_eq!((m.life, m.len.to_bits(), m.alpha.to_bits()), (i(o, g + 0x44), f(o, g + 0x48).to_bits(), f(o, g + 0x4c).to_bits()), "frame {n} mark {k}");
        }
        assert_eq!(b.puffs.len(), i(o, 0xe68) as usize, "frame {n} puffs");
        for (k, p) in b.puffs.iter().enumerate() {
            let g = 0xd60 + 0x40 * k;
            assert_eq!(bits(p.pos), bits(v4(o, g + 0x10)), "frame {n} puff {k} pos");
            assert_eq!((p.life, p.size.to_bits(), p.alpha.to_bits()), (i(o, g), f(o, g + 0x20).to_bits(), f(o, g + 0x28).to_bits()), "frame {n} puff {k}");
        }
        assert_eq!(b.ring.live, o[0xd10] != 0, "frame {n} ring live");
        assert!(o[0xe6c] == 0 && !b.crater.live, "frame {n}: a smash landing (the port's test has none)");
        if b.ring.live {
            if b.ring.times()[1] == 0.0 {
                rings += 1;
                let want: Vec<u32> = (0..16).map(|j| f(o, 0xd20 + 4 * j).to_bits()).collect();
                assert_eq!(b.ring_at.as_flattened().iter().map(|v| v.to_bits()).collect::<Vec<_>>(), want, "frame {n} ring matrix {:?}", b.ring_at);
            }
            let p = &s[m0..];
            let t = b.ring.times();
            assert_eq!([t[0].to_bits(), t[1].to_bits()], [f(p, 0x38).to_bits(), f(p, 0xe0 + 0x1c).to_bits()], "frame {n} ring times");
            for (j, &tg) in wt.iter().enumerate() {
                assert_eq!(b.ring.weights[tg].to_bits(), f(p, 0x140 + 0x28 * j + 0x24).to_bits(), "frame {n} ring weight {j}");
            }
        }
    }
    eprintln!("{} frames, {marks} marks, {rings} rings", frames.len());
    assert!(marks >= 8 && rings >= 8);
}
