//! Graphics → Mesh (B364, remaster-only): characters drawn with fewer triangles, for weak GPUs and heavy mod
//! characters. Quadric-error edge collapse (Garland–Heckbert) that only rewrites a mesh's index list: each collapse
//! moves a vertex onto a neighbour that already exists, so the vertex buffer (skin, UVs, colours, morph targets,
//! `noise.rs`'s moved-vertex indices) stays as it is and every kept vertex keeps its own attributes exactly.
//! Vertices on an open border (a material's edge), on a seam (one position, several vertices: UV or normal splits)
//! or that a morph target moves (faces) never move, so parts don't crack apart and faces keep their shapes.
//! At 100 % (the default) nothing runs: the meshes are the disc's or the mod's own.

use bevy::mesh::{Indices, VertexAttributeValues};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// `mesh` (a triangle list) at the Mesh setting (`graphics::MESH`); `locked` as in [`reduce`].
pub fn detail(mesh: &mut bevy::mesh::Mesh, locked: &[bool]) {
    let pc = crate::graphics::MESH.load(std::sync::atomic::Ordering::Relaxed);
    if pc >= 100 {
        return;
    }
    let (Some(VertexAttributeValues::Float32x3(pos)), Some(idx)) = (mesh.attribute(bevy::mesh::Mesh::ATTRIBUTE_POSITION), mesh.indices()) else { return };
    let idx: Vec<u32> = idx.iter().map(|i| i as u32).collect();
    let uv = match mesh.attribute(bevy::mesh::Mesh::ATTRIBUTE_UV_0) {
        Some(VertexAttributeValues::Float32x2(uv)) => &uv[..],
        _ => &[],
    };
    let out = reduce(pos, &idx, &Extra { same: &keys(mesh), skin: &skins(mesh), uv, locked }, pc);
    mesh.insert_indices(Indices::U32(out));
}

/// Per vertex, a hash of every attribute it has but its position: the disc's packets repeat a vertex where they
/// meet, and two vertices with the same position and key are drawn the same, so [`reduce`] can treat them as one.
pub fn keys(mesh: &bevy::mesh::Mesh) -> Vec<u64> {
    use std::hash::{Hash, Hasher};
    let n = mesh.count_vertices();
    let mut h: Vec<std::hash::DefaultHasher> = vec![Default::default(); n];
    for (a, v) in mesh.attributes().filter(|(a, _)| a.id != bevy::mesh::Mesh::ATTRIBUTE_POSITION.id) {
        let bytes = v.get_bytes();
        let size = a.format.size() as usize;
        for (k, h) in h.iter_mut().enumerate() {
            bytes.get(k * size..(k + 1) * size).hash(h);
        }
    }
    h.into_iter().map(|h| h.finish()).collect()
}

/// Per vertex, its joints and weights (none for a mesh without a skin).
pub fn skins(mesh: &bevy::mesh::Mesh) -> Vec<Skin> {
    match (mesh.attribute(bevy::mesh::Mesh::ATTRIBUTE_JOINT_INDEX), mesh.attribute(bevy::mesh::Mesh::ATTRIBUTE_JOINT_WEIGHT)) {
        (Some(VertexAttributeValues::Uint16x4(j)), Some(VertexAttributeValues::Float32x4(w))) => j.iter().zip(w).map(|(j, w)| (*j, *w)).collect(),
        _ => Vec::new(),
    }
}

/// A vertex's joints and their weights.
pub type Skin = ([u16; 4], [f32; 4]);

/// How far apart two skins are: the weights that differ, summed over every joint (0 the same, 2 no joint shared).
fn apart(a: &Skin, b: &Skin) -> f32 {
    let w = |s: &Skin, j: u16| (0..4).filter(|&k| s.0[k] == j).map(|k| s.1[k]).sum::<f32>();
    let mut d = 0.0;
    for (k, &j) in a.0.iter().enumerate() {
        if !a.0[..k].contains(&j) {
            d += (w(a, j) - w(b, j)).abs();
        }
    }
    for (k, &j) in b.0.iter().enumerate() {
        if !b.0[..k].contains(&j) && !a.0.contains(&j) {
            d += w(b, j);
        }
    }
    d
}

/// The most two skins may differ for a vertex to move onto the other: further apart, the pair bends differently and
/// a posed triangle would stretch between them (a skirt pulled along by a thigh).
const SKIN_APART: f32 = 0.2;

