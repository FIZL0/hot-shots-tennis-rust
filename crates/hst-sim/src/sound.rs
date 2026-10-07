//! Where a positional sound plays from: the sound library places every game sound against a fixed listener behind
//! the near baseline, attenuates it with distance on the floor plane and splits it into the sequence's left and
//! right volume by the bearing.

use crate::libm::atan2f;
use crate::ps2::{add, div, madd, msub, mul, sqrt, sub};

/// The listener: identity orientation at (0, 0, −10) in game space (the library's fixed listener, the only mode
/// the game uses).
const LISTENER: [f32; 3] = [0.0, 0.0, -10.0];
const PI: f32 = f32::from_bits(0x4049_0fdb);
const DEGREES: f32 = f32::from_bits(0x4265_2ee1); // 57.29578
/// Distances (whole metres) are clamped to this; it is also where a sound falls silent.
const FAR: i32 = 128;
/// Full volume up to here.
const NEAR: f32 = 10.0;

/// Bearing from the listener in whole degrees (0..359, 0 straight ahead along +z) and floor distance in whole
/// metres (0..128) of a sound at `pos`.
pub fn place(pos: [f32; 3]) -> (i32, i32) {
    let (dx, dz) = (sub(pos[0], LISTENER[0]), sub(pos[2], LISTENER[2]));
    let yaw = 0.0; // atan2 of the listener's identity forward axis
    let a = msub(mul(add(PI, atan2f(dx, dz)), DEGREES), add(PI, yaw), DEGREES) as i32;
    let a = if a < 0 { a + 360 } else { a };
    let d = sqrt(madd(mul(dz, dz), dx, dx)) as i32;
    (a.clamp(0, 359), d.clamp(0, FAR))
}

/// `volume` (0..128) at `dist` metres: full up to 10 m, then linearly down to nothing at 128 m.
pub fn falloff(volume: i32, dist: i32) -> i32 {
    let v = volume as f32;
    let v = if dist as f32 <= NEAR {
        v
    } else if dist > FAR {
        0.0
    } else {
        let span = sub(FAR as f32, NEAR);
        let step = if span == 0.0 { 0.0 } else { div(v, span) };
        msub(add(v, 0.0), step, sub(dist as f32, NEAR))
    };
    (v as i32).clamp(0, 128)
}

/// The sequence's left and right volume for `volume` at bearing `angle`, from `exe::stereo_tables`. The tables'
/// signs (phase inversion) only apply with the library's surround mode, which the game leaves off.
pub fn stereo(volume: i32, angle: i32, tables: &[Vec<i32>; 2]) -> [i32; 2] {
    let i = ((angle + 90) % 720) as usize;
    tables.each_ref().map(|t| {
        let g = if t[i] == -4096 { 0 } else { t[i].abs() };
        (volume * g) >> 11
    })
}

/// One sound the game asks for: bank slot, program and key, volume before falloff, play speed (1 = as recorded).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Play {
    pub slot: u8,
    pub program: u8,
    pub key: u8,
    pub volume: i32,
    pub speed: f32,
}

const fn play(slot: u8, program: u8, key: u8, volume: i32) -> Play {
    Play { slot, program, key, volume, speed: 1.0 }
}

/// What the racket-hit effect reads when a player's racket meets the ball.
#[derive(Clone, Copy, Debug, Default)]
pub struct Hit {
    /// Contact: 0 serve, 1 ground stroke, 2 volley, 3 dive, 4 smash.
    pub branch: u8,
    /// Timing grade (1 and 2 are the clean ones).
    pub grade: u8,
    /// Frames off the sweet frame.
    pub offset: i32,
    /// Shot kind: 0 topspin, 1 slice, 2 flat, 3 lob, 4 drop.
    pub kind: i32,
    /// A mistimed hit off the frame (player +0x3f06): only the frame clank plays.
    pub framed: bool,
    /// A mistimed hit that loses power (+0x3f0c): only the dull hit plays.
    pub dull: bool,
    /// A volley overpowering the incoming shot: the two characters' power gap (+0x3f07/+0x3f08).
    pub power_gap: Option<i32>,
    /// The game's random bit for an unclean stroke's key (2 or 3).
    pub random_bit: bool,
    /// Hits this rally including this one (1 = the serve).
    pub hits: i32,
    /// The server tossed strong (✕).
    pub strong_toss: bool,
    /// Only one player in the game.
    pub solo: bool,
}

/// The racket-hit sounds in the order the game plays them: on the court SE bank (slot 0) program 6 — key 1 for a
/// smash (0x80 on the sweet frame, else 0x6c, sped up 1.05 / slowed 0.95), key 5 for a sliced smash, keys 0/2
/// (0x76) for clean strokes, a random 2/3 otherwise, the drop shot's program 5 key 4 — plus the crisp key 4 (0x62) on top of a clean, well-timed stroke, a mis-hit's own
/// sound in place of the hit, and the overpowering volley's key 8 played twice (the second slowed to 0.9).
pub fn hit_sounds(h: &Hit) -> Vec<Play> {
    let mut out = Vec::new();
    let sweet = h.offset.abs() < 2;
    let clean = matches!(h.grade, 1 | 2);
    if h.framed {
        out.push(play(9, 0, 4, 0x80));
    } else if h.dull {
        out.push(play(0, 6, 3, 0x80));
    } else if let Some(gap) = h.power_gap.filter(|&g| g > 3) {
        let v = if gap < 7 { 0x46 } else { 0x5a };
        out.extend([play(0, 6, 8, v), Play { speed: f32::from_bits(0x3f66_6666), ..play(0, 6, 8, v) }]);
    }
    let (program, key, volume) = if h.branch == 4 {
        match h.kind {
            1 => (6, 5, 0x80),
            _ => (6, 1, if sweet { 0x80 } else { 0x6c }),
        }
    } else if h.kind == 4 {
        (5, 4, 0x80)
    } else if clean {
        (6, if sweet && h.branch != 3 { 0 } else { 2 }, 0x76)
    } else if h.random_bit {
        (6, 3, 0x80)
    } else {
        (6, 2, 0x76)
    };
    if !h.framed && !h.dull {
        let speed = match key {
            1 => f32::from_bits(if sweet { 0x3f86_6666 } else { 0x3f73_3333 }), // 1.05, 0.95
            _ => 1.0,
        };
        out.push(Play { speed, ..play(0, program, key, volume) });
    }
    // ponytail: character 9's lob (slot 9 key 7) is a voice — N3d
    if h.power_gap.is_none_or(|g| g < 4)
        && h.kind != 3
        && h.kind != 4
        && clean
        && sweet
        && h.branch != 3
        && (h.solo || h.hits != 1 || h.strong_toss)
    {
        out.push(play(0, 6, 4, 0x62));
    }
    // The ball-speed whooshes (keys 6–8 by km/h) are gated on a per-court table that is all zero on the disc and
    // never written, so they never play.
    out
}

/// The play-speed scale word the driver multiplies the pitch by: the speed clamped to 0..2, in 1/4096ths.
pub fn speed_word(speed: f32) -> u32 {
    (crate::ps2::mul(speed.clamp(0.0, 2.0), 4096.0) as i32) as u32
}
