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

/// B49/B164: the change-ends move against the original's two recorded glides (`research/b174_swing_probe.py`:
/// slot 5, and the Grand Palais with the camera on the human's side, `context/b174/orig/`): from the saved start
/// view (shot 2's first) to the match camera's view about court spot 0x76, every tick's eye and look as the
/// game's (skipped without the recordings).
#[test]
fn change_ends_swing() {
    use hst_sim::camera::{SWING, swing, swing_weight};
    assert!(swing_weight(SWING - 1) < 1e-3 && swing_weight(0) == 1.0);
    assert!((1..SWING).all(|k| swing_weight(k - 1) >= swing_weight(k)));
    for name in ["palais", "slot5"] {
        let path = format!("{}/../../context/b174/orig/{name}.txt", env!("CARGO_MANIFEST_DIR"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            eprintln!("no {path}, skipped");
            continue;
        };
        // per line: vsync, the counter, the shown view (eye, forward, down, fov in degrees), the frame, the
        // saved start (4 rows), its fov (half-angle), the pivot
        let rows: Vec<(u32, View, View, [f32; 3])> = text
            .lines()
            .map(|l| {
                let w: Vec<&str> = l.split_whitespace().collect();
                let at = |k: &str| w.iter().position(|x| *x == k).unwrap() + 1;
                let n = |i: usize| w[i].parse::<f32>().unwrap();
                let v = |i: usize| [n(i), n(i + 1), n(i + 2)];
                let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
                let s = at("shown");
                let (fwd, down) = (v(s + 4), v(s + 8));
                let shown = View { rot: [cross(down, fwd), down, fwd], eye: v(s), fov: n(s + 12) * 0.5 * 0.017453292 };
                let t = at("start");
                let start = View { rot: [v(t), v(t + 4), v(t + 8)], eye: v(t + 12), fov: n(at("fov0")) };
                (w[2].parse().unwrap(), shown, start, v(at("pivot")))
            })
            .collect();
        let to = rows.last().unwrap().1;
        let mut worst = (0.0f32, 0.0f32);
        for (left, shown, start, pivot) in rows.iter().filter(|r| (1..SWING).contains(&r.0)) {
            let v = swing(start, &to, swing_weight(*left), *pivot);
            let e = (0..3).map(|j| (v.eye[j] - shown.eye[j]).abs()).fold(0.0, f32::max);
            let r = (0..3).map(|j| (v.rot[2][j] - shown.rot[2][j]).abs()).fold(0.0, f32::max);
            worst = (worst.0.max(e), worst.1.max(r));
        }
        eprintln!("{name}: worst eye {:.4} m, look {:.4}", worst.0, worst.1);
        // the rest (under 2 cm, near the end) is the game's end view: its live match camera wobbles a little
        // where the port's is fixed (the recording's last tick stands in for it here)
        assert!(worst.0 < 0.02 && worst.1 < 0.001, "{name}: {worst:?}");
    }
}
