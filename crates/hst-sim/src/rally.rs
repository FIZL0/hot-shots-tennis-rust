//! The doubles AI's rally routines: the receive (waiting for a ball coming its way, running to the contact spot,
//! timing the swing), with the path copy, the landing and bounce picks, the dive check, the smash search, the
//! ready check and the stroke-frame pickers it calls; and the NET and BASE rally routines (waiting in the formation
//! between shots, the shot records the partners leave each other, the volley search).

use crate::ai::{AiParams, PathBall, Runner, Searcher};
use crate::aim::Pair;
use crate::position::{Cue, Formation, Return, Team};
use crate::player::{self, ReachStats, Stats};
use crate::ps2::{add, div, madd, msub, mul, sqrt, sub};

/// The most path entries the AI copies.
pub const PATH_MAX: usize = 180;
/// The seen flags the tiered search marks (one per path entry).
pub const SEEN: usize = 0xb4;
/// How many ball-lost frames the player logs.
pub const LOST_LOG: usize = 50;

/// One entry of the predicted ball path as the AI copies it: position, velocity (4 words each) and bounces.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ball {
    pub pos: [f32; 4],
    pub vel: [f32; 4],
    pub bounces: i32,
}

/// The AI's copy of the predicted path, shared by both AIs: the frame it was taken and its entries.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathCopy {
    pub stamp: i32,
    pub balls: Vec<Ball>,
}

/// The match as the rally routines read it.
#[derive(Clone, Debug, Default)]
pub struct World {
    pub frame: i32,
    /// Match phase (3 rally, 4 point over).
    pub phase: u8,
    pub players: i32,
    /// Court side: 0 deuce, 1 ad.
    pub court: i32,
    pub receiver: i32,
    /// The last hitter's slot (−1 none) and the shot count.
    pub hitter: i32,
    pub shots: i32,
    /// The ball's position and its two hit counters (equal once the hit is settled).
    pub ball: [f32; 4],
    pub hits: [i32; 2],
    pub gravity: f32,
    pub drag: f32,
    pub floor: i32,
    /// The predicted path (its live window) the AI copies from.
    pub path: Vec<Ball>,
    /// The players' shot records (by slot), the path object's mode byte and its line gap.
    pub records: [Shot; 4],
    pub path_mode: u8,
    pub path_gap: f32,
}

/// A player's shot record: the frame it was written, the frames to its contact (relative to `stamp`; −2 none,
/// −3 hitting), the contact kind, where the player stood and where it runs to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shot {
    pub stamp: i32,
    pub n: i32,
    pub kind: i32,
    pub pos: [f32; 4],
    pub vec: [f32; 4],
}

impl World {
    /// The record of `slot`, its frames made absolute.
    fn shot(&self, slot: i32) -> Shot {
        let mut s = self.records[slot as usize];
        if s.n >= 0 {
            s.n += s.stamp;
        }
        s
    }

    fn put(&mut self, slot: i32, n: i32, kind: i32, pos: [f32; 4], vec: [f32; 4]) {
        self.records[slot as usize] = Shot { stamp: self.frame, n, kind, pos, vec };
    }
}

/// The AI's player as the rally routines read it.
#[derive(Clone, Debug, Default)]
pub struct Body {
    pub pos: [f32; 4],
    pub facing: [f32; 4],
    /// Run velocity x, z.
    pub vel: [f32; 2],
    pub side: f32,
    /// Hand (±1).
    pub hand: f32,
    pub team: i32,
    pub size: i32,
    pub stats: Stats,
    pub stamina: i32,
    pub tick: i32,
    pub run: i32,
    /// Move state (see `Runner::moving`).
    pub moving: u8,
    /// Depth stat; the smash stand offset (x, z).
    pub depth: f32,
    pub smash_off: [f32; 2],
    /// Controller (< 0x20 human), doubles formation.
    pub control: i32,
    pub formation: u8,
    /// Swing countdown (−1 idle); stroke busy (0 once the stroke is over).
    pub swing: i32,
    pub stroke: u8,
    /// Ball-lost state and the frames it was lost on.
    pub lost: i32,
    pub lost_log: Vec<i32>,
    pub reach: ReachStats,
    /// TParam's hand (see `Searcher::strong`).
    pub strong: u8,
    /// The partner's and the opponents' positions; the partner's voice state; the hit mark.
    pub mate: [f32; 4],
    pub opp: [[f32; 4]; 2],
    pub mate_voice: u8,
    pub mark: u8,
}

/// The doubles AI's rally state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rally {
    pub level: u8,
    /// Beside a human partner (> 0).
    pub mate: i32,
    /// The path window: first entry, entries wanted, singles, entries copied.
    pub first: i32,
    pub window: i32,
    pub singles: bool,
    pub len: i32,
    pub state: u8,
    /// The contact found: stand spot, ball, run frames, index (from `first`).
    pub stand: [f32; 4],
    pub ball: [f32; 4],
    pub frames: i32,
    pub index: i32,
    /// A new ball to search for; the contact kind (0 tiered, 1 reach, 2 volley, 3 smash, 4 dive).
    pub fresh: bool,
    pub kind: u8,
    pub stick: [f32; 4],
    pub hold: bool,
    pub plan: u8,
    /// Frames to the swing, and the lead it presses the button by (per kind: stroke, volley, smash).
    pub swing: i32,
    pub lead: i32,
    pub leads: [i32; 3],
    pub dive: bool,
    /// The guess (1 or 2) and its walk's frames left.
    pub guess: u8,
    pub wait: i32,
    pub body: bool,
    pub low: bool,
    pub push: bool,
    /// The spot the guess walk started from (x, z).
    pub from: [f32; 2],
    /// NET/BASE: the substate, the volley level pick, the centre rate and radius.
    pub sub: u8,
    pub volley_level: u8,
    pub rate: i32,
    pub radius: f32,
    /// Chase the bounce while nothing's found; held back (a line call coming); the formation spot (x, z of 4).
    pub chase: bool,
    pub held: bool,
    pub spot: [f32; 4],
    /// The path entry the next search starts from (−1 the window's start).
    pub next: i32,
    /// Following the partner's shot; holding the middle; its team hit (with the frames since).
    pub follow: bool,
    pub middle: bool,
    pub ours: bool,
    pub tick: i32,
    pub voice: bool,
    pub lean: i32,
    pub back: Return,
    pub lane: u8,
    pub front: bool,
    pub forward: bool,
    /// Giving way to the partner: the rounds before the roll is retaken, the rate, the roll, the flag.
    pub rounds: i32,
    pub defer_rate: i32,
    pub defer_roll: bool,
    pub defer: bool,
    /// The caller's stack quad the bounce picks start from (the game leaves it uninitialised).
    pub stash: [f32; 4],
}