/// How far a collapse may move the surface at 50 %, as a share of the mesh's bounding diagonal (twice that at 0 %).
const MOVE: f64 = 0.01;

/// What else [`reduce`] keeps to, per vertex (each may be empty).
#[derive(Default, Clone, Copy)]
pub struct Extra<'a> {
    /// Vertices with the same position and key (missing = 0) count as one ([`keys`]).
    pub same: &'a [u64],
    /// A vertex only moves onto one skinned within [`SKIN_APART`] of it.
    pub skin: &'a [Skin],
    /// No triangle's texture turns over.
    pub uv: &'a [[f32; 2]],
    /// These stay where they are and apart from the rest.
    pub locked: &'a [bool],
}

/// The triangles of `idx` (a triangle list over `pos`) reduced to about `percent` % of them, or fewer collapses
/// where no more can be made without moving a locked vertex, folding a triangle or its texture over, tearing the
/// surface or moving it further than [`MOVE`] allows.
pub fn reduce(pos: &[[f32; 3]], idx: &[u32], x: &Extra, percent: u32) -> Vec<u32> {
    let Extra { same, skin, uv, locked } = *x;
    let tris = idx.len() / 3;
    let target = tris * percent.min(100) as usize / 100;
    if percent >= 100 || tris < 4 || idx.iter().any(|&i| i as usize >= pos.len()) {
        return idx.to_vec();
    }
    let n = pos.len();
    // one group per position: a seam's vertices share one, and the collapses move whole groups
    let mut groups: HashMap<[u32; 3], u32> = HashMap::new();
    let group: Vec<u32> = pos.iter().map(|q| { let k = q.map(f32::to_bits); let g = groups.len() as u32; *groups.entry(k).or_insert(g) }).collect();
    let gn = groups.len();
    let mut at = vec![[0f64; 3]; gn];
    for v in 0..n {
        at[group[v] as usize] = pos[v].map(|x| x as f64);
    }
    // a repeated vertex drawn as its first copy
    let mut first: HashMap<(u32, u64, u32), u32> = HashMap::new();
    let one: Vec<u32> = (0..n)
        .map(|v| {
            let lock = locked.get(v).copied().unwrap_or(false);
            *first.entry((group[v], same.get(v).copied().unwrap_or(0), if lock { v as u32 } else { u32::MAX })).or_insert(v as u32)
        })
        .collect();
    let mut tri: Vec<[u32; 3]> = idx.as_chunks::<3>().0.iter().map(|t| [t[0], t[1], t[2]].map(|v| one[v as usize])).collect();
    let g3 = |t: &[u32; 3]| t.map(|v| group[v as usize]);
    let mut alive: Vec<bool> = tri.iter().map(|t| { let g = g3(t); g[0] != g[1] && g[1] != g[2] && g[0] != g[2] }).collect();
    let mut live = alive.iter().filter(|&&a| a).count();

    // what can't move: the given vertices, and borders / non-manifold edges (by position)
    let mut fixed = vec![false; gn];
    for v in (0..n).filter(|&v| locked.get(v).copied().unwrap_or(false)) {
        fixed[group[v] as usize] = true;
    }
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
    for t in tri.iter().zip(&alive).filter(|x| *x.1).map(|x| g3(x.0)) {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            *edges.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    for (&(a, b), _) in edges.iter().filter(|e| *e.1 != 2) {
        fixed[a as usize] = true;
        fixed[b as usize] = true;
    }

    // per group: its triangles and its error quadric (the area-weighted planes of its triangles)
    let mut around: Vec<Vec<u32>> = vec![Vec::new(); gn];
    let mut quad = vec![[0f64; 11]; gn];
    for (i, t) in tri.iter().enumerate().filter(|(i, _)| alive[*i]) {
        let g = g3(t);
        let [a, b, c] = g.map(|g| at[g as usize]);
        let nr = cross(sub(b, a), sub(c, a));
        let len = dot(nr, nr).sqrt();
        let q = if len > 0.0 {
            let u = nr.map(|x| x / len);
            plane(u, -dot(u, a), len * 0.5)
        } else {
            [0.0; 11]
        };
        for g in g {
            around[g as usize].push(i as u32);
            add(&mut quad[g as usize], &q);
        }
    }

    // each triangle's facing as drawn: no collapse may turn one away from it
    let facing: Vec<[f64; 3]> = tri.iter().map(|t| { let [a, b, c] = g3(t).map(|g| at[g as usize]); cross(sub(b, a), sub(c, a)) }).collect();
    let mut m = Mesh { tri: &mut tri, alive: &mut alive, around: &mut around, group: &group, at: &at, skin, uv, facing: &facing };
    // how far a collapse may move the surface (squared): a share of the mesh's size that grows as the setting drops,
    // so a light model keeps its shape where a heavy one sheds most of its triangles
    let (lo, hi) = at.iter().fold(([f64::MAX; 3], [f64::MIN; 3]), |(lo, hi), p| (std::array::from_fn(|k| lo[k].min(p[k])), std::array::from_fn(|k| hi[k].max(p[k]))));
    let size = dot(sub(hi, lo), sub(hi, lo)).sqrt();
    let limit = (size * MOVE * (100 - percent.min(100)) as f64 / 50.0).powi(2);
    let mut stamp = vec![0u32; gn];
    let mut heap = BinaryHeap::new();
    for u in (0..gn).filter(|&u| !fixed[u]) {
        if let Some(&(c, _)) = m.partners(u, &quad).first() {
            heap.push(Reverse((c, u as u32, 0u32)));
        }
    }
    while live > target {
        let Some(Reverse((cost, u, s))) = heap.pop() else { break };
        if cost.0 > limit {
            break;
        }
        let u = u as usize;
        if s != stamp[u] {
            continue;
        }
        stamp[u] += 1;
        // the cheapest partner u can move onto; a dearer one waits its turn in the heap
        let Some((c, v, onto)) = m.partners(u, &quad).into_iter().find_map(|(c, v)| m.collapse(u, v as usize).map(|o| (c, v as usize, o))) else {
            continue;
        };
        if c > cost {
            heap.push(Reverse((c, u as u32, stamp[u])));
            continue;
        }
        // u's triangles: those on edge uv go, the rest take v's copy matching each of u's
        for t in std::mem::take(&mut m.around[u]) {
            if !m.alive[t as usize] {
                continue;
            }
            if g3(&m.tri[t as usize]).contains(&(v as u32)) {
                m.alive[t as usize] = false;
                live -= 1;
            } else {
                for w in m.tri[t as usize].iter_mut().filter(|w| group[**w as usize] == u as u32) {
                    *w = onto.iter().find(|o| o.0 == *w).unwrap().1;
                }
                m.around[v].push(t);
            }
        }
        let q = quad[u];
        add(&mut quad[v], &q);
        // v and its neighbours have new costs
        let mut near: Vec<usize> = m.around[v].iter().filter(|&&t| m.alive[t as usize]).flat_map(|&t| g3(&m.tri[t as usize])).map(|g| g as usize).collect();
        near.sort_unstable();
        near.dedup();
        for w in near.into_iter().filter(|&w| !fixed[w]) {
            stamp[w] += 1;
            let Mesh { around, alive, .. } = &mut m;
            around[w].retain(|&t| alive[t as usize]);
            if let Some(&(c, _)) = m.partners(w, &quad).first() {
                heap.push(Reverse((c, w as u32, stamp[w])));
            }
        }
    }
    tri.iter().zip(&alive).filter(|x| *x.1).flat_map(|x| *x.0).collect()
}

/// [`reduce`]'s working mesh: triangles (vertex ids, a repeated vertex as its first copy), position groups.
struct Mesh<'a> {
    tri: &'a mut Vec<[u32; 3]>,
    alive: &'a mut Vec<bool>,
    /// Per group, the triangles at it (dead ones dropped lazily).
    around: &'a mut Vec<Vec<u32>>,
    group: &'a [u32],
    at: &'a [[f64; 3]],
    skin: &'a [Skin],
    uv: &'a [[f32; 2]],
    facing: &'a [[f64; 3]],
}

