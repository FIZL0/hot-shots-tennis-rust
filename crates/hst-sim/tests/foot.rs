//! Footstep puffs and footprints against a recorded bot doubles match on court 10 (`context/fixtures/foot_s05.bin`,
//! `tools/record_foot.py 5`): from each player's toe bones, motion and matrix, every frame's step state, dust puffs
//! (the first 32) and footprints match the game bit for bit.

use hst_data::exe;
use hst_sim::foot::{Bit, DiveWatch, Feet, Print, Puff, Runner};

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v4(b: &[u8], o: usize) -> [f32; 4] {
    std::array::from_fn(|k| f(b, o + 4 * k))
}
fn m4(b: &[u8], o: usize) -> [[f32; 4]; 4] {
    std::array::from_fn(|r| v4(b, o + 16 * r))
}
fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

// Per frame: vsync 4, manager 0x40, run object 0x130, puffs 0x1000, prints 0xc80, print count 8, flags 0x10,
// tornado 0x100, ball 0x290, weather 0x10 + wind 0x10; per player (4) matrix 0x40, +0x3fa4 8, +0x3db0 8, motion
// object 0xa0, right and left toe bones 0x40.
const RUN: usize = 0x44;
const PUFFS: usize = RUN + 0x130;
const PRINTS: usize = PUFFS + 0x1000;
const NPRINT: usize = PRINTS + 0xc80;
const FLAGS: usize = NPRINT + 8;
const WIND: usize = FLAGS + 0x10 + 0x100 + 0x290 + 0x10;
const PL: usize = WIND + 0x10;
const PLSZ: usize = 0x40 + 8 + 8 + 0xa0 + 0x80;
const SIZE: usize = PL + 4 * PLSZ;

fn puff(b: &[u8]) -> Puff {
    Puff {
        phase: i(b, 4),
        timer: i(b, 8),
        size: f(b, 0xc),
        grow: f(b, 0x10),
        alpha: f(b, 0x14),
        fade: f(b, 0x18),
        pos: v4(b, 0x20),
        dir: v4(b, 0x30),
        foot: b[0x40] as usize,
        player: i(b, 0x44) as usize,
        aim: b[0x48] != 0,
        speed: f(b, 0x4c),
        fresh: b[0x50] != 0,
        scale: f(b, 0x70),
        kind: i(b, 0) as u8,
        vel: v4(b, 0x60),
    }
}
fn puff_key(u: &Puff) -> (Vec<u32>, [i32; 4], [bool; 2]) {
    // the game leaves the direction slot stale until the puff is aimed
    let dir = if u.aim { [0.0; 4] } else { u.dir };
    let v = [[u.size, u.grow, u.alpha, u.fade, u.speed, u.scale].as_slice(), &u.pos, &dir].concat();
    (bits(&v), [u.phase, u.timer, u.foot as i32, u.player as i32], [u.aim, u.fresh])
}
fn print(b: &[u8]) -> Print {
    Print { m: m4(b, 0), alpha: f(b, 0x40), life: i(b, 0x44), fade: f(b, 0x48) }
}
fn print_key(p: &Print) -> (Vec<u32>, i32) {
    (bits(&[p.m.concat().as_slice(), &[p.alpha, p.fade]].concat()), p.life)
}

/// The recorded state: step flags, cooldowns, type-0 puffs (None past the 32 recorded) and footprints.
fn state(fr: &[u8]) -> Feet {
    let h = &fr[RUN..];
    let n = i(h, 0x11c) as usize;
    let puffs = (0..n.min(32)).map(|k| &fr[PUFFS + 0x80 * k..]).filter(|b| i(b, 0) == 0).map(puff).collect();
    let np = i(fr, NPRINT) as usize;
    Feet {
        armed: std::array::from_fn(|p| std::array::from_fn(|f| h[0xd4 + 2 * p + f] != 0)),
        cooldown: std::array::from_fn(|p| std::array::from_fn(|f| i(h, 0xfc + 8 * p + 4 * f))),
        puffs,
        prints: (0..np).map(|k| print(&fr[PRINTS + 0x50 * k..])).collect(),
        after_point: h[0xc0] != 0,
        ..Feet::default()
    }
}