/// What a call leaves for the player: the run target or stick, and the button.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Out {
    pub stick: [f32; 4],
    pub button: Option<u32>,
}

fn abs(v: f32) -> f32 {
    if v < 0.0 { -v } else { v }
}

fn sign(v: f32) -> i32 {
    if v < 0.0 { -1 } else { 1 }
}

fn draw(roll: &mut dyn FnMut() -> u32, n: u32) -> u32 {
    (roll() >> 16 & 0x7fff) % n
}

fn chance(roll: &mut dyn FnMut() -> u32, p: i32) -> bool {
    (draw(roll, 100) as i32) < p
}

fn zero(v: &[f32; 4]) -> bool {
    v.iter().all(|c| c.to_bits() == 0)
}

/// The plan's button: 1 ○, 2 ×, 4 △.
pub fn button(plan: u8) -> u32 {
    const B: [u32; 19] = [2, 1, 4, 1, 2, 1, 4, 1, 2, 1, 4, 2, 1, 2, 1, 4, 2, 1, 4];
    B.get(plan as usize).copied().unwrap_or(1)
}

impl Body {
    /// One frame's run speed as the AI's moves estimate it.
    fn step(&self, w: &World) -> f32 {
        let (run, st) = match self.moving {
            0 | 2 => (0, self.stamina),
            1 => (
                self.run + 1,
                player::drain(&self.stats, self.stamina, self.tick, w.players, w.phase == 3, w.floor).0,
            ),
            _ => (self.run, self.stamina),
        };
        player::run_speed(&self.stats, run, st, self.size)
    }

    /// The walk toward `out`: leaves the unit direction there (zero when there, or within ⅔ of a step unless
    /// `keep`); true when it arrives.
    fn walk(&self, w: &World, out: &mut [f32; 4], keep: bool) -> bool {
        let mut d: [f32; 4] = std::array::from_fn(|k| sub(out[k], self.pos[k]));
        if abs(d[0]) < f32::from_bits(0x3a83_126f) {
            d[0] = 0.0;
        }
        if abs(d[2]) < f32::from_bits(0x3a83_126f) {
            d[2] = 0.0;
        }
        (d[1], d[3]) = (0.0, 0.0);
        let mut len = sqrt(madd(mul(d[2], d[2]), d[0], d[0]));
        if len <= 0.0 {
            *out = [0.0; 4];
            return true;
        }
        let inv = div(1.0, len);
        *out = d.map(|c| mul(c, inv));
        let step = self.step(w);
        if !keep && len <= div(mul(2.0, step), 3.0) {
            *out = [0.0; 4];
        } else {
            len = sub(len, step);
        }
        len <= 0.0 || zero(out)
    }

    /// One frame's move along `dir` as the stick would give it.
    fn nudge(&self, w: &World, dir: [f32; 4], me: &mut [f32; 4]) {
        let b = player::bot_stick([dir[0], dir[2]]);
        let [x, z] = player::stick_dir(b, 3);
        let step = self.step(w);
        me[0] = add(me[0], mul(x, step));
        me[2] = add(me[2], mul(z, step));
    }

    /// The ball-lost mark (a computer player's, in the rally).
    fn lose(&mut self, w: &World) {
        if self.control < 0x20 {
            return;
        }
        if w.phase == 3 && self.lost_log.len() < LOST_LOG {
            self.lost_log.push(w.frame);
        }
        self.lost = 2;
    }
}

/// A stroke-frame picker's limits.
struct Pick {
    /// Stop past this many bounces; skip balls nearer the net than `net`, with fewer than `min` bounces.
    stop: i32,
    net: f32,
    min: i32,
    low: f32,
    high: f32,
    /// Distance band; `flat` measures it on the ground.
    near: f32,
    far: f32,
    flat: bool,
    /// The depth the score stands back by.
    depth: f32,
}

impl Rally {
    fn path(&self, c: &PathCopy) -> Vec<PathBall> {
        c.balls.iter().map(|b| PathBall { pos: [b.pos[0], b.pos[1], b.pos[2]], bounces: b.bounces }).collect()
    }

    /// Copies the predicted path (once a frame, shared) unless the ball isn't coming its way; the entries copied.
    pub fn copy_path(&mut self, b: &Body, w: &World, c: &mut PathCopy, seen: &mut [bool], check: bool) -> i32 {
        seen.fill(false);
        if check {
            let h = w.hitter;
            if h < 0 {
                self.len = 0;
                return 0;
            }
            let ok = (h & 1) != (b.team & 1)
                && (self.singles || w.shots > 1 || w.receiver == b.team)
                && (w.shots != 1 || w.hits[0] == w.hits[1]);
            if !ok
                || (w.hits[0] != w.hits[1]
                    && !(w.ball[1] <= f32::from_bits(0xbf68_f5c3))
                    && sign(b.side) == sign(w.ball[2]))
            {
                self.len = 0;
                return 0;
            }
        }
        if c.stamp == w.frame {
            self.len = c.balls.len() as i32;
            return self.len;
        }
        let n = (self.window.max(0) as usize).min(w.path.len());
        c.balls = w.path[..n].to_vec();
        c.stamp = w.frame;
        self.len = n as i32;
        self.len
    }

    fn landing_short(&self, c: &PathCopy) -> bool {
        self.len != 0 && c.balls[self.len as usize - 1].bounces < 2
    }

    /// The path entry `n`.
    fn entry(&self, c: &PathCopy, n: i32) -> Option<[f32; 4]> {
        (n < self.len).then(|| c.balls[n as usize].pos)
    }

    /// The last entry; true when it ends on the other half.
    fn last(&self, b: &Body, c: &PathCopy, out: &mut [f32; 4]) -> bool {
        if self.len == 0 {
            return false;
        }
        *out = c.balls[self.len as usize - 1].pos;
        sign(b.side) != sign(out[2])
    }

