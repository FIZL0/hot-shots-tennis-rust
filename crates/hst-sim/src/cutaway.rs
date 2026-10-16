//! The match's scripted camera shots (the cut-away after a point): a shot number picks a record and a script
//! from the game program (`hst_data::exe::Game::camera_shots`). The script sets keyed channels — pitch, yaw,
//! framing angles, distance, field of view — that run from frame to frame with the game's easing; each frame the
//! view orbits a subject frame (a player's ground spot facing the way they face), sits at the shot's distance
//! with the framing offset, then widens and tilts so the subject's head stays in the picture, and keeps clear of
//! the ground. Game space (Y down); views are rows right, down, forward and an eye, as [`crate::camera::View`].
//! Arithmetic through `ps2::F` like the match camera: the view never feeds back into play.

#![deny(clippy::float_arithmetic)]

use crate::camera::{ASPECT, View};
use crate::ps2::{F, fs};
pub use hst_data::exe::{CameraShot, ReplayCameras};

pub type M4 = [[f32; 4]; 4];

/// A shot's keyed channels ([`Shot::ch`]); the ones below `FIRST` are unused.
pub mod ch {
    pub const FIRST: usize = 5;
    /// The orbit about the subject frame: pitch, yaw (radians), and the picture's roll.
    pub const PITCH: usize = 5;
    pub const YAW: usize = 6;
    pub const ROLL: usize = 7;
    /// The subject's offset in its own axes (x, y, z).
    pub const OFFSET: [usize; 3] = [8, 9, 10];
    /// Framing: the angles the subject sits off-centre by.
    pub const FRAME_PITCH: usize = 11;
    pub const FRAME_YAW: usize = 12;
    pub const DISTANCE: usize = 13;
    /// The horizontal half-angle.
    pub const FOV: usize = 14;
    /// The angle a second subject turns the view to (0: no turn).
    pub const TURN: usize = 15;
    pub const COUNT: usize = 17;
}

/// A shot script's ops: 0..=4 set a channel through the field of view, `PITCH..=0x10` the channel of the same
/// number, then the timing ops.
mod op {
    use super::ch;
    /// The orbit's pitch and yaw as shares of the vertical / horizontal half-angle.
    pub const PITCH_SHARE: u8 = 0;
    pub const YAW_SHARE: u8 = 1;
    /// The framing pitch, yaw and the turn as the tangent's share of the half-angle's (the pitch negated).
    pub const FRAME_PITCH_SHARE: u8 = 2;
    pub const FRAME_YAW_SHARE: u8 = 3;
    pub const TURN_SHARE: u8 = 4;
    pub const PITCH: u8 = ch::PITCH as u8;
    pub const YAW: u8 = ch::YAW as u8;
    pub const ROLL: u8 = ch::ROLL as u8;
    pub const OFFSET_X: u8 = ch::OFFSET[0] as u8;
    pub const OFFSET_Y: u8 = ch::OFFSET[1] as u8;
    pub const OFFSET_Z: u8 = ch::OFFSET[2] as u8;
    pub const FRAME_PITCH: u8 = ch::FRAME_PITCH as u8;
    pub const FRAME_YAW: u8 = ch::FRAME_YAW as u8;
    pub const DISTANCE: u8 = ch::DISTANCE as u8;
    pub const FOV: u8 = ch::FOV as u8;
    pub const TURN: u8 = ch::TURN as u8;
    /// `TIME..=LAST_TIME`: the next keys' time, of key kind op − `TIME`.
    pub const TIME: u8 = 0x11;
    pub const LAST_TIME: u8 = 0x16;
    pub const EASE: u8 = 0x17;
    pub const EASE_OUT: u8 = 0x18;
    /// Ignored.
    pub const SKIP: u8 = 0x19;
    /// The script's end (the compiler appends it).
    pub const END: u8 = 0x1a;
}

