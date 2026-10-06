//! The collision world's props, built operation for operation as the game does at court load: where each placed
//! model stands (world matrix from the plant record, its inverse into model space, bounding sphere) and the 20 m
//! grid of cells the live ball looks props up in. Angles and bounds on the FPU (`ps2`), matrices on VU0 (`vu0`).
//! Matrices are 4 rows of 4, row vectors (`p' = p · M`, translation in row 3); game space is Y-down.

use crate::ps2;
use crate::vu0::{self, V4};

pub type M4 = [V4; 4];

pub const IDENTITY: M4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const PI: f32 = f32::from_bits(0x4049_0fdb);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
/// The sine polynomial's coefficients of x⁹, x⁷, x⁵, x³.
const SIN: V4 = [f32::from_bits(0x362e_9c14), f32::from_bits(0xb94f_b21f), f32::from_bits(0x3c08_873e), f32::from_bits(0xbe2a_aaa4)];
/// Grid cell size in metres.
pub const CELL: f32 = 20.0;

/// (sin θ, cos θ) for |θ| ≤ π: cos is the sine polynomial at π/2 − |θ|, sin = ±√(1 − cos²) (VU0 Q square root).
pub fn sincos(t: f32) -> (f32, f32) {
    let neg = t < 0.0;
    let x = if neg { ps2::add(HALF_PI, t) } else { ps2::sub(HALF_PI, t) };
    let x2 = vu0::mul(x, x);
    let x3: V4 = std::array::from_fn(|k| vu0::mul(vu0::mul(SIN[k], x), x2));
    let x5: V4 = std::array::from_fn(|k| if k < 3 { vu0::mul(x3[k], x2) } else { x3[k] });
    let x7: V4 = std::array::from_fn(|k| if k < 2 { vu0::mul(x5[k], x2) } else { x5[k] });
    let x9 = vu0::mul(x7[0], x2);
    let s = vu0::add(vu0::add(vu0::add(vu0::add(vu0::add(0.0, x), x3[3]), x5[2]), x7[1]), x9);
    let s = vu0::add(0.0, s);
    let q = vu0::add(0.0, vu0::sqrt(vu0::sub(1.0, vu0::mul(s, s))));
    (if neg { vu0::sub(0.0, q) } else { vu0::add(0.0, q) }, s)
}

/// Rotation about Y (rows (c, 0, −s), (0, 1, 0), (s, 0, c)), built in a zeroed register as the game does.
pub fn rot_y(t: f32) -> M4 {
    let (s, c) = sincos(t);
    let z = 0.0;
    [[vu0::add(z, c), z, vu0::sub(z, s), z], [z, vu0::add(z, 1.0), z, z], [vu0::add(z, s), z, vu0::add(z, c), z], [z, z, z, vu0::add(z, 1.0)]]
}

/// Rotation about X (rows (1, 0, 0), (0, c, s), (0, −s, c)).
pub fn rot_x(t: f32) -> M4 {
    let (s, c) = sincos(t);
    let z = 0.0;
    [[vu0::add(z, 1.0), z, z, z], [z, vu0::add(z, c), vu0::add(z, s), z], [z, vu0::sub(z, s), vu0::add(z, c), z], [z, z, z, vu0::add(z, 1.0)]]
}

/// `a` then `b`: every row of `a` through `b`.
pub fn mat_mul(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|i| vu0::transform(b, a[i]))
}

/// Inverse of a rotation + translation: transposed 3×3 (w = 0) and row 3 = −(t · Rᵀ), keeping t.w.
pub fn inverse(m: &M4) -> M4 {
    let t = m[3];
    let r: [V4; 3] = std::array::from_fn(|k| [m[0][k], m[1][k], m[2][k], 0.0]);
    let v: [f32; 3] = std::array::from_fn(|k| vu0::madd(vu0::madd(vu0::mul(r[0][k], t[0]), r[1][k], t[1]), r[2][k], t[2]));
    [r[0], r[1], r[2], [vu0::sub(0.0, v[0]), vu0::sub(0.0, v[1]), vu0::sub(0.0, v[2]), t[3]]]
}

