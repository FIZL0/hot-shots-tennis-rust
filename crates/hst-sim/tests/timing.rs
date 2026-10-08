//! Timing grades against the live-ball recordings: every player's grade and bias tables from its character's
//! TParam counts, and every recorded stroke and volley (scattered or not) through the timing error at the lock,
//! its launch scaling, the table mode it picks and the scatter, to the launch velocity, flight frames and stored
//! target, all bit for bit; mis-hits too: a dull hit's launch takes one of `mis_hit`'s scales, a framed hit's
//! one of `wild_aim`'s spots and errors.

use hst_sim::player::ReachStats;
use hst_sim::replay::{Frame, frames_live};
use hst_sim::shot::{Bounds, Margins, Table, launch, lookup};
use hst_sim::swing::{TimingError, high_blend, mis_hit, lob_variant, mode_variant, table_mode, timing, timing_error, timing_launch};
use hst_sim::ps2::mul;

const FIXTURES: [(&str, Option<&str>); 5] = [
    ("match_s05.bin", Some("slot5_ee.bin")),
    ("lob_smash_s05.bin", Some("slot5_ee.bin")),
    ("1p3goodcpus.bin", Some("1p3goodcpus_ee.bin")),
    ("new_recording.bin", None),
    ("human_smash_s04.bin", None),
];

fn word(fr: Frame, p: usize, off: usize) -> i32 {
    fr.player_f32(p, off & !3).to_bits() as i32
}
fn byte(fr: Frame, p: usize, off: usize) -> u8 {
    word(fr, p, off).to_le_bytes()[off & 3]
}

struct Disc {
    stats: Vec<ReachStats>,
    right: Vec<bool>,
    game_bin: Vec<u8>,
    cnf: Vec<u8>,
}

fn disc() -> Option<Disc> {
    use hst_data::{iso::Iso, xb::Archive};
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let mut iso = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")).ok()?;
    let data = iso.read("PCDATA/PCDATA.XB").unwrap();
    let arc = Archive::parse(&data).unwrap();
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).unwrap();
    let csv = arc.read(e).unwrap();
    let rows: Vec<&[u8]> = (0..14).map(|n| csv.split(|&b| b == b'\n').find(|l| l.starts_with(format!("{n},").as_bytes())).unwrap()).collect();
    Some(Disc {
        stats: rows.iter().map(|r| ReachStats::from_tparam(&String::from_utf8_lossy(r))).collect(),
        right: rows.iter().map(|r| r.split(|&b| b == b',').nth(6).unwrap().starts_with(&[0x89, 0x45])).collect(),
        game_bin: std::fs::read(format!("{ctx}/iso/ZZBIN/GAME.BIN")).ok()?,
        cnf: std::fs::read(format!("{ctx}/iso/SYSTEM.CNF")).ok()?,
    })
}

fn dir() -> String {
    std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into())
}

/// Every player's grade (+0x1510, length +0x154c) and bias (+0x1420) tables and the power / pressure stats the
/// timing uses (+0x138c..+0x139c) against its character's TParam row.
#[test]
fn timing_tables_like_the_game() {
    let Some(d) = disc() else { return eprintln!("disc absent, skipped") };
    let mut n = 0;
    for (fx, _) in FIXTURES {
        let Ok(data) = std::fs::read(format!("{}/{fx}", dir())) else { continue };
        let fr = *frames_live(&data).last().unwrap();
        for p in 0..4 {
            let c = fr.global(0x422fa8 + 4 * p) as usize;
            let s = &d.stats[c];
            let len = word(fr, p, 0x154c) as usize;
            let (g, b) = timing(s.after, s.before);
            let want: Vec<u8> = (0..len).map(|k| byte(fr, p, 0x1510 + k)).collect();
            let want_b: Vec<i32> = (0..len).map(|k| word(fr, p, 0x1420 + 4 * k)).collect();
            assert_eq!((g, b), (want, want_b), "{fx} player {p} character {c}");
            assert_eq!([s.body_down, s.back_down, s.volley_down, s.low_power[0], s.low_power[1]], [0x138c, 0x1390, 0x1394, 0x1398, 0x139c].map(|o| word(fr, p, o)), "{fx} character {c}");
            n += 1;
        }
    }
    assert!(n == 0 || n >= 12, "{n} players");
}