/// The step puffs (this recording has no bones for the sit-down bursts; `foot_extras` checks every kind).
fn steps0(a: &Feet) -> impl Iterator<Item = &Puff> {
    a.puffs.iter().filter(|u| u.kind == 0)
}

#[test]
fn footsteps_s05() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Ok(cnf), Ok(bin)) = (
        std::fs::read(format!("{root}/context/fixtures/foot_s05.bin")),
        std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")),
        std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")),
    ) else {
        return eprintln!("foot_s05.bin or disc absent, skipped");
    };
    let t = exe::Game::new(&cnf, &bin).unwrap().foot();
    let court = i(&data, 0) as usize;
    let frames: Vec<&[u8]> = data[8..].chunks_exact(SIZE).collect();
    let mut feet = state(frames[0]);
    let (mut steps, mut prints, mut checked, mut skipped, mut doubled, mut resets) = (0, 0, 0, 0, 0, 0);
    for k in 1..frames.len() {
        let (fr, h) = (frames[k], &frames[k][RUN..]);
        assert_eq!(i(fr, 0), i(frames[k - 1], 0) + 1, "frame {k}: gap");
        let was = state(frames[k - 1]);
        // the next point's start clears everything
        if was.after_point && h[0xc0] == 0 {
            resets += 1;
            feet = state(fr);
            continue;
        }
        feet.after_point = h[0xc0] != 0;
        let runners: Vec<Runner> = (0..4)
            .map(|p| {
                let b = &fr[PL + PLSZ * p..];
                Runner {
                    character: i(h, 0x7c + 4 * p) as usize,
                    motion: i(b, 0x50 + 0x20),
                    sub: i(b, 0x48),
                    toes: [v4(b, 0xf0 + 0x30), v4(b, 0x130 + 0x30)],
                    m: m4(b, 0),
                    slide: b[0x41] != 0,
                    ..Runner::default()
                }
            })
            .collect();
        let before = feet.prints.len();
        let want = state(fr);
        let full = i(h, 0x11c) <= 32;
        let same = |a: &Feet| {
            (a.armed, a.cooldown) == (want.armed, want.cooldown)
                && a.prints.iter().map(print_key).eq(want.prints.iter().map(print_key))
                && (!full || steps0(a).map(puff_key).eq(want.puffs.iter().map(puff_key)))
        };
        // the game sometimes skips the update a frame and catches up with two the next
        let tick = |a: &mut Feet| a.tick(&t, court, fr[FLAGS] != 0, fr[FLAGS + 1] != 0, v4(fr, WIND), &runners, h[0x120] != 0, &mut || unreachable!());
        let mut one = feet.clone();
        tick(&mut one);
        if !same(&one) && same(&feet) {
            skipped += 1;
            continue;
        }
        if !same(&one) {
            let mut two = one.clone();
            tick(&mut two);
            if same(&two) {
                doubled += 1;
                one = two;
            }
        }
        feet = one;
        assert_eq!((feet.armed, feet.cooldown), (want.armed, want.cooldown), "frame {k}: step state");
        assert_eq!(feet.prints.iter().map(print_key).collect::<Vec<_>>(), want.prints.iter().map(print_key).collect::<Vec<_>>(), "frame {k}: prints");
        if full {
            assert_eq!(steps0(&feet).map(puff_key).collect::<Vec<_>>(), want.puffs.iter().map(puff_key).collect::<Vec<_>>(), "frame {k}: puffs");
            checked += 1;
        }
        steps += feet.puffs.iter().filter(|u| u.timer == t.puffs[0].fade_in && u.phase == 0).count();
        prints += (feet.prints.len() > before) as usize;
    }
    eprintln!("{} frames ({skipped} without an update, {doubled} with two, {resets} points), {checked} puff frames, {steps} puffs, {prints} footprint frames", frames.len());
    assert!(steps > 100 && prints > 100);
}

