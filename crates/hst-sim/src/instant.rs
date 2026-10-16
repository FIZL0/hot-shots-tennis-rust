//! The instant replay's decision: whether a decided point gets an instant replay, ported from the match
//! handler's point message (the replay run, its cameras and the between-points skip are later tasks).
//!
//! It runs once per point in a match (practice has none), after the scoreboard has awarded the point. It also
//! keeps two flags of the replay recorder: `allowed` (the point may still replay; set here unless the rally
//! ran too long or the replay option cut it) and `recorded` (the rally left enough recorded shots to replay).
#![deny(clippy::float_arithmetic)]

use crate::lines::{BASELINE, SERVICE_LINE, SINGLES_SIDELINE, sideline};
use crate::ps2::F;

/// The rally block's states the decision reads (`Point::rally`).
pub mod rally {
    /// The serve wasn't returned.
    pub const UNRETURNED: u8 = 0;
    /// No rally to show.
    pub const NONE: u8 = 3;
}

/// The instant replay option (`Point::option`).
pub mod option {
    pub const OFF: u8 = 0;
    pub const ON: u8 = 1;
    /// On for long rallies too.
    pub const LONG: u8 = 2;
}

/// Controllers below this are human pads (`Point::controller`).
pub const FIRST_CPU: i32 = 0x20;
/// The ball kept no shot this rally to replay in these states (`Point::ball_state`).
const BALL_NO_REPLAY: [u8; 2] = [4, 5];
/// The ball's last-shot kinds: fast ones (replayed above their speed) and the special one (`Point::ball_kind`).
const BALL_FAST: u8 = 1;
const BALL_FASTER: u8 = 2;
const BALL_SPECIAL: u8 = 3;
/// A bounce this close to a line (m) is a call worth replaying.
const CLOSE_CALL: f32 = 0.1;

/// What the decision reads. Player indices are 0..4; `None` where the game keeps −1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Players on court (2 singles, 4 doubles); under 2 is practice.
    pub players: i32,
    /// The scoreboard awarded a point (false once the match is already decided).
    pub awarded: bool,
    /// An instant replay is running (this point is the replay's own).
    pub replaying: bool,
    /// The point may replay (in), and the rally cancelled it (too long, or the option is off for long rallies).
    pub allowed: bool,
    pub cancelled: bool,
    /// The rally left recorded shots to replay (in/out).
    pub recorded: bool,
    /// Shots the replay recorder holds for this rally.
    pub record_shots: i32,
    /// The point's winner by player, and the player who hit last.
    pub winner: Option<usize>,
    pub hitter: Option<usize>,
    /// The receiver of this point's serve.
    pub receiver: usize,
    /// The rally block's state (`rally`).
    pub rally: u8,
    /// Shots this rally (1 = the serve).
    pub shots: i32,
    /// Faults before this serve (2 = a double fault).
    pub faults: i32,
    /// The ball's velocity (m/frame), its last shot's kind and sub-kind as the ball keeps them (kinds 1 and 2
    /// replay above 90 and 70 km/h, the special kind 3 with sub-kind 0 always), its state (4/5: no replay) and its
    /// bounces.
    pub ball_vel: [f32; 3],
    pub ball_kind: u8,
    pub ball_sub: i32,
    pub ball_state: u8,
    pub ball_bounces: i32,
    /// The side the ball went out on is set (the game's −1 = none).
    pub out_side: bool,
    /// The ball mark of the last first bounce on the court (x, z): the landing spot, then its tail (the spot pushed
    /// along the ball's ground speed × 0.8).
    pub bounces: [[f32; 2]; 2],
    /// The judge's verdict kind (0 = a plain point).
    pub verdict: u8,
    /// Attract (demo) play: replays regardless of the option and controllers.
    pub demo: bool,
    /// The point won the match.
    pub match_won: bool,
    /// The instant replay option (`option`).
    pub option: u8,
    /// The hitter's controller (under `FIRST_CPU` = a human pad).
    pub controller: i32,
    /// The hitter's last swing timing (frames off perfect, signed).
    pub timing: i32,
    /// The scoreboard's event: 0 a point, 1 a game, 2 a set.
    pub event: i32,
    /// Ticks the rally ran.
    pub ticks: i32,
}

