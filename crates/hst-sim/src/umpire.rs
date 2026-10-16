//! The chair umpire, ported from the original's umpire actor (only in games of 2+ players).
//!
//! The match tells the umpire about its phases; the umpire calls the score in words after each point, calls
//! faults, outs and lets, announces games, sets, the tiebreak, change of ends and the match, turns toward the
//! point's winner and follows the ball with her head during the rally.
//!
//! Voices are (program, key) of her voice bank (`gag_vcNN`, sound slot 5). Program 0 holds the score words
//! (keys 0..4 the server's points love..40, 4 + n the receiver's, 8 "all", 9 deuce, 10 advantage, 11/12 server /
//! receiver, 13 deuce again: no disc umpire has a take for it, so it's silent), program 1 the calls (key = judge
//! call − 1), program 2 the announcements (0 tiebreak, 1 game, 2 set, 3 change ends, 4 match). Words checked by
//! transcript of the five disc banks (M8f2). A two-part call plays its second word when the first voice ends, or
//! at the latest after the first word's countdown.
#![deny(clippy::float_arithmetic)]

use crate::judge::verdict;
use crate::npc::STADIUM;
use crate::ps2::{F, f32s};
use crate::score::{Event, Score};

/// Ticks a voice still counts as sounding after its clip has played out (the clip's length in 60 Hz frames, as
/// the mixer renders it): the game hears of the stop through the sound driver's round trip. Counted from the tick
/// the caller plays it, it holds for her own words (her next word: clip + 5) and, since the app plays the line call
/// a tick before the game does (at the verdict, not the next tick's phase entry), for the call sprite's settle too
/// (the scoreboard's read is clip + 4 after the game's play). The game's own match-over wait reads one tick
/// sooner still (`PostPoint::calling_match`). Measured on slot 5 (umpire 4 a): seven clips, the mode of each; a
/// tick less in about one case in four (the driver isn't locked to the frame).
pub const VOICE_LAG: u32 = 5;

/// Motions: idle, turned to her right, turned to her left (anims `_h`, `_r`, `_l`).
pub const IDLE: u8 = 0;
pub const RIGHT: u8 = 1;
pub const LEFT: u8 = 2;

/// Her voice bank's programs.
pub mod program {
    /// The score in words (`word`).
    pub const SCORE: u8 = 0;
    /// The line calls: key = the judge's verdict − 1.
    pub const CALL: u8 = 1;
    /// The announcements (`announce`).
    pub const ANNOUNCE: u8 = 2;
}

/// The score program's keys.
pub mod word {
    /// The server's points love..40 are keys 0..4; the receiver's are `RECEIVER_POINTS` + the points.
    pub const RECEIVER_POINTS: i32 = 4;
    pub const ALL: i32 = 8;
    pub const DEUCE: i32 = 9;
    pub const ADVANTAGE: i32 = 10;
    /// "server" / "receiver" after a game, set or advantage.
    pub const SERVER: i32 = 11;
    pub const RECEIVER: i32 = 12;
    /// Deuce again (silent: no disc umpire has a take).
    pub const DEUCE_AGAIN: i32 = 13;
}

/// The announcement program's keys.
pub mod announce {
    pub const TIEBREAK: i32 = 0;
    pub const GAME: i32 = 1;
    pub const SET: i32 = 2;
    pub const CHANGE_ENDS: i32 = 3;
    pub const MATCH: i32 = 4;
}

/// A queued call's part: none, the first word, the second, the single-word deuce.
mod part {
    pub const NONE: i32 = -1;
    pub const FIRST: i32 = 0;
    pub const SECOND: i32 = 1;
    pub const DEUCE: i32 = 3;
}

/// The ball is watched within the service line, and the head looks no lower than 2 m up.
const NET_ZONE: f32 = crate::lines::SERVICE_LINE;
const HEAD_LOW: f32 = -2.0;
/// Ticks the eye takes to ease back to the centre.
const BACK_TICKS: i32 = 10;
const BACK_STEP: f32 = 0.1;

/// What the head looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Look {
    /// The court centre (and the eye eases there too, as for `Back`).
    Centre,
    /// The ball during the rally.
    Ball,
    /// Easing back to the court centre after the point (10 ticks).
    Back,
}

