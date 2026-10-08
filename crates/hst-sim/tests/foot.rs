//! Footstep puffs and footprints against a recorded bot doubles match on court 10 (`context/fixtures/foot_s05.bin`,
//! `tools/record_foot.py 5`): from each player's toe bones, motion and matrix, every frame's step state, dust puffs
//! (the first 32) and footprints match the game bit for bit.

use hst_data::exe;
use hst_sim::foot::{Feet, Print, Puff, Runner};

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
    }
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
                }
            })
            .collect();
        let before = feet.prints.len();
        let want = state(fr);
        let full = i(h, 0x11c) <= 32;
        let same = |a: &Feet| {
            (a.armed, a.cooldown) == (want.armed, want.cooldown)
                && a.prints.iter().map(print_key).eq(want.prints.iter().map(print_key))
                && (!full || a.puffs.iter().map(puff_key).eq(want.puffs.iter().map(puff_key)))
        };
        // the game sometimes skips the update a frame and catches up with two the next
        let tick = |a: &mut Feet| a.tick(&t, court, fr[FLAGS] != 0, fr[FLAGS + 1] != 0, v4(fr, WIND), &runners, h[0x120] != 0);
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
            assert_eq!(feet.puffs.iter().map(puff_key).collect::<Vec<_>>(), want.puffs.iter().map(puff_key).collect::<Vec<_>>(), "frame {k}: puffs");
            checked += 1;
        }
        steps += feet.puffs.iter().filter(|u| u.timer == t.puffs[0].fade_in && u.phase == 0).count();
        prints += (feet.prints.len() > before) as usize;
    }
    eprintln!("{} frames ({skipped} without an update, {doubled} with two, {resets} points), {checked} puff frames, {steps} puffs, {prints} footprint frames", frames.len());
    assert!(steps > 100 && prints > 100);
}