/// Keys a channel holds at most.
const MAX_KEYS: usize = 0x20;
/// The script's angles in degrees, and the field of view's full angle to the half-angle in radians.
const DEG: f32 = 0.017453292;
const HALF_DEG: f32 = 0.008726646;
/// The key kind that marks a channel's end.
const END: u8 = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct Key {
    value: f32,
    /// 0: `time` frames; other kinds are event-relative times (not used by the cut-aways); `END` ends the channel.
    kind: u8,
    time: f32,
    ease: f32,
    ease_out: f32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct Channel {
    keys: Vec<Key>,
    /// The key being moved toward, its duration and frames left, and the value now.
    to: usize,
    frames: f32,
    left: i32,
    ease: f32,
    ease_out: f32,
    value: f32,
}

/// Script → channel keys, as the game's compiler: values convert on read (fov ×π/360, angles ×π/180, the
/// framing ops 0–4 scale through the current field of view), the fov of a segment is applied first, and a time
/// op (`op::TIME..=op::LAST_TIME`) starts the next keys of every channel set after it.
fn compile(script: &[(u8, f32)], mirror: bool) -> Vec<Vec<Key>> {
    use op::*;
    // pass 1: conversions and segments, the fov moved to the start of its segment
    let mut buf: Vec<(u8, f32)> = Vec::new();
    let (mut seg, mut timing, mut fov) = (0usize, false, 0.0f32);
    for &(op, v) in script.iter().filter(|e| e.0 != SKIP).chain(std::iter::once(&(END, 0.0))) {
        if !(TIME..=EASE_OUT).contains(&op) {
            if timing {
                timing = false;
                seg = buf.len();
            }
        } else if op != EASE && op != EASE_OUT {
            timing = true;
        }
        if (timing || op == END) && fov != 0.0 {
            buf.insert(seg, (FOV, fov));
            fov = 0.0;
        }
        if op == END {
            break;
        }
        let mut v = match op {
            FOV => (F(v) * HALF_DEG).0,
            TURN | FRAME_YAW | FRAME_PITCH | ROLL | YAW | PITCH => (F(v) * DEG).0,
            _ => v,
        };
        if op == FRAME_PITCH || op == FRAME_PITCH_SHARE {
            v = (-F(v)).0;
        }
        // mirrored: everything horizontal turns the other way
        if mirror && matches!(op, OFFSET_X | TURN | TURN_SHARE | FRAME_YAW | FRAME_YAW_SHARE | ROLL | YAW | YAW_SHARE) {
            v = (-F(v)).0;
        }
        if op == FOV {
            fov = v;
        } else {
            buf.push((op, v));
        }
    }
    // pass 2: keys per channel (each ends where its count stops; the game marks it with an end key)
    let mut keys: Vec<Vec<Key>> = vec![Vec::new(); ch::COUNT];
    let mut count = [0usize; ch::COUNT];
    let (mut kind, mut time, mut ease, mut ease_out, mut timed) = (0u8, 0.0f32, 1.0f32, 0.0f32, 0);
    // the compiler's starting field of view: 40 (radians, as the game has it)
    let (mut hfov, mut vfov) = (40.0f32, musl::atanf((F(musl::tanf(40.0)) * ASPECT).0));
    let mut first = true;
    for (op, v) in buf {
        match op {
            TIME..=LAST_TIME => {
                kind = op - TIME;
                time = v;
                timed += 1;
                continue;
            }
            EASE => {
                ease = v;
                continue;
            }
            EASE_OUT => {
                ease_out = v;
                continue;
            }
            _ => {}
        }
        if first {
            first = false;
            for (c, k) in keys.iter_mut().enumerate().skip(ch::FIRST) {
                *k = vec![Key { value: if c == ch::FOV { hfov } else { 0.0 }, kind, time, ease, ease_out }];
            }
        }
        let (c, value) = match op {
            FOV => {
                hfov = v;
                vfov = musl::atanf((F(musl::tanf(v)) * ASPECT).0);
                (ch::FOV, v)
            }
            TURN_SHARE => (ch::TURN, musl::atanf((F(v) * musl::tanf(hfov)).0)),
            FRAME_YAW_SHARE => (ch::FRAME_YAW, musl::atanf((F(v) * musl::tanf(hfov)).0)),
            FRAME_PITCH_SHARE => (ch::FRAME_PITCH, musl::atanf((F(v) * musl::tanf(vfov)).0)),
            YAW_SHARE => (ch::YAW, (F(v) * hfov).0),
            PITCH_SHARE => (ch::PITCH, (F(v) * vfov).0),
            op => (op as usize, v),
        };
        if !(ch::FIRST..ch::COUNT).contains(&c) || count[c] >= MAX_KEYS {
            continue;
        }
        let k = &mut keys[c];
        // a channel first set inside a timed segment holds (this value before a second time op, else 0) first
        if count[c] == 0 && (kind != 0 || time != 0.0) {
            k[0].value = if timed == 1 { value } else { 0.0 };
            count[c] += 1;
        }
        k.truncate(count[c]);
        k.push(Key { value, kind, time, ease, ease_out });
        count[c] += 1;
    }
    for (c, k) in keys.iter_mut().enumerate().skip(ch::FIRST) {
        k.truncate(count[c].max(1));
    }
    keys
}

impl Channel {
    fn new(keys: Vec<Key>) -> Self {
        let mut c = Channel { keys, ..Channel::default() };
        c.advance(0);
        c
    }

    /// Start toward key `from + 1` (or hold key `from` when it is the last).
    fn advance(&mut self, from: usize) {
        let to = from + 1;
        self.to = to;
        match self.keys.get(to).copied() {
            Some(k) if k.kind != END => {
                self.frames = k.time.trunc();
                self.left = k.time as i32;
                self.ease = k.ease;
                self.ease_out = k.ease_out;
            }
            _ => {
                self.frames = 0.0;
                self.left = 0;
                self.ease = 1.0;
                self.ease_out = 0.0;
            }
        }
        if self.frames == 0.0 {
            self.value = self.keys.get(from).map_or(0.0, |k| k.value);
        } else {
            self.lerp();
        }
    }

    fn lerp(&mut self) {
        let mut t = (1.0 - F(self.left as f32) / self.frames).0;
        let e = if self.ease != 0.0 { self.ease } else { 1.0 };
        if 0.0 < e && 0.0 <= self.ease_out {
            t = musl::powf(musl::powf(t, musl::powf((1.0 - F(t)).0, self.ease_out)), e);
        }
        let (a, b) = (self.keys[self.to - 1].value, self.keys[self.to].value);
        self.value = (t * (F(b) - a) + a).0;
    }

    fn step(&mut self) {
        if self.keys.get(self.to).is_none_or(|k| k.kind == END) {
            return;
        }
        if self.left < 1 {
            self.advance(self.to);
        } else {
            self.left -= 1;
            self.lerp();
        }
    }
}

/// Row-vector matrix product `b · a` (the game's `dst = b × a`).
pub(crate) fn mul(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|r| {
        let b = fs(b[r]);
        std::array::from_fn(|c| (b[0] * a[0][c] + b[1] * a[1][c] + b[2] * a[2][c] + b[3] * a[3][c]).0)
    })
}
pub(crate) fn rot_y(t: f32) -> M4 {
    let (s, c) = musl::sincosf(t);
    [[c, 0.0, (-F(s)).0, 0.0], [0.0, 1.0, 0.0, 0.0], [s, 0.0, c, 0.0], [0.0, 0.0, 0.0, 1.0]]
}
pub(crate) fn rot_x(t: f32) -> M4 {
    let (s, c) = musl::sincosf(t);
    [[1.0, 0.0, 0.0, 0.0], [0.0, c, s, 0.0], [0.0, (-F(s)).0, c, 0.0], [0.0, 0.0, 0.0, 1.0]]
}
pub const IDENTITY: M4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

/// A point in the camera's frame (x right, y down, z forward).
fn local(cam: &M4, p: [f32; 3]) -> [f32; 3] {
    let d: [F; 3] = std::array::from_fn(|i| F(p[i]) - cam[3][i]);
    std::array::from_fn(|i| (d[0] * cam[i][0] + d[1] * cam[i][1] + d[2] * cam[i][2]).0)
}

fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let (a, b) = (fs(a), fs(b));
    [(a[1] * b[2] - a[2] * b[1]).0, (a[2] * b[0] - a[0] * b[2]).0, (a[0] * b[1] - a[1] * b[0]).0, 0.0]
}

