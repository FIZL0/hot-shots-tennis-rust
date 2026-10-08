//! The court's sun-shade map: in clear and cloudy weather the game darkens the ball and the NPCs in the shade of
//! the court's shadow casters.
//!
//! - At load it views the hole model from straight above over a 1280 × 896 screen that just takes in the model's
//!   box, draws each caster's shadow texture onto the ground along the sun, and keeps every pixel whose red is
//!   above 0x6f as shade, one bit per pixel, rows of 0x500 bits (game z), 0x380 rows (game x), least significant
//!   bit first ([`Frame`], [`build`]).
//! - Each frame the ball's and the NPCs' light scale is the bilinear blend of the four map pixels around their
//!   (x, z), 0 where shade and 1 where not ([`lookup`]); 1.0 off the map, on court 7 and in rain. The ball's is at
//!   least its height above the ground, so 1.0 from a unit up ([`ball`]); off the court the height comes from a
//!   ray cast down at the court model ([`ball_height`]). The players are never shaded. The scale multiplies the
//!   model's directional light colour (VU1), not the ambient.

use crate::{mesh, ps2, vu0};

/// Bits per map row (game z) and rows (game x).
pub const COLS: usize = 0x500;
pub const ROWS: usize = 0x380;
pub const BYTES: usize = COLS * ROWS / 8;

/// Where the map lies on the court: the corner it starts at and map pixels per game unit, (z, x).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub origin: [f32; 2],
    pub scale: [f32; 2],
}

impl Frame {
    /// The game's map frame for a hole model whose packet boxes span `lo`..`hi` (model space), drawn at scale `s`
    /// and translation `t`. The camera height is worked in double precision (the C library's `tan`),
    /// truncated back to single.
    pub fn new(lo: [f32; 3], hi: [f32; 3], s: f32, t: [f32; 3]) -> Frame {
        let (m, a) = (ps2::mul, ps2::add);
        let centre = |k: usize| m(m(a(lo[k], hi[k]), 0.5), s);
        let (cx, cz) = (centre(0), centre(2));
        let zext = ps2::sub(m(hi[2], s), m(lo[2], s));
        let xext = ps2::sub(m(hi[0], s), m(lo[0], s));
        let tan = (f32::from_bits(0x3eb2_b8c2) as f64).tan(); // 20°: half the field of view
        // the camera height (game y, negative is up) that frames the longer side
        let h = if ps2::div(zext, xext) > 1.333_333_4 {
            ps2::chop(m(-zext, 0.5) as f64 / tan)
        } else {
            ps2::chop(4.0 * (m(-xext, 0.5) as f64 / tan) / 3.0)
        };
        let hz = ps2::chop(-h as f64 * tan);
        let hx = ps2::chop(3.0 * (-h as f64 * tan) / 4.0);
        Frame {
            origin: [ps2::sub(a(t[2], cz), hz), ps2::sub(a(t[0], cx), hx)],
            scale: [ps2::div(1.0, ps2::div(m(hz, 2.0), 1280.0)), ps2::div(1.0, ps2::div(m(hx, 2.0), 896.0))],
        }
    }
}

/// `cvt.w.s` read as unsigned, the halved path for values from 2³¹.
fn ftou(f: f32) -> u32 {
    if f < 2_147_483_648.0 { f as i32 as u32 } else { (ps2::sub(f, 2_147_483_648.0) as i32 as u32) | 0x8000_0000 }
}

/// The light scale at game (x, z): 1.0 off the map.
pub fn lookup(map: &[u8], f: &Frame, x: f32, z: f32) -> f32 {
    let fr = ps2::mul(f.scale[1], ps2::sub(x, f.origin[1]));
    let row = ftou(fr);
    if row >= ROWS as u32 - 1 {
        return 1.0;
    }
    let fc = ps2::mul(f.scale[0], ps2::sub(z, f.origin[0]));
    let col = ftou(fc);
    if col >= COLS as u32 - 1 {
        return 1.0;
    }
    let lit = |i: u32| if map[(i >> 3) as usize] >> (i & 7) & 1 != 0 { 0.0 } else { 1.0 };
    let i = col + row * COLS as u32;
    let a = [lit(i), lit(i + COLS as u32), lit(i + 1), lit(i + COLS as u32 + 1)];
    let (wr, wc) = (ps2::sub(fr, ps2::utof(row)), ps2::sub(fc, ps2::utof(col)));
    let (m, s) = (ps2::mul, ps2::sub);
    let near = ps2::add(m(s(1.0, wr), a[0]), m(wr, a[1]));
    let far = ps2::add(m(s(1.0, wr), a[2]), m(wr, a[3]));
    ps2::add(m(s(1.0, wc), near), m(wc, far))
}

/// The ball's light scale: under a unit above the ground (`height`, up positive) the map's but at least the height
/// (the shade fades out as it rises; at least the smallest denormal below the ground), else 1.0.
pub fn ball(map: &[u8], f: &Frame, x: f32, z: f32, height: f32) -> f32 {
    if height >= 1.0 {
        return 1.0;
    }
    let h = if height < f32::from_bits(1) { f32::from_bits(1) } else { height };
    let s = lookup(map, f, x, z);
    if h <= s { s } else { h }
}

