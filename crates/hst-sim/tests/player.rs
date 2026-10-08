//! Locomotion of every player through two doubles matches, frame by frame: run velocity bit-exact and the motion the
//! game sets for standing and running players. Slot 5 (`context/fixtures/match_s05.bin`, court 10): characters 0, 2,
//! 1, 5, all right-handed CPUs. `1p3goodcpus.bin` (court 11): human Carol (6) and CPUs Will (11), 2
//! and 10; Carol and Will left-handed. Each line-up from its save state's RAM.

use hst_sim::player::{Body, Facing, Scene, Stats, pad_dir, mover, stroke_stamina, turn, StanceInput, angle, base_motion, dashing, run_motion, run_speed, run_velocity, stance, stand_motion, with_tiredness};
use hst_sim::replay::{Frame, frames_live};

fn p_u8(fr: Frame, p: usize, off: usize) -> u8 {
    fr.player_f32(p, off & !3).to_bits().to_le_bytes()[off & 3]
}
fn p_i32(fr: Frame, p: usize, off: usize) -> i32 {
    fr.player_f32(p, off).to_bits() as i32
}
fn p_v3(fr: Frame, p: usize, off: usize) -> [f32; 3] {
    [0, 4, 8].map(|o| fr.player_f32(p, off + o))
}
fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

/// The match recordings and their save states' RAM.
const MATCHES: [(&str, &str); 2] = [("match_s05.bin", "slot5_ee.bin"), ("1p3goodcpus.bin", "1p3goodcpus_ee.bin")];

/// A player as its save state's RAM has it: TParam's stats as the game copied them (+0x1374 speed, +0x1378 stamina,
/// +0x137c.. dive/backhand/smash costs, +0x1388 agility, already ×1.5 in weather 2 and 3), hand +0x12b4 and the
/// turn's per-motion pelvis rows (+0x6b0 + motion·0x40).
struct Player {
    s: Stats,
    hand: f32,
    pelvis: Vec<[f32; 2]>,
}

/// Every match present: its name, recording and line-up.
fn matches() -> Vec<(&'static str, Vec<u8>, Vec<Player>)> {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    MATCHES
        .iter()
        .filter_map(|&(name, ram)| {
            let (Ok(data), Ok(ram)) = (std::fs::read(format!("{dir}/{name}")), std::fs::read(format!("{dir}/{ram}"))) else {
                eprintln!("{name} or {ram} absent, skipped");
                return None;
            };
            let ru = |a: usize| u32::from_le_bytes(ram[a & 0x1ff_ffff..][..4].try_into().unwrap()) as usize;
            let i = |a: usize| ru(a) as i32;
            let players = (0..4)
                .map(|p| {
                    let pl = ru(ru(0x422f80) + 0xa8 + 4 * p);
                    let s = Stats { speed: f(&ram, pl + 0x1374), agility: i(pl + 0x1388), stamina: i(pl + 0x1378), dive: i(pl + 0x137c), backhand: i(pl + 0x1380), smash: i(pl + 0x1384) };
                    let pelvis = (0..48).map(|m| [f(&ram, pl + 0x6b0 + m * 0x40 + 0x20), f(&ram, pl + 0x6b0 + m * 0x40 + 0x28)]).collect();
                    Player { s, hand: f(&ram, pl + 0x12b4), pelvis }
                })
                .collect();
            Some((name, data, players))
        })
        .collect()
}

