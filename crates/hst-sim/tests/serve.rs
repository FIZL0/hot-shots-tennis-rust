//! The serve against every serve of the live doubles matches (`MATCHES`): toss release point, apex and launch
//! velocity, and the contact frame the swing press locks onto. Per-player timing tables, the line-up, hands and
//! court come from each match's save state's RAM.

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

/// The live doubles matches with their save states' RAM: slot 5 (`match_s05.bin`, court 10, characters 0, 2,
/// 1, 5, all right-handed) and `1p3goodcpus.bin` (court 11, Carol, Will, 2 and 10; the first two left-handed).
const MATCHES: [(&str, &str); 2] = [("match_s05.bin", "slot5_ee.bin"), ("1p3goodcpus.bin", "1p3goodcpus_ee.bin")];

/// Each match present: its name, recording and RAM.
fn matches() -> Vec<(&'static str, Vec<u8>, Vec<u8>)> {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    MATCHES.into_iter().filter_map(|(m, r)| match (std::fs::read(format!("{dir}/{m}")), std::fs::read(format!("{dir}/{r}"))) {
        (Ok(data), Ok(ram)) => Some((m, data, ram)),
        _ => { eprintln!("{m} or {r} absent, skipped"); None }
    }).collect()
}
fn ru(ram: &[u8], a: usize) -> usize {
    u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize
}
/// Player `p`'s character (the menu's player slot) and hand (+1 right, −1 left), and the court.
fn character(ram: &[u8], p: usize) -> usize {
    ram[0x2ef7f4 + 12 * p] as usize
}
fn hand(ram: &[u8], p: usize) -> f32 {
    f(ram, ru(ram, ru(ram, 0x422f80) + 0xa8 + 4 * p) + 0x12b4)
}
fn court(ram: &[u8]) -> usize {
    ru(ram, 0x422f90)
}

/// Player `p`'s serve data in a match: heights as recorded in `fr`, timing and depth tables, the toss hand and
/// racket and the stats from the save state's RAM.
fn ram_data(ram: &[u8], p: usize, fr: Frame) -> ServeData {
    let ru = |a: usize| ru(ram, a);
    let obj = ru(ru(0x422f80) + 0xa8 + 4 * p);
    let ri = |o: usize| ru(obj + o) as i32;
    let grades = |o: usize, n: usize| ram[obj + o..obj + o + ru(obj + n)].to_vec();
    let bias = |o: usize, n: usize| (0..ru(obj + n)).map(|k| ri(o + 4 * k)).collect();
    let v4 = |o: usize| std::array::from_fn(|k| f(ram, obj + o + 4 * k));
    ServeData {
        over: [0x13cc, 0x13d0, 0x13d4].map(|o| fr.player_f32(p, o)),
        under: [0x13d8, 0x13dc, 0x13e0].map(|o| fr.player_f32(p, o)),
        strong_grades: grades(0x1644, 0x1680),
        weak_grades: grades(0x1774, 0x17b0),
        hand_over: v4(0x1840),
        hand_under: v4(0x1850),
        racket_over: std::array::from_fn(|r| v4(0x1860 + 16 * r)),
        racket_under: std::array::from_fn(|r| v4(0x18a0 + 16 * r)),
        miss: [50, 30, 100],
        strong_bias: bias(0x1554, 0x1680),
        weak_bias: bias(0x1684, 0x17b0),
        reach: [ri(0x1364), ri(0x1368)],
        short_miss: fr.player_f32(p, 0x13a8),
        power: ri(0x12e4),
        low_power: ri(0x1398),
        max_angle: 22.0,
    }
}

