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
