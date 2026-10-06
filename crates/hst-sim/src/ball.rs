//! Ball flight and ground bounces. The original integrates in f32 with this exact operation order;
//! keep it, rounding is gameplay. Game space is Y-down: the court is the plane y = 0, height is negative y.

pub type V3 = [f32; 3];
pub use crate::vu0::V4;

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

/// How a collision material treats the ball (one row of the game's material table).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// The playing surface: bounces by the court's `Surface`.
    pub court: bool,
    /// Soft obstacles (the net): counted separately; the first touch also kills most sliding speed.
    pub special: bool,
    pub restitution: f32,
    /// Spin lost per bounce, and how fast spin relaxes, off the court surface.
    pub spin_loss: f32,
}

/// The court plane.
pub const COURT: Material = Material { court: true, special: false, restitution: 0.0, spin_loss: 0.0 };

/// The net, as the game's own path predictor sees it: a plane at z = 0 up to 0.91 m, across both courts' width.
// ponytail: the live ball queries the court collision mesh (`step_world`); this flat net is the predictor's view.
pub const NET_TOP: f32 = 0.91;
pub const NET_HALF_WIDTH: f32 = 6.4;
pub const NET: Material = Material { court: false, special: true, restitution: 0.18, spin_loss: 0.95 };
/// After touching the net the ball is pushed this much per frame away from it so it cannot stick.
const NET_PUSH: f32 = 0.003;

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
    /// Turn of the velocity's heading (radians) at the first bounce (+0x1b0; some characters' serves).
    pub bounce_turn: f32,
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
            bounce_turn: 0.0,
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

impl Ball {
    /// Velocity update for one airborne frame: drag, Magnus, gravity. `dt` is the slow-motion scale (1 normally).
    pub fn accelerate(&mut self, p: &Params, dt: f32) {
        // The original's instruction sequence, on PS2 float arithmetic (see `ps2`): every product, sum and
        // accumulator step in the same order, so recorded flights reproduce bit for bit.
        use crate::ps2::{add, div, madd, msub, mul, sqrt};
        let g = mul(dt, mul(GRAVITY_PER_FRAME, p.gravity));
        let v = self.vel.map(|c| mul(c, dt));
        let sq = |v: V3| madd(add(add(0.0, mul(v[1], v[1])), mul(v[0], v[0])), v[2], v[2]);
        let s = sqrt(sq(v));
        if s == 0.0 {
            self.vel = v;
            return;
        }
        // drag: v − ((v·(1/|v|))·|v|)·|v|·k
        let inv = div(1.0, sqrt(sq(v)));
        let v = v.map(|c| msub(add(0.0, c), mul(mul(mul(c, inv), s), s), p.drag));
        // Magnus: c = axis × v, w = v × c, v += (((w/|w|)·s)·spin)·magnus
        let [ax, ay, az] = p.axis;
        let c = [msub(mul(ay, v[2]), az, v[1]), msub(mul(az, v[0]), ax, v[2]), msub(mul(ax, v[1]), ay, v[0])];
        let w = [msub(mul(v[1], c[2]), v[2], c[1]), msub(mul(v[2], c[0]), v[0], c[2]), msub(mul(v[0], c[1]), v[1], c[0])];
        let winv = div(1.0, sqrt(madd(add(mul(w[1], w[1]), mul(w[0], w[0])), w[2], w[2])));
        let mut v = [0, 1, 2].map(|i| madd(add(0.0, v[i]), mul(mul(mul(w[i], winv), s), self.spin), p.magnus));
        v[1] = add(v[1], g);
        self.vel = v;
    }

    /// One airborne frame with no contact and no curve: semi-implicit Euler.
    pub fn fly(&mut self, p: &Params) {
        self.accelerate(p, 1.0);
        self.pos = [0, 1, 2].map(|i| crate::ps2::add(self.pos[i], self.vel[i]));
    }
}

/// Rows of a 4×4 from three basis rows (w = 0) plus the homogeneous row, as the game stores frames.
pub fn rows4(r: [V3; 3]) -> [V4; 4] {
    let w = |v: V3| [v[0], v[1], v[2], 0.0];
    [w(r[0]), w(r[1]), w(r[2]), [0.0, 0.0, 0.0, 1.0]]
}

