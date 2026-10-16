//! Where a computer player puts its shot: the aim it chooses when it has found its contact point, as the stick
//! a human would hold (world x and z, length up to 1, turned by its side), and the shot plan that goes with it.
//! Drawn from the AIParam row's angle width, length mix (`key_level`), special return rate and base level mix,
//! and from where both players stand (`position::zone_in`, the finer `fine_zone`). Floats on the FPU (`ps2`), the
//! turn on VU0 (`world`).
#![deny(clippy::float_arithmetic)]

use crate::ai::AiParams;
use crate::lines::{BASELINE, DEG, SERVICE_LINE, SINGLES_SIDELINE};
use crate::position::zone_in;
use crate::ps2::{F, utof};
use crate::rng::{TWO_POW_M32, coin, r15};
use crate::{vu0, world};

/// The plans an aim comes with (`Aim::plan`), and the button each presses (`button`).
pub mod plan {
    /// Ground strokes: topspin (✕), slice (○), lob (△; also the special return).
    pub const TOPSPIN: u8 = 7;
    pub const SLICE: u8 = 8;
    pub const LOB: u8 = 10;
    /// Volleys: ✕, ○, △, and the drop volley (○).
    pub const VOLLEY: u8 = 0xc;
    pub const VOLLEY_SLICE: u8 = 0xd;
    pub const VOLLEY_LOB: u8 = 0xf;
    pub const VOLLEY_DROP: u8 = 0x10;
    /// The smash (✕).
    pub const SMASH: u8 = 0x11;
    /// The serve's tosses: weak (○), strong (✕), underhand (△).
    pub const TOSS_WEAK: u8 = 0;
    pub const TOSS_STRONG: u8 = 1;
    pub const TOSS_UNDER: u8 = 2;
    /// The serve's swings: ✕, ○, △ (after an underhand toss).
    pub const SWING_CROSS: u8 = 3;
    pub const SWING_CIRCLE: u8 = 4;
    pub const SWING_UNDER: u8 = 6;
}

/// The shot buttons a plan presses.
pub mod button {
    pub const CROSS: u8 = 1;
    pub const CIRCLE: u8 = 2;
    pub const TRIANGLE: u8 = 4;
}

/// The contact the AI found (`Look::kind`, `Pair::kind`).
pub mod contact {
    /// Ground strokes: from the tiered search, at reach.
    pub const TIERED: u8 = 0;
    pub const REACH: u8 = 1;
    pub const VOLLEY: u8 = 2;
    pub const SMASH: u8 = 3;
    pub const DIVE: u8 = 4;
}

/// The opponent's last shot kind when it smashed (`Look::opp_kind`).
const OPP_SMASH: i32 = 3;
/// A third, two thirds and five sixths, as the game's literals (the stick length bands).
const THIRD: f32 = f32::from_bits(0x3eaa_aaab);
const TWO_THIRDS: f32 = f32::from_bits(0x3f2a_aaab);
const FIVE_SIXTHS: f32 = f32::from_bits(0x3f55_5555);

/// `fmodf`: its result is exact, so IEEE `%` is the game's.
#[allow(clippy::float_arithmetic)]
fn fmod(a: F, b: f32) -> F {
    F(a.0 % b)
}

/// The aim (the AI's stick: x, y, z, w) and the plan byte the shot is played with.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Aim {
    pub stick: [f32; 4],
    pub plan: u8,
}

/// What the aim choosers read when the AI picks its shot.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Look {
    /// Its own and its opponent's position (x, z).
    pub me: [f32; 2],
    pub opp: [f32; 2],
    /// Its side sign.
    pub side: f32,
    /// The court half width it plays to: singles, else doubles.
    pub singles: bool,
    /// The contact it found (`contact`).
    pub kind: u8,
    /// The opponent's last shot kind (`OPP_SMASH`).
    pub opp_kind: i32,
    /// The opponent plays left-handed.
    pub opp_lefty: bool,
    /// The opponent's spot the AI keeps (x, z).
    pub mark: [f32; 2],
    /// The contact point's height (game y, down) and the player's three reach heights it is held against.
    pub contact_y: f32,
    pub heights: [f32; 3],
    /// Its volley and high-ball level picks (P11g's draws), and its level.
    pub volley_level: u8,
    pub high_level: u8,
    pub level: u8,
}

/// Depth mix (%: deep, mid; rest short) by base level, for an opponent deep, mid, near the net: the receive's
/// and the rally's.
const RECEIVE_DEPTH: [i32; 18] = [10, 40, 20, 70, 80, 20, 70, 10, 10, 60, 50, 0, 35, 30, 35, 30, 35, 30];
const RALLY_DEPTH: [i32; 18] = [80, 0, 80, 0, 100, 0, 50, 50, 50, 50, 50, 50, 35, 30, 35, 30, 35, 30];
/// A volley's short-angle chance (%) by level.
const VOLLEY_SHORT: [i32; 4] = [30, 20, 10, 5];

fn percent(roll: &mut impl FnMut() -> u32) -> i32 {
    (r15(roll()) % 100) as i32
}

fn chance(roll: &mut impl FnMut() -> u32, p: i32) -> bool {
    percent(roll) < p
}

/// The MT word as a fraction of 2³².
fn uniform(roll: &mut impl FnMut() -> u32) -> f32 {
    (F(TWO_POW_M32) * utof(roll())).0
}

fn width(singles: bool) -> f32 {
    crate::lines::sideline(!singles)
}

/// Whether a point is on the court (within the baseline and the half width).
fn on_court(p: [f32; 2], w: f32) -> bool {
    p[1].abs() <= BASELINE && p[0].abs() <= w
}

/// Pick 2, 1 or 0 from a cumulative mix.
fn pick(u: i32, a: i32, b: i32) -> u8 {
    if u < a {
        2
    } else if u < a + b {
        1
    } else {
        0
    }
}