fn normalize(v: [f32; 4]) -> [f32; 4] {
    let v = fs(v);
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [(v[0] / l).0, (v[1] / l).0, (v[2] / l).0, 0.0]
}

/// Vertical angle of `v` above the horizon (Y down).
pub(crate) fn pitch(v: [f32; 4]) -> f32 {
    let v = fs(v);
    musl::atan2f((-v[1]).0, (v[0] * v[0] + v[2] * v[2]).sqrt().0)
}

/// Wrapped difference a − b.
pub(crate) fn diff(a: f32, b: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let d = (F(a) + PI) - (F(b) + PI);
    let (p, m) = (d + TAU, d - TAU);
    if (p.abs() <= d.abs() || m.abs() <= d.abs()) && m.abs() <= p.abs() {
        m.0
    } else if p.abs() <= d.abs() || m.abs() <= d.abs() {
        p.0
    } else {
        d.0
    }
}

/// A shot record's fields (byte offsets): frame numbers (0: none), flags (non-zero: on) and floats.
mod rec {
    /// The subject frame, the frame it looks at, the frame whose spot it moves to, and a second subject the view
    /// turns to (also looked at).
    pub const SUBJECT: usize = 0;
    pub const LOOK_AT: usize = 1;
    pub const MOVE_TO: usize = 2;
    pub const SECOND: usize = 3;
    /// Flags: the subject turned round; kept upright.
    pub const TURN_ROUND: usize = 4;
    pub const UPRIGHT: usize = 5;
    /// The frame the view turns toward ([`super::frame::FOLLOW`] is the shown player's spot).
    pub const TARGET: usize = 0x48;
    /// Flags: follow the target's pitch (only upward with `PITCH_UP_ONLY`), its yaw (holding back what the pull
    /// can't take with `HOLD_YAW`).
    pub const FOLLOW_PITCH: usize = 0x4a;
    pub const PITCH_UP_ONLY: usize = 0x4b;
    pub const FOLLOW_YAW: usize = 0x4c;
    pub const HOLD_YAW: usize = 0x4d;
    /// Floats: the pitch framing's offset, dead zone, pull limit and ease; the yaw framing's from `YAW_FRAMING`.
    pub const PITCH_FRAMING: [usize; 4] = [0x50, 0x54, 0x58, 0x5c];
    pub const YAW_FRAMING: [usize; 4] = [0x60, 0x64, 0x68, 0x6c];
    /// Flag: the yaw error clamped to the pull limit.
    pub const CLAMP_YAW: usize = 0x70;
    /// Frames of the heads kept in the picture, and the float share of the half-height they keep within.
    pub const HEADS: [usize; 2] = [0x84, 0x85];
    pub const HEAD_ROOM: usize = 0x88;
    /// Frames kept above / below the band, and the float band (share of the vertical half-angle).
    pub const BAND_ABOVE: usize = 0x8c;
    pub const BAND_BELOW: usize = 0x8d;
    pub const BAND: usize = 0x90;
    /// Float: the eye's clearance above the ground.
    pub const CLEARANCE: usize = 0xac;
}

/// The reference frames the shots name (the game's table; [`Frames::get`]).
pub mod frame {
    /// The table's first frame, a direction (w = 0); the court centre (the intro's subject).
    pub const ORIGIN: u8 = 0;
    pub const COURT_CENTRE: u8 = 1;
    /// The court corner view.
    pub const CORNER: u8 = 6;
    /// A record's target that means the shown player's ground spot (`SPOT`).
    pub const FOLLOW: u8 = 9;
    pub const SPOT: u8 = 19;
    /// The shown player's frames ([`super::subject_frames`]): ground, above the head, the head, the chest; the
    /// other player's are each one up.
    pub const SHOWN: [u8; 4] = [29, 45, 61, 77];
    /// The shown player's live head.
    pub const HEAD: u8 = 35;
}

/// Post-point shot numbers [`pick`] treats specially.
pub mod post {
    /// The winner's shot, and the loser's that replaces it on a game-ending point.
    pub const WINNER: u8 = 0x61;
    pub const LOSER: u8 = 0x65;
    /// The close-up some characters lack.
    pub const CLOSE_UP: u8 = 0x64;
}

/// A running shot.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Shot {
    pub number: u8,
    record: Vec<u8>,
    channels: Vec<Channel>,
    /// The view this frame: rows right, down, forward and the eye; horizontal half-angle.
    pub cam: M4,
    pub fov: f32,
    /// Framing: the yaw the view has been held back by so far, and this frame's share of it.
    yaw_held: f32,
    yaw_over: f32,
}

impl Shot {
    /// Start shot `number` (`mirror`: the game alternates the side of the orbit on every post-point shot).
    pub fn new(shots: &[CameraShot], number: u8, mirror: bool) -> Self {
        let s = &shots[number as usize];
        let channels = compile(&s.script, mirror).into_iter().map(Channel::new).collect();
        Shot { number, record: s.record.clone(), channels, cam: IDENTITY, fov: 0.0, yaw_held: 0.0, yaw_over: 0.0 }
    }

    fn byte(&self, o: usize) -> u8 {
        self.record[o]
    }
    fn float(&self, o: usize) -> f32 {
        f32::from_le_bytes(self.record[o..o + 4].try_into().unwrap())
    }
    /// Channel `c`'s value now.
    pub fn ch(&self, c: usize) -> f32 {
        self.channels[c].value
    }

    /// The frame index of the shot's subject (`frames` resolves it: the game's table of court spots and
    /// players by role).
    pub fn subject(&self) -> u8 {
        self.byte(rec::SUBJECT)
    }

    /// Advance one frame and build the view from the subject frames (`frame(i)`: the game's reference frame i).
    pub fn step(&mut self, frame: impl Fn(u8) -> M4) {
        for c in &mut self.channels[ch::FIRST..] {
            c.step();
        }
        if self.byte(rec::FOLLOW_YAW) != 0 && self.byte(rec::HOLD_YAW) != 0 {
            self.yaw_held = (F(self.yaw_held) + self.yaw_over).0;
        }
        self.build(&frame);
    }