/// The ball's height above the ground for [`ball`], from its game position (y down), over [`ground`].
pub fn ball_height(court: &mesh::Object, models: &[mesh::Model], pos: [f32; 4]) -> Option<f32> {
    ground(court, models, pos).map(|(g, _)| -ps2::sub(pos[1], g[1]))
}

/// The ground under the ball (game position, y down) and its normal (towards the ball), which the ball's light and
/// its shadow use. On the court (|x| ≤ 10.685, |z| ≤ 19.885) the point (x, 0, z) and straight up; off it the game
/// casts a ray 200 units straight down at the court model (`court`); `None` when it misses (the game keeps the
/// last light scale and shadow).
pub fn ground(court: &mesh::Object, models: &[mesh::Model], pos: [f32; 4]) -> Option<([f32; 4], [f32; 4])> {
    let [x, y, z, w] = pos;
    if x.abs() <= 10.685 && z.abs() <= 19.885 {
        return Some(([x, 0.0, z, w], [-0.0, -1.0, -0.0, -0.0]));
    }
    let mut hit = mesh::Hit::NONE;
    court.ray(models, &mut hit, pos, [x, ps2::add(y, 200.0), z, w]).then_some((hit.centre, hit.normal))
}

/// The ball shadow's placement (game space, row vectors: across, the ground's down axis, towards the camera, then
/// the position) on the ground `point` with `normal` from [`ground`], seen from the camera at `eye`: turned about
/// the normal to face the camera (or along game z on ground steeper than 0.1 from vertical), 0.005 above the ground.
pub fn ball_shadow(point: [f32; 4], normal: [f32; 4], eye: [f32; 4]) -> [[f32; 4]; 4] {
    let down = normal.map(|v| -v);
    let to_eye = if down[1] > 0.1 { std::array::from_fn(|k| ps2::sub(eye[k], point[k])) } else { [0.0, -1.0, 0.0, 0.0] };
    let y = vu0::normalize(down);
    let x = vu0::normalize(vu0::cross(y, to_eye));
    let z = vu0::normalize(vu0::cross(x, y));
    [x, y, z, std::array::from_fn(|k| if k == 3 { point[3] } else { ps2::add(point[k], ps2::mul(normal[k], 0.005)) })]
}

/// How far the ball shadow is drawn stretched along its towards-the-camera axis: 1 within 10 units (across the
/// ground) of the camera `eye`, rising to 3 at 50 and beyond, so it keeps its size on screen.
pub fn ball_shadow_stretch(pos: [f32; 4], eye: [f32; 4]) -> f32 {
    let (dx, dz) = (ps2::sub(pos[0], eye[0]), ps2::sub(pos[2], eye[2]));
    let d = ps2::sqrt(ps2::add(ps2::mul(dx, dx), ps2::mul(dz, dz)));
    if d <= 10.0 {
        return 1.0;
    }
    let t = ps2::div(ps2::sub(d, 10.0), ps2::sub(50.0, 10.0)).clamp(0.0, 1.0);
    ps2::add(ps2::mul(t, ps2::sub(3.0, 1.0)), 1.0)
}

/// tan of half the camera's full field of view `fov` (degrees), as the ball draw works it out each frame.
pub fn half_fov_tan(fov: f32) -> f32 {
    crate::libm::tanf(ps2::mul(ps2::mul(fov, 0.5), 0.017453292))
}

/// The ball model's draw scale for its object scale `scale` (2.0) at view depth `depth` with `t` from
/// [`half_fov_tan`]: in the serve, the rally and after the point (`grow`, match phase above 1) it grows with depth
/// beyond 1/(0.13·t), so it keeps a least size on screen.
pub fn ball_scale(scale: f32, depth: f32, t: f32, grow: bool) -> f32 {
    if !grow {
        return scale;
    }
    ps2::mul(scale, ps2::mul(0.13, ps2::mul(depth, t)).max(1.0))
}

/// The ball's outline billboard (a second `ballshadow.mdl`, drawn only at full ball alpha): scale 2 × the ball's
/// draw scale `ball` × 0.4·depth·t clamped to 0.8..1, and its placement (game space, row vectors) at the ball `pos`
/// seen from `eye`: across = normalize((pos − eye) × down), then normalize(down × across), down (the camera's
/// down axis), so its xz quad stands facing the camera.
pub fn ball_outline(ball: f32, depth: f32, t: f32, pos: [f32; 4], eye: [f32; 4], down: [f32; 4]) -> (f32, [[f32; 4]; 4]) {
    let s = ps2::mul(0.4, ps2::mul(depth, t));
    let s = if s < 0.8 { 0.8 } else { s.min(1.0) };
    let d = vu0::normalize(down);
    let to = std::array::from_fn(|k| ps2::sub(pos[k], eye[k]));
    let x = vu0::normalize(vu0::cross(to, d));
    let y = vu0::normalize(vu0::cross(d, x));
    (ps2::mul(ps2::mul(ball, 2.0), s), [x, y, d, pos])
}