/// A ball in play from the moment it was struck. Field comments give the original's ball-object offsets.
#[derive(Clone, Copy, Debug)]
pub struct Flight {
    pub ball: Ball,
    /// Frames since the shot (+0xac).
    pub frame: i32,
    /// Bounces, counted once per frame (+0x224).
    pub bounces: i32,
    /// Court contacts that count toward rolling (+0x228).
    pub contacts: i32,
    /// Touches of special materials such as the net (+0x22c).
    pub special_contacts: i32,
    /// Last contact was rolling rather than a bounce (+0xa4 == 2).
    pub rolling: bool,
    /// Orientation of the ball's spin, 4 rows: side, up, forward, homogeneous (+0x160).
    pub spin_frame: [V4; 4],
    /// Last contact frame, 4 rows (+0x1c0); reused when the ball meets a surface without sliding.
    pub contact: [V4; 4],
    /// Sliding speed over the radius at the last contact (+0x200).
    pub slide: f32,
    /// Collide with the net. The live ball does; the game's stored path (what the AI reads) is stepped
    /// against the court plane only, so replays of recorded paths turn this off.
    pub net: bool,
    /// Make line calls at the first landing (the live ball in a rally, and the game's path predictor).
    pub lines: Option<crate::judge::Lines>,
    /// Line call so far (+0xa5).
    pub call: crate::judge::Call,
    /// Signed distance from the called landing to the nearest line (+0x230).
    pub line_distance: f32,
    /// A rally is on (shots this rally > 0); the first-bounce turn only happens then.
    pub in_play: bool,
}

/// Turns `d` by `pitch` and `yaw` (radians), keeping its length: pitch clamped to ±π/2, heading wrapped to ±π.
fn turn(d: V4, pitch: f32, yaw: f32) -> V4 {
    use crate::{libm::atan2f, ps2, world};
    const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
    let (pi, two_pi) = (f32::from_bits(0x4049_0fdb), f32::from_bits(0x40c9_0fdb));
    let p = ps2::add(atan2f(-d[1], ps2::sqrt(ps2::madd(ps2::mul(d[2], d[2]), d[0], d[0]))), pitch);
    let p = if p < -HALF_PI { -HALF_PI } else if p <= HALF_PI { p } else { HALF_PI };
    let mut y = ps2::add(atan2f(d[0], d[2]), yaw);
    if !(y <= pi) {
        y = ps2::sub(y, two_pi);
    } else if y < -pi {
        y = ps2::add(two_pi, y);
    }
    let m = world::mat_mul(&world::mat_mul(&world::IDENTITY, &world::rot_x(p)), &world::rot_y(y));
    let len = ps2::sqrt(ps2::madd(ps2::madd(ps2::mul(d[1], d[1]), d[0], d[0]), d[2], d[2]));
    m[2].map(|c| ps2::mul(c, len))
}

/// Contact state that lasts for the whole frame across sub-steps.
#[derive(Default)]
struct FrameFlags {
    /// The bounce counters were advanced.
    counted: bool,
    /// A special material was touched (it then counts as special for rolling and court contacts).
    special: bool,
    /// The last special touch was the flight's first.
    special_first: bool,
    /// Touches of the ghost material.
    ghosts: i32,
}

/// Constants of the original's contact code (exact bit patterns).
mod k {
    pub const ROLL_SPEED: f32 = 0.02;
    pub const SPECIAL_FIRST_SLIDE: f32 = 0.1;
    pub const SPECIAL_KICK: f32 = 0.1;
    pub const SPECIAL_DEFAULT_RESTITUTION: f32 = 0.05;
    pub const ROLL_RELAX: f32 = 0.1;
    pub const ROLL_SPIN_TO_SPEED: f32 = 0.0;
    pub const TURN_BASE: f32 = 1.0;
    pub const TURN_CAP: f32 = 1.0;
    pub const KIND1_KICK: f32 = 0.5;
    pub const KIND4_KICK: f32 = 0.1;
    pub const KIND1_RESTITUTION: f32 = 1.0;
    pub const KIND4_RESTITUTION: f32 = 1.0;
    pub const MIN_REMAINDER: f32 = 0.1;
    pub const UP: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
}

impl Flight {
    pub fn new(ball: Ball, spin_frame: [V4; 4], contact: [V4; 4]) -> Self {
        Self { ball, frame: 0, bounces: 0, contacts: 0, special_contacts: 0, rolling: false, spin_frame, contact, slide: 0.0, net: true, lines: None, call: crate::judge::Call::None, line_distance: 0.0, in_play: true }
    }

