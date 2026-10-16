//! The court intro's camera flyover: the script built from each court's camera file reaches the file's poses
//! (the orbit segments too), and the channels and eye follow recordings of the original (`research/b42_intro_camera.py`:
//! court 8 in `context/b42/cam_c08.bin`, courts 1–7, 9, 11 in `context/b42d/cam_cNN.bin`, all doubles).

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
    let mut recorded = 0;
    for n in 1..=11 {
        let cmn = iso.read(&format!("COURT/{n:02}/CMN.XB")).unwrap();
        let arc = Archive::parse(&cmn).unwrap();
        let e = arc.entries.iter().find(|e| e.name.ends_with(&format!("cam_c{n:02}_00.dat"))).unwrap();
        let dat = arc.read(e).unwrap();
        // every record's eye, roll and fov reached at the end of its move (a cut lands two frames after the last
        // move ends)
        let mut sh = Shot::intro(&shots, &dat);
        let mut at = 0;
        for (i, r) in dat.as_chunks::<40>().0.iter().enumerate() {
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
        let cuts = dat.as_chunks::<40>().0.iter().skip(1).filter(|r| r[0] == 0 && r[1] == 0).count() as u32;
        assert_eq!(at, cutaway::intro_frames(&dat) + 2 * cuts);
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
        let rec = [format!("{root}/b42/cam_c{n:02}.bin"), format!("{root}/b42d/cam_c{n:02}.bin")]
            .iter()
            .find_map(|p| std::fs::read(p).ok());
        if let Some(rec) = rec {
            follows(n, &shots, &dat, &rec);
            recorded += 1;
        }
    }
    eprintln!("{recorded} courts recorded");
}

/// Court `n`'s recorded shot camera (`rec`) against the port's, every sample up to the shot's end.
fn follows(n: u32, shots: &[hst_data::exe::CameraShot], dat: &[u8], rec: &[u8]) {
    let cuts = dat.as_chunks::<40>().0.iter().skip(1).filter(|r| r[0] == 0 && r[1] == 0).count() as u32;
    let end = cutaway::intro_frames(dat) + 2 * cuts;
    let frames: Vec<&[u8]> = rec.as_chunks::<SAMPLE>().0.iter().map(|s| s.as_slice()).collect();
    let ch = |s: &[u8], c: usize| f(s, CAM + 0x2d58 + c * 0x1c);
    // the shot steps with the match's frame counter (it holds while the loading screen is up)
    let count = |s: &[u8]| u32::from_le_bytes(s[4 + 0x58..4 + 0x5c].try_into().unwrap());
    let start = frames.iter().position(|s| u32::from_le_bytes(s[CAM + 0x114..CAM + 0x118].try_into().unwrap()) == 107).unwrap();
    let (mut sh, mut at, mut k) = (Shot::intro(shots, dat), 0, 0);
    let (mut prev, mut worst) = (sh.clone(), 0f32);
    for s in &frames[start..] {
        if count(s) > end {
            break;
        }
        while at < count(s) {
            prev = sh.clone();
            sh.step(Shot::court);
            at += 1;
        }
        // a sample read between the counter's tick and the camera's update has last frame's channels or eye (or
        // both: court 3's first frame has the new channels and the old eye); angles match
        // modulo 2π and the eye within 1 mm + 2e-4 of its distance (the orbit courts 1, 2, 6, 11 have frames up to
        // ~7 mm off at 40–55 m, e.g. where the eye crosses an axis or the yaw wraps; the other courts ~0)
        let eye = [f(s, CAM + 0x90), f(s, CAM + 0x94), f(s, CAM + 0x98)];
        let dist = eye.iter().map(|e| e * e).sum::<f32>().sqrt();
        let off = |sh: &Shot| (0..3).map(|i| (sh.view().eye[i] - eye[i]).abs()).fold(0.0, f32::max) / (1e-3 + 2e-4 * dist);
        let chans = |sh: &Shot| {
            [5, 6, 7, 8, 9, 10, 14].iter().all(|&c| {
                let d = (sh.ch(c) - ch(s, c)).abs();
                d.min((d - std::f32::consts::TAU).abs()) < 1e-4 * (1.0 + ch(s, c).abs())
            })
        };
        worst = worst.max(off(&sh).min(off(&prev)));
        assert!((chans(&sh) || chans(&prev)) && worst < 1.0, "court {n} frame {at}: ch 6 {} vs {}, eye {:?} vs {eye:?}", sh.ch(6), ch(s, 6), sh.view().eye);
        k += 1;
    }
    assert!(k + 50 > end, "court {n}: {k} samples of {end}");
    eprintln!("court {n}: {k} samples, worst eye error {worst:.2} of the tolerance");
}
