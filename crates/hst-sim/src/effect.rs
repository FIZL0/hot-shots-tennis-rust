//! Effect models (`AZUMA/C_EFF/EFFCT.XB0`): a model played by key channels, each on its own clock — node
//! animation (`.ANI`, a hierarchy under a scene root), morph-target weights (`.MOR`) and material alpha (`.MTA`,
//! tracks named after the materials they drive). Starting an effect sets every clock to 0 and plays its first frame; each
//! frame advances them as a motion clock ([`Clock`], speed 1, clamped), the model's nodes first, then the
//! morphs, then the alphas. The effect ends the frame the morph clock's sampled time reaches the morph length (the
//! last key of its bound tracks, in frames): the frames before that are drawn.
//!
//! `.UVA` files sit beside some effects but never play: the game loads them into the alpha channel as a second
//! clip it never selects.

#![deny(clippy::float_arithmetic)]

use hst_data::{ani::Anim, mdl::Model, mor::Tracks, mtl::Material};

use crate::{
    face,
    motion::Clock,
    pose::{Clip, M4, Skeleton},
    ps2::{F, f32s, fs},
    world,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Track {
    /// Morph target or material index.
    target: usize,
    ticks: Vec<i32>,
    values: Vec<[f32; 4]>,
    cursor: usize,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Channel {
    tracks: Vec<Track>,
    ticks_per_frame: f32,
    pub(crate) length: f32,
    pub(crate) clock: Clock,
}

impl Channel {
    /// Tracks whose name `bind` finds, in file order.
    pub(crate) fn new(t: &Tracks, bind: impl Fn(&str) -> Option<usize>) -> Channel {
        let tracks: Vec<Track> = t
            .tracks
            .iter()
            .filter_map(|tr| Some(Track { target: bind(&tr.name)?, ticks: tr.ticks.clone(), values: tr.values.clone(), cursor: 0 }))
            .collect();
        let length = face::length(tracks.iter().map(|t| &t.ticks[..]), t.ticks_per_frame);
        Channel { tracks, ticks_per_frame: t.ticks_per_frame as f32, length, clock: Clock::start(1.0, false, None) }
    }

    /// Sample every track at the clock's sampled time into `out[target]` (`empty` for a track without keys).
    pub(crate) fn apply(&mut self, out: &mut [f32], empty: f32) {
        let t = (F(self.clock.sampled) * self.ticks_per_frame).0;
        for tr in &mut self.tracks {
            out[tr.target] = if tr.ticks.is_empty() { empty } else { face::sample(&tr.ticks, &tr.values, t, &mut tr.cursor, false)[0] };
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Effect {
    pub skeleton: Skeleton,
    pub clip: Clip,
    model: Clock,
    morph: Channel,
    alpha: Channel,
    /// Per morph target of the model its weight.
    pub weights: Vec<f32>,
    /// Per material its alpha (1 = the MTL's 0x80): the `.MTA` value for driven materials, else the MTL's own.
    pub alphas: Vec<f32>,
    /// Started and not yet ended: drawn this frame.
    pub live: bool,
    /// The morph and alpha clocks wrap and the effect never ends (the landing marker's pulse).
    pub looping: bool,
    /// Past the end the clocks stay on the last frame and the effect stays live (the smash marker, drawn until the
    /// ball bounces).
    pub hold: bool,
}

impl Effect {
    pub fn new(mdl: &Model, ani: &Anim, mor: &Tracks, mta: &Tracks, materials: &[Material]) -> Effect {
        let skeleton = Skeleton { names: mdl.node_names.clone(), parent: mdl.node_parent.clone(), rest: mdl.node_local.clone() };
        let clip = Clip::new(&skeleton, ani);
        let morph = Channel::new(mor, |n| mdl.morph_names.iter().position(|m| m == n));
        let alpha = Channel::new(mta, |n| materials.iter().position(|m| m.name == n));
        Effect {
            skeleton,
            clip,
            model: Clock::start(1.0, false, None),
            morph,
            alpha,
            weights: vec![0.0; mdl.morph_names.len()],
            alphas: materials.iter().map(|m| m.color[3]).collect(),
            live: false,
            looping: false,
            hold: false,
        }
    }

    /// Start (or restart) the effect: its first frame, sampled at time 0.
    pub fn start(&mut self) {
        self.live = true;
        self.set_zero();
        self.tick();
    }

    fn set_zero(&mut self) {
        self.model = Clock::start(1.0, false, None);
        for c in [&mut self.morph.clock, &mut self.alpha.clock] {
            *c = Clock::start(1.0, self.looping, None);
        }
        self.morph.apply(&mut self.weights, 0.0);
        self.alpha.apply(&mut self.alphas, 1.0);
    }

    /// One frame: advance every channel; past the morph length the effect resets to 0 and ends (unless held).
    pub fn tick(&mut self) {
        if !self.live {
            return;
        }
        self.model.tick(self.clip.length);
        self.morph.clock.tick(self.morph.length);
        self.morph.apply(&mut self.weights, 0.0);
        self.alpha.clock.tick(self.alpha.length);
        self.alpha.apply(&mut self.alphas, 1.0);
        if !self.looping && !self.hold && self.morph.clock.done(self.morph.length) {
            self.live = false;
            self.set_zero();
        }
    }

    /// The model clock's sampled time and the morph/alpha clocks' (frames).
    pub fn times(&self) -> [f32; 3] {
        [self.model.sampled, self.morph.clock.sampled, self.alpha.clock.sampled]
    }

    /// Every node's local matrix this frame.
    pub fn locals(&self) -> Vec<M4> {
        self.clip.locals(&self.skeleton, self.model.sampled)
    }
}

/// One entry of the landing marker's predicted ball path (y up, as the game stores it): position, velocity and
/// bounces so far.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct PathEntry {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub bounces: i32,
}

/// The smash-point marker's search: for each human on the receiving team (up to two), the point where the
/// predicted ball, having risen to that player's smash top, comes back down below the middle of the smash window
/// moving away from the net at least 1.5 m from it. Searched a frame's worth of new path entries at a time; two
/// players' points are drawn at their midpoint.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SmashSearch {
    /// Per candidate: smash top, middle height, state (0 rising, 1 falling, 2 placed, −1 gave up).
    cands: Vec<(f32, f32, i32)>,
    pending: usize,
    next: usize,
    /// The points placed (x, z), in order.
    pub marks: Vec<[f32; 2]>,
}

impl SmashSearch {
    /// Candidates' smash `[top, middle]` heights.
    pub fn new(heights: &[[f32; 2]]) -> SmashSearch {
        SmashSearch { cands: heights.iter().map(|&[t, m]| (t, m, 0)).collect(), pending: heights.len(), next: 0, marks: vec![] }
    }

    /// Search `path` from where the last call stopped; true when this call placed the first point (the marker's
    /// animation starts).
    pub fn search(&mut self, path: &[PathEntry]) -> bool {
        let before = self.marks.len();
        let mut i = self.next;
        while self.pending > 0 && i < path.len() {
            let e = path[i];
            for c in &mut self.cands {
                match c.2 {
                    0 if c.0 <= e.pos[1] => c.2 = 1,
                    0 if e.bounces > 0 => {
                        c.2 = -1;
                        self.pending -= 1;
                    }
                    1 if e.vel[1] <= 0.0 && e.pos[1] < c.1 => {
                        let p = path[i - 1].pos;
                        let sign = |v: f32| if v < 0.0 { -1 } else { 1 };
                        if sign(e.vel[2]) == sign(p[2]) && p[2].abs() >= 1.5 {
                            self.marks.push([p[0], p[2]]);
                            c.2 = 2;
                        } else {
                            c.2 = -1;
                        }
                        self.pending -= 1;
                    }
                    _ => {}
                }
            }
            if self.pending == 0 {
                break;
            }
            i += 1;
        }
        self.next = i;
        before == 0 && !self.marks.is_empty()
    }

    /// Where the marker stands: the first point, or the midpoint of two.
    pub fn at(&self) -> Option<[f32; 2]> {
        match self.marks[..] {
            [a] => Some(a),
            [a, b, ..] => Some([0, 1].map(|k| ((F(a[k]) + b[k]) * 0.5).0)),
            [] => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lob from z = −10 toward +z: up at 0.1 m/frame, falling back under gravity.
    #[allow(clippy::float_arithmetic)] // test input built on the host FPU, not ported math
    fn lob(n: usize) -> Vec<PathEntry> {
        (0..n)
            .map(|i| {
                let t = i as f32;
                PathEntry { pos: [0.5, 1.0 + 0.1 * t - 0.001 * t * t, -10.0 + 0.2 * t], vel: [0.0, 0.1 - 0.002 * t, 0.2], bounces: 0 }
            })
            .collect()
    }

    #[test]
    fn smash_point_on_the_way_down() {
        let mut path = lob(120);
        let mut s = SmashSearch::new(&[[3.0, 2.5], [9.0, 2.0]]);
        // a frame's 15 entries: still rising
        assert!(!s.search(&path[..15]));
        assert!(s.at().is_none());
        assert!(s.search(&path));
        // first below 2.5 falling at t = 82; the point is the entry before (t = 81)
        assert_eq!(s.at(), Some([0.5, path[81].pos[2]]));
        // the second never reaches its top: it gives up at the bounce
        path.push(PathEntry { bounces: 1, ..path[119] });
        assert!(!s.search(&path));
        assert_eq!((s.marks.len(), s.pending), (1, 0));
    }

    #[test]
    fn no_point_coming_toward_the_net() {
        // the same lob mirrored to the −z side: moving −z it is placed, moving +z (toward the net) not
        let away: Vec<PathEntry> = lob(120).into_iter().map(|e| PathEntry { pos: [e.pos[0], e.pos[1], -e.pos[2]], vel: [0.0, e.vel[1], -0.2], ..e }).collect();
        let toward: Vec<PathEntry> = away.iter().map(|e| PathEntry { vel: [0.0, e.vel[1], 0.2], ..*e }).collect();
        let (mut s, mut t) = (SmashSearch::new(&[[3.0, 2.5]]), SmashSearch::new(&[[3.0, 2.5]]));
        assert!(s.search(&away));
        assert!(!t.search(&toward));
        assert_eq!(t.pending, 0);
    }
}

/// The racket-impact effect's scale for shot kind `kind` (0 top … 4 drop) from the ball's velocity as the shot
/// leaves: twice the speed for top and flat shots, 1 otherwise.
pub fn impact_scale(kind: i32, vel: [f32; 3]) -> f32 {
    if kind != 0 && kind != 2 {
        return 1.0;
    }
    let [x, y, z] = fs(vel);
    ((x * x + y * y + z * z).sqrt() * 2.0).0
}

/// The racket-impact effect's world matrix (rows x, y, z, translation): z along the ball's velocity `vel` (its w
/// scaled alike), x level (up × z), y = z × x, at the ball's position `pos`, fixed as the shot leaves.
pub fn impact_matrix(vel: [f32; 4], pos: [f32; 3]) -> [[f32; 4]; 4] {
    let len = |[x, y, z]: [F; 3]| 1.0 / (x * x).madd(y, y).madd(z, z).sqrt();
    let v = fs(vel);
    let q = len([v[0], v[1], v[2]]);
    let [x, y, z, w] = v.map(|c| c * q);
    let place = [pos[0], pos[1], pos[2], 1.0];
    if y.abs() >= 0.999999 {
        // ponytail: straight up/down never happens off a racket; the game turns a quarter turn about x
        let (s, neg) = if y < 0.0 { (1.0, -1.0) } else { (-1.0, 1.0) };
        return [[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, s, 0.0], [0.0, neg, 0.0, 0.0], place];
    }
    let a = [z * 1.0 - y * 0.0, x * 0.0 - z * 0.0, y * 0.0 - x * 1.0];
    let b = [y * a[2] - z * a[1], z * a[0] - x * a[2], x * a[1] - y * a[0]];
    let (qa, qb) = (len(a), len(b));
    [
        f32s([a[0] * qa, a[1] * qa, a[2] * qa, qa * 0.0]),
        f32s([b[0] * qb, b[1] * qb, b[2] * qb, qb * 0.0]),
        f32s([x, y, z, w]),
        place,
    ]
}

/// Hit sparks: up to [`SPARKS`] billboards thrown off the ball as a shot leaves, each from a pre-rolled
/// [`Roll`] (the game re-rolls a quarter of them when a burst has died out).
pub const SPARKS: usize = 25;
/// The frames left below which a spark fades out (alpha `life/FADE`).
pub const SPARK_FADE: i32 = 20;

/// One pre-rolled spark: a turn about the shot direction and three uniforms in [0, 1) (spread, speed, life).
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Roll {
    pub turn: M4,
    pub u: [f32; 3],
}

impl Roll {
    /// A turn by `angle` about z (rows x, y; the game's sincos keeps sin ≥ 0 for angle ≥ 0, so it turns by under
    /// half a circle) and the three uniforms.
    pub fn new(angle: f32, u: [f32; 3]) -> Roll {
        Roll { turn: crate::world::mat_mul(&crate::world::IDENTITY, &crate::shot::rot_z(angle)), u }
    }

    /// One roll from four uniform draws: the angle (a unit times 2π), then the uniforms.
    pub fn draw(rng: &mut crate::rng::Mt) -> Roll {
        let angle = (F(rng.unit()) * 6.283_185_5).0;
        Roll::new(angle, [rng.unit(), rng.unit(), rng.unit()])
    }

    /// Roll the table afresh from the sound generator: `all` (the reseed messages) redoes every roll, otherwise
    /// (a burst has died out) every fourth from a random one of the first four.
    pub fn reroll(rolls: &mut [Roll; SPARKS], all: bool, rng: &mut crate::rng::Mt) {
        let (start, step) = if all { (0, 1) } else { ((rng.next() >> 16 & 3) as usize, 4) };
        for r in rolls.iter_mut().skip(start).step_by(step) {
            *r = Roll::draw(rng);
        }
    }
}

#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct Spark {
    pub pos: [f32; 4],
    pub vel: [f32; 4],
    /// Half the billboard's side.
    pub size: f32,
    /// Frames left; 0 = gone.
    pub life: i32,
}

/// Per shot kind (0 top … 4 drop): whether the burst scales with the ball's speed, then for a fixed burst its
/// forward offset, spread, speed range, speed exponent, size range; both: life range.
struct Burst {
    by_speed: bool,
    offset: f32,
    spread: f32,
    speed: [f32; 2],
    exponent: f32,
    size: [f32; 2],
}

const BURSTS: [Burst; 5] = {
    const fn b(by_speed: bool, size_lo: f32) -> Burst {
        Burst { by_speed, offset: 2.0, spread: 3.5, speed: [if by_speed { 0.1 } else { 0.3 }, if by_speed { 0.5 } else { 0.3 }], exponent: 1.1, size: [size_lo, if by_speed { 0.3 } else { size_lo }] }
    }
    [b(true, 0.1), b(false, 0.5), b(true, 0.1), b(false, 0.5), b(false, 0.4)]
};
const LIFE: [i32; 2] = [10, 40];

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Sparks {
    pub sparks: [Spark; SPARKS],
    pub rolls: [Roll; SPARKS],
    /// A burst is playing (until its last spark dies).
    pub live: bool,
}

impl Sparks {
    pub fn new(rolls: [Roll; SPARKS]) -> Sparks {
        Sparks { sparks: [Spark::default(); SPARKS], rolls, live: false }
    }

    /// A reseed message (match start, change of ends, a new point after a point's end): every roll anew from `rng`
    /// and the burst stops.
    pub fn restart(&mut self, rng: &mut crate::rng::Mt) {
        Roll::reroll(&mut self.rolls, true, rng);
        self.live = false;
    }

    /// Throw a burst for shot kind `kind` (`smash`: the hitter's smash branch) from the ball at `pos` moving
    /// `vel`: sparks 0..n restart, any older ones past n play on.
    pub fn start(&mut self, kind: i32, smash: bool, pos: [f32; 4], vel: [f32; 4]) {
        self.live = true;
        let b = &BURSTS[kind.clamp(0, 4) as usize];
        let [x, y, z, _] = fs(vel);
        let speed = (y * y).madd(x, x).madd(z, z).sqrt();
        let (n, offset, spread, lo, hi, size_lo, size_hi, base) = if smash || b.by_speed {
            let n = (F::ZERO + 5.0).madd(40.0, speed);
            let n = if SPARKS as f32 <= n { SPARKS } else { n.0 as usize };
            let sp = F(f32::from_bits(0x3e23_d70a)) * speed;
            let lo = 0.1 + sp;
            (n, 0.5 + speed, 6.0 * speed, lo, 0.3 + sp, lo, (F::ZERO + 0.3).madd(0.5, speed), speed)
        } else {
            let [offset, spread, lo, hi, size_lo, size_hi] =
                fs([b.offset, b.spread, b.speed[0], b.speed[1], b.size[0], b.size[1]]);
            (SPARKS, offset, spread, lo, hi, size_lo, size_hi, F(f32::from_bits(0x3ea8_f5c3)))
        };
        let (range, size_range) = (hi - lo, size_hi - size_lo);
        let mut m = impact_matrix(vel, [pos[0], pos[1], pos[2]]);
        m[3] = std::array::from_fn(|k| (F::ZERO + pos[k]).madd(m[2][k], offset).0);
        let pace = if b.exponent > 0.0 { F(crate::libm::powf(base.0, b.exponent)) } else { base };
        for (s, r) in self.sparks[..n].iter_mut().zip(&self.rolls) {
            m = std::array::from_fn(|i| crate::vu0::transform(&m, r.turn[i]));
            let u = F(r.u[0]);
            let d: [F; 4] = std::array::from_fn(|k| (F::ZERO + m[3][k]).madd(F(m[0][k]) * u, spread) - pos[k]);
            let q = 1.0 / (d[1] * d[1]).madd(d[0], d[0]).madd(d[2], d[2]).sqrt();
            let v = (F::ZERO + lo).madd(r.u[1], range) * pace;
            *s = Spark {
                pos,
                vel: f32s(d.map(|c| c * q * v)),
                size: ((F::ZERO + size_lo).madd(1.0 - u, size_range) / 2.0).0,
                life: (F::ZERO + LIFE[0] as f32).madd(r.u[2], (LIFE[1] - LIFE[0]) as f32).0 as i32,
            };
        }
    }

    /// One frame: every spark ages, moves and slows; when none is left the burst ends and a quarter of the table
    /// is rolled afresh from `rng` (the sound generator).
    pub fn tick(&mut self, rng: &mut crate::rng::Mt) {
        if !self.live {
            return;
        }
        let mut any = false;
        for s in self.sparks.iter_mut().filter(|s| s.life > 0) {
            s.life -= 1;
            if s.life > 0 {
                any = true;
                s.pos = std::array::from_fn(|k| (F(s.pos[k]) + s.vel[k]).0);
                s.vel = s.vel.map(|c| (F(c) * 0.9).0);
                s.size = (F(s.size) * 0.95).0;
            }
        }
        if !any {
            self.live = false;
            Roll::reroll(&mut self.rolls, false, rng);
        }
    }
}

/// A swing's trail: two points along the racket (its y axis at 0.47 and 0.93 from the grip) sampled every frame of
/// the swing, drawn as a ribbon fading in and out, swept by a B-spline whose tail catches up with the racket.
pub const TRAIL_POINTS: usize = 120;

#[derive(Clone, Default)]
pub struct Trail {
    /// Inner and outer point per sample; the newest sits at `count - 1`.
    pub points: Vec<[[f32; 4]; 2]>,
    /// Samples kept (the last one is overwritten while the racket stays within 0.01 of it).
    stored: usize,
    pub live: bool,
    /// Frames left and the trail's length in frames.
    pub life: i32,
    pub length: i32,
    /// The swing's motion: the trail ends when another starts (bar the two follow-throughs).
    motion: i32,
    /// How far back the tail reaches (1 = the first sample), the width and the alpha at the racket (0–255 of 128).
    pub reach: f32,
    pub width: f32,
    pub alpha: f32,
    pub count: usize,
}

impl Trail {
    /// A swing of `motion` starts with `frames` of it left to play.
    pub fn start(&mut self, motion: i32, frames: i32) {
        *self = Trail { points: std::mem::take(&mut self.points), live: true, life: frames, length: frames, motion, ..Default::default() };
        self.points.clear();
    }

    /// One frame with the player playing `motion` and the racket at `racket` (rows; y and translation used).
    pub fn tick(&mut self, motion: i32, racket: &M4) {
        if !self.live {
            return;
        }
        self.life -= 1;
        // the game samples on after either end, but never draws it
        if self.life == 0 || (motion != self.motion && !matches!(motion, 0x1c | 0x1d)) {
            self.live = false;
            return;
        }
        self.motion = motion;
        if self.stored >= TRAIL_POINTS {
            return;
        }
        let at = |c: f32| std::array::from_fn(|k| (F::ZERO + racket[3][k]).madd(racket[1][k], c).0);
        let p = [at(0.47), at(0.93)];
        let dist = |a: [f32; 4], b: [f32; 4]| {
            let d: [F; 3] = std::array::from_fn(|k| F(a[k]) - b[k]);
            (d[1] * d[1]).madd(d[0], d[0]).madd(d[2], d[2]).sqrt()
        };
        let close = self.stored > 0 && dist(self.points[self.stored - 1][1], p[1]) <= 0.01;
        self.points.truncate(self.stored);
        self.points.push(p);
        if !close {
            self.stored += 1;
        }
        self.count = self.points.len();
        if self.count < 2 {
            return;
        }
        // fade in over the first 30% of the swing, out over the last 40%; a fast racket draws brighter
        let (life, length) = (self.life, self.length);
        // a trail started with no frames left (a rollback restoring a swing past its end) never ends at 0 and
        // divides by 0 here: the game's `div` doesn't trap, so it gets the EE's quotient (see `ee_div`)
        let (rise, fall) = ((F(length as f32) * 0.3).0 as i32, (F(length as f32) * 0.4).0 as i32);
        let a = if length - life < rise {
            ee_div((length - life) * 128, rise) as f32
        } else if life < fall {
            ee_div(life * 128, fall) as f32
        } else {
            128.0
        };
        let n = self.count;
        let a = (F::ZERO + a).madd(dist(self.points[n - 1][1], self.points[n - 2][1]), 64.0);
        self.alpha = if a <= 255.0 { a.0 } else { 255.0 };
        self.reach = (F(life as f32) / length as f32).0;
        let x = std::f32::consts::PI * F(crate::libm::powf((1.0 - F(self.reach)).0, 0.5));
        if (-F(std::f32::consts::PI)..=F(std::f32::consts::PI)).contains(&x) {
            self.width = (0.5 * F(table_sin(x.0))).0;
            self.reach = crate::libm::powf(self.reach, 0.5);
        }
    }

    /// The ribbon to draw, tail to racket: per row its inner and outer edge, texture v and alpha (0–255 of 128).
    pub fn ribbon(&self) -> Vec<([f32; 3], [f32; 3], f32, f32)> {
        let n = self.count;
        if !self.live || n < 2 {
            return vec![];
        }
        let rows = 2 * n - 1;
        (0..rows)
            .map(|i| {
                let t = F(i as f32) / (rows - 1) as f32;
                let back = 1.0 - t;
                let [a, b] = if i == rows - 1 {
                    self.points[n - 1]
                } else {
                    spline((back * self.reach * -F((n - 1) as f32) + (n - 1) as f32).0, &self.points)
                };
                let d: [F; 3] = std::array::from_fn(|k| F(b[k]) - a[k]);
                let s = back * self.width / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                (
                    std::array::from_fn(|k| (F(a[k]) - d[k] * s).0),
                    std::array::from_fn(|k| (F(b[k]) + d[k] * s).0),
                    (0.98 * t).0,
                    (self.alpha * t).0,
                )
            })
            .collect()
    }
}

/// The EE's `div` quotient: by 0 it is 1 for a negative dividend, −1 otherwise (no trap; PCSX2 alike).
pub fn ee_div(a: i32, b: i32) -> i32 {
    match b {
        0 => if a < 0 { 1 } else { -1 },
        _ => a.wrapping_div(b),
    }
}

/// The game's sample spline at `x` (in samples): a uniform cubic B-spline, its first two spans weighted to start on
/// the first sample, ends clamped.
fn spline(x: f32, p: &[[[f32; 4]; 2]]) -> [[f32; 4]; 2] {
    let i = x as usize;
    let u = F(x) - i as f32;
    let (u2, u3, v) = (u * u, u * u * u, 1.0 - u);
    let [w0, w1, w2, w3] = match i {
        0 => [v * v * v, 3.0 * u + (21.0 * u3 / 12.0 - 9.0 * u2 / 2.0), -11.0 * u3 / 12.0 + 3.0 * u2 / 2.0, u3 / 6.0],
        1 => [v * v * v * 0.25, u / 4.0 + (7.0 * u3 / 12.0 - 5.0 * u2 / 4.0) + 0.5833333, u / 2.0 - u3 / 2.0 + u2 / 2.0 + 0.1666666, u3 * 0.1666666],
        _ => [v * v * v * 0.1666666, u3 * 0.5 - u2 + 0.6666667, (u - u3 + u2 + 0.33333334) * 0.5, u3 * 0.1666666],
    };
    let last = p.len() - 1;
    // ponytail: a trail with no frames left (reach ±max) samples far outside; held to the samples (the game reads
    // past its buffer there, draw only)
    let i = i.min(last);
    let q = [p[i.saturating_sub(1)], p[i], p[(i + 1).min(last)], p[(i + 2).min(last)]];
    std::array::from_fn(|e| {
        std::array::from_fn(|k| (w0 * q[0][e][k] + w1 * q[1][e][k] + w2 * q[2][e][k] + w3 * q[3][e][k]).0)
    })
}

/// The ball's flight ribbon: the ball's positions since the last hit (a ring of 50; a sample in line with the two
/// before it, or within 0.01 of the last kept one, moves the last instead), drawn in the shot kind's colour.
pub const FLIGHT_POINTS: usize = 50;

/// The ribbon colour (RGBA, 128 = full) by shot kind (top, slice, flat, lob, drop), then the smash.
// ponytail: the game also has a second smash colour (128, 116, 34, 128) for a mode flag the port doesn't have
pub const FLIGHT_COLOURS: [u32; 6] = [0x3e0e7f46, 0x80300846, 0x33338046, 0x27802746, 0x800c8046, 0x757d4146];

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Flight {
    #[serde(with = "crate::big")]
    pub points: [[f32; 4]; FLIGHT_POINTS],
    /// Samples held, the oldest and the next slot to write.
    pub count: usize,
    pub tail: usize,
    pub head: usize,
    pub live: bool,
    /// `FLIGHT_COLOURS` entry.
    pub colour: u32,
    /// The ball's speed (per frame) at the last sample.
    pub speed: f32,
}

impl Default for Flight {
    fn default() -> Self {
        Flight { points: [[0.0; 4]; FLIGHT_POINTS], count: 0, tail: 0, head: 0, live: false, colour: 0, speed: 0.0 }
    }
}

impl Flight {
    /// A hit: the ribbon starts afresh in the shot's colour.
    pub fn start(&mut self, kind: i32, smash: bool) {
        *self = Flight { points: self.points, live: true, colour: FLIGHT_COLOURS[if smash { 5 } else { kind.clamp(0, 4) as usize }], ..Default::default() };
    }

    /// One frame of the ball at `pos` moving at `vel`.
    pub fn tick(&mut self, pos: [f32; 4], vel: [f32; 4]) {
        if !self.live {
            return;
        }
        let n = FLIGHT_POINTS;
        let back = |i: usize, by: usize| (i + n - by) % n;
        let mut keep = true;
        if self.count > 1 {
            let j = back(self.head, 2);
            if self.count > 2 {
                let (a, b) = (self.points[j], self.points[back(j, 1)]);
                let d: [F; 3] = std::array::from_fn(|k| F(pos[k]) - a[k]);
                let e: [F; 3] = std::array::from_fn(|k| F(a[k]) - b[k]);
                let r = 1.0 / (e[2] * e[2] + e[0] * e[0] + e[1] * e[1]).sqrt();
                let e = e.map(|v| v * r);
                let c = [d[0] * e[1] - d[1] * e[0], d[2] * e[0] - d[0] * e[2], d[1] * e[2] - d[2] * e[1]];
                if (c[0] * c[0] + c[2] * c[2] + c[1] * c[1]).sqrt() < 0.05 {
                    keep = false;
                }
            }
            let a = self.points[j];
            let d: [F; 3] = std::array::from_fn(|k| F(pos[k]) - a[k]);
            if keep && (d[2] * d[2] + d[0] * d[0] + d[1] * d[1]).sqrt() < 0.01 {
                keep = false;
            }
        }
        if keep {
            self.points[self.head] = pos;
            self.head = (self.head + 1) % n;
            if self.count < n {
                self.count += 1;
            } else {
                self.tail = (self.tail + 1) % n;
            }
        } else {
            self.points[back(self.head, 1)] = pos;
        }
        let v = fs(vel);
        self.speed = (v[0] * v[0]).madd(v[1], v[1]).madd(v[2], v[2]).sqrt().0;
    }
}

/// The glow at the ball after a stroke (not a smash): `impactef_*` spinning about the flight, 30 frames, fading
/// out over the last 15.
pub const GLOW_FRAMES: i32 = 30;

#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct Glow {
    pub life: i32,
    /// `impactef_*` by shot kind: flat shows top's and drop shows slice's.
    pub texture: usize,
}

impl Glow {
    pub fn start(&mut self, kind: i32) {
        *self = Glow { life: GLOW_FRAMES, texture: match kind { 2 => 0, 4 => 1, k => k.clamp(0, 4) as usize } };
    }

    pub fn tick(&mut self) {
        self.life = (self.life - 1).max(0);
    }
}

/// What a bounce effect needs of one of the ball's contacts this frame (its contact record).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Contact {
    pub point: [f32; 4],
    pub vel: [f32; 4],
    pub normal: [f32; 4],
    /// The material is the playing surface.
    pub court: bool,
}

/// Ball marks kept at once (the oldest goes) and the frames one stays.
pub const MARKS: usize = 20;
pub const MARK_LIFE: i32 = 1800;
const PUFFS: usize = 2;
const PUFF_LIFE: i32 = 20;

/// A ball mark on the court: a ground quad stretched along the bounce by the ball's level speed.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Mark {
    /// Rows side, up, along the bounce, centre (0.01 below the court).
    pub m: [[f32; 4]; 4],
    pub life: i32,
    /// The ball's level speed into the bounce.
    pub len: f32,
    /// 128 fading to 0 over its life.
    pub alpha: f32,
}

/// A dust puff (court colour): a camera-facing quad growing and fading out over 20 frames.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Puff {
    pub pos: [f32; 4],
    pub life: i32,
    pub size: f32,
    grow: f32,
    pub alpha: f32,
    fade: f32,
    /// Born this frame: left as is.
    fresh: bool,
}