impl Mesh<'_> {
    fn live(&self, g: usize) -> impl Iterator<Item = [u32; 3]> + '_ {
        self.around[g].iter().filter(|&&t| self.alive[t as usize]).map(|&t| self.tri[t as usize])
    }

    /// The groups around `g`.
    fn ring(&self, g: usize) -> Vec<u32> {
        let mut r: Vec<u32> = self.live(g).flatten().map(|v| self.group[v as usize]).filter(|&x| x as usize != g).collect();
        r.sort_unstable();
        r.dedup();
        r
    }

    /// u's partners, cheapest first.
    fn partners(&self, u: usize, quad: &[[f64; 11]]) -> Vec<(Key, u32)> {
        let mut out: Vec<(Key, u32)> = self
            .ring(u)
            .into_iter()
            .map(|v| {
                let mut q = quad[u];
                add(&mut q, &quad[v as usize]);
                // the mean squared distance to the planes it stood for
                (Key(error(&q, self.at[v as usize]).max(0.0) / q[10].max(1e-30)), v)
            })
            .collect();
        out.sort_unstable();
        out
    }

    /// Moving group `u` onto group `v`, if that keeps the surface a surface: they share exactly two triangles whose
    /// third corners are their only common neighbours, each of u's copies (a seam's) sits by exactly one of v's and
    /// becomes it (skinned alike), and none of u's other triangles flips or degenerates. The copies' pairs.
    fn collapse(&self, u: usize, v: usize) -> Option<Vec<(u32, u32)>> {
        let g3 = |t: &[u32; 3]| t.map(|w| self.group[w as usize]);
        let shared: Vec<[u32; 3]> = self.live(u).filter(|t| g3(t).contains(&(v as u32))).collect();
        if shared.len() != 2 {
            return None;
        }
        let mut third: Vec<u32> = shared.iter().flat_map(g3).filter(|&g| g as usize != u && g as usize != v).collect();
        third.sort_unstable();
        third.dedup();
        let rv = self.ring(v);
        if third.len() != 2 || self.ring(u).iter().filter(|g| rv.binary_search(g).is_ok()).count() != 2 {
            return None;
        }
        // u's copies, each with the one copy of v its triangles meet
        let mut onto: Vec<(u32, u32)> = Vec::new();
        for t in self.live(u) {
            let cu = t.iter().copied().find(|&w| self.group[w as usize] as usize == u)?;
            if !onto.iter().any(|o| o.0 == cu) {
                onto.push((cu, u32::MAX));
            }
        }
        for o in onto.iter_mut() {
            let mut cv = self.live(u).filter(|t| t.contains(&o.0)).flatten().filter(|&w| self.group[w as usize] as usize == v);
            let c = cv.next()?;
            if cv.any(|w| w != c) {
                return None;
            }
            o.1 = c;
            if let (Some(a), Some(b)) = (self.skin.get(o.0 as usize), self.skin.get(c as usize))
                && apart(a, b) > SKIN_APART
            {
                return None;
            }
        }
        let to = self.at[v];
        let flat = self.around[u].iter().filter(|&&t| self.alive[t as usize]).map(|&t| (t, self.tri[t as usize])).filter(|(_, t)| !g3(t).contains(&(v as u32))).all(|(i, t)| {
            let g = g3(&t);
            let p = |g: u32| self.at[g as usize];
            let q = |g: u32| if g as usize == u { to } else { p(g) };
            let before = cross(sub(p(g[1]), p(g[0])), sub(p(g[2]), p(g[0])));
            let after = cross(sub(q(g[1]), q(g[0])), sub(q(g[2]), q(g[0])));
            let (lb, la) = (dot(before, before).sqrt(), dot(after, after).sqrt());
            let f = self.facing[i as usize];
            // the texture: the same corners with u's copy swapped for v's must wind the same way
            let area = |c: [u32; 3]| match c.map(|w| self.uv.get(w as usize)) {
                [Some(a), Some(b), Some(c)] => (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]),
                _ => 1.0,
            };
            let swapped = t.map(|w| onto.iter().find(|o| o.0 == w).map_or(w, |o| o.1));
            let (ub, ua) = (area(t), area(swapped));
            la > lb * 1e-3 && dot(before, after) > 0.5 * lb * la && dot(f, after) > 0.5 * dot(f, f).sqrt() * la && (ub == 0.0 || ub * ua > 0.0)
        });
        flat.then_some(onto)
    }
}

