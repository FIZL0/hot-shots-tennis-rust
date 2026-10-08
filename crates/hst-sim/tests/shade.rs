//! The ball's sun-shade light scale against the game, frame by frame: `context/p17t/ball05.txt` (slot 5, court 10, a
//! bot rally; per frame the ball's position x y z w and its model's light scale, as bits; made by
//! `research/p17t_ball_rec.py`) with the game's own map (`context/p17t/map05.bin`). On the court the height is from
//! y 0, off it from a ray cast down at court 10's collision model built from the disc; a miss keeps the last
//! scale. Skips when the recording or the disc is absent.

use hst_data::iso::Iso;
use hst_sim::{court, shade};

#[test]
fn ball_light_scale_matches_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(rec), Ok(map), Ok(mut iso)) = (
        std::fs::read_to_string(format!("{root}/context/p17t/ball05.txt")),
        std::fs::read(format!("{root}/context/p17t/map05.bin")),
        Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")),
    ) else {
        eprintln!("recording or disc missing, skipped");
        return;
    };
    let world = court::world(&mut iso, 10);
    let frame = shade::Frame::new([-113.8475, -10.730181, -141.48601], [102.94255, 17.31305, 158.12808], 1.0, [0.0; 3]);
    let (mut exact, mut off, mut missed, mut bad) = (0, 0, 0, Vec::new());
    let mut last = None;
    for line in rec.lines().filter(|l| !l.starts_with('#')) {
        let w: Vec<u32> = line.split_whitespace().skip(1).map(|h| u32::from_str_radix(h, 16).unwrap()).collect();
        let pos = [0, 1, 2, 3].map(|k| f32::from_bits(w[k]));
        let want = w[4];
        if pos[0] == 0.0 && pos[2] == 0.0 {
            // the between-points reset puts the ball at the origin after that frame's light update
            last = Some(want);
            continue;
        }
        if pos[0].abs() > 10.685 || pos[2].abs() > 19.885 {
            off += 1;
        }
        let got = match shade::ball_height(&world.court, &world.models, pos) {
            Some(h) => Some(shade::ball(&map, &frame, pos[0], pos[2], h).to_bits()),
            None => {
                missed += 1;
                last
            }
        };
        match got {
            Some(g) if g == want => exact += 1,
            None => {}
            Some(g) => bad.push(format!("{line}: port {g:08x}")),
        }
        last = Some(want);
    }
    eprintln!("{exact} frames bit-exact, {off} off the court, {missed} ray misses");
    assert!(bad.is_empty(), "{} frames differ:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    assert!(off > 20 && exact > 1250);
    // far outside the court model the ray hits nothing
    assert_eq!(shade::ball_height(&world.court, &world.models, [500.0, -0.5, 500.0, 1.0]), None);
}