#[test]
fn match_locomotion() {
    for (name, data, lineup) in matches() {
        let frames = frames_live(&data);
        let (mut speeds, mut motions, mut bad_speed, mut bad_motion, mut moves, mut bad_move, mut pushes) = (0, 0, 0, 0, 0, 0, 0);
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            // the recording's tail (frame 26108 on) runs the game several ticks per sample (phase timer gm+0x58)
            if f(fr.gm(), 0x58).to_bits() as i32 - f(a.gm(), 0x58).to_bits() as i32 > 2 {
                break;
            }
            for p in 0..4 {
                let (s, hand) = (&lineup[p].s, lineup[p].hand);
                let mode = p_u8(fr, p, 0x3fa5);
                if mode > 1 || p_u8(a, p, 0x3fa5) > 1 {
                    continue;
                }
                // the player's own half is behind its forward
                let fwd = if fr.player_f32(p, 0x3d78) < 0.0 { 1.0 } else { -1.0 };
                let (run, stamina) = (p_i32(fr, p, 0x3dfc), p_i32(fr, p, 0x3df4));
                let target = p_v3(fr, p, 0x3dc0);
                // the turn toward the run direction ends (flag cleared) after the motion is chosen
                let turned = p_u8(fr, p, 0x3dd1) != 0 || p_u8(a, p, 0x3dd1) != 0;
                if mode == 1 && target != [0.0, 0.0, fwd] && target != [-0.0, -0.0, -fwd] {
                    let v = run_velocity(target, run_speed(s, run, stamina, 100));
                    let got = p_v3(fr, p, 0x3e00);
                    speeds += 1;
                    if v.map(f32::to_bits) != got.map(f32::to_bits) {
                        bad_speed += 1;
                        if bad_speed <= 10 {
                            eprintln!("{name} speed k={k} p={p} run={run} st={stamina}: {v:?} vs {got:?}");
                        }
                    }
                }
                let current = base_motion(p_i32(a, p, 0x3df0));
                let want = p_i32(fr, p, 0x3df0);
                // only frames where the game set a stand/run motion (when it does is the player state machine's call)
                if want >= 16 {
                    continue;
                }
                // the mover: partner push, step, bounds (the partner has moved already if it updates first)
                let mate = p ^ 2;
                let mate_pos = if mate < p { fr.player_pos(mate) } else { a.player_pos(mate) };
                let ticked = f(fr.gm(), 0x50).to_bits().wrapping_sub(f(a.gm(), 0x50).to_bits()) == 1 && fr.gm()[0x55] == a.gm()[0x55];
                let delta = if mode == 1 { p_v3(fr, p, 0x3e00) } else { [0.0; 3] };
                let moved = mover(a.player_pos(p), delta, fwd, Some(mate_pos), false);
                moves += ticked as i32;
                pushes += (ticked && mover(a.player_pos(p), delta, fwd, None, false) != moved) as i32;
                if ticked && [moved[0], moved[2]].map(f32::to_bits) != [fr.player_pos(p)[0], fr.player_pos(p)[2]].map(f32::to_bits) {
                    bad_move += 1;
                    if bad_move <= 20 {
                        eprintln!("{name} move k={k} p={p} mode={mode}: {moved:?} vs {:?} from {:?} mate {mate_pos:?}", fr.player_pos(p), a.player_pos(p));
                    }
                }
                let m = if mode == 1 {
                    let d = if turned { p_v3(a, p, 0x3d60) } else { target };
                    run_motion(current, angle(d, fwd, hand), dashing(s, run))
                } else {
                    let d = if turned { p_v3(a, p, 0x3d60) } else { [0.0, 0.0, fwd] };
                    // the hitter changes the ball during its own update: players before it still see the old one
                    let (old, new) = (a.global(0x423058), fr.global(0x423058));
                    let before = old != new && (p as i32) < new;
                    let ball = if before { a.live_ball() } else { fr.live_ball() };
                    stand_motion(angle(d, fwd, hand), || {
                        stance(&StanceInput {
                            players: 4,
                            phase: fr.gm()[0x55],
                            // players update in index order: the hitter sets it during its own update
                            last_hitter: if before { old } else { new },
                            team: p as i32,
                            current,
                            watching: true,
                            pos: fr.player_pos(p),
                            ball: [f(ball, 0xe0), f(ball, 0xe4), f(ball, 0xe8)],
                            ball_dir: [f(ball, 0x140), f(ball, 0x144), f(ball, 0x148)],
                            hand,
                        })
                    })
                };
                motions += 1;
                if with_tiredness(m, stamina) != want {
                    bad_motion += 1;
                    if bad_motion <= 10 {
                        eprintln!("{name} motion k={k} p={p} mode={mode} current={current} turned={turned}: {m} vs {want}");
                    }
                }
            }
        }
        eprintln!("{name}: speed {speeds} checked, {bad_speed} off; motion {motions} checked, {bad_motion} off; moves {moves} checked ({pushes} partner pushes), {bad_move} off");
        assert!(speeds > 300 && motions > 3000 && moves > 3000, "{name}");
        assert_eq!((bad_speed, bad_motion, bad_move), (0, 0, 0), "{name}");
    }
}

/// Every stamina drop at a stroke's contact (all but the running drain) is the stroke's cost.
#[test]
fn match_stroke_stamina() {
    for (name, data, lineup) in matches() {
        let frames = frames_live(&data);
        let mut n = 0;
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            for p in 0..4 {
                let (s, hand) = (&lineup[p].s, lineup[p].hand);
                let (before, after) = (p_i32(a, p, 0x3df4), p_i32(fr, p, 0x3df4));
                let drained = before - after == 1 && p_i32(fr, p, 0x3df8) == 0;
                if after >= before || drained {
                    continue;
                }
                // +0x3f50 bit 0: the ball on the right-hander's forehand side, bit 1 the left-hander's
                let forehand = p_i32(fr, p, 0x3f50) & (if hand < 0.0 { 2 } else { 1 }) != 0;
                assert_eq!(stroke_stamina(s, before, p_u8(fr, p, 0x3ec1), forehand, 0), after, "{name} k={k} p={p}");
                n += 1;
            }
        }
        eprintln!("{name}: {n} stroke costs");
        assert!(n > 10, "{name} {n}");
    }
}

