//! The original's in-match camera (mode 0, the broadcast view from behind the −z baseline). Every frame it
//! rebuilds a target view from the mode's orbit, then:
//! 1. dollies straight back (smoothed) until the near team's deepest player stands above the bottom edge;
//! 2. zooms in, keeping the top edge, until the bottom edge is halfway to the lowest point it must show (4 m
//!    behind a near player, or the ball's spot on the court);
//! 3. slides sideways and widens so the court (±5 m) and every player (±1.7 m) fit across;
//! then eases the shown view 10 % toward that target and finally widens upward if the ball would leave the top
//! 2 % of the picture. Game space (Y down); the camera's rows are right, down, forward.
//! Float math is plain f32 here, not the PS2 sequence: the camera never feeds back into play.

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

#[derive(Clone, Copy, Debug, PartialEq)]
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
        (self.fov.tan() * ASPECT).atan()
    }
    /// Pitch the view about its right axis by `a` (positive tilts the forward axis down).
    fn tilt(&mut self, a: f32) {
        let (s, c) = a.sin_cos();
        let (d, f) = (self.rot[1], self.rot[2]);
        self.rot[1] = add(scale(d, c), scale(f, -s));
        self.rot[2] = add(scale(f, c), scale(d, s));
    }
}

/// The camera's state between frames.
#[derive(Clone, Copy, Debug)]
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
    let fov = (FOV_DEG * 0.5).to_radians(); // the table holds the full angle
    let vertical = (fov.tan() * ASPECT).atan();
    let lift = (FRAMING * vertical.tan()).atan();
    let forward = [0.0, pitch.sin(), pitch.cos()];
    let orbit = pitch - lift;
    View {
        rot: [[1.0, 0.0, 0.0], [0.0, pitch.cos(), -pitch.sin()], forward],
        eye: [0.0, -DISTANCE * orbit.sin(), -DISTANCE * orbit.cos()],
        fov,
    }
}

/// The resting view from behind the +z baseline instead (the game turns the camera round when its human
/// player is on the +z half).
pub fn base_turned() -> View {
    let v = base();
    let t = |a: V3| [-a[0], a[1], -a[2]];
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
        let pitch = v.rot[2][1].asin();
        // horizontal distance at which the frame's bottom edge meets the ground
        let reach = -v.eye[1] / (vertical + pitch).tan();
        let ahead = norm([v.rot[2][0], 0.0, v.rot[2][2]]);
        let along = dot(sub(p, v.eye), ahead);
        let want = if along < reach { along - reach } else { 0.0 };
        self.dolly += DOLLY_EASE * (want - self.dolly);
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
        .map(|p| v.local([p[0], p[1], p[2] - BEHIND * forward_sign]))
        .min_by(|a, b| a[2].total_cmp(&b[2]))
        .unwrap_or([0.0, 0.0, 1.0]);
    if s.players.len() > 1 {
        let b = v.local([s.ball[0], 0.0, s.ball[2].clamp(-HALF_LENGTH, HALF_LENGTH)]);
        if lowest[1] / lowest[2] < b[1] / b[2] {
            lowest = b;
        }
    }
    let angle = (lowest[1] / lowest[2]).atan();
    let vertical = v.vertical();
    let mid = (vertical + angle) * 0.5;
    if mid < vertical {
        v.tilt(mid - vertical);
        v.fov = (mid.tan() / ASPECT).atan();
    }
}

/// Points that must fit across: the court's sides at the centre, and each player ± their width.
fn across(s: &Scene) -> Vec<V3> {
    let mut pts = vec![[-COURT_SIDE, 0.0, 0.0], [COURT_SIDE, 0.0, 0.0]];
    for p in s.players {
        pts.push([p[0] - PLAYER_SIDE, p[1], p[2]]);
        pts.push([p[0] + PLAYER_SIDE, p[1], p[2]]);
    }
    pts
}

/// Slide sideways by what sticks out left plus what sticks out right, then widen to fit whatever still does.
fn fit_sides(v: &mut View, s: &Scene) {
    let pts = across(s);
    let (mut left, mut right) = (0.0f32, 0.0f32);
    for &p in &pts {
        let c = v.local(p);
        let a = (c[0] / c[2]).atan();
        if v.fov < -a {
            left = left.min(-(c[2] * (-a - v.fov).tan()));
        }
        if v.fov < a {
            right = right.max(c[2] * (a - v.fov).tan());
        }
    }
    v.eye = add(v.eye, scale(v.rot[0], left + right));
    let widest = pts.iter().map(|&p| {
        let c = v.local(p);
        (c[0] / c[2]).atan().abs()
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
    let top = TOP_MARGIN * depth * vertical.tan();
    let over = -c[1] - top;
    if over > 0.0 {
        let edge = (top / depth).atan();
        let delta = ((over * 0.5 + top) / depth).atan() - edge;
        v.fov = ((vertical + delta).tan() / ASPECT).atan();
        v.tilt(-delta);
    }
}

/// Ease `from` toward `to` by `t`: rotation by normalised blend of the axes, eye and field of view linearly.
fn ease(from: View, to: View, t: f32) -> View {
    let mix = |a: V3, b: V3| add(a, scale(sub(b, a), t));
    let f = norm(mix(from.rot[2], to.rot[2]));
    let d0 = mix(from.rot[1], to.rot[1]);
    let r = norm(cross(d0, f));
    let d = cross(f, r);
    View { rot: [r, d, f], eye: mix(from.eye, to.eye), fov: from.fov + (to.fov - from.fov) * t }
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: V3) -> V3 {
    scale(a, 1.0 / dot(a, a).sqrt())
}