/// A cost the heap orders (never NaN: quadrics of finite positions).
#[derive(Clone, Copy, PartialEq)]
struct Key(f64);
impl Eq for Key {}
impl PartialOrd for Key {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Key {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0)
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
/// The plane n·x + d = 0's quadric times `w`: the symmetric 4×4 (xx xy xz xd yy yz yd zz zd dd), then `w`.
fn plane(n: [f64; 3], d: f64, w: f64) -> [f64; 11] {
    let [a, b, c] = n;
    [a * a, a * b, a * c, a * d, b * b, b * c, b * d, c * c, c * d, d * d, 1.0].map(|x| x * w)
}
fn add(q: &mut [f64; 11], o: &[f64; 11]) {
    q.iter_mut().zip(o).for_each(|(a, b)| *a += b);
}
fn error(q: &[f64; 11], [x, y, z]: [f64; 3]) -> f64 {
    q[0] * x * x + 2.0 * q[1] * x * y + 2.0 * q[2] * x * z + 2.0 * q[3] * x + q[4] * y * y + 2.0 * q[5] * y * z + 2.0 * q[6] * y + q[7] * z * z + 2.0 * q[8] * z + q[9]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A closed, bumpy sphere (a UV sphere welded at its poles and seam), `rings` × `segs`.
    fn sphere(rings: usize, segs: usize) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut pos = vec![[0.0, 1.0, 0.0], [0.0, -1.0, 0.0]];
        for r in 1..rings {
            let th = std::f32::consts::PI * r as f32 / rings as f32;
            for s in 0..segs {
                let ph = std::f32::consts::TAU * s as f32 / segs as f32;
                let k = 1.0 + 0.05 * (3.0 * ph).sin() * th.sin();
                pos.push([k * th.sin() * ph.cos(), k * th.cos(), k * th.sin() * ph.sin()]);
            }
        }
        let at = |r: usize, s: usize| (2 + (r - 1) * segs + s % segs) as u32;
        let mut idx = Vec::new();
        for s in 0..segs {
            idx.extend([0, at(1, s + 1), at(1, s)]);
            idx.extend([1, at(rings - 1, s), at(rings - 1, s + 1)]);
            for r in 1..rings - 1 {
                idx.extend([at(r, s), at(r, s + 1), at(r + 1, s)]);
                idx.extend([at(r, s + 1), at(r + 1, s + 1), at(r + 1, s)]);
            }
        }
        (pos, idx)
    }

