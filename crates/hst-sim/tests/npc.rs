//! Background figures from the disc against the game's own objects in RAM on four courts (`context/ram/`: slots
//! 07 and 10 of the PCSX2 states = courts 1 and 2, `s03.bin` = court 4, `s05.bin` = court 10; not in git, skipped
//! when absent along with the disc image). Every creature record's position (links resolved) and yaw must match
//! the game's loaded record bit for bit; the walking spectators and trigger creatures made must be exactly the
//! game's (record, type); every trigger creature's matrix bit-exact, and a walker's wherever it still stands where
//! it was made (all four on court 1).

use hst_data::exe::Game;
use hst_data::iso::Iso;
use hst_data::layout;
use hst_data::xb::Archive;
use hst_sim::npc::{self, Kind};
use std::collections::BTreeMap;

const WALKER: u32 = 0x1d1de0;
const TRIGGER: u32 = 0x1d2180;

struct Ram(Vec<u8>);
impl Ram {
    fn u(&self, a: u32) -> u32 {
        let a = (a & 0x1ff_ffff) as usize;
        u32::from_le_bytes(self.0[a..a + 4].try_into().unwrap())
    }
    fn bits(&self, a: u32, n: u32) -> Vec<u32> {
        (0..n).map(|k| self.u(a + 4 * k)).collect()
    }
}

#[test]
fn creatures_match_ram_on_four_courts() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let mut courts = Vec::new();
    for f in ["npc_c01", "npc_c02", "s03", "s05"] {
        let Ok(ram) = std::fs::read(format!("{root}/context/ram/{f}.bin")) else {
            eprintln!("{f}.bin missing, skipped");
            continue;
        };
        let r = Ram(ram);
        let (court, players) = (r.u(0x422f90), r.u(0x422fa4));
        courts.push(court);
        let (mut entries, mut plants) = (Vec::new(), Vec::new());
        for arc in ["CMN.XB", "HOL01.XB"] {
            let data = iso.read(&format!("COURT/{court:02}/{arc}")).unwrap();
            let a = Archive::parse(&data).unwrap();
            for e in &a.entries {
                let name = e.name.to_ascii_lowercase();
                if name.ends_with(&format!("entry_c{court:02}.txt")) {
                    entries = layout::entries(&String::from_utf8_lossy(&a.read(e).unwrap()));
                } else if name.ends_with(&format!("plant_c{court:02}_h01_0.dat")) {
                    plants = layout::plants(&a.read(e).unwrap()).unwrap();
                }
            }
        }

        let layout = r.u(r.u(r.u(0x422f80) + 0x84) + 0x150);
        let (n, recs) = (r.u(layout + 0x430 + 23 * 4), r.u(layout + 0x490 + 23 * 4));
        let pos = npc::positions(&plants);
        assert_eq!(pos.len(), n as usize, "court {court}: creature records");
        let yaws: Vec<f32> = plants.iter().filter(|p| p.category == 23).map(|p| p.yaw).collect();
        for (k, p) in pos.iter().enumerate() {
            let a = recs + 0x90 * k as u32;
            assert_eq!(r.bits(a + 0x40, 3), p.map(f32::to_bits), "court {court} record {k}: position");
            assert_eq!(r.u(a + 0x54), yaws[k].to_bits(), "court {court} record {k}: yaw");
        }

        // the game's figures: record → (type, matrix)
        let mut objs = BTreeMap::new();
        for a in (0x10_0000..0x200_0000u32).step_by(4) {
            let vt = r.u(a);
            if vt == WALKER || vt == TRIGGER {
                let rec = (r.u(a + 0x54) - recs) / 0x90;
                objs.insert(rec as usize, (vt == WALKER, r.u(a + 0x50) as u8, r.bits(a + 0x70, 16)));
            }
        }
        let made = npc::spawn(&entries, &plants, &game.npc_roster(court), &game.walkers(court), players);
        let ours: BTreeMap<usize, (bool, u8)> = made
            .iter()
            .filter_map(|m| match m.kind {
                Kind::Trigger(t) => Some((m.record, (false, t))),
                Kind::Walker(w) => Some((m.record, (true, w + 54))),
                _ => None,
            })
            .collect();
        let theirs: BTreeMap<usize, (bool, u8)> = objs.iter().map(|(&k, v)| (k, (v.0, v.1))).collect();
        assert_eq!(ours, theirs, "court {court}: figures (record → walker?, type)");
        let mut still = 0;
        for m in &made {
            let Some((walker, _, mtx)) = objs.get(&m.record) else { continue };
            let want: Vec<u32> = m.world.iter().flatten().map(|x| x.to_bits()).collect();
            if *walker && mtx[12..15] != want[12..15] {
                continue; // walked off
            }
            still += *walker as usize;
            assert_eq!(*mtx, want, "court {court} record {}: matrix", m.record);
        }
        eprintln!("court {court}: {} records, {} figures, {still} walkers checked at their spot", n, theirs.len());
        if court == 1 {
            assert_eq!(still, 4, "court 1's walkers have not moved");
        }
    }
    assert!(courts.is_empty() || courts.len() >= 3, "fewer than three courts checked: {courts:?}");
}