/// A shadow caster: its model's triangles (model space) drawn solid, those of its alpha-tested materials drawn
/// through their texture ([`Shape::new`]), and its placement, model axes (scaled, game space) and position.
pub struct Caster {
    pub axes: [[f32; 3]; 3],
    pub pos: [f32; 3],
    pub tris: Vec<[[f32; 3]; 3]>,
    pub cut: Vec<Cutout>,
}

/// An alpha-tested material's triangles (model space, with their texture coordinates) and its texture's alpha
/// (PS2, 0x80 opaque) and GS wrap modes.
#[derive(Clone, Default)]
pub struct Cutout {
    pub width: usize,
    pub height: usize,
    pub alpha: Vec<u8>,
    pub wrap: [u8; 2],
    /// The red its triangles blend toward: 0x80, fogged (PRIM FGE, fog 0xff) 0x7f.
    pub red: u8,
    pub tris: Vec<([[f32; 3]; 3], [[f32; 2]; 3])>,
}

/// A caster model as its shadow texture draws it: an alpha-tested (TEST mode 10..29), textured material's
/// triangles are drawn textured where the texture's bilinear alpha is at least 1 and blended by it (the game's
/// shadow draw: TEST GEQUAL 1, TFX HIGHLIGHT2 on a black vertex, ALPHA (Cs − Cd)·As + Cd, bilinear, top level),
/// every other one solid.
#[derive(Clone, Default)]
pub struct Shape {
    pub tris: Vec<[[f32; 3]; 3]>,
    pub cut: Vec<Cutout>,
}

impl Shape {
    pub fn new(model: &hst_data::mdl::Model, mats: &hst_data::mtl::Mtl) -> Shape {
        let mut s = Shape::default();
        for (mi, packets) in model.materials.iter().enumerate() {
            let mat = mats.materials.get(mi);
            let mode = mat.map_or(0, |m| i16::from_le_bytes([m.header[0x1e], m.header[0x1f]]));
            let tex = mat.and_then(|m| m.texture).and_then(|t| mats.textures.get(t)).filter(|_| (10..=29).contains(&mode));
            let mut cut = tex.map(|t| Cutout {
                width: t.width as usize,
                height: t.height as usize,
                // back from the expanded alpha (a·255/128) to the PS2's
                alpha: t.rgba.chunks_exact(4).map(|c| ((c[3] as u32 * 128).div_ceil(255)) as u8).collect(),
                wrap: model.wrap.get(mi).copied().unwrap_or([0, 0]),
                red: if packets.iter().any(|p| p.prim & 0x30 == 0x30) { 0x7f } else { 0x80 },
                tris: Vec::new(),
            });
            for pk in packets {
                for t in &pk.triangles {
                    let v = t.map(|i| pk.vertices[i as usize]);
                    match &mut cut {
                        Some(c) if pk.prim & 0x10 != 0 => c.tris.push((v.map(|v| v.pos), v.map(|v| v.uv))),
                        _ => s.tris.push(v.map(|v| v.pos)),
                    }
                }
            }
            s.cut.extend(cut.filter(|c| !c.tris.is_empty()));
        }
        s
    }
}

type V3 = [f64; 3];
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn scale(a: V3, k: f64) -> V3 {
    a.map(|x| x * k)
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn norm(a: V3) -> V3 {
    scale(a, 1.0 / dot(a, a).sqrt())
}

/// A caster's light frame: game point → shadow texture (u, v) in −1..1, and the planes (n, d: n·p + d ≥ 0
/// inside) bounding the box's shadow volume.
struct Light {
    centre: V3,
    to_uv: [V3; 2],
    planes: Vec<[f64; 4]>,
}

impl Light {
    /// The caster's model box, its half-extents grown by a tenth, seen along the sun `d` (unit, sun → ground):
    /// texture axes `lu` = (0, 0, 1) × d, `lv` = d × `lu`, scaled so the box's wider side just fills the texture.
    fn new(c: &Caster, d: V3) -> Light {
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in c.tris.iter().chain(c.cut.iter().flat_map(|k| k.tris.iter().map(|t| &t.0))).flatten() {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k] as f64);
                hi[k] = hi[k].max(p[k] as f64);
            }
        }
        let axes = c.axes.map(|a| a.map(f64::from));
        let mid: V3 = std::array::from_fn(|k| (lo[k] + hi[k]) * 0.5);
        let centre = add((0..3).fold([0.0; 3], |s, k| add(s, scale(axes[k], mid[k]))), c.pos.map(f64::from));
        let mut a: [V3; 3] = std::array::from_fn(|k| scale(axes[k], (hi[k] - lo[k]) * 0.5 * 1.1));
        let lu = if d[2].abs() < 0.99999 { norm(cross([0.0, 0.0, 1.0], d)) } else { [1.0, 0.0, 0.0] };
        let lv = cross(d, lu);
        let s = a.iter().map(|a| dot(*a, lu).abs()).sum::<f64>().max(a.iter().map(|a| dot(*a, lv).abs()).sum());
        // (p − centre) · inverse([lu; lv; d]) / s: the inverse's first two columns
        let det = dot(lu, cross(lv, d));
        let to_uv = [scale(cross(lv, d), 1.0 / (det * s)), scale(cross(d, lu), 1.0 / (det * s))];
        // the box's axes turned away from the sun; its silhouette hexagon along the sun bounds the volume's sides
        let mut dd = [0.0; 3];
        for k in 0..3 {
            if dot(a[k], d) > 0.0 {
                a[k] = scale(a[k], -1.0);
            }
            dd[k] = dot(a[k], d);
        }
        let p: [V3; 3] = std::array::from_fn(|k| add(a[k], scale(d, dd[k])));
        let comb = |i: f64, j: f64, k: f64| add(add(scale(p[0], i), scale(p[1], j)), scale(p[2], k));
        let hex = [comb(1., 1., -1.), comb(-1., 1., -1.), comb(-1., 1., 1.), comb(-1., -1., 1.), comb(1., -1., 1.), comb(1., -1., -1.)];
        // the near cap: the sun's heading (its height dropped unless it is nearly overhead) through the corner
        let mut n = d;
        if n[1].abs() < 0.99 {
            n[1] = 0.0;
        }
        let n = norm(n);
        let corner = add(centre, add(add(a[0], a[1]), a[2]));
        let mut planes = vec![[n[0], n[1], n[2], -dot(n, corner)]];
        let mut prev = hex[5];
        for cur in hex {
            let mut n = cross(d, [cur[0] - prev[0], cur[1] - prev[1], cur[2] - prev[2]]);
            let f = dot(n, prev);
            if f.abs() > 1e-7 {
                if f > 0.0 {
                    n = scale(n, -1.0);
                }
                let n = norm(n);
                planes.push([n[0], n[1], n[2], -dot(n, add(prev, centre))]);
            }
            prev = cur;
        }
        Light { centre, to_uv, planes }
    }

    fn uv(&self, p: V3) -> [f64; 2] {
        let r = [p[0] - self.centre[0], p[1] - self.centre[1], p[2] - self.centre[2]];
        [dot(r, self.to_uv[0]), dot(r, self.to_uv[1])]
    }
}