/// The body's facing every frame: snap within 22.5°, else a 22.5° step the way the game decides. The per-motion
/// pelvis rows are the game's own (save-state RAM, player + 0x6b0 + motion·0x40).
#[test]
fn match_facing() {
    for (name, data, lineup) in matches() {
        let frames = frames_live(&data);
        let p_v4 = |fr: Frame, p: usize, off: usize| [0, 4, 8, 12].map(|o| fr.player_f32(p, off + o));
        let (mut n, mut steps, mut bad) = (0, 0, 0);
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            if f(fr.gm(), 0x58).to_bits() as i32 - f(a.gm(), 0x58).to_bits() as i32 > 2 {
                break;
            }
            if f(fr.gm(), 0x50).to_bits().wrapping_sub(f(a.gm(), 0x50).to_bits()) != 1 || fr.gm()[0x55] != a.gm()[0x55] {
                continue;
            }
            for p in 0..4 {
                // the turn runs in the play state (+0x3fa4 = 0), not while serving or after the point
                if p_u8(fr, p, 0x3fa4) != 0 {
                    continue;
                }
                let fwd = if fr.player_f32(p, 0x3d78) < 0.0 { 1.0 } else { -1.0 };
                let mut face = Facing {
                    dir: p_v4(a, p, 0x3d60),
                    turned: p_u8(fr, p, 0x3dd1) != 0,
                    reversed: p_u8(fr, p, 0x3dd0) != 0,
                    way: p_i32(a, p, 0x3dd4),
                    cross: p_v4(a, p, 0x3de0),
                };
                let (start, now) = (p_i32(a, p, 0x3df0), p_i32(fr, p, 0x3df0));
                if !(0..48).contains(&start) || !(0..48).contains(&now) {
                    continue;
                }
                let target = p_v4(fr, p, 0x3dc0);
                // a stroke or dive starting this frame squares the body up first (its target is set alike)
                let mode = p_u8(fr, p, 0x3fa5);
                if mode >= 2 && mode != p_u8(a, p, 0x3fa5) {
                    face.dir = target;
                }
                let stepping = target.map(f32::to_bits) != face.dir.map(f32::to_bits);
                turn(&mut face, target, fwd, lineup[p].hand, start as usize, base_motion(start), now as usize, &lineup[p].pelvis);
                n += 1;
                steps += stepping as i32;
                let want = (p_v4(fr, p, 0x3d60), p_i32(fr, p, 0x3dd4), p_v4(fr, p, 0x3de0));
                if (face.dir.map(f32::to_bits), face.way, face.cross.map(f32::to_bits)) != (want.0.map(f32::to_bits), want.1, want.2.map(f32::to_bits)) {
                    bad += 1;
                    if bad <= 10 {
                        eprintln!("{name} facing k={k} p={p} start={start:#x} now={now:#x}: {:?} way {} vs {:?} way {} (cross {:?} vs {:?})", face.dir, face.way, want.0, want.1, face.cross, want.2);
                    }
                }
            }
        }
        eprintln!("{name}: facing {n} checked ({steps} turning), {bad} off");
        assert!(steps > 100, "{name}");
        assert_eq!(bad, 0, "{name}");
    }
}

/// The turn's per-motion pelvis rows computed from the disc (skeleton + each motion's first keys) agree with
/// the game's table in slot-5 RAM and `1p3goodcpus_ee.bin` (characters 6, 11, 2, 10), and in P7b's per-character
/// dumps (`p7b_cNN_pelvis.bin`) for all 14.
#[test]
fn pelvis_table_from_disc() {
    use hst_data::{ani, iso::Iso, mdl, xb::Archive};
    use hst_sim::pose::{Skeleton, first_frame};
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(ram), Ok(mut iso)) = (std::fs::read(format!("{dir}/slot5_ee.bin")), Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"))) else {
        return eprintln!("slot5_ee.bin or ISO absent, skipped");
    };
    let ru = |a: usize| u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize;
    let gm = ru(0x422f80);
    let mut worst = 0.0f32;
    let mut tables: Vec<(usize, Vec<u8>)> = [(0, 0), (1, 2), (2, 1), (3, 5)].map(|(p, c)| (c, ram[ru(gm + 0xa8 + 4 * p) + 0x6b0..][..48 * 0x40].to_vec())).into();
    if let Ok(ram) = std::fs::read(format!("{dir}/1p3goodcpus_ee.bin")) {
        let ru = |a: usize| u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize;
        tables.extend((0..4).map(|p| ru(ru(0x422f80) + 0xa8 + 4 * p)).map(|pl| (ru(pl + 0x12bc), ram[pl + 0x6b0..][..48 * 0x40].to_vec())));
    }
    tables.extend((0..14).filter_map(|c| Some((c, std::fs::read(format!("{dir}/p7b_c{c:02}_pelvis.bin")).ok()?))));
    for (c, table) in tables {
        let data = iso.read(&format!("PC/PC{c:02}C00.XB")).unwrap();
        let arc = Archive::parse(&data).unwrap();
        let e = arc.entries.iter().find(|e| { let n = e.name.to_ascii_lowercase(); n.contains(&format!("pc{c:02}_t")) && n.ends_with("_c00.mdl") }).unwrap();
        let m = mdl::parse(&arc.read(e).unwrap()).unwrap();
        let sk = Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() };
        let pelvis = sk.names.iter().position(|n| n == "Bip01Pelvis").unwrap();
        let anims = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
        let aarc = Archive::parse(&anims).unwrap();
        for mo in 0..48 {
            let stem = ani::motion_name(mo, c).unwrap().to_ascii_lowercase();
            let Some(e) = aarc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))) else { continue };
            let a = ani::parse(&aarc.read(e).unwrap()).unwrap();
            let row = first_frame(&sk, &a)[pelvis][2];
            let want = [f(&table, mo * 0x40 + 0x20), f(&table, mo * 0x40 + 0x28)];
            let d = (row[0] - want[0]).abs().max((row[2] - want[1]).abs());
            if d > 1e-3 { eprintln!("char {c} motion {mo:#x}: {:?} vs {want:?}", [row[0], row[2]]); }
            worst = worst.max(d);
        }
    }
    eprintln!("worst pelvis row difference {worst}");
    assert!(worst < 1e-4);
}

