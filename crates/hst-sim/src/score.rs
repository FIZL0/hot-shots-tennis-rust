//! Match score, server rotation and ends, ported from the original's scoreboard logic.
//! Teams: 0 = players 0 and 2, 1 = players 1 and 3. Point index 0..4 = 0, 15, 30, 40, advantage.

/// Cap on the deuce counter (game data value; only shown by the scoreboard).
const DEUCE_COUNT_CAP: i32 = 99;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    /// Sets needed to win the match.
    pub sets: i32,
    /// Games needed to win a set.
    pub games: i32,
    /// Deuce off: 40-40 is sudden death, sets end at `games` without a 2-game lead, tiebreak to 7 flat.
    pub no_deuce: bool,
    /// Every point wins a game.
    pub one_point_games: bool,
    /// Players on court (2 singles, 4 doubles).
    pub players: i32,
}

/// What a point did to the score (the scoreboard's event code).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Point = 1,
    Game = 3,
    Set = 4,
    TiebreakPoint = 6,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Score {
    pub points: [i32; 2],
    pub games: [i32; 2],
    pub sets: [i32; 2],
    /// 40-40 (or 7-7 in a tiebreak) and waiting for advantage.
    pub deuce: bool,
    pub advantage: bool,
    pub deuce_count: i32,
    pub tiebreak: bool,
    /// Tiebreak points played + 1; the server changes when it turns even, ends every 6 points.
    pub tiebreak_count: i32,
    /// Server rotation counter; the server is `rotation % players`.
    pub rotation: i32,
    /// Teams have changed ends.
    pub swapped: bool,
    /// Set by a finished game, consumed by `next_point`.
    pub game_changed: bool,
    /// Games played in this set (ends change when odd).
    pub games_played: i32,
    pub match_over: bool,
    /// Rotation and ends when the current tiebreak began (to pick the next set's server).
    pub tiebreak_start: Option<(i32, bool)>,
    pub server: i32,
    /// 0 = deuce court, 1 = ad court.
    pub side: i32,
    pub receiver: i32,
    /// The pending serve is a second serve (fault on the first): no rotation.
    pub second_serve: bool,
    /// The point was a let: replay it, no rotation.
    pub let_: bool,
    /// The set being played, 0-based (counts on past the last set won; the scoreboard's result board reads it).
    pub set: i32,
    /// Each team's games in each set, as the result board shows them (a tiebreak set as 7-6).
    pub set_games: [[i32; 5]; 2],
}

impl Score {
    /// A fresh match: first server player 0, receiver player 1.
    pub fn new() -> Self {
        Score { receiver: 1, ..Score::default() }
    }

    /// `winner` takes the point. Returns the scoreboard event, or `None` when the match is already decided.
    pub fn point(&mut self, r: &Rules, winner: usize) -> Option<Event> {
        let loser = winner ^ 1;
        if self.tiebreak {
            self.tiebreak_count += 1;
            if self.tiebreak_count % 2 == 0 {
                self.game_changed = true;
                self.rotation += 1;
            }
            self.points[winner] += 1;
            let p = self.points[winner];
            let more = if r.no_deuce { p < 7 } else { p < 7 || (p - self.points[loser]).abs() < 2 };
            if more && !self.deuce && !self.advantage {
                if p == self.points[loser] && p == 7 {
                    self.deuce = true;
                    self.deuce_count = (self.deuce_count + 1).min(DEUCE_COUNT_CAP);
                }
                return Some(Event::TiebreakPoint);
            }
            if self.deuce {
                self.deuce = false;
                self.advantage = true;
                return Some(Event::TiebreakPoint);
            }
            if self.advantage {
                self.advantage = false;
                if self.points[winner] == self.points[loser] {
                    self.points[winner] -= 1;
                    self.points[loser] -= 1;
                    self.deuce = true;
                    self.deuce_count = (self.deuce_count + 1).min(DEUCE_COUNT_CAP);
                    return Some(Event::TiebreakPoint);
                }
            }
            // Tiebreak won: the next set starts one rotation after the tiebreak's first server.
            let (rot, swapped) = self.tiebreak_start.unwrap_or((-1, false));
            self.rotation = rot + 1;
            self.swapped = swapped;
            self.games_played += 1;
            self.game_changed = true;
            if self.sets[winner] < r.sets {
                self.tiebreak = false;
                let e = self.win_set(r, winner);
                self.set_games[winner][(self.set - 1).min(4) as usize] += 1;
                return Some(e);
            }
        }
        if !r.one_point_games {
            let p = self.points[winner];
            if p < 3 {
                self.points[winner] += 1;
                if !r.no_deuce && self.points[winner] == self.points[loser] && self.points[winner] == 3 {
                    self.deuce = true;
                    self.deuce_count = (self.deuce_count + 1).min(DEUCE_COUNT_CAP);
                }
                return Some(Event::Point);
            }
            if self.deuce {
                self.points[winner] += 1;
                self.deuce = false;
                self.advantage = true;
                return Some(Event::Point);
            }
            if self.advantage {
                self.points[winner] += 1;
                self.advantage = false;
                if self.points[winner] == self.points[loser] {
                    self.points[winner] -= 1;
                    self.points[loser] -= 1;
                    self.deuce = true;
                    self.deuce_count = (self.deuce_count + 1).min(DEUCE_COUNT_CAP);
                    return Some(Event::Point);
                }
                self.points[winner] -= 1;
            }
        }
        // Game.
        self.points[winner] += 1;
        self.game_changed = true;
        self.rotation += 1;
        self.games_played += 1;
        self.deuce_count = 0;
        self.games[winner] += 1;
        self.set_games[winner][self.set.min(4) as usize] = self.games[winner];
        let (g, o) = (self.games[winner], self.games[loser]);
        if g < r.games {
            return Some(Event::Game);
        }
        if g - o < 2 && !self.tiebreak && !r.no_deuce {
            if g == o {
                self.tiebreak = true;
                self.side = 1;
                self.tiebreak_count = 1;
            }
            return Some(Event::Game);
        }
        if r.sets <= self.sets[winner] {
            return None;
        }
        self.tiebreak = false;
        Some(self.win_set(r, winner))
    }