/// The GS's fill of triangle `v` (fixed point, `unit` per pixel) over a `w` × `h` grid: pixel centres on whole
/// pixels, top-left fill convention; `put(column, row, barycentric weights)`.
fn raster(mut v: [[i64; 2]; 3], unit: i64, w: usize, h: usize, mut put: impl FnMut(usize, usize, [f64; 3])) {
    let cr = |a: [i64; 2], b: [i64; 2], c: [i64; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let mut area = cr(v[0], v[1], v[2]);
    if area == 0 {
        return;
    }
    let swapped = area < 0;
    if swapped {
        v.swap(1, 2);
        area = -area;
    }
    let span = |k: usize, n: usize| {
        let lo = -(-v.iter().map(|p| p[k]).min().unwrap()).div_euclid(unit);
        let hi = v.iter().map(|p| p[k]).max().unwrap().div_euclid(unit);
        lo.max(0)..(hi + 1).min(n as i64)
    };
    let edge = |p: [i64; 2], q: [i64; 2], x: i64, y: i64| {
        let e = (q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0]);
        let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
        (e, e > 0 || e == 0 && (dy < 0 || dy == 0 && dx > 0))
    };
    for r in span(1, h) {
        for c in span(0, w) {
            let (x, y) = (c * unit, r * unit);
            let ((ea, ia), (eb, ib), (ec, ic)) = (edge(v[1], v[2], x, y), edge(v[2], v[0], x, y), edge(v[0], v[1], x, y));
            if ia && ib && ic {
                let wt = [ea as f64 / area as f64, eb as f64 / area as f64, ec as f64 / area as f64];
                put(c as usize, r as usize, if swapped { [wt[0], wt[2], wt[1]] } else { wt });
            }
        }
    }
}

/// Shadow texture side (texels).
const TEX: usize = 128;

/// A caster's shadow texture (palette index 0..15, red = index·0x20 clamped): its triangles drawn in light space at
/// twice the size with red 0x80 over 0, every other pixel kept as red >> 4. Solid triangles write 0x80; a
/// [`Shape`]'s cut ones blend toward it by their texture's bilinear alpha ((0x80 − Rd)·At >> 7 + Rd) where At ≥ 1.
fn texture(c: &Caster, l: &Light) -> Vec<u8> {
    let mut red = vec![0u32; TEX * TEX];
    let axes = c.axes.map(|a| a.map(f64::from));
    let screen = |t: &[[f32; 3]; 3]| {
        t.map(|p| {
            let g = add((0..3).fold([0.0; 3], |s, k| add(s, scale(axes[k], p[k] as f64))), c.pos.map(f64::from));
            l.uv(g).map(|u| ((128.0 * u + 126.0) * 16.0).floor() as i64)
        })
    };
    for k in &c.cut {
        for (t, uv) in &k.tris {
            raster(screen(t), 32, TEX, TEX, |x, y, w| {
                let st: [f64; 2] = std::array::from_fn(|i| w[0] * uv[0][i] as f64 + w[1] * uv[1][i] as f64 + w[2] * uv[2][i] as f64);
                let a = alpha(k, st[0], st[1]);
                if a >= 1 {
                    let d = &mut red[x + y * TEX];
                    *d = (((k.red as i32 - *d as i32) * a as i32 >> 7) + *d as i32) as u32;
                }
            });
        }
    }
    for t in &c.tris {
        raster(screen(t), 32, TEX, TEX, |x, y, _| red[x + y * TEX] = 0x80);
    }
    red.iter().map(|&r| (r >> 4) as u8).collect()
}

