//! The world mesh the live ball collides with, operation for operation as the game sweeps it: the court model and
//! the grid's props, each a tree of nodes holding triangles; a swept sphere against every triangle whose box it
//! touches (face, then edges, then corners), the nearest hit kept, its ground material read from the material's
//! attribute map at the hit's texture coordinate. Geometry arithmetic on the FPU (`ps2`) and VU0 (`vu0`) exactly
//! where the game uses each.

use crate::ps2;
use crate::vu0::{self, V4};
use crate::world::{self, M4};

/// The material id the ball passes through (it only drags the ball); after touching it once in a frame the
/// rest of the frame's sweeps ignore it.
pub const GHOST: u32 = 32;
/// Material of a triangle whose material has no attribute map.
const UNMAPPED: u32 = 4;

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
    [ps2::sub(a[0], b[0]), ps2::sub(a[1], b[1]), ps2::sub(a[2], b[2]), 1.0]
}
/// `vsub.xyzw`.
fn vsub(a: V4, b: V4) -> V4 {
    std::array::from_fn(|k| vu0::sub(a[k], b[k]))
}
fn sub4(a: V4, b: V4) -> V4 {
    std::array::from_fn(|k| ps2::sub(a[k], b[k]))
}
/// `start + n·k` on the FPU, w included (n.w is 0, the scaled vector's w is 1).
fn push(start: V4, n: V4, k: f32) -> V4 {
    let s = [ps2::mul(n[0], k), ps2::mul(n[1], k), ps2::mul(n[2], k), 1.0];
    std::array::from_fn(|i| ps2::add(start[i], s[i]))
}

