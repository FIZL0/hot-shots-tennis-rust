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
                objs.insert(rec as usize, (vt == WALKER, r.u(a + 0x50) as u8, r.bits(a + 0x70, 16), r.bits(a + 0x120, 16)));
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
            let Some((walker, _, mtx, drawn)) = objs.get(&m.record) else { continue };
            if *walker {
                // drawn turned to the court centre (the serve placement before every state)
                let centre: Vec<u32> = npc::walker_facing(m.world[3], 0.0).iter().flatten().map(|x| x.to_bits()).collect();
                assert_eq!(*drawn, centre, "court {court} record {}: walker facing", m.record);
            }
            let want: Vec<u32> = m.world.iter().flatten().map(|x| x.to_bits()).collect();
            if *walker && mtx[12..15] != want[12..15] {
                continue; // walked off
            }
            still += *walker as usize;
            assert_eq!(*mtx, want, "court {court} record {}: matrix", m.record);
        }
        // each trigger creature's path: its link (+0x58: path id, start node) into the path manager's list (+0x50 of
        // the layout object: a ring of [last, next, id, count, nodes], nodes 0x70 bytes with the position at +0x50)
        let (mut paths, mut resets) = (0, 0);
        for a in (0x10_0000..0x200_0000u32).step_by(4) {
            if r.u(a) != TRIGGER {
                continue;
            }
            let m = made.iter().find(|m| m.record == ((r.u(a + 0x54) - recs) / 0x90) as usize).unwrap();
            let link = r.u(a + 0x58);
            if link == 0 {
                assert!(m.path.is_empty(), "court {court} record {}: a path the game has not", m.record);
                continue;
            }
            let (head, id) = (r.u(layout + 0x50), r.u(link));
            let mut e = head;
            while r.u(e + 8) != id {
                assert_ne!(e, r.u(head), "court {court}: path {id} not in the list");
                e = r.u(e + 4);
            }
            let nodes: Vec<Vec<u32>> = (0..r.u(e + 12)).map(|k| r.bits(r.u(e + 16) + 0x70 * k + 0x50, 4)).collect();
            let radii: Vec<u32> = (0..r.u(e + 12)).map(|k| r.u(r.u(e + 16) + 0x70 * k + 0x10)).collect();
            let ours: Vec<Vec<u32>> = m.path.iter().map(|p| p.map(f32::to_bits).to_vec()).collect();
            let our_radii: Vec<u32> = m.radii.iter().map(|r| r.to_bits()).collect();
            assert_eq!((ours, our_radii, m.start as u32), (nodes, radii, r.u(link + 4)), "court {court} record {}: path {id}", m.record);
            // the reset's first leg heads where the game's does (its direction bit, +0x134, as the game drew it; a
            // row without `exact` aims about the node: some draw gives the game's target bit for bit)
            let ty = r.u(a + 0x50) as u8;
            let (row, reverse) = (game.trigger(ty), r.u(a + 0x134) & 0xff != 0);
            let mut t = npc::Trigger { ty, anchor: m.world[3], home: m.world, world: m.world, path: m.path.clone(), radii: m.radii.clone(), start: m.start, ..Default::default() };
            t.reset(&row, &mut || if reverse { 0x1_0000 } else { 0 });
            let f4 = |o: u32| r.bits(a + o, 4);
            if m.path.len() > 1 && r.u(a + 0x135) & 0xff == 0 && f4(0x1e0) == m.world[3].map(f32::to_bits).to_vec() {
                assert_eq!(t.node as u16 as u32, r.u(a + 0x130) & 0xffff, "court {court} record {} type {ty}: reset's node", m.record);
                let n = m.path.len() as i16;
                let i = if reverse { n - 1 - t.node % n } else { t.node % n } as usize;
                // a row that snaps each waypoint to the ground (ponytail: not ported) has the court's height in y
                let want: Vec<u32> = f4(0x150);
                let same = |p: [f32; 4]| (0..4).all(|c| (c == 1 && row.snap == -1) || p[c].to_bits() == want[c]);
                let hit = if row.exact {
                    same(t.target)
                } else {
                    // the draw whose angle points at the target, and its neighbours
                    let (dx, dz) = (f32::from_bits(want[0]) - m.path[i][0], f32::from_bits(want[2]) - m.path[i][2]);
                    let guess = ((dx.atan2(dz) as f64 + std::f64::consts::PI) / std::f64::consts::TAU * 4294967296.0) as i64;
                    (-0x10000..0x10000).any(|d| same(t.node_at(i, &row, &mut || (guess + d).clamp(0, u32::MAX as i64) as u32)))
                };
                assert!(hit, "court {court} record {} type {ty}: reset's target {want:x?} (node {:?}, exact {}, snap {})", m.record, m.path[i], row.exact, row.snap);
                resets += 1;
            }
            paths += 1;
        }
        eprintln!("court {court}: {} records, {} figures, {still} walkers checked at their spot, {paths} paths, {resets} resets", n, theirs.len());
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
                looping: matches!(u(s, o + 200), 3 | 5),
                frame: f(s, c + 0x38),
                next: f(s, c + 0x3c),
                speed: f(s, c + 0x34),
                advancing: s[o + 0xcc] != 0,
                lens,
                wander: Default::default(),
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

