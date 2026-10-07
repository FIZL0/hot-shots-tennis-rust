//! Every stroke decision of the slot-5 doubles match (`context/fixtures/match_s05.bin`) through the ported
//! contact search: same frame, branch, swing side and contact ball as the game. The players' animation-measured
//! reach values come from the slot-5 save state's RAM (`context/fixtures/slot5_ee.bin`).

use hst_data::{ani, iso::Iso, xb::Archive};
use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};
use hst_sim::ps2::{add, mul};
use hst_sim::replay::{Frame, frames_live};
use hst_sim::player::{Stats, drain, mover, run_speed};
use hst_sim::swing::{Branch, DIVE_HORIZON, PathPoint, Reach, approach, dive, search};

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
    contact_search("match_s05.bin", "slot5_ee.bin");
}

/// The doubles match with one human (Carol, left-handed) and three CPUs on court 11 (`1p3goodcpus.bin`), with its
/// own save state's RAM: lobs, the smashes off them and the human's swings included.
#[test]
fn goodcpus_contact_search() {
    let n = contact_search("1p3goodcpus.bin", "1p3goodcpus_ee.bin");
    // 12 of them the human's, and the four smashes off the lobs (vsync 19649 p0, 20767, 21200, 21340 p3)
    assert!(n == 0 || n == 38, "{n} decisions");
}

/// The slot-5 bot game again with every smash turned into a △ smash (`tools/record_lob_smash.py`): the returns
/// of those lob smashes, and the △ smash off a lob, are searched like any other ball.
#[test]
fn lob_smash_contact_search() {
    contact_search("lob_smash_s05.bin", "slot5_ee.bin");
}

/// The recorded reach of player `p` (as the game holds it at frame `fr`); `obj` its object in the save state's RAM.
fn reach_of(fr: Frame, p: usize, ram: &[u8], obj: usize) -> Reach {
    let rf = |o: usize| f32::from_le_bytes(ram[obj + o..obj + o + 4].try_into().unwrap());
    let look = p_i32(fr, p, 0x154c) as usize;
    Reach {
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
    }
}

/// A player's movement stats as the game parsed them from TParam.csv into its object (`obj` in `ram`): speed
/// +0x1374 (SPE / 10), stamina +0x1378, dive/backhand/smash costs +0x137c.., agility +0x1388.
fn stats_of(ram: &[u8], obj: usize) -> Stats {
    let ri = |o: usize| i32::from_le_bytes(ram[obj + o..obj + o + 4].try_into().unwrap());
    Stats { speed: f32::from_bits(ri(0x1374) as u32), agility: ri(0x1388), stamina: ri(0x1378), dive: ri(0x137c), backhand: ri(0x1380), smash: ri(0x1384) }
}

/// Every approach run of the slot-5 match (a press with the ball out of reach: the player runs square to the
/// ball's line first, +0x3f24 set): same frame count as the game and the same direction (3 of 4 bit-exact).
#[test]
fn match_s05_approaches() {
    let Some(seen) = approaches("match_s05.bin", "slot5_ee.bin") else { return };
    assert!(seen >= 4);
}

/// The same in the doubles match with one human and three CPUs on court 11 (`1p3goodcpus.bin`).
#[test]
fn goodcpus_approaches() {
    let Some(seen) = approaches("1p3goodcpus.bin", "1p3goodcpus_ee.bin") else { return };
    assert_eq!(seen, 4);
}

