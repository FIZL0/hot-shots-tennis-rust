//! The computer players' tuning (AIParam.csv) and which row a player gets.
//!
//! The table has 168 rows: six blocks of 14 characters for singles, then the same for doubles (row + 84). A row is
//! picked per player from its character and outfit (`menu_row`) and placed at match start (`Choice::new`).

use crate::ps2;

/// Rows in AIParam.csv.
pub const ROWS: usize = 168;

/// One AIParam.csv row as the game stores it (0x118 bytes; frame counts, percentages, metres, degrees).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AiParams {
    /// 段位 (rank) modulo 17.
    pub rank: u8,
    /// Play style: 1 NET, 2 BASE, 3 ALL.
    pub style: u8,
    /// Doubles formation beside a human partner: 0 staggered (雁行), 1 attacking, 2 defensive.
    pub formation: u8,
    /// Timing error (frames): stroke, volley, smash base, smash random, serve.
    pub stroke_error: i32,
    pub volley_error: i32,
    pub smash_error: i32,
    pub smash_error_random: i32,
    pub serve_error: i32,
    /// Change-of-pace error (%/frm/frm/%), rally and serve.
    pub pace_error: [i32; 4],
    pub serve_pace_error: [i32; 4],
    /// Rates (%): dive, quick serve, net dash.
    pub dive_rate: i32,
    pub quick_serve_rate: i32,
    pub net_dash_rate: i32,
    /// Base reaction time (frames): singles baseline, doubles baseline, singles net, doubles net, after a smash.
    pub react_singles_base: i32,
    pub react_doubles_base: i32,
    pub react_singles_net: i32,
    pub react_doubles_net: i32,
    pub react_after_smash: i32,
    /// Reaction to ball speed (km/frm/%).
    pub react_speed: [i32; 3],
    /// Reaction (frames): random part, lob, special serve.
    pub react_random: i32,
    pub react_lob: i32,
    pub react_special_serve: i32,
    /// Rates (%): body shot, low shot, running round to the strong side.
    pub body_shot_rate: i32,
    pub low_shot_rate: i32,
    pub strong_side_rate: i32,
    /// How far the strong side reaches (m).
    pub strong_side_extend: f32,
    /// Going back to the centre: rate (%) and radius (m), singles then doubles.
    pub singles_center_rate: i32,
    pub singles_center_radius: f32,
    pub doubles_center_rate: i32,
    pub doubles_center_radius: f32,
    /// Near-the-line margin (m) and angle width (degrees).
    pub line_margin: f32,
    pub angle_width: f32,
    /// Shot level mixes (% each, summing to 100): base, then special-return chance (%), key, serve, volley,
    /// return, high contact.
    pub base_level: [i32; 3],
    pub special_return_rate: i32,
    pub key_level: [i32; 4],
    pub serve_level: [i32; 3],
    pub volley_level: [i32; 4],
    pub return_level: [i32; 3],
    pub high_level: [i32; 3],
    /// Shot kind lock (%): flat, drop, lob, top, slice.
    pub kind_lock: [i32; 5],
    /// Serve kind (%): flat, top, slice.
    pub serve_kind: [i32; 3],
    /// Guessing (ヤマ張り): chance %, move frames, stuck frames when wrong.
    pub guess: [i32; 3],
}

/// The game's `atoi`: optional sign after spaces, digits, stops at anything else.
fn atoi(s: &[u8]) -> i32 {
    let s = &s[s.iter().take_while(|c| c.is_ascii_whitespace()).count()..];
    let (neg, s) = match s.first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let n = s.iter().take_while(|c| c.is_ascii_digit()).fold(0i32, |n, &c| n.wrapping_mul(10).wrapping_add((c - b'0') as i32));
    if neg { n.wrapping_neg() } else { n }
}

/// The game's own decimal reader: the digits after the first '.' (×0.1, ×0.01, …), then those before it read right
/// to left (×1, ×10, …), each step a multiply-add; no sign.
pub fn atof(s: &[u8]) -> f32 {
    let mut sum = 0.0f32;
    let dot = s.iter().position(|&c| c == b'.');
    if let Some(d) = dot {
        let mut w = 0.1f32;
        for &c in s[d + 1..].iter().take_while(|c| c.is_ascii_digit()) {
            sum = ps2::madd(sum, ps2::utof((c - b'0') as u32), w);
            w = ps2::div(w, 10.0);
        }
    }
    let mut w = 1.0f32;
    for &c in s[..dot.unwrap_or(s.len())].iter().rev().take_while(|c| c.is_ascii_digit()) {
        sum = ps2::madd(sum, ps2::utof((c - b'0') as u32), w);
        w = ps2::mul(w, 10.0);
    }
    sum
}

