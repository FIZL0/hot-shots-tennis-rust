//! Shot creation from the precomputed trajectory tables (`TRAJ/tr_pcNN_<type>.dat`).
//!
//! A table is a 16×16×16 grid of u32 cells: bits 0–11 signed elevation in π/4096 rad, bits 12–23 signed
//! speed in 1/1024 m/frame, top byte flight frames. Axes, for a hitter mirrored onto the z < 0 half:
//! x = distance from the hitter to where the shot line crosses the net, y = hit height, z = distance from
//! the net crossing to the target. The game interpolates trilinearly in f32; this keeps its order.
#![deny(clippy::float_arithmetic)]

use crate::ball::V3;
use crate::libm::cosf;
use crate::lines::{BASELINE, BOX_MIDDLE, DEG, LENGTH, sideline};
use crate::ps2::{F, f32s, fs};
use crate::replay::Pad;
use crate::vu0;
use crate::world::{self, M4};

/// Shot classes: the trajectory tables' and the shot records' first index (`Launch`, `Margins`, `params`).
pub mod class {
    pub const SERVE: u8 = 0;
    pub const STROKE: u8 = 1;
    pub const VOLLEY: u8 = 2;
    pub const SMASH: u8 = 3;
}

/// Shot kinds. The buttons give topspin (✕), slice (○) and lob (△); at contact the stick turns a topspin flat and
/// a slice into a drop shot (`stick_kind`). A serve's kinds are its own (`UNDERHAND`), and a smash's `LOB` is the
/// △ smash.
pub mod kind {
    pub const TOPSPIN: i32 = 0;
    pub const SLICE: i32 = 1;
    pub const FLAT: i32 = 2;
    pub const LOB: i32 = 3;
    pub const DROP: i32 = 4;
    /// The underhand serve (the serve's kind 3).
    pub const UNDERHAND: i32 = 3;
}

/// The contact a swing locked onto (`Hitter::branch`): its own numbering, not the shot class.
pub mod branch {
    pub const SERVE: u8 = 0;
    pub const GROUND: u8 = 1;
    pub const VOLLEY: u8 = 2;
    pub const DIVE: u8 = 3;
    pub const SMASH: u8 = 4;

    /// A rally contact: a ground stroke, volley or dive.
    pub fn rally(b: u8) -> bool {
        (GROUND..=DIVE).contains(&b)
    }
}

/// cos 60°: a stick within 60° of straight ahead turns a topspin flat.
const FLAT_CONE: f32 = 0.5;
/// The hitter-to-net axis of every rally table, as negative depths: 0.5 m to 18.17 m from the net crossing.
const NET_NEAR: f32 = -0.5;
const NET_FAR: f32 = -18.17;
/// The game constant the stroke tables' height axis tops out from (1.1638…).
const STROKE_TOP: f32 = f32::from_bits(0x3f94_f893);

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

    /// (elevation, speed, frames) of one cell.
    fn cell(&self, i: usize) -> (f32, f32, i32) {
        let w = self.0[i];
        let sext = |v: u32| ((v << 20) as i32 >> 20) as f32;
        ((F(sext(w & 0xfff)) * ANGLE_UNIT).0, (F(sext((w >> 12) & 0xfff)) * SPEED_UNIT).0, (w >> 24) as i32)
    }
}

/// Where a coordinate falls on a 16-point axis: cell index and fraction, with the game's edge rule (at the far end
/// the last cell pair at fraction 1; a zero-width axis divides to ±max and clamps).
fn axis(num: F, den: F) -> (usize, F) {
    let s = (N - 1) as f32 * (num / den).clamp(0.0, 1.0);
    let i = s.0 as usize;
    if i == N - 1 { (N - 2, F::ONE) } else { (i, s - i as f32) }
}

const RADIUS: f32 = 0.064;
/// The ball's radius above the ground (Y-down).
const ABOVE_GROUND: f32 = -RADIUS;

/// The low end of the height axis for strokes and volleys: from 0.2 m below the ground at the baseline up to the
/// ground at the net, but never above the ball's radius below it.
fn low_bound(hit_z: f32) -> f32 {
    let u = ((11.385 - (F(hit_z.abs()) - 0.5)) / 11.385).clamp(0.0, 1.0);
    F(-0.2).lerp(0.0, u).min(ABOVE_GROUND).0
}

