//! Effect models (`AZUMA/C_EFF/EFFCT.XB0`): a model played by key channels, each on its own clock — node
//! animation (`.ANI`, a hierarchy under a scene root), morph-target weights (`.MOR`) and material alpha (`.MTA`,
//! tracks named after the materials they drive). Starting an effect sets every clock to 0 and plays its first frame; each
//! frame advances them as a motion clock ([`Clock`], speed 1, clamped), the model's nodes first, then the
//! morphs, then the alphas. The effect ends the frame the morph clock's sampled time reaches the morph length (the
//! last key of its bound tracks, in frames): the frames before that are drawn.
//!
//! `.UVA` files sit beside some effects but never play: the game loads them into the alpha channel as a second
//! clip it never selects.

use hst_data::{ani::Anim, mdl::Model, mor::Tracks, mtl::Material};

use crate::{
    face,
    motion::Clock,
    pose::{Clip, M4, Skeleton},
    ps2, world,
};

pub(crate) struct Track {
    /// Morph target or material index.
    target: usize,
    ticks: Vec<i32>,
    values: Vec<[f32; 4]>,
    cursor: usize,
}

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
        let t = ps2::mul(self.clock.sampled, self.ticks_per_frame);
        for tr in &mut self.tracks {
            out[tr.target] = if tr.ticks.is_empty() { empty } else { face::sample(&tr.ticks, &tr.values, t, &mut tr.cursor, false)[0] };
        }
    }
}

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
#[derive(Clone, Copy)]
pub struct PathEntry {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub bounces: i32,
}

/// The smash-point marker's search: for each human on the receiving team (up to two), the point where the
/// predicted ball, having risen to that player's smash top, comes back down below the middle of the smash window
/// moving away from the net at least 1.5 m from it. Searched a frame's worth of new path entries at a time; two
/// players' points are drawn at their midpoint.
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
            [a, b, ..] => Some([0, 1].map(|k| ps2::mul(ps2::add(a[k], b[k]), 0.5))),
            [] => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lob from z = −10 toward +z: up at 0.1 m/frame, falling back under gravity.
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
    use ps2::{add, mul};
    if kind != 0 && kind != 2 {
        return 1.0;
    }
    let [x, y, z] = vel;
    mul(ps2::sqrt(add(add(mul(x, x), mul(y, y)), mul(z, z))), 2.0)
}

/// The racket-impact effect's world matrix (rows x, y, z, translation): z along the ball's velocity `vel` (its w
/// scaled alike), x level (up × z), y = z × x, at the ball's position `pos`, fixed as the shot leaves.
pub fn impact_matrix(vel: [f32; 4], pos: [f32; 3]) -> [[f32; 4]; 4] {
    use ps2::{div, mul, sub};
    let len = |[x, y, z]: [f32; 3]| div(1.0, ps2::sqrt(ps2::madd(ps2::madd(mul(x, x), y, y), z, z)));
    let q = len([vel[0], vel[1], vel[2]]);
    let [x, y, z, w] = vel.map(|c| mul(c, q));
    let place = [pos[0], pos[1], pos[2], 1.0];
    if y.abs() >= 0.999999 {
        // ponytail: straight up/down never happens off a racket; the game turns a quarter turn about x
        let s = if y < 0.0 { 1.0 } else { -1.0 };
        return [[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, s, 0.0], [0.0, -s, 0.0, 0.0], place];
    }
    let a = [sub(mul(z, 1.0), mul(y, 0.0)), sub(mul(x, 0.0), mul(z, 0.0)), sub(mul(y, 0.0), mul(x, 1.0))];
    let b = [sub(mul(y, a[2]), mul(z, a[1])), sub(mul(z, a[0]), mul(x, a[2])), sub(mul(x, a[1]), mul(y, a[0]))];
    let (qa, qb) = (len(a), len(b));
    [[mul(a[0], qa), mul(a[1], qa), mul(a[2], qa), mul(qa, 0.0)], [mul(b[0], qb), mul(b[1], qb), mul(b[2], qb), mul(qb, 0.0)], [x, y, z, w], place]
}