#[test]
fn match_s05_serves() {
    for (name, data, ram) in matches() {
        let frames = frames_live(&data);
        let (mut tosses, mut swings, mut misses, mut lefty, mut under, mut carol) = (0, 0, 0, 0, 0, 0);
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            // only where the recorder kept up (one game tick per sample; it falls behind at the very end)
            let tick = |f: Frame| u32::from_le_bytes(f.gm()[0x58..0x5c].try_into().unwrap());
            if tick(fr) != tick(a) + 1 {
                continue;
            }
            for p in 0..4 {
                let toss = toss_of(pi(fr, p, 0x3ea0));
                let d = ram_data(&ram, p, fr);
                if pu8(fr, p, 0x3ec0) == 1 && pu8(a, p, 0x3ec0) == 0 {
                    let player: [[f32; 4]; 4] = std::array::from_fn(|r| std::array::from_fn(|c| fr.player_f32(p, 0x3d40 + 16 * r + 4 * c)));
                    let (hand, apex) = serve::toss_points(&d, toss, &player);
                    let rec_hand = [0x3eb0, 0x3eb4, 0x3eb8].map(|o| fr.player_f32(p, o));
                    let rec_apex = [0x3e90, 0x3e94, 0x3e98].map(|o| fr.player_f32(p, o));
                    let close = |x: [f32; 3], y: [f32; 3], e: f32| (0..3).all(|j| (x[j] - y[j]).abs() < e);
                    assert_eq!([hand, apex].map(|v| v.map(f32::to_bits)), [rec_hand, rec_apex].map(|v| v.map(f32::to_bits)), "{name} vsync {} p{p} (character {}) {toss:?}: hand {hand:?} vs {rec_hand:?}, apex {apex:?} vs {rec_apex:?}", fr.vsync(), character(&ram, p));
                    carol += (character(&ram, p) == 6) as i32;
                    let vel = serve::toss_velocity(rec_hand, rec_apex);
                    let rec_vel = v3(fr.live_ball(), 0x130);
                    assert!(close(vel, rec_vel, 1e-5), "{name} vsync {} p{p}: toss velocity {vel:?} vs {rec_vel:?}", fr.vsync());
                    tosses += 1;
                }
                if pu8(fr, p, 0x3fa6) == 3 && pu8(a, p, 0x3fa6) != 3 {
                    let (mut fl, shot) = load(fr.live_ball());
                    let mut path = vec![];
                    for _ in 0..d.grades(toss).len() {
                        path.push(PathPoint { pos: fl.ball.pos, bounces: fl.bounces });
                        fl.step(&shot, &COURTS[court(&ram)]);
                    }
                    let got = serve::search(&d, toss, &path);
                    let off = pi(fr, p, 0x3fa0);
                    let want = (off != 999).then_some(off + serve::SWEET_FRAME);
                    if got.map(|k| k as i32) != want {
                        eprintln!("{name} vsync {} p{p} {toss:?}: contact frame {got:?} vs {want:?}", fr.vsync());
                        misses += 1;
                    }
                    swings += 1;
                    lefty += (hand(&ram, p) < 0.0) as i32;
                    under += (toss == Toss::Under) as i32;
                }
            }
        }
        eprintln!("{name}: {tosses} tosses ({carol} Carol's, left-handed), {swings} swings ({lefty} left-handed, {under} underhand), {misses} misses");
        assert_eq!(misses, 0, "{name}");
        // 1p3goodcpus: Carol (left-handed) serves all five, the underhand one at vsync ~20479
        let (n, l) = if name == "match_s05.bin" { (30, 0) } else { (5, 5) };
        assert!(tosses >= n && swings >= n && lefty >= l && carol >= l && under >= 1, "{name}: {tosses} tosses ({carol} Carol's), {swings} swings, {lefty} left-handed, {under} underhand");
    }
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

/// Each match player's toss hand and racket (`ServeData`, kept in RAM at +0x1840..+0x18e0) sampled from their
/// character's motions on the disc: bit-exact for characters 0, 1, 2 and 5; within 4e-6 for 6, 10 and 11 (the
/// left-handers Carol and Will among them; the sampler's rounding there is not yet the game's).
#[test]
fn toss_setup_ram() {
    use hst_data::{ani, iso::Iso, mdl, xb::Archive};
    use hst_sim::pose::{Clip, Skeleton};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(mut iso) = Iso::open(format!("{root}Hot Shots Tennis (USA).iso")) else { return eprintln!("ISO absent, skipped") };
    let (mut seen, mut exact) = (0, 0);
    for (name, _, ram) in matches() {
        for p in 0..4 {
            let c = character(&ram, p);
            let arc_data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
            let arc = Archive::parse(&arc_data).unwrap();
            let md = iso.read(&format!("PC/PC{c:02}C00.XB")).unwrap();
            let marc = Archive::parse(&md).unwrap();
            let m = mdl::parse(&marc.read(marc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl")).unwrap()).unwrap()).unwrap();
            let sk = Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() };
            let clip = |k: usize| {
                let stem = ani::motion_name(k, c).unwrap();
                Clip::new(&sk, &ani::parse(&arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).unwrap()).unwrap()).unwrap())
            };
            let obj = ru(&ram, ru(&ram, 0x422f80) + 0xa8 + 4 * p);
            let bits = |o: usize, n: usize| (0..n).map(|k| ru(&ram, obj + o + 4 * k) as u32).collect::<Vec<_>>();
            for (k, hand_at, racket_at) in [(0x23, 0x1840, 0x1860), (0x24, 0x1850, 0x18a0)] {
                let (hand, racket) = serve::toss_setup(&sk, &clip(k), &clip(k + 2));
                let off = |got: &[f32], want: Vec<u32>| got.iter().zip(want).map(|(g, w)| (g - f32::from_bits(w)).abs()).fold(0.0, f32::max);
                let e = off(&hand, bits(hand_at, 4)).max(off(racket.as_flattened(), bits(racket_at, 16)));
                assert!(e < 1e-5, "{name} p{p} character {c} motions {k:#x}/{:#x}: off by {e:e}", k + 2);
                exact += (e == 0.0) as i32;
            }
            seen += 1;
        }
    }
    eprintln!("{seen} players' toss setups, {exact} of {} overhand/underhand pairs bit-exact", 2 * seen);
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
    let (mut stance, mut hand, mut bounces) = (0, 0, 0);
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
                // the stance loop is the bounce: the track takes the ball down to the court (y up is negative, radius ~0.06) twice a loop
                bounces += (got[1] > -0.07) as i32;
            } else if (mode == 1 || (mode == 2 && blk[0x2c0] == 0 && blk[0x248] == 0)) && !fading && ch.clips.contains_key(&motion) {
                let local = ch.clips[&motion].locals(&ch.sk, t);
                let got = transform(&node_world(&ch.sk, &local, ch.finger, &rows), serve::HAND_BALL);
                assert_eq!(got[..3].iter().map(|v| v.to_bits()).collect::<Vec<_>>(), want.map(f32::to_bits), "k={k} p={p} motion {motion:#x} t={t}");
                hand += 1;
            }
        }
    }
    eprintln!("{stance} stance frames bit-exact ({bounces} on the court); {hand} hand frames bit-exact");
    assert!(stance > 500 && hand > 50 && bounces > 20);
}


