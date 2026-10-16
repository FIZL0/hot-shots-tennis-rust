//! Ball flight and ground bounces. The original integrates in f32 with this exact operation order;
//! keep it, rounding is gameplay. Game space is Y-down: the court is the plane y = 0, height is negative y.
#![deny(clippy::float_arithmetic)]

use crate::ps2::{F, f32s, fs};

pub type V3 = [f32; 3];
pub use crate::vu0::V4;

/// 9.8 m/s² expressed per 60 Hz frame².
pub const GRAVITY_PER_FRAME: f32 = 0.0027222224;

/// Per-ball constants that stay fixed for a whole flight.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Material {
    /// The playing surface: bounces by the court's `Surface`.
    pub court: bool,
    /// Soft obstacles (the net): counted separately; the first touch also kills most sliding speed.
    pub special: bool,
    pub restitution: f32,
    /// Spin lost per bounce, and how fast spin relaxes, off the court surface.
    pub spin_loss: f32,
    /// Its row in the table (material id).
    pub id: u8,
    /// Bounce effect kind and court program 2 sound key (`sound::Bounces`).
    pub effect: u8,
    pub sound: u8,
}

/// Bounce effect kinds (`Material::effect`, 1..=15; 0 none) with rules of their own: the net's, and grass the ball
/// rolls on.
pub const EFFECT_NET: u8 = 2;
pub const EFFECT_GRASS: u8 = 5;

/// The court plane.
pub const COURT: Material = Material { court: true, special: false, restitution: 0.0, spin_loss: 0.0, id: 1, effect: 0, sound: 0 };

/// The net, as the game's own path predictor sees it: a plane at z = 0 up to 0.91 m, across both courts' width.
// ponytail: the live ball queries the court collision mesh (`step_world`); this flat net is the predictor's view.
pub const NET_TOP: f32 = 0.91;
pub const NET_HALF_WIDTH: f32 = 6.4;
pub const NET: Material = Material { court: false, special: true, restitution: 0.18, spin_loss: 0.95, id: 2, effect: EFFECT_NET, sound: 2 };
/// After touching the net the ball is pushed this much per frame away from it so it cannot stick.
const NET_PUSH: f32 = 0.003;

/// Shot classes (`Shot::class`) and kinds (`Shot::kind`) the bounce reads: ground strokes and volleys, sliced or
/// dropped.
const STROKE: u8 = 1;
const VOLLEY: u8 = 2;
const SLICE: i32 = 1;
const DROP: i32 = 4;

/// The court plane for the sweep: a point on it and its normal into the ground (game space is Y-down).
const ORIGIN: V4 = [0.0, 0.0, 0.0, 1.0];
const INTO_GROUND: V4 = [-0.0, -1.0, -0.0, -0.0];
/// A frame is swept in at most this many sub-steps, and stops once no more than `k::MIN_REMAINDER` of it is left.
const MAX_SUBSTEPS: i32 = 30;
/// The game's predicted path: steps filled at the hit, then each frame, and the contacts that end it.
const PATH_FIRST_STEPS: u32 = 14;
const PATH_STEPS: u32 = 15;
const PATH_CONTACTS: i32 = 2;

/// Everything decided when the ball is struck.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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
    /// Shot class (0 serve, 1 stroke, 2 volley, 3 smash) and kind (0 topspin, 1 slice, 2 flat, 3 lob, 4 drop); a
    /// sliced or dropped stroke or volley softens the first bounce's spin kick.
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

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
        let dt = F(dt);
        let g = dt * (GRAVITY_PER_FRAME * F(p.gravity));
        let v = fs(self.vel).map(|c| c * dt);
        let sq = |v: [F; 3]| (F::ZERO + v[1] * v[1] + v[0] * v[0]).madd(v[2], v[2]);
        let s = sq(v).sqrt();
        if s == 0.0 {
            self.vel = f32s(v);
            return;
        }
        // drag: v − ((v·(1/|v|))·|v|)·|v|·k
        let inv = 1.0 / sq(v).sqrt();
        let v = v.map(|c| (F::ZERO + c).msub(c * inv * s * s, p.drag));
        // Magnus: c = axis × v, w = v × c, v += (((w/|w|)·s)·spin)·magnus
        let [ax, ay, az] = fs(p.axis);
        let c = [(ay * v[2]).msub(az, v[1]), (az * v[0]).msub(ax, v[2]), (ax * v[1]).msub(ay, v[0])];
        let w = [(v[1] * c[2]).msub(v[2], c[1]), (v[2] * c[0]).msub(v[0], c[2]), (v[0] * c[1]).msub(v[1], c[0])];
        let winv = 1.0 / (w[1] * w[1] + w[0] * w[0]).madd(w[2], w[2]).sqrt();
        let mut v = [0, 1, 2].map(|i| (F::ZERO + v[i]).madd(w[i] * winv * s * self.spin, p.magnus));
        v[1] += g;
        self.vel = f32s(v);
    }

    /// One airborne frame with no contact and no curve: semi-implicit Euler.
    pub fn fly(&mut self, p: &Params) {
        self.accelerate(p, 1.0);
        self.pos = [0, 1, 2].map(|i| (F(self.pos[i]) + self.vel[i]).0);
    }
}

