//! Where a doubles bot waits between its shots: the team's formation (one up at the net, one back, each covering
//! one half of the width), the spot it walks back to, and when it bothers to walk there.
//!
//! Each bot keeps a lane (which half it covers) and whether it's the front player. They're set at the start of a
//! point from who serves and receives, then re-picked from its own, its partner's or a shot's position whenever its
//! team hits, its partner shapes to hit, or every 30 frames while it waits. A new spot rolls the row's centre rate
//! once: on a pass the bot walks there, after a 60-frame wait if it fails, and stops once inside the centre radius.

use crate::ps2;

/// Half the doubles court's width split three ways: the middle third is lane 1 in the 3-lane split.
const WIDTH: f32 = 5.485;
/// A spot's x: the middle of one half of the doubles width.
const HALF: f32 = 2.7425;

/// The bot's place in the formation (lane 0: not placed yet).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Formation {
    /// 1: the half on the side sign's +x, 2: the other half.
    pub lane: u8,
    /// Plays at the net.
    pub front: bool,
    /// The spot it heads for between shots (x, z).
    pub spot: [f32; 2],
}

/// What a re-pick looks at.
#[derive(Clone, Copy, Debug)]
pub enum Cue {
    /// Back to the middle of its half at the service line's depth, lane and front kept.
    Middle,
    /// The bot itself hit last: it compares its depth with its partner's (at that position).
    Me([f32; 2]),
    /// Its partner or the opponents: the shot's (or the partner's) position.
    Other([f32; 2]),
}

/// The fixed inputs of a re-pick.
#[derive(Clone, Copy, Debug)]
pub struct Team {
    /// The side sign (+1 plays at −z).
    pub side: f32,
    /// The player's formation byte: 0 staggered, 1 attacking, 2 defensive.
    pub formation: u8,
    /// Two bots of opposite styles: 1 for the NET one beside a BASE partner, 0 for the BASE one, −1 otherwise.
    pub lean: i32,
}

impl Team {
    /// `lean` from both styles (1 NET, 2 BASE) when the formation is staggered and neither player is human.
    pub fn lean(formation: u8, both_bots: bool, mine: u8, partners: u8) -> i32 {
        match (formation == 0 && both_bots, mine, partners) {
            (true, 1, 2) => 1,
            (true, 2, 1) => 0,
            _ => -1,
        }
    }

    /// A doubles player's formation byte as the match's first point places it (players 0 and 1; 2 and 3 take
    /// their partner's). `humans`: (me, my partner) on a pad; `rows`: (my, my partner's) AIParam formation;
    /// `setup`: the third byte of my match-setup word (`ai::Choice::strategy`); `draw`: my placement draw. Beside
    /// a human both take the computer player's row; two humans have no AI and keep 0; two computer players take
    /// the setup byte, staggered one time in five.
    pub fn pick(humans: (bool, bool), rows: (u8, u8), setup: u8, draw: u32) -> u8 {
        match humans {
            (true, true) => 0,
            (true, false) => rows.1,
            (false, true) => rows.0,
            (false, false) if (draw >> 16 & 0x7fff) % 100 < 20 => 0,
            (false, false) => setup,
        }
    }
}

/// x · sign of z (−z is the near side): which way across a point lies, seen from its own end.
fn across(p: [f32; 2]) -> f32 {
    let s = if p[1] < 0.0 { 1.0 } else { -1.0 };
    p[0] * s
}

/// The width in thirds: 1 the middle, 2 the +x third seen from its end, 0 the other.
fn third(p: [f32; 2]) -> u8 {
    if p[0].abs() <= ps2::div(WIDTH, 3.0) {
        1
    } else if across(p) > 0.0 {
        2
    } else {
        0
    }
}

