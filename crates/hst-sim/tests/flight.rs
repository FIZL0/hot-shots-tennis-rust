//! Ball-flight ribbon and glow against a recorded bot match (`context/fixtures/flight_s05.bin`,
//! `tools/record_flight.py 5`): from each reset and the ball the game had, every frame's samples, ring indices and
//! speed match bit for bit, and the glow's life.

use hst_sim::effect::{FLIGHT_COLOURS, FLIGHT_POINTS, Flight, Glow};

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v4(b: &[u8], o: usize) -> [f32; 4] {
    std::array::from_fn(|k| f(b, o + 4 * k))
}

// per frame: vsync 8, effects 0x100, flight 0xb0, its 50 samples 0x10, glow 0x70, ball 0x290, court marker 0x40
const FX: usize = 8;
const FL: usize = FX + 0x100;
const PTS: usize = FL + 0xb0;
const GLOW: usize = PTS + 0x320;
const BALL: usize = GLOW + 0x70;
const MARK: usize = BALL + 0x290;
const SIZE: usize = MARK + 0x40;

#[test]
fn flight_s05() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/flight_s05.bin")) else { return eprintln!("flight_s05.bin absent, skipped") };
    let frames: Vec<&[u8]> = data.chunks_exact(SIZE).collect();
    let (mut fl, mut glow) = (Flight::default(), Glow::default());
    let (mut starts, mut glows, mut checked) = (0, 0, 0);
    for (n, w) in frames.windows(2).enumerate() {
        let (was, s) = (w[0], w[1]);
        let frame = n + 1;
        let live = s[FL + 0x70] != 0;
        let colour = u32::from_le_bytes(std::array::from_fn(|k| f(s, FL + 0x90 + 4 * (3 - k)) as u8));
        if live && i(s, FL + 0x74) == 1 && (was[FL + 0x70] == 0 || i(was, FL + 0x74) != 1) {
            fl.start(i(s, FX + 0xd0), colour == FLIGHT_COLOURS[5]);
            starts += 1;
        }
        if !live {
            fl.live = false;
        }
        if i(s, GLOW + 0x60) == 29 && i(was, GLOW + 0x60) != 29 {
            glow.start(i(s, FX + 0xd0));
            glows += 1;
        }
        fl.tick(v4(s, MARK + 0x30), v4(s, BALL + 0x140));
        glow.tick();
        assert_eq!(fl.live, live, "frame {frame} live");
        assert_eq!(glow.life, if i(s, GLOW + 0x50) == 1 { i(s, GLOW + 0x60) } else { 0 }, "frame {frame} glow");
        if !live {
            continue;
        }
        assert_eq!(fl.colour, colour, "frame {frame} colour");
        assert_eq!((fl.count, fl.tail, fl.head), (i(s, FL + 0x74) as usize, i(s, FL + 0x78) as usize, i(s, FL + 0x7c) as usize), "frame {frame} ring");
        for k in 0..fl.count {
            let j = (fl.tail + k) % FLIGHT_POINTS;
            assert_eq!(fl.points[j].map(f32::to_bits), v4(s, PTS + 0x10 * j).map(f32::to_bits), "frame {frame} sample {j}");
        }
        assert_eq!(fl.speed.to_bits(), f(s, FL + 0xa4).to_bits(), "frame {frame} speed");
        checked += 1;
    }
    eprintln!("{starts} ribbons, {glows} glows, {checked} frames");
    assert!(starts >= 20 && glows >= 15 && checked >= 1000);
}