/// A human's movement from the pad, replayed: slot 4 with P1 (player 0) driven by a scripted stick/d-pad, as Carol
/// (character 6) and as Kaito (character 3: his speed and agility written into player 0, Carol's body and pelvis
/// rows kept). Stats from TParam.csv; every play-state frame stepped from the previous simulated frame (resynced
/// only when the play state starts), position, velocity, motion and facing bit-exact. Then all 14 characters as P1
/// with their own bodies (`research/p7b_record.py`: picked on the doubles character select, whose P1 has the
/// "switch hand" toggle on, so each plays with the hand opposite to TParam's), each with its own dumped pelvis rows.
/// Last the human player 0 of `1p3goodcpus.bin`: Carol left-handed, against three CPUs, pelvis rows from its RAM.
#[test]
fn human_pad_replay() {
    use hst_data::{iso::Iso, xb::Archive};
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(pel), Ok(mut iso)) = (std::fs::read(format!("{dir}/p7_pelvis_s04.bin")), Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"))) else {
        return eprintln!("p7_pelvis_s04.bin or ISO absent, skipped");
    };
    let rows = |pel: &[u8]| -> Vec<[f32; 2]> { (0..48).map(|m| [f(pel, m * 0x40 + 0x20), f(pel, m * 0x40 + 0x28)]).collect() };
    let data = iso.read("PCDATA/PCDATA.XB").unwrap();
    let arc = Archive::parse(&data).unwrap();
    let csv = arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).unwrap()).unwrap();
    let stats = |n: usize| {
        let row = csv.split(|&b| b == b'\n').find(|l| l.starts_with(format!("{n},").as_bytes())).unwrap();
        let row: Vec<String> = row.split(|&b| b == b',').map(|c| String::from_utf8_lossy(c).trim().to_string()).collect();
        let c: Vec<i32> = row[42].split('/').map(|v| v.parse().unwrap()).collect();
        Stats::new(row[40].parse().unwrap(), row[43].parse().unwrap(), row[41].parse().unwrap(), [c[0], c[1], c[2]], 0)
    };
    let p_v4 = |fr: Frame, p: usize, off: usize| [0, 4, 8, 12].map(|o| fr.player_f32(p, off + o));
    let hand = |n: usize| {
        let row = csv.split(|&b| b == b'\n').find(|l| l.starts_with(format!("{n},").as_bytes())).unwrap();
        if row.split(|&b| b == b',').nth(6).unwrap().trim_ascii() == [0x89, 0x45] { 1.0 } else { -1.0 } // 右
    };
    let mut runs = vec![("p7_carol_s04.bin".to_string(), 6, rows(&pel), 1.0), ("p7_kaito_s04.bin".into(), 3, rows(&pel), 1.0)];
    // P5's press recording: d-pad diagonals held (full length, the d-pad path of `pad_dir`)
    runs.push(("p5_presses_s04.bin".into(), 6, rows(&pel), 1.0));
    for ch in 0..14 {
        if let Ok(pel) = std::fs::read(format!("{dir}/p7b_c{ch:02}_pelvis.bin")) {
            runs.push((format!("p7b_c{ch:02}.bin"), ch, rows(&pel), -hand(ch)));
        }
    }
    for (name, _, lineup) in matches().into_iter().filter(|m| m.0 == "1p3goodcpus.bin") {
        runs.push((name.into(), 6, lineup[0].pelvis.clone(), lineup[0].hand));
    }
    for (file, ch, pelvis, hand) in runs {
        let Ok(rec) = std::fs::read(format!("{dir}/{file}")) else {
            eprintln!("{file} absent, skipped");
            continue;
        };
        let s = stats(ch);
        let frames = frames_live(&rec);
        // speed (+0x1374) isn't recorded; the run velocities check it
        assert_eq!(p_i32(frames[0], 0, 0x1388), s.agility, "{file}: TParam row {ch}");
        let (mut n, mut moving, mut bad, mut body) = (0, 0, 0, None::<Body>);
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            let live = |x: Frame| p_u8(x, 0, 0x3fa4) == 0 && p_u8(x, 0, 0x3fa5) <= 1;
            // a face button in play starts a swing (P5), not running: skip it and resync after (a held button steers
            // the approach until the frame after its release, when the stroke starts)
            let swing = (a.pad(0).buttons | fr.pad(0).buttons) & 0xf000 != 0;
            if !live(a) || !live(fr) || swing || f(fr.gm(), 0x50).to_bits().wrapping_sub(f(a.gm(), 0x50).to_bits()) != 1 {
                body = None;
                continue;
            }
            let fwd = if a.player_f32(0, 0x3d78) < 0.0 { 1.0 } else { -1.0 };
            let mut b = body.unwrap_or(Body {
                pos: a.player_pos(0),
                vel: p_v3(a, 0, 0x3e00),
                running: p_u8(a, 0, 0x3fa5) == 1,
                run: p_i32(a, 0, 0x3dfc),
                stamina: p_i32(a, 0, 0x3df4),
                stamina_tick: p_i32(a, 0, 0x3df8),
                motion: p_i32(a, 0, 0x3df0),
                target: p_v4(a, 0, 0x3dc0),
                face: Facing { dir: p_v4(a, 0, 0x3d60), turned: p_u8(a, 0, 0x3dd1) != 0, reversed: p_u8(a, 0, 0x3dd0) != 0, way: p_i32(a, 0, 0x3dd4), cross: p_v4(a, 0, 0x3de0) },
            });
            // player 0 updates first: the hitter of this frame (a later player) hasn't changed the ball yet
            let (old, new) = (a.global(0x423058), fr.global(0x423058));
            let ball = if old != new { a.live_ball() } else { fr.live_ball() };
            let sc = Scene {
                players: 4,
                phase: fr.gm()[0x55],
                last_hitter: old,
                team: 0,
                forward: fwd,
                hand,
                mate: Some(a.player_pos(2)),
                ball: [f(ball, 0xe0), f(ball, 0xe4), f(ball, 0xe8)],
                ball_dir: [f(ball, 0x140), f(ball, 0x144), f(ball, 0x148)],
                short: false,
            };
            let pad = fr.pad(0);
            b.step(&s, pad_dir(pad.buttons, pad.lx, pad.ly, sc.phase), &sc, &pelvis);
            body = Some(b);
            n += 1;
            moving += b.running as i32;
            let got = (b.pos, b.vel, b.motion, b.face.dir);
            let want = (fr.player_pos(0), p_v3(fr, 0, 0x3e00), p_i32(fr, 0, 0x3df0), p_v4(fr, 0, 0x3d60));
            if (got.0.map(f32::to_bits), got.1.map(f32::to_bits), got.2, got.3.map(f32::to_bits)) != (want.0.map(f32::to_bits), want.1.map(f32::to_bits), want.2, want.3.map(f32::to_bits)) {
                bad += 1;
                if bad <= 10 {
                    eprintln!("{file} k={k} pad {:04x} {:02x},{:02x}: {got:?}\n   vs {want:?}", pad.buttons, pad.lx, pad.ly);
                }
                body = None;
            }
        }
        eprintln!("{file}: {n} frames stepped ({moving} running), {bad} off");
        assert!(moving > 50);
        assert_eq!(bad, 0, "{file}");
    }
}

