//! The match statistics the original keeps over a match and shows on the stats screen at its end: per player the
//! fastest serve, points won, missed shots, double faults, aces, sweet-spot rate, net play rate, forehand rate and
//! (per team) the match rating; over the match the longest rally and the match time.
//!
//! When each is counted:
//! - every frame of play: the match time (capped at 99:59:59);
//! - each serve: the exchange counter is cleared; the serve's speed is read the frame after it is struck (× 0.9);
//! - each hit: hits, serves struck (shot 1), dives, smashes, forehand ground strokes and volleys, sweet-spot hits
//!   (the sweet balloon), net play (shots from the third on, and of those the ones struck within 6.4 m of the
//!   net); a hit by the serving team after the other team's counts one exchange of the rally;
//! - each point played out: missed shots (out, out after the net) to the hitter and the point to the other team's
//!   last hitter, an ace to the server (the serve alone won it), else the point to its hitter; then the fastest
//!   serve (not on a fault or let), double faults, the longest rally and the rates;
//! - each game won: each team's share of the game's points, scaled so the leader's is 100, summed for the rating.

/// One player's counts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Player {
    pub fastest_kmh: f32,
    pub points_won: u32,
    pub outs: u32,
    pub net_outs: u32,
    pub double_faults: u32,
    pub aces: u32,
    pub hits: u32,
    pub serves_struck: u32,
    pub dives: u32,
    pub smashes: u32,
    pub sweet: u32,
    pub forehands: u32,
    /// Shots from the third on, and of those the ones struck from within 6.4 m of the net.
    pub late: u32,
    pub net: u32,
}

impl Player {
    pub fn missed(&self) -> u32 {
        self.outs + self.net_outs
    }
}

fn rate(n: u32, of: u32) -> f32 {
    if of == 0 { 0.0 } else { n as f32 / of as f32 * 100.0 }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub players: [Player; 4],
    pub longest_rally: u32,
    /// Frames of play.
    pub frames: u32,
    pub games: u32,
    /// Each team's game shares, summed.
    pub share: [i32; 2],
    rally: u32,
    serve_kmh: f32,
    /// Last and previous hitter (−1 none).
    hitter: i32,
    prev: i32,
    /// The finished match's table, set at its end (`finish`).
    pub result: Option<Table>,
}

/// The screen's values: per player the 9 rows (fastest serve in mph, points won, missed shots, double faults,
/// aces, sweet-spot %, net play %, forehand %, rating; the rating on players 0 and 1 only, by team).
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub rows: [[f32; 9]; 4],
    pub longest_rally: u32,
    pub frames: u32,
    pub winner: usize,
}

impl Stats {
    pub fn new() -> Self {
        Stats { hitter: -1, prev: -1, ..Default::default() }
    }

    /// A frame of play.
    pub fn tick(&mut self) {
        self.frames = (self.frames + 1).min(21599940);
    }

    pub fn serve(&mut self) {
        self.rally = 0;
        self.serve_kmh = 0.0;
        (self.hitter, self.prev) = (-1, -1);
    }

    /// The ball's speed (km/h) the frame after the serve was struck.
    pub fn serve_speed(&mut self, kmh: f32) {
        if self.serve_kmh <= 0.0 {
            self.serve_kmh = kmh * 0.9;
        }
    }

    /// Shot `shot` (1 = the serve) struck by `who` on contact branch `branch` (0 serve, 1 ground, 2 volley,
    /// 3 dive, 4 smash), `net_z` its distance from the net.
    pub fn hit(&mut self, who: usize, shot: i32, server: usize, branch: u8, forehand: bool, sweet: bool, net_z: f32) {
        let p = &mut self.players[who];
        p.hits += 1;
        p.serves_struck += (shot == 1 && who == server) as u32;
        p.dives += (branch == 3) as u32;
        p.smashes += (branch == 4) as u32;
        p.forehands += ((branch == 1 || branch == 2) && forehand) as u32;
        p.sweet += sweet as u32;
        if shot > 2 {
            p.late += 1;
            p.net += (net_z.abs() <= 6.4) as u32;
        }
        if shot > 1 && self.hitter >= 0 && self.hitter & 1 != server as i32 & 1 {
            self.rally += 1;
        }
        (self.prev, self.hitter) = (self.hitter, who as i32);
    }