    /// Build the view for the current channel values (`step` without advancing).
    pub fn build(&mut self, frame: &impl Fn(u8) -> M4) {
        let basis = self.basis(frame);
        // orbit: the subject's frame turned by yaw then pitch, rebuilt from its forward without roll
        let cam = mul(&basis, &rot_y(self.ch(ch::YAW)));
        let cam = mul(&cam, &rot_x(self.ch(ch::PITCH)));
        let f = cam[2];
        let (p, y) = (pitch(f), musl::atan2f(f[0], f[2]));
        let mut cam = mul(&rot_y(y), &rot_x(p));
        // framing: the eye moves so the subject sits off-centre by the framing angles
        let a11 = musl::atanf((F(musl::tanf(self.ch(ch::FRAME_PITCH))) * musl::cosf(self.ch(ch::FRAME_YAW))).0);
        let o = mul(&mul(&cam, &rot_y(self.ch(ch::FRAME_YAW))), &rot_x(a11));
        let d = self.ch(ch::DISTANCE);
        let eye = |i: usize| (F(basis[3][i]) - F(o[2][i]) * d).0;
        cam[3] = [eye(0), eye(1), eye(2), 1.0];
        // a second subject (a frame with w ≠ 0): the view turns to face it across the court plane
        let second = frame(self.byte(rec::SECOND))[3];
        if second[3] != 0.0 {
            use std::f32::consts::{PI, TAU};
            let a0 = local(&cam, [basis[3][0], basis[3][1], basis[3][2]]);
            let b0 = local(&cam, [second[0], second[1], second[2]]);
            let (a0, b0) = ([a0[0], 0.0, a0[2]], [b0[0], 0.0, b0[2]]);
            // with a turn angle: aim past the second subject so the first sits at that angle
            let mut aim = [0.0f32; 3];
            let ch15 = self.ch(ch::TURN);
            if ch15 != 0.0 {
                let reach = musl::hypotf(a0[0], a0[2]);
                let v = [F(a0[0]) - b0[0], F::ZERO, F(a0[2]) - b0[2]];
                let gap = F(musl::hypotf(v[0].0, v[2].0));
                let ang = diff(ch15, self.ch(ch::FRAME_YAW)).abs();
                let mut beta = musl::asinf((reach * F(musl::sinf(ang)) / gap).0);
                let side = gap / musl::sinf(ang);
                let len = (reach + gap).max(side * musl::sinf((PI - (F(ang) + beta)).0));
                if ch15 < self.ch(ch::FRAME_YAW) {
                    beta = (-F(beta)).0;
                }
                let (sn, cs) = musl::sincosf((-F(beta)).0);
                let r = [v[0] * cs + v[2] * sn, F::ZERO, -v[0] * sn + v[2] * cs];
                let inv = 1.0 / (r[0] * r[0] + r[2] * r[2]).sqrt();
                aim = [(r[0] * inv * len + b0[0]).0, 0.0, (r[2] * inv * len + b0[2]).0];
            }
            let (dx, dz) = (F(b0[0]) - aim[0], F(b0[2]) - aim[2]);
            let mut yaw = F(musl::atan2f(dx.0, dz.0)) - ch15;
            if yaw > PI {
                yaw -= TAU;
            } else if yaw < -F(PI) {
                yaw += TAU;
            }
            // the turn is made from the aim point: the eye moves there
            let mut turn = rot_y(yaw.0);
            turn[3] = [aim[0], aim[1], aim[2], 1.0];
            cam = mul(&cam, &turn);
        }
        self.cam = cam;
        self.fov = self.ch(ch::FOV);
        // keep the eye above the ground, looking at the subject: the subject frame's own spot (the frame it's moved
        // to, before the channel offset), not the basis. A shot whose offset puts the eye on the basis (the
        // close-up, from the ground in front of the player) would otherwise take the asin of a zero-length look (B367)
        let clear = self.float(rec::CLEARANCE);
        if clear != 0.0 && 0.0 < F(self.cam[3][1]) + clear {
            let up = F(self.cam[3][1]) + clear;
            self.cam[3][1] = (F(self.cam[3][1]) - up).0;
            let s = frame(if self.byte(rec::MOVE_TO) != 0 { self.byte(rec::MOVE_TO) } else { self.byte(rec::SUBJECT) })[3];
            let l = fs(local(&self.cam, [s[0], s[1], s[2]]));
            let a = musl::asinf((up / (l[2] * l[2] + l[1] * l[1]).sqrt()).0);
            self.cam = mul(&self.cam, &rot_x((-F(a)).0));
        }
        // a second point kept within a vertical band
        let band = self.float(rec::BAND);
        if band != 0.0 {
            for (o, below) in [(rec::BAND_ABOVE, false), (rec::BAND_BELOW, true)] {
                if self.byte(o) != 0 {
                    let p = frame(self.byte(o))[3];
                    self.band(band, [p[0], p[1], p[2]], below);
                }
            }
        }
        // the head kept inside the top of the picture
        let amount = self.float(rec::HEAD_ROOM);
        if amount != 0.0 {
            for o in rec::HEADS {
                if self.byte(o) != 0 {
                    let p = frame(self.byte(o))[3];
                    self.head(amount, [p[0], p[1], p[2]]);
                }
            }
        }
        if self.byte(rec::FOLLOW_PITCH) != 0 || self.byte(rec::FOLLOW_YAW) != 0 {
            self.frame_target(frame);
        }
    }

