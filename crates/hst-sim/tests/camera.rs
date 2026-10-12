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

/// B49: the change-ends move starts on the view it was given, ends on the match camera's, and swings round
/// the court at the views' distance (it doesn't slide through the court).
#[test]
fn change_ends_swing() {
    use hst_sim::camera::{SWING, base, base_turned, swing, swing_weight};
    let (a, b, pivot) = (base(), base_turned(), [0.0; 3]);
    let close = |x: &View, y: &View| (0..3).all(|j| (x.eye[j] - y.eye[j]).abs() < 1e-3 && (x.rot[2][j] - y.rot[2][j]).abs() < 1e-4);
    assert!(close(&swing(&a, &b, 0.0, pivot), &a) && close(&swing(&a, &b, 1.0, pivot), &b));
    // halfway it looks across the court from beside it, as far out as it started
    let flat = |v: &View| v.eye[0].hypot(v.eye[2]);
    let mid = swing(&a, &b, 0.5, pivot);
    assert!((flat(&mid) - flat(&a)).abs() < 1e-2 && mid.eye[2].abs() < 1e-2, "{mid:?}");
    assert!(swing_weight(SWING - 1) < 1e-3 && swing_weight(0) == 1.0);
    assert!((1..SWING).all(|k| swing_weight(k - 1) >= swing_weight(k)));
}