// `foot_s05x.bin` (`record_foot.py 5 5000 … extras`): each frame as above, then the shared MT 0x9c8, run object
// +0x9ec0..+0xa0a0, per player +0x3f80 0x10, +0x3ec0 0x10 and the pelvis, spine and head world matrices.
const MT: usize = SIZE;
const DASH: usize = MT + 0x9c8;
const EX: usize = DASH + 0x1f0;
const EXSZ: usize = 0x20 + 0xc0;

/// A puff's live fields by kind (the game leaves the rest of a reused slot stale).
fn any_key(u: &Puff) -> (u8, Vec<u32>, [i32; 4], [bool; 2]) {
    match u.kind {
        0 => {
            let (v, i, b) = puff_key(u);
            (0, v, i, b)
        }
        1 => {
            let v = [[u.size, u.grow, u.alpha, u.fade, u.speed].as_slice(), &u.pos, &u.dir, &u.vel].concat();
            (1, bits(&v), [u.phase, u.timer, u.player as i32, 0], [u.fresh, false])
        }
        _ => (2, bits(&[[u.size, u.grow, u.alpha, u.fade, u.scale].as_slice(), &u.pos].concat()), [u.phase, u.timer, 0, 0], [false; 2]),
    }
}

fn mt(fr: &[u8]) -> hst_sim::weather::Mt {
    hst_sim::weather::Mt(std::array::from_fn(|k| i(fr, MT + 4 + 4 * k) as u32), i(fr, MT + 0x9c4) as usize)
}