/// Every stroke and volley launch: the hitter's lock (grade +0x3ee8, bias +0x3f98 and offset +0x3fa0 from its
/// timing tables at the contact frame +0x3ec4), the timing error the lock leaves (+0x3ecc/+0x3ed0/+0x3ed8 the frame
/// before the launch), the launch's scatter and blend from it (the stored +0x3ecc/+0x3ed0 after), the table mode,
/// and the ball: velocity, flight frames and target (the aim pulled inside, plus the scatter).
#[test]
fn timed_launches_like_the_game() {
    let Some(d) = disc() else { return eprintln!("disc absent, skipped") };
    let game = hst_data::exe::Game::new(&d.cnf, &d.game_bin).unwrap();
    let (lines, angle) = game.court_margins();
    let m = Margins { lines, angle };
    let down2 = game.down2_blend();
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let table = |c: usize, name: &str| {
        ["A", "B"].iter().find_map(|ab| std::fs::read(format!("{ctx}/xb/TRAJ/TRAJ{c:02}{ab}.XB/data/hatsuyama/traj/tr_pc{c:02}_{name}.dat")).ok()).and_then(|b| Table::parse(&b))
    };
    let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let i = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
    let (mut seen, mut scattered, mut moded, mut bad) = (0, 0, 0, vec![]);
    let (mut dulls, mut framed) = (0, 0);
    for (fx, ram) in FIXTURES {
        let Ok(data) = std::fs::read(format!("{}/{fx}", dir())) else { continue };
        let ram = ram.and_then(|r| std::fs::read(format!("{}/{r}", dir())).ok());
        let frames = frames_live(&data);
        let hand = |p: usize, c: usize| match &ram {
            Some(ram) => {
                let ru = |a: usize| u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize;
                f(ram, ru(ru(0x422f80) + 0xa8 + 4 * p) + 0x12b4) >= 0.0
            }
            None => d.right[c],
        };
        for k in 1..frames.len() {
            let (w0, w1) = (frames[k - 1], frames[k]);
            let (a, b) = (w0.live_ball(), w1.live_ball());
            let class = b[0x58];
            if !(class == 1 || class == 2) || i(b, 0xac) != 0 || i(b, 0x260) == 0 || (i(a, 0xac) == 0 && v3(a, 0x70) == v3(b, 0x70)) {
                continue;
            }
            let (hit, target, vel, kind) = (v3(b, 0x70), v3(b, 0x80), v3(b, 0x130), i(b, 0x5c));
            // the hitter: the one whose launch just stored its aim's flags and is on a rally branch
            let d2 = |p: usize| {
                let q = w1.player_pos(p);
                (q[0] - hit[0]).powi(2) + (q[2] - hit[2]).powi(2)
            };
            let Some(p) = (0..4).filter(|&p| word(w0, p, 0x3ec4) >= 0).min_by(|&x, &y| d2(x).total_cmp(&d2(y))) else {
                bad.push(format!("{fx} vsync {}: no swing locked", w1.vsync()));
                continue;
            };
            let at = format!("{fx} vsync {} p{p} class {class} kind {kind}", w1.vsync());
            let branch = byte(w1, p, 0x3ec1);
            let c = w1.global(0x422fa8 + 4 * p) as usize;
            let s = &d.stats[c];
            // the lock: the contact frame +0x3ec4 counts down to the hit from the frame it locked on
            let Some(lock) = (1..k).rev().find(|&j| word(frames[j - 1], p, 0x3ec4) < 0).map(|j| word(frames[j], p, 0x3ec4)) else {
                continue; // locked before the recording starts
            };
            let (grades, bias) = timing(s.after, s.before);
            let grade = byte(w1, p, 0x3ee8);
            assert_eq!((grade, word(w1, p, 0x3f98), word(w1, p, 0x3fa0)), (grades[lock as usize], bias[lock as usize], lock - 8), "{at}: lock");
            // the error, set up at the launch and clamped there (+0x3ecc depth, +0x3ed0 side with the aim's nudge
            // +0x3ed4, +0x3ed8 mode)
            let flags = word(w1, p, 0x3f50);
            let right = hand(p, c);
            let end = if w1.player_pos(p)[2] < 0.0 { 1.0 } else { -1.0 };
            let e = timing_error(s, branch, word(w1, p, 0x3ee4) == 4, bias[lock as usize], lock - 8, w1.player_f32(p, 0x3f44), flags & 1 != 0, flags & if right { 1 } else { 2 } != 0, flags & 4 != 0, [w1.player_f32(p, 0x3e60), w1.player_f32(p, 0x3e68)], end);
            let short_only = byte(w1, p, 0x3eca) != 0;
            let nudge = word(w1, p, 0x3ed4);
            let clean = grade == 1 || grade == 2;
            let stored = TimingError {
                depth: if clean { 0 } else { e.depth }.clamp(-10, if short_only { 0 } else { 10 }),
                side: (if clean { 0 } else { e.side } + nudge).clamp(-10, 10),
                mode: e.mode.clamp(-10, 10),
            };
            let want_e = TimingError { depth: word(w1, p, 0x3ecc), side: word(w1, p, 0x3ed0), mode: word(w1, p, 0x3ed8) };
            if stored != want_e {
                bad.push(format!("{at}: branch {branch} grade {grade} flags {flags} right {right} error {e:?} stored {stored:?} want {want_e:?}"));
                continue;
            }
            let height = w1.player_f32(p, 0x3f44);
            if byte(w1, p, 0x3f06) != 0 {
                // a framed hit: a lob (kind 3, blend 0, the hitter's own table) to `wild_aim`'s spot pulled inside,
                // scattered by its side or depth error; the draws are the game's, so solve for the spot and error
                // that give the recorded target and launch from them
                framed += 1;
                let doubles = w1.global(0x422fa4) >= 3;
                let t3 = table(c, &format!("{}3", if class == 1 { "strk" } else { "voly" })).unwrap();
                // (spot, side, depth) of each way `wild_aim` can land, by its two free numbers
                let shapes: [&dyn Fn(f32, f32) -> ([f32; 3], f32, f32); 4] = [
                    &|x, z| ([x, 0.0, z], 0.0, 0.0),
                    &|x, d| ([x, 0.0, mul(11.885, end)], 0.0, d),
                    &|z, sd| ([if doubles { 5.485 } else { 4.115 }, 0.0, z], sd, 0.0),
                    &|z, sd| ([if doubles { -5.485 } else { -4.115 }, 0.0, z], sd, 0.0),
                ];
                let fly = |shape: &dyn Fn(f32, f32) -> ([f32; 3], f32, f32), a: f32, bb: f32| {
                    let (spot, sd, dp) = shape(a, bb);
                    let pulled = m.inside(class, 3, false, doubles, c, false, hit, spot);
                    let sc = hst_sim::serve::scatter_along(sd, dp, hit, pulled);
                    let to = [hst_sim::ps2::add(pulled[0], sc[0]), pulled[1], hst_sim::ps2::add(pulled[2], sc[2])];
                    let hz = hst_sim::ps2::sub(hit[2], sc[2]);
                    let bounds = if class == 1 { Bounds::stroke(3, hz) } else { Bounds::volley(3, hz) };
                    let l = lookup(&t3, &bounds, [hst_sim::ps2::sub(hit[0], sc[0]), hit[1], hz], pulled);
                    (to, l.frames + 1 == i(b, 0x260) && launch(hit, to, l.elevation, l.speed) == vel)
                };
                let up = |x: f32, k: i32| f32::from_bits((x.to_bits() as i32 + k) as u32);
                let ok = shapes.iter().enumerate().any(|(n, shape)| {
                    // Newton on the two free numbers from the target itself, then the floats around the root
                    let (mut a, mut bb) = if n == 0 { (target[0], target[2]) } else if n == 1 { (target[0], 0.0) } else { (target[2], 0.0) };
                    for _ in 0..30 {
                        let r = |a: f32, bb: f32| { let t = fly(*shape, a, bb).0; [(t[0] - target[0]) as f64, (t[2] - target[2]) as f64] };
                        let (r0, ra, rb) = (r(a, bb), r(a + 1e-3, bb), r(a, bb + 1e-3));
                        let j = [[(ra[0] - r0[0]) / 1e-3, (rb[0] - r0[0]) / 1e-3], [(ra[1] - r0[1]) / 1e-3, (rb[1] - r0[1]) / 1e-3]];
                        let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
                        if det.abs() < 1e-9 {
                            break;
                        }
                        a -= ((r0[0] * j[1][1] - r0[1] * j[0][1]) / det) as f32;
                        bb -= ((j[0][0] * r0[1] - j[1][0] * r0[0]) / det) as f32;
                    }
                    (-60..=60).any(|da| (-60..=60).any(|db| {
                        let (to, flew) = fly(*shape, up(a, da), up(bb, db));
                        to == target && flew
                    }))
                });
                if kind != 3 || !ok {
                    bad.push(format!("{at}: framed hit off every wild aim, target {target:?}"));
                }
                continue;
            }
            let dull = byte(w1, p, 0x3f0c) != 0;
            // the launch
            let (sx, sz, mut blend) = timing_launch(s, branch, kind, grade, e, nudge, short_only);
            let counter = byte(w1, p, 0x3f07) == 1;
            let src = if counter { w0.global(0x423058) as usize } else { p };
            let ch = w1.global(0x422fa8 + 4 * src) as usize;
            if branch == 1 && class == 2 {
                blend = high_blend(s);
            }
            if counter {
                blend = 0.0;
            }
            let mode = table_mode(class, c, grade, kind, blend, &down2);
            let variant = if kind == 3 { lob_variant(&d.stats[ch], grade) } else { mode_variant(mode) }.filter(|&v| game.shot_variants(ch).iter().any(|x| x.class == class as usize && x.kind == kind as usize && x.uses[v] == 1));
            let aim = [0x3e90, 0x3e94, 0x3e98].map(|o| w1.player_f32(p, o));
            let pulled = m.inside(class, kind, false, w1.global(0x422fa4) >= 3, ch, false, hit, aim);
            let sc = hst_sim::serve::scatter_along(sx, sz, hit, pulled);
            let hz = hst_sim::ps2::sub(hit[2], sc[2]);
            let (stem, bounds) = if class == 1 { ("strk", Bounds::stroke(kind, hz)) } else { ("voly", Bounds::volley(kind, hz)) };
            let suffix = variant.map_or("", |v| ["_up1", "_dw1", "_dw2", "_dw3"][v]);
            let to = [hst_sim::ps2::add(pulled[0], sc[0]), pulled[1], hst_sim::ps2::add(pulled[2], sc[2])];
            let t = table(ch, &format!("{stem}{kind}{suffix}")).unwrap_or_else(|| panic!("{at}: no table {stem}{kind}{suffix}"));
            let l = lookup(&t, &bounds, [hst_sim::ps2::sub(hit[0], sc[0]), hit[1], hz], pulled);
            // the mis-hit roll: none (draws of 99) or, on a dull hit, the scale entry it drew
            let roll = |k: u32| { let mut r = [0, k, k].into_iter().map(|x| x << 16); move || r.next().unwrap() };
            let scaled = |k| mis_hit(grade, branch, kind, false, false, height, roll(k));
            let Some(miss) = (if dull { [99, 97, 98].map(scaled).into_iter().find(|m| launch(hit, to, mul(l.elevation, m.scale), l.speed) == vel) } else { Some(mis_hit(grade, branch, kind, false, false, height, || 99 << 16)) }) else {
                bad.push(format!("{at}: dull hit off every scale"));
                continue;
            };
            dulls += dull as usize;
            let v = launch(hit, to, mul(l.elevation, miss.scale), l.speed);
            if v != vel || l.frames + 1 != i(b, 0x260) || to != target {
                bad.push(format!("{at}: grade {grade} branch {branch} error {e:?} mode {mode}{suffix} scatter {sx},{sz}: vel {v:?} want {vel:?}, frames {} want {}, target {to:?} want {target:?}", l.frames + 1, i(b, 0x260)));
            }
            seen += 1;
            scattered += (sx != 0.0 || sz != 0.0) as usize;
            moded += variant.is_some() as usize;
        }
    }
    for b in &bad {
        eprintln!("{b}");
    }
    eprintln!("{seen} launches, {scattered} scattered, {moded} off a variant table, {dulls} dull, {framed} framed, {} wrong", bad.len());
    assert!(bad.is_empty() && (seen == 0 || dulls > 0 && framed > 0) && (seen == 0 || seen > 100), "{} of {seen} launches wrong", bad.len());
}