impl Formation {
    /// At the start of a point. `serving`: the team serves. `starts`: this player serves or receives (it stays
    /// back). `ad`: the ad court. The spot drops the attacking formation's numbers as the game does (it reads the
    /// formation byte twice for 0), so 1 and 2 share the defensive spots here.
    pub fn start(t: &Team, starts: bool, ad: bool) -> Formation {
        let lane = if starts != ad { 1 } else { 2 };
        let front = !starts;
        let x = ps2::mul(ps2::mul(t.side, HALF), if lane == 1 { 1.0 } else { -1.0 });
        let z = match (t.formation, front) {
            (_, false) => -11.0,
            (0, true) => -3.2,
            (_, true) => -7.9,
        };
        Formation { lane, front, spot: [x, ps2::mul(t.side, z)] }
    }

    /// Re-pick the lane, front and spot. `me`: own position. `ball_z`: the ball's depth. `our_hit`: this team hit
    /// the ball last. Returns whether the spot moved (the caller then restarts the walk back: `Return::new`).
    pub fn repick(&mut self, t: &Team, cue: Cue, me: [f32; 2], ball_z: f32, our_hit: bool) -> bool {
        let old = self.spot;
        let (a, b) = match t.lean {
            0 => (0.75, -0.75),
            1 => (-0.75, 0.75),
            _ => (0.0, 0.0),
        };
        let (at, mine) = match cue {
            Cue::Middle => {
                self.spot = [0.0, ps2::mul(t.side, -6.4)];
                return self.spot != old;
            }
            Cue::Me(at) => (at, true),
            Cue::Other(at) => (at, false),
        };
        self.front = ps2::add(a, me[1].abs()) <= ps2::add(b, at[1].abs());
        // its own hit: at the net it covers the half it stands in, at the back it moves off a wide third; another
        // shot: at the back it takes the half the shot comes across, at the net it follows a wide third
        if self.front == mine {
            let p = if mine { me } else { at };
            self.lane = if (across(p) > 0.0) == mine { 1 } else { 2 };
        } else {
            match (third(if mine { me } else { at }), mine) {
                (0, true) | (2, false) => self.lane = 2,
                (2, true) | (0, false) => self.lane = 1,
                _ => {}
            }
        }
        let side = t.side;
        let x = ps2::mul(ps2::mul(side, HALF), if self.lane == 1 { 1.0 } else { -1.0 });
        let inside = mine || at[0].abs() <= 4.115 && at[1].abs() <= 11.885;
        // a shot from outside the court: hold the service-line depth, in the middle when it's well wide
        let wide = |front_z: f32| [if at[0].abs() > 5.985 { 0.0 } else { x }, ps2::mul(side, front_z)];
        // a net player stays closer in after its partner's shot unless its own team hit a deep ball on this side
        let net = |extra: f32| {
            let deep_here = our_hit && (ball_z < 0.0) == (side < 0.0) && ball_z.abs() > 3.2;
            let d = if mine || deep_here { 0.0 } else { extra };
            [x, ps2::mul(-ps2::add(d, 3.2), side)]
        };
        self.spot = match (t.formation, self.front) {
            (0, false) => {
                let d = match (t.lean > 0, mine) {
                    (false, _) => 0.0,
                    (true, false) => 1.0,
                    (true, true) => 2.0,
                };
                [x, ps2::mul(-ps2::sub(11.0, d), side)]
            }
            (0, true) if !inside => wide(-6.2),
            (0, true) => net(2.0),
            (1, false) => [x, ps2::mul(side, -4.9)],
            (1, true) if !inside => wide(-6.2),
            (1, true) => net(0.5),
            (_, false) => [x, ps2::mul(side, -11.0)],
            (_, true) if !inside => wide(-7.9),
            (_, true) => [x, ps2::mul(side, -7.9)],
        };
        self.spot != old
    }

    /// Before a partner's volley at the net: stay at its depth and cover the half the ball isn't in.
    pub fn hold(&mut self, me: [f32; 2], ball_x: f32) -> bool {
        let old = self.spot;
        self.spot = [if ball_x < 0.0 { HALF } else { -HALF }, me[1]];
        self.spot != old
    }
}

/// The walk back to the spot (or to the centre in singles).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Return {
    /// Frames waited since a failed roll.
    pub wait: i32,
    /// The roll passed: walk to the spot.
    pub going: bool,
    /// Inside the radius: stay put until the spot moves.
    pub there: bool,
}

