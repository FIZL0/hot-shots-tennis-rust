//! The game's random sources against `context/p3b/rng_s05.bin` (research/p3b_rng_rec.py, slot 5; skipped when
//! absent): from every frame's RAM-dumped generator, `Mt::next` must reach the next frame's state exactly (the
//! words through every regeneration and the index), or a reseed with one of the `rand()` outputs drawn in between
//! must: at a new point the shared generator takes the frame's first output and the court's the second
//! (`Rngs::new_point`), and the sound manager's (`Rngs::change_ends`) the next (8656's sample is torn between them:
//! `sound_draws_like_the_game` has the order).
use hst_sim::rng::{Mt, Rand, Rngs};

/// Draws from `a` that land on `b`, if any within `max`.
fn reach(a: &Mt, b: &Mt, max: usize) -> Option<usize> {
    let mut m = a.clone();
    (0..=max).find(|_| m == *b || {
        m.next();
        false
    })
}

#[test]
fn generators_step_like_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(d) = std::fs::read(format!("{root}/context/p3b/rng_s05.bin")) else {
        return eprintln!("rng_s05.bin missing, skipped");
    };
    const NAMES: [&str; 4] = ["shared", "ai", "court", "sound"];
    let size = 4 + 8 + 4 * 0x9d0;
    let samples: Vec<(u32, Rand, [Mt; 4])> = d
        .chunks_exact(size)
        .map(|s| {
            let u = |o: usize| u32::from_le_bytes(s[o..o + 4].try_into().unwrap());
            let rand = Rand(u64::from_le_bytes(s[4..12].try_into().unwrap()));
            (u(0), rand, std::array::from_fn(|k| Mt::from_ram(&s[12 + k * 0x9d0..])))
        })
        .collect();
    let (mut draws, mut reseeds) = ([0usize; 4], [0usize; 4]);
    for w in samples.windows(2) {
        let ((va, ra, a), (vb, rb, b)) = (&w[0], &w[1]);
        // rand() steps from one frame's state to the next
        let mut r = *ra;
        for _ in 0..64 {
            if r == *rb {
                break;
            }
            r.next();
        }
        assert_eq!(r, *rb, "vsync {va}→{vb}: rand() state not reached");
        for k in 0..4 {
            if let Some(n) = reach(&a[k], &b[k], 4000) {
                draws[k] += n;
                continue;
            }
            let mut g = Rngs::new(*ra, a[0].clone());
            if k == 3 { g.change_ends() } else { g.new_point() }
            let m = [&g.shared, &g.ai, &g.court, &g.sound][k];
            assert!(k != 1 && reach(m, &b[k], 4000).is_some(), "vsync {va}→{vb}: {} generator neither stepped nor reseeded as the game does", NAMES[k]);
            reseeds[k] += 1;
        }
    }
    eprintln!("{} frames: draws {draws:?}, reseeds {reseeds:?}", samples.len());
    assert!(draws.iter().all(|&n| n > 0), "every generator draws over the recording");
}

