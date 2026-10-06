//! The match camera against a recorded bot match (`context/fixtures/camera_s05.bin`, `tools/record_camera.py 5`):
//! every serve and rally frame, the port's view (eye, forward axis, field of view) follows the game's from the
//! same players and ball.

use hst_sim::camera::{Camera, Scene, View};

const GM: usize = 4;
const CH: usize = GM + 0x100;
const CAM: usize = CH + 0x200;
const PLAYERS: usize = CAM + 0x140 + 2 * (0x200 + 0x1c0);
const BALL: usize = PLAYERS + 4 * 0x40;
const SAMPLE: usize = BALL + 0x290;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v3(b: &[u8], o: usize) -> [f32; 3] {
    [f(b, o), f(b, o + 4), f(b, o + 8)]
}
fn view(s: &[u8]) -> View {
    let m = CAM + 0x60;
    View { rot: [v3(s, m), v3(s, m + 0x10), v3(s, m + 0x20)], eye: v3(s, m + 0x30), fov: f(s, CAM + 0xa4) }
}

#[test]
fn match_s05_camera() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/camera_s05.bin")) else {
        return eprintln!("camera_s05.bin absent, skipped");
    };
    let frames: Vec<&[u8]> = data.chunks_exact(SAMPLE).collect();
    let (rest, base) = (view(frames[0]), hst_sim::camera::base());
    assert!((0..3).all(|j| (rest.eye[j] - base.eye[j]).abs() < 2e-3) && (rest.fov - base.fov).abs() < 1e-6, "resting view {base:?} vs {rest:?}");
    let mut cam = Camera::from_view(view(frames[0]));
    let (mut worst, mut n) = (0.0f32, 0);
    for (k, s) in frames.iter().enumerate().skip(1) {
        let phase = s[GM + 0x55];
        if !(phase == 2 || phase == 3) {
            cam = Camera::from_view(view(s));
            continue;
        }
        if phase == 2 && frames[k - 1][GM + 0x55] != 2 {
            cam.cut();
        }
        let players: Vec<[f32; 3]> = (0..4).map(|i| v3(s, PLAYERS + i * 0x40 + 0x30)).collect();
        cam.step(&Scene { players: &players, ball: v3(s, BALL + 0xe0) });
        let want = view(s);
        let err = (0..3).map(|j| (cam.view.eye[j] - want.eye[j]).abs()).fold(0.0, f32::max).max((cam.view.rot[2][1] - want.rot[2][1]).abs() * 10.0).max((cam.view.fov - want.fov).abs() * 10.0);
        if err > worst {
            worst = err;
            eprintln!("frame {k}: err {err:.4} port eye {:?} fov {} fwd {:?} | game eye {:?} fov {} fwd {:?}", cam.view.eye, cam.view.fov, cam.view.rot[2], want.eye, want.fov, want.rot[2]);
        }
        n += 1;
    }
    eprintln!("{n} frames, worst {worst}");
    assert!(worst < 0.01, "worst error {worst}");
}
