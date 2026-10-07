//! Replays the live ball of a recorded bot match (`context/live/net_s05.bin`, not in git; made by
//! `tools/record_live.py 5 …` from save-state slot 5, court 10) one frame at a time: each frame's ball object is
//! loaded into a `Flight`, stepped once against court 10's collision world (built from the disc: the court model
//! and the grid of props) and compared bit for bit with the next recorded frame: airborne frames and every contact
//! (court, ground around it, walls, net, net cord, the ghost material) must match exactly. Skips when the recording
//! or the disc is absent.

use hst_data::exe;
use hst_data::iso::Iso;
use hst_sim::ball::{Ball, COURTS, Flight, Material, Params, Shot};
use hst_sim::court;

const SAMPLE: usize = 4 + 0x290 + 0x290 + 0x40;
const COURT: usize = 10;

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

/// The ball object (0x290 bytes from the live ball pointer `*(gm+0x88)`) as a flight and its shot.
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

struct Replay {
    exact: usize,
    /// Contact frames by material (+0x220).
    touches: std::collections::BTreeMap<u8, usize>,
    failures: Vec<String>,
}

/// A `record_live.py` file as (vsync, live ball) samples.
fn samples(data: &[u8]) -> Vec<(i32, &[u8])> {
    data.chunks_exact(SAMPLE).map(|w| (i(w, 0), &w[4..4 + 0x290])).collect()
}

/// Steps every recorded frame pair once against `court` and compares the next frame.
/// `poked`: vsyncs after which the recorder rewrote the ball's velocity (`--aim`); the two frames after are skipped.
fn replay(s: &[(i32, &[u8])], poked: &[i32], court: usize, world: &hst_sim::mesh::World, materials: &[Material]) -> Replay {
    let mut r = Replay { exact: 0, touches: Default::default(), failures: Vec::new() };
    for w in s.windows(2) {
        let ((va, a), (vsync, b)) = (w[0], w[1]);
        // only frames the ball physically flies: consecutive vsyncs, same shot, in play (+0xa4 0 or 1)
        // and not carried (the server's toss/bounce moves it with zero velocity)
        let carried = |o: &[u8]| v3(o, 0x130) == [0.0; 3];
        if vsync != va + 1 || i(b, 0xac) != i(a, 0xac) + 1 || a[0xa4] > 1 || b[0xa4] > 1 || carried(a) || carried(b) {
            continue;
        }
        if poked.iter().any(|&v| vsync == v + 1 || vsync == v + 2) {
            continue;
        }
        let (mut fl, shot) = load(a);
        fl.step_world(&shot, &COURTS[court], world, materials);
        let want = [v3(b, 0xe0), v3(b, 0x130)].concat();
        let got = [fl.ball.pos, fl.ball.vel].concat();
        let same = (0..6).all(|k| got[k].to_bits() == want[k].to_bits());
        // a contact moves a counter or changes the material of the last contact
        if i(b, 0x224) != i(a, 0x224) || i(b, 0x228) != i(a, 0x228) || i(b, 0x22c) != i(a, 0x22c) || b[0x220] != a[0x220] {
            *r.touches.entry(b[0x220]).or_insert(0) += 1;
        }
        if same {
            r.exact += 1;
        } else {
            r.failures.push(format!("vsync {vsync} material {} pos {:?}: got {got:?} want {want:?}", b[0x220], v3(b, 0xe0)));
        }
    }
    r
}

fn disc(court: u32) -> Option<(hst_sim::mesh::World, Vec<Material>)> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(mut iso), Ok(cnf), Ok(bin)) = (
        Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")),
        std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")),
        std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")),
    ) else {
        return None;
    };
    Some((court::world(&mut iso, court), court::materials(&exe::Game::new(&cnf, &bin).unwrap())))
}

#[test]
fn live_ball_frames_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Some((world, materials))) = (std::fs::read(format!("{root}/context/live/net_s05.bin")), disc(COURT as u32)) else {
        eprintln!("recording or disc missing, skipped");
        return;
    };
    let r = replay(&samples(&data), &[], COURT, &world, &materials);
    eprintln!("{} frames bit-exact; contact frames by material: {:?}", r.exact, r.touches);
    assert!(r.failures.is_empty(), "{} frames diverged:\n{}", r.failures.len(), r.failures[..r.failures.len().min(20)].join("\n"));
    assert!(r.exact > 12000);
}