impl Puff {
    fn new(pos: [f32; 4], size: f32, grow: f32) -> Puff {
        let life = PUFF_LIFE as f32;
        Puff {
            pos,
            life: PUFF_LIFE,
            size,
            grow: (F(grow) / life).0,
            alpha: 128.0,
            fade: (-(F(128.0) / life)).0,
            fresh: true,
        }
    }

    /// One frame; false once it has died out.
    fn tick(&mut self, mut more: impl FnMut(&mut Puff)) -> bool {
        if std::mem::take(&mut self.fresh) {
            return true;
        }
        self.size = (F(self.size) + self.grow).0;
        self.alpha = (F(self.alpha) + self.fade).0;
        more(self);
        self.life -= 1;
        self.life >= 1
    }
}

/// The ball's bounce effects: on the first bounce off the court a mark and a puff of dust, and the `ballbound` ring
/// model lying in the bounce; a second bounce puffs again. A smash landing (more than one player, 85 km/h or more)
/// plays the court's `chakudan` crater model instead, with a drifting dust cloud and no mark.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Bounce {
    pub marks: Vec<Mark>,
    pub puffs: Vec<Puff>,
    /// The smash dust cloud, its level drift and speed.
    pub dust: Option<(Puff, [f32; 4], f32)>,
    pub ring: Effect,
    /// The ring's world matrix: turned to lie in the bounce surface, at the bounce.
    pub ring_at: [[f32; 4]; 4],
    pub crater: Effect,
    /// The crater's world matrix: rows side, up, along the bounce, at the bounce.
    pub crater_at: [[f32; 4]; 4],
    /// Marks age (in play); held marks stay.
    pub fading: bool,
    /// The court raises dust (`BounceLook::puffs`): puffs only animate on such courts.
    pub puffing: bool,
    /// Bounces seen so far this flight.
    seen: i32,
}