/// The GS's bilinear read (4-bit fraction) of a cutout's alpha at (s, t), wrapped or clamped as its material sets.
fn alpha(k: &Cutout, s: f64, t: f64) -> u32 {
    let fix = |s: f64, n: usize| ((s * n as f64 - 0.5) * 16.0).floor() as i64;
    let (u, v) = (fix(s, k.width), fix(t, k.height));
    let (u0, v0, fu, fv) = (u >> 4, v >> 4, (u & 15) as u32, (v & 15) as u32);
    let wrap = |x: i64, n: usize, mode: u8| if mode == 0 { x.rem_euclid(n as i64) } else { x.clamp(0, n as i64 - 1) } as usize;
    let at = |x: i64, y: i64| k.alpha[wrap(x, k.width, k.wrap[0]) + wrap(y, k.height, k.wrap[1]) * k.width] as u32;
    (at(u0, v0) * (16 - fu) * (16 - fv) + at(u0 + 1, v0) * fu * (16 - fv) + at(u0, v0 + 1) * (16 - fu) * fv + at(u0 + 1, v0 + 1) * fu * fv) >> 8
}

/// The GS's bilinear read (4-bit fraction, clamped) of a shadow texture (red index·0x20, clamped) at (s, t) in 0..1.
fn sample(tex: &[u8], s: f64, t: f64) -> u32 {
    let fix = |s: f64| ((s * TEX as f64 - 0.5) * 16.0).floor() as i64;
    let (u, v) = (fix(s), fix(t));
    let (u0, v0, fu, fv) = (u >> 4, v >> 4, (u & 15) as u32, (v & 15) as u32);
    let at = |x: i64, y: i64| (tex[x.clamp(0, TEX as i64 - 1) as usize + y.clamp(0, TEX as i64 - 1) as usize * TEX] as u32 * 0x20).min(0xff);
    (at(u0, v0) * (16 - fu) * (16 - fv) + at(u0 + 1, v0) * fu * (16 - fv) + at(u0, v0 + 1) * (16 - fu) * fv + at(u0 + 1, v0 + 1) * fu * fv) >> 8
}

/// Build the map: every caster's shadow texture is drawn along the sun `dir` (unit, sun → ground) onto each
/// hole `ground` triangle (game space) whose corners aren't all outside one side of the caster's shadow volume,
/// added up without depth over a screen viewing the hole from straight above, and the red kept above 0x6f.
///
/// ponytail: the screen is orthographic with columns at ¾ of the map's scale, fitted to the game's map (the
/// camera on paper is 40° perspective), the setup is worked in double precision, not the game's VU1 floats, and
/// every fogged cutout blends toward 0x7f (the game's court 10 seat and statue stay at 0x80, cause not found):
/// IoU 0.975 against the game's map on court 10 (P17v, P17v2 exact).
pub fn build(f: &Frame, dir: [f32; 3], casters: &[Caster], ground: &[[[f32; 3]; 3]]) -> Vec<u8> {
    let d = dir.map(f64::from);
    let lights: Vec<Light> = casters.iter().map(|c| Light::new(c, d)).collect();
    let texs: Vec<Vec<u8>> = casters.iter().zip(&lights).map(|(c, l)| texture(c, l)).collect();
    let (sz, sx) = (f.scale[0] as f64, f.scale[1] as f64);
    let (cz, cx) = (f.origin[0] as f64 + 640.0 / sz, f.origin[1] as f64 + 448.0 / sx);
    let mut red = vec![0u32; COLS * ROWS];
    for t in ground {
        let p = t.map(|p| p.map(f64::from));
        let v = p.map(|p| [((639.5 + 0.75 * sz * (p[2] - cz)) * 16.0).floor() as i64, ((447.5 + sx * (p[0] - cx)) * 16.0).floor() as i64]);
        let lit: Vec<usize> = (0..casters.len()).filter(|&i| !lights[i].planes.iter().any(|n| p.iter().all(|p| dot(*p, [n[0], n[1], n[2]]) + n[3] < 0.0))).collect();
        if lit.is_empty() {
            continue;
        }
        raster(v, 16, COLS, ROWS, |c, r, w| {
            let g: V3 = std::array::from_fn(|k| w[0] * p[0][k] + w[1] * p[1][k] + w[2] * p[2][k]);
            for &i in &lit {
                let [u, v] = lights[i].uv(g);
                red[c + r * COLS] += sample(&texs[i], 0.5 * u + 0.5, 0.5 * v + 0.5);
            }
        });
    }
    let mut map = vec![0u8; BYTES];
    for (i, &r) in red.iter().enumerate() {
        if r.min(0xff) > 0x6f {
            map[i >> 3] |= 1 << (i & 7);
        }
    }
    map
}

