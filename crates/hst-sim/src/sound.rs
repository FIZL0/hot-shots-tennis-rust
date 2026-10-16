//! Where a positional sound plays from: the sound library places every game sound against a fixed listener behind
//! the near baseline, attenuates it with distance on the floor plane and splits it into the sequence's left and
//! right volume by the bearing.

#![deny(clippy::float_arithmetic)]

use crate::judge::verdict;
use crate::libm::atan2f;
use crate::ps2::{F, fs};
use crate::shot::{branch, kind};

/// The sound banks' slots.
pub mod slot {
    /// The court's sound effects.
    pub const COURT: u8 = 0;
    /// The umpire's voice.
    pub const UMPIRE: u8 = 5;
    /// The gallery (the court archive's `galsg` bank).
    pub const GALLERY: u8 = 6;
    /// The jingles.
    pub const JINGLES: u8 = 8;
    /// The system sound effects.
    pub const SYSTEM: u8 = 9;

    /// Player `player`'s voice bank (1..=4).
    pub fn voice(player: usize) -> u8 {
        1 + player as u8
    }
}

/// The court bank's programs.
mod court {
    /// The ball's bounces (key 0 on the playing surface, else the material's).
    pub const BOUNCE: u8 = 2;
    /// A dive's thud.
    pub const THUD: u8 = 3;
    /// The swing whoosh.
    pub const WHOOSH: u8 = 4;
    /// The ball's flight (the whistle and the rush), and the drop shot's hit.
    pub const FLIGHT: u8 = 5;
    /// The racket hits ([`super::racket`]).
    pub const RACKET: u8 = 6;
}

/// The court bank's racket-hit keys.
mod racket {
    pub const SWEET: u8 = 0;
    pub const SMASH: u8 = 1;
    pub const STROKE: u8 = 2;
    /// A dull hit (or the other unclean stroke).
    pub const DULL: u8 = 3;
    /// The crisp ring on top of a clean, well-timed stroke.
    pub const CRISP: u8 = 4;
    pub const SLICED_SMASH: u8 = 5;
    /// The overpowering volley.
    pub const OVERPOWER: u8 = 8;
}

/// The flight program's keys.
mod flight {
    /// Lola's special slice serve's rush.
    pub const LOLA_RUSH: u8 = 0;
    pub const WHISTLE: u8 = 2;
    /// The drop shot's hit.
    pub const DROP_HIT: u8 = 4;
    /// The special lob's whistle of characters 5 and 13.
    pub const SPECIAL_WHISTLE: u8 = 5;
    pub const RUSH: u8 = 6;
}

/// The voice banks' programs (shouts 0..=4 are `Voice::shout`'s).
mod voice {
    /// A shout that goes with a stroke (`stroke_shout`'s 0).
    pub const SHOUT: u8 = 0;
    /// The sweet-frame shout (`stroke_shout`'s 1).
    pub const SWEET_SHOUT: u8 = 1;
    /// A strain (`stroke_shout`'s 2).
    pub const STRAIN: u8 = 2;
    pub const HIT_CRY: u8 = 5;
    pub const CALL_OUT: u8 = 6;
    /// The reaction voices: cheer, groan, the set cheer and groan.
    pub const CHEER: u8 = 7;
    pub const GROAN: u8 = 8;
    pub const SET_CHEER: u8 = 9;
    pub const SET_GROAN: u8 = 10;
}

/// The umpire bank's programs.
mod umpire {
    /// The score words (`score_call`'s keys).
    pub const SCORE: u8 = 0;
    /// The line calls (key `verdict` − 1).
    pub const LINE: u8 = 1;
    /// The other calls.
    pub const CALL: u8 = 2;
}

/// The reaction motions `reaction_voice` tells apart (`motion::reaction`): the winners' `gu` and losers' `di`,
/// and their `_set` forms.
mod reaction_motion {
    pub const GU: i32 = 0x2c;
    pub const DI: i32 = 0x2d;
    pub const GU_SET: i32 = 0x2e;
    pub const DI_SET: i32 = 0x2f;
}

/// A play's full volume.
const FULL: i32 = 0x80;

/// The listener: identity orientation at (0, 0, −10) in game space (the library's fixed listener, the only mode
/// the game uses).
const LISTENER: [f32; 3] = [0.0, 0.0, -10.0];
const PI: f32 = std::f32::consts::PI;
const DEGREES: f32 = 57.295_78;
/// Distances (whole metres) are clamped to this; it is also where a sound falls silent.
const FAR: i32 = 128;
/// Full volume up to here.
const NEAR: f32 = 10.0;

/// Bearing from the listener in whole degrees (0..359, 0 straight ahead along +z) and floor distance in whole
/// metres (0..128) of a sound at `pos`.
pub fn place(pos: [f32; 3]) -> (i32, i32) {
    let (dx, dz) = (F(pos[0]) - LISTENER[0], F(pos[2]) - LISTENER[2]);
    let yaw = 0.0; // atan2 of the listener's identity forward axis
    let a = ((PI + F(atan2f(dx.0, dz.0))) * DEGREES).msub(PI + F(yaw), DEGREES).0 as i32;
    let a = if a < 0 { a + 360 } else { a };
    let d = (dz * dz).madd(dx, dx).sqrt().0 as i32;
    (a.clamp(0, 359), d.clamp(0, FAR))
}

