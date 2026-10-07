//! Positional sounds of a recorded bot match (`context/fixtures/sound_s05.bin`, not in git; made by
//! `tools/record_sound.py 5 …` from save-state slot 5, court 10): every ball bounce plays at the bearing and distance
//! `sound::place` gives for the ball, and every court sound the driver keyed on got the voice volume our chain gives
//! from the play's bearing and volume — `sound::stereo` into the sequence volume, the court bank's volume, the tone
//! (resolved from the key-on's ADSR and sample address) and `Level::volume`. Skips without the recording or disc.

use hst_data::{exe, iso::Iso, snd::{Bank, Level}, xb::Archive};
use hst_sim::sound;

const CONTACTS: usize = 4 + 0x290 * 2 + 0x40;
const FIX: usize = CONTACTS + 6 * 0x50;

fn u(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

struct Sample<'a> {
    ball: &'a [u8],
    /// The live ball's first 6 contact records, 0x50 bytes each: position at +0x10, material at +0x40.
    contacts: &'a [u8],
    /// The library's last positional play: bearing, distance, volume after falloff.
    play: [i32; 3],
    /// Ring commands {cmd, voice, word8, wordC} written this frame.
    cmds: Vec<[u32; 4]>,
}

fn samples(d: &[u8]) -> Vec<Sample<'_>> {
    let mut out = Vec::new();
    let mut o = 0;
    while o + FIX + 16 <= d.len() {
        let n = u(d, o + FIX + 12) as usize;
        let c = o + FIX + 16;
        if c + 16 * n > d.len() {
            break;
        }
        let cmds = (0..n).map(|i| std::array::from_fn(|k| u(d, c + 16 * i + 4 * k))).collect();
        out.push(Sample { ball: &d[o + 4..o + 4 + 0x290], contacts: &d[o + CONTACTS..o + FIX], play: std::array::from_fn(|k| u(d, o + FIX + 4 * k) as i32), cmds });
        o = c + 16 * n;
    }
    out
}

#[test]
fn positional_sounds_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Ok(mut iso)) = (std::fs::read(format!("{root}/context/fixtures/sound_s05.bin")), Iso::open(format!("{root}/Hot Shots Tennis (USA).iso"))) else {
        return eprintln!("recording or disc missing, skipped");
    };
    let elf = iso.read("SCUS_976.10").unwrap();
    let (pan, gain) = exe::sound_tables(&elf).unwrap();
    let stereo = exe::stereo_tables(&elf).unwrap();
    let bank_volume = exe::bank_volumes(&elf).unwrap()[0];
    assert_eq!(bank_volume, 118);
    let xb = iso.read("SND/COURT/C_SND10A.XB0").unwrap();
    let arc = Archive::parse(&xb).unwrap();
    let read = |name: &str| arc.read(arc.entries.iter().find(|e| e.name.replace('\\', "/").ends_with(name)).unwrap()).unwrap();
    let (hd, bd) = (read("data/sound/SE/court/co_se10.hd"), read("data/sound/SE/court/co_se10.bd"));
    let bank = Bank::parse(&hd).unwrap();
    const BASE: u32 = 0x7f1c0; // where slot 5 has co_se10 in SPU RAM (spu_s05.csv)
    let s = samples(&data);
    // one-shot key-ons (scale 0x1000) of court sounds: ADSR, sample address and volume L/R of one voice
    let keyed = |x: &Sample| -> Vec<((u16, u16), u32, [i16; 2])> {
        let mut out = Vec::new();
        for c2 in x.cmds.iter().filter(|c| c[0] == 2 && c[3] != 8) {
            let find = |cmd| x.cmds.iter().find(|c| c[0] == cmd && c[1] == c2[1]);
            if let (Some(c3), Some(c1), Some(c4)) = (find(3), find(1), find(4))
                && c4[3] & 0xffff == 0x1000
                && (BASE..BASE + bd.len() as u32).contains(&c3[2])
            {
                out.push(((c2[2] as u16, c2[3] as u16), c3[2], [c1[2] as i16, c1[3] as i16]));
            }
        }
        out
    };
    // a key-on is a tone of the bank played at `[bearing, _, volume]`
    let gives = |&(adsr, addr, want): &((u16, u16), u32, [i16; 2]), [angle, _, volume]: [i32; 3]| {
        let seq = sound::stereo(volume, angle, &stereo).map(|x| x as u32);
        (0..128).flat_map(|p| (0..128).map(move |k| (p, k))).any(|(p, k)| {
            bank.key_ons(p, k).into_iter().flatten().any(|e| {
                let (Some(t), Some(set)) = (bank.tone(e.set as usize, e.note), bank.set_volume(e.set as usize)) else { return false };
                let mut l = Level { seq, bank: bank_volume, velocity: e.velocity as u32, pan: [0x40; 3], ..Default::default() };
                l.tone(set, t, &gain);
                t.adsr() == adsr && BASE + t.sample() as u32 == addr && l.volume(&pan) == want
            })
        })
    };
    let (mut bounces, mut keys, mut missed) = (0, 0, 0);
    for w in s.windows(3) {
        let (a, b) = (&w[0], &w[1]);
        let n = u(b.ball, 0x224) as usize;
        // a live bounce (first or second) on the court (material 1) plays at the contact point, volume 0x80, keyed
        // this frame or the next (dead-ball bounces and other materials have their own keys and conditions: N3c4)
        if n > u(a.ball, 0x224) as usize && n <= 2 && b.contacts[(n - 1) * 0x50 + 0x40] == 1 {
            let pos = std::array::from_fn(|k| f32::from_bits(u(b.contacts, (n - 1) * 0x50 + 0x10 + 4 * k)));
            let (angle, dist) = sound::place(pos);
            let play = [angle, dist, sound::falloff(0x80, dist)];
            assert!(w[1..].iter().flat_map(keyed).any(|k| gives(&k, play)), "bounce {n} at {pos:?}: {play:?}");
            bounces += 1;
        }
        // the recording only holds each frame's last play; a key-on of an earlier play that frame is not checkable
        for k in keyed(b) {
            if gives(&k, b.play) || gives(&k, a.play) {
                keys += 1;
            } else {
                missed += 1;
            }
        }
    }
    eprintln!("{bounces} bounces, {keys} court key-ons ({missed} of other plays)");
    assert!(bounces >= 10 && keys >= 60 && keys > 2 * missed);
}