/// `strtok`: the non-empty pieces between any of `delims`.
fn tokens<'a>(s: &'a [u8], delims: &'a [u8]) -> impl Iterator<Item = &'a [u8]> {
    s.split(move |c| delims.contains(c)).filter(|t| !t.is_empty())
}

/// The next cell as an integer, a decimal, or split at '/' into `N` integers (missing cells read as 0).
fn int<'a>(c: &mut impl Iterator<Item = &'a [u8]>) -> i32 {
    atoi(c.next().unwrap_or(&[]))
}

fn float<'a>(c: &mut impl Iterator<Item = &'a [u8]>) -> f32 {
    atof(c.next().unwrap_or(&[]))
}

fn split<'a, const N: usize>(c: &mut impl Iterator<Item = &'a [u8]>) -> [i32; N] {
    let mut parts = tokens(c.next().unwrap_or(&[]), b"/");
    std::array::from_fn(|_| atoi(parts.next().unwrap_or(&[])))
}

impl AiParams {
    /// One data line after its serial number (`#`, name, rank, style, …). None for a style the game doesn't know.
    fn parse_row<'a>(c: &mut impl Iterator<Item = &'a [u8]>) -> Option<Self> {
        c.next(); // character number
        c.next(); // name
        let rank = (int(c) % 17) as u8;
        let style = match c.next() {
            Some(b"ALL") => 3,
            Some(b"NET") => 1,
            Some(b"BASE") => 2,
            _ => return None,
        };
        Some(AiParams {
            rank,
            style,
            formation: int(c) as u8,
            stroke_error: int(c),
            volley_error: int(c),
            smash_error: int(c),
            smash_error_random: int(c),
            serve_error: int(c),
            pace_error: split(c),
            serve_pace_error: split(c),
            dive_rate: int(c),
            quick_serve_rate: int(c),
            net_dash_rate: int(c),
            react_singles_base: int(c),
            react_doubles_base: int(c),
            react_singles_net: int(c),
            react_doubles_net: int(c),
            react_after_smash: int(c),
            react_speed: split(c),
            react_random: int(c),
            react_lob: int(c),
            react_special_serve: int(c),
            body_shot_rate: int(c),
            low_shot_rate: int(c),
            strong_side_rate: int(c),
            strong_side_extend: float(c),
            singles_center_rate: int(c),
            singles_center_radius: float(c),
            doubles_center_rate: int(c),
            doubles_center_radius: float(c),
            line_margin: float(c),
            angle_width: float(c),
            base_level: split(c),
            special_return_rate: int(c),
            key_level: split(c),
            serve_level: split(c),
            volley_level: split(c),
            return_level: split(c),
            high_level: split(c),
            kind_lock: split(c),
            serve_kind: split(c),
            guess: split(c),
        })
    }

    /// The table from AIParam.csv's bytes, as the game loads it: lines whose first cell is a number 0..=167 fill that
    /// row, up to and including row 167; anything else is skipped.
    pub fn table(csv: &[u8]) -> Vec<AiParams> {
        let mut rows = vec![AiParams::default(); ROWS];
        for line in tokens(csv, b"\n\r") {
            let mut cells = tokens(line, b",");
            let Some(first) = cells.next() else { continue };
            if !first[0].is_ascii_digit() {
                continue;
            }
            let n = atoi(first) as u32 as usize;
            if n >= ROWS {
                continue;
            }
            if let Some(a) = Self::parse_row(&mut cells) {
                rows[n] = a;
            }
            if n == ROWS - 1 {
                break;
            }
        }
        rows
    }

    /// The record's 0x118 bytes as the game holds them in memory.
    pub fn bytes(&self) -> Vec<u8> {
        let mut b = vec![self.rank, self.style, self.formation, 0];
        let mut put = |v: &[i32]| v.iter().for_each(|x| b.extend_from_slice(&x.to_le_bytes()));
        put(&[self.stroke_error, self.volley_error, self.smash_error, self.smash_error_random, self.serve_error]);
        put(&self.pace_error);
        put(&self.serve_pace_error);
        put(&[self.dive_rate, self.quick_serve_rate, self.net_dash_rate]);
        put(&[self.react_singles_base, self.react_doubles_base, self.react_singles_net, self.react_doubles_net, self.react_after_smash]);
        put(&self.react_speed);
        put(&[self.react_random, self.react_lob, self.react_special_serve, self.body_shot_rate, self.low_shot_rate, self.strong_side_rate]);
        let f = |x: f32| x.to_bits() as i32;
        put(&[f(self.strong_side_extend), self.singles_center_rate, f(self.singles_center_radius), self.doubles_center_rate]);
        put(&[f(self.doubles_center_radius), f(self.line_margin), f(self.angle_width)]);
        put(&self.base_level);
        put(&[self.special_return_rate]);
        put(&self.key_level);
        put(&self.serve_level);
        put(&self.volley_level);
        put(&self.return_level);
        put(&self.high_level);
        put(&self.kind_lock);
        put(&self.serve_kind);
        put(&self.guess);
        b
    }
}