impl Bounce {
    pub fn new(ring: Effect, crater: Effect, puffing: bool) -> Bounce {
        let z = [[0.0; 4]; 4];
        Bounce { marks: vec![], puffs: vec![], dust: None, ring, ring_at: z, crater, crater_at: z, fading: false, puffing, seen: 0 }
    }

    fn puff(&mut self, p: Puff) {
        if self.puffs.len() == PUFFS {
            self.puffs.remove(0);
        }
        self.puffs.push(p);
    }

    /// One frame of the ball with `bounces` so far and its contact records (`contacts` empty when it has none);
    /// `smash` when a first bounce now would be a smash landing.
    // ponytail: wet courts (spray model, rising puffs) and the replay's larger crater are left out
    pub fn tick(&mut self, bounces: i32, contacts: &[Contact], smash: bool) {
        let (mut ring, mut crater) = (false, false);
        if let Some(c) = contacts.first().filter(|_| bounces == 1 && self.seen != 1) {
            let mut p = c.point;
            let v: [F; 4] = std::array::from_fn(|k| F(p[k]) + c.vel[k] - p[k]);
            let q = 1.0 / (v[0] * v[0] + v[2] * v[2]).sqrt();
            let fwd = [(v[0] * q).0, 0.0, (v[2] * q).0, 0.0];
            let up = [0.0, 1.0, 0.0, 0.0];
            let (u, w) = (fs(up), fs(fwd));
            let side = [(u[1] * w[2] - u[2] * 0.0).0, (u[2] * w[0] - u[0] * w[2]).0, (u[0] * 0.0 - u[1] * w[0]).0, 0.0];
            if c.court {
                p[1] = 0.0; // effects sit on the court plane
            }
            if c.court && smash {
                crater = true;
                self.crater.start();
                self.crater_at = [side, up, fwd, p];
                let d = Puff::new([p[0], 0.1, p[2], p[3]], (F(0.15) * 2.0).0, (F(0.3) * 2.0).0);
                self.dust = Some((d, [fwd[0], 0.0, fwd[2], 0.0], 0.05));
            } else if c.court {
                if self.marks.len() == MARKS {
                    self.marks.remove(0);
                }
                let len = (F(c.vel[0]) * c.vel[0] + F(c.vel[2]) * c.vel[2]).sqrt().0;
                self.marks.push(Mark {
                    m: [side, up, fwd, [p[0], -0.01, p[2], p[3]]],
                    life: MARK_LIFE,
                    len,
                    alpha: 0.0,
                });
                self.puff(Puff::new(p, 0.15, 0.3));
            }
            if !self.crater.live {
                ring = true;
                let n = c.normal;
                let pitch = crate::libm::atan2f((-F(n[1])).0, (F(n[2]) * n[2]).madd(n[0], n[0]).sqrt().0);
                let m = world::mat_mul(&world::rot_x(pitch), &world::rot_y(crate::libm::atan2f(n[0], n[2])));
                self.ring_at = [m[0], m[1], m[2], [p[0], p[1], p[2], 1.0]];
                self.ring.start();
            }
        }
        if let Some(c) = contacts.get(1).filter(|c| bounces == 2 && self.seen != 2 && c.court) {
            self.puff(Puff::new(c.point, 0.1, 0.2));
        }

        let fading = self.fading;
        self.marks.retain_mut(|m| {
            let before = m.life;
            m.life -= fading as i32;
            m.alpha = (F((before << 7) as f32) / MARK_LIFE as f32).0;
            m.life >= 0
        });
        if self.puffing {
            self.puffs.retain_mut(|p| p.tick(|_| {}));
        }
        if let Some((d, dir, speed)) = &mut self.dust {
            let alive = d.tick(|d| {
                d.pos = std::array::from_fn(|k| (F(dir[k]) * *speed + d.pos[k] + 0.0).0);
                *speed = (F(*speed) * 0.95).0;
            });
            if !alive {
                self.dust = None;
            }
        }
        if !ring {
            self.ring.tick();
        }
        if !crater {
            self.crater.tick();
        }
        self.seen = if contacts.is_empty() { 0 } else { bounces };
    }
}