/// The drawn ball's model matrix after one ball step (+0xb0, the end of the original's step): a ball with spin and a
/// spin axis (`frame`'s forward row) turns its rotation R to R·Fᵀ·Rx(−θ)·F (F the spin frame, θ = spin·`dt` wrapped to
/// ±π), rebuilt orthonormal from its forward row; any ball is placed at `pos`.
pub fn spin_model(model: &[V4; 4], frame: &[V4; 4], spin: f32, dt: f32, pos: V3) -> [V4; 4] {
    use crate::{vu0, world};
    let mut m = *model;
    let a = fs(frame[2]);
    if spin != 0.0 && (a[2] * a[2]).madd(a[0], a[0]).madd(a[1], a[1]) + 0.0 + 0.0 != 0.0 {
        let t: [V4; 4] = std::array::from_fn(|r| std::array::from_fn(|c| frame[c][r]));
        let mut th = F(spin) * dt;
        if world::PI < th {
            th -= world::TWO_PI;
        } else if th < -F(world::PI) {
            th += world::TWO_PI;
        }
        m[3] = [0.0, 0.0, 0.0, 1.0];
        let r = world::mat_mul(&world::mat_mul(&world::mat_mul(&m, &t), &world::rot_x((-th).0)), frame);
        let z = vu0::normalize(r[2]);
        let x = vu0::normalize(vu0::cross(r[1], z));
        let y = vu0::normalize(vu0::cross(z, x));
        m = [x, y, z, r[3]];
    }
    m[3] = [pos[0], pos[1], pos[2], 1.0];
    m
}

const IDENTITY: [V4; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

/// `rotation`'s first three rows with `pos` as the fourth (w = 1).
fn placed(rotation: &[V4; 4], pos: V3) -> [V4; 4] {
    [rotation[0], rotation[1], rotation[2], [pos[0], pos[1], pos[2], 1.0]]
}

/// Rows of a 4×4 from three basis rows (w = 0) plus the homogeneous row, as the game stores frames.
pub fn rows4(r: [V3; 3]) -> [V4; 4] {
    let w = |v: V3| [v[0], v[1], v[2], 0.0];
    [w(r[0]), w(r[1]), w(r[2]), [0.0, 0.0, 0.0, 1.0]]
}

/// A ball in play from the moment it was struck. Field comments give the original's ball-object offsets.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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
    /// The latest counted bounce's contact record (+0x8f8, one per bounce): the point on the surface and its material.
    pub landing: ([f32; 3], Material),
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
    /// The drawn ball's model matrix (+0xb0): its spin rotation, turned every step (`spin_model`), and its position.
    pub model: [V4; 4],
    /// The ball's last move (+0x140): the launch velocity, then each normal step's position change; kept while
    /// the ball stands or takes a plane step.
    pub last: V3,
}

/// Turns `d` by `pitch` and `yaw` (radians), keeping its length: pitch clamped to ±π/2, heading wrapped to ±π.
fn turn(d: V4, pitch: f32, yaw: f32) -> V4 {
    use crate::{libm::atan2f, world};
    const HALF_PI: F = F(world::HALF_PI);
    let (pi, two_pi) = (F(world::PI), F(world::TWO_PI));
    let d = fs(d);
    let p = F(atan2f((-d[1]).0, (d[2] * d[2]).madd(d[0], d[0]).sqrt().0)) + pitch;
    let p = if p < -HALF_PI { -HALF_PI } else if p <= HALF_PI { p } else { HALF_PI };
    let mut y = F(atan2f(d[0].0, d[2].0)) + yaw;
    if !(y <= pi) {
        y -= two_pi;
    } else if y < -pi {
        y = two_pi + y;
    }
    let m = world::mat_mul(&world::mat_mul(&world::IDENTITY, &world::rot_x(p.0)), &world::rot_y(y.0));
    let len = (d[1] * d[1]).madd(d[0], d[0]).madd(d[2], d[2]).sqrt();
    m[2].map(|c| (c * len).0)
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
    /// A plane step (`Flight::step_plane`): responds as a first special touch.
    plane: bool,
}

