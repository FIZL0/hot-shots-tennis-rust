//! Positional sounds of a recorded bot match (`context/fixtures/sound_s05.bin`, not in git; made by
//! `tools/record_sound.py 5 …` from save-state slot 5, court 10): every ball bounce plays at the bearing and distance
//! `sound::place` gives for the ball, and every court sound the driver keyed on got the voice volume our chain gives
//! from the play's bearing and volume — `sound::stereo` into the sequence volume, the court bank's volume, the tone
//! (resolved from the key-on's ADSR and sample address) and `Level::volume`. Skips without the recording or disc.

use hst_data::{exe, iso::Iso, snd::{Bank, Level}, xb::Archive};
use hst_sim::sound::{self, Bounces};

const CONTACTS: usize = 4 + 0x290 * 2 + 0x40;
const FIX: usize = CONTACTS + 6 * 0x50;

fn u(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

struct Sample<'a> {
    ball: &'a [u8],
    /// The rally block from 0x3165f0 (serve faults at +0x18).
    rally: &'a [u8],
    /// The live ball's first 6 contact records, 0x50 bytes each: position at +0x10, material at +0x40.
    contacts: &'a [u8],
    /// The library's last positional play: bearing, distance, volume after falloff.
    play: [i32; 3],
    /// `hits_s05.bin` only: the hit state block `tools/record_sound.py` appends after the play.
    hit: &'a [u8],
    /// Ring commands {cmd, voice, word8, wordC} written this frame.
    cmds: Vec<[u32; 4]>,
}

/// `extra` is the size of the block between the play and the command count (0, or 0x330 with hit state).
fn samples(d: &[u8], extra: usize) -> Vec<Sample<'_>> {
    let mut out = Vec::new();
    let mut o = 0;
    while o + FIX + 16 + extra <= d.len() {
        let n = u(d, o + FIX + 12 + extra) as usize;
        let c = o + FIX + 16 + extra;
        if c + 16 * n > d.len() {
            break;
        }
        let cmds = (0..n).map(|i| std::array::from_fn(|k| u(d, c + 16 * i + 4 * k))).collect();
        out.push(Sample { ball: &d[o + 4..o + 4 + 0x290], rally: &d[o + 4 + 0x520..CONTACTS + o], contacts: &d[o + CONTACTS..o + FIX], play: std::array::from_fn(|k| u(d, o + FIX + 4 * k) as i32), hit: &d[o + FIX + 12..c - 4], cmds });
        o = c + 16 * n;
    }
    out
}

type Key = ((u16, u16), u32, [i16; 2]);

/// The disc's tables and court 10's bank, as slot 5 has it in SPU RAM.
struct Court {
    pan: Vec<u16>,
    gain: Vec<u16>,
    stereo: [Vec<i32>; 2],
    bank_volume: u32,
    hd: Vec<u8>,
    bd_len: u32,
}

const BASE: u32 = 0x7f1c0; // where slot 5 has co_se10 in SPU RAM (spu_s05.csv)