/// Walking spectators' facing against the same recordings: every sample, every walker's drawn matrix (+0x120) is
/// [`npc::walker_facing`] from its home (+0x70 row 3): toward the court centre, or, from the tick it reacts to a
/// point (mode 0 → 2), toward the middle of the winners' half (the winning team's first player's z, as the walker
/// copied it the tick before). Any other turn is back to the centre (a serve placement).
#[test]
fn walkers_face_like_the_game() {
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
        let (g, mgr2) = (4, 4 + 0x180 + 0x9d0 + 0x280);
        let wo = |k: usize| mgr2 + 0x20 + 8 + k * 0x380;
        let samples: Vec<&[u8]> = d[4 + 4 * n..].chunks_exact(wo(n)).collect();
        let court = u(samples[0], g + 0x10);
        let drawn = |s: &[u8], k: usize| -> Vec<u32> { (0..16).map(|i| u(s, wo(k) + 0x120 + 4 * i)).collect() };
        let bits = |m: [[f32; 4]; 4]| -> Vec<u32> { m.iter().flatten().map(|x| x.to_bits()).collect() };
        let (mut turns, mut reactions, mut blank) = (0, 0, 0);
        // the last sample with a matrix, per walker (a reset zeroes the object before it writes it again)
        let mut last: Vec<Option<usize>> = vec![None; n];
        for i in 0..samples.len() {
            let b = samples[i];
            for k in 0..n {
                let home = std::array::from_fn(|c| f(b, wo(k) + 0xa0 + 4 * c));
                let centre = bits(npc::walker_facing(home, 0.0));
                let have = drawn(b, k);
                if have.iter().all(|&x| x == 0) {
                    blank += 1;
                    continue;
                }
                let Some(a) = last[k].replace(i).map(|j| samples[j]) else {
                    assert!([6.4, -6.4, 0.0].iter().any(|&z| bits(npc::walker_facing(home, z)) == have), "court {court}: walker {k} first facing");
                    continue;
                };
                if u(a, wo(k) + 0x210) == 0 && u(b, wo(k) + 0x210) == 2 {
                    let team = u(b, g + 0x128) as usize;
                    let z = if 0.0 <= f(a, wo(k) + 0x220 + 0x10 * team + 8) { 6.4 } else { -6.4 };
                    assert_eq!(have, bits(npc::walker_facing(home, z)), "court {court} vsync {} walker {k}: reaction", u(b, 0));
                    reactions += 1;
                } else if have != drawn(a, k) {
                    assert_eq!(have, centre, "court {court} vsync {} walker {k}: turn", u(b, 0));
                    turns += 1;
                }
            }
        }
        eprintln!("court {court}: {} samples, {reactions} reaction turns, {turns} turns back to the centre, {blank} blank", samples.len());
        assert!(blank * 100 < samples.len() * n, "court {court}: {blank} blank matrices");
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
    for slot in [5, 7, 10, 3] {
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
        // a frame read after the game ticked again shows the objects unchanged in the next sample (court 2, vsync 33655)
        let still = |i: usize| i > 0 && samples[i][wo(0)..] == samples[i - 1][wo(0)..];
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
    assert!(courts.is_empty() || courts.len() >= 3, "fewer than three courts checked: {courts:?}");
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
        let v = |s: &[u8], a: usize| -> [f32; 4] { std::array::from_fn(|c| f32::from_bits(u(s, o + a + 4 * c))) };
        let (anchor, target) = (v(samples[0], 0x160), v(samples[0], 0x150));
        let state = |s: &[u8], counter: i32| trigger_at(s, o, &row, anchor, target, counter);
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
                    // the model's weight lives off the object: not recorded
                    let want = npc::Trigger { counter: t.counter, weight: t.weight, ..state(b, 0) };
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


/// The startled creatures against `context/fixtures/prox_cNN.bin` (tools/record_npc.py with `prox`, HST_POKE moving a
/// still creature next to a player or the ball; not in git, skipped when absent): courts 1 (types 0, 1), 2 (5),
/// 7 (27–29; `struck` against the game's message 0x14) and 11 (48), any of 8, 9 too. Every tick of every creature of those types from the recorded state before, with that
/// tick's players' and ball's positions, must give the game's state after bit for bit, startled latch and 48's scrub
/// too. The type flags are computed (from the first sample, set by a startle, cleared by a one-shot path's end and a
/// reset message) and must match the game's after every tick. Skipped: message ticks, a poke's tick (moved while standing) and a new leg (the path manager is not
/// ported); a stalled recording's catch-up of up to three ticks is stepped as such.
#[test]
fn startled_creatures_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let mut courts = Vec::new();
    for court in [1, 2, 7, 8, 9, 11] {
        let Ok(d) = std::fs::read(format!("{root}/context/fixtures/prox_c{court:02}.bin")) else {
            eprintln!("prox_c{court:02}.bin missing, skipped");
            continue;
        };
        let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let f = |b: &[u8], o: usize| f32::from_bits(u(b, o));
        let n = u(&d, 0) as usize;
        let (mt, wo) = (4 + 0x180, |k: usize| 4 + 0x180 + 0x9d0 + 0x280 + 0x20 + 8 + k * 0x300);
        let x = wo(n);
        let samples: Vec<&[u8]> = d[4 + 4 * n..].chunks_exact(x + 0x90 + 8 * n).collect();
        // the players' positions, then the ball's; the type flags
        let near = |s: &[u8]| npc::Near {
            pos: (1..=u(s, 4 + 0x24) as usize).chain([0]).map(|i| std::array::from_fn(|c| f(s, x + 0x10 * i + 4 * c))).collect(),
            flags: std::array::from_fn(|t| s[x + 0x50 + t] != 0),
            ..Default::default()
        };
        let (mut ticks, mut startles, mut turns, mut blocked, mut struck, mut sounds, mut legs, mut aimed) = (0, 0, 0, 0, 0, 0, 0, 0);
        // the creatures' paths from the disc, by their home position
        let (entries, plants) = court_layout(&mut iso, court);
        let paths: Vec<([f32; 4], Vec<[f32; 4]>, Vec<f32>, i16)> = npc::spawn(&entries, &plants, &game.npc_roster(court), &game.walkers(court), 2)
            .into_iter()
            .map(|m| (m.world[3], m.path, m.radii, m.start))
            .collect();
        let kinds: Vec<(usize, u8)> = (0..n)
            .map(|k| (k, samples[0][wo(k) + 0x50]))
            .filter(|&(_, ty)| matches!(ty, 0 | 1 | 5 | 27..=29 | 31 | 32 | 39 | 48))
            .collect();
        // the type flags are carried from the first sample on, set and cleared only by the creatures' ticks
        let mut flags = near(samples[0]).flags;
        for i in 1..samples.len() {
            let (a, b) = (samples[i - 1], samples[i]);
            let mut tick14 = false;
            for &(k, ty) in &kinds {
                let o = wo(k);
                let row = game.trigger(ty);
                let v = |s: &[u8], a: usize| -> [f32; 4] { std::array::from_fn(|c| f(s, o + a + 4 * c)) };
                let state = |s: &[u8]| npc::Trigger {
                    startled: s[o + 0x278] != 0,
                    scrub: if ty == 48 { (u(s, x + 0x90 + 8 * k) as i32, u(s, x + 0x94 + 8 * k) as i32) } else { (0, 0) },
                    ..trigger_at(s, o, &row, v(s, 0x160), v(s, 0x150), 0)
                };
                let (mut before, mut after) = (state(a), state(b));
                if let Some((_, path, radii, start)) = paths.iter().find(|p| p.0 == before.home[3] && !p.1.is_empty()) {
                    (before.path, before.radii, before.start) = (path.clone(), radii.clone(), *start);
                    (after.path, after.radii, after.start) = (path.clone(), radii.clone(), *start);
                }
                let disc = !before.path.is_empty() && paths.iter().any(|p| p.1 == before.path);
                // a poke moved it before the tick (HST_POKE): it starts from there
                let poked = !before.moving && before.world[3] != after.world[3];
                if poked {
                    before.world[3] = after.world[3];
                }
                // a creature struck by the ball (27–29) tells everyone 0x14 mid-tick, which only stores the message
                let new14 = after.msg == 0x14 && before.msg != 0x14;
                if new14 {
                    before.msg = 0x14;
                }
                if u(b, 0) != u(a, 0) + 1 || before.msg != after.msg || (before.target != after.target && !disc) {
                    // not stepped; a reset message (0xc a game's end, 0xe a point's end, 0x1a) clears its type's flag
                    if before.msg != after.msg && matches!(after.msg, 0xc | 0xe | 0x1a) {
                        let mut seen = npc::Near { flags, ..near(b) };
                        before.clone().reset_near(&row, &mut seen, &mut || 0);
                        flags = seen.flags;
                    }
                    continue;
                }
                let mut rng = Mt::of(&a[mt..]);
                let (end, mut out) = (Mt::of(&b[mt..]), Vec::new());
                while rng != end && out.len() < 2000 {
                    out.push(rng.next());
                }
                let found = [1, 0, 2, 3].into_iter().find_map(|steps| {
                    (0..=out.len()).find_map(|j| {
                        let (mut t, mut it, mut seen, mut heard) = (before.clone(), out[j..].iter(), near(b), Vec::new());
                        seen.flags = flags;
                        for _ in 0..steps {
                            heard.extend(t.step_near(&row, &mut seen, &mut || *it.next().unwrap_or(&0)));
                        }
                        // the model's weight lives off the object: not recorded
                        let want = npc::Trigger { struck: t.struck, path: t.path.clone(), anchor: t.anchor, weight: t.weight, ..after.clone() };
                        (t == want).then_some((t, seen, heard))
                    })
                });
                let Some((t, seen, heard)) = found else {
                    let (mut t, mut seen) = (before.clone(), near(b));
                    seen.flags = flags;
                    t.step_near(&row, &mut seen, &mut || out[0]);
                    panic!("court {court} vsync {} creature {k} type {ty}:\n from {before:?}\n want {after:?}\n  got {t:?}", u(b, 0))
                };
                assert!(!t.struck || after.msg == 0x14, "court {court} vsync {} creature {k}: struck without message 0x14", u(b, 0));
                struck += t.struck as usize;
                // a sound started: the game's handle to it (+0xc4) is new
                assert_eq!(!heard.is_empty(), u(b, o + 0xc4) != u(a, o + 0xc4), "court {court} vsync {} creature {k}: sound {heard:?}", u(b, 0));
                sounds += heard.len();
                tick14 |= t.struck;
                startles += (t.startled && !before.startled) as usize;
                turns += (t.scrub.0 != before.scrub.0) as usize;
                blocked += (!t.startled && flags[ty as usize] && near(b).pos.iter().any(|p| {
                    let d: [f32; 4] = std::array::from_fn(|c| p[c] - t.world[3][c]);
                    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() < 2.0
                })) as usize;
                flags = seen.flags;
                ticks += 1;
                legs += (disc && t.moving) as usize;
                aimed += (disc && !row.exact && t.target != before.target) as usize;
            }
            // where 27–29 stand, a new 0x14 came from one of them
            if let Some(&(k, _)) = kinds.iter().find(|&&(_, ty)| (27..=29).contains(&ty)) {
                let new14 = u(b, wo(k) + 0xc0) == 0x14 && u(a, wo(k) + 0xc0) != 0x14 && u(b, 0) == u(a, 0) + 1;
                assert_eq!(tick14, new14, "court {court} vsync {}: message 0x14 vs struck", u(b, 0));
            }
            for &(_, ty) in &kinds {
                assert_eq!(flags[ty as usize], near(b).flags[ty as usize], "court {court} vsync {}: type {ty}'s flag", u(b, 0));
            }
        }
        eprintln!("court {court}: {ticks} ticks, {startles} startles, {turns} scrub turns, {blocked} ticks held back by the flag, {struck} struck, {sounds} sounds, {legs} moving along the disc's paths, {aimed} legs aimed about a node");
        assert!(court != 1 || legs > 0, "court 1: nothing moved along a disc path");
        assert!(court != 7 || struck > 0, "court 7: no creature struck by the ball");
        assert!(startles + turns > 0, "court {court}: nothing startled");
        courts.push(court);
    }
    assert!(courts.is_empty() || courts.len() >= 3, "fewer than three courts checked: {courts:?}");
}

/// Court `court`'s entry list and hole-01 placement records.
fn court_layout(iso: &mut Iso, court: u32) -> (Vec<layout::Entry>, Vec<layout::Placement>) {
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
    (entries, plants)
}

/// A trigger creature's engine state from its recorded object at `o` in sample `s` (`trig` layout); the path is the
/// recorded leg (`anchor` its start, `target` its end) while it moves, the type's own counter given.
fn trigger_at(s: &[u8], o: usize, row: &hst_data::exe::TriggerRow, anchor: [f32; 4], target: [f32; 4], counter: i32) -> npc::Trigger {
    let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let f = |s: &[u8], a: usize| f32::from_bits(u(s, o + a));
    let h = |s: &[u8], a: usize| i16::from_le_bytes([s[o + a], s[o + a + 1]]);
    let v = |s: &[u8], a: usize| -> [f32; 4] { std::array::from_fn(|c| f(s, a + 4 * c)) };
    let m = |s: &[u8], a: usize| -> [[f32; 4]; 4] { std::array::from_fn(|r| v(s, a + 16 * r)) };
    npc::Trigger {
        ty: s[o + 0x50],
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
        pitch: f(s, 0x184),
        yaw: f(s, 0x188),
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
        done: s[o + 0xbc] != 0,
        counter,
        gap: 0,
        voice: (-1, 0),
        startled: false,
        scrub: (0, 0),
        struck: false,
        ..Default::default()
    }
}

/// Court 4's passing ball (type 15) against `context/fixtures/ball_c04.bin` (research/b24_rec15.py, an all-computer
/// doubles match from its first change of ends: route 0) and `ball_c04_r1..3.bin` (`b24_rec15.py -9 … <mti>` from a
/// scratch save before a change of ends, its played flags cleared, the generator's index set so the roll picks route
/// 1–3; not in git, skipped when absent): every tick from the recorded state before, through the change of ends'
/// reset that rolls it off (the route's matrix), its play with the bounce sounds and the fade, to the reset that
/// leaves it inactive, must give the game's state after bit for bit.
#[test]
fn passing_ball_matches_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let (row, routes) = (game.trigger(15), game.ball_routes());
    for (want, name) in ["ball_c04", "ball_c04_r1", "ball_c04_r2", "ball_c04_r3"].into_iter().enumerate() {
        let Ok(d) = std::fs::read(format!("{root}/context/fixtures/{name}.bin")) else {
            eprintln!("{name}.bin missing, skipped");
            continue;
        };
        let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        // a sample: vsync, match state, the object, its buffer (playing, route), controller, animation header, generator
        let samples: Vec<&[u8]> = d[4..].chunks_exact(5 + 0x290 + 8 + 0x40 + 0x30 + 0x9d0).collect();
        let mt = 5 + 0x290 + 8 + 0x40 + 0x30;
        let state = |s: &[u8]| {
            // the object, controller and header as tools/record_npc.py lays them out
            let rec = [&s[5..5 + 0x290], &s[5 + 0x298..mt]].concat();
            let f = |a: usize| f32::from_bits(u(&rec, a));
            let v = |a: usize| -> [f32; 4] { std::array::from_fn(|c| f(a + 4 * c)) };
            let buf = &s[5 + 0x290..];
            npc::Trigger {
                playing: buf[0] != 0,
                route: u(buf, 4),
                routes,
                done: rec[0xbc] != 0,
                // the animation's length (the game's controller has none until it first plays)
                len: 240.0,
                // ponytail: its path (the path manager is not ported; it stands still, its animation carries it)
                path: vec![],
                moving: false,
                target: [0.0; 4],
                from: [0.0; 4],
                to: [0.0; 4],
                left: 0,
                ..trigger_at(&rec, 0, &row, v(0x160), v(0x150), 0)
            }
        };
        let (mut ticks, mut resets, mut sounds, mut rolled) = (0, 0, 0, None);
        for i in 1..samples.len() {
            let (a, b) = (samples[i - 1], samples[i]);
            assert_eq!(u(b, 0), u(a, 0) + 1, "a sample missed at {i}");
            let (before, after) = (state(a), state(b));
            let mut rng = Mt::of(&a[mt..]);
            let (end, mut out) = (Mt::of(&b[mt..]), Vec::new());
            while rng != end && out.len() < 2000 {
                out.push(rng.next());
            }
            // a message before the tick: 0xc (entering the change of ends), 0xe (the serve set-up: a point's end, the
            // change of ends' state 1) and 0x1a reset it; only the reset in state 1 rolls
            let reset = after.msg != before.msg && matches!(after.msg, 0xc | 0xe | 0x1a);
            let near = |s: &[u8]| npc::Near { ends: s[4] == 1, players: 4, ..Default::default() };
            let found = (0..=out.len()).find_map(|j| {
                let (mut t, mut it, mut heard) = (before.clone(), out[j..].iter(), Vec::new());
                let mut roll = || *it.next().unwrap_or(&0);
                // the creature's handler: 0xc message 8 and a reset, 0xe/0x1a a reset gated by the match's state
                if after.msg != before.msg {
                    heard.extend(t.hear(&row, after.msg, false, b[4], &mut near(b), &mut roll));
                }
                heard.extend(t.step_near(&row, &mut near(b), &mut roll));
                // (what the serve set-up keeps aside is checked by `serve_setup_keeps_and_restores_like_the_game`)
                (t.kept, t.kept_done) = (after.kept, after.kept_done);
                (t == after).then_some((t, heard))
            });
            let Some((t, heard)) = found else {
                let mut t = npc::Trigger { msg: after.msg, ..before.clone() };
                if reset {
                    t.reset_near(&row, &mut near(b), &mut || out.first().copied().unwrap_or(0));
                }
                t.step_near(&row, &mut near(b), &mut || out.first().copied().unwrap_or(0));
                panic!("vsync {} (reset {reset}):\n from {before:?}\n want {after:?}\n  got {t:?}", u(b, 0))
            };
            if t.playing && !before.playing {
                rolled = Some(t.route);
            }
            // each bounce plays at its route's next sound point, ×1.2 the effect level
            if !heard.is_empty() {
                let p = game.ball_sounds()[t.route as usize][sounds];
                assert_eq!(npc::Trigger { bounces: game.ball_sounds(), ..t.clone() }.sound_at(), [p[0], p[1], p[2]]);
                assert_eq!(npc::trigger_sound(15, heard[0]).volume, 138);
                eprintln!("bounce {sounds} at frame {} plays at {p:?}", t.frame);
            }
            (ticks, resets, sounds) = (ticks + 1, resets + reset as usize, sounds + heard.len());
        }
        eprintln!("{name}: {ticks} ticks, {resets} resets, {sounds} bounce sounds");
        assert!(rolled.is_some() && sounds == 4, "{name}: the ball did not play through");
        assert_eq!(rolled, Some(want as u32), "{name}: the roll picked another route");
    }
}