/// The normal a plane step that misses its plane responds to: what the original's hit record holds there, never
/// written by the sweep (recorded on court 7: a creature struck by the falling serve toss).
const MISSED_NORMAL: V4 = [f32::from_bits(0x0098_8fcc), 0.0, 0.0, 0.0];

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
    pub const SLICE_KICK: f32 = 0.5;
    pub const DROP_KICK: f32 = 0.1;
    pub const SLICE_RESTITUTION: f32 = 1.0;
    pub const DROP_RESTITUTION: f32 = 1.0;
    pub const MIN_REMAINDER: f32 = 0.1;
    pub const UP: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
}

impl Flight {
    pub fn new(ball: Ball, spin_frame: [V4; 4], contact: [V4; 4]) -> Self {
        Self { ball, frame: 0, bounces: 0, contacts: 0, special_contacts: 0, rolling: false, landing: ([0.0; 3], COURT), spin_frame, contact, slide: 0.0, net: true, lines: None, call: crate::judge::Call::None, line_distance: 0.0, in_play: true, model: placed(&IDENTITY, ball.pos), last: ball.vel }
    }

    /// A struck ball: as `new`, with the model's rotation set to the shot's spin frame (the hit sets both).
    pub fn struck(ball: Ball, spin_frame: [V4; 4], contact: [V4; 4]) -> Self {
        Self { model: placed(&spin_frame, ball.pos), ..Self::new(ball, spin_frame, contact) }
    }

    /// Court contacts the original's predicted path holds `t` frames after this (just struck) ball left the
    /// racket: the path is stepped against the court only, 14 steps on the hit and 15 more each frame, and stops
    /// filling once it reaches two contacts. A net touch re-seeds the path from the ball but keeps this count.
    pub fn predicted_contacts(&self, shot: &Shot, surface: &Surface, t: u32) -> i32 {
        let mut f = *self;
        f.net = false;
        for _ in 0..PATH_FIRST_STEPS + PATH_STEPS * t {
            if f.contacts >= PATH_CONTACTS {
                break;
            }
            f.step(shot, surface);
        }
        f.contacts
    }