/// The game's table sine: a quarter turn in 256 steps (sin rounded to six decimals; nine steps a millionth off),
/// read linearly between steps.
pub fn table_sin(x: f32) -> f32 {
    static TABLE: std::sync::OnceLock<[f32; 258]> = std::sync::OnceLock::new();
    let t = TABLE.get_or_init(sin_table);
    let (sign, mut x) = if x < 0.0 { (-1.0, -F(x)) } else { (1.0, F(x)) };
    if x > std::f32::consts::FRAC_PI_2 {
        x = std::f32::consts::PI - x;
    }
    let at = f32::from_bits(0x4322_f983) * x;
    let i = at.0 as i32 as usize;
    (sign * (F::ZERO + t[i]).madd(at - i as f32, F(t[i + 1]) - t[i])).0
}

/// `table_sin`'s table, rebuilt in host f64: the game ships it as data.
#[allow(clippy::float_arithmetic)]
fn sin_table() -> [f32; 258] {
    std::array::from_fn(|i| {
        if i > 256 {
            return 0.0;
        }
        let off = match i {
            59 => -1.0,
            79 | 93 | 110 | 145 | 209 | 214 | 226 | 248 => 1.0,
            _ => 0.0,
        };
        (((musl::sin(i as f64 * std::f64::consts::PI / 512.0) * 1e6).round() + off) / 1e6) as f32
    })
}
