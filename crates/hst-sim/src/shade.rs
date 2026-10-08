//! The court's sun-shade map: in clear and cloudy weather the game darkens the ball and the NPCs in the shade of
//! the court's shadow casters.
//!
//! - At load it views the hole model from straight above (a 40° camera over a 1280 × 896 screen that just takes in
//!   the model's box) and draws the shadows; every pixel whose red is above 0x6f is shade, one bit per pixel, rows
//!   of 0x500 bits (game z), 0x380 rows (game x), least significant bit first ([`Frame`], [`rasterize`]).
//! - Each frame the ball's and the NPCs' light scale is the bilinear blend of the four map pixels around their
//!   (x, z), 0 where shade and 1 where not ([`lookup`]); 1.0 off the map, on court 7 and in rain. The ball's is at
//!   least its height above the ground, so 1.0 from a unit up ([`ball`]); off the court the height comes from a
//!   ray cast down at the court model ([`ball_height`]). The players are never shaded. The scale multiplies the
//!   model's directional light colour (VU1), not the ambient.
//!
//! ponytail: [`rasterize`] casts the casters' exact silhouettes (no texture alpha, no shadow-texture resolution or
//! filtering, binary instead of the red > 0x6f threshold); the game draws its shadow textures onto the hole's
//! ground through the GS and reads the frame back (P17r).

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

/// Fill every pixel centre inside triangle `t` (map space (column, row), with a value per corner) over a
/// `cols` × `rows` grid: `put(index, value interpolated there)`.
fn fill(t: [[f32; 3]; 3], cols: usize, rows: usize, mut put: impl FnMut(usize, f32)) {
    let [a, b, c] = t;
    let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if area == 0.0 || !area.is_finite() {
        return;
    }
    let edge = |p: [f32; 3], q: [f32; 3], x: f32, y: f32| ((q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0])) / area;
    let span = |k: usize, n: usize| {
        let lo = a[k].min(b[k]).min(c[k]).floor().max(0.0) as usize;
        let hi = (a[k].max(b[k]).max(c[k]).ceil().max(0.0) as usize).min(n);
        lo..hi
    };
    for r in span(1, rows) {
        for col in span(0, cols) {
            let (x, y) = (col as f32 + 0.5, r as f32 + 0.5);
            let (wa, wb, wc) = (edge(b, c, x, y), edge(c, a, x, y), edge(a, b, x, y));
            if wa >= 0.0 && wb >= 0.0 && wc >= 0.0 {
                put(col + r * cols, wa * a[2] + wb * b[2] + wc * c[2]);
            }
        }
    }
}

/// Mark where the hole's ground (`ground`, game-space triangles; the top-most surface seen from above) lies in
/// the shadow of `casters` cast along `dir` (sun → ground, y down). Like a projected shadow texture there is no
/// depth: a ground point is shaded when its line to the sun crosses a caster, wherever it is along the line.
pub fn rasterize(map: &mut [u8], f: &Frame, dir: [f32; 3], casters: impl IntoIterator<Item = [[f32; 3]; 3]>, ground: impl IntoIterator<Item = [[f32; 3]; 3]>) {
    let px = |x: f32, z: f32| [(z - f.origin[0]) * f.scale[0], (x - f.origin[1]) * f.scale[1]];
    let mut height = vec![f32::INFINITY; COLS * ROWS];
    for t in ground {
        fill(t.map(|p| { let [c, r] = px(p[0], p[2]); [c, r, p[1]] }), COLS, ROWS, |i, y| height[i] = height[i].min(y));
    }
    // the casters' silhouette on the plane y = 0, over the map and a margin for the shift of raised ground
    const PAD: usize = 256;
    let (w, h) = (COLS + 2 * PAD, ROWS + 2 * PAD);
    let mut sil = vec![false; w * h];
    let on_plane = |p: [f32; 3]| {
        let t = -p[1] / dir[1];
        let [c, r] = px(p[0] + dir[0] * t, p[2] + dir[2] * t);
        [c + PAD as f32, r + PAD as f32, 0.0]
    };
    for t in casters {
        fill(t.map(on_plane), w, h, |i, _| sil[i] = true);
    }
    for (i, &y) in height.iter().enumerate().filter(|(_, y)| y.is_finite()) {
        // the ground point's own shadow position on y = 0, in map pixels
        let t = -y / dir[1];
        let (c, r) = ((i % COLS) as f32 + 0.5 + dir[2] * t * f.scale[0], (i / COLS) as f32 + 0.5 + dir[0] * t * f.scale[1]);
        let (c, r) = ((c + PAD as f32).floor(), (r + PAD as f32).floor());
        if c >= 0.0 && r >= 0.0 && (c as usize) < w && (r as usize) < h && sil[c as usize + r as usize * w] {
            map[i >> 3] |= 1 << (i & 7);
        }
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
        let mut m = vec![0u8; BYTES];
        let floor = [[[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 0.0, 100.0]]];
        rasterize(&mut m, &f, [0.0, 1.0, 0.0], [[[10.0, -5.0, 20.0], [12.0, -5.0, 20.0], [10.0, -5.0, 23.0]]], floor);
        assert_eq!(m, { let mut e = vec![0u8; BYTES]; for (r, c) in [(10, 20), (11, 20), (10, 21)] { e[(c + r * COLS) >> 3] |= 1 << ((c + r * COLS) & 7) } e });
    }
}