    /// Advance one 60 Hz frame on `surface`, in the original's order: accelerate, add wind and the curve
    /// profile (first flight only), then sweep the step against the court (and the net when live), placing
    /// the ball at each contact and responding, for up to 30 sub-steps while more than 10% of the frame remains.
    pub fn step(&mut self, shot: &Shot, surface: &Surface) {
        let (r, net) = (shot.params.radius, self.net);
        self.advance(shot, surface, |pos, target, _| {
            let ground = crate::contact::sweep(pos, target, r, ORIGIN, INTO_GROUND, f32::MAX);
            let mut hit = ground.map(|h| (h, [h.centre[0], 0.0, h.centre[2]], COURT, false));
            if net {
                let side = if pos[2] > 0.0 { 1.0 } else if pos[2] < 0.0 { -1.0 } else { 0.0 };
                if side != 0.0 {
                    let best = hit.map_or(f32::MAX, |(h, ..)| h.t);
                    if let Some(h) = crate::contact::sweep(pos, target, r, ORIGIN, [0.0, 0.0, side, 0.0], best) {
                        if h.centre[1] > -F(NET_TOP) && h.centre[0].abs() <= NET_HALF_WIDTH {
                            hit = Some((h, [h.centre[0], h.centre[1], 0.0], NET, false));
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
            // the contact record's point is where the ball is placed for the hit (`contact_point`) less radius ×
            // normal, in world space (B85: not the model-space point carried through the node, which differs in the
            // last bits off the net's tilted faces; B134: nor the hit's centre, which differs off a prop's)
            world.sweep(pos, target, r, ignore).map(|h| {
                let c = crate::contact::contact_point(Some(&h.contact()), pos, target, r);
                let p = std::array::from_fn(|k| (F(c[k]) + 0.0 - F(h.normal[k]) * r).0);
                (h.contact(), p, materials[h.material as usize], h.material == GHOST)
            })
        });
    }

    /// The extra step the original gives the ball when it hits a player: no acceleration, wind or curve; the step
    /// is swept once against the plane through its end point facing back along the ball's heading (`material`,
    /// the table's material 0), and responds as a first special touch, also when the sweep misses (a slow ball
    /// such as a falling toss: then off `MISSED_NORMAL`, which stops it dead). The ball stays where it is; only its
    /// velocity (and spin, frame, counts) change. No line calls.
    pub fn step_plane(&mut self, shot: &Shot, surface: &Surface, material: Material) {
        let v = fs(self.ball.vel);
        let inv = 1.0 / (v[2] * v[2]).madd(v[0], v[0]).sqrt();
        let n: V4 = f32s([-(v[0] * inv), F(-0.0), -(v[2] * inv), F(-0.0)]);
        let pos: V4 = [self.ball.pos[0], self.ball.pos[1], self.ball.pos[2], 1.0];
        let d: V4 = f32s([v[0], v[1], v[2], F::ZERO]);
        let end: V4 = std::array::from_fn(|k| (F::ZERO + pos[k]).madd(d[k], 1.0).0);
        // the original responds whether or not the sweep met the plane; on a miss its hit record keeps the normal
        // its stack held (MISSED_NORMAL) and the contact point is the step's end
        let h = crate::contact::sweep(pos, end, shot.params.radius, end, n, f32::MAX);
        {
            let mut f = FrameFlags { plane: true, ..Default::default() };
            let d = self.respond(shot, surface, d, h.map_or(MISSED_NORMAL, |h| h.normal), material, false, &mut f);
            if f.counted {
                // the record holds the ball's centre at the contact
                let c = crate::contact::contact_point(h.as_ref(), pos, end, shot.params.radius);
                self.landing = ([c[0], c[1], c[2]], material);
            }
            let d = if self.in_play && !self.rolling && self.bounces == 1 && shot.bounce_turn != 0.0 { turn(d, 0.0, shot.bounce_turn) } else { d };
            self.ball.vel = [d[0], d[1], d[2]];
            if f.special {
                self.ball.vel[2] = (F(self.ball.vel[2]) + F(pos[2].signum()) * NET_PUSH).0;
            }
        }
        self.model = spin_model(&self.model, &self.spin_frame, self.ball.spin, 1.0, self.ball.pos);
        self.frame += 1;
    }

    /// The frame itself; `query(start, end, ghost touches so far)` finds the nearest contact of a sub-step: the
    /// hit, the point on the surface, its material and whether it is the ghost material (passed through, not counted).
    fn advance(&mut self, shot: &Shot, surface: &Surface, mut query: impl FnMut(V4, V4, i32) -> Option<(crate::contact::Hit, [f32; 3], Material, bool)>) {
        let r = shot.params.radius;
        self.ball.accelerate(&shot.params, 1.0);
        let v = self.ball.vel;
        let mut d: V4 = [v[0], v[1], v[2], 0.0];
        if self.bounces == 0 {
            for k in 0..3 {
                d[k] = (F(d[k]) + F(shot.wind[k]) * 1.0).0;
            }
            // curve profile: Δ of sin(π·progress) over this frame, progress capped at 1
            let n = self.frame + 1;
            let total = F(shot.curve_frames.max(n) as f32);
            let a = (F((n - 1) as f32) / total).min(1.0);
            let b = ((F((n - 1) as f32) + 1.0) / total).min(1.0);
            let pi = std::f32::consts::PI;
            let dk = F(crate::libm::sinf((pi * b).0)) - crate::libm::sinf((pi * a).0);
            let up = [0.0, 1.0, 0.0, 0.0];
            let side = [shot.side[0], shot.side[1], shot.side[2], 0.0];
            for k in 0..4 {
                d[k] = (F::ZERO + d[k]).madd(F(up[k]) * shot.curve, dk).0;
            }
            for k in 0..4 {
                d[k] = (F::ZERO + d[k]).madd(F(side[k]) * shot.bend, dk).0;
            }
        }
        let mut pos: V4 = [self.ball.pos[0], self.ball.pos[1], self.ball.pos[2], 1.0];
        let mut remaining = F::ONE;
        let mut contacts = 0;
        let mut f = FrameFlags::default();
        loop {
            let target: V4 = std::array::from_fn(|k| (F::ZERO + pos[k]).madd(d[k], remaining).0);
            let Some((h, point, material, ghost)) = query(pos, target, f.ghosts) else {
                pos = target;
                break;
            };
            contacts += 1;
            pos = crate::contact::contact_point(Some(&h), pos, target, r);
            if ghost {
                f.ghosts += 1;
            }
            let counted = f.counted;
            d = self.respond(shot, surface, d, h.normal, material, ghost, &mut f);
            if f.counted && !counted {
                self.landing = (point, material);
            }
            if self.in_play && contacts == 1 && !self.rolling && self.bounces == 1 && shot.bounce_turn != 0.0 {
                d = turn(d, 0.0, shot.bounce_turn);
            }
            if let Some(l) = self.lines.filter(|_| !self.rolling && (self.bounces == 1 || self.contacts == 1)) {
                if let Some((call, dist)) = crate::judge::call_landing(self.call, self.special_contacts != 0, pos, &l) {
                    self.call = call;
                    self.line_distance = dist.unwrap_or(self.line_distance);
                }
            }
            remaining = remaining * (1.0 - F(h.t));
            if remaining <= k::MIN_REMAINDER || contacts >= MAX_SUBSTEPS {
                break;
            }
        }
        if contacts != 0 {
            self.ball.vel = [d[0], d[1], d[2]];
        }
        if f.special {
            self.ball.vel[2] = (F(self.ball.vel[2]) + F(pos[2].signum()) * NET_PUSH).0;
        }
        let old = self.ball.pos;
        self.ball.pos = [pos[0], pos[1], pos[2]];
        self.last = std::array::from_fn(|k| (F(pos[k]) - old[k]).0);
        self.model = spin_model(&self.model, &self.spin_frame, self.ball.spin, 1.0, self.ball.pos);
        self.frame += 1;
    }

    /// Contact response to a surface with normal `n` (pointing out of the surface toward the ball, as the
    /// sweep reports it); returns the new velocity. `counted`: the bounce counters were already advanced this frame.
    fn respond(&mut self, shot: &Shot, s: &Surface, d: V4, n: V4, material: Material, ghost: bool, f: &mut FrameFlags) -> V4 {
        use crate::{quat, vu0};
        let r = F(shot.params.radius);
        let court = material.court;
        if material.special {
            // both stay set for the rest of the frame
            f.special = true;
            f.special_first = self.special_contacts == 0;
            self.special_contacts += 1;
        }
        let (special, special_first) = (f.special, f.special_first || f.plane);
        let (d, n) = (fs(d), fs(n));
        let sq = |v: [F; 4]| (v[1] * v[1]).madd(v[0], v[0]).madd(v[2], v[2]);
        let dot = (d[1] * n[1]).madd(d[0], n[0]).madd(d[2], n[2]);
        let mut vn = n.map(|c| c * dot);
        let mut vt: [F; 4] = std::array::from_fn(|k| d[k] - vn[k]);
        if self.bounces == 0 && !(shot.first_bounce_spin <= 0.0) {
            self.ball.spin = shot.first_bounce_spin;
        }
        let normal_speed = sq(vn).sqrt();
        let mut old_spin = F(self.ball.spin);
        // rolling: settled on the court after two contacts (|n.y| ≥ sin 45°, not a special surface)
        let rolling = self.contacts >= 2 && normal_speed <= k::ROLL_SPEED && !(n[1].abs() < musl::sinf(std::f32::consts::FRAC_PI_4)) && !special;
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
            vn = [F::ZERO; 4];
        }
        self.rolling = rolling;

        // friction
        let ratio = normal_speed * s.friction / sq(vt).sqrt();
        vt = if !(ratio < 1.0) { [F::ZERO; 4] } else { vt.map(|c| (F::ZERO + c).msub(c, ratio)) };
        if special_first {
            vt = vt.map(|c| c * k::SPECIAL_FIRST_SLIDE);
        }
        self.slide = (sq(vt).sqrt() / r).0;
        if sq(vt) != 0.0 {
            let up = vu0::normalize(f32s(n.map(|c| -c)));
            let side = vu0::normalize(vu0::cross(up, f32s(vt)));
            let fwd = vu0::normalize(vu0::cross(side, up));
            self.contact = [side, up, fwd, self.contact[3]];
        }
        let spin_loss = F(material.spin_loss);
        if !rolling && !court {
            // the game keeps this reduced spin as the "spin before" the kick is measured from
            self.ball.spin = (F(self.ball.spin) * (1.0 - spin_loss)).0;
            old_spin = F(self.ball.spin);
        }

        // spin drags the slide toward rolling speed (in the contact frame)
        let spin_r = F(self.ball.spin) * r;
        let slide_r = F(self.slide) * r;
        let transpose = |m: &[V4; 4]| -> [V4; 4] { std::array::from_fn(|j| std::array::from_fn(|k| m[k][j])) };
        let sa = fs(vu0::transform(&transpose(&self.contact), self.spin_frame[0]));
        let [ax, ay, az, _] = fs(k::UP);
        let cx = (sa[1] * az).msub(sa[2], ay);
        let cz = (sa[0] * ay).msub(sa[1], ax);
        let kk = F(if rolling { k::ROLL_SPIN_TO_SPEED } else { s.spin_to_speed });
        let tx = (F::ZERO + 0.0).madd(cx * spin_r - 0.0, kk);
        let ty = (F::ZERO + 0.0).madd(0.0, kk);
        let tz = (F::ZERO + slide_r).madd(cz * spin_r - slide_r, kk);
        let tw = (F::ZERO + 0.0).madd(0.0 * spin_r - 0.0, kk);
        let vt = vu0::transform(&self.contact, f32s([tx, ty, tz, tw]));

        // the surface pulls spin toward rolling spin
        let along = F(vu0::transform(&transpose(&self.spin_frame), vt)[2]);
        let target = along / r;
        let relax = F(if court { if rolling { k::ROLL_RELAX } else { s.spin_relax } } else { spin_loss.0 });
        self.ball.spin = (F::ZERO + self.ball.spin).madd(relax, target - self.ball.spin).0;

        // spin frame turns toward the contact frame by slide / (slide + |spin|)
        let level = |side: V4| -> [V4; 4] {
            let r0 = vu0::normalize(side);
            let r2 = vu0::normalize(vu0::cross(r0, k::UP));
            let r1 = vu0::normalize(vu0::cross(r2, r0));
            [r0, r1, r2, [0.0, 0.0, 0.0, 1.0]]
        };
        let l1 = level(self.spin_frame[0]);
        let mut c = fs(self.contact[0]);
        if (c[1] * l1[0][1]).madd(c[0], l1[0][0]).madd(c[2], l1[0][2]) < 0.0 {
            c = c.map(|x| x * -1.0);
        }
        let l2 = level(f32s(c));
        let mut turn = if self.ball.spin == 0.0 {
            F::ONE
        } else {
            k::TURN_BASE * (F(self.slide) / (F(self.slide) + self.ball.spin.abs()))
        };
        if !(turn <= k::TURN_CAP) {
            turn = F(k::TURN_CAP);
        }
        self.spin_frame = quat::to_matrix(quat::slerp(quat::from_matrix(&l1), quat::from_matrix(&l2), turn.0));

        // restitution and the spin kick off the surface
        let mut e = F(if special_first {
            if material.restitution == 0.0 { k::SPECIAL_DEFAULT_RESTITUTION } else { material.restitution }
        } else if court {
            s.restitution
        } else {
            material.restitution
        });
        let mut kick = F(if special_first { k::SPECIAL_KICK } else { s.spin_kick });
        if self.bounces == 1 {
            if matches!(shot.class, STROKE | VOLLEY) {
                match shot.kind {
                    SLICE => e = e * k::SLICE_RESTITUTION,
                    DROP => e = e * k::DROP_RESTITUTION,
                    _ => {}
                }
            }
            if shot.first_bounce_restitution != 0.0 {
                e = e * shot.first_bounce_restitution;
            }
            if matches!(shot.class, STROKE | VOLLEY) {
                match shot.kind {
                    SLICE => kick = kick * k::SLICE_KICK,
                    DROP => kick = kick * k::DROP_KICK,
                    _ => {}
                }
            }
        }
        let vn = if ghost {
            // passes through, slowed
            vn.map(|c| c * e)
        } else {
            let ne = -e;
            let vn = vn.map(|c| c * ne);
            let change = (F(self.ball.spin) - old_spin).abs();
            let inv = 1.0 / sq(vn).sqrt();
            vn.map(|c| (F::ZERO + c).madd(c * inv * change, kick))
        };
        if !rolling && court {
            self.ball.spin = (F(self.ball.spin) * (1.0 - F(s.spin_relax))).0;
        }
        let vt = fs(vt);
        f32s(std::array::from_fn(|k| vn[k] + vt[k]))
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