/// What the decision does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Decision {
    /// Run an instant replay of this point.
    pub replay: bool,
    /// The post-point camera's pick is reset (a close line call replayed).
    pub reset_cut: bool,
    pub allowed: bool,
    pub recorded: bool,
}

/// The ball's speed in km/h, as the game computes it.
pub fn speed_kmh(v: [f32; 3]) -> f32 {
    let acc = (F(v[1]) * v[1]).madd(v[0], v[0]);
    let s = acc.madd(v[2], v[2]).sqrt();
    (3600.0 * (60.0 * s) / 1000.0).0
}

/// A bounce's distances past the side line and the base line (singles lines while serving, `wide` for the
/// doubles alleys).
fn past_lines(p: [f32; 2], shots: i32, wide: bool) -> (f32, f32) {
    if shots < 2 {
        // the service box: nearer of the centre line and the side line
        let x = F(p[0].abs());
        let d = x - SINGLES_SIDELINE;
        (if x.abs() <= d.abs() { x.0 } else { d.0 }, (F(p[1].abs()) - SERVICE_LINE).0)
    } else {
        ((F(p[0].abs()) - sideline(wide)).0, (F(p[1].abs()) - BASELINE).0)
    }
}

/// The ball's last bounces make a line call worth showing: one landed within `CLOSE_CALL` of a line, or they fell on
/// the two sides of one (no call for a ball gone out before its second bounce, or a judged verdict).
fn line_call(p: &Point) -> bool {
    if !(!p.out_side || p.ball_bounces > 1) || p.verdict != crate::judge::verdict::POINT {
        return false;
    }
    let wide = p.players > 2;
    let (a, b) = past_lines(p.bounces[0], p.shots, wide);
    let (c, d) = past_lines(p.bounces[1], p.shots, wide);
    if !(a.abs() > CLOSE_CALL && c.abs() > CLOSE_CALL && b.abs() > CLOSE_CALL && d.abs() > CLOSE_CALL) {
        return true;
    }
    if a <= 0.0 && c >= 0.0 {
        return true;
    }
    !(b > 0.0 || d < 0.0)
}

/// Whether the point replays (the match handler's point message, after the score).
pub fn decide(p: &Point) -> Decision {
    let mut o = Decision { allowed: p.allowed, recorded: p.recorded, ..Decision::default() };
    if p.players < 2 || !p.awarded {
        return o;
    }
    if !p.cancelled {
        o.allowed = true;
    }
    if p.replaying {
        return o;
    }
    let Some(hitter) = p.hitter else { return o };
    let hitter_won = p.winner == Some(hitter);
    if p.rally == rally::NONE || (p.shots < 3 && p.rally == rally::UNRETURNED) || (p.shots == 2 && hitter != p.receiver) || p.faults == 2 {
        return o;
    }
    if o.allowed && p.record_shots.wrapping_add(1) > 1 {
        o.recorded = true;
    }
    if !hitter_won {
        return o;
    }
    let speed = || speed_kmh(p.ball_vel);
    let ace = p.shots == 1 && p.faults == 0 && speed() > 100.0;
    let special = p.ball_kind == BALL_SPECIAL && p.ball_sub == 0;
    let not_return = p.shots != 2;
    let line = line_call(p);
    let fast_kind = || (p.ball_kind == BALL_FAST && speed() > 90.0) || (p.ball_kind == BALL_FASTER && speed() > 70.0);
    let human = p.demo || p.controller < FIRST_CPU;
    let plain = p.verdict == crate::judge::verdict::POINT;
    let replay = if o.recorded && p.match_won {
        true
    } else if !p.demo && (BALL_NO_REPLAY.contains(&p.ball_state) || !o.recorded || p.option == option::OFF) {
        false
    } else if human && p.event > 0 && plain && (ace || special || !not_return || fast_kind()) {
        true
    } else if (!p.demo && p.option != option::LONG) || p.event > 0 || !plain || !human {
        false
    } else if special {
        true
    } else if !ace
        && not_return
        && (!line || p.timing.wrapping_abs() > 1 || p.ball_kind.wrapping_sub(1) > 1 || p.ball_sub == 3 || speed() < 100.0)
    {
        p.shots >= 10 && p.ticks <= 900 && fast_kind()
    } else {
        true
    };
    if replay {
        o.replay = true;
        o.reset_cut = line && p.shots > 2 && !special;
    }
    o
}
