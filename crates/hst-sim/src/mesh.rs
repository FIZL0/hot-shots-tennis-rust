//! The world mesh the live ball collides with, operation for operation as the game sweeps it: the court model and
//! the grid's props, each a tree of nodes holding triangles; a swept sphere against every triangle whose box it
//! touches (face, then edges, then corners), the nearest hit kept, its ground material read from the material's
//! attribute map at the hit's texture coordinate. Geometry arithmetic on the FPU (`ps2`) and VU0 (`vu0`) exactly
//! where the game uses each.
#![deny(clippy::float_arithmetic)]

use crate::contact::{APPROACH, SKIN, SUNK};
use crate::ps2::{F, f};
use crate::vu0::{self, V4, Vf};
use crate::world::{self, M4};

/// The material id the ball passes through (it only drags the ball); after touching it once in a frame the
/// rest of the frame's sweeps ignore it.
pub const GHOST: u32 = 32;
/// Material of a triangle whose material has no attribute map.
const UNMAPPED: u32 = 4;
/// The attribute map's "no material" id: such a hit is dropped (the ball's hit record defaults to it).
const NO_MATERIAL: u32 = 0;

/// How far outside an edge (m, model space) the sphere's centre may touch a face and still count as inside it.
const EDGE_SLACK: f32 = -0.0005;
/// Where an edge or corner hit starts the step already touching, the ball is pushed out this many radii and the
/// hit reports this penetration (in radii).
const PUSH_OUT: f32 = 0.005;
const PUSH_OUT_PENETRATION: f32 = -PUSH_OUT;
/// A corner already nearer the start than 0.98 radii is skipped: (0.98 r)² as the game stores it, one ulp above
/// 0.9604.
const CORNER_CLEAR: f32 = f32::from_bits(0x3f75_dcc7);
/// After a hit, later triangles are only tested against the step up to 10% past it.
const LOOK_PAST: f32 = 1.1;

/// What a sweep found, in the layout the game's hit record has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Fraction of the step at contact (`f32::MAX` = nothing hit yet).
    pub t: f32,
    /// How far the ball started inside the surface, in radii (≤ 0; set only by hits at t = 0).
    pub penetration: f32,
    /// Ball centre at contact, contact point on the surface, surface normal (towards the ball).
    pub centre: V4,
    pub point: V4,
    pub normal: V4,
    pub material: u32,
}

impl Hit {
    pub const NONE: Hit = Hit { t: f32::MAX, penetration: 0.0, centre: [0.0; 4], point: [0.0; 4], normal: [0.0; 4], material: 0 };

    pub fn contact(&self) -> crate::contact::Hit {
        crate::contact::Hit { t: self.t, penetration: self.penetration, centre: self.centre, normal: self.normal }
    }
}

fn sub3(a: V4, b: V4) -> V4 {
    [(F(a[0]) - b[0]).0, (F(a[1]) - b[1]).0, (F(a[2]) - b[2]).0, 1.0]
}
/// `vsub.xyzw`.
fn vsub(a: V4, b: V4) -> V4 {
    std::array::from_fn(|k| (Vf(a[k]) - b[k]).0)
}
fn sub4(a: V4, b: V4) -> V4 {
    std::array::from_fn(|k| (F(a[k]) - b[k]).0)
}
/// `start + n·k` on the FPU, w included (n.w is 0, the scaled vector's w is 1).
fn push(start: V4, n: V4, k: F) -> V4 {
    let s = [F(n[0]) * k, F(n[1]) * k, F(n[2]) * k, F::ONE];
    std::array::from_fn(|i| (start[i] + s[i]).0)
}