/// Axis bounds for a stroke kind (class 1).
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    /// Hitter-to-net distance range, as negative depths (game constants -0.5, -18.17).
    pub near: f32,
    pub far: f32,
    /// Hit height range (Y-down).
    pub low: f32,
    /// The low end follows the hit's distance from the net (`low_bound`): `lookup` sets it from its own hit point.
    pub sloped: bool,
    pub high: f32,
    /// Net-to-target distance range.
    pub short: f32,
    pub long: f32,
}

impl Bounds {
    /// Ground strokes (class 1) by the ball's stored shot `kind` (0..4): kind 2 starts the target axis deeper; a
    /// slice (1) starts the height axis at the ground; a drop shot (4) has a flat height axis at 0.3 m and a short
    /// target axis (2–8.22 m). `hit_z` sets `low` for a caller's use; `lookup` redoes it at its own hit point.
    pub fn stroke(kind: i32, hit_z: f32) -> Self {
        let high = ((-F(STROKE_TOP) + 0.0).msub(1.3, 1.3) - 0.5).0;
        let (near, far) = (NET_NEAR, NET_FAR);
        match kind {
            kind::DROP => Self { near, far, low: -0.3, sloped: false, high: -0.3, short: 2.0, long: 8.22 },
            kind::SLICE => Self { near, far, low: ABOVE_GROUND, sloped: false, high, short: 3.0, long: 16.17 },
            _ => Self { near, far, low: low_bound(hit_z), sloped: true, high, short: if kind == kind::FLAT { 6.4 } else { 3.0 }, long: 16.17 },
        }
    }

    /// Volleys (class 2, the `voly` tables) by kind: the stroke ranges with a lower top (1.6 m × 1.3 + 0.5); a
    /// slice (kind 1) starts the height axis at the ground.
    pub fn volley(kind: i32, hit_z: f32) -> Self {
        Self {
            low: if kind == kind::SLICE { ABOVE_GROUND } else { low_bound(hit_z) },
            sloped: kind != kind::SLICE,
            high: (F(-1.6) * 1.3 - 0.5).0,
            short: match kind {
                kind::FLAT => 6.4,
                kind::DROP => 2.0,
                _ => 3.0,
            },
            long: if kind == kind::DROP { 8.22 } else { 16.17 },
            ..Self::stroke(kind::TOPSPIN, hit_z)
        }
    }

    /// Serves (class 0); `underhand` is `kind::UNDERHAND`. The low height bound never sits above the ball's radius.
    pub fn serve(underhand: bool, radius: f32) -> Self {
        let (low, high) = if underhand { (0.0, -1.25) } else { (-1.5, -3.45) };
        let top = -F(radius);
        Self { near: -8.885, far: -17.42, low: if top <= low { top.0 } else { low }, sloped: false, high, short: 3.0, long: 8.22 }
    }
}

impl Bounds {
    /// Smashes (class 3) by smash kind (0 ✕/○, 1 △): kind 0 starts the target axis deeper.
    pub fn smash(kind: i32) -> Self {
        Self { near: NET_NEAR, far: NET_FAR, low: -1.7, sloped: false, high: -3.05, short: if kind == 0 { 6.9425 } else { 3.0 }, long: 16.17 }
    }
}

#[derive(Clone, Copy)]
pub struct Lookup {
    pub elevation: f32,
    pub speed: f32,
    pub frames: i32,
}

