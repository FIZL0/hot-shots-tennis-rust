//! The serve against every serve of the slot-5 doubles match (`context/fixtures/match_s05.bin`): toss release
//! point, apex and launch velocity, and the contact frame the swing press locks onto. Per-player timing tables
//! come from the slot-5 save state's RAM (`context/fixtures/slot5_ee.bin`).

use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};
use hst_sim::replay::{Frame, frames_live};
use hst_sim::serve::{self, ServeData, Toss};
use hst_sim::swing::PathPoint;

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
fn load(b: &[u8]) -> (Flight, Shot) {
    let ball = Ball { pos: v3(b, 0xe0), vel: v3(b, 0x130), spin: f(b, 0x1a4) };
    let mut fl = Flight::new(ball, rows(b, 0x160), rows(b, 0x1c0));
    fl.frame = i(b, 0xac);
    fl.bounces = i(b, 0x224);
    fl.contacts = i(b, 0x228);
    fl.special_contacts = i(b, 0x22c);
    fl.rolling = b[0xa4] == 2;
    fl.slide = f(b, 0x200);
    let shot = Shot { params: Params::default(), curve: f(b, 0x254), bend: f(b, 0x250), side: v3(b, 0x90), curve_frames: i(b, 0x260), wind: v3(b, 0x240), first_bounce_spin: f(b, 0x1a8), first_bounce_restitution: f(b, 0x1ac), class: b[0x58], kind: i(b, 0x5c), bounce_turn: f(b, 0x1b0) };
    (fl, shot)
}
fn pu8(fr: Frame, p: usize, off: usize) -> u8 {
    fr.player_f32(p, off & !3).to_bits().to_le_bytes()[off & 3]
}
fn pi(fr: Frame, p: usize, off: usize) -> i32 {
    fr.player_f32(p, off).to_bits() as i32
}
fn toss_of(kind: i32) -> Toss {
    match kind {
        4 => Toss::Under,
        1 => Toss::Strong,
        _ => Toss::Weak,
    }
}

#[test]
fn match_s05_serves() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(ram)) = (std::fs::read(format!("{dir}/match_s05.bin")), std::fs::read(format!("{dir}/slot5_ee.bin"))) else {
        return eprintln!("match_s05.bin or slot5_ee.bin absent, skipped");
    };
    let ru = |a: usize| u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize;
    let gm = ru(0x422f80);
    let frames = frames_live(&data);
    let data_for = |p: usize, fr: Frame| {
        let obj = ru(gm + 0xa8 + 4 * p);
        let grades = |o: usize, n: usize| ram[obj + o..obj + o + ru(obj + n)].to_vec();
        ServeData {
            over: [0x13cc, 0x13d0, 0x13d4].map(|o| fr.player_f32(p, o)),
            under: [0x13d8, 0x13dc, 0x13e0].map(|o| fr.player_f32(p, o)),
            strong_grades: grades(0x1644, 0x1680),
            weak_grades: grades(0x1774, 0x17b0),
            hand_over: [0.144, -1.546, 0.12],
            hand_under: [0.016, -0.774, 0.271],
            apex_drift_over: [-0.133, 0.15],
            apex_drift_under: [0.184, 0.033],
            miss: [50, 30, 100],
            max_angle: 22.0,
        }
    };
    let (mut tosses, mut swings, mut misses) = (0, 0, 0);
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        // only where the recorder kept up (one game tick per sample; it falls behind at the very end)
        let tick = |f: Frame| u32::from_le_bytes(f.gm()[0x58..0x5c].try_into().unwrap());
        if tick(fr) != tick(a) + 1 {
            continue;
        }
        for p in 0..4 {
            let toss = toss_of(pi(fr, p, 0x3ea0));
            let pos = fr.player_pos(p);
            let facing = if pos[2] < 0.0 { 1.0 } else { -1.0 };
            let d = data_for(p, fr);
            if pu8(fr, p, 0x3ec0) == 1 && pu8(a, p, 0x3ec0) == 0 {
                let (hand, apex) = serve::toss_points(&d, toss, pos, facing);
                let rec_hand = [0x3eb0, 0x3eb4, 0x3eb8].map(|o| fr.player_f32(p, o));
                let rec_apex = [0x3e90, 0x3e94, 0x3e98].map(|o| fr.player_f32(p, o));
                let close = |x: [f32; 3], y: [f32; 3], e: f32| (0..3).all(|j| (x[j] - y[j]).abs() < e);
                // the hand and drift are character 0's toss animation (player 0)
                assert!(p != 0 || close(hand, rec_hand, 2e-3) && close(apex, rec_apex, 2e-3), "vsync {} p{p} {toss:?}: hand {hand:?} vs {rec_hand:?}, apex {apex:?} vs {rec_apex:?}", fr.vsync());
                let vel = serve::toss_velocity(rec_hand, rec_apex);
                let rec_vel = v3(fr.live_ball(), 0x130);
                assert!(close(vel, rec_vel, 1e-5), "vsync {} p{p}: toss velocity {vel:?} vs {rec_vel:?}", fr.vsync());
                tosses += 1;
            }
            if pu8(fr, p, 0x3fa6) == 3 && pu8(a, p, 0x3fa6) != 3 {
                let (mut fl, shot) = load(fr.live_ball());
                let mut path = vec![];
                for _ in 0..d.grades(toss).len() {
                    path.push(PathPoint { pos: fl.ball.pos, bounces: fl.bounces });
                    fl.step(&shot, &COURTS[10]);
                }
                let got = serve::search(&d, toss, &path);
                let off = pi(fr, p, 0x3fa0);
                let want = (off != 999).then_some(off + serve::SWEET_FRAME);
                if got.map(|k| k as i32) != want {
                    eprintln!("vsync {} p{p} {toss:?}: contact frame {got:?} vs {want:?}", fr.vsync());
                    misses += 1;
                }
                swings += 1;
            }
        }
    }
    eprintln!("{tosses} tosses, {swings} swings, {misses} misses");
    assert_eq!(misses, 0);
    assert!(tosses >= 30 && swings >= 30, "{tosses} tosses, {swings} swings");
}