/// Where the head looks when it isn't watching the ball: 2 m up over the court centre.
const CENTRE: [f32; 3] = [0.0, -2.0, 0.0];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Umpire {
    /// Chair position (game space) and the direction she faces (x, z), toward the court centre.
    pub pos: [f32; 3],
    pub forward: [f32; 2],
    /// The chair is on the side whose turn motions are mirrored (from the court's table).
    pub side: bool,
    court: u8,
    /// Score-word countdowns (program 0, 14 keys) and announcement countdowns (program 2, 5 keys), in ticks.
    words: ([i32; 14], [i32; 5]),
    pub motion: u8,
    /// The match is over: the idle motion plays on.
    pub over: bool,
    /// A tiebreak announcement is due at the next serve, and its copy kept for replays.
    pub tiebreak: [bool; 2],
    pub look: Look,
    pub look_ticks: i32,
    pub look_step: [f32; 3],
    /// The last look target she stored; `head` is what the head looks at this tick.
    pub eye: [f32; 3],
    pub head: [f32; 3],
    pub rally: bool,
    /// The ball has come within 6.4 m of the net this rally.
    pub seen: bool,
    /// A queued call is playing: the part being played (`part`), its keys and programs.
    pub calling: bool,
    pub part: i32,
    pub keys: [i32; 2],
    pub programs: [u8; 2],
    /// A voice has been played since the call was queued.
    pub voiced: bool,
    pub countdown: i32,
    /// The voice to play this tick, if any (program, key). The caller plays it, stopping the previous one.
    pub voice: Option<(u8, i32)>,
}

impl Umpire {
    /// `pos` the chair's placement, `side` the court's umpire side flag, `court` the court number, `words` the
    /// countdowns for this umpire and language (`hst_data::exe::Game::umpire_words`), `ball` the ball position.
    pub fn new(pos: [f32; 3], side: bool, court: u8, words: ([i32; 14], [i32; 5]), ball: [f32; 3]) -> Self {
        let mut u = Umpire {
            pos,
            forward: [0.0; 2],
            side,
            court,
            words,
            motion: IDLE,
            over: false,
            tiebreak: [false; 2],
            look: Look::Centre,
            look_ticks: 0,
            look_step: [0.0; 3],
            eye: ball,
            head: CENTRE,
            rally: false,
            seen: false,
            calling: false,
            part: part::NONE,
            keys: [0; 2],
            programs: [0; 2],
            voiced: false,
            countdown: 0,
            voice: None,
        };
        u.reset(ball);
        u
    }

    /// Back to her seat state: no call, idle, facing the court centre, the eye on the ball.
    fn reset(&mut self, ball: [f32; 3]) {
        let (x, z) = (F::ZERO - self.pos[0], F::ZERO - self.pos[2]);
        let n = F::ONE / (x * x + z * z).sqrt();
        self.forward = f32s([x * n, z * n]);
        (self.look, self.look_ticks, self.look_step) = (Look::Centre, 0, [0.0; 3]);
        (self.rally, self.seen, self.calling, self.part, self.keys, self.programs) = (false, false, false, part::NONE, [0; 2], [0; 2]);
        (self.over, self.motion, self.eye) = (false, IDLE, ball);
    }

    fn play(&mut self, program: u8, key: i32) {
        self.voice = Some((program, key));
        self.voiced = true;
        self.countdown = 0;
        let next = match (program, key) {
            (program::ANNOUNCE, announce::GAME | announce::SET) => Some(self.words.1[key as usize]),
            (program::SCORE, _) => Some(self.words.0[key as usize]),
            _ => None,
        };
        if let Some(c) = next {
            if self.part == part::FIRST {
                self.part = part::SECOND;
            } else {
                self.calling = false;
            }
            self.countdown = c;
        }
    }

    /// The match starts (phase 0) or the players change ends (`change_ends`: also announced).
    pub fn start(&mut self, change_ends: bool, ball: [f32; 3]) {
        self.reset(ball);
        if change_ends {
            self.play(program::ANNOUNCE, announce::CHANGE_ENDS);
        }
    }

    /// A serve is set up. `fresh`: not straight after the match start or change of ends (those already reset
    /// her). `replay`: the instant replay re-runs the serve (its tiebreak announcement plays again).
    pub fn serve(&mut self, fresh: bool, replay: bool, ball: [f32; 3]) {
        if fresh {
            self.reset(ball);
        }
        (self.seen, self.rally, self.look) = (false, false, Look::Ball);
        if replay {
            self.tiebreak[0] = self.tiebreak[1];
        } else {
            self.tiebreak[1] = self.tiebreak[0];
        }
        if self.tiebreak[0] {
            self.play(program::ANNOUNCE, announce::TIEBREAK);
            self.tiebreak = [false; 2];
        }
    }

