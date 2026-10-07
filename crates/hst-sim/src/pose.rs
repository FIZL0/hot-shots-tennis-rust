//! Skeleton poses from the game's motions: a node's local matrix from its keys (rotation quaternion, position),
//! model-space matrices down the hierarchy. Row-vector matrices as the game's (model = local · parent).

use hst_data::ani::Anim;

use crate::{libm, ps2};

pub type M4 = [[f32; 4]; 4];

/// A skeleton: node names, parents (index before child) and rest local matrices.
pub struct Skeleton {
    pub names: Vec<String>,
    pub parent: Vec<Option<usize>>,
    pub rest: Vec<M4>,
}

fn mul(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..4).map(|k| a[r][k] * b[k][c]).sum()))
}

/// Row-vector rotation of a stored key (the key is the conjugate of the column-sense local rotation, so the
/// row-vector matrix is the key's own column-sense matrix).
fn rotation([x, y, z, w]: [f32; 4]) -> M4 {
    [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w), 0.0],
        [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w), 0.0],
        [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Model-space matrix of every node at the motion's first frame.
/// ponytail: first keys, unscaled (what the turn's pelvis table needs, within 1e-4); [`Clip`] is the game's
/// exact sampler.
pub fn first_frame(sk: &Skeleton, anim: &Anim) -> Vec<M4> {
    let mut model: Vec<M4> = Vec::with_capacity(sk.names.len());
    for i in 0..sk.names.len() {
        let mut local = sk.rest[i];
        if let Some(t) = anim.tracks.iter().find(|t| t.name == sk.names[i]) {
            if let Some(&(_, q)) = t.rotation.first() {
                let r = rotation(q);
                for row in 0..3 {
                    local[row] = r[row];
                }
            }
            if let Some(&(_, p)) = t.position.first() {
                local[3] = [p[0], p[1], p[2], 1.0];
            }
        }
        model.push(match sk.parent[i] {
            Some(p) => mul(&local, &model[p]),
            None => local,
        });
    }
    model
}

/// A motion bound to a skeleton as the game's ANI player holds it: per keyed track its node, position scale and
/// key lists with the interpolation tangents worked out at load.
#[derive(Clone)]
pub struct Clip {
    pub ticks_per_frame: f32,
    /// Length in game frames: the last key over all tracks.
    pub length: f32,
    pub tracks: Vec<ClipTrack>,
}

#[derive(Clone)]
pub struct ClipTrack {
    pub node: usize,
    /// Position keys × this: the skeleton's bone length over the motion's (1 for the pelvis and unknown lengths).
    pub scale: f32,
    rot: Keys,
    pos: Keys,
}

/// Keys of one list: tick, value and tangents (rotation: squad control quaternion in `[0]`; position: incoming
/// tangent `[0]`, outgoing `[1]`).
#[derive(Clone, Default)]
struct Keys {
    tick: Vec<i32>,
    value: Vec<[f32; 4]>,
    tan: Vec<[[f32; 4]; 2]>,
}

/// conj(c) · q.
fn inv_mul(c: [f32; 4], q: [f32; 4]) -> [f32; 4] {
    use ps2::{madd, msub, mul};
    let [cx, cy, cz, cw] = c;
    let [x, y, z, w] = q;
    [
        madd(msub(msub(mul(cw, x), cx, w), cy, z), cz, y),
        madd(msub(msub(mul(cw, y), cy, w), cz, x), cx, z),
        madd(msub(msub(mul(cw, z), cz, w), cx, y), cy, x),
        madd(madd(madd(mul(cx, x), cw, w), cy, y), cz, z),
    ]
}

fn norm3([x, y, z, _]: [f32; 4]) -> f32 {
    use ps2::{madd, mul};
    ps2::sqrt(madd(madd(mul(y, y), x, x), z, z))
}

fn q_log(q: [f32; 4]) -> [f32; 4] {
    let n = norm3(q);
    let a = libm::atan2f(n, q[3]);
    let k = if n <= 0.0 { n } else { ps2::div(a, n) };
    [ps2::mul(k, q[0]), ps2::mul(k, q[1]), ps2::mul(k, q[2]), 0.0]
}

fn q_exp(v: [f32; 4]) -> [f32; 4] {
    let n = norm3(v);
    let k = if n <= f32::from_bits(0x38d1_b717) { 1.0 } else { ps2::div(libm::sinf(n), n) };
    [ps2::mul(k, v[0]), ps2::mul(k, v[1]), ps2::mul(k, v[2]), libm::cosf(n)]
}

/// Squad control point of `cur` between `prev` and `next`: cur · exp(−(log(cur⁻¹prev) + log(cur⁻¹next)) / 8).
fn squad_control(prev: [f32; 4], cur: [f32; 4], next: [f32; 4]) -> [f32; 4] {
    use ps2::{madd, msub, mul};
    let (a, b) = (q_log(inv_mul(cur, prev)), q_log(inv_mul(cur, next)));
    let e = q_exp(std::array::from_fn(|k| mul(ps2::add(b[k], a[k]), -0.125)));
    let [cx, cy, cz, cw] = cur;
    let [x, y, z, w] = e;
    [
        msub(madd(madd(mul(cw, x), cx, w), cy, z), cz, y),
        msub(madd(madd(mul(cw, y), cy, w), cz, x), cx, z),
        msub(madd(madd(mul(cw, z), cz, w), cx, y), cy, x),
        msub(msub(msub(mul(cw, w), cx, x), cy, y), cz, z),
    ]
}

impl Keys {
    fn new(keys: &[(i32, [f32; 4])]) -> Keys {
        Keys { tick: keys.iter().map(|k| k.0).collect(), value: keys.iter().map(|k| k.1).collect(), tan: vec![] }
    }

    fn rotation(keys: &[(i32, [f32; 4])]) -> Keys {
        let mut k = Keys::new(keys);
        let q = &k.value;
        let n = q.len();
        if n >= 2 {
            k.tan = (0..n)
                .map(|i| {
                    let s = match i {
                        0 => squad_control(q[1], q[0], q[1]),
                        _ if i == n - 1 => squad_control(q[n - 2], q[n - 1], q[n - 2]),
                        _ => squad_control(q[i - 1], q[i], q[i + 1]),
                    };
                    [s, [0.0; 4]]
                })
                .collect();
        }
        k
    }

    /// Catmull-Rom tangents scaled by the neighbouring key intervals; the ends mirror their neighbour's.
    fn position(keys: &[(i32, [f32; 4])]) -> Keys {
        use ps2::{div, mul, sub};
        let mut k = Keys::new(keys);
        let (p, t) = (&k.value, &k.tick);
        let n = p.len();
        let d = |a: usize, b: usize| -> [f32; 4] { std::array::from_fn(|c| sub(p[a][c], p[b][c])) };
        if n == 2 {
            k.tan = vec![[d(1, 0); 2]; 2];
        } else if n > 2 {
            let mut tan = vec![[[0.0; 4]; 2]; n];
            for i in 1..n - 1 {
                let span = (t[i + 1] - t[i - 1]) as f32;
                let (fin, fout) = (div((t[i] - t[i - 1]) as f32, span), div((t[i + 1] - t[i]) as f32, span));
                let v = d(i + 1, i - 1);
                tan[i] = [v.map(|c| mul(c, fin)), v.map(|c| mul(c, fout))];
            }
            let end = |v: [f32; 4], m: [f32; 4]| -> [f32; 4] { std::array::from_fn(|c| mul(sub(mul(v[c], 3.0), m[c]), 0.5)) };
            let first = end(d(1, 0), tan[1][0]);
            let last = end(d(n - 1, n - 2), tan[n - 2][1]);
            tan[0] = [first; 2];
            tan[n - 1] = [last; 2];
            k.tan = tan;
        }
        k
    }

    /// The key pair around tick `t` and the fraction between them, or the key held at either end.
    fn find(&self, t: f32) -> Result<(usize, f32), usize> {
        let n = self.tick.len();
        if self.tick[n - 1] as f32 <= t {
            return Err(n - 1);
        }
        if t <= self.tick[0] as f32 {
            return Err(0);
        }
        let i = self.tick.partition_point(|&k| k as f32 <= t) - 1;
        Ok((i, ps2::div(ps2::sub(t, self.tick[i] as f32), (self.tick[i + 1] - self.tick[i]) as f32)))
    }

    fn squad(&self, t: f32) -> [f32; 4] {
        use crate::quat::micro_slerp;
        match self.find(t) {
            Err(i) => self.value[i],
            Ok((i, u)) if u == 1.0 => self.value[i + 1],
            Ok((i, u)) if u == 0.0 => self.value[i],
            Ok((i, u)) => {
                let a = micro_slerp(self.value[i], self.value[i + 1], u);
                let b = micro_slerp(self.tan[i][0], self.tan[i + 1][0], u);
                micro_slerp(a, b, ps2::mul(ps2::mul(2.0, ps2::sub(1.0, u)), u))
            }
        }
    }

    fn hermite(&self, t: f32) -> [f32; 4] {
        use ps2::{add, madd, msub, mul, sub};
        let (i, u) = match self.find(t) {
            Err(i) => return self.value[i],
            Ok(k) => k,
        };
        let u2 = mul(u, u);
        let u3 = mul(u2, u);
        let h01 = madd(mul(3.0, u2), -2.0, u3);
        let h00 = sub(1.0, h01);
        let h11 = sub(u3, u2);
        let h10 = add(u, msub(add(0.0, u3), 2.0, u2));
        let (v0, v1, m0, m1) = (self.value[i], self.value[i + 1], self.tan[i][1], self.tan[i + 1][0]);
        std::array::from_fn(|c| add(add(add(mul(v0[c], h00), mul(m0[c], h10)), mul(m1[c], h11)), mul(v1[c], h01)))
    }
}

impl Clip {
    /// Bind `anim` to `sk`: tracks with keys whose name is a node, in file order.
    pub fn new(sk: &Skeleton, anim: &Anim) -> Clip {
        let ticks_per_frame = anim.ticks_per_frame as f32;
        let tracks = anim
            .tracks
            .iter()
            .filter(|t| !(t.rotation.is_empty() && t.position.is_empty()))
            .filter_map(|t| {
                let node = sk.names.iter().position(|n| *n == t.name)?;
                let r = sk.rest[node][3];
                let scale = if t.length == 0.0 || t.name == "Bip01Pelvis" {
                    1.0
                } else {
                    let bone = if t.name == "Bip01" { r[1].abs() } else { norm3(r) };
                    ps2::div(bone, t.length)
                };
                Some(ClipTrack { node, scale, rot: Keys::rotation(&t.rotation), pos: Keys::position(&t.position) })
            })
            .collect();
        Clip { ticks_per_frame, length: ps2::div(anim.end_tick() as f32, ticks_per_frame), tracks }
    }

    /// The player's time after setting it to `t` frames (`wrap`).
    pub fn wrap(&self, t: f32, looping: bool) -> f32 {
        wrap(t, self.length, looping)
    }

    /// Track `k` at `t` frames: its rotation quaternion and its scaled position (`None` for an unkeyed list:
    /// the node keeps its rest value).
    pub fn sample(&self, k: usize, t: f32) -> (Option<[f32; 4]>, Option<[f32; 3]>) {
        let tr = &self.tracks[k];
        let tick = ps2::mul(t, self.ticks_per_frame);
        let rot = (!tr.rot.tick.is_empty()).then(|| tr.rot.squad(tick));
        let pos = (!tr.pos.tick.is_empty()).then(|| {
            let p = tr.pos.hermite(tick);
            [0, 1, 2].map(|c| ps2::mul(p[c], tr.scale))
        });
        (rot, pos)
    }

    /// Every node's local matrix at `t` frames (rest where no track keys it).
    pub fn locals(&self, sk: &Skeleton, t: f32) -> Vec<M4> {
        let mut local = sk.rest.clone();
        for k in 0..self.tracks.len() {
            let n = self.tracks[k].node;
            let (rot, pos) = self.sample(k, t);
            if let Some(q) = rot {
                let m = q_matrix(q);
                local[n][..3].copy_from_slice(&m[..3]);
            }
            if let Some([x, y, z]) = pos {
                // w is the interpolated key w (unscaled), not always exactly 1
                let w = self.tracks[k].pos.hermite(ps2::mul(t, self.ticks_per_frame))[3];
                local[n][3] = [x, y, z, w];
            }
        }
        local
    }
}

/// A root path (the `*_dummy` motions: where a reaction carries the player): the first track's position keys,
/// sampled as any position list, unscaled; (0, 0, 0, 1) without keys.
pub struct Path {
    ticks_per_frame: f32,
    keys: Keys,
}

impl Path {
    pub fn new(anim: &Anim) -> Option<Path> {
        let t = anim.tracks.first()?;
        Some(Path { ticks_per_frame: anim.ticks_per_frame as f32, keys: Keys::position(&t.position) })
    }

    /// The path point at `t` frames.
    pub fn at(&self, t: f32) -> [f32; 4] {
        if self.keys.tick.is_empty() {
            return [0.0, 0.0, 0.0, 1.0];
        }
        self.keys.hermite(ps2::mul(t, self.ticks_per_frame))
    }
}

/// Model-space matrices from local ones down the hierarchy.
pub fn model(sk: &Skeleton, local: &[M4]) -> Vec<M4> {
    let mut out: Vec<M4> = Vec::with_capacity(local.len());
    for i in 0..local.len() {
        out.push(match sk.parent[i] {
            Some(p) => mul(&local[i], &out[p]),
            None => local[i],
        });
    }
    out
}

/// The sampler's quaternion → rotation rows (no re-orthonormalisation; the w column and row 3 are left alone).
pub fn q_matrix([x, y, z, w]: [f32; 4]) -> M4 {
    use ps2::{add, mul, sub};
    let (x2, y2, z2) = (mul(2.0, x), mul(2.0, y), mul(2.0, z));
    let (wx, xx, xy, wy) = (mul(w, x2), mul(x, x2), mul(x, y2), mul(w, y2));
    let (zz, wz, yy, xz, yz) = (mul(z, z2), mul(w, z2), mul(y, y2), mul(x, z2), mul(y, z2));
    [
        [sub(1.0, add(yy, zz)), sub(xy, wz), add(xz, wy), 0.0],
        [add(xy, wz), sub(1.0, add(xx, zz)), sub(yz, wx), 0.0],
        [sub(xz, wy), add(yz, wx), sub(1.0, add(xx, yy)), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// A motion player's time after setting it to `t` frames of a clip `len` frames long: wrapped into [0, len) by
/// whole lengths when looping, else clamped to [0, len].
pub fn wrap(t: f32, len: f32, looping: bool) -> f32 {
    if !looping || len == 0.0 {
        return if len < t { len } else if t < 0.0 { 0.0 } else { t };
    }
    let mut t = t;
    while len <= t {
        t = ps2::sub(t, len);
    }
    while t < 0.0 {
        t = ps2::add(t, len);
    }
    t
}

/// The game's VU0 matrix product a·b (row i of the result is row i of `a` transformed by `b`).
pub fn vmul(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|i| crate::vu0::transform(b, a[i]))
}

/// Row 3 of `m`: the translation.
fn row3(m: &M4) -> [f32; 4] {
    m[3]
}

/// The 3×3 transpose of `m` (row 3 and column 3 of the identity): the inverse of a pure rotation.
fn transpose3(m: &M4) -> M4 {
    [[m[0][0], m[1][0], m[2][0], 0.0], [m[0][1], m[1][1], m[2][1], 0.0], [m[0][2], m[1][2], m[2][2], 0.0], [0.0, 0.0, 0.0, 1.0]]
}

/// Rotation by `angle` about `axis` on the FPU (sine first, then cosine; the axis enters negated).
pub fn axis_angle(angle: f32, axis: [f32; 3]) -> M4 {
    let s = libm::sinf(angle);
    let c = libm::cosf(angle);
    axis_rotation(c, s, axis)
}

/// The rotation matrix from cos, sin and the axis, operation for operation.
pub fn axis_rotation(c: f32, s: f32, axis: [f32; 3]) -> M4 {
    use ps2::{add, madd, msub, mul, sub};
    let k = sub(1.0, c);
    let [ax, ay, az] = axis.map(|v| mul(v, -1.0));
    let (xy, zx, yz) = (mul(ax, ay), mul(az, ax), mul(ay, az));
    [
        [madd(add(0.0, c), k, mul(ax, ax)), msub(mul(k, xy), az, s), madd(mul(ay, s), k, zx), 0.0],
        [madd(mul(az, s), k, xy), madd(add(0.0, c), k, mul(ay, ay)), msub(mul(k, yz), ax, s), 0.0],
        [msub(mul(k, zx), ay, s), madd(mul(ax, s), k, yz), madd(add(0.0, c), k, mul(az, az)), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// One stroke's entry of the arm table (motions 0x10–0x19 posed at frame 8).
#[derive(Clone, Debug)]
pub struct ArmPose {
    /// Racket, right hand, right forearm, right upper arm (translation zeroed), left upper arm locals.
    pub racket: M4,
    pub r_hand: M4,
    pub r_forearm: M4,
    pub r_upper: M4,
    pub l_upper: M4,
    /// Model matrices of the upper arms' parents (the right one's translation zeroed).
    pub r_chain: M4,
    pub l_chain: M4,
    /// The right upper arm turned so the racket points along ±z in the body's frame (translation zeroed).
    pub r_upper_aimed: M4,
    /// That turn about y.
    pub yaw: M4,
    /// The shoulder (right upper arm's model translation).
    pub shoulder: [f32; 4],
    /// The racket point's forward reach (model z).
    pub reach: f32,
    /// The racket point's distance off the vertical axis with the elbow straightened.
    pub radius: f32,
}

/// The arm table the game builds at player load: per stroke motion, the pose at frame 8 and the arm's geometry.
pub struct ArmTable {
    /// Strokes 0x10–0x19, then the volleys 0x1a/0x1b (of these the volley solve reads the right arm's locals,
    /// chain and shoulder).
    pub poses: Vec<ArmPose>,
    /// Mean forward reach of the racket point.
    pub reach: f32,
    /// Mean |shoulder x| over the even strokes but 6, plus the hand's offset and 0.3.
    pub side: f32,
    /// The racket hand's model x in stroke 6 (+0.1 for character 5).
    pub hand_x: f32,
}

/// The node names the table reads.
const ARM_NODES: [&str; 5] = ["Racket", "Bip01RHand", "Bip01RForearm", "Bip01RUpperArm", "Bip01LUpperArm"];

/// The arm table from the skeleton and the stroke and volley motions (0x10 … 0x1b) of `character`.
pub fn arm_table(sk: &Skeleton, clips: &[Clip], character: i32) -> ArmTable {
    use ps2::{add, div, madd, msub, mul, sqrt, sub};
    use crate::vu0::transform;
    let idx = ARM_NODES.map(|n| sk.names.iter().position(|x| x == n).unwrap_or_else(|| panic!("no node {n}")));
    let chain = |local: &[M4], n: usize| {
        let mut acc: M4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        let mut p = sk.parent[n];
        while let Some(i) = p {
            acc = vmul(&acc, &local[i]);
            p = sk.parent[i];
        }
        acc
    };
    let tip = [0.0, 0.7, 0.0, 1.0];
    let zero_t = [0.0, 0.0, 0.0, 1.0];
    let (mut reach, mut side, mut hand_x, mut hand_len) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut poses = Vec::with_capacity(clips.len());
    for (i, clip) in clips.iter().enumerate() {
        let local = clip.locals(sk, clip.wrap(8.0, false));
        let [racket, r_hand, r_forearm, mut r_upper, l_upper] = idx.map(|n| local[n]);
        let mut r_chain = chain(&local, idx[3]);
        let l_chain = chain(&local, idx[4]);
        let sign = if i % 2 == 1 { -1.0 } else { 1.0 };
        let p = [racket, r_hand, r_forearm, r_upper, r_chain].iter().fold(tip, |v, m| transform(m, v));
        if i < 10 {
            reach = add(reach, p[2]);
        }
        if i == 0 {
            let t = r_hand[3];
            hand_len = sqrt(madd(madd(mul(t[1], t[1]), t[0], t[0]), t[2], t[2]));
        }
        if i == 6 {
            hand_x = row3(&vmul(&vmul(&vmul(&r_hand, &r_forearm), &r_upper), &r_chain))[0];
        }
        let upper = vmul(&r_upper, &r_chain);
        let sh = row3(&upper);
        if i % 2 == 0 && i != 6 && i < 10 {
            side = add(side, sh[0].abs());
        }
        // turn about y so the racket point lies along ±z from the shoulder
        let (dx, dz) = (sub(p[0], sh[0]), sub(p[2], sh[2]));
        let inv = div(1.0, sqrt(madd(mul(dz, dz), dx, dx)));
        let (nx, nz) = (mul(dx, inv), mul(dz, inv));
        let dot = madd(madd(mul(0.0, 0.0), nx, sign), nz, 0.0);
        let mut angle = libm::acosf(dot.clamp(-1.0, 1.0));
        if msub(mul(nz, sign), nx, 0.0) < 0.0 {
            angle = mul(angle, -1.0);
        }
        let yaw = axis_angle(angle, [0.0, 1.0, 0.0]);
        let mut aimed = vmul(&vmul(&vmul(&upper, &yaw), &axis_angle(f32::from_bits(0x4016_cbe4), [0.0, 0.0, sign])), &transpose3(&r_chain));
        aimed[3] = zero_t;
        r_upper[3] = zero_t;
        r_chain[3] = zero_t;
        // straighten the elbow: turn the forearm about z so the hand lies along the shoulder→elbow line
        let elbow_m = vmul(&vmul(&r_forearm, &aimed), &r_chain);
        let h = row3(&vmul(&vmul(&vmul(&r_hand, &r_forearm), &aimed), &r_chain));
        let e = row3(&elbow_m);
        let d: [f32; 4] = std::array::from_fn(|k| sub(h[k], e[k]));
        let unit = |v: [f32; 4]| {
            let inv = div(1.0, sqrt(madd(madd(mul(v[1], v[1]), v[0], v[0]), v[2], v[2])));
            v.map(|c| mul(c, inv))
        };
        let (n1, n2) = (unit(d), unit(e));
        let dot = madd(madd(mul(n1[1], n2[1]), n1[0], n2[0]), n1[2], n2[2]);
        let mut angle = libm::acosf(dot.clamp(-1.0, 1.0));
        if msub(mul(n1[0], n2[1]), n1[1], n2[0]) < 0.0 {
            angle = mul(angle, -1.0);
        }
        let bend = axis_angle(angle, [0.0, 0.0, 1.0]);
        let frame = vmul(&aimed, &r_chain);
        let forearm = vmul(&vmul(&elbow_m, &bend), &transpose3(&frame));
        let q = [racket, r_hand, forearm, aimed, r_chain].iter().fold(tip, |v, m| transform(m, v));
        let radius = sqrt(madd(mul(q[1], q[1]), q[0], q[0]));
        poses.push(ArmPose { racket, r_hand, r_forearm, r_upper, l_upper, r_chain, l_chain, r_upper_aimed: aimed, yaw, shoulder: sh, reach: p[2], radius });
    }
    let reach = div(reach, 10.0);
    let side = add(div(side, 4.0), add(0.3, hand_len));
    let hand_x = if character == 5 { add(hand_x, 0.1) } else { hand_x };
    ArmTable { poses, reach, side, hand_x }
}

/// Rotation about `axis` from its cosine and the sine's sign (sine = sign·√(1 − c²)).
pub fn axis_cos(c: f32, sign: f32, axis: [f32; 3]) -> M4 {
    use ps2::{add, msub, mul, sqrt};
    axis_rotation(c, mul(sign, sqrt(msub(add(0.0, 1.0), c, c))), axis)
}

/// The racket point in the racket's frame.
const TIP: [f32; 4] = [0.0, 0.7, 0.0, 1.0];

/// What the contact solve leaves for the swing: the body's step (x sideways, z forward; player units) and the
/// turn of right hand, right forearm, right upper arm and left upper arm from the stroke's frame-8 pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArmSolve {
    pub step: [f32; 4],
    pub quats: [[f32; 4]; 4],
}

/// Unit (x, y) of `a − b`.
fn unit_xy(a: [f32; 4], b: [f32; 4]) -> [f32; 2] {
    use ps2::{div, madd, mul, sqrt, sub};
    let (x, y) = (sub(a[0], b[0]), sub(a[1], b[1]));
    let inv = div(1.0, sqrt(madd(mul(y, y), x, x)));
    [mul(x, inv), mul(y, inv)]
}

/// `m` with row 3 of `keep`.
fn keep_row3(m: M4, keep: &M4) -> M4 {
    [m[0], m[1], m[2], keep[3]]
}

/// Reach the contact point with stroke `i`'s arm: the shoulder turns about y to face it, the body steps forward
/// or back what the arm can't reach, the forearm and upper arm bend in turn about z (up to 50 passes, CCD) to
/// bring the racket point onto it, then the hand, with the body stepping sideways half the remaining x once.
/// `contact` and `pos` are world points, `scale` the player's two size factors.
pub fn contact_solve(t: &ArmTable, i: usize, contact: [f32; 4], pos: [f32; 4], scale: [f32; 2]) -> ArmSolve {
    use crate::vu0::transform;
    use ps2::{add, div, madd, msub, mul, sqrt, sub};
    if i >= 10 {
        return volley_solve(&t.poses[i], contact, pos, scale);
    }
    let a = &t.poses[i];
    let sign = if i % 2 == 1 { -1.0 } else { 1.0 };
    let d: [f32; 4] = std::array::from_fn(|k| sub(contact[k], pos[k]));
    let s01 = mul(scale[0], scale[1]);
    let sh = a.shoulder;
    let mut v = [sub(mul(d[0], s01), sh[0]), sub(d[1], sh[1]), sub(mul(d[2], scale[0]), sh[2]), sub(d[3], sh[3])];
    let mut step = [0.0f32; 4];
    if v[2] < 0.0 {
        step[2] = v[2];
    } else if a.reach < v[2] {
        step[2] = sub(v[2], a.reach);
    }
    v[2] = sub(v[2], step[2]);
    // face the point: turn about y taking +z (−z for odd strokes) onto it
    let inv = div(1.0, sqrt(madd(mul(v[2], v[2]), v[0], v[0])));
    let (nx, nz) = (mul(v[0], inv), mul(v[2], inv));
    let dot = madd(madd(mul(0.0, 0.0), nx, sign), nz, 0.0);
    let turn = if msub(mul(nz, sign), nx, 0.0) < 0.0 { -1.0 } else { 1.0 };
    let r0 = axis_cos(dot.clamp(-1.0, 1.0), turn, [0.0, 1.0, 0.0]);
    let mut v = transform(&r0, v);
    let t0 = transpose3(&r0);
    let facing = v;
    let len = sqrt(madd(mul(v[1], v[1]), v[0], v[0]));
    if a.radius < len {
        let f = div(a.radius, len);
        v = v.map(|c| mul(c, f));
    }
    let (chain, mut hand, mut forearm, mut aimed) = (a.r_chain, a.r_hand, a.r_forearm, a.r_upper_aimed);
    let at_hand = transform(&vmul(&a.racket, &hand), TIP);
    let tip = |forearm: &M4, aimed: &M4| transform(&chain, transform(aimed, transform(forearm, at_hand)));
    let dist = |v: [f32; 4], p: [f32; 4]| {
        let (dx, dy) = (sub(v[0], p[0]), sub(v[1], p[1]));
        madd(mul(dy, dy), dx, dx)
    };
    let mut p = tip(&forearm, &aimed);
    let mut elbow = true;
    for _ in 0..50 {
        if dist(v, p) <= f32::from_bits(0x3c23_d70a) {
            break;
        }
        let (m, frame) = if elbow { (vmul(&vmul(&forearm, &aimed), &chain), vmul(&aimed, &chain)) } else { (vmul(&aimed, &chain), chain) };
        let (u, w) = (unit_xy(p, m[3]), unit_xy(v, m[3]));
        let dot = madd(madd(mul(u[1], w[1]), u[0], w[0]), 0.0, 0.0);
        if !(dot < f32::from_bits(0x3f7f_ff58)) {
            break;
        }
        let c = if dot < f32::from_bits(0x3f5d_b3d7) { f32::from_bits(0x3f5d_b3d7) } else { dot };
        let s = if dot < 0.0 {
            -sign
        } else if msub(add(0.0, mul(u[0], w[1])), u[1], w[0]) < 0.0 {
            -1.0
        } else {
            1.0
        };
        let bent = vmul(&vmul(&m, &axis_cos(c, s, [0.0, 0.0, 1.0])), &transpose3(&frame));
        let j = if elbow { &mut forearm } else { &mut aimed };
        *j = keep_row3(bent, j);
        p = tip(&forearm, &aimed);
        elbow = !elbow;
    }
    // the hand, then a sideways step of half what's left (twice at most)
    v = facing;
    let frame = vmul(&vmul(&forearm, &aimed), &chain);
    for k in 0..2 {
        let m = vmul(&hand, &frame);
        let (u, w) = (unit_xy(p, m[3]), unit_xy(v, m[3]));
        let dot = madd(madd(mul(u[1], w[1]), u[0], w[0]), 0.0, 0.0);
        if !(dot < f32::from_bits(0x3f5d_b3d7)) {
            let s = if msub(mul(u[0], w[1]), u[1], w[0]) < 0.0 { -1.0 } else { 1.0 };
            let bent = vmul(&vmul(&m, &axis_cos(dot.clamp(-1.0, 1.0), s, [0.0, 0.0, 1.0])), &transpose3(&frame));
            hand = keep_row3(bent, &hand);
        }
        if k == 1 {
            break;
        }
        p = transform(&frame, transform(&hand, transform(&a.racket, TIP)));
        if v[0].abs() <= p[0].abs() {
            break;
        }
        step[0] = div(sub(v[0], p[0]), 2.0);
        v[0] = sub(v[0], step[0]);
    }
    // back out of the facing turn
    aimed = vmul(&vmul(&vmul(&aimed, &chain), &t0), &transpose3(&chain));
    let l_upper = vmul(&vmul(&vmul(&vmul(&a.l_upper, &a.l_chain), &a.yaw), &t0), &transpose3(&a.l_chain));
    step[0] = mul(step[0], s01);
    step[2] = mul(step[2], scale[0]);
    let q = |pose: &M4, m: &M4| crate::quat::from_matrix(&vmul(&transpose3(pose), m));
    ArmSolve { step, quats: [q(&a.r_hand, &hand), q(&a.r_forearm, &forearm), q(&a.r_upper, &aimed), q(&a.l_upper, &l_upper)] }
}

/// The volley's contact solve: no facing turn or reach clamp; the hand, the upper arm (to 30°) and the hand again
/// each turn about z once to bring the racket point toward the contact point, then the body steps sideways half the
/// x left, the hand turns once more (within 30°), and steps forward or back the z left. Forearm and left arm stay.
fn volley_solve(a: &ArmPose, contact: [f32; 4], pos: [f32; 4], scale: [f32; 2]) -> ArmSolve {
    use crate::vu0::transform;
    use ps2::{div, madd, msub, mul, sub};
    let id: M4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    let d: [f32; 4] = std::array::from_fn(|k| sub(contact[k], pos[k]));
    let s01 = mul(scale[0], scale[1]);
    let sh = a.shoulder;
    let mut v = [sub(mul(d[0], s01), sh[0]), sub(d[1], sh[1]), sub(mul(d[2], scale[0]), sh[2]), sub(d[3], sh[3])];
    let mut m = [a.racket, a.r_hand, a.r_forearm, a.r_upper, a.r_chain];
    let dot_cross = |p: [f32; 4], v: [f32; 4], o: [f32; 4]| {
        let (u, w) = (unit_xy(p, o), unit_xy(v, o));
        (madd(madd(mul(u[1], w[1]), u[0], w[0]), 0.0, 0.0), if msub(mul(u[0], w[1]), u[1], w[0]) < 0.0 { -1.0 } else { 1.0 })
    };
    let turn = |m: &mut [M4; 5], j: usize, frame: &M4, c: f32, s: f32| {
        let bent = vmul(&vmul(&vmul(&m[j], frame), &axis_cos(c, s, [0.0, 0.0, 1.0])), &transpose3(frame));
        m[j] = keep_row3(bent, &m[j]);
    };
    for j in [1, 3, 1] {
        let frame = m[j + 1..].iter().fold(id, |acc, x| vmul(&acc, x));
        let p = transform(&m.iter().fold(id, |acc, x| vmul(&acc, x)), TIP);
        let (dot, s) = dot_cross(p, v, vmul(&m[j], &frame)[3]);
        let c = dot.clamp(-1.0, 1.0);
        if c < f32::from_bits(0x3f7f_ff58) {
            turn(&mut m, j, &frame, if j != 1 && c < f32::from_bits(0x3f5d_b3d7) { f32::from_bits(0x3f5d_b3d7) } else { c }, s);
        }
    }
    let frame = vmul(&vmul(&m[2], &m[3]), &m[4]);
    let tip = |m: &[M4; 5]| transform(&frame, transform(&m[1], transform(&m[0], TIP)));
    let mut p = tip(&m);
    let mut step = [0.0f32; 4];
    if p[0].abs() < v[0].abs() {
        step[0] = div(sub(v[0], p[0]), 2.0);
        v[0] = sub(v[0], step[0]);
        let (dot, s) = dot_cross(p, v, vmul(&m[1], &frame)[3]);
        if !(dot < f32::from_bits(0x3f5d_b3d7)) {
            turn(&mut m, 1, &frame, dot.clamp(-1.0, 1.0), s);
            p = tip(&m);
        }
    }
    step[2] = sub(v[2], p[2]);
    step[0] = mul(step[0], s01);
    step[2] = mul(step[2], scale[0]);
    let q = |pose: &M4, m: &M4| crate::quat::from_matrix(&vmul(&transpose3(pose), m));
    let one = [0.0, 0.0, 0.0, 1.0];
    ArmSolve { step, quats: [q(&a.r_hand, &m[1]), one, q(&a.r_upper, &m[3]), one] }
}

/// The swing's IK frames from the frames left to contact: (ramp length, start offset).
pub fn ik_frames(to_contact: i32) -> (i32, i32) {
    if to_contact < 9 { (to_contact, 0) } else { (8, 8 - to_contact) }
}

/// The contact IK as the play state runs it each frame after the solve: `n` counts toward the contact frame
/// `frames` (negative: still waiting), the arm weight ramps up n/frames, then back down to 0 over half as many
/// frames, when the IK ends (from the contact frame on, the ramp is 8 frames long). While ramping up the body is moved to the solve position plus step·weight, until
/// something else moves it or the mover clamps it.
#[derive(Clone, Copy, Debug)]
pub struct ArmIk {
    pub solve: ArmSolve,
    pub frames: i32,
    pub n: i32,
    pub on: bool,
    /// the body may still be moved by the step
    pub free: bool,
    /// the position at the solve
    pub pos0: [f32; 4],
    /// the position after the last step move
    pub seen: [f32; 4],
}

impl ArmIk {
    pub fn new(solve: ArmSolve, to_contact: i32, pos: [f32; 4]) -> Self {
        let (frames, n) = ik_frames(to_contact);
        ArmIk { solve, frames, n, on: true, free: true, pos0: pos, seen: pos }
    }

    /// One frame: the arm weight, with `pos` moved through `mover(pos, delta) -> (pos, unclamped)`, and whether the
    /// motion's speed goes back to 1 now (the frame before contact).
    pub fn tick(&mut self, pos: &mut [f32; 4], mover: impl FnOnce([f32; 4], [f32; 4]) -> ([f32; 4], bool)) -> (f32, bool) {
        use ps2::{add, div, mul, sub};
        if !self.on {
            return (0.0, false);
        }
        let (n, f) = (self.n, self.frames as f32);
        let mut unit = false;
        let mut w = 0.0;
        if n > self.frames {
            w = sub(1.0, div(mul(2.0, sub(add(n as f32, 0.0), f)), f));
            if w <= 0.0 {
                self.on = false;
                return (0.0, false);
            }
        } else if n >= 0 {
            w = div(add(n as f32, 0.0), f);
            if self.free {
                if self.seen[0] == pos[0] && self.seen[2] == pos[2] {
                    let s = self.solve.step;
                    let d = std::array::from_fn(|k| sub(add(self.pos0[k], mul(s[k], w)), pos[k]));
                    let (p, free) = mover(*pos, d);
                    (*pos, self.free, self.seen) = (p, free, p);
                } else {
                    self.free = false;
                }
            }
            // at the contact frame the ramp down restarts from 8 of 8 (4 frames)
            if w >= 1.0 {
                (self.frames, self.n) = (8, 8);
            } else {
                unit = n + 1 >= self.frames;
            }
        }
        self.n += 1;
        (w, unit)
    }

    /// Joint `j`'s local (right hand, right forearm, right upper arm, left upper arm) turned by weight `w` of its
    /// solved rotation, keeping its translation.
    pub fn apply(&self, j: usize, w: f32, local: &M4) -> M4 {
        let q = crate::quat::slerp([0.0, 0.0, 0.0, 1.0], self.solve.quats[j], w);
        keep_row3(vmul(local, &crate::quat::to_matrix(q)), local)
    }
}