/// Where a point lies on a court of half width `w` in sixths: its column (0..=5 across, from the −x side seen from
/// its end) and depth band (0..=5) past 1.5 from the net. `blur` rolls between neighbours near the edges.
pub fn fine_zone(p: [f32; 2], w: f32, blur: Option<&mut dyn FnMut() -> u32>) -> (u8, u8) {
    let ax = p[0].abs();
    let s = if p[1] < 0.0 { 1.0 } else { -1.0 };
    let d = (F(p[1].abs()) - 1.5).0;
    let Some(roll) = blur else {
        let out = 0.0 < (F(p[0]) * s).0;
        let col = if ax <= (F(w) / 3.0).0 {
            if out { 3 } else { 2 }
        } else if ax <= (F(w) * 2.0 / 3.0).0 {
            if out { 4 } else { 1 }
        } else if out {
            5
        } else {
            0
        };
        let band = [1.7308334, 3.4616668, 5.1925, 6.9233336, 8.654167].iter().take_while(|&&b| b < d).count();
        return (col, band as u8);
    };
    let mut roll = || (F::ZERO + 0.0).madd(1.0, F(TWO_POW_M32) * utof(roll())).0;
    let q = (F(w) * 2.0 / 6.0 / 4.0).0;
    let x = (F::ZERO + w).madd(p[0], s).0;
    let mut col = 5;
    for k in 1..=5u8 {
        let edge = (F(w) * (k as f32) / 3.0).0;
        let lo = (F(edge) - q).0;
        if x <= lo {
            col = k - 1;
            break;
        }
        if x < (F(edge) + q).0 {
            col = if roll() < ((F(x) - lo) / (F(2.0) * q)).0 { k } else { k - 1 };
            break;
        }
    }
    const EDGE: [f32; 10] =
        [1.298125, 2.1635418, 3.0289586, 3.894375, 4.759792, 5.6252084, 6.4906254, 7.356042, 8.221458, 9.086876];
    let mut band = 5;
    for k in 0..5u8 {
        let (lo, hi) = (EDGE[2 * k as usize], EDGE[2 * k as usize + 1]);
        if d <= lo {
            band = k;
            break;
        }
        if d < hi {
            band = k + (roll() < ((F(d) - lo) / 0.8654167).0) as u8;
            break;
        }
    }
    (col, band)
}

impl AiParams {
    /// The stick for a target zone (column 0..=2 seen from the opponent's lane rule, depth 0 deep..=2 short): a
    /// direction the angle width either side of the zone's (the middle zone: any direction) and a length from the
    /// `key_level` mix (the middle zone: up to a third), turned by `side` (the AI's side sign).
    pub fn aim(&self, zone: (u8, u8), side: f32, roll: &mut impl FnMut() -> u32) -> [f32; 4] {
        let (angle, lo, hi) = if zone == (1, 1) {
            ((F::ZERO + -180.0).madd(360.0, uniform(roll)).0, 0.0, THIRD)
        } else {
            let base = match zone {
                (1, 0) => 0.0,
                (2, 0) => 45.0,
                (2, 1) => 90.0,
                (2, 2) => 135.0,
                (1, 2) => 180.0,
                (0, 2) => -135.0,
                (0, 1) => -90.0,
                _ => -45.0,
            };
            let aw = self.angle_width;
            let angle = (base + (F::ZERO + -F(aw)).madd(F(aw) - -F(aw), uniform(roll))).0;
            let r = percent(roll);
            let k = &self.key_level;
            let band = if r < k[0] {
                (THIRD, 0.5)
            } else if r < k[0] + k[1] {
                (0.5, TWO_THIRDS)
            } else if r < k[0] + k[1] + k[2] {
                (TWO_THIRDS, FIVE_SIXTHS)
            } else {
                (FIVE_SIXTHS, 1.0)
            };
            (angle, band.0, band.1)
        };
        let mut rad = (F(DEG) * angle).0;
        if rad > world::PI {
            rad = (fmod(F(world::PI) + rad, world::TWO_PI) - world::PI).0;
        } else if rad < -F(world::PI) {
            rad = (F(world::PI) + fmod(F(rad) - world::PI, world::TWO_PI)).0;
        }
        let m = world::mat_mul(&world::IDENTITY, &world::rot_y(rad));
        let v = vu0::transform(&m, [0.0, 0.0, 1.0, 0.0]);
        let len = (F::ZERO + lo).madd(F(hi) - lo, uniform(roll)).0;
        v.map(|c| (F(c) * -F(side) * len).0)
    }

    /// The singles NET rally routine's aim (its shots and dives; once taken for the aim at a return of serve).
    pub fn receive_aim(&self, l: &Look, roll: &mut impl FnMut() -> u32) -> Aim {
        if l.kind == contact::VOLLEY {
            return self.volley_aim(l, roll);
        }
        let w = width(l.singles);
        let (ox, od) = zone_in(l.opp, w, Some(&mut *roll));
        let (_, md) = zone_in(l.me, w, None);
        let s = if 0.0 <= l.opp[1] { -1.0 } else { 1.0 };
        let u = percent(roll);
        let away = if 0.0 < (F(l.opp[0]) * s).0 { 0 } else { 2 };
        if l.kind == contact::SMASH {
            if md == 0 && od == 0 && chance(roll, 50) {
                if let Some(stick) = body(l) {
                    return Aim { stick, plan: plan::SMASH };
                }
            }
            let mut d = match od {
                2 => (u < 10) as u8,
                1 if u < 40 => 2,
                1 => (u < 60) as u8,
                _ if u < 40 => 2,
                _ => 0,
            };
            if md == 2 && d == 0 {
                d = reroll(roll);
            }
            return Aim { stick: self.aim((away, d), l.side, roll), plan: plan::SMASH };
        }
        if chance(roll, self.special_return_rate) {
            let x = self.lane_away(ox, roll);
            return Aim { stick: self.aim((x, 2), l.side, roll), plan: plan::LOB };
        }
        let t = &RECEIVE_DEPTH[self.base_pick(roll) * 6..];
        let k = 2 * (2 - od as usize);
        let d = pick(u, t[k], t[k + 1]);
        let plan = match (od, d) {
            (0, 2) => plan::LOB,
            (0, _) => plan::TOPSPIN,
            (_, 0) => plan::SLICE,
            _ => plan::TOPSPIN,
        };
        Aim { stick: self.aim((away, d), l.side, roll), plan }
    }

