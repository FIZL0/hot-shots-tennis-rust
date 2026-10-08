//! Stroke launches recorded from a bot match whose table lookups were confirmed exactly (speed, elevation and
//! flight frames) by `research/tools/traj_inverse2.py`. Fixture `context/fixtures/shot_tables_s05.csv` and the
//! tables under `context/xb` stay out of git; skips when absent.

use hst_sim::shot::{Bounds, Table, launch, launch_frame, lookup};

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

/// Every smash of the slot-5 doubles match (`match_s05.bin`, all smash kind 0) against the hitter's own character's
/// `smsh0` table: the lookup runs from the hit to the ball's stored target (+0x70, +0x80), both shifted back by the
/// hitter's timing scatter (1.5 × the swing's late/early offset +0x3ecc along the shot, scaled by +0x3ee0 less
/// +0x3edc, and 1.5 × the side offset across it, see `smashes`); speed, elevation and flight frames must match the launch. (Four of them, by characters 2 and 5, were
/// off while character 0's table stood in for everyone.)
#[test]
fn smashes_launch_like_the_game() {
    let Some((smashes, exact)) = smashes("match_s05.bin", 0) else { return };
    assert_eq!((smashes, exact), (9, 9));
}

/// The human's smashes in a slot-4 doubles game driven by the virtual pad (`tools/record_human_smash.py`, P1 human,
/// the opponents' strokes turned into lobs): two with each of ✕ and ○ (kind 0) and △ (kind 1), plus the partner's.
#[test]
fn human_smashes_launch_like_the_game() {
    let (Some(k0), Some(k1)) = (smashes("human_smash_s04.bin", 0), smashes("human_smash_s04.bin", 1)) else { return };
    eprintln!("kind 0 {k0:?}, kind 1 {k1:?}");
    assert!(k0.0 >= 4 && k0.0 == k0.1 && k1.0 >= 2 && k1.0 == k1.1, "kind 0 {k0:?}, kind 1 {k1:?}");
}

/// The smash in the slot-3 match with a human (`new_recording.bin`), by the bot opponent (character 8).
#[test]
fn human_match_smash_launches_like_the_game() {
    let Some((smashes, exact)) = smashes("new_recording.bin", 0) else { return };
    assert_eq!((smashes, exact), (1, 1));
}

/// The same bot match with every smash turned into a △ smash (`tools/record_lob_smash.py`, +0x3ee4 = 4 once the
/// search locks a smash): kind 1 from the hitter's `smsh1` table, one of them off a lob.
#[test]
fn lob_smashes_launch_like_the_game() {
    let Some((smashes, exact)) = smashes("lob_smash_s05.bin", 1) else { return };
    assert_eq!((smashes, exact), (3, 3));
}

/// The smashes off the four lobs of the doubles match with one human and three CPUs on court 11
/// (`1p3goodcpus.bin`): the human Carol's ✕ smash (kind 0, vsync 19655) off Will's lob and character 10's three △
/// smashes (kind 1).
#[test]
fn goodcpus_lob_smashes_launch_like_the_game() {
    let (Some(k0), Some(k1)) = (smashes("1p3goodcpus.bin", 0), smashes("1p3goodcpus.bin", 1)) else { return };
    assert_eq!((k0, k1), ((1, 1), (3, 3)));
}