/// `volume` (0..=`FULL`) at `dist` metres: full up to 10 m, then linearly down to nothing at 128 m.
pub fn falloff(volume: i32, dist: i32) -> i32 {
    let v = F(volume as f32);
    let v = if dist as f32 <= NEAR {
        v
    } else if dist > FAR {
        F::ZERO
    } else {
        let span = F(FAR as f32) - NEAR;
        let step = if span == 0.0 { F::ZERO } else { v / span };
        (v + 0.0).msub(step, F(dist as f32) - NEAR)
    };
    (v.0 as i32).clamp(0, FULL)
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
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// Contact (`shot::branch`).
    pub branch: u8,
    /// Timing grade (1 and 2 are the clean ones).
    pub grade: u8,
    /// Frames off the sweet frame.
    pub offset: i32,
    /// Shot kind (`shot::kind`).
    pub kind: i32,
    /// A mistimed hit off the frame: only the frame clank plays.
    pub framed: bool,
    /// A mistimed hit that loses power: only the dull hit plays.
    pub dull: bool,
    /// A volley overpowering the incoming shot: the two characters' power gap.
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
        out.push(play(slot::SYSTEM, 0, 4, FULL));
    } else if h.dull {
        out.push(play(slot::COURT, court::RACKET, racket::DULL, FULL));
    } else if let Some(gap) = h.power_gap.filter(|&g| g > 3) {
        let v = if gap < 7 { 0x46 } else { 0x5a };
        let p = play(slot::COURT, court::RACKET, racket::OVERPOWER, v);
        out.extend([p, Play { speed: 0.9, ..p }]);
    }
    let (program, key, volume) = if h.branch == branch::SMASH {
        match h.kind {
            kind::SLICE => (court::RACKET, racket::SLICED_SMASH, FULL),
            _ => (court::RACKET, racket::SMASH, if sweet { FULL } else { 0x6c }),
        }
    } else if h.kind == kind::DROP {
        (court::FLIGHT, flight::DROP_HIT, FULL)
    } else if clean {
        (court::RACKET, if sweet && h.branch != branch::DIVE { racket::SWEET } else { racket::STROKE }, 0x76)
    } else if h.random_bit {
        (court::RACKET, racket::DULL, FULL)
    } else {
        (court::RACKET, racket::STROKE, 0x76)
    };
    if !h.framed && !h.dull {
        let speed = match key {
            racket::SMASH => if sweet { 1.05 } else { 0.95 },
            _ => 1.0,
        };
        out.push(Play { speed, ..play(slot::COURT, program, key, volume) });
    }
    // ponytail: character 9's lob (slot 9 key 7) is a voice — N3d
    if h.power_gap.is_none_or(|g| g < 4)
        && h.kind != kind::LOB
        && h.kind != kind::DROP
        && clean
        && sweet
        && h.branch != branch::DIVE
        && (h.solo || h.hits != 1 || h.strong_toss)
    {
        out.push(play(slot::COURT, court::RACKET, racket::CRISP, 0x62));
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
        branch::SERVE if faults == 0 => Some(4),
        branch::SMASH if kind != kind::LOB => Some(0),
        _ => None,
    }
}

/// The whoosh `swing_sound` plays.
pub const SWING: Play = play(slot::COURT, court::WHOOSH, 0, FULL);

/// The thud of a dive (court bank program 3 at 0x80, at the diver's model origin), played as the dive starts, whether
/// it reaches the ball or not, and again `dive_echo` ticks later; key 3 in weather 2 and 3 (the court's), else 0.
pub fn dive_thud(weather: u8) -> Play {
    play(slot::COURT, court::THUD, thud_key(weather), FULL)
}

/// The thud's key: 3 in weather 2 and 3, else 0.
fn thud_key(weather: u8) -> u8 {
    if weather.wrapping_sub(2) < 2 { 3 } else { 0 }
}

/// A footfall (`foot::Feet::stepped`) in an instant replay of a singles match, by a running player, while the
/// camera is a close-up one (mode 0x56..0x59) that [`step_heard`]: the dive thud's program and key at 0x69.
// ponytail: unchecked against a recording (no singles replay with that camera captured yet)
pub fn step(weather: u8) -> Play {
    play(slot::COURT, court::THUD, thud_key(weather), 0x69)
}

/// The camera at `eye` looking along `look` hears a footfall at the player's origin `at`: level bearing within 50°
/// of the look, and within 5° or its depth through `view` near enough that `focal` over it passes 125.
pub fn step_heard(eye: [f32; 4], look: [f32; 4], view: &[[f32; 4]; 4], focal: f32, at: [f32; 4]) -> bool {
    let cos = bearing_cos(eye, look, at);
    cos >= COS_50 && (cos >= COS_5 || 125.0 < F(focal) / crate::vu0::transform(view, at)[2])
}