/// Sweep a sphere of radius `r` from `start` to `end` against triangle `v` (one winding; the face is the side
/// `(v1 − v0) × (v2 − v1)` points to). Updates `hit` and returns true when this triangle is nearer than `hit.t`.
pub fn triangle(v: &[V4; 3], hit: &mut Hit, start: V4, end: V4, r: f32) -> bool {
    use ps2::{add, div, madd, msub, mul, sqrt, sub};
    let one = 1.0;
    let n = vu0::normalize(vu0::cross(sub3(v[1], v[0]), sub3(v[2], v[1])));
    let skin = mul(f32::from_bits(0x3f80_a3d7), r); // 1.005
    let d_end = vu0::dot3(n, sub3(end, v[0]));
    if !(d_end <= skin) {
        return false;
    }
    let d_start = vu0::dot3(n, sub3(start, v[0]));
    if d_start < -skin {
        return false;
    }
    let travel = sub(d_end, d_start);
    if !(travel <= mul(f32::from_bits(0x3ba3_d70a), r)) {
        return false;
    }
    if r < d_end && r < d_start {
        return false;
    }
    if !(-r <= d_end) && !(-r <= d_start) {
        return false;
    }
    // the face: the sphere touches the plane inside all three edges
    if !(d_start < mul(f32::from_bits(0x3f7a_e148), r)) {
        let t = if r < d_start {
            let t = div(sub(r, d_start), travel);
            if t <= 0.0 { 0.0 } else if t <= one { t } else { one }
        } else {
            -1.0
        };
        if t < hit.t {
            let centre = if 0.0 < t {
                vu0::lerp(end, start, t)
            } else if t < 0.0 {
                push(start, n, sub(r, d_start))
            } else {
                start
            };
            let inward = [-n[0], -n[1], -n[2], n[3]];
            let inside = [(0, 2), (1, 0), (2, 1)].iter().all(|&(a, b)| {
                let edge = vsub(v[a], v[b]);
                let to = vsub(v[b], centre);
                !(vu0::dot3(vu0::normalize(vu0::cross(inward, edge)), to) < f32::from_bits(0xba03_126f)) // −0.0005
            });
            if inside {
                if t < 0.0 || !(skin <= d_start) {
                    hit.penetration = div(-sub(skin, d_start), r);
                    hit.t = 0.0;
                } else {
                    hit.t = t;
                }
                hit.normal = n;
                hit.centre = [centre[0], centre[1], centre[2], 1.0];
                hit.point = [sub(centre[0], mul(n[0], r)), sub(centre[1], mul(n[1], r)), sub(centre[2], mul(n[2], r)), 1.0];
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
        let off = vu0::dot3(ax, sub3(start, a));
        if !(-r <= off) || !(off <= r) {
            continue;
        }
        let reach = sqrt(msub(mul(r, r), off, off));
        let bx = vu0::normalize(vu0::cross(ax, e));
        let de = sub3(end, a);
        let d_end = madd(madd(mul(bx[1], de[1]), bx[0], de[0]), bx[2], de[2]);
        if !(d_end <= reach) {
            continue;
        }
        let ds = sub3(start, a);
        let d_start = madd(add(0.0, madd(add(0.0, mul(bx[1], ds[1])), bx[0], ds[0])), bx[2], ds[2]);
        if d_start < mul(f32::from_bits(0x3f7a_e148), reach) {
            continue;
        }
        let mut t = if d_end == d_start {
            one
        } else {
            let t = div(sub(reach, d_start), sub(d_end, d_start));
            if t <= one { t } else { one }
        };
        if !(t < hit.t) {
            continue;
        }
        let nn = vu0::normalize([
            add(mul(ax[0], off), mul(bx[0], reach)),
            add(mul(ax[1], off), mul(bx[1], reach)),
            add(mul(ax[2], off), mul(bx[2], reach)),
            one,
        ]);
        let centre = if t <= 0.0 { push(start, nn, mul(f32::from_bits(0x3ba3_d70a), r)) } else { vu0::lerp(end, start, t) };
        if vu0::dot3(e, sub3(centre, a)) < 0.0 || !(vu0::dot3(e, sub3(centre, b)) <= 0.0) {
            continue;
        }
        found = true;
        if t <= 0.0 {
            hit.penetration = f32::from_bits(0xbba3_d70a); // −0.005
            t = 0.0;
        }
        hit.t = t;
        hit.normal = nn;
        hit.centre = [centre[0], centre[1], centre[2], 1.0];
        let dir = vu0::normalize(e);
        let k = div(vu0::dot3(dir, sub3(centre, a)), vu0::dot3(dir, sub3(b, a)));
        let ab = vsub(b, a);
        let p: V4 = std::array::from_fn(|i| vu0::add(vu0::mul(ab[i], k), a[i]));
        hit.point = [p[0], p[1], p[2], 1.0];
    }
    if found {
        return true;
    }

    // the corners
    let dir = vu0::normalize(m);
    let r2 = mul(r, r);
    let back = mul(f32::from_bits(0x3ba3_d70a), r);
    let clear = mul(f32::from_bits(0x3f75_dcc7), r2); // 0.9604 r² = (0.98 r)²
    for &c in v {
        let d_end = vu0::dot3(dir, sub3(end, c));
        if d_end < -r {
            continue;
        }
        let ds = sub3(start, c);
        let d_start = vu0::dot3(dir, ds);
        if !(d_start <= 0.0) {
            continue;
        }
        if madd(add(0.0, madd(add(0.0, mul(ds[1], ds[1])), ds[0], ds[0])), ds[2], ds[2]) < clear {
            continue;
        }
        let lim = if d_end < 0.0 { sqrt(msub(add(0.0, r2), d_end, d_end)) } else { r };
        let vs = sub3(c, start);
        let along = vu0::dot3(vs, dir);
        let p = [sub(vs[0], mul(dir[0], along)), sub(vs[1], mul(dir[1], along)), sub(vs[2], mul(dir[2], along))];
        let miss = sqrt(madd(madd(mul(p[1], p[1]), p[0], p[0]), p[2], p[2]));
        if !(miss <= lim) {
            continue;
        }
        let depth = sqrt(msub(add(0.0, r2), miss, miss));
        let mut t = if d_end == d_start {
            one
        } else {
            let t = div(sub(-depth, d_start), sub(d_end, d_start));
            if t <= one { t } else { one }
        };
        if !(t < hit.t) {
            continue;
        }
        found = true;
        hit.point = [c[0], c[1], c[2], 1.0];
        let centre = vu0::lerp(end, start, t);
        hit.centre = [centre[0], centre[1], centre[2], 1.0];
        let mut nn = vu0::normalize([sub(centre[0], c[0]), sub(centre[1], c[1]), sub(centre[2], c[2]), 0.0]);
        nn[3] = 0.0;
        hit.normal = nn;
        if t <= 0.0 {
            t = 0.0;
            let p = push(start, nn, back);
            hit.centre = [p[0], p[1], p[2], 1.0];
            hit.penetration = f32::from_bits(0xbba3_d70a);
        }
        hit.t = t;
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
        if vu0::dot3(side, vsub(b, start)) < f32::from_bits(0xba03_126f) {
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
    let t = if de < ds { let t = ps2::div(ds, ps2::sub(ds, de)); if t <= 1.0 { t } else { 1.0 } } else { 1.0 };
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

#[derive(Clone, Debug)]
pub struct Material {
    /// Collides from both sides.
    pub two_sided: bool,
    /// Has a texture: the texture coordinate is used (otherwise texel 0).
    pub textured: bool,
    /// Wrap mode along u and v (0 repeat, 1 clamp, 2 region clamp, 3 region repeat).
    pub wrap: [u8; 2],
    pub map: Option<AttributeMap>,
}

/// A texture coordinate (0..1 across the texture) to a texel index along an axis of `size` texels.
fn texel(f: f32, size: i32, mode: u8) -> i32 {
    let x = ps2::mul(f, size as f32);
    let n = size as f64;
    let fold = |x: f32| -> i32 {
        // fmod in double precision, then back to single
        let r = if x < 0.0 { n - (-x as f64) % n } else { x as f64 % n };
        let v = r as f32 as i32;
        if v == size { size - 1 } else { v }
    };
    match mode {
        0 => fold(x),
        3 => size, // region repeat with the game's (0, size) region: always the region's upper bound
        1 => (if x <= 0.0 { 0 } else { x as i32 }).min(size - 1),
        _ => (if x <= 0.0 { 0 } else { x as i32 } as u32).min((size - 1) as u32) as i32,
    }
}

/// Length of the xy part: √(y·y + x·x).
fn len2(v: V4) -> f32 {
    ps2::sqrt(ps2::madd(ps2::mul(v[1], v[1]), v[0], v[0]))
}

/// The ground material under a hit's contact point: the triangle `v` (model space, in the order it was swept)
/// and its texture coordinates `uv` give the attribute-map texel. Projects the triangle onto its plane, finds
/// where the line from the corner farthest from the point (roughly) through the point meets the opposite edge,
/// and interpolates the coordinates along that edge and then towards the corner.
pub fn material(point: V4, v: &[V4; 3], uv: &[V4; 3], mat: &Material) -> u32 {
    use ps2::{div, madd, msub, mul, sub};
    let Some(map) = &mat.map else { return UNMAPPED };
    let (u, w) = if mat.textured {
        let dist = |c: V4| {
            let d = sub4(c, point);
            madd(madd(mul(d[1], d[1]), d[0], d[0]), d[2], d[2])
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
        let (ay, ax) = (sub(a[1], p[1]), sub(a[0], p[0]));
        let (cy, cx) = (sub(c[1], b[1]), sub(c[0], b[0]));
        let det = msub(mul(ax, cy), ay, cx);
        let q = if det == 0.0 {
            [c[0], c[1]]
        } else {
            let s = div(msub(ps2::add(0.0, mul(cx, sub(p[1], b[1]))), cy, sub(p[0], b[0])), det);
            [madd(ps2::add(0.0, p[0]), ax, s), madd(ps2::add(0.0, p[1]), ay, s)]
        };
        let clamp = |s: f32| {
            let s = if 0.0 <= s { s } else { 0.0 };
            if s <= 1.0 { s } else { 1.0 }
        };
        let lerp = |a: V4, b: V4, s: f32| -> V4 { std::array::from_fn(|k| vu0::add(vu0::mul(vu0::sub(a[k], b[k]), s), b[k])) };
        let xyz = |c: V4| [c[0], c[1], c[2], 0.0];
        let s1 = clamp(div(len2([sub(q[0], b[0]), sub(q[1], b[1]), 0.0, 0.0]), len2(sub4(c, b))));
        let edge = lerp(xyz(uv[idx[2]]), xyz(uv[idx[1]]), s1);
        let s2 = clamp(div(len2(sub4(p, a)), len2([sub(q[0], a[0]), sub(q[1], a[1]), 0.0, 0.0])));
        let f = lerp(edge, xyz(uv[idx[0]]), s2);
        let k = div(1.0, f[2]);
        (texel(mul(f[0], k), map.width, mat.wrap[0]), texel(mul(f[1], k), map.height, mat.wrap[1]))
    } else {
        (0, 0)
    };
    let i = u + w * map.stride;
    let i = if i < 0 { i + 1 } else { i } >> 1;
    // ponytail: a texel outside the map reads memory past it in the game; never seen, treated as id 0
    let byte = usize::try_from(i).ok().and_then(|i| map.texels.get(i)).copied().unwrap_or(0) as i32;
    let nibble = if u & 1 != 0 { byte >> 4 } else { byte };
    map.table[(nibble & 0xf) as usize] as u32
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
            b[0][k] = ps2::sub(start[k], rs);
            b[1][k] = ps2::add(end[k], re);
        } else {
            b[0][k] = ps2::sub(end[k], re);
            b[1][k] = ps2::add(start[k], rs);
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
            let k = ps2::div(1.0, self.scale);
            rs = ps2::mul(rs, k);
            re = ps2::mul(re, k);
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
            let mut reversed = !(0.0 <= tri.uv[2][3]);
            for _ in 0..if mat.two_sided { 2 } else { 1 } {
                let v = if reversed { [tri.pos[2], tri.pos[1], tri.pos[0]] } else { tri.pos };
                let before = *hit;
                if test(&v, hit, s, e, rs) {
                    if reversed {
                        uv.swap(0, 2);
                    }
                    // id 0 falls back to the hit record's default material, which is 0 for the ball
                    let id = material(hit.point, &v, &uv, mat);
                    if id == 0 || ignore.contains(&id) {
                        *hit = before;
                    } else {
                        any = true;
                        let node = &self.nodes[tri.node];
                        hit.material = id;
                        hit.centre = vu0::transform(node, hit.centre);
                        hit.point = vu0::transform(node, hit.point);
                        hit.normal = vu0::normalize(vu0::transform(node, [hit.normal[0], hit.normal[1], hit.normal[2], 0.0]));
                        if 0.0 < hit.t {
                            let f = ps2::mul(f32::from_bits(0x3f8c_cccd), hit.t); // 1.1
                            let f = if f <= 1.0 { f } else { 1.0 };
                            bounds = sweep_box(start, rs, vu0::lerp(end, start, f), re);
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
        use ps2::{add, div, madd, mul, sqrt, sub};
        let mut hit = Hit::NONE;
        self.court.sweep(&self.models, &mut hit, start, end, r, ignore);

        let half = f32::from_bits(0x3f00_0000);
        let grow = f32::from_bits(0x3f8c_cccd); // 1.1
        let mid: V4 = std::array::from_fn(|k| mul(add(start[k], end[k]), half));
        let d = sub4(end, start);
        let reach = mul(grow, add(r, div(sqrt(madd(add(mul(d[0], d[0]), mul(d[1], d[1])), d[2], d[2])), 2.0)));
        // the move's box, grown by 10% about its centre at the low corner, then at the high corner
        let mut lo: V4 = std::array::from_fn(|k| if k < 3 { if sub(end[k], r) < sub(start[k], r) { sub(end[k], r) } else { sub(start[k], r) } } else { 1.0 });
        let mut hi: V4 = std::array::from_fn(|k| if k < 3 { if add(start[k], r) < add(end[k], r) { add(end[k], r) } else { add(start[k], r) } } else { 1.0 });
        let c: V4 = std::array::from_fn(|k| mul(add(lo[k], hi[k]), half));
        lo = std::array::from_fn(|k| add(mul(sub(lo[k], c[k]), grow), c[k]));
        let c: V4 = std::array::from_fn(|k| mul(add(lo[k], hi[k]), half));
        hi = std::array::from_fn(|k| add(mul(sub(hi[k], c[k]), grow), c[k]));
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
                        let rr = madd(add(0.0, reach), p.scale, p.radius);
                        if vu0::dot3(sub4(mid, p.center), sub4(mid, p.center)) <= mul(rr, rr) {
                            p.sweep(&self.models, &mut hit, start, end, r, ignore);
                        }
                    }
                }
            }
        }
        (hit.t != f32::MAX).then_some(hit)
    }
}