    /// Turn toward the record's target frame (the cut-aways' `FOLLOW` is the shown player's spot): pitch and yaw
    /// each follow the target past a dead zone (a fraction of the half-angle, `rec::PITCH_FRAMING` /
    /// `YAW_FRAMING`) with an eased pull. The yaw left over once the pull is at its limit is held for the next
    /// frames (`rec::HOLD_YAW`).
    fn frame_target(&mut self, frame: &impl Fn(u8) -> M4) {
        use std::f32::consts::{PI, TAU};
        let wrap = |a: F| {
            if PI < a {
                a - TAU
            } else if a < -F(PI) {
                a + TAU
            } else {
                a
            }
        };
        let sign = |a: F| if a < 0.0 { -1.0 } else { 1.0 };
        // ponytail: the record 0x14 blend of these from (4, 1, 1) over 90 frames and the eye move of record 0x49
        // are left out: no cut-away sets them
        let [p_off, p_dead, p_scale, p_ease] = rec::PITCH_FRAMING.map(|o| F(self.float(o)));
        let [y_off, y_dead, y_scale, y_ease] = rec::YAW_FRAMING.map(|o| F(self.float(o)));
        let target = frame(match self.byte(rec::TARGET) {
            frame::FOLLOW => frame::SPOT,
            b => b,
        })[3];
        // the reference: this view, its yaw moved by what was held back
        let mut r = self.cam;
        if self.byte(rec::FOLLOW_YAW) != 0 && self.byte(rec::HOLD_YAW) != 0 {
            let f = r[2];
            let eye = r[3];
            r = mul(&rot_y(wrap(F(musl::atan2f(f[0], f[2])) + self.yaw_held).0), &rot_x(pitch(f)));
            r[3] = eye;
        }
        let (fov, vfov) = (self.fov, self.vfov());
        let d: [f32; 4] = std::array::from_fn(|i| (F(target[i]) - r[3][i]).0);
        self.yaw_over = 0.0;
        let mut yaw = wrap(y_off * fov + musl::atan2f(r[2][0], r[2][2]));
        let (mut ey, mut pull_y) = (F::ZERO, F::ZERO);
        if self.byte(rec::FOLLOW_YAW) != 0 {
            ey = F(diff(musl::atan2f(d[0], d[2]), yaw.0));
            if y_dead != 0.0 {
                pull_y = F(musl::atanf((y_dead * musl::tanf(fov)).0));
                let lim = pull_y * y_scale;
                if self.byte(rec::CLAMP_YAW) != 0 && 1.0 < ey.abs() / lim {
                    ey = lim * sign(ey);
                }
                let mut t = (ey.abs() / lim).min(1.0);
                if t == 1.0 {
                    self.yaw_over = diff(ey.0, (lim * sign(ey)).0);
                }
                if y_ease != 0.0 {
                    t = 1.0 - F(musl::powf((1.0 - t).0, y_ease.0));
                }
                pull_y = -pull_y * t * sign(ey);
                ey += pull_y;
            }
            yaw = wrap(yaw + wrap(ey));
        }
        let mut pch = wrap(p_off * vfov + pitch(r[2]));
        let mut ep = F::ZERO;
        if self.byte(rec::FOLLOW_PITCH) != 0 {
            let e = F(diff(pitch(d), pch.0));
            if self.byte(rec::PITCH_UP_ONLY) == 0 || 0.0 <= e {
                let mut pull = F::ZERO;
                if p_dead != 0.0 {
                    let a = F(musl::atanf((p_dead * musl::tanf(vfov)).0));
                    let mut t = (e.abs() / (a * p_scale)).min(1.0);
                    if p_ease != 0.0 {
                        t = 1.0 - F(musl::powf((1.0 - t).0, p_ease.0));
                    }
                    pull = F(musl::atanf((F(musl::tanf((-a * t * sign(e)).0)) * musl::cosf(pull_y.0)).0));
                }
                ep = wrap(e + pull);
            }
            pch = wrap(pch + ep);
        }
        if ep == 0.0 && ey == 0.0 {
            return;
        }
        self.cam = mul(&rot_y(yaw.0), &rot_x(pch.0));
        self.cam[3] = r[3];
    }

    /// The subject frame: the record's frame (turned round, kept upright on request, moved to another frame's
    /// spot, offset by the offset channels in its own axes), aimed at a look-at frame when the record names one.
    pub fn basis(&self, frame: &impl Fn(u8) -> M4) -> M4 {
        let up = [0.0, 1.0, 0.0, 0.0];
        let mut m = frame(self.byte(rec::SUBJECT));
        if self.byte(rec::TURN_ROUND) != 0 {
            m = mul(&m, &rot_y(std::f32::consts::PI));
        }
        if self.byte(rec::UPRIGHT) != 0 && self.byte(rec::LOOK_AT) == 0 && self.byte(rec::SECOND) == 0 {
            m[1] = up;
            m[0] = normalize(cross(up, m[2]));
            m[2] = normalize(cross(m[0], up));
        }
        if self.byte(rec::MOVE_TO) != 0 {
            m[3] = frame(self.byte(rec::MOVE_TO))[3];
        }
        let o = ch::OFFSET.map(|c| self.ch(c));
        let pos: [f32; 4] =
            std::array::from_fn(|i| (F(m[3][i]) + F(m[2][i]) * o[2] + F(m[0][i]) * o[0] + F(m[1][i]) * o[1]).0);
        let look = if self.byte(rec::SECOND) != 0 { self.byte(rec::SECOND) } else { self.byte(rec::LOOK_AT) };
        if look == 0 {
            m[3] = pos;
            return m;
        }
        let t = frame(look)[3];
        let mut z = normalize(std::array::from_fn(|i| (F(t[i]) - pos[i]).0));
        let mut x = normalize(cross(m[1], z));
        let mut y = normalize(cross(z, x));
        if self.byte(rec::UPRIGHT) != 0 {
            y = up;
            x = normalize(cross(y, z));
            z = normalize(cross(x, y));
        }
        [x, y, z, pos]
    }

    fn vfov(&self) -> f32 {
        musl::atanf((F(musl::tanf(self.fov)) * ASPECT).0)
    }

    /// Widen and tilt up so `point` stays within `amount` of the half-height above centre (`MAX_VFOV` at most).
    fn head(&mut self, amount: f32, point: [f32; 3]) {
        /// 35° in radians.
        const MAX_VFOV: f32 = 0.610_865_24;
        let vfov = self.vfov();
        let l = fs(local(&self.cam, point));
        let z = l[2].abs();
        let lim = F(amount) * z * musl::tanf(vfov);
        let over = -l[1] - lim;
        let mut d = (F(musl::atanf(((lim + over) / amount / z).0)) - musl::atanf((lim / z).0)) * 0.5;
        if MAX_VFOV < F(vfov) + d {
            d = MAX_VFOV - F(vfov);
        }
        if 0.0 < d {
            self.fov = musl::atanf((F(musl::tanf((F(vfov) + d).0)) / ASPECT).0);
            self.cam = mul(&self.cam, &rot_x(d.0));
        }
    }