    /// The aim in a rally (`short`: the AI's flag that its last pick was a short ball, cleared and maybe set again).
    pub fn rally_aim(&self, l: &Look, short: &mut bool, roll: &mut impl FnMut() -> u32) -> Aim {
        match l.kind {
            contact::VOLLEY => return self.volley_aim(l, roll),
            contact::REACH => return self.low_aim(l, roll),
            _ => {}
        }
        let was = std::mem::take(short);
        let w = width(l.singles);
        let (ox, od) = zone_in(l.opp, w, Some(&mut *roll));
        let (fine, _) = fine_zone(l.opp, w, Some(&mut *roll));
        let (_, md) = zone_in(l.me, w, None);
        let s = if 0.0 <= l.opp[1] { -1.0 } else { 1.0 };
        let u = percent(roll);
        let near = ((F(l.opp[1]) - l.mark[1]) * s).0 <= 1.5;
        let low = 0.2 <= if l.kind <= contact::REACH { (F(l.contact_y) - l.heights[1]).0 } else { 0.0 };
        let by_fine = if fine < 3 { 2 } else { 0 };
        let (x, d, plan) = if l.opp_kind == OPP_SMASH && l.kind != contact::DIVE {
            (by_fine, 2, plan::VOLLEY)
        } else {
            let x = match ox {
                0 => 2,
                2 => 0,
                _ if l.opp_lefty => 2,
                _ => 0,
            };
            match l.kind {
                contact::DIVE => match od {
                    0 => (by_fine, 2, plan::VOLLEY_LOB),
                    1 => (by_fine, 2, plan::VOLLEY_SLICE),
                    _ => (by_fine, 0, plan::VOLLEY_SLICE),
                },
                contact::SMASH => {
                    let mut d = if od != 0 { if u < 80 { 2 } else { 0 } } else { 2 };
                    if md == 2 && d == 0 {
                        d = reroll(roll);
                    }
                    (x, d, plan::SMASH)
                }
                _ if chance(roll, self.special_return_rate) => (self.lane_away(ox, roll), 2, plan::LOB),
                _ => {
                    let t = &RALLY_DEPTH[self.base_pick(roll) * 6..];
                    let soft = if low { plan::TOPSPIN } else { plan::LOB };
                    if od == 0 {
                        (x, pick(u, t[4], t[5]), soft)
                    } else if was {
                        let plan = if (2..=3).contains(&fine) && u <= 49 && !low { plan::LOB } else { plan::TOPSPIN };
                        (if fine > 2 { 0 } else { 2 }, 2, plan)
                    } else if near || u > 49 {
                        let k = if od == 2 { 0 } else { 2 };
                        match pick(u, t[k], t[k + 1]) {
                            0 => {
                                *short = true;
                                (x, 0, plan::SLICE)
                            }
                            d => (x, d, plan::TOPSPIN),
                        }
                    } else {
                        (x, 2, soft)
                    }
                }
            }
        };
        Aim { stick: self.aim((x, d), l.side, roll), plan }
    }

    /// The far lane from the opponent's (the middle: a coin).
    fn lane_away(&self, ox: u8, roll: &mut impl FnMut() -> u32) -> u8 {
        match ox {
            0 => 2,
            2 => 0,
            _ if chance(roll, 50) => 0,
            _ => 2,
        }
    }

    /// The base level from the row's mix.
    fn base_pick(&self, roll: &mut impl FnMut() -> u32) -> usize {
        let r = percent(roll);
        if r < self.base_level[0] {
            0
        } else if r < self.base_level[0] + self.base_level[1] {
            1
        } else {
            2
        }
    }

    /// The aim of a volley, by its level pick (3: a drop at the open lane, 2/1: deep, 0: at the kept spot).
    fn volley_aim(&self, l: &Look, roll: &mut impl FnMut() -> u32) -> Aim {
        let w = width(l.singles);
        let opp_in = on_court(l.opp, w);
        let (ox, od) = zone_in(l.opp, w, Some(&mut *roll));
        let (_, fd) = fine_zone(l.me, w, None);
        let (mx, md) = zone_in(l.me, w, None);
        let high = l.contact_y < l.heights[0];
        let (x, d, plan) = match l.volley_level {
            3 => (self.lane_away(ox, roll), 0, plan::VOLLEY_DROP),
            1 => (self.lane_away(ox, roll), 2, if fd > 2 && high { plan::VOLLEY_SLICE } else { plan::VOLLEY }),
            0 => {
                let (x, _) = zone_in(l.mark, w, Some(&mut *roll));
                if od == 0 && chance(roll, 30) {
                    (x, if chance(roll, VOLLEY_SHORT[l.level as usize & 3]) { 1 } else { 2 }, plan::VOLLEY_LOB)
                } else {
                    (x, 2, plan::VOLLEY)
                }
            }
            2 => {
                let x = self.lane_away(ox, roll);
                let d = if opp_in && od == 2 && chance(roll, 30) { 1 } else { 2 };
                // a NET player tries the drop more often than an ALL one
                let bold = |net: i32, all: i32, roll: &mut dyn FnMut() -> u32| {
                    let mut roll = || roll();
                    if self.style == 1 { chance(&mut roll, net) } else { self.style == 3 && chance(&mut roll, all) }
                };
                if fd < 3 && od == 0 {
                    if !matches!((mx, ox), (0, 2) | (2, 0) | (1, 1)) {
                        (x, d, plan::VOLLEY)
                    } else if chance(roll, 10) {
                        (x, 2, plan::VOLLEY_LOB)
                    } else if bold(10, 5, roll) {
                        (ox, 0, plan::VOLLEY_DROP)
                    } else {
                        (x, d, plan::VOLLEY)
                    }
                } else if fd > 2 && high {
                    (x, d, plan::VOLLEY_SLICE)
                } else if md == 0 && od == 2 && bold(30, 15, roll) {
                    (x, 0, plan::VOLLEY_DROP)
                } else {
                    (x, d, plan::VOLLEY)
                }
            }
            _ => (0, 0, 0),
        };
        Aim { stick: self.aim((x, d), l.side, roll), plan }
    }

    /// The aim of a reach contact (`contact::REACH`) in a rally, by its high-ball level pick.
    fn low_aim(&self, l: &Look, roll: &mut impl FnMut() -> u32) -> Aim {
        let w = width(l.singles);
        let (ox, _) = zone_in(l.opp, w, Some(&mut *roll));
        let (mx, _) = zone_in(l.me, w, None);
        if chance(roll, self.special_return_rate) {
            let x = self.lane_away(ox, roll);
            return Aim { stick: self.aim((x, 2), l.side, roll), plan: plan::LOB };
        }
        let (x, d) = match l.high_level {
            2 => (if ox == 1 { if chance(roll, 50) { 0 } else { 2 } } else { ox }, 2),
            1 => (if mx == 1 { if chance(roll, 50) { 0 } else { 2 } } else { mx }, 1),
            0 => (self.lane_away(ox, roll), 2),
            _ => (0, 0),
        };
        Aim { stick: self.aim((x, d), l.side, roll), plan: plan::TOPSPIN }
    }