/// Sweep a sphere of radius `r` from `start` to `end` against triangle `v` (one winding; the face is the side
/// `(v1 − v0) × (v2 − v1)` points to). Updates `hit` and returns true when this triangle is nearer than `hit.t`.
pub fn triangle(v: &[V4; 3], hit: &mut Hit, start: V4, end: V4, r: f32) -> bool {
    let r = F(r);
    let one = F::ONE;
    let n = vu0::normalize(vu0::cross(sub3(v[1], v[0]), sub3(v[2], v[1])));
    let skin = f(SKIN) * r;
    let d_end = F(vu0::dot3(n, sub3(end, v[0])));
    if !(d_end <= skin) {
        return false;
    }
    let d_start = F(vu0::dot3(n, sub3(start, v[0])));
    if d_start < -skin {
        return false;
    }
    let travel = d_end - d_start;
    if !(travel <= f(APPROACH) * r) {
        return false;
    }
    if r < d_end && r < d_start {
        return false;
    }
    if !(-r <= d_end) && !(-r <= d_start) {
        return false;
    }
    // the face: the sphere touches the plane inside all three edges
    if !(d_start < f(SUNK) * r) {
        let t = if r < d_start {
            let t = (r - d_start) / travel;
            if t <= 0.0 { F::ZERO } else if t <= one { t } else { one }
        } else {
            F(-1.0)
        };
        if t < hit.t {
            let centre = if 0.0 < t {
                vu0::lerp(end, start, t.0)
            } else if t < 0.0 {
                push(start, n, r - d_start)
            } else {
                start
            };
            let inward = [(-F(n[0])).0, (-F(n[1])).0, (-F(n[2])).0, n[3]];
            let inside = [(0, 2), (1, 0), (2, 1)].iter().all(|&(a, b)| {
                let edge = vsub(v[a], v[b]);
                let to = vsub(v[b], centre);
                !(vu0::dot3(vu0::normalize(vu0::cross(inward, edge)), to) < EDGE_SLACK)
            });
            if inside {
                if t < 0.0 || !(skin <= d_start) {
                    hit.penetration = (-(skin - d_start) / r).0;
                    hit.t = 0.0;
                } else {
                    hit.t = t.0;
                }
                hit.normal = n;
                hit.centre = [centre[0], centre[1], centre[2], 1.0];
                hit.point = [(centre[0] - F(n[0]) * r).0, (centre[1] - F(n[1]) * r).0, (centre[2] - F(n[2]) * r).0, 1.0];
                return true;
            }
        }
    }

    // the edges: the sphere's path against each edge's cylinder
    let m = vsub(end, start);
    let mut found = false;
    for (a, b) in [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
        let e = sub3(b, a);
        let ax = vu0::normalize(vu0::cross(m, e));
        let off = F(vu0::dot3(ax, sub3(start, a)));
        if !(-r <= off) || !(off <= r) {
            continue;
        }
        let reach = (r * r).msub(off, off).sqrt();
        let bx = vu0::normalize(vu0::cross(ax, e));
        let de = sub3(end, a);
        let d_end = (F(bx[1]) * de[1]).madd(bx[0], de[0]).madd(bx[2], de[2]);
        if !(d_end <= reach) {
            continue;
        }
        let ds = sub3(start, a);
        let d_start = (F::ZERO + (F::ZERO + F(bx[1]) * ds[1]).madd(bx[0], ds[0])).madd(bx[2], ds[2]);
        if d_start < f(SUNK) * reach {
            continue;
        }
        let mut t = if d_end == d_start {
            one
        } else {
            let t = (reach - d_start) / (d_end - d_start);
            if t <= one { t } else { one }
        };
        if !(t < hit.t) {
            continue;
        }
        let nn = vu0::normalize([
            (F(ax[0]) * off + F(bx[0]) * reach).0,
            (F(ax[1]) * off + F(bx[1]) * reach).0,
            (F(ax[2]) * off + F(bx[2]) * reach).0,
            one.0,
        ]);
        let centre = if t <= 0.0 { push(start, nn, f(PUSH_OUT) * r) } else { vu0::lerp(end, start, t.0) };
        if vu0::dot3(e, sub3(centre, a)) < 0.0 || !(vu0::dot3(e, sub3(centre, b)) <= 0.0) {
            continue;
        }
        found = true;
        if t <= 0.0 {
            hit.penetration = PUSH_OUT_PENETRATION;
            t = F::ZERO;
        }
        hit.t = t.0;
        hit.normal = nn;
        hit.centre = [centre[0], centre[1], centre[2], 1.0];
        let dir = vu0::normalize(e);
        let k = (F(vu0::dot3(dir, sub3(centre, a))) / vu0::dot3(dir, sub3(b, a))).0;
        let ab = vsub(b, a);
        let p: V4 = std::array::from_fn(|i| (Vf(ab[i]) * k + a[i]).0);
        hit.point = [p[0], p[1], p[2], 1.0];
    }
    if found {
        return true;
    }

    // the corners
    let dir = vu0::normalize(m);
    let r2 = r * r;
    let back = f(PUSH_OUT) * r;
    let clear = f(CORNER_CLEAR) * r2;
    for &c in v {
        let d_end = F(vu0::dot3(dir, sub3(end, c)));
        if d_end < -r {
            continue;
        }
        let ds = sub3(start, c);
        let d_start = F(vu0::dot3(dir, ds));
        if !(d_start <= 0.0) {
            continue;
        }
        if (F::ZERO + (F::ZERO + F(ds[1]) * ds[1]).madd(ds[0], ds[0])).madd(ds[2], ds[2]) < clear {
            continue;
        }
        let lim = if d_end < 0.0 { (F::ZERO + r2).msub(d_end, d_end).sqrt() } else { r };
        let vs = sub3(c, start);
        let along = F(vu0::dot3(vs, dir));
        let p = [vs[0] - F(dir[0]) * along, vs[1] - F(dir[1]) * along, vs[2] - F(dir[2]) * along];
        let miss = (p[1] * p[1]).madd(p[0], p[0]).madd(p[2], p[2]).sqrt();
        if !(miss <= lim) {
            continue;
        }
        let depth = (F::ZERO + r2).msub(miss, miss).sqrt();
        let mut t = if d_end == d_start {
            one
        } else {
            let t = (-depth - d_start) / (d_end - d_start);
            if t <= one { t } else { one }
        };
        if !(t < hit.t) {
            continue;
        }
        found = true;
        hit.point = [c[0], c[1], c[2], 1.0];
        let centre = vu0::lerp(end, start, t.0);
        hit.centre = [centre[0], centre[1], centre[2], 1.0];
        let mut nn = vu0::normalize([(F(centre[0]) - c[0]).0, (F(centre[1]) - c[1]).0, (F(centre[2]) - c[2]).0, 0.0]);
        nn[3] = 0.0;
        hit.normal = nn;
        if t <= 0.0 {
            t = F::ZERO;
            let p = push(start, nn, back);
            hit.centre = [p[0], p[1], p[2], 1.0];
            hit.penetration = PUSH_OUT_PENETRATION;
        }
        hit.t = t.0;
    }
    found
}

