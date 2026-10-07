//! Locomotion of every player through the slot-5 doubles match (`context/fixtures/match_s05.bin`): run velocity
//! bit-exact and the motion the game sets for standing and running players, frame by frame. Characters 0, 2, 1, 5
//! (TParam.csv SPE/Agili/STA), all right-handed.

use hst_sim::player::{Facing, Stats, mover, stroke_stamina, turn, StanceInput, angle, base_motion, dashing, run_motion, run_speed, run_velocity, stance, stand_motion, with_tiredness};
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

const STATS: [(i32, i32, i32, [i32; 3]); 4] = [(10, 40, 40, [8, 4, 7]), (11, 40, 40, [8, 4, 7]), (9, 40, 40, [6, 3, 7]), (12, 10, 40, [8, 6, 6])];

#[test]
fn match_s05_locomotion() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/match_s05.bin")) else {
        return eprintln!("match_s05.bin absent, skipped");
    };
    let frames = frames_live(&data);
    let (mut speeds, mut motions, mut bad_speed, mut bad_motion, mut moves, mut bad_move, mut pushes) = (0, 0, 0, 0, 0, 0, 0);
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        // the recording's tail (frame 26108 on) runs the game several ticks per sample (phase timer gm+0x58)
        if f(fr.gm(), 0x58).to_bits() as i32 - f(a.gm(), 0x58).to_bits() as i32 > 2 {
            break;
        }
        for p in 0..4 {
            let (spe, agi, sta, costs) = STATS[p];
            let s = Stats::new(spe, agi, sta, costs, 0);
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
                let v = run_velocity(target, run_speed(&s, run, stamina, 100));
                let got = p_v3(fr, p, 0x3e00);
                speeds += 1;
                if v.map(f32::to_bits) != got.map(f32::to_bits) {
                    bad_speed += 1;
                    if bad_speed <= 10 {
                        eprintln!("speed k={k} p={p} run={run} st={stamina}: {v:?} vs {got:?}");
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
                    eprintln!("move k={k} p={p} mode={mode}: {moved:?} vs {:?} from {:?} mate {mate_pos:?}", fr.player_pos(p), a.player_pos(p));
                }
            }
            let m = if mode == 1 {
                let d = if turned { p_v3(a, p, 0x3d60) } else { target };
                run_motion(current, angle(d, fwd, 1.0), dashing(&s, run))
            } else {
                let d = if turned { p_v3(a, p, 0x3d60) } else { [0.0, 0.0, fwd] };
                // the hitter changes the ball during its own update: players before it still see the old one
                let (old, new) = (a.global(0x423058), fr.global(0x423058));
                let before = old != new && (p as i32) < new;
                let ball = if before { a.live_ball() } else { fr.live_ball() };
                stand_motion(angle(d, fwd, 1.0), || {
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
                        hand: 1.0,
                    })
                })
            };
            motions += 1;
            if with_tiredness(m, stamina) != want {
                bad_motion += 1;
                if bad_motion <= 10 {
                    eprintln!("motion k={k} p={p} mode={mode} current={current} turned={turned}: {m} vs {want}");
                }
            }
        }
    }
    eprintln!("speed {speeds} checked, {bad_speed} off; motion {motions} checked, {bad_motion} off; moves {moves} checked ({pushes} partner pushes: none in this match), {bad_move} off");
    assert!(speeds > 1000 && motions > 10000 && moves > 10000);
    assert_eq!((bad_speed, bad_motion, bad_move), (0, 0, 0));
}

/// Every stamina drop at a stroke's contact (all but the running drain) is the stroke's cost.
#[test]
fn match_s05_stroke_stamina() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/match_s05.bin")) else {
        return eprintln!("match_s05.bin absent, skipped");
    };
    let frames = frames_live(&data);
    let mut n = 0;
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        for p in 0..4 {
            let (spe, agi, sta, costs) = STATS[p];
            let s = Stats::new(spe, agi, sta, costs, 0);
            let (before, after) = (p_i32(a, p, 0x3df4), p_i32(fr, p, 0x3df4));
            let drained = before - after == 1 && p_i32(fr, p, 0x3df8) == 0;
            if after >= before || drained {
                continue;
            }
            // +0x3f50 bit 0: the ball on the right-hander's forehand side
            let forehand = p_i32(fr, p, 0x3f50) & 1 != 0;
            assert_eq!(stroke_stamina(&s, before, p_u8(fr, p, 0x3ec1), forehand, 0), after, "k={k} p={p}");
            n += 1;
        }
    }
    assert!(n > 60, "{n}");
}

/// The body's facing every frame: snap within 22.5°, else a 22.5° step the way the game decides. The per-motion
/// pelvis rows are the game's own (slot-5 RAM, player + 0x6b0 + motion·0x40).
#[test]
fn match_s05_facing() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(ram)) = (std::fs::read(format!("{dir}/match_s05.bin")), std::fs::read(format!("{dir}/slot5_ee.bin"))) else {
        return eprintln!("match_s05.bin or slot5_ee.bin absent, skipped");
    };
    let ru = |a: usize| u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize;
    let gm = ru(0x422f80);
    let pelvis: Vec<Vec<[f32; 2]>> = (0..4)
        .map(|p| {
            let obj = ru(gm + 0xa8 + 4 * p);
            (0..48).map(|m| [f(&ram, obj + 0x6b0 + m * 0x40 + 0x20), f(&ram, obj + 0x6b0 + m * 0x40 + 0x28)]).collect()
        })
        .collect();
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
            turn(&mut face, target, fwd, 1.0, start as usize, base_motion(start), now as usize, &pelvis[p]);
            n += 1;
            steps += stepping as i32;
            let want = (p_v4(fr, p, 0x3d60), p_i32(fr, p, 0x3dd4), p_v4(fr, p, 0x3de0));
            if (face.dir.map(f32::to_bits), face.way, face.cross.map(f32::to_bits)) != (want.0.map(f32::to_bits), want.1, want.2.map(f32::to_bits)) {
                bad += 1;
                if bad <= 10 {
                    eprintln!("facing k={k} p={p} start={start:#x} now={now:#x}: {:?} way {} vs {:?} way {} (cross {:?} vs {:?})", face.dir, face.way, want.0, want.1, face.cross, want.2);
                }
            }
        }
    }
    eprintln!("facing {n} checked ({steps} turning), {bad} off");
    assert!(steps > 100);
    assert_eq!(bad, 0);
}

/// The turn's per-motion pelvis rows computed from the disc (skeleton + each motion's first keys) agree with
/// the game's table in slot-5 RAM.
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
    for (p, c) in [(0, 0), (1, 2), (2, 1), (3, 5)] {
        let obj = ru(gm + 0xa8 + 4 * p);
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
            let want = [f(&ram, obj + 0x6b0 + mo * 0x40 + 0x20), f(&ram, obj + 0x6b0 + mo * 0x40 + 0x28)];
            let d = (row[0] - want[0]).abs().max((row[2] - want[1]).abs());
            if d > 1e-3 { eprintln!("char {c} motion {mo:#x}: {:?} vs {want:?}", [row[0], row[2]]); }
            worst = worst.max(d);
        }
    }
    eprintln!("worst pelvis row difference {worst}");
    assert!(worst < 1e-4);
}
