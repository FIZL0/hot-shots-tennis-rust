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
/// Ticks the umpire's call sprite holds once settled, and its fade-out.
const CALL_HOLD: i32 = 29;
const CALL_FADE: i32 = 5;

/// Where the match goes when the point-over phase ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    Serve,
    ChangeEnds,
    MatchOver,
}

/// The point-over phase: the umpire's call sprite (if any), the scoreboard's pause, wait and score show, and
/// the match deciding what's next.
#[derive(Clone, Debug)]
pub struct PostPoint {
    /// Ticks run in this phase.
    pub tick: u32,
    /// What the point did to the score; `None` for no point (a fault or let): only the call shows.
    pub event: Option<Event>,
    board: Board,
    /// Pause before the score show: 30, or 1 after a call sprite that chains into it.
    pause: i32,
    /// The scoreboard has paused; the players have been told to react to the point.
    pub reacted: bool,
    /// The new score is on the board (a point: from its roll, after the 6-frame fade-in; a game or set: the show's start).
    pub shown: bool,
    /// The score the board shows until `shown` (the caller keeps it: the score before the point).
    pub before: Option<Score>,
}

#[derive(Clone, Copy, Debug)]
enum Board {
    Call(CallShow),
    Pause(i32),
    Wait(i32),
    Show(Show),
    Done,
}

/// The umpire's call model: up once its animation has ended and its voice line ends (or its countdown runs out),
/// held, faded out.
#[derive(Clone, Copy, Debug)]
struct CallShow {
    /// The judge's call (1 out, 2 fault, 3 double fault, 4 let, 5 out after net).
    call: u8,
    /// Frames the model has played, and its animation's length (0: no animation).
    frames: i32,
    anim_end: f32,
    /// Out, out after net or double fault: the score show follows after a one-tick pause.
    chain: bool,
    countdown: i32,
    settled: bool,
    held: i32,
    /// Fade-out frames left, -1 before the fade.
    fade: i32,
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
    /// The point that produced `event` was just scored, without an umpire call.
    pub fn new(event: Event) -> Self {
        PostPoint { tick: 0, event: Some(event), board: Board::Pause(0), pause: PAUSE, reacted: false, shown: false, before: None }
    }

    /// The point ended on the judge's `call` (1 out, 2 fault, 3 double fault, 4 let, 5 out after net, 6
    /// illegal hit; 0 none): `event` what it did to the score, `None` for no point. Calls 1..5 show the umpire's
    /// call sprite first (its countdown from `t.call_wait`); out, out after net and double fault then chain into
    /// the score show, a fault or let ends the phase.
    pub fn called(event: Option<Event>, call: u8, t: &ScoreboardTiming) -> Self {
        let mut p = PostPoint { tick: 0, event, board: Board::Pause(0), pause: PAUSE, reacted: false, shown: false, before: None };
        if (1..=5).contains(&call) {
            let chain = matches!(call, 1 | 3 | 5);
            p.board = Board::Call(CallShow { call, frames: 0, anim_end: 0.0, chain, countdown: t.call_wait[call as usize], settled: false, held: 0, fade: -1 });
        }
        p
    }

    /// The scoreboard's pause is over (the umpire calls the score then, `Umpire::call_score`).
    pub fn paused(&self) -> bool {
        matches!(self.board, Board::Pause(_))
    }

    /// The score show has started (a game or set: the umpire announces it then, `Umpire::announce`).
    pub fn showing(&self) -> bool {
        matches!(self.board, Board::Show(_) | Board::Done)
    }

    /// The new score has settled on the board: a human's press ends the phase from here.
    pub fn settled(&self) -> bool {
        match self.board {
            Board::Show(s) => s.settled,
            Board::Done => true,
            _ => false,
        }
    }

    /// One tick without a voice line or a press (bots, or a call that runs on its countdown).
    pub fn step(&mut self, score: &mut Score, rally: &mut Rally, t: &ScoreboardTiming) -> Option<Next> {
        self.step_with(score, rally, t, false, false)
    }

