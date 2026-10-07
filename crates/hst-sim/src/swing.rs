//! The original's stroke contact search: when a shot button is pressed, the player scans the ball's predicted
//! path for the frame to hit it on, which branch (smash, volley, ground stroke) and which swing (forehand,
//! backhand, body shot). Ported operation for operation; the path is the predictor's (court plane only).

use crate::ps2::{add, div, mul, sqrt, sub};

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
    Some(Dive { frame: k, contact, dir, slide, tick: 0, start: [pos[0], pos[2]], prev: [0.0; 2], origin: k, stopped: false })
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
            }
        }
        Some(now)
    }
}