/// The ball-hit obstacles (14 on court 4, 38 on court 9, 43 on court 10) against `context/fixtures/hit_cNN.bin`
/// (`tools/record_npc.py … prox` with `HST_BALL=1 HST_MATS=1` and pokes onto the ball and players; not in git, skipped
/// when absent): every tick of each obstacle from the recorded state before must give the game's state after bit for
/// bit — the latch, animation and clip, type 38's turn to the ball — and a struck one must be the tick's message 0x14.
/// What they see (players' and ball's boxes, the ball's last move) is the state before the tick (the creatures run
/// before the ball and players: taking any of the three from the state after fails).
#[test]
fn ball_hit_obstacles_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let (mut files, mut hits, mut anims, mut torn) = (0, 0, 0, 0);
    for name in ["hit_c04", "hit_c09", "hit_c10", "hit_c10b"] {
        let Ok(d) = std::fs::read(format!("{root}/context/fixtures/{name}.bin")) else {
            eprintln!("{name}.bin missing, skipped");
            continue;
        };
        files += 1;
        let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let f = |b: &[u8], o: usize| f32::from_bits(u(b, o));
        let n = u(&d, 0) as usize;
        let (mt, wo) = (4 + 0x180, |k: usize| 4 + 0x180 + 0x9d0 + 0x280 + 0x20 + 8 + k * 0x300);
        let (x, ball) = (wo(n), wo(n) + 0x90 + 8 * n);
        let samples: Vec<&[u8]> = d[4 + 4 * n..].chunks_exact(ball + 0x290 + 0x100).collect();
        let m = |s: &[u8], o: usize| -> [[f32; 4]; 4] { std::array::from_fn(|r| std::array::from_fn(|c| f(s, o + 16 * r + 4 * c))) };
        let near = |s: &[u8]| {
            let players = u(s, 4 + 0x24) as usize;
            npc::Near {
                pos: (1..=players).chain([0]).map(|i| std::array::from_fn(|c| f(s, x + 0x10 * i + 4 * c))).collect(),
                boxes: (0..players).map(|i| m(s, ball + 0x290 + 0x40 * i)).chain([m(s, ball + 0xb0)]).collect(),
                kmh: npc::ball_kmh(std::array::from_fn(|c| f(s, ball + 0x140 + 4 * c))),
                ..Default::default()
            }
        };
        // type 38's clip lengths, as each clip played in the recording (from its model on the disc in the app)
        let clip_lens = |k: usize| -> [f32; 2] {
            std::array::from_fn(|c| samples.iter().find(|s| u(s, wo(k) + 0x290 + 0x20) == c as u32).map_or(0.0, |s| f(s, wo(k) + 0x2d0 + 0x2c)))
        };
        let kinds: Vec<(usize, u8)> =
            (0..n).map(|k| (k, samples[0][wo(k) + 0x50])).filter(|&(_, ty)| matches!(ty, 14 | 21 | 38 | 43)).collect();
        let mut skip = vec![false; n];
        for i in 1..samples.len() {
            let (a, b) = (samples[i - 1], samples[i]);
            if u(b, 0) != u(a, 0) + 1 {
                continue;
            }
            let mut tick14 = false;
            for &(k, ty) in &kinds {
                if std::mem::take(&mut skip[k]) {
                    continue;
                }
                let o = wo(k);
                let row = game.trigger(ty);
                let v = |s: &[u8], a: usize| -> [f32; 4] { std::array::from_fn(|c| f(s, o + a + 4 * c)) };
                let state = |s: &[u8]| npc::Trigger {
                    startled: s[o + 0x278] != 0,
                    alt: ty == 38 && u(s, o + 0x290 + 0x20) == 1,
                    clip_lens: if ty == 38 { clip_lens(k) } else { [0.0; 2] },
                    ..trigger_at(s, o, &row, v(s, 0x160), v(s, 0x150), 0)
                };
                let (mut before, after) = (state(a), state(b));
                if !before.moving && before.world[3] != after.world[3] {
                    before.world[3] = after.world[3]; // poked before the tick
                }
                let new14 = after.msg == 0x14 && before.msg != 0x14;
                if new14 {
                    before.msg = 0x14;
                }
                let mut rng = Mt::of(&a[mt..]);
                let (end, mut out) = (Mt::of(&b[mt..]), Vec::new());
                while rng != end && out.len() < 2000 {
                    out.push(rng.next());
                }
                if before.msg != after.msg {
                    // a reset message: the obstacle's reset (msg 4)
                    if matches!(after.msg, 0xc | 0xe | 0x1a) {
                        let mut t = before.clone();
                        t.reset_near(&row, &mut near(a), &mut || out[0]);
                        // what the obstacle's reset owns (the route fields, which a path-less one never reads, are the
                        // generic reset's)
                        let own = |t: &npc::Trigger| (t.on, t.moving, t.world, t.startled, t.animating, t.alt, t.frame);
                        assert_eq!(own(&t), own(&after), "{name} vsync {} creature {k} type {ty}: reset", u(b, 0));
                    }
                    continue;
                }
                let found = (0..=out.len()).find_map(|j| {
                    let (mut t, mut it) = (before.clone(), out[j..].iter());
                    let heard = t.step_near(&row, &mut npc::Near { pos: near(b).pos, ..near(a) }, &mut || *it.next().unwrap_or(&0));
                    let want = npc::Trigger { struck: t.struck, path: t.path.clone(), anchor: t.anchor, ..after.clone() };
                    (t == want).then_some((t, heard))
                });
                let Some((t, heard)) = found else {
                    let mut t = before.clone();
                    t.step_near(&row, &mut npc::Near { pos: near(b).pos, ..near(a) }, &mut || out.first().copied().unwrap_or(0));
                    // a torn real-time sample: an animating one read a tick late (it shows the next sample's state)
                    let next = samples.get(i + 1).map(|c| state(c));
                    if t.startled && before.startled && next.as_ref().is_some_and(|c| c.frame == after.frame && t.frame < after.frame) {
                        (torn, skip[k]) = (torn + 1, true); // and the tick from it
                        continue;
                    }
                    panic!("{name} vsync {} creature {k} type {ty} (kmh {}):\n from {before:?}\n want {after:?}\n  got {t:?}", u(b, 0), near(a).kmh)
                };
                assert!(!t.struck || after.msg == 0x14, "{name} vsync {} creature {k}: struck without message 0x14", u(b, 0));
                tick14 |= t.struck;
                hits += (t.startled && !before.startled) as usize;
                anims += (t.startled && !before.startled && t.animating) as usize;
                // its sound with its animation (14/21 by a player too; the sound handle isn't kept to check)
                assert_eq!(!heard.is_empty(), t.animating && !before.animating, "{name} vsync {}: sound {heard:?}", u(b, 0));
            }
            let new14 = kinds.iter().any(|&(k, _)| u(b, wo(k) + 0xc0) == 0x14 && u(a, wo(k) + 0xc0) != 0x14);
            // (a second 0x14 onto creatures still holding the first changes nothing to see)
            assert!(tick14 || !new14, "{name} vsync {}: message 0x14 with nothing struck", u(b, 0));
        }
    }
    eprintln!("{files} files: {hits} hits, {anims} with the animation ({torn} torn samples skipped)");
    assert!(torn <= 2, "{torn} torn samples");
}