/// cos 50° and cos 5°, the close views' bearing limits.
const COS_50: f32 = f32::from_bits(0x3f24_8dbb);
const COS_5: f32 = f32::from_bits(0x3f7f_069e);

/// The cosine of the level bearing from `eye` to `at` against the level `look`.
fn bearing_cos(eye: [f32; 4], look: [f32; 4], at: [f32; 4]) -> f32 {
    let unit = |x: F, z: F| {
        let q = 1.0 / (x * x).madd(z, z).sqrt();
        (x * q, z * q)
    };
    let (look, eye, at) = (fs(look), fs(eye), fs(at));
    let (lx, lz) = unit(look[0], look[2]);
    let (dx, dz) = unit(at[0] - eye[0], at[2] - eye[2]);
    (dz * lz).madd(dx, lx).madd(0.0, 0.0).0
}

/// The post-point camera (eye, level `look`, world→camera `view`, projection scale `focal` = 240 / tan of the
/// horizontal half-angle) has a reacting player in close view: their model origin `origin` lies more than a
/// millimetre ahead and not so near that `focal` over its depth reaches 1000, and their spot `at` lies within 50° of
/// the look and near enough that `focal` over its depth passes 125 (80 within 5°).
pub fn reaction_view(eye: [f32; 4], look: [f32; 4], view: &[[f32; 4]; 4], focal: f32, origin: [f32; 4], at: [f32; 4]) -> bool {
    let z = crate::vu0::transform(view, origin)[2];
    if !(0.001 < z && F(1.0) * focal / z < 1000.0) {
        return false;
    }
    let cos = bearing_cos(eye, look, at);
    cos >= COS_50 && if cos >= COS_5 { 80.0 } else { 125.0 } < F(focal) / crate::vu0::transform(view, at)[2]
}

/// A reaction voice: program, key range and the voice's key-memory slot (`Voice::key`).
pub type ReactionVoice = (u8, i32, i32, Option<usize>);

/// The voice a player in [`reaction_view`] gives on their reaction's 5th frame, by reaction motion: the cheers 0x2c
/// / 0x2e and groans 0x2d / 0x2f voice programs 7 / 9 / 8 / 10; any other motion (a team reaction, a body-hit's)
/// voices program 8 for a player of the side that lost the point, in singles only 30% of the time (`roll` draws
/// 0..99). Singles keys: 7 1..2 (0 in an instant replay), 8 0..1 (2), 9 and 10 0..1; doubles: 7 0..1, the rest 0,
/// the loser's 8 1 (singles 3..4). With three players nobody voices.
pub fn reaction_voice(players: u32, motion: i32, lost: bool, replay: bool, roll: impl FnOnce() -> u32) -> Option<ReactionVoice> {
    use reaction_motion::*;
    use voice::{CHEER, GROAN, SET_CHEER, SET_GROAN};
    if players < 3 {
        match motion {
            GU_SET => Some((SET_CHEER, 0, 1, Some(4))),
            DI_SET => Some((SET_GROAN, 0, 1, Some(5))),
            GU if replay => Some((CHEER, 0, 0, None)),
            GU => Some((CHEER, 1, 2, None)),
            DI if replay => Some((GROAN, 2, 2, None)),
            DI => Some((GROAN, 0, 1, None)),
            _ if lost && roll() < 30 => Some((GROAN, 3, 4, Some(3))),
            _ => None,
        }
    } else if players == 4 {
        match motion {
            GU_SET => Some((SET_CHEER, 0, 0, None)),
            DI_SET => Some((SET_GROAN, 0, 0, None)),
            GU => Some((CHEER, 0, 1, None)),
            DI => Some((GROAN, 0, 0, None)),
            _ if lost => Some((GROAN, 1, 1, None)),
            _ => None,
        }
    } else {
        None
    }
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
        branch::DIVE => None,
        _ if h.framed || h.dull => Some(voice::STRAIN),
        branch::SERVE => (h.strong_toss && matches!(h.grade, 1 | 2)).then_some(voice::SHOUT),
        branch::SMASH => Some(voice::SHOUT),
        _ if h.offset.abs() < 2 || (h.grade == 2 && roll() < chance) => Some(voice::SWEET_SHOUT),
        _ if matches!(h.grade, 3 | 4) && roll() < if matches!(character, 9 | 10) { chance / 2 } else { chance } => Some(voice::STRAIN),
        _ => None,
    }
}

/// The dive's shout program, played as the dive starts.
pub const DIVE_SHOUT: u8 = 3;
/// A missed swing's shout program, at the contact pose where the stroke turns into its miss motion. It is muted
/// when the player's previous swing also ended in its miss motion with no swing start between (the re-press window
/// a miss opens stays open, even across points, until a stroke press or a serve swing starts): of a run of
/// whiffs only the first shouts. A missed serve swing always shouts.
pub const WHIFF_SHOUT: u8 = 4;

/// A doubles CPU player's call (program 6 key 3 or 4 by a random bit, 0x80 at the player) as it leaves an incoming
/// ball to its partner, one time in four.
pub fn call_out(player: usize, bit: bool) -> Play {
    play(slot::voice(player), voice::CALL_OUT, 3 + bit as u8, FULL)
}

