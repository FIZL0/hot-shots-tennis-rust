//! The original's in-match camera (mode 0, the broadcast view from behind the −z baseline). Every frame it
//! rebuilds a target view from the mode's orbit, then:
//! 1. dollies straight back (smoothed) until the near team's deepest player stands above the bottom edge;
//! 2. zooms in, keeping the top edge, until the bottom edge is halfway to the lowest point it must show (4 m
//!    behind a near player, or the ball's spot on the court);
//! 3. slides sideways and widens so the court (±5 m) and every player (±1.7 m) fit across;
//! then eases the shown view 10 % toward that target and finally widens upward if the ball would leave the top
//! 2 % of the picture. Game space (Y down); the camera's rows are right, down, forward.
//! Arithmetic through `ps2::F` (the FPU's rounding; transcendentals from `musl`, the same on every platform), but
//! not the game's exact sequence: the camera never feeds back into play.

#![deny(clippy::float_arithmetic)]

use crate::ps2::F;

/// The field of view is the horizontal half-angle of the 4:3 picture; the vertical is scaled by this.
pub const ASPECT: f32 = 0.7;

/// Mode 0's orbit: distance to the court centre, pitch and the target's framing offset (fraction of the
/// half-height), and its horizontal half-angle.
const DISTANCE: f32 = 41.0;
const PITCH_DEG: f32 = 17.0;
const FRAMING: f32 = 0.02;
const FOV_DEG: f32 = 20.0;
/// Dolly smoothing, view easing, top margin.
const DOLLY_EASE: f32 = 0.1;
const VIEW_EASE: f32 = 0.1;
const TOP_MARGIN: f32 = 0.98;
/// Depth kept in view behind the near players, and the ball spot's depth clamp (the court's half-length).
const BEHIND: f32 = 4.0;
const HALF_LENGTH: f32 = 11.885;
/// Court half-width and per-player half-width that must fit across.
const COURT_SIDE: f32 = 5.0;
const PLAYER_SIDE: f32 = 1.7;

type V3 = [f32; 3];

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    /// Rows: right, down, forward (camera to world).
    pub rot: [V3; 3],
    pub eye: V3,
    /// Horizontal half-angle (radians).
    pub fov: f32,
}

impl View {
    /// World point in camera space (x right, y down, z depth).
    pub fn local(&self, p: V3) -> V3 {
        let d = sub(p, self.eye);
        [dot(self.rot[0], d), dot(self.rot[1], d), dot(self.rot[2], d)]
    }
    fn vertical(&self) -> f32 {
        musl::atanf((F(musl::tanf(self.fov)) * ASPECT).0)
    }
    /// Pitch the view about its right axis by `a` (positive tilts the forward axis down).
    fn tilt(&mut self, a: f32) {
        let (s, c) = musl::sincosf(a);
        let (d, f) = (self.rot[1], self.rot[2]);
        self.rot[1] = add(scale(d, c), scale(f, -F(s)));
        self.rot[2] = add(scale(f, c), scale(d, s));
    }
}

/// The camera's state between frames.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Camera {
    pub view: View,
    dolly: f32,
    /// The next frame shows its target view directly (no easing).
    cut: bool,
    /// Looking from behind the +z baseline.
    pub turned: bool,
}

/// What the camera frames: player positions (index = player number) and the ball.
pub struct Scene<'a> {
    pub players: &'a [V3],
    pub ball: V3,
}

/// Mode 0's resting view (also the view while the serve is set up), behind the −z baseline.
pub fn base() -> View {
    let pitch = PITCH_DEG.to_radians();
    let fov = (F(FOV_DEG) * 0.5).0.to_radians(); // the table holds the full angle
    let vertical = musl::atanf((F(musl::tanf(fov)) * ASPECT).0);
    let lift = musl::atanf((FRAMING * F(musl::tanf(vertical))).0);
    let forward = [0.0, musl::sinf(pitch), musl::cosf(pitch)];
    let orbit = (F(pitch) - lift).0;
    View {
        rot: [[1.0, 0.0, 0.0], [0.0, musl::cosf(pitch), (-F(musl::sinf(pitch))).0], forward],
        eye: [0.0, (-F(DISTANCE) * musl::sinf(orbit)).0, (-F(DISTANCE) * musl::cosf(orbit)).0],
        fov,
    }
}

/// The resting view from behind the +z baseline instead (the game turns the camera round when its human
/// player is on the +z half).
pub fn base_turned() -> View {
    let v = base();
    let t = |a: V3| [(-F(a[0])).0, a[1], (-F(a[2])).0];
    View { rot: [t(v.rot[0]), t(v.rot[1]), t(v.rot[2])], eye: t(v.eye), fov: v.fov }
}

impl Camera {
    pub fn new() -> Self {
        Camera { view: base(), dolly: 0.0, cut: true, turned: false }
    }

    /// Start from a view (e.g. a recorded one) with the dolly at rest.
    pub fn from_view(view: View) -> Self {
        Camera { view, dolly: 0.0, cut: false, turned: false }
    }