#[test]
fn foot_extras_s05() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(cnf), Ok(bin)) = (std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")), std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN"))) else {
        return eprintln!("disc absent, skipped");
    };
    let t = exe::Game::new(&cnf, &bin).unwrap().foot();
    let (mut kinds, mut rings, mut dashes) = ([0; 3], 0, 0);
    // the w and d recordings force the court wet and dusty (`FOOT_POKE`) for the dive rings
    for name in ["x", "w", "d"] {
    let Ok(data) = std::fs::read(format!("{root}/context/fixtures/foot_s05{name}.bin")) else {
        eprintln!("foot_s05{name}.bin absent, skipped");
        continue;
    };
    let court = i(&data, 0) as usize;
    let frames: Vec<&[u8]> = data[8..].chunks_exact(EX + 4 * EXSZ).collect();
    let full = |fr: &[u8]| {
        let mut s = state(fr);
        let n = i(fr, RUN + 0x11c) as usize;
        s.puffs = (0..n.min(32)).map(|k| puff(&fr[PUFFS + 0x80 * k..])).collect();
        s.burst = std::array::from_fn(|p| fr[DASH + 0x1dd + p] != 0);
        s
    };
    let dash = |fr: &[u8]| -> [Option<[[f32; 4]; 4]>; 4] { std::array::from_fn(|p| (fr[DASH + 0x50 + 0x50 * p] != 0).then(|| m4(fr, DASH + 0x10 + 0x50 * p))) };
    let mut feet = full(frames[0]);
    feet.dash = dash(frames[0]);
    let mut skipped = 0;
    for k in 1..frames.len() {
        let (fr, h) = (frames[k], &frames[k][RUN..]);
        // a frame the recorder missed, or a point's end or the next one's start: start over from the recording
        let gap = i(fr, 0) != i(frames[k - 1], 0) + 1;
        if gap || feet.after_point != (h[0xc0] != 0) {
            feet = full(fr);
            feet.dash = dash(fr);
            continue;
        }
        feet.after_point = h[0xc0] != 0;
        let runners: Vec<Runner> = (0..4)
            .map(|p| {
                let (b, x) = (&fr[PL + PLSZ * p..], &fr[EX + EXSZ * p..]);
                Runner {
                    character: i(h, 0x7c + 4 * p) as usize,
                    motion: i(b, 0x50 + 0x20),
                    sub: i(b, 0x48),
                    toes: [v4(b, 0xf0 + 0x30), v4(b, 0x130 + 0x30)],
                    m: m4(b, 0),
                    slide: b[0x41] != 0,
                    pelvis: m4(x, 0x20),
                    spine: m4(x, 0x60),
                    head: v4(x, 0xa0 + 0x30),
                    // the manager's hit event: flag at +0x758 + p, branch (3 a dive) at +0x761 + 8p
                    dive: fr[4 + 0x18 + p] != 0 && fr[4 + 0x21 + 8 * p] == 3,
                    lunge: f(x, 0),
                    dive_over: x[0x10 + 9] != 0,
                    ..Runner::default()
                }
            })
            .collect();
        let want = full(fr);
        let fits = i(h, 0x11c) <= 32;
        let key = |a: &Feet| (a.burst, if fits { a.puffs.iter().map(any_key).collect() } else { vec![] }, a.dash.map(|d| d.map(|m| bits(&m.concat()))));
        let mut wkey = key(&want);
        wkey.2 = dash(fr).map(|d| d.map(|m| bits(&m.concat())));
        let diving = runners.iter().any(|r| r.dive);
        // the dive rings draw from the shared generator somewhere in the frame's draws: find where
        let mut found = None;
        for skip in 0..if diving { 400 } else { 1 } {
            let mut g = mt(frames[k - 1]);
            (0..skip).for_each(|_| {
                g.next();
            });
            let mut one = feet.clone();
            one.tick(&t, court, fr[FLAGS] != 0, fr[FLAGS + 1] != 0, v4(fr, WIND), &runners, h[0x120] != 0, &mut || g.next());
            if key(&one) == wkey {
                found = Some(one);
                break;
            }
        }
        if found.is_none() && !diving {
            // or catches up with two
            let mut two = feet.clone();
            for _ in 0..2 {
                two.tick(&t, court, fr[FLAGS] != 0, fr[FLAGS + 1] != 0, v4(fr, WIND), &runners, h[0x120] != 0, &mut || unreachable!());
            }
            found = (key(&two) == wkey).then_some(two);
        }
        let Some(one) = found else {
            // the game sometimes skips the update a frame (see `footsteps_s05`)
            if key(&feet) != wkey {
                let mut one = feet.clone();
                let mut g = mt(frames[k - 1]);
                one.tick(&t, court, fr[FLAGS] != 0, fr[FLAGS + 1] != 0, v4(fr, WIND), &runners, h[0x120] != 0, &mut || g.next());
                let got = key(&one);
                eprintln!("burst {:?} vs {:?}\ndash {:?}\n vs {:?}", got.0, wkey.0, got.2, wkey.2);
                let fl = |v: &Vec<u32>| v.iter().map(|&b| f32::from_bits(b)).collect::<Vec<_>>();
                got.1.iter().for_each(|a| eprintln!("got  {} {:?} {:?} {:?}", a.0, fl(&a.1), a.2, a.3));
                key(&feet).1.iter().for_each(|a| eprintln!("old  {} {:?} {:?} {:?}", a.0, fl(&a.1), a.2, a.3));
                wkey.1.iter().for_each(|a| eprintln!("want {} {:?} {:?} {:?}", a.0, fl(&a.1), a.2, a.3));
                panic!("frame {k}: no match ({} vs {} puffs)", got.1.len(), wkey.1.len());
            }
            skipped += 1;
            continue;
        };
        rings += diving as usize;
        dashes += one.dash_start.iter().filter(|&&s| s).count();
        // a ring puff's timer starts at fade_in (3 on both rows) and counts down
        kinds[1] += one.puffs.iter().filter(|u| u.kind == 1 && u.phase == 0 && u.timer == 3).count();
        kinds[2] += 4 * (0..4).filter(|&p| one.burst[p] && !feet.burst[p]).count();
        feet = one;
    }
    eprintln!("{name}: {} frames ({skipped} without an update)", frames.len());
    }
    eprintln!("puffs born by kind {kinds:?}, {rings} dive frames, {dashes} streaks");
    assert!(kinds[1] >= 10 && kinds[2] >= 4 && dashes > 0);
}

// `foot_s05g.bin` / `foot_s05c.bin` (`FOOT_DEBRIS=grass|dirt FOOT_FROM=11450 record_foot.py 5 120 … extras`): an
// instant replay forced on every point (the third has a dive), the dirt one with the court made clay. Each frame as
// `foot_s05x`, then the debris slots 0x2d00, the C `rand()` state 8 and per player the motion's start matrix 0x40.
const DB: usize = EX + 4 * EXSZ;
const ANCHOR: usize = DB + 0x2d00 + 8;