/// The game's percentage roll: (MT word >> 16 & 0x7fff) % 100 < p.
fn chance(roll: &mut impl FnMut() -> u32, p: i32) -> bool {
    ((roll() >> 16 & 0x7fff) % 100) < p.max(0) as u32
}

impl Return {
    /// A new spot: roll the centre rate.
    pub fn new(rate: i32, roll: &mut impl FnMut() -> u32) -> Return {
        Return { wait: 0, going: chance(roll, rate), there: false }
    }

    /// One waiting frame: whether to walk toward `spot` now. A failed roll is re-rolled every 60 frames.
    pub fn step(&mut self, rate: i32, radius: f32, spot: [f32; 2], pos: [f32; 2], roll: &mut impl FnMut() -> u32) -> bool {
        if self.there {
            return false;
        }
        if !self.going {
            self.wait += 1;
            if self.wait >= 60 {
                self.wait = 0;
                self.going = chance(roll, rate);
            }
            return false;
        }
        let (dz, dx) = (ps2::sub(spot[1], pos[1]), ps2::sub(spot[0], pos[0]));
        if ps2::mul(radius, radius) < ps2::madd(ps2::mul(dx, dx), dz, dz) {
            return true;
        }
        self.there = true;
        false
    }
}

/// Half the singles court's width.
const SINGLES: f32 = 4.115;
/// How far across the dash spot may go: two thirds of the singles half-width.
const DASH_X: f32 = 2.743333;

/// Where a point lies on the singles court seen from its own end: its lane (1 the middle third, 2 the third on the
/// +x side seen from that end, 0 the other) and depth band (0 nearest the net, 2 deepest) past 1.5 from the net.
/// `blur`: the game's look at where the opponent stands, which rolls between bands near their edges.
pub fn zone(p: [f32; 2], blur: Option<&mut dyn FnMut() -> u32>) -> (u8, u8) {
    zone_in(p, SINGLES, blur)
}

/// [`zone`] on a court of half width `w` (the AI's: 4.115 singles, 5.485 doubles).
pub fn zone_in(p: [f32; 2], w: f32, blur: Option<&mut dyn FnMut() -> u32>) -> (u8, u8) {
    let (x, d) = (p[0].abs(), ps2::sub(p[1].abs(), 1.5));
    let third = ps2::div(w, 3.0);
    let wide = if across(p) > 0.0 { 2 } else { 0 };
    let Some(roll) = blur else {
        let lane = if x <= third { 1 } else { wide };
        let depth = if d <= 3.4616668 { 0 } else if d <= 6.9233336 { 1 } else { 2 };
        return (lane, depth);
    };
    // the MT word as a fraction of 2³²
    let mut roll = || ps2::mul(2.3283064e-10, ps2::utof(roll()));
    let q = ps2::div(ps2::div(ps2::mul(w, 2.0), 3.0), 4.0);
    let inner = ps2::sub(third, q);
    let lane = if x <= inner {
        1
    } else if ps2::add(third, q) <= x {
        wide
    } else {
        let t = ps2::div(ps2::sub(x, inner), ps2::mul(q, 2.0));
        if roll() < t { wide } else { 1 }
    };
    let band = |lo: f32, d: f32, roll: &mut dyn FnMut() -> f32| roll() < ps2::div(ps2::sub(d, lo), 1.7308334);
    let depth = if d <= 2.59625 {
        0
    } else if d < 4.3270836 {
        band(2.59625, d, &mut roll) as u8
    } else if d <= 6.057917 {
        1
    } else if d < 7.78875 {
        1 + band(6.057917, d, &mut roll) as u8
    } else {
        2
    };
    (lane, depth)
}

/// A singles bot between its shots: the centre spot it walks back to, the spot it dashes in to instead when it
/// means to take the net, and the walk (`Return`, rolled afresh after each of its shots).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Single {
    /// Plays the net style's after-hit and dash spots (NET, or ALL on its coin), not the baseline one's.
    pub net: bool,
    pub spot: [f32; 2],
    pub dash: Option<[f32; 2]>,
    pub back: Return,
}

