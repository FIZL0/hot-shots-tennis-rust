//! From a decided point to the next serve, ported from the original's match phases and scoreboard.
//!
//! The match runs in phases: serve, rally, point over, change ends, match over. When a point is decided it
//! waits on the scoreboard: a pause, then (once the players are told to react) a wait, then the score show
//! (fade in, roll, hold, fade out). When the show is over the ends may change, and the next frame leaves for
//! the change-ends phase (a fixed camera cut) or straight to the next serve. Counted in the match's own ticks:
//! the game sometimes spends several vsyncs on one tick while it loads, and those don't count.

use crate::judge::Rally;
use crate::score::{Event, Score};
use hst_data::exe::ScoreboardTiming;

/// Ticks of the change-ends phase (the camera's change-ends cut), before the next serve is set up.
pub const CHANGE_ENDS: u32 = 80;
/// Ticks the scoreboard pauses after a decided point before it shows anything.
const PAUSE: i32 = 30;

/// Where the match goes when the point-over phase ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    Serve,
    ChangeEnds,
    MatchOver,
}

/// The point-over phase: the scoreboard's pause, wait and score show, and the match deciding what's next.
#[derive(Clone, Debug)]
pub struct PostPoint {
    /// Ticks run in this phase.
    pub tick: u32,
    pub event: Event,
    board: Board,
    /// The scoreboard has paused; the players have been told to react to the point.
    pub reacted: bool,
}

#[derive(Clone, Copy, Debug)]
enum Board {
    Pause(i32),
    Wait(i32),
    Show(Show),
    Done,
}

/// One score show. `step` counts the show's own stages; `held` the frames since the new score settled.
#[derive(Clone, Copy, Debug)]
struct Show {
    event: Event,
    step: u8,
    t: i32,
    n: i32,
    fading_out: bool,
    settled: bool,
    held: i32,
}

impl PostPoint {
    /// The point that produced `event` was just scored.
    pub fn new(event: Event) -> Self {
        PostPoint { tick: 0, event, board: Board::Pause(0), reacted: false }
    }

    /// The scoreboard's pause is over (the umpire calls the score then, `Umpire::call_score`).
    pub fn paused(&self) -> bool {
        matches!(self.board, Board::Pause(_))
    }

    /// The score show has started (a game or set: the umpire announces it then, `Umpire::announce`).
    pub fn showing(&self) -> bool {
        matches!(self.board, Board::Show(_) | Board::Done)
    }

    /// One tick: the match checks the scoreboard, then the scoreboard runs. Returns where the match goes,
    /// on the tick it decides (the phase changes on the next one). After `Serve` or `ChangeEnds` the caller
    /// moves to the next point (`Score::next_point`, `Rally::next_point`).
    pub fn step(&mut self, score: &mut Score, rally: &mut Rally, t: &ScoreboardTiming) -> Option<Next> {
        let mut next = None;
        if !self.reacted && !matches!(self.board, Board::Pause(_)) {
            // ponytail: player reactions (P12a); the match-over branch waits on the umpire's voice line (P0b4c)
            self.reacted = true;
        }
        if self.reacted && score.match_over {
            next = Some(Next::MatchOver);
        } else if matches!(self.board, Board::Done) {
            score.second_serve = rally.faults == 1;
            score.let_ = rally.let_;
            let swapped = score.swapped;
            score.change_ends();
            if self.event == Event::Set {
                score.games_played = 0;
            }
            next = Some(if swapped == score.swapped || self.event == Event::Set { Next::Serve } else { Next::ChangeEnds });
        }
        self.board = match self.board {
            Board::Pause(n) if n + 1 > PAUSE => {
                // a scored point ends the serve's faults
                rally.faults = 0;
                Board::Wait(0)
            }
            Board::Pause(n) => Board::Pause(n + 1),
            Board::Wait(n) => {
                // ponytail: a player already mid-reaction would shorten the point wait (never seen: they're idle then)
                let wait = if matches!(self.event, Event::Game | Event::Set) { t.game_wait } else { t.point_wait };
                if n + 1 > wait {
                    match self.event {
                        Event::Game => score.new_game(),
                        Event::Set => score.new_set(),
                        _ => {}
                    }
                    let mut s = Show::new(self.event);
                    if s.step(score, t) { Board::Done } else { Board::Show(s) }
                } else {
                    Board::Wait(n + 1)
                }
            }
            Board::Show(mut s) => {
                if s.step(score, t) { Board::Done } else { Board::Show(s) }
            }
            Board::Done => Board::Done,
        };
        self.tick += 1;
        next
    }
}

/// What the score show is drawing this tick (after `PostPoint::step`), for the pop-ups: the show's stage
/// (0 fade in, 1 roll, 2 flash, 3 settle, 4 hold), its countdown `t` and roll step `n`, and whether it is fading out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowState {
    pub event: Event,
    pub stage: u8,
    pub t: i32,
    pub n: i32,
    pub fading_out: bool,
}