/// A left-hander's stand/run motion and facing: Will (character 11, the CPU's player 3 in slot 4, hand −1)
/// through the slot-4 recordings, with his pelvis rows from the disc and stats from TParam.csv.
#[test]
fn lefty_s04() {
    use hst_data::{ani, iso::Iso, mdl, xb::Archive};
    use hst_sim::pose::{Skeleton, first_frame};
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("ISO absent, skipped");
    };
    let c = 11;
    let data = iso.read(&format!("PC/PC{c:02}C00.XB")).unwrap();
    let arc = Archive::parse(&data).unwrap();
    let e = arc.entries.iter().find(|e| { let n = e.name.to_ascii_lowercase(); n.contains(&format!("pc{c:02}_t")) && n.ends_with("_c00.mdl") }).unwrap();
    let m = mdl::parse(&arc.read(e).unwrap()).unwrap();
    let sk = Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() };
    let hip = sk.names.iter().position(|n| n == "Bip01Pelvis").unwrap();
    let anims = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
    let aarc = Archive::parse(&anims).unwrap();
    let pelvis: Vec<[f32; 2]> = (0..48)
        .map(|mo| {
            let stem = ani::motion_name(mo, c).unwrap().to_ascii_lowercase();
            aarc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).map_or([0.0, 1.0], |e| {
                let r = first_frame(&sk, &ani::parse(&aarc.read(e).unwrap()).unwrap())[hip][2];
                [r[0], r[2]]
            })
        })
        .collect();
    let p_v4 = |fr: Frame, p: usize, off: usize| [0, 4, 8, 12].map(|o| fr.player_f32(p, off + o));
    let (p, hand) = (3, -1.0);
    let (mut motions, mut bad_motion, mut faces, mut bad_face) = (0, 0, 0, 0);
    for file in ["p7_carol_s04.bin", "p7_kaito_s04.bin"] {
        let Ok(rec) = std::fs::read(format!("{dir}/{file}")) else {
            eprintln!("{file} absent, skipped");
            continue;
        };
        let frames = frames_live(&rec);
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            if f(fr.gm(), 0x50).to_bits().wrapping_sub(f(a.gm(), 0x50).to_bits()) != 1 || fr.gm()[0x55] != a.gm()[0x55] {
                continue;
            }
            let fwd = if fr.player_f32(p, 0x3d78) < 0.0 { 1.0 } else { -1.0 };
            let (mode, run, stamina) = (p_u8(fr, p, 0x3fa5), p_i32(fr, p, 0x3dfc), p_i32(fr, p, 0x3df4));
            let target = p_v3(fr, p, 0x3dc0);
            let turned = p_u8(fr, p, 0x3dd1) != 0 || p_u8(a, p, 0x3dd1) != 0;
            let current = base_motion(p_i32(a, p, 0x3df0));
            let want = p_i32(fr, p, 0x3df0);
            if p_u8(fr, p, 0x3fa4) == 0 && mode <= 1 && p_u8(a, p, 0x3fa5) <= 1 && want < 16 {
                let m = if mode == 1 {
                    let d = if turned { p_v3(a, p, 0x3d60) } else { target };
                    run_motion(current, angle(d, fwd, hand), dashing(&Stats::new(11, 25, 40, [8, 4, 7], 0), run))
                } else {
                    let d = if turned { p_v3(a, p, 0x3d60) } else { [0.0, 0.0, fwd] };
                    let (old, new) = (a.global(0x423058), fr.global(0x423058));
                    let before = old != new && (p as i32) < new;
                    let ball = if before { a.live_ball() } else { fr.live_ball() };
                    stand_motion(angle(d, fwd, hand), || {
                        stance(&StanceInput {
                            players: 4,
                            phase: fr.gm()[0x55],
                            last_hitter: if before { old } else { new },
                            team: p as i32,
                            current,
                            watching: true,
                            pos: fr.player_pos(p),
                            ball: [f(ball, 0xe0), f(ball, 0xe4), f(ball, 0xe8)],
                            ball_dir: [f(ball, 0x140), f(ball, 0x144), f(ball, 0x148)],
                            hand,
                        })
                    })
                };
                motions += 1;
                if with_tiredness(m, stamina) != want {
                    bad_motion += 1;
                    if bad_motion <= 10 {
                        eprintln!("{file} motion k={k} mode={mode} current={current} turned={turned} target {target:?}: {m} vs {want}");
                    }
                }
            }
            if p_u8(fr, p, 0x3fa4) == 0 {
                let mut face = Facing { dir: p_v4(a, p, 0x3d60), turned: p_u8(fr, p, 0x3dd1) != 0, reversed: p_u8(fr, p, 0x3dd0) != 0, way: p_i32(a, p, 0x3dd4), cross: p_v4(a, p, 0x3de0) };
                let (start, now) = (p_i32(a, p, 0x3df0), p_i32(fr, p, 0x3df0));
                if !(0..48).contains(&start) || !(0..48).contains(&now) {
                    continue;
                }
                let target = p_v4(fr, p, 0x3dc0);
                if mode >= 2 && mode != p_u8(a, p, 0x3fa5) {
                    face.dir = target;
                }
                turn(&mut face, target, fwd, hand, start as usize, base_motion(start), now as usize, &pelvis);
                faces += 1;
                let want = (p_v4(fr, p, 0x3d60), p_i32(fr, p, 0x3dd4));
                if (face.dir.map(f32::to_bits), face.way) != (want.0.map(f32::to_bits), want.1) {
                    bad_face += 1;
                    if bad_face <= 10 {
                        eprintln!("{file} facing k={k} start={start:#x} now={now:#x}: {:?} way {} vs {:?} way {}", face.dir, face.way, want.0, want.1);
                    }
                }
            }
        }
    }
    eprintln!("lefty: motion {motions} checked, {bad_motion} off; facing {faces} checked, {bad_face} off");
    assert_eq!((bad_motion, bad_face), (0, 0));
}