/// The match-event creatures (ported from the game's code, no recording: in-match presentation): court 1's
/// landing-spot creature, court 2's flyover, courts 5 and 8's game-end rolls, court 6's and 10's deciding-set pause,
/// court 9's cheer. Their draws: only the game-end roll (one each, armed or not).
#[test]
fn match_event_creatures() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let make = |ty: u8| {
        let row = game.trigger(ty);
        let mut t = npc::Trigger { ty, len: 30.0, ..Default::default() };
        t.home[3] = [1.0, 0.0, 2.0, 1.0];
        t.reset(&row, &mut || 0);
        (t, row)
    };
    let mut none = || -> u32 { panic!("no draw") };

    // 33: a game's end arms its sound one time in two, played at the reset, once a match
    let (mut t, row) = make(33);
    let mut near = npc::Near::default();
    assert!(t.message(&row, 8, &mut near, &mut || 1 << 16).is_empty() && !t.armed);
    t.message(&row, 8, &mut near, &mut || 0);
    assert_eq!(t.reset_near(&row, &mut near, &mut none), vec![row.sound]);
    t.message(&row, 8, &mut near, &mut || 0);
    assert!(!t.armed);
    assert!(t.reset_near(&row, &mut near, &mut none).is_empty());

    // 19: its sweep, a call every 10 ticks for 360, louder to the middle and turning; then gone for the match
    let (mut t, row) = make(19);
    t.message(&row, 8, &mut near, &mut || 0);
    assert!(t.reset_near(&row, &mut near, &mut none).is_empty() && t.playing);
    let mut calls = Vec::new();
    for _ in 0..400 {
        if !t.step_near(&row, &mut near, &mut none).is_empty() {
            calls.push((t.bearing, t.volume));
        }
    }
    assert_eq!((calls.len(), calls[0], calls[18]), (36, (135, 0), (45, 128)));
    assert!(t.done && !t.playing);
    t.reset_near(&row, &mut near, &mut none);
    assert!(!t.active);

    // 40: cheers when the team with no human wins a game in under three shots
    let (mut t, row) = make(40);
    near.humans = [true, false];
    t.reset_near(&row, &mut near, &mut none);
    near.game = true;
    for (winner, shots, cheer) in [(1, 2, true), (0, 2, false), (1, 3, false)] {
        (near.winner, near.shots) = (winner, shots);
        assert_eq!(t.message(&row, 7, &mut near, &mut none) == vec![row.sound], cheer);
    }
    near.humans = [true, true];
    t.reset_near(&row, &mut near, &mut none);
    (near.winner, near.shots) = (1, 1);
    assert!(t.message(&row, 7, &mut near, &mut none).is_empty());

    // 2: at the first bounce's spot (in the court) after a game won on a plain call with a named point, facing the eye
    let (mut t, row) = make(2);
    (near.bounces, near.landing, near.in_court, near.eye, near.named, near.call) = (1, [3.0, 0.0, 4.0, 1.0], true, [3.0, 5.0, 10.0, 1.0], true, 0);
    t.step_near(&row, &mut near, &mut none);
    near.bounces = 2;
    t.step_near(&row, &mut near, &mut none);
    assert_eq!(t.landing, Some([3.0, 0.0, 4.0, 1.0]));
    t.message(&row, 6, &mut near, &mut none);
    assert!(t.on && near.flags[2]);
    assert_eq!((t.world[2], t.world[3]), ([0.0, 0.0, 1.0, 0.0], [3.0, 0.0, 4.0, 1.0]));
    t.reset_near(&row, &mut near, &mut none);
    assert!(!t.on && t.landing.is_none() && !near.flags[2]);

    // 6: the flyover's sound fades with |z|; it flies on at y −60
    let (mut t, row) = make(6);
    t.vel = [0.0, 0.0, 1.0, 0.0];
    assert_eq!(t.message(&row, 7, &mut near, &mut none), vec![row.sound]);
    assert_eq!(t.volume, 127);
    t.step_near(&row, &mut near, &mut none);
    assert_eq!(t.world[3][1..3], [-60.0, 3.0]);
    near.game = false;
    t.reset_near(&row, &mut near, &mut none);
    assert!(t.message(&row, 7, &mut near, &mut none).is_empty() && !t.on);

    // 22 and 44: the gallery's manager paused at a game's end in the deciding set only
    for ty in [22, 44] {
        let (mut t, row) = make(ty);
        for deciding in [true, false] {
            near.deciding = deciding;
            t.reset_near(&row, &mut near, &mut || 0);
            near.paused = !deciding;
            t.message(&row, 8, &mut near, &mut none);
            assert_eq!(near.paused, deciding);
        }
    }
}

