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
    event: Event,
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
