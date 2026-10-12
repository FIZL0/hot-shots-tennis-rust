//! The court intro's camera flyover: the script built from each court's camera file reaches the file's poses
//! (the orbit segments too), and court 8's channels and eye follow a recording of the original
//! (`context/b42/cam_c08.bin`, `research/b42_intro_camera.py`).

use hst_data::{exe::Game, iso::Iso, xb::Archive};
use hst_sim::cutaway::{self, Shot};

const CAM: usize = 4 + 0x100 + 0x200;
const SAMPLE: usize = CAM + 0x3100;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

#[test]
fn intro_cameras() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("ISO absent, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let shots = Game::new(&cnf, &bin).unwrap().camera_shots();
    let mut c08 = Vec::new();
    for n in 1..=11 {
        let cmn = iso.read(&format!("COURT/{n:02}/CMN.XB")).unwrap();
        let arc = Archive::parse(&cmn).unwrap();
        let e = arc.entries.iter().find(|e| e.name.ends_with(&format!("cam_c{n:02}_00.dat"))).unwrap();
        let dat = arc.read(e).unwrap();
        // every record's eye, roll and fov reached at the end of its move (a cut lands two frames after the last
        // move ends)
        let mut sh = Shot::intro(&shots, &dat);
        let mut at = 0;
        for (i, r) in dat.chunks_exact(40).enumerate() {
            let time = (i16::from_le_bytes([r[0], r[1]]) as u32).max(2 * (i > 0) as u32);
            for _ in 0..time {
                sh.step(Shot::court);
            }
            at += time;
            let v = sh.view();
            let want = [f(r, 0xc), f(r, 0x10), f(r, 0x14)];
            for i in 0..3 {
                assert!((v.eye[i] - want[i]).abs() < 2e-3 * (1.0 + want[i].abs()), "court {n} frame {at}: eye {:?} vs {want:?}", v.eye);
            }
            assert!((v.fov - f(r, 0x24) * 0.008726646).abs() < 1e-5, "court {n} frame {at}: fov");
            assert!((sh.ch(7) - f(r, 0x20) * 0.017453292).abs() < 1e-5, "court {n} frame {at}: roll");
        }
        let cuts = dat.chunks_exact(40).skip(1).filter(|r| r[0] == 0 && r[1] == 0).count() as u32;
        assert_eq!(at, cutaway::intro_frames(&dat) + 2 * cuts);
        if n == 8 {
            c08 = dat;
        }
    }
    let Ok(rec) = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/b42/cam_c08.bin")) else {
        return eprintln!("cam_c08.bin absent, recording check skipped");
    };
    let frames: Vec<&[u8]> = rec.chunks_exact(SAMPLE).collect();
    let ch = |s: &[u8], c: usize| f(s, CAM + 0x2d58 + c * 0x1c);
    // the shot steps with the match's frame counter (it holds while the loading screen is up)
    let count = |s: &[u8]| u32::from_le_bytes(s[4 + 0x58..4 + 0x5c].try_into().unwrap());
    let start = frames.iter().position(|s| ch(s, 8) != 0.0).unwrap();
    let (mut sh, mut at, mut n) = (Shot::intro(&shots, &c08), 0, 0);
    let mut prev = sh.clone();
    for s in &frames[start..] {
        if count(s) > 300 {
            break;
        }
        while at < count(s) {
            prev = sh.clone();
            sh.step(Shot::court);
            at += 1;
        }
        // a sample read between the counter's tick and the camera's update has last frame's channels
        let eye = [f(s, CAM + 0x90), f(s, CAM + 0x94), f(s, CAM + 0x98)];
        let near = |sh: &Shot| {
            [5, 6, 7, 8, 9, 10, 14].iter().all(|&c| (sh.ch(c) - ch(s, c)).abs() < 1e-4 * (1.0 + ch(s, c).abs()))
                && (0..3).all(|i| (sh.view().eye[i] - eye[i]).abs() < 1e-3)
        };
        assert!(near(&sh) || near(&prev), "frame {at}: ch 6 {} vs {}, eye {:?} vs {eye:?}", sh.ch(6), ch(s, 6), sh.view().eye);
        n += 1;
    }
    assert!(n > 250, "{n} samples");
}
