//! The motion numbers of strokes, serves and post-point reactions through the slot-5 doubles match
//! (`context/fixtures/match_s05.bin`), from the recorded contact-search results and point outcomes.

use hst_sim::motion::{reaction, serve_walk, soft_follow, stroke_start, team_reactions, whiff, SWING_LEAD};
use hst_sim::replay::{Frame, frames_live};

fn p_u8(fr: Frame, p: usize, off: usize) -> u8 {
    fr.player_f32(p, off & !3).to_bits().to_le_bytes()[off & 3]
}
fn p_i32(fr: Frame, p: usize, off: usize) -> i32 {
    fr.player_f32(p, off).to_bits() as i32
}
fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
const CHARS: [i32; 4] = [0, 2, 1, 5];

fn load() -> Option<Vec<u8>> {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    std::fs::read(format!("{dir}/match_s05.bin")).ok()
}

#[test]
fn match_s05_stroke_motions() {
    let Some(data) = load() else { return eprintln!("match_s05.bin absent, skipped") };
    let frames = frames_live(&data);
    let (mut starts, mut switches, mut whiffs, mut softs) = (0, 0, 0, 0);
    for k in 1..frames.len() - 1 {
        let (a, fr, next) = (frames[k - 1], frames[k], frames[k + 1]);
        if f(fr.gm(), 0x58).to_bits() as i32 - f(a.gm(), 0x58).to_bits() as i32 > 2 {
            break;
        }
        for p in 0..4 {
            if p_u8(fr, p, 0x3fa4) != 0 {
                continue;
            }
            let (was, now) = (p_u8(a, p, 0x3fa5), p_u8(fr, p, 0x3fa5));
            let (m0, m1) = (p_i32(a, p, 0x3df0), p_i32(fr, p, 0x3df0));
            let (branch, left) = (p_u8(fr, p, 0x3ec1), p_i32(fr, p, 0x3ec4));
            if now == 2 && was != 2 {
                // the swing is +0x3e40 while it waits, +0x3e44 once playing
                let pending = p_i32(fr, p, 0x3e40);
                let anim = if pending >= 0 { pending } else { p_i32(fr, p, 0x3e44) };
                let (m, _, wait) = stroke_start(branch, left, anim);
                assert_eq!((m, wait.is_some()), (m1, pending >= 0), "start k={k} p={p}");
                starts += 1;
            } else if now == 2 && was == 2 && m1 != m0 {
                if left == SWING_LEAD && p_i32(a, p, 0x3e40) >= 0 {
                    assert_eq!(m1, p_i32(a, p, 0x3e40), "switch k={k} p={p}");
                    switches += 1;
                } else if (0x27..=0x2a).contains(&m1) {
                    assert_eq!(whiff(m0), Some(m1), "whiff k={k} p={p}");
                    whiffs += 1;
                }
            }
            // the contact: countdown 1 → −1 and this player the last hitter
            if p_i32(a, p, 0x3ec4) == 1 && left == -1 && fr.global(0x423058) == p as i32 {
                let b = fr.live_ball();
                let v = [f(b, 0x140), f(b, 0x144), f(b, 0x148)];
                let soft = soft_follow(branch, p_i32(fr, p, 0x3e44), v, p_i32(fr, p, 0x3f50) as u32, 1.0);
                let m = p_i32(next, p, 0x3df0);
                match soft {
                    Some(s) => assert_eq!(m, s, "soft k={k} p={p}"),
                    None => assert!(!(0x1c..=0x1d).contains(&m), "no soft k={k} p={p}"),
                }
                softs += 1;
            }
        }
    }
    eprintln!("{starts} starts, {switches} switches, {whiffs} whiffs, {softs} contacts");
    assert!(starts > 100 && switches > 20 && whiffs > 3 && softs > 100);
}

#[test]
fn match_s05_serve_walk() {
    let Some(data) = load() else { return eprintln!("match_s05.bin absent, skipped") };
    let frames = frames_live(&data);
    let mut n = 0;
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        for p in 0..4 {
            let m = p_i32(fr, p, 0x3df0);
            if p_u8(fr, p, 0x3fa4) != 1 || !(0x21..=0x22).contains(&m) {
                continue;
            }
            let dx = fr.player_pos(p)[0] - a.player_pos(p)[0];
            if dx == 0.0 {
                continue;
            }
            let fwd = if fr.player_pos(p)[2] < 0.0 { 1.0 } else { -1.0 };
            assert_eq!(serve_walk(dx, fwd, 1.0), m, "k={k} p={p}");
            n += 1;
        }
    }
    assert!(n > 1000, "{n}");
}

/// Every post-point reaction: the outcome's base reaction or a team reaction of the character's set not taken
/// by a player updated before it (the draw itself is the game's random number).
#[test]
fn match_s05_reactions() {
    let Some(data) = load() else { return eprintln!("match_s05.bin absent, skipped") };
    let frames = frames_live(&data);
    let (mut n, mut team) = (0, 0);
    let mut taken: Vec<i32> = vec![];
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        if f(fr.gm(), 0x58).to_bits() as i32 - f(a.gm(), 0x58).to_bits() as i32 > 2 {
            break;
        }
        for p in 0..4 {
            if p_u8(fr, p, 0x3fa4) != 2 || p_u8(a, p, 0x3fa4) == 2 {
                continue;
            }
            if p == 0 || (0..p).all(|q| p_u8(a, q, 0x3fa4) == 2 || p_u8(fr, q, 0x3fa4) != 2) {
                taken.clear();
            }
            let won = (p as i32 & 1) == fr.global(0x4230a8);
            let base = reaction(p_u8(a, p, 0x3fa5) == 3, 4, won, fr.global(0x4230b8) != 0, false);
            let got = p_i32(fr, p, 0x3db0);
            if got >= 0x30 {
                let c = got - 0x30;
                assert!((0x2c..=0x2d).contains(&base) && team_reactions(CHARS[p]).contains(&c) && !taken.contains(&c), "k={k} p={p} {got:#x}");
                taken.push(c);
                team += 1;
            } else {
                assert_eq!(got, base, "k={k} p={p}");
            }
            n += 1;
        }
    }
    eprintln!("{n} reactions ({team} team)");
    assert!(n > 100 && team > 30);
}