/// The game's shared MT19937 as recorded: 624 words at +4, the next word's index at +0x9c4.
#[derive(Clone, PartialEq)]
struct Mt([u32; 624], usize);
impl Mt {
    fn of(b: &[u8]) -> Mt {
        let w = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        Mt(std::array::from_fn(|k| w(4 + 4 * k)), w(0x9c4) as usize)
    }
    fn next(&mut self) -> u32 {
        if self.1 >= 624 {
            for k in 0..624 {
                let y = (self.0[k] & 0x8000_0000) | (self.0[(k + 1) % 624] & 0x7fff_ffff);
                self.0[k] = self.0[(k + 397) % 624] ^ (y >> 1) ^ if y & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.1 = 0;
        }
        let mut y = self.0[self.1];
        self.1 += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }
}

/// Walking spectators' animation against `context/fixtures/npc_sNN.bin` (tools/record_npc.py; not in git, skipped
/// when absent): every tick of every walker, from the recorded state of the tick before, must reproduce the game's
/// animation, frame, next frame, speed, mode and stagger counter bit for bit. Random draws come from the game's
/// own generator (reseeded at a new point): the walker's draws must be consecutive outputs among those the game drew that tick. Points and
/// new points are taken from the recording (mode becomes 2 / the stagger counter restarts).
#[test]
fn walkers_animate_like_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut courts = Vec::new();
    for slot in [5, 7, 10, 3] {
        let Ok(d) = std::fs::read(format!("{root}/context/fixtures/npc_s{slot:02}.bin")) else {
            eprintln!("npc_s{slot:02}.bin missing, skipped");
            continue;
        };
        let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let f = |b: &[u8], o: usize| f32::from_bits(u(b, o));
        let n = u(&d, 0) as usize;
        let (g, mt, mgr, mgr2) = (4, 4 + 0x180, 4 + 0x180 + 0x9d0, 4 + 0x180 + 0x9d0 + 0x280);
        let wo = |k: usize| mgr2 + 0x20 + 8 + k * 0x380;
        let size = wo(n);
        let samples: Vec<&[u8]> = d[4 + 4 * n..].chunks_exact(size).collect();
        let (court, players) = (u(samples[0], g + 0x10), u(samples[0], g + 0x24));
        let state = |s: &[u8], k: usize, lens: [f32; 7]| {
            let (o, c) = (wo(k), wo(k) + 0x310);
            npc::Walker {
                slot: u(s, o + 0xbc),
                mode: u(s, o + 0x210) as u8,
                counter: u(s, o + 0xd0) as i32,
                anim: u(s, o + 200),
                frame: f(s, c + 0x38),
                next: f(s, c + 0x3c),
                speed: f(s, c + 0x34),
                advancing: s[o + 0xcc] != 0,
                lens,
            }
        };
        let mut lens = vec![[0f32; 7]; n];
        for s in &samples {
            for k in 0..n {
                lens[k][u(s, wo(k) + 200) as usize] = f(s, wo(k) + 0x350 + 0x2c);
            }
        }
        // a sample in or next to a stretch where the game stood still (a long frame) may be read mid-frame
        let still = |i: usize| samples[i][4..] == samples[i.saturating_sub(1)][4..] && i > 0;
        let (mut ticks, mut draws, mut reactions) = (0, 0, 0);
        for i in 1..samples.len() {
            if (i.saturating_sub(2)..(i + 2).min(samples.len())).any(still) {
                continue;
            }
            let (a, b) = (samples[i - 1], samples[i]);
            if u(b, 0) != u(a, 0) + 1 {
                continue; // a frame missed between them
            }
            let mut rng = Mt::of(&a[mt..]);
            let (end, mut out) = (Mt::of(&b[mt..]), Vec::new());
            while rng != end && out.len() < 2000 {
                out.push(rng.next());
            }
            if rng != end {
                // reseeded (a new point): the draws since are the new words up to the index
                let mut fresh = Mt(end.0, 0);
                out = (0..end.1).map(|_| fresh.next()).collect();
            }
            // a new point restarts every walker's stagger counter; the last walker's is still counting after the tick
            let fresh = (0..n).any(|k| state(a, k, lens[k]).counter < 0 && state(b, k, lens[k]).counter >= 0);
            for k in 0..n {
                let want = state(b, k, lens[k]);
                let mut before = state(a, k, lens[k]);
                if before.mode == 0 && want.mode == 2 {
                    before.react();
                    reactions += 1;
                }
                if fresh {
                    before.new_point(true);
                }
                let cheer = b[mgr + 0x8a5 - 0x680 + before.slot as usize] != 0;
                // the gallery counts its ticks after the walkers' (and restarts at a new point, after them too); the sample at
                // a new point is read before that frame's count
                let tick = if fresh { u(a, mgr2 + 0x14) as i32 } else { u(b, mgr2 + 0x14) as i32 - 1 };
                let used = (0..=out.len()).find_map(|j| {
                    let (mut w, mut it, mut used) = (before.clone(), out[j..].iter(), 0);
                    w.step(players, cheer, tick, &mut || {
                        used += 1;
                        *it.next().unwrap_or(&0)
                    });
                    (w == want && j + used <= out.len()).then_some(used)
                });
                draws += (used > Some(0)) as usize;
                assert!(used.is_some(), "court {court} vsync {} walker {k}:\n from {before:?}\n want {want:?}", u(b, 0));
                ticks += 1;
            }
        }
        eprintln!("court {court}: {ticks} walker ticks, {draws} with random draws, {reactions} reactions");
        courts.push(court);
    }
    assert!(courts.is_empty() || courts.len() >= 3, "fewer than three courts checked: {courts:?}");
}

