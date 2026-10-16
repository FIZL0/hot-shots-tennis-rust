//! The post-point cut-away shots against a recording of the original (`context/fixtures/cutaway_s05.bin`,
//! `tools/record_cutaway.py`): shots and scripts from the disc's GAME.BIN, subject frames from the recorded
//! reference table; every frame's channels and view (eye, axes, field of view) follow the game's.

use hst_data::{exe::Game, iso::Iso};
use hst_sim::cutaway::{M4, Shot};

const GM: usize = 4;
const CH: usize = GM + 0x100;
const CAM: usize = CH + 0x100;
const TAB: usize = CAM + 0x3100;
const SAMPLE: usize = TAB + 0x1800 + 0x200;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn mat(b: &[u8], o: usize) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| f(b, o + 16 * r + 4 * c)))
}

#[test]
fn cutaway_s05() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(mut iso)) = (std::fs::read(format!("{dir}/cutaway_s05.bin")), Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"))) else {
        return eprintln!("cutaway_s05.bin or ISO absent, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let shots = Game::new(&cnf, &bin).unwrap().camera_shots();
    let frames: Vec<&[u8]> = data.as_chunks::<SAMPLE>().0.iter().map(|s| s.as_slice()).collect();
    let vsync = |s: &[u8]| u32::from_le_bytes(s[0..4].try_into().unwrap());
    let (mut shot, mut prev): (Option<Shot>, Option<Shot>) = (None, None);
    let (mut n, mut worst, mut stale, mut court) = (0, 0.0f32, 0, 0.0f32);
    for (k, s) in frames.iter().enumerate() {
        let number = s[CAM + 0x114];
        let new_run = k == 0 || vsync(s) != vsync(frames[k - 1]) + 1 || number != frames[k - 1][CAM + 0x114];
        let table = |i: u8| mat(s, TAB + i as usize * 0x40);
        let blank = hst_sim::cutaway::Frames { corner: table(6), shown: [[table(29); 4]; 2], head: table(35), spot: table(19)[3] };
        assert_eq!(blank.get(0), table(0), "frame 0 is the port's court centre");
        if new_run {
            // the recording starts a frame or two into the shot: catch up on the yaw channel
            let mut sh = Shot::new(&shots, number, s[CAM + 0x50] == 0xff);
            for _ in 0..8 {
                if (sh.ch(6) - f(s, CAM + 0x2d58 + 6 * 0x1c)).abs() < 1e-5 {
                    break;
                }
                sh.step(table);
            }
            sh.build(&table);
            shot = Some(sh);
        } else if let Some(sh) = &mut shot {
            prev = Some(sh.clone());
            // the game's frame counter: stalled frames don't advance the shot
            let ticks = u32::from_le_bytes(s[GM + 0x50..GM + 0x54].try_into().unwrap()).wrapping_sub(u32::from_le_bytes(frames[k - 1][GM + 0x50..GM + 0x54].try_into().unwrap()));
            for _ in 0..ticks {
                sh.step(table);
            }
        }
        let sh = shot.as_ref().unwrap();
        // a sample read between the frame counter's tick and the camera's update: the channels are last frame's
        let near = |sh: &Shot| [5, 6, 11, 13, 14].iter().all(|&c| (sh.ch(c) - f(s, CAM + 0x2d58 + c * 0x1c)).abs() < 1e-4);
        if !new_run && !near(sh) && prev.as_ref().is_some_and(near) {
            stale += 1;
            continue;
        }
        for c in [5, 6, 11, 13, 14] {
            let want = f(s, CAM + 0x2d58 + c * 0x1c);
            assert!((sh.ch(c) - want).abs() < 1e-4, "frame {k} shot {number:#x} channel {c}: {} vs {want}", sh.ch(c));
        }
        let v = sh.view();
        let want: Vec<f32> = (0..12).map(|i| f(s, CAM + 0x60 + 16 * (i / 3) + 4 * (i % 3))).collect();
        let got: Vec<f32> = v.rot.iter().chain(std::iter::once(&v.eye)).flatten().copied().collect();
        let err = got.iter().zip(&want).map(|(a, b)| (a - b).abs()).fold((v.fov - f(s, CAM + 0xa4)).abs(), f32::max);
        if matches!(number, 0x0c | 0x0d | 0x11) { court = court.max(err); }
        if err > worst {
            worst = err;
            eprintln!("frame {k} shot {number:#x}: err {err:.6}");
        }
        n += 1;
    }
    eprintln!("{n} frames checked, worst {worst} (court views {court}); {stale} stale samples");
    assert!(n > 1000 && stale < 5 && worst < 1e-3, "worst {worst}");
}

#[test]
#[ignore]
fn debug_frame() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures");
    let data = std::fs::read(format!("{dir}/cutaway_s05.bin")).unwrap();
    let mut iso = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")).unwrap();
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let shots = Game::new(&cnf, &bin).unwrap().camera_shots();
    let k: usize = std::env::var("K").unwrap().parse().unwrap();
    let s = data.as_chunks::<SAMPLE>().0.get(k).unwrap();
    let number = s[CAM + 0x114];
    let table = |i: u8| mat(s, TAB + i as usize * 0x40);
    let mut sh = Shot::new(&shots, number, s[CAM + 0x50] == 0xff);
    eprintln!("script {:?}", shots[number as usize].script);
    for _ in 0..400 {
        if (sh.ch(6) - f(s, CAM + 0x2d58 + 6 * 0x1c)).abs() < 1e-5 { break; }
        sh.step(table);
    }
    sh.build(&table);
    for c in 5..17 { eprintln!("ch{c} ours {} game {}", sh.ch(c), f(s, CAM + 0x2d58 + c * 0x1c)); }
    eprintln!("ours {:?}\ngame {:?} fov {}", sh.view(), mat(s, CAM + 0x60), f(s, CAM + 0xa4));
    eprintln!("basis ours {:?}\n      game {:?}", sh.basis(&table), mat(s, CAM + 0x2f50));
    for i in [0x1d, 0x1e, 0x2e, 0x3e] { eprintln!("frame {i:#x} {:?}", table(i)[3]); }
}