/// The character-select's pick for a computer player: the outfit chooses a block of 14 (outfits 0–3 block 0, 5–8
/// block 1, 4 and 9 block 3), plus the character.
pub fn menu_row(character: u8, outfit: u8) -> u8 {
    let mut block = (outfit > 4) as u8 + 2 * (outfit == 4) as u8 + 2 * (outfit == 9) as u8;
    if block == 2 {
        block = 3;
    }
    block * 14 + character
}

/// A computer player's row and level at match start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Choice {
    pub row: usize,
    /// 0..=3; 3 unless the match setup forces a level.
    pub level: u8,
    /// The setup word's third byte (a challenge match's strategy index), kept by the AI.
    pub strategy: u8,
    /// A distance the AI starts with (2.5, +0.35 at level 1, +0.7 from level 2).
    pub reach: f32,
}

impl Choice {
    /// From the player's setup word (row in the low byte; a forced level 1..=4 in the second byte, which then puts
    /// the character in that block) and the character; doubles use the second half of the table.
    pub fn new(setup: u32, character: u8, doubles: bool) -> Self {
        let forced = (setup >> 8) & 0xff;
        let (level, row) = if (1..=4).contains(&forced) {
            (forced as u8 - 1, character as u32 + 14 * (forced - 1))
        } else {
            // the game looks the level up in a six-entry table by block; every entry is 3
            (3, setup & 0xff)
        };
        let row = row.min(83) + if doubles { 84 } else { 0 };
        let reach = match level {
            0 => 2.5,
            1 => ps2::add(2.5, 0.35000002),
            _ => ps2::add(2.5, 0.70000005),
        };
        Choice { row: row as usize, level, strategy: (setup >> 16) as u8, reach }
    }
}

/// An opponent's shot as the AI remembers it: its kind (0 serve, 1 ground stroke, 2 volley, 3 smash, …) and the
/// ball's velocity off the racket (m/frame).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Seen {
    pub kind: i32,
    pub vel: [f32; 3],
}

/// What the AI weighs when an opponent has just hit: that shot, the opponents' shot before it, and the serve before
/// the last one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shots {
    pub last: Seen,
    pub before: Option<Seen>,
    pub serve_before: Option<[f32; 3]>,
}

/// The timing errors an AI draws for its next hit (frames; positive presses late), and what the ball's pace added.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Timing {
    pub stroke: i32,
    pub volley: i32,
    pub smash: i32,
    pub serve: i32,
    /// Change of pace: + the ball came faster than the one before (late), − slower (early); 0 none.
    pub pace: i32,
    /// Extra reaction for a fast ball (frames; added by `AiParams::reaction`).
    pub fast_ball: i32,
    /// A change of pace or the fast-ball reaction took hold (even a pace of 0 frames); the AI then doesn't guess.
    pub reacted: bool,
}

/// The ball's speed as the game sums it (y² first).
fn speed(v: [f32; 3]) -> f32 {
    ps2::sqrt(ps2::madd(ps2::madd(ps2::mul(v[1], v[1]), v[0], v[0]), v[2], v[2]))
}

/// Change-of-pace error between two shots' velocities: `p` = [threshold %, frames per 5 % past it, most frames,
/// chance %]. None below the threshold.
fn pace(last: [f32; 3], earlier: [f32; 3], p: [i32; 4]) -> Option<i32> {
    let (a, b) = (speed(last), speed(earlier));
    let (r, sign) = if a < b { (ps2::div(b, a), -1) } else { (ps2::div(a, b), 1) };
    let pct = ps2::mul(ps2::sub(r, 1.0), 100.0) as i32;
    (pct >= p[0]).then(|| (p[1] * (pct - p[0]) / 5).min(p[2]) * sign)
}