/// Table lookup for a hit at `hit` aimed at `target` (both game space, Y-down; target height ignored).
pub fn lookup(t: &Table, b: &Bounds, hit: V3, target: V3) -> Lookup {
    // the hit never sits lower than the ball's radius above the ground
    let (mut hit, mut tgt) = ([hit[0], hit[1].min(ABOVE_GROUND), hit[2]], target);
    if hit[2] > 0.0 {
        // tables are authored for the near side; turn the far side half a turn about the court centre (VU0)
        let m = world::rot_y(world::PI);
        let turn = |v: V3| {
            let r = vu0::transform(&m, [v[0], v[1], v[2], 1.0]);
            [r[0], r[1], r[2]]
        };
        (hit, tgt) = (turn(hit), turn(tgt));
    }
    let b = &Bounds { low: if b.sloped { low_bound(hit[2]) } else { b.low }, ..*b };
    let (hit, mut tgt) = (fs(hit), fs(tgt));
    tgt[2] += TARGET_PULL;
    let (dx, dz) = (tgt[0] - hit[0], tgt[2] - hit[2]);
    let dist = (F::ZERO + 0.0).madd(dx, dx).madd(dz, dz).sqrt();
    let past_net = dist * tgt[2] / dz;
    let (ix, fx) = axis(-(-(dist - past_net) - b.near), -(F(b.far) - b.near));
    let (iy, fy) = axis(-(hit[1] - b.low), -(F(b.high) - b.low));
    let (iz, fz) = axis(past_net - b.short, F(b.long) - b.short);
    let at = |x: usize, y: usize, z: usize| t.cell(x + N * y + N * N * z);
    let lerp = |a: f32, b: f32, u: F| F(a).lerp(b, u);
    let plane = |z: usize, k: fn((f32, f32, i32)) -> f32| {
        let a = lerp(k(at(ix, iy, z)), k(at(ix + 1, iy, z)), fx);
        let b = lerp(k(at(ix, iy + 1, z)), k(at(ix + 1, iy + 1, z)), fx);
        a.lerp(b, fy)
    };
    let tri = |k: fn((f32, f32, i32)) -> f32| {
        let a = plane(iz, k);
        a.lerp(plane(iz + 1, k), fz)
    };
    // frames are bytes, interpolated with integer truncation at every stage like the original
    let frames_plane = |z: usize| {
        let row = |y: usize| lerp(at(ix, y, z).2 as f32, at(ix + 1, y, z).2 as f32, fx).0 as i32 & 0xff;
        let (a, b) = (row(iy), row(iy + 1));
        (F::ZERO + a as f32).madd(fy, (b - a) as f32).0 as i32
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
        let near = cosf(((1.0 - (1.0 - ((to_net - 0.5) / (F(6.0) - 0.5)).clamp(0.0, 1.0))) * 1.5707964).0);
        let high = cosf(((1.0 - ((height - 0.0) / (F(1.3) - 0.0)).clamp(0.0, 1.0)) * 1.5707964).0);
        let rise = elevation - lowest;
        let w = if rise >= 0.0 { (rise / (F(7.0) * DEG)).min(1.0) } else { F::ZERO };
        if w != 0.0 {
            let k = w * (1.0 - (rise.abs() / 1.05).min(1.0));
            speed = (F::ZERO + speed).madd(k * (F(high) * near), slowest - speed);
        }
    }
    // the frames' last step is the same lerp idiom as `frames_plane`'s
    Lookup { elevation: elevation.0, speed: speed.0, frames: (F::ZERO + f0 as f32).madd(fz, (f1 - f0) as f32).0 as i32 }
}

/// The launch frame: row 2 heads for the target horizontally, row 1 is up, row 0 = up × row 2, the whole frame
/// then pitched by `elevation` about row 0. The game builds it as here (FPU horizontal unit vector, VU0
/// normalize/cross, rotation about X), so the ball's spin frame is the game's bit for bit.
pub fn launch_frame(hit: V3, target: V3, elevation: f32) -> M4 {
    let (hit, target) = (fs(hit), fs(target));
    let (dx, dz) = (target[0] - hit[0], target[2] - hit[2]);
    let inv = 1.0 / (dz * dz).madd(dx, dx).sqrt();
    let up = vu0::normalize([0.0, 1.0, 0.0, 0.0]);
    let side = vu0::normalize(vu0::cross(up, f32s([dx * inv, F::ZERO, dz * inv, F::ZERO])));
    let ahead = vu0::normalize(vu0::cross(side, up));
    let frame = [side, up, ahead, [0.0, 0.0, 0.0, 1.0]];
    world::mat_mul(&world::mat_mul(&world::IDENTITY, &world::rot_x(elevation)), &frame)
}