/// (smashes of `kind` launched in `fixture`, how many exactly), None when absent.
fn smashes(fixture: &str, kind: i32) -> Option<(usize, usize)> {
    use hst_sim::replay::frames_live;
    let ctx = std::env::var("HST_CONTEXT").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context").into());
    let table = |c: i32| {
        ["A", "B"].iter().find_map(|ab| std::fs::read(format!("{ctx}/xb/TRAJ/TRAJ{c:02}{ab}.XB/data/hatsuyama/traj/tr_pc{c:02}_smsh{kind}.dat")).ok())
            .and_then(|b| Table::parse(&b))
    };
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| format!("{ctx}/fixtures"));
    let (Ok(data), Some(_)) = (std::fs::read(format!("{dir}/{fixture}")), table(0)) else {
        eprintln!("fixture missing, skipped");
        return None;
    };
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
        if !launched(b) || launched(a) || speed < 0.1 || i32::from_le_bytes(b[0x5c..0x60].try_into().unwrap()) != kind {
            continue;
        }
        let (hit, target) = (v3(b, 0x70), v3(b, 0x80));
        let Some(p) = (0..4).find(|&p| w[1].player_f32(p, 0x3ec0).to_bits().to_le_bytes()[1] == 4) else { continue };
        let late = w[1].player_f32(p, 0x3ecc).to_bits() as i32 as f32 / 10.0;
        let scatter = 1.5 * (if late < 0.0 { 2.0 * late } else { late } * w[1].player_f32(p, 0x3ee0) - w[1].player_f32(p, 0x3edc));
        // and 1.5 × the side offset (+0x3ed0, already stepped by +0x3ed4 and clamped) across it, to the right of the
        // unscattered shot direction u (side = up × u = (u.z, −u.x)); the stored target carries the scatter, so u is
        // recovered from w = target − hit = c·u + side·(u.z, −u.x)
        let side = 1.5 * (w[1].player_f32(p, 0x3ed0).to_bits() as i32 as f32 / 10.0 / 2.0);
        let (wx, wz) = (target[0] - hit[0], target[2] - hit[2]);
        let w2 = wx * wx + wz * wz;
        let c = (w2 - side * side).sqrt();
        let u = [(c * wx - side * wz) / w2, (c * wz + side * wx) / w2];
        let s = [scatter * u[0] + side * u[1], scatter * u[1] - side * u[0]];
        let l = lookup(&table(w[1].global(0x422fa8 + 4 * p)).unwrap(), &Bounds::smash(kind), [hit[0] - s[0], hit[1], hit[2] - s[1]], [target[0] - s[0], 0.0, target[2] - s[1]]);
        let v = launch(hit, target, l.elevation, l.speed);
        let err = (0..3).map(|j| (v[j] - vel[j]).abs()).fold(0.0, f32::max);
        let frames_ok = l.frames + 1 == i32::from_le_bytes(b[0x260..0x264].try_into().unwrap());
        eprintln!("vsync {} p{p}: velocity off by {err:.1e}, frames {}, spin {}", w[1].vsync(), if frames_ok { "ok" } else { "off" }, f(b, 0x1a4).to_degrees());
        smashes += 1;
        exact += (err < 2e-5 && frames_ok) as usize;
    }
    Some((smashes, exact))
}

/// Every unscattered volley (class 2) launch in the live-ball recordings against the hitter's own `voly` table
/// (base, or the up1/dw1/dw2 the game's mode pick took) with the volley bounds and the near-net speed correction.
#[test]
fn volleys_launch_like_the_game() {
    let (seen, exact) = launches(&["match_s05.bin", "new_recording.bin", "1p3goodcpus.bin"], 2, None);
    if seen == 0 {
        return eprintln!("fixtures missing, skipped");
    }
    eprintln!("{exact} of {seen} volleys exact");
    assert!(exact == seen && seen >= 9, "only {exact} of {seen} volleys exact");
}

/// The lobs (kind 3) of the doubles match with one human and three CPUs on court 11 (`1p3goodcpus.bin`): Will's
/// stroke (vsync 19585, his `strk3`) and character 2's volley (20682); Carol's two (21143, 21279) carry timing
/// scatter and are left out like any other. Every unscattered ground stroke launches bit for bit too.
#[test]
fn goodcpus_lobs_launch_like_the_game() {
    let lobs = [1, 2].map(|class| launches(&["1p3goodcpus.bin"], class, Some(3)));
    if lobs[1].0 == 0 {
        return eprintln!("1p3goodcpus.bin absent, skipped");
    }
    let (seen, exact) = launches(&["1p3goodcpus.bin"], 1, None);
    assert_eq!(seen, exact, "strokes off their tables");
    assert_eq!(lobs, [(1, 1), (1, 1)]);
}