/// The cry of a player the ball hits (program 5 key 0, 0x80 at the player), as the ball touches them.
pub fn hit_cry(player: usize) -> Play {
    play(slot::voice(player), voice::HIT_CRY, 0, FULL)
}

/// The last key of each of the player's six key-memory slots (programs 1 and 2's shouts in 0 and 1, the reaction
/// voices in 3–5; slot 2 is the remaster's emotes', `play::emote`).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Voice([i32; 6]);

impl Default for Voice {
    fn default() -> Self {
        Self([-1; 6])
    }
}

impl Voice {
    /// Player `player`'s shout of `program` (bank slot 1 + player, 0x80 at the player): a random key, `r` a random
    /// draw, from 0..=hi other than the program's last — hi 4 for program 1 and 2 for the others with up to two
    /// players, else 1; the whiff's hi is 1 with exactly two players, else 0.
    pub fn shout(&mut self, player: usize, program: u8, players: u32, r: u32) -> Play {
        let hi = if program == WHIFF_SHOUT {
            (players == 2) as i32
        } else if players > 2 {
            1
        } else if program == voice::SWEET_SHOUT {
            4
        } else {
            2
        };
        let slot = matches!(program, voice::SWEET_SHOUT | voice::STRAIN).then(|| program as usize - 1);
        let key = self.key(0, hi, slot, || r).expect("a shout has keys");
        play(slot::voice(player), program, key as u8, FULL)
    }

    /// A random key from `lo..=hi` other than the last one `slot` remembers (which then keeps it), by `draw` (a
    /// random draw); `None`, with no draw taken, when that leaves none.
    pub fn key(&mut self, lo: i32, hi: i32, slot: Option<usize>, draw: impl FnOnce() -> u32) -> Option<i32> {
        let keys: Vec<i32> = (lo..=hi).filter(|&k| slot.is_none_or(|s| self.0[s] != k)).collect();
        if keys.is_empty() {
            return None;
        }
        let key = keys[draw() as usize % keys.len()];
        if let Some(s) = slot {
            self.0[s] = key;
        }
        Some(key)
    }

    /// The remaster's: the key after the last one `slot` remembers in `lo..=hi`, back to `lo` past `hi` (which then
    /// keeps it), so every take comes round in turn, none twice running. No draw.
    pub fn cycle(&mut self, lo: i32, hi: i32, slot: usize) -> i32 {
        let k = self.0[slot];
        self.0[slot] = if (lo..hi).contains(&k) { k + 1 } else { lo };
        self.0[slot]
    }

    /// Player `player`'s reaction voice (`reaction_voice`), on bank slot 1 + player at 0x80.
    pub fn react(&mut self, player: usize, (program, lo, hi, slot): ReactionVoice, draw: impl FnOnce() -> u32) -> Option<Play> {
        let key = self.key(lo, hi, slot, draw)?;
        Some(play(slot::voice(player), program, key as u8, FULL))
    }
}

/// The play-speed scale word the driver multiplies the pitch by: the speed clamped to 0..2, in 1/4096ths.
pub fn speed_word(speed: f32) -> u32 {
    ((F(speed).clamp(0.0, 2.0) * 4096.0).0 as i32) as u32
}

/// The bounce sounds of one ball in play, all on the court bank (slot 0) program 2 at the bounce's contact point:
/// the ball object's key 0 for each of the first three bounces on a playing surface (the first slowed to 0.5 after a
/// smash still at 85 km/h or more), and the ball effects' sound key of the bounced material (the net's key 2 not
/// again until the ball meets a playing surface), plus key 0 for materials 0x0d, 0x17 and 0x30. An effect-spawning
/// material holds the next 4 frames' material sounds off.
/// ponytail: the rolling scrape (key 3) needs a ball flag never set in the recordings; the menu option
/// that swaps key 0 for 0xd and the 10-effect cap (effects expire too fast to reach it) are left out.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Bounces {
    /// Frames the material sounds are held off.
    cooldown: i32,
    /// The last material sound was a soft obstacle's (effect kind `SOFT`).
    soft: bool,
}

/// A material's effect kinds `Bounces` tells apart: a soft obstacle, and one that makes no sound.
const SOFT: u8 = 2;
const SILENT: u8 = 5;
/// Frames an effect-spawning material holds the next material sounds off.
const EFFECT_HOLD: i32 = 4;
/// Materials that also play the playing surface's bounce key.
const BOUNCE_TOO: [u8; 3] = [0x0d, 0x17, 0x30];
/// A smash landing at this speed (km/h) or more: the first bounce plays slowed.
const SMASH_KMH: f32 = 85.0;