/// How many approach runs `name` holds (None when it or `ram_name` is absent); panics unless all match.
fn approaches(name: &str, ram_name: &str) -> Option<usize> {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(ram)) = (std::fs::read(format!("{dir}/{name}")), std::fs::read(format!("{dir}/{ram_name}"))) else {
        eprintln!("{name} or {ram_name} absent, skipped");
        return None;
    };
    let ru = |a: usize| u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize;
    let gm = ru(0x422f80);
    let frames = frames_live(&data);
    let court = &COURTS[frames[0].global(0x422f90) as usize];
    let (mut seen, mut bad) = (0, 0);
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        for p in 0..4 {
            if !(p_u8(fr, p, 0x3f24) == 1 && p_u8(a, p, 0x3f24) == 0) {
                continue;
            }
            let reach = reach_of(fr, p, &ram, ru(gm + 0xa8 + 4 * p));
            let pos = a.player_pos(p);
            let facing = if pos[2] < 0.0 { 1.0 } else { -1.0 };
            let (mut fl, shot) = load(fr.live_ball());
            let mut path = vec![];
            for _ in 0..reach.grades.len() + 20 {
                path.push(PathPoint { pos: fl.ball.pos, bounces: fl.bounces });
                fl.step(&shot, court);
            }
            let s = stats_of(&ram, ru(gm + 0xa8 + 4 * p));
            let running = p_u8(a, p, 0x3fa5) == 1;
            let (mut stamina, mut tick, mut run) = (p_i32(a, p, 0x3df4), p_i32(a, p, 0x3df8), if running { p_i32(a, p, 0x3dfc) } else { 0 });
            let mate = a.player_pos(p ^ 2);
            let got = approach(&reach, &path, pos, facing, |at, d| {
                let speed = run_speed(&s, run, stamina, 100);
                let delta = [mul(d[0], speed), 0.0, mul(d[1], speed)];
                let to = mover([at[0], pos[1], at[1]], delta, facing, Some(mate), false);
                run += 1;
                (stamina, tick) = drain(&s, stamina, tick, 4, true, 0);
                (to[0] == add(at[0], delta[0]) && to[2] == add(at[1], delta[2])).then_some([to[0], to[2]])
            });
            let want = (p_i32(fr, p, 0x3f28) as usize + 1, [fr.player_f32(p, 0x3f30), fr.player_f32(p, 0x3f38)]);
            seen += 1;
            // the game steps its own predicted path, which can drift ~1e-7 from this flight (vsync 29934's end ball)
            let close = got.is_some_and(|(n, d)| n == want.0 && (0..2).all(|j| (d[j] - want.1[j]).abs() < 1e-6));
            if !close {
                bad += 1;
                eprintln!("vsync {} p{p}: want {want:?} got {got:?}", fr.vsync());
            }
        }
    }
    eprintln!("{seen} approaches, {bad} off");
    assert_eq!(bad, 0);
    Some(seen)
}

/// Every stroke decision of `name` through the contact search, with the reach from its save state's RAM (`ram_name`).
fn contact_search(name: &str, ram_name: &str) -> usize {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(ram)) = (std::fs::read(format!("{dir}/{name}")), std::fs::read(format!("{dir}/{ram_name}"))) else {
        eprintln!("{name} or {ram_name} absent, skipped");
        return 0;
    };
    let ru = |a: usize| u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize;
    let gm = ru(0x422f80);
    let frames = frames_live(&data);
    let court = &COURTS[frames[0].global(0x422f90) as usize];
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
                fl.step(&shot, court);
            }
            let got = search(&reach, &path, pos, facing, 0);
            let want_ball = [fr.player_f32(p, 0x3f40), fr.player_f32(p, 0x3f44), fr.player_f32(p, 0x3f48)];
            let want = (idx, branch, p_i32(fr, p, 0x3f50));
            // flags: 1 forehand / 2 backhand, | 4 body shot (smash: none)
            let g = got.map(|s| {
                let flags = if s.branch == Branch::Smash { 3 } else { (if s.forehand { 1 } else { 2 }) | (if s.body { 4 } else { 0 }) };
                (s.frame as i32, match s.branch { Branch::Ground => 1u8, Branch::Volley => 2, Branch::Smash => 4 }, flags)
            });
            // the game's own prediction (+0x3f40) can drift ~1e-5 from its flight; the ball where the swing meets it
            // (lob_smash vsync 9309) is then matched exactly instead
            let real = frames.get(k + idx as usize).map(|h| {
                let b = h.live_ball();
                [0x120, 0x128].map(|o| f32::from_le_bytes(b[o..o + 4].try_into().unwrap()))
            });
            let ok = g == Some(want)
                && got.is_some_and(|s| {
                    (s.ball[0], s.ball[2]) == (want_ball[0], want_ball[2])
                        || ((s.ball[0] - want_ball[0]).abs() < 1e-5 && (s.ball[2] - want_ball[2]).abs() < 1e-5)
                        || real == Some([s.ball[0], s.ball[2]])
                });
            if !ok {
                eprintln!("vsync {} p{p}: want {want:?} ball {want_ball:?} got {g:?} {:?}", fr.vsync(), got.map(|s| s.ball));
            }
            checked += 1;
            misses += !ok as i32;
        }
    }
    eprintln!("{checked} decisions, {misses} misses");
    assert_eq!(misses, 0);
    checked
}

