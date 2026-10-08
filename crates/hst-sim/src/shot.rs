//! Shot creation from the precomputed trajectory tables (`TRAJ/tr_pcNN_<type>.dat`).
//!
//! A table is a 16×16×16 grid of u32 cells: bits 0–11 signed elevation in π/4096 rad, bits 12–23 signed
//! speed in 1/1024 m/frame, top byte flight frames. Axes, for a hitter mirrored onto the z < 0 half:
//! x = distance from the hitter to where the shot line crosses the net, y = hit height, z = distance from
//! the net crossing to the target. The game interpolates trilinearly in f32; this keeps its order.

use crate::ball::V3;
use crate::libm::cosf;
use crate::world::{self, M4};
use crate::{ps2, vu0};

const N: usize = 16;
const ANGLE_UNIT: f32 = 0.0007669904; // π / 4096
const SPEED_UNIT: f32 = 0.0009765625; // 1 / 1024
/// The game aims 15 cm short of the requested target.
const TARGET_PULL: f32 = -0.15;

pub struct Table(Vec<u32>);

impl Table {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        (bytes.len() == N * N * N * 4).then(|| Table(bytes.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()))
    }

    /// (elevation, speed, frames) of one cell. At an axis maximum the game reads one cell past the grid
    /// (whatever follows in memory); ponytail: clamped to the last cell, revisit if a capture ever hits it.
    fn cell(&self, i: usize) -> (f32, f32, i32) {
        let w = self.0[i.min(self.0.len() - 1)];
        let sext = |v: u32| ((v << 20) as i32 >> 20) as f32;
        (ps2::mul(sext(w & 0xfff), ANGLE_UNIT), ps2::mul(sext((w >> 12) & 0xfff), SPEED_UNIT), (w >> 24) as i32)
    }
}

/// Where a coordinate falls on a 16-point axis: cell index and fraction, with the game's edge rule.
fn axis(num: f32, den: f32) -> (usize, f32) {
    let u = if den == 0.0 { if num > 0.0 { 1.0 } else { 0.0 } } else { ps2::div(num, den).clamp(0.0, 1.0) };
    let s = ps2::mul((N - 1) as f32, u);
    let i = s as usize;
    (i, if i == N - 1 { 1.0 } else { ps2::sub(s, i as f32) })
}

const RADIUS: f32 = 0.064;

/// The low end of the height axis for strokes and volleys: from 0.2 m below the ground at the baseline up to the
/// ground at the net, but never above the ball's radius below it.
fn low_bound(hit_z: f32) -> f32 {
    use crate::ps2::{div, lerp, sub};
    let u = div(sub(11.385, sub(hit_z.abs(), 0.5)), 11.385).clamp(0.0, 1.0);
    lerp(-0.2, 0.0, u).min(-RADIUS)
}

/// Axis bounds for a stroke kind (class 1).
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    /// Hitter-to-net distance range, as negative depths (game constants -0.5, -18.17).
    pub near: f32,
    pub far: f32,
    /// Hit height range (Y-down).
    pub low: f32,
    pub high: f32,
    /// Net-to-target distance range.
    pub short: f32,
    pub long: f32,
}

impl Bounds {
    /// Ground strokes (class 1) by the ball's stored shot `kind` (0..4). Verified on recorded kinds 0–4:
    /// only kind 2 starts the target axis deeper. (The decompiled range function has extra branches for
    /// kinds 1 and 4, but recorded shots match these bounds — its index is not the ball's stored kind.)
    pub fn stroke(kind: i32, hit_z: f32) -> Self {
        Self {
            near: -0.5,
            far: -18.17,
            low: low_bound(hit_z),
            high: ps2::sub(ps2::msub(ps2::add(-1.1638, 0.0), 1.3, 1.3), 0.5),
            short: if kind == 2 { 6.4 } else { 3.0 },
            long: 16.17,
        }
    }