/// The court generator's draws over a point against `context/p3b/rng_s05.bin` and the same run's
/// `context/fixtures/npc_s05.bin` (walkers) and `trig_s05.bin` (figures): from the generator reseeded at the
/// new point (vsync 7566) to the frame before the next one (8654), the walkers (their state at the start), the
/// gallery's pick of cheerers, its cheer and cheer marks, and the emitters, each frame stepping the ticks the game
/// ran in it (its emitters' countdowns), must leave the generator where the game's is every frame. The point is
/// decided where the walkers react.
#[test]
fn court_draws_like_the_game() {
    use hst_data::{exe::Game, iso::Iso};
    use hst_sim::{npc, sound};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(rng), Ok(walk), Ok(trig), Ok(mut iso)) = (
        std::fs::read(format!("{root}/context/p3b/rng_s05.bin")),
        std::fs::read(format!("{root}/context/fixtures/npc_s05.bin")),
        std::fs::read(format!("{root}/context/fixtures/trig_s05.bin")),
        Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")),
    ) else {
        return eprintln!("rng_s05.bin, npc_s05.bin, trig_s05.bin or the disc missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let f = |b: &[u8], o: usize| f32::from_bits(u(b, o));
    let at = |d: &[u8], size: usize, head: usize, v: u32| d[head..].chunks_exact(size).find(|s| u(s, 0) == v).map(|s| s.to_vec());
    // the court generator at the start of each frame
    let court = |v: u32| at(&rng, 12 + 4 * 0x9d0, 0, v).map(|s| Mt::from_ram(&s[12 + 2 * 0x9d0..]));
    // walkers
    let n = u(&walk, 0) as usize;
    let (mgr2, head) = (4 + 0x180 + 0x9d0 + 0x280, 4 + 4 * n);
    let wo = |k: usize| mgr2 + 0x20 + 8 + k * 0x380;
    let mut lens = vec![[0f32; 7]; n];
    for s in walk[head..].chunks_exact(wo(n)) {
        for (k, l) in lens.iter_mut().enumerate() {
            l[u(s, wo(k) + 200) as usize] = f(s, wo(k) + 0x350 + 0x2c);
        }
    }
    let walker = |s: &[u8], k: usize| {
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
            lens: lens[k],
        }
    };
    let walkers_at = |v: u32| at(&walk, wo(n), head, v).unwrap();
    // figures: the emitters and their countdowns
    let m = u(&trig, 0) as usize;
    let to = |k: usize| 4 + 0x180 + 0x9d0 + 0x280 + 0x20 + 8 + k * 0x300;
    let figures_at = |v: u32| at(&trig, to(m), 4 + 4 * m, v).unwrap();
    let ours: Vec<usize> = (0..m).filter(|&k| npc::EMITTERS.contains(&figures_at(7566)[to(k) + 0x50])).collect();
    let timer = |s: &[u8], k: usize| u(s, to(k) + 200) as i32;
    // the ticks in a frame: how far the countdowns that didn't re-arm went
    let ticks = |v: u32| {
        let (a, b) = (figures_at(v), figures_at(v + 1));
        ours.iter().map(|&k| timer(&a, k) - timer(&b, k)).filter(|&d| d >= 0).max().unwrap()
    };
    let (start, end) = (7566, 8654);
    let s = walkers_at(start);
    let players = u(&s, 4 + 0x24);
    let mut walkers: Vec<npc::Walker> = (0..n).map(|k| walker(&s, k)).collect();
    let decided = (start..end).find(|&v| walker(&walkers_at(v), 0).mode == 0 && walker(&walkers_at(v + 1), 0).mode == 2).unwrap();
    let s = figures_at(start);
    let mut emitters: Vec<npc::Emitter> = ours
        .iter()
        .map(|&k| {
            let ty = s[to(k) + 0x50];
            npc::Emitter { ty, row: game.emitter(ty), pos: [0.0; 3], timer: timer(&s, k), saved: 0, sweep: 0, pan: 0.0, down: false }
        })
        .collect();
    let (mut gallery, mut cheers, mut cheer) = (sound::Gallery::default(), npc::Cheers::default(), [false; 6]);
    let mut tick = u(&walkers_at(start + 1), mgr2 + 0x14) as i32 - ticks(start);
    let mut rng = court(start).unwrap();
    // the reseed's own tick (the frame before `start`): rng_s05 reads the generator fresh from the reseed (index
    // 624), but the walkers step after it in that tick, and walker 0 restarts its idle loop (one draw); npc_s05
    // shows them after it. Replay it from the frame before.
    let s = walkers_at(start - 1);
    let mut restarted: Vec<npc::Walker> = (0..n).map(|k| walker(&s, k)).collect();
    let mut draws = 0;
    for w in &mut restarted {
        w.new_point(players >= 3);
        w.step(players, false, tick - 1, &mut || {
            draws += 1;
            rng.next()
        });
    }
    assert_eq!((restarted, draws), (walkers.clone(), 1), "the reseed's tick");
    let mut frames = 0;
    for v in start..end {
        let mut draws = 0;
        let mut roll = || {
            draws += 1;
            rng.next()
        };
        for _ in 0..ticks(v) {
            if v == decided {
                cheer = npc::cheerers(n, &mut roll);
                gallery.point(sound::Reaction { cheer: true, event: Some(0), chain: false }, &mut roll);
                walkers.iter_mut().for_each(npc::Walker::react);
            }
            for w in &mut walkers {
                let react = w.mode == 1;
                w.step(players, cheer[w.slot as usize], tick, &mut roll);
                if react && w.anim == 5 {
                    cheers.add([0.0; 3], w.slot);
                }
            }
            // the gallery's manager after the walkers: its cheer, its cheer marks, its tick
            gallery.step(10, players, false, &mut roll);
            cheers.step(&mut roll);
            tick += 1;
            for e in &mut emitters {
                e.step(&mut roll);
            }
        }
        let want = court(v + 1).unwrap();
        let recorded = reach(&court(v).unwrap(), &want, 100);
        assert!(rng == want, "vsync {v}: {draws} court draws, the game {recorded:?}");
        frames += 1;
    }
    eprintln!("{frames} frames from {start}, the point decided at {decided}");
}