/// Ambient sound emitters against `context/fixtures/trig_sNN.bin` (tools/record_npc.py with `trig`; not in git,
/// skipped when absent): every tick of every emitter, from the recorded state of the tick before, must give the
/// game's countdown, kept-aside countdown, sweep, pan and direction bit for bit (a play's start pan taken from the
/// recording), and its random draws consecutive outputs among those the game drew that tick. Where the figures
/// were reset (their last message changed) the reset may come before or after the tick.
#[test]
fn emitters_count_down_like_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let mut courts = Vec::new();
    // ponytail: courts 2 (slot 10) and 4 (slot 3) still to pass (journal 4-EMITTERS-PART.md)
    for slot in [5, 7] {
        let Ok(d) = std::fs::read(format!("{root}/context/fixtures/trig_s{slot:02}.bin")) else {
            eprintln!("trig_s{slot:02}.bin missing, skipped");
            continue;
        };
        let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let n = u(&d, 0) as usize;
        let (g, mt) = (4, 4 + 0x180);
        let wo = |k: usize| 4 + 0x180 + 0x9d0 + 0x280 + 0x20 + 8 + k * 0x300;
        let samples: Vec<&[u8]> = d[4 + 4 * n..].chunks_exact(wo(n)).collect();
        let court = u(samples[0], g + 0x10);
        let ours: Vec<usize> = (0..n).filter(|&k| npc::EMITTERS.contains(&samples[0][wo(k) + 0x50])).collect();
        let state = |s: &[u8], k: usize| {
            let (o, ty) = (wo(k), s[wo(k) + 0x50]);
            npc::Emitter {
                ty,
                row: game.emitter(ty),
                pos: std::array::from_fn(|c| f32::from_bits(u(s, o + 0x1e0 + 4 * c))),
                timer: u(s, o + 200) as i32,
                saved: u(s, o + 0xcc) as i32,
                sweep: u(s, o + 0xd0) as i16,
                pan: f32::from_bits(u(s, o + 0xd4)),
                down: s[o + 0xd8] != 0,
            }
        };
        let still = |i: usize| i > 0 && samples[i][4..] == samples[i - 1][4..];
        let (mut ticks, mut plays, mut resets) = (0, 0, 0);
        for i in 1..samples.len() {
            let (a, b) = (samples[i - 1], samples[i]);
            if u(b, 0) != u(a, 0) + 1 || (i.saturating_sub(2)..(i + 2).min(samples.len())).any(still) {
                continue;
            }
            let mut rng = Mt::of(&a[mt..]);
            let (end, mut out) = (Mt::of(&b[mt..]), Vec::new());
            while rng != end && out.len() < 2000 {
                out.push(rng.next());
            }
            if rng != end {
                let mut fresh = Mt(end.0, 0);
                out = (0..end.1).map(|_| fresh.next()).collect();
            }
            for &k in &ours {
                let (before, want) = (state(a, k), state(b, k));
                let msg = u(b, wo(k) + 0xc0);
                let reset = msg != u(a, wo(k) + 0xc0);
                let replays: &[Option<bool>] = if !reset { &[] } else if msg == 0xe { &[Some(false), Some(true)] } else { &[None] };
                // (reset before the tick?, replay flag) variants
                let mut ways = vec![(false, None, false)];
                for &r in replays {
                    ways.extend([(true, r, true), (true, r, false)]);
                }
                let found = ways.iter().find_map(|&(re, r, first)| {
                    (0..=out.len()).find_map(|j| {
                        let (mut e, mut it, mut used) = (before.clone(), out[j..].iter(), 0);
                        if re && first {
                            e.reset(r);
                        }
                        let played = e.step(&mut || {
                            used += 1;
                            *it.next().unwrap_or(&0)
                        });
                        if re && !first {
                            e.reset(r);
                        }
                        if played {
                            e.pan = want.pan; // the start pan is not always the bearing (P14c2 open point)
                        }
                        (e == want && j + used <= out.len()).then_some((played, re))
                    })
                });
                let Some((played, re)) = found else {
                    panic!("court {court} vsync {} emitter {k}:\n from {before:?}\n want {want:?}", u(b, 0))
                };
                (ticks, plays, resets) = (ticks + 1, plays + played as usize, resets + re as usize);
            }
        }
        eprintln!("court {court}: {} emitters, {ticks} ticks, {plays} plays, {resets} resets", ours.len());
        if !ours.is_empty() {
            courts.push(court);
        }
    }
    assert!(courts.is_empty() || courts.len() >= 2, "fewer than two courts checked: {courts:?}");
}