/// The segment `start` → `end` against triangle `v` (one winding): crossing its plane from the face side inside all
/// three edges (0.0005 slack). Updates `hit` (t, centre = point on the segment, normal) and returns true when nearer
/// than `hit.t`.
pub fn ray_triangle(v: &[V4; 3], hit: &mut Hit, start: V4, end: V4) -> bool {
    let d = sub4(end, start);
    for (a, b) in [(v[0], v[2]), (v[1], v[0]), (v[2], v[1])] {
        let side = vu0::normalize(vu0::cross(d, vsub(a, b)));
        if vu0::dot3(side, vsub(b, start)) < EDGE_SLACK {
            return false;
        }
    }
    let n = vu0::cross(sub3(v[1], v[0]), sub3(v[2], v[1]));
    let de = vu0::dot3(n, sub3(end, v[0]));
    if !(de <= 0.0) {
        return false;
    }
    let ds = vu0::dot3(n, sub3(start, v[0]));
    if ds < -0.0 {
        return false;
    }
    let t = if de < ds { let t = (F(ds) / (F(ds) - de)).0; if t <= 1.0 { t } else { 1.0 } } else { 1.0 };
    if hit.t <= t {
        return false;
    }
    hit.t = t;
    hit.centre = vu0::lerp(end, start, t);
    hit.centre[3] = 1.0;
    hit.point = hit.centre;
    hit.normal = vu0::normalize(n);
    true
}