/// (unscattered launches of `class` (1 stroke, 2 volley; of `only` kind if given) in `fixtures`, how many exact
/// against the hitter's `strk`/`voly` table (base, or the up1/dw1/dw2 the game's mode pick took)). A counter
/// (+0x3f07: perfect timing against a stronger hitter's topspin or flat) launches from the incoming hitter's tables.
fn launches(fixtures: &[&str], class: u8, only: Option<i32>) -> (usize, usize) {
    use hst_sim::replay::{frames_live, SAMPLE_LIVE};
    let ctx = std::env::var("HST_CONTEXT").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context").into());
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| format!("{ctx}/fixtures"));
    let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let i = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
    let table = |c: i32, name: &str| {
        ["A", "B"].iter().find_map(|ab| {
            std::fs::read(format!("{ctx}/xb/TRAJ/TRAJ{c:02}{ab}.XB/data/hatsuyama/traj/tr_pc{c:02}_{name}.dat")).ok()
        })
        .and_then(|b| Table::parse(&b))
    };
    let (mut seen, mut exact) = (0, 0);
    for fx in fixtures {
        let Ok(data) = std::fs::read(format!("{dir}/{fx}")) else { continue };
        assert_eq!(data.len() % SAMPLE_LIVE, 0, "{fx}: not a live-ball recording");
        for w in frames_live(&data).windows(2) {
            let (a, b) = (w[0].live_ball(), w[1].live_ball());
            // the launch frame: the class, flight frame 0, a new hit point, a table's flight time (0 off the table)
            if b[0x58] != class || i(b, 0xac) != 0 || i(b, 0x260) == 0 || (i(a, 0xac) == 0 && v3(a, 0x70) == v3(b, 0x70)) {
                continue;
            }
            let (hit, target, vel, kind) = (v3(b, 0x70), v3(b, 0x80), v3(b, 0x130), i(b, 0x5c));
            if only.is_some_and(|k| k != kind) {
                continue;
            }
            let d2 = |p: usize| {
                let q = w[1].player_pos(p);
                (q[0] - hit[0]).powi(2) + (q[2] - hit[2]).powi(2)
            };
            let p = (0..4).min_by(|&x, &y| d2(x).total_cmp(&d2(y))).unwrap();
            // late/side timing scatter moves the lookup's hit point away from the ball's; those are P3's
            if w[1].player_f32(p, 0x3ecc) != 0.0 || w[1].player_f32(p, 0x3ed0) != 0.0 {
                continue;
            }
            seen += 1;
            let counter = w[1].player_f32(p, 0x3f04).to_bits().to_le_bytes()[3] == 1;
            let ch = w[1].global(0x422fa8 + 4 * if counter { w[0].global(0x423058) as usize } else { p });
            let (stem, bounds) = if class == 1 { ("strk", Bounds::stroke(kind, hit[2])) } else { ("voly", Bounds::volley(kind, hit[2])) };
            let ok = ["", "_up1", "_dw1", "_dw2"].iter().any(|suf| {
                let Some(t) = table(ch, &format!("{stem}{kind}{suf}")) else { return false };
                let l = lookup(&t, &bounds, hit, target);
                // a poorly timed (grade 4) stroke's elevation is scaled down (P3's mis-hit roll): 0.95, or one of 0.9..0.8
                [1.0, 0.95, 0.9, 0.85, 0.8].iter().any(|&scale| {
                    let elevation = if scale == 1.0 { l.elevation } else { hst_sim::ps2::mul(l.elevation, scale) };
                    let (frame, v) = (launch_frame(hit, target, elevation), launch(hit, target, elevation, l.speed));
                    let rows = (0..4).all(|r| (0..4).all(|k| frame[r][k] == f(b, 0x160 + 16 * r + 4 * k)));
                    v == vel && rows
                }) && l.frames + 1 == i(b, 0x260)
            });
            if !ok {
                eprintln!("{fx} vsync {} p{p} (character {ch}): class {class} kind {kind} off its tables", w[1].vsync());
            }
            exact += ok as usize;
        }
    }
    (seen, exact)
}