    /// Volleys (class 2, the `voly` tables) by kind: the stroke ranges with a lower top (1.6 m × 1.3 + 0.5); a
    /// slice (kind 1) starts the height axis at the ground.
    pub fn volley(kind: i32, hit_z: f32) -> Self {
        Self {
            low: if kind == 1 { -RADIUS } else { low_bound(hit_z) },
            high: ps2::sub(ps2::mul(-1.6, 1.3), 0.5),
            ..Self::stroke(kind, hit_z)
        }
    }

    /// Serves (class 0); `underhand` is serve kind 3. The low height bound never sits above the ball's radius.
    pub fn serve(underhand: bool, radius: f32) -> Self {
        let (low, high) = if underhand { (0.0, -1.25) } else { (-1.5, -3.45) };
        Self { near: -8.885, far: -17.42, low: if -radius <= low { -radius } else { low }, high, short: 3.0, long: 8.22 }
    }
}

impl Bounds {
    /// Smashes (class 3) by smash kind (0 ✕/○, 1 △): kind 0 starts the target axis deeper.
    pub fn smash(kind: i32) -> Self {
        Self { near: -0.5, far: -18.17, low: -1.7, high: -3.05, short: if kind == 0 { 6.9425 } else { 3.0 }, long: 16.17 }
    }
}

pub struct Lookup {
    pub elevation: f32,
    pub speed: f32,
    pub frames: i32,
}

/// Table lookup for a hit at `hit` aimed at `target` (both game space, Y-down; target height ignored).
pub fn lookup(t: &Table, b: &Bounds, hit: V3, target: V3) -> Lookup {
    // the hit never sits lower than the ball's radius above the ground
    let (mut hit, mut tgt) = ([hit[0], hit[1].min(-RADIUS), hit[2]], target);
    if hit[2] > 0.0 {
        // tables are authored for the near side; mirror the far side through the court centre
        hit = [-hit[0], hit[1], -hit[2]];
        tgt = [-tgt[0], tgt[1], -tgt[2]];
    }
    use crate::ps2::{add, div, madd, mul, sqrt, sub};
    tgt[2] = add(tgt[2], TARGET_PULL);
    let (dx, dz) = (sub(tgt[0], hit[0]), sub(tgt[2], hit[2]));
    let dist = sqrt(madd(madd(add(0.0, 0.0), dx, dx), dz, dz));
    let past_net = div(mul(dist, tgt[2]), dz);
    let (ix, fx) = axis(-sub(-sub(dist, past_net), b.near), -sub(b.far, b.near));
    let (iy, fy) = axis(-sub(hit[1], b.low), -sub(b.high, b.low));
    let (iz, fz) = axis(sub(past_net, b.short), sub(b.long, b.short));
    let at = |x: usize, y: usize, z: usize| t.cell(x + N * y + N * N * z);
    let lerp = |a: f32, b: f32, u: f32| ps2::lerp(a, b, u);
    let plane = |z: usize, k: fn((f32, f32, i32)) -> f32| {
        let a = lerp(k(at(ix, iy, z)), k(at(ix + 1, iy, z)), fx);
        let b = lerp(k(at(ix, iy + 1, z)), k(at(ix + 1, iy + 1, z)), fx);
        lerp(a, b, fy)
    };
    let tri = |k: fn((f32, f32, i32)) -> f32| {
        let a = plane(iz, k);
        lerp(a, plane(iz + 1, k), fz)
    };
    // frames are bytes, interpolated with integer truncation at every stage like the original
    let frames_plane = |z: usize| {
        let row = |y: usize| lerp(at(ix, y, z).2 as f32, at(ix + 1, y, z).2 as f32, fx) as i32 & 0xff;
        let (a, b) = (row(iy), row(iy + 1));
        madd(add(0.0, a as f32), fy, (b - a) as f32) as i32
    };
    let (f0, f1) = (frames_plane(iz), frames_plane(iz + 1));
    let (elevation, mut speed) = (tri(|c| c.0), tri(|c| c.1));
    // Near the net (hitter 0.5–6 m from the net crossing) and up to 1.3 m high, a cell straddling a steep rise in
    // elevation would interpolate a ball that flies long: the speed is pulled toward the cell's slowest corner,
    // the more the closer to the net, the higher the hit and the further the elevation sits above the lowest
    // corner (full past 7°, easing off again toward 1.05 rad). As the game, only the far target plane's corners
    // count.
    let (to_net, height) = (sub(dist, past_net).abs(), hit[1].abs());
    if (0.5..=6.0).contains(&to_net) && (0.0..=1.3).contains(&height) {
        let least = |k: fn((f32, f32, i32)) -> f32| {
            [at(ix, iy, iz + 1), at(ix + 1, iy, iz + 1), at(ix, iy + 1, iz + 1), at(ix + 1, iy + 1, iz + 1)]
                .map(k)
                .into_iter()
                .fold(f32::MAX, f32::min)
        };
        let (lowest, slowest) = (least(|c| c.0), least(|c| c.1));
        let near = cosf(mul(sub(1.0, sub(1.0, div(sub(to_net, 0.5), sub(6.0, 0.5)).clamp(0.0, 1.0))), 1.5707964));
        let high = cosf(mul(sub(1.0, div(sub(height, 0.0), sub(1.3, 0.0)).clamp(0.0, 1.0)), 1.5707964));
        let rise = sub(elevation, lowest);
        let w = if rise >= 0.0 { div(rise, mul(7.0, 0.017453292)).min(1.0) } else { 0.0 };
        if w != 0.0 {
            let k = mul(w, sub(1.0, div(rise.abs(), 1.05).min(1.0)));
            speed = madd(add(0.0, speed), mul(mul(k, high), near), sub(slowest, speed));
        }
    }
    Lookup { elevation, speed, frames: (fz * (f1 - f0) as f32 + f0 as f32 + 0.0) as i32 }
}

