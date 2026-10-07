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
    let frames: Vec<&[u8]> = data.chunks_exact(SAMPLE).collect();
    let vsync = |s: &[u8]| u32::from_le_bytes(s[0..4].try_into().unwrap());
    let (mut shot, mut prev): (Option<Shot>, Option<Shot>) = (None, None);
    let (mut n, mut worst, mut stale, mut skipped) = (0, 0.0f32, 0, std::collections::BTreeSet::new());
    for (k, s) in frames.iter().enumerate() {
        let number = s[CAM + 0x114];
        let new_run = k == 0 || vsync(s) != vsync(frames[k - 1]) + 1 || number != frames[k - 1][CAM + 0x114];
        let table = |i: u8| mat(s, TAB + i as usize * 0x40);
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
        // the court views (0x0c, 0x0d, 0x11) frame several players (not ported yet)
        if matches!(number, 0x0c | 0x0d | 0x11) {
            skipped.insert(number);
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
        if err > worst {
            worst = err;
            eprintln!("frame {k} shot {number:#x}: err {err:.6}");
        }
        n += 1;
    }
    eprintln!("{n} frames checked, worst {worst}; {stale} stale samples; court views skipped: {skipped:x?}");
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
    let s = data.chunks_exact(SAMPLE).nth(k).unwrap();
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