/// A `p` % chance on the game's generator (bits 16..30, mod 100).
fn chance(roll: &mut impl FnMut() -> u32, p: i32) -> bool {
    ((roll() >> 16 & 0x7fff) % 100) < p.max(0) as u32
}

/// A random error of up to `n` frames either way (`base` added to its size): a sign draw, then a size draw.
fn error(roll: &mut impl FnMut() -> u32, base: i32, n: i32) -> i32 {
    let sign = if roll() >> 16 & 1 == 0 { -1 } else { 1 };
    sign * (base + (roll() >> 16 & 0x7fff) as i32 % (n + 1))
}

/// The pace error adds to an error's size and gives it its sign.
fn with_pace(e: i32, pace: i32) -> i32 {
    match pace.signum() {
        1 => e.abs() + pace,
        -1 => -(e.abs() - pace),
        _ => e,
    }
}

impl AiParams {
    /// The timing errors an AI draws for its next hit, at the start of a point, before its serve and whenever a
    /// ball is hit. `shots`: the opponents' shots when one of them just hit (None otherwise). `receiver`: this
    /// player receives the serve. `first`: its team hasn't hit yet this point (the stroke error is halved).
    /// `smash_third`: the doubles AI's case that takes a third of the smash base. `roll`: the AI's MT19937.
    pub fn timing(
        &self,
        shots: Option<&Shots>,
        receiver: bool,
        first: bool,
        smash_third: bool,
        roll: &mut impl FnMut() -> u32,
    ) -> Timing {
        let mut t = Timing::default();
        if let Some(s) = shots {
            // the rally's change of pace (the shot before wasn't a serve), else the serve's against the last serve
            let rally = match s.before {
                Some(b) if b.kind != 0 && chance(roll, self.pace_error[3]) => Some(pace(s.last.vel, b.vel, self.pace_error)),
                _ => None,
            };
            let paced = match rally {
                Some(p) => p,
                None => match s.serve_before {
                    Some(v) if s.last.kind == 0 && receiver && chance(roll, self.serve_pace_error[3]) => {
                        pace(s.last.vel, v, self.serve_pace_error)
                    }
                    _ => None,
                },
            };
            t.reacted = paced.is_some();
            t.pace = paced.unwrap_or(0);
            // a ball faster than react_speed[0] km/h adds react_speed[1] frames, react_speed[2] % of the time
            if !first
                && chance(roll, self.react_speed[2])
                && (s.last.kind != 0 || receiver)
                && ps2::div(ps2::mul(3600.0, ps2::mul(60.0, speed(s.last.vel))), 1000.0) > self.react_speed[0] as f32
            {
                t.fast_ball = self.react_speed[1];
                t.reacted = true;
            }
        }
        t.stroke = with_pace(error(roll, 0, self.stroke_error), t.pace);
        if first {
            t.stroke /= 2;
        }
        t.volley = with_pace(error(roll, 0, self.volley_error), t.pace);
        let base = if smash_third { self.smash_error / 3 } else { self.smash_error };
        t.smash = with_pace(error(roll, base, self.smash_error_random), t.pace);
        t.serve = error(roll, 0, self.serve_error);
        t
    }
}

/// Which way an AI guesses (ヤマ張り) the serve will come before reading it, as its draw leaves it: `Wide` the
/// half on its side sign's +x, `Other` the opposite one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guess {
    Wide,
    Other,
}

/// How a guess turned out once the AI reads the ball: right (it then runs harder and times the hit within a
/// frame), wrong (it stops dead for the stuck frames, then reads again) or neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Right,
    Wrong,
    Neither,
}

/// The shot-choice picks an AI draws right after its timing errors (and before the reaction on an opponent's hit).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Picks {
    pub quick_serve: bool,
    /// Drawn only at a reset or after the AI's own dive (`picks`' `dive`); kept otherwise.
    pub dive: Option<bool>,
    pub body: bool,
    pub low: bool,
    /// Indices into the row's mixes: serve 0..2, volley 0..3, return 0..2, high contact 0..2.
    pub serve_level: u8,
    pub volley_level: u8,
    pub return_level: u8,
    pub high_level: u8,
}

