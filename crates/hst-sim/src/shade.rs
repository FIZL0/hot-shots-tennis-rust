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

#![deny(clippy::float_arithmetic)]

use crate::ps2::{F, fs};
use crate::vu0::Vf;
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
    #[allow(clippy::float_arithmetic)] // the C library's double-precision `tan` work, not the FPU
    pub fn new(lo: [f32; 3], hi: [f32; 3], s: f32, t: [f32; 3]) -> Frame {
        let (lo, hi, t) = (fs(lo), fs(hi), fs(t));
        let centre = |k: usize| (lo[k] + hi[k]) * 0.5 * s;
        let (cx, cz) = (centre(0), centre(2));
        let zext = hi[2] * s - lo[2] * s;
        let xext = hi[0] * s - lo[0] * s;
        let tan = musl::tan(f32::from_bits(0x3eb2_b8c2) as f64); // 20°: half the field of view
        // the camera height (game y, negative is up) that frames the longer side
        let h = if zext / xext > 1.333_333_4 {
            ps2::chop((-zext * 0.5).0 as f64 / tan)
        } else {
            ps2::chop(4.0 * ((-xext * 0.5).0 as f64 / tan) / 3.0)
        };
        let hz = ps2::chop(-h as f64 * tan);
        let hx = ps2::chop(3.0 * (-h as f64 * tan) / 4.0);
        Frame {
            origin: [(t[2] + cz - hz).0, (t[0] + cx - hx).0],
            scale: [(1.0 / (F(hz) * 2.0 / 1280.0)).0, (1.0 / (F(hx) * 2.0 / 896.0)).0],
        }
    }
}

/// `cvt.w.s` read as unsigned, the halved path for values from 2³¹.
fn ftou(f: f32) -> u32 {
    if f < 2_147_483_648.0 { f as i32 as u32 } else { ((F(f) - 2_147_483_648.0).0 as i32 as u32) | 0x8000_0000 }
}

/// The light scale at game (x, z): 1.0 off the map.
pub fn lookup(map: &[u8], f: &Frame, x: f32, z: f32) -> f32 {
    let fr = F(f.scale[1]) * (F(x) - f.origin[1]);
    let row = ftou(fr.0);
    if row >= ROWS as u32 - 1 {
        return 1.0;
    }
    let fc = F(f.scale[0]) * (F(z) - f.origin[0]);
    let col = ftou(fc.0);
    if col >= COLS as u32 - 1 {
        return 1.0;
    }
    let lit = |i: u32| {
        if map[(i >> 3) as usize] >> (i & 7) & 1 != 0 { 0.0 } else { 1.0 }
    };
    let i = col + row * COLS as u32;
    let a = [lit(i), lit(i + COLS as u32), lit(i + 1), lit(i + COLS as u32 + 1)];
    let (wr, wc) = (fr - ps2::utof(row), fc - ps2::utof(col));
    let near = (1.0 - wr) * a[0] + wr * a[1];
    let far = (1.0 - wr) * a[2] + wr * a[3];
    ((1.0 - wc) * near + wc * far).0
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
    ground(court, models, pos).map(|(g, _)| (-(F(pos[1]) - g[1])).0)
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
    court.ray(models, &mut hit, pos, [x, (F(y) + 200.0).0, z, w]).then_some((hit.centre, hit.normal))
}

/// The ball shadow's placement (game space, row vectors: across, the ground's down axis, towards the camera, then
/// the position) on the ground `point` with `normal` from [`ground`], seen from the camera at `eye`: turned about
/// the normal to face the camera (or along game z on ground steeper than 0.1 from vertical), 0.005 above the ground.
pub fn ball_shadow(point: [f32; 4], normal: [f32; 4], eye: [f32; 4]) -> [[f32; 4]; 4] {
    let down = normal.map(|v| (-F(v)).0);
    let to_eye = if down[1] > 0.1 { std::array::from_fn(|k| (F(eye[k]) - point[k]).0) } else { [0.0, -1.0, 0.0, 0.0] };
    let y = vu0::normalize(down);
    let x = vu0::normalize(vu0::cross(y, to_eye));
    let z = vu0::normalize(vu0::cross(x, y));
    [x, y, z, std::array::from_fn(|k| if k == 3 { point[3] } else { (point[k] + F(normal[k]) * 0.005).0 })]
}

/// How far the ball shadow is drawn stretched along its towards-the-camera axis: 1 within 10 units (across the
/// ground) of the camera `eye`, rising to 3 at 50 and beyond, so it keeps its size on screen.
pub fn ball_shadow_stretch(pos: [f32; 4], eye: [f32; 4]) -> f32 {
    let (dx, dz) = (F(pos[0]) - eye[0], F(pos[2]) - eye[2]);
    let d = (dx * dx + dz * dz).sqrt();
    if d <= 10.0 {
        return 1.0;
    }
    let t = ((d - 10.0) / (F(50.0) - 10.0)).clamp(0.0, 1.0);
    (t * (F(3.0) - 1.0) + 1.0).0
}

/// tan of half the camera's full field of view `fov` (degrees), as the ball draw works it out each frame.
pub fn half_fov_tan(fov: f32) -> f32 {
    crate::libm::tanf((F(fov) * 0.5 * 0.017453292).0)
}

/// The ball model's draw scale for its object scale `scale` (2.0) at view depth `depth` with `t` from
/// [`half_fov_tan`]: in the serve, the rally and after the point (`grow`, match phase above 1) it grows with depth
/// beyond 1/(0.13·t), so it keeps a least size on screen.
pub fn ball_scale(scale: f32, depth: f32, t: f32, grow: bool) -> f32 {
    if !grow {
        return scale;
    }
    (F(scale) * (0.13 * (F(depth) * t)).max(1.0)).0
}