/// The shot's sideways axis (the ball's +0x90, the bend's direction): up × the flat unit vector to the target.
/// Unlike `launch_frame`'s row 0, the game renormalizes the flat vector on the VU first.
pub fn side_axis(hit: V3, target: V3) -> vu0::V4 {
    let (hit, target) = (fs(hit), fs(target));
    let (dx, dz) = (target[0] - hit[0], target[2] - hit[2]);
    let inv = 1.0 / (dz * dz).madd(dx, dx).sqrt();
    let ahead = vu0::normalize(f32s([dx * inv, F::ZERO, dz * inv, F::ZERO]));
    vu0::normalize(vu0::cross(vu0::normalize([0.0, 1.0, 0.0, 0.0]), ahead))
}

/// The special condition the shot effects need: a strong-tossed topspin or slice serve hit within a frame of the
/// sweet spot (timing `offset`), any other shot hit with a clean timing `grade` (1 or 2).
pub fn special(class: u8, kind: i32, strong: bool, grade: u8, offset: i32) -> bool {
    if class == class::SERVE && strong && kind <= kind::SLICE { offset.abs() < 2 } else { grade == 1 || grade == 2 }
}

/// A special shot's effect, as launched: (`Shot::bend`, `Shot::curve`, `Shot::bounce_turn`). The character's
/// bend and curve grow with the flat distance up to a court's length (`LENGTH`); a left-hander's bend and turn
/// are mirrored, and so is the bend of a shot hit with `flip`.
// ponytail: the game's debug switch that forces every effect, and its practice-only exception (one player on
// court, player 1 hitting, gets row 0's lob curve and no bend or turn), are left out
pub fn effect(e: &hst_data::exe::ShotEffects, class: u8, kind: i32, hit: V3, target: V3, lefty: bool, flip: bool) -> (f32, f32, f32) {
    let (mut bend, mut curve, mut turn) = match (class, kind) {
        (class::SERVE, kind::TOPSPIN) => (F(e.bend[0]), F::ZERO, F(e.turn) * DEG),
        (class::SERVE, kind::SLICE) => (F(e.bend[1]), F::ZERO, F::ZERO),
        (class::STROKE, kind::SLICE) => (F(e.bend[2]), F::ZERO, F::ZERO),
        (class::STROKE, kind::LOB) => (F::ZERO, F(e.curve), F::ZERO),
        _ => (F::ZERO, F::ZERO, F::ZERO),
    };
    if bend != 0.0 || curve != 0.0 {
        let (hit, target) = (fs(hit), fs(target));
        let (dx, dz) = (target[0] - hit[0], target[2] - hit[2]);
        let s = ((dz * dz).madd(dx, dx).sqrt() / LENGTH).clamp(0.0, 1.0);
        if bend != 0.0 {
            bend = bend * s;
            if flip {
                bend = bend * -1.0;
            }
            if lefty {
                bend = bend * -1.0;
            }
        }
        if curve != 0.0 {
            curve = curve * s;
        }
    }
    if turn != 0.0 && lefty {
        turn = turn * -1.0;
    }
    (bend.0, curve.0, turn.0)
}

/// Launch velocity: along the launch frame's row 2 at `speed` per frame.
pub fn launch(hit: V3, target: V3, elevation: f32, speed: f32) -> V3 {
    let ahead = launch_frame(hit, target, elevation)[2];
    std::array::from_fn(|k| (F(ahead[k]) * speed).0)
}

/// A launch with its side angle (record field 10, nonzero only on slice serves) and the frames it flies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub vel: V3,
    /// The ball's spin frame.
    pub frame: M4,
    /// Per-frame pull that bends the flight onto the target (w included).
    pub wind: [f32; 4],
    /// Spin, made positive when the side angle turned it.
    pub spin: f32,
    pub frames: i32,
}