/// A material's attribute map: a 4-bit texture whose texels pick ground material ids through `table`.
#[derive(Clone, Debug)]
pub struct AttributeMap {
    pub width: i32,
    pub height: i32,
    /// Texels per row as stored.
    pub stride: i32,
    pub texels: Vec<u8>,
    pub table: [u8; 16],
}

impl AttributeMap {
    /// The 4-bit texel at column `u`, row `w`: two to a byte, the even column in the low nibble. The byte index
    /// halves the texel index rounding toward zero, as the game's signed divide does.
    fn texel(&self, u: i32, w: i32) -> u8 {
        let i = u + w * self.stride;
        let i = if i < 0 { i + 1 } else { i } >> 1;
        // ponytail: a texel outside the map reads memory past it in the game; never seen, treated as id 0
        let byte = usize::try_from(i).ok().and_then(|i| self.texels.get(i)).copied().unwrap_or(0);
        if u & 1 != 0 { byte >> 4 } else { byte & 0xf }
    }
}

#[derive(Clone, Debug)]
pub struct Material {
    /// Collides from both sides.
    pub two_sided: bool,
    /// Has a texture: the texture coordinate is used (otherwise texel 0).
    pub textured: bool,
    /// Wrap mode along u and v.
    pub wrap: [Wrap; 2],
    pub map: Option<AttributeMap>,
}

/// How a texture coordinate outside 0..1 finds its texel (the GS's wrap modes, as the model file numbers them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    Repeat,
    Clamp,
    RegionClamp,
    RegionRepeat,
}

impl Wrap {
    /// The model file's mode number (0..3; any other number reads as region clamp).
    pub fn from_mode(mode: u8) -> Wrap {
        match mode {
            0 => Wrap::Repeat,
            1 => Wrap::Clamp,
            3 => Wrap::RegionRepeat,
            _ => Wrap::RegionClamp,
        }
    }
}

/// A texture coordinate (0..1 across the texture) to a texel index along an axis of `size` texels.
fn texel(f: f32, size: i32, mode: Wrap) -> i32 {
    let x = (F(f) * size as f32).0;
    let n = size as f64;
    // the game's libm fmod: double precision, not the FPU
    #[allow(clippy::float_arithmetic)]
    let fold = |x: f32| -> i32 {
        // fmod in double precision, then back to single
        let r = if x < 0.0 { n - (-x as f64) % n } else { x as f64 % n };
        let v = r as f32 as i32;
        if v == size { size - 1 } else { v }
    };
    match mode {
        Wrap::Repeat => fold(x),
        Wrap::RegionRepeat => size, // with the game's (0, size) region: always the region's upper bound
        Wrap::Clamp => (if x <= 0.0 { 0 } else { x as i32 }).min(size - 1),
        Wrap::RegionClamp => (if x <= 0.0 { 0 } else { x as i32 } as u32).min((size - 1) as u32) as i32,
    }
}

/// Length of the xy part: √(y·y + x·x).
fn len2(v: V4) -> f32 {
    (F(v[1]) * v[1]).madd(v[0], v[0]).sqrt().0
}

