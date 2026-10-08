//! The game's random sources against `context/p3b/rng_s05.bin` (research/p3b_rng_rec.py, slot 5; skipped when
//! absent): from every frame's RAM-dumped generator, `Mt::next` must reach the next frame's state exactly (the
//! words through every regeneration and the index), or a reseed with one of the `rand()` outputs drawn in between
//! must: at a new point the shared generator takes the frame's first output and the court's the second
//! (`Rngs::new_point`), and the sound manager's takes the first at change ends (`Rngs::change_ends`).
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