/// A plant-record code character as a number: '0'–'9', then 'A'… = 10… (signed byte, as the game reads it).
fn digit(c: u8) -> f32 {
    let c = c as i8 as i32;
    (if c < 0x3a { c - 0x30 } else { c - 0x37 }) as f32
}

/// World matrix of a placed prop: translation `pos`, turned by `yaw`; trees, props and structures (categories
/// 17–19) also take a tilt about X from code byte 1 (n·π/36, folded to ≤ π/2) and an extra turn from code byte 0
/// (n·2π/36, folded into ±π). A tree within 1 m (horizontally) of the origin is the net post: it stands unrotated
/// at the origin, and the flag that marks it is returned.
pub fn place(category: u8, pos: [f32; 3], yaw: f32, code: [u8; 4]) -> (M4, bool) {
    let mut m = IDENTITY;
    m[3] = [pos[0], pos[1], pos[2], 1.0];
    m = mat_mul(&mat_mul(&IDENTITY, &rot_y(yaw)), &m);
    if (17..=19).contains(&category) {
        if code[1] != 0 {
            let mut a = ps2::div(ps2::mul(PI, digit(code[1])), 36.0);
            if a > HALF_PI {
                a = ps2::add(a, -PI);
            }
            m = mat_mul(&mat_mul(&IDENTITY, &rot_x(a)), &m);
        }
        if code[0] != 0 {
            let mut a = ps2::div(ps2::mul(TWO_PI, digit(code[0])), 36.0);
            if a > PI {
                a = ps2::sub(a, TWO_PI);
            } else if a < -PI {
                a = ps2::add(TWO_PI, a);
            }
            m = mat_mul(&mat_mul(&IDENTITY, &rot_y(a)), &m);
        }
    }
    if category == 17 && ps2::sqrt(ps2::madd(ps2::mul(m[3][2], m[3][2]), m[3][0], m[3][0])) < 1.0 {
        return (IDENTITY, true);
    }
    (m, false)
}

/// A placed model instance.
#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub world: M4,
    /// World → model space: the rigid inverse, then 1/scale.
    pub to_model: M4,
    /// Root node matrices: through the scaled world matrix (drawing, sphere) and the unscaled one.
    pub node: M4,
    pub node_unscaled: M4,
    /// Bounding sphere in world space.
    pub center: V4,
    pub radius: f32,
}

/// Instance a model at `world` with uniform `scale`; `local` is the root node's own matrix, `center`/`radius`
/// its bounding sphere.
pub fn instance(world: M4, scale: f32, local: &M4, center: V4, radius: f32) -> Prop {
    let (scaled, to_model) = if scale == 1.0 {
        (world, inverse(&world))
    } else {
        let mut s = world;
        for row in &mut s[..3] {
            for v in &mut row[..3] {
                *v = vu0::mul(*v, scale);
            }
        }
        let k = ps2::div(1.0, scale);
        let d = [[k, 0.0, 0.0, 0.0], [0.0, k, 0.0, 0.0], [0.0, 0.0, k, 0.0], [0.0, 0.0, 0.0, 1.0]];
        (s, mat_mul(&inverse(&world), &d))
    };
    let node = mat_mul(local, &scaled);
    Prop { world, to_model, node, node_unscaled: mat_mul(local, &world), center: vu0::transform(&node, center), radius: ps2::mul(scale, radius) }
}

/// Order of the game's prop list for `n` props created in order: the first, then the rest newest first
/// (each new prop is linked in right after the head).
pub fn list_order(n: usize) -> impl Iterator<Item = usize> {
    (0..n.min(1)).chain((1..n).rev())
}