/// `wild_aim`'s five ways by its draws (the case draws in the top half-word, as the game's RNG gives them).
#[test]
fn wild_aims_by_draw() {
    use hst_sim::swing::wild_aim;
    let aim = |draws: &[u32]| {
        let mut it = draws.iter().copied();
        wild_aim(false, -1.0, || it.next().unwrap())
    };
    let half = 1 << 31;
    // a corner: the sideline by the sign bit, 3 + 3.4·u deep toward −z
    assert_eq!(aim(&[14 << 16, 1 << 16, 0]), ([-4.115, 0.0, -3.0], 0.0, 0.0));
    assert_eq!(aim(&[0, 0, half]).0, [4.115, 0.0, -4.7]);
    // anywhere across
    assert_eq!(aim(&[64 << 16, half, 0]).0, [0.0, 0.0, -3.0]);
    // the baseline with a depth error (−0.66 + 1.66·u) / 1.5
    let near = |a: f32, b: f32| (a - b).abs() < 1e-6;
    let (a, side, depth) = aim(&[99 << 16, 32 << 16, 0, 0]);
    assert!(a == [-4.115, 0.0, -11.885] && side == 0.0 && near(depth, -0.44), "{depth}");
    // a sideline 3 + 8.885·u deep with a side error, on the end's side or the other
    let (a, side, _) = aim(&[99 << 16, 65 << 16, 0, u32::MAX]);
    assert!(a == [4.115, 0.0, -3.0] && near(side, -1.0 / 1.5), "{side}");
    let (a, side, _) = aim(&[99 << 16, 66 << 16, 0, u32::MAX]);
    assert!(a[0] == -4.115 && near(side, 1.0 / 1.5), "{side}");
}