/// The serve set-up's message against `context/fixtures/trig_replay_s05.bin` (tools/record_npc.py 5 1400 … trig with
/// HST_GM=1, HST_REPLAY=1 and HST_BYTE="10:0:0xbc:1;500:0:0xbc:0;500:2:0xbc:1": court 10's creatures with `done`
/// poked on before the first serve set-up and swapped during the point, then a forced replay; not in git, skipped
/// when absent): at each of its three arrivals (the match's first serve, the replay's start, the serve after it) every
/// creature's `done`, kept `done`, kept matrix and place and `active` must be the game's.
#[test]
fn serve_setup_keeps_and_restores_like_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    let Ok(d) = std::fs::read(format!("{root}/context/fixtures/trig_replay_s05.bin")) else {
        return eprintln!("trig_replay_s05.bin missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let h = |b: &[u8], o: usize| i16::from_le_bytes([b[o], b[o + 1]]);
    let m = |b: &[u8], o: usize| -> [[f32; 4]; 4] { std::array::from_fn(|r| std::array::from_fn(|c| f32::from_bits(u(b, o + 16 * r + 4 * c)))) };
    let n = u(&d, 0) as usize;
    let wo = |k: usize| 4 + 0x180 + 0x9d0 + 0x280 + 0x20 + 8 + k * 0x300;
    let samples: Vec<&[u8]> = d[4 + 4 * n..].chunks_exact(wo(n) + 0x360).collect();
    let gm = wo(n);
    let (mut arrivals, mut restored) = (Vec::new(), 0);
    for i in 1..samples.len() {
        let (a, b) = (samples[i - 1], samples[i]);
        if u(b, wo(0) + 0xc0) != 0xe || u(a, wo(0) + 0xc0) == 0xe {
            continue;
        }
        // (the replay's start can miss a frame: a tick between only steps, which leaves these fields alone)
        let (replaying, phase) = (b[gm + 0x344] != 0, b[gm + 0x54]);
        arrivals.push((replaying, phase));
        for k in 0..n {
            let o = wo(k);
            let row = game.trigger(a[o + 0x50]);
            let mut t = trigger_at(a, o, &row, [0.0; 4], [0.0; 4], 0);
            // the drawn matrix (the one kept) is the step's copy of the matrix
            (t.world, t.kept_done) = (m(a, o + 0x1f0), a[o + 0xbd] != 0);
            t.kept = (m(a, o + 0xe0), h(a, o + 0x120), h(a, o + 0x122), h(a, o + 0x124), h(a, o + 0xda));
            t.serve_setup(&row, replaying, phase, &mut npc::Near::default(), &mut || panic!("rolled"));
            let what = format!("sample {i}, creature {k} (type {})", t.ty);
            assert_eq!((t.done, t.kept_done, t.active), (b[o + 0xbc] != 0, b[o + 0xbd] != 0, b[o + 0x12c] != 0), "{what}");
            assert_eq!(t.kept, (m(b, o + 0xe0), h(b, o + 0x120), h(b, o + 0x122), h(b, o + 0x124), h(b, o + 0xda)), "{what}");
            restored += (replaying && a[o + 0xbc] != b[o + 0xbc]) as usize;
        }
    }
    eprintln!("serve set-ups {arrivals:?}, {restored} done flags put back");
    assert_eq!(arrivals.iter().map(|a| a.0).collect::<Vec<_>>(), [false, true, false]);
    assert_eq!(restored, 2, "the poked creatures' done not put back at the replay");
}

/// Court 5's own creatures against `context/p14d/run1/c5.bin` (research/p14d_record.py: a doubles match on court 5
/// from the intro; not in git, skipped when absent). Every tick of all six, from the recorded state of the tick
/// before, must reproduce the game's tick count, cheering flag, UV frame, level and direction: seated again at the
/// serve set-up and when a point's end is left, standing up at a point's end when [`npc::crowd_cheers_point`] says
/// so, each level one draw of the shared generator in creature order.
#[test]
fn court5_crowd_like_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(d) = std::fs::read(format!("{root}/context/p14d/run1/c5.bin")) else {
        eprintln!("c5.bin missing, skipped");
        return;
    };
    let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let n = u(&d, 0) as usize;
    let (mt, cr) = (0x34, 0x34 + 0x9d0);
    let samples: Vec<&[u8]> = d[4 + 4 * n..].chunks_exact(cr + 0xc0 * n).collect();
    let state = |s: &[u8], k: usize| {
        let o = cr + 0xc0 * k;
        npc::Crowd {
            base: u(s, o + 0x58) as i32,
            on: s[o + 0xa8] != 0,
            tick: u(s, o + 0xa4) as i32,
            frame: u(s, o + 0xac) as i32,
            level: u(s, o + 0xb0) as i32,
            up: s[o + 0xb4] != 0,
        }
    };
    let (mut ticks, mut cheers, mut resets, mut stalls) = (0, 0, 0, 0);
    for i in 1..samples.len() {
        let (a, b) = (samples[i - 1], samples[i]);
        if u(b, 0) != u(a, 0) + 1 {
            continue; // a frame missed between them
        }
        let (pa, pb) = (a[9], b[9]);
        let reset = pa != pb && (pb == 2 || pa == 4);
        let cheer = pa != pb && pb == 4 && npc::crowd_cheers_point(b[0x1a] != 0, u(b, 0x24) != 0, b[0x32]);
        let mut rng = Mt::of(&a[mt..]);
        let end = Mt::of(&b[mt..]);
        let mut out = Vec::new();
        while rng != end && out.len() < 2000 {
            out.push(rng.next());
        }
        // a long frame: the game ticks twice between two vsyncs
        let run = |j: usize, steps: usize| {
            let mut it = out[j..].iter().copied();
            let mut roll = || it.next().unwrap_or(0);
            (0..n)
                .map(|k| {
                    let mut c = state(a, k);
                    // the phase's message reaches them before their tick
                    if reset {
                        c.reset();
                    }
                    if cheer {
                        c.cheer(&mut roll);
                    }
                    (0..steps).for_each(|_| c.step());
                    c
                })
                .collect::<Vec<_>>()
        };
        let want: Vec<_> = (0..n).map(|k| state(b, k)).collect();
        if !reset && !cheer && (0..n).all(|k| state(a, k) == want[k]) {
            stalls += want.iter().any(|c| c.on) as usize; // the objects weren't ticked this frame (the intro's loading, a long frame)
            continue;
        }
        let starts = if cheer { 0..out.len().saturating_sub(n - 1) } else { 0..1 };
        assert!(
            starts.clone().any(|j| (1..=2).any(|t| run(j, t) == want)),
            "vsync {} phase {pa}->{pb}:\n from {:?}\n got {:?}\n want {want:?}",
            u(b, 0),
            (0..n).map(|k| state(a, k)).collect::<Vec<_>>(),
            run(0, 1)
        );
        ticks += n;
        cheers += cheer as usize;
        resets += reset as usize;
    }
    eprintln!("court 5: {ticks} creature ticks, {cheers} cheers, {resets} resets, {stalls} cheering frames not ticked");
    assert!(cheers >= 5 && resets >= 5 && stalls < samples.len() / 10);
}