/// The launch of `launch_frame` with the record's `spin` and `side` angle (radians), the table's flight
/// `frames`, and the shot's `bend` (0 until the special shots, P6). A lefty hitter's side-angled shot spins the
/// other way. With a side angle the velocity heads off the target line by the spin (as a yaw) and the wind
/// brings the ball back onto the target over the flight; the spin frame is turned a quarter about Z and by the
/// side angle about Y, toward the spin's sign. Off a serve clear of the service boxes' middle (`BOX_MIDDLE`) the flight
/// gets one frame less or more, by which side of that line the ball crosses to.
#[allow(clippy::too_many_arguments)]
pub fn launch_turned(class: u8, hit: [f32; 4], target: [f32; 4], elevation: f32, speed: f32, spin: f32, side: f32, bend: f32, lefty: bool, frames: i32) -> Launch {
    let frame = launch_frame([hit[0], hit[1], hit[2]], [target[0], target[1], target[2]], elevation);
    let mut spin = if side != 0.0 && lefty { F(spin) * -1.0 } else { F(spin) };
    let mut n = frames + 1;
    if side != 0.0 || bend != 0.0 {
        let mut step = 0;
        if class == class::SERVE {
            let toward = if target[2] < 0.0 { -1.0 } else { 1.0 };
            if (F(target[0].abs()) - BOX_MIDDLE).abs() > 1.0 {
                let past = F(target[0]) * toward;
                step = if F(hit[0]) * toward > 0.0 { if past < -F(BOX_MIDDLE) { 1 } else { -1 } } else if past < BOX_MIDDLE { 1 } else { -1 };
                if spin < 0.0 || bend < 0.0 {
                    step = -step;
                }
            }
        }
        n += step;
    }
    let ahead = frame[2];
    if side == 0.0 {
        let vel = std::array::from_fn(|k| (F(ahead[k]) * speed).0);
        return Launch { vel, frame, wind: [0.0; 4], spin: spin.0, frames: n };
    }
    // ponytail: the yaw is never past ±π (a record's spin is a few tens of degrees), so the game's wrap is left out
    let (hit, target) = (fs(hit), fs(target));
    let d = f32s([target[0] - hit[0], target[1] - hit[1], target[2] - hit[2], F::ONE]);
    let turned = world::mat_mul(&[frame[0], frame[1], frame[2], d], &world::rot_y(spin.0));
    let inv = 1.0 / F(n as f32);
    let back = |k: usize| (target[k] - (F(turned[3][k]) + hit[k])) * inv;
    let wind = f32s([back(0), back(1), back(2), (target[3] - 1.0) * inv]);
    let vel = std::array::from_fn(|k| (F(turned[2][k]) * speed).0);
    let (quarter, side) = if spin >= 0.0 { (F(world::HALF_PI), F(side)) } else { spin = -spin; (-F(world::HALF_PI), -F(side)) };
    let frame = world::mat_mul(&world::mat_mul(&world::IDENTITY, &rot_z(quarter.0)), &frame);
    let frame = world::mat_mul(&world::mat_mul(&world::IDENTITY, &world::rot_y(side.0)), &frame);
    Launch { vel, frame, wind, spin: spin.0, frames: n }
}

/// Rotation about Z (rows (c, s, 0), (−s, c, 0), (0, 0, 1)), built like `world::rot_x`.
pub(crate) fn rot_z(t: f32) -> M4 {
    let (s, c) = world::sincos(t);
    let z = vu0::Vf(0.0);
    [[(z + c).0, (z + s).0, 0.0, 0.0], [(z - s).0, (z + c).0, 0.0, 0.0], [0.0, 0.0, (z + 1.0).0, 0.0], [0.0, 0.0, 0.0, (z + 1.0).0]]
}

/// The shot press a frame's pad gives (`before`, `now`: the pad words): only a button going down this frame
/// counts, ✕ before ○ before △ when several do; its kind (✕ topspin, ○ slice, △ lob). Holding a button gives
/// nothing more: no charge, no repeat.
pub fn press_kind(before: u16, now: u16) -> Option<i32> {
    let down = now & !before;
    [(Pad::CROSS, kind::TOPSPIN), (Pad::CIRCLE, kind::SLICE), (Pad::TRIANGLE, kind::LOB)].into_iter().find(|&(b, _)| down & b != 0).map(|(_, k)| k)
}

