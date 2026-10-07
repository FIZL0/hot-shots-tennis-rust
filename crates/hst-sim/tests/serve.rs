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

/// The ball in the serve stance (serve mode 0, motion 0x20) in the save states that caught one: the character's
/// `*_serve_ad00_ball` track at the motion's sampled time, turned by the server's rows and moved to their spot,
/// is the ball model's matrix translation, bit-exact.
#[test]
fn stance_ball_ram() {
    use hst_data::{ani, iso::Iso, xb::Archive};
    use hst_sim::pose::Path;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(mut iso) = Iso::open(format!("{root}Hot Shots Tennis (USA).iso")) else { return eprintln!("ISO absent, skipped") };
    let mut seen = 0;
    for s in ["s03", "s04", "s05", "s08", "s09"] {
        let Ok(ram) = std::fs::read(format!("{root}context/ram/{s}.bin")) else { return eprintln!("{s}.bin absent, skipped") };
        let u = |a: usize| u32::from_le_bytes(ram[(a & 0x1ff_ffff)..][..4].try_into().unwrap()) as usize;
        let fr = |a: usize| f32::from_bits(u(a) as u32);
        let gm = u(0x422f80);
        for p in 0..4 {
            let pl = u(gm + 0xa8 + 4 * p);
            let an = u(pl + 0x54);
            if ram[(pl + 0x3fa4) & 0x1ff_ffff] != 1 || ram[(pl + 0x3fa6) & 0x1ff_ffff] != 0 || u(an + 0x20) != 0x20 {
                continue;
            }
            let rows: [[f32; 4]; 3] = std::array::from_fn(|r| std::array::from_fn(|k| fr(pl + 0x3d40 + 16 * r + 4 * k)));
            let pos = [0, 4, 8].map(|o| fr(pl + 0x3d70 + o));
            let want = [0, 4, 8].map(|o| u(u(pl + 0x13fc) + 0xb0 + 0x30 + o) as u32);
            let t = fr(an + 0x38);
            // the track's character: whichever one's ball track lands on the ball
            let hit = (0..16).find(|&c| {
                let Ok(data) = iso.read(&format!("PCANI/PC{c:02}ANI.XB")) else { return false };
                let arc = Archive::parse(&data).unwrap();
                let stem = ani::motion_name(52, c).unwrap();
                let Some(e) = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))) else { return false };
                let path = Path::new(&ani::parse(&arc.read(e).unwrap()).unwrap()).unwrap();
                serve::stance_ball(path.at(t), rows, pos).map(f32::to_bits) == want
            });
            assert!(hit.is_some(), "{s} p{p} t {t}: no character's ball track gives {:?}", want.map(f32::from_bits));
            eprintln!("{s} p{p}: character {} at t {t}", hit.unwrap());
            seen += 1;
        }
    }
    assert!(seen >= 2);
}

/// The held ball through the slot-5 match's serves: `anim_s05.bin` (each player's motion and sampled time) beside
/// `match_s05.bin` one vsync later (the server's rows and spot, the live ball). Stance (mode 0): the stance ball
/// track, bit-exact. Walk and toss before release (mode 1, 2): the left hand's `Bip01 LFinger21` × (−0.05, 0.03, −0.05), bit-exact
/// (the bone's world built leaf upward). Fading frames are left out.
#[test]
fn anim_s05_held_ball() {
    use hst_data::{ani, iso::Iso, mdl, xb::Archive};
    use hst_sim::pose::{Clip, Path, Skeleton, node_world};
    use hst_sim::vu0::transform;
    const CHARS: [usize; 4] = [0, 2, 1, 5];
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let (Ok(anim), Ok(mdata)) = (std::fs::read(format!("{dir}context/fixtures/anim_s05.bin")), std::fs::read(format!("{dir}context/fixtures/match_s05.bin"))) else {
        return eprintln!("fixtures absent, skipped");
    };
    let Ok(mut iso) = Iso::open(format!("{dir}Hot Shots Tennis (USA).iso")) else { return eprintln!("ISO absent, skipped") };
    struct Char { sk: Skeleton, clips: std::collections::HashMap<usize, Clip>, ball: Path, finger: usize }
    let chars: Vec<Char> = CHARS.iter().map(|&c| {
        let arc_data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
        let arc = Archive::parse(&arc_data).unwrap();
        let get = |m: usize| { let stem = ani::motion_name(m, c).unwrap(); ani::parse(&arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).unwrap()).unwrap()).unwrap() };
        let md = iso.read(&format!("PC/PC{c:02}C00.XB")).unwrap();
        let marc = Archive::parse(&md).unwrap();
        let m = mdl::parse(&marc.read(marc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl")).unwrap()).unwrap()).unwrap();
        let sk = Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() };
        let clips = (0x20..=0x24).map(|k| (k, Clip::new(&sk, &get(k)))).collect();
        let finger = sk.names.iter().position(|n| n.contains("LFinger21")).unwrap();
        Char { ball: Path::new(&get(52)).unwrap(), clips, finger, sk }
    }).collect();
    const S: usize = 4 + 0x100 + 4 * 0x484;
    let frames = frames_live(&mdata);
    let (mut stance, mut hand) = (0, 0);
    for k in 0..(anim.len() / S).min(frames.len() - 1) {
        let fr = frames[k + 1];
        for p in 0..4 {
            let blk = &anim[k * S + 0x104 + p * 0x484..][..0x484];
            let pos = fr.player_pos(p);
            // a serve's first frame still has the ball where the point ended
            let first = k == 0 || anim[(k - 1) * S + 0x104 + p * 0x484 + 0x3a4] != 1;
            if first || blk[0x3a4] != 1 || v3(blk, 0x170).map(f32::to_bits) != pos.map(f32::to_bits) {
                continue;
            }
            let (mode, motion, t, old) = (blk[0x3a6], i(blk, 0x420) as usize, f(blk, 0x438), i(blk, 0x444));
            let fading = i(blk, 0x448) != 0;
            let rows: [[f32; 4]; 4] = std::array::from_fn(|r| std::array::from_fn(|c| fr.player_f32(p, 0x3d40 + 16 * r + 4 * c)));
            let want = v3(fr.live_ball(), 0xe0);
            let ch = &chars[p];
            if mode == 0 && motion == 0x20 && !(fading && (old == 0x23 || old == 0x24)) {
                let got = serve::stance_ball(ch.ball.at(t), [rows[0], rows[1], rows[2]], pos);
                assert_eq!(got.map(f32::to_bits), want.map(f32::to_bits), "k={k} p={p} t={t}");
                stance += 1;
            } else if (mode == 1 || (mode == 2 && blk[0x2c0] == 0 && blk[0x248] == 0)) && !fading && ch.clips.contains_key(&motion) {
                let local = ch.clips[&motion].locals(&ch.sk, t);
                let got = transform(&node_world(&ch.sk, &local, ch.finger, &rows), serve::HAND_BALL);
                assert_eq!(got[..3].iter().map(|v| v.to_bits()).collect::<Vec<_>>(), want.map(f32::to_bits), "k={k} p={p} motion {motion:#x} t={t}");
                hand += 1;
            }
        }
    }
    eprintln!("{stance} stance frames bit-exact; {hand} hand frames bit-exact");
    assert!(stance > 500 && hand > 50);
}
