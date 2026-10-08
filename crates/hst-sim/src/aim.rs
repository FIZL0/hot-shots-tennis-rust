//! Where a computer player puts its shot: the aim it chooses when it has found its contact point, as the stick
//! a human would hold (world x and z, length up to 1, turned by its side), and the shot plan that goes with it.
//! Drawn from the AIParam row's angle width, length mix (`key_level`), special return rate and base level mix,
//! and from where both players stand (`position::zone_in`, the finer `fine_zone`). Floats on the FPU (`ps2`), the
//! turn on VU0 (`world`).

use crate::ai::AiParams;
use crate::position::zone_in;
use crate::{ps2, vu0, world};

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
    /// The contact it found: 0/1 ground stroke, 2 volley, 3 smash, 4 dive.
    pub kind: u8,
    /// The opponent's last shot kind (3 smash).
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
    ((roll() >> 16 & 0x7fff) % 100) as i32
}

fn chance(roll: &mut impl FnMut() -> u32, p: i32) -> bool {
    percent(roll) < p
}

/// The MT word as a fraction of 2³².
fn uniform(roll: &mut impl FnMut() -> u32) -> f32 {
    ps2::mul(2.3283064e-10, ps2::utof(roll()))
}

fn width(singles: bool) -> f32 {
    if singles { 4.115 } else { 5.485 }
}