/// The ground material under a hit's contact point: the triangle `v` (model space, in the order it was swept)
/// and its texture coordinates `uv` give the attribute-map texel. Projects the triangle onto its plane, finds
/// where the line from the corner farthest from the point (roughly) through the point meets the opposite edge,
/// and interpolates the coordinates along that edge and then towards the corner.
pub fn material(point: V4, v: &[V4; 3], uv: &[V4; 3], mat: &Material) -> u32 {
    let Some(map) = &mat.map else { return UNMAPPED };
    let (u, w) = if mat.textured {
        let dist = |c: V4| {
            let d = sub4(c, point);
            (F(d[1]) * d[1]).madd(d[0], d[0]).madd(d[2], d[2])
        };
        let mut idx = [0usize, 1, 2];
        let (d1, d2) = (dist(v[1]), dist(v[2]));
        let mut far = dist(v[0]);
        if far < d1 {
            idx.swap(0, 1);
            far = d1;
        }
        if far < d2 {
            idx.swap(0, 2);
        }
        let rel = |c: V4| {
            let mut d = sub4(c, v[0]);
            d[3] = 1.0;
            d
        };
        // a basis in the triangle's plane: z along the normal
        let r2 = vu0::normalize({
            let c = vu0::cross(sub3(v[1], v[0]), sub3(v[2], v[1]));
            [c[0], c[1], c[2], 0.0]
        });
        let r1 = vu0::normalize(vu0::cross(r2, sub4(v[1], v[0])));
        let r0 = vu0::normalize(vu0::cross(r1, r2));
        let t: M4 = [[r0[0], r1[0], r2[0], 0.0], [r0[1], r1[1], r2[1], 0.0], [r0[2], r1[2], r2[2], 0.0], [0.0, 0.0, 0.0, 1.0]];
        let flat = |c: V4| vu0::transform(&t, c);
        let (a, b, c, p) = (flat(rel(v[idx[0]])), flat(rel(v[idx[1]])), flat(rel(v[idx[2]])), flat(rel(point)));
        // where the line a → p meets the line c → b
        let (ay, ax) = (F(a[1]) - p[1], F(a[0]) - p[0]);
        let (cy, cx) = (F(c[1]) - b[1], F(c[0]) - b[0]);
        let det = (ax * cy).msub(ay, cx);
        let q = if det == 0.0 {
            [F(c[0]), F(c[1])]
        } else {
            let s = (F::ZERO + cx * (F(p[1]) - b[1])).msub(cy, F(p[0]) - b[0]) / det;
            [(F::ZERO + p[0]).madd(ax, s), (F::ZERO + p[1]).madd(ay, s)]
        };
        let clamp = |s: F| {
            let s = if 0.0 <= s { s } else { F::ZERO };
            if s <= 1.0 { s } else { F::ONE }
        };
        let lerp = |a: V4, b: V4, s: F| -> V4 { std::array::from_fn(|k| ((Vf(a[k]) - b[k]) * s.0 + b[k]).0) };
        let xyz = |c: V4| [c[0], c[1], c[2], 0.0];
        let s1 = clamp(F(len2([(q[0] - b[0]).0, (q[1] - b[1]).0, 0.0, 0.0])) / len2(sub4(c, b)));
        let edge = lerp(xyz(uv[idx[2]]), xyz(uv[idx[1]]), s1);
        let s2 = clamp(F(len2(sub4(p, a))) / len2([(q[0] - a[0]).0, (q[1] - a[1]).0, 0.0, 0.0]));
        let tex = lerp(edge, xyz(uv[idx[0]]), s2);
        let k = 1.0 / F(tex[2]);
        (texel((tex[0] * k).0, map.width, mat.wrap[0]), texel((tex[1] * k).0, map.height, mat.wrap[1]))
    } else {
        (0, 0)
    };
    map.table[map.texel(u, w) as usize] as u32
}

/// A collision triangle in model space.
#[derive(Clone, Debug)]
pub struct Tri {
    /// Pre-order node index.
    pub node: usize,
    pub material: usize,
    pub pos: [V4; 3],
    pub uv: [V4; 3],
    /// Box (min, max) the sweep's box must overlap for the triangle to be tested.
    pub bounds: [V4; 2],
}

impl Tri {
    /// Stored the other way round: the face is on the side of `pos` in reverse order. The model file flags it with
    /// the sign of the third texture coordinate's w (negative, or NaN).
    pub fn reversed(&self) -> bool {
        !(0.0 <= self.uv[2][3])
    }
}

#[derive(Clone, Debug)]
pub struct Model {
    /// In the order the game tests them (node pre-order, then material, then file order).
    pub tris: Vec<Tri>,
    pub materials: Vec<Material>,
    /// Per node, model space → node space.
    pub node_from_model: Vec<M4>,
}

