//! Swing trails against a recorded bot match (`context/fixtures/trails_s05.bin`, `tools/record_trails.py 5`): from
//! each swing's start (its length in frames as the game set it), every frame the racket matrix and motion the game
//! had give the port's samples, reach, width, alpha and end, bit for bit.

use hst_sim::effect::Trail;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

// per frame: vsync, clock 8, effects 0x100, per player: trail 0xa8, 48 samples 0x30, motion state 0x90, racket 0x40
const PLAYER: usize = 0xa8 + 48 * 0x30 + 0x90 + 0x40;
const SIZE: usize = 0x10c + 4 * PLAYER;

#[test]
fn trails_s05() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/trails_s05.bin")) else { return eprintln!("trails_s05.bin absent, skipped") };
    let frames: Vec<&[u8]> = data.chunks_exact(SIZE).collect();
    let mut trails = vec![Trail::default(); 4];
    let (mut starts, mut drawn) = (0, 0);
    for (n, w) in frames.windows(2).enumerate() {
        for (k, tr) in trails.iter_mut().enumerate() {
            let at = 0x10c + k * PLAYER;
            let (was, s) = (&w[0][at..], &w[1][at..]);
            let (t, pts, mot, mat) = (&s[..0xa8], &s[0xa8..], &s[0x9a8..], &s[0xa38..]);
            let live = i(t, 0x50) == 1;
            if live && (i(was, 0x50) != 1 || i(t, 0x70) != i(was, 0x70) || i(t, 0x6c) > i(was, 0x6c)) {
                tr.start(i(t, 0x74), i(t, 0x70));
                starts += 1;
            }
            let racket: [[f32; 4]; 4] = std::array::from_fn(|r| std::array::from_fn(|c| f(mat, 0x10 * r + 4 * c)));
            tr.tick(i(mot, 0x20), &racket);
            let frame = n + 1;
            assert_eq!(tr.live, live, "frame {frame} player {k} live");
            if !live {
                continue;
            }
            assert_eq!((tr.life, tr.count), (i(t, 0x6c), i(t, 0x84) as usize), "frame {frame} player {k} life, count");
            for (j, p) in tr.points.iter().enumerate().take(48) {
                let want: Vec<u32> = (0..8).map(|c| f(pts, 0x30 * j + 4 * c).to_bits()).collect();
                assert_eq!(p.as_flattened().iter().map(|v| v.to_bits()).collect::<Vec<_>>(), want, "frame {frame} player {k} sample {j}");
            }
            if tr.count >= 2 {
                drawn += 1;
                let want = [f(t, 0x78), f(t, 0x7c), f(t, 0x80)];
                assert_eq!([tr.reach.to_bits(), tr.width.to_bits(), tr.alpha.to_bits()], want.map(f32::to_bits), "frame {frame} player {k} reach, width, alpha {:?} want {want:?}", [tr.reach, tr.width, tr.alpha]);
            }
        }
    }
    eprintln!("{starts} swings, {drawn} drawn frames");
    assert!(starts >= 15 && drawn >= 300);
}