    /// Whether the AI lets a ball go whose first bounce `at` (x, z) is out by at least its line margin: past the
    /// singles or doubles court in a rally, the service box (either half) for a serve.
    pub fn lets_go(&self, at: [f32; 2], serve: bool, singles: bool) -> bool {
        let (w, l) = if serve { (SINGLES_SIDELINE, SERVICE_LINE) } else { (width(singles), BASELINE) };
        let out = at[0].abs() > w || at[1].abs() > l;
        out && self.line_margin <= out_by(at, w, l)
    }
}

/// What a doubles AI reads when it picks its shot.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pair {
    /// Its own, its partner's and the two opponents' positions (x, z).
    pub me: [f32; 2],
    pub mate: [f32; 2],
    pub opp: [[f32; 2]; 2],
    /// Its side sign.
    pub side: f32,
    pub singles: bool,
    /// The contact it found (`contact`), its volley level pick and its level.
    pub kind: u8,
    pub volley_level: u8,
    pub level: u8,
    /// Its formation (`DEFENSIVE`: both back) and the smash-third flag.
    pub formation: u8,
    pub smash_third: bool,
}

/// The defensive doubles formation (both players back; `ai::AiParams::formation`).
const DEFENSIVE: u8 = 2;

/// The lanes (0..=2) neither opponent stands in, and how many.
fn open_lanes(a: u8, b: u8) -> ([u8; 3], usize) {
    let (mut out, mut n) = ([0; 3], 0);
    for k in 0..3 {
        if k != a && k != b {
            out[n] = k;
            n += 1;
        }
    }
    (out, n)
}

/// One of the open lanes, at random.
fn any_open(open: ([u8; 3], usize), roll: &mut impl FnMut() -> u32) -> u8 {
    open.0[(r15(roll()) as usize) % open.1]
}

/// Of three lanes, `a`, `b` or else `c` by two coin flips weighted `p`: `a` when the roll is under `p`.
fn lane_or(roll: &mut impl FnMut() -> u32, p: i32, a: u8, b: u8) -> u8 {
    if chance(roll, p) { a } else { b }
}

/// The unit direction from `from` to `to` on the ground (x, z), 1/√ as the game takes it.
fn toward(from: [f32; 2], to: [f32; 2]) -> [f32; 2] {
    let (dx, dz) = ((F(to[0]) - from[0]).0, (F(to[1]) - from[1]).0);
    let inv = (1.0 / (F(dz) * dz).madd(dx, dx).sqrt()).0;
    [(F(dx) * inv).0, (F(dz) * inv).0]
}

impl AiParams {
    /// A doubles AI's aim, by its contact: a volley, a kind-1 stroke, else a ground stroke (a smash then aimed
    /// again).
    pub fn pair_aim(&self, l: &Pair, roll: &mut impl FnMut() -> u32) -> Aim {
        match l.kind {
            2 => self.pair_volley(l, roll),
            1 => self.pair_low(l, roll),
            _ => self.pair_ground(l, roll),
        }
    }

    /// The stick at a point: straight from the player through it, out to the baseline or the side line, as a
    /// stick on the court (x over the half width, depth over the back court); a length from the row's 4-way
    /// mix in fifths and a turn within the angle width; z by the side.
    pub fn aim_at(&self, me: [f32; 2], at: [f32; 2], side: f32, singles: bool, roll: &mut impl FnMut() -> u32) -> [f32; 4] {
        let w = width(singles);
        let d = toward(me, at);
        let reach = ((F(BASELINE) + me[1].abs()) / d[1].abs()).0;
        let mut x = (F::ZERO + me[0]).madd(d[0], reach).0;
        let depth = if x.abs() <= w {
            BASELINE
        } else {
            x = if d[0] < 0.0 { (-F(w)).0 } else { w };
            ((F(x) - me[0]) / d[0] * d[1].abs()).0
        };
        let (x0, z0) = ((F(x) / w).0, ((F(depth) - 4.4425) / 4.4425).0);
        let inv = (1.0 / (F(z0) * z0).madd(x0, x0).sqrt()).0;
        let r = percent(roll);
        let k = &self.key_level;
        let f = |b: u32| f32::from_bits(b);
        let (lo, hi) = if r < k[0] {
            (f(0x3e4c_cccd), f(0x3ecc_cccd))
        } else if r < k[0] + k[1] {
            (f(0x3ecc_cccd), f(0x3f19_999a))
        } else if r < k[0] + k[1] + k[2] {
            (f(0x3f19_999a), f(0x3f4c_cccd))
        } else {
            (f(0x3f4c_cccd), 1.0)
        };
        let len = (F::ZERO + lo).madd(F(hi) - lo, uniform(roll)).0;
        let v = [(F(x0) * inv * len).0, 0.0, (F(z0) * inv * len).0, 0.0];
        let aw = self.angle_width;
        let mut rad = (DEG * (F::ZERO + -F(aw)).madd(F(aw) - -F(aw), uniform(roll))).0;
        if rad > world::PI {
            rad = (fmod(F(world::PI) + rad, world::TWO_PI) - world::PI).0;
        } else if rad < -F(world::PI) {
            rad = (F(world::PI) + fmod(F(rad) - world::PI, world::TWO_PI)).0;
        }
        let m = world::mat_mul(&world::IDENTITY, &world::rot_y(rad));
        let mut v = vu0::transform(&m, v);
        v[2] = (F(v[2]) * side).0;
        v
    }

