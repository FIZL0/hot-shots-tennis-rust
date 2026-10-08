//! The match's scripted camera shots (the cut-away after a point): a shot number picks a record and a script
//! from the game program (`hst_data::exe::Game::camera_shots`). The script sets keyed channels — pitch, yaw,
//! framing angles, distance, field of view — that run from frame to frame with the game's easing; each frame the
//! view orbits a subject frame (a player's ground spot facing the way they face), sits at the shot's distance
//! with the framing offset, then widens and tilts so the subject's head stays in the picture, and keeps clear of
//! the ground. Game space (Y down); views are rows right, down, forward and an eye, as [`crate::camera::View`].
//! Plain f32 like the match camera: the view never feeds back into play.

use crate::camera::View;
pub use hst_data::exe::CameraShot;

pub type M4 = [[f32; 4]; 4];

/// Channels: 5 pitch, 6 yaw, 7 roll, 11 / 12 framing pitch / yaw, 13 distance, 14 field of view.
const CHANNELS: usize = 17;
const END: u8 = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Key {
    value: f32,
    /// 0: `time` frames; other kinds are event-relative times (not used by the cut-aways).
    kind: u8,
    time: f32,
    ease: f32,
    ease_out: f32,
}

#[derive(Clone, Debug, Default)]
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
/// op (0x11..0x16) starts the next keys of every channel set after it.
fn compile(script: &[(u8, f32)], mirror: bool) -> Vec<Vec<Key>> {
    // pass 1: conversions and segments, the fov moved to the start of its segment
    let mut buf: Vec<(u8, f32)> = Vec::new();
    let (mut seg, mut timing, mut fov) = (0usize, false, 0.0f32);
    for &(op, v) in script.iter().filter(|e| e.0 != 0x19).chain(std::iter::once(&(0x1a, 0.0))) {
        if !(0x11..=0x18).contains(&op) {
            if timing {
                timing = false;
                seg = buf.len();
            }
        } else if op != 0x17 && op != 0x18 {
            timing = true;
        }
        if (timing || op == 0x1a) && fov != 0.0 {
            buf.insert(seg, (0x0e, fov));
            fov = 0.0;
        }
        if op == 0x1a {
            break;
        }
        let mut v = match op {
            0x0e => v * 0.008726646,
            0x0f | 0x0c | 0x0b | 0x07 | 0x06 | 0x05 => v * 0.017453292,
            _ => v,
        };
        if op == 0x0b || op == 0x02 {
            v = -v;
        }
        if mirror && matches!(op, 8 | 0x0f | 4 | 0x0c | 3 | 7 | 6 | 1) {
            v = -v;
        }
        if op == 0x0e {
            fov = v;
        } else {
            buf.push((op, v));
        }
    }
    // pass 2: keys per channel (each ends where its count stops; the game marks it with an end key)
    let mut keys: Vec<Vec<Key>> = vec![Vec::new(); CHANNELS];
    let mut count = [0usize; CHANNELS];
    let (mut kind, mut time, mut ease, mut ease_out, mut timed) = (0u8, 0.0f32, 1.0f32, 0.0f32, 0);
    let (mut hfov, mut vfov) = (40.0f32, (40.0f32.tan() * 0.7).atan());
    let mut first = true;
    for (op, v) in buf {
        match op {
            0x11..=0x16 => {
                kind = op - 0x11;
                time = v;
                timed += 1;
                continue;
            }
            0x17 => {
                ease = v;
                continue;
            }
            0x18 => {
                ease_out = v;
                continue;
            }
            _ => {}
        }
        if first {
            first = false;
            for (c, k) in keys.iter_mut().enumerate().skip(5) {
                *k = vec![Key { value: if c == 0x0e { hfov } else { 0.0 }, kind, time, ease, ease_out }];
            }
        }
        let (ch, value) = match op {
            0x0e => {
                hfov = v;
                vfov = (v.tan() * 0.7).atan();
                (0x0e, v)
            }
            4 => (0x0f, (v * hfov.tan()).atan()),
            3 => (0x0c, (v * hfov.tan()).atan()),
            2 => (0x0b, (v * vfov.tan()).atan()),
            1 => (6, v * hfov),
            0 => (5, v * vfov),
            op => (op as usize, v),
        };
        if !(5..CHANNELS).contains(&ch) || count[ch] >= 0x20 {
            continue;
        }
        let k = &mut keys[ch];
        // a channel first set inside a timed segment holds (this value before a second time op, else 0) first
        if count[ch] == 0 && (kind != 0 || time != 0.0) {
            k[0].value = if timed == 1 { value } else { 0.0 };
            count[ch] += 1;
        }
        k.truncate(count[ch]);
        k.push(Key { value, kind, time, ease, ease_out });
        count[ch] += 1;
    }
    for (c, k) in keys.iter_mut().enumerate().skip(5) {
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
        let mut t = 1.0 - self.left as f32 / self.frames;
        let e = if self.ease != 0.0 { self.ease } else { 1.0 };
        if 0.0 < e && 0.0 <= self.ease_out {
            t = t.powf((1.0 - t).powf(self.ease_out)).powf(e);
        }
        let (a, b) = (self.keys[self.to - 1].value, self.keys[self.to].value);
        self.value = t * (b - a) + a;
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
fn mul(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..4).map(|k| b[r][k] * a[k][c]).sum()))
}
fn rot_y(t: f32) -> M4 {
    let (s, c) = t.sin_cos();
    [[c, 0.0, -s, 0.0], [0.0, 1.0, 0.0, 0.0], [s, 0.0, c, 0.0], [0.0, 0.0, 0.0, 1.0]]
}
fn rot_x(t: f32) -> M4 {
    let (s, c) = t.sin_cos();
    [[1.0, 0.0, 0.0, 0.0], [0.0, c, s, 0.0], [0.0, -s, c, 0.0], [0.0, 0.0, 0.0, 1.0]]
}
pub const IDENTITY: M4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