/// The shot buttons give three kinds: ✕ topspin, ○ slice, △ lob. Flat and drop have no button: at contact the
/// stick (court x, z) turns a topspin into a flat shot when it points within 60° of `facing` (the hitter's end,
/// +1 toward +z) and a slice into a drop shot when it points within 45° of straight back. `branch` is the contact
/// (`branch`): serves only go flat, smashes never change.
pub fn stick_kind(branch: u8, kind: i32, stick: [f32; 2], facing: f32) -> i32 {
    let stick = fs(stick);
    let len = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt();
    if branch >= branch::SMASH || len <= 0.0 {
        return kind;
    }
    let along = F(facing) * stick[1] * (1.0 / len);
    match kind {
        kind::TOPSPIN if along >= FLAT_CONE => kind::FLAT,
        kind::SLICE if branch != branch::SERVE && -along >= std::f32::consts::FRAC_1_SQRT_2 => kind::DROP,
        _ => kind,
    }
}

/// The character values a human's aim reads (TParam): the widest angle off straight per contact, in degrees
/// (strokes, volleys/dives/smashes, serves), its share in % for a body shot (stroke, volley) and for a stroke
/// or volley taken under `under` metres (rising ball).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AimStats {
    pub con: [i32; 3],
    pub body: [i32; 2],
    pub rising: i32,
    pub under: f32,
}

impl AimStats {
    /// Indices into `con` (all three) and `body` (the first two).
    pub const STROKE: usize = 0;
    pub const VOLLEY: usize = 1;
    pub const SERVE: usize = 2;
}

/// The hitter at the moment of the shot.
#[derive(Clone, Copy, Debug, Default)]
pub struct Hitter {
    /// The player's end: +1 hits toward +z.
    pub end: f32,
    /// The contact (`branch`).
    pub branch: u8,
    /// After `stick_kind` (`kind`; a smash's `LOB` is the △ smash).
    pub kind: i32,
    /// Timing offset in frames from the sweet frame.
    pub offset: i32,
    /// A body shot (the ball too close sideways).
    pub body: bool,
    /// Where the swing started (x, z) and the contact height.
    pub from: [f32; 2],
    pub height: f32,
}

/// A human's aim for a rally shot (before the court margins and the timing scatter): a base point mid-way down
/// the far half (shorter and narrower off the sweet spot) plus the stick (court x, z) mapped square onto the
/// half (a full diagonal reaches the singles' or, with four players, the doubles' corner), then the
/// character's widest angle off straight, the dip of an angled drive back onto the straight line's depth and
/// a 3 m (drop 2 m) minimum past the net. `button` is the original's 0x20 aim (deep), which only the AI sets;
/// `incoming` is Some(was sweet) when the ball being struck is a slice (not a smash) in a rally under way.
/// Also what the aim leaves for the launch (see `Aim`); `roll` is the game's RNG (bit 16 is the coin), drawn twice
/// for the nudge.
pub fn aim(h: &Hitter, s: &AimStats, stick: [f32; 2], four: bool, button: bool, incoming: Option<bool>, roll: &mut impl FnMut() -> u32) -> Aim {
    let (end, b) = (h.end, h.branch);
    let rally_kind = |k: i32| branch::rally(b) && h.kind == k;
    let drop = rally_kind(kind::DROP);
    let sweet = b != branch::DIVE && !h.body && h.offset.abs() < 2;
    // depth of the target band and its near edge
    let (long, near) = if drop {
        (9.885, 2.0)
    } else {
        (8.885, 3.0)
    };
    let long = if sweet { F(long) } else { F(long) - if rally_kind(kind::LOB) { 0.5 } else { 1.0 } };
    let half = long / 2.0;
    let base_z = (F(near) + half) * end;
    let width = F(sideline(four));
    let width = if sweet { width } else { width - 0.5 };
    let target = aim_from(h, s, stick, four, button, incoming, [0.0, base_z.0], width.0, half.0);
    // a plain smash with the stick pulled back holds the pull as a shorter launch (the stick's own depth, not
    // the square's)
    let held = if !button && b == branch::SMASH && h.kind != kind::LOB && F(stick[1]) * end < 0.0 { (half / 1.5 * stick[1].abs() / 2.0).0 } else { 0.0 };
    // aimed close to the centre line with the stick level: a random sideways nudge of 5 or 10 tenths
    let mut coin = || roll() >> 16 & 1 != 0;
    let nudge = if target[0].abs() <= 1.0 && stick[0] == 0.0 {
        let n = if coin() { 5 } else { 10 };
        if coin() { n } else { -n }
    } else {
        0
    };
    Aim { target, nudge, short_only: drop, held }
}

