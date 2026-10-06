//! Every stroke decision of the slot-5 doubles match (`context/fixtures/match_s05.bin`) through the ported
//! contact search: same frame, branch, swing side and contact ball as the game. The players' animation-measured
//! reach values come from the slot-5 save state's RAM (`context/fixtures/slot5_ee.bin`).

use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};
use hst_sim::replay::{Frame, frames_live};
use hst_sim::swing::{Branch, PathPoint, Reach, search};

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v3(b: &[u8], o: usize) -> [f32; 3] {
    [f(b, o), f(b, o + 4), f(b, o + 8)]
}
fn rows(b: &[u8], o: usize) -> [[f32; 4]; 4] {
    std::array::from_fn(|r| std::array::from_fn(|k| f(b, o + 16 * r + 4 * k)))
}

/// A ball object (0x290 bytes) as a flight and its shot (as in tests/live.rs).
fn load(b: &[u8]) -> (Flight, Shot) {
    let ball = Ball { pos: v3(b, 0xe0), vel: v3(b, 0x130), spin: f(b, 0x1a4) };
    let mut fl = Flight::new(ball, rows(b, 0x160), rows(b, 0x1c0));
    fl.frame = i(b, 0xac);
    fl.bounces = i(b, 0x224);
    fl.contacts = i(b, 0x228);
    fl.special_contacts = i(b, 0x22c);
    fl.rolling = b[0xa4] == 2;
    fl.slide = f(b, 0x200);
    let shot = Shot {
        params: Params::default(),
        curve: f(b, 0x254),
        bend: f(b, 0x250),
        side: v3(b, 0x90),
        curve_frames: i(b, 0x260),
        wind: v3(b, 0x240),
        first_bounce_spin: f(b, 0x1a8),
        first_bounce_restitution: f(b, 0x1ac),
        class: b[0x58],
        kind: i(b, 0x5c),
        bounce_turn: f(b, 0x1b0),
    };
    (fl, shot)
}

fn p_u8(fr: Frame, p: usize, off: usize) -> u8 {
    fr.player_f32(p, off & !3).to_bits().to_le_bytes()[off & 3]
}
fn p_i32(fr: Frame, p: usize, off: usize) -> i32 {
    fr.player_f32(p, off).to_bits() as i32
}

#[test]
fn match_s05_contact_search() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(ram)) = (std::fs::read(format!("{dir}/match_s05.bin")), std::fs::read(format!("{dir}/slot5_ee.bin"))) else {
        return eprintln!("match_s05.bin or slot5_ee.bin absent, skipped");
    };
    let ru = |a: usize| u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize;
    let gm = ru(0x422f80);
    let frames = frames_live(&data);
    let (mut checked, mut misses) = (0, 0);
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        for p in 0..4 {
            let idx = p_i32(fr, p, 0x3ec4);
            let branch = p_u8(fr, p, 0x3ec1);
            if idx < 0 || !(branch == 1 || branch == 2 || branch == 4) || p_i32(a, p, 0x3ec4) >= 0 {
                continue;
            }
            let obj = ru(gm + 0xa8 + 4 * p);
            let rf = |o: usize| f32::from_le_bytes(ram[obj + o..obj + o + 4].try_into().unwrap());
            let look = p_i32(fr, p, 0x154c) as usize;
            let reach = Reach {
                base: fr.player_f32(p, 0x13ac),
                reach: fr.player_f32(p, 0x13b0),
                stroke_height: fr.player_f32(p, 0x13b8),
                volley_height: fr.player_f32(p, 0x13bc),
                smash_top: fr.player_f32(p, 0x13c0),
                smash_bottom: fr.player_f32(p, 0x13c8),
                ahead: rf(0x3050),
                smash_ahead: rf(0x38f8),
                body_low: rf(0x3054),
                body_high: rf(0x3058),
                grades: (0..look).map(|j| p_u8(fr, p, 0x1510 + j)).collect(),
                hand: rf(0x12b4),
            };
            let pos = a.player_pos(p);
            let facing = if pos[2] < 0.0 { 1.0 } else { -1.0 };
            let (mut fl, shot) = load(fr.live_ball());
            let mut path = vec![];
            for _ in 0..look + 1 {
                path.push(PathPoint { pos: fl.ball.pos, bounces: fl.bounces });
                fl.step(&shot, &COURTS[10]);
            }
            let got = search(&reach, &path, pos, facing, 0);
            let want_ball = [fr.player_f32(p, 0x3f40), fr.player_f32(p, 0x3f44), fr.player_f32(p, 0x3f48)];
            let want = (idx, branch, p_i32(fr, p, 0x3f50));
            // flags: 1 forehand / 2 backhand, | 4 body shot (smash: none)
            let g = got.map(|s| {
                let flags = if s.branch == Branch::Smash { 3 } else { (if s.forehand { 1 } else { 2 }) | (if s.body { 4 } else { 0 }) };
                (s.frame as i32, match s.branch { Branch::Ground => 1u8, Branch::Volley => 2, Branch::Smash => 4 }, flags)
            });
            let ok = g == Some(want) && got.is_some_and(|s| (s.ball[0], s.ball[2]) == (want_ball[0], want_ball[2]) || ((s.ball[0] - want_ball[0]).abs() < 1e-5 && (s.ball[2] - want_ball[2]).abs() < 1e-5));
            if !ok {
                eprintln!("vsync {} p{p}: want {want:?} ball {want_ball:?} got {g:?} {:?}", fr.vsync(), got.map(|s| s.ball));
            }
            checked += 1;
            misses += !ok as i32;
        }
    }
    eprintln!("{checked} decisions, {misses} misses");
    assert_eq!(misses, 0);
}