    /// A point's verdict (`judge::Verdict::call`) with the rally's shot count and faults: the counts the
    /// scoreboard keeps, then the point's end.
    pub fn point(&mut self, call: u8, winner: Option<usize>, shots: i32, server: usize, faults: i32) {
        let (hitter, other) = (self.hitter.max(0) as usize, self.prev.max(0) as usize);
        match call {
            1 | 5 => {
                let p = &mut self.players[hitter];
                *(if call == 1 { &mut p.outs } else { &mut p.net_outs }) += 1;
                // ponytail: the original credits its own "other" hitter; the opposing team's last hitter stands in
                let to = if other & 1 != hitter & 1 { other } else { hitter ^ 1 };
                self.players[to].points_won += 1;
            }
            0 if shots == 1 && winner == Some(server & 1) => {
                self.players[server].aces += 1;
                self.players[server].points_won += 1;
            }
            0 => self.players[hitter].points_won += 1,
            6 => self.players[hitter ^ 1].points_won += 1,
            _ => {}
        }
        if call == 4 {
            return;
        }
        if !matches!(call, 2 | 3) {
            let p = &mut self.players[server];
            p.fastest_kmh = p.fastest_kmh.max(self.serve_kmh);
        }
        if faults == 2 {
            self.players[server].double_faults += 1;
        }
        self.longest_rally = self.longest_rally.max(self.rally).min(999);
        self.rally = 0;
    }

    /// A game (or a tiebreak) won with the final `points`.
    pub fn game(&mut self, points: [i32; 2]) {
        let top = points[0].max(points[1]) as f32;
        self.games += 1;
        for t in 0..2 {
            self.share[t] += (100.0 / top * points[t] as f32 + 0.5) as i32;
        }
    }

    /// The match's end, `winner` the winning team: the screen's table.
    pub fn finish(&self, winner: usize) -> Table {
        let g = self.games as f32;
        let avg = self.share.map(|s| (s as f32 / g + 0.5) as i32);
        let mut rating = [0; 2];
        rating[0] = (avg[0] as f32 / (avg[0] + avg[1]) as f32 * 100.0 + 0.5) as i32;
        if rating[0] == 50 {
            rating[0] = if winner == 0 { 51 } else { 49 };
        }
        rating[1] = 100 - rating[0];
        if rating[winner] < 50 {
            (rating[winner], rating[winner ^ 1]) = (51, 49);
        }
        let rows = std::array::from_fn(|i| {
            let p = &self.players[i];
            let strokes = p.hits.saturating_sub(p.serves_struck + p.dives + p.smashes);
            [
                p.fastest_kmh.clamp(0.0, 999.0) as i32 as f32 * 0.62137,
                p.points_won.min(999) as f32,
                p.missed().min(999) as f32,
                p.double_faults.min(999) as f32,
                p.aces.min(999) as f32,
                rate(p.sweet, p.hits),
                rate(p.net, p.late),
                rate(p.forehands, strokes),
                if i < 2 { rating[i].clamp(0, 100) as f32 } else { 0.0 },
            ]
        });
        Table { rows, longest_rally: self.longest_rally, frames: self.frames, winner }
    }
}

/// Frames as hours, minutes, seconds.
pub fn hms(frames: u32) -> [u32; 3] {
    let s = frames / 60;
    [s / 3600, s % 3600 / 60, s % 60]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A short singles match: player 0 serves, a 5-shot rally player 0 hits out, an ace, a double fault.
    #[test]
    fn counts() {
        let mut s = Stats::new();
        s.serve();
        s.hit(0, 1, 0, 0, false, true, 11.9);
        s.serve_speed(150.0);
        for (k, who) in [1, 0, 1, 0].into_iter().enumerate() {
            s.hit(who, k as i32 + 2, 0, 1, who == 0, false, if who == 0 { 5.0 } else { 10.0 });
        }
        s.point(1, Some(1), 5, 0, 0);
        assert_eq!(s.players[0].outs, 1);
        assert_eq!(s.players[1].points_won, 1);
        assert_eq!(s.longest_rally, 2);
        s.serve();
        s.hit(0, 1, 0, 0, false, false, 11.9);
        s.serve_speed(180.0);
        s.point(0, Some(0), 1, 0, 0);
        assert_eq!((s.players[0].aces, s.players[0].points_won), (1, 1));
        assert_eq!(s.players[0].fastest_kmh, 162.0);
        s.serve();
        s.serve_speed(200.0);
        s.point(3, Some(1), 1, 0, 2);
        assert_eq!(s.players[0].double_faults, 1);
        assert_eq!(s.players[0].fastest_kmh, 162.0);
        s.game([4, 2]);
        s.game([1, 4]);
        let t = s.finish(0);
        assert_eq!(t.rows[0][0], 162.0 * 0.62137);
        assert_eq!(t.rows[0][5], 25.0);
        assert_eq!(t.rows[0][6], 100.0);
        assert_eq!(t.rows[0][7], 100.0);
        assert_eq!(t.rows[0][2], 1.0);
        // shares 100+25 and 50+100 over 2 games: 63 and 75, 46 to 54, and the winner never rates below 51
        assert_eq!((t.rows[0][8], t.rows[1][8]), (51.0, 49.0));
        assert_eq!(hms(60 * 3725), [1, 2, 5]);
    }
}