/// A rally aim and what it leaves for the launch: the centred stick's sideways `nudge` (tenths, added to the
/// timing error's side), `short_only` (a drop shot: the depth error can only fall short) and the plain smash's
/// `held` depth (m) taken off its depth error.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Aim {
    pub target: V3,
    pub nudge: i32,
    pub short_only: bool,
    pub held: f32,
}

/// `aim` from its base point (x, z), the stick's full reach across (`width`) and along (`half`): the stick, the
/// widest angle, the straight line's depth and the minimum past the net. The serve's aim shares it (branch 0).
#[allow(clippy::too_many_arguments)]
pub fn aim_from(h: &Hitter, s: &AimStats, stick: [f32; 2], four: bool, button: bool, incoming: Option<bool>, base: [f32; 2], width: f32, half: f32) -> V3 {
    use crate::libm::{acosf, tanf};
    let (end, b) = (h.end, h.branch);
    let rally = branch::rally(b);
    let rally_kind = |k: i32| rally && h.kind == k;
    let drop = rally_kind(kind::DROP);
    let plain_smash = b == branch::SMASH && h.kind != kind::LOB;
    let near = F(if drop { 2.0 } else { 3.0 });
    // the stick's circle stretched onto the square
    let (mut ox, mut oz) = (F::ZERO, F::ZERO);
    let stick = fs(stick);
    let len = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt();
    if len > 0.0 {
        let inv = 1.0 / len;
        let (nx, nz) = (stick[0] * inv, stick[1] * inv);
        let m = 1.0 / if nx.abs() <= nz.abs() { nz.abs() } else { nx.abs() };
        ox = nx * m * len * width;
        oz = nz * m * len * half;
    }
    if button {
        oz = F(half) * end;
    } else {
        if plain_smash && oz * end < 0.0 {
            oz = F::ZERO;
        }
        if drop {
            oz = -F(half) * end;
        }
    }
    let (mut tx, mut tz) = (ox + base[0], oz + base[1]);
    // the widest angle off straight
    let mut a = F(if four { 3.0 } else { 0.0 });
    a += s.con[match b {
        branch::SERVE => AimStats::SERVE,
        branch::GROUND => AimStats::STROKE,
        _ => AimStats::VOLLEY,
    }] as f32;
    if plain_smash && h.offset.abs() > 1 {
        let z = h.from[1].abs().clamp(3.0, BASELINE);
        a = a * F(1.0).msub(0.4, (F(z) - 3.0) / 8.885);
    }
    if (b == branch::GROUND && (h.kind == kind::SLICE || h.kind == kind::LOB)) || rally_kind(kind::LOB) {
        a = F(90.0);
    } else {
        if incoming.is_none() {
            if h.body {
                a = a * s.body[if b == branch::GROUND { AimStats::STROKE } else { AimStats::VOLLEY }] as f32 / 100.0;
            }
            if (b == branch::GROUND || b == branch::VOLLEY) && h.height < s.under {
                a = a * s.rising as f32 / 100.0;
            }
        }
        a = match incoming {
            Some(false) => a * 0.6,
            Some(true) => a * 0.45,
            None if drop => a * 0.5,
            None => a,
        };
    }
    let a = a.max(0.0) * DEG;
    let (dx, dz) = (tx - h.from[0], tz - h.from[1]);
    let inv = 1.0 / (dz * dz + dx * dx).sqrt();
    if a < acosf((F(end) * (dz * inv)).clamp(-1.0, 1.0).0) {
        let w = (tz - h.from[1]).abs() * tanf(a.0);
        tx = if h.from[0] < tx { F(h.from[0]) + w } else { F(h.from[0]) - w };
    }
    // an angled drive keeps the straight line's depth
    if plain_smash || (rally && h.kind <= kind::SLICE) {
        let (dx, dz) = (tx - h.from[0], tz - h.from[1]);
        let r = (dz * dz).sqrt() / (dz * dz + dx * dx).sqrt();
        tx = F(h.from[0]) + dx * r;
        tz = F(h.from[1]) + dz * r;
    }
    let short = near - tz.abs();
    if short > 0.0 {
        let (dx, dz) = (tx - h.from[0], tz - h.from[1]);
        tz = near * end;
        if dx != 0.0 {
            tx += short * dx / dz.abs();
        }
    }
    [tx.0, 0.0, tz.0]
}