/// Whether a point is on the court (within the baseline and the half width).
fn on_court(p: [f32; 2], w: f32) -> bool {
    p[1].abs() <= 11.885 && p[0].abs() <= w
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
    let d = ps2::sub(p[1].abs(), 1.5);
    let Some(roll) = blur else {
        let out = 0.0 < ps2::mul(p[0], s);
        let col = if ax <= ps2::div(w, 3.0) {
            if out { 3 } else { 2 }
        } else if ax <= ps2::div(ps2::mul(w, 2.0), 3.0) {
            if out { 4 } else { 1 }
        } else if out {
            5
        } else {
            0
        };
        let band = [1.7308334, 3.4616668, 5.1925, 6.9233336, 8.654167].iter().take_while(|&&b| b < d).count();
        return (col, band as u8);
    };
    let mut roll = || ps2::madd(ps2::add(0.0, 0.0), 1.0, ps2::mul(2.3283064e-10, ps2::utof(roll())));
    let q = ps2::div(ps2::div(ps2::mul(w, 2.0), 6.0), 4.0);
    let x = ps2::madd(ps2::add(0.0, w), p[0], s);
    let mut col = 5;
    for k in 1..=5u8 {
        let edge = ps2::div(ps2::mul(w, k as f32), 3.0);
        let lo = ps2::sub(edge, q);
        if x <= lo {
            col = k - 1;
            break;
        }
        if x < ps2::add(edge, q) {
            col = if roll() < ps2::div(ps2::sub(x, lo), ps2::mul(2.0, q)) { k } else { k - 1 };
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
            band = k + (roll() < ps2::div(ps2::sub(d, lo), 0.8654167)) as u8;
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
            (ps2::madd(ps2::add(0.0, -180.0), 360.0, uniform(roll)), 0.0, f32::from_bits(0x3eaa_aaab))
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
            let angle = ps2::add(base, ps2::madd(ps2::add(0.0, -aw), ps2::sub(aw, -aw), uniform(roll)));
            let r = percent(roll);
            let k = &self.key_level;
            let (third, two) = (f32::from_bits(0x3eaa_aaab), f32::from_bits(0x3f2a_aaab));
            let five = f32::from_bits(0x3f55_5555);
            let band = if r < k[0] {
                (third, 0.5)
            } else if r < k[0] + k[1] {
                (0.5, two)
            } else if r < k[0] + k[1] + k[2] {
                (two, five)
            } else {
                (five, 1.0)
            };
            (angle, band.0, band.1)
        };
        let mut rad = ps2::mul(f32::from_bits(0x3c8e_fa35), angle);
        if rad > world::PI {
            rad = ps2::sub(ps2::add(world::PI, rad) % world::TWO_PI, world::PI);
        } else if rad < -world::PI {
            rad = ps2::add(world::PI, ps2::sub(rad, world::PI) % world::TWO_PI);
        }
        let m = world::mat_mul(&world::IDENTITY, &world::rot_y(rad));
        let v = vu0::transform(&m, [0.0, 0.0, 1.0, 0.0]);
        let len = ps2::madd(ps2::add(0.0, lo), ps2::sub(hi, lo), uniform(roll));
        v.map(|c| ps2::mul(ps2::mul(c, -side), len))
    }

    /// The aim at a return of serve (`kind` ≠ 2; a volleyed return takes the volley's choice).
    pub fn receive_aim(&self, l: &Look, roll: &mut impl FnMut() -> u32) -> Aim {
        if l.kind == 2 {
            return self.volley_aim(l, roll);
        }
        let w = width(l.singles);
        let (ox, od) = zone_in(l.opp, w, Some(&mut *roll));
        let (_, md) = zone_in(l.me, w, None);
        let s = if 0.0 <= l.opp[1] { -1.0 } else { 1.0 };
        let u = percent(roll);
        let away = if 0.0 < ps2::mul(l.opp[0], s) { 0 } else { 2 };
        if l.kind == 3 {
            if md == 0 && od == 0 && chance(roll, 50) {
                if let Some(stick) = body(l) {
                    return Aim { stick, plan: 0x11 };
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
            return Aim { stick: self.aim((away, d), l.side, roll), plan: 0x11 };
        }
        if chance(roll, self.special_return_rate) {
            let x = self.lane_away(ox, roll);
            return Aim { stick: self.aim((x, 2), l.side, roll), plan: 10 };
        }
        let t = &RECEIVE_DEPTH[self.base_pick(roll) * 6..];
        let k = 2 * (2 - od as usize);
        let d = pick(u, t[k], t[k + 1]);
        let plan = match (od, d) {
            (0, 2) => 10,
            (0, _) => 7,
            (_, 0) => 8,
            _ => 7,
        };
        Aim { stick: self.aim((away, d), l.side, roll), plan }
    }

    /// The aim in a rally (`short`: the AI's flag that its last pick was a short ball, cleared and maybe set again).
    pub fn rally_aim(&self, l: &Look, short: &mut bool, roll: &mut impl FnMut() -> u32) -> Aim {
        match l.kind {
            2 => return self.volley_aim(l, roll),
            1 => return self.low_aim(l, roll),
            _ => {}
        }
        let was = std::mem::take(short);
        let w = width(l.singles);
        let (ox, od) = zone_in(l.opp, w, Some(&mut *roll));
        let (fine, _) = fine_zone(l.opp, w, Some(&mut *roll));
        let (_, md) = zone_in(l.me, w, None);
        let s = if 0.0 <= l.opp[1] { -1.0 } else { 1.0 };
        let u = percent(roll);
        let near = ps2::mul(ps2::sub(l.opp[1], l.mark[1]), s) <= 1.5;
        let low = 0.2 <= if l.kind < 2 { ps2::sub(l.contact_y, l.heights[1]) } else { 0.0 };
        let by_fine = if fine < 3 { 2 } else { 0 };
        let (x, d, plan) = if l.opp_kind == 3 && l.kind != 4 {
            (by_fine, 2, 0xc)
        } else {
            let x = match ox {
                0 => 2,
                2 => 0,
                _ if l.opp_lefty => 2,
                _ => 0,
            };
            match l.kind {
                4 => match od {
                    0 => (by_fine, 2, 0xf),
                    1 => (by_fine, 2, 0xd),
                    _ => (by_fine, 0, 0xd),
                },
                3 => {
                    let mut d = if od != 0 { if u < 80 { 2 } else { 0 } } else { 2 };
                    if md == 2 && d == 0 {
                        d = reroll(roll);
                    }
                    (x, d, 0x11)
                }
                _ if chance(roll, self.special_return_rate) => (self.lane_away(ox, roll), 2, 10),
                _ => {
                    let t = &RALLY_DEPTH[self.base_pick(roll) * 6..];
                    let soft = if low { 7 } else { 10 };
                    if od == 0 {
                        (x, pick(u, t[4], t[5]), soft)
                    } else if was {
                        let plan = if (2..=3).contains(&fine) && u <= 49 && !low { 10 } else { 7 };
                        (if fine > 2 { 0 } else { 2 }, 2, plan)
                    } else if near || u > 49 {
                        let k = if od == 2 { 0 } else { 2 };
                        match pick(u, t[k], t[k + 1]) {
                            0 => {
                                *short = true;
                                (x, 0, 8)
                            }
                            d => (x, d, 7),
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
            3 => (self.lane_away(ox, roll), 0, 0x10),
            1 => (self.lane_away(ox, roll), 2, if fd > 2 && high { 0xd } else { 0xc }),
            0 => {
                let (x, _) = zone_in(l.mark, w, Some(&mut *roll));
                if od == 0 && chance(roll, 30) {
                    (x, if chance(roll, VOLLEY_SHORT[l.level as usize & 3]) { 1 } else { 2 }, 0xf)
                } else {
                    (x, 2, 0xc)
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
                        (x, d, 0xc)
                    } else if chance(roll, 10) {
                        (x, 2, 0xf)
                    } else if bold(10, 5, roll) {
                        (ox, 0, 0x10)
                    } else {
                        (x, d, 0xc)
                    }
                } else if fd > 2 && high {
                    (x, d, 0xd)
                } else if md == 0 && od == 2 && bold(30, 15, roll) {
                    (x, 0, 0x10)
                } else {
                    (x, d, 0xc)
                }
            }
            _ => (0, 0, 0),
        };
        Aim { stick: self.aim((x, d), l.side, roll), plan }
    }

    /// The aim of a kind-1 contact in a rally, by its high-ball level pick.
    fn low_aim(&self, l: &Look, roll: &mut impl FnMut() -> u32) -> Aim {
        let w = width(l.singles);
        let (ox, _) = zone_in(l.opp, w, Some(&mut *roll));
        let (mx, _) = zone_in(l.me, w, None);
        if chance(roll, self.special_return_rate) {
            let x = self.lane_away(ox, roll);
            return Aim { stick: self.aim((x, 2), l.side, roll), plan: 10 };
        }
        let (x, d) = match l.high_level {
            2 => (if ox == 1 { if chance(roll, 50) { 0 } else { 2 } } else { ox }, 2),
            1 => (if mx == 1 { if chance(roll, 50) { 0 } else { 2 } } else { mx }, 1),
            0 => (self.lane_away(ox, roll), 2),
            _ => (0, 0),
        };
        Aim { stick: self.aim((x, d), l.side, roll), plan: 7 }
    }

    /// Whether the AI lets a ball go that lands out (`out`: the ball's call) by `by` metres: at least its margin.
    pub fn lets_go(&self, out: bool, by: f32) -> bool {
        out && self.line_margin <= by
    }
}

/// A smash at the opponent's body: their spot (pulled out along the line from the AI to 3 m from the net when
/// they stand closer), as a stick on the singles court (x over the half width, depth past the service line over
/// the back court), unit length or shorter. None when that x is wide of the singles court.
fn body(l: &Look) -> Option<[f32; 4]> {
    let (mut x, mut z) = (l.opp[0], l.opp[1]);
    let az = l.opp[1].abs();
    if az < 3.0 {
        let gap = ps2::sub(3.0, az);
        let dx = ps2::div(ps2::mul(gap, ps2::sub(l.opp[0], l.me[0])), ps2::sub(l.opp[1], l.me[1])).abs();
        let sx = if l.opp[0] < 0.0 { -1.0 } else { 1.0 };
        x = ps2::mul(ps2::add(dx, l.opp[0].abs()), sx);
        z = ps2::mul(gap, if l.opp[1] < 0.0 { -1.0 } else { 1.0 });
    }
    if x.abs() > 4.115 {
        return None;
    }
    let a8 = ps2::div(ps2::sub(z.abs(), 7.4425), 4.4425);
    let a0 = ps2::div(x, 4.115);
    let m = if a0.abs() <= a8.abs() { a8.abs() } else { a0.abs() };
    let inv = ps2::div(1.0, ps2::sqrt(ps2::madd(ps2::mul(a8, a8), a0, a0)));
    let mut v = [ps2::mul(a0, inv), 0.0, ps2::mul(a8, inv), 0.0];
    if m < 1.0 {
        v = v.map(|c| ps2::mul(c, m));
    }
    v[2] = ps2::mul(v[2], if z < 0.0 { -1.0 } else { 1.0 });
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
    }
}
