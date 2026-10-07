//! Face morph weights against the five save-state RAM dumps (`context/ram/s0N.bin`): every live morph player
//! (ticks per frame 80, sampled at least once) holds, per bound track, the weight `face::sample` gives at its last
//! sampled time, and the key cursor. Also every `.MOR` on the disc parses and binds to its model's targets.

use hst_sim::face::{length, sample};

fn u(d: &[u8], a: u32) -> u32 {
    let a = (a & 0x1ff_ffff) as usize;
    u32::from_le_bytes(d[a..a + 4].try_into().unwrap())
}
fn f(d: &[u8], a: u32) -> f32 {
    f32::from_bits(u(d, a))
}

#[test]
fn morph_weights_ram() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let (mut players, mut weights) = (0, 0);
    for s in ["s03", "s04", "s05", "s08", "s09"] {
        let Ok(ram) = std::fs::read(format!("{root}context/ram/{s}.bin")) else { return eprintln!("{s}.bin absent, skipped") };
        // the morph player: +8 count, +0xc bindings, +0x10 ticks per frame, +0x14 length, +0x1c last sampled time,
        // +0x28 weight; a binding: +0x14 track (+0 header, +4 keys; +0xc ticks, +0x10 values), +0x20 cursor, +0x24 weight
        for a in (0..ram.len() as u32 - 0x30).step_by(4) {
            let (n, list) = (u(&ram, a + 8), u(&ram, a + 0xc));
            if ram[a as usize] != 1 || u(&ram, a + 0x10) != 0x42a0_0000 || f(&ram, a + 0x28) != 1.0 || !(1..40).contains(&n) || !(0x10_0000..0x200_0000).contains(&list) {
                continue;
            }
            let mut tracks = Vec::new();
            for i in 0..n {
                let b = u(&ram, list + 4 * i);
                let tr = u(&ram, b + 0x14);
                let keys = u(&ram, u(&ram, tr) + 4);
                let ticks: Vec<i32> = (0..keys).map(|k| u(&ram, u(&ram, tr + 0xc) + 4 * k) as i32).collect();
                let values: Vec<[f32; 4]> = (0..keys).map(|k| [f(&ram, u(&ram, tr + 0x10) + 4 * k), 0.0, 0.0, 0.0]).collect();
                tracks.push((b, ticks, values));
            }
            assert_eq!(length(tracks.iter().map(|t| &t.1[..]), 80), f(&ram, a + 0x14), "{s} {a:#x} length");
            let last = f(&ram, a + 0x1c);
            if last < 0.0 {
                continue; // never sampled
            }
            for (b, ticks, values) in &tracks {
                let mut cur = u(&ram, b + 0x20) as usize;
                let w = sample(ticks, values, hst_sim::ps2::mul(last, 80.0), &mut cur, false)[0];
                assert_eq!((w.to_bits(), cur), (f(&ram, b + 0x24).to_bits(), u(&ram, b + 0x20) as usize), "{s} {a:#x} binding {b:#x} t {last}");
                weights += 1;
            }
            players += 1;
        }
    }
    eprintln!("{players} morph players, {weights} weights bit-exact");
    assert!(weights > 10);
}

#[test]
fn every_morph_binds() {
    use hst_data::{iso::Iso, mdl, mor, xb::Archive};
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
    let (mut files, mut tracks, mut unbound) = (0, 0, 0);
    for c in 0..10 {
        let data = iso.read(&format!("PC/PC{c:02}C00.XB")).unwrap();
        let arc = Archive::parse(&data).unwrap();
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().contains(&format!("pc{c:02}_t")) && e.name.to_ascii_lowercase().ends_with("_c00.mdl")).unwrap();
        let m = mdl::parse(&arc.read(e).unwrap()).unwrap();
        // every packet's morph blocks: one per target, entries inside the packet
        for pk in m.materials.iter().flatten().filter(|p| !p.morphs.is_empty()) {
            assert_eq!(pk.morphs.len(), m.morph_names.len());
            assert!(pk.morphs.iter().flatten().all(|&(i, _)| i < pk.vertices.len()));
        }
        let data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
        let arc = Archive::parse(&data).unwrap();
        for e in arc.entries.iter().filter(|e| e.name.to_ascii_uppercase().ends_with(".MOR")) {
            let t = mor::parse(&arc.read(e).unwrap(), 1).unwrap_or_else(|err| panic!("{}: {err}", e.name));
            assert_eq!(t.ticks_per_frame, 80, "{}", e.name);
            for tr in &t.tracks {
                // the game binds by exact name: a few tracks name targets the model lacks or carry trailing spaces
                if !m.morph_names.iter().any(|n| n == &tr.name) {
                    unbound += 1;
                    continue;
                }
                assert!(tr.ticks.windows(2).all(|w| w[0] < w[1]) && tr.ticks.len() == tr.values.len(), "{}", e.name);
                tracks += 1;
            }
            files += 1;
        }
    }
    eprintln!("{files} MOR files, {tracks} tracks bound, {unbound} not");
    assert!(files > 300 && unbound * 20 < tracks);
}