/// Every recorded stroke and volley leaves with the spin, first-bounce spin and first-bounce restitution
/// `params::rally_spin` gives for one of the hitting team's shot records (the base record, or the up1/dw1/dw2
/// variant the game picked with the table mode, P3), bit for bit. The hitter is any player of the recording
/// whose record fits, since the nearest one isn't always the hitter.
#[test]
fn rally_spins_like_the_game() {
    use hst_sim::params::{ShotParams, rally_spin, record_of};
    use hst_sim::replay::frames_live;
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| format!("{ctx}/fixtures"));
    let (Ok(cnf), Ok(bin)) = (std::fs::read(format!("{ctx}/iso/SYSTEM.CNF")), std::fs::read(format!("{ctx}/iso/ZZBIN/GAME.BIN"))) else {
        return eprintln!("extracted disc absent, skipped");
    };
    let game = hst_data::exe::Game::new(&cnf, &bin).unwrap();
    let src = game.shot_params();
    let params = ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
    let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let i = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
    let (mut base, mut all, mut kinds) = (0, 0, [[0; 5]; 2]);
    for name in ["match_s05.bin", "new_recording.bin", "lob_smash_s05.bin", "human_smash_s04.bin", "1p3goodcpus.bin"] {
        let Ok(data) = std::fs::read(format!("{dir}/{name}")) else { continue };
        for w in frames_live(&data).windows(2) {
            let (a, b) = (w[0].live_ball(), w[1].live_ball());
            let class = b[0x58] as usize;
            if !(class == 1 || class == 2) || i(b, 0xac) != 0 || i(b, 0x260) == 0 || (i(a, 0xac) == 0 && v3(a, 0x70) == v3(b, 0x70)) {
                continue;
            }
            let (hit, target, kind) = (v3(b, 0x70), v3(b, 0x80), i(b, 0x5c) as usize);
            let got = [f(b, 0x1a4), f(b, 0x1a8), f(b, 0x1ac)].map(f32::to_bits);
            let fits = |rec: &[f32]| {
                let (s, fb, r) = rally_spin(rec, class, kind, hit, target);
                [s, fb, r].map(f32::to_bits) == got
            };
            let chars: Vec<usize> = (0..4).map(|p| w[1].global(0x422fa8 + 4 * p) as usize).collect();
            let on_base = chars.iter().any(|&c| fits(params.record(class, kind, record_of(c))));
            let on_variant = chars.iter().any(|&c| {
                game.shot_variants(c).iter().filter(|v| v.class == class && v.kind == kind).any(|v| fits(&params.variant(class, kind, record_of(c), v.weight)))
            });
            assert!(on_base || on_variant, "{name} vsync {}: class {class} kind {kind} hit {hit:?} spins {got:x?}", w[1].vsync());
            base += on_base as usize;
            all += 1;
            kinds[class - 1][kind] += 1;
        }
    }
    eprintln!("{all} strokes/volleys, {base} on the base record; by class/kind {kinds:?}");
    assert!(all > 100 && base > 50, "{all} launches, {base} on base records");
}