/// The ball the ball-hit obstacles test (P14c4a) against `hit_c04.bin` and `hit_c09.bin` (`HST_BALL=1`; not in git,
/// skipped when absent), every tick: each ball step turns its model matrix (+0xb0) by its spin (`ball::spin_model`,
/// a tick with an obstacle's plane step takes two) and places it, and its last move (+0x140) is the step; a tick
/// without a step keeps both; a launch (toss, serve hit, new point) takes `Flight::new`/`Flight::struck`'s.
#[test]
fn ball_model_and_last_move_match_the_game() {
    use hst_sim::ball::{spin_model, Ball, Flight};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (mut steps, mut doubles, mut still, mut new, mut struck) = (0, 0, 0, 0, 0);
    for name in ["hit_c04", "hit_c09"] {
        let Ok(d) = std::fs::read(format!("{root}/context/fixtures/{name}.bin")) else {
            eprintln!("{name}.bin missing, skipped");
            continue;
        };
        let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let n = u(&d, 0) as usize;
        let ball = 4 + 0x180 + 0x9d0 + 0x280 + 0x20 + 8 + n * 0x300 + 0x90 + 8 * n;
        let f = |s: &[u8], a: usize| f32::from_bits(u(s, ball + a));
        let v = |s: &[u8], a: usize| -> [f32; 3] { std::array::from_fn(|c| f(s, a + 4 * c)) };
        let m = |s: &[u8], a: usize| -> [[f32; 4]; 4] { std::array::from_fn(|r| std::array::from_fn(|c| f(s, a + 16 * r + 4 * c))) };
        let bits = |m: [[f32; 4]; 4]| m.map(|r| r.map(f32::to_bits));
        let samples: Vec<&[u8]> = d[4 + 4 * n..].chunks_exact(ball + 0x290 + 0x100).collect();
        for w in samples.windows(2) {
            let (a, b) = (w[0], w[1]);
            if u(b, 0) != u(a, 0) + 1 {
                continue;
            }
            let (pa, p, want, last) = (v(a, 0xe0), v(b, 0xe0), bits(m(b, 0xb0)), v(b, 0x140).map(f32::to_bits));
            let turn = |x: &[[f32; 4]; 4], s: &[u8]| spin_model(x, &m(s, 0x160), f(s, 0x1a4), 1.0, p);
            let at = format!("{name} vsync {}", u(b, 0));
            let (fa, fb) = (u(a, ball + 0xac), u(b, ball + 0xac));
            if fb == 0 && fa != 0 {
                // launched this tick: the model as the launch sets it, the last move its velocity
                let l = Ball { pos: p, vel: v(b, 0x130), spin: f(b, 0x1a4) };
                let (fresh, hit) = (Flight::new(l, m(b, 0x160), m(b, 0x160)), Flight::struck(l, m(b, 0x160), m(b, 0x160)));
                assert!(bits(fresh.model) == want || bits(hit.model) == want, "{at}: launch model {:?}", m(b, 0xb0));
                if bits(hit.model) == want && bits(fresh.model) != want { struck += 1 } else { new += 1 }
                assert_eq!(hit.last.map(f32::to_bits), last, "{at}: launch move");
                continue;
            }
            // a still ball placed on the server's hand before the toss steps by nothing although it was moved
            let held = v(a, 0x130) == [0.0; 3] && v(b, 0x130) == [0.0; 3] && last == [0; 3];
            let diff: [u32; 3] = std::array::from_fn(|c| hst_sim::ps2::sub(p[c], pa[c]).to_bits());
            let step = if held { [0; 3] } else { diff };
            match fb - fa {
                0 => {
                    assert_eq!(bits(m(a, 0xb0))[..3], want[..3], "{at}: standing ball turned");
                    assert_eq!(v(a, 0x140).map(f32::to_bits), last, "{at}: standing ball's move not kept");
                    still += 1;
                }
                1 => {
                    assert_eq!(bits(turn(&m(a, 0xb0), b)), want, "{at}: step turn");
                    assert_eq!(step, last, "{at}: step move");
                    steps += 1;
                }
                2 => {
                    // the plane step's own frame and spin aren't recorded: it turned by the tick's start or end ones
                    let x = m(a, 0xb0);
                    assert!([turn(&turn(&x, b), b), turn(&turn(&x, a), b)].iter().any(|t| bits(*t) == want), "{at}: two-step turn");
                    assert_eq!(step, last, "{at}: two-step move");
                    doubles += 1;
                }
                k => panic!("{at}: {k} ball steps in a tick"),
            }
        }
    }
    eprintln!("{steps} steps, {doubles} two-step ticks, {still} standing, launches: {new} new, {struck} struck");
    if steps > 0 {
        assert!(steps > 300 && doubles >= 5 && still > 50 && new >= 2 && struck >= 2);
    }
}

