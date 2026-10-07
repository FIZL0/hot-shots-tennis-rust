//! Locomotion of every player through the slot-5 doubles match (`context/fixtures/match_s05.bin`): run velocity
//! bit-exact and the motion the game sets for standing and running players, frame by frame. Characters 0, 2, 1, 5
//! (TParam.csv SPE/Agili/STA), all right-handed.

use hst_sim::player::{Stats, StanceInput, angle, base_motion, dashing, run_motion, run_speed, run_velocity, stance, stand_motion, with_tiredness};
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

const STATS: [(i32, i32, i32); 4] = [(10, 40, 40), (11, 40, 40), (9, 40, 40), (12, 10, 40)];

#[test]
fn match_s05_locomotion() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/match_s05.bin")) else {
        return eprintln!("match_s05.bin absent, skipped");
    };
    let frames = frames_live(&data);
    let (mut speeds, mut motions, mut bad_speed, mut bad_motion) = (0, 0, 0, 0);
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        // the recording's tail (frame 26108 on) runs the game several ticks per sample (phase timer gm+0x58)
        if f(fr.gm(), 0x58).to_bits() as i32 - f(a.gm(), 0x58).to_bits() as i32 > 2 {
            break;
        }
        for p in 0..4 {
            let (spe, agi, sta) = STATS[p];
            let s = Stats::new(spe, agi, sta, 0);
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
    eprintln!("speed {speeds} checked, {bad_speed} off; motion {motions} checked, {bad_motion} off");
    assert!(speeds > 1000 && motions > 10000);
    assert_eq!((bad_speed, bad_motion), (0, 0));
}