    fn pair_volley(&self, l: &Pair, roll: &mut impl FnMut() -> u32) -> Aim {
        let w = width(l.singles);
        let z1 = zone_in(l.opp[0], w, Some(&mut *roll));
        let z2 = zone_in(l.opp[1], w, Some(&mut *roll));
        let (mx, _) = zone_in(l.me, w, None);
        let aim = |zone, roll: &mut dyn FnMut() -> u32| {
            let mut roll = || roll();
            self.aim(zone, l.side, &mut roll)
        };
        if l.volley_level == 3 {
            let x = any_open(open_lanes(z1.0, z2.0), roll);
            return Aim { stick: aim((x, 0), roll), plan: plan::VOLLEY_DROP };
        }
        let half = |a: f32, b: f32| ((F(a) + b) * 0.5).0;
        let mid = [half(l.opp[0][0], l.opp[1][0]), half(l.opp[0][1], l.opp[1][1])];
        let dir = toward(l.me, mid);
        // the line between the opponents, from the nearer to the farther one
        let (n, f) = if l.opp[0][1].abs() < l.opp[1][1].abs() { (0, 1) } else { (1, 0) };
        let (dx, dz) = ((F(l.opp[f][0]) - l.opp[n][0]).0, (F(l.opp[f][1]) - l.opp[n][1]).0);
        let len = (F(dz) * dz).madd(dx, dx).sqrt().0;
        let (ux, uz) = if 0.0 < len { ((dx * (F(1.0) / len)).0, (dz * (F(1.0) / len)).0) } else { (dx, dz) };
        let (ex, ez) = ((F(l.opp[0][0]) - l.opp[1][0]).0, (F(l.opp[0][1]) - l.opp[1][1]).0);
        let gap = (F(ez) * ez).madd(ex, ex).sqrt().0;
        let dot = (F(dir[0]) * ux).madd(dir[1], uz).0.abs();
        // its lane down the line is open past a deep opponent, or (from the middle) both side lanes are
        let line = mx ^ 2;
        let open = match mx {
            0 | 2 => (z1.0 != line || z1.1 != 0) && (z2.0 != line || z2.1 != 0),
            _ => (z1.0 != 0 || z1.1 != 0) && (z1.0 != 2 || z1.1 != 0) && (z2.0 != 0 || z2.1 != 0) && (z2.0 != 2 || z2.1 != 0),
        };
        if open && chance(roll, 30) {
            let x = if mx == 1 { lane_or(roll, 50, 0, 2) } else { line };
            return Aim { stick: aim((x, 0), roll), plan: plan::VOLLEY };
        }
        if dot < 0.8660254 && 2.5 <= gap && chance(roll, 60) {
            return Aim { stick: self.aim_at(l.me, mid, l.side, l.singles, roll), plan: plan::VOLLEY };
        }
        // the nearer opponent (to the net) and the other one
        let (near, far, zn, zf) = if l.opp[1][1].abs() < l.opp[0][1].abs() {
            (l.opp[1], l.opp[0], z2.0, z1.0)
        } else {
            (l.opp[0], l.opp[1], z1.0, z2.0)
        };
        if SERVICE_LINE < near[1].abs() {
            let x = any_open(open_lanes(z1.0, z2.0), roll);
            let d = if x == mx && mx != 1 { 0 } else { (!chance(roll, 80)) as u8 };
            let stick = aim((x, d), roll);
            let plan = if d == 0 && chance(roll, 10) { plan::VOLLEY_SLICE } else { plan::VOLLEY };
            return Aim { stick, plan };
        }
        let top = if far[1].abs() <= SERVICE_LINE { 70 } else { 90 };
        let u = percent(roll);
        if u < 60 {
            return Aim { stick: self.aim_at(l.me, near, l.side, l.singles, roll), plan: plan::VOLLEY };
        }
        if top <= u {
            let x = if zn != zf {
                zn
            } else {
                match zf {
                    1 => lane_or(roll, 50, 0, 2),
                    0 => lane_or(roll, 50, 1, 2),
                    _ => lane_or(roll, 50, 0, 1),
                }
            };
            return Aim { stick: aim((x, 2), roll), plan: plan::VOLLEY_LOB };
        }
        let x = match (mx, zn) {
            (1, 1) => lane_or(roll, 50, 2, 0),
            (1, 0) => lane_or(roll, 90, 2, 1),
            (1, _) => lane_or(roll, 90, 0, 1),
            (0, 1) => lane_or(roll, 90, 2, 0),
            (0, 0) => lane_or(roll, 70, 2, 1),
            (0, _) => lane_or(roll, 80, 0, 1),
            (_, 1) => lane_or(roll, 90, 0, 2),
            (_, 0) => lane_or(roll, 70, 2, 1),
            _ => lane_or(roll, 80, 0, 1),
        };
        let d = (!chance(roll, 80)) as u8;
        let stick = aim((x, d), roll);
        let plan = if d == 0 && chance(roll, 10) { plan::VOLLEY_SLICE } else { plan::VOLLEY };
        Aim { stick, plan }
    }