/// The ball's outline billboard (a second `ballshadow.mdl`, drawn only at full ball alpha): scale 2 × the ball's
/// draw scale `ball` × 0.4·depth·t clamped to 0.8..1, and its placement (game space, row vectors) at the ball `pos`
/// seen from `eye`: across = normalize((pos − eye) × down), then normalize(down × across), down (the camera's
/// down axis), so its xz quad stands facing the camera.
pub fn ball_outline(
    ball: f32,
    depth: f32,
    t: f32,
    pos: [f32; 4],
    eye: [f32; 4],
    down: [f32; 4],
) -> (f32, [[f32; 4]; 4]) {
    let s = (0.4 * (F(depth) * t)).0;
    let s = if s < 0.8 { 0.8 } else { s.min(1.0) };
    let d = vu0::normalize(down);
    let to = std::array::from_fn(|k| (F(pos[k]) - eye[k]).0);
    let x = vu0::normalize(vu0::cross(to, d));
    let y = vu0::normalize(vu0::cross(d, x));
    ((F(ball) * 2.0 * s).0, [x, y, d, pos])
}

/// A shadow caster: its model's triangles (model space) drawn solid, those of its alpha-tested materials drawn
/// through their texture ([`Shape::new`]), its model matrix (scaled, game space, row vectors), and the
/// [`light_matrix`] and [`planes`] the game builds for it at load.
pub struct Caster {
    pub item: [vu0::V4; 4],
    pub light: [vu0::V4; 4],
    pub planes: Planes,
    pub tris: Vec<[[f32; 3]; 3]>,
    pub cut: Vec<Cutout>,
}

impl Caster {
    /// The caster `shape` placed by its plant record `p` (a tree, prop or structure), lit along the [`sun_axes`]
    /// `axes`: the item matrix [`world::place`](crate::world::place)s it, its 3×3 times the plant's scale unless 1.
    pub fn new(shape: &Shape, p: &hst_data::layout::Placement, axes: &[vu0::V4; 3]) -> Caster {
        let (mut item, _) = crate::world::place(p.category, p.pos, p.yaw, p.code);
        if p.scale != 1.0 {
            item[..3].iter_mut().for_each(|r| r[..3].iter_mut().for_each(|x| *x = (Vf(*x) * p.scale).0));
        }
        let (light, planes) = (light_matrix(&item, p.scale, shape.centre, shape.half, axes), planes(&item, p.scale, shape.centre, shape.half, axes));
        Caster { item, light, planes, tris: shape.tris.clone(), cut: shape.cut.clone() }
    }
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
    /// The model's root box centre and half extents (EE floats), which the light frame takes in.
    pub centre: [f32; 3],
    pub half: [f32; 3],
}

