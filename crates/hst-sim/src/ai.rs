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

/// What the AI's run estimate reads off its player: its side and spot, its running state and the match's
/// stamina rules (`player::drain`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Runner {
    /// Speed, agility (as the player has it, weather included) and full stamina.
    pub stats: crate::player::Stats,
    /// Size (%).
    pub size: i32,
    pub side: f32,
    /// Position (x, z).
    pub pos: [f32; 2],
    pub stamina: i32,
    /// Frames into the current stamina second, and frames run so far.
    pub tick: i32,
    pub run: i32,
    /// Its move state: 1 running (the estimate runs on from `run`), 0 and 2 standing (from a standstill); any
    /// other keeps `run`.
    pub moving: u8,
    pub players: i32,
    pub rally: bool,
    pub floor: i32,
}

/// The PS2's `neg.s`-if-negative absolute value (keeps −0).
fn abs(v: f32) -> f32 {
    if v < 0.0 { -v } else { v }
}

impl Runner {
    /// Frames the AI's run to `to` (x, z) takes, as the game estimates it: speeding up frame by frame until full
    /// speed, draining stamina every 60 frames on the way. 9999 for a spot off its half of the court.
    pub fn frames_to(&self, to: [f32; 2]) -> i32 {
        let sign = |v: f32| if v < 0.0 { -1 } else { 1 };
        let [x, z] = to;
        if sign(self.side) == sign(z) || !(abs(x) <= 8.685) || abs(z) < 1.5 || !(abs(z) <= 17.885) {
            return 9999;
        }
        let (dz, dx) = (ps2::sub(z, self.pos[1]), ps2::sub(x, self.pos[0]));
        let mut dist = ps2::sqrt(ps2::madd(ps2::mul(dz, dz), dx, dx));
        if dist <= 0.0 {
            return 0; // ponytail: the game runs on with its speed register unset; standing on the spot is rare
        }
        let s = &self.stats;
        let drain = |st, tick| crate::player::drain(s, st, tick, self.players, self.rally, self.floor);
        let speed = |st, run| crate::player::run_speed(s, run, st, self.size);
        let fit = |dist, v| {
            let q = ps2::div(dist, v);
            let k = q as i32;
            k + ((k as f32) < q) as i32
        };
        let (mut st, mut tick, mut run) = (self.stamina, self.tick, self.run);
        match self.moving {
            0 | 2 => run = 0,
            1 => {
                run += 1;
                (st, tick) = drain(st, tick);
            }
            _ => {}
        }
        let mut v = speed(st, run);
        let mut frames = 1;
        dist = ps2::sub(dist, v);
        while run < s.agility && !(dist <= 0.0) {
            run += 1;
            (st, tick) = drain(st, tick);
            v = speed(st, run);
            frames += 1;
            dist = ps2::sub(dist, v);
        }
        if st >= crate::player::TIRED {
            // the frames left before it tires, in one go
            let n = (59 - tick) + (st - crate::player::TIRED) * 60;
            let fresh = ps2::mul(v, n as f32);
            if dist <= fresh {
                return frames + fit(dist, v);
            }
            dist = ps2::sub(dist, fresh);
            (frames, tick, st) = (frames + n, 59, crate::player::TIRED);
        }
        while !(dist <= 0.0) {
            let drained = tick + 1 >= crate::player::STAMINA_FRAMES;
            (st, tick) = drain(st, tick);
            if drained {
                v = speed(st, run);
            }
            if st == 0 {
                return frames + fit(dist, v);
            }
            frames += 1;
            dist = ps2::sub(dist, v);
        }
        frames
    }
}

/// One entry of the AI's predicted ball path: the ball (x, height, z) and its bounces so far.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PathBall {
    pub pos: [f32; 3],
    pub bounces: i32,
}

/// Where a contact search found the ball: the path entry (`at`, an index into the whole path), the frames the
/// run there takes, and the spot to stand at (x, z; the stand height is the ball's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub at: usize,
    pub frames: i32,
    pub stand: [f32; 2],
}

/// What the AI's contact searches read: its row, its character's reach, its player and the predicted path.
pub struct Searcher<'a> {
    pub row: &'a AiParams,
    pub reach: &'a crate::player::ReachStats,
    /// TParam's hand: 1 minus, 2 plus (see `stand_side`).
    pub strong: u8,
    pub singles: bool,
    /// A doubles AI beside a human partner (voids the run-round roll).
    pub beside_human: bool,
    pub runner: Runner,
    /// The player's depth stat: it stands depth · side / 2 short of the ball.
    pub depth: f32,
    pub path: &'a [PathBall],
    /// The AI's window on the path: from `first` (now) up to `end`.
    pub first: usize,
    pub end: usize,
}

/// A search's limits: contact height `low..=top` and squared reach `d2`.
struct Window {
    low: f32,
    top: f32,
    d2: [f32; 2],
}

