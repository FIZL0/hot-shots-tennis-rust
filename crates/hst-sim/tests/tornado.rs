//! The ball's wind tornado against the recorded bot doubles match (`context/fixtures/foot_s05.bin`,
//! `tools/record_foot.py 5`): from the ball and the hit messages, every frame's on flag, scale, fade, alpha and
//! matrix match the game bit for bit.

use hst_sim::tornado::Tornado;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v4(b: &[u8], o: usize) -> [f32; 4] {
    std::array::from_fn(|k| f(b, o + 4 * k))
}

// Per frame (see tests/foot.rs): vsync 4, manager (+0x740) 0x40, run object …, tornado at TOR, ball at BALL.
const MGR: usize = 4;
const TOR: usize = 0x44 + 0x130 + 0x1000 + 0xc80 + 8 + 0x10;
const BALL: usize = TOR + 0x100;
const SIZE: usize = BALL + 0x290 + 0x20 + 4 * (0x40 + 8 + 8 + 0xa0 + 0x80);

fn key(t: &Tornado) -> (bool, i32, Vec<u32>) {
    // the alpha slot holds a stale value (the last fade's, or 0 after a reset) until the fade starts
    let v = [[t.t, t.end, t.speed, t.opacity()].as_slice(), &t.m.concat()].concat();
    (t.on, t.fade, v.iter().map(|x| x.to_bits()).collect())
}

#[test]
fn tornado_s05() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Ok(cnf), Ok(bin)) = (
        std::fs::read(format!("{root}/context/fixtures/foot_s05.bin")),
        std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")),
        std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")),
    ) else {
        return eprintln!("foot_s05.bin or disc absent, skipped");
    };
    let fade = hst_data::exe::Game::new(&cnf, &bin).unwrap().tornado_fade();
    assert_eq!(fade, 30);
    let frames: Vec<&[u8]> = data[8..].chunks_exact(SIZE).collect();
    let state = |fr: &[u8]| {
        let t = &fr[TOR..];
        Tornado {
            on: t[0x62] != 0,
            t: f(t, 0xb8),
            end: f(t, 0xc8),
            speed: f(t, 0xc0),
            fade: i(t, 0xb0),
            alpha: f(t, 0xbc),
            m: std::array::from_fn(|r| v4(t, 0x70 + 16 * r)),
        }
    };
    let mut tor = state(frames[0]);
    let (mut pending, mut starts, mut on) = (false, 0, 0);
    for (k, fr) in frames.iter().enumerate().skip(1) {
        let (m, b) = (&fr[MGR..], &fr[BALL..]);
        // a shot is latched as it is struck, and starts the tornado once the game flags the ball away
        pending |= (0..4).any(|p| m[0x18 + p] != 0 && ![999, 9999].contains(&i(m, 0x1c + 8 * p)));
        if pending && m[0x10] != 0 {
            pending = false;
            tor.start(v4(b, 0x140), fade);
            starts += 1;
        }
        tor.tick(i(b, 0x224), v4(b, 0xe0), v4(b, 0x140), fade);
        let want = state(fr);
        // the game leaves the rest as it was once off
        if want.on || tor.on {
            assert_eq!(key(&tor), key(&want), "frame {k}");
            on += 1;
        }
        tor.m = want.m;
    }
    eprintln!("{} frames, {starts} starts, {on} frames on", frames.len());
    assert!(starts > 5 && on > 300);
}

/// Forced constant ×4 slow motion (`context/fixtures/tornado_slowmo.bin`, `research/p17q_slowmo_rec.py 5`): every
/// in-between frame's drawn scale, UV time, alpha, on flag and matrix match the game bit for bit.
#[test]
fn tornado_slowmo() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(data) = std::fs::read(format!("{root}/context/fixtures/tornado_slowmo.bin")) else {
        return eprintln!("tornado_slowmo.bin absent, skipped");
    };
    // per frame: vsync 4, slow-motion block 0x30, tornado 0x190, its model +0x120 (8), its UV animation 0x30, ball
    const SLOW: usize = 4;
    const T: usize = SLOW + 0x30;
    const MODEL: usize = T + 0x190;
    const ANIM: usize = MODEL + 8;
    const B: usize = ANIM + 0x30;
    let fade = i(&data, 0);
    let frames: Vec<&[u8]> = data[4..].chunks_exact(B + 0x290).collect();
    let state = |fr: &[u8]| {
        let t = &fr[T..];
        Tornado {
            on: t[0x62] != 0,
            t: f(t, 0xb8),
            end: f(t, 0xc8),
            speed: f(t, 0xc0),
            fade: i(t, 0xb0),
            alpha: f(t, 0xbc),
            m: std::array::from_fn(|r| v4(t, 0x70 + 16 * r)),
        }
    };
    let mut checked = 0;
    for w in frames.windows(2) {
        let (prev, fr) = (w[0], w[1]);
        let frac = f(fr, SLOW + 4);
        if fr[SLOW] == 0 || frac == 1.0 || prev[T + 0x62] == 0 {
            continue;
        }
        let mut tor = state(prev);
        let uv = [f(&fr[T..], 0x180), f(&fr[T..], 0x17c)];
        let (scale, time) = tor.between(i(&fr[B..], 0x224), v4(&fr[B..], 0xe0), v4(&fr[B..], 0x140), frac, uv, fade, 0);
        // the UV animation's setter wraps a looping time into its length
        let (len, mut time) = (f(&fr[ANIM..], 0x14), time);
        if fr[ANIM + 0x28] != 0 && len != 0.0 {
            while len <= time {
                time = hst_sim::ps2::sub(time, len);
            }
        }
        let want = state(fr);
        assert_eq!(scale.to_bits(), f(fr, MODEL).to_bits(), "scale, vsync {}", i(fr, 0));
        assert_eq!(time.to_bits(), f(fr, ANIM + 0x20).to_bits(), "uv, vsync {}", i(fr, 0));
        assert_eq!(key(&tor), key(&want), "vsync {}", i(fr, 0));
        assert_eq!(tor.alpha.to_bits(), want.alpha.to_bits(), "alpha, vsync {}", i(fr, 0));
        checked += 1;
    }
    eprintln!("{checked} in-between frames");
    assert!(checked > 60);
}
