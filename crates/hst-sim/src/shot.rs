//! Shot creation from the precomputed trajectory tables (`TRAJ/tr_pcNN_<type>.dat`).
//!
//! A table is a 16×16×16 grid of u32 cells: bits 0–11 signed elevation in π/4096 rad, bits 12–23 signed
//! speed in 1/1024 m/frame, top byte flight frames. Axes, for a hitter mirrored onto the z < 0 half:
//! x = distance from the hitter to where the shot line crosses the net, y = hit height, z = distance from
//! the net crossing to the target. The game interpolates trilinearly in f32; this keeps its order.

use crate::ball::V3;
use crate::libm::cosf;

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
        (sext(w & 0xfff) * ANGLE_UNIT, sext((w >> 12) & 0xfff) * SPEED_UNIT, (w >> 24) as i32)
    }
}

/// Where a coordinate falls on a 16-point axis: cell index and fraction, with the game's edge rule.
fn axis(num: f32, den: f32) -> (usize, f32) {
    let u = if den == 0.0 { if num > 0.0 { 1.0 } else { 0.0 } } else { (num / den).clamp(0.0, 1.0) };
    let s = (N - 1) as f32 * u;
    let i = s as usize;
    (i, if i == N - 1 { 1.0 } else { s - i as f32 })
}

const RADIUS: f32 = 0.064;

/// The low end of the height axis for strokes and volleys: from 0.2 m below the ground at the baseline up to the
/// ground at the net, but never above the ball's radius below it.
fn low_bound(hit_z: f32) -> f32 {
    let u = ((11.385 - (hit_z.abs() - 0.5)) / 11.385).clamp(0.0, 1.0);
    (u * (0.0 - -0.2) + -0.2 + 0.0).min(-RADIUS)
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
            high: ((-1.1638 + 0.0) - 1.3 * 1.3) - 0.5,
            short: if kind == 2 { 6.4 } else { 3.0 },
            long: 16.17,
        }
    }

    /// Volleys (class 2, the `voly` tables) by kind: the stroke ranges with a lower top (1.6 m × 1.3 + 0.5); a
    /// slice (kind 1) starts the height axis at the ground.
    pub fn volley(kind: i32, hit_z: f32) -> Self {
        Self {
            low: if kind == 1 { -RADIUS } else { low_bound(hit_z) },
            high: -1.6 * 1.3 - 0.5,
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
    tgt[2] += TARGET_PULL;
    let dz = tgt[2] - hit[2];
    let dist = (dz * dz + (tgt[0] - hit[0]) * (tgt[0] - hit[0]) + 0.0).sqrt();
    let past_net = (dist * tgt[2]) / dz;
    let (ix, fx) = axis(-(-(dist - past_net) - b.near), -(b.far - b.near));
    let (iy, fy) = axis(-(hit[1] - b.low), -(b.high - b.low));
    let (iz, fz) = axis(past_net - b.short, b.long - b.short);
    let at = |x: usize, y: usize, z: usize| t.cell(x + N * y + N * N * z);
    let lerp = |a: f32, b: f32, u: f32| u * (b - a) + a + 0.0;
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
        (fy * (b - a) as f32 + a as f32 + 0.0) as i32
    };
    let (f0, f1) = (frames_plane(iz), frames_plane(iz + 1));
    let (elevation, mut speed) = (tri(|c| c.0), tri(|c| c.1));
    // Near the net (hitter 0.5–6 m from the net crossing) and up to 1.3 m high, a cell straddling a steep rise in
    // elevation would interpolate a ball that flies long: the speed is pulled toward the cell's slowest corner,
    // the more the closer to the net, the higher the hit and the further the elevation sits above the lowest
    // corner (full past 7°, easing off again toward 1.05 rad). As the game, only the far target plane's corners
    // count.
    let (to_net, height) = ((dist - past_net).abs(), hit[1].abs());
    if (0.5..=6.0).contains(&to_net) && (0.0..=1.3).contains(&height) {
        let least = |k: fn((f32, f32, i32)) -> f32| {
            [at(ix, iy, iz + 1), at(ix + 1, iy, iz + 1), at(ix, iy + 1, iz + 1), at(ix + 1, iy + 1, iz + 1)]
                .map(k)
                .into_iter()
                .fold(f32::MAX, f32::min)
        };
        let (lowest, slowest) = (least(|c| c.0), least(|c| c.1));
        let near = cosf((1.0 - (1.0 - ((to_net - 0.5) / (6.0 - 0.5)).clamp(0.0, 1.0))) * 1.5707964);
        let high = cosf((1.0 - (height / 1.3).clamp(0.0, 1.0)) * 1.5707964);
        let rise = elevation - lowest;
        let w = if rise >= 0.0 { (rise / (7.0 * 0.017453292)).min(1.0) } else { 0.0 };
        if w != 0.0 {
            let k = w * (1.0 - (rise.abs() / 1.05).min(1.0));
            speed = k * high * near * (slowest - speed) + speed + 0.0;
        }
    }
    Lookup { elevation, speed, frames: (fz * (f1 - f0) as f32 + f0 as f32 + 0.0) as i32 }
}

/// Launch velocity: head for the target horizontally, pitched up by `elevation`, at `speed` per frame.
pub fn launch(hit: V3, target: V3, elevation: f32, speed: f32) -> V3 {
    let (dx, dz) = (target[0] - hit[0], target[2] - hit[2]);
    let inv = 1.0 / (dx * dx + dz * dz).sqrt();
    let (s, c) = elevation.sin_cos();
    [dx * inv * c * speed, -s * speed, dz * inv * c * speed]
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
