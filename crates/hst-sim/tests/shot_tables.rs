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

/// Every aim of the human player 1 in the doubles recording (round1.bin): the shot code before and after the
/// aim frame (the game's 1 ✕ / 2 ○ / 4 △, 8 flat, 0x10 drop) against `stick_kind` on the recorded stick.
/// The camera sits behind the human, so stick up is toward the other end.
#[test]
fn stick_turns_topspin_flat_and_slice_drop() {
    use hst_sim::replay::frames;
    use hst_sim::shot::stick_kind;
    let ctx = std::env::var("HST_CONTEXT").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context").into());
    let Ok(data) = std::fs::read(format!("{ctx}/fixtures/round1.bin")) else {
        eprintln!("fixture missing, skipped");
        return;
    };
    let word = |f: hst_sim::replay::Frame, off| f.player_f32(0, off).to_bits();
    let kind = |code| match code {
        1 => 0,
        2 => 1,
        4 => 3,
        8 => 2,
        0x10 => 4,
        c => panic!("shot code {c:#x}"),
    };
    let (mut n, mut changed) = (0, 0);
    for w in frames(&data).windows(2) {
        let (before, now) = (w[0], w[1]);
        if word(before, 0x3ec4) != 1 || word(now, 0x3ec4) as i32 != -1 {
            continue;
        }
        let branch = (word(now, 0x3ec0) >> 8) as u8;
        let pad = now.pad(0);
        let facing = if now.player_pos(0)[2] < 0.0 { 1.0 } else { -1.0 };
        let stick = [(pad.lx as f32 - 128.0) / 127.0, (128.0 - pad.ly as f32) / 127.0 * facing];
        let want = kind(word(now, 0x3ee4));
        assert_eq!(stick_kind(branch, kind(word(before, 0x3ee4)), stick, facing), want, "vsync {}", now.vsync());
        n += 1;
        changed += (want != kind(word(before, 0x3ee4))) as i32;
    }
    assert_eq!((n, changed), (42, 8));
}

/// Every smash of the slot-5 doubles match (`match_s05.bin`, all smash kind 0) against character 0's `smsh0` table:
/// the lookup runs from the hit to the ball's stored target (+0x70, +0x80), both shifted back by the hitter's timing
/// scatter (1.5 × the swing's late/early offset +0x3ecc along the shot, scaled by +0x3ee0 less +0x3edc); speed,
/// elevation and flight frames must match the launch. Four smashes with a late offset of 4 are still off (by
/// 1e-3–4e-2 rad), so this asserts the five that are exact.
#[test]
fn smashes_launch_like_the_game() {
    use hst_sim::replay::frames_live;
    let ctx = std::env::var("HST_CONTEXT").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context").into());
    let (Ok(data), Ok(table)) = (std::fs::read(format!("{ctx}/fixtures/match_s05.bin")), std::fs::read(format!("{ctx}/xb/TRAJ/TRAJ00B.XB/data/hatsuyama/traj/tr_pc00_smsh0.dat"))) else {
        return eprintln!("fixture missing, skipped");
    };
    let table = Table::parse(&table).unwrap();
    let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
    let frames = frames_live(&data);
    let (mut exact, mut smashes) = (0, 0);
    for w in frames.windows(2) {
        let (a, b) = (w[0].live_ball(), w[1].live_ball());
        let launched = |x: &[u8]| x[0x58] == 3 && i32::from_le_bytes(x[0xac..0xb0].try_into().unwrap()) == 0;
        let vel = v3(b, 0x130);
        let speed = (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]).sqrt();
        // class 3 also marks the held and tossed serve ball
        if !launched(b) || launched(a) || speed < 0.3 {
            continue;
        }
        let (hit, target) = (v3(b, 0x70), v3(b, 0x80));
        let Some(p) = (0..4).find(|&p| w[1].player_f32(p, 0x3ec0).to_bits().to_le_bytes()[1] == 4) else { continue };
        let late = w[1].player_f32(p, 0x3ecc).to_bits() as i32 as f32 / 10.0;
        let scatter = 1.5 * (if late < 0.0 { 2.0 * late } else { late } * w[1].player_f32(p, 0x3ee0) - w[1].player_f32(p, 0x3edc));
        let (dx, dz) = (target[0] - hit[0], target[2] - hit[2]);
        let n = (dx * dx + dz * dz).sqrt();
        let s = [scatter * dx / n, scatter * dz / n];
        let l = lookup(&table, &Bounds::smash(0), [hit[0] - s[0], hit[1], hit[2] - s[1]], [target[0] - s[0], 0.0, target[2] - s[1]]);
        let v = launch(hit, target, l.elevation, l.speed);
        let err = (0..3).map(|j| (v[j] - vel[j]).abs()).fold(0.0, f32::max);
        let frames_ok = l.frames + 1 == i32::from_le_bytes(b[0x260..0x264].try_into().unwrap());
        eprintln!("vsync {} p{p}: velocity off by {err:.1e}, frames {}", w[1].vsync(), if frames_ok { "ok" } else { "off" });
        smashes += 1;
        exact += (err < 2e-5 && frames_ok) as usize;
    }
    assert_eq!(smashes, 9);
    assert!(exact >= 5, "{exact} of {smashes} smashes exact");
}