    /// Tilt so `point` stays above (`below` false) or below the band of `amount` × the vertical half-angle.
    fn band(&mut self, amount: f32, point: [f32; 3], below: bool) {
        let lim = -F(musl::atanf((F(amount) * musl::tanf(self.vfov())).0));
        let d = |i: usize| (F(point[i]) - self.cam[3][i]).0;
        let v = [d(0), d(1), d(2), 0.0];
        let a = diff(pitch(v), pitch(self.cam[2]));
        if !below && 0.0 < diff(lim.0, a) {
            self.cam = mul(&self.cam, &rot_x(diff(a, lim.0)));
        } else if below && diff(lim.0, a) < 0.0 {
            self.cam = mul(&self.cam, &rot_x((-F(diff(lim.0, a))).0));
        }
    }

    /// The view as the match camera's type.
    pub fn view(&self) -> View {
        let r = |i: usize| [self.cam[i][0], self.cam[i][1], self.cam[i][2]];
        View { rot: [r(0), r(1), r(2)], eye: r(3), fov: self.fov }
    }
}

/// What the post-point shot pick looks at.
pub struct PickInput {
    /// The shown player plays a doubles team reaction.
    pub team: bool,
    /// The point ends a game or a set.
    pub game_end: bool,
    /// The shown player's team lost the point.
    pub lost: bool,
    /// The shown player's head (frame `SHOWN[2]`) is below 0.8 m (crouched).
    pub low: bool,
    /// Replays shown so far in the match (gm+0x358).
    pub replays: i32,
    pub character: i32,
}

/// Who the cut-away shows and who is the other subject, from the roles `[a, b]` (a = the last hitter, b = the one
/// before; the server and receiver until the return): `a`, unless it isn't on the last hitter's (`last`, −1 for
/// none) team; on a point that ends a game whichever of the two is on the winning team (`winner`, team = index & 1).
/// After every third instant replay of the match (`replays`) they swap again, unless the other one is `diving`.
pub fn roles([a, b]: [usize; 2], last: i32, game_end: bool, winner: i32, replays: i32, diving: impl Fn(usize) -> bool) -> [usize; 2] {
    let swap = if game_end { a as i32 & 1 != (winner != 0) as i32 } else { last >= 0 && a as i32 & 1 != last & 1 };
    let [s, o] = if swap { [b, a] } else { [a, b] };
    if replays != 0 && replays % 3 == 0 && !diving(o) { [o, s] } else { [s, o] }
}

/// Which post-point shot comes next: the point's list (a point, a game-ending point, a team reaction) cycled by
/// its own counter, skipping shots that don't fit (the down-the-line shots when the shown player lost, the high
/// shots when crouched, the long shots on a game-ending point before any replay, the close-up some characters
/// lack), swapping the winner's shot for the loser's on a game-ending point.
pub fn pick(lists: &[Vec<u8>; 3], counters: &mut [usize; 3], i: &PickInput) -> u8 {
    use post::*;
    /// The down-the-line shots, the high shots, the long shots.
    const DOWN_THE_LINE: [u8; 2] = [0x62, 0x63];
    const HIGH: [u8; 2] = [CLOSE_UP, 0x68];
    const LONG: [u8; 4] = [0x63, 0x66, 0x67, 0x68];
    /// The characters without the close-up (and so without the reaction it shows), and those who lose with the
    /// winner's shot (or, character 6, the first three).
    const NO_CLOSE_UP: [i32; 6] = [0, 2, 4, 6, 10, 12];
    const NO_WINNER_SHOT: [i32; 3] = [0, 4, 10];
    let list = if i.team { 2 } else { i.game_end as usize };
    let l = &lists[list];
    let len = l.len();
    let next = |c: &mut [usize; 3]| {
        let n = l[c[list] % len];
        c[list] += 1;
        n
    };
    let mut n = next(counters);
    if !i.team && i.lost {
        while DOWN_THE_LINE.contains(&n) {
            n = next(counters);
        }
    }
    if i.low {
        while HIGH.contains(&n) {
            n = next(counters);
        }
    }
    if i.game_end && i.replays == 0 {
        while LONG.contains(&n) {
            n = next(counters);
        }
    }
    if NO_CLOSE_UP.contains(&i.character) {
        while n == CLOSE_UP {
            n = next(counters);
        }
    }
    if i.game_end && i.lost
        && ((NO_WINNER_SHOT.contains(&i.character) && n == WINNER)
            || (i.character == 6 && (WINNER..CLOSE_UP).contains(&n)))
        {
            n = LOSER;
            counters[list] += 1;
        }
    n
}

/// The instant replay's camera picks (`hst_data::exe::Game::replay_cameras`), each list with its own counter.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReplayPicks {
    pub lists: ReplayCameras,
    /// Per shot-type list (the last: other types on the stroke list), the closing lists, the tracking pairs.
    pub by_type: [usize; 6],
    pub end: [usize; 2],
    pub tracking: usize,
    /// A fast-forwarded replay's first shot (P0b4d3 shows it); a replayed close line call starts it over.
    pub serve: usize,
    /// The last shot picked from a list.
    pub last: u8,
}

/// What a replay pick knows of a player: the costume (0..9) and the character.
#[derive(Clone, Copy, Debug, Default)]
pub struct Look {
    pub costume: i32,
    pub character: i32,
}

impl Look {
    /// Costumes 5–9 of characters 0, 4, 10 and 12 lack some shots (or have their own).
    fn special(&self) -> bool {
        (5..10).contains(&self.costume) && matches!(self.character, 0 | 4 | 10 | 12)
    }
}

impl ReplayPicks {
    pub fn new(lists: ReplayCameras) -> Self {
        ReplayPicks { lists, by_type: [0; 6], end: [0; 2], tracking: 0, serve: 0, last: 0 }
    }

