//! Per-tick state hashes for the cross-platform check (`tests/platforms.rs`): `state_hash <camera_s05.bin>`.
//! Replays a recorded bot match's players and ball through the match camera (as `tests/camera.rs`) and prints one
//! FNV-1a hash of the camera state per tick, then one hash line per platform-independent transcendental the
//! sim uses (`hst_sim::musl`, `hst_sim::libm`) over a sweep of inputs. Same build on x86_64, aarch64 and wasm32
//! must print the same lines. Inputs stay in each function's domain: NaN bits differ by platform (x86 makes −NaN,
//! ARM +NaN), so a NaN anywhere in the state would read as a desync.

use hst_sim::camera::{Camera, Scene, View};
use hst_sim::{libm, musl};

// tests/camera.rs's sample layout
const GM: usize = 4;
const CAM: usize = GM + 0x300;
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

struct Fnv(u64);
impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn f(&mut self, x: f32) {
        for b in x.to_bits().to_le_bytes() {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    fn d(&mut self, x: f64) {
        let [a, b] = [(x.to_bits() >> 32) as u32, x.to_bits() as u32];
        self.f(f32::from_bits(a));
        self.f(f32::from_bits(b));
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: state_hash <camera_s05.bin>");
    let data = std::fs::read(path).expect("read fixture");
    let frames: Vec<&[u8]> = data.chunks_exact(SAMPLE).collect();
    let mut cam = Camera::from_view(view(frames[0]));
    for (k, s) in frames.iter().enumerate().skip(1) {
        let phase = s[GM + 0x55];
        if !(phase == 2 || phase == 3) {
            cam = Camera::from_view(view(s));
        } else {
            if frames[k - 1][GM + 0x55] != 2 && phase == 2 {
                cam.cut();
            }
            let players: Vec<[f32; 3]> = (0..4).map(|i| v3(s, PLAYERS + i * 0x40 + 0x30)).collect();
            cam.step(&Scene { players: &players, ball: v3(s, BALL + 0xe0) });
        }
        let mut h = Fnv::new();
        let v = cam.view;
        v.rot.iter().flatten().chain(&v.eye).chain([&v.fov]).for_each(|&x| h.f(x));
        println!("{k} {:016x}", h.0);
    }
    // one hash per function, so a mismatch names it
    const NAMES: [&str; 21] = ["musl::sinf", "musl::cosf", "musl::tanf", "musl::atanf", "musl::asinf", "musl::atan2f",
        "musl::atan2f'", "musl::hypotf", "musl::powf", "musl::sincosf.0", "musl::sincosf.1", "libm::sinf", "libm::cosf",
        "libm::tanf", "libm::atanf", "libm::atan2f", "libm::acosf", "libm::powf", "libm::table_sin", "musl::tan",
        "musl::sin"];
    let mut h: Vec<Fnv> = NAMES.iter().map(|_| Fnv::new()).collect();
    for i in -20000i32..=20000 {
        let x = i as f32 * 1e-3 + 1.234_567e-4;
        let y = i.rem_euclid(997) as f32 * 7e-3 + 0.1;
        let (s, c) = musl::sincosf(x);
        // the game's own (`libm`) run on PS2 arithmetic, so they match everywhere by construction; checked anyway
        let r = [musl::sinf(x), musl::cosf(x), musl::tanf(x), musl::atanf(x), musl::asinf(x * 0.0499), musl::atan2f(x, y),
            musl::atan2f(y, -x), musl::hypotf(x, y), musl::powf(y, x * 0.1), s, c, libm::sinf(x), libm::cosf(x),
            libm::tanf(x * 0.1), libm::atanf(x), libm::atan2f(x, y), libm::acosf(x * 0.0499), libm::powf(y, 1.1),
            libm::table_sin(x * 0.1)];
        r.iter().zip(&mut h).for_each(|(&r, h)| h.f(r));
        h[19].d(musl::tan(x as f64 * 0.1));
        h[20].d(musl::sin(x as f64));
    }
    NAMES.iter().zip(&h).for_each(|(n, h)| println!("{n} {:016x}", h.0));
}
