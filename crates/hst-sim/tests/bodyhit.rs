//! The ball's course after it hits a player, against a recorded bot rally on court 10
//! (`context/fixtures/bodyhit_s05.bin`, `research/bodyhit_rec.py 5 … 4.0`: every player's collision radius poked
//! to 4 so a body hit comes soon; not in git, skipped when absent along with the disc). The frame before the hit
//! is stepped against the court world, then once more against the plane facing back along its heading
//! (`Flight::step_plane`, material 0): it must give the hit frame's ball bit for bit — same position, the
//! reflected velocity, spin, frames and counts — and the frame after must follow by the ordinary step.

use hst_data::{exe, iso::Iso};
use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};

const SAMPLE: usize = 4 + 0x60 + 0x290 + 4 * 0x140;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v3(b: &[u8], o: usize) -> [f32; 3] {
    [f(b, o), f(b, o + 4), f(b, o + 8)]
}
fn rows(b: &[u8], o: usize) -> [[f32; 4]; 4] {
    std::array::from_fn(|r| std::array::from_fn(|k| f(b, o + 16 * r + 4 * k)))
}

fn load(b: &[u8]) -> (Flight, Shot) {
    let mut fl = Flight::new(Ball { pos: v3(b, 0xe0), vel: v3(b, 0x130), spin: f(b, 0x1a4) }, rows(b, 0x160), rows(b, 0x1c0));
    fl.frame = i(b, 0xac);
    fl.bounces = i(b, 0x224);
    fl.contacts = i(b, 0x228);
    fl.special_contacts = i(b, 0x22c);
    fl.rolling = b[0xa4] == 2;
    fl.slide = f(b, 0x200);
    let shot = Shot {
        params: Params::default(),
        curve: f(b, 0x254),
        bend: f(b, 0x250),
        side: v3(b, 0x90),
        curve_frames: i(b, 0x260),
        wind: v3(b, 0x240),
        first_bounce_spin: f(b, 0x1a8),
        first_bounce_restitution: f(b, 0x1ac),
        class: b[0x58],
        kind: i(b, 0x5c),
        bounce_turn: f(b, 0x1b0),
    };
    (fl, shot)
}

fn ball(s: &[u8]) -> &[u8] {
    &s[4 + 0x60..][..0x290]
}

fn same(fl: &Flight, b: &[u8], what: &str) {
    let bits = |v: [f32; 3]| v.map(f32::to_bits);
    assert_eq!(bits(fl.ball.pos), bits(v3(b, 0xe0)), "{what}: position");
    assert_eq!(bits(fl.ball.vel), bits(v3(b, 0x130)), "{what}: velocity");
    assert_eq!(fl.ball.spin.to_bits(), f(b, 0x1a4).to_bits(), "{what}: spin");
    assert_eq!(fl.spin_frame.map(|r| r.map(f32::to_bits)), rows(b, 0x160).map(|r| r.map(f32::to_bits)), "{what}: spin frame");
    assert_eq!((fl.frame, fl.bounces, fl.contacts), (i(b, 0xac), i(b, 0x224), i(b, 0x228)), "{what}: counts");
}

#[test]
fn ball_off_a_player_s05() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Ok(mut iso), Ok(cnf), Ok(bin)) = (
        std::fs::read(format!("{root}/context/fixtures/bodyhit_s05.bin")),
        Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")),
        std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")),
        std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")),
    ) else {
        return eprintln!("bodyhit_s05.bin or disc absent, skipped");
    };
    let (world, materials) = (hst_sim::court::world(&mut iso, 10), hst_sim::court::materials(&exe::Game::new(&cnf, &bin).unwrap()));
    let frames: Vec<&[u8]> = data.chunks_exact(SAMPLE).collect();
    // the hit: a player's hit tick (+0x3b60) turns from -1
    let hit = |s: &[u8]| (0..4).any(|p| i(s, 4 + 0x60 + 0x290 + p * 0x140 + 0x20) != -1);
    let n = frames.iter().position(|s| hit(s)).expect("no body hit recorded");
    let (mut fl, shot) = load(ball(frames[n - 1]));
    fl.step_world(&shot, &COURTS[10], &world, &materials);
    fl.step_plane(&shot, &COURTS[10], materials[0]);
    same(&fl, ball(frames[n]), "hit frame");
    let (mut fl, shot) = load(ball(frames[n]));
    fl.step_world(&shot, &COURTS[10], &world, &materials);
    same(&fl, ball(frames[n + 1]), "frame after");
}