    /// A new point is set up: the next frame cuts to its view and the dolly starts from rest.
    pub fn cut(&mut self) {
        self.cut = true;
        self.dolly = 0.0;
    }

    /// One frame.
    pub fn step(&mut self, s: &Scene) {
        let mut v = if self.turned { base_turned() } else { base() };
        self.dolly_back(&mut v, s);
        fit_bottom(&mut v, s);
        fit_sides(&mut v, s);
        self.view = if std::mem::take(&mut self.cut) { v } else { ease(self.view, v, VIEW_EASE) };
        fit_ball_top(&mut self.view, s.ball);
    }

    /// The near team: whoever of player 0's team stands on the camera's side of the court centre.
    fn dolly_back(&mut self, v: &mut View, s: &Scene) {
        let deepest = near_team(v, s).map(|p| (p, v.local(p))).min_by(|a, b| a.1[2].total_cmp(&b.1[2]));
        let Some((p, _)) = deepest else { return };
        let vertical = v.vertical();
        let pitch = musl::asinf(v.rot[2][1]);
        // horizontal distance at which the frame's bottom edge meets the ground
        let reach = (-F(v.eye[1]) / musl::tanf((F(vertical) + pitch).0)).0;
        let ahead = norm([v.rot[2][0], 0.0, v.rot[2][2]]);
        let along = dot(sub(p, v.eye), ahead);
        let want = if along < reach { (F(along) - reach).0 } else { 0.0 };
        self.dolly = (F(self.dolly) + DOLLY_EASE * (F(want) - self.dolly)).0;
        // the eye moves along the forward axis with its height part dropped, not renormalised
        v.eye = add(v.eye, scale([v.rot[2][0], 0.0, v.rot[2][2]], self.dolly));
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

fn near_team<'a>(v: &View, s: &'a Scene) -> impl Iterator<Item = V3> + 'a {
    let first = s.players.first().copied().unwrap_or([0.0; 3]);
    let team = if dot(first, v.rot[2]) <= 0.0 { 0 } else { 1 };
    s.players.iter().copied().skip(team).step_by(2)
}

/// Zoom in keeping the top edge: the bottom edge moves halfway toward the lowest point that must stay in view.
fn fit_bottom(v: &mut View, s: &Scene) {
    let forward_sign = if v.rot[2][2] < 0.0 { -1.0 } else { 1.0 };
    let mut lowest = near_team(v, s)
        .map(|p| v.local([p[0], p[1], (F(p[2]) - F(BEHIND) * forward_sign).0]))
        .min_by(|a, b| a[2].total_cmp(&b[2]))
        .unwrap_or([0.0, 0.0, 1.0]);
    if s.players.len() > 1 {
        let b = v.local([s.ball[0], 0.0, s.ball[2].clamp((-F(HALF_LENGTH)).0, HALF_LENGTH)]);
        if F(lowest[1]) / lowest[2] < F(b[1]) / b[2] {
            lowest = b;
        }
    }
    let angle = musl::atanf((F(lowest[1]) / lowest[2]).0);
    let vertical = v.vertical();
    let mid = (F(vertical) + angle) * 0.5;
    if mid < vertical {
        v.tilt((mid - vertical).0);
        v.fov = musl::atanf((F(musl::tanf(mid.0)) / ASPECT).0);
    }
}

/// Points that must fit across: the court's sides at the centre, and each player ± their width.
fn across(s: &Scene) -> Vec<V3> {
    let mut pts = vec![[(-F(COURT_SIDE)).0, 0.0, 0.0], [COURT_SIDE, 0.0, 0.0]];
    for p in s.players {
        pts.push([(F(p[0]) - PLAYER_SIDE).0, p[1], p[2]]);
        pts.push([(F(p[0]) + PLAYER_SIDE).0, p[1], p[2]]);
    }
    pts
}

/// Slide sideways by what sticks out left plus what sticks out right, then widen to fit whatever still does.
fn fit_sides(v: &mut View, s: &Scene) {
    let pts = across(s);
    let (mut left, mut right) = (0.0f32, 0.0f32);
    for &p in &pts {
        let c = v.local(p);
        let a = musl::atanf((F(c[0]) / c[2]).0);
        if v.fov < -F(a) {
            left = left.min((-(F(c[2]) * musl::tanf((-F(a) - v.fov).0))).0);
        }
        if v.fov < a {
            right = right.max((F(c[2]) * musl::tanf((F(a) - v.fov).0)).0);
        }
    }
    v.eye = add(v.eye, scale(v.rot[0], F(left) + right));
    let widest = pts.iter().map(|&p| {
        let c = v.local(p);
        musl::atanf((F(c[0]) / c[2]).0).abs()
    });
    let widest = widest.fold(0.0f32, f32::max);
    if v.fov < widest {
        v.fov = widest;
    }
}