/// The director against `cutaway2_s05.bin` (`record_cutaway.py --players`: the same samples plus the point's
/// globals, the pick counters and each player's head/Spine2 bones, predicted head, matrix, reaction, motion):
/// every cut-away's shot pick and orbit side, the shown players' frames from their bones posed where the reaction
/// leaves them (the motion runs 117 frames into the cut-away, then holds), the corner view and the live head frame.
#[test]
fn director_s05() {
    use hst_sim::cutaway::{self, PickInput};
    const X: usize = SAMPLE;
    const PL: usize = X + 0xd8;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(mut iso)) = (std::fs::read(format!("{dir}/cutaway2_s05.bin")), Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"))) else {
        return eprintln!("cutaway2_s05.bin or ISO absent, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let lists = Game::new(&cnf, &bin).unwrap().cutaway_lists();
    let i32_ = |s: &[u8], o: usize| i32::from_le_bytes(s[o..o + 4].try_into().unwrap());
    let samples: Vec<&[u8]> = data.as_chunks::<{ PL + 4 * 0x170 }>().0.iter().map(|s| s.as_slice()).collect();
    let mut runs = vec![0];
    for k in 1..samples.len() {
        if i32_(samples[k], 0) != i32_(samples[k - 1], 0) + 1 || samples[k][CAM + 0x114] != samples[k - 1][CAM + 0x114] {
            runs.push(k);
        }
    }
    runs.push(samples.len());
    let near = |a: &M4, b: &M4, what: &str| {
        let e = (0..16).map(|i| (a[i / 4][i % 4] - b[i / 4][i % 4]).abs()).fold(0.0, f32::max);
        assert!(e < 1e-4, "{what}: {a:?} vs {b:?}");
    };
    let (mut counters, mut mirror, mut posed) = ([0; 3], true, 0);
    for (j, w) in runs.windows(2).enumerate() {
        let (s, last) = (samples[w[0]], samples[w[1] - 1]);
        let pl = |i: usize| PL + 0x170 * i;
        // who: from the roles (last hitter, the one before), the last hitter, game end and the point's winner
        let ab = [i32_(s, TAB + 0x1800 + 0x198) as usize, i32_(s, TAB + 0x1800 + 0x19c) as usize];
        let [shown, other] = cutaway::roles(ab, i32_(s, X + 0x18), i32_(s, X + 0x78) > 0, i32_(s, X + 0x68), i32_(s, X + 0x98 + 0x18), |_| false);
        assert_eq!([shown, other], [0x1a0, 0x1a4].map(|o| i32_(s, TAB + 0x1800 + o) as usize), "cut-away {j} roles");
        let lost = shown as i32 & 1 != i32_(s, X + 0x68) & 1;
        let number = cutaway::pick(&lists, &mut counters, &PickInput {
            team: i32_(s, pl(shown) + 0x110) >= 0x30,
            game_end: i32_(s, X + 0x78) > 0,
            lost,
            low: mat(s, TAB + 61 * 0x40)[3][1] > -0.8,
            replays: i32_(s, X + 0x98 + 0x18),
            character: i32_(s, pl(shown) + 0x124),
        });
        assert_eq!(number, s[CAM + 0x114], "cut-away {j} shot");
        assert_eq!(counters.map(|c| c as i32), std::array::from_fn(|i| i32_(s, X + 0x80 + 8 * i)), "cut-away {j} counters");
        mirror = !mirror;
        if !matches!(number, 0x0c | 0x0d | 0x11) {
            assert_eq!(s[CAM + 0x50] == 0xff, mirror, "cut-away {j} side");
        }
        let spot = mat(s, TAB + 19 * 0x40)[3];
        near(&cutaway::corner_frame(spot), &mat(s, TAB + 6 * 0x40), "corner");
        // the shown pair's frames, once the reaction has reached the predicted pose (the head at +0x17e0)
        let held = [shown, other].iter().all(|&i| (0..3).all(|c| (f(last, pl(i) + 0x30 + 4 * c) - f(s, pl(i) + 0x80 + 4 * c)).abs() < 1e-5));
        if held {
            for (k, &i) in [shown, other].iter().enumerate() {
                let fr = cutaway::subject_frames(&mat(last, pl(i)), &mat(last, pl(i) + 0x40));
                for (n, e) in [29, 45, 61, 77].into_iter().enumerate() {
                    near(&fr[n], &mat(s, TAB + (e + k) * 0x40), &format!("cut-away {j} frame {}", e + k));
                }
            }
            posed += 1;
        }
        // the live head frame
        for &t in &samples[w[0]..w[1]] {
            let a = i32_(t, TAB + 0x1800 + 0x198) as usize;
            let left = cutaway::head_frame(&mat(t, pl(a)), true);
            let right = cutaway::head_frame(&mat(t, pl(a)), false);
            let want = mat(t, TAB + 35 * 0x40);
            let e = |m: &M4| (0..16).map(|i| (m[i / 4][i % 4] - want[i / 4][i % 4]).abs()).fold(0.0, f32::max);
            assert!(e(&left).min(e(&right)) < 1e-4, "cut-away {j} head frame");
        }
    }
    eprintln!("{} cut-aways, {posed} with the pose held", runs.len() - 1);
    assert!(runs.len() > 10 && posed > 8);
}

/// The director's prediction against `cutaway2_s05.bin`: each cut-away's shown and other player posed from the
/// disc (their reaction at 117 frames, 378 on a game-ending point, clamped to its length; `gu_set` and the team
/// reactions moved along their root path from where they began, `cutaway::reaction_spot`) give the recorded
/// subject frames 29/45/61/77 (+1): the game-ending point's `gu_set` included, whose head once came out with w = 2
/// and turned the shot away from the winner.
#[test]
fn prediction_s05() {
    type Anims = std::collections::HashMap<usize, (ani::Anim, Option<Path>)>;
    use hst_data::{ani, mdl, xb::Archive};
    use hst_sim::cutaway;
    use hst_sim::pose::{Clip, Path, Skeleton, node_world};
    const X: usize = SAMPLE;
    const PL: usize = X + 0xd8;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(mut iso)) = (std::fs::read(format!("{dir}/cutaway2_s05.bin")), Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"))) else {
        return eprintln!("cutaway2_s05.bin or ISO absent, skipped");
    };
    let i32_ = |s: &[u8], o: usize| i32::from_le_bytes(s[o..o + 4].try_into().unwrap());
    let v4 = |s: &[u8], o: usize| [0, 4, 8, 12].map(|d| f(s, o + d));
    let samples: Vec<&[u8]> = data.as_chunks::<{ PL + 4 * 0x170 }>().0.iter().map(|s| s.as_slice()).collect();
    let cg_data = iso.read("PCDATA/PCCG0.XB").unwrap();
    let cg = Archive::parse(&cg_data).unwrap();
    let team = ["co01_f", "co02_f", "co03", "co04", "co05"];
    let cg_anim = |name: String| cg.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(&name)).map(|e| ani::parse(&cg.read(e).unwrap()).unwrap());
    // per character: every costume's skeleton, and the reaction anims and root paths by motion number
    let mut chars = std::collections::HashMap::new();
    let (mut checked, mut game_ends) = (0, 0);
    for k in 0..samples.len() {
        let s = samples[k];
        if k > 0 && i32_(s, 0) == i32_(samples[k - 1], 0) + 1 && s[CAM + 0x114] == samples[k - 1][CAM + 0x114] {
            continue;
        }
        let game_end = i32_(s, X + 0x78) > 0;
        game_ends += game_end as usize;
        let t = if game_end { 378.0 } else { 117.0 };
        for (n, o) in [0x1a0, 0x1a4].into_iter().enumerate() {
            let i = i32_(s, TAB + 0x1800 + o) as usize;
            let pl = PL + 0x170 * i;
            let (c, m) = (i32_(s, pl + 0x124), i32_(s, pl + 0x110) as usize);
            if !(0x2c..0x35).contains(&m) {
                continue;
            }
            let (skeletons, anims): &(Vec<Skeleton>, Anims) = chars.entry(c).or_insert_with(|| {
                let sks = (0..10)
                    .filter_map(|costume| {
                        let d = iso.read(&format!("PC/PC{c:02}C{costume:02}.XB")).ok()?;
                        let a = Archive::parse(&d).unwrap();
                        let e = a.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl"))?;
                        let m = mdl::parse(&a.read(e).unwrap()).unwrap();
                        Some(Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() })
                    })
                    .collect();
                let d = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
                let arc = Archive::parse(&d).unwrap();
                let get = |m: usize| {
                    let stem = ani::motion_name(m, c as usize).unwrap().to_ascii_lowercase();
                    arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).map(|e| ani::parse(&arc.read(e).unwrap()).unwrap())
                };
                let mut anims = std::collections::HashMap::new();
                for m in 0x2c..0x30 {
                    let path = (m == 0x2e).then(|| get(0x35).and_then(|a| Path::new(&a))).flatten();
                    anims.insert(m, (get(m).unwrap(), path));
                }
                for (j, stem) in team.iter().enumerate() {
                    let path = cg_anim(format!("mtgrl/re_pc00_{stem}_dummy.ani2")).and_then(|a| Path::new(&a));
                    anims.insert(0x30 + j, (cg_anim(format!("mtgrl/re_pc00_{stem}.ani2")).unwrap(), path));
                }
                (sks, anims)
            });
            let (anim, path) = &anims[&m];
            let rows = [v4(s, pl + 0x90), v4(s, pl + 0xa0), v4(s, pl + 0xb0)];
            let base = v4(s, pl + 0xe0);
            // the costume isn't recorded: the one whose skeleton fits best
            let best = skeletons
                .iter()
                .map(|sk| {
                    let clip = Clip::new(sk, anim);
                    let len = anim.end_tick() as f32 / anim.ticks_per_frame as f32;
                    let pos = match path {
                        Some(p) => cutaway::reaction_spot(p.at(f32::min(t, len)), m, c, rows, base),
                        None => [base[0], base[1], base[2], 1.0],
                    };
                    let player = [rows[0], rows[1], rows[2], pos];
                    let locals = clip.locals(sk, f32::min(t, clip.length));
                    let bone = |name: &str| node_world(sk, &locals, sk.names.iter().position(|x| x == name).unwrap(), &player);
                    let fr = cutaway::subject_frames(&bone("Bip01Head"), &bone("Bip01Spine2"));
                    [29, 45, 61, 77].into_iter().enumerate().flat_map(|(q, e)| (0..16).map(move |x| (q, e, x))).map(|(q, e, x)| (fr[q][x / 4][x % 4] - mat(s, TAB + (e + n) * 0x40)[x / 4][x % 4]).abs()).fold(0.0, f32::max)
                })
                .fold(f32::MAX, f32::min);
            assert!(best < 1e-4, "cut-away at sample {k} (game end {game_end}) player {i} motion {m:#x}: frames off by {best}");
            checked += 1;
        }
    }
    eprintln!("{checked} predicted subjects match, {game_ends} game-ending cut-aways");
    assert!(checked > 15 && game_ends > 0);
}