    /// The camera for a recorded shot of type `ty` (the ball's class: 0 serve, 1–2 strokes, 3 smash; 4 the △
    /// smash, 6 a dive) by a player who looks like `who`, skipping the shots their model lacks.
    pub fn shot(&mut self, ty: u8, who: Look) -> u8 {
        let (l, c) = match ty {
            0 => (0, 0),
            1 | 2 => (1, 1),
            3 => (2, 2),
            5 => (3, 3),
            6 => (4, 4),
            _ => (1, 5),
        };
        let list = &self.lists.by_type[l];
        let counter = &mut self.by_type[c];
        let mut next = || {
            let n = list[*counter % list.len()];
            *counter += 1;
            n
        };
        // ponytail: the game holds a stroke's counter while its slow motion runs (P0b4d3's); it never runs here yet
        let mut n = next();
        let (special, ch) = (who.special(), who.character);
        while special && n == 0x1d {
            n = next();
        }
        while special && ch == 12 && n == 0x22 {
            n = next();
        }
        if special {
            while n == 0x38 || n == 0x30 {
                n = next();
            }
            n = match n {
                0x2b => 0x2c,
                0x2d => 0x2e,
                n => n,
            };
        }
        while special && ch == 12 && n == 0x3b {
            n = next();
        }
        if special {
            if n == 0x45 {
                n = 0x46;
            }
            if n == 0x47 {
                n = next();
            }
        }
        if matches!(ch, 0 | 2 | 4 | 6 | 10 | 12) && n == 0x4e {
            n = next();
        }
        self.last = n;
        n
    }

    /// The tracking camera between shots on the match's `replay`-th replay (from 0), with `players` on court.
    pub fn tracking(&mut self, replay: i32, players: usize) -> u8 {
        let pair = self.lists.tracking[replay.rem_euclid(5) as usize];
        let count = pair.iter().filter(|&&n| n != 0).count().max(1);
        let n = pair[self.tracking % count];
        self.tracking += 1;
        if n == 8 && players > 2 { 9 } else { n }
    }

    /// The closing shot once the last shot's window is over: after a close-up of the hitter its own list, else the
    /// other; `a` and `b` are the last hitter and the one before.
    pub fn end(&mut self, a: Look, b: Look) -> u8 {
        let l = matches!(self.last, 0x41 | 0x3d | 0x33 | 0x4d | 0x37) as usize ^ 1;
        let list = self.lists.end[l];
        let mut n = list[self.end[l] % 3];
        self.end[l] += 1;
        if n == 0x56 && self.last == 0x47 {
            n = list[self.end[l] % 3];
            self.end[l] += 1;
        }
        match n {
            0x56 if a.special() => 0x57,
            0x58 if b.special() => 0x59,
            n => n,
        }
    }
}

/// The reference frames the post-point shots look at, set when the cut-away starts.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Frames {
    /// The court corner view (`frame::CORNER`).
    pub corner: M4,
    /// The shown player and the other one: [`subject_frames`].
    pub shown: [[M4; 4]; 2],
    /// The shown player's live head (`frame::HEAD`): [`head_frame`].
    pub head: M4,
    /// The shown player's live ground spot (`frame::SPOT`, the court views' target).
    pub spot: [f32; 4],
}

impl Frames {
    /// Frame `i` of the game's table; the ones no post-point shot uses are the court centre.
    pub fn get(&self, i: u8) -> M4 {
        use frame::*;
        let first = SHOWN[0] as usize;
        match i {
            CORNER => self.corner,
            29 | 30 | 45 | 46 | 61 | 62 | 77 | 78 => self.shown[(i as usize - first) % 2][(i as usize - first) / 16],
            HEAD => self.head,
            SPOT => [IDENTITY[0], IDENTITY[1], IDENTITY[2], self.spot],
            // the table's first frame has a zero last row (w = 0), so a shot channel offset by it stays a direction
            ORIGIN => [IDENTITY[0], IDENTITY[1], IDENTITY[2], [0.0; 4]],
            _ => IDENTITY,
        }
    }
}

/// Where a post-point reaction that carries the player (`gu_set` 0x2e, the team reactions 0x30..) leaves them:
/// its root path point `p` turned by the player's `rows` from where it began (`base`), as
/// [`crate::motion::reaction_root`], as a position (w = 1: the root's spot also adds up the path point's w, and a
/// shot aimed at a head with w = 2 looks away from the player).
pub fn reaction_spot(p: [f32; 4], motion: usize, character: i32, rows: [[f32; 4]; 3], base: [f32; 4]) -> [f32; 4] {
    let [x, y, z, _] = crate::motion::reaction_root(p, motion >= 0x30, character, rows, base, base);
    [x, y, z, 1.0]
}

/// On the court corner nearest `spot` (the shown player's ground spot) yet more than 5 m from it, looking at the
/// court centre.
pub fn corner_frame(spot: [f32; 4]) -> M4 {
    let mut best = (99.0, [0.0, 0.0]);
    for c in [[-5.485, -11.885], [5.485, -11.885], [-5.485, 11.885], [5.485, 11.885]] {
        let (dx, dz) = (F(c[0]) - spot[0], F(c[1]) - spot[2]);
        let d = (dx * dx + dz * dz).sqrt().0;
        if d < best.0 && 5.0 < d {
            best = (d, c);
        }
    }
    let [x, z] = best.1;
    let up = [0.0, 1.0, 0.0, 0.0];
    let f = normalize([(-F(x)).0, 0.0, (-F(z)).0, 0.0]);
    let r = normalize(cross(up, f));
    [r, up, normalize(cross(r, up)), [x, 0.0, z, 1.0]]
}

/// A shown player's frames as their reaction will leave them, from the head and Spine2 bone matrices posed at
/// that time: upright, facing the way the face looks (from the chest to the head instead when the face looks
/// back against the chest or the head lies tilted over), at [the ground under the head, 0.4 m above the head,
/// 0.2 m above it, the chest] (frames 29, 45, 61, 77; +1 for the other player).
pub fn subject_frames(head: &M4, spine: &M4) -> [M4; 4] {
    let up = [0.0, 1.0, 0.0, 0.0];
    let (h, s) = (head[3], spine[3]);
    let mut z = head[2].map(|v| (-F(v)).0);
    if F(z[0]) * -F(spine[2][0]) + F(z[2]) * -F(spine[2][2]) < 0.0 || head[0][1].abs() < 0.4 {
        z = normalize([(F(h[0]) - s[0]).0, 0.0, (F(h[2]) - s[2]).0, 0.0]);
    }
    let x = normalize(cross(up, z));
    let z = normalize(cross(x, up));
    let p = [h[0], (F(h[1]) - 0.2).0, h[2], h[3]];
    let at = |pos| [x, up, z, pos];
    [at([p[0], 0.0, p[2], p[3]]), at([p[0], (F(p[1]) - 0.2).0, p[2], p[3]]), at(p), at(s)]
}