/// Hit sparks: up to [`SPARKS`] billboards thrown off the ball as a shot leaves, each from a pre-rolled
/// [`Roll`] (the game re-rolls a quarter of them when a burst has died out).
pub const SPARKS: usize = 25;
/// The frames left below which a spark fades out (alpha `life/FADE`).
pub const SPARK_FADE: i32 = 20;

/// One pre-rolled spark: a turn about the shot direction and three uniforms in [0, 1) (spread, speed, life).
#[derive(Clone, Copy)]
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
        let angle = ps2::mul(rng.unit(), 6.283_185_5);
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

#[derive(Clone, Copy, Default)]
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
        use ps2::{add, div, madd, mul, sub};
        self.live = true;
        let b = &BURSTS[kind.clamp(0, 4) as usize];
        let [x, y, z, _] = vel;
        let speed = ps2::sqrt(madd(madd(mul(y, y), x, x), z, z));
        let (n, offset, spread, lo, hi, size_lo, size_hi, base) = if smash || b.by_speed {
            let n = madd(add(0.0, 5.0), 40.0, speed);
            let n = if SPARKS as f32 <= n { SPARKS } else { n as usize };
            let sp = mul(f32::from_bits(0x3e23_d70a), speed);
            let lo = add(0.1, sp);
            (n, add(0.5, speed), mul(6.0, speed), lo, add(0.3, sp), lo, madd(add(0.0, 0.3), 0.5, speed), speed)
        } else {
            (SPARKS, b.offset, b.spread, b.speed[0], b.speed[1], b.size[0], b.size[1], f32::from_bits(0x3ea8_f5c3))
        };
        let (range, size_range) = (sub(hi, lo), sub(size_hi, size_lo));
        let mut m = impact_matrix(vel, [pos[0], pos[1], pos[2]]);
        m[3] = std::array::from_fn(|k| madd(add(0.0, pos[k]), m[2][k], offset));
        let pace = if b.exponent > 0.0 { crate::libm::powf(base, b.exponent) } else { base };
        for (s, r) in self.sparks[..n].iter_mut().zip(&self.rolls) {
            m = std::array::from_fn(|i| crate::vu0::transform(&m, r.turn[i]));
            let u = r.u[0];
            let d: [f32; 4] = std::array::from_fn(|k| sub(madd(add(0.0, m[3][k]), mul(m[0][k], u), spread), pos[k]));
            let q = div(1.0, ps2::sqrt(madd(madd(mul(d[1], d[1]), d[0], d[0]), d[2], d[2])));
            let v = mul(madd(add(0.0, lo), r.u[1], range), pace);
            *s = Spark {
                pos,
                vel: d.map(|c| mul(mul(c, q), v)),
                size: div(madd(add(0.0, size_lo), sub(1.0, u), size_range), 2.0),
                life: madd(add(0.0, LIFE[0] as f32), r.u[2], (LIFE[1] - LIFE[0]) as f32) as i32,
            };
        }
    }

    /// One frame: every spark ages, moves and slows; when none is left the burst ends and a quarter of the table
    /// is rolled afresh from `rng` (the sound generator).
    pub fn tick(&mut self, rng: &mut crate::rng::Mt) {
        use ps2::{add, mul};
        if !self.live {
            return;
        }
        let mut any = false;
        for s in self.sparks.iter_mut().filter(|s| s.life > 0) {
            s.life -= 1;
            if s.life > 0 {
                any = true;
                s.pos = std::array::from_fn(|k| add(s.pos[k], s.vel[k]));
                s.vel = s.vel.map(|c| mul(c, 0.9));
                s.size = mul(s.size, 0.95);
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
        use ps2::{add, div, madd, mul, sub};
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
        let at = |c: f32| std::array::from_fn(|k| madd(add(0.0, racket[3][k]), racket[1][k], c));
        let p = [at(0.47), at(0.93)];
        let dist = |a: [f32; 4], b: [f32; 4]| {
            let d: [f32; 3] = std::array::from_fn(|k| sub(a[k], b[k]));
            ps2::sqrt(madd(madd(mul(d[1], d[1]), d[0], d[0]), d[2], d[2]))
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
        let (rise, fall) = (mul(length as f32, 0.3) as i32, mul(length as f32, 0.4) as i32);
        let a = if length - life < rise {
            ((length - life) * 128 / rise) as f32
        } else if life < fall {
            (life * 128 / fall) as f32
        } else {
            128.0
        };
        let n = self.count;
        let a = madd(add(0.0, a), dist(self.points[n - 1][1], self.points[n - 2][1]), 64.0);
        self.alpha = if a <= 255.0 { a } else { 255.0 };
        self.reach = div(life as f32, length as f32);
        let x = mul(std::f32::consts::PI, crate::libm::powf(sub(1.0, self.reach), 0.5));
        if (-std::f32::consts::PI..=std::f32::consts::PI).contains(&x) {
            self.width = mul(0.5, crate::libm::table_sin(x));
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
                let t = i as f32 / (rows - 1) as f32;
                let back = 1.0 - t;
                let [a, b] = if i == rows - 1 { self.points[n - 1] } else { spline(back * self.reach * -((n - 1) as f32) + (n - 1) as f32, &self.points) };
                let d: [f32; 3] = std::array::from_fn(|k| b[k] - a[k]);
                let s = back * self.width / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                (std::array::from_fn(|k| a[k] - d[k] * s), std::array::from_fn(|k| b[k] + d[k] * s), 0.98 * t, self.alpha * t)
            })
            .collect()
    }
}

/// The game's sample spline at `x` (in samples): a uniform cubic B-spline, its first two spans weighted to start on
/// the first sample, ends clamped.
fn spline(x: f32, p: &[[[f32; 4]; 2]]) -> [[f32; 4]; 2] {
    let i = x as usize;
    let u = x - i as f32;
    let (u2, u3, v) = (u * u, u * u * u, 1.0 - u);
    let [w0, w1, w2, w3] = match i {
        0 => [v * v * v, 3.0 * u + (21.0 * u3 / 12.0 - 9.0 * u2 / 2.0), -11.0 * u3 / 12.0 + 3.0 * u2 / 2.0, u3 / 6.0],
        1 => [v * v * v * 0.25, u / 4.0 + (7.0 * u3 / 12.0 - 5.0 * u2 / 4.0) + 0.5833333, u / 2.0 - u3 / 2.0 + u2 / 2.0 + 0.1666666, u3 * 0.1666666],
        _ => [v * v * v * 0.1666666, u3 * 0.5 - u2 + 0.6666667, (u - u3 + u2 + 0.33333334) * 0.5, u3 * 0.1666666],
    };
    let last = p.len() - 1;
    let q = [p[i.saturating_sub(1)], p[i], p[(i + 1).min(last)], p[(i + 2).min(last)]];
    std::array::from_fn(|e| std::array::from_fn(|k| w0 * q[0][e][k] + w1 * q[1][e][k] + w2 * q[2][e][k] + w3 * q[3][e][k]))
}

/// The ball's flight ribbon: the ball's positions since the last hit (a ring of 50; a sample in line with the two
/// before it, or within 0.01 of the last kept one, moves the last instead), drawn in the shot kind's colour.
pub const FLIGHT_POINTS: usize = 50;

/// The ribbon colour (RGBA, 128 = full) by shot kind (top, slice, flat, lob, drop), then the smash.
// ponytail: the game also has a second smash colour (128, 116, 34, 128) for a mode flag the port doesn't have
pub const FLIGHT_COLOURS: [u32; 6] = [0x3e0e7f46, 0x80300846, 0x33338046, 0x27802746, 0x800c8046, 0x757d4146];

#[derive(Clone)]
pub struct Flight {
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
        use ps2::{add, div, madd, mul, sub};
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
                let d: [f32; 3] = std::array::from_fn(|k| sub(pos[k], a[k]));
                let e: [f32; 3] = std::array::from_fn(|k| sub(a[k], b[k]));
                let r = div(1.0, ps2::sqrt(add(add(mul(e[2], e[2]), mul(e[0], e[0])), mul(e[1], e[1]))));
                let e = e.map(|v| mul(v, r));
                let c = [sub(mul(d[0], e[1]), mul(d[1], e[0])), sub(mul(d[2], e[0]), mul(d[0], e[2])), sub(mul(d[1], e[2]), mul(d[2], e[1]))];
                if ps2::sqrt(add(add(mul(c[0], c[0]), mul(c[2], c[2])), mul(c[1], c[1]))) < 0.05 {
                    keep = false;
                }
            }
            let a = self.points[j];
            let d: [f32; 3] = std::array::from_fn(|k| sub(pos[k], a[k]));
            if keep && ps2::sqrt(add(add(mul(d[2], d[2]), mul(d[0], d[0])), mul(d[1], d[1]))) < 0.01 {
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
        self.speed = ps2::sqrt(madd(madd(mul(vel[0], vel[0]), vel[1], vel[1]), vel[2], vel[2]));
    }
}

