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

/// The swing whoosh (court bank program 4 key 0 at 0x80, played at the swinger's model origin) for a swing of
/// `branch` (as `Hit::branch`) and shot `kind`, with `faults` serve faults this point: the frames it waits after
/// the swing starts, or `None`. Only a first serve (4 frames late) and a smash that is not a lob (at once) whoosh.
/// ponytail: the other strokes' path needs a game flag that normal play never sets, so it is left out.
pub fn swing_sound(branch: u8, kind: i32, faults: i32) -> Option<u32> {
    match branch {
        0 if faults == 0 => Some(4),
        4 if kind != 3 => Some(0),
        _ => None,
    }
}

/// The whoosh `swing_sound` plays.
pub const SWING: Play = play(0, 4, 0, 0x80);

/// The thud of a dive (court bank program 3 at 0x80, at the diver's model origin), played as the dive starts, whether
/// it reaches the ball or not, and again `dive_echo` ticks later; key 3 in weather 2 and 3 (court +0x135), else 0.
/// ponytail: the walking footstep (0x69) only sounds behind a close-up camera (mode 0x56..0x59) the match never
/// uses, so it is left out.
pub fn dive_thud(weather: u8) -> Play {
    play(0, 3, if weather.wrapping_sub(2) < 2 { 3 } else { 0 }, 0x80)
}

/// Ticks between a dive's two thuds with `players` in the game: the game counts 5 down once per player per frame.
pub fn dive_echo(players: u32) -> u32 {
    4 / players
}

/// The shout a stroke's launch plays on the hitter's voice bank (`Voice::shout`), by program: 0 for a serve off a
/// strong toss with grade 1 or 2 and for every smash; 2 (a strain) for a mis-hit; for a ground stroke or volley 1 on
/// the sweet frame or by chance at grade 2, else 2 by chance at grade 3 or 4. The chance is 40% (20% with more than
/// two players; halved for characters 9 and 10 at grades 3/4); `roll` draws 0..99. Dives shout at their start
/// (`DIVE_SHOUT`), the other strokes stay quiet.
/// ponytail: the singles close-up camera's own shout (program 0/1 when the stroke stayed quiet) is left out with the
/// camera.
pub fn stroke_shout(h: &Hit, character: i32, players: u32, mut roll: impl FnMut() -> u32) -> Option<u8> {
    let chance = if players < 3 { 40 } else { 20 };
    match h.branch {
        3 => None,
        _ if h.framed || h.dull => Some(2),
        0 => (h.strong_toss && matches!(h.grade, 1 | 2)).then_some(0),
        4 => Some(0),
        _ if h.offset.abs() < 2 || (h.grade == 2 && roll() < chance) => Some(1),
        _ if matches!(h.grade, 3 | 4) && roll() < if matches!(character, 9 | 10) { chance / 2 } else { chance } => Some(2),
        _ => None,
    }
}

/// The dive's shout program, played as the dive starts.
pub const DIVE_SHOUT: u8 = 3;

/// One player's voice: the last key of programs 1 and 2 (no shout repeats its program's last key).
#[derive(Clone, Copy, Debug)]
pub struct Voice([i32; 2]);

impl Default for Voice {
    fn default() -> Self {
        Self([-1; 2])
    }
}

impl Voice {
    /// Player `player`'s shout of `program` (bank slot 1 + player, 0x80 at the player): a random key, `r` a random
    /// draw, from 0..=hi other than the program's last — hi 4 for program 1 and 2 for the others with up to two
    /// players, else 1.
    pub fn shout(&mut self, player: usize, program: u8, players: u32, r: u32) -> Play {
        let hi = if players > 2 { 1 } else if program == 1 { 4 } else { 2 };
        let last = match program {
            1 | 2 => Some(&mut self.0[program as usize - 1]),
            _ => None,
        };
        let keys: Vec<i32> = (0..=hi).filter(|&k| last.as_ref().is_none_or(|l| **l != k)).collect();
        let key = keys[r as usize % keys.len()];
        if let Some(l) = last {
            *l = key;
        }
        play(1 + player as u8, program, key as u8, 0x80)
    }
}