    /// The ball is in play.
    pub fn rally(&mut self) {
        self.rally = true;
    }

    /// The point is over: `event` what it did to the score (`None` for no point: a fault or let), `winner` the
    /// team that won it, `swapped` the teams' ends have changed, `call` the judge's verdict (`judge::verdict`).
    pub fn point_over(&mut self, event: Option<Event>, winner: i32, swapped: bool, call: u8) {
        if event.is_some() {
            // she turns toward the winner's end
            self.motion = if (winner == 0) ^ swapped ^ self.side { RIGHT } else { LEFT };
        }
        if call != verdict::POINT && call != verdict::ILLEGAL_HIT {
            self.play(program::CALL, call as i32 - 1);
        }
        (self.look, self.look_ticks) = (Look::Back, 0);
    }

    /// The scoreboard shows the new score: she queues the score in words (after a point). `winner` the team that
    /// won the point.
    pub fn call_score(&mut self, s: &Score, event: Option<Event>, winner: i32) {
        self.calling = true;
        let receiver = (s.server != 0 && s.server != 2) as usize;
        (self.voiced, self.countdown) = (false, 0);
        match event {
            Some(Event::Point | Event::TiebreakPoint) => {
                (self.part, self.programs[0], self.programs[1]) = (part::FIRST, program::SCORE, program::SCORE);
                if s.deuce {
                    self.part = part::DEUCE;
                    self.keys[0] = if s.deuce_count == 2 && self.court != STADIUM { word::DEUCE_AGAIN } else { word::DEUCE };
                } else if s.advantage {
                    self.keys = [word::ADVANTAGE, if receiver as i32 == winner { word::SERVER } else { word::RECEIVER }];
                } else if s.tiebreak {
                    self.calling = false;
                } else {
                    let (a, b) = (s.points[receiver], s.points[1 - receiver]);
                    self.keys = if a == b { [a, word::ALL] } else { [a, b + word::RECEIVER_POINTS] };
                }
            }
            Some(_) => {
                self.calling = false;
                self.tiebreak[0] = s.tiebreak;
            }
            None => self.calling = false,
        }
    }

    /// The scoreboard starts showing a game or set: she announces it and who won it.
    pub fn announce(&mut self, s: &Score, event: Event, winner: i32) {
        let receiver = (s.server != 0 && s.server != 2) as i32;
        let key = match event {
            Event::Game => announce::GAME,
            Event::Set => announce::SET,
            _ => return,
        };
        (self.calling, self.part, self.programs[0], self.programs[1]) = (true, part::FIRST, program::ANNOUNCE, program::SCORE);
        self.keys = [key, if receiver == winner { word::SERVER } else { word::RECEIVER }];
    }

    /// The match is decided: she announces it (the match then waits for the voice to end).
    pub fn call_match(&mut self) {
        self.play(program::ANNOUNCE, announce::MATCH);
    }

    /// The match-over phase begins.
    pub fn match_over(&mut self) {
        self.tiebreak = [false; 2];
        (self.over, self.motion) = (true, IDLE);
    }

    /// One tick. `playing`: her last voice is still sounding.
    pub fn step(&mut self, playing: bool, ball: [f32; 3]) {
        self.voice = None;
        if self.calling {
            self.countdown -= 1;
            if !playing || !self.voiced || self.countdown <= 0 {
                let i = self.part;
                // the deuce word reads a program byte that is never set: the score's
                let program = self.programs.get(i as usize).copied().unwrap_or(program::SCORE);
                self.play(program, self.keys[if i == part::FIRST || i == part::SECOND { i as usize } else { 0 }]);
            }
        }
        if self.rally && ball[2].abs() < NET_ZONE {
            self.seen = true;
        }
        if self.look == Look::Ball {
            let at = [ball[0], ball[1].min(HEAD_LOW), ball[2]];
            if ball[2].abs() < NET_ZONE || !self.seen {
                (self.head, self.eye) = (at, at);
            }
            return;
        }
        if self.look == Look::Centre {
            self.head = CENTRE;
        }
        // both ease the eye back to the centre
        if self.look_ticks == 0 {
            self.look_step = std::array::from_fn(|k| ((F(CENTRE[k]) - self.eye[k]) * BACK_STEP).0);
        }
        if self.look_ticks < BACK_TICKS {
            self.eye = std::array::from_fn(|k| (F(self.eye[k]) + self.look_step[k]).0);
            self.head = self.eye;
            self.look_ticks += 1;
        }
    }
}