    /// One tick: the match checks the scoreboard, then the scoreboard runs. Returns where the match goes,
    /// on the tick it decides (the phase changes on the next one). After `Serve` or `ChangeEnds` the caller
    /// moves to the next point (`Score::next_point`, `Rally::next_point`). `voice_idle`: the umpire's call
    /// line has ended (settles the call sprite). `press`: a human pressed a face button this tick (ends the
    /// phase once the score has settled).
    pub fn step_with(&mut self, score: &mut Score, rally: &mut Rally, t: &ScoreboardTiming, voice_idle: bool, press: bool) -> Option<Next> {
        let mut next = None;
        if !self.reacted && self.event.is_some() && !matches!(self.board, Board::Call(_) | Board::Pause(_)) {
            // ponytail: the match-over branch waits on the umpire's voice line (P0b4c)
            self.reacted = true;
        }
        if self.reacted && score.match_over {
            next = Some(Next::MatchOver);
        } else if matches!(self.board, Board::Done) || (press && self.event.is_some() && self.settled()) {
            score.second_serve = rally.faults == 1;
            score.let_ = rally.let_;
            let swapped = score.swapped;
            score.change_ends();
            if self.event == Some(Event::Set) {
                score.games_played = 0;
            }
            next = Some(if swapped == score.swapped || self.event == Some(Event::Set) { Next::Serve } else { Next::ChangeEnds });
        }
        self.board = match self.board {
            Board::Call(mut c) => {
                if c.fade >= 0 {
                    c.fade -= 1;
                } else {
                    if !c.settled {
                        c.countdown -= 1;
                        c.frames += 1;
                        c.settled = c.anim_end <= c.frames as f32 && (voice_idle || c.countdown < 1);
                    }
                    if c.settled {
                        c.held += 1;
                        if c.held > CALL_HOLD {
                            c.fade = CALL_FADE;
                        }
                    }
                }
                if c.fade >= 0 || c.held <= CALL_HOLD {
                    Board::Call(c)
                } else if c.chain {
                    self.pause = 1;
                    Board::Pause(0)
                } else {
                    Board::Done
                }
            }
            Board::Pause(n) if n + 1 > self.pause => {
                // a scored point ends the serve's faults
                rally.faults = 0;
                Board::Wait(0)
            }
            Board::Pause(n) => Board::Pause(n + 1),
            Board::Wait(n) => {
                // ponytail: a player already mid-reaction would shorten the point wait (never seen: they're idle then)
                let event = self.event.unwrap_or(Event::Point);
                let wait = if matches!(event, Event::Game | Event::Set) { t.game_wait } else { t.point_wait };
                if n + 1 > wait {
                    match event {
                        Event::Game => score.new_game(),
                        Event::Set => score.new_set(),
                        _ => {}
                    }
                    let mut s = Show::new(event);
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
        self.shown |= match self.board {
            Board::Show(s) => s.step >= 1 || matches!(s.event, Event::Game | Event::Set),
            Board::Done => true,
            _ => false,
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

/// What the umpire's call model is drawing this tick (after `PostPoint::step`): the call and the model's alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CallState {
    pub call: u8,
    pub alpha: f32,
}

impl PostPoint {
    /// The call model's animation is `frames` long (from its `.ANI`; set before the first step): it settles no
    /// earlier.
    pub fn set_call_anim(&mut self, frames: f32) {
        if let Board::Call(c) = &mut self.board {
            c.anim_end = frames;
        }
    }

    /// The call model showing, if any: alpha 1 until the fade, then 128·t/5 (in 128ths) down to 0.
    pub fn call(&self) -> Option<CallState> {
        match self.board {
            Board::Call(c) => Some(CallState { call: c.call, alpha: if c.fade < 0 { 1.0 } else { (128 * c.fade / CALL_FADE) as f32 / 128.0 } }),
            _ => None,
        }
    }

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
                        // the tiebreak's old points keep sliding up while the new ones flash in
                        self.n += 1;
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
