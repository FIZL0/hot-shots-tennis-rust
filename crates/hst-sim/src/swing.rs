//! The original's stroke contact search: when a shot button is pressed, the player scans the ball's predicted
//! path for the frame to hit it on, which branch (smash, volley, ground stroke) and which swing (forehand,
//! backhand, body shot). Ported operation for operation; the path is the predictor's (court plane only).
#![deny(clippy::float_arithmetic)]

use crate::lines::{BASELINE, SERVICE_LINE, sideline};
use crate::pose::M4;
use crate::ps2::{F, utof};
use crate::rng::{TWO_POW_M32, coin, r15};
use crate::serve::SWEET_FRAME;
use crate::shot::{branch, class, kind};
use std::f32::consts::FRAC_1_SQRT_2;

/// The swing motions a search picks (`Swing::anim`); one more is the same swing on the other hand's side.
pub mod anim {
    /// Ground strokes by shot kind: topspin (and flat, drop), slice, lob.
    pub const TOPSPIN: u8 = 0x10;
    pub const SLICE: u8 = 0x12;
    pub const LOB: u8 = 0x14;
    /// High and low volleys.
    pub const VOLLEY_HIGH: u8 = 0x16;
    pub const VOLLEY_LOW: u8 = 0x18;
    /// The body shot (ground stroke or volley).
    pub const BODY: u8 = 0x1a;
    pub const SMASH: u8 = 0x1f;
}

/// Timing grades (the timing tables' values by frame from the press).
pub mod grade {
    /// The frame can't hit.
    pub const NONE: u8 = 0;
    /// The sweet frame, and the clean frames around it.
    pub const SWEET: u8 = 1;
    pub const CLEAN: u8 = 2;
    /// Off the timing: a frame's bias each, then the slow frames (two each, and the mis-hit roll).
    pub const OFF: u8 = 3;
    pub const SLOW: u8 = 4;

    /// A clean hit: no timing error.
    pub fn clean(g: u8) -> bool {
        g == SWEET || g == CLEAN
    }
}

/// The search only takes a ball on the player's own half, at least this far from the net.
const NET_CLEAR: f32 = 0.5;
/// The reach and the volley height stretched by this.
const STRETCH: f32 = 1.3;
/// A dive only goes for a ball within 22.5° of the facing (cos), and a blocked dive ends past 25° off its line.
const DIVE_CONE: f32 = f32::from_bits(0x3f6c_835e);
const DIVE_STRAY: f32 = f32::from_bits(0x3f68_03ca);
/// Timing errors are held to ±1 m, in tenths.
const MAX_ERROR: i32 = 10;
/// Power stats run 0..=16.
const STAT_MAX: i32 = 16;

/// One predicted ball frame: position in game space (Y down) and bounces so far.
#[derive(Clone, Copy, Debug)]
pub struct PathPoint {
    pub pos: [f32; 3],
    pub bounces: i32,
}

/// A player's reach for the search. Character data (TParam.csv) except `ahead`, `smash_ahead` and the body-shot
/// widths, which the game measures on the character's swing animations at their contact frame.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Reach {
    /// Height of the reach centre (m).
    pub base: f32,
    /// Horizontal reach (m).
    pub reach: f32,
    /// Ground strokes / volleys: ideal contact height; a volley below it is a low volley.
    pub stroke_height: f32,
    pub volley_height: f32,
    /// Smash window (m).
    pub smash_top: f32,
    pub smash_bottom: f32,
    /// Contact point distance in front of the body: strokes / smash.
    pub ahead: f32,
    pub smash_ahead: f32,
    /// A ball closer sideways than this is a body shot: low volley or stroke / high volley.
    pub body_low: f32,
    pub body_high: f32,
    /// Timing grade by frames from the press (0 = can't hit there). Its length is the search horizon.
    pub grades: Vec<u8>,
    /// +1 right-handed, −1 left-handed.
    pub hand: f32,
    /// The dive arm: model matrices of the racket and the right upper arm in the receive motion at time 5
    /// (`pose::dive_arm`).
    pub dive_arm: [M4; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Branch {
    Ground,
    Volley,
    Smash,
}

/// The swing a press locked onto.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Swing {
    pub branch: Branch,
    /// Path index of the contact (frames from now).
    pub frame: usize,
    /// The ball there.
    pub ball: [f32; 3],
    /// Ball on the racket-hand side (in the player's frame, mirrored for left-handers).
    pub forehand: bool,
    /// Ball too close sideways for a full swing.
    pub body: bool,
    /// Swing motion (`anim`; +1 for the other hand's side, never on the smash).
    pub anim: u8,
}

