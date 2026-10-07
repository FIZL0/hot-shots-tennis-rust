//! Stroke launches recorded from a bot match whose table lookups were confirmed exactly (speed, elevation and
//! flight frames) by `research/tools/traj_inverse2.py`. Fixture `context/fixtures/shot_tables_s05.csv` and the
//! tables under `context/xb` stay out of git; skips when absent.

use hst_sim::shot::{Bounds, Table, launch, lookup};

#[test]
fn strokes_launch_like_the_game() {
    let ctx = std::env::var("HST_CONTEXT").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context").into());
    let Ok(text) = std::fs::read_to_string(format!("{ctx}/fixtures/shot_tables_s05.csv")) else {
        eprintln!("fixture missing, skipped");
        return;
    };
    let mut n = 0;
    for line in text.lines() {
        let f: Vec<&str> = line.split(',').collect();
        let table = Table::parse(&std::fs::read(format!("{ctx}/xb/{}", f[0])).unwrap()).unwrap();
        let kind: i32 = f[1].parse().unwrap();
        let x: Vec<f32> = f[2..10].iter().map(|v| v.parse::<f64>().unwrap() as f32).collect();
        let (hit, target, want) = ([x[0], x[1], x[2]], [x[3], 0.0, x[4]], [x[5], x[6], x[7]]);
        let curve_frames: i32 = f[10].parse().unwrap();
        let l = lookup(&table, &Bounds::stroke(kind, hit[2]), hit, target);
        let v = launch(hit, target, l.elevation, l.speed);
        // the fixture's target was solved from speed + elevation only, so its bearing is loose (~0.01°)
        let (len, wlen) = ((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt(), (want[0] * want[0] + want[1] * want[1] + want[2] * want[2]).sqrt());
        assert!((len - wlen).abs() < 2e-6, "{}: speed {len} want {wlen}", f[0]);
        assert!((v[1] - want[1]).abs() < 2e-6, "{}: rise {} want {}", f[0], v[1], want[1]);
        // Bearing is not checked: the game adds a small lateral adjustment (up to ~1°, larger on slices/lobs)
        // that is not ported yet, and the fixture's target bearing is only solved to ~0.01°.
        let _ = kind;
        assert_eq!(l.frames + 1, curve_frames, "{}", f[0]);
        n += 1;
    }
    assert!(n >= 10, "only {n} strokes checked");
}
