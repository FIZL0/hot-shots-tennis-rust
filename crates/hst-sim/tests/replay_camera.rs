//! The instant replay's shots on the original's own reference frames (`context/fixtures/replay_frames_s05.bin`,
//! `research/b172_frames.py` from slot 5's match-point replay): fed the recorded frame table, the port's shot
//! puts the eye where the game's camera is. Per vsync: u32 vsync, u8 camera handler +0x51, u32 shot camera +0x114, the frame
//! table (0x79 × 0x40), 0x58 bytes of recorder roles, the camera eye/look/up/fov (0x40).

use hst_data::{exe::Game, iso::Iso};
use hst_sim::cutaway::{M4, Shot};

const TAB: usize = 9;
const CAM: usize = TAB + 0x79 * 0x40 + 0x58;
const SAMPLE: usize = CAM + 0x40;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn mat(b: &[u8], o: usize) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| f(b, o + 16 * r + 4 * c)))
}

#[test]
fn replay_shots_on_the_games_frames() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(mut iso)) = (std::fs::read(format!("{dir}/replay_frames_s05.bin")), Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"))) else {
        return eprintln!("replay_frames_s05.bin or ISO absent, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let shots = Game::new(&cnf, &bin).unwrap().camera_shots();
    let (mut shot, mut run): (Option<Shot>, usize) = (None, 0);
    let mut worst = [(0u8, 0.0f32, 0.0f32); 2];
    for s in data.as_chunks::<SAMPLE>().0 {
        // the shot camera's own byte (+0x114) changes at the cut, the handler's (+0x51) some frames later; during
        // 0x38 the former reads 6 (meaning not traced)
        let number = match s[5] {
            6 => s[4],
            n => n,
        };
        if shot.as_ref().is_none_or(|sh| sh.number != number) {
            (shot, run) = (Some(Shot::new(&shots, number, false)), 0);
        }
        let sh = shot.as_mut().unwrap();
        sh.step(|i| mat(s, TAB + i as usize * 0x40));
        run += 1;
        // the game's camera eases in over the first frames of a shot
        if run < 30 {
            continue;
        }
        let v = sh.view();
        let eye = (0..3).map(|i| (v.eye[i] - f(s, CAM + 4 * i)).powi(2)).sum::<f32>().sqrt();
        let look = (0..3).map(|i| (v.rot[2][i] - f(s, CAM + 16 + 4 * i)).powi(2)).sum::<f32>().sqrt();
        let w = &mut worst[(number == 0x53) as usize];
        *w = (number, w.1.max(eye), w.2.max(look));
    }
    eprintln!("worst (shot, eye m, look) {worst:x?}");
    // 0x38 (the recorded pelvis of the winning shot's hitter): on the game's eye
    assert_eq!(worst[0].0, 0x38);
    assert!(worst[0].1 < 0.05 && worst[0].2 < 0.2, "0x38 off the game's camera");
    // 0x53 (the closing shot, the last hitter's head looking at the last bounce): looks the same way
    assert_eq!(worst[1].0, 0x53);
    assert!(worst[1].2 < 0.1, "0x53 looks elsewhere");
}