/// Search `path` (index 0 = this frame) for the contact of a shot of kind `kind` (`shot::kind`) by a player at
/// `pos` facing `facing` (+1 toward +z).
pub fn search(r: &Reach, path: &[PathPoint], pos: [f32; 3], facing: f32, kind: i32) -> Option<Swing> {
    let n = path.len().min(r.grades.len());
    // per frame: horizontal distance and depth from the stroke / smash contact points
    let horiz: Vec<f32> = path[..n]
        .iter()
        .map(|p| {
            let (dx, dz) = ((F(p.pos[0]) - pos[0]).0, (F(p.pos[2]) - pos[2]).0);
            (F(dx) * dx + F(dz) * dz).sqrt().0
        })
        .collect();
    let depth = |ahead: f32, p: &PathPoint| (F(ahead) * facing + pos[2] + 0.0 - p.pos[2]).0.abs();
    // ball on our half and clear of the net, with a timing grade
    let ours = |k: usize| {
        let z = path[k].pos[2];
        z.abs() >= NET_CLEAR && (z >= 0.0) != (facing >= 0.0) && r.grades[k] != grade::NONE
    };
    let best = |max_bounces: i32, ok: &dyn Fn(usize) -> bool, metric: &dyn Fn(usize) -> f32| {
        let mut best: Option<usize> = None;
        for k in 0..n {
            if path[k].bounces > max_bounces {
                break;
            }
            if ours(k) && ok(k) && best.is_none_or(|b| metric(k) < metric(b)) {
                best = Some(k);
            }
        }
        best
    };
    let height = |k: usize| (-F(path[k].pos[1])).0;
    let swing = |branch, k: usize, body: bool, base: u8| {
        let side = ((F(path[k].pos[0]) - pos[0]) * facing).0;
        let forehand = side >= 0.0;
        let other = if forehand { r.hand < 0.0 } else { r.hand >= 0.0 };
        Swing { branch, frame: k, ball: path[k].pos, forehand, body, anim: base + other as u8 }
    };

    let smash = best(1, &|k| r.smash_bottom <= height(k) && height(k) <= r.smash_top && horiz[k] <= r.reach, &|k| depth(r.smash_ahead, &path[k]));
    if let Some(k) = smash {
        return Some(Swing { anim: anim::SMASH, ..swing(Branch::Smash, k, false, 0) });
    }
    let wide = (F(r.reach) * STRETCH).0;
    let volley = best(0, &|k| 0.0 <= height(k) && height(k) <= (F(r.volley_height) * STRETCH).0 && horiz[k] <= wide, &|k| depth(r.ahead, &path[k]));
    if let Some(k) = volley {
        let low = height(k) < r.stroke_height;
        let body = (F(path[k].pos[0]) - pos[0]).0.abs() <= if low { r.body_low } else { r.body_high };
        let base = if body { anim::BODY } else if low { anim::VOLLEY_LOW } else { anim::VOLLEY_HIGH };
        return Some(swing(Branch::Volley, k, body, base));
    }
    let ground = best(1, &|k| height(k) <= (F(r.base) + wide).0 && horiz[k] <= wide, &|k| depth(r.ahead, &path[k]));
    ground.map(|k| {
        let body = (F(path[k].pos[0]) - pos[0]).0.abs() <= r.body_low;
        let base = if body {
            anim::BODY
        } else {
            match kind {
                kind::SLICE => anim::SLICE,
                kind::LOB => anim::LOB,
                _ => anim::TOPSPIN,
            }
        };
        swing(Branch::Ground, k, body, base)
    })
}

/// Frames a dive search looks ahead.
pub const DIVE_HORIZON: usize = 16;
/// Frames on the ground after the dive's slide (the receive motion's recovery).
pub const DIVE_RECOVERY: usize = 50;

/// A dive: what a running player does when the press finds no stroke, a lunge toward the ball along `dir`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Dive {
    /// Path index the slide ends on (the contact, if `contact`).
    pub frame: usize,
    /// The racket reaches the ball there (else the dive only lunges toward it).
    pub contact: bool,
    /// Unit horizontal direction (x, z) toward the ball; the new facing.
    pub dir: [f32; 2],
    /// Slide length over `frame` frames (m).
    pub slide: f32,
    /// Frames into the dive.
    pub tick: usize,
    /// Where it started (x, z), the offset applied so far, the tick the root path's time counts from and
    /// whether a blocked move ended the lunge.
    start: [f32; 2],
    prev: [f32; 2],
    origin: usize,
    stopped: bool,
    /// A blocked move cut the slide or the lunge short; it stays set after the dive.
    cut: bool,
}