/// Every character's reach and contact heights from the disc's TParam.csv against the game's parsed records in RAM
/// (`p7c_tparam_s0N.bin`: the 14 records of 0x118 bytes as the parser left them, then per player its character and
/// its copy at +0x12d8), from save slots 5 and 3; and each player's copy equals its character's record.
#[test]
fn reach_stats_from_tparam() {
    use hst_data::{iso::Iso, xb::Archive};
    use hst_sim::player::ReachStats;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc absent, skipped");
    };
    let data = iso.read("PCDATA/PCDATA.XB").unwrap();
    let arc = Archive::parse(&data).unwrap();
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).unwrap();
    let csv = String::from_utf8_lossy(&arc.read(e).unwrap()).into_owned();
    let stats: Vec<ReachStats> = (0..14)
        .map(|n| ReachStats::from_tparam(csv.lines().find(|l| l.starts_with(&format!("{n},"))).unwrap()))
        .collect();
    for slot in [5, 3] {
        let Ok(ram) = std::fs::read(format!("{dir}/p7c_tparam_s0{slot}.bin")) else {
            return eprintln!("p7c_tparam_s0{slot}.bin absent, skipped");
        };
        for (n, s) in stats.iter().enumerate() {
            let rec = &ram[n * 0x118..(n + 1) * 0x118];
            let i = |o: usize| i32::from_le_bytes(rec[o - 0x12d8..o - 0x12d4].try_into().unwrap());
            let fl = |o: usize| f(rec, o - 0x12d8).to_bits();
            assert_eq!([s.body_adj, s.vbody_adj, s.rising_adj, s.smash_low_pow, s.stroke_high_pow], [0x1310, 0x1314, 0x1318, 0x13a0, 0x13a4].map(i), "character {n}");
            let ints = [s.power.as_slice(), &s.after, &s.before, &s.stroke_miss, &s.volley_miss, &[s.body_down, s.back_down, s.volley_down], &s.low_power].concat();
            let offs = [0x12e4, 0x12e8, 0x12ec, 0x12f0, 0x12f4, 0x131c, 0x1320, 0x1324, 0x1328, 0x132c, 0x1330, 0x1364, 0x1368, 0x136c, 0x1370, 0x138c, 0x1390, 0x1394, 0x1398, 0x139c];
            assert_eq!(ints, offs.map(i), "character {n}");
            let mine = [
                [s.serve_scatter, s.base, s.reach, s.under_min, s.stroke_height, s.volley_height].as_slice(),
                &s.smash, &s.serve, &s.under_serve, &[s.dive_start, s.dive_limit, s.collision],
            ]
            .concat();
            assert_eq!(mine.iter().map(|v| v.to_bits()).collect::<Vec<_>>(), (0x13a8..0x13f0).step_by(4).map(fl).collect::<Vec<_>>(), "character {n}");
        }
        for p in ram[14 * 0x118..].chunks(0x11c) {
            let c = u32::from_le_bytes(p[..4].try_into().unwrap()) as usize;
            let (copy, rec) = (&p[4..], &ram[c * 0x118..(c + 1) * 0x118]);
            // +0x12e0 is a byte: the three after it belong to something else
            assert!(copy[..9] == rec[..9] && copy[12..] == rec[12..], "slot {slot} player with character {c}");
        }
    }
}