/// Every smash launch: the scatter `smash_scatter` gives from the lock (grade +0x3ee8, bias +0x3f98), the contact
/// height, the aim's nudge (+0x3ed4) and held depth (+0x3edc) and `smash_scale` (+0x3ee0), to the stored target
/// (the aim pulled inside, plus the scatter) and the launch velocity off the base smash table, bit for bit.
#[test]
fn timed_smashes_like_the_game() {
    use hst_sim::swing::{smash_scale, smash_scatter};
    let Some(d) = disc() else { return eprintln!("disc absent, skipped") };
    let game = hst_data::exe::Game::new(&d.cnf, &d.game_bin).unwrap();
    let (lines, angle) = game.court_margins();
    let m = Margins { lines, angle };
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let table = |c: usize, kind: i32| {
        ["A", "B"].iter().find_map(|ab| std::fs::read(format!("{ctx}/xb/TRAJ/TRAJ{c:02}{ab}.XB/data/hatsuyama/traj/tr_pc{c:02}_smsh{kind}.dat")).ok()).and_then(|b| Table::parse(&b)).unwrap()
    };
    let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let i = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
    let (mut seen, mut scattered, mut held, mut scaled, mut bad) = (0, 0, 0, 0, vec![]);
    for (fx, _) in FIXTURES {
        let Ok(data) = std::fs::read(format!("{}/{fx}", dir())) else { continue };
        let frames = frames_live(&data);
        for k in 1..frames.len() {
            let (w0, w1) = (frames[k - 1], frames[k]);
            let (a, b) = (w0.live_ball(), w1.live_ball());
            let launched = |x: &[u8]| x[0x58] == 3 && i(x, 0xac) == 0;
            let vel = v3(b, 0x130);
            // class 3 also marks the held and tossed serve ball
            if !launched(b) || launched(a) || mul(vel[0], vel[0]) + mul(vel[1], vel[1]) + mul(vel[2], vel[2]) < 0.01 {
                continue;
            }
            let Some(p) = (0..4).find(|&p| byte(w1, p, 0x3ec1) == 4) else { continue };
            let (hit, target, kind) = (v3(b, 0x70), v3(b, 0x80), i(b, 0x5c));
            let at = format!("{fx} vsync {} p{p} kind {kind}", w1.vsync());
            let c = w1.global(0x422fa8 + 4 * p) as usize;
            let (grade, bias, offset) = (byte(w1, p, 0x3ee8), word(w1, p, 0x3f98), word(w1, p, 0x3fa0));
            let (nudge, hold) = (word(w1, p, 0x3ed4), w1.player_f32(p, 0x3edc));
            let scale = smash_scale(word(w1, p, 0x3ee4) != 4, offset, w1.player_f32(p, 0x3ef8));
            if scale.to_bits() != w1.player_f32(p, 0x3ee0).to_bits() {
                bad.push(format!("{at}: scale {scale} want {}", w1.player_f32(p, 0x3ee0)));
                continue;
            }
            let (sx, sz) = smash_scatter(&d.stats[c], grade, bias, w1.player_f32(p, 0x3f44), nudge, scale, hold);
            let aim = [0x3e90, 0x3e94, 0x3e98].map(|o| w1.player_f32(p, o));
            let pulled = m.inside(3, kind, false, w1.global(0x422fa4) >= 3, c, false, hit, aim);
            let sc = hst_sim::serve::scatter_along(sx, sz, hit, pulled);
            let to = [hst_sim::ps2::add(pulled[0], sc[0]), pulled[1], hst_sim::ps2::add(pulled[2], sc[2])];
            let l = lookup(&table(c, kind), &Bounds::smash(kind), [hst_sim::ps2::sub(hit[0], sc[0]), hit[1], hst_sim::ps2::sub(hit[2], sc[2])], pulled);
            let v = launch(hit, to, l.elevation, l.speed);
            if v != vel || l.frames + 1 != i(b, 0x260) || to != target {
                bad.push(format!("{at}: grade {grade} bias {bias} offset {offset} nudge {nudge} held {hold} scale {scale} scatter {sx},{sz}: target {to:?} want {target:?}, vel {v:?} want {vel:?}, frames {} want {}", l.frames + 1, i(b, 0x260)));
            }
            seen += 1;
            scattered += (sx != 0.0 || sz != 0.0) as usize;
            held += (hold != 0.0) as usize;
            scaled += (scale != 1.0) as usize;
        }
    }
    for b in &bad {
        eprintln!("{b}");
    }
    eprintln!("{seen} smashes, {scattered} scattered, {held} held, {scaled} scaled, {} wrong", bad.len());
    assert!(bad.is_empty() && (seen == 0 || scattered > 0), "{} of {seen} smashes wrong", bad.len());
}