impl PostPoint {
    /// The running score show, if any.
    pub fn show(&self) -> Option<ShowState> {
        match self.board {
            Board::Show(s) => Some(ShowState { event: s.event, stage: s.step, t: s.t, n: s.n, fading_out: s.fading_out }),
            _ => None,
        }
    }
}

impl Show {
    fn new(event: Event) -> Self {
        let t = if matches!(event, Event::Game | Event::Set) { 14 } else { 5 };
        Show { event, step: 0, t, n: 0, fading_out: false, settled: false, held: 0 }
    }

    /// One frame of the show; true when it is over.
    fn step(&mut self, score: &mut Score, tm: &ScoreboardTiming) -> bool {
        if self.fading_out {
            self.t -= 1;
            if self.t < 0 {
                if self.event == Event::Game && score.tiebreak {
                    score.swapped = false;
                }
                return true;
            }
        } else {
            let deuce = score.deuce;
            match (self.event, self.step) {
                (_, 0) => {
                    self.t -= 1;
                    if self.t < 0 {
                        self.step = 1;
                        self.n = 0;
                    }
                }
                (Event::Game | Event::Set, 1) => {
                    self.n += 1;
                    if self.n > tm.game_rise {
                        self.step = 2;
                        self.n = tm.game_drop;
                    }
                }
                (Event::Game | Event::Set, 2) => {
                    self.n -= 1;
                    if self.n < 0 {
                        self.step = 4;
                    }
                }
                (Event::Point, 1) => {
                    self.n += 1;
                    if self.n > if deuce { 2 } else { 4 } {
                        self.step = 2;
                        self.n = if deuce { 3 } else { 0 };
                    }
                }
                (Event::TiebreakPoint, 1) => {
                    self.n += 1;
                    if !deuce && self.n > 7 {
                        (self.step, self.t) = (2, 3);
                    } else if deuce && self.n >= 3 {
                        (self.step, self.n) = (2, 3);
                    }
                }
                (_, 2) => {
                    let to_settle = if deuce {
                        self.n -= 1;
                        self.n < 1
                    } else if self.event == Event::Point {
                        self.n += 1;
                        self.n > 4
                    } else {
                        self.t -= 1;
                        self.t < 0
                    };
                    if to_settle {
                        (self.step, self.t) = (3, 15);
                    }
                }
                (_, 3) => {
                    self.t -= 1;
                    if self.t < 0 {
                        self.step = 4;
                    }
                }
                _ => {}
            }
            self.settled |= self.step == 4;
        }
        if self.settled {
            self.held += 1;
            let hold = if matches!(self.event, Event::Point | Event::TiebreakPoint) { tm.point_hold } else { tm.game_hold };
            if hold * 60.0 <= self.held as f32 && !self.fading_out {
                (self.fading_out, self.t) = (true, 5);
            }
        }
        false
    }
}

/// Baseline depth of the server and (first serve) the receiver.
const BASELINE: f32 = 12.25;
/// Where a player is put when the next serve is set up (entering the serve or change-ends phase), in game space.
/// `facing` is +1 for players looking toward +z (even players at the start, before ends change), else -1; the
/// player stands on the −`facing` half.
///
/// - The server stands on the baseline at `stance` from the centre line on the side's court: `stance` is how far
///   off centre this player was at their last toss (3.0 for the match's first point).
/// - The receiver waits at 3.0 off centre on the baseline, 1.25 m inside it for a second serve.
/// - Everyone else (doubles partners — and the singles receiver on the ad side, whose receiver index names an
///   absent player) stands 2.7425 off centre on the other half: the server's partner 3.2 m from the net, the
///   receiver's 5.2 m, both 7.9 m when their formation is 2 (back).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub pos: [f32; 3],
    pub facing: f32,
}

/// `player` 0..3 (even = first team), `faults` this point's faults so far, `formation` the player's doubles
/// formation (2 = both back), `swapped` the teams' ends have changed.
pub fn serve_placement(player: i32, s: &Score, faults: i32, stance: f32, formation: u8, swapped: bool) -> Placement {
    let facing = if (player & 1 == 0) != swapped { 1.0 } else { -1.0 };
    let court: f32 = if s.side == 0 { 1.0 } else { -1.0 };
    let deep = if formation == 2 { -7.9 } else { 0.0 };
    let (x, z) = if player == s.server {
        (court * stance * facing, facing * -BASELINE)
    } else if s.server & 1 == player & 1 {
        (facing * -2.7425 * court, facing * if deep != 0.0 { deep } else { -3.2 })
    } else if player == s.receiver {
        (facing * 3.0 * court, facing * if faults == 0 { -BASELINE } else { -11.0 })
    } else {
        (facing * -2.7425 * court, facing * if deep != 0.0 { deep } else { -5.2 })
    };
    Placement { pos: [x, 0.0, z], facing }
}