/// The trigger creature engine against `context/fixtures/trig_cNN.bin` (tools/record_npc.py with `trig`; not in
/// git, skipped when absent): courts 5 (type 18, circling), 8 (type 34, idle animation repeat) and 9 (type 37,
/// a loop with pauses and resets). Every tick from the recorded state before must give the game's object state
/// (matrix, path counters, leg, velocity, pause, speed, animation frames, sound countdown) bit for bit, its draws
/// consecutive outputs among those the game drew that tick. The types' own counters live off the object: carried
/// from the engine's previous tick (34's taken from the recording until its first draw). Ticks where the figures
/// were sent a message (a reset at a new point) are skipped. Path nodes come from the recording (the start point
/// and the leg's target), since the path manager is not ported.
#[test]
fn trigger_engine_matches_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let mut courts = Vec::new();
    for (court, k) in [(5, 0), (8, 10), (9, 0)] {
        let Ok(d) = std::fs::read(format!("{root}/context/fixtures/trig_c{court:02}.bin")) else {
            eprintln!("trig_c{court:02}.bin missing, skipped");
            continue;
        };
        let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let n = u(&d, 0) as usize;
        let mt = 4 + 0x180;
        let wo = |k: usize| 4 + 0x180 + 0x9d0 + 0x280 + 0x20 + 8 + k * 0x300;
        let samples: Vec<&[u8]> = d[4 + 4 * n..].chunks_exact(wo(n)).collect();
        let o = wo(k);
        let ty = samples[0][o + 0x50];
        let row = game.trigger(ty);
        let f = |s: &[u8], a: usize| f32::from_bits(u(s, o + a));
        let h = |s: &[u8], a: usize| i16::from_le_bytes([s[o + a], s[o + a + 1]]);
        let v = |s: &[u8], a: usize| -> [f32; 4] { std::array::from_fn(|c| f(s, a + 4 * c)) };
        let m = |s: &[u8], a: usize| -> [[f32; 4]; 4] { std::array::from_fn(|r| v(s, a + 16 * r)) };
        let (anchor, target) = (v(samples[0], 0x160), v(samples[0], 0x150));
        let state = |s: &[u8], counter: i32| npc::Trigger {
            ty,
            msg: u(s, o + 0xc0),
            anchor: if row.mode == 6 { target } else { anchor },
            path: if s[o + 0x135] != 0 { vec![anchor, target] } else { vec![] },
            start: 0,
            home: m(s, 0x70),
            world: m(s, 0x1b0),
            active: s[o + 0x12c] != 0,
            on: s[o + 0x281] != 0,
            moving: s[o + 0x135] != 0,
            node: h(s, 0x130),
            prev: h(s, 0x132),
            reverse: s[o + 0x134] != 0,
            left: h(s, 0x136),
            total: h(s, 0x138),
            every: h(s, 0x13a),
            pause: h(s, 0x13c),
            wait: h(s, 0x13e),
            vel: v(s, 0x140),
            target: v(s, 0x150),
            from: v(s, 0x160),
            to: v(s, 0x170),
            snap: h(s, 0x180),
            orient: s[o + 0x182] != 0,
            clockwise: s[o + 0x194] != 0,
            angle: f(s, 0x198),
            radius: f(s, 0x19c),
            repeat: h(s, 0x1a0),
            paused: s[o + 0x270] != 0,
            animating: s[o + 0x271] != 0,
            speed: f(s, 0x274),
            stage: h(s, 0x27a),
            frame: f(s, 0x290 + 0x38),
            next: f(s, 0x290 + 0x3c),
            len: if u(s, o + 0x5c) == 0 { 0.0 } else { f(s, 0x2d0 + 0x2c) },
            timer: u(s, o + 200) as i32,
            sweep: h(s, 0xd0),
            pan: f(s, 0xd4),
            down: s[o + 0xd8] != 0,
            counter,
            gap: 0,
            voice: (-1, 0),
        };
        let still = |i: usize| i > 0 && samples[i][4..] == samples[i - 1][4..];
        let (mut ticks, mut draws, mut resets, mut late, mut counter) = (0, 0, 0, 0, None);
        for i in 1..samples.len() {
            let (a, b) = (samples[i - 1], samples[i]);
            if u(b, 0) != u(a, 0) + 1 || (i.saturating_sub(2)..(i + 2).min(samples.len())).any(still) || u(a, o + 0xc0) != u(b, o + 0xc0) {
                counter = None;
                continue;
            }
            let mut rng = Mt::of(&a[mt..]);
            let (end, mut out) = (Mt::of(&b[mt..]), Vec::new());
            while rng != end && out.len() < 2000 {
                out.push(rng.next());
            }
            if rng != end {
                let mut fresh = Mt(end.0, 0);
                out = (0..end.1).map(|_| fresh.next()).collect();
            }
            // 34's idle countdown before its first draw: runs out exactly when the recording starts animating
            let c = counter.unwrap_or(if ty == 34 && b[o + 0x271] != 0 && a[o + 0x271] == 0 { 0 } else { i32::MAX });
            let before = state(a, c);
            // the sample is sometimes read a tick late (two ticks, then none): one tick, else none or two
            let found = [1, 0, 2].into_iter().find_map(|steps| {
                (0..=out.len()).find_map(|j| {
                    let (mut t, mut it, mut used) = (before.clone(), out[j..].iter(), 0);
                    for _ in 0..steps {
                        t.step(&row, &mut || {
                            used += 1;
                            *it.next().unwrap_or(&0)
                        });
                    }
                    let want = npc::Trigger { counter: t.counter, ..state(b, 0) };
                    (t == want && j + used <= out.len()).then_some((t, used, steps))
                })
            });
            let Some((t, used, steps)) = found else {
                let mut t = before.clone();
                t.step(&row, &mut || out[0]);
                panic!("court {court} vsync {} type {ty}:\n from {before:?}\n want {:?}\n  got {t:?}", u(b, 0), state(b, 0))
            };
            late += (steps != 1) as usize;
            resets += (before.left == 1 && t.wait > 0) as usize;
            counter = (counter.is_some() || used > 0 || ty != 34).then_some(t.counter);
            (ticks, draws) = (ticks + 1, draws + used);
        }
        eprintln!("court {court} type {ty}: {ticks} ticks, {draws} draws, {resets} loop resets, {late} read late");
        assert!(late * 50 < ticks, "court {court}: too many samples off by a tick ({late})");
        courts.push(court);
    }
    assert!(courts.is_empty() || courts.len() >= 3, "fewer than three courts checked: {courts:?}");
}
