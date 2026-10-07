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
/// A missed swing's shout program, at the contact pose where the stroke turns into its miss motion. It is muted
/// when the player's previous swing also ended in its miss motion with no swing start between (the re-press window
/// a miss opens stays open, even across points, until a stroke press or a serve swing starts): of a run of
/// whiffs only the first shouts. A missed serve swing always shouts.
pub const WHIFF_SHOUT: u8 = 4;

/// A doubles CPU player's call (program 6 key 3 or 4 by a random bit, 0x80 at the player) as it leaves an incoming
/// ball to its partner, one time in four.
/// ponytail: the reaction voices after a point (programs 7–10 by reaction motion) only play for a player the
/// post-point camera has in close view; left out with that check.
pub fn call_out(player: usize, bit: bool) -> Play {
    play(1 + player as u8, 6, 3 + bit as u8, 0x80)
}

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
    /// players, else 1; the whiff's hi is 1 with exactly two players, else 0.
    pub fn shout(&mut self, player: usize, program: u8, players: u32, r: u32) -> Play {
        let hi = if program == WHIFF_SHOUT { (players == 2) as i32 } else if players > 2 { 1 } else if program == 1 { 4 } else { 2 };
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

/// The umpire's voice bank slot; the umpire's plays are not positional: 0x80 at the centre (`stereo` at bearing 0).
const UMPIRE: u8 = 5;
/// Where a non-positional play sounds as if it were placed: straight ahead of the listener, at full volume.
pub const CENTRE: [f32; 3] = [0.0, 0.0, -9.0];
/// The umpire calls the line as the point ends: the judge's call (1 out, 2 fault, 3 double fault, 4 let, 5 out
/// after the net) is program 1's key call − 1; none for no call (0) or 6.
pub fn line_call(call: u8) -> Option<Play> {
    (call != 0 && call != 6).then(|| play(UMPIRE, 1, call - 1, 0x80))
}
/// The umpire's "change ends", as the change-ends phase begins.
pub const CHANGE_ENDS_CALL: Play = play(UMPIRE, 2, 3, 0x80);
/// The umpire's tiebreak call, as the first tiebreak serve is set up (the game that made 6-all).
pub const TIEBREAK_CALL: Play = play(UMPIRE, 2, 0, 0x80);
/// The umpire's match call, once the scoreboard has shown the match point (the match ends when it is over).
pub const MATCH_CALL: Play = play(UMPIRE, 2, 4, 0x80);

/// The umpire's score after a point that wins no game (program 0 keys, in order): one word at deuce (9; 13 for the
/// game's second deuce except on court 5), "advantage" (10) then the server's (11) or the receiver's (12) by who
/// won the point, nothing in a tiebreak, else the serving team's points (0..3) then "all" (8) or the receiving
/// team's points (4..7).
pub fn score_call(s: &crate::score::Score, winner: usize, court: u8) -> Vec<u8> {
    let side = (s.server & 1) as usize;
    if s.deuce {
        vec![if s.deuce_count == 2 && court != 5 { 13 } else { 9 }]
    } else if s.advantage {
        vec![10, if winner == side { 11 } else { 12 }]
    } else if s.tiebreak {
        vec![]
    } else {
        let (a, b) = (s.points[side], s.points[side ^ 1]);
        vec![a as u8, if a == b { 8 } else { b as u8 + 4 }]
    }
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
        Some(play(UMPIRE, 0, key, 0x80))
    }
}

/// The gallery's bank slot (the court archive's `galsg` bank).
const GALLERY: u8 = 6;
/// The gallery's bearings (degrees from the listener): the cheer goes round them in order, a reaction shout takes
/// one at random, the applause plays at the first, fourth and seventh at once.
const STANDS: [i32; 8] = [0, 180, 45, 225, 90, 270, 135, 325];
/// The cheer's volume, 0.9 · 128 (the game's level table is 0.9 at every level).
const CHEER: i32 = 115;
/// The applause's and groan's volume, and the shouts'.
const CROWD: i32 = 0x4c;
const SHOUT: i32 = 0x66;

/// How the gallery takes a decided point: `cheer` (the rolling cheer, at once without an `event`, else once the
/// event's shout has had its time), `event` (0 applause, 1 groan) and `chain` (a shout follows: 5 after the
/// applause, 6 after the groan).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reaction {
    pub cheer: bool,
    pub event: Option<u8>,
    pub chain: bool,
}

/// The gallery's reaction to the judge's `call` (`judge::Verdict::call`) on a point that wins a `game` (or set) or
/// not. `applause`: the point was won outright — on a plain point a smash winner, on a game point a side the
/// gallery favours (`favoured`). `errors` counts the errors in a row (every second one draws a shout); none for a
/// fault or let.
/// ponytail: a plain point is also applauded for an ace with a strong toss, a return winner, a dive winner and a
/// ground or volley winner that passes the receivers; only smash winners are told apart yet.
pub fn reaction(call: u8, game: bool, applause: bool, errors: &mut u32) -> Option<Reaction> {
    let error = matches!(call, 1 | 3 | 5 | 6);
    match call {
        2 | 4 => None,
        _ if game && error => {
            *errors += 1;
            Some(Reaction { cheer: true, event: Some(1), chain: false })
        }
        _ if game => {
            *errors = 0;
            Some(Reaction { cheer: true, event: applause.then_some(0), chain: false })
        }
        6 => Some(Reaction { cheer: false, event: Some(1), chain: false }),
        _ if error => {
            *errors += 1;
            Some(Reaction { cheer: false, event: Some(1), chain: *errors % 2 == 0 })
        }
        _ => {
            *errors = 0;
            Some(Reaction { cheer: true, event: applause.then_some(0), chain: false })
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

/// The gallery (slot 6): after a point the cheer, program 10 at the next stand every 12–40 ticks (21–70 on court
/// 5) at a random pitch, and the applause or groan (programs 0 and 1 from three stands), perhaps followed by a
/// shout (5 or 6 from a random stand). `roll` is the game's random number generator.
/// ponytail: the match-start cheer, the match-end ceremony (programs 3/4 then 7/8), the shout when a player runs
/// near the stands on courts 1, 4 and 6 (program 9) and the gallery's silence in the rain are left out; a shout's
/// end is not waited on.
#[derive(Clone, Debug)]
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
            self.wait = if e == 4 || e < 2 { 1 } else { 0x8c };
            self.event = Some(e);
            self.start = self.wait + if e == 1 { 0x3c } else { 0x1e };
        }
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
                let speed = roll() as f32 * 2.3283064e-10 * (1.25 - 0.875) + 0.875;
                out.push((Play { speed, ..play(GALLERY, 10, key, CHEER) }, STANDS[self.stand]));
                self.stand = (self.stand + 1) % 8;
                let every = if court == 5 { 70.0 } else { 40.0 };
                self.next = (every * (1.0 - 0.7 * roll() as f32 * 2.3283064e-10)) as i32;
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
        match e {
            0 | 1 => {
                let key = turn(e as usize, 3 - e as i32);
                out.extend([0, 3, 6].map(|s| (play(GALLERY, e, key, CROWD), STANDS[s])));
            }
            5 | 6 => {
                let key = turn(e as usize, if e == 5 { 3 } else { 2 });
                out.push((play(GALLERY, e, key, SHOUT), STANDS[(roll() >> 16 & 7) as usize]));
            }
            _ => {}
        }
        if self.chain && e < 2 {
            self.event = Some(e + 5);
            self.wait = if game { 0xa0 } else { 0x3c };
        } else {
            self.event = None;
        }
        out
    }
}
