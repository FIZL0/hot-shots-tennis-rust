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

use crate::{mesh, ps2};

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

/// The ball's height above the ground for [`ball`], from its game position (y down). On the court (|x| ≤ 10.685,
/// |z| ≤ 19.885) the ground is y 0; off it the game casts a ray 200 units straight down against the court model
/// (`court`), and `None` when it misses (the ball keeps its last light scale).
pub fn ball_height(court: &mesh::Object, models: &[mesh::Model], pos: [f32; 4]) -> Option<f32> {
    let [x, y, z, _] = pos;
    let ground = if x.abs() <= 10.685 && z.abs() <= 19.885 {
        0.0
    } else {
        let mut hit = mesh::Hit::NONE;
        if !court.ray(models, &mut hit, pos, [x, ps2::add(y, 200.0), z, pos[3]]) {
            return None;
        }
        hit.centre[1]
    };
    Some(-ps2::sub(y, ground))
}

/// A shadow caster: its model's triangles (model space) and its placement, model axes (scaled, game space) and
/// position.
pub struct Caster {
    pub axes: [[f32; 3]; 3],
    pub pos: [f32; 3],
    pub tris: Vec<[[f32; 3]; 3]>,
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
        for p in c.tris.iter().flatten() {
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

/// A caster's shadow texture: its triangles drawn solid in light space at twice the size, every other pixel kept.
fn texture(c: &Caster, l: &Light) -> Vec<bool> {
    let mut tex = vec![false; TEX * TEX];
    let axes = c.axes.map(|a| a.map(f64::from));
    for t in &c.tris {
        let v = t.map(|p| {
            let g = add((0..3).fold([0.0; 3], |s, k| add(s, scale(axes[k], p[k] as f64))), c.pos.map(f64::from));
            l.uv(g).map(|u| ((128.0 * u + 126.0) * 16.0).floor() as i64)
        });
        raster(v, 32, TEX, TEX, |x, y, _| tex[x + y * TEX] = true);
    }
    tex
}

/// The GS's bilinear read (4-bit fraction, clamped) of a shadow texture (red 0 or 0xff) at (s, t) in 0..1.
fn sample(tex: &[bool], s: f64, t: f64) -> u32 {
    let fix = |s: f64| ((s * TEX as f64 - 0.5) * 16.0).floor() as i64;
    let (u, v) = (fix(s), fix(t));
    let (u0, v0, fu, fv) = (u >> 4, v >> 4, (u & 15) as u32, (v & 15) as u32);
    let at = |x: i64, y: i64| tex[x.clamp(0, TEX as i64 - 1) as usize + y.clamp(0, TEX as i64 - 1) as usize * TEX] as u32;
    let k = at(u0, v0) * (16 - fu) * (16 - fv) + at(u0 + 1, v0) * fu * (16 - fv) + at(u0, v0 + 1) * (16 - fu) * fv + at(u0 + 1, v0 + 1) * fu * fv;
    0xff * k / 256
}

/// Build the map: every caster's shadow texture is drawn along the sun `dir` (unit, sun → ground) onto each
/// hole `ground` triangle (game space) whose corners aren't all outside one side of the caster's shadow volume,
/// added up without depth over a screen viewing the hole from straight above, and the red kept above 0x6f.
///
/// ponytail: the screen is orthographic with columns at ¾ of the map's scale, fitted to the game's map (the
/// camera on paper is 40° perspective); the casters are drawn solid (no leaf alpha) and the setup is worked in
/// double precision, not the game's VU1 floats: IoU 0.947 against the game's map on court 10 (P17u leaf alpha, P17v exact).
pub fn build(f: &Frame, dir: [f32; 3], casters: &[Caster], ground: &[[[f32; 3]; 3]]) -> Vec<u8> {
    let d = dir.map(f64::from);
    let lights: Vec<Light> = casters.iter().map(|c| Light::new(c, d)).collect();
    let texs: Vec<Vec<bool>> = casters.iter().zip(&lights).map(|(c, l)| texture(c, l)).collect();
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