impl Bounces {
    /// One frame: `bounce` is the ball's new bounce count and its contact's material, if the count changed;
    /// `smash_kmh` the ball's speed when the shot was a smash with more than one player. Returns the plays.
    pub fn frame(&mut self, bounce: Option<(i32, &crate::ball::Material)>, smash_kmh: Option<f32>) -> Vec<Play> {
        let mut out = Vec::new();
        let bounce_play = |key| play(slot::COURT, court::BOUNCE, key, FULL);
        if let Some((n, m)) = bounce {
            if (1..4).contains(&n) && m.court {
                let fast = n == 1 && smash_kmh.is_some_and(|v| v >= SMASH_KMH);
                out.push(Play { speed: if fast { 0.5 } else { 1.0 }, ..bounce_play(0) });
            }
            if n < 6 && self.cooldown < 1 {
                self.soft &= !m.court;
                if !(self.soft && m.effect == SOFT) && m.effect != SILENT {
                    if m.effect != 0 {
                        self.cooldown = EFFECT_HOLD;
                    }
                    if m.sound != 0 {
                        out.push(bounce_play(m.sound));
                    }
                    if BOUNCE_TOO.contains(&m.id) {
                        out.push(bounce_play(0));
                    }
                    self.soft = m.effect == SOFT;
                }
            }
        }
        self.cooldown -= (self.cooldown != 0) as i32;
        out
    }
}

/// The ball's speed as the game shows it: metres per frame to km/h.
pub fn kmh(vel: [f32; 3]) -> f32 {
    let v = fs(vel);
    ((v[2] * v[2]).madd(v[0], v[0]).madd(v[1], v[1]).sqrt() * 60.0 * 3600.0 / 1000.0).0
}

/// The whistle of a lob or a framed mis-hit, on the court bank (slot 0) program 5 key 2 at the ball: from the hit
/// until the ball's first bounce or the next hit, re-placed at the ball every frame.
/// `Flight::start` picks it (or the rush) and the special lobs' key and range.
/// ponytail: the training mode's whistle (its own volume table) is left out.
pub const FLIGHT: Play = play(slot::COURT, court::FLIGHT, flight::WHISTLE, FULL);

/// Lola's character number (her special slice serve rushes).
const LOLA: i32 = 10;
/// A serve or smash this fast (km/h, after the hit) or more rushes.
const RUSH_KMH: f32 = 140.0;

/// The sound a hit sends with the ball, on the court bank program 5, until its first bounce or the next hit.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Flight {
    /// `FLIGHT`'s whistle with key `key`; its start speed is `flight_speed(height, low, top)`.
    Whistle { key: u8, low: f32, top: f32 },
    /// The rush of a fast serve or smash (and Lola's special slice serve): `play` re-placed every frame; once the
    /// ball's speed falls to `threshold` km/h it fades out over four frames (`Flight::frame`).
    Rush { play: Play, threshold: f32, count: i32 },
}

impl Flight {
    /// What hit `h` by `character` starts: the whistle for a lob or a framed mis-hit; the rush for a serve (topspin
    /// or flat) or smash at ≥140 km/h (`kmh` × 0.9, the ball's speed after the hit), or character 10's special slice
    /// serve. A special lob of characters 5 and 13 whistles on key 5, and of 5, 6, 9 and 13 starts from 0.3 with
    /// the top at 8 m.
    /// ponytail: the instant replay's restarts (the app has none, P0b4d) are left out.
    pub fn start(h: &Hit, character: i32, special: bool, kmh: f32) -> Option<Flight> {
        const SMASH: i32 = 5; // the smash as a kind of its own
        let v = (F(kmh) * 0.9).0;
        let serve = h.hits == 1;
        let shot = if h.branch == branch::SMASH { SMASH } else { h.kind };
        if h.framed && shot != kind::LOB {
            return Some(Flight::Whistle { key: flight::WHISTLE, low: 0.0, top: 10.0 });
        }
        let lola = character == LOLA && special;
        let rush = match shot {
            kind::TOPSPIN | kind::FLAT => serve && v >= RUSH_KMH,
            kind::SLICE => lola && serve,
            kind::LOB => {
                let s = special.then_some(character);
                return Some(Flight::Whistle {
                    key: if matches!(s, Some(5 | 13)) { flight::SPECIAL_WHISTLE } else { flight::WHISTLE },
                    low: if matches!(s, Some(5 | 6 | 9 | 13)) { 0.3 } else { 0.0 },
                    top: if matches!(s, Some(5 | 6 | 9 | 13)) { 8.0 } else { 10.0 },
                });
            }
            kind::DROP => false,
            _ => v >= RUSH_KMH,
        };
        if !rush {
            return None;
        }
        let smash = h.branch == branch::SMASH;
        let bands: [i32; 6] = if lola { [FULL; 6] } else if smash { [0x46, 0x50, 0x5a, 100, 0x6e, 0x80] } else { [0x55, 0x5a, 100, 0x6e, 0x73, 0x80] };
        let band = [149.0, 159.0, 169.0, 179.0, 199.0].iter().take_while(|&&t| v > t).count();
        let key = if lola { flight::LOLA_RUSH } else { flight::RUSH };
        Some(Flight::Rush {
            play: Play { speed: if lola { 1.5 } else { 1.0 }, ..play(slot::COURT, court::FLIGHT, key, bands[band]) },
            threshold: if lola || smash { 50.0 } else { 100.0 },
            count: 0,
        })
    }

    /// The play at the start (ball height `height`).
    pub fn play(&self, height: f32) -> Play {
        match *self {
            Flight::Whistle { key, low, top } => Play { key, speed: flight_speed(height, low, top), ..FLIGHT },
            Flight::Rush { play, .. } => play,
        }
    }