/// A caster's texture matrix in the game's EE floats: each row of its light matrix `light` (game point → (u, v)
/// in −1..1, w ≈ 1; the row-vector form, translation last) times the bias that takes u, v to 0.5u + 0.5, 0.5v + 0.5
/// and leaves the third component the light's w. The game first multiplies the receiver node's matrix in; the
/// court's hole nodes are identity, which leaves `light` bit for bit.
///
/// ponytail: `light` is taken as the game builds it at load (its own box/sun setup in EE floats isn't ported).
pub fn tex_matrix(light: &[vu0::V4; 4]) -> [vu0::V4; 4] {
    const B: [vu0::V4; 4] = [[0.5, 0.0, 0.0, 0.0], [0.0, 0.5, 0.0, 0.0], [0.0; 4], [0.5, 0.5, 1.0, 1.0]];
    // the EE sums the products as ((r1·b1 + r0·b0) + r2·b2) + r3·b3
    light.map(|r| std::array::from_fn(|j| ps2::madd(ps2::madd(ps2::madd(ps2::mul(r[1], B[1][j]), r[0], B[0][j]), r[2], B[2][j]), r[3], B[3][j])))
}

/// One shade receiver vertex as the game's VU1 program sends it to the GS: screen X, Y (12.4 fixed point) and
/// S, T, Q. `item` is the receiver model's matrix (court hole: y flattened), `vp` the shade camera's world →
/// screen matrix (40°, straight down), `tex` the caster's [`tex_matrix`], `p` the model-space vertex. The (s, t, w)
/// the game precomputes per vertex at load (VU0) are multiplied by Q = 1/w of the screen position, so Q itself
/// carries the light's w (1 ± an ulp or two).
pub fn receiver_vertex(item: &[vu0::V4; 4], vp: &[vu0::V4; 4], tex: &[vu0::V4; 4], p: [f32; 3]) -> ([u16; 2], [f32; 3]) {
    let v = [p[0], p[1], p[2], 1.0];
    let clip = vu0::transform(&item.map(|r| vu0::transform(vp, r)), v);
    let q = vu0::div(1.0, clip[3]);
    let st = vu0::transform(tex, v);
    let ftoi4 = |x: f32| (vu0::mul(x, q) * 16.0) as i32 as u16; // FTOI4 truncates
    ([ftoi4(clip[0]), ftoi4(clip[1])], [vu0::mul(st[0], q), vu0::mul(st[1], q), vu0::mul(st[2], q)])
}

/// A vertex of the shadow texture pass: position in the 256² target (12.4 fixed point), texture coordinates (STQ
/// with Q = 1) and alpha (0x80 opaque).
#[derive(Clone, Copy, Debug)]
pub struct PassVertex {
    pub xy: [i32; 2],
    pub st: [f32; 2],
    pub a: u8,
}

/// How a texture coordinate past the texture's edge reads: wrapped, or clamped to the texels min..=max.
#[derive(Clone, Copy, Debug)]
pub enum Wrap {
    Repeat,
    Clamp(i32, i32),
}

/// A texture the pass reads: its alpha per texel (row-major, PS2 0x80 opaque), log2 width and height, wrap per axis.
pub struct PassTexture {
    pub alpha: Vec<u8>,
    pub log2: [u32; 2],
    pub wrap: [Wrap; 2],
}

/// One draw of the pass, one GS state over its triangles: solid (each triangle's last vertex alpha, flat) or
/// textured with bilinear filtering, the texture's alpha as is (HIGHLIGHT2) or times the Gouraud vertex alpha
/// (`modulate`). Pixels are written where the target's alpha is below 0x80 (destination alpha test) and the new
/// alpha is at least `aref`.
pub struct PassDraw {
    pub tris: Vec<[PassVertex; 3]>,
    pub tex: Option<PassTexture>,
    pub modulate: bool,
    pub aref: u8,
}

/// Target side and the scissor every caster draw keeps inside (texels 4..=251).
const TARGET: usize = 256;
const SCISSOR: [i32; 4] = [4, 4, 252, 252];
/// Pixels per scanline step of the software GS (its 128-bit code path).
const LANES: i32 = 4;

/// A caster's 128² shadow texture (palette index 0..15) as the game's load pass draws it, bit for bit with PCSX2's
/// software GS: the 256² target's alpha cleared to 0, every draw in order, the four bilinear sprites that halve the
/// target in place (each texel the mean of its 2 × 2, 4-bit weights), and the copy of alpha's top nibble into the
/// texture. The arithmetic is SSE's single floats (round to nearest), as PCSX2's x86 renderer does it.
///
/// ponytail: the draws are taken as the GS receives them (`research/p17v2_fixture.py`); building them from the
/// casters' models and light matrices in game floats is P17v7.
pub fn pass_texture(draws: &[PassDraw]) -> Vec<u8> {
    let mut target = vec![0u8; TARGET * TARGET];
    for d in draws {
        for t in &d.tris {
            pass_tri(&mut target, d, t);
        }
    }
    let half = |p: usize, q: usize| p as i32 + ((q as i32 - p as i32) * 8 >> 4);
    (0..TEX * TEX)
        .map(|i| {
            let (x, y) = (2 * (i % TEX), 2 * (i / TEX));
            let at = |dx: usize, dy: usize| target[x + dx + (y + dy) * TARGET] as usize;
            let (r0, r1) = (half(at(0, 0), at(1, 0)), half(at(0, 1), at(1, 1)));
            ((r0 + ((r1 - r0) * 8 >> 4)) >> 4) as u8
        })
        .collect()
}