/// The fixed inputs of a singles bot.
#[derive(Clone, Copy, Debug)]
pub struct Court {
    /// The side sign (+1 plays at −z).
    pub side: f32,
    /// The volley reach depth (2.5, 2.85 or 3.2 by the AI's level).
    pub reach: f32,
    /// The centre rate and radius.
    pub rate: i32,
    pub radius: f32,
}

impl Single {
    /// At the start of a point: in the middle, 10 back (net style) or 11.
    pub fn start(c: &Court, net: bool, roll: &mut impl FnMut() -> u32) -> Single {
        let z = ps2::mul(c.side, if net { -10.0 } else { -11.0 });
        Single { net, spot: [0.0, z], dash: None, back: Return::new(c.rate, roll) }
    }

    /// The net spot: straight up at the reach depth, or a third of the way back from it after shot choice 10.
    fn net_spot(&self, c: &Court, deep: bool) -> [f32; 2] {
        let d = if deep { ps2::add(ps2::div(ps2::sub(11.885, c.reach), 3.0), c.reach) } else { c.reach };
        [0.0, ps2::mul(c.side, -d)]
    }

    /// Go in to the net (a smash seen): to the middle at the reach depth.
    pub fn go_in(&mut self, c: &Court) {
        self.dash.get_or_insert(self.net_spot(c, false));
    }

    /// The ball has passed it on its own side: the dash is off.
    pub fn passed(&mut self, me: [f32; 2], ball: [f32; 2]) {
        if (ball[1] < 0.0) == (me[1] < 0.0) && me[1].abs() < ball[1].abs() {
            self.dash = None;
        }
    }

    /// After its own shot toward `target` (`dash`: a net player's volley or smash, a baseliner's smash; `deep`: the
    /// AI picked shot choice 10, which dashes only a third of the way in). A net player with no dash on picks its spot from the shot's zone against the
    /// opponent's: back at its own depth (no deeper than 10) on a middle shot or one at the opponent, else two
    /// thirds of the way back from the reach depth (no deeper than the opponent) and in to the net when either
    /// stands near it. Then the walk back is rolled again.
    #[allow(clippy::too_many_arguments)]
    pub fn after_hit(&mut self, c: &Court, dash: bool, deep: bool, target: [f32; 2], opp: [f32; 2], me: [f32; 2], roll: &mut dyn FnMut() -> u32) {
        if dash {
            self.dash.get_or_insert(self.net_spot(c, false));
        }
        if self.net {
            if self.dash.is_none() {
                let shot = zone(target, None);
                let them = zone(opp, Some(roll));
                let mine = zone(me, None);
                self.spot[0] = 0.0;
                if shot.0 == 1 || shot == them {
                    self.spot[1] = if me[1].abs() > 10.0 { ps2::mul(c.side, -10.0) } else { me[1] };
                } else {
                    let back = ps2::add(ps2::div(ps2::mul(ps2::sub(11.885, c.reach), 2.0), 3.0), c.reach);
                    let back = if opp[1].abs() < back { opp[1].abs() } else { back };
                    self.spot[1] = ps2::mul(c.side, -back);
                    if mine.1 < 2 || them.1 < 2 {
                        self.dash = Some([0.0, 0.0]);
                    }
                }
            }
            if self.dash.is_some() {
                let x = target[0].clamp(-DASH_X, DASH_X);
                self.dash = Some([x, self.net_spot(c, deep)[1]]);
            }
        }
        self.back = Return::new(c.rate, &mut || roll());
    }

    /// One waiting frame: where to walk, if anywhere (a baseliner dashes to the middle at the reach depth).
    pub fn step(&mut self, c: &Court, me: [f32; 2], roll: &mut impl FnMut() -> u32) -> Option<[f32; 2]> {
        if let Some(d) = self.dash {
            return Some(if self.net { d } else { self.net_spot(c, false) });
        }
        self.back.step(c.rate, c.radius, self.spot, me, roll).then_some(self.spot)
    }
}