/// A level picked from a % mix: the first bucket the draw (mod 100) falls under, else the last.
fn level(roll: &mut impl FnMut() -> u32, mix: &[i32]) -> u8 {
    let u = ((roll() >> 16 & 0x7fff) % 100) as i32;
    let mut edge = 0;
    for (i, &m) in mix[..mix.len() - 1].iter().enumerate() {
        edge += m;
        if u < edge {
            return i as u8;
        }
    }
    (mix.len() - 1) as u8
}

impl AiParams {
    /// The picks drawn after `timing`. `dive`: the dive chance is drawn too (a reset or serve, or the AI's own
    /// team just hit with the AI's own dive); 7 draws without it, 8 with.
    pub fn picks(&self, dive: bool, roll: &mut impl FnMut() -> u32) -> Picks {
        let quick_serve = chance(roll, self.quick_serve_rate);
        let dive = dive.then(|| chance(roll, self.dive_rate));
        let body = chance(roll, self.body_shot_rate);
        let low = chance(roll, self.low_shot_rate);
        Picks {
            quick_serve,
            dive,
            body,
            low,
            serve_level: level(roll, &self.serve_level),
            volley_level: level(roll, &self.volley_level),
            return_level: level(roll, &self.return_level),
            high_level: level(roll, &self.high_level),
        }
    }
}

impl AiParams {
    /// The reaction (frames the AI stands before going for the ball) drawn after an opponent's hit, after the
    /// timing errors and the shot-choice draws: after a smash (`last.kind` 4) the after-smash frames, else the base
    /// one (the net value within 6.4 m of the net, `near_net`; the doubles columns beside a human partner, else the
    /// singles ones) plus the fast-ball frames; then a random part up to the row's, and a lob (`lob`: a △ ground
    /// stroke, volley or dive) or special serve adds half its frames plus up to half again.
    pub fn reaction(
        &self,
        t: &Timing,
        last: &Seen,
        lob: bool,
        special_serve: bool,
        beside_human: bool,
        near_net: bool,
        roll: &mut impl FnMut() -> u32,
    ) -> i32 {
        let mut upto = |n: i32| (roll() >> 16 & 0x7fff) as i32 % (n + 1);
        let r = if last.kind == 4 {
            self.react_after_smash + upto(self.react_random)
        } else {
            let base = match (beside_human, near_net) {
                (true, true) => self.react_doubles_net,
                (true, false) => self.react_doubles_base,
                (false, true) => self.react_singles_net,
                (false, false) => self.react_singles_base,
            };
            t.fast_ball + upto(self.react_random) + base
        };
        let half = |n: i32, upto: &mut dyn FnMut(i32) -> i32| n / 2 + upto(n / 2);
        r + if (1..=3).contains(&last.kind) && lob {
            half(self.react_lob, &mut upto)
        } else if last.kind == 0 && special_serve {
            half(self.react_special_serve, &mut upto)
        } else {
            0
        }
    }
}

impl AiParams {
    /// The guess drawn at the tail of a hit-message draw, when an opponent has just served: never if a change of
    /// pace or the fast-ball reaction took hold (`t.reacted`), beside a human partner, or for any shot but the serve.
    /// `partner_bot`: true in singles. `roll`: the generator right after `reaction`'s draws. A guess replaces the
    /// reaction with the move frames (`guess[1]`).
    pub fn guess(&self, t: &Timing, serve: bool, partner_bot: bool, roll: &mut impl FnMut() -> u32) -> Option<Guess> {
        if t.reacted || !serve || !partner_bot || !chance(roll, self.guess[0]) {
            return None;
        }
        Some(if chance(roll, 50) { Guess::Wide } else { Guess::Other })
    }
}

impl Guess {
    /// The x the AI runs to while it guesses, keeping its depth: the doubles sideline on the guessed half when that
    /// is the half the serve comes from (`ad`: the ad court serves), else the centre line. `side`: its side sign.
    pub fn target_x(self, side: f32, ad: bool) -> f32 {
        match (self, ad) {
            (Guess::Wide, false) => side * 5.485,
            (Guess::Other, true) => side * -5.485,
            _ => 0.0,
        }
    }