/// The dive branch of the search, tried after `search` fails while the player runs: a player at `pos`, court end
/// `end` (±1), facing `face` (x, z) and moving at `vel` (x, z per frame).
pub fn dive(r: &Reach, path: &[PathPoint], pos: [f32; 3], end: f32, face: [f32; 2], vel: [f32; 2]) -> Option<Dive> {
    let speed = ((F(vel[0]) * vel[0] + F(vel[1]) * vel[1]).sqrt() * 1.25).0;
    let wide = (F(r.reach) * STRETCH).0;
    let (mut hit, mut miss): (Option<(usize, f32)>, Option<(usize, f32, f32)>) = (None, None);
    for (k, p) in path.iter().enumerate().take(DIVE_HORIZON).skip(5) {
        let (z, h) = (p.pos[2], (-F(p.pos[1])).0);
        if !(z.abs() >= NET_CLEAR && (z >= 0.0) != (end >= 0.0) && (F(r.stroke_height) * 0.2).0 <= h && h <= (F(r.stroke_height) * 1.8).0) {
            continue;
        }
        let (dx, dz) = ((F(p.pos[0]) - pos[0]).0, (F(z) - pos[2]).0);
        let dist = (F(dx) * dx + F(dz) * dz).sqrt().0;
        let inv = (F(1.0) / dist).0;
        let dot = (face[1] * (F(dz) * inv) + face[0] * (F(dx) * inv) + 0.0).0;
        if dot < DIVE_CONE {
            continue;
        }
        let lunge = (F(speed) * (k as f32)).0;
        let limit = (F(lunge) + wide).0;
        if !(limit < dist) && p.bounces <= 1 {
            if hit.is_none_or(|(_, best)| z.abs() > best) {
                hit = Some((k, z.abs()));
            }
        } else if dist <= (F(limit) * 2.0).0 && miss.is_none_or(|(_, best, _)| dot > best) {
            miss = Some((k, dot, lunge));
        }
    }
    let (k, contact) = match (hit, miss) {
        (Some((k, _)), _) => (k, true),
        (None, Some((k, _, _))) => (k, false),
        _ => return None,
    };
    let (dx, dz) = ((F(path[k].pos[0]) - pos[0]).0, (F(path[k].pos[2]) - pos[2]).0);
    let inv = (1.0 / (F::ZERO * 0.0).madd(dx, dx).madd(dz, dz).sqrt()).0;
    let dir = [(F(dx) * inv).0, (F(dz) * inv).0];
    // the game's predicted path holds the ball with y up
    let ball = [path[k].pos[0], (-F(path[k].pos[1])).0, path[k].pos[2]];
    let slide = if contact { arm_slide(&r.dive_arm, &dive_matrix(pos, dir, r.hand), ball, dir) } else { miss.unwrap().2 };
    Some(Dive { frame: k, contact, dir, slide, tick: 0, start: [pos[0], pos[2]], prev: [0.0; 2], origin: k, stopped: false, cut: false })
}

/// How far a contact dive slides so the arm reaches the ball: the racket point and the shoulder of the dive arm
/// `arm` turned by the player matrix `m` (rows side, up, `dir`, position; the arm takes only its turn), against the
/// ball (y up, as the predicted path holds it; the arm's model is y down) from the matrix's position; 0 when the arm already reaches. `dir` is the dive's unit direction (x, z).
pub fn arm_slide(arm: &[M4; 2], m: &M4, ball: [f32; 3], dir: [f32; 2]) -> f32 {
    use crate::pose::{TIP, vmul};
    use crate::vu0::transform;
    let d = [(F(ball[0]) - m[3][0]).0, (F(ball[1]) * -1.0 - m[3][1]).0, (F(ball[2]) - m[3][2]).0];
    let turn = [m[0], m[1], m[2], [0.0, 0.0, 0.0, 1.0]];
    let tip = transform(&vmul(&arm[0], &turn), TIP);
    let sh = vmul(&arm[1], &turn)[3];
    let (tx, ty, tz) = (F(tip[0]) - sh[0], F(tip[1]) - sh[1], F(tip[2]) - sh[2]);
    let reach = (ty * ty).madd(tx, tx).madd(tz, tz);
    let (ax, ay, az) = (F(d[0]) - sh[0], F(d[1]) - sh[1], F(d[2]) - sh[2]);
    let dist = (ay * ay).madd(ax, ax).madd(az, az);
    if dist.sqrt() <= reach.sqrt() {
        return 0.0;
    }
    // the ball's distance off the line through the shoulder along `dir`, and the arm's reach left across it
    let t = -(F::ZERO + F(dir[1]) * (F(sh[2]) - d[2])).madd(dir[0], F(sh[0]) - d[0]);
    let px = F(d[0]) - (F::ZERO + sh[0]).madd(dir[0], t);
    let pz = F(d[2]) - (F::ZERO + sh[2]).madd(dir[1], t);
    let side = (F::ZERO + (F::ZERO + pz * pz).madd(px, px)).madd(ay, ay);
    ((F::ZERO + az * az).madd(ax, ax).sqrt() - (reach - side).sqrt()).0
}

/// The player matrix the dive's slide is measured in: side (up × `dir`, by the hand), up, `dir` (x, z), `pos`.
fn dive_matrix(pos: [f32; 3], dir: [f32; 2], hand: f32) -> M4 {
    // up × direction by the hand, its zeros signed as the game's
    let side = [(F(dir[1]) * hand).0, (F::ZERO * hand).0, ((F::ZERO - dir[0]) * hand).0, (F(hand) * 0.0).0];
    [side, [0.0, 1.0, 0.0, 0.0], [dir[0], 0.0, dir[1], 0.0], [pos[0], pos[1], pos[2], 1.0]]
}

