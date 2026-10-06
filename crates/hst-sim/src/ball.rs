//! Ball flight and ground bounces. The original integrates in f32 with this exact operation order;
//! keep it, rounding is gameplay. Game space is Y-down: the court is the plane y = 0, height is negative y.

pub type V3 = [f32; 3];

/// 9.8 m/s² expressed per 60 Hz frame².
pub const GRAVITY_PER_FRAME: f32 = 0.0027222224;

/// Per-ball constants that stay fixed for a whole flight.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Quadratic drag coefficient (0.04 for the regular ball).
    pub drag: f32,
    /// Gravity multiplier (0.9 for the regular ball).
    pub gravity: f32,
    /// Magnus strength (0.0015).
    pub magnus: f32,
    /// Spin axis in world space; (0, 1, 0) for normal shots.
    pub axis: V3,
    /// Collision radius; also the lever arm between spin and surface speed.
    pub radius: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self { drag: 0.04, gravity: 0.9, magnus: 0.0015, axis: [0.0, 1.0, 0.0], radius: 0.064 }
    }
}

/// How a court surface treats a bouncing ball (one entry per court in the original's tables).
#[derive(Clone, Copy, Debug)]
pub struct Surface {
    /// Coulomb friction: tangential speed lost per unit of normal speed.
    pub friction: f32,
    /// How far spin moves toward the rolling spin on contact (and the share it then loses).
    pub spin_relax: f32,
    /// How strongly spin drags tangential speed toward rolling speed.
    pub spin_to_speed: f32,
    pub restitution: f32,
    /// Extra bounce height per unit of spin change.
    pub spin_kick: f32,
}

/// Surfaces of the 12 courts, indexed like the original (court 0..11).
pub const COURTS: [Surface; 12] = {
    const RELAX: [f32; 12] = [0.4, 0.1, 0.1, 0.1, 0.4, 0.3, 0.3, 0.4, 0.3, 0.1, 0.3, 0.4];
    const KICK: [f32; 12] = [0.012, 0.012, 0.012, 0.012, 0.012, 0.003, 0.003, 0.012, 0.003, 0.012, 0.003, 0.012];
    let mut out = [Surface { friction: 0.3, spin_relax: 0.0, spin_to_speed: 0.2, restitution: 0.7, spin_kick: 0.0 }; 12];
    let mut i = 0;
    while i < 12 {
        out[i].spin_relax = RELAX[i];
        out[i].spin_kick = KICK[i];
        i += 1;
    }
    out
};

/// Used once the ball is rolling instead of bouncing.
const ROLL_RELAX: f32 = 0.1;
const ROLL_SPIN_TO_SPEED: f32 = 0.0;
/// Below this normal speed (after the second contact) a contact becomes rolling.
const ROLL_SPEED: f32 = 0.02;
/// A sub-step that leaves no more than this share of the frame ends the frame at the contact point.
const MIN_REMAINDER: f32 = 0.1;

/// Everything decided when the ball is struck.
#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub params: Params,
    /// Vertical dip/rise spread over the flight as a half sine, until the first bounce.
    pub curve: f32,
    /// Sideways bend along `side`, same profile as `curve`.
    pub bend: f32,
    pub side: V3,
    /// Frames the curve profile is stretched over (planned frames to the first bounce).
    pub curve_frames: i32,
    pub wind: V3,
    /// Spin forced at the first bounce: positive replaces spin before contact, negative after it.
    pub first_bounce_spin: f32,
    /// Restitution multiplier for the first bounce (0 = none).
    pub first_bounce_restitution: f32,
    /// Shot class and kind; kinds 1 and 4 of classes 1–2 soften the first bounce's spin kick.
    pub class: u8,
    pub kind: i32,
}