/// The court margins `inside` pulls a rally shot's aim by (`hst_data::exe::Game::court_margins`).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
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
        let abs = |v: F| if v < 0.0 { -v } else { v };
        let hi = F(sideline(doubles));
        let lo = -hi;
        let row = fs(self.lines[class as usize][kind as usize]);
        let (mut mx, mut mz) = if low { (row[2], row[3]) } else { (row[0], row[1]) };
        let topspin = class == class::STROKE && kind == kind::TOPSPIN;
        let (hit, aim) = (fs(hit), fs(aim));
        let (dx, dz) = (aim[0] - hit[0], aim[2] - hit[2]);
        let inv = 1.0 / (dz * dz).madd(dx, dx).sqrt();
        let step = [(F::ZERO + hit[0]).madd(dx * inv, 18.0), (F::ZERO + hit[2]).madd(dz * inv, 18.0)];
        let outward = topspin && ((step[0] < lo - mx && hit[0] < lo) || (hi + mx < step[0] && hi < hit[0]));
        let p = if outward {
            step
        } else {
            let across = (aim[0] < 0.0 && lo < hit[0]) || (0.0 < aim[0] && hit[0] < hi);
            if across && !special {
                mx = mx * abs(dx * inv);
                mz = mz * abs(dz * inv);
            }
            let extra = if topspin && across { self.angled(lo, hi, character, hit, aim) } else { F::ZERO };
            if mx <= extra {
                mx = extra;
            }
            [aim[0], aim[2]]
        };
        let (x0, x1) = (lo + mx, hi - mx);
        let x = if p[0] < x0 { x0 } else if p[0] <= x1 { p[0] } else { x1 };
        let z1 = BASELINE - mz;
        let z = if p[1] < -z1 { -z1 } else if p[1] <= z1 { p[1] } else { z1 };
        f32s([aim[0] + (x - p[0]), aim[1], aim[2] + (z - p[1])])
    }

    /// The topspin's widened sideline margin for a short angled ball (0 when it isn't one).
    fn angled(&self, lo: F, hi: F, character: usize, hit: [F; 3], aim: [F; 3]) -> F {
        use crate::libm::{atanf, tanf};
        let abs = |v: F| if v < 0.0 { -v } else { v };
        let cx = if aim[0] < lo { lo } else if aim[0] <= hi { aim[0] } else { hi };
        let (dx, dz) = (cx - hit[0], aim[2] - hit[2]);
        let len = F::ZERO.madd(dx, dx).madd(dz, dz).sqrt();
        // from the hit to where the shot crosses the net
        let net = len - len * aim[2] / dz;
        let deep = abs(aim[2]);
        let far = F(3.0) + 3.0;
        if net <= 9.0 || !(deep < far) {
            return F::ZERO;
        }
        let width = F(self.angle[character]) * tanf(atanf((abs(dx) / abs(dz)).0));
        let mut short = (deep - 3.0) / (far - 3.0);
        if short < 0.0 {
            short = F::ZERO;
        }
        let short = 1.0 - short;
        // the path length of the shot's last 3 m of depth: 3 m straight, longer the more it angles
        let off = (F::ZERO + aim[2]).msub(3.0, if aim[2] < 0.0 { -1.0 } else { 1.0 });
        let (hz, az) = (hit[2] - off, aim[2] - off);
        let (ex, ez) = (cx - hit[0], az - hz);
        let run = abs(F::ZERO.madd(ex, ex).madd(ez, ez).sqrt() * az / ez);
        let straight = if run <= 3.0 {
            F::ONE
        } else {
            let q = (run - 3.0) / (F(5.0) - 3.0);
            1.0 - if q <= 1.0 { q } else { F::ONE }
        };
        let ramp = if net < 15.0 {
            let q = (net - 9.0) / (F(15.0) - 9.0);
            if q <= 1.0 { q } else { F::ONE }
        } else {
            let q = (net - 15.0) / (F(19.0) - 15.0);
            1.0 - if q <= 1.0 { q } else { F::ONE }
        };
        width * (ramp * (short * straight))
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
