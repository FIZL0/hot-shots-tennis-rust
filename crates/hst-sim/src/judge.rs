//! Line calls and point judging, ported from the original: the call made when a ball lands, the per-frame
//! "is the point over?" check, the check made on every hit, and the umpire's verdict.
//! Players 0..3; team = player & 1 (0 and 2 against 1 and 3).
#![deny(clippy::float_arithmetic)]

use crate::lines::{BASELINE, DOUBLES_SIDELINE, SERVICE_LINE, SINGLES_SIDELINE};
use crate::ps2::F;

/// A ball's line call (the ball keeps it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Call {
    /// Not landed yet.
    #[default]
    None = 0,
    In = 1,
    Out = 2,
    /// Touched the net before landing.
    Net = 3,
    /// Landed in after touching the net.
    NetIn = 4,
    /// Landed out after touching the net (or stopped on it).
    NetOut = 5,
}

/// What the line call needs to know about the shot in play.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Lines {
    /// Shots this rally (1 = the serve, 0 = no shot yet).
    pub shots: i32,
    /// Four players on court: rally balls may land in the doubles alleys.
    pub doubles: bool,
    /// Court side being served from (`score::DEUCE_COURT`, `score::AD_COURT`).
    pub side: i32,
    /// The hitter stands on the +z half, so the ball must land on the −z half.
    pub hitter_far: bool,
    /// Tolerance added to every line (game data).
    pub margin: f32,
}

/// The serve may cross the centre service line by this much (plus the margin).
const CENTRE_SLACK: f32 = 0.05;
/// A ball that has not been called is outside the arena past these (|x|, |z|).
const ARENA: (f32, f32) = (10.685, 19.885);

fn same_team(a: i32, b: i32) -> bool {
    a >= 0 && b >= 0 && a & 1 == b & 1
}

/// Call a contact at `at` (the ball's centre at contact). `call` is the ball's call so far, `net` whether it
/// has touched the net. Returns the new call, and the signed distance to the nearest line (negative inside)
/// when the landing was measured.
pub fn call_landing(call: Call, net: bool, at: [f32; 4], l: &Lines) -> Option<(Call, Option<f32>)> {
    if call != Call::None && call != Call::Net {
        return None;
    }
    if l.shots == 0 {
        return Some((Call::Out, None));
    }
    let serve = l.shots < 2;
    if net && call != Call::Net {
        return Some((Call::Net, None));
    }
    // ponytail: the original also skips contacts with material 0; every surface the port collides with has one.
    let w = (F(l.margin) + if !l.doubles || serve { SINGLES_SIDELINE } else { DOUBLES_SIDELINE }).0;
    let len = (F(l.margin) + if serve { SERVICE_LINE } else { BASELINE }).0;
    let (x, z) = (at[0], at[2]);
    let mut inside = false;
    if x.abs() <= w {
        inside = if !l.hitter_far { 0.0 < z && z <= len } else { z < 0.0 && !(z < -F(len)) };
        if serve && inside {
            inside = if l.side == (!l.hitter_far) as i32 { -F(CENTRE_SLACK) - l.margin <= x } else { x <= F(l.margin) + CENTRE_SLACK };
        }
    }
    let after_net = call == Call::Net;
    let mut new = match (inside, after_net) {
        (true, false) => Call::In,
        (true, true) => Call::NetIn,
        (false, false) => Call::Out,
        (false, true) => Call::NetOut,
    };
    if !serve && new == Call::NetOut && !(x.abs() <= w && z.abs() <= len) {
        new = Call::Out;
    }
    Some((new, Some(line_distance(at, l))))
}

/// Signed distance from the landing to the nearest line that matters (negative inside).
fn line_distance(at: [f32; 4], l: &Lines) -> f32 {
    let (x, z) = (at[0].abs(), at[2].abs());
    let nearer = |a: f32, b: f32| if b.abs() <= a.abs() { b } else { a };
    if l.shots < 2 {
        // centre service line, sideline, service line
        let side = (F(x) - SINGLES_SIDELINE).0;
        nearer(if x <= side.abs() { x } else { side }, (F(z) - SERVICE_LINE).0)
    } else {
        let w = if l.doubles { DOUBLES_SIDELINE } else { SINGLES_SIDELINE };
        nearer((F(x) - w).0, (F(z) - BASELINE).0)
    }
}

/// The ball as the point-over check sees it.
#[derive(Clone, Copy, Debug)]
pub struct BallState {
    pub call: Call,
    /// Court contacts.
    pub contacts: i32,
    /// Came to rest.
    pub stopped: bool,
    pub pos: [f32; 3],
}

/// The rally state the umpire judges from (the original keeps it in one block, snapshotted when the point
/// ends). Fields at the end persist across points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Rally {
    /// Ball call, shots, last hitter and server when the point ended.
    pub call: Call,
    pub shots: i32,
    pub hitter: i32,
    pub server: i32,
    /// An illegal hit lost the point for the hitter.
    pub illegal_hit: bool,
    /// Faults on this point (1 = a second serve is pending, 2 = double fault).
    pub faults: i32,
    pub let_: bool,
    /// The serve's call while it was the only shot.
    pub serve_call: Call,
    /// Shots at the last hit check.
    pub checked_shots: i32,
    /// The return was struck after the serve bounced.
    pub return_bounced: bool,
    /// A player was hit by the ball and the serve decided it (fault or let).
    pub hit_on_serve: bool,
    /// Pending flags raised by the hit check, consumed by the next point-over check.
    pub lose: bool,
    pub serve_bounced: bool,
}

