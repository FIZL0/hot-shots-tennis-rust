//! Replays whole shots recorded from a bot match (fixture `context/fixtures/flights_s05.csv`, not in git;
//! made by `research/tools/fixture_flights.py`) from frame 3 through every bounce to the end of the
//! game's own path, requiring every frame's position and velocity to match bit for bit. Skips when the fixture is absent.

use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot, rows4};

struct Case {
    court: usize,
    start: i32,
    shot: Shot,
    spin: f32,
    spin_frame: [[f32; 3]; 3],
    contact: [[f32; 3]; 3],
    path: Vec<(i32, [f32; 6])>,
}

fn load() -> Option<Vec<Case>> {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let text = std::fs::read_to_string(format!("{dir}/flights_s05.csv")).ok()?;
    let mut cases: Vec<Case> = Vec::new();
    for line in text.lines() {
        let (kind, rest) = line.split_once(',').unwrap();
        let f: Vec<f64> = rest.split(',').map(|x| x.parse().unwrap()).collect();
        let v3 = |i: usize| [f[i] as f32, f[i + 1] as f32, f[i + 2] as f32];
        match kind {
            "S" => cases.push(Case {
                court: f[1] as usize,
                start: f[2] as i32,
                spin: f[3] as f32,
                shot: Shot {
                    params: Params::default(),
                    curve: f[4] as f32,
                    bend: f[5] as f32,
                    curve_frames: f[6] as i32,
                    first_bounce_spin: f[7] as f32,
                    first_bounce_restitution: f[8] as f32,
                    class: f[9] as u8,
                    kind: f[10] as i32,
                    side: v3(11),
                    wind: v3(14),
                    bounce_turn: 0.0,
                },
                spin_frame: [v3(17), v3(20), v3(23)],
                contact: [v3(26), v3(29), v3(32)],
                path: Vec::new(),
            }),
            _ => cases.last_mut().unwrap().path.push((f[1] as i32, [f[2], f[3], f[4], f[5], f[6], f[7]].map(|x| x as f32))),
        }
    }
    Some(cases)
}

#[test]
fn whole_shots_match_the_game() {
    let Some(cases) = load() else {
        eprintln!("fixture missing, skipped");
        return;
    };
    let (mut frames, mut failures) = (0, Vec::new());
    for (n, c) in cases.iter().enumerate() {
        let p0 = c.path[0].1;
        let mut fl = Flight::new(Ball { pos: [p0[0], p0[1], p0[2]], vel: [p0[3], p0[4], p0[5]], spin: c.spin }, rows4(c.spin_frame), rows4(c.contact));
        fl.frame = c.start;
        fl.net = false; // the game records paths against the court plane only
        for &(i, want) in &c.path[1..] {
            fl.step(&c.shot, &COURTS[c.court]);
            let got = [fl.ball.pos, fl.ball.vel].concat();
            if (0..6).any(|k| got[k].to_bits() != want[k].to_bits()) {
                failures.push(format!("shot {n}: frame {i} (bounces {}): got {got:?}, want {want:?}", fl.bounces));
                break;
            }
            frames += 1;
        }
    }
    eprintln!("{frames} frames bit-exact across {} shots", cases.len());
    assert!(failures.is_empty(), "{} shots diverged:\n{}", failures.len(), failures.join("\n"));
    assert!(frames > 3000);
}