/// A model placed in the world.
#[derive(Clone, Debug)]
pub struct Object {
    pub model: usize,
    pub scale: f32,
    /// World → model space.
    pub to_model: M4,
    /// Per node, node space → world (the node's drawing matrix).
    pub nodes: Vec<M4>,
    /// Bounding sphere in world space (centre) and the model's own radius (unscaled).
    pub center: V4,
    pub radius: f32,
}

/// The sweep's box: both ends' spheres.
fn sweep_box(start: V4, rs: f32, end: V4, re: f32) -> [V4; 2] {
    let mut b = [[0.0, 0.0, 0.0, 1.0]; 2];
    for k in 0..3 {
        if start[k] < end[k] {
            b[0][k] = (F(start[k]) - rs).0;
            b[1][k] = (F(end[k]) + re).0;
        } else {
            b[0][k] = (F(end[k]) - re).0;
            b[1][k] = (F(start[k]) + rs).0;
        }
    }
    b
}

/// The VU0 box test: overlap on every axis, touching does not count.
fn overlaps(a: &[V4; 2], b: &[V4; 2]) -> bool {
    (0..3).all(|k| a[1][k] > b[0][k] && b[1][k] > a[0][k])
}

impl Object {
    /// Sweep against this object; true when it produced the nearest hit so far (`hit` updated, in world space).
    /// `ignore`: materials that do not collide.
    // ponytail: the game also skips whole nodes, batches and packets by their boxes first; each holds its
    // triangles' boxes, so the triangle test alone keeps the same set.
    pub fn sweep(&self, models: &[Model], hit: &mut Hit, start: V4, end: V4, r: f32, ignore: &[u32]) -> bool {
        self.cast(models, hit, start, end, r, ignore, triangle)
    }

    /// The nearest hit of the segment `start` → `end` with this object (the game's ground ray): like [`Self::sweep`]
    /// with no radius, each triangle met by [`ray_triangle`].
    pub fn ray(&self, models: &[Model], hit: &mut Hit, start: V4, end: V4) -> bool {
        self.cast(models, hit, start, end, 0.0, &[], |v, hit, s, e, _| ray_triangle(v, hit, s, e))
    }

    #[allow(clippy::too_many_arguments)]
    fn cast(&self, models: &[Model], hit: &mut Hit, start: V4, end: V4, r: f32, ignore: &[u32], test: impl Fn(&[V4; 3], &mut Hit, V4, V4, f32) -> bool) -> bool {
        let model = &models[self.model];
        let (mut rs, mut re) = (r, r);
        let start = vu0::transform(&self.to_model, [start[0], start[1], start[2], 1.0]);
        let end = vu0::transform(&self.to_model, [end[0], end[1], end[2], 1.0]);
        if self.scale != 1.0 {
            let k = 1.0 / F(self.scale);
            rs = (F(rs) * k).0;
            re = (F(re) * k).0;
        }
        let mut bounds = sweep_box(start, rs, end, re);
        let mut any = false;
        for tri in &model.tris {
            if !overlaps(&bounds, &tri.bounds) {
                continue;
            }
            let mat = &model.materials[tri.material];
            let local = &model.node_from_model[tri.node];
            let (s, e) = (vu0::transform(local, start), vu0::transform(local, end));
            let mut uv = tri.uv;
            let mut reversed = tri.reversed();
            for _ in 0..if mat.two_sided { 2 } else { 1 } {
                let v = if reversed { [tri.pos[2], tri.pos[1], tri.pos[0]] } else { tri.pos };
                let before = *hit;
                if test(&v, hit, s, e, rs) {
                    if reversed {
                        uv.swap(0, 2);
                    }
                    let id = material(hit.point, &v, &uv, mat);
                    if id == NO_MATERIAL || ignore.contains(&id) {
                        *hit = before;
                    } else {
                        any = true;
                        let node = &self.nodes[tri.node];
                        hit.material = id;
                        hit.centre = vu0::transform(node, hit.centre);
                        hit.point = vu0::transform(node, hit.point);
                        hit.normal = vu0::normalize(vu0::transform(node, [hit.normal[0], hit.normal[1], hit.normal[2], 0.0]));
                        if 0.0 < hit.t {
                            let ahead = f(LOOK_PAST) * hit.t;
                            let ahead = if ahead <= 1.0 { ahead } else { F::ONE };
                            bounds = sweep_box(start, rs, vu0::lerp(end, start, ahead.0), re);
                        }
                    }
                }
                reversed = !reversed;
            }
        }
        any
    }
}