impl Court {
    /// The recording `name` and the court; `None` (test skipped) without either.
    fn load(name: &str) -> Option<(Vec<u8>, Self)> {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let (Ok(data), Ok(mut iso)) = (std::fs::read(format!("{root}/context/fixtures/{name}")), Iso::open(format!("{root}/Hot Shots Tennis (USA).iso"))) else {
            eprintln!("recording or disc missing, skipped");
            return None;
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
        Some((data, Self { pan, gain, stereo, bank_volume, hd, bd_len: bd.len() as u32 }))
    }

    /// One-shot key-ons of court sounds: ADSR, sample address and volume L/R of one voice. Without `next` only
    /// those at play speed 1 (scale 0x1000); with it any speed, also with the L/R the voice gets in `next`.
    fn keyed(&self, x: &Sample, next: Option<&Sample>) -> Vec<Key> {
        let mut out = Vec::new();
        for c2 in x.cmds.iter().filter(|c| c[0] == 2 && c[3] != 8) {
            let find = |cmd| x.cmds.iter().find(|c| c[0] == cmd && c[1] == c2[1]);
            if let (Some(c3), Some(c1), Some(c4)) = (find(3), find(1), find(4))
                && (next.is_some() || c4[3] & 0xffff == 0x1000)
                && (BASE..BASE + self.bd_len).contains(&c3[2])
            {
                let later = next.and_then(|n| n.cmds.iter().find(|c| c[0] == 1 && c[1] == c2[1]));
                for c1 in [Some(c1), later].into_iter().flatten() {
                    out.push(((c2[2] as u16, c2[3] as u16), c3[2], [c1[2] as i16, c1[3] as i16]));
                }
            }
        }
        out
    }

    /// A key-on is a tone of one of `keys` (program, key) played at `[bearing, _, volume]`.
    fn gives(&self, &(adsr, addr, want): &Key, [angle, _, volume]: [i32; 3], keys: impl IntoIterator<Item = (usize, usize)>) -> bool {
        let bank = Bank::parse(&self.hd).unwrap();
        let seq = sound::stereo(volume, angle, &self.stereo).map(|x| x as u32);
        keys.into_iter().any(|(p, k)| {
            bank.key_ons(p, k).into_iter().flatten().any(|e| {
                let (Some(t), Some(set)) = (bank.tone(e.set as usize, e.note), bank.set_volume(e.set as usize)) else { return false };
                let mut l = Level { seq, bank: self.bank_volume, velocity: e.velocity as u32, pan: [0x40; 3], ..Default::default() };
                l.tone(set, t, &self.gain);
                t.adsr() == adsr && BASE + t.sample() as u32 == addr && l.volume(&self.pan) == want
            })
        })
    }

    fn gives_any(&self, key: &Key, play: [i32; 3]) -> bool {
        self.gives(key, play, (0..128).flat_map(|p| (0..128).map(move |k| (p, k))))
    }
}

#[test]
fn positional_sounds_match_the_game() {
    let Some((data, court)) = Court::load("sound_s05.bin") else { return };
    let s = samples(&data, 0);
    let keyed = |x: &Sample| court.keyed(x, None);
    let gives = |k: &Key, play| court.gives_any(k, play);
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

/// Every racket hit of a recorded doubles bot match (`context/fixtures/hits_s05.bin`, `tools/record_sound.py 5 … hits`):
/// `sound::hit_sounds` from the hit's state gives the court key-ons the driver started — volume after falloff from
/// the ball, L/R from the bearing — and the play-speed scale word the next frame.
#[test]
fn hit_sounds_match_the_game() {
    let Some((data, court)) = Court::load("hits_s05.bin") else { return };
    let s = samples(&data, 0x330);
    let i = |b: &[u8], o: usize| u(b, o) as i32;
    // hit block: effects state from +0xb8, rally from 0x423040 at 0x68, players 0 and 1 from +0x3e90 at 0xf0
    let (fx, rally, players) = (|o: usize| o - 0xb8, 0x68, |p: usize, o: usize| 0xf0 + 0x120 * p + o - 0x3e90);
    let (mut hits, mut keys) = (0, 0);
    for k in 1..s.len() - 2 {
        let (a, h) = (s[k - 1].hit, s[k].hit);
        let (n, hitter) = (i(h, rally + 0x20), i(h, rally + 0x18));
        if n == i(a, rally + 0x20) || n == 0 {
            continue;
        }
        let p = hitter as usize;
        let rec = fx(0xd8 + 8 * p);
        let gap = (p < 2 && h[players(p, 0x3f07)] != 0).then(|| i(h, players(p, 0x3f08)));
        let hit = |random_bit| sound::Hit {
            branch: h[rec + 5],
            grade: h[rec + 4],
            offset: i(h, rec),
            kind: i(h, fx(0xd0)),
            framed: h[fx(0xbc)] != 0,
            dull: h[fx(0xbd)] != 0,
            power_gap: gap,
            random_bit,
            hits: n,
            strong_toss: i(h, players(0, 0x3ea0)) == 1,
            solo: i(h, 0x48 + 4) == 1,
        };
        // the ball where it was hit, this frame or the last: the recorded play (hit sounds, or the flight sound
        // started after them) has its bearing and distance
        let (angle, dist) = s[k - 1..=k]
            .iter()
            .map(|x| sound::place(std::array::from_fn(|j| f32::from_bits(u(x.ball, 0xe0 + 4 * j)))))
            .find(|&(angle, dist)| s[k..=k + 1].iter().any(|x| x.play[..2] == [angle, dist]))
            .unwrap_or_else(|| panic!("hit {n} at frame {k}: no play at the ball, {:?}", s[k].play));
        // a hit's key-on can carry the voice's unplaced volume, its L/R following the next frame
        let window: Vec<Key> = (k..=k + 1).flat_map(|j| court.keyed(&s[j], Some(&s[j + 1]))).collect();
        let keyed = |pl: &sound::Play| {
            let play = [angle, dist, sound::falloff(pl.volume, dist)];
            window.iter().filter(|w| court.gives(w, play, [(pl.program as usize, pl.key as usize)])).count()
        };
        // ponytail: slot 9 (framed hits) is the character's bank, not recorded here — its plays are not checked
        let court_plays = |hit| sound::hit_sounds(&hit).into_iter().filter(|pl| pl.slot == 0).collect::<Vec<_>>();
        let Some(plays) = [false, true].map(|r| court_plays(hit(r))).into_iter().find(|p| p.iter().all(|pl| keyed(pl) > 0)) else {
            panic!("hit {n} at frame {k}: {:?} / {:?} not keyed in {window:x?}", hit(false), court_plays(hit(false)))
        };
        for pl in &plays {
            keys += keyed(pl);
            if pl.speed != 1.0 {
                let word = sound::speed_word(pl.speed);
                assert!(s[k..k + 3].iter().flat_map(|x| &x.cmds).any(|c| c[0] == 4 && c[3] & 0xffff == word), "hit {n}: scale {word:#x}");
            }
        }
        hits += 1;
    }
    eprintln!("{hits} hits, {keys} key-ons");
    assert!(hits >= 25);
}

/// The swing whooshes of the same match: every hit whose swing `sound::swing_sound` gives a whoosh (first serves,
/// smashes) had program 4 key 0 keyed shortly before it, and no other swing did; a serve's plays at the recorded
/// bearing (the whoosh is that frame's last play).
#[test]
fn swing_sounds_match_the_game() {
    let Some((data, court)) = Court::load("hits_s05.bin") else { return };
    let s = samples(&data, 0x330);
    let i = |b: &[u8], o: usize| u(b, o) as i32;
    let bank = Bank::parse(&court.hd).unwrap();
    let sw = sound::SWING;
    // frames where a whoosh tone keys on (any volume)
    let whooshes: Vec<usize> = (0..s.len())
        .filter(|&k| {
            s[k].cmds.iter().filter(|c| c[0] == 2 && c[3] != 8).any(|c2| {
                let Some(c3) = s[k].cmds.iter().find(|c| c[0] == 3 && c[1] == c2[1]) else { return false };
                bank.key_ons(sw.program as usize, sw.key as usize).into_iter().flatten().any(|e| {
                    bank.tone(e.set as usize, e.note).is_some_and(|t| t.adsr() == (c2[2] as u16, c2[3] as u16) && BASE + t.sample() as u32 == c3[2])
                })
            })
        })
        .collect();
    let (fx, rally) = (|o: usize| o - 0xb8, 0x68);
    let mut expected = 0;
    for k in 8..s.len() {
        let (a, h) = (s[k - 1].hit, s[k].hit);
        let (n, hitter) = (i(h, rally + 0x20), i(h, rally + 0x18));
        if n == i(a, rally + 0x20) || n == 0 {
            continue;
        }
        let rec = fx(0xd8 + 8 * hitter as usize);
        let (branch, kind) = (h[rec + 5], i(h, fx(0xd0)));
        let faults = i(s[k].rally, 0x18);
        let want = sound::swing_sound(branch, kind, faults).is_some();
        let near: Vec<usize> = whooshes.iter().copied().filter(|&w| (k - 8..=k).contains(&w)).collect();
        assert_eq!(!near.is_empty(), want, "hit {n} at frame {k}: branch {branch} kind {kind} faults {faults}, whooshes {near:?}");
        if want {
            expected += 1;
        }
        if want && branch == 0 {
            let w = near[0];
            let play = [s[w].play[0], s[w].play[1], sound::falloff(sw.volume, s[w].play[1])];
            assert!(court.keyed(&s[w], None).iter().any(|key| court.gives(key, play, [(4, 0)])), "serve whoosh at {w}: {play:?}");
        }
    }
    eprintln!("{expected} whooshes");
    assert!(expected >= 6 && expected == whooshes.len());
}

/// The bounce sounds of the same match: every bounce `sound::Bounces` gives sounds for (court bounces, the ground
/// beyond, the net) keyed its program 2 plays at the contact record's point, this frame or the next. Bounce sounds
/// stop once the point is decided: the double bounce still plays, the dead ball after it does not (until the next
/// serve). Program 2 key 0 keyed only for those, bar two plays well away from the ball a moment after a point.
#[test]
fn bounce_sounds_match_the_game() {
    let Some((data, court)) = Court::load("hits_s05.bin") else { return };
    let s = samples(&data, 0x330);
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut iso = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")).unwrap();
    let materials = hst_sim::court::materials(&exe::Game::new(&iso.read("SYSTEM.CNF").unwrap(), &iso.read("ZZBIN/GAME.BIN").unwrap()).unwrap());
    let bank = Bank::parse(&court.hd).unwrap();
    let i = |b: &[u8], o: usize| u(b, o) as i32;
    let f = |b: &[u8], o: usize| f32::from_bits(u(b, o));
    // court bounce tones (program 2 key 0) keyed, at any volume
    let tone = |k: usize| {
        s[k].cmds.iter().filter(|c| c[0] == 2 && c[3] != 8).filter(|c2| {
            let Some(c3) = s[k].cmds.iter().find(|c| c[0] == 3 && c[1] == c2[1]) else { return false };
            bank.key_ons(2, 0).into_iter().flatten().any(|e| bank.tone(e.set as usize, e.note).is_some_and(|t| t.adsr() == (c2[2] as u16, c2[3] as u16) && BASE + t.sample() as u32 == c3[2]))
        }).count()
    };
    let heard: usize = (0..s.len()).map(tone).sum();
    let (fx, rally) = (|o: usize| o - 0xb8, 0x68);
    let mut bounces = Bounces::default();
    let (mut plays, mut zeros, mut superseded, mut dead) = (0, 0, 0, false);
    for k in 1..s.len() - 2 {
        let n = i(s[k].ball, 0x224);
        // ponytail: the point is decided by the double bounce here (no bounce in this match lands out)
        dead &= n != 0;
        let bounce = (n != i(s[k - 1].ball, 0x224) && n > 0 && !dead).then(|| (n, &materials[s[k].contacts[(n as usize - 1) * 0x50 + 0x40] as usize]));
        dead |= n >= 2;
        let h = s[k].hit;
        let hitter = i(h, rally + 0x18);
        let smash = (hitter >= 0 && h[fx(0xd8 + 8 * hitter as usize) + 5] == 4).then(|| sound::kmh(std::array::from_fn(|j| f(s[k].ball, 0x140 + 4 * j))));
        for pl in bounces.frame(bounce, smash) {
            let (n, _) = bounce.unwrap();
            let pos = std::array::from_fn(|j| f(s[k].contacts, (n as usize - 1) * 0x50 + 0x10 + 4 * j));
            let (angle, dist) = sound::place(pos);
            let play = [angle, dist, sound::falloff(pl.volume, dist)];
            let window: Vec<Key> = (k..=k + 1).flat_map(|j| court.keyed(&s[j], Some(&s[j + 1]))).collect();
            let found = |play| window.iter().any(|w| court.gives(w, play, [(2, pl.key as usize)]));
            if !found(play) {
                // a later play of the same key that frame took the voice over (one recorded play a frame)
                assert!((k..=k + 1).any(|j| s[j].play != play && found(s[j].play)), "bounce {n} at frame {k} {pos:?}: {pl:?} {play:?}");
                superseded += 1;
            }
            if pl.speed != 1.0 {
                let word = sound::speed_word(pl.speed);
                assert!(s[k..k + 3].iter().flat_map(|x| &x.cmds).any(|c| c[0] == 4 && c[3] & 0xffff == word), "bounce {n} at {k}: scale {word:#x}");
            }
            plays += 1;
            zeros += (pl.key == 0) as usize;
        }
    }
    eprintln!("{plays} bounce plays ({zeros} key 0, {superseded} superseded), {heard} bounce tones keyed");
    assert!(plays >= 20 && superseded <= 1 && zeros + 2 == heard);
}
/// The flight whistle (`sound::FLIGHT`): every lob and framed mis-hit of `hits_s05.bin` starts it; each later frame
/// its scale word is `flight_speed` of the ball height and its L/R the court chain for program 5 key 2 at the ball,
/// re-placed at 0x80 (from this frame's ball or the last, the recording's phase); it stops at the first bounce.
#[test]
fn flight_whistle_matches_the_game() {
    let Some((data, court)) = Court::load("hits_s05.bin") else { return };
    let s = samples(&data, 0x330);
    let f = |b: &[u8], o: usize| f32::from_bits(u(b, o));
    let ball = |k: usize| std::array::from_fn(|j| f(s[k].ball, 0xe0 + 4 * j));
    let (fx, rally) = (|o: usize| o - 0xb8, 0x68);
    // hits that start it: a lob (kind 3) or a framed mis-hit
    let mut want = Vec::new();
    for k in 1..s.len() {
        let (a, h) = (s[k - 1].hit, s[k].hit);
        let n = u(h, rally + 0x20);
        if n != 0 && n != u(a, rally + 0x20) && (u(h, fx(0xd0)) == 3 || h[fx(0xbc)] != 0) {
            want.push(k);
        }
    }
    let (mut voice, mut adsr, mut starts, mut frames) = (None, (0, 0), Vec::new(), 0);
    for k in 2..s.len() {
        for c in &s[k].cmds {
            match (c[0], voice) {
                (2, _) if s[k].cmds.iter().any(|d| d[0] == 3 && d[1] == c[1] && d[2] == BASE + 0x9e30) => {
                    (voice, adsr) = (Some(c[1]), (c[2] as u16, c[3] as u16));
                    starts.push(k);
                }
                (3, Some(v)) if c[1] == v && c[2] != BASE + 0x9e30 => voice = None,
                (4, Some(v)) if c[1] == v && k > *starts.last().unwrap() + 1 => {
                    assert!((k - 1..=k).any(|j| sound::speed_word(sound::flight_speed(ball(j)[1], 0.3, 10.0)) == c[3] & 0xffff), "frame {k}: scale {:x}", c[3]);
                    assert!((k - 2..=k).any(|j| u(s[j].ball, 0x224) == 0), "frame {k}: whistle after the bounce");
                    frames += 1;
                }
                (1, Some(v)) if c[1] == v && k > *starts.last().unwrap() => {
                    let key = (adsr, BASE + 0x9e30, [c[2] as i16, c[3] as i16]);
                    let at = |j| { let (angle, dist) = sound::place(ball(j)); [angle, dist, sound::falloff(0x80, dist)] };
                    let p = sound::FLIGHT;
                    assert!((k - 1..=k).any(|j| court.gives(&key, at(j), [(p.program as usize, p.key as usize)])), "frame {k}: L/R {key:?}");
                }
                _ => {}
            }
        }
    }
    eprintln!("{} whistles from hits {want:?}, keyed {starts:?}, {frames} frames", want.len());
    assert!(want.len() >= 5 && frames >= 200);
    assert!(want.iter().zip(&starts).all(|(&h, &k)| (h..=h + 1).contains(&k)) && want.len() == starts.len());
}

/// The dive thud (`sound::dive_thud`): every dive of `hits_s05.bin` (a hit record with branch 3, missed ones too) keys
/// program 3 key 0 the next frame and again `dive_echo(4)` frames on, nothing else keys it, and the thud's triple
/// carries full volume.
#[test]
fn dive_thuds_match_the_game() {
    let Some((data, court)) = Court::load("hits_s05.bin") else { return };
    let s = samples(&data, 0x330);
    let bank = Bank::parse(&court.hd).unwrap();
    let thud = sound::dive_thud(0);
    let keyed = |x: &Sample| {
        x.cmds.iter().filter(|c| c[0] == 2 && c[3] != 8).any(|c2| {
            let Some(c3) = x.cmds.iter().find(|c| c[0] == 3 && c[1] == c2[1]) else { return false };
            bank.key_ons(thud.program as usize, thud.key as usize).into_iter().flatten().any(|e| bank.tone(e.set as usize, e.note).is_some_and(|t| t.adsr() == (c2[2] as u16, c2[3] as u16) && BASE + t.sample() as u32 == c3[2]))
        })
    };
    let dives: Vec<usize> = (0..s.len()).filter(|&k| (0..4).any(|p| s[k].hit[0xd4 - 0xb8 + p] != 0 && s[k].hit[0xd8 - 0xb8 + 8 * p + 5] == 3)).collect();
    let want: Vec<usize> = dives.iter().flat_map(|&k| [k + 1, k + 1 + sound::dive_echo(4) as usize]).collect();
    let heard: Vec<usize> = (0..s.len()).filter(|&k| keyed(&s[k])).collect();
    eprintln!("{} dives, thuds at {heard:?}", dives.len());
    assert_eq!(dives.len(), 2);
    assert_eq!(heard, want);
    assert_eq!(s[want[0]].play[2], thud.volume);
}

/// The players' shouts in the same match (doubles, voice banks of `spu_s05.csv`): every launch keys
/// `sound::stroke_shout`'s program on the hitter's own bank that frame or the next — always where the shout is
/// certain, never where it cannot be, by chance otherwise — and every dive program 3 as it starts (that frame or the next); each
/// key is in its range and programs 1 and 2 never repeat a player's last key. No other stroke or dive shouts.
#[test]
fn shouts_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(csv) = std::fs::read_to_string(format!("{root}/context/fixtures/spu_s05.csv")) else { return };
    let Some((data, _)) = Court::load("hits_s05.bin") else { return };
    let mut iso = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")).unwrap();
    // the four players' banks: name, header, SPU address, sample size
    let mut banks = Vec::new();
    let rows: Vec<Vec<&str>> = csv.lines().map(|l| l.split(',').collect()).collect();
    for v in rows.iter().filter(|v| v[0] == "bank" && v[2].contains("/DBL/")) {
        let xb = iso.read(v[1]).unwrap();
        let arc = Archive::parse(&xb).unwrap();
        let read = |name: &str| arc.read(arc.entries.iter().find(|e| e.name.replace('\\', "/").ends_with(name)).unwrap()).unwrap();
        banks.push((v[2], read(v[2]), v[3].parse::<u32>().unwrap(), read(&v[2].replace(".hd", ".bd")).len() as u32));
    }
    assert_eq!(banks.len(), 4);
    let s = samples(&data, 0x330);
    // voice key-ons of frame k: bank index, program, key
    let heard = |k: usize| -> Vec<(usize, u8, u8)> {
        let x = &s[k];
        let mut out = Vec::new();
        for c2 in x.cmds.iter().filter(|c| c[0] == 2 && c[3] != 8) {
            let Some(c3) = x.cmds.iter().find(|c| c[0] == 3 && c[1] == c2[1]) else { continue };
            for (b, (_, hd, base, _)) in banks.iter().enumerate().filter(|(_, b)| (b.2..b.2 + b.3).contains(&c3[2])) {
                let bank = Bank::parse(hd).unwrap();
                let pk = (0..16).flat_map(|p| (0..16).map(move |k| (p, k))).find(|&(p, k)| {
                    bank.key_ons(p, k).into_iter().flatten().any(|e| bank.tone(e.set as usize, e.note).is_some_and(|t| t.adsr() == (c2[2] as u16, c2[3] as u16) && base + t.sample() as u32 == c3[2]))
                });
                out.push((b, pk.unwrap().0 as u8, pk.unwrap().1 as u8));
            }
        }
        out
    };
    let i = |b: &[u8], o: usize| u(b, o) as i32;
    let (fx, rally) = (|o: usize| o - 0xb8, 0x68);
    let (mut bank_of, mut last) = ([None; 4], [[None; 2]; 4]);
    let (mut sure, mut chance, mut dives, mut quiet) = (0, 0, 0, 0);
    let mut check = |k: usize, p: usize, want: Option<u8>, may: Option<u8>| {
        let got: Vec<_> = (k..=k + 1).flat_map(|j| heard(j)).filter(|v| v.1 < 6).collect();
        assert!(got.len() <= 1, "frame {k}: {got:?}");
        match (got.first(), want, may) {
            (None, None, _) => quiet += (may.is_none()) as i32,
            (Some(&(b, prog, key)), _, Some(m)) if prog == m && want.is_none_or(|w| w == m) => {
                assert_eq!(*bank_of[p].get_or_insert(b), b, "player {p} on two banks");
                assert!(key <= 1, "frame {k}: key {key}");
                if let 1 | 2 = prog {
                    let l = &mut last[p][prog as usize - 1];
                    assert_ne!(*l, Some(key), "frame {k}: player {p} repeats program {prog} key {key}");
                    *l = Some(key);
                }
                if prog == sound::DIVE_SHOUT { dives += 1 } else if want.is_some() { sure += 1 } else { chance += 1 }
            }
            (got, _, _) => panic!("frame {k} player {p}: heard {got:?}, want {want:?} / may {may:?}"),
        }
    };
    for k in 1..s.len() - 2 {
        let (a, h) = (s[k - 1].hit, s[k].hit);
        for p in 0..4 {
            if h[fx(0xd4) + p] != 0 && h[fx(0xd8) + 8 * p + 5] == 3 {
                check(k, p, Some(sound::DIVE_SHOUT), Some(sound::DIVE_SHOUT));
            }
        }
        let n = i(h, rally + 0x20);
        if n == i(a, rally + 0x20) || n == 0 {
            continue;
        }
        let p = i(h, rally + 0x18) as usize;
        let rec = fx(0xd8 + 8 * p);
        let hit = sound::Hit {
            branch: h[rec + 5],
            grade: h[rec + 4],
            offset: i(h, rec),
            kind: i(h, fx(0xd0)),
            framed: h[fx(0xbc)] != 0,
            dull: h[fx(0xbd)] != 0,
            strong_toss: i(h, 0xf0 + 0x3ea0 - 0x3e90) == 1,
            ..Default::default()
        };
        if hit.branch == 3 {
            continue;
        }
        check(k, p, sound::stroke_shout(&hit, 0, 4, || 99), sound::stroke_shout(&hit, 0, 4, || 0));
    }
    eprintln!("{sure} certain shouts, {chance} by chance, {dives} dives, {quiet} quiet strokes; banks {bank_of:?}");
    assert!(sure >= 10 && dives == 2);
    // ponytail: programs 4 (a missed swing) and 6–10 (reactions after the point) are N3d2
}