    /// A later frame at ball height `height` and speed `kmh`, with `placed` the volume after falloff the re-placing
    /// gave: the play to re-place it with, and for a fading rush `Some(volume)` to set instead (`Some(0)`: stop).
    pub fn frame(&mut self, height: f32, kmh: f32, placed: i32) -> (Play, Option<i32>) {
        match self {
            Flight::Whistle { key, .. } => (Play { key: *key, speed: flight_speed(height, 0.3, 10.0), ..FLIGHT }, None),
            Flight::Rush { play, threshold, count } => {
                if *count == 0 && kmh <= *threshold {
                    *count = 1;
                }
                if *count == 0 {
                    return (*play, None);
                }
                *count += 1;
                let q = placed / *count;
                (*play, Some(if q < 1 || *count >= 5 { 0 } else { q }))
            }
        }
    }
}

/// The whistle's play speed at ball height `height` (|y|): `low` below 1 m, rising linearly by (2 − low) over `top`
/// metres, 2 from `top` up. The start uses low 0 and top 10, every later frame low 0.3 and top 10.
pub fn flight_speed(height: f32, low: f32, top: f32) -> f32 {
    let h = F(height).abs();
    let s = if h >= top {
        F(2.0)
    } else if h < 1.0 {
        F(low)
    } else {
        ((2.0 - F(low)) / top * (h - 1.0)).abs() + low
    };
    s.clamp(0.0, 2.0).0
}

// The umpire's plays (`slot::UMPIRE`) are not positional: full volume at the centre (`stereo` at bearing 0).
/// Where a non-positional play sounds as if it were placed: straight ahead of the listener, at full volume.
pub const CENTRE: [f32; 3] = [0.0, 0.0, -9.0];
/// The umpire calls the line as the point ends: the judge's call (`judge::verdict`: out, fault, double fault,
/// let, out after the net) is program 1's key call − 1; none for no call (`POINT`) or an illegal hit.
pub fn line_call(call: u8) -> Option<Play> {
    (call != verdict::POINT && call != verdict::ILLEGAL_HIT).then(|| play(slot::UMPIRE, umpire::LINE, call - 1, FULL))
}
/// The umpire's "change ends", as the change-ends phase begins.
pub const CHANGE_ENDS_CALL: Play = play(slot::UMPIRE, umpire::CALL, 3, FULL);
/// The umpire's tiebreak call, as the first tiebreak serve is set up (the game that made 6-all).
pub const TIEBREAK_CALL: Play = play(slot::UMPIRE, umpire::CALL, 0, FULL);
/// The umpire's match call, once the scoreboard has shown the match point (the match ends when it is over).
pub const MATCH_CALL: Play = play(slot::UMPIRE, umpire::CALL, 4, FULL);

/// The umpire's score after a point that wins no game (program 0 keys, in order): one word at deuce (9; 13 for the
/// game's second deuce except on court 5), "advantage" (10) then the server's (11) or the receiver's (12) by who
/// won the point, nothing in a tiebreak, else the serving team's points (0..3) then "all" (8) or the receiving
/// team's points (4..7).
pub fn score_call(s: &crate::score::Score, winner: usize, court: u8) -> Vec<u8> {
    use word::*;
    let side = (s.server & 1) as usize;
    if s.deuce {
        vec![if s.deuce_count == 2 && court != 5 { DEUCE_AGAIN } else { DEUCE }]
    } else if s.advantage {
        vec![ADVANTAGE, if winner == side { SERVER } else { RECEIVER }]
    } else if s.tiebreak {
        vec![]
    } else {
        let (a, b) = (s.points[side], s.points[side ^ 1]);
        vec![SERVING + a as u8, if a == b { ALL } else { RECEIVING + b as u8 }]
    }
}

/// The umpire's score words (program 0's keys).
mod word {
    /// The serving team's points 0..=3, then the receiving team's.
    pub const SERVING: u8 = 0;
    pub const RECEIVING: u8 = 4;
    pub const ALL: u8 = 8;
    pub const DEUCE: u8 = 9;
    pub const ADVANTAGE: u8 = 10;
    /// "… server" / "… receiver" after "advantage".
    pub const SERVER: u8 = 11;
    pub const RECEIVER: u8 = 12;
    /// The game's second deuce.
    pub const DEUCE_AGAIN: u8 = 13;
}

/// Ticks from the point to the score call's first word (the scoreboard's pause, then the umpire's turn): the
/// recordings' sound ring has it 33 (sound_s05) or 34 (hits_s05) frames after the score changed.
const CALL_DELAY: i32 = 33;

/// The umpire's queued score words: the first `CALL_DELAY` ticks after the point, each next one when the last
/// word's gap is over. `gaps` (ticks per key, `exe::Game::umpire_words`) is measured from the word's call; the
/// words land 2 ticks before it.
/// ponytail: the game also moves on once the word stops sounding; with the bank's words shorter than their gaps
/// that never shows in the recordings, so the gap alone times them.
#[derive(Clone, Debug, Default)]
pub struct Umpire {
    words: Vec<u8>,
    wait: i32,
}