/// A point in the camera's frame (x right, y down, z forward).
fn local(cam: &M4, p: [f32; 3]) -> [f32; 3] {
    let d = [p[0] - cam[3][0], p[1] - cam[3][1], p[2] - cam[3][2]];
    std::array::from_fn(|i| d[0] * cam[i][0] + d[1] * cam[i][1] + d[2] * cam[i][2])
}

fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0], 0.0]
}

fn normalize(v: [f32; 4]) -> [f32; 4] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / l, v[1] / l, v[2] / l, 0.0]
}

/// Vertical angle of `v` above the horizon (Y down).
fn pitch(v: [f32; 4]) -> f32 {
    (-v[1]).atan2((v[0] * v[0] + v[2] * v[2]).sqrt())
}

/// Wrapped difference a − b.
fn diff(a: f32, b: f32) -> f32 {
    use std::f32::consts::TAU;
    let d = (a + std::f32::consts::PI) - (b + std::f32::consts::PI);
    let (p, m) = (d + TAU, d - TAU);
    if (p.abs() <= d.abs() || m.abs() <= d.abs()) && m.abs() <= p.abs() {
        m
    } else if p.abs() <= d.abs() || m.abs() <= d.abs() {
        p
    } else {
        d
    }
}

/// A running shot.
#[derive(Clone, Debug)]
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
        self.byte(0)
    }

    /// Advance one frame and build the view from the subject frames (`frame(i)`: the game's reference frame i).
    pub fn step(&mut self, frame: impl Fn(u8) -> M4) {
        for c in &mut self.channels[5..] {
            c.step();
        }
        if self.byte(0x4c) != 0 && self.byte(0x4d) != 0 {
            self.yaw_held += self.yaw_over;
        }
        self.build(&frame);
    }

    /// Build the view for the current channel values (`step` without advancing).
    pub fn build(&mut self, frame: &impl Fn(u8) -> M4) {
        let basis = self.basis(frame);
        // orbit: the subject's frame turned by yaw then pitch, rebuilt from its forward without roll
        let cam = mul(&basis, &rot_y(self.ch(6)));
        let cam = mul(&cam, &rot_x(self.ch(5)));
        let f = cam[2];
        let (p, y) = (pitch(f), f[0].atan2(f[2]));
        let mut cam = mul(&rot_y(y), &rot_x(p));
        // framing: the eye moves so the subject sits off-centre by the framing angles
        let a11 = (self.ch(11).tan() * self.ch(12).cos()).atan();
        let o = mul(&mul(&cam, &rot_y(self.ch(12))), &rot_x(a11));
        let d = self.ch(13);
        cam[3] = [basis[3][0] - o[2][0] * d, basis[3][1] - o[2][1] * d, basis[3][2] - o[2][2] * d, 1.0];
        // a second subject (a frame with w ≠ 0): the view turns to face it across the court plane
        let second = frame(self.byte(3))[3];
        if second[3] != 0.0 {
            use std::f32::consts::{PI, TAU};
            let a0 = local(&cam, [basis[3][0], basis[3][1], basis[3][2]]);
            let b0 = local(&cam, [second[0], second[1], second[2]]);
            let (a0, b0) = ([a0[0], 0.0, a0[2]], [b0[0], 0.0, b0[2]]);
            // with a turn angle (ch 15): aim past the second subject so the first sits at that angle
            let mut aim = [0.0f32; 3];
            let ch15 = self.ch(15);
            if ch15 != 0.0 {
                let reach = a0[0].hypot(a0[2]);
                let v = [a0[0] - b0[0], 0.0, a0[2] - b0[2]];
                let gap = v[0].hypot(v[2]);
                let ang = diff(ch15, self.ch(12)).abs();
                let mut beta = (reach * ang.sin() / gap).asin();
                let side = gap / ang.sin();
                let len = (reach + gap).max(side * (PI - (ang + beta)).sin());
                if ch15 < self.ch(12) {
                    beta = -beta;
                }
                let (sn, cs) = (-beta).sin_cos();
                let r = [v[0] * cs + v[2] * sn, 0.0, -v[0] * sn + v[2] * cs];
                let inv = 1.0 / (r[0] * r[0] + r[2] * r[2]).sqrt();
                aim = [r[0] * inv * len + b0[0], 0.0, r[2] * inv * len + b0[2]];
            }
            let (dx, dz) = (b0[0] - aim[0], b0[2] - aim[2]);
            let mut yaw = dx.atan2(dz) - ch15;
            if yaw > PI {
                yaw -= TAU;
            } else if yaw < -PI {
                yaw += TAU;
            }
            // the turn is made from the aim point: the eye moves there
            let mut turn = rot_y(yaw);
            turn[3] = [aim[0], aim[1], aim[2], 1.0];
            cam = mul(&cam, &turn);
        }
        self.cam = cam;
        self.fov = self.ch(14);
        // keep the eye above the ground, looking at the subject
        let clear = self.float(0xac);
        if clear != 0.0 && 0.0 < self.cam[3][1] + clear {
            let up = self.cam[3][1] + clear;
            self.cam[3][1] -= up;
            let l = local(&self.cam, [basis[3][0], basis[3][1], basis[3][2]]);
            let a = (up / (l[2] * l[2] + l[1] * l[1]).sqrt()).asin();
            self.cam = mul(&self.cam, &rot_x(-a));
        }
        // a second point kept within a vertical band
        let band = self.float(0x90);
        if band != 0.0 {
            for (o, below) in [(0x8c, false), (0x8d, true)] {
                if self.byte(o) != 0 {
                    let p = frame(self.byte(o))[3];
                    self.band(band, [p[0], p[1], p[2]], below);
                }
            }
        }
        // the head kept inside the top of the picture
        let amount = self.float(0x88);
        if amount != 0.0 {
            for o in [0x84, 0x85] {
                if self.byte(o) != 0 {
                    let p = frame(self.byte(o))[3];
                    self.head(amount, [p[0], p[1], p[2]]);
                }
            }
        }
        if self.byte(0x4a) != 0 || self.byte(0x4c) != 0 {
            self.frame_target(frame);
        }
    }

    /// Turn toward the record's target frame (the cut-aways' 9 is the shown player's spot, 19): pitch and yaw
    /// each follow the target past a dead zone (a fraction of the half-angle, record 0x54.. / 0x64..) with an
    /// eased pull. The yaw left over once the pull is at its limit is held for the next frames (record 0x4d).
    fn frame_target(&mut self, frame: &impl Fn(u8) -> M4) {
        use std::f32::consts::{PI, TAU};
        let wrap = |a: f32| if PI < a { a - TAU } else if a < -PI { a + TAU } else { a };
        let sign = |a: f32| if a < 0.0 { -1.0 } else { 1.0 };
        // ponytail: the record 0x14 blend of these from (4, 1, 1) over 90 frames and the eye move of record 0x49
        // are left out: no cut-away sets them
        let (p_off, p_dead, p_scale, p_ease) = (self.float(0x50), self.float(0x54), self.float(0x58), self.float(0x5c));
        let (y_off, y_dead, y_scale, y_ease) = (self.float(0x60), self.float(0x64), self.float(0x68), self.float(0x6c));
        let target = frame(match self.byte(0x48) {
            9 => 19,
            b => b,
        })[3];
        // the reference: this view, its yaw moved by what was held back
        let mut r = self.cam;
        if self.byte(0x4c) != 0 && self.byte(0x4d) != 0 {
            let f = r[2];
            let eye = r[3];
            r = mul(&rot_y(wrap(f[0].atan2(f[2]) + self.yaw_held)), &rot_x(pitch(f)));
            r[3] = eye;
        }
        let (fov, vfov) = (self.fov, self.vfov());
        let d = [target[0] - r[3][0], target[1] - r[3][1], target[2] - r[3][2], target[3] - r[3][3]];
        self.yaw_over = 0.0;
        let mut yaw = wrap(y_off * fov + r[2][0].atan2(r[2][2]));
        let (mut ey, mut pull_y) = (0.0, 0.0);
        if self.byte(0x4c) != 0 {
            ey = diff(d[0].atan2(d[2]), yaw);
            if y_dead != 0.0 {
                pull_y = (y_dead * fov.tan()).atan();
                let lim = pull_y * y_scale;
                if self.byte(0x70) != 0 && 1.0 < ey.abs() / lim {
                    ey = lim * sign(ey);
                }
                let mut t = (ey.abs() / lim).min(1.0);
                if t == 1.0 {
                    self.yaw_over = diff(ey, lim * sign(ey));
                }
                if y_ease != 0.0 {
                    t = 1.0 - (1.0 - t).powf(y_ease);
                }
                pull_y = -pull_y * t * sign(ey);
                ey += pull_y;
            }
            yaw = wrap(yaw + wrap(ey));
        }
        let mut pch = wrap(p_off * vfov + pitch(r[2]));
        let mut ep = 0.0;
        if self.byte(0x4a) != 0 {
            let e = diff(pitch(d), pch);
            if self.byte(0x4b) == 0 || 0.0 <= e {
                let mut pull = 0.0;
                if p_dead != 0.0 {
                    let a = (p_dead * vfov.tan()).atan();
                    let mut t = (e.abs() / (a * p_scale)).min(1.0);
                    if p_ease != 0.0 {
                        t = 1.0 - (1.0 - t).powf(p_ease);
                    }
                    pull = ((-a * t * sign(e)).tan() * pull_y.cos()).atan();
                }
                ep = wrap(e + pull);
            }
            pch = wrap(pch + ep);
        }
        if ep == 0.0 && ey == 0.0 {
            return;
        }
        self.cam = mul(&rot_y(yaw), &rot_x(pch));
        self.cam[3] = r[3];
    }

    /// The subject frame: the record's frame (turned round, kept upright on request, moved to another frame's
    /// spot, offset by channels 8–10 in its own axes), aimed at a look-at frame when the record names one.
    pub fn basis(&self, frame: &impl Fn(u8) -> M4) -> M4 {
        let up = [0.0, 1.0, 0.0, 0.0];
        let mut m = frame(self.byte(0));
        if self.byte(4) != 0 {
            m = mul(&m, &rot_y(std::f32::consts::PI));
        }
        if self.byte(5) != 0 && self.byte(1) == 0 && self.byte(3) == 0 {
            m[1] = up;
            m[0] = normalize(cross(up, m[2]));
            m[2] = normalize(cross(m[0], up));
        }
        if self.byte(2) != 0 {
            m[3] = frame(self.byte(2))[3];
        }
        let o = [self.ch(8), self.ch(9), self.ch(10)];
        let pos: [f32; 4] = std::array::from_fn(|i| m[3][i] + m[2][i] * o[2] + m[0][i] * o[0] + m[1][i] * o[1]);
        let look = if self.byte(3) != 0 { self.byte(3) } else { self.byte(1) };
        if look == 0 {
            m[3] = pos;
            return m;
        }
        let t = frame(look)[3];
        let mut z = normalize([t[0] - pos[0], t[1] - pos[1], t[2] - pos[2], t[3] - pos[3]]);
        let mut x = normalize(cross(m[1], z));
        let mut y = normalize(cross(z, x));
        if self.byte(5) != 0 {
            y = up;
            x = normalize(cross(y, z));
            z = normalize(cross(x, y));
        }
        [x, y, z, pos]
    }

    fn vfov(&self) -> f32 {
        (self.fov.tan() * 0.7).atan()
    }

    /// Widen and tilt up so `point` stays within `amount` of the half-height above centre (35° at most).
    fn head(&mut self, amount: f32, point: [f32; 3]) {
        let vfov = self.vfov();
        let l = local(&self.cam, point);
        let z = l[2].abs();
        let lim = amount * z * vfov.tan();
        let over = -l[1] - lim;
        let mut d = (((lim + over) / amount / z).atan() - (lim / z).atan()) * 0.5;
        if 0.610_865_24 < vfov + d {
            d = 0.610_865_24 - vfov;
        }
        if 0.0 < d {
            self.fov = ((vfov + d).tan() / 0.7).atan();
            self.cam = mul(&self.cam, &rot_x(d));
        }
    }

    /// Tilt so `point` stays above (`below` false) or below the band of `amount` × the vertical half-angle.
    fn band(&mut self, amount: f32, point: [f32; 3], below: bool) {
        let lim = (amount * self.vfov().tan()).atan();
        let v = [point[0] - self.cam[3][0], point[1] - self.cam[3][1], point[2] - self.cam[3][2], 0.0];
        let a = diff(pitch(v), pitch(self.cam[2]));
        if !below && 0.0 < diff(-lim, a) {
            self.cam = mul(&self.cam, &rot_x(diff(a, -lim)));
        } else if below && diff(-lim, a) < 0.0 {
            self.cam = mul(&self.cam, &rot_x(-diff(-lim, a)));
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
    /// The shown player's head (frame 61) is below 0.8 m (crouched).
    pub low: bool,
    /// Replays shown so far in the match (none until replays are ported).
    pub replays: i32,
    pub character: i32,
}

/// Who the cut-away shows and who is the other subject, from the roles `[a, b]` (a = the last hitter, b = the one
/// before; the server and receiver until the return): `a`, unless it isn't on the last hitter's (`last`, −1 for
/// none) team; on a point that ends a game whichever of the two is on the winning team (`winner`, team = index & 1).
/// ponytail: the original also swaps them on every third instant replay (unless the other one's character has
/// some flag); replays aren't ported (P0b4d), so the count is always 0 here
pub fn roles([a, b]: [usize; 2], last: i32, game_end: bool, winner: i32) -> [usize; 2] {
    let swap = if game_end { a as i32 & 1 != (winner != 0) as i32 } else { last >= 0 && a as i32 & 1 != last & 1 };
    if swap { [b, a] } else { [a, b] }
}

/// Which post-point shot comes next: the point's list (a point, a game-ending point, a team reaction) cycled by
/// its own counter, skipping shots that don't fit (the down-the-line shots when the shown player lost, the high
/// shots when crouched, the long shots on a game-ending point before any replay, the close-up some characters
/// lack), swapping the winner's shot for the loser's on a game-ending point.
pub fn pick(lists: &[Vec<u8>; 3], counters: &mut [usize; 3], i: &PickInput) -> u8 {
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
        while n == 0x62 || n == 0x63 {
            n = next(counters);
        }
    }
    if i.low {
        while n == 0x64 || n == 0x68 {
            n = next(counters);
        }
    }
    if i.game_end && i.replays == 0 {
        while matches!(n, 0x63 | 0x66 | 0x67 | 0x68) {
            n = next(counters);
        }
    }
    if matches!(i.character, 0 | 2 | 4 | 6 | 10 | 12) {
        while n == 0x64 {
            n = next(counters);
        }
    }
    if i.game_end && i.lost {
        if (matches!(i.character, 0 | 4 | 10) && n == 0x61) || (i.character == 6 && (0x61..0x64).contains(&n)) {
            n = 0x65;
            counters[list] += 1;
        }
    }
    n
}

