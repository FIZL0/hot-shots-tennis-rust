//! The original's stroke contact search: when a shot button is pressed, the player scans the ball's predicted
//! path for the frame to hit it on, which branch (smash, volley, ground stroke) and which swing (forehand,
//! backhand, body shot). Ported operation for operation; the path is the predictor's (court plane only).

use crate::ps2::{add, div, madd, mul, sqrt, sub, utof};

/// One predicted ball frame: position in game space (Y down) and bounces so far.
#[derive(Clone, Copy, Debug)]
pub struct PathPoint {
    pub pos: [f32; 3],
    pub bounces: i32,
}

/// A player's reach for the search. Character data (TParam.csv) except `ahead`, `smash_ahead` and the body-shot
/// widths, which the game measures on the character's swing animations at their contact frame.
#[derive(Clone, Debug)]
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
    /// Dive arm in the player's frame (x right, y down, z forward): shoulder and racket tip.
    pub shoulder: [f32; 3],
    pub tip: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Branch {
    Ground,
    Volley,
    Smash,
}

/// The swing a press locked onto.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Swing animation: 0x10.. ground strokes by shot type, 0x16/0x18 high/low volley, 0x1a body shot (+1 for
    /// the other hand side); 0x1f smash.
    pub anim: u8,
}

/// Search `path` (index 0 = this frame) for the contact of a shot of type `kind` (0 topspin, 1 slice, 2 flat,
/// 3 lob, 4 drive) by a player at `pos` facing `facing` (+1 toward +z).
pub fn search(r: &Reach, path: &[PathPoint], pos: [f32; 3], facing: f32, kind: i32) -> Option<Swing> {
    let n = path.len().min(r.grades.len());
    // per frame: horizontal distance and depth from the stroke / smash contact points
    let horiz: Vec<f32> = path[..n]
        .iter()
        .map(|p| {
            let (dx, dz) = (sub(p.pos[0], pos[0]), sub(p.pos[2], pos[2]));
            sqrt(add(mul(dx, dx), mul(dz, dz)))
        })
        .collect();
    let depth = |ahead: f32, p: &PathPoint| sub(add(add(mul(ahead, facing), pos[2]), 0.0), p.pos[2]).abs();
    // ball on our half and at least 0.5 m from the net, with a nonzero timing grade
    let ours = |k: usize| {
        let z = path[k].pos[2];
        z.abs() >= 0.5 && (z >= 0.0) != (facing >= 0.0) && r.grades[k] != 0
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
    let height = |k: usize| -path[k].pos[1];
    let swing = |branch, k: usize, body: bool, base: u8| {
        let side = mul(sub(path[k].pos[0], pos[0]), facing);
        let forehand = side >= 0.0;
        let other = if forehand { r.hand < 0.0 } else { r.hand >= 0.0 };
        Swing { branch, frame: k, ball: path[k].pos, forehand, body, anim: base + other as u8 }
    };

    let smash = best(1, &|k| r.smash_bottom <= height(k) && height(k) <= r.smash_top && horiz[k] <= r.reach, &|k| depth(r.smash_ahead, &path[k]));
    if let Some(k) = smash {
        return Some(Swing { anim: 0x1f, ..swing(Branch::Smash, k, false, 0) });
    }
    let wide = mul(r.reach, 1.3);
    let volley = best(0, &|k| 0.0 <= height(k) && height(k) <= mul(r.volley_height, 1.3) && horiz[k] <= wide, &|k| depth(r.ahead, &path[k]));
    if let Some(k) = volley {
        let low = height(k) < r.stroke_height;
        let body = sub(path[k].pos[0], pos[0]).abs() <= if low { r.body_low } else { r.body_high };
        let base = if body { 0x1a } else if low { 0x18 } else { 0x16 };
        return Some(swing(Branch::Volley, k, body, base));
    }
    let ground = best(1, &|k| height(k) <= add(r.base, wide) && horiz[k] <= wide, &|k| depth(r.ahead, &path[k]));
    ground.map(|k| {
        let body = sub(path[k].pos[0], pos[0]).abs() <= r.body_low;
        let base = if body {
            0x1a
        } else {
            match kind {
                1 => 0x12,
                3 => 0x14,
                _ => 0x10,
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
#[derive(Clone, Copy, Debug, PartialEq)]
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
    let speed = mul(sqrt(add(mul(vel[0], vel[0]), mul(vel[1], vel[1]))), 1.25);
    let wide = mul(r.reach, 1.3);
    let (mut hit, mut miss): (Option<(usize, f32)>, Option<(usize, f32, f32)>) = (None, None);
    for (k, p) in path.iter().enumerate().take(DIVE_HORIZON).skip(5) {
        let (z, h) = (p.pos[2], -p.pos[1]);
        if !(z.abs() >= 0.5 && (z >= 0.0) != (end >= 0.0) && mul(r.stroke_height, 0.2) <= h && h <= mul(r.stroke_height, 1.8)) {
            continue;
        }
        let (dx, dz) = (sub(p.pos[0], pos[0]), sub(z, pos[2]));
        let dist = sqrt(add(mul(dx, dx), mul(dz, dz)));
        let inv = div(1.0, dist);
        let dot = add(add(mul(face[1], mul(dz, inv)), mul(face[0], mul(dx, inv))), 0.0);
        if dot < f32::from_bits(0x3f6c_835e) {
            continue; // more than 22.5° off the facing
        }
        let lunge = mul(speed, k as f32);
        let limit = add(lunge, wide);
        if !(limit < dist) && p.bounces <= 1 {
            if hit.is_none_or(|(_, best)| z.abs() > best) {
                hit = Some((k, z.abs()));
            }
        } else if dist <= mul(limit, 2.0) && miss.is_none_or(|(_, best, _)| dot > best) {
            miss = Some((k, dot, lunge));
        }
    }
    let (k, contact) = match (hit, miss) {
        (Some((k, _)), _) => (k, true),
        (None, Some((k, _, _))) => (k, false),
        _ => return None,
    };
    let (dx, dz) = (sub(path[k].pos[0], pos[0]), sub(path[k].pos[2], pos[2]));
    let inv = div(1.0, sqrt(add(add(mul(dz, dz), mul(dx, dx)), 0.0)));
    let dir = [mul(dx, inv), mul(dz, inv)];
    let slide = if contact { arm_slide(r, dir, sub3(path[k].pos, pos)) } else { miss.unwrap().2 };
    Some(Dive { frame: k, contact, dir, slide, tick: 0, start: [pos[0], pos[2]], prev: [0.0; 2], origin: k, stopped: false, cut: false })
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [sub(a[0], b[0]), sub(a[1], b[1]), sub(a[2], b[2])]
}

fn len3(v: [f32; 3]) -> f32 {
    sqrt(add(add(mul(v[0], v[0]), mul(v[1], v[1])), mul(v[2], v[2])))
}

/// How far a contact dive slides so the arm (shoulder to racket tip, turned to `dir`) reaches the ball at `d`
/// from the player: 0 when it already does.
fn arm_slide(r: &Reach, dir: [f32; 2], d: [f32; 3]) -> f32 {
    let world = |v: [f32; 3]| {
        let side = mul(v[0], r.hand);
        [add(mul(side, dir[1]), mul(v[2], dir[0])), v[1], sub(mul(v[2], dir[1]), mul(side, dir[0]))]
    };
    let (a, b) = (world(r.shoulder), world(r.tip));
    let arm = len3(sub3(b, a));
    if arm >= len3(sub3(d, a)) {
        return 0.0;
    }
    let t = -add(mul(dir[0], sub(a[0], d[0])), mul(dir[1], sub(a[2], d[2])));
    let px = sub(d[0], add(mul(dir[0], t), a[0]));
    let pz = sub(d[2], add(mul(dir[1], t), a[2]));
    let (ax, ay, az) = (sub(d[0], a[0]), sub(d[1], a[1]), sub(d[2], a[2]));
    let side = add(add(mul(ay, ay), mul(px, px)), mul(pz, pz));
    sub(sqrt(add(mul(ax, ax), mul(az, az))), sqrt(sub(mul(arm, arm), side)))
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
            let f = div(n as f32, k as f32);
            [mul(mul(self.dir[0], self.slide), f), mul(mul(self.dir[1], self.slide), f)]
        } else {
            // the motion is held at time 4 through the slide, so it is 5 when the root phase starts
            let z = root_z((n - self.origin + 5) as f32);
            [mul(self.dir[0], z), mul(self.dir[1], z)]
        };
        let delta = [sub(at[0], self.prev[0]), sub(at[1], self.prev[1])];
        let now = mover(delta);
        self.prev = if n == k { [0.0; 2] } else { at };
        if now != [add(pos[0], delta[0]), add(pos[1], delta[1])] {
            let (dx, dz) = (sub(now[0], self.start[0]), sub(now[1], self.start[1]));
            let inv = div(1.0, sqrt(add(mul(dx, dx), mul(dz, dz))));
            if add(add(mul(mul(dz, inv), self.dir[1]), mul(mul(dx, inv), self.dir[0])), 0.0) < f32::from_bits(0x3f68_03ca) {
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

/// A press whose search found nothing swings a smash at nothing (0x1f) when the ball is within 2 m across the
/// ground of `pos` and the path rises above `middle` (the smash window's middle height) before its 2nd bounce.
pub fn smash_whiff(path: &[PathPoint], ball: [f32; 3], pos: [f32; 3], middle: f32) -> bool {
    let (dx, dz) = (ball[0] - pos[0], ball[2] - pos[2]);
    dx * dx + dz * dz < 4.0 && path.iter().take_while(|p| p.bounces <= 1).any(|p| middle < -p.pos[1])
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
    let height = |k: usize| -path[k].pos[1];
    let wide = mul(r.reach, 1.3);
    let mut at = [pos[0], pos[2]];
    for i in 0..=20 {
        let end = (look + i).min(n);
        let (mut d3, mut d2) = (vec![0.0; n], vec![0.0; n]);
        for k in i..end {
            let (x, y, z) = (sub(path[k].pos[0], at[0]), sub(height(k), r.base), sub(path[k].pos[2], at[1]));
            d3[k] = sqrt(add(add(mul(z, z), mul(x, x)), mul(y, y)));
            d2[k] = sqrt(add(mul(x, x), mul(z, z)));
        }
        let first = |max_bounces: i32, ok: &dyn Fn(usize) -> bool| {
            (i..end).take_while(|&k| path[k].bounces <= max_bounces).find(|&k| {
                let z = path[k].pos[2];
                z.abs() >= 0.5 && (z >= 0.0) != (facing >= 0.0) && r.grades[k - i] != 0 && ok(k)
            })
        };
        let found = first(1, &|k| r.smash_bottom <= height(k) && height(k) <= r.smash_top && d2[k] <= r.reach)
            .or_else(|| first(0, &|k| 0.0 <= height(k) && height(k) <= mul(r.volley_height, 1.3) && d3[k] <= wide))
            .or_else(|| first(1, &|k| height(k) <= add(r.base, wide) && d2[k] <= wide));
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
        let inv = div(1.0, sqrt(add(mul(x, x), mul(z, z))));
        [mul(x, inv), mul(z, inv)]
    };
    let [dx, dz] = unit(sub(path[n - 1].pos[0], path[0].pos[0]), sub(path[n - 1].pos[2], path[0].pos[2]));
    let mut dir = [sub(mul(dz, 1.0), 0.0), sub(0.0, mul(dx, 1.0))];
    let [qx, qz] = unit(sub(pos[0], path[0].pos[0]), sub(pos[2], path[0].pos[2]));
    if 0.0 < add(add(mul(qz, dir[1]), mul(qx, dir[0])), mul(sub(mul(dx, 0.0), mul(dz, 0.0)), 0.0)) {
        dir = [mul(dir[0], -1.0), mul(dir[1], -1.0)];
    }
    dir
}

/// A character's rally timing tables, as the original builds them at setup: per frame from the swing press the
/// grade (8 the sweet frame, grade 1; `after` frames of grades 2, 3, 4 behind it and `before` frames ahead of
/// it; the first two frames never hit) and the bias the frame throws the shot off by (tenths of a metre: grade 3
/// one per frame, grade 4 two per frame on top of the grade-3 run; negative before the sweet frame). Both run to
/// the first frame past the sweet one that has no grade.
pub fn timing(after: [i32; 3], before: [i32; 3]) -> (Vec<u8>, Vec<i32>) {
    // ponytail: the game's index cap (60) is never reached by the disc's counts
    let (mut grades, mut bias) = ([0u8; 60], [0i32; 60]);
    grades[8] = 1;
    for (sign, counts) in [(1i32, after), (-1, before)] {
        let mut k = 8i32;
        for (g, n) in (2u8..).zip(counts) {
            for i in 0..n {
                k += sign;
                if k < 0 {
                    continue;
                }
                grades[k as usize] = g;
                bias[k as usize] = sign * match g {
                    3 => i + 1,
                    4 => counts[1] + (i + 1) * 2,
                    _ => 0,
                };
            }
        }
    }
    let len = (8..60).find(|&k| grades[k] == 0).unwrap_or(60);
    grades[0] = 0;
    grades[1] = 0;
    (grades[..len].to_vec(), bias[..len].to_vec())
}

/// A dive's lock from the path index of its contact `frame`: (grade, offset), the bias being the offset. Its own
/// sweet frame is 10, not the timing tables': grade 2 within a frame of it, else 4.
pub fn dive_lock(frame: usize) -> (u8, i32) {
    let offset = frame as i32 - 10;
    (if offset.abs() < 2 { 2 } else { 4 }, offset)
}

/// The launch's motion flags of a ground stroke (`branch` 1) or volley (2) from the hitter's current `motion` (the
/// game's motion number): (odd, awkward). An odd motion (the other hand's side) mirrors the shot's bend; it, or
/// the body-shot swing (0x1a/0x1b), adds to the mis-hit chance. Other branches have neither.
pub fn launch_motion(branch: u8, motion: i32) -> (bool, bool) {
    let rally = branch == 1 || branch == 2;
    let odd = rally && motion & 1 != 0;
    (odd, odd || rally && (motion == 0x1a || motion == 0x1b))
}

/// A rally swing's timing error as the original sets it up at the lock (tenths of a metre): `side` and `depth`
/// off the aim, and `mode` the pressure toward the weaker (down) trajectory tables. Ground strokes (branch 1),
/// volleys (2) and dives (3); `bias` is the timing table's for the contact frame and `offset` frames from the
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
    if branch == 3 {
        let ahead = add(mul(end, dive[1]), 0.0);
        if ahead >= 0.70710677 {
            e.depth = bias.abs();
        } else if ahead <= -0.70710677 {
            e.depth = -bias.abs();
        } else if sub(add(mul(end, dive[0]), 0.0), 0.0) > 0.0 {
            e.side = bias.abs();
        } else {
            e.side = -bias.abs();
        }
    } else {
        e.side = if right { -bias } else { bias };
    }
    let (ideal, scale, [from, step]) = match branch {
        1 => (s.stroke_height, 1.0, s.stroke_miss),
        _ => (s.volley_height, 0.5, s.volley_miss),
    };
    e.depth += height_steps(height, ideal, [from, step], scale);
    let mut down = match branch {
        1 if !forehand => s.back_down,
        3 => 10,
        _ => 0,
    };
    if branch == 2 && offset.abs() > 1 {
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
    let off = sub(height, ideal);
    let cm = mul(off.abs(), 100.0) as i32 / 10 * 10;
    let steps = match cm - from {
        n if n < 0 => 0,
        n if n < step => n / 10,
        n => step / 10 + (n - step) / 10 * 2,
    };
    mul(steps as f32, scale) as i32 * if off > 0.0 { 1 } else { -1 }
}

/// A smash's timing scatter as the original sets it up at the swing and the launch: (side, depth) in metres
/// before `serve::scatter_along`'s 1.5. The depth error is the timing `bias` plus the contact `height` off the
/// ideal smash height (ground stroke thresholds), dropped on a clean `grade` (1, 2); the side error is only the
/// aim's `nudge`; each held to ±1 m. A short error counts double, then the whole depth × the reach `scale`
/// (`smash_scale`) less the aim's `held` pull.
// ponytail: the smash's mode blend isn't taken; its launch reads the base smash table
pub fn smash_scatter(s: &crate::player::ReachStats, grade: u8, bias: i32, height: f32, nudge: i32, scale: f32, held: f32) -> (f32, f32) {
    let clean = grade == 1 || grade == 2;
    let depth = if clean { 0 } else { bias + height_steps(height, s.smash[1], s.stroke_miss, 1.0) }.clamp(-10, 10);
    let side = nudge.clamp(-10, 10);
    let mut sz = div(depth as f32, 10.0);
    if sz < 0.0 {
        sz = mul(sz, 2.0);
    }
    (div(div(side as f32, 10.0), 2.0), sub(mul(sz, scale), held))
}

/// A smash's depth scale: 1, or for a `plain` (not △) one off its timing (offset beyond ±1) 1 at 1.5 m from the
/// net growing to 2 at the baseline, by where the swing started (`from_z`).
pub fn smash_scale(plain: bool, offset: i32, from_z: f32) -> f32 {
    if !plain || offset.abs() < 2 {
        return 1.0;
    }
    add(div(sub(from_z.abs().clamp(1.5, 11.885), 1.5), 10.385), 1.0)
}

/// See `timing_error`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimingError {
    pub side: i32,
    pub depth: i32,
    pub mode: i32,
}

/// What a rally shot's timing error does at the launch, as the original: (side, depth) scatter in metres before
/// the 1.5 scale, and the mode blend. A clean grade (1, 2) drops the error but keeps the aim's random `nudge`;
/// each part is held to ±1 m (depth to [−1, 0] when `short_only`, the aim's flag), the blend is scaled by how
/// much the character's power stat leaves for its low-power (or, for a raising ground stroke, high-power) stat
/// to move it. A ground stroke's short error counts double unless it is kind 4.
pub fn timing_launch(s: &crate::player::ReachStats, branch: u8, kind: i32, grade: u8, e: TimingError, nudge: i32, short_only: bool) -> (f32, f32, f32) {
    let clean = grade == 1 || grade == 2;
    let side = ((if clean { 0 } else { e.side }) + nudge).clamp(-10, 10);
    let depth = if clean { 0 } else { e.depth }.clamp(-10, if short_only { 0 } else { 10 });
    let mode = e.mode.clamp(-10, 10);
    let (sx, mut sz, mut blend) = (div(div(side as f32, 10.0), 2.0), div(depth as f32, 10.0), div(mode as f32, 10.0));
    let (base, mut add_) = match branch {
        1 => (s.power[1], s.low_power[0]),
        2 => (s.power[2], s.low_power[1]),
        _ => (s.power[2], s.low_power[0]),
    };
    if blend >= 0.0 {
        if branch == 1 {
            add_ = s.stroke_high_pow;
        }
        let top = (base + add_).min(16);
        blend = if base < 16 { mul(blend, div((top - base) as f32, (16 - base) as f32)) } else { 0.0 };
    } else {
        let low = (base - add_).max(0);
        blend = if base < 1 { 0.0 } else { mul(blend, div((base - low) as f32, base as f32)) };
    }
    if branch == 1 && kind != 4 && sz < 0.0 {
        sz = mul(sz, 2.0);
    }
    (sx, sz, blend)
}

/// The high-ball ground stroke's (class-2 launch) blend: how much the volley power leaves the high-power stat.
pub fn high_blend(s: &crate::player::ReachStats) -> f32 {
    let (base, top) = (s.power[2], (s.power[2] + s.stroke_high_pow).min(16));
    if base < 16 { div((top - base) as f32, (16 - base) as f32) } else { 0.0 }
}

/// The trajectory table mode a launch blend picks, as the original: for a shot class 2 launch (volleys, dives
/// and the high-ball ground stroke) up (1) above 0.1, else down 1..3 by the character's thresholds (`lowest`,
/// character 9 its own three steps); for class 1 down 1 below −0.4 (character 4 never, character 9 down 2 below
/// −0.7), and a grade-4 non-lob always at least down 1.
pub fn table_mode(class: u8, character: usize, grade: u8, kind: i32, blend: f32, lowest: &[f32; 14]) -> i32 {
    match class {
        2 if blend > 0.1 => 1,
        2 if character == 9 => [(-0.9, -3), (-0.7, -2), (-0.4, -1)].iter().find(|t| blend < t.0).map_or(0, |t| t.1),
        2 if blend < lowest[character] => -2,
        2 => -((blend < -0.3) as i32),
        1 => {
            let m = match character {
                4 => 0,
                9 => -2 * (blend < -0.7) as i32,
                _ => -((blend < -0.4) as i32),
            };
            if grade == 4 && kind != 3 && m == 0 { -1 } else { m }
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

/// A lob's table variant, as the original: on a clean grade (1, 2) the character's Lob POW against its Lob2 POW
/// (each held to 0..16) picks up1 when lower, the variant list's flag 1 when higher, base when equal. Only a
/// character with that variant listed has the table.
// ponytail: the original's practice-mode exception (solo with player 1 hitting) is left out.
pub fn lob_variant(s: &crate::player::ReachStats, grade: u8) -> Option<usize> {
    let [a, b] = [s.power[3], s.power[4]].map(|p| p.clamp(0, 16));
    match grade {
        1 | 2 if a < b => Some(0),
        1 | 2 if a > b => Some(1),
        _ => None,
    }
}

/// A grade-4 (SLOW) contact's mis-hit roll at the launch, as the original.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MisHit {
    /// The table elevation's scale.
    pub scale: f32,
    /// A dull hit (+0x3f0c): the elevation drops to a random 0.9/0.85/0.8 (contact at 0.2 or lower) or
    /// 0.95/0.9/0.85 (at 0.4 or lower), a volley's by only 1/2.2 of the drop.
    pub dull: bool,
    /// A framed hit (+0x3f06): a lob to a random spot (`wild_aim`).
    pub wild: bool,
}

/// The mis-hit roll of a `grade`-4 ground stroke (`branch` 1, flat/topspin/slice `kind`), volley (2) or dive (3)
/// at contact `height`: a 20% chance at 0.2 or lower, 35% at 0.4 or lower (a slice 15 points less), 20 more when
/// `tired` (stamina below 10; 10 for a slice) and 10 more on an `awkward` swing (the body-shot swing or the
/// other-hand side); a dive 20 more and nothing else. A mis-hit is framed one time in two (three in ten when
/// tired or diving), a slice's always dull. With no mis-hit a slow flat or topspin ground stroke still drops its
/// elevation to 0.95. `roll` draws from the game's RNG.
pub fn mis_hit(grade: u8, branch: u8, kind: i32, tired: bool, awkward: bool, height: f32, mut roll: impl FnMut() -> u32) -> MisHit {
    let mut m = MisHit { scale: 1.0, dull: false, wild: false };
    if grade != 4 {
        return m;
    }
    let mut draw = |n: u32| (roll() >> 16 & 0x7fff) % n;
    if (branch == 1 && kind <= 2) || branch == 2 || branch == 3 {
        let slice = kind == 1 && branch != 3;
        let (mut chance, mut framed) = if branch == 3 { (20, 20) } else if slice { (-15, 0) } else { (0, 0) };
        if branch != 3 && tired {
            chance += if slice { 10 } else { 20 };
            framed = 20;
        }
        if branch != 3 && awkward {
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
    if m.scale == 1.0 && !m.wild && branch == 1 && (kind == 0 || kind == 2) {
        m.scale = 0.95;
    }
    if m.scale < 1.0 && branch == 2 {
        m.scale = sub(1.0, div(sub(1.0, m.scale), 2.2));
    }
    m
}

/// A framed hit's lob: (aim, side, depth error in metres before the 1.5 scale) for a hitter at `end` (+1 hitting
/// toward +z), as the original draws it from `roll` (the game's RNG). 15%: a sideline corner (singles or doubles
/// line) 3..6.4 m deep; 50%: anywhere across, 3..6.4 deep; else a third of the time the baseline anywhere across
/// with a depth error, or a sideline 3..11.885 deep with a side error, either side.
pub fn wild_aim(doubles: bool, end: f32, mut roll: impl FnMut() -> u32) -> ([f32; 3], f32, f32) {
    let u = |r: u32| mul(2.3283064e-10, utof(r));
    let line = if doubles { 5.485 } else { 4.115 };
    let across = |r: u32| madd(add(0.0, -line), add(line, line), u(r));
    let off = |r: u32| div(madd(add(0.0, -0.66), 1.66, u(r)), 1.5);
    let (mut side, mut depth) = (0.0, 0.0);
    let pick = roll() >> 16 & 0x7fff;
    let (x, z) = if pick % 100 < 15 {
        let x = if roll() >> 16 & 1 != 0 { -line } else { line };
        (x, madd(add(0.0, 3.0), 3.4, u(roll())))
    } else if pick % 100 < 65 {
        let x = across(roll());
        (x, madd(add(0.0, 3.0), 3.4, u(roll())))
    } else {
        let pick = (roll() >> 16 & 0x7fff) % 100;
        if pick < 33 {
            let x = across(roll());
            depth = off(roll());
            (x, 11.885)
        } else {
            let (x, e) = if pick < 66 { (line, end) } else { (-line, -end) };
            let z = madd(add(0.0, 3.0), 8.885, u(roll()));
            side = mul(off(roll()), e);
            (x, z)
        }
    };
    ([x, 0.0, mul(z, end)], side, depth)
}