/// The collision world: the court object, the props and the grid they are filed in.
#[derive(Clone, Debug)]
pub struct World {
    pub models: Vec<Model>,
    pub court: Object,
    /// Props in the game's list order; the grid lists indices into this.
    pub props: Vec<Object>,
    pub grid: world::Grid,
}

impl World {
    /// The nearest hit of a sphere of radius `r` moving from `start` to `end`: the court first, then every prop in
    /// the grid cells around the move whose bounding sphere it can reach.
    pub fn sweep(&self, start: V4, end: V4, r: f32, ignore: &[u32]) -> Option<Hit> {
        let mut hit = Hit::NONE;
        self.court.sweep(&self.models, &mut hit, start, end, r, ignore);

        let half = 0.5;
        let grow = 1.1; // the reach and the move's box, 10% larger
        let mid: V4 = std::array::from_fn(|k| ((F(start[k]) + end[k]) * half).0);
        let d = sub4(end, start);
        let reach = grow * (F(r) + (F(d[0]) * d[0] + F(d[1]) * d[1]).madd(d[2], d[2]).sqrt() / 2.0);
        // the move's box, grown by 10% about its centre at the low corner, then at the high corner
        let (s, e) = (start.map(F), end.map(F));
        let mut lo: V4 = std::array::from_fn(|k| if k < 3 { if e[k] - r < s[k] - r { (e[k] - r).0 } else { (s[k] - r).0 } } else { 1.0 });
        let mut hi: V4 = std::array::from_fn(|k| if k < 3 { if s[k] + r < e[k] + r { (e[k] + r).0 } else { (s[k] + r).0 } } else { 1.0 });
        let c: V4 = std::array::from_fn(|k| ((F(lo[k]) + hi[k]) * half).0);
        lo = std::array::from_fn(|k| ((F(lo[k]) - c[k]) * grow + c[k]).0);
        let c: V4 = std::array::from_fn(|k| ((F(lo[k]) + hi[k]) * half).0);
        hi = std::array::from_fn(|k| ((F(hi[k]) - c[k]) * grow + c[k]).0);
        let (a, b): ([i32; 3], [i32; 3]) = (std::array::from_fn(|k| world::cell_of(lo[k])), std::array::from_fn(|k| world::cell_of(hi[k])));
        let g = &self.grid;
        if (0..3).any(|k| b[k] < g.lo[k] || g.hi[k] < a[k]) {
            return (hit.t != f32::MAX).then_some(hit);
        }
        let mut seen = Vec::new();
        for x in a[0].clamp(g.lo[0], g.hi[0])..=b[0].clamp(g.lo[0], g.hi[0]) {
            for y in a[1].clamp(g.lo[1], g.hi[1])..=b[1].clamp(g.lo[1], g.hi[1]) {
                for z in a[2].clamp(g.lo[2], g.hi[2])..=b[2].clamp(g.lo[2], g.hi[2]) {
                    for &id in g.cell([x, y, z]) {
                        if seen.contains(&id) {
                            continue;
                        }
                        seen.push(id);
                        let p = &self.props[id];
                        let rr = (F::ZERO + reach).madd(p.scale, p.radius);
                        if vu0::dot3(sub4(mid, p.center), sub4(mid, p.center)) <= rr * rr {
                            p.sweep(&self.models, &mut hit, start, end, r, ignore);
                        }
                    }
                }
            }
        }
        (hit.t != f32::MAX).then_some(hit)
    }
}