/// Every dive of the live recordings (new_recording.bin, match_s05.bin, lob_smash_s05.bin, human_smash_s04.bin
/// P7b's p7b_cNN.bin runs, P7e's p7e_c03/07.bin and 1p3goodcpus.bin): the dive search picks the game's frame, kind and slide, and the body's slide and
/// recovery follow each player's own character's receive motion root path bit-exact. Characters aren't recorded:
/// each player's is found from its TParam record copy (+0x13a8..), and where several characters share the record
/// the test needs them to share the root path too.
#[test]
fn recorded_dives() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc absent, skipped");
    };
    let roots: Vec<hst_sim::pose::Path> = (0..14)
        .map(|c| {
            let xb = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
            let arc = Archive::parse(&xb).unwrap();
            let stem = ani::motion_name(51, c).unwrap().to_ascii_lowercase();
            let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).unwrap();
            hst_sim::pose::Path::new(&ani::parse(&arc.read(e).unwrap()).unwrap()).unwrap()
        })
        .collect();
    let data = iso.read("PCDATA/PCDATA.XB").unwrap();
    let arc = Archive::parse(&data).unwrap();
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).unwrap();
    let csv = String::from_utf8_lossy(&arc.read(e).unwrap()).into_owned();
    let records: Vec<Vec<u32>> = (0..14)
        .map(|n| {
            let s = hst_sim::player::ReachStats::from_tparam(csv.lines().find(|l| l.starts_with(&format!("{n},"))).unwrap());
            [[s.serve_scatter, s.base, s.reach, s.under_min, s.stroke_height, s.volley_height].as_slice(), &s.smash, &s.serve, &s.under_serve, &[s.dive_start, s.dive_limit, s.collision]]
                .concat()
                .iter()
                .map(|v| v.to_bits())
                .collect()
        })
        .collect();
    let mut files: Vec<String> = ["new_recording.bin", "match_s05.bin", "lob_smash_s05.bin", "human_smash_s04.bin"].map(String::from).to_vec();
    files.extend((0..14).map(|c| format!("p7b_c{c:02}.bin")));
    // Kaito and Bull, whose root paths are their own, by research/p7e_dive_record.py (P1 dives on purpose)
    files.extend(["p7e_c03.bin", "p7e_c07.bin"].map(String::from));
    // the doubles match with one human (Carol, left-handed) and three CPUs on court 11
    files.push("1p3goodcpus.bin".into());
    let (mut dives, mut steps, mut skipped, mut chars) = (0, 0, 0, std::collections::BTreeSet::new());
    for file in &files {
        let Ok(data) = std::fs::read(format!("{dir}/{file}")) else {
            eprintln!("{file} absent, skipped");
            continue;
        };
        let frames = frames_live(&data);
        // a player's character: those whose record its copy equals, all with one root path
        let mut root = |fr: Frame, p: usize| {
                let rec: Vec<u32> = (0x13a8..0x13f0).step_by(4).map(|o| fr.player_f32(p, o).to_bits()).collect();
                let cs: Vec<usize> = (0..14).filter(|&c| records[c] == rec).collect();
                assert!(!cs.is_empty(), "{file} p{p}: no character's record {:?} best {:?}", rec.iter().map(|&b| f32::from_bits(b)).collect::<Vec<_>>(), (0..14).map(|c| records[c].iter().zip(&rec).filter(|(a, b)| a != b).count()).collect::<Vec<_>>());
                let same = |c: usize| (0..80).all(|t| roots[c].at(t as f32)[2].to_bits() == roots[cs[0]].at(t as f32)[2].to_bits());
                assert!(cs.iter().all(|&c| same(c)), "{file} p{p}: characters {cs:?} differ in root path");
                chars.insert(cs[0]);
                &roots[cs[0]]
        };
        for k in 1..frames.len().saturating_sub(DIVE_HORIZON) {
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
                let at = format!("{file} vsync {} p{p}", fr.vsync());
                // the game's predictor flies through the net: a ball that meets it (a bounce at the net) in the
                // horizon was searched on a path not recorded (human_smash_s04 vsync 15054)
                if path.windows(2).any(|w| w[1].bounces > w[0].bounces && w[1].pos[2].abs() < 0.5) {
                    eprintln!("{file} vsync {} p{p}: ball meets the net, skipped", fr.vsync());
                    skipped += 1;
                    continue;
                }
                let d = dive(&reach, &path, pos, end, face, vel).unwrap_or_else(|| panic!("{at}: no dive"));
                let contact = p_i32(fr, p, 0x3ec4) >= 0;
                let want = (p_i32(fr, p, 0x3f84) - 50, contact);
                assert_eq!((d.frame as i32, d.contact), want, "{at}");
                // ponytail: a contact dive's arm slide needs the character's receive-pose shoulder and racket tip
                // (zero here); its recorded slide drives the body instead
                if !contact {
                    assert_eq!(d.slide, fr.player_f32(p, 0x3f80), "{at}");
                }
                let want_dir = [fr.player_f32(p, 0x3e60), fr.player_f32(p, 0x3e68)];
                assert!((d.dir[0] - want_dir[0]).abs() < 1e-5 && (d.dir[1] - want_dir[1]).abs() < 1e-5, "{at}: dir {:?} want {want_dir:?}", d.dir);
                dives += 1;
                let root = root(fr, p);
                // from the game's direction and slide (the path's ulps aside)
                let (mut d, mut at_) = (d, [pos[0], pos[2]]);
                d.dir = want_dir;
                d.slide = fr.player_f32(p, 0x3f80);
                // the recorded dive counter (+0x3f88) is n + 1 after frame n; it stalls when the game pauses
                let mut n = 0;
                while k + n < frames.len() && p_i32(frames[k + n], p, 0x3f88) == n as i32 + 1 {
                    let q = d.step(at_, |t| root.at(t)[2], |m| {
                        // the partner (p ^ 2) as of this player's update: players update in order
                        let mate = if p < 2 { frames[k + n - 1].player_pos(p ^ 2) } else { frames[k + n].player_pos(p ^ 2) };
                        let q = mover([at_[0], 0.0, at_[1]], [m[0], 0.0, m[1]], end, Some(mate), false);
                        [q[0], q[2]]
                    });
                    at_ = q.expect("dive over early");
                    let rec = frames[k + n].player_pos(p);
                    assert_eq!(at_, [rec[0], rec[2]], "{at} frame {n} of the dive");
                    steps += 1;
                    n += 1;
                }
                assert!(n == d.len() || n > 40 || k + n == frames.len(), "{at}: {n} frames");
            }
        }
    }
    eprintln!("{dives} dives, {steps} dive frames, {skipped} skipped, characters {chars:?}");
    assert!(skipped <= 1);
}