/// The launch frame: row 2 heads for the target horizontally, row 1 is up, row 0 = up × row 2, the whole frame
/// then pitched by `elevation` about row 0. The game builds it as here (FPU horizontal unit vector, VU0
/// normalize/cross, rotation about X), so the ball's spin frame is the game's bit for bit.
pub fn launch_frame(hit: V3, target: V3, elevation: f32) -> M4 {
    use crate::ps2::{div, madd, mul, sqrt, sub};
    let (dx, dz) = (sub(target[0], hit[0]), sub(target[2], hit[2]));
    let inv = div(1.0, sqrt(madd(mul(dz, dz), dx, dx)));
    let up = vu0::normalize([0.0, 1.0, 0.0, 0.0]);
    let side = vu0::normalize(vu0::cross(up, [mul(dx, inv), 0.0, mul(dz, inv), 0.0]));
    let ahead = vu0::normalize(vu0::cross(side, up));
    let frame = [side, up, ahead, [0.0, 0.0, 0.0, 1.0]];
    world::mat_mul(&world::mat_mul(&world::IDENTITY, &world::rot_x(elevation)), &frame)
}

/// Launch velocity: along the launch frame's row 2 at `speed` per frame.
pub fn launch(hit: V3, target: V3, elevation: f32, speed: f32) -> V3 {
    let ahead = launch_frame(hit, target, elevation)[2];
    std::array::from_fn(|k| crate::ps2::mul(ahead[k], speed))
}

/// The shot buttons give three kinds: ✕ topspin (0), ○ slice (1), △ lob (3). Flat (2) and drop (4) have no
/// button: at contact the stick (court x, z) turns a topspin into a flat shot when it points within 60° of
/// `facing` (the hitter's end, +1 toward +z) and a slice into a drop shot when it points within 45° of straight
/// back. `branch` is the contact (0 serve, 1 ground, 2 volley, 3 dive, 4 smash): serves only go flat, smashes
/// never change.
pub fn stick_kind(branch: u8, kind: i32, stick: [f32; 2], facing: f32) -> i32 {
    use crate::ps2::{add, div, mul, sqrt};
    let len = sqrt(add(mul(stick[0], stick[0]), mul(stick[1], stick[1])));
    if branch > 3 || len <= 0.0 {
        return kind;
    }
    let along = mul(mul(facing, stick[1]), div(1.0, len));
    match kind {
        0 if along >= 0.5 => 2,
        1 if branch > 0 && -along >= std::f32::consts::FRAC_1_SQRT_2 => 4,
        _ => kind,
    }
}