impl Umpire {
    /// The point was just scored: call `words` (`score_call`).
    pub fn call(&mut self, words: Vec<u8>) {
        *self = Umpire { words, wait: CALL_DELAY };
    }

    /// Any other umpire call drops the words still to come.
    pub fn hush(&mut self) {
        self.words.clear();
    }

    /// One tick (the first is the tick after `call`): the word due now, if any.
    pub fn step(&mut self, gaps: &[i32; 14]) -> Option<Play> {
        if self.words.is_empty() {
            return None;
        }
        self.wait -= 1;
        if self.wait > 0 {
            return None;
        }
        let key = self.words.remove(0);
        self.wait = gaps[key as usize] - 2;
        Some(play(slot::UMPIRE, umpire::SCORE, key, FULL))
    }
}

/// The gallery's events (`Reaction::event`); the ones up to `GROAN_SHOUT` play the gallery program of their own
/// number.
pub mod event {
    pub const APPLAUSE: u8 = 0;
    pub const GROAN: u8 = 1;
    /// The match's end: the shown side won (the ovation) or lost.
    pub const WON: u8 = 3;
    pub const LOST: u8 = 4;
    /// The shouts after the applause and the groan.
    pub const APPLAUSE_SHOUT: u8 = 5;
    pub const GROAN_SHOUT: u8 = 6;
    /// After the match's end: two cheers (program 7 keys 0, 1) or a sigh (program 8).
    pub const CHEER: u8 = 7;
    pub const CHEER_AGAIN: u8 = 8;
    pub const SIGH: u8 = 9;
}
/// The gallery's rolling cheer program, and the cheers' and the sigh's after the match.
const ROLLING_CHEER: u8 = 10;
const CHEERS: u8 = 7;
const SIGH: u8 = 8;
/// The gallery's bearings (degrees from the listener): the cheer goes round them in order, a reaction shout takes
/// one at random, the applause plays at the first, fourth and seventh at once.
const STANDS: [i32; 8] = [0, 180, 45, 225, 90, 270, 135, 325];
/// The cheer's volume, 0.9 · 128 (the game's level table is 0.9 at every level).
const CHEER_VOLUME: i32 = 115;
/// The applause's and groan's volume, and the shouts'.
const CROWD: i32 = 0x4c;
const SHOUT: i32 = 0x66;

/// How the gallery takes a decided point: `cheer` (the rolling cheer, at once without an `event`, else once the
/// event's shout has had its time), `event` ([`event`]: the applause or groan; at the match's end won or lost)
/// and `chain` (a shout follows the applause or groan; after the match's end two cheers or a sigh).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Reaction {
    pub cheer: bool,
    pub event: Option<u8>,
    pub chain: bool,
}

/// The gallery's reaction to the judge's `call` (`judge::Verdict::call`) on a point that wins a `game` (or set) or
/// not. `applause`: the gallery applauds — on a game point a side it favours (`favoured`), on a plain point one
/// won outright (`play::popups::Finish::applause`) — `Some(true)` with a shout after it (plain points only).
/// `errors` counts the errors in a row (every second one draws a shout); none for a fault or let.
pub fn reaction(call: u8, game: bool, applause: Option<bool>, errors: &mut u32) -> Option<Reaction> {
    use event::{APPLAUSE, GROAN};
    let error = matches!(call, verdict::OUT | verdict::DOUBLE_FAULT | verdict::NET_OUT | verdict::ILLEGAL_HIT);
    match call {
        verdict::FAULT | verdict::LET => None,
        _ if game && error => {
            *errors += 1;
            Some(Reaction { cheer: true, event: Some(GROAN), chain: false })
        }
        _ if game => {
            *errors = 0;
            Some(Reaction { cheer: true, event: applause.map(|_| APPLAUSE), chain: false })
        }
        verdict::ILLEGAL_HIT => Some(Reaction { cheer: false, event: Some(GROAN), chain: false }),
        _ if error => {
            *errors += 1;
            Some(Reaction { cheer: false, event: Some(GROAN), chain: (*errors).is_multiple_of(2) })
        }
        _ => {
            *errors = 0;
            Some(Reaction { cheer: true, event: applause.map(|_| APPLAUSE), chain: applause == Some(true) })
        }
    }
}

/// Whether the gallery favours each side on a game point: a side with a human player, or both when both sides or
/// neither have one.
pub fn favoured(human: &[bool]) -> [bool; 2] {
    let mut side = [false; 2];
    for (i, &h) in human.iter().enumerate() {
        side[i & 1] |= h;
    }
    if side[0] == side[1] { [true; 2] } else { side }
}

/// The gallery (`slot::GALLERY`): after a point the cheer, program 10 at the next stand every 12–40 ticks (21–70 on court
/// 5) at a random pitch, and the applause or groan (programs 0 and 1 from three stands), perhaps followed by a
/// shout (5 or 6 from a random stand). `roll` is the game's random number generator.
/// At the match's end (`event` 3/4) the ovation or groan (program 3 or 4 from three stands) and its follow-ups.
/// ponytail: the match-start cheer, the shout when a player runs
/// near the stands on courts 1, 4 and 6 (program 9) are left out; a shout's end is not waited on. The manager that
/// skips `step` while paused or raining is `play::npcs`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Gallery {
    cheering: bool,
    pending: bool,
    start: i32,
    next: i32,
    stand: usize,
    count: u32,
    event: Option<u8>,
    wait: i32,
    chain: bool,
    /// The last key of each program, for the ones that take turns.
    last: [i32; 7],
}

