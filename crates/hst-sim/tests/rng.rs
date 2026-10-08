//! The game's random sources against `context/p3b/rng_s05.bin` (research/p3b_rng_rec.py, slot 5; skipped when
//! absent): from every frame's RAM-dumped generator, `Mt::next` must reach the next frame's state exactly (the
//! words through every regeneration and the index), or a reseed with one of the `rand()` outputs drawn in between
//! must: at a new point the shared generator takes the frame's first output and the court's the second
//! (`Rngs::new_point`), and the sound manager's takes the first at change ends (`Rngs::change_ends`).
use hst_sim::rng::{Mt, Rand, Rngs};

/// Draws from `a` that land on `b`, if any within `max`.
fn reach(a: &Mt, b: &Mt, max: usize) -> Option<usize> {
    let mut m = a.clone();
    (0..=max).find(|_| m == *b || {
        m.next();
        false
    })
}

#[test]
fn generators_step_like_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(d) = std::fs::read(format!("{root}/context/p3b/rng_s05.bin")) else {
        return eprintln!("rng_s05.bin missing, skipped");
    };
    const NAMES: [&str; 4] = ["shared", "ai", "court", "sound"];
    let size = 4 + 8 + 4 * 0x9d0;
    let samples: Vec<(u32, Rand, [Mt; 4])> = d
        .chunks_exact(size)
        .map(|s| {
            let u = |o: usize| u32::from_le_bytes(s[o..o + 4].try_into().unwrap());
            let rand = Rand(u64::from_le_bytes(s[4..12].try_into().unwrap()));
            (u(0), rand, std::array::from_fn(|k| Mt::from_ram(&s[12 + k * 0x9d0..])))
        })
        .collect();
    let (mut draws, mut reseeds) = ([0usize; 4], [0usize; 4]);
    for w in samples.windows(2) {
        let ((va, ra, a), (vb, rb, b)) = (&w[0], &w[1]);
        // rand() steps from one frame's state to the next
        let mut r = *ra;
        for _ in 0..64 {
            if r == *rb {
                break;
            }
            r.next();
        }
        assert_eq!(r, *rb, "vsync {va}→{vb}: rand() state not reached");
        for k in 0..4 {
            if let Some(n) = reach(&a[k], &b[k], 4000) {
                draws[k] += n;
                continue;
            }
            let mut g = Rngs::new(*ra, a[0].clone());
            if k == 3 { g.change_ends() } else { g.new_point() }
            let m = [&g.shared, &g.ai, &g.court, &g.sound][k];
            assert!(k != 1 && reach(m, &b[k], 4000).is_some(), "vsync {va}→{vb}: {} generator neither stepped nor reseeded as the game does", NAMES[k]);
            reseeds[k] += 1;
        }
    }
    eprintln!("{} frames: draws {draws:?}, reseeds {reseeds:?}", samples.len());
    assert!(draws.iter().all(|&n| n > 0), "every generator draws over the recording");
}