/// B367: shot 0x64 films the shown player from the ground 2 m in front (its eye sits on its basis). Its
/// ground clear tilts toward the subject frame's spot, so the view stays finite, 0.1 m up, with the player's head
/// in the picture (it used to take the asin of a zero-length look, and the view's axes blew up to ~1e38).
#[test]
fn low_angle_shot_stays_finite() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("ISO absent, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let shots = Game::new(&cnf, &bin).unwrap().camera_shots();
    // a stock character standing at the near baseline, facing the net (−z), as the motions pose its head and Spine2
    let head: M4 = [[-0.01, 0.97, -0.22, 0.0], [1.0, 0.01, 0.0, 0.0], [0.0, -0.22, -0.97, 0.0], [0.0, -1.05, 11.0, 1.0]];
    let spine: M4 = [[0.0, 1.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [0.0, 0.0, -1.0, 0.0], [0.0, -0.6, 11.0, 1.0]];
    let s = hst_sim::cutaway::subject_frames(&head, &spine);
    let frames = hst_sim::cutaway::Frames { corner: s[0], shown: [s, s], head: hst_sim::cutaway::head_frame(&head, false), spot: s[0][3] };
    for mirror in [false, true] {
        let mut sh = Shot::new(&shots, 0x64, mirror);
        for t in 0..120 {
            sh.step(|i| frames.get(i));
            let v = sh.view();
            assert!(v.rot.iter().flatten().chain(&v.eye).all(|x| x.is_finite() && x.abs() < 100.0), "frame {t}: {v:?}");
            assert!((v.eye[1] + 0.1).abs() < 1e-3, "frame {t}: eye {:?} not 0.1 m up", v.eye);
            let d = [0.0 - v.eye[0], -1.05 - v.eye[1], 11.0 - v.eye[2]];
            let l = v.rot.map(|r| r[0] * d[0] + r[1] * d[1] + r[2] * d[2]);
            assert!(l[2] > 0.0 && (l[0] / l[2]).abs() < v.fov.tan() && (l[1] / l[2]).abs() < v.fov.tan(), "frame {t}: head out of the picture {l:?}");
        }
    }
}