/// Facing along `dir` (normalized) at `pos`, rolled upright: x = up × z, y = z × x. The game builds the ball's
/// frames and the shot recorder's per-hit frames this way.
pub fn facing(dir: [f32; 4], pos: [f32; 4]) -> M4 {
    let z = normalize([dir[0], dir[1], dir[2], 0.0]);
    let x = normalize(cross([0.0, 1.0, 0.0, 0.0], z));
    let m = [x, normalize(cross(z, x)), z, pos];
    if m.iter().flatten().all(|v| v.is_finite()) { m } else { [IDENTITY[0], IDENTITY[1], IDENTITY[2], pos] }
}

/// The live head frame: the head bone turned to the face's axes (`left`: a left-hander's model is mirrored),
/// 0.2 m above the head along it and 0.2 m higher still.
pub fn head_frame(head: &M4, left: bool) -> M4 {
    let s = if left { -1.0 } else { 1.0 };
    let r = |i: usize, k: f32| head[i].map(|v| (F(v) * k).0);
    let p: [f32; 4] = std::array::from_fn(|i| (F(head[3][i]) - 0.2 * F(head[0][i]) - if i == 1 { 0.2 } else { 0.0 }).0);
    [r(1, s), r(0, 1.0), r(2, -1.0), p]
}

/// The court intro's shot: its record only names the court centre as the subject, the script comes from the
/// court's camera file.
pub const INTRO_SHOT: u8 = 107;

/// The court intro's script from the court's camera file (`data/camed/cam_cNN_00.dat`: 40-byte records of
/// frames (0 starts a new segment), an orbit flag, ease, ease-out, eye x y z, pitch, yaw, roll in degrees and
/// the field of view). Each record after the first moves every channel there over its frames; a segment
/// whose first record has the flag set runs as an orbit around the court centre (framing angles and a distance
/// instead of an eye), so its moves swing round rather than slide.
pub fn intro_script(dat: &[u8]) -> Vec<(u8, f32)> {
    use op::*;
    let mut out = Vec::new();
    let mut orbit = true;
    for (i, r) in dat.as_chunks::<40>().0.iter().take(20).enumerate() {
        let h = |o: usize| i16::from_le_bytes([r[o], r[o + 1]]);
        let f = |o: usize| f32::from_le_bytes(r[o..o + 4].try_into().unwrap());
        let time = h(0);
        if time == 0 {
            orbit = h(2) != 0;
        }
        if i != 0 {
            let ease = if f(4) == 0.0 { 1.0 } else { f(4) };
            out.extend([(TIME, time as f32), (EASE, ease), (EASE_OUT, f(8))]);
        }
        let (eye, (p, y, roll)) = ([f(0xc), f(0x10), f(0x14)], (f(0x18), f(0x1c), f(0x20)));
        out.extend([(PITCH, p), (YAW, y), (ROLL, roll)]);
        if orbit {
            // the eye as framing angles and a distance from the court centre, seen from the view without roll
            let cam = mul(&rot_y(y.to_radians()), &rot_x(p.to_radians()));
            let l = local(&cam, eye);
            let l = [(-F(l[0])).0, (-F(l[1])).0, (-F(l[2])).0, 0.0];
            let (yl, pl) = (musl::atan2f(l[0], l[2]), pitch(l));
            let a = musl::atanf((F(musl::tanf(pl)) / musl::cosf(yl)).0);
            let e = fs(eye);
            let d = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt().0;
            out.extend([
                (FRAME_PITCH, (-F(a.to_degrees())).0),
                (FRAME_YAW, yl.to_degrees()),
                (DISTANCE, d),
                (OFFSET_X, 0.0),
                (OFFSET_Y, 0.0),
                (OFFSET_Z, 0.0),
            ]);
        } else {
            out.extend([
                (OFFSET_X, eye[0]),
                (OFFSET_Y, eye[1]),
                (OFFSET_Z, eye[2]),
                (FRAME_PITCH, 0.0),
                (FRAME_YAW, 0.0),
                (DISTANCE, 0.0),
            ]);
        }
        out.push((FOV, f(0x24)));
    }
    out
}

/// The intro's frames: the camera file's record times added up.
pub fn intro_frames(dat: &[u8]) -> u32 {
    dat.as_chunks::<40>().0.iter().take(20).map(|r| i16::from_le_bytes([r[0], r[1]]).max(0) as u32).sum()
}

impl Shot {
    /// The court intro (`shots`: the game's camera shots, `dat`: the court's camera file).
    pub fn intro(shots: &[CameraShot], dat: &[u8]) -> Self {
        let channels = compile(&intro_script(dat), false).into_iter().map(Channel::new).collect();
        let mut s = Shot {
            number: INTRO_SHOT,
            record: shots[INTRO_SHOT as usize].record.clone(),
            channels,
            cam: IDENTITY,
            fov: 0.0,
            yaw_held: 0.0,
            yaw_over: 0.0,
        };
        s.build(&Self::court);
        s
    }

    /// The intro's only reference frame: the court centre (the others are unset).
    pub fn court(i: u8) -> M4 {
        if i == frame::COURT_CENTRE { IDENTITY } else { [[0.0; 4]; 4] }
    }

    /// The view turned by the roll channel about its forward axis (the intro tilts the picture; the view the
    /// shot builds never rolls).
    pub fn rolled(&self) -> View {
        let mut v = self.view();
        let (s, c) = musl::sincosf(self.ch(ch::ROLL));
        let [r, d, _] = v.rot;
        v.rot[0] = std::array::from_fn(|i| (F(r[i]) * c + F(d[i]) * s).0);
        v.rot[1] = std::array::from_fn(|i| (F(d[i]) * c - F(r[i]) * s).0);
        v
    }
}