impl Dive {
    /// A blocked move cut the slide or the lunge short.
    pub fn cut(&self) -> bool {
        self.cut
    }

    /// Frames the dive locks the player, counting the search frame.
    pub fn len(&self) -> usize {
        self.frame + DIVE_RECOVERY + 1
    }

    /// One frame of the dive for a player at `pos` (x, z); `None` once over. The slide is spread evenly up to
    /// `frame`, then the receive motion's root path is followed along `dir` (`root_z`: its forward offset at a
    /// motion time). `mover` applies a step and returns the new position; a step it alters that leaves the
    /// player more than 25° off `dir` from the start ends the slide (or, later, the lunge).
    pub fn step(&mut self, pos: [f32; 2], root_z: impl Fn(f32) -> f32, mover: impl FnOnce([f32; 2]) -> [f32; 2]) -> Option<[f32; 2]> {
        let (n, k) = (self.tick, self.frame);
        if n >= self.len() {
            return None;
        }
        self.tick += 1;
        if n > k && self.stopped {
            return Some(pos);
        }
        let at = if n <= k {
            let f = (F(n as f32) / (k as f32)).0;
            [(F(self.dir[0]) * self.slide * f).0, (F(self.dir[1]) * self.slide * f).0]
        } else {
            // the motion is held at time 4 through the slide, so it is 5 when the root phase starts
            let z = root_z((n - self.origin + 5) as f32);
            [(F(self.dir[0]) * z).0, (F(self.dir[1]) * z).0]
        };
        let delta = [(F(at[0]) - self.prev[0]).0, (F(at[1]) - self.prev[1]).0];
        let now = mover(delta);
        self.prev = if n == k { [0.0; 2] } else { at };
        if now != [(F(pos[0]) + delta[0]).0, (F(pos[1]) + delta[1]).0] {
            let (dx, dz) = ((F(now[0]) - self.start[0]).0, (F(now[1]) - self.start[1]).0);
            let inv = (1.0 / (F(dx) * dx + F(dz) * dz).sqrt()).0;
            if (F(dz) * inv * self.dir[1] + F(dx) * inv * self.dir[0] + 0.0).0 < DIVE_STRAY {
                if n <= k {
                    // ponytail: the slide's early end restarts the motion at time 5, as decompiled; no recorded case
                    self.prev = [0.0; 2];
                    self.tick = k + 2;
                    self.origin = k + 1;
                } else {
                    self.stopped = true;
                }
                self.cut = true;
            }
        }
        Some(now)
    }
}

/// A press whose search found nothing swings a smash at nothing (`anim::SMASH`) when the ball is within 2 m across the
/// ground of `pos` and the path rises above `middle` (the smash window's middle height) before its 2nd bounce.
pub fn smash_whiff(path: &[PathPoint], ball: [f32; 3], pos: [f32; 3], middle: f32) -> bool {
    let (dx, dz) = (F(ball[0]) - pos[0], F(ball[2]) - pos[2]);
    (dz * dz).madd(dx, dx) < 4.0 && path.iter().take_while(|p| p.bounces <= 1).any(|p| middle < -F(p.pos[1]))
}

/// The approach run a press starts before its search (the original's stroke state, on a press while standing or
/// running): a unit direction square to the ball's line, toward it from `pos` (x, z), and how many frames to run
/// it before the one search. `path` is the ball from this frame, at most the horizon plus 20 frames long. Each
/// frame from 0 to 20 checks the window of `grades.len()` frames starting there against the player's position
/// after that many steps, first match per branch (smash, volley, ground): found at 0 searches at once. `step`
/// moves the position one frame along the direction (the run speed and the mover), None when the mover clamps
/// it (court edge, net, partner). None: nothing within 20 frames, or blocked.
pub fn approach(r: &Reach, path: &[PathPoint], pos: [f32; 3], facing: f32, mut step: impl FnMut([f32; 2], [f32; 2]) -> Option<[f32; 2]>) -> Option<(usize, [f32; 2])> {
    let n = path.len();
    let dir = approach_dir(path, pos);
    let look = r.grades.len();
    let height = |k: usize| (-F(path[k].pos[1])).0;
    let wide = (F(r.reach) * STRETCH).0;
    let mut at = [pos[0], pos[2]];
    for i in 0..=20 {
        let end = (look + i).min(n);
        let (mut d3, mut d2) = (vec![0.0; n], vec![0.0; n]);
        for k in i..end {
            let (x, y, z) = ((F(path[k].pos[0]) - at[0]).0, (F(height(k)) - r.base).0, (F(path[k].pos[2]) - at[1]).0);
            d3[k] = (F(z) * z + F(x) * x + F(y) * y).sqrt().0;
            d2[k] = (F(x) * x + F(z) * z).sqrt().0;
        }
        let first = |max_bounces: i32, ok: &dyn Fn(usize) -> bool| {
            (i..end).take_while(|&k| path[k].bounces <= max_bounces).find(|&k| {
                let z = path[k].pos[2];
                z.abs() >= NET_CLEAR && (z >= 0.0) != (facing >= 0.0) && r.grades[k - i] != grade::NONE && ok(k)
            })
        };
        let found = first(1, &|k| r.smash_bottom <= height(k) && height(k) <= r.smash_top && d2[k] <= r.reach)
            .or_else(|| first(0, &|k| 0.0 <= height(k) && height(k) <= (F(r.volley_height) * STRETCH).0 && d3[k] <= wide))
            .or_else(|| first(1, &|k| height(k) <= (F(r.base) + wide).0 && d2[k] <= wide));
        if found.is_some() {
            return Some((i, dir));
        }
        if i == 20 {
            break;
        }
        at = step(at, dir)?;
    }
    None
}