    /// Judged when the AI first finds its contact point: the direction from where it stood at the draw (x, z) to
    /// that point, against the guessed side.
    pub fn verdict(self, side: f32, from: [f32; 2], contact: [f32; 2]) -> Verdict {
        let dir = if self == Guess::Wide { side } else { -side };
        let (dx, dz) = (ps2::sub(contact[0], from[0]), ps2::sub(contact[1], from[1]));
        let inv = ps2::div(1.0, ps2::sqrt(ps2::madd(ps2::mul(dx, dx), dz, dz)));
        let d = ps2::add(ps2::madd(ps2::mul(ps2::mul(dz, inv), 0.0), ps2::mul(dx, inv), dir), 0.0);
        if d >= 0.5 {
            Verdict::Right
        } else if d < 0.0 {
            Verdict::Wrong
        } else {
            Verdict::Neither
        }
    }
}

/// A side of the ball the AI's contact search can stand on: the path entry it found (its index, counted from the
/// path's start) and the ball's x and z there.
pub type Spot = (i32, [f32; 2]);

impl AiParams {
    /// The contact search's run-round roll, drawn on every search (whether it finds the ball or not): a
    /// `strong_side_rate` % chance, void for a doubles AI beside a human partner.
    pub fn run_round(&self, beside_human: bool, roll: &mut impl FnMut() -> u32) -> bool {
        chance(roll, self.strong_side_rate) && !beside_human
    }

    /// How far out the AI runs round: the court's half width (singles or doubles) plus the row's extend.
    pub fn run_round_width(&self, singles: bool) -> f32 {
        ps2::add(if singles { 4.115 } else { 5.485 }, self.strong_side_extend)
    }
}

/// Where the AI stands when its contact search found the ball on both sides of it: `minus` (standing at ball x −
/// reach·side, the forehand of a right-hander) or `plus` (ball x + reach·side). It keeps the stand spot nearer to
/// `from` (its position, x and z), unless the farther one is its `strong` side (1 minus, 2 plus: TParam's hand,
/// right 1) and the run-round roll passed (`width`: Some(`run_round_width`)) and that spot's |x| is inside the
/// width. Each spot stands `depth` (player stat) · side / 2 short of the ball. Some(true) for minus, Some(false) for
/// plus, None when neither was found. As the game, a side found at the path's first entry (index 0) isn't weighed.
/// ponytail: only minus at index 0 found makes the game read the entry before the path; this keeps minus then.
pub fn stand_side(
    minus: Option<Spot>,
    plus: Option<Spot>,
    from: [f32; 2],
    reach: f32,
    side: f32,
    depth: f32,
    strong: u8,
    width: Option<f32>,
) -> Option<bool> {
    let (m, p) = match (minus, plus) {
        (None, None) => return None,
        (Some(m), Some(p)) if m.0 > 0 && p.0 > 0 => (m.1, p.1),
        (Some(m), p) => return Some(m.0 >= 1 || p.is_none()),
        (None, Some(_)) => return Some(false),
    };
    let off = ps2::mul(reach, side);
    let x = [ps2::sub(m[0], off), ps2::add(p[0], off)];
    let back = ps2::div(ps2::mul(depth, side), 2.0);
    let dist = |x: f32, z: f32| {
        let (dx, dz) = (ps2::sub(x, from[0]), ps2::sub(ps2::sub(z, back), from[1]));
        ps2::madd(ps2::mul(dz, dz), dx, dx)
    };
    let nearer_minus = dist(x[0], m[1]) <= dist(x[1], p[1]);
    let strong_minus = match strong {
        1 => true,
        2 => false,
        _ => return Some(nearer_minus),
    };
    let run = strong_minus != nearer_minus
        && width.is_some_and(|w| x[if strong_minus { 0 } else { 1 }].abs() < w);
    Some(if run { strong_minus } else { nearer_minus })
}

/// The bounds an ALL-style AI's net rate is kept in after each point (the same for both AI classes).
pub const NET_RATE: (i32, i32) = (15, 85);

/// What an AI's per-frame update runs: its top-level state. A point starts it at `Start`; on the first update it
/// becomes `Serve` (the server), `Receive` (the receiver) or `Rally` (the partners), and the serve and receive
/// states hand over to `Rally` once their stroke is done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Start,
    Serve,
    Receive,
    Rally,
}

/// The rally routine an AI plays: from the net or from the baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    Net,
    Base,
}