/// Aimed shots from slot 5 (`record_live.py 5 … --aim`, `context/live/p15/*.bin`): balls into the net post (22),
/// the net (26) and the cord (2) — clipped cords that dribble over, in rallies and on serves, and balls the net stops.
#[test]
fn aimed_net_cord_and_post_hits_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(dir), Some((world, materials))) = (std::fs::read_dir(format!("{root}/context/live/p15")), disc(COURT as u32)) else {
        eprintln!("recordings or disc missing, skipped");
        return;
    };
    let mut files: Vec<_> = dir.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "bin")).collect();
    files.sort();
    let mut touches = std::collections::BTreeMap::new();
    for path in files {
        let data = std::fs::read(&path).unwrap();
        let poke = std::fs::read_to_string(path.with_extension("bin.poke")).unwrap_or_default();
        let poked: Vec<i32> = poke.lines().filter_map(|l| l.trim().parse().ok()).collect();
        let r = replay(&samples(&data), &poked, COURT, &world, &materials);
        eprintln!("{}: {} frames bit-exact; contact frames by material: {:?}", path.display(), r.exact, r.touches);
        assert!(r.failures.is_empty(), "{}: {} frames diverged:\n{}", path.display(), r.failures.len(), r.failures[..r.failures.len().min(20)].join("\n"));
        for (m, n) in r.touches {
            *touches.entry(m).or_insert(0) += n;
        }
    }
    for m in [2, 22, 26] {
        assert!(touches.contains_key(&m), "no contact with material {m} recorded: {touches:?}");
    }
}

/// The live ball of `1p3goodcpus` (`context/fixtures/1p3goodcpus.bin`, tools/play_p2m2.py; a doubles match on court
/// 11 with one human): every flying frame bit-exact against court 11's world, including two net-cord balls that
/// drop over the net (vsyncs 19805, 20193) and one that falls back (21615).
#[test]
fn goodcpus_live_ball_matches_the_game() {
    use hst_sim::replay::frames_live;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Some((world, materials))) = (std::fs::read(format!("{root}/context/fixtures/1p3goodcpus.bin")), disc(11)) else {
        return eprintln!("1p3goodcpus.bin or disc missing, skipped");
    };
    let frames = frames_live(&data);
    assert_eq!(frames[0].global(0x422f90), 11, "court");
    let s: Vec<(i32, &[u8])> = frames.iter().map(|f| (f.vsync() as i32, f.live_ball())).collect();
    let r = replay(&s, &[], 11, &world, &materials);
    eprintln!("{} frames bit-exact; contact frames by material: {:?}", r.exact, r.touches);
    assert!(r.failures.is_empty(), "{} frames diverged:\n{}", r.failures.len(), r.failures[..r.failures.len().min(20)].join("\n"));
    assert!(r.exact > 2000 && r.touches.get(&2).is_some_and(|&n| n >= 5), "{:?}", r.touches);
}

#[test]
fn every_court_builds_its_world() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    for n in 1..=11 {
        let w = court::world(&mut iso, n);
        assert!(!w.models[0].tris.is_empty(), "court {n} has no collision triangles");
    }
}

#[test]
fn net_touch_keeps_the_predicted_contacts() {
    // On a net touch the game re-seeds its predicted path (`*(gm+0x98)`, the second ball object of each sample)
    // from the live ball but keeps the court contacts the old prediction had reached: in a rally those are 2,
    // past every contact search, so a net cord that drops over goes unplayed and wins the point for the hitter.
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(data) = std::fs::read(format!("{root}/context/live/net_s05.bin")) else {
        return eprintln!("recording missing, skipped");
    };
    let s: Vec<&[u8]> = data.chunks_exact(SAMPLE).collect();
    fn live(w: &[u8]) -> &[u8] {
        &w[4..4 + 0x290]
    }
    fn pred(w: &[u8]) -> &[u8] {
        &w[4 + 0x290..4 + 0x520]
    }
    let mut seen = 0;
    for k in 1..s.len() {
        let (a, b) = (live(s[k - 1]), live(s[k]));
        if i(a, 0x22c) != 0 || i(b, 0x22c) == 0 || i(s[k], 0) != i(s[k - 1], 0) + 1 {
            continue;
        }
        // the hit: the last sample before the touch whose shot frame is 0
        let Some(h) = (0..k).rev().find(|&h| i(live(s[h]), 0xac) == 0) else { continue };
        let (hit, shot) = load(live(s[h]));
        let t = i(a, 0xac) as u32;
        let want = i(pred(s[k]), 0x228);
        eprintln!("vsync {} t {t}: predictor contacts {want}, live {}", i(s[k], 0), i(b, 0x228));
        assert_eq!(hit.predicted_contacts(&shot, &COURTS[COURT], t), want, "vsync {}", i(s[k], 0));
        seen += 1;
    }
    assert!(seen >= 8);
}