/// The GS's texel coordinate rounding (as PCSX2 applies it when the draw's Z is constant): S and T lose their low 9
/// mantissa bits, more by how far their exponent is below Q's (1.0 here).
fn texel_round(st: f32) -> f32 {
    let (b, e) = (st.to_bits(), (st.to_bits() >> 23 & 0xff) as i32);
    f32::from_bits(b & !((1u32 << (9 + e.max(127) - e).min(23)) - 1))
}

/// Integer coordinates along a span from `left`: truncated at the span start, then per lane its truncated offset
/// from the aligned group of [`LANES`] at or before `left` and whole truncated group steps.
fn pass_lanes(t0: f32, d: f32, left: i32, n: i32) -> impl Iterator<Item = i32> {
    let (skip, step) = (left % LANES, (d * LANES as f32) as i32);
    (skip..skip + n).map(move |j| (t0 as i32).wrapping_add((d * (j % LANES - skip) as f32) as i32).wrapping_add(j / LANES * step))
}

/// The Gouraud channel (8.7 fixed point) along a span: 16-bit lanes from the truncated start, kept ≥ 0 per group step.
fn pass_colour(c0: f32, d: f32, left: i32, n: i32) -> Vec<i32> {
    let (skip, step) = (left % LANES, (d * LANES as f32) as i32 & 0xffff);
    let mut c: Vec<i32> = (0..LANES).map(|k| (c0 as i32 + ((d * (k - skip) as f32) as i32 & 0xffff)) & 0xffff).collect();
    let mut out = Vec::new();
    for j in skip..skip + n {
        if j % LANES == 0 && j > 0 {
            c.iter_mut().for_each(|c| *c = if (*c + step) & 0xffff >= 0x8000 { 0 } else { (*c + step) & 0xffff });
        }
        out.push(c[(j % LANES) as usize]);
    }
    out
}

/// The bilinear alpha (4-bit weights) at 16.16 texel coordinates u, v (half texel already taken off).
fn pass_sample(t: &PassTexture, u: i32, v: i32) -> i32 {
    let ax = |c: i32, k: usize| {
        let n = 1 << t.log2[k];
        [c >> 16, (c >> 16) + 1].map(|b| match t.wrap[k] {
            Wrap::Repeat => b & (n - 1),
            Wrap::Clamp(lo, hi) => b.clamp(lo, hi),
        })
    };
    let ([x0, x1], [y0, y1], uf, vf) = (ax(u, 0), ax(v, 1), (u & 0xffff) >> 12, (v & 0xffff) >> 12);
    let at = |x: i32, y: i32| t.alpha[(x + (y << t.log2[0])) as usize] as i32;
    let r0 = at(x0, y0) + ((at(x1, y0) - at(x0, y0)) * uf >> 4);
    let r1 = at(x0, y1) + ((at(x1, y1) - at(x0, y1)) * uf >> 4);
    r0 + ((r1 - r0) * vf >> 4)
}