/// The per-player AI object (the singles and doubles classes share all of this): its state, whether it drives
/// its stick at all this point, and the ALL-style player's choice between net and baseline play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mind {
    pub phase: Phase,
    /// Off, it stands still: at each point's end a coin flip, back on at the next point.
    pub active: bool,
    /// An ALL-style player's pick: plays the net routine (else the baseline one).
    pub net: bool,
    /// The chance (%) of picking the net, moved 5 after each point by how the pick fared.
    pub net_rate: i32,
    /// Its team's hits left before it picks again.
    pub net_left: i32,
}

impl Default for Mind {
    fn default() -> Self {
        Mind { phase: Phase::Start, active: true, net: false, net_rate: 50, net_left: 0 }
    }
}

impl Mind {
    /// The AI's point reset (before the timing draw). `match_start`: the first point, which deals a fresh net
    /// pick at even odds; doubles only does so when its partner is a computer player too (`partner_bot`, true in
    /// singles).
    pub fn reset(&mut self, match_start: bool, partner_bot: bool, roll: &mut impl FnMut() -> u32) {
        self.active = true;
        if match_start && partner_bot {
            self.net_rate = 50;
            self.net = chance(roll, 50);
        }
        self.phase = Phase::Start;
    }

    /// The tail of a timing draw made at a point reset or the serve: its team gets 2 to 4 hits before the next pick.
    pub fn count(&mut self, roll: &mut impl FnMut() -> u32) {
        self.net_left = (roll() >> 16 & 0x7fff) as i32 % 3 + 2;
    }

    /// Its team has hit (the hit-message draw, in place of `count`).
    pub fn own_hit(&mut self) {
        self.net_left -= 1;
    }

    /// The first update of a point: which state it takes.
    pub fn start(&mut self, server: bool, receiver: bool) -> Phase {
        self.phase = if server {
            Phase::Serve
        } else if receiver {
            Phase::Receive
        } else {
            Phase::Rally
        };
        self.phase
    }

    /// Entering the rally (also on each new ball path while it rallies): picks again once its team has used up
    /// its hits. The caller first rolls its return-to-centre (`position::Return::new`).
    pub fn rally(&mut self, roll: &mut impl FnMut() -> u32) {
        self.phase = Phase::Rally;
        if self.net_left < 1 {
            self.net = chance(roll, self.net_rate);
            self.count(roll);
        }
    }

    /// The routine its rally state plays, by the row's style: 1 net, 2 baseline, 3 (ALL) its pick.
    pub fn play(&self, style: u8) -> Play {
        match style {
            1 => Play::Net,
            2 => Play::Base,
            _ if self.net => Play::Net,
            _ => Play::Base,
        }
    }

    /// The point is over: whether it keeps moving until the next.
    pub fn point_over(&mut self, roll: &mut impl FnMut() -> u32) {
        self.active = chance(roll, 50);
    }

    /// The players react to the point: an ALL-style AI (`style` 3) leans 5 toward the pick that won it (away from
    /// the one that lost), within `NET_RATE`, and picks again.
    pub fn point_result(&mut self, style: u8, won: bool, roll: &mut impl FnMut() -> u32) {
        if style != 3 {
            return;
        }
        self.net_rate += if won == self.net { 5 } else { -5 };
        self.net_rate = self.net_rate.clamp(NET_RATE.0, NET_RATE.1);
        self.net = chance(roll, self.net_rate);
    }
}

/// Where a serving AI walks along the baseline before its toss (x; its side sign `side`, `ad` the ad court): its
/// usual spot near the centre mark, or at serve level 0 one of three spots out to the singles or doubles sideline.
pub fn serve_spot(level0: bool, doubles: bool, side: f32, ad: bool, roll: &mut impl FnMut() -> u32) -> f32 {
    let x = if level0 {
        match (roll() >> 16 & 0x7fff) % 3 {
            0 => 0.7,
            1 if doubles => 2.7425,
            1 => 2.0575,
            // one ulp off the round numbers, as the game has them
            _ if doubles => f32::from_bits(0x4099_1eb9), // 4.785
            _ => f32::from_bits(0x405a_8f5b),            // 3.415
        }
    } else {
        0.7
    };
    ps2::mul(ps2::mul(x, side), if ad { -1.0 } else { 1.0 })
}

/// How long a serving AI stands on its spot before the toss: 60 to 119 frames.
pub fn serve_wait(roll: &mut impl FnMut() -> u32) -> i32 {
    (roll() >> 16 & 0x7fff) as i32 % 60 + 60
}