/// The court margins `inside` pulls a rally shot's aim by (`hst_data::exe::Game::court_margins`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Margins {
    /// Per class (0 serve .. 3 smash) and kind: (across, along) for a shot mode ≥ 0, then for one < 0.
    pub lines: [[[f32; 4]; 5]; 4],
    /// Per character: the widest margin across a short angled topspin stroke gets.
    pub angle: [f32; 14],
}

impl Margins {
    /// A stroke's, volley's or smash's (`class` 1..3) aim pulled inside the court, as the original before the
    /// timing scatter: the sidelines (doubles' with three or more players) and the baselines (11.885), less the
    /// kind's margins (`low`: the smash's mode is negative, its wider pair). When the shot heads for the far
    /// sideline, the margins are scaled by the share of the hit→aim direction along each (unless it is a
    /// `special` shot, not modelled yet), and a topspin stroke's across margin grows to the character's
    /// `angle` × the angle's tangent for a short cross-court ball (aim under 6 m deep, crossing the net 9–19 m
    /// from the hit). A topspin from outside a sideline aiming further out is instead judged where it is
    /// 18 m along the shot, its unscaled margins moving the aim by as much.
    #[allow(clippy::too_many_arguments)]
    pub fn inside(&self, class: u8, kind: i32, low: bool, doubles: bool, character: usize, special: bool, hit: V3, aim: V3) -> V3 {
        use crate::ps2::{add, div, madd, mul, sqrt, sub};
        let abs = |v: f32| if v < 0.0 { -v } else { v };
        let hi = if doubles { 5.485 } else { 4.115 };
        let lo = -hi;
        let row = self.lines[class as usize][kind as usize];
        let (mut mx, mut mz) = if low { (row[2], row[3]) } else { (row[0], row[1]) };
        let topspin = class == 1 && kind == 0;
        let (dx, dz) = (sub(aim[0], hit[0]), sub(aim[2], hit[2]));
        let inv = div(1.0, sqrt(madd(mul(dz, dz), dx, dx)));
        let step = [madd(add(0.0, hit[0]), mul(dx, inv), 18.0), madd(add(0.0, hit[2]), mul(dz, inv), 18.0)];
        let outward = topspin && ((step[0] < sub(lo, mx) && hit[0] < lo) || (add(hi, mx) < step[0] && hi < hit[0]));
        let p = if outward {
            step
        } else {
            let across = (aim[0] < 0.0 && lo < hit[0]) || (0.0 < aim[0] && hit[0] < hi);
            if across && !special {
                mx = mul(mx, abs(mul(dx, inv)));
                mz = mul(mz, abs(mul(dz, inv)));
            }
            let extra = if topspin && across { self.angled(lo, hi, character, hit, aim) } else { 0.0 };
            if mx <= extra {
                mx = extra;
            }
            [aim[0], aim[2]]
        };
        let (x0, x1) = (add(lo, mx), sub(hi, mx));
        let x = if p[0] < x0 { x0 } else if p[0] <= x1 { p[0] } else { x1 };
        let z1 = sub(11.885, mz);
        let z = if p[1] < -z1 { -z1 } else if p[1] <= z1 { p[1] } else { z1 };
        [add(aim[0], sub(x, p[0])), aim[1], add(aim[2], sub(z, p[1]))]
    }