/// The approach direction: square to the ball's line from `path[0]` to its end, turned toward that line from `pos`.
pub fn approach_dir(path: &[PathPoint], pos: [f32; 3]) -> [f32; 2] {
    let n = path.len();
    let unit = |x: f32, z: f32| {
        let inv = (1.0 / (F(x) * x + F(z) * z).sqrt()).0;
        [(F(x) * inv).0, (F(z) * inv).0]
    };
    let [dx, dz] = unit((F(path[n - 1].pos[0]) - path[0].pos[0]).0, (F(path[n - 1].pos[2]) - path[0].pos[2]).0);
    let mut dir = [(F(dz) * 1.0 - 0.0).0, (0.0 - F(dx) * 1.0).0];
    let [qx, qz] = unit((F(pos[0]) - path[0].pos[0]).0, (F(pos[2]) - path[0].pos[2]).0);
    if 0.0 < (F(qz) * dir[1] + F(qx) * dir[0] + (F(dx) * 0.0 - F(dz) * 0.0) * 0.0).0 {
        dir = [(F(dir[0]) * -1.0).0, (F(dir[1]) * -1.0).0];
    }
    dir
}

/// A character's rally timing tables, as the original builds them at setup: per frame from the swing press the
/// grade (`SWEET_FRAME` the sweet one, `grade::SWEET`; `after` frames of grades clean, off, slow behind it and
/// `before` frames ahead of it; the first two frames never hit) and the bias the frame throws the shot off by
/// (tenths of a metre: an off frame one per frame, a slow one two per frame on top of the off run; negative before
/// the sweet frame). Both run to the first frame past the sweet one that has no grade.
pub fn timing(after: [i32; 3], before: [i32; 3]) -> (Vec<u8>, Vec<i32>) {
    // ponytail: the game's index cap (60) is never reached by the disc's counts
    const CAP: usize = 60;
    const SWEET: usize = SWEET_FRAME as usize;
    let (mut grades, mut bias) = ([grade::NONE; CAP], [0i32; CAP]);
    grades[SWEET] = grade::SWEET;
    for (sign, counts) in [(1i32, after), (-1, before)] {
        let mut k = SWEET as i32;
        for (g, n) in (grade::CLEAN..).zip(counts) {
            for i in 0..n {
                k += sign;
                if k < 0 {
                    continue;
                }
                grades[k as usize] = g;
                bias[k as usize] = sign * match g {
                    grade::OFF => i + 1,
                    grade::SLOW => counts[1] + (i + 1) * 2,
                    _ => 0,
                };
            }
        }
    }
    let len = (SWEET..CAP).find(|&k| grades[k] == grade::NONE).unwrap_or(CAP);
    grades[0] = grade::NONE;
    grades[1] = grade::NONE;
    (grades[..len].to_vec(), bias[..len].to_vec())
}

/// A dive's lock from the path index of its contact `frame`: (grade, offset), the bias being the offset. Its own
/// sweet frame is 10, not the timing tables': clean within a frame of it, else slow.
pub fn dive_lock(frame: usize) -> (u8, i32) {
    const DIVE_SWEET: i32 = 10;
    let offset = frame as i32 - DIVE_SWEET;
    (if offset.abs() < 2 { grade::CLEAN } else { grade::SLOW }, offset)
}

/// The launch's motion flags of a ground stroke or volley (`branch`) from the hitter's current `motion` (the
/// game's motion number): (odd, awkward). An odd motion (the other hand's side) mirrors the shot's bend; it, or
/// the body-shot swing (`anim::BODY` either side), adds to the mis-hit chance. Other branches have neither.
pub fn launch_motion(branch: u8, motion: i32) -> (bool, bool) {
    let rally = branch == branch::GROUND || branch == branch::VOLLEY;
    let odd = rally && motion & 1 != 0;
    let body = anim::BODY as i32;
    (odd, odd || rally && (motion == body || motion == body + 1))
}

