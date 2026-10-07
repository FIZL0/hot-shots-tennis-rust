//! Sound banks and pitch against the game's sound state in save states (`context/fixtures/spu_s0{3,4,5}.csv` from
//! `research/tools/fixture_spu.py`): every voice the game keyed on is a tone our bank lookup resolves — same ADSR,
//! same sample in SPU RAM, same note, root and fine tune — and every voice's SPU pitch register equals our pitch.
//! Skips without the ISO or the fixtures.

use hst_data::{exe, iso::Iso, snd::{self, Bank}, xb::Archive};

#[test]
fn key_ons_and_pitch_match_save_states() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or(concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("no ISO, skipped");
    };
    let table = exe::pitch_table(&iso.read("MODULES2/SG2IOPM1.IRX").unwrap()).unwrap();
    let (mut tones, mut pitches) = (0, 0);
    for s in ["03", "04", "05"] {
        let Ok(csv) = std::fs::read_to_string(format!("{dir}/spu_s{s}.csv")) else { return eprintln!("spu_s{s}.csv absent, skipped") };
        let rows: Vec<Vec<&str>> = csv.lines().map(|l| l.split(',').collect()).collect();
        let n = |r: &[&str], i: usize| r[i].parse::<u32>().unwrap();
        let banks: Vec<(Vec<u8>, u32)> = rows
            .iter()
            .filter(|r| r[0] == "bank")
            .map(|r| {
                let xb = iso.read(r[1]).unwrap();
                let arc = Archive::parse(&xb).unwrap();
                let e = arc.entries.iter().find(|e| e.name.replace('\\', "/").ends_with(r[2])).unwrap();
                (arc.read(e).unwrap(), n(r, 3))
            })
            .collect();
        let banks: Vec<(Bank, u32)> = banks.iter().map(|(hd, base)| (Bank::parse(hd).unwrap(), *base)).collect();
        for r in rows.iter().filter(|r| r[0] == "tone") {
            let (adsr, addr, [_, fine, note, root]) = ((n(r, 2) as u16, n(r, 3) as u16), n(r, 4), n(r, 5).to_le_bytes());
            let found = banks.iter().any(|(b, base)| {
                (0..256).any(|p| {
                    (0..256).any(|k| {
                        b.key_ons(p, k).into_iter().flatten().any(|e| {
                            b.tone(e.set as usize, e.note).is_some_and(|t| {
                                e.note == note
                                    && t.root() == root
                                    && t.fine() == fine as i8
                                    && t.adsr() == adsr
                                    && base + t.sample() as u32 == addr
                            })
                        })
                    })
                })
            });
            assert!(found, "s{s}: no bank tone for {r:?}");
            tones += 1;
        }
        for r in rows.iter().filter(|r| r[0] == "pitch") {
            assert_eq!(snd::pitch(&table, n(r, 2), n(r, 3)), n(r, 4) as u16, "s{s}: {r:?}");
            pitches += 1;
        }
    }
    eprintln!("{tones} key-ons, {pitches} pitch registers");
    assert!(tones > 50 && pitches > 100);
}