/// The reference frames the post-point shots look at, set when the cut-away starts.
#[derive(Clone, Copy, Debug)]
pub struct Frames {
    /// The court corner view (frame 6).
    pub corner: M4,
    /// The shown player and the other one: [`subject_frames`].
    pub shown: [[M4; 4]; 2],
    /// The shown player's live head (frame 35): [`head_frame`].
    pub head: M4,
    /// The shown player's live ground spot (frame 19, the court views' target).
    pub spot: [f32; 4],
}

impl Frames {
    /// Frame `i` of the game's table; the ones no post-point shot uses are the court centre.
    pub fn get(&self, i: u8) -> M4 {
        match i {
            6 => self.corner,
            29 | 30 | 45 | 46 | 61 | 62 | 77 | 78 => self.shown[(i as usize - 29) % 2][(i as usize - 29) / 16],
            35 => self.head,
            19 => [IDENTITY[0], IDENTITY[1], IDENTITY[2], self.spot],
            _ => IDENTITY,
        }
    }
}

/// On the court corner nearest `spot` (the shown player's ground spot) yet more than 5 m from it, looking at the
/// court centre.
pub fn corner_frame(spot: [f32; 4]) -> M4 {
    let mut best = (99.0, [0.0, 0.0]);
    for c in [[-5.485, -11.885], [5.485, -11.885], [-5.485, 11.885], [5.485, 11.885]] {
        let d = ((c[0] - spot[0]) * (c[0] - spot[0]) + (c[1] - spot[2]) * (c[1] - spot[2])).sqrt();
        if d < best.0 && 5.0 < d {
            best = (d, c);
        }
    }
    let [x, z] = best.1;
    let up = [0.0, 1.0, 0.0, 0.0];
    let f = normalize([-x, 0.0, -z, 0.0]);
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
    let mut z = [-head[2][0], -head[2][1], -head[2][2], -head[2][3]];
    if z[0] * -spine[2][0] + z[2] * -spine[2][2] < 0.0 || head[0][1].abs() < 0.4 {
        z = normalize([h[0] - s[0], 0.0, h[2] - s[2], 0.0]);
    }
    let x = normalize(cross(up, z));
    let z = normalize(cross(x, up));
    let p = [h[0], h[1] - 0.2, h[2], h[3]];
    let at = |pos| [x, up, z, pos];
    [at([p[0], 0.0, p[2], p[3]]), at([p[0], p[1] - 0.2, p[2], p[3]]), at(p), at(s)]
}

/// The live head frame: the head bone turned to the face's axes (`left`: a left-hander's model is mirrored),
/// 0.2 m above the head along it and 0.2 m higher still.
pub fn head_frame(head: &M4, left: bool) -> M4 {
    let s = if left { -1.0 } else { 1.0 };
    let r = |i: usize, k: f32| [head[i][0] * k, head[i][1] * k, head[i][2] * k, head[i][3] * k];
    let p: [f32; 4] = std::array::from_fn(|i| head[3][i] - 0.2 * head[0][i] - if i == 1 { 0.2 } else { 0.0 });
    [r(1, s), r(0, 1.0), r(2, -1.0), p]
}