impl Default for Rally {
    fn default() -> Self {
        Rally {
            call: Call::None,
            shots: 0,
            hitter: -1,
            server: -1,
            illegal_hit: false,
            faults: 0,
            let_: false,
            serve_call: Call::None,
            checked_shots: 0,
            return_bounced: false,
            hit_on_serve: false,
            lose: false,
            serve_bounced: false,
        }
    }
}

/// The calls a verdict shows (`Verdict::call`).
pub mod verdict {
    /// A plain point.
    pub const POINT: u8 = 0;
    pub const OUT: u8 = 1;
    pub const FAULT: u8 = 2;
    pub const DOUBLE_FAULT: u8 = 3;
    pub const LET: u8 = 4;
    /// Out after touching the net.
    pub const NET_OUT: u8 = 5;
    pub const ILLEGAL_HIT: u8 = 6;
}

/// The umpire's verdict on a finished point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verdict {
    /// The call shown (`verdict`).
    pub call: u8,
    /// Winning team, `None` when no point is played out (fault or let: the serve is taken again).
    pub winner: Option<usize>,
}

impl Rally {
    /// A new point: the snapshot is cleared, the counters and pending flags stay.
    pub fn new_point(&mut self) {
        self.call = Call::None;
        self.shots = 0;
        self.hitter = -1;
        self.server = -1;
    }

    /// The point after a let or a first-serve fault is the same point again; otherwise faults start over.
    pub fn next_point(&mut self) {
        if !(self.faults == 1 || self.let_) {
            self.faults = 0;
        }
        self.let_ = false;
        self.hit_on_serve = false;
        self.serve_call = Call::None;
    }

    /// A player struck the ball. `shots` already counts this hit; `contacts` = the ball's court contacts
    /// before it was struck.
    pub fn on_hit(&mut self, shots: i32, hitter: i32, server: i32, receiver: i32, contacts: i32) {
        if shots == 2 && contacts > 0 {
            self.return_bounced = true;
        }
        let lose = if shots == 1 && server != hitter {
            true
        } else if shots == 1 && contacts > 0 {
            self.serve_bounced = true;
            false
        } else if shots == 2 && (!self.return_bounced || hitter != receiver || self.serve_call == Call::Net) {
            true
        } else {
            let twice = shots > 0 && self.checked_shots != shots && same_team(hitter, self.hitter);
            self.checked_shots = shots;
            twice
        };
        if lose {
            self.lose = true;
        }
        self.hitter = hitter;
    }

    /// Once per rally frame: is the point over? `body` = the player the ball hit, if any. On `true` the
    /// state is snapshotted for `judge`.
    pub fn check(&mut self, b: &BallState, shots: i32, hitter: i32, server: i32, body: Option<i32>, arena_limits: bool) -> bool {
        self.illegal_hit = false;
        let over = if self.lose {
            self.illegal_hit = true;
            true
        } else if self.serve_bounced {
            self.faults += 1;
            true
        } else if let Some(p) = body {
            if shots == 1 {
                if same_team(server, p) {
                    self.hit_on_serve = true;
                    self.faults += 1;
                } else if b.stopped {
                    self.hit_on_serve = true;
                    self.let_ = true;
                }
            }
            true
        } else {
            match b.call {
                Call::In | Call::NetIn if b.call == Call::In || shots > 1 => {
                    shots != 0 && (b.contacts >= 2 || outside(b.pos, arena_limits) || b.stopped)
                }
                Call::NetIn if shots == 1 => {
                    self.let_ = true;
                    true
                }
                Call::Out | Call::NetOut => {
                    if shots < 2 {
                        self.faults += 1;
                    }
                    true
                }
                _ => false,
            }
        };
        if shots == 1 {
            self.serve_call = b.call;
        }
        if over {
            self.call = b.call;
            self.shots = shots;
            self.hitter = hitter;
            self.server = server;
            self.lose = false;
            self.serve_bounced = false;
            self.return_bounced = false;
        }
        over
    }

    /// The umpire's verdict on the snapshot. `body` as in `check`.
    pub fn judge(&self, body: Option<i32>) -> Verdict {
        let (h, v) = (self.hitter, self.server);
        let other = |p: i32| (p & 1 ^ 1) as usize;
        use verdict::*;
        let fault = |faults: i32| if faults == 1 { Verdict { call: FAULT, winner: None } } else { Verdict { call: DOUBLE_FAULT, winner: Some(other(v)) } };
        match body {
            None if self.illegal_hit => Verdict { call: ILLEGAL_HIT, winner: Some(other(h)) },
            None => match (self.call, self.shots) {
                (Call::NetIn, 1) => Verdict { call: LET, winner: None },
                (Call::NetOut, 1) => fault(self.faults),
                (Call::NetOut, _) => Verdict { call: NET_OUT, winner: Some(other(h)) },
                (Call::Out, s) if s < 2 => fault(self.faults),
                (Call::Out, _) => Verdict { call: OUT, winner: Some(other(h)) },
                _ => Verdict { call: POINT, winner: Some((h & 1) as usize) },
            },
            Some(p) if !self.hit_on_serve => Verdict { call: POINT, winner: Some(other(p)) },
            Some(p) if same_team(v, p) => fault(self.faults),
            Some(_) => Verdict { call: LET, winner: None },
        }
    }
}

/// Outside the arena (only where the court has arena limits).
fn outside(pos: [f32; 3], arena_limits: bool) -> bool {
    arena_limits && !(pos[0].abs() <= ARENA.0 && pos[2].abs() <= ARENA.1)
}
