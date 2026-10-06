//! Replays the live ball of a recorded bot match (`context/live/net_s05.bin`, not in git; made by
//! `tools/record_live.py 5 …` from save-state slot 5, court 10) one frame at a time: each frame's ball object is
//! loaded into a `Flight`, stepped once against court 10's collision world (built from the disc: the court model
//! and the grid of props) and compared bit for bit with the next recorded frame: airborne frames and every contact
//! (court, ground around it, walls, net, net cord, the ghost material) must match exactly. Skips when the recording
//! or the disc is absent.

use hst_data::exe;
use hst_data::iso::Iso;
use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};
use hst_sim::court;

const SAMPLE: usize = 4 + 0x290 + 0x290 + 0x40;
const COURT: usize = 10;

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

/// The ball object (0x290 bytes from the live ball pointer `*(gm+0x88)`) as a flight and its shot.
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

#[test]
fn live_ball_frames_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Ok(mut iso), Ok(cnf), Ok(bin)) = (
        std::fs::read(format!("{root}/context/live/net_s05.bin")),
        Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")),
        std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")),
        std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")),
    ) else {
        eprintln!("recording or disc missing, skipped");
        return;
    };
    let world = court::world(&mut iso, COURT as u32);
    let materials = court::materials(&exe::Game::new(&cnf, &bin).unwrap());

    let s: Vec<&[u8]> = data.chunks_exact(SAMPLE).collect();
    let (mut exact, mut touches, mut failures) = (0, std::collections::BTreeMap::new(), Vec::new());
    for w in s.windows(2) {
        let (a, b) = (&w[0][4..4 + 0x290], &w[1][4..4 + 0x290]);
        let vsync = i(w[1], 0);
        // only frames the ball physically flies: consecutive vsyncs, same shot, in play (+0xa4 0 or 1)
        // and not carried (the server's toss/bounce moves it with zero velocity)
        let carried = |o: &[u8]| v3(o, 0x130) == [0.0; 3];
        if vsync != i(w[0], 0) + 1 || i(b, 0xac) != i(a, 0xac) + 1 || a[0xa4] > 1 || b[0xa4] > 1 || carried(a) || carried(b) {
            continue;
        }
        let (mut fl, shot) = load(a);
        fl.step_world(&shot, &COURTS[COURT], &world, &materials);
        let want = [v3(b, 0xe0), v3(b, 0x130)].concat();
        let got = [fl.ball.pos, fl.ball.vel].concat();
        let same = (0..6).all(|k| got[k].to_bits() == want[k].to_bits());
        // a contact moves a counter or changes the material of the last contact
        if i(b, 0x224) != i(a, 0x224) || i(b, 0x228) != i(a, 0x228) || i(b, 0x22c) != i(a, 0x22c) || b[0x220] != a[0x220] {
            *touches.entry(b[0x220]).or_insert(0) += 1;
        }
        if same {
            exact += 1;
        } else {
            failures.push(format!("vsync {vsync} material {} pos {:?}: got {got:?} want {want:?}", b[0x220], v3(b, 0xe0)));
        }
    }
    eprintln!("{exact} frames bit-exact; contact frames by material: {touches:?}");
    assert!(failures.is_empty(), "{} frames diverged:\n{}", failures.len(), failures[..failures.len().min(20)].join("\n"));
    assert!(exact > 12000);
}

#[test]
fn every_court_builds_its_world() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    for n in 1..=11 {
        let w = court::world(&mut iso, n);
        assert!(!w.models[0].tris.is_empty(), "court {n} has no collision triangles");
    }
}