    /// The first bounce's descent after it rises; true when it is on the other half.
    fn after_bounce(&self, b: &Body, c: &PathCopy, out: &mut [f32; 4]) -> bool {
        let mut up = false;
        for e in &c.balls[..self.len.max(0) as usize] {
            if !up && e.bounces == 1 && !(e.vel[1] <= 0.0) {
                up = true;
            }
            if up && e.bounces == 1 && e.vel[1] < 0.0 {
                *out = e.pos;
                return sign(b.side) != sign(out[2]);
            }
        }
        false
    }

    /// Where the ball first lands (flying the last entry on if it hasn't), `half` a metre on, a reach to the
    /// nearer side.
    fn landing(&self, b: &Body, w: &World, c: &PathCopy, half: f32, out: &mut [f32; 4]) -> bool {
        let len = self.len.max(0) as usize;
        if c.balls[0].bounces > 0 {
            return false;
        }
        let me = b.pos;
        let horiz = |v: &[f32; 4]| sqrt(madd(mul(v[2], v[2]), v[0], v[0]));
        let mut v;
        if let Some(e) = c.balls[..len].iter().find(|e| e.bounces > 0) {
            *out = e.pos;
            v = e.vel;
            let h = horiz(&v);
            if !(h <= 0.0) {
                let inv = div(1.0, h);
                v = v.map(|c| mul(c, inv));
            }
        } else {
            let e = c.balls[len - 1];
            *out = e.pos;
            v = e.vel;
            while !(out[1] <= 0.0) {
                let s = sqrt(madd(madd(mul(v[1], v[1]), v[0], v[0]), v[2], v[2]));
                v = v.map(|c| msub(add(0.0, c), mul(c, s), w.drag));
                v[1] = msub(add(0.0, v[1]), f32::from_bits(0x3b32_6750), w.gravity);
                for k in 0..4 {
                    out[k] = add(out[k], v[k]);
                }
            }
            let h = horiz(&v);
            if !(h <= 0.0) {
                let inv = div(1.0, h);
                v = v.map(|c| mul(c, inv));
            }
        }
        let p: [f32; 4] = std::array::from_fn(|k| madd(add(0.0, out[k]), v[k], half));
        let rs = mul(b.reach.reach, b.side);
        let dz = sub(p[2], me[2]);
        let (l, r) = (sub(p[0], rs), add(p[0], rs));
        let (dl, dr) = (sub(l, me[0]), sub(r, me[0]));
        let d1 = madd(mul(dz, dz), dl, dl);
        let d2 = madd(mul(dz, dz), dr, dr);
        *out = p;
        out[0] = if d1 <= d2 { l } else { r };
        true
    }

    /// The dive check: false (dive) when a ball 5..16 frames on, on the other half, ahead of the player within
    /// dive height, is just out of its run.
    fn no_dive(&self, b: &Body, c: &PathCopy) -> bool {
        if b.moving != 1 {
            return true;
        }
        let n = (self.len as usize).min(16);
        let speed = sqrt(madd(mul(b.vel[1], b.vel[1]), b.vel[0], b.vel[0]));
        let run = mul(1.25, speed);
        let r = &b.reach;
        let (low, high) = (mul(f32::from_bits(0x3e4c_cccd), r.stroke_height), mul(f32::from_bits(0x3fe6_6666), r.stroke_height));
        let far = mul(f32::from_bits(0x3fa6_6666), r.reach);
        for t in 5..n {
            let e = c.balls[t].pos;
            if abs(e[2]) < 0.5 || sign(b.side) == sign(e[2]) || !(low <= e[1] && e[1] <= high) {
                continue;
            }
            let mut d: [f32; 4] = std::array::from_fn(|k| sub(e[k], b.pos[k]));
            d[1] = 0.0;
            let len = sqrt(madd(mul(d[2], d[2]), d[0], d[0]));
            let inv = div(1.0, len);
            let n = [mul(d[0], inv), mul(0.0, inv), mul(d[2], inv)];
            let f = b.facing;
            let dot = madd(madd(mul(f[1], n[1]), f[0], n[0]), f[2], n[2]);
            if dot < f32::from_bits(0x3f6c_835e) {
                continue;
            }
            let lim = mul(2.0, madd(add(0.0, far), run, t as f32));
            if !(lim < len) {
                return false;
            }
        }
        true
    }

    /// The smash search: the ball nearest the ideal smash height, falling, in the smash window and in time.
    fn smash_search(&self, b: &Body, w: &World, c: &PathCopy, from: i32, min: i32) -> Option<([f32; 4], [f32; 4], i32, i32)> {
        let r = &b.reach;
        let [high, ideal, low] = r.smash;
        let runner = self.runner(b, w);
        let off_x = |x| sub(x, div(mul(b.hand, mul(b.smash_off[0], b.side)), 2.0));
        let off_z = |z| sub(z, div(mul(b.smash_off[1], b.side), 2.0));
        let mut best: Option<(usize, f32, i32)> = None;
        for at in (if from < 0 { self.first } else { from }).max(0) as usize..self.len.max(0) as usize {
            let e = c.balls[at];
            if e.bounces >= 2 {
                break;
            }
            let [x, y, z, _] = e.pos;
            if e.bounces < min || sign(b.side) == sign(z) || abs(z) < 1.5 || !(e.vel[1] <= 0.0) {
                continue;
            }
            if !(low <= y && y <= high) {
                continue;
            }
            let (sx, sz) = (off_x(x), off_z(z));
            if !(abs(sz) < 11.885) {
                continue;
            }
            let score = abs(sub(y, ideal));
            let frames = runner.frames_to([sx, sz]);
            if (at as i32 - self.first) < frames {
                continue;
            }
            if best.is_none_or(|(_, s, _)| score < s) {
                best = Some((at, score, frames));
            }
        }
        let (at, _, frames) = best?;
        let ball = c.balls[at].pos;
        let mut stand = ball;
        stand[0] = off_x(stand[0]);
        stand[2] = off_z(stand[2]);
        Some((stand, ball, frames, at as i32 - self.first))
    }