/// The server's baseline walk before the toss (play state 1): Carol in slot 4's next serve, walked by a vpad
/// script (far and centre-mark limits, partial stick, dead square, diagonals, d-pad), and the same with her hand
/// poked to −1 (`research/p7d_record.py`); then the left-handed human Carol's serves in `1p3goodcpus.bin`. Position and
/// motion bit-exact every frame.
#[test]
fn serve_walk_replay() {
    use hst_sim::player::serve_walk;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let mut runs = vec![("p7d_s04.bin", 1.0, 300), ("p7d_s04_lefty.bin", -1.0, 300)];
    runs.extend(matches().into_iter().filter(|m| m.0 == "1p3goodcpus.bin").map(|(name, _, lineup)| (name, lineup[0].hand, 0)));
    for (file, hand, least) in runs {
        let Ok(rec) = std::fs::read(format!("{dir}/{file}")) else {
            eprintln!("{file} absent, skipped");
            continue;
        };
        let frames = frames_live(&rec);
        let (mut n, mut walked, mut bad) = (0, 0, 0);
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            let pre = |x: Frame| p_u8(x, 0, 0x3fa4) == 1 && p_u8(x, 0, 0x3fa6) <= 1;
            // a press tosses (sub-state 2): P6
            if !pre(a) || !pre(fr) || f(fr.gm(), 0x50).to_bits().wrapping_sub(f(a.gm(), 0x50).to_bits()) != 1 {
                continue;
            }
            let pad = fr.pad(0);
            let end = if a.player_f32(0, 0x3d78) < 0.0 { 1.0 } else { -1.0 };
            let x = a.player_f32(0, 0x3d70);
            let got = serve_walk(x, pad_dir(pad.buttons, pad.lx, pad.ly, fr.gm()[0x55]), end, fr.global(0x423050), fr.global(0x422fa4) > 2, hand);
            let want = (fr.player_f32(0, 0x3d70), p_i32(fr, 0, 0x3df0));
            n += 1;
            walked += (want.0 != x) as i32;
            if (got.0.to_bits(), got.1) != (want.0.to_bits(), want.1) {
                bad += 1;
                if bad <= 10 {
                    eprintln!("{file} k={k} pad {:04x} {:02x},{:02x} x {x}: {got:?} vs {want:?}", pad.buttons, pad.lx, pad.ly);
                }
            }
        }
        eprintln!("{file}: {n} frames, {walked} walked, {bad} off");
        assert!(walked >= least, "{file}");
        assert_eq!(bad, 0, "{file}");
    }
}

/// `research/p7f_record.py 5 3000` (slot 5, all four players computer): per frame the stick bytes the AI left at
/// +0x17d4, the position, run target, motion/stamina/run, velocity, play state/mode and the AI object.
struct BotFrame {
    phase: u8,
    players: i32,
    p: Vec<BotPlayer>,
}
#[derive(Clone)]
struct BotPlayer {
    stick: [u8; 2],
    pos: [f32; 3],
    target: [f32; 4],
    st: i32,
    tick: i32,
    run: i32,
    vel: [f32; 3],
    mode: u8,
    ai: Vec<u8>,
}

fn bot_frames(d: &[u8]) -> Vec<BotFrame> {
    let n = u32::from_le_bytes(d[0..4].try_into().unwrap()) as usize;
    let (pp, mut o) = (0x2d8, 4 + n * 0x48);
    let size = 4 + 0x180 + 0x60 + n * pp;
    let i = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let mut out = vec![];
    while o + size <= d.len() {
        let g = &d[o + 4..];
        let p = (0..n)
            .map(|k| {
                let r = &d[o + 0x1e4 + k * pp..o + 0x1e4 + (k + 1) * pp];
                let s = u16::from_le_bytes([r[4], r[5]]);
                BotPlayer {
                    stick: [(s >> 8) as u8, s as u8],
                    pos: [0, 4, 8].map(|x| f(r, 0x10 + x)),
                    target: [0, 4, 8, 12].map(|x| f(r, 0x20 + x)),
                    st: i(r, 0x34),
                    tick: i(r, 0x38),
                    run: i(r, 0x3c),
                    vel: [0, 4, 8].map(|x| f(r, 0x40 + x)),
                    mode: r[0x51],
                    ai: r[0x58..].to_vec(),
                }
            })
            .collect();
        out.push(BotFrame { phase: g[0x180 + 0x55], players: i(g, 0x24), p });
        o += size;
    }
    out
}