/// A rally swing's timing error as the original sets it up at the lock (tenths of a metre): `side` and `depth`
/// off the aim, and `mode` the pressure toward the weaker (down) trajectory tables. Ground strokes, volleys and
/// dives (`branch`); `bias` is the timing table's for the contact frame and `offset` frames from the
/// sweet one, `height` the contact height. `right` is the ball on the right of the body (the path's side bit 0),
/// `forehand` on the hitter's forehand side, `body` a body-shot swing; a dive's error follows its direction
/// `dive` (x, z) against the player's end sign `end`.
#[allow(clippy::too_many_arguments)]
pub fn timing_error(
    s: &crate::player::ReachStats,
    branch: u8,
    lob: bool,
    bias: i32,
    offset: i32,
    height: f32,
    right: bool,
    forehand: bool,
    body: bool,
    dive: [f32; 2],
    end: f32,
) -> TimingError {
    let mut e = TimingError::default();
    if branch == branch::DIVE {
        let ahead = (F(end) * dive[1] + 0.0).0;
        if ahead >= FRAC_1_SQRT_2 {
            e.depth = bias.abs();
        } else if ahead <= -FRAC_1_SQRT_2 {
            e.depth = -bias.abs();
        } else if (F(end) * dive[0] + 0.0 - 0.0).0 > 0.0 {
            e.side = bias.abs();
        } else {
            e.side = -bias.abs();
        }
    } else {
        e.side = if right { -bias } else { bias };
    }
    let (ideal, scale, [from, step]) = match branch {
        branch::GROUND => (s.stroke_height, 1.0, s.stroke_miss),
        _ => (s.volley_height, 0.5, s.volley_miss),
    };
    e.depth += height_steps(height, ideal, [from, step], scale);
    let mut down = match branch {
        branch::GROUND if !forehand => s.back_down,
        branch::DIVE => 10,
        _ => 0,
    };
    if branch == branch::VOLLEY && offset.abs() > 1 {
        down += s.volley_down;
    }
    if body {
        down += s.body_down;
    }
    // the lob button keeps its tables
    e.mode = if lob { 0 } else { -down };
    e
}

/// The depth error (tenths) of a contact `height` off the `ideal`: whole 10 cm past `from` cm count one each, past
/// `step` cm two, × `scale`, signed by the side of the ideal.
fn height_steps(height: f32, ideal: f32, [from, step]: [i32; 2], scale: f32) -> i32 {
    let off = (F(height) - ideal).0;
    let cm = (F(off.abs()) * 100.0).0 as i32 / 10 * 10;
    let steps = match cm - from {
        n if n < 0 => 0,
        n if n < step => n / 10,
        n => step / 10 + (n - step) / 10 * 2,
    };
    (F(steps as f32) * scale).0 as i32 * if off > 0.0 { 1 } else { -1 }
}

/// A smash's timing scatter as the original sets it up at the swing and the launch: (side, depth) in metres
/// before `serve::scatter_along`'s 1.5. The depth error is the timing `bias` plus the contact `height` off the
/// ideal smash height (ground stroke thresholds), dropped on a clean `grade`; the side error is only the
/// aim's `nudge`; each held to ±1 m. A short error counts double, then the whole depth × the reach `scale`
/// (`smash_scale`) less the aim's `held` pull.
// ponytail: the smash's mode blend isn't taken; its launch reads the base smash table
pub fn smash_scatter(s: &crate::player::ReachStats, grade: u8, bias: i32, height: f32, nudge: i32, scale: f32, held: f32) -> (f32, f32) {
    let depth = if grade::clean(grade) { 0 } else { bias + height_steps(height, s.smash[1], s.stroke_miss, 1.0) }.clamp(-MAX_ERROR, MAX_ERROR);
    let side = nudge.clamp(-MAX_ERROR, MAX_ERROR);
    let mut sz = (F(depth as f32) / 10.0).0;
    if sz < 0.0 {
        sz = (F(sz) * 2.0).0;
    }
    ((F(side as f32) / 10.0 / 2.0).0, (F(sz) * scale - held).0)
}

/// A smash's depth scale: 1, or for a `plain` (not △) one off its timing (offset beyond ±1) 1 at 1.5 m from the
/// net growing to 2 at the baseline, by where the swing started (`from_z`).
pub fn smash_scale(plain: bool, offset: i32, from_z: f32) -> f32 {
    if !plain || offset.abs() < 2 {
        return 1.0;
    }
    ((F(from_z.abs().clamp(1.5, BASELINE)) - 1.5) / 10.385 + 1.0).0
}

/// See `timing_error`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimingError {
    pub side: i32,
    pub depth: i32,
    pub mode: i32,
}

