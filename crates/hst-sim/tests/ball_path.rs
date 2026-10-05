//! Replays flight segments of a path recorded from the real game (fixture CSV, not in git:
//! `context/fixtures/ball_path_s08.csv`, made by `context/tools/fixture_ball_path.py`). Skips when absent.

use hst_sim::ball::{Ball, Params};

fn load() -> Option<Vec<[f32; 6]>> {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let text = std::fs::read_to_string(format!("{dir}/ball_path_s08.csv")).ok()?;
    Some(
        text.lines()
            .map(|l| {
                let f: Vec<f32> = l.split(',').skip(1).map(|x| x.parse::<f64>().unwrap() as f32).collect();
                [f[0], f[1], f[2], f[3], f[4], f[5]]
            })
            .collect(),
    )
}

#[test]
fn flight_matches_recorded_path() {
    let Some(path) = load() else {
        eprintln!("fixture missing, skipped");
        return;
    };
    // Segments between bounces and the spin the game held during each (solved from the recording).
    for (from, to, spin) in [(0, 65, -1.7235f32), (66, 95, 1.3388), (96, 104, 0.9723)] {
        let p = Params::default();
        let e = path[from];
        let mut b = Ball { pos: [e[0], e[1], e[2]], vel: [e[3], e[4], e[5]], spin };
        for (i, want) in path.iter().enumerate().take(to + 1).skip(from + 1) {
            b.fly(&p);
            for k in 0..3 {
                let (dp, dv) = ((b.pos[k] - want[k]).abs(), (b.vel[k] - want[k + 3]).abs());
                assert!(dp < 2e-4 && dv < 2e-5, "frame {i} axis {k}: pos err {dp}, vel err {dv}");
            }
        }
    }
}

#[test]
fn flight_matches_bot_match_shots() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(text) = std::fs::read_to_string(format!("{dir}/shots_s05.csv")) else {
        eprintln!("fixture missing, skipped");
        return;
    };
    let rows: Vec<Vec<f64>> = text.lines().map(|l| l.split(',').map(|x| x.parse().unwrap()).collect()).collect();
    let p = Params::default();
    let mut checked = 0;
    for w in rows.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if a[0] != b[0] {
            continue; // shot boundary
        }
        let mut ball = Ball { pos: [a[3] as f32, a[4] as f32, a[5] as f32], vel: [a[6] as f32, a[7] as f32, a[8] as f32], spin: a[2] as f32 };
        ball.fly(&p);
        for k in 0..3 {
            let dv = (ball.vel[k] - b[6 + k] as f32).abs();
            assert!(dv < 2e-6, "shot {} frame {}: vel[{k}] off by {dv}", b[0], b[1]);
        }
        checked += 1;
    }
    assert!(checked > 1000, "only {checked} frames checked");
}