    /// Advance one 60 Hz frame on `surface`, in the original's order: accelerate, add wind and the curve
    /// profile (first flight only), then sweep the step against the court (and the net when live), placing
    /// the ball at each contact and responding, for up to 30 sub-steps while more than 10% of the frame remains.
    pub fn step(&mut self, shot: &Shot, surface: &Surface) {
        let (r, net) = (shot.params.radius, self.net);
        self.advance(shot, surface, |pos, target, _| {
            let ground = crate::contact::sweep(pos, target, r, [0.0, 0.0, 0.0, 1.0], [-0.0, -1.0, -0.0, -0.0], f32::MAX);
            let mut hit = ground.map(|h| (h, COURT, false));
            if net {
                let side = if pos[2] > 0.0 { 1.0 } else if pos[2] < 0.0 { -1.0 } else { 0.0 };
                if side != 0.0 {
                    let best = hit.map_or(f32::MAX, |(h, _, _)| h.t);
                    if let Some(h) = crate::contact::sweep(pos, target, r, [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, side, 0.0], best) {
                        if h.centre[1] > -NET_TOP && h.centre[0].abs() <= NET_HALF_WIDTH {
                            hit = Some((h, NET, false));
                        }
                    }
                }
            }
            hit
        });
    }

    /// One frame of the live ball against the world mesh (`step`, with every sub-step swept through `world`).
    /// `materials` is the game's collision material table, by material id.
    pub fn step_world(&mut self, shot: &Shot, surface: &Surface, world: &crate::mesh::World, materials: &[Material]) {
        use crate::mesh::GHOST;
        let r = shot.params.radius;
        self.advance(shot, surface, |pos, target, ghosts| {
            let ignore: &[u32] = if ghosts == 0 { &[] } else { &[GHOST] };
            world.sweep(pos, target, r, ignore).map(|h| (h.contact(), materials[h.material as usize], h.material == GHOST))
        });
    }

    /// The frame itself; `query(start, end, ghost touches so far)` finds the nearest contact of a sub-step: the
    /// hit, its material and whether it is the ghost material (passed through, not counted).
    fn advance(&mut self, shot: &Shot, surface: &Surface, mut query: impl FnMut(V4, V4, i32) -> Option<(crate::contact::Hit, Material, bool)>) {
        use crate::ps2;
        let r = shot.params.radius;
        self.ball.accelerate(&shot.params, 1.0);
        let v = self.ball.vel;
        let mut d: V4 = [v[0], v[1], v[2], 0.0];
        if self.bounces == 0 {
            for k in 0..3 {
                d[k] = ps2::add(d[k], ps2::mul(shot.wind[k], 1.0));
            }
            // curve profile: Δ of sin(π·progress) over this frame, progress capped at 1
            let n = self.frame + 1;
            let total = shot.curve_frames.max(n) as f32;
            let a = ps2::div((n - 1) as f32, total).min(1.0);
            let b = ps2::div(ps2::add((n - 1) as f32, 1.0), total).min(1.0);
            let pi = std::f32::consts::PI;
            let dk = ps2::sub(crate::libm::sinf(ps2::mul(pi, b)), crate::libm::sinf(ps2::mul(pi, a)));
            let up = [0.0, 1.0, 0.0, 0.0];
            let side = [shot.side[0], shot.side[1], shot.side[2], 0.0];
            for k in 0..4 {
                d[k] = ps2::madd(ps2::add(0.0, d[k]), ps2::mul(up[k], shot.curve), dk);
            }
            for k in 0..4 {
                d[k] = ps2::madd(ps2::add(0.0, d[k]), ps2::mul(side[k], shot.bend), dk);
            }
        }
        let mut pos: V4 = [self.ball.pos[0], self.ball.pos[1], self.ball.pos[2], 1.0];
        let mut remaining = 1.0f32;
        let mut contacts = 0;
        let mut f = FrameFlags::default();
        loop {
            let target: V4 = std::array::from_fn(|k| ps2::madd(ps2::add(0.0, pos[k]), d[k], remaining));
            let Some((h, material, ghost)) = query(pos, target, f.ghosts) else {
                pos = target;
                break;
            };
            contacts += 1;
            pos = crate::contact::contact_point(Some(&h), pos, target, r);
            if ghost {
                f.ghosts += 1;
            }
            d = self.respond(shot, surface, d, h.normal, material, ghost, &mut f);
            if self.in_play && contacts == 1 && !self.rolling && self.bounces == 1 && shot.bounce_turn != 0.0 {
                d = turn(d, 0.0, shot.bounce_turn);
            }
            if let Some(l) = self.lines.filter(|_| !self.rolling && (self.bounces == 1 || self.contacts == 1)) {
                if let Some((call, dist)) = crate::judge::call_landing(self.call, self.special_contacts != 0, pos, &l) {
                    self.call = call;
                    self.line_distance = dist.unwrap_or(self.line_distance);
                }
            }
            remaining = ps2::mul(remaining, ps2::sub(1.0, h.t));
            if remaining <= k::MIN_REMAINDER || contacts >= 30 {
                break;
            }
        }
        if contacts != 0 {
            self.ball.vel = [d[0], d[1], d[2]];
        }
        if f.special {
            self.ball.vel[2] = ps2::add(self.ball.vel[2], ps2::mul(pos[2].signum(), NET_PUSH));
        }
        self.ball.pos = [pos[0], pos[1], pos[2]];
        self.frame += 1;
    }