/// The computer players' stick: the bytes the AI leaves decode to the run target and velocity bit-exact on every
/// run frame; in the receive state every run step is the step toward the AI's contact point (+0x70, kept on to
/// the end) as bytes, and in the rally the step toward that point or the AI's court spot (+0xc0) accounts for most
/// (the rest head for spots the rally routines work out on the fly, P11e/f); the serve walk is a full ±x stick.
#[test]
fn p7f_bot_stick() {
    use hst_sim::player::{bot_stick, stick_dir};
    use hst_sim::ps2::{div, madd, mul, sqrt};
    // slot 5's line-up (characters 0, 2, 1, 5; TParam SPE/Agili/STA and costs)
    const STATS: [(i32, i32, i32, [i32; 3]); 4] = [(10, 40, 40, [8, 4, 7]), (11, 40, 40, [8, 4, 7]), (9, 40, 40, [6, 3, 7]), (12, 10, 40, [8, 6, 6])];
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/p7f_s05.bin")) else {
        return eprintln!("p7f_s05.bin absent, skipped");
    };
    let fr = bot_frames(&data);
    let (mut runs, mut bad_dir, mut bad_vel, mut receive, mut rally, mut walks) = (0, 0, 0, 0, 0, 0);
    for k in 1..fr.len() {
        for p in 0..4 {
            let (a, b, phase) = (&fr[k - 1].p[p], &fr[k].p[p], fr[k].phase);
            let (spe, agi, sta, costs) = STATS[p];
            let s = Stats::new(spe, agi, sta, costs, 0);
            if b.mode == 1 && a.mode <= 1 {
                runs += 1;
                let v = stick_dir(b.stick, phase);
                let inv = div(1.0, sqrt(madd(mul(v[1], v[1]), v[0], v[0])));
                let d = [mul(v[0], inv), mul(v[1], inv)];
                let snapped = b.target[0] == 0.0 && b.target[2].abs() == 1.0; // straight ahead: ±forward
                bad_dir += (!snapped && [b.target[0], b.target[2]].map(f32::to_bits) != d.map(f32::to_bits)) as i32;
                let vel = run_velocity([d[0], 0.0, d[1]], run_speed(&s, b.run, b.st, 100));
                bad_vel += (vel.map(f32::to_bits) != b.vel.map(f32::to_bits)) as i32;
            }
            if b.stick == [0x80, 0x80] || a.mode > 1 || !(2..=4).contains(&phase) {
                continue;
            }
            let body = Body { pos: a.pos, running: a.mode == 1, run: a.run, stamina: a.st, stamina_tick: a.tick, ..Body::default() };
            let go = |o: usize, keep: bool| bot_stick(body.toward(&s, [f(&b.ai, o), f(&b.ai, o + 8)], fr[k].players, phase, keep).0);
            match b.ai[0x54] {
                1 => walks += (b.stick[1] == 0x80 && [1, 255].contains(&b.stick[0])) as i32,
                2 => {
                    assert_eq!(go(0x70, true), b.stick, "receive k{k} p{p}");
                    receive += 1;
                }
                _ => rally += (go(0x70, true) == b.stick || go(0xc0, false) == b.stick) as i32,
            }
        }
    }
    eprintln!("runs {runs}, receive steps {receive}, rally steps {rally}, serve walk {walks}");
    assert_eq!((bad_dir, bad_vel), (0, 0), "of {runs} run frames");
    assert!(runs > 1500 && receive > 50 && rally > 1300 && walks > 60);
}

#[test]
fn toward_stops_short() {
    use hst_sim::player::{bot_stick, stick_dir};
    let s = Stats::new(10, 40, 40, [8, 4, 7], 0);
    let b = Body { stamina: 40, ..Body::default() };
    let step = run_speed(&s, 0, 40, 100);
    let to = |t: [f32; 2], keep| { let (d, there) = b.toward(&s, t, 4, 3, keep); (bot_stick(d), there) };
    // within ⅔ of a step: no direction (unless kept on); within a step: there, still running
    assert_eq!(to([0.0, step * 0.6], false), ([0x80, 0x80], true));
    assert_eq!(to([0.0, step * 0.6], true), ([0x80, 255], true));
    assert_eq!(to([0.0, step * 0.9], false), ([0x80, 255], true));
    assert_eq!(to([3.0, 4.0], false), ([0x80 + 76, 0x80 + 102], false));
    // an axis within 1 mm is dropped
    assert_eq!(b.toward(&s, [0.0009, 4.0], 4, 3, false).0[0], 0.0);
    // bytes: half away from zero, clamped; back to a unit at most
    assert_eq!(bot_stick([0.6, -0.8]), [0x80 + 76, 0x80 - 102]);
    assert_eq!(bot_stick([1.0, -1.0]), [255, 1]);
    assert_eq!(bot_stick([-0.0, 0.0]), [0x80, 0x80]);
    assert_eq!(stick_dir([255, 0x80], 3), [1.0, 0.0]);
    assert_eq!(stick_dir([255, 0x80], 1), [0.0, 0.0]);
}
