//! Hit sparks against a recorded bot match (`context/fixtures/sparks_s05.bin`, `tools/record_sparks.py 5`): from the
//! game's own rolls, every burst it throws matches the port's spark for spark, and every frame after it every
//! spark's position, velocity, size and life, bit for bit.

use hst_sim::effect::{Roll, SPARKS, Spark, Sparks};

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v4(b: &[u8], o: usize) -> [f32; 4] {
    std::array::from_fn(|k| f(b, o + 4 * k))
}

// per frame: vsync, effects 0x100, sparks 0x70, sparks 25×0x40, rolls 25×0x50, ball 0x290, court marker 0x40
const SIZE: usize = 4 + 0x100 + 0x70 + SPARKS * 0x40 + SPARKS * 0x50 + 0x290 + 0x40;
const FX: usize = 4;
const SP: usize = 0x104;
const PARTS: usize = 0x174;
const ROLLS: usize = 0x7b4;
const BALL: usize = 0xf84;
const MARK: usize = 0x1214;

fn rolls(s: &[u8]) -> [Roll; SPARKS] {
    std::array::from_fn(|i| {
        let r = &s[ROLLS + 0x50 * i..];
        Roll { turn: std::array::from_fn(|j| v4(r, 0x10 * j)), u: [f(r, 0x40), f(r, 0x44), f(r, 0x48)] }
    })
}
fn spark(s: &[u8], i: usize) -> (bool, Spark) {
    let p = &s[PARTS + 0x40 * i..];
    (u(p, 0) != 0, Spark { pos: v4(p, 0x10), vel: v4(p, 0x20), size: f(p, 0x30), life: u(p, 0x34) as i32 })
}
fn bits(s: &Spark) -> Vec<u32> {
    s.pos.iter().chain(&s.vel).chain([&s.size]).map(|v| v.to_bits()).chain([s.life as u32]).collect()
}

#[test]
fn sparks_s05() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/sparks_s05.bin")) else { return eprintln!("sparks_s05.bin absent, skipped") };
    let frames: Vec<&[u8]> = data.chunks_exact(SIZE).collect();
    let mut sp = Sparks::new(rolls(frames[0]));
    let (mut bursts, mut kinds) = (0, [0; 5]);
    for (i, w) in frames.windows(2).enumerate() {
        let (was, s) = (w[0], w[1]);
        // a burst restarts spark 0 (all but the weakest smash throw more than one)
        let ((a0, p0), (a1, p1)) = (spark(was, 0), spark(s, 0));
        let started = a1 && (!a0 || p1.life != p0.life - 1);
        if started {
            let kind = u(s, FX + 0xd0) as i32;
            let smash = u(s, SP + 0x54) == u(s, FX + 0x7c);
            sp.rolls = rolls(s);
            sp.start(kind, smash, v4(s, MARK + 0x30), v4(s, BALL + 0x140));
            bursts += 1;
            kinds[kind as usize] += 1;
        }
        // the game's own rerolls (the sound generator isn't recorded here; `sound_draws_like_the_game` checks them)
        sp.tick(&mut hst_sim::rng::Mt::new(1));
        sp.rolls = rolls(s);
        for k in 0..SPARKS {
            let (active, want) = spark(s, k);
            let got = &sp.sparks[k];
            assert_eq!(got.life > 0, active, "frame {} spark {k} active", i + 1);
            if active {
                assert_eq!(bits(got), bits(&want), "frame {} spark {k} {:?} want {:?}", i + 1, (got.pos, got.vel, got.size, got.life), (want.pos, want.vel, want.size, want.life));
            }
        }
    }
    eprintln!("{bursts} bursts, per kind {kinds:?}");
    assert!(bursts >= 15 && kinds.iter().filter(|&&n| n > 0).count() >= 3);
}
