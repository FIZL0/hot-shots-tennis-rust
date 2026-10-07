//! Every stroke decision of the slot-5 doubles match (`context/fixtures/match_s05.bin`) through the ported
//! contact search: same frame, branch, swing side and contact ball as the game. The players' animation-measured
//! reach values come from the slot-5 save state's RAM (`context/fixtures/slot5_ee.bin`).

use hst_data::{ani, iso::Iso, xb::Archive};
use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};
use hst_sim::replay::{Frame, frames_live};
use hst_sim::player::mover;
use hst_sim::swing::{Branch, DIVE_HORIZON, PathPoint, Reach, dive, search};

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
                shoulder: [0.0; 3],
                tip: [0.0; 3],
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

/// Every dive of `context/fixtures/new_recording.bin` (all lunges that miss the ball): the dive search picks the
/// game's frame, direction and slide.
#[test]
fn new_recording_dives() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/new_recording.bin")) else {
        return eprintln!("new_recording.bin absent, skipped");
    };
    let frames = frames_live(&data);
    // the receive motion's root path by player: the recording's characters (not recorded, found by their
    // paths) are p1 0 (or 1, 2, 4, 6, 8, 9, 12, 13: same path), p2 10, p3 11
    let roots = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")).ok().map(|mut iso| {
        [0, 0, 10, 11].map(|c| {
            let xb = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
            let arc = Archive::parse(&xb).unwrap();
            let stem = ani::motion_name(51, c).unwrap().to_ascii_lowercase();
            let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).unwrap();
            hst_sim::pose::Path::new(&ani::parse(&arc.read(e).unwrap()).unwrap()).unwrap()
        })
    });
    let (mut dives, mut steps) = (0, 0);
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        for p in 0..4 {
            if p_u8(fr, p, 0x3f58) != 1 || p_u8(a, p, 0x3f58) != 0 {
                continue;
            }
            let reach = Reach {
                base: fr.player_f32(p, 0x13ac),
                reach: fr.player_f32(p, 0x13b0),
                stroke_height: fr.player_f32(p, 0x13b8),
                volley_height: fr.player_f32(p, 0x13bc),
                smash_top: fr.player_f32(p, 0x13c0),
                smash_bottom: fr.player_f32(p, 0x13c8),
                ahead: 0.0,
                smash_ahead: 0.0,
                body_low: 0.0,
                body_high: 0.0,
                grades: vec![],
                hand: 1.0,
                shoulder: [0.0; 3],
                tip: [0.0; 3],
            };
            let pos = [fr.player_f32(p, 0x3f60), fr.player_f32(p, 0x3f64), fr.player_f32(p, 0x3f68)];
            let end = if pos[2] < 0.0 { 1.0 } else { -1.0 };
            // the recorded flight as the path: the game's predictor matches it, while replaying a ball from its
            // object drifts by millimetres after a bounce
            let path: Vec<PathPoint> = frames[k..k + DIVE_HORIZON]
                .iter()
                .map(|f| {
                    let b = f.live_ball();
                    PathPoint { pos: v3(b, 0xe0), bounces: i(b, 0x224) }
                })
                .collect();
            let face = [a.player_f32(p, 0x3d60), a.player_f32(p, 0x3d68)];
            let vel = [fr.player_f32(p, 0x3e00), fr.player_f32(p, 0x3e08)];
            let d = dive(&reach, &path, pos, end, face, vel).unwrap_or_else(|| panic!("vsync {} p{p}: no dive", fr.vsync()));
            let want = (p_i32(fr, p, 0x3f84) - 50, p_i32(fr, p, 0x3ec4) >= 0, fr.player_f32(p, 0x3f80));
            assert_eq!((d.frame as i32, d.contact, d.slide), want, "vsync {} p{p}", fr.vsync());
            let want_dir = [fr.player_f32(p, 0x3e60), fr.player_f32(p, 0x3e68)];
            assert!((d.dir[0] - want_dir[0]).abs() < 1e-6 && (d.dir[1] - want_dir[1]).abs() < 1e-6, "vsync {} p{p}: dir {:?} want {want_dir:?}", fr.vsync(), d.dir);
            dives += 1;
            let Some(root) = roots.as_ref().map(|r| &r[p]) else { continue };
            // from the game's direction (the path's ulps aside)
            let (mut d, mut at) = (d, [pos[0], pos[2]]);
            d.dir = want_dir;
            // the recorded dive counter (+0x3f88) is n + 1 after frame n; it stalls when the game pauses
            let mut n = 0;
            while p_i32(frames[k + n], p, 0x3f88) == n as i32 + 1 {
                let q = d.step(at, |t| root.at(t)[2], |m| {
                    // the partner (p ^ 2) as of this player's update: players update in order
                    let mate = if p < 2 { frames[k + n - 1].player_pos(p ^ 2) } else { frames[k + n].player_pos(p ^ 2) };
                    let q = mover([at[0], 0.0, at[1]], [m[0], 0.0, m[1]], end, Some(mate), false);
                    [q[0], q[2]]
                });
                at = q.expect("dive over early");
                let rec = frames[k + n].player_pos(p);
                assert_eq!(at, [rec[0], rec[2]], "vsync {} p{p} frame {n} of the dive", frames[k + n].vsync());
                steps += 1;
                n += 1;
            }
            assert!(n == d.len() || n > 40, "vsync {} p{p}: {n} frames", fr.vsync());
        }
    }
    eprintln!("{dives} dives, {steps} dive frames");
    assert!(dives == 8 || frames.is_empty());
}