    fn pair_low(&self, l: &Pair, roll: &mut impl FnMut() -> u32) -> Aim {
        let w = width(l.singles);
        // the nearer opponent's lane first
        let (a, b) = if l.opp[0][1].abs() < l.opp[1][1].abs() { (l.opp[0], l.opp[1]) } else { (l.opp[1], l.opp[0]) };
        let (n, _) = zone_in(a, w, Some(&mut *roll));
        let (f, _) = zone_in(b, w, Some(&mut *roll));
        let (mx, md) = zone_in(l.me, w, None);
        let open = open_lanes(n, f);
        let free = |k: u8| k != n && k != f;
        let lvl = l.level as i32;
        let line = 2 - mx;
        // a lane next to the near opponent's, skipping the line when `skip`
        let beside = |roll: &mut dyn FnMut() -> u32, skip: bool| {
            let i = (n as i32 + 1) % 3;
            let pair = [(i % 3) as u8, ((i + 1) % 3) as u8];
            let k = coin(roll()) as usize;
            if skip && pair[k] == line { pair[(k + 1) & 1] } else { pair[k] }
        };
        let mut r = || roll();
        let (x, d, plan) = if l.formation == DEFENSIVE && !l.smash_third {
            if free(line) && chance(&mut r, lvl * 15 + 30) {
                (line, 2, plan::TOPSPIN)
            } else if chance(&mut r, lvl * 10 + 30) {
                let i = (f as i32 + 1) % 3;
                let pair = [(i % 3) as u8, ((i + 1) % 3) as u8];
                (pair[coin(r()) as usize], 2, plan::LOB)
            } else if chance(&mut r, lvl * 10 + 30) {
                (any_open(open, &mut r), 2, plan::TOPSPIN)
            } else if chance(&mut r, lvl * 10 + 30) {
                let x = beside(&mut r, true);
                (x, 2, if chance(&mut r, 60) { plan::TOPSPIN } else { plan::SLICE })
            } else {
                let x = (r15(r()) % 3) as u8;
                let u = percent(&mut r);
                let d = if u < 40 { 0 } else if u < 80 { 2 } else { 1 };
                (x, d, if chance(&mut r, 50) { plan::TOPSPIN } else { plan::SLICE })
            }
        } else if md != 2 {
            if free(line) && chance(&mut r, lvl * 10 + 30) {
                (line, 2, plan::TOPSPIN)
            } else if chance(&mut r, lvl * 15 + 30) {
                let x = beside(&mut r, true);
                (x, (!chance(&mut r, lvl * 5 + 55)) as u8, plan::TOPSPIN)
            } else {
                let x = (r15(r()) % 3) as u8;
                let u = percent(&mut r);
                let d = if u < 40 { 0 } else if u < 80 { 2 } else { 1 };
                (x, d, if chance(&mut r, 60) { plan::TOPSPIN } else { plan::SLICE })
            }
        } else if free(line) && chance(&mut r, lvl * 15 + 30) {
            (line, 2, plan::TOPSPIN)
        } else if chance(&mut r, lvl * 10 + 30) {
            (any_open(open, &mut r), 2, plan::TOPSPIN)
        } else if chance(&mut r, lvl * 10 + 30) {
            (beside(&mut r, false), 2, plan::TOPSPIN)
        } else {
            let x = (r15(r()) % 3) as u8;
            (x, 2, if chance(&mut r, 60) { plan::TOPSPIN } else { plan::SLICE })
        };
        Aim { stick: self.aim((x, d), l.side, roll), plan }
    }

    fn pair_ground(&self, l: &Pair, roll: &mut impl FnMut() -> u32) -> Aim {
        let w = width(l.singles);
        let (mx, md) = zone_in(l.me, w, None);
        let mate_in = on_court(l.mate, w);
        let me_in = on_court(l.me, w);
        let u = percent(roll);
        let (x, mut d, mut plan);
        if l.formation == DEFENSIVE && !l.smash_third {
            // the nearer opponent (to the net) and the other one, both zones blurred
            let (near, far) = if l.opp[1][1].abs() < l.opp[0][1].abs() { (l.opp[1], l.opp[0]) } else { (l.opp[0], l.opp[1]) };
            let (zn, _) = zone_in(near, w, Some(&mut *roll));
            let (zf, _) = zone_in(far, w, Some(&mut *roll));
            let cross = 2 - mx;
            let avoid = |k: u8| zn != k && zf != k;
            if avoid(cross) {
                x = cross;
                d = 2;
                plan = plan::TOPSPIN;
            } else if near[1].abs() <= SERVICE_LINE && !(SERVICE_LINE < far[1].abs() && chance(roll, 30)) {
                x = match zf {
                    0 => lane_or(roll, 50, 1, 2),
                    1 => lane_or(roll, 50, 0, 2),
                    _ => (!chance(roll, 50)) as u8,
                };
                d = 2;
                plan = plan::LOB;
            } else if near[1].abs() <= SERVICE_LINE {
                x = match zn {
                    0 => 2,
                    2 => 0,
                    _ => lane_or(roll, 50, 0, 2),
                };
                d = (!chance(roll, 70)) as u8;
                plan = plan::TOPSPIN;
            } else {
                x = if mx == 1 { lane_or(roll, 50, 0, 2) } else { mx };
                d = (!chance(roll, 80)) as u8;
                plan = plan::TOPSPIN;
            }
        } else if l.formation == 0 && u < 30 && !l.smash_third {
            let (a, _) = zone_in(l.opp[0], w, Some(&mut *roll));
            let (b, _) = zone_in(l.opp[1], w, Some(&mut *roll));
            x = any_open(open_lanes(a, b), roll);
            return Aim { stick: self.aim((x, 2), l.side, roll), plan: plan::LOB };
        } else {
            x = match mx {
                0 => if u < 70 { 0 } else { 2 },
                2 => if u < 70 { 2 } else { 0 },
                _ => {
                    lane_or(roll, 50, 0, 2)
                }
            };
            d = 2;
            plan = plan::TOPSPIN;
            let base = if u < 70 { 20 } else { 30 };
            let p = if me_in { base } else { base + 10 } + if mate_in { 0 } else { 10 };
            if chance(roll, p) {
                plan = plan::SLICE;
            }
        }
        let mut stick = self.aim((x, d), l.side, roll);
        if l.kind == contact::SMASH {
            if md == 2 && d == 0 {
                match percent(roll) {
                    u if u < 50 => d = 1,
                    u if u < 70 => d = 2,
                    _ => {}
                }
            }
            plan = plan::SMASH;
            stick = self.aim((x, d), l.side, roll);
        }
        Aim { stick, plan }
    }
}

/// How far a bounce lies past the nearer of the side line (half width `w`) and the end line (`l` from the net),
/// negative inside: the game's measure.
pub fn out_by(at: [f32; 2], w: f32, l: f32) -> f32 {
    let (x, z) = ((F(at[0].abs()) - w).0, (F(at[1].abs()) - l).0);
    if z.abs() <= x.abs() { z } else { x }
}

/// A smash at the opponent's body: their spot (pulled out along the line from the AI to 3 m from the net when
/// they stand closer), as a stick on the singles court (x over the half width, depth past the service line over
/// the back court), unit length or shorter. None when that x is wide of the singles court.
fn body(l: &Look) -> Option<[f32; 4]> {
    let (mut x, mut z) = (l.opp[0], l.opp[1]);
    let az = l.opp[1].abs();
    if az < 3.0 {
        let gap = (F(3.0) - az).0;
        let dx = (gap * (F(l.opp[0]) - l.me[0]) / (F(l.opp[1]) - l.me[1])).0.abs();
        let sx = if l.opp[0] < 0.0 { -1.0 } else { 1.0 };
        x = ((F(dx) + l.opp[0].abs()) * sx).0;
        z = (F(gap) * (if l.opp[1] < 0.0 { -1.0 } else { 1.0 })).0;
    }
    if x.abs() > SINGLES_SIDELINE {
        return None;
    }
    let a8 = ((F(z.abs()) - 7.4425) / 4.4425).0;
    let a0 = (F(x) / SINGLES_SIDELINE).0;
    let m = if a0.abs() <= a8.abs() { a8.abs() } else { a0.abs() };
    let inv = (1.0 / (F(a8) * a8).madd(a0, a0).sqrt()).0;
    let mut v = [(F(a0) * inv).0, 0.0, (F(a8) * inv).0, 0.0];
    if m < 1.0 {
        v = v.map(|c| (F(c) * m).0);
    }
    v[2] = (F(v[2]) * (if z < 0.0 { -1.0 } else { 1.0 })).0;
    Some(v)
}