    /// The topspin's widened sideline margin for a short angled ball (0 when it isn't one).
    fn angled(&self, lo: f32, hi: f32, character: usize, hit: V3, aim: V3) -> f32 {
        use crate::libm::{atanf, tanf};
        use crate::ps2::{add, div, madd, msub, mul, sqrt, sub};
        let abs = |v: f32| if v < 0.0 { -v } else { v };
        let cx = if aim[0] < lo { lo } else if aim[0] <= hi { aim[0] } else { hi };
        let (dx, dz) = (sub(cx, hit[0]), sub(aim[2], hit[2]));
        let len = sqrt(madd(madd(0.0, dx, dx), dz, dz));
        // from the hit to where the shot crosses the net
        let net = sub(len, div(mul(len, aim[2]), dz));
        let deep = abs(aim[2]);
        let far = add(3.0, 3.0);
        if net <= 9.0 || !(deep < far) {
            return 0.0;
        }
        let width = mul(self.angle[character], tanf(atanf(div(abs(dx), abs(dz)))));
        let mut short = div(sub(deep, 3.0), sub(far, 3.0));
        if short < 0.0 {
            short = 0.0;
        }
        let short = sub(1.0, short);
        // the path length of the shot's last 3 m of depth: 3 m straight, longer the more it angles
        let off = msub(add(0.0, aim[2]), 3.0, if aim[2] < 0.0 { -1.0 } else { 1.0 });
        let (hz, az) = (sub(hit[2], off), sub(aim[2], off));
        let (ex, ez) = (sub(cx, hit[0]), sub(az, hz));
        let run = abs(div(mul(sqrt(madd(madd(0.0, ex, ex), ez, ez)), az), ez));
        let straight = if run <= 3.0 {
            1.0
        } else {
            let q = div(sub(run, 3.0), sub(5.0, 3.0));
            sub(1.0, if q <= 1.0 { q } else { 1.0 })
        };
        let ramp = if net < 15.0 {
            let q = div(sub(net, 9.0), sub(15.0, 9.0));
            if q <= 1.0 { q } else { 1.0 }
        } else {
            let q = div(sub(net, 15.0), sub(19.0, 15.0));
            sub(1.0, if q <= 1.0 { q } else { 1.0 })
        };
        mul(width, mul(ramp, mul(short, straight)))
    }
}

#[cfg(test)]
mod tests {
    use super::Margins;

    /// Hand-worked cases for the branches no recording reaches (the live recordings cover the plain, scaled and
    /// angled pulls bit for bit in `tests/shot_tables.rs`).
    #[test]
    fn inside_pulls_smashes_and_outward_topspin() {
        let mut m = Margins::default();
        m.lines[1][0] = [0.15, 0.0, 0.15, 0.0];
        m.lines[3][0] = [0.15, 0.15, 0.9, 0.75];
        let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 2e-6);
        // a low smash straight down the middle: the deep column's 0.75 from the baseline, unscaled (not across)
        let got = m.inside(3, 0, true, false, 0, false, [0.0, 1.0, -10.0], [0.0, 0.0, 11.8]);
        assert!(near(got, [0.0, 0.0, 11.135]), "{got:?}");
        // a high smash angled 3 m: the 0.15 depth margin scaled by |uz| = 21.8 / sqrt(484.24)
        let got = m.inside(3, 0, false, false, 0, false, [0.0, 1.0, -10.0], [3.0, 0.0, 11.8]);
        assert!(near(got, [3.0, 0.0, 11.885 - 0.15 * 21.8 / 484.24f32.sqrt()]), "{got:?}");
        // a topspin hit from outside the singles sideline and running further out: its point 18 m along the shot
        // (x = 5 + 18 / sqrt(401)) is pulled to the sideline less the unscaled 0.15, the aim moved by as much
        let got = m.inside(1, 0, false, false, 0, false, [5.0, 1.0, -10.0], [6.0, 0.0, 10.0]);
        assert!(near(got, [6.0 + 3.965 - (5.0 + 18.0 / 401f32.sqrt()), 0.0, 10.0]), "{got:?}");
    }
}