    fn edges(idx: &[u32], pos: &[[f32; 3]]) -> HashMap<([u32; 3], [u32; 3]), u32> {
        let mut e = HashMap::new();
        for t in idx.as_chunks::<3>().0 {
            for k in 0..3 {
                let (a, b) = (pos[t[k] as usize].map(f32::to_bits), pos[t[(k + 1) % 3] as usize].map(f32::to_bits));
                *e.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        e
    }

    /// 100 % changes nothing; lower keeps about that share, the sphere stays closed (every edge on two triangles),
    /// and every triangle still faces out (within 60° of the centre's direction).
    #[test]
    fn a_closed_mesh_halves_and_stays_closed() {
        let (pos, idx) = sphere(24, 32);
        assert_eq!(reduce(&pos, &idx, &Extra::default(), 100), idx);
        for pc in [75, 50, 25] {
            let out = reduce(&pos, &idx, &Extra::default(), pc);
            let want = idx.len() / 3 * pc as usize / 100;
            assert!((want..want + 3).contains(&(out.len() / 3)), "{pc}%: {} of {}", out.len() / 3, idx.len() / 3);
            assert!(edges(&out, &pos).values().all(|&c| c == 2), "{pc}%: a hole or a fin");
            for t in out.as_chunks::<3>().0 {
                let [a, b, c] = [0, 1, 2].map(|k| glam(pos[t[k] as usize]));
                let mid = (a + b + c) / 3.0;
                let cos = (b - a).cross(c - a).normalize().dot(mid.normalize());
                assert!(cos > 0.5, "{pc}%: a triangle tilted off the surface: {cos} {t:?}");
            }
        }
    }

    /// A locked vertex stays in; a UV seam (the sphere's s = 0 meridian, a second copy at u = 1 for the last
    /// column) slides as a seam: no triangle ends up stitched to the far side's copy.
    #[test]
    fn locks_seams_and_borders_stay() {
        let (rings, segs) = (16, 24);
        let (mut pos, mut idx) = sphere(rings, segs);
        // the texture's (u, v): around and down; the poles have no u
        let mut uv: Vec<[f32; 2]> = (0..pos.len()).map(|v| if v < 2 { [-1.0, 0.0] } else { [((v - 2) % segs) as f32 / segs as f32, ((v - 2) / segs + 1) as f32 / rings as f32] }).collect();
        let mut same = vec![0; pos.len()];
        let mut copy = HashMap::new();
        for r in 1..rings {
            let v = 2 + (r - 1) * segs;
            copy.insert(v as u32, pos.len() as u32);
            pos.push(pos[v]);
            uv.push([1.0, uv[v][1]]);
            same.push(1);
        }
        // the last column's triangles take the copies
        for t in idx.as_chunks_mut::<3>().0 {
            if t.iter().any(|&v| v >= 2 && (v as usize - 2) % segs == segs - 1) {
                t.iter_mut().for_each(|v| *v = copy.get(v).copied().unwrap_or(*v));
            }
        }
        let mut locked = vec![false; pos.len()];
        locked[100] = true;
        let out = reduce(&pos, &idx, &Extra { same: &same, uv: &uv, locked: &locked, ..Default::default() }, 30);
        assert!(out.len() < idx.len() * 2 / 5, "{} of {}", out.len() / 3, idx.len() / 3);
        assert!(out.contains(&100));
        assert!(edges(&out, &pos).values().all(|&c| c == 2));
        for t in out.as_chunks::<3>().0 {
            // every triangle winds the same way on the texture as on the sphere; one stitched across the seam to the
            // wrong copy wraps the long way round and turns over
            if t.iter().all(|&v| v >= 2) {
                let [a, b, c] = [0, 1, 2].map(|k| uv[t[k] as usize]);
                let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                assert!(area > 0.0, "a triangle across the seam: {t:?} {:?}", [a, b, c]);
            }
        }
        // two materials' meshes: each half keeps its border, so together they stay closed
        let half = idx.len() / 2;
        let border = |i: &[u32]| {
            let mut e: Vec<_> = edges(i, &pos).into_iter().filter(|x| x.1 == 1).map(|x| x.0).collect();
            e.sort();
            e
        };
        let mut kept = Vec::new();
        for part in [&idx[..half], &idx[half..]] {
            let out = reduce(&pos, part, &Extra::default(), 30);
            assert!(out.len() < part.len() * 3 / 5, "{} of {}", out.len() / 3, part.len() / 3);
            assert_eq!(border(&out), border(part));
            kept.extend(out);
        }
        assert!(edges(&kept, &pos).values().all(|&c| c == 2));
    }

    /// The disc's characters and the test mods (`context/mods`, `mods.rs`'s tests) at 50 % and 25 %: each part keeps its
    /// border (no gap opens between parts), its seams and its morphing vertices, and no edge gets more than two
    /// triangles. Prints the triangle counts and the time (the journal's table).
    #[test]
    fn disc_and_mod_characters_reduce() {
        use crate::character::{self, CharacterData};
        use bevy::prelude::*;
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let Ok(mut iso) = hst_data::iso::Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
        let (mut m, mut t, mut i, mut b) = (Assets::<Mesh>::default(), Assets::<StandardMaterial>::default(), Assets::<Image>::default(), Assets::default());
        let mut who: Vec<(String, CharacterData)> = [0, 6].iter().map(|&n| (format!("pc{n:02}"), character::load_disc(&mut iso, n, 0, &mut m, &mut t, &mut i, &mut b).unwrap())).collect();
        for id in ["getagrip_pc00_emi", "fore_pc00_phoebe"] {
            let Ok(md) = crate::mods::read(&std::path::Path::new(root).join("context/mods").join(id)) else { continue };
            who.push((id.into(), crate::mods::load(&mut iso, &md, 0, &mut m, &mut t, &mut i, &mut b).unwrap()));
        }
        for (name, c) in &who {
            for pc in [50, 25] {
                let (mut before, mut after, start) = (0, 0, std::time::Instant::now());
                for (h, _, _) in &c.parts {
                    let mesh = m.get(h).unwrap();
                    let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { panic!() };
                    let idx: Vec<u32> = mesh.indices().unwrap().iter().map(|v| v as u32).collect();
                    let uv = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
                        Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) => &uv[..],
                        _ => &[],
                    };
                    let out = reduce(pos, &idx, &Extra { same: &keys(mesh), skin: &skins(mesh), uv, ..Default::default() }, pc);
                    let border = |i: &[u32]| {
                        let mut e: Vec<_> = edges(i, pos).into_iter().filter(|x| x.1 == 1).map(|x| x.0).collect();
                        e.sort();
                        e
                    };
                    assert_eq!(border(&out), border(&idx), "{name} {pc}%: a border moved");
                    assert!(edges(&out, pos).values().all(|&k| k <= 2) || edges(&idx, pos).values().any(|&k| k > 2), "{name} {pc}%: a fin");
                    (before, after) = (before + idx.len() / 3, after + out.len() / 3);
                }
                eprintln!("{name} {pc}%: {before} → {after} triangles in {:?}", start.elapsed());
                assert!(after < before, "{name} {pc}%");
            }
        }
    }

    fn glam(p: [f32; 3]) -> bevy::math::Vec3 {
        bevy::math::Vec3::from(p)
    }
}