fn debris(fr: &[u8], feet: &mut Feet) {
    feet.bits = std::array::from_fn(|g| {
        std::array::from_fn(|s| {
            let o = DB + 0x90 * (20 * g + s);
            Bit { alive: fr[o] != 0, m: m4(fr, o + 0x10), vel: v4(fr, o + 0x50), size: f(fr, o + 0x60), cell: [i(fr, o + 0x64), i(fr, o + 0x68)], spin: v4(fr, o + 0x70), rgb: [f(fr, o + 0x80), f(fr, o + 0x84), f(fr, o + 0x88)] }
        })
    });
    feet.groups = i(fr, DASH) as usize;
    feet.kinds = std::array::from_fn(|k| i(fr, DASH + 4 + 4 * k));
    feet.crand = hst_sim::weather::Rand(u64::from_le_bytes(fr[DB + 0x2d00..DB + 0x2d08].try_into().unwrap()));
}

/// The live fields of every bit alive in `want` (by its group's kind), and every alive flag.
fn debris_key(a: &Feet, want: &Feet) -> (usize, [i32; 3], Vec<Vec<u32>>) {
    let mut v = vec![];
    for g in 0..3 {
        for (s, w) in a.bits[g].iter().zip(&want.bits[g]) {
            v.push(vec![s.alive as u32]);
            if w.alive {
                v.push(bits(&[s.m[3].as_slice(), &s.vel, &[s.size, f32::from_bits(s.cell[0] as u32), f32::from_bits(s.cell[1] as u32)]].concat()));
                if want.kinds[g] == 1 {
                    v.push(bits(&[s.m[..3].concat().as_slice(), &s.spin, &s.rgb].concat()));
                }
            }
        }
    }
    (a.groups, [a.kinds[0], a.kinds[1], a.kinds[2]], v)
}

