//! The instant replay's decision: whether a decided point gets an instant replay, ported from the match
//! handler's point message (the replay run, its cameras and the between-points skip are later tasks).
//!
//! It runs once per point in a match (practice has none), after the scoreboard has awarded the point. It also
//! keeps two flags of the replay recorder: `allowed` (the point may still replay; set here unless the rally
//! ran too long or the replay option cut it) and `recorded` (the rally left enough recorded shots to replay).

use crate::ps2;

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
    /// The rally block's state (3 = no rally to show, 0 = the serve wasn't returned).
    pub rally: u8,
    /// Shots this rally (1 = the serve).
    pub shots: i32,
    /// Faults before this serve (2 = a double fault).
    pub faults: i32,
    /// The ball's velocity (m/frame), its last shot's kind and sub-kind as the ball keeps them (kinds 1 and 2
    /// replay above 90 and 70 km/h, kind 3 with sub-kind 0 always), its state (4/5: no replay) and its bounces.
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
    /// The instant replay option: 0 off, 1 on, 2 on for long rallies too.
    pub option: u8,
    /// The hitter's controller (under 0x20 = a human pad).
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
    let acc = ps2::madd(ps2::mul(v[1], v[1]), v[0], v[0]);
    let s = ps2::sqrt(ps2::madd(acc, v[2], v[2]));
    ps2::div(ps2::mul(3600.0, ps2::mul(60.0, s)), 1000.0)
}

/// A bounce's distances past the side line and the base line (singles lines while serving, `wide` for the
/// doubles alleys).
fn past_lines(p: [f32; 2], shots: i32, wide: bool) -> (f32, f32) {
    if shots < 2 {
        // the service box: nearer of the centre line and the side line
        let x = p[0].abs();
        let d = x - 4.115;
        (if x.abs() <= d.abs() { x } else { d }, p[1].abs() - 6.4)
    } else {
        (p[0].abs() - if wide { 5.485 } else { 4.115 }, p[1].abs() - 11.885)
    }
}

/// The ball's last bounces make a line call worth showing: one landed within 0.1 m of a line, or they fell on
/// the two sides of one (no call for a ball gone out before its second bounce, or a judged verdict).
fn line_call(p: &Point) -> bool {
    if !(!p.out_side || p.ball_bounces > 1) || p.verdict != 0 {
        return false;
    }
    let wide = p.players > 2;
    let (a, b) = past_lines(p.bounces[0], p.shots, wide);
    let (c, d) = past_lines(p.bounces[1], p.shots, wide);
    if !(a.abs() > 0.1 && c.abs() > 0.1 && b.abs() > 0.1 && d.abs() > 0.1) {
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
    if p.rally == 3 || (p.shots < 3 && p.rally == 0) || (p.shots == 2 && hitter != p.receiver) || p.faults == 2 {
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
    let special = p.ball_kind == 3 && p.ball_sub == 0;
    let not_return = p.shots != 2;
    let line = line_call(p);
    let fast_kind = || (p.ball_kind == 1 && speed() > 90.0) || (p.ball_kind == 2 && speed() > 70.0);
    let human = p.demo || p.controller < 0x20;
    let replay = if o.recorded && p.match_won {
        true
    } else if !p.demo && (p.ball_state == 4 || p.ball_state == 5 || !o.recorded || p.option == 0) {
        false
    } else if human && p.event > 0 && p.verdict == 0 && (ace || special || !not_return || fast_kind()) {
        true
    } else if (!p.demo && p.option != 2) || p.event > 0 || p.verdict != 0 || !human {
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