/// The glow at the ball after a stroke (not a smash): `impactef_*` spinning about the flight, 30 frames, fading
/// out over the last 15.
pub const GLOW_FRAMES: i32 = 30;

#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Debug, Default)]
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
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug)]
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
        Puff { pos, life: PUFF_LIFE, size, grow: ps2::div(grow, life), alpha: 128.0, fade: -ps2::div(128.0, life), fresh: true }
    }

    /// One frame; false once it has died out.
    fn tick(&mut self, mut more: impl FnMut(&mut Puff)) -> bool {
        if std::mem::take(&mut self.fresh) {
            return true;
        }
        self.size = ps2::add(self.size, self.grow);
        self.alpha = ps2::add(self.alpha, self.fade);
        more(self);
        self.life -= 1;
        self.life >= 1
    }
}

/// The ball's bounce effects: on the first bounce off the court a mark and a puff of dust, and the `ballbound` ring
/// model lying in the bounce; a second bounce puffs again. A smash landing (more than one player, 85 km/h or more)
/// plays the court's `chakudan` crater model instead, with a drifting dust cloud and no mark.
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
        use ps2::{add, div, mul, sub};
        let (mut ring, mut crater) = (false, false);
        if let Some(c) = contacts.first().filter(|_| bounces == 1 && self.seen != 1) {
            let mut p = c.point;
            let v: [f32; 4] = std::array::from_fn(|k| sub(add(p[k], c.vel[k]), p[k]));
            let q = div(1.0, ps2::sqrt(add(mul(v[0], v[0]), mul(v[2], v[2]))));
            let fwd = [mul(v[0], q), 0.0, mul(v[2], q), 0.0];
            let up = [0.0, 1.0, 0.0, 0.0];
            let side = [
                sub(mul(up[1], fwd[2]), mul(up[2], 0.0)),
                sub(mul(up[2], fwd[0]), mul(up[0], fwd[2])),
                sub(mul(up[0], 0.0), mul(up[1], fwd[0])),
                0.0,
            ];
            if c.court {
                p[1] = 0.0; // effects sit on the court plane
            }
            if c.court && smash {
                crater = true;
                self.crater.start();
                self.crater_at = [side, up, fwd, p];
                let d = Puff::new([p[0], 0.1, p[2], p[3]], mul(0.15, 2.0), mul(0.3, 2.0));
                self.dust = Some((d, [fwd[0], 0.0, fwd[2], 0.0], 0.05));
            } else if c.court {
                if self.marks.len() == MARKS {
                    self.marks.remove(0);
                }
                let len = ps2::sqrt(add(mul(c.vel[0], c.vel[0]), mul(c.vel[2], c.vel[2])));
                self.marks.push(Mark { m: [side, up, fwd, [p[0], -0.01, p[2], p[3]]], life: MARK_LIFE, len, alpha: 0.0 });
                self.puff(Puff::new(p, 0.15, 0.3));
            }
            if !self.crater.live {
                ring = true;
                let n = c.normal;
                let pitch = crate::libm::atan2f(-n[1], ps2::sqrt(ps2::madd(mul(n[2], n[2]), n[0], n[0])));
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
            m.alpha = div((before << 7) as f32, MARK_LIFE as f32);
            m.life >= 0
        });
        if self.puffing {
            self.puffs.retain_mut(|p| p.tick(|_| {}));
        }
        if let Some((d, dir, speed)) = &mut self.dust {
            let alive = d.tick(|d| {
                d.pos = std::array::from_fn(|k| add(add(mul(dir[k], *speed), d.pos[k]), 0.0));
                *speed = mul(*speed, 0.95);
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