/// Each △ smash of `lob_smash_s05.bin` flies as the game's ball from its launch to its first bounce (bounces
/// in these live recordings all land ~1e-6 off, strokes too: see research/journal/2026-10-07-n5a-lob-off-lob).
#[test]
fn lob_smash_flights() {
    let Some(flown) = smash_flights("lob_smash_s05.bin") else { return };
    // the first is volleyed before it bounces
    assert_eq!(flown, [44, 67, 73]);
}

/// The four smashes off lobs in the doubles match with one human and three CPUs on court 11 (`1p3goodcpus.bin`):
/// Carol's ✕ smash and character 10's three △ smashes.
#[test]
fn goodcpus_smash_flights() {
    let Some(flown) = smash_flights("1p3goodcpus.bin") else { return };
    // the first is volleyed before it bounces
    assert_eq!(flown, [24, 75, 68, 62]);
}

/// Frames each smash launched in `name` flies as the game's ball before its first bounce (None when absent).
fn smash_flights(name: &str) -> Option<Vec<usize>> {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/{name}")) else {
        eprintln!("{name} absent, skipped");
        return None;
    };
    let frames = frames_live(&data);
    let court = &COURTS[frames[0].global(0x422f90) as usize];
    let mut flown = vec![];
    for k in 0..frames.len() {
        let b = frames[k].live_ball();
        // class 3 also marks the held and tossed serve ball
        if b[0x58] != 3 || i(b, 0xac) != 0 || i(b, 0x260) == 0 {
            continue;
        }
        let (mut fl, shot) = load(b);
        let mut j = k + 1;
        while j < frames.len() && i(frames[j].live_ball(), 0xac) != 0 {
            fl.step(&shot, court);
            if fl.bounces != 0 {
                break;
            }
            let want = frames[j].live_ball();
            assert_eq!((fl.ball.pos, fl.ball.vel), (v3(want, 0xe0), v3(want, 0x130)), "vsync {} (launched {})", frames[j].vsync(), frames[k].vsync());
            j += 1;
        }
        flown.push(j - k - 1);
    }
    Some(flown)
}