    /// Contact response to a surface with normal `n` (pointing out of the surface toward the ball, as the
    /// sweep reports it); returns the new velocity. `counted`: the bounce counters were already advanced this frame.
    fn respond(&mut self, shot: &Shot, s: &Surface, d: V4, n: V4, material: Material, ghost: bool, f: &mut FrameFlags) -> V4 {
        use crate::ps2::{add, div, madd, msub, mul, sqrt, sub};
        use crate::{quat, vu0};
        let r = shot.params.radius;
        let court = material.court;
        if material.special {
            // both stay set for the rest of the frame
            f.special = true;
            f.special_first = self.special_contacts == 0;
            self.special_contacts += 1;
        }
        let (special, special_first) = (f.special, f.special_first);
        let sq = |v: V4| madd(madd(mul(v[1], v[1]), v[0], v[0]), v[2], v[2]);
        let dot = madd(madd(mul(d[1], n[1]), d[0], n[0]), d[2], n[2]);
        let mut vn: V4 = n.map(|c| mul(c, dot));
        let mut vt: V4 = std::array::from_fn(|k| sub(d[k], vn[k]));
        if self.bounces == 0 && !(shot.first_bounce_spin <= 0.0) {
            self.ball.spin = shot.first_bounce_spin;
        }
        let normal_speed = sqrt(sq(vn));
        let mut old_spin = self.ball.spin;
        // rolling: settled on the court after two contacts (|n.y| ≥ sin 45°, not a special surface)
        let rolling = self.contacts >= 2 && normal_speed <= k::ROLL_SPEED && !(n[1].abs() < std::f32::consts::FRAC_PI_4.sin()) && !special;
        if !rolling {
            if !ghost && !f.counted {
                f.counted = true;
                self.bounces += 1;
                if !special {
                    self.contacts += 1;
                }
            }
            if self.bounces == 1 && shot.first_bounce_spin < 0.0 {
                self.ball.spin = shot.first_bounce_spin;
            }
        } else {
            vn = [0.0; 4];
        }
        self.rolling = rolling;

        // friction
        let ratio = div(mul(normal_speed, s.friction), sqrt(sq(vt)));
        vt = if !(ratio < 1.0) { [0.0; 4] } else { vt.map(|c| msub(add(0.0, c), c, ratio)) };
        if special_first {
            vt = vt.map(|c| mul(c, k::SPECIAL_FIRST_SLIDE));
        }
        self.slide = div(sqrt(sq(vt)), r);
        if sq(vt) != 0.0 {
            let up = vu0::normalize(n.map(|c| -c));
            let side = vu0::normalize(vu0::cross(up, vt));
            let fwd = vu0::normalize(vu0::cross(side, up));
            self.contact = [side, up, fwd, self.contact[3]];
        }
        let spin_loss = material.spin_loss;
        if !rolling && !court {
            // the game keeps this reduced spin as the "spin before" the kick is measured from
            self.ball.spin = mul(self.ball.spin, sub(1.0, spin_loss));
            old_spin = self.ball.spin;
        }

        // spin drags the slide toward rolling speed (in the contact frame)
        let spin_r = mul(self.ball.spin, r);
        let slide_r = mul(self.slide, r);
        let transpose = |m: &[V4; 4]| -> [V4; 4] { std::array::from_fn(|j| std::array::from_fn(|k| m[k][j])) };
        let sa = vu0::transform(&transpose(&self.contact), self.spin_frame[0]);
        let [ax, ay, az, _] = k::UP;
        let cx = msub(mul(sa[1], az), sa[2], ay);
        let cz = msub(mul(sa[0], ay), sa[1], ax);
        let kk = if rolling { k::ROLL_SPIN_TO_SPEED } else { s.spin_to_speed };
        let tx = madd(add(0.0, 0.0), sub(mul(cx, spin_r), 0.0), kk);
        let ty = madd(add(0.0, 0.0), 0.0, kk);
        let tz = madd(add(0.0, slide_r), sub(mul(cz, spin_r), slide_r), kk);
        let tw = madd(add(0.0, 0.0), sub(mul(0.0, spin_r), 0.0), kk);
        let vt = vu0::transform(&self.contact, [tx, ty, tz, tw]);

        // the surface pulls spin toward rolling spin
        let along = vu0::transform(&transpose(&self.spin_frame), vt)[2];
        let target = div(along, r);
        let relax = if court { if rolling { k::ROLL_RELAX } else { s.spin_relax } } else { spin_loss };
        self.ball.spin = madd(add(0.0, self.ball.spin), relax, sub(target, self.ball.spin));

        // spin frame turns toward the contact frame by slide / (slide + |spin|)
        let level = |side: V4| -> [V4; 4] {
            let r0 = vu0::normalize(side);
            let r2 = vu0::normalize(vu0::cross(r0, k::UP));
            let r1 = vu0::normalize(vu0::cross(r2, r0));
            [r0, r1, r2, [0.0, 0.0, 0.0, 1.0]]
        };
        let l1 = level(self.spin_frame[0]);
        let mut c = self.contact[0];
        if madd(madd(mul(c[1], l1[0][1]), c[0], l1[0][0]), c[2], l1[0][2]) < 0.0 {
            c = c.map(|x| mul(x, -1.0));
        }
        let l2 = level(c);
        let mut turn = if self.ball.spin == 0.0 {
            1.0
        } else {
            mul(k::TURN_BASE, div(self.slide, add(self.slide, self.ball.spin.abs())))
        };
        if !(turn <= k::TURN_CAP) {
            turn = k::TURN_CAP;
        }
        self.spin_frame = quat::to_matrix(quat::slerp(quat::from_matrix(&l1), quat::from_matrix(&l2), turn));

        // restitution and the spin kick off the surface
        let mut e = if special_first {
            if material.restitution == 0.0 { k::SPECIAL_DEFAULT_RESTITUTION } else { material.restitution }
        } else if court {
            s.restitution
        } else {
            material.restitution
        };
        let mut kick = if special_first { k::SPECIAL_KICK } else { s.spin_kick };
        if self.bounces == 1 {
            if matches!(shot.class, 1 | 2) {
                match shot.kind {
                    1 => e = mul(e, k::KIND1_RESTITUTION),
                    4 => e = mul(e, k::KIND4_RESTITUTION),
                    _ => {}
                }
            }
            if shot.first_bounce_restitution != 0.0 {
                e = mul(e, shot.first_bounce_restitution);
            }
            if matches!(shot.class, 1 | 2) {
                match shot.kind {
                    1 => kick = mul(kick, k::KIND1_KICK),
                    4 => kick = mul(kick, k::KIND4_KICK),
                    _ => {}
                }
            }
        }
        let vn: V4 = if ghost {
            // passes through, slowed
            vn.map(|c| mul(c, e))
        } else {
            let ne = -e;
            let vn: V4 = vn.map(|c| mul(c, ne));
            let change = sub(self.ball.spin, old_spin).abs();
            let inv = div(1.0, sqrt(sq(vn)));
            vn.map(|c| madd(add(0.0, c), mul(mul(c, inv), change), kick))
        };
        if !rolling && court {
            self.ball.spin = mul(self.ball.spin, sub(1.0, s.spin_relax));
        }
        std::array::from_fn(|k| add(vn[k], vt[k]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_drive_into_the_net_drops_back_on_the_hitters_side() {
        let frame = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mut f = Flight::new(Ball { pos: [0.0, -0.5, -3.0], vel: [0.0, 0.0, 0.4], spin: 0.0 }, rows4(frame), rows4(frame));
        for _ in 0..60 {
            f.step(&Shot::default(), &COURTS[0]);
        }
        assert!(f.special_contacts >= 1, "never touched the net");
        assert!(f.ball.pos[2] < 0.0, "went through: {:?}", f.ball.pos);
        assert!(f.ball.vel[2].abs() < 0.1, "net should kill the pace: {:?}", f.ball.vel);
    }
}