    fn win_set(&mut self, r: &Rules, winner: usize) -> Event {
        self.sets[winner] += 1;
        self.set = (self.set + 1).min(5);
        self.deuce_count = 0;
        if self.sets[winner] == r.sets {
            self.match_over = true;
        }
        Event::Set
    }

    /// The scoreboard notes where a tiebreak began (it does this while the tiebreak is on).
    pub fn note_tiebreak_start(&mut self) {
        if !self.tiebreak {
            self.tiebreak_start = None;
        } else if self.tiebreak_start.is_none() {
            self.tiebreak_start = Some((self.rotation, self.swapped));
        }
    }

    /// Clear the points for a new game (the scoreboard does this when its game animation starts).
    pub fn new_game(&mut self) {
        self.points = [0; 2];
    }

    /// Clear points and games for a new set.
    pub fn new_set(&mut self) {
        self.points = [0; 2];
        self.games = [0; 2];
    }

    /// Next server, receiver and court side after a point (no change after a let or a first-serve fault).
    pub fn next_point(&mut self, r: &Rules) {
        if self.second_serve || self.let_ {
            self.let_ = false;
            return;
        }
        if self.game_changed {
            self.server = self.rotation % r.players;
            self.game_changed = false;
            self.side = if !self.tiebreak { 0 } else { (self.side == 0) as i32 };
            let first_team = self.server == 0 || self.server == 2;
            self.receiver = match (self.side, first_team) {
                (0, true) => 1,
                (0, false) => 0,
                (_, true) => 3,
                (_, false) => 2,
            } % r.players;
        } else {
            self.side ^= 1;
            if r.players > 2 {
                self.receiver ^= 2;
            }
        }
    }

    /// Change ends after odd games (every 6 tiebreak points). Call before `next_point`, which consumes
    /// `game_changed`. Returns whether the teams are swapped.
    pub fn change_ends(&mut self) -> bool {
        if !(self.second_serve || self.let_) {
            if !self.tiebreak {
                if self.game_changed && self.games_played % 2 != 0 {
                    self.swapped = !self.swapped;
                }
            } else if (self.tiebreak_count - 1) % 6 == 0 && self.tiebreak_count != 1 {
                self.swapped = !self.swapped;
            }
        }
        self.swapped
    }

    /// Is this team one point from the game (the umpire's game/set point check)?
    pub fn game_point(&self, r: &Rules, team: usize) -> bool {
        if self.deuce {
            return false;
        }
        let p = self.points[team];
        if !self.tiebreak {
            p > if self.advantage { 3 } else { 2 }
        } else if self.advantage {
            p == 8
        } else if r.no_deuce {
            p == 6
        } else {
            p == 6 && self.points[team ^ 1] < 6 || p == 7 && self.points[team ^ 1] < 7
        }
    }
}