#[test]
fn foot_debris_s05() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(cnf), Ok(bin)) = (std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")), std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN"))) else {
        return eprintln!("disc absent, skipped");
    };
    let t = exe::Game::new(&cnf, &bin).unwrap().foot();
    let mut thrown = [0; 2];
    for (name, kind) in [("g", 1), ("c", 0)] {
        let Ok(data) = std::fs::read(format!("{root}/context/fixtures/foot_s05{name}.bin")) else {
            eprintln!("foot_s05{name}.bin absent, skipped");
            continue;
        };
        let mut t = t.clone();
        let court = i(&data, 0) as usize;
        // the dirt recording turns the court's grass off and clay on
        (t.courts[court].grass, t.courts[court].clay) = (kind == 1, kind == 0);
        let frames: Vec<&[u8]> = data[8..].chunks_exact(ANCHOR + 0x100).collect();
        let (mut alive, mut skipped) = (0, 0);
        for k in 1..frames.len() {
            let (fr, prev, h) = (frames[k], frames[k - 1], &frames[k][RUN..]);
            assert_eq!(i(fr, 0), i(prev, 0) + 1, "frame {k}: gap");
            let mut feet = state(prev);
            feet.dash = std::array::from_fn(|p| (prev[DASH + 0x50 + 0x50 * p] != 0).then(|| m4(prev, DASH + 0x10 + 0x50 * p)));
            feet.burst = std::array::from_fn(|p| prev[DASH + 0x1dd + p] != 0);
            feet.replay = true;
            debris(prev, &mut feet);
            let mut want = state(fr);
            debris(fr, &mut want);
            let runners: Vec<Runner> = (0..4)
                .map(|p| {
                    let (b, x) = (&fr[PL + PLSZ * p..], &fr[EX + EXSZ * p..]);
                    Runner {
                        character: i(h, 0x7c + 4 * p) as usize,
                        motion: i(b, 0x50 + 0x20),
                        sub: i(b, 0x48),
                        toes: [v4(b, 0xf0 + 0x30), v4(b, 0x130 + 0x30)],
                        m: m4(b, 0),
                        slide: b[0x41] != 0,
                        pelvis: m4(x, 0x20),
                        spine: m4(x, 0x60),
                        head: v4(x, 0xa0 + 0x30),
                        dive: fr[4 + 0x18 + p] != 0 && fr[4 + 0x21 + 8 * p] == 3,
                        lunge: f(x, 0),
                        dive_over: x[0x10 + 9] != 0,
                        anchor: v4(fr, ANCHOR + 0x40 * p + 0x30),
                    }
                })
                .collect();
            let wkey = debris_key(&want, &want);
            let diving = runners.iter().any(|r| r.dive);
            // other `rand()` callers draw first in the frame: a blade group's 60 draws end at the recorded state
            let mut r = feet.crand;
            let n = (0..5000usize).find(|_| r.0 == want.crand.0 || r.next() == u32::MAX).unwrap_or(0);
            for _ in 0..n.saturating_sub(60) {
                feet.crand.next();
            }
            let tick = |skip: usize| {
                let mut g = mt(prev);
                (0..skip).for_each(|_| {
                    g.next();
                });
                let mut one = feet.clone();
                one.tick(&t, court, fr[FLAGS] != 0, fr[FLAGS + 1] != 0, v4(fr, WIND), &runners, h[0x120] != 0, &mut || g.next());
                one
            };
            if let Some(one) = (0..if diving { 400 } else { 1 }).map(tick).find(|one| debris_key(one, &want) == wkey) {
                thrown[kind] += diving as usize;
                alive += one.bits.iter().flatten().filter(|s| s.alive).count();
                continue;
            }
            // the game sometimes skips the update a frame
            if debris_key(&feet, &want) == wkey {
                skipped += 1;
                continue;
            }
            let (got, w) = (debris_key(&tick(0), &want), wkey);
            eprintln!("groups {} {:?} vs {} {:?}", got.0, got.1, w.0, w.1);
            for (n, (a, b)) in got.2.iter().zip(&w.2).enumerate().filter(|(_, (a, b))| a != b).take(6) {
                let fl = |v: &Vec<u32>| v.iter().map(|&b| f32::from_bits(b)).collect::<Vec<_>>();
                eprintln!("{n}: got  {:?}\n{n}: want {:?}", fl(a), fl(b));
            }
            panic!("{name} frame {k} (vsync {}): debris differs", i(fr, 0));
        }
        eprintln!("{name}: {} frames ({skipped} without an update), {alive} bit-frames", frames.len());
        assert!(alive > 100, "{name}: no debris");
    }
    eprintln!("dives throwing [clods, blades] {thrown:?}");
}

/// The app's dive inputs (`DiveWatch` over each player's dive: from the search frame for its length, with its slide
/// and cut-short flag) against the run object's: the hit event's dive branch, +0x3f80 and +0x3ec9, frame for frame.
#[test]
fn dive_inputs_s05() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut dives = 0;
    for name in ["x", "w", "d"] {
        let Ok(data) = std::fs::read(format!("{root}/context/fixtures/foot_s05{name}.bin")) else {
            eprintln!("foot_s05{name}.bin absent, skipped");
            continue;
        };
        let frames: Vec<&[u8]> = data[8..].chunks_exact(EX + 4 * EXSZ).collect();
        for p in 0..4 {
            let (mut watch, mut until, mut seen) = (DiveWatch::default(), None, false);
            for fr in &frames {
                let (vsync, x) = (i(fr, 0), &fr[EX + EXSZ * p..]);
                let event = fr[4 + 0x18 + p] != 0 && fr[4 + 0x21 + 8 * p] == 3;
                // a dive lasts its search frame and its counter's run to +0x3f84 (the app's `Dive::len`)
                if event {
                    until = Some(vsync + i(x, 4));
                    seen = true;
                    dives += 1;
                }
                let live = until.is_some_and(|u| vsync <= u);
                let (start, lunge, over) = watch.see(live.then(|| (f(x, 0), x[0x19] != 0)));
                // before the player's first dive its flag holds whatever was there
                if seen {
                    assert_eq!((start, lunge.to_bits(), over), (event, f(x, 0).to_bits(), x[0x10 + 9] != 0), "{name} p{p} vsync {vsync}");
                }
            }
        }
    }
    eprintln!("{dives} dives");
}