impl Default for Shot {
    fn default() -> Self {
        Self {
            params: Params::default(),
            curve: 0.0,
            bend: 0.0,
            side: [0.0; 3],
            curve_frames: 0,
            wind: [0.0; 3],
            first_bounce_spin: 0.0,
            first_bounce_restitution: 0.0,
            class: 0,
            kind: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub pos: V3,
    /// Displacement per frame.
    pub vel: V3,
    /// Signed spin rate about the spin frame's side axis.
    pub spin: f32,
}

fn len(v: V3) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn scale(v: V3, s: f32) -> V3 {
    v.map(|c| c * s)
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn unit(v: V3) -> V3 {
    scale(v, 1.0 / len(v))
}

/// Level frame (side, up, forward) built from a side axis and the world up axis, as the original rebuilds it.
fn level_frame(side: V3, axis: V3) -> [V3; 3] {
    let side = unit(side);
    let fwd = unit(cross(side, axis));
    [side, unit(cross(fwd, side)), fwd]
}

/// Rotate the spin frame's side axis toward `target` (flipped onto the same half) by `t`, about `axis`.
/// The original slerps quaternions of the two level frames; both turn about `axis`, so this is the same rotation.
fn turn_toward(from: V3, target: V3, axis: V3, t: f32) -> [V3; 3] {
    let a = level_frame(from, axis);
    let target = if dot(target, a[0]) < 0.0 { scale(target, -1.0) } else { target };
    let b = level_frame(target, axis);
    let angle = dot(cross(a[0], b[0]), a[1]).atan2(dot(a[0], b[0])) * t;
    let (s, c) = angle.sin_cos();
    level_frame(add(scale(a[0], c), scale(a[2], -s)), axis)
}

impl Ball {
    /// Velocity update for one airborne frame: drag, Magnus, gravity. `dt` is the slow-motion scale (1 normally).
    pub fn accelerate(&mut self, p: &Params, dt: f32) {
        let v = self.vel.map(|c| c * dt);
        let s = len(v);
        if s == 0.0 {
            self.vel = v;
            return;
        }
        let inv = 1.0 / s;
        let v = v.map(|c| c - c * inv * s * s * p.drag);
        // lift acts along v × (axis × v), i.e. perpendicular to travel within the spin plane
        let w = cross(v, cross(p.axis, v));
        let wl = 1.0 / len(w);
        let mut v = [0, 1, 2].map(|i| w[i] * wl * s * self.spin * p.magnus + v[i]);
        v[1] += dt * p.gravity * GRAVITY_PER_FRAME;
        self.vel = v;
    }

    /// One airborne frame with no contact and no curve: semi-implicit Euler.
    pub fn fly(&mut self, p: &Params) {
        self.accelerate(p, 1.0);
        self.pos = add(self.pos, self.vel);
    }
}

/// A ball in play from the moment it was struck.
#[derive(Clone, Copy, Debug)]
pub struct Flight {
    pub ball: Ball,
    /// Frames since the shot.
    pub frame: i32,
    pub bounces: i32,
    /// Contacts that counted toward rolling (bounces against the court).
    pub contacts: i32,
    pub rolling: bool,
    /// Orientation of the ball's spin: rows side, up, forward. Turned toward the contact frame on each bounce.
    pub spin_frame: [V3; 3],
    /// Last contact frame (side, up, forward); reused when the ball meets the court without sliding.
    pub contact: [V3; 3],
}

impl Flight {
    pub fn new(ball: Ball, spin_frame: [V3; 3], contact: [V3; 3]) -> Self {
        Self { ball, frame: 0, bounces: 0, contacts: 0, rolling: false, spin_frame, contact }
    }

    /// Advance one 60 Hz frame on `surface`.
    pub fn step(&mut self, shot: &Shot, surface: &Surface) {
        let r = shot.params.radius;
        self.ball.accelerate(&shot.params, 1.0);
        let mut disp = add(self.ball.vel, shot.wind);
        if self.bounces == 0 && (shot.curve != 0.0 || shot.bend != 0.0) {
            let total = shot.curve_frames.max(self.frame + 1) as f32;
            let a = (self.frame as f32 / total).min(1.0);
            let b = ((self.frame as f32 + 1.0) / total).min(1.0);
            let k = (b * std::f32::consts::PI).sin() - (a * std::f32::consts::PI).sin();
            disp[1] += shot.curve * k;
            disp = add(disp, scale(shot.side, shot.bend * k));
        }
        let mut remaining = 1.0f32;
        for _ in 0..30 {
            let target = add(self.ball.pos, scale(disp, remaining));
            if target[1] <= -r {
                self.ball.pos = target;
                break;
            }
            // swept sphere vs the court plane
            let t = (-r - self.ball.pos[1]) / (target[1] - self.ball.pos[1]);
            self.ball.pos = add(self.ball.pos, scale(sub(target, self.ball.pos), t));
            disp = self.bounce(shot, surface, disp);
            self.ball.vel = disp;
            remaining *= 1.0 - t;
            if remaining <= MIN_REMAINDER {
                break;
            }
        }
        self.frame += 1;
    }

    /// Contact response against the flat court; returns the new velocity.
    fn bounce(&mut self, shot: &Shot, s: &Surface, v: V3) -> V3 {
        let r = shot.params.radius;
        let n: V3 = [0.0, -1.0, 0.0];
        let first = self.bounces == 0;
        let vn = scale(n, dot(v, n));
        let vt = sub(v, vn);
        if first && shot.first_bounce_spin > 0.0 {
            self.ball.spin = shot.first_bounce_spin;
        }
        let old_spin = self.ball.spin;
        let normal_speed = len(vn);
        let rolling = self.contacts >= 2 && normal_speed <= ROLL_SPEED && n[1].abs() >= std::f32::consts::FRAC_1_SQRT_2;
        if !rolling {
            self.bounces += 1;
            self.contacts += 1;
        }
        self.rolling = rolling;
        if self.bounces == 1 && shot.first_bounce_spin < 0.0 {
            self.ball.spin = shot.first_bounce_spin;
        }
        let vn = if rolling { [0.0; 3] } else { vn };

        // friction on the tangential part
        let vtl = len(vt);
        let f = if vtl == 0.0 { f32::INFINITY } else { normal_speed * s.friction / vtl };
        let vt = if f >= 1.0 { [0.0; 3] } else { sub(vt, scale(vt, f)) };
        let speed = len(vt);

        // contact frame: up off the surface, forward along the slide
        if speed != 0.0 {
            let up = unit(scale(n, -1.0));
            let side = unit(cross(up, vt));
            self.contact = [side, up, unit(cross(side, up))];
        }
        let [side, up, fwd] = self.contact;

        // spin pulls the tangential speed toward rolling speed
        let a = self.spin_frame[0];
        let sa = [dot(side, a), dot(up, a), dot(fwd, a)];
        let k = if rolling { ROLL_SPIN_TO_SPEED } else { s.spin_to_speed };
        let sr = self.ball.spin * r;
        let tx = (-sa[2] * sr) * k;
        let tz = (sa[0] * sr - speed) * k + speed;
        let vt = add(scale(side, tx), scale(fwd, tz));

        // and the surface pulls spin toward rolling spin
        let relax = if rolling { ROLL_RELAX } else { s.spin_relax };
        let along = dot(self.spin_frame[2], vt);
        self.ball.spin = relax * (along / r - self.ball.spin) + self.ball.spin;
        let turn = if self.ball.spin == 0.0 { 1.0 } else { (speed / r / (speed / r + self.ball.spin.abs())).min(1.0) };
        self.spin_frame = turn_toward(self.spin_frame[0], side, shot.params.axis, turn);

        let mut e = s.restitution;
        let mut kick = s.spin_kick;
        if first {
            if matches!(shot.class, 1 | 2) && shot.kind == 1 {
                kick *= 0.5;
            } else if matches!(shot.class, 1 | 2) && shot.kind == 4 {
                kick *= 0.1;
            }
            if shot.first_bounce_restitution != 0.0 {
                e *= shot.first_bounce_restitution;
            }
        }
        let mut vn = scale(vn, -e);
        let nl = len(vn);
        if nl != 0.0 {
            vn = add(vn, scale(vn, (self.ball.spin - old_spin).abs() * kick / nl));
        }
        if !rolling {
            self.ball.spin *= 1.0 - relax; // a real bounce sheds this share of spin; rolling contact does not
        }
        add(vn, vt)
    }
}