/// Every serve's launch against the game's (velocity within 2e-5 as for smashes, the launch's trig not being
/// the game's own; flight frames and spin exact): the server's own trajectory table (the `dw1` variant
/// for a weak toss), looked up from the contact less the scatter toward the aim, and the spin of the
/// character's shot record (the variant's for a weak toss). Slice serves (kind 1) bend by a wind not ported
/// yet (P6) and are skipped.
#[test]
fn serves_launch_like_the_game() {
    use hst_sim::params::{ShotParams, record_of, spin};
    use hst_sim::shot::Table;
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let (Ok(cnf), Ok(bin)) = (std::fs::read(format!("{ctx}/iso/SYSTEM.CNF")), std::fs::read(format!("{ctx}/iso/ZZBIN/GAME.BIN"))) else {
        return eprintln!("the extracted disc absent, skipped");
    };
    let game = hst_data::exe::Game::new(&cnf, &bin).unwrap();
    let src = game.shot_params();
    let params = ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
    let table = |c: usize, k: i32, dw: bool| {
        let (ab, sfx) = if dw { ("B", "_dw1") } else { ("A", "") };
        Table::parse(&std::fs::read(format!("{ctx}/xb/TRAJ/TRAJ{c:02}{ab}.XB/data/hatsuyama/traj/tr_pc{c:02}_serv{k}{sfx}.dat")).unwrap()).unwrap()
    };
    let launched = |x: &[u8]| x[0x58] == 0 && i(x, 0xac) == 0 && f(x, 0x130).abs() + f(x, 0x138).abs() > 0.05;
    for (name, data, ram) in matches() {
        let (mut checked, mut lefty) = ([0; 3], 0);
        for w in frames_live(&data).windows(2) {
            let b = w[1].live_ball();
            if !launched(b) || launched(w[0].live_ball()) || i(b, 0x5c) == 1 {
                continue;
            }
            let kind = i(b, 0x5c);
            let p = (0..4).find(|&p| pu8(w[1], p, 0x3ec1) == 0 && pu8(w[1], p, 0x3fa6) == 3).expect("server");
            let c = character(&ram, p);
            let weak = pi(w[1], p, 0x3ea0) == 2;
            let hit = v3(b, 0x70);
            let aim = serve::inside(w[1].player_pos(p)[0], hit, [0x3e90, 0x3e94, 0x3e98].map(|o| w[1].player_f32(p, o)));
            let scatter = [f(b, 0x80) - aim[0], 0.0, f(b, 0x88) - aim[2]];
            let (vel, frames) = serve::launch(&table(c, kind, weak), kind == 3, Params::default().radius, hit, aim, scatter);
            let want = v3(b, 0x130);
            let at = format!("{name} vsync {} player {p} (character {c}, hand {}) kind {kind}", w[1].vsync(), hand(&ram, p));
            assert!((0..3).all(|j| (vel[j] - want[j]).abs() < 2e-5), "{at}: {vel:?} vs {want:?}");
            assert_eq!(frames + 1, i(b, 0x260), "{at}");
            let rec = if weak { params.variant(0, kind as usize, record_of(c), -0.5) } else { params.record(0, kind as usize, record_of(c)).try_into().unwrap() };
            assert_eq!(spin(&rec).to_bits(), f(b, 0x1a4).to_bits(), "{at}: spin");
            checked[if weak { 1 } else if kind == 3 { 2 } else { 0 }] += 1;
            lefty += (hand(&ram, p) < 0.0) as i32;
        }
        eprintln!("{name}: strong/weak/underhand serves {checked:?}, {lefty} left-handed");
        // 1p3goodcpus has no weak toss; its slice (kind 1) is skipped
        let (want, l) = if name == "match_s05.bin" { ([1, 1, 1], 0) } else { ([3, 0, 1], 4) };
        assert!((0..3).all(|k| checked[k] >= want[k]) && lefty >= l, "{name}: {checked:?}, {lefty} left-handed");
    }
}