impl Default for Gallery {
    fn default() -> Self {
        Gallery { cheering: false, pending: false, start: 0, next: 0, stand: 0, count: 0, event: None, wait: 0, chain: false, last: [-1; 7] }
    }
}

impl Gallery {
    /// The point was just decided (`reaction`).
    pub fn point(&mut self, r: Reaction, roll: &mut impl FnMut() -> u32) {
        self.cheering = r.cheer;
        self.count = 0;
        self.stand = (roll() >> 16 & 7) as usize;
        self.next = 1;
        if let Some(e) = r.event {
            self.cheering = false;
            self.pending = r.cheer;
            self.chain = r.chain;
            self.wait = if e == event::LOST || e < event::WON { 1 } else { 0x8c };
            self.event = Some(e);
            self.start = self.wait + if e == event::GROAN { 0x3c } else { 0x1e };
        }
    }

    /// A tick the manager does not run (paused, or raining): its pending reaction is dropped.
    pub fn idle(&mut self) {
        self.event = None;
    }

    /// The next serve: the gallery falls quiet until the next point.
    pub fn hush(&mut self) {
        (self.cheering, self.pending, self.event) = (false, false, None);
    }

    /// One tick (the first is the tick of `point`) on `court` with `players`; `game` when the point won a game.
    /// The plays due now, each at its bearing (whole degrees, not placed: full volume
    /// at that bearing).
    pub fn step(&mut self, court: u8, players: u32, game: bool, roll: &mut impl FnMut() -> u32) -> Vec<(Play, i32)> {
        let mut out = Vec::new();
        if !self.cheering {
            if self.pending && self.start != 0 {
                self.start -= 1;
                if self.start == 0 {
                    (self.pending, self.cheering) = (false, true);
                }
            }
        } else if self.next != 0 {
            self.next -= 1;
            if self.next == 0 {
                let key = match (court == 5, self.count) {
                    (false, 0) => 0,
                    (false, 1) => 2,
                    (false, _) => 3,
                    (true, 0) => 1,
                    (true, 1) => 4,
                    (true, _) => 5,
                };
                self.count += 1;
                let speed = (F(roll() as f32) * 2.3283064e-10 * (F(1.25) - 0.875) + 0.875).0;
                out.push((Play { speed, ..play(slot::GALLERY, ROLLING_CHEER, key, CHEER_VOLUME) }, STANDS[self.stand]));
                self.stand = (self.stand + 1) % 8;
                let every = if court == 5 { 70.0 } else { 40.0 };
                self.next = (F(every) * (1.0 - 0.7 * F(roll() as f32) * 2.3283064e-10)).0 as i32;
            }
        }
        if players < 2 || self.wait < 1 {
            return out;
        }
        let Some(e) = self.event else { return out };
        self.wait -= 1;
        if self.wait != 0 {
            return out;
        }
        let mut turn = |p: usize, n: i32| {
            self.last[p] = (self.last[p] + 1) % n;
            self.last[p] as u8
        };
        use event::*;
        match e {
            APPLAUSE | GROAN => {
                let key = turn(e as usize, 3 - e as i32);
                out.extend([0, 3, 6].map(|s| (play(slot::GALLERY, e, key, CROWD), STANDS[s])));
            }
            APPLAUSE_SHOUT | GROAN_SHOUT => {
                let key = turn(e as usize, if e == APPLAUSE_SHOUT { 3 } else { 2 });
                out.push((play(slot::GALLERY, e, key, SHOUT), STANDS[(roll() >> 16 & 7) as usize]));
            }
            // the game picks WON when the shown side is the winner, else LOST: the ceremony's event says the same
            WON | LOST => out.extend([0, 3, 6].map(|s| (play(slot::GALLERY, e, 0, CROWD), STANDS[s]))),
            CHEER..=SIGH => {
                let program = if e == SIGH { self::SIGH } else { CHEERS };
                out.push((play(slot::GALLERY, program, (e == CHEER_AGAIN) as u8, SHOUT), STANDS[(roll() >> 16 & 7) as usize]));
            }
            _ => {}
        }
        let next = match e {
            APPLAUSE => Some(APPLAUSE_SHOUT),
            GROAN => Some(GROAN_SHOUT),
            WON => Some(CHEER),
            LOST => Some(SIGH),
            CHEER => Some(CHEER_AGAIN),
            _ => None,
        };
        if let Some(n) = next.filter(|_| self.chain) {
            self.event = Some(n);
            self.wait = match n {
                SIGH => 0x78,
                CHEER | CHEER_AGAIN => 0x3c,
                _ if game => 0xa0,
                _ => 0x3c,
            };
        } else {
            self.event = None;
        }
        out
    }
}