    fn runner(&self, b: &Body, w: &World) -> Runner {
        Runner {
            stats: b.stats,
            size: b.size,
            side: b.side,
            pos: [b.pos[0], b.pos[2]],
            stamina: b.stamina,
            tick: b.tick,
            run: b.run,
            moving: b.moving,
            players: w.players,
            rally: w.phase == 3,
            floor: w.floor,
        }
    }

    /// A stroke-frame pick: the entry nearest where the player stands back by `depth`; (absolute index, past 8).
    fn pick(&self, b: &Body, c: &PathCopy, p: Pick) -> Option<(i32, bool)> {
        let mut me = b.pos;
        me[1] = b.reach.base;
        let mut best: Option<(usize, f32)> = None;
        for at in self.first.max(0) as usize..self.len.max(0) as usize {
            let e = c.balls[at];
            if e.bounces > p.stop {
                break;
            }
            let [x, y, z, _] = e.pos;
            if abs(z) < p.net || sign(b.side) == sign(z) || e.bounces < p.min || !(p.low <= y && y <= p.high) {
                continue;
            }
            let (dx, dy, dz) = (sub(x, me[0]), sub(y, me[1]), sub(z, me[2]));
            let d = if p.flat {
                sqrt(madd(mul(dz, dz), dx, dx))
            } else {
                sqrt(madd(madd(mul(dy, dy), dx, dx), dz, dz))
            };
            if !(p.near <= d && d <= p.far) {
                continue;
            }
            let score = abs(sub(madd(add(0.0, me[2]), p.depth, b.side), z));
            if best.is_none_or(|(_, s)| score < s) {
                best = Some((at, score));
            }
        }
        best.map(|(at, _)| (at as i32, at >= 9))
    }

    /// The pick for the contact kind: (swing frames).
    fn swing_frame(&self, b: &Body, c: &PathCopy) -> Option<i32> {
        let r = &b.reach;
        let m = mul;
        let far = m(f32::from_bits(0x3fa6_6666), r.reach);
        let base = Pick { stop: 1, net: 0.5, min: 0, low: 0.0, high: 0.0, near: 0.0, far, flat: false, depth: b.depth };
        let got = match self.kind {
            3 => self.pick(b, c, Pick { min: 0, low: r.smash[2], high: r.smash[0], far: r.reach, flat: true, depth: b.smash_off[1], ..base }),
            2 => self.pick(b, c, Pick {
                stop: 0,
                net: if self.singles { 0.5 } else { 1.5 },
                min: i32::MIN,
                high: m(f32::from_bits(0x3fa6_6666), r.volley_height),
                ..base
            }),
            1 => self.pick(b, c, Pick { min: 1, low: r.stroke_height, high: r.volley_height, ..base }),
            0 => (0..4).find_map(|tier| {
                let (h, rr) = (r.stroke_height, r.reach);
                let (lo, hi, near, far) = match tier {
                    0 => (m(0.9, h), m(f32::from_bits(0x3f8c_cccd), h), m(0.9, rr), m(f32::from_bits(0x3f8c_cccd), rr)),
                    1 => (m(0.7, h), m(1.3, h), m(0.7, rr), m(1.3, rr)),
                    2 => (m(0.3, h), m(f32::from_bits(0x3fd9_999a), h), m(0.3, rr), m(1.3, rr)),
                    _ => (0.0, m(3.0, h), 0.0, m(1.3, rr)),
                };
                self.pick(b, c, Pick { low: lo, high: hi, near, far, ..base })
            }),
            _ => None,
        };
        got.map(|(at, _)| at)
    }

    /// The ready check: 1 when a frame's move toward the stand spot brings the ball in reach, 2 when it already
    /// is, 0 when neither.
    fn ready(&self, b: &Body, w: &World) -> i32 {
        let r = &b.reach;
        let far = mul(f32::from_bits(0x3fa6_6666), r.reach);
        let mut me = b.pos;
        me[1] = r.base;
        let (dx, dz) = (sub(self.stand[0], me[0]), sub(self.stand[2], me[2]));
        let len = sqrt(madd(mul(dz, dz), dx, dx));
        let dir = if len <= 0.0 {
            [0.0; 4]
        } else {
            let inv = div(1.0, len);
            [mul(dx, inv), mul(0.0, inv), mul(dz, inv), mul(0.0, inv)]
        };
        b.nudge(w, dir, &mut me);
        for v in 1..=2 {
            let (dx, dy, dz) = (sub(self.ball[0], me[0]), sub(self.ball[1], me[1]), sub(self.ball[2], me[2]));
            let d3 = sqrt(madd(madd(mul(dy, dy), dx, dx), dz, dz));
            let d2 = sqrt(madd(mul(dz, dz), dx, dx));
            let ok = match self.kind {
                3 => d2 <= r.reach,
                2 => d3 <= far,
                0 | 1 => d2 <= far,
                _ => false,
            };
            if ok {
                return v;
            }
            me = b.pos;
            me[1] = r.base;
        }
        0
    }

    fn set_state(&mut self, s: u8, roll: &mut impl FnMut() -> u32) {
        self.state = s;
        match s {
            2 => {
                self.lead = match self.kind {
                    2 => self.leads[1],
                    3 => self.leads[2],
                    _ => self.leads[0],
                };
                if self.push {
                    self.lead = draw(roll, 3) as i32 - 1;
                }
            }
            0 => {
                self.fresh = true;
                self.push = false;
            }
            _ => {}
        }
    }