/// What a rally shot's timing error does at the launch, as the original: (side, depth) scatter in metres before
/// the 1.5 scale, and the mode blend. A clean grade drops the error but keeps the aim's random `nudge`;
/// each part is held to ±1 m (depth to [−1, 0] when `short_only`, the aim's flag), the blend is scaled by how
/// much the character's power stat leaves for its low-power (or, for a raising ground stroke, high-power) stat
/// to move it. A ground stroke's short error counts double unless it is a drop shot.
pub fn timing_launch(s: &crate::player::ReachStats, branch: u8, kind: i32, grade: u8, e: TimingError, nudge: i32, short_only: bool) -> (f32, f32, f32) {
    let clean = grade::clean(grade);
    let side = ((if clean { 0 } else { e.side }) + nudge).clamp(-MAX_ERROR, MAX_ERROR);
    let depth = if clean { 0 } else { e.depth }.clamp(-MAX_ERROR, if short_only { 0 } else { MAX_ERROR });
    let mode = e.mode.clamp(-MAX_ERROR, MAX_ERROR);
    let (sx, mut sz, mut blend) = ((F(side as f32) / 10.0 / 2.0).0, (F(depth as f32) / 10.0).0, (F(mode as f32) / 10.0).0);
    let (base, mut add_) = match branch {
        branch::GROUND => (s.power[1], s.low_power[0]),
        branch::VOLLEY => (s.power[2], s.low_power[1]),
        _ => (s.power[2], s.low_power[0]),
    };
    if blend >= 0.0 {
        if branch == branch::GROUND {
            add_ = s.stroke_high_pow;
        }
        let top = (base + add_).min(STAT_MAX);
        blend = if base < STAT_MAX { (blend * (F((top - base) as f32) / ((STAT_MAX - base) as f32))).0 } else { 0.0 };
    } else {
        let low = (base - add_).max(0);
        blend = if base < 1 { 0.0 } else { (blend * (F((base - low) as f32) / (base as f32))).0 };
    }
    if branch == branch::GROUND && kind != kind::DROP && sz < 0.0 {
        sz = (F(sz) * 2.0).0;
    }
    (sx, sz, blend)
}

/// A ground stroke the original launches as a volley (class volley): topspin or slice, not framed, locked inside
/// the service line (`from_z`, the hitter's z at the lock) on a ball `height` 0.6 m up or more.
pub fn high_volley(branch: u8, kind: i32, framed: bool, from_z: f32, height: f32) -> bool {
    branch == branch::GROUND && !framed && kind <= kind::SLICE && from_z.abs() < SERVICE_LINE && height >= 0.6
}

/// The high-ball ground stroke's (volley-class launch) blend: how much the volley power leaves the high-power stat.
pub fn high_blend(s: &crate::player::ReachStats) -> f32 {
    let (base, top) = (s.power[2], (s.power[2] + s.stroke_high_pow).min(STAT_MAX));
    if base < STAT_MAX { (F((top - base) as f32) / ((STAT_MAX - base) as f32)).0 } else { 0.0 }
}

/// The trajectory table mode a launch blend picks, as the original: for a volley-class launch (volleys, dives
/// and the high-ball ground stroke) up (1) above 0.1, else down 1..3 by the character's thresholds (`lowest`,
/// character 9 its own three steps); for a stroke down 1 below −0.4 (character 4 never, character 9 down 2 below
/// −0.7), and a slow non-lob always at least down 1.
pub fn table_mode(class: u8, character: usize, grade: u8, kind: i32, blend: f32, lowest: &[f32; 14]) -> i32 {
    match class {
        class::VOLLEY if blend > 0.1 => 1,
        class::VOLLEY if character == 9 => [(-0.9, -3), (-0.7, -2), (-0.4, -1)].iter().find(|t| blend < t.0).map_or(0, |t| t.1),
        class::VOLLEY if blend < lowest[character] => -2,
        class::VOLLEY => -((blend < -0.3) as i32),
        class::STROKE => {
            let m = match character {
                4 => 0,
                9 => -2 * (blend < -0.7) as i32,
                _ => -((blend < -0.4) as i32),
            };
            if grade == grade::SLOW && kind != kind::LOB && m == 0 { -1 } else { m }
        }
        _ => 0,
    }
}

/// The trajectory table and shot-record variant a mode selects: up1, dw1, dw2, dw3 for modes 1, −1, −2, −3 (the
/// variant list's use flags 0..3), base otherwise.
pub fn mode_variant(mode: i32) -> Option<usize> {
    match mode {
        1 => Some(0),
        -1 => Some(1),
        -2 => Some(2),
        -3 => Some(3),
        _ => None,
    }
}

/// A lob's table variant, as the original: on a clean grade the character's Lob POW against its Lob2 POW
/// (each held to 0..16) picks up1 when lower, the variant list's flag 1 when higher, base when equal. Only a
/// character with that variant listed has the table.
// ponytail: the original's practice-mode exception (solo with player 1 hitting) is left out.
pub fn lob_variant(s: &crate::player::ReachStats, grade: u8) -> Option<usize> {
    let [a, b] = [s.power[3], s.power[4]].map(|p| p.clamp(0, STAT_MAX));
    match grade {
        _ if !grade::clean(grade) => None,
        _ if a < b => Some(0),
        _ if a > b => Some(1),
        _ => None,
    }
}