impl Shape {
    pub fn new(model: &hst_data::mdl::Model, mats: &hst_data::mtl::Mtl) -> Shape {
        let mut s = Shape {
            centre: std::array::from_fn(|k| ((F(model.lo[k]) + model.hi[k]) * 0.5).0),
            half: std::array::from_fn(|k| ((F(model.hi[k]) - model.lo[k]) * 0.5).0),
            ..Shape::default()
        };
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

/// A game point through a caster's light matrix `l` (row vectors): its shadow texture (u, v) in −1..1, over the
/// light's w.
#[allow(clippy::float_arithmetic)] // double-precision raster standing in for VU1/GS (render only), not the FPU
fn light_uv(l: &[vu0::V4; 4], p: [f64; 3]) -> [f64; 2] {
    let c = |j: usize| p[0] * l[0][j] as f64 + p[1] * l[1][j] as f64 + p[2] * l[2][j] as f64 + l[3][j] as f64;
    let w = c(3);
    [c(0) / w, c(1) / w]
}

/// A model point through a caster's item matrix (row vectors) into game space.
#[allow(clippy::float_arithmetic)] // double-precision raster standing in for VU1/GS (render only), not the FPU
fn to_world(m: &[vu0::V4; 4], p: [f32; 3]) -> [f64; 3] {
    std::array::from_fn(|j| p[0] as f64 * m[0][j] as f64 + p[1] as f64 * m[1][j] as f64 + p[2] as f64 * m[2][j] as f64 + m[3][j] as f64)
}

/// The GS's fill of triangle `v` (fixed point, `unit` per pixel) over a `w` × `h` grid: pixel centres on whole
/// pixels, top-left fill convention; `put(column, row, barycentric weights)`.
#[allow(clippy::float_arithmetic)] // double-precision raster standing in for VU1/GS (render only), not the FPU
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
#[allow(clippy::float_arithmetic)] // double-precision raster standing in for VU1/GS (render only), not the FPU
fn texture(c: &Caster) -> Vec<u8> {
    let mut red = vec![0u32; TEX * TEX];
    let screen = |t: &[[f32; 3]; 3]| t.map(|p| light_uv(&c.light, to_world(&c.item, p)).map(|u| ((128.0 * u + 126.0) * 16.0).floor() as i64));
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
#[allow(clippy::float_arithmetic)] // double-precision raster standing in for VU1/GS (render only), not the FPU
fn alpha(k: &Cutout, s: f64, t: f64) -> u32 {
    let fix = |s: f64, n: usize| ((s * n as f64 - 0.5) * 16.0).floor() as i64;
    let (u, v) = (fix(s, k.width), fix(t, k.height));
    let (u0, v0, fu, fv) = (u >> 4, v >> 4, (u & 15) as u32, (v & 15) as u32);
    let wrap = |x: i64, n: usize, mode: u8| if mode == 0 { x.rem_euclid(n as i64) } else { x.clamp(0, n as i64 - 1) } as usize;
    let at = |x: i64, y: i64| k.alpha[wrap(x, k.width, k.wrap[0]) + wrap(y, k.height, k.wrap[1]) * k.width] as u32;
    (at(u0, v0) * (16 - fu) * (16 - fv) + at(u0 + 1, v0) * fu * (16 - fv) + at(u0, v0 + 1) * (16 - fu) * fv + at(u0 + 1, v0 + 1) * fu * fv) >> 8
}

/// The GS's bilinear read (4-bit fraction, clamped) of a shadow texture (red index·0x20, clamped) at (s, t) in 0..1.
#[allow(clippy::float_arithmetic)] // double-precision raster standing in for VU1/GS (render only), not the FPU
fn sample(tex: &[u8], s: f64, t: f64) -> u32 {
    let fix = |s: f64| ((s * TEX as f64 - 0.5) * 16.0).floor() as i64;
    let (u, v) = (fix(s), fix(t));
    let (u0, v0, fu, fv) = (u >> 4, v >> 4, (u & 15) as u32, (v & 15) as u32);
    let at = |x: i64, y: i64| (tex[x.clamp(0, TEX as i64 - 1) as usize + y.clamp(0, TEX as i64 - 1) as usize * TEX] as u32 * 0x20).min(0xff);
    (at(u0, v0) * (16 - fu) * (16 - fv) + at(u0 + 1, v0) * fu * (16 - fv) + at(u0, v0 + 1) * (16 - fu) * fv + at(u0 + 1, v0 + 1) * fu * fv) >> 8
}

/// Build the map: every caster's shadow texture is drawn along its light matrix onto the hole triangles its
/// receiver pass keeps ([`receiver_tris`] of the `hole` model, game space at load), added up without depth over a
/// screen viewing the hole from straight above, and the red kept above 0x6f.
///
/// ponytail: the screen is orthographic with columns at ¾ of the map's scale, fitted to the game's map (the
/// camera on paper is 40° perspective), the raster and sampling are worked in double precision, not the game's
/// VU1/GS, and every fogged cutout blends toward 0x7f (the game's court 10 seat and statue stay at 0x80, cause not
/// found): IoU ≈ 0.97 against the game's map on court 10 (rendering only has to look right).
#[allow(clippy::float_arithmetic)] // double-precision raster standing in for VU1/GS (render only), not the FPU
pub fn build(f: &Frame, casters: &[Caster], hole: &hst_data::mdl::Model, mats: &hst_data::mtl::Mtl) -> Vec<u8> {
    let texs: Vec<Vec<u8>> = casters.iter().map(texture).collect();
    let (sz, sx) = (f.scale[0] as f64, f.scale[1] as f64);
    let (cz, cx) = (f.origin[0] as f64 + 640.0 / sz, f.origin[1] as f64 + 448.0 / sx);
    let mut red = vec![0u32; COLS * ROWS];
    for (c, tex) in casters.iter().zip(&texs) {
        for t in receiver_tris(&c.planes, hole, mats) {
            let p = t.map(|p| p.map(f64::from));
            let v = p.map(|p| [((639.5 + 0.75 * sz * (p[2] - cz)) * 16.0).floor() as i64, ((447.5 + sx * (p[0] - cx)) * 16.0).floor() as i64]);
            raster(v, 16, COLS, ROWS, |col, r, w| {
                let g: [f64; 3] = std::array::from_fn(|k| w[0] * p[0][k] + w[1] * p[1][k] + w[2] * p[2][k]);
                let [u, v] = light_uv(&c.light, g);
                red[col + r * COLS] += sample(tex, 0.5 * u + 0.5, 0.5 * v + 0.5);
            });
        }
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
/// court's hole nodes are identity, which leaves `light` bit for bit. `light` is [`light_matrix`].
pub fn tex_matrix(light: &[vu0::V4; 4]) -> [vu0::V4; 4] {
    const B: [vu0::V4; 4] = [[0.5, 0.0, 0.0, 0.0], [0.0, 0.5, 0.0, 0.0], [0.0; 4], [0.5, 0.5, 1.0, 1.0]];
    // the EE sums the products as ((r1·b1 + r0·b0) + r2·b2) + r3·b3
    light.map(|r| {
        std::array::from_fn(|j| (F(r[1]) * B[1][j]).madd(r[0], B[0][j]).madd(r[2], B[2][j]).madd(r[3], B[3][j]).0)
    })
}

/// The sun at a match's start as the game sets it up, from the court's time-of-day row (`start`, `end` pitch,
/// `tilt`), the hole's `azimuth` (degrees) and the hour: the light (the third row of Rx(−pitch)·Rz(tilt)·Ry(−π/2 −
/// azimuth), normalized; game space, sun → ground) and the shade sun, rebuilt from the light's heading and
/// elevation with the elevation raised to at least 50°.
pub fn sun(start: f32, end: f32, tilt: f32, azimuth: f32, hour: i32) -> (vu0::V4, vu0::V4) {
    use crate::{libm, shot, world};
    let deg = f32::from_bits(0x3c8e_fa35); // π/180
    let t = (F((hour - 1) as f32) / 17.0).0;
    let pitch = (-(F::ZERO + start).madd(t, F(end) - start)).0;
    let mut yaw = (F::ZERO + -F(world::HALF_PI)).msub(deg, azimuth).0;
    if yaw > world::PI {
        yaw = (F(yaw) - world::TWO_PI).0;
    } else if yaw < -F(world::PI) {
        yaw = (F(world::TWO_PI) + yaw).0;
    }
    let m = world::mat_mul(&world::mat_mul(&world::mat_mul(&world::IDENTITY, &world::rot_x(pitch)), &shot::rot_z(tilt)), &world::rot_y(yaw));
    let [x, y, z, w] = m[2];
    let r = (1.0 / (F(y) * y).madd(x, x).madd(z, z).sqrt()).0;
    let light = [(F(x) * r).0, (F(y) * r).0, (F(z) * r).0, (F(w) * r).0];
    let [x, y, z, w] = light;
    let h = libm::atan2f(x, z);
    let e = libm::atan2f(y, (F(z) * libm::cosf(h)).madd(x, libm::sinf(h)).0);
    let e = e.max((F(deg) * 50.0).0);
    let c = libm::cosf(e);
    (light, [(F(c) * libm::sinf(h)).0, libm::sinf(e), (F(c) * libm::cosf(h)).0, w])
}

/// The light axes the game derives from the sun (unit, sun → ground) at load: lu = (0, 0, 1) × sun normalized (or
/// (1, 0, 0) when the sun is near vertical in z), lv = sun × lu, and the sun itself. Rows of [`light_matrix`].
pub fn sun_axes(sun: vu0::V4) -> [vu0::V4; 3] {
    let [x, y, z, _] = sun;
    let lu = if z.abs() < 0.99999 {
        let k = [0.0, 0.0, 1.0];
        let c = [(F(k[1]) * z).msub(k[2], y).0, (F(k[2]) * x).msub(k[0], z).0, (F(k[0]) * y).msub(k[1], x).0];
        let r = (1.0 / (F(c[1]) * c[1]).madd(c[0], c[0]).madd(c[2], c[2]).sqrt()).0;
        [(F(c[0]) * r).0, (F(c[1]) * r).0, (F(c[2]) * r).0, (F::ZERO * r).0]
    } else {
        [1.0, 0.0, 0.0, 0.0]
    };
    let lv = [(F(y) * lu[2]).msub(z, lu[1]).0, (F(z) * lu[0]).msub(x, lu[2]).0, (F(x) * lu[1]).msub(y, lu[0]).0, 0.0];
    [lu, lv, sun]
}

/// A caster's light matrix (world → light space, u/v in ±1 over the caster, z along the sun) as the game builds
/// it at load in EE floats: `item` is the caster's model matrix (scaled by `scale`), `centre`/`half` its model
/// box, `axes` the [`sun_axes`]. The matrix is the inverse of [lu; lv; sun; box centre] scaled to the caster's
/// extent across the light.
pub fn light_matrix(item: &[vu0::V4; 4], scale: f32, centre: [f32; 3], half: [f32; 3], axes: &[vu0::V4; 3]) -> [vu0::V4; 4] {
    let (w, o) = box_frame(item, scale, centre);
    let a: [vu0::V4; 3] = std::array::from_fn(|i| w[i].map(|x| (F(x) * half[i]).0));
    // |a0·l| + |a1·l|, then |a2·l| + that; each dot as (y·y + x·x) + z·z of a and l
    let reach = |l: vu0::V4| {
        let d = |v: vu0::V4| dot_yxz(v, l).abs();
        (d(a[2]) + (F(d(a[0])) + d(a[1]))).0
    };
    let (su, sv) = (reach(axes[0]), reach(axes[1]));
    let s = (F(1.0) / if su < sv { sv } else { su }).0;
    let o = o.map(|x| (F(x) * s).0);
    let r = (F(1.0) / o[3]).0;
    inverse(&[axes[0], axes[1], axes[2], o].map(|v| v.map(|x| (F(x) * r).0)))
}

/// The EE's (a.y·b.y + a.x·b.x) + a.z·b.z (mula, madda, madd).
fn dot_yxz(a: vu0::V4, b: vu0::V4) -> f32 {
    (F(a[1]) * b[1]).madd(a[0], b[0]).madd(a[2], b[2]).0
}

/// The caster's matrix over its scale (`w`) and its box centre through it (`o`, w last), as the load-time setup
/// works them out for both [`light_matrix`] and [`planes`].
fn box_frame(item: &[vu0::V4; 4], scale: f32, centre: [f32; 3]) -> ([vu0::V4; 4], vu0::V4) {
    let ws = (F(1.0) / scale).0;
    let w = item.map(|r| r.map(|x| (F(x) * ws).0));
    let [cx, cy, cz] = centre;
    let c: [f32; 4] =
        std::array::from_fn(|k| (F(cy) * w[1][k]).madd(cx, w[0][k]).madd(cz, w[2][k]).madd(0.0, w[3][k]).0);
    let f = (F(1.0) / w[3][3]).0;
    let mut o = [0.0; 4];
    for k in 0..3 {
        o[k] = (F::ZERO + w[3][k]).madd(F(c[k]) * f, w[3][3]).0;
    }
    o[3] = (F(w[3][3]) + 0.0).0;
    (w, o)
}

/// A caster's shadow-volume [`Planes`] as the game builds them at load next to its [`light_matrix`] (same
/// arguments), in EE floats. Its box's half axes (over w) are turned away from the sun (`axes[2]`), each moved
/// along the sun by its dot with it (pᵢ); the silhouette is the hexagon ±p0 ±p1 ±p2 around the box centre. Plane 0 is
/// the cap: the sun with its y dropped (unless |y| ≥ 0.99), normalized, through the centre + the three half axes;
/// then a side plane per hexagon edge, sun × edge facing away from the hexagon, through the edge's start (skipped
/// when the edge's plane passes within 1e-7 of the centre; unused slots (0, 0, 0, 1e18)); last the sun's own plane,
/// through the centre pushed back by Σ|pᵢ·sun|.
pub fn planes(item: &[vu0::V4; 4], scale: f32, centre: [f32; 3], half: [f32; 3], axes: &[vu0::V4; 3]) -> Planes {
    let (w, o) = box_frame(item, scale, centre);
    let s = axes[2];
    let r = (F(1.0) / o[3]).0;
    let o = o.map(|x| (F(x) * r).0);
    let f = (F(1.0) / w[3][3]).0;
    let neg = |v: vu0::V4| v.map(|x| (F(x) * -1.0).0);
    let mut d = [0.0; 3];
    let a: [vu0::V4; 3] = std::array::from_fn(|i| {
        let a = w[i].map(|x| (F(x) * half[i] * f).0);
        d[i] = dot_yxz(a, s);
        if d[i] > 0.0 {
            d[i] = (F(d[i]) * -1.0).0;
            return neg(a);
        }
        a
    });
    let corner: vu0::V4 = std::array::from_fn(|k| (F(o[k]) + a[0][k] + a[1][k] + a[2][k]).0);
    let p: [vu0::V4; 3] = std::array::from_fn(|i| {
        let mut p: vu0::V4 = std::array::from_fn(|k| (a[i][k] + F(s[k]) * d[i]).0);
        p[0] = (F::ZERO + a[i][0]).madd(s[0], d[i]).0;
        p
    });
    let h = |g: fn(f32, f32, f32) -> f32| -> vu0::V4 { std::array::from_fn(|k| g(p[0][k], p[1][k], p[2][k])) };
    let hex = [
        h(|a, b, c| (F(a) + b - c).0),
        h(|a, b, c| (-F(a) + b - c).0),
        h(|a, b, c| (-F(a) + b + c).0),
        h(|a, b, c| (-F(a) - b + c).0),
        h(|a, b, c| (F(a) - b + c).0),
        h(|a, b, c| (F(a) - b - c).0),
    ];
    let unit = |n: vu0::V4| {
        let r = (1.0 / F(dot_yxz(n, n)).sqrt()).0;
        n.map(|x| (F(x) * r).0)
    };
    let mut cap = s;
    if cap[1].abs() < f32::from_bits(0x3f7d_70a4) {
        cap[1] = 0.0; // 0.99
    }
    let cap = unit(cap);
    let mut out = [[0.0, 0.0, 0.0, f32::from_bits(0x5d5e_0b6b)]; 8]; // 1e18
    out[0] = [cap[0], cap[1], cap[2], (-F(vu0::dot3(cap, corner))).0];
    let mut n_out = 1;
    let mut prev = hex[5];
    for cur in hex {
        let e: [f32; 3] = std::array::from_fn(|k| (F(cur[k]) - prev[k]).0);
        let mut n = [
            (F(s[1]) * e[2]).msub(s[2], e[1]).0,
            (F(s[2]) * e[0]).msub(s[0], e[2]).0,
            (F(s[0]) * e[1]).msub(s[1], e[0]).0,
            0.0,
        ];
        let dd = vu0::dot3(n, prev);
        if dd.abs() > f32::from_bits(0x33d6_bf95) {
            // 1e-7
            if dd > 0.0 {
                n = neg(n);
            }
            let n = unit(n);
            let at: vu0::V4 = std::array::from_fn(|k| (F(prev[k]) + o[k]).0);
            out[n_out] = [n[0], n[1], n[2], (-F(vu0::dot3(n, at))).0];
            n_out += 1;
        }
        prev = cur;
    }
    let back = d.iter().fold((-F(dot_yxz(s, o))).0, |acc, d| (F(d.abs()) + acc).0);
    out[7] = [s[0], s[1], s[2], back];
    std::array::from_fn(|b| std::array::from_fn(|r| std::array::from_fn(|j| out[4 * b + j][r])))
}

/// The game's general 4×4 inverse in EE floats: the 2×2 minors' determinant, the 16 cofactors in the game's op order
/// (generated from its code), each times 1/det; the identity when det is 0.
fn inverse(m: &[vu0::V4; 4]) -> [vu0::V4; 4] {
    let m: [F; 16] = std::array::from_fn(|i| F(m[i / 4][i % 4]));
    let p = |a: usize, b: usize, c: usize, d: usize| (m[a] * m[b]).msub(m[c], m[d]);
    let det = (p(0, 5, 1, 4) * p(10, 15, 11, 14)).msub(p(0, 6, 2, 4), p(9, 15, 11, 13));
    let det = det
        .madd(p(0, 7, 3, 4), p(9, 14, 10, 13))
        .madd(p(1, 6, 2, 5), p(8, 15, 11, 12))
        .msub(p(1, 7, 3, 5), p(8, 14, 10, 12))
        .madd(p(2, 7, 3, 6), p(8, 13, 9, 12));
    if det == 0.0 {
        return std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1.0 } else { 0.0 }));
    }
    let r = F(1.0) / det;
    let t2 = m[2] * m[13];
    let t3 = m[0] * m[13];
    let t4 = m[0] * m[5];
    let t5 = m[1] * m[12];
    let t6 = m[1] * m[4];
    let t7 = m[10] * m[12];
    let t8 = m[8] * m[14];
    let t9 = m[9] * m[14];
    let t10 = m[10] * m[13];
    let t11 = m[8] * m[13];
    let t12 = m[9] * m[12];
    let t13 = m[5] * m[11];
    let t14 = m[7] * m[9];
    let t15 = m[7] * m[8];
    let t16 = m[4] * m[11];
    let t17 = m[3] * m[5];
    let t18 = m[1] * m[7];
    let t19 = m[0] * m[7];
    let t20 = m[3] * m[4];
    let t21 = m[3] * m[13];
    let t22 = m[1] * m[15];
    let t23 = m[0] * m[15];
    let t24 = m[3] * m[12];
    let t25 = m[11] * m[13];
    let t26 = m[9] * m[15];
    let t27 = m[8] * m[15];
    let t28 = m[11] * m[12];
    let t29 = m[7] * m[10];
    let t30 = m[6] * m[11];
    let t31 = m[2] * m[7];
    let t32 = m[3] * m[6];
    let t33 = m[2] * m[15];
    let t34 = m[3] * m[14];
    let t35 = m[10] * m[15];
    let t36 = m[11] * m[14];
    let t37 = m[0] * m[14];
    let t38 = t9 - t10;
    let t39 = m[1] * m[14];
    let t40 = t10 - t9;
    let t41 = m[1] * m[6];
    let t42 = t35 - t36;
    let t43 = m[2] * m[5];
    let t44 = t36 - t35;
    let t45 = m[6] * m[9];
    let t46 = t25 - t26;
    let t47 = m[5] * m[10];
    let t48 = t26 - t25;
    let t49 = m[2] * m[12];
    let t50 = m[6] * t46;
    let t51 = m[0] * m[6];
    let t52 = (F::ZERO + t50).madd(m[5], t42);
    let t53 = m[2] * m[4];
    let t54 = (F::ZERO + t52).madd(m[7], t38);
    let t55 = m[4] * m[10];
    let t56 = m[4] * t48;
    let t57 = m[6] * m[8];
    let t58 = t39 - t2;
    let t59 = m[5] * m[8];
    let t60 = t2 - t39;
    let t61 = m[4] * m[9];
    let t62 = t33 - t34;
    let t63 = t34 - t33;
    let t64 = t21 - t22;
    let t65 = m[10] * t64;
    let t66 = t22 - t21;
    let t67 = m[8] * t66;
    let t68 = t41 - t43;
    let t69 = (F::ZERO + t65).madd(m[9], t62);
    let t70 = t43 - t41;
    let t71 = (F::ZERO + t69).madd(m[11], t58);
    let t72 = t31 - t32;
    let t73 = t32 - t31;
    let t74 = t37 - t49;
    let t75 = t17 - t18;
    let t76 = t18 - t17;
    let t77 = m[14] * t75;
    let t78 = (F::ZERO + t77).madd(m[13], t72);
    let t79 = (F::ZERO + t78).madd(m[15], t68);
    let t80 = m[12] * t76;
    let t81 = t45 - t47;
    let t82 = t47 - t45;
    let t83 = t29 - t30;
    let t84 = t30 - t29;
    let t85 = t13 - t14;
    let t86 = t14 - t13;
    let t87 = m[2] * t85;
    let t88 = (F::ZERO + t87).madd(m[1], t83);
    let t89 = (F::ZERO + t88).madd(m[3], t81);
    let t90 = m[0] * t86;
    let t91 = t27 - t28;
    let t92 = t28 - t27;
    let t93 = t7 - t8;
    let t94 = t8 - t7;
    let t95 = m[7] * t93;
    let t96 = (F::ZERO + t95).madd(m[6], t91);
    let t97 = m[5] * t94;
    let t98 = (F::ZERO + t96).madd(m[4], t44);
    let t99 = t23 - t24;
    let t100 = t24 - t23;
    let t101 = t49 - t37;
    let t102 = m[11] * t101;
    let t103 = (F::ZERO + t102).madd(m[10], t99);
    let t104 = m[9] * t74;
    let t105 = (F::ZERO + t103).madd(m[8], t63);
    let t106 = t19 - t20;
    let t107 = t20 - t19;
    let t108 = t53 - t51;
    let t109 = m[15] * t108;
    let t110 = (F::ZERO + t109).madd(m[14], t106);
    let t111 = t51 - t53;
    let t112 = (F::ZERO + t110).madd(m[12], t73);
    let t113 = m[13] * t111;
    let t114 = t15 - t16;
    let t115 = t16 - t15;
    let t116 = t55 - t57;
    let t117 = t57 - t55;
    let t118 = m[3] * t116;
    let t119 = (F::ZERO + t118).madd(m[2], t114);
    let t120 = m[1] * t117;
    let t121 = (F::ZERO + t119).madd(m[0], t84);
    let t122 = t11 - t12;
    let t123 = (F::ZERO + t56).madd(m[7], t122);
    let t124 = (F::ZERO + t123).madd(m[5], t92);
    let t125 = t12 - t11;
    let t126 = t3 - t5;
    let t127 = (F::ZERO + t67).madd(m[11], t126);
    let t128 = (F::ZERO + t127).madd(m[9], t100);
    let t129 = t5 - t3;
    let t130 = t4 - t6;
    let t131 = (F::ZERO + t80).madd(m[15], t130);
    let t132 = (F::ZERO + t131).madd(m[13], t107);
    let t133 = t59 - t61;
    let t134 = (F::ZERO + t90).madd(m[3], t133);
    let t135 = (F::ZERO + t134).madd(m[1], t115);
    let t136 = (F::ZERO + t97).madd(m[4], t40);
    let t137 = (F::ZERO + t136).madd(m[6], t125);
    let t138 = t6 - t4;
    let t139 = (F::ZERO + t104).madd(m[8], t60);
    let t140 = (F::ZERO + t139).madd(m[10], t129);
    let t141 = t61 - t59;
    let t142 = (F::ZERO + t113).madd(m[12], t70);
    let t143 = (F::ZERO + t142).madd(m[14], t138);
    let t144 = (F::ZERO + t120).madd(m[0], t82);
    let t145 = (F::ZERO + t144).madd(m[2], t141);
    let o = [t54, t71, t79, t89, t98, t105, t112, t121, t124, t128, t132, t135, t137, t140, t143, t145];
    std::array::from_fn(|i| std::array::from_fn(|j| (o[4 * i + j] * r).0))
}

/// One shade receiver vertex as the game's VU1 program sends it to the GS: screen X, Y (12.4 fixed point) and
/// S, T, Q. `item` is the receiver model's matrix (court hole: y flattened), `vp` the shade camera's world →
/// screen matrix (40°, straight down), `tex` the caster's [`tex_matrix`], `p` the model-space vertex. The (s, t, w)
/// the game precomputes per vertex at load (VU0) are multiplied by Q = 1/w of the screen position, so Q itself
/// carries the light's w (1 ± an ulp or two).
pub fn receiver_vertex(item: &[vu0::V4; 4], vp: &[vu0::V4; 4], tex: &[vu0::V4; 4], p: [f32; 3]) -> ([u16; 2], [f32; 3]) {
    let v = [p[0], p[1], p[2], 1.0];
    let clip = vu0::transform(&item.map(|r| vu0::transform(vp, r)), v);
    let q = (Vf(1.0) / clip[3]).0;
    let st = vu0::transform(tex, v);
    let ftoi4 = |x: f32| (Vf(x) * q * 16.0).0 as i32 as u16; // FTOI4 truncates
    ([ftoi4(clip[0]), ftoi4(clip[1])], [(Vf(st[0]) * q).0, (Vf(st[1]) * q).0, (Vf(st[2]) * q).0])
}

/// A caster's shadow-volume planes as the game keeps them: two blocks of four planes, row k of a block holding
/// component k (nx, ny, nz, d) of each; block 0 is the cap and sides 1–3, block 1 sides 4–6 and the plane facing
/// the sun. A point is inside a plane where d + n·p > 0.
pub type Planes = [[vu0::V4; 4]; 2];

/// `planes` taken into a receiver node's space by its matrix `m` (VU0: row k = P0·Mk.x + P1·Mk.y + P2·Mk.z +
/// P3·Mk.w). The hole's nodes are identity.
pub fn node_planes(planes: &Planes, m: &[vu0::V4; 4]) -> Planes {
    planes.map(|p| {
        m.map(|r| {
            std::array::from_fn(|j| (Vf(p[0][j]) * r[0]).madd(p[1][j], r[1]).madd(p[2][j], r[2]).madd(p[3][j], r[3]).0)
        })
    })
}

/// The game's box test (VU0): a box is out when for some plane ((((((d + |nz|·hz) + |ny|·hy) + |nx|·hx) + nz·cz)
/// + ny·cy) + nx·cx) ≤ 0, −0 included.
pub fn box_out(p: &Planes, centre: [f32; 3], half: [f32; 3]) -> bool {
    p.iter().any(|b| {
        (0..4).any(|j| {
            let v = [
                (b[2][j].abs(), half[2]),
                (b[1][j].abs(), half[1]),
                (b[0][j].abs(), half[0]),
                (b[2][j], centre[2]),
                (b[1][j], centre[1]),
                (b[0][j], centre[0]),
            ];
            v.iter().fold(b[3][j], |a, &(n, x)| Vf(a).madd(n, x).0) <= 0.0
        })
    })
}

/// Per plane, whether `v` is outside it: ((d + nz·z) + ny·y) + nx·x is negative or −0.
fn outside(p: &Planes, v: [f32; 3]) -> [bool; 8] {
    std::array::from_fn(|i| {
        let b = &p[i / 4];
        let j = i % 4;
        Vf(b[3][j]).madd(b[2][j], v[2]).madd(b[1][j], v[1]).madd(b[0][j], v[0]).0.is_sign_negative()
    })
}

/// Whether the triangle a strip vertex closes is culled: all three corners outside one plane, or, with a corner
/// outside the cap, its normal ((v2 − v1)·−w × (v3 − v1), `w` the closing vertex's winding) facing away from the
/// sun plane's normal (the dot below zero).
fn tri_out(p: &Planes, v1: [f32; 3], v2: [f32; 3], v3: [f32; 3], w: f32) -> bool {
    let s = [v1, v2, v3].map(|v| outside(p, v));
    if (0..8).any(|k| s.iter().all(|s| s[k])) {
        return true;
    }
    if !s.iter().any(|s| s[0]) {
        return false;
    }
    let a: [f32; 3] = std::array::from_fn(|k| ((Vf(v2[k]) - v1[k]) * -Vf(w)).0);
    let b: [f32; 3] = std::array::from_fn(|k| (Vf(v3[k]) - v1[k]).0);
    let n = [
        (Vf(a[1]) * b[2]).msub(b[1], a[2]).0,
        (Vf(a[2]) * b[0]).msub(b[2], a[0]).0,
        (Vf(a[0]) * b[1]).msub(b[0], a[1]).0,
    ];
    let sun = &p[1];
    (Vf(sun[2][3]) * n[2]).madd(sun[1][3], n[1]).madd(sun[0][3], n[0]).0 < 0.0
}

/// One packet's strip as a caster's receiver pass draws it, the game's load-time cull: each vertex closes a
/// triangle (none while `flags` bit 15, no-kick, is set) that is tested ([`tri_out`]); on a third culled one in a
/// row the previous vertex and this one are kept as no-kick, else the vertex is kept with its own flag; culled
/// vertices left at the end are dropped. Returns the kept vertices and their no-kick flags.
pub fn receiver_strip(p: &Planes, pos: &[[f32; 3]], flags: &[u16], uv_w: &[f32]) -> Vec<([f32; 3], bool)> {
    let (mut out, mut h): (Vec<([f32; 3], bool)>, u32) = (Vec::new(), 0);
    for (i, &v) in pos.iter().enumerate() {
        let skip = flags[i] & 0x8000 != 0; // a strip's first two vertices always are
        h = (h & 3) << 1 | (skip || i < 2 || tri_out(p, pos[i - 2], pos[i - 1], v, uv_w[i])) as u32;
        if h == 7 {
            h = 3;
            let n = out.len();
            out[n - 2] = (out[n - 1].0, true);
            out[n - 1] = (v, true);
        } else {
            out.push((v, skip));
        }
    }
    while h & 1 != 0 {
        out.pop();
        h >>= 1;
    }
    out
}

/// The hole triangles (model space) a caster's receiver pass draws: the ones the game collides with (static
/// batches of attribute-mapped materials), every packet whose box (header centre, half the bounds' extent) isn't
/// out ([`box_out`]), through [`receiver_strip`].
///
/// ponytail: the game also tests each node's and batch's box first and skips materials whose lighting flags or
/// colour differ; on court 10 neither drops a triangle the packet test keeps (P17v6).
pub fn receiver_tris(p: &Planes, hole: &hst_data::mdl::Model, mats: &hst_data::mtl::Mtl) -> Vec<[[f32; 3]; 3]> {
    let p = &node_planes(p, &[[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
    let mut tris = Vec::new();
    let hits = |m: usize| mats.materials.get(m).is_some_and(|m| m.attributes.is_some());
    for pk in hole.materials.iter().enumerate().filter(|&(m, _)| hits(m)).flat_map(|(_, ps)| ps).filter(|pk| pk.fixed && pk.flags & 8 != 0) {
        let [lo, hi] = pk.bounds;
        if box_out(p, pk.centre, std::array::from_fn(|k| ((F(hi[k]) - lo[k]) * 0.5).0)) {
            continue;
        }
        let pos: Vec<[f32; 3]> = pk.vertices.iter().map(|v| v.pos).collect();
        let o = receiver_strip(p, &pos, &pk.entry_flags, &pk.uv_w);
        tris.extend((2..o.len()).filter(|&i| !o[i].1).map(|i| [o[i - 2].0, o[i - 1].0, o[i].0]));
    }
    tris
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
/// mantissa bits, more by how far their exponent is below Q's (`q`'s biased exponent; 127 for Q = 1).
fn texel_round(st: f32, q: i32) -> f32 {
    let (b, e) = (st.to_bits(), (st.to_bits() >> 23 & 0xff) as i32);
    f32::from_bits(b & !((1u32 << (9 + e.max(q) - e).min(23)) - 1))
}

/// Integer coordinates along a span from `left`: truncated at the span start, then per lane its truncated offset
/// from the aligned group of [`LANES`] at or before `left` and whole truncated group steps.
#[allow(clippy::float_arithmetic)] // PCSX2's software GS: x86 SSE singles (round to nearest), not the EE FPU
fn pass_lanes(t0: f32, d: f32, left: i32, n: i32) -> impl Iterator<Item = i32> {
    let (skip, step) = (left % LANES, (d * LANES as f32) as i32);
    (skip..skip + n).map(move |j| (t0 as i32).wrapping_add((d * (j % LANES - skip) as f32) as i32).wrapping_add(j / LANES * step))
}

/// The Gouraud channel (8.7 fixed point) along a span: 16-bit lanes from the truncated start, kept ≥ 0 per group step.
#[allow(clippy::float_arithmetic)] // PCSX2's software GS: x86 SSE singles (round to nearest), not the EE FPU
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

/// One triangle into the target: the vertices' (s, t, colour) as the GS's software setup takes them, then
/// [`gs_tri`]'s spans sampled and written where the destination alpha test and the alpha reference pass.
#[allow(clippy::float_arithmetic)] // PCSX2's software GS: x86 SSE singles (round to nearest), not the EE FPU
fn pass_tri(target: &mut [u8], d: &PassDraw, t: &[PassVertex; 3]) {
    let tsz = d.tex.as_ref().map_or([0.0; 2], |x| x.log2.map(|l| (0x10000u32 << l) as f32));
    let v: [[f32; 5]; 3] = t.map(|p| {
        let st = p.st.map(|c| texel_round(c, 127));
        [p.xy[0] as f32 * (1.0 / 16.0), p.xy[1] as f32 * (1.0 / 16.0), st[0] * tsz[0] - 32768.0, st[1] * tsz[1] - 32768.0, ((p.a as u32) << 7) as f32]
    });
    let flat = t[2].a as i32;
    gs_tri(v, SCISSOR, |y, left, right, tc, dscan| {
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
    });
}

/// One triangle with PCSX2's software setup: vertices (x, y, then three attributes) sorted by y, the edge and scan
/// gradients from the cross product, spans [left, right) from ceil(edge x) at each row inside the scissor, and the
/// attributes at the span start interpolated from the top vertex of its section, handed to `span` with the scan
/// gradient.
#[allow(clippy::float_arithmetic)] // PCSX2's software GS: x86 SSE singles (round to nearest), not the EE FPU
fn gs_tri(v: [[f32; 5]; 3], scissor: [i32; 4], mut span: impl FnMut(i32, i32, i32, [f32; 5], [f32; 5])) {
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
    let slope = |a: [f32; 5]| {
        if a[1] != 0.0 { a[0] / a[1] } else { f32::INFINITY }
    };
    let dd = [slope(dv0), dv1[0] / dv1[1], slope(dv2)];
    let c = [dv0[0], dv0[1], dv1[0], dv1[1]].map(|x| x / cross);
    let dscan: [f32; 5] = std::array::from_fn(|k| dv1[k] * c[1] - dv0[k] * c[3]);
    let dedge: [f32; 5] = std::array::from_fn(|k| dv0[k] * c[2] - dv1[k] * c[0]);
    let ceil = |y: f32| y.ceil() as i32;
    let mut section = |top: i32, bottom: i32, ex: [f32; 2], dex: [f32; 2], p0: [f32; 5]| {
        for y in top.max(scissor[1])..bottom.min(scissor[3]) {
            let dy = y as f32 - p0[1];
            let left = ceil(ex[0] + dex[0] * dy).max(scissor[0]);
            let right = ceil(ex[1] + dex[1] * dy).min(scissor[2]);
            if right <= left {
                continue;
            }
            let pre = left as f32 - p0[0];
            let tc: [f32; 5] = std::array::from_fn(|k| (p0[k] + dedge[k] * dy) + dscan[k] * pre);
            span(y, left, right, tc, dscan);
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

/// A receiver vertex as the GS gets it: x, y in 12.4 relative to the tile's XYOFFSET, and S, T, Q.
#[derive(Clone, Copy, Debug)]
pub struct ReceiverVertex {
    pub xy: [i32; 2],
    pub stq: [f32; 3],
}

/// One draw (one GS batch) of the receiver pass: a caster's 128² shadow texture, its red through the CLUT in
/// `alpha` and clamped at its edges, over the hole triangles it shades.
pub struct ReceiverDraw<'a> {
    pub tex: &'a PassTexture,
    pub tris: Vec<[ReceiverVertex; 3]>,
}

/// One 640 × 224 tile of the shade map's 1280 × 896 frame: the receiver pass's reds as PCSX2's software GS draws them.
/// Each draw's bilinear red is added to the frame, clamped at 255 (ALPHA Cs + Cd, COLCLAMP). Before the
/// setup the GS rounds S, T and Q (the draw's Z is constant); when Q is the same over the whole batch, S/Q and T/Q are
/// taken per vertex and stepped as integers like the texture pass's, else divided per pixel, Q stepped per group of
/// [`LANES`].
#[allow(clippy::float_arithmetic)] // PCSX2's software GS: x86 SSE singles (round to nearest), not the EE FPU
pub fn receiver_tile(draws: &[ReceiverDraw]) -> Vec<u8> {
    let mut fb = vec![0u8; 640 * 224];
    for d in draws {
        let q0 = d.tris[0][0].stq[2];
        let const_q = d.tris.iter().flatten().all(|p| p.stq[2] == q0);
        for t in &d.tris {
            let v: [[f32; 5]; 3] = t.map(|p| {
                let qe = (p.stq[2].to_bits() >> 23 & 0xff) as i32;
                let [s, t] = [p.stq[0], p.stq[1]].map(|c| texel_round(c, qe));
                let q = f32::from_bits(p.stq[2].to_bits() & !0xff);
                let tsz = (0x10000u32 << 7) as f32;
                let (x, y) = (p.xy[0] as f32 * (1.0 / 16.0), p.xy[1] as f32 * (1.0 / 16.0));
                match (const_q, q != 1.0) {
                    (true, true) => [x, y, s / q * tsz - 32768.0, t / q * tsz - 32768.0, q],
                    (true, false) => [x, y, s * tsz - 32768.0, t * tsz - 32768.0, q],
                    (false, _) => [x, y, s * tsz, t * tsz, q],
                }
            });
            gs_tri(v, [0, 0, 640, 224], |y, left, right, tc, ds| {
                let n = right - left;
                let uv: Vec<(i32, i32)> = if const_q {
                    pass_lanes(tc[2], ds[2], left, n).zip(pass_lanes(tc[3], ds[3], left, n)).collect()
                } else {
                    let skip = left % LANES;
                    let mut g: [[f32; 3]; 4] = std::array::from_fn(|k| std::array::from_fn(|c| tc[2 + c] + ds[2 + c] * (k as i32 - skip) as f32));
                    let step: [f32; 3] = std::array::from_fn(|c| ds[2 + c] * LANES as f32);
                    let mut out = Vec::new();
                    for j in skip..skip + n {
                        if j % LANES == 0 && j > 0 {
                            g.iter_mut().for_each(|l| (0..3).for_each(|c| l[c] += step[c]));
                        }
                        let [s, t, q] = g[(j % LANES) as usize];
                        out.push(((s / q) as i32 - 0x8000, (t / q) as i32 - 0x8000));
                    }
                    out
                };
                for (x, (u, v)) in (left..right).zip(uv) {
                    let dst = &mut fb[x as usize + y as usize * 640];
                    *dst = (*dst as i32 + pass_sample(d.tex, u, v)).min(255) as u8;
                }
            });
        }
    }
    fb
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