    /// The return's aim: the plan and the stick.
    fn aim(&mut self, row: &AiParams, b: &Body, w: &World, roll: &mut impl FnMut() -> u32) {
        let deuce = w.court == 0;
        let l = self.level as usize;
        const SURE: [i32; 4] = [50, 60, 70, 80];
        const WIDE: [i32; 4] = [55, 67, 80, 95];
        let out = mul(100.0, sub(abs(b.pos[0]), 5.485));
        let zone = 'z: {
            if !(out <= 0.0) {
                let p = if out <= 50.0 { out } else { 50.0 };
                if chance(roll, div(p, 2.5) as i32) {
                    let lane = crate::position::zone_in([b.pos[0], b.pos[2]], 5.485, None).0;
                    if chance(roll, 50) {
                        self.plan = 10;
                        let mut x = if lane != 0 { 0 } else { 2 };
                        if chance(roll, 30) {
                            x = draw(roll, 3) as u8;
                        }
                        break 'z (x, 2);
                    }
                    self.plan = 11;
                    break 'z (if lane != 0 { 2 } else { 0 }, 0);
                }
            }
            let u = draw(roll, 100) as i32;
            let v = draw(roll, 100) as i32;
            let lv = self.level as i32;
            let short = |me: &mut Rally, roll: &mut dyn FnMut() -> u32| {
                if b.formation == 2 && me.mate == 0 && chance(&mut *roll, 5 * lv) {
                    me.plan = 10;
                }
            };
            if u < SURE[l] {
                self.plan = 7;
                let x = if deuce { 2 } else { 0 };
                if v < 5 * lv + 45 {
                    (x, 0)
                } else if v < 5 * lv + 65 {
                    if chance(roll, 30) {
                        self.plan = 8;
                    }
                    (x, 1)
                } else {
                    if chance(roll, 50) {
                        self.plan = 8;
                    }
                    (x, 2)
                }
            } else if u < WIDE[l] {
                self.plan = 7;
                short(self, roll);
                (if deuce { 0 } else { 2 }, 2)
            } else {
                self.plan = if chance(roll, 60) { 7 } else { 8 };
                if u < 60 {
                    short(self, roll);
                    (1, 2)
                } else if u < 90 {
                    (1, 1)
                } else {
                    (1, 0)
                }
            }
        };
        self.stick = row.aim(zone, b.side, roll);
    }

    /// One frame of the doubles receive. `out` is the run target or stick (as the caller passed it), the button
    /// set when it presses; true once the stroke is over.
    #[allow(clippy::too_many_arguments)]
    pub fn receive(
        &mut self,
        row: &AiParams,
        b: &mut Body,
        w: &World,
        c: &mut PathCopy,
        seen: &mut [bool],
        out: &mut Out,
        roll: &mut impl FnMut() -> u32,
    ) -> bool {
        self.frames -= 1;
        self.index -= 1;
        self.swing -= 1;
        let phase = w.phase;
        let mut done = false;
        let mut s = self.state;
        if s == 0 {
            if self.hold {
                return false;
            }
            if self.wait > 0 {
                self.wait -= 1;
                if self.guess == 0 {
                    return false;
                }
                out.stick[2] = b.pos[2];
                out.stick[0] = match (self.guess, w.court) {
                    (1, 0) => mul(5.485, b.side),
                    (2, 1) => mul(-5.485, b.side),
                    _ => 0.0,
                };
                b.walk(w, &mut out.stick, false);
                if zero(&out.stick) {
                    self.wait = 0;
                }
                return false;
            }
            if self.copy_path(b, w, c, seen, true) == 0 {
                return false;
            }
            let mut found = false;
            let mut searched = false;
            if self.fresh && !self.landing_short(c) {
                self.fresh = false;
                searched = true;
                let path = self.path(c);
                let s = Searcher {
                    row,
                    reach: &b.reach,
                    strong: b.strong,
                    singles: self.singles,
                    beside_human: self.mate > 0,
                    runner: self.runner(b, w),
                    depth: b.depth,
                    path: &path,
                    first: self.first.max(0) as usize,
                    end: self.len.max(0) as usize,
                };
                let hit = |me: &mut Rally, ct: crate::ai::Contact, kind| {
                    let e = c.balls[ct.at].pos;
                    me.stand = [ct.stand[0], e[1], ct.stand[1], e[3]];
                    me.ball = e;
                    me.frames = ct.frames;
                    me.index = ct.at as i32 - me.first;
                    me.kind = kind;
                };
                if row.style == 2 {
                    if let Some(ct) = s.reach_search(None, 1, true, self.body, roll) {
                        hit(self, ct, 1);
                        found = true;
                    }
                }
                if !found {
                    for tier in if self.low { 3 } else { 0 }..4 {
                        if let Some(ct) = s.tier_search(None, tier, 1, true, self.body, self.low, seen, roll) {
                            hit(self, ct, 0);
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    if let Some((stand, ball, frames, index)) = self.smash_search(b, w, c, -1, 1) {
                        (self.stand, self.ball, self.frames, self.index, self.kind) = (stand, ball, frames, index, 3);
                        found = true;
                    }
                }
            }
            let _ = searched;
            let mut go = false;
            if found {
                if self.guess != 0 {
                    go = true;
                } else {
                    self.set_state(1, roll);
                    s = 1;
                }
            } else {
                let mut t = [0.0; 4];
                let short = self.landing_short(c);
                let dive_ok = !short && self.dive && phase != 4 && !self.no_dive(b, c);
                if dive_ok {
                    self.kind = 4;
                    out.button = Some(1);
                    self.set_state(3, roll);
                } else if (short && self.landing(b, w, c, 0.5, &mut t))
                    || self.after_bounce(b, c, &mut t)
                    || self.entry(c, 11).map(|e| t = e).is_some()
                    || self.last(b, c, &mut t)
                {
                    out.stick = t;
                    b.walk(w, &mut out.stick, false);
                }
            }
            if s == 0 {
                // the guess's end
                if self.guess == 0 {
                    return false;
                }
                let f6 = if self.guess == 1 { b.side } else { -b.side };
                self.guess = 0;
                if !go {
                    return false;
                }
                let (dz, dx) = (sub(self.stand[2], self.from[1]), sub(self.stand[0], self.from[0]));
                let inv = div(1.0, sqrt(madd(mul(dz, dz), dx, dx)));
                let (nz, nx) = (mul(dz, inv), mul(dx, inv));
                let dot = madd(madd(add(0.0, 0.0), nx, f6), nz, 0.0);
                if !(dot < 0.5) {
                    self.push = true;
                } else if dot < 0.0 {
                    b.lose(w);
                    out.stick = [0.0; 4];
                    self.wait = row.guess[2];
                    self.set_state(0, roll);
                    return false;
                }
                self.set_state(1, roll);
                s = 1;
            }
        }
        if s == 1 {
            let go = if self.frames == 1 { self.ready(b, w) == 2 } else { self.frames < 1 };
            if !go {
                out.stick = self.stand;
                b.walk(w, &mut out.stick, true);
                return false;
            }
            self.copy_path(b, w, c, seen, true);
            self.swing = self.swing_frame(b, c).unwrap_or(8);
            self.set_state(2, roll);
            s = 2;
        }
        if s == 2 {
            if self.swing + self.lead < 9 {
                self.aim(row, b, w, roll);
                out.button = Some(button(self.plan));
                self.set_state(3, roll);
            }
            return false;
        }
        if s == 3 {
            if b.swing == 1 {
                if self.push {
                    let k = if self.body { 1.5 } else { 2.0 };
                    self.stick = self.stick.map(|c| mul(c, k));
                    let len = sqrt(madd(mul(self.stick[2], self.stick[2]), self.stick[0], self.stick[0]));
                    if !(len <= 1.0) {
                        let inv = div(1.0, len);
                        self.stick = self.stick.map(|c| mul(c, inv));
                    }
                }
                out.stick = self.stick;
            }
            done = b.stroke == 0;
            if b.swing < 0 && w.hitter >= 0 && (w.hitter & 1) != (b.team & 1) && self.wait > 0 {
                self.wait -= 1;
            }
        }
        done
    }
}

/// What a search found: stand, ball, run frames, index (from `first`).
type Found = ([f32; 4], [f32; 4], i32, i32);

impl Rally {
    fn our_hit(b: &Body, w: &World) -> bool {
        w.hitter >= 0 && (w.hitter & 1) == (b.team & 1)
    }

    /// The formation re-pick; a new spot restarts the walk back.
    fn repick(&mut self, b: &Body, w: &World, cue: Cue, roll: &mut impl FnMut() -> u32) {
        let mut f = Formation { lane: self.lane, front: self.front, spot: [self.spot[0], self.spot[2]] };
        let t = Team { side: b.side, formation: b.formation, lean: self.lean };
        let moved = f.repick(&t, cue, [b.pos[0], b.pos[2]], w.ball[2], Self::our_hit(b, w));
        (self.lane, self.front, self.spot[0], self.spot[2]) = (f.lane, f.front, f.spot[0], f.spot[1]);
        if !matches!(cue, Cue::Middle) {
            self.forward = match b.formation {
                0 => self.front,
                f => f == 1,
            };
        }
        if moved {
            self.back = Return::new(self.rate, roll);
        }
    }

    /// Stand at its depth on the half the ball isn't in.
    fn hold_spot(&mut self, b: &Body, w: &World) {
        self.spot = b.pos;
        self.spot[0] = mul(f32::from_bits(0xc02f_851f), sign(w.ball[0]) as f32);
    }

    /// The walk back to the spot: true when it walks this frame (`out` then heads there).
    fn walk_back(&mut self, b: &Body, w: &World, out: &mut Out, roll: &mut impl FnMut() -> u32) {
        if self.back.step(self.rate, self.radius, [self.spot[0], self.spot[2]], [b.pos[0], b.pos[2]], roll) {
            out.stick = self.spot;
            b.walk(w, &mut out.stick, false);
        }
    }

    fn set_sub(&mut self, b: &Body, w: &mut World, s: u8, keep: bool, roll: &mut impl FnMut() -> u32) {
        self.sub = s;
        match s {
            0 => {
                if !keep {
                    (self.fresh, self.next, self.chase) = (true, -1, true);
                    (self.middle, self.follow, self.ours) = (false, false, false);
                    self.back = Return::new(self.rate, roll);
                }
                w.put(b.team, -2, 0, b.pos, [0.0; 4]);
                if self.rounds <= 0 {
                    self.defer_roll = chance(roll, self.defer_rate);
                    self.rounds = draw(roll, 3) as i32 + 2;
                }
                self.defer = false;
            }
            1 => w.put(b.team, self.index, self.kind as i32, b.pos, self.stand),
            2 => {
                self.lead = match self.kind {
                    4 => 0,
                    3 => self.leads[2],
                    2 => self.leads[1],
                    0 | 1 => self.leads[0],
                    _ => self.lead,
                }
            }
            _ => {}
        }
    }

    /// The partner's voice line on giving way (one roll, a second when it speaks).
    fn call(&mut self, b: &Body, roll: &mut impl FnMut() -> u32) {
        if self.voice && b.mate_voice < 2 && chance(roll, 25) {
            roll(); // ponytail: picks the voice line; the sound isn't played (P11n)
        }
        self.voice = false;
    }

    /// Leave the ball to the partner: its own and the partner's shot records compared.
    fn give_way(&mut self, b: &Body, mine: &Shot, theirs: &Shot) -> bool {
        if self.defer {
            return false;
        }
        let (t, m) = (theirs.n, mine.n);
        let run = |s: &Shot| {
            let (dz, dx) = (sub(s.vec[2], s.pos[2]), sub(s.vec[0], s.pos[0]));
            sqrt(madd(mul(dz, dz), dx, dx))
        };
        let yes = t >= 0 && {
            let (f4, f3) = (run(mine), run(theirs));
            if self.mate > 0 {
                if f4 < 2.5 && add(1.0, abs(b.pos[2])) < abs(b.mate[2]) {
                    false
                } else if f3 < 2.0 {
                    true
                } else if f4 < 2.5 {
                    false
                } else if t.wrapping_sub(m).wrapping_abs() < 30 {
                    add(f3, 1.0) < f4
                } else {
                    t < m
                }
            } else if t < m && f3 < f4 {
                true
            } else if m < t && f4 < f3 {
                false
            } else if f3 < 2.5 || f4 < 2.5 {
                f3 < f4
            } else {
                t < m
            }
        };
        if !yes && self.mate > 0 && (t == -2 || t >= 0) {
            self.defer = true;
        }
        yes
    }

    /// The entry after the last one with `k` bounces, when the path ends past `k` and `t` (as it comes in) lies on the
    /// other half.
    fn after(&self, b: &Body, c: &PathCopy, k: i32, t: &mut [f32; 4]) -> bool {
        let len = self.len as usize;
        if self.len < 2 || c.balls[len - 1].bounces <= k {
            return false;
        }
        let Some(a) = (0..len - 1).rev().find(|&a| c.balls[a].bounces == k) else { return false };
        let v = sign(b.side) != sign(t[2]);
        if v {
            *t = c.balls[a + 1].pos;
        }
        v
    }

    /// The partner stands nearer `t` than it does.
    fn mate_nearer(b: &Body, t: &[f32; 4]) -> bool {
        let d = |p: &[f32; 4]| {
            let (dz, dx) = (sub(t[2], p[2]), sub(t[0], p[0]));
            madd(mul(dz, dz), dx, dx)
        };
        d(&b.mate) < d(&b.pos)
    }

    /// Its team plays forward and the ball is on the other half, or nearer the net than it on this one.
    fn forward_ball(&self, b: &Body, w: &World) -> bool {
        let (oz, bz) = (b.pos[2], w.ball[2]);
        self.forward && (sign(oz) != sign(bz) || abs(bz) <= abs(oz))
    }

    /// The volley search: before the bounce, on the other half past 1.5, under 1.3 volley heights and in reach of
    /// the base height, nearest the volley height, in time; stops at the first found unless `all`.
    fn volley_search(&self, b: &Body, w: &World, c: &PathCopy, from: i32, all: bool, body: bool) -> Option<Found> {
        let r = &b.reach;
        let vh = r.volley_height;
        let top = mul(f32::from_bits(0x3fa6_6666), vh);
        let r2 = mul(r.reach, r.reach);
        let runner = self.runner(b, w);
        let (mx, mz) = (b.pos[0], b.pos[2]);
        let mut best: Option<(i32, f32, i32, [f32; 4])> = None;
        let mut at = if from < 0 { self.first } else { from };
        while at < self.len {
            let e = c.balls[at as usize];
            if e.bounces > 0 {
                break;
            }
            let p = e.pos;
            if sign(b.side) != sign(p[2]) && !(abs(p[2]) < 1.5) {
                'c: {
                    if !(0.0 <= p[1] && p[1] <= top) {
                        break 'c;
                    }
                    let dy = sub(p[1], r.base);
                    let f = msub(add(0.0, r2), dy, dy);
                    if f < 0.0 {
                        break 'c;
                    }
                    let half = if body { 0.0 } else { sqrt(f) };
                    let (l, rr) = (sub(p[0], half), add(p[0], half));
                    let dz = sub(p[2], mz);
                    let (dl, dr) = (sub(l, mx), sub(rr, mx));
                    let mut s = p;
                    s[0] = if madd(mul(dz, dz), dl, dl) <= madd(mul(dz, dz), dr, dr) { l } else { rr };
                    let score = abs(sub(p[1], vh));
                    s[2] = sub(p[2], div(mul(b.depth, b.side), 2.0));
                    let frames = runner.frames_to([s[0], s[2]]);
                    if at - self.first < frames {
                        break 'c;
                    }
                    if best.is_none_or(|(_, bs, _, _)| !(bs <= score)) {
                        best = Some((at, score, frames, s));
                    }
                }
                if !all && best.is_some() {
                    break;
                }
            }
            at += 1;
        }
        let (at, _, frames, stand) = best?;
        Some((stand, c.balls[at as usize].pos, frames, at - self.first))
    }

    fn pair_aim(&mut self, row: &AiParams, b: &Body, roll: &mut impl FnMut() -> u32) {
        let xz = |p: [f32; 4]| [p[0], p[2]];
        let a = row.pair_aim(
            &Pair {
                me: xz(b.pos),
                mate: xz(b.mate),
                opp: [xz(b.opp[0]), xz(b.opp[1])],
                side: b.side,
                singles: self.singles,
                kind: self.kind,
                volley_level: self.volley_level,
                level: self.level,
                formation: b.formation,
                smash_third: self.mate != 0,
            },
            roll,
        );
        (self.stick, self.plan) = (a.stick, a.plan);
    }

    /// The searches in the NET (`net`) or BASE order: the contact kind found.
    #[allow(clippy::too_many_arguments)]
    fn search(&mut self, net: bool, row: &AiParams, b: &Body, w: &World, c: &PathCopy, seen: &mut [bool], roll: &mut impl FnMut() -> u32) -> Option<u8> {
        let take = |me: &mut Rally, (stand, ball, frames, index): Found| {
            (me.stand, me.ball, me.frames, me.index) = (stand, ball, frames, index);
        };
        if let Some(f) = self.smash_search(b, w, c, self.next, 0) {
            take(self, f);
            return Some(3);
        }
        let near = self.forward_ball(b, w) || (self.mate > 0 && abs(b.pos[2]) < abs(b.mate[2]));
        if net || near {
            if let Some(f) = self.volley_search(b, w, c, self.next, !net, self.body) {
                take(self, f);
                return Some(2);
            }
        }
        let path = self.path(c);
        let s = Searcher {
            row,
            reach: &b.reach,
            strong: b.strong,
            singles: self.singles,
            beside_human: self.mate > 0,
            runner: self.runner(b, w),
            depth: b.depth,
            path: &path,
            first: self.first.max(0) as usize,
            end: self.len.max(0) as usize,
        };
        let hit = |me: &mut Rally, ct: crate::ai::Contact| {
            let e = c.balls[ct.at].pos;
            me.stand = [ct.stand[0], e[1], ct.stand[1], e[3]];
            (me.ball, me.frames, me.index) = (e, ct.frames, ct.at as i32 - me.first);
        };
        let tiers = if self.low { 3 } else { 0 }..4;
        if near {
            let from = (self.next >= 0).then_some(self.next as usize);
            for tier in tiers {
                if let Some(ct) = s.tier_search(from, tier, 0, !net, self.body, self.low, seen, roll) {
                    hit(self, ct);
                    return Some(0);
                }
            }
        } else if self.fresh && !self.landing_short(c) {
            self.fresh = false;
            if !net {
                if let Some(ct) = s.reach_search(None, 1, true, self.body, roll) {
                    hit(self, ct);
                    return Some(1);
                }
            }
            for min in [1, 0] {
                for tier in tiers.clone() {
                    if let Some(ct) = s.tier_search(None, tier, min, true, self.body, self.low, seen, roll) {
                        hit(self, ct);
                        return Some(0);
                    }
                }
            }
        }
        None
    }

    /// One frame of the doubles NET (`net`) or BASE rally routine. `out` is the run target or stick (as the caller
    /// passed it), the button set when it presses.
    #[allow(clippy::too_many_arguments)]
    pub fn rally(
        &mut self,
        net: bool,
        row: &AiParams,
        b: &mut Body,
        w: &mut World,
        c: &mut PathCopy,
        seen: &mut [bool],
        out: &mut Out,
        roll: &mut impl FnMut() -> u32,
    ) {
        self.frames -= 1;
        self.index -= 1;
        self.swing -= 1;
        let (frame, phase) = (w.frame, w.phase);
        let (slot, mate) = (b.team, b.team ^ 2);
        let mut s = self.sub;
        if s == 0 {
            if self.hold || (phase == 4 && w.shots == 1) || w.shots < 2 || self.held {
                return;
            }
            if self.wait > 0 {
                self.wait -= 1;
                return;
            }
            if !self.ours && Self::our_hit(b, w) {
                (self.ours, self.tick) = (true, 0);
            }
            if self.ours {
                self.tick += 1;
                if self.tick >= 30 {
                    self.tick = 0;
                    let at = [b.mate[0], b.mate[2]];
                    self.repick(b, w, if w.hitter == slot { Cue::Me(at) } else { Cue::Other(at) }, roll);
                }
                self.walk_back(b, w, out, roll);
                return;
            }
            if !self.follow {
                let p = w.shot(mate);
                if p.n >= 0 && (!self.fresh || (self.front && sign(b.pos[2]) == sign(w.ball[2]))) {
                    self.repick(b, w, Cue::Other([p.vec[0], p.vec[2]]), roll);
                    self.follow = true;
                }
            }
            if self.follow {
                let p = w.shot(mate);
                let (pz, bz, oz) = (b.mate[2], w.ball[2], b.pos[2]);
                let same = sign(pz) == sign(bz);
                let check = p.n < 0
                    || p.n < frame
                    || (p.n < frame + 8 && same && !(abs(bz) <= abs(pz)))
                    || (!(abs(bz) <= add(2.0, abs(pz))) && same);
                if check {
                    let near = || {
                        let (dz, dx) = (sub(oz, bz), sub(b.pos[0], w.ball[0]));
                        sqrt(madd(mul(dz, dz), dx, dx)) <= 3.0
                    };
                    if !self.middle && (sign(oz) != sign(bz) || abs(bz) <= abs(oz) || near()) {
                        (self.follow, self.fresh) = (false, true);
                    } else {
                        self.hold_spot(b, w);
                    }
                }
            }
            if self.follow {
                let p = w.shot(mate);
                if p.n >= 0 {
                    self.repick(b, w, Cue::Other([p.vec[0], p.vec[2]]), roll);
                }
                self.walk_back(b, w, out, roll);
                return;
            }
            let n = self.copy_path(b, w, c, seen, true);
            if n <= 0 {
                return;
            }
            if let Some(kind) = self.search(net, row, b, w, c, seen, roll) {
                self.kind = kind;
                self.set_sub(b, w, 1, false, roll);
                s = 1;
            } else {
                let mut t = self.stash;
                let mut go = |me: &mut Rally, t: [f32; 4]| {
                    out.stick = t;
                    if b.walk(w, &mut out.stick, false) {
                        me.chase = false;
                    }
                };
                if self.landing_short(c) {
                    if self.landing(b, w, c, f32::from_bits(0x3e99_999a), &mut t)
                        || self.after_bounce(b, c, &mut t)
                        || self.entry(c, 11).map(|e| t = e).is_some()
                        || self.last(b, c, &mut t)
                    {
                        let chase = self.chase;
                        go(self, t);
                        self.chase = chase;
                    }
                } else {
                    if self.after(b, c, 1, &mut t) {
                        let p = w.shot(mate);
                        if p.n >= 0 {
                            self.repick(b, w, Cue::Other([p.vec[0], p.vec[2]]), roll);
                            self.follow = true;
                            self.call(b, roll);
                            self.stash = t;
                            return;
                        }
                        if self.mate == 0 && Self::mate_nearer(b, &t) {
                            self.after(b, c, 0, &mut t);
                            if self.mate == 0 && Self::mate_nearer(b, &t) {
                                self.repick(b, w, Cue::Middle, roll);
                                (self.follow, self.middle) = (true, true);
                                self.stash = t;
                                return;
                            }
                        }
                    }
                    if self.dive && phase != 4 && !self.no_dive(b, c) {
                        self.kind = 4;
                        self.pair_aim(row, b, roll);
                        out.button = Some(button(self.plan));
                        self.set_sub(b, w, 3, false, roll);
                    } else if self.chase
                        && (self.after_bounce(b, c, &mut t)
                            || self.entry(c, 11).map(|e| t = e).is_some()
                            || self.last(b, c, &mut t))
                    {
                        go(self, t);
                    }
                }
                self.stash = t;
                self.next = n - 1;
            }
        }
        if s == 1 {
            let (mine, theirs) = (w.shot(slot), w.shot(mate));
            if self.give_way(b, &mine, &theirs) {
                self.repick(b, w, Cue::Other([theirs.vec[0], theirs.vec[2]]), roll);
                self.follow = true;
                self.call(b, roll);
                self.set_sub(b, w, 0, true, roll);
                return;
            }
            let go = if self.frames == 1 { self.ready(b, w) == 2 } else { self.frames < 1 };
            if !go {
                out.stick = self.stand;
                b.walk(w, &mut out.stick, true);
                return;
            }
            self.copy_path(b, w, c, seen, true);
            self.swing = self.swing_frame(b, c).unwrap_or(8);
            self.set_sub(b, w, 2, false, roll);
            s = 2;
        }
        if s == 2 {
            let mine = w.shot(slot);
            if mine.n >= 0 && mine.n < frame {
                w.put(slot, -2, 0, b.pos, [0.0; 4]);
            }
            if self.swing + self.lead < 9 {
                if w.path_mode == 2 && !(w.path_gap < row.line_margin) {
                    self.held = true;
                    self.set_sub(b, w, 0, false, roll);
                } else {
                    self.pair_aim(row, b, roll);
                    out.button = Some(button(self.plan));
                    self.set_sub(b, w, 3, false, roll);
                }
            }
            return;
        }
        if s == 3 {
            if b.swing == 1 {
                out.stick = self.stick;
            }
            if b.mark != 0 {
                w.put(slot, -3, 0, b.pos, [0.0; 4]);
            }
            if b.stroke == 0 {
                self.set_sub(b, w, 0, false, roll);
            }
            if b.swing < 0 && w.hitter >= 0 && (w.hitter & 1) != (b.team & 1) && self.wait > 0 {
                self.wait -= 1;
            }
        }
    }
}