/// A slow (`grade::SLOW`) contact's mis-hit roll at the launch, as the original.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MisHit {
    /// The table elevation's scale.
    pub scale: f32,
    /// A dull hit (+0x3f0c): the elevation drops to a random 0.9/0.85/0.8 (contact at 0.2 or lower) or
    /// 0.95/0.9/0.85 (at 0.4 or lower), a volley's by only 1/2.2 of the drop.
    pub dull: bool,
    /// A framed hit (+0x3f06): a lob to a random spot (`wild_aim`).
    pub wild: bool,
}

/// The mis-hit roll of a slow ground stroke (flat, topspin or slice `kind`), volley or dive (`branch`) at
/// contact `height`: a 20% chance at 0.2 or lower, 35% at 0.4 or lower (a slice 15 points less), 20 more when
/// `tired` (stamina below 10; 10 for a slice) and 10 more on an `awkward` swing (the body-shot swing or the
/// other-hand side); a dive 20 more and nothing else. A mis-hit is framed one time in two (three in ten when
/// tired or diving), a slice's always dull. With no mis-hit a slow flat or topspin ground stroke still drops its
/// elevation to 0.95. `roll` draws from the game's RNG.
pub fn mis_hit(grade: u8, branch: u8, kind: i32, tired: bool, awkward: bool, height: f32, mut roll: impl FnMut() -> u32) -> MisHit {
    let mut m = MisHit { scale: 1.0, dull: false, wild: false };
    if grade != grade::SLOW {
        return m;
    }
    let mut draw = |n: u32| r15(roll()) % n;
    let dive = branch == branch::DIVE;
    if (branch == branch::GROUND && kind <= kind::FLAT) || branch == branch::VOLLEY || dive {
        let slice = kind == kind::SLICE && !dive;
        let (mut chance, mut framed) = if dive { (20, 20) } else if slice { (-15, 0) } else { (0, 0) };
        if !dive && tired {
            chance += if slice { 10 } else { 20 };
            framed = 20;
        }
        if !dive && awkward {
            chance += 10;
        }
        let low = if height <= 0.2 { Some((20, [0.9, 0.85, 0.8])) } else if height <= 0.4 { Some((35, [0.95, 0.9, 0.85])) } else { None };
        if let Some((base, scales)) = low
            && (draw(100) as i32) < chance + base
        {
            if slice || draw(100) >= framed + 50 {
                m.dull = true;
                m.scale = scales[draw(3) as usize];
            } else {
                m.wild = true;
            }
        }
    }
    if m.scale == 1.0 && !m.wild && branch == branch::GROUND && (kind == kind::TOPSPIN || kind == kind::FLAT) {
        m.scale = 0.95;
    }
    if m.scale < 1.0 && branch == branch::VOLLEY {
        m.scale = (1.0 - (F(1.0) - m.scale) / 2.2).0;
    }
    m
}

/// A framed hit's lob: (aim, side, depth error in metres before the 1.5 scale) for a hitter at `end` (+1 hitting
/// toward +z), as the original draws it from `roll` (the game's RNG). 15%: a sideline corner (singles or doubles
/// line) 3..6.4 m deep; 50%: anywhere across, 3..6.4 deep; else a third of the time the baseline anywhere across
/// with a depth error, or a sideline 3..11.885 deep with a side error, either side.
pub fn wild_aim(doubles: bool, end: f32, mut roll: impl FnMut() -> u32) -> ([f32; 3], f32, f32) {
    let u = |r: u32| (F(TWO_POW_M32) * utof(r)).0;
    let line = sideline(doubles);
    let across = |r: u32| (F::ZERO + -F(line)).madd(F(line) + line, u(r)).0;
    let off = |r: u32| ((F::ZERO + -0.66).madd(1.66, u(r)) / 1.5).0;
    let (mut side, mut depth) = (0.0, 0.0);
    let pick = r15(roll());
    let (x, z) = if pick % 100 < 15 {
        let x = if coin(roll()) { (-F(line)).0 } else { line };
        (x, (F::ZERO + 3.0).madd(3.4, u(roll())).0)
    } else if pick % 100 < 65 {
        let x = across(roll());
        (x, (F::ZERO + 3.0).madd(3.4, u(roll())).0)
    } else {
        let pick = r15(roll()) % 100;
        if pick < 33 {
            let x = across(roll());
            depth = off(roll());
            (x, BASELINE)
        } else {
            let (x, e) = if pick < 66 { (line, end) } else { ((-F(line)).0, (-F(end)).0) };
            let z = (F::ZERO + 3.0).madd(8.885, u(roll())).0;
            side = (F(off(roll())) * e).0;
            (x, z)
        }
    };
    ([x, 0.0, (F(z) * end).0], side, depth)
}
