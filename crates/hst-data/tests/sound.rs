//! Sound banks, pitch and voices against the game's sound state in save states (`context/fixtures/spu_s0{3,4,5}.csv`
//! from `research/tools/fixture_spu.py`): every voice the game keyed on is a tone our bank lookup resolves — same
//! ADSR, same sample in SPU RAM, same note, root and fine tune — every voice's SPU pitch register equals our pitch,
//! every sounding voice stepped from its key-on with our `Voice` reaches PCSX2's decoder and envelope state, and
//! every voice's L/R volume is our `Level`. Skips without the ISO or the fixtures.

use hst_data::{exe, iso::Iso, snd::{self, Bank, Level, Voice}, xb::Archive};

#[test]
fn key_ons_pitch_and_voices_match_save_states() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or(concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("no ISO, skipped");
    };
    let table = exe::pitch_table(&iso.read("MODULES2/SG2IOPM1.IRX").unwrap()).unwrap();
    let (pan, gain) = exe::sound_tables(&iso.read("SCUS_976.10").unwrap()).unwrap();
    assert_eq!(snd::gaussian()[0], [0x12c7, 0x59b3, 0x1307, -1]);
    assert_eq!(snd::gaussian()[128], [0x019c, 0x3def, 0x3e4c, 0x01a8]);
    assert_eq!(snd::gaussian()[255], [-1, 0x1307, 0x59b3, 0x12c7]);
    let (mut tones, mut pitches, mut voices, mut levels) = (0, 0, 0, 0);
    for s in ["03", "04", "05"] {
        let Ok(csv) = std::fs::read_to_string(format!("{dir}/spu_s{s}.csv")) else { return eprintln!("spu_s{s}.csv absent, skipped") };
        let rows: Vec<Vec<&str>> = csv.lines().map(|l| l.split(',').collect()).collect();
        let n = |r: &[&str], i: usize| r[i].parse::<u32>().unwrap();
        let int = |r: &[&str], i: usize| r[i].parse::<i32>().unwrap();
        let banks: Vec<(Vec<u8>, Vec<u8>, u32)> = rows
            .iter()
            .filter(|r| r[0] == "bank")
            .map(|r| {
                let xb = iso.read(r[1]).unwrap();
                let arc = Archive::parse(&xb).unwrap();
                let read = |name: &str| arc.read(arc.entries.iter().find(|e| e.name.replace('\\', "/").ends_with(name)).unwrap()).unwrap();
                (read(r[2]), read(&r[2].replace(".hd", ".bd")), n(r, 3))
            })
            .collect();
        let banks: Vec<(Bank, &[u8], u32)> = banks.iter().map(|(hd, bd, base)| (Bank::parse(hd).unwrap(), &bd[..], *base)).collect();
        // per voice: the bank tones its last key-on can be, with their set volume
        let mut keyed = std::collections::HashMap::new();
        for r in rows.iter().filter(|r| r[0] == "tone") {
            let (adsr, addr, [_, fine, note, root]) = ((n(r, 2) as u16, n(r, 3) as u16), n(r, 4), n(r, 5).to_le_bytes());
            let mut found = Vec::new();
            for (b, _, base) in &banks {
                for (p, k) in (0..256).flat_map(|p| (0..256).map(move |k| (p, k))) {
                    for e in b.key_ons(p, k).into_iter().flatten() {
                        let Some(t) = b.tone(e.set as usize, e.note) else { continue };
                        if e.note == note && t.root() == root && t.fine() == fine as i8 && t.adsr() == adsr && base + t.sample() as u32 == addr {
                            found.push((b.set_volume(e.set as usize).unwrap(), t));
                        }
                    }
                }
            }
            assert!(!found.is_empty(), "s{s}: no bank tone for {r:?}");
            keyed.insert(r[1], found);
            tones += 1;
        }
        for r in rows.iter().filter(|r| r[0] == "pitch") {
            assert_eq!(snd::pitch(&table, n(r, 2), n(r, 3)), n(r, 4) as u16, "s{s}: {r:?}");
            pitches += 1;
        }
        for r in rows.iter().filter(|r| r[0] == "spu") {
            let (pitch, ssa, sp, read) = (n(r, 7), 2 * n(r, 4), n(r, 13), n(r, 15));
            // steps since key-on; a voice whose pitch changed meanwhile cannot be replayed
            if (read * 4096 + sp) % pitch != 0 {
                continue;
            }
            let (_, bd, base) = banks.iter().find(|(_, bd, base)| (*base..base + bd.len() as u32).contains(&ssa)).unwrap();
            assert_eq!(base % 16, 0);
            let h = base / 2;
            let mut v = Voice::key_on(ssa / 2 - h, (n(r, 2) as u16, n(r, 3) as u16), pitch as u16, [n(r, 18) as i16, n(r, 19) as i16]);
            for _ in 0..(read * 4096 + sp) / pitch {
                v.tick(bd);
            }
            let got = (v.loop_start + h, v.next + h, v.phase as u32, v.level, v.counter, v.prev, v.sp, v.write, v.read, v.flags as u32, v.out);
            let want = (n(r, 5), n(r, 6), n(r, 8), int(r, 9), n(r, 10), [int(r, 11), int(r, 12)], sp, n(r, 14), read, n(r, 16), int(r, 17));
            assert_eq!(got, want, "s{s}: voice {}", r[1]);
            assert_eq!(v.fifo.to_vec(), (20..52).map(|i| int(r, i)).collect::<Vec<_>>(), "s{s}: voice {} queue", r[1]);
            voices += 1;
        }
        for r in rows.iter().filter(|r| r[0] == "level") {
            let l = Level {
                seq: [n(r, 2), n(r, 3)],
                bank: n(r, 4),
                tone: n(r, 5),
                velocity: n(r, 6),
                pan: [n(r, 7), n(r, 8), n(r, 9)],
                centre: n(r, 10) != 0,
                gain: [n(r, 11), n(r, 12)],
            };
            let volume = [n(r, 13) as i16, n(r, 14) as i16];
            assert_eq!(l.volume(&pan), volume, "s{s}: {r:?}");
            if let Some(v) = rows.iter().find(|v| v[0] == "spu" && v[1] == r[1]) {
                assert_eq!([n(v, 18) as i16, n(v, 19) as i16], volume, "s{s}: voice {} registers", r[1]);
            }
            let found = &keyed[r[1]];
            assert!(found.iter().any(|&(set, t)| {
                let mut m = l;
                m.tone(set, t, &gain);
                m == l
            }), "s{s}: no bank tone gives {r:?}");
            levels += 1;
        }
    }
    eprintln!("{tones} key-ons, {pitches} pitch registers, {voices} voices, {levels} volumes");
    assert!(tones > 50 && pitches > 100 && voices >= 10 && levels >= 10);
}