/// A short pick turned mid (50%) or deep (20%) when the AI itself stands deep.
fn reroll(roll: &mut impl FnMut() -> u32) -> u8 {
    match percent(roll) {
        u if u < 50 => 1,
        u if u < 70 => 2,
        _ => 0,
    }
}

/// The button (`button`) an aim's plan (`plan`, the serve's too) presses.
pub fn button(plan: u8) -> u8 {
    use button::{CIRCLE as O, CROSS as X, TRIANGLE as T};
    const BUTTONS: [u8; 19] = [O, X, T, X, O, X, T, X, O, X, T, O, X, O, X, T, O, X, T];
    BUTTONS.get(plan as usize).copied().unwrap_or(X)
}

/// A stick value through the pad's byte (1..=255 about 0x80) and back, unsigned.
fn pad_abs(x: f32) -> f32 {
    let half = if x < 0.0 { -0.5 } else { 0.5 };
    let b = ((F(x) * 127.0 + half).0 as i32 + 0x80).clamp(1, 255);
    (F((b - 0x80) as f32) / 127.0).0.abs()
}

/// Snapped down to the pad's 1/127 steps.
fn pad_step(v: f32) -> f32 {
    (F(((F(v) / 0.007874016).0 as i32) as f32) * 0.007874016).0
}

impl AiParams {
    /// Singles kind lock, on the button the AI presses: △ may turn ✕ (`kind_lock[2]`), ✕ may turn ○ (`[3]`, also
    /// right after △ turned ✕), else ○ may turn ✕ (`[4]`). Each chance is drawn only when its button is held.
    pub fn lock_button(&self, button: u8, roll: &mut impl FnMut() -> u32) -> u8 {
        let mut b = button;
        if b == button::TRIANGLE && chance(roll, self.kind_lock[2]) {
            b = button::CROSS;
        }
        if b == button::CROSS {
            if chance(roll, self.kind_lock[3]) {
                b = button::CIRCLE;
            }
        } else if b == button::CIRCLE && chance(roll, self.kind_lock[4]) {
            b = button::CROSS;
        }
        b
    }

    /// Singles kind lock on the stick, the frame before the contact, for the button kept from the press: a slice
    /// (○) pulled back far enough for a drop shot (`shot::stick_kind`) is, `kind_lock[1]` % of the time, bent
    /// short of it; a topspin (✕) pushed far enough for a flat shot likewise by `kind_lock[0]`. `side` is the
    /// player's end. Only z changes (0 when x is 0).
    pub fn lock_stick(&self, kept: u8, stick: [f32; 4], side: f32, roll: &mut impl FnMut() -> u32) -> [f32; 4] {
        let [x, y, z, w] = stick;
        let len = (F(x) * x + F(z) * z).sqrt().0;
        let along = |z: f32| (F(side) * z * (F(1.0) / len)).0;
        let bend = |k: f32| if x == 0.0 { 0.0 } else { pad_step((F(side) * pad_abs(x) * k).0) };
        let z = if len <= 0.0 {
            z
        } else if kept == button::CIRCLE && -F(along(z)) >= std::f32::consts::FRAC_1_SQRT_2 && chance(roll, self.kind_lock[1]) {
            bend(-0.96568877)
        } else if kept == button::CROSS && along(z) >= 0.5 && chance(roll, self.kind_lock[0]) {
            bend(0.55430907)
        } else {
            z
        };
        [x, y, z, w]
    }

    /// The serve's toss plan: strong on a first serve; on a second serve strong `second_rate` % of the time (the
    /// character's), else weak; at serve level 0 one in ten tosses underhand instead.
    pub fn serve_toss(second: bool, second_rate: i32, level: u8, roll: &mut impl FnMut() -> u32) -> u8 {
        let toss = if !second || chance(roll, second_rate) { plan::TOSS_STRONG } else { plan::TOSS_WEAK };
        if level == 0 && chance(roll, 10) { plan::TOSS_UNDER } else { toss }
    }

    /// The serve's swing plan, drawn as it presses: △ for an underhand toss, else ✕ `serve_kind[1]` % of the time,
    /// ○ otherwise.
    pub fn serve_swing(&self, toss: u8, roll: &mut impl FnMut() -> u32) -> u8 {
        if toss == plan::TOSS_UNDER {
            plan::SWING_UNDER
        } else if chance(roll, self.serve_kind[1]) {
            plan::SWING_CROSS
        } else {
            plan::SWING_CIRCLE
        }
    }

    /// The serve's stick, the frame before the contact, by serve level and swing plan: level 0 the middle; a ✕
    /// swing goes for the wide corner `serve_kind[0]` % of the time (ad court: the other side), else level 1 picks
    /// a near-middle lane and level 2 any lane and depth; other swings aim at the side away from the hand (`lefty`)
    /// at level 1, any depth at level 2. A non-wide middle-depth aim never points back (z toward `side`).
    pub fn serve_aim(&self, level: u8, swing: u8, ad: bool, lefty: bool, side: f32, roll: &mut impl FnMut() -> u32) -> [f32; 4] {
        let (zone, wide, _) = self.serve_zone(level, swing, ad, lefty, roll);
        self.serve_stick(zone, wide, side, roll)
    }