/// Every serve's error off its aim against the game's, bit-exact: the depth error from the timing table's bias
/// and the contact height (+0x3ecc), and where the ball is sent (the aim pulled inside the box + scatter, the
/// ball's +0x80); the weak toss's `dw1` choice from the stats.
#[test]
fn serves_scatter_like_the_game() {
    use hst_sim::ps2;
    use hst_sim::serve::Miss;
    let launched = |x: &[u8]| x[0x58] == 0 && i(x, 0xac) == 0 && f(x, 0x130).abs() + f(x, 0x138).abs() > 0.05;
    for (name, data, ram) in matches() {
        let (mut serves, mut off, mut lefty, mut under, mut pulled) = (0, 0, 0, 0, 0);
        for w in frames_live(&data).windows(2) {
            let (fr, b) = (w[1], w[1].live_ball());
            if !launched(b) || launched(w[0].live_ball()) {
                continue;
            }
            let p = (0..4).find(|&p| pu8(fr, p, 0x3ec1) == 0 && pu8(fr, p, 0x3fa6) == 3).unwrap_or_else(|| panic!("server at vsync {}", fr.vsync()));
            let d = ram_data(&ram, p, fr);
            let toss = toss_of(pi(fr, p, 0x3ea0));
            let (k, grade) = ((pi(fr, p, 0x3fa0) + serve::SWEET_FRAME) as usize, pu8(fr, p, 0x3ee8));
            let hit = v3(b, 0x70);
            let raw = [0x3e90, 0x3e94, 0x3e98].map(|o| fr.player_f32(p, o));
            let aim = serve::inside(fr.player_pos(p)[0], hit, raw);
            pulled += (aim != raw) as usize;
            let at = format!("{name} vsync {} player {p} {toss:?}", fr.vsync());
            assert_eq!(d.bias(toss)[k], pi(fr, p, 0x3f98), "{at}");
            assert_eq!(ps2::sub(-hit[1], d.window(toss)[1]).to_bits(), fr.player_f32(p, 0x3f9c).to_bits(), "{at}");
            let error = serve::depth_error(&d, toss, k, grade, -hit[1]);
            assert_eq!(error, pi(fr, p, 0x3ecc), "{at}");
            assert_eq!(pi(fr, p, 0x3ed8), if toss == Toss::Weak { -5 } else { 0 }, "{at}");
            assert!(toss != Toss::Weak || serve::dw1(&d), "{at}");
            let miss = Miss { side: fr.player_f32(p, 0x3f10), depth: fr.player_f32(p, 0x3f18), nudge: pi(fr, p, 0x3ed0) };
            let s = serve::scatter(&d, toss, miss, error, hit, aim);
            assert_eq!([ps2::add(aim[0], s[0]).to_bits(), ps2::add(aim[2], s[2]).to_bits()], [f(b, 0x80).to_bits(), f(b, 0x88).to_bits()], "{at}: {s:?}");
            serves += 1;
            off += (s != [0.0; 3]) as usize;
            lefty += (hand(&ram, p) < 0.0) as usize;
            under += (toss == Toss::Under) as usize;
        }
        eprintln!("{name}: {serves} serves ({lefty} left-handed, {under} underhand), {off} off their aim, {pulled} pulled inside");
        // 1p3goodcpus: Carol's aims at the box's corners, pulled inside before the launch
        let (n, o, l) = if name == "match_s05.bin" { (31, 11, 0) } else { (5, 0, 5) };
        assert!(serves >= n && off >= o && lefty >= l && pulled >= l && under >= 1, "{name}: {serves} serves, {off} off, {pulled} pulled, {lefty} left-handed, {under} underhand");
    }
}