/// If the ball is above the top 2 % of the picture, tilt up and widen so the top edge covers half the excess
/// (the bottom edge stays).
fn fit_ball_top(v: &mut View, ball: V3) {
    let vertical = v.vertical();
    let c = v.local(ball);
    let depth = c[2].abs();
    let top = F(TOP_MARGIN) * depth * musl::tanf(vertical);
    let over = -F(c[1]) - top;
    if over > 0.0 {
        let edge = musl::atanf((top / depth).0);
        let delta = F(musl::atanf(((over * 0.5 + top) / depth).0)) - edge;
        v.fov = musl::atanf((F(musl::tanf((F(vertical) + delta).0)) / ASPECT).0);
        v.tilt((-delta).0);
    }
}

/// Ease `from` toward `to` by `t`: rotation by normalised blend of the axes, eye and field of view linearly.
fn ease(from: View, to: View, t: f32) -> View {
    let mix = |a: V3, b: V3| add(a, scale(sub(b, a), t));
    let f = norm(mix(from.rot[2], to.rot[2]));
    let d0 = mix(from.rot[1], to.rot[1]);
    let r = norm(cross(d0, f));
    let d = cross(f, r);
    View { rot: [r, d, f], eye: mix(from.eye, to.eye), fov: (F(from.fov) + (F(to.fov) - from.fov) * t).0 }
}

/// Ticks of the change-ends phase's camera move (the phase is as long).
pub const SWING: u32 = 80;

/// The change-ends move's progress with `left` of its [`SWING`] ticks still to go (counted down before use,
/// so the first tick has 79 left): t = 1 − left/80 eased as t^(2·(1−t)²), slow out, fast through the middle.
pub fn swing_weight(left: u32) -> f32 {
    let t = (1.0 - F(left as f32) / SWING as f32).0;
    musl::powf(musl::powf(t, musl::powf((1.0 - F(t)).0, 2.0)), 2.0)
}

/// The change-ends move: the view the phase started from swung toward this tick's `to` by `w`, as the game's
/// camera does it: both views taken apart as orbit parameters in a reference frame (`to`'s rotation at `pivot`):
/// the look's pitch and heading as differences from the frame's (the short way round), the pivot's angles as
/// seen from the eye in the look's own axes, its distance and the field of view; the parts blended linearly and
/// the view rebuilt the way a scripted shot builds its orbit (`cutaway::Shot::build`).
pub fn swing(from: &View, to: &View, w: f32, pivot: V3) -> View {
    use crate::cutaway::{M4, diff, mul, pitch, rot_x, rot_y};
    let v4 = |v: V3| [v[0], v[1], v[2], 0.0];
    let frame: M4 = [v4(to.rot[0]), v4(to.rot[1]), v4(to.rot[2]), [pivot[0], pivot[1], pivot[2], 1.0]];
    let yaw = |f: [f32; 4]| musl::atan2f(f[0], f[2]);
    // ponytail: the game also takes the look's roll apart and blends it; both views are upright (roll 0)
    let parts = |v: &View| {
        let d = sub(pivot, v.eye);
        let l = v4(std::array::from_fn(|i| dot(d, v.rot[i])));
        let (f, r) = (v4(v.rot[2]), frame[2]);
        let (lp, ly) = (pitch(l), yaw(l));
        [
            diff(pitch(f), pitch(r)),
            diff(yaw(f), yaw(r)),
            musl::atanf((F(musl::tanf(lp)) / musl::cosf(ly)).0),
            ly,
            F(dot(d, d)).sqrt().0,
            v.fov,
        ]
    };
    let (a, b) = (parts(from), parts(to));
    let [p, h, ep, ey, d, fov]: [f32; 6] = std::array::from_fn(|i| ((F(b[i]) - a[i]) * w + a[i] + 0.0).0);
    let cam = mul(&mul(&frame, &rot_y(h)), &rot_x(p));
    let f = cam[2];
    let cam = mul(&rot_y(yaw(f)), &rot_x(pitch(f)));
    let o = mul(&mul(&cam, &rot_y(ey)), &rot_x(musl::atanf((F(musl::tanf(ep)) * musl::cosf(ey)).0)));
    let r = |i: usize| [cam[i][0], cam[i][1], cam[i][2]];
    View { rot: [r(0), r(1), r(2)], eye: std::array::from_fn(|i| (F(pivot[i]) - F(o[2][i]) * d).0), fov }
}

fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| (F(a[i]) + b[i]).0)
}
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| (F(a[i]) - b[i]).0)
}
fn scale(a: V3, k: impl Into<F>) -> V3 {
    let k = k.into();
    std::array::from_fn(|i| (F(a[i]) * k).0)
}
fn dot(a: V3, b: V3) -> f32 {
    (F(a[0]) * b[0] + F(a[1]) * b[1] + F(a[2]) * b[2]).0
}
fn cross(a: V3, b: V3) -> V3 {
    let [a, b] = [a, b].map(|v| v.map(F));
    [(a[1] * b[2] - a[2] * b[1]).0, (a[2] * b[0] - a[0] * b[2]).0, (a[0] * b[1] - a[1] * b[0]).0]
}
fn norm(a: V3) -> V3 {
    scale(a, 1.0 / F(dot(a, a)).sqrt())
}
