//! The original's stroke contact search: when a shot button is pressed, the player scans the ball's predicted
//! path for the frame to hit it on, which branch (smash, volley, ground stroke) and which swing (forehand,
//! backhand, body shot). Ported operation for operation; the path is the predictor's (court plane only).

use crate::ps2::{add, mul, sqrt, sub};

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

/// Frames before contact over which the body slides by its swing step (the rest of the wind-up stands still).
pub fn step_frames(frame: usize) -> usize {
    frame.min(8)
}
