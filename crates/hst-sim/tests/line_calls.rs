//! Line calls against the user's round1 recording (fixture `context/fixtures/round1.bin` from
//! `tools/record_p2m2.py`, not in git; skipped when absent). The recorded ball object (`gm+0x98`) is the game's
//! path predictor, which steps 15 frames per game frame and makes the same line calls as the live ball. Every
//! recorded frame before its landing is called is replayed from the previous frame's object through `Flight`
//! with line calls on: the call (+0xa5) and line distance (+0x230) must match bit for bit, and so must the
//! position and velocity. Needs the extracted disc under `context/iso` for the line margin.

use hst_data::exe::Game;
use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};
use hst_sim::judge::{Call, Lines};
use hst_sim::replay::frames;

const STEPS_PER_FRAME: i32 = 15;

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
    std::array::from_fn(|r| std::array::from_fn(|c| f(b, o + 16 * r + 4 * c)))
}
fn call(c: u8) -> Call {
    [Call::None, Call::In, Call::Out, Call::Net, Call::NetIn, Call::NetOut][c as usize]
}

/// The predictor's flight and shot as recorded in its ball object.
fn flight(b: &[u8]) -> (Flight, Shot) {
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
    let mut fl = Flight::new(Ball { pos: v3(b, 0xe0), vel: v3(b, 0x130), spin: f(b, 0x1a4) }, rows(b, 0x160), rows(b, 0x1c0));
    fl.frame = i(b, 0xac);
    fl.bounces = i(b, 0x224);
    fl.contacts = i(b, 0x228);
    fl.special_contacts = i(b, 0x22c);
    fl.rolling = b[0xa4] == 2;
    fl.slide = f(b, 0x200);
    fl.net = false;
    fl.call = call(b[0xa5]);
    fl.line_distance = f(b, 0x230);
    (fl, shot)
}

#[test]
fn round1_line_calls() {
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let (Ok(cnf), Ok(bin), Ok(data)) = (
        std::fs::read(format!("{ctx}/iso/SYSTEM.CNF")),
        std::fs::read(format!("{ctx}/iso/ZZBIN/GAME.BIN")),
        std::fs::read(format!("{ctx}/fixtures/round1.bin")),
    ) else {
        return eprintln!("disc or round1.bin absent, skipped");
    };
    let margin = Game::new(&cnf, &bin).unwrap().line_margin();
    let frames = frames(&data);
    // (vsync, previous ball, current ball, lines, shots this rally)
    let mut cases = Vec::new();
    for w in frames.windows(2) {
        let (a, b) = (w[0].ball(), w[1].ball());
        let open = matches!(call(a[0xa5]), Call::None | Call::Net);
        if !open || i(b, 0xac) != i(a, 0xac) + STEPS_PER_FRAME || i(a, 0xac) == 0 {
            continue;
        }
        let lines = Lines {
            shots: w[0].global(0x423060),
            doubles: w[0].global(0x422fa4) > 2,
            side: w[0].global(0x423050),
            hitter_far: f(a, 0x138) < 0.0,
            margin,
        };
        // No calls before the rally phase (match phase gm+0x55 < 3, sub-phase gm+0x56 != 4).
        let gm = w[0].gm();
        let lines = (!(gm[0x55] < 3 && gm[0x56] != 4)).then_some(lines);
        cases.push((w[1].vsync(), a, b, lines, w[0].global(0x423060)));
    }
    assert!(!cases.is_empty(), "no predictor frames in round1.bin");
    // The court surface only shapes the flight after a bounce; take the one that reproduces the most frames.
    // Returns (call mismatches, flight mismatches).
    let run = |court: usize| -> (Vec<String>, Vec<String>) {
        let (mut calls, mut flights) = (Vec::new(), Vec::new());
        for &(v, a, b, lines, shots) in &cases {
            let (mut fl, shot) = flight(a);
            fl.lines = lines;
            fl.in_play = shots > 0;
            for _ in 0..STEPS_PER_FRAME {
                fl.step(&shot, &COURTS[court]);
            }
            if (fl.call, fl.line_distance.to_bits()) != (call(b[0xa5]), f(b, 0x230).to_bits()) {
                calls.push(format!("vsync {v}: got {:?} line {}, want {:?} line {} ({lines:?})", fl.call, fl.line_distance, call(b[0xa5]), f(b, 0x230)));
            }
            let want = (v3(b, 0xe0).map(f32::to_bits), v3(b, 0x130).map(f32::to_bits));
            if (fl.ball.pos.map(f32::to_bits), fl.ball.vel.map(f32::to_bits)) != want {
                flights.push(format!("vsync {v}: pos {:?} vel {:?}, want {:?} {:?}", fl.ball.pos, fl.ball.vel, v3(b, 0xe0), v3(b, 0x130)));
            }
        }
        (calls, flights)
    };
    let (court, (calls, flights)) = (0..COURTS.len()).map(|c| (c, run(c))).min_by_key(|(_, (c, f))| c.len() + f.len()).unwrap();
    let landed = cases.iter().filter(|(_, a, b, _, _)| a[0xa5] != b[0xa5]).count();
    eprintln!("round1: {} predictor frames ({landed} calls) on court {court}: {} call and {} flight mismatches", cases.len(), calls.len(), flights.len());
    assert!(calls.is_empty(), "{}", calls[..calls.len().min(10)].join("\n"));
    assert!(flights.is_empty(), "{}", flights[..flights.len().min(10)].join("\n"));
    assert!(landed > 0);
}