/// Every unscattered stroke, volley and smash launch of the live-ball recordings against the hitter's aim (+0x3e90)
/// pulled inside the court by `Margins::inside`, bit for bit: the ball's stored target (+0x80) is that aim plus
/// the timing scatter, added after the pull (a launch counts as unscattered when the swing's late, side and
/// smash depth offsets +0x3ecc/+0x3ed0/+0x3edc are all zero). 1p3goodcpus (doubles) has character 10's two
/// short angled topspins pulled in by the angle margin (vsync 19076) or onto the doubles sideline (19143), and
/// human_smash_s04 a volley pulled in from the baseline (11267). No recorded smash reaches a line; they are
/// covered by the hand-worked `shot::tests::inside_pulls_smashes_and_outward_topspin`.
#[test]
fn rally_aims_pulled_inside_like_the_game() {
    use hst_sim::replay::frames_live;
    use hst_sim::shot::Margins;
    let ctx = std::env::var("HST_CONTEXT").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context").into());
    let (Ok(cnf), Ok(bin)) = (std::fs::read(format!("{ctx}/iso/SYSTEM.CNF")), std::fs::read(format!("{ctx}/iso/ZZBIN/GAME.BIN"))) else {
        return eprintln!("the extracted disc absent, skipped");
    };
    let (lines, angle) = hst_data::exe::Game::new(&cnf, &bin).unwrap().court_margins();
    let m = Margins { lines, angle };
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| format!("{ctx}/fixtures"));
    let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let i = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
    for (fx, want_n, want_moved) in [("match_s05.bin", 57, 0), ("new_recording.bin", 15, 0), ("1p3goodcpus.bin", 18, 2), ("human_smash_s04.bin", 7, 1), ("lob_smash_s05.bin", 18, 0)] {
        let Ok(data) = std::fs::read(format!("{dir}/{fx}")) else {
            eprintln!("{fx} absent, skipped");
            continue;
        };
        let (mut n, mut moved) = (0, 0);
        for w in frames_live(&data).windows(2) {
            let (a, b) = (w[0].live_ball(), w[1].live_ball());
            let class = b[0x58];
            // a launch: a stroke, volley or smash on its flight frame 0 with a new hit point and a table's flight time
            if !(1..=3).contains(&class) || i(b, 0xac) != 0 || i(b, 0x260) == 0 || (i(a, 0xac) == 0 && v3(a, 0x70) == v3(b, 0x70)) {
                continue;
            }
            let (hit, target, kind) = (v3(b, 0x70), v3(b, 0x80), i(b, 0x5c));
            let d2 = |p: usize| {
                let q = w[1].player_pos(p);
                (q[0] - hit[0]).powi(2) + (q[2] - hit[2]).powi(2)
            };
            let p = if class == 3 {
                (0..4).find(|&p| w[1].player_f32(p, 0x3ec0).to_bits().to_le_bytes()[1] == 4).expect("smasher")
            } else {
                (0..4).min_by(|&x, &y| d2(x).total_cmp(&d2(y))).unwrap()
            };
            if [0x3ecc, 0x3ed0, 0x3edc].iter().any(|&o| w[1].player_f32(p, o) != 0.0) {
                continue;
            }
            let aim = [0x3e90, 0x3e94, 0x3e98].map(|o| w[1].player_f32(p, o));
            // a counter is judged with the incoming hitter's character, as its tables
            let counter = w[1].player_f32(p, 0x3f04).to_bits().to_le_bytes()[3] == 1;
            let ch = w[1].global(0x422fa8 + 4 * if counter { w[0].global(0x423058) as usize } else { p }) as usize;
            // the smash's shot mode (+0x3ed8 tenths, its sign kept by the scaling)
            let low = class == 3 && (w[1].player_f32(p, 0x3ed8).to_bits() as i32) < 0;
            let got = m.inside(class, kind, low, w[1].global(0x422fa4) >= 3, ch, false, hit, aim);
            let at = format!("{fx} vsync {} player {p} (character {ch}) class {class} kind {kind}: hit {hit:?} aim {aim:?}", w[1].vsync());
            assert_eq!([got[0].to_bits(), got[2].to_bits()], [target[0].to_bits(), target[2].to_bits()], "{at}: {got:?} vs {target:?}");
            n += 1;
            moved += (aim != target) as usize;
        }
        eprintln!("{fx}: {n} unscattered launches, {moved} pulled inside");
        assert_eq!((n, moved), (want_n, want_moved), "{fx}");
    }
}

/// P5: the game's own swings off P1's recorded pad (`research/p5_input_rec.py`, slot 4: ✕ held for seconds, two and
/// three buttons on one frame, double taps, d-pad diagonals). Every swing the game starts (the stroke countdown
/// +0x3ec4 leaving −1) comes on a frame `press_kind` reads a press, with its shot code (+0x3ee4: 1 ✕, 2 ○, 4 △);
/// a button held on gives none.
#[test]
fn presses_start_swings_like_the_game() {
    use hst_sim::replay::frames_live;
    use hst_sim::shot::press_kind;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(rec) = std::fs::read(format!("{dir}/p5_presses_s04.bin")) else {
        return eprintln!("p5_presses_s04.bin absent, skipped");
    };
    let frames = frames_live(&rec);
    let word = |k: usize, off: usize| frames[k].player_f32(0, off).to_bits() as i32;
    let (mut swings, mut held) = (0, 0);
    for k in 1..frames.len() {
        let (before, now) = (frames[k - 1].pad(0).buttons, frames[k].pad(0).buttons);
        let press = press_kind(before, now);
        if word(k - 1, 0x3ec4) == -1 && word(k, 0x3ec4) != -1 {
            let code = press.map(|k| [1, 2, 0, 4][k as usize]);
            assert_eq!(code, Some(word(k, 0x3ee4)), "frame {k}: pad {before:04x} -> {now:04x}");
            swings += 1;
        } else if press.is_none() && now & 0x7000 != 0 {
            held += 1;
        }
    }
    eprintln!("{swings} swings, {held} held frames without one");
    assert!(swings >= 10 && held > 100);
}