/// Cells of `CELL` metres over the props' bounding boxes; every cell lists the props whose box touches it.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    /// Corners of the union of the props' boxes (xyz).
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// Inclusive cell index range per axis.
    pub lo: [i32; 3],
    pub hi: [i32; 3],
    /// Cells, z fastest then y then x; each lists prop ids in the order given to [`grid`].
    pub cells: Vec<Vec<usize>>,
}

impl Grid {
    pub fn dims(&self) -> [i32; 3] {
        std::array::from_fn(|k| self.hi[k] - self.lo[k] + 1)
    }
    /// The props in cell `c` (absolute cell coordinates within `lo..=hi`).
    pub fn cell(&self, c: [i32; 3]) -> &[usize] {
        let d = self.dims();
        let r: [i32; 3] = std::array::from_fn(|k| c[k] - self.lo[k]);
        &self.cells[(d[2] * (r[1] + r[0] * d[1]) + r[2]) as usize]
    }
}

/// Cell index of a coordinate: floor-like for negatives by stepping one cell down first, then truncating.
pub fn cell_of(v: f32) -> i32 {
    let v = if v < 0.0 { ps2::sub(v, CELL) } else { v };
    ps2::div(v, CELL) as i32
}

fn bounds(spheres: impl Iterator<Item = (V4, f32)>) -> ([f32; 3], [f32; 3]) {
    let (mut lo, mut hi) = ([f32::MAX; 3], [-f32::MAX; 3]);
    for (c, r) in spheres {
        for p in [std::array::from_fn::<f32, 3, _>(|k| ps2::sub(c[k], r)), std::array::from_fn(|k| ps2::add(c[k], r))] {
            for k in 0..3 {
                if p[k] < lo[k] {
                    lo[k] = p[k];
                }
                if hi[k] < p[k] {
                    hi[k] = p[k];
                }
            }
        }
    }
    (lo, hi)
}

/// The grid over collision props `(id, sphere centre, radius)`, given in the game's list order.
pub fn grid(props: &[(usize, V4, f32)]) -> Grid {
    let (min, max) = bounds(props.iter().map(|p| (p.1, p.2)));
    let (lo, hi) = (min.map(cell_of), max.map(cell_of));
    let d: [i32; 3] = std::array::from_fn(|k| hi[k] - lo[k] + 1);
    let mut cells = vec![Vec::new(); (d[0] * d[1] * d[2]).max(0) as usize];
    for &(id, c, r) in props {
        let (a, b) = bounds(std::iter::once((c, r)));
        let span = |k: usize| cell_of(a[k]).clamp(lo[k], hi[k]) - lo[k]..=cell_of(b[k]).clamp(lo[k], hi[k]) - lo[k];
        for x in span(0) {
            for y in span(1) {
                for z in span(2) {
                    cells[(d[2] * (y + x * d[1]) + z) as usize].push(id);
                }
            }
        }
    }
    Grid { min, max, lo, hi, cells }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_are_rotations() {
        for t in [-3.1, -1.0, -0.2, 0.0, 0.4, 1.5, 3.0] {
            let (s, c) = sincos(t);
            assert!((s - t.sin()).abs() < 1e-5 && (c - t.cos()).abs() < 1e-5, "{t}: {s} {c}");
        }
        let m = mat_mul(&rot_y(0.7), &IDENTITY);
        let back = mat_mul(&m, &inverse(&m));
        for (i, row) in back.iter().enumerate() {
            for (k, v) in row.iter().enumerate() {
                assert!((v - IDENTITY[i][k]).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn list_links_after_the_head() {
        assert_eq!(list_order(4).collect::<Vec<_>>(), [0, 3, 2, 1]);
        assert_eq!(list_order(0).count(), 0);
    }

    #[test]
    fn negative_coordinates_step_a_cell_down() {
        assert_eq!((cell_of(-0.5), cell_of(0.5), cell_of(-20.0), cell_of(39.9)), (-1, 0, -2, 1));
    }
}