/// One triangle into the target with PCSX2's software setup: vertices sorted by y, the edge and scan gradients from
/// the cross product, spans from ceil(edge x) at each row, and (s, t, colour) at the span start interpolated from the
/// top vertex of its section.
fn pass_tri(target: &mut [u8], d: &PassDraw, t: &[PassVertex; 3]) {
    let tsz = d.tex.as_ref().map_or([0.0; 2], |x| x.log2.map(|l| (0x10000u32 << l) as f32));
    let v: [[f32; 5]; 3] = t.map(|p| {
        let st = p.st.map(texel_round);
        [p.xy[0] as f32 * (1.0 / 16.0), p.xy[1] as f32 * (1.0 / 16.0), st[0] * tsz[0] - 32768.0, st[1] * tsz[1] - 32768.0, ((p.a as u32) << 7) as f32]
    });
    let flat = t[2].a as i32;
    let ys = [v[0][1], v[1][1], v[2][1]];
    let m1 = (ys[0] > ys[1]) as usize | ((ys[0] > ys[2]) as usize) << 1 | ((ys[1] > ys[2]) as usize) << 2;
    let [i0, i1, i2] = [[0, 1, 2], [1, 0, 2], [0; 3], [1, 2, 0], [0, 2, 1], [0; 3], [2, 0, 1], [2, 1, 0]][m1];
    let (v0, v1, v2) = (v[i0], v[i1], v[i2]);
    if v0[1] == v1[1] && v1[1] == v2[1] {
        return;
    }
    let sub = |a: [f32; 5], b: [f32; 5]| -> [f32; 5] { std::array::from_fn(|k| a[k] - b[k]) };
    let (dv0, dv1, dv2) = (sub(v1, v0), sub(v2, v0), sub(v2, v1));
    let cross = dv0[1] * dv1[0] - dv0[0] * dv1[1];
    if cross == 0.0 {
        return;
    }
    let m2 = cross < 0.0;
    let slope = |a: [f32; 5]| if a[1] != 0.0 { a[0] / a[1] } else { f32::INFINITY };
    let dd = [slope(dv0), dv1[0] / dv1[1], slope(dv2)];
    let c = [dv0[0], dv0[1], dv1[0], dv1[1]].map(|x| x / cross);
    let dscan: [f32; 5] = std::array::from_fn(|k| dv1[k] * c[1] - dv0[k] * c[3]);
    let dedge: [f32; 5] = std::array::from_fn(|k| dv0[k] * c[2] - dv1[k] * c[0]);
    let ceil = |y: f32| y.ceil() as i32;
    let mut section = |top: i32, bottom: i32, ex: [f32; 2], dex: [f32; 2], p0: [f32; 5]| {
        for y in top.max(SCISSOR[1])..bottom.min(SCISSOR[3]) {
            let dy = y as f32 - p0[1];
            let left = ceil(ex[0] + dex[0] * dy).max(SCISSOR[0]);
            let right = ceil(ex[1] + dex[1] * dy).min(SCISSOR[2]);
            if right <= left {
                continue;
            }
            let pre = left as f32 - p0[0];
            let tc: [f32; 5] = std::array::from_fn(|k| (p0[k] + dedge[k] * dy) + dscan[k] * pre);
            let n = right - left;
            let at: Vec<i32> = match &d.tex {
                None => vec![flat; n as usize],
                Some(tex) => {
                    let s = pass_lanes(tc[2], dscan[2], left, n).zip(pass_lanes(tc[3], dscan[3], left, n)).map(|(u, v)| pass_sample(tex, u, v));
                    if d.modulate {
                        s.zip(pass_colour(tc[4], dscan[4], left, n)).map(|(a, g)| ((a << 2) * g >> 16).min(255)).collect()
                    } else {
                        s.collect()
                    }
                }
            };
            for (x, a) in (left..right).zip(at) {
                let dst = &mut target[x as usize + y as usize * TARGET];
                if *dst < 0x80 && a >= d.aref as i32 {
                    *dst = a as u8;
                }
            }
        }
    };
    if v0[1] == v1[1] {
        let (a, b, e) = if m2 { (v0, v1, [dd[1], dd[2]]) } else { (v1, v0, [dd[2], dd[1]]) };
        section(ceil(v0[1]), ceil(v2[1]), [a[0], b[0]], e, a);
    } else {
        let e = if m2 { [dd[1], dd[0]] } else { [dd[0], dd[1]] };
        section(ceil(v0[1]), ceil(v1[1]), [v0[0]; 2], e, v0);
        let x = e.map(|s| v0[0] + s * dv0[1]);
        section(ceil(v1[1]), ceil(v2[1]), x, if m2 { [dd[1], dd[2]] } else { [dd[2], dd[1]] }, v1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_matches_game_ram() {
        // the hole models' packet boxes (court 10 greece_h01, court 04 park_h01) and the frames the game kept
        let c10 = Frame::new([-113.8475, -10.730181, -141.48601], [102.94255, 17.31305, 158.12808], 1.0, [0.0; 3]);
        assert_eq!(c10.origin.map(f32::to_bits), [0xc30d7c69, 0xc2eb9d91]);
        assert_eq!(c10.scale.map(f32::to_bits), [0x4088b58f, 0x407f30c6]);
        let c04 = Frame::new([-84.99254, -1.0187869, -103.35643], [102.43255, 2.5186331, 102.78649], 1.0, [0.0; 3]);
        assert_eq!(c04.origin.map(f32::to_bits), [0xc2fa7854, 0xc2a9fc2c]);
        assert_eq!(c04.scale.map(f32::to_bits), [0x40a3e7cf, 0x4098fa7d]);
    }

    #[test]
    fn lookup_blends_four_pixels() {
        let f = Frame { origin: [0.0, 0.0], scale: [1.0, 1.0] };
        let mut map = vec![0u8; BYTES];
        assert_eq!(lookup(&map, &f, 10.5, 20.5), 1.0);
        map[(20 + 10 * COLS) >> 3] |= 1 << ((20 + 10 * COLS) & 7); // row 10 (x), column 20 (z)
        assert_eq!(lookup(&map, &f, 10.0, 20.0), 0.0);
        assert_eq!(lookup(&map, &f, 10.5, 20.5), 0.75);
        assert_eq!(lookup(&map, &f, 10.5, 20.0), 0.5);
        // off the map and its last row/column
        assert_eq!(lookup(&map, &f, -2.0, 20.0), 1.0);
        assert_eq!(lookup(&map, &f, 10.0, (COLS - 1) as f32), 1.0);
        // in the air
        assert_eq!(ball(&map, &f, 10.0, 20.0, 1.0), 1.0);
        assert_eq!(ball(&map, &f, 10.0, 20.0, 0.25), 0.25);
        assert_eq!(ball(&map, &f, 10.0, 20.0, -0.5), f32::from_bits(1));
        assert_eq!(ball(&map, &f, 10.5, 20.5, 0.25), 0.75);
    }
}