/// The sound manager's generator over a rally against `context/p3e2/sound_s05.bin` (research/p3e2_sound_rec.py 5
/// 1600: the effects object, the hit sparks and the generator per frame) and the same run's `rng_s05.bin`: every
/// frame's draws are the game's, word for word. A stroke flag (effects +0xd4, a byte per player) names the hitter;
/// a dive (branch 3) throws its ring (10 puffs, two uniforms each: the court is dusty); the hit (a spark burst
/// starting) draws the stroke's bit unless the stroke is clean (grade 1 or 2, branch 4 or a drop); a burst dying out
/// rerolls a quarter of the spark table. A new point (the shared generator's reseed in `rng_s05`) after a played
/// point rerolls all of it from the old generator and stops the burst, then reseeds the generator with the `rand()`
/// output after the shared and court seeds; the recording opens in a change of ends, whose new point (7566) leaves
/// both alone. The rolls match the game's bit for bit.
#[test]
fn sound_draws_like_the_game() {
    use hst_sim::effect::{Roll, SPARKS, Spark, Sparks};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(d), Ok(r)) = (std::fs::read(format!("{root}/context/p3e2/sound_s05.bin")), std::fs::read(format!("{root}/context/p3b/rng_s05.bin"))) else {
        return eprintln!("sound_s05.bin or rng_s05.bin missing, skipped");
    };
    const FX: usize = 4;
    const SP: usize = FX + 0x100;
    const PARTS: usize = SP + 0x70;
    const ROLLS: usize = PARTS + SPARKS * 0x40;
    const MT: usize = ROLLS + SPARKS * 0x50;
    let u = |s: &[u8], o: usize| u32::from_le_bytes(s[o..o + 4].try_into().unwrap());
    let rolls = |s: &[u8]| -> Vec<u32> { (0..SPARKS).flat_map(|i| (0..0x13).map(move |k| ROLLS + 0x50 * i + 4 * k)).map(|o| u(s, o)).collect() };
    let mine = |t: &[Roll; SPARKS]| -> Vec<u32> { t.iter().flat_map(|r| r.turn.concat().into_iter().chain(r.u)).map(f32::to_bits).collect() };
    let parts = |s: &[u8]| -> [Spark; SPARKS] { std::array::from_fn(|i| Spark { life: if u(s, PARTS + 0x40 * i) != 0 { u(s, PARTS + 0x40 * i + 0x34) as i32 } else { 0 }, ..Spark::default() }) };
    let frames: Vec<&[u8]> = d.chunks_exact(MT + 0x9d0).collect();
    // rng_s05: vsync → (rand(), shared generator)
    let rng: std::collections::HashMap<u32, (Rand, Mt)> = r
        .chunks_exact(12 + 4 * 0x9d0)
        .map(|s| (u(s, 0), (Rand(u64::from_le_bytes(s[4..12].try_into().unwrap())), Mt::from_ram(&s[12..]))))
        .collect();
    let f0 = frames[0];
    let table = |s: &[u8]| -> [Roll; SPARKS] {
        std::array::from_fn(|i| {
            let f = |k: usize| f32::from_bits(u(s, ROLLS + 0x50 * i + 4 * k));
            Roll { turn: std::array::from_fn(|j| std::array::from_fn(|k| f(4 * j + k))), u: [f(16), f(17), f(18)] }
        })
    };
    let mut sp = Sparks::new(table(f0));
    (sp.sparks, sp.live) = (parts(f0), u(f0, SP + 0x50) != 0);
    let mut mt = Mt::from_ram(&f0[MT..]);
    let (mut hitter, mut tally, mut played) = (None, [0usize; 5], false);
    for w in frames.windows(2) {
        let (a, b) = (w[0], w[1]);
        let v = u(b, 0);
        assert_eq!(v, u(a, 0) + 1, "vsync {v}: a frame is missing");
        let record = |p: usize| (b[FX + 0xdc + 8 * p], b[FX + 0xdd + 8 * p]);
        for p in (0..4).filter(|&p| b[FX + 0xd4 + p] != 0) {
            hitter = Some(p);
            if record(p).1 == 3 {
                // ponytail: counted only; the ring itself is checked by foot_extras_s05
                (0..20).for_each(|_| _ = mt.next());
                tally[0] += 1;
            }
        }
        let (live0, live1) = (u(a, PARTS) != 0, u(b, PARTS) != 0);
        let started = live1 && (!live0 || u(b, PARTS + 0x34) != u(a, PARTS + 0x34).wrapping_sub(1));
        let fresh = rng.get(&(v - 1)).zip(rng.get(&v)).is_some_and(|((_, s0), (_, s1))| reach(s0, s1, 4000).is_none());
        if fresh && std::mem::take(&mut played) {
            let (rand, shared) = rng[&(v - 1)].clone();
            let mut g = Rngs::new(rand, shared);
            g.new_point();
            sp.restart(&mut mt.clone());
            g.change_ends();
            mt = g.sound;
            tally[4] += 1;
        } else if started {
            played = true;
            let p = hitter.expect("a hit before any stroke");
            let ((grade, branch), kind) = (record(p), u(b, FX + 0xd0));
            if branch != 4 && kind != 4 && !matches!(grade, 1 | 2) {
                mt.bit();
                tally[1] += 1;
            }
            (sp.sparks, sp.live) = (parts(b), true);
        } else {
            let was = sp.live;
            sp.tick(&mut mt);
            tally[2] += (was && !sp.live) as usize;
            assert_eq!(sp.sparks.map(|s| s.life), parts(b).map(|s| s.life), "vsync {v}: the sparks' lives");
        }
        tally[3] += 1;
        assert!(mt == Mt::from_ram(&b[MT..]), "vsync {v}: the sound generator isn't the game's ({:?} draws from the last frame's)", reach(&Mt::from_ram(&a[MT..]), &Mt::from_ram(&b[MT..]), 400));
        assert_eq!(mine(&sp.rolls), rolls(b), "vsync {v}: the spark rolls");
        assert_eq!(sp.live, u(b, SP + 0x50) != 0, "vsync {v}: the burst");
    }
    eprintln!("{} frames: {} dive rings, {} stroke bits, {} burst ends, {} reseeds", tally[3], tally[0], tally[1], tally[2], tally[4]);
    assert!(tally[0] > 0 && tally[1] >= 5 && tally[2] >= 10 && tally[4] == 1);
}