    /// The singles serve's stick (`serve_aim`'s zones) and the net dash it may set off: at level 2 a serve down
    /// the lane on the receiver's forehand side of the T (a wide ✕ from the deuce court's left lane or the ad
    /// court's right, another swing from the deuce court's right lane or the ad's left) sends a NET player (ALL: 80 %)
    /// in toward the net at `reach` depth, x 0..1.37 m (✕) or 1.37..2.74 m (else) to the serve's side; then any
    /// singles serve dashes `net_dash_rate` % of the time. Returns the stick, whether it dashes and the drawn
    /// dash spot (None: the plain net spot).
    #[allow(clippy::too_many_arguments)]
    pub fn serve_aim_singles(
        &self,
        level: u8,
        swing: u8,
        ad: bool,
        lefty: bool,
        side: f32,
        reach: f32,
        roll: &mut impl FnMut() -> u32,
    ) -> ([f32; 4], bool, Option<[f32; 2]>) {
        let (zone, wide, follow) = self.serve_zone(level, swing, ad, lefty, roll);
        let mut spot = None;
        if follow && (self.style == 1 || self.style == 3 && chance(roll, 80)) {
            let x = F(if swing == plan::SWING_CROSS { 0.0 } else { 1.3716666 }).madd(1.3716666, F(TWO_POW_M32) * utof(roll())).0;
            spot = Some([(F(x) * (if ad { 1.0 } else { -1.0 }) * side).0, (-F(reach) * side).0]);
        }
        let dash = chance(roll, self.net_dash_rate) || spot.is_some();
        (self.serve_stick(zone, wide, side, roll), dash, spot)
    }

    /// The serve's zone, whether it is the wide one, and whether a singles net player follows it in.
    fn serve_zone(&self, level: u8, swing: u8, ad: bool, lefty: bool, roll: &mut impl FnMut() -> u32) -> ((u8, u8), bool, bool) {
        let away = if lefty { 0 } else { 2 };
        let draw = |roll: &mut dyn FnMut() -> u32| r15(roll());
        let ((x, z), wide) = match (level, swing == plan::SWING_CROSS) {
            (0, _) => ((1, 1), false),
            (1, true) if chance(roll, self.serve_kind[0]) => ((if ad { 2 } else { 0 }, 2), true),
            (1, true) => ((if ad { (!chance(roll, 50)) as u8 } else { 2 - chance(roll, 50) as u8 }, 1), false),
            (1, false) => ((away, 1), false),
            (_, true) if chance(roll, self.serve_kind[0]) => (((draw(roll) % 3) as u8, 2), true),
            (_, true) => {
                let x = (draw(roll) % 3) as u8;
                ((x, (draw(roll) & 1) as u8), false)
            }
            _ => ((away, (draw(roll) % 3) as u8), false),
        };
        let follow = level >= 2 && (wide && x == if ad { 2 } else { 0 } || swing != plan::SWING_CROSS && x == if ad { 0 } else { 2 });
        ((x, z), wide, follow)
    }

    fn serve_stick(&self, zone: (u8, u8), wide: bool, side: f32, roll: &mut impl FnMut() -> u32) -> [f32; 4] {
        let mut s = self.aim(zone, side, roll);
        if !wide && zone.1 == 1 && (F(s[2]) * side).0 > 0.0 {
            s[2] = 0.0;
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticks_point_at_the_far_court() {
        let row = AiParams { angle_width: 10.0, key_level: [0, 0, 0, 100], ..Default::default() };
        let mut n = 0u32;
        let mut roll = || {
            n = n.wrapping_mul(1664525).wrapping_add(1013904223);
            n
        };
        // straight on, deep: forward for the −z end (side +1) is −z, full length
        let v = row.aim((1, 0), 1.0, &mut roll);
        assert!(v[2] < -0.8 && v[0].abs() < 0.18, "{v:?}");
        let v = row.aim((1, 2), -1.0, &mut roll);
        assert!(v[2] < -0.8, "{v:?}");
        assert_eq!(fine_zone([0.1, -5.0], 4.115, None), (3, 2));
        assert_eq!(fine_zone([-4.0, 11.0], 4.115, None), (5, 5));
        let row = AiParams { line_margin: 0.5, ..Default::default() };
        assert!(row.lets_go([0.0, 12.5], false, true) && !row.lets_go([0.0, 12.2], false, true));
        assert!(!row.lets_go([5.0, 3.0], false, false) && row.lets_go([5.0, 3.0], false, true));
        assert!(row.lets_go([1.0, 7.0], true, false) && !row.lets_go([1.0, 7.0], false, false));
    }

    #[test]
    fn plans_press_and_lock() {
        assert_eq!([0, 1, 2, 6, 0x11, 30].map(button), [2, 1, 4, 4, 1, 1]);
        let row = AiParams { kind_lock: [100, 100, 100, 100, 0], ..Default::default() };
        let mut zero = || 0u32;
        // △ → ✕ → ○ chained; ○ stays with its chance 0
        assert_eq!(row.lock_button(4, &mut zero), 2);
        assert_eq!(row.lock_button(2, &mut zero), 2);
        // a flat-shot stick (toward the far end of side +1) is bent below the flat line; a drop stick likewise
        let s = row.lock_stick(1, [0.3, 0.0, 0.9, 0.0], 1.0, &mut zero);
        assert_eq!(crate::shot::stick_kind(1, 0, [s[0], s[2]], 1.0), 0, "{s:?}");
        let s = row.lock_stick(2, [-0.3, 0.0, -0.9, 0.0], 1.0, &mut zero);
        assert_eq!(crate::shot::stick_kind(1, 1, [s[0], s[2]], 1.0), 1, "{s:?}");
        assert_eq!(row.lock_stick(2, [0.0, 0.0, -0.9, 0.0], 1.0, &mut zero)[2], 0.0);
        // serves: first serve strong, a level-0 roll under 10 tosses underhand (△ swing)
        assert_eq!(AiParams::serve_toss(false, 0, 1, &mut zero), 1);
        assert_eq!(AiParams::serve_toss(true, 0, 1, &mut || 99 << 16), 0);
        assert_eq!(AiParams::serve_toss(true, 100, 0, &mut zero), 2);
        let row = AiParams { serve_kind: [0, 100, 0], key_level: [0, 0, 0, 100], angle_width: 5.0, ..Default::default() };
        assert_eq!((row.serve_swing(2, &mut zero), row.serve_swing(1, &mut zero)), (6, 3));
        let s = row.serve_aim(1, 4, false, false, 1.0, &mut zero);
        assert!(s[0].abs() > 0.5 && s[2] <= 0.0, "{s:?}");
    }
}