/// The beach plane (trigger 6 on court 2): a named game win sets it off with its engine sound; it then flies at
/// y = -60 about 4 units a tick from the sea end to the far end, as the game's RAM shows (z 343.77, then 335.78,
/// 327.79 ... at -60 every other tick), its volume (|z| over 550, squared) loudest overhead.
#[test]
fn beach_plane_flies_over() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut iso = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")).unwrap();
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let (entries, plants) = court_layout(&mut iso, 2);
    let c = npc::spawn(&entries, &plants, &game.npc_roster(2), &game.walkers(2), 2)
        .into_iter()
        .find(|c| matches!(c.kind, npc::Kind::Trigger(6)))
        .unwrap();
    let row = game.trigger(6);
    assert_eq!(row.sound, 30);
    let mut t = npc::Trigger { ty: 6, anchor: c.world[3], home: c.world, world: c.world, path: c.path.clone(), radii: c.radii.clone(), start: c.start, len: 30.0, ..Default::default() };
    t.reset(&row, &mut || 0);
    let mut near = npc::Near { game: true, named: false, ..Default::default() };
    assert!(t.message(&row, 7, &mut near, &mut || 0).is_empty(), "an unnamed point leaves it home");
    near.named = true;
    assert_eq!(t.message(&row, 7, &mut near, &mut || 0), [30]);
    let (mut z, mut loudest) = (t.world[3][2], 0);
    for _ in 0..180 {
        t.step_near(&row, &mut near, &mut || 0);
        assert!(t.on && t.world[3][1] == -60.0 && (z - t.world[3][2] - 3.9857666).abs() < 1e-3);
        z = t.world[3][2];
        loudest = loudest.max(t.volume);
    }
    assert!(z < -370.0 && loudest >= 127 && t.volume < 70);
    t.message(&row, 4, &mut near, &mut || 0);
    assert!(!t.on && t.world == t.home && t.volume == -1);
}