impl Searcher<'_> {
    /// The search for a ground stroke or volley (and, `body`, a body shot: reach 0, minus side only): the entry
    /// highest above the stroke height, within 1.3 reach of the base height and below the volley height, that
    /// the AI can run to in time. From `from` (else the window's start), entries with `min_bounces` or more; it
    /// stops at the first frame that finds one unless `all`. Then `stand_side` picks the side.
    pub fn reach_search(
        &self,
        from: Option<usize>,
        min_bounces: i32,
        all: bool,
        body: bool,
        roll: &mut impl FnMut() -> u32,
    ) -> Option<Contact> {
        let r = self.reach;
        let far = ps2::mul(1.3, r.reach);
        let run = self.row.run_round(self.beside_human, roll);
        let top = ps2::add(r.base, far);
        let w = Window {
            low: r.stroke_height,
            top: if r.volley_height <= top { r.volley_height } else { top },
            d2: [0.0, ps2::mul(far, far)],
        };
        let low = r.stroke_height;
        let reach = if body { 0.0 } else { r.reach };
        self.scan(from, min_bounces, all, reach, !body, run, w, None, |b| ps2::sub(b[1], low), |new, best| !(new <= best))
    }

    /// The tiered search: the entry nearest the stroke height (`no_height`: nearest the ground), with a penalty
    /// for a ball past the baseline or the run-round width, in a height and reach band set by `tier` (0 the
    /// tightest, 3 the loosest). `no_reach` stands on the ball (reach 0, minus side only). Entries it checks in
    /// reach are marked in `seen` and skipped by later searches.
    #[allow(clippy::too_many_arguments)]
    pub fn tier_search(
        &self,
        from: Option<usize>,
        tier: u8,
        min_bounces: i32,
        all: bool,
        no_reach: bool,
        no_height: bool,
        seen: &mut [bool],
        roll: &mut impl FnMut() -> u32,
    ) -> Option<Contact> {
        let r = self.reach;
        let (h, m) = (r.stroke_height, |k: f32, v: f32| ps2::mul(k, v));
        // ponytail: a tier past 3 leaves the game's band unset; it gets tier 3's
        let (low, high, near, far) = match tier {
            0 => (m(0.9, h), m(1.1, h), m(0.9, r.reach), m(1.1, r.reach)),
            1 => (m(0.7, h), m(1.3, h), m(0.7, r.reach), m(1.3, r.reach)),
            2 => (m(0.3, h), m(1.7, h), m(0.3, r.reach), m(1.3, r.reach)),
            _ => (0.0, m(3.0, h), 0.0, m(1.3, r.reach)),
        };
        let run = self.row.run_round(self.beside_human, roll);
        let top = ps2::add(r.base, far);
        let w = Window { low, top: if high <= top { high } else { top }, d2: [m(near, near), m(far, far)] };
        let width = self.row.run_round_width(self.singles);
        let h = if no_height { 0.0 } else { h };
        let reach = if no_reach { 0.0 } else { r.reach };
        let over = |v: f32| ps2::div(ps2::mul(0.05, v), 0.5);
        let score = |b: [f32; 3]| {
            let mut s = abs(ps2::sub(h, b[1]));
            for p in [over(ps2::sub(abs(b[2]), 11.885)), over(ps2::sub(abs(b[0]), width))] {
                if !(p <= 0.0) {
                    s = ps2::add(s, p);
                }
            }
            s
        };
        self.scan(from, min_bounces, all, reach, !no_reach, run, w, Some(seen), score, |new, best| !(best <= new))
    }

    /// Both searches' walk over the path: per side, the best-scoring entry in the window it can reach in time.
    #[allow(clippy::too_many_arguments)]
    fn scan(
        &self,
        from: Option<usize>,
        min_bounces: i32,
        all: bool,
        reach: f32,
        plus: bool,
        run: bool,
        w: Window,
        mut seen: Option<&mut [bool]>,
        score: impl Fn([f32; 3]) -> f32,
        better: impl Fn(f32, f32) -> bool,
    ) -> Option<Contact> {
        let rn = &self.runner;
        let side = rn.side;
        let off = ps2::mul(reach, side);
        let back = |z| ps2::sub(z, ps2::div(ps2::mul(self.depth, side), 2.0));
        let sign = |v: f32| if v < 0.0 { -1 } else { 1 };
        let base = self.reach.base;
        // per side (minus, plus): the entry, its score and its run frames
        let mut best: [Option<(usize, f32, i32)>; 2] = [None, None];
        for at in from.unwrap_or(self.first)..self.end {
            if seen.as_ref().is_some_and(|s| s[at]) {
                continue;
            }
            let b = self.path[at];
            if b.bounces >= 2 {
                break;
            }
            let [x, y, z] = b.pos;
            if sign(side) == sign(z) || abs(z) < 1.5 {
                continue;
            }
            if b.bounces >= min_bounces && w.low <= y && y <= w.top {
                for (k, sx) in [ps2::sub(x, off), ps2::add(x, off)].into_iter().enumerate().take(1 + plus as usize) {
                    let sz = back(z);
                    let (dy, dx, dz) = (ps2::sub(base, y), ps2::sub(sx, x), ps2::sub(sz, z));
                    let d2 = ps2::madd(ps2::madd(ps2::mul(dy, dy), dx, dx), dz, dz);
                    if !(w.d2[0] <= d2 && d2 <= w.d2[1]) {
                        continue;
                    }
                    if let Some(s) = seen.as_deref_mut() {
                        s[at] = true;
                    }
                    let sc = score(b.pos);
                    if best[k].is_some_and(|(_, s, _)| !better(sc, s)) {
                        continue;
                    }
                    let frames = rn.frames_to([sx, sz]);
                    if at as i32 - (self.first as i32) < frames {
                        continue;
                    }
                    best[k] = Some((at, sc, frames));
                }
            }
            if !all && best.iter().any(Option::is_some) {
                break;
            }
        }
        let spot = |k: usize| best[k].map(|(at, _, _)| (at as i32, [self.path[at].pos[0], self.path[at].pos[2]]));
        let width = self.row.run_round_width(self.singles);
        let minus = stand_side(spot(0), spot(1), rn.pos, reach, side, self.depth, self.strong, run.then_some(width))?;
        let (at, _, frames) = best[if minus { 0 } else { 1 }]?;
        let [x, _, z] = self.path[at].pos;
        let acc = ps2::add(0.0, x);
        let sx = if minus { ps2::msub(acc, reach, side) } else { ps2::madd(acc, reach, side) };
        Some(Contact { at, frames, stand: [sx, back(z)] })
    }
}