/// The play-speed scale word the driver multiplies the pitch by: the speed clamped to 0..2, in 1/4096ths.
pub fn speed_word(speed: f32) -> u32 {
    (crate::ps2::mul(speed.clamp(0.0, 2.0), 4096.0) as i32) as u32
}

/// The bounce sounds of one ball in play, all on the court bank (slot 0) program 2 at the bounce's contact point:
/// the ball object's key 0 for each of the first three bounces on a playing surface (the first slowed to 0.5 after a
/// smash still at 85 km/h or more), and the ball effects' sound key of the bounced material (the net's key 2 not
/// again until the ball meets a playing surface), plus key 0 for materials 0x0d, 0x17 and 0x30. An effect-spawning
/// material holds the next 4 frames' material sounds off.
/// ponytail: the rolling scrape (key 3) needs a ball flag (+0x264) never set in the recordings; the menu option
/// that swaps key 0 for 0xd and the 10-effect cap (effects expire too fast to reach it) are left out.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bounces {
    /// Frames the material sounds are held off (+0x840).
    cooldown: i32,
    /// The last material sound was a soft obstacle's (effect kind 2, +0x849).
    soft: bool,
}

impl Bounces {
    /// One frame: `bounce` is the ball's new bounce count and its contact's material, if the count changed;
    /// `smash_kmh` the ball's speed when the shot was a smash with more than one player. Returns the plays.
    pub fn frame(&mut self, bounce: Option<(i32, &crate::ball::Material)>, smash_kmh: Option<f32>) -> Vec<Play> {
        let mut out = Vec::new();
        if let Some((n, m)) = bounce {
            if (1..4).contains(&n) && m.court {
                let fast = n == 1 && smash_kmh.is_some_and(|v| v >= 85.0);
                out.push(Play { speed: if fast { 0.5 } else { 1.0 }, ..play(0, 2, 0, 0x80) });
            }
            if n < 6 && self.cooldown < 1 {
                self.soft &= !m.court;
                if !(self.soft && m.effect == 2) && m.effect != 5 {
                    if m.effect != 0 {
                        self.cooldown = 4;
                    }
                    if m.sound != 0 {
                        out.push(play(0, 2, m.sound, 0x80));
                    }
                    if matches!(m.id, 0x0d | 0x17 | 0x30) {
                        out.push(play(0, 2, 0, 0x80));
                    }
                    self.soft = m.effect == 2;
                }
            }
        }
        self.cooldown -= (self.cooldown != 0) as i32;
        out
    }
}

/// The ball's speed as the game shows it: metres per frame to km/h.
pub fn kmh(vel: [f32; 3]) -> f32 {
    div(mul(mul(sqrt(madd(madd(mul(vel[2], vel[2]), vel[0], vel[0]), vel[1], vel[1])), 60.0), 3600.0), 1000.0)
}

/// The whistle of a lob or a framed mis-hit, on the court bank (slot 0) program 5 key 2 at the ball: from the hit
/// until the ball's first bounce or the next hit, re-placed at the ball every frame.
/// ponytail: two characters' own key (5) and range (low 0.3, top 8) at the start, the training mode's whistle (its
/// own volume table) and the fast-serve/smash rush (mode 2, ≥140 km/h) are left out.
pub const FLIGHT: Play = play(0, 5, 2, 0x80);

/// The whistle's play speed at ball height `height` (|y|): `low` below 1 m, rising linearly by (2 − low) over `top`
/// metres, 2 from `top` up. The start uses low 0 and top 10, every later frame low 0.3 and top 10.
pub fn flight_speed(height: f32, low: f32, top: f32) -> f32 {
    let h = height.abs();
    let s = if h >= top {
        2.0
    } else if h < 1.0 {
        low
    } else {
        add(mul(div(sub(2.0, low), top), sub(h, 1.0)).abs(), low)
    };
    s.clamp(0.0, 2.0)
}
