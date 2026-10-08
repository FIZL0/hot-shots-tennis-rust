//! `.MDL` models. The file is a sequence of 16-byte-aligned reads:
//!
//! ```text
//! u8 version
//! node tree      : node = 0x40 header (+0x38 extra size), extra, 3 × mat4, u32 n, n × group
//!                  group = 0x40 header, extra, 3 × mat4, u32 n, n × node
//! u32 material count (== MTL material count)
//!   per material : 0xc header (+0 batch count, +4 GS TEX1 K, +8/+0xa texture wrap u/v)
//!     per batch  : 0x34 header (+0x1c packet count, +0x31 GS PRIM bits)
//!       per packet: 0x60 header, VIF stream (+0x34 qwords), bone list (+0x3c + 1 bytes),
//!                   +0x38 bytes, +0x38 × 32 bytes, [+0x57 qwords if +0x56], +0x54 × morph blocks,
//!                   [4 bytes if +0x4c < 0]
//!   morph block   : 0xc header (+0 qwords k, k − 3, 3), k qwords of VIF: an UNPACK of (count, 1.0, 0x50, 0x31),
//!                   an UNPACK of `count` × (dx, dy, dz, position entry as an int)
//! u32 n, n × { u32 size, name }  (morph target names, one per morph block)
//! ```
//! Geometry is the VU1 upload itself: per strip a GIF tag, V4-32 positions, V4-16 normals whose
//! `w` bit 15 is the GS no-kick flag (strip restart), a colour (STROW constant or V4-8), V4-32 UVs.
//!
//! Collision reads the same packets in place: packet header +0x44/+0x48/+0x4c/+0x50 are qword offsets of the
//! positions, normals, colours (< 0: the 4-byte constant after the packet) and UVs in the VIF stream; the
//! "bone list" is the strip-slot → position/normal index map, the +0x38 bytes are the strip slots that start a
//! drawn triangle and the +0x38 × 32 bytes their bounding boxes. Batch header +0 = 1 marks a static batch,
//! +4 is its node (pre-order index over nodes and groups); a node's third matrix (+0x80 of the 3 × mat4) takes
//! model space to node space. Only batches whose material has an attribute map collide ([`Model::colliding`]).

use crate::xb::Error;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// PS2 colour, 0x80 = 1.0.
    pub color: [u8; 4],
}

#[derive(Debug, Default)]
pub struct Packet {
    /// One per position entry (for rigid packets, one per drawn vertex).
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<[u32; 3]>,
    /// Drawn vertex → first position entry (one more than the drawn vertices; in rigid packets the identity).
    pub bones: Vec<u8>,
    /// Per position entry: its weight (the position's `w`) and the raw `w` of its normal (bits 3..5: the bone
    /// slot in `palette`; bit 15: no-kick).
    pub entry_weight: Vec<f32>,
    pub entry_flags: Vec<u16>,
    /// Per drawn vertex, in order: UV and colour.
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[u8; 4]>,
    /// Nodes this packet's batch is bound to (bone slots).
    pub palette: Vec<usize>,
    /// The batch's GS PRIM bits (batch header +0x31): 0x10 TME (textured), 0x20 FGE (fogged), 0x40 ABE (blended).
    pub prim: u8,
    /// Packet header +0x58: the packet's texture coordinates are stored as (v, u); a `.UVA` offset is swapped to
    /// match before it is added.
    pub uv_swap: bool,
    /// Per morph target ([`Model::morph_names`]): (position entry, offset) — the entry moves by weight × offset.
    pub morphs: Vec<Vec<(usize, [f32; 3])>>,
    /// The node owning the packet's batch (batch header +0x2a): a `.NOI` deformer moves its node's packets.
    pub group: i16,
    /// Bounding box min, max (packet header +0, +0x10).
    pub bounds: [[f32; 3]; 2],
    /// Per position entry, its share of the noise deformer (the VIF unpack after the bounding boxes, when packet
    /// header +0x56 is set); empty when the packet has none.
    pub noise: Vec<f32>,
}

/// A drawn vertex of a skinned model in its bind pose (model space), with up to four (node, weight) bindings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkinVertex {
    pub pos: [f32; 3],
    /// Bind-space normals of the entries on `joints[0]` and `joints[1]`, pre-weighted (|n| = the weight) and not
    /// normalised: VU1 lights the vertex with their sum rotated by each bone, as is.
    pub normals: [[f32; 3]; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

/// One triangle as the game's mesh sweep reads it (positions and UVs are the raw V4-32 qwords).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionTri {
    pub node: usize,
    /// MTL material index.
    pub material: usize,
    pub pos: [[f32; 4]; 3],
    /// `w` is ±1, the strip winding of the triangle.
    pub uv: [[f32; 4]; 3],
    pub color: [[u8; 4]; 3],
    /// Low byte of each vertex normal's `w`, shifted right by 3.
    pub flags: [u8; 3],
    /// Bounding box min, max.
    pub bounds: [[f32; 4]; 2],
}

#[derive(Debug, Default)]
pub struct Model {
    pub node_count: usize,
    /// `materials[m]` = packets drawn with MTL material `m`.
    pub materials: Vec<Vec<Packet>>,
    /// Per node (pre-order), model space → node space.
    pub node_inverse: Vec<[[f32; 4]; 4]>,
    /// Per node (pre-order), the node's own placement (its second matrix).
    pub node_local: Vec<[[f32; 4]; 4]>,
    /// Per node (pre-order), its parent's index (none for the root).
    pub node_parent: Vec<Option<usize>>,
    /// Per node (pre-order), node space → model space in the bind pose (its first matrix).
    pub node_bind: Vec<[[f32; 4]; 4]>,
    /// Per node (pre-order), its name (3ds Max Biped names on characters: `Bip01Pelvis`, …, `Racket`).
    pub node_names: Vec<String>,
    /// Root node bounding sphere: centre (w = 1) and radius (header +0x20, +0x30).
    pub center: [f32; 4],
    pub radius: f32,
    /// Per material, the texture wrap mode along u and v (GS CLAMP WMS/WMT: 0 repeat, 1 clamp, 2 region clamp,
    /// 3 region repeat; material header +8, +0xa).
    pub wrap: Vec<[u8; 2]>,
    /// Per material, the mipmap LOD bias (GS TEX1 K: the low 12 bits of header +4, signed, 4 fraction bits).
    pub lod_k: Vec<f32>,
    /// Material of every static batch, in file order.
    pub static_materials: Vec<usize>,
    /// Triangles of static batches, in the order the game tests them: by node, then material, then file order.
    pub collision: Vec<CollisionTri>,
    /// Face morph targets (`face\x01blink_eye`, `face\x01joy_mouth`, …), what `.MOR` tracks bind to by name.
    pub morph_names: Vec<String>,
}

impl Model {
    /// Every material's packets as skinned geometry in the bind pose. A drawn vertex is the sum of its position
    /// entries, each stored in its bone's space already multiplied by its weight (the weight is the entry's `w`):
    /// Σ [p, w] · bind(bone). Normals per bone (pre-weighted, rotated by the bone: `SkinVertex::normals`). Bones
    /// beyond the four heaviest are dropped and the rest renormalised. Last, per morph target ([`Model::morph_names`]), each drawn
    /// vertex's model-space offset at weight 1 (empty for a packet without morphs).
    #[allow(clippy::type_complexity)]
    pub fn skinned(&self) -> Vec<(usize, Vec<SkinVertex>, Vec<[u32; 3]>, Vec<Vec<[f32; 3]>>)> {
        let mut out = Vec::new();
        for (material, packets) in self.materials.iter().enumerate() {
            for pk in packets {
                let n = pk.bones.len().saturating_sub(1).min(pk.uvs.len());
                let mut verts = Vec::with_capacity(n);
                let mut kick = Vec::with_capacity(n);
                let mut morph = vec![vec![[0.0f32; 3]; n]; pk.morphs.len()];
                for v in 0..n {
                    let (a, b) = (pk.bones[v] as usize, (pk.bones[v + 1] as usize).min(pk.vertices.len()));
                    let mut sv = SkinVertex { uv: pk.uvs[v], color: pk.colors.get(v).copied().unwrap_or([0x80; 4]), ..Default::default() };
                    let mut binds: Vec<(usize, f32, [f32; 3])> = Vec::new();
                    let mut pos = [0.0f32; 3];
                    for e in a..b {
                        let flags = pk.entry_flags.get(e).copied().unwrap_or(0);
                        let node = pk.palette.get(((flags >> 3) & 7) as usize).copied().unwrap_or(pk.palette.first().copied().unwrap_or(0));
                        let w = pk.entry_weight.get(e).copied().unwrap_or(1.0);
                        let m = self.node_bind.get(node).copied().unwrap_or(IDENTITY);
                        let p = pk.vertices[e].pos;
                        let q = pk.vertices[e].normal;
                        let mut nrm = [0.0f32; 3];
                        for k in 0..3 {
                            pos[k] += p[0] * m[0][k] + p[1] * m[1][k] + p[2] * m[2][k] + w * m[3][k];
                            nrm[k] = q[0] * m[0][k] + q[1] * m[1][k] + q[2] * m[2][k];
                        }
                        binds.push((node, w, nrm));
                        // a morph offset moves the entry in its bone's space: rotate it like the position
                        for (t, target) in pk.morphs.iter().enumerate() {
                            for &(_, d) in target.iter().filter(|(i, _)| *i == e) {
                                for k in 0..3 {
                                    morph[t][v][k] += d[0] * m[0][k] + d[1] * m[1][k] + d[2] * m[2][k];
                                }
                            }
                        }
                    }
                    binds.sort_by(|x, y| y.1.total_cmp(&x.1));
                    binds.truncate(4);
                    let total: f32 = binds.iter().map(|b| b.1).sum();
                    // ponytail: normals of a third and fourth bone are dropped; no vertex on the disc has more than two
                    for (k, (node, w, nrm)) in binds.iter().enumerate() {
                        sv.joints[k] = *node as u16;
                        sv.weights[k] = if total > 0.0 { w / total } else { 0.0 };
                        if k < 2 {
                            sv.normals[k] = *nrm;
                        }
                    }
                    sv.pos = pos;
                    verts.push(sv);
                    kick.push(pk.entry_flags.get(a).is_some_and(|f| f & 0x8000 == 0));
                }
                // one strip per packet: a kicked vertex closes a triangle with the two before it, winding
                // alternating along the strip (as the rigid decode)
                let mut tris = Vec::new();
                for (i, &k) in kick.iter().enumerate() {
                    if k && i >= 2 {
                        let i = i as u32;
                        tris.push(if i % 2 == 0 { [i - 2, i - 1, i] } else { [i - 1, i - 2, i] });
                    }
                }
                out.push((material, verts, tris, morph));
            }
        }
        out
    }

    /// The triangles the game collides with: those whose material has an attribute map.
    /// Whether the game treats the model as a collision object: some static batch has an attribute-mapped material.
    pub fn collides(&self, mtl: &crate::mtl::Mtl) -> bool {
        self.static_materials.iter().any(|&m| mtl.materials.get(m).is_some_and(|m| m.attributes.is_some()))
    }

    pub fn colliding<'a>(&'a self, mtl: &'a crate::mtl::Mtl) -> impl Iterator<Item = &'a CollisionTri> {
        self.collision.iter().filter(|t| mtl.materials.get(t.material).is_some_and(|m| m.attributes.is_some()))
    }
}

const IDENTITY: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

struct Cursor<'a> {
    d: &'a [u8],
    p: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let b = self
            .d
            .get(self.p..self.p + n)
            .ok_or_else(|| Error(format!("mdl: need {n:#x} bytes at {:#x} of {:#x}", self.p, self.d.len())))?;
        self.p = (self.p + n + 15) & !15;
        Ok(b)
    }
    fn count(&mut self, n: usize) -> Result<usize, Error> {
        let v = i32_at(self.take(n)?, 0);
        usize::try_from(v).map_err(|_| Error(format!("mdl: negative count {v}")))
    }
}

fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn size_at(b: &[u8], o: usize) -> Result<usize, Error> {
    usize::try_from(i32_at(b, o)).map_err(|_| Error("mdl: negative size".into()))
}

/// Header, extra blob, 3 matrices, then a child count. Nodes hold groups, groups hold nodes.
fn tree_item(c: &mut Cursor, depth: u32, parent: Option<usize>, model: &mut Model) -> Result<(), Error> {
    if depth > 64 {
        return Err(Error("mdl: node tree too deep".into()));
    }
    let h = c.take(0x40)?;
    if depth == 0 {
        model.center = std::array::from_fn(|k| f32_at(h, 0x20 + 4 * k));
        model.radius = f32_at(h, 0x30);
    }
    let name = c.take(size_at(h, 0x38)?)?;
    model.node_names.push(String::from_utf8_lossy(name.split(|&b| b == 0).next().unwrap_or_default()).into_owned());
    let m = c.take(0xc0)?;
    let mat = |o: usize| std::array::from_fn(|r| std::array::from_fn(|k| f32_at(m, o + 16 * r + 4 * k)));
    model.node_bind.push(mat(0));
    model.node_local.push(mat(0x40));
    model.node_inverse.push(mat(0x80));
    model.node_parent.push(parent);
    let me = Some(model.node_local.len() - 1);
    for _ in 0..c.count(4)? {
        tree_item(c, depth + 1, me, model)?;
    }
    Ok(())
}

pub fn parse(d: &[u8]) -> Result<Model, Error> {
    let mut c = Cursor { d, p: 0 };
    let mut m = Model::default();
    c.take(1)?;
    tree_item(&mut c, 0, None, &mut m)?;
    m.node_count = m.node_inverse.len();
    for material in 0..c.count(4)? {
        let mut packets = Vec::new();
        let mh = c.take(0xc)?;
        m.wrap.push([mh[8], mh[0xa]]);
        m.lod_k.push(((i32_at(mh, 4) << 20) >> 20) as f32 / 16.0);
        for _ in 0..size_at(mh, 0)? {
            let bh = c.take(0x34)?;
            let node = size_at(bh, 4)?;
            let palette: Vec<usize> = (0..size_at(bh, 0)?.min(12)).map(|k| size_at(bh, 4 + 4 * k)).collect::<Result<_, _>>()?;
            if i32_at(bh, 0) == 1 {
                m.static_materials.push(material);
            }
            for _ in 0..size_at(bh, 0x1c)? {
                let ph = c.take(0x60)?;
                let vif = c.take(size_at(ph, 0x34)? << 4)?;
                let bones = c.take(size_at(ph, 0x3c)? + 1)?.to_vec();
                let n = size_at(ph, 0x38)?;
                let starts = c.take(n)?;
                let bounds = c.take(n << 5)?;
                // STCYCL, UNPACK S-32 of `num` words (0 = 256), the words
                let noise = if ph[0x56] != 0 {
                    let x = c.take((ph[0x57] as usize) << 4)?;
                    let num = x.get(6).map_or(0, |&n| if n == 0 { 256 } else { n as usize });
                    x.get(8..).unwrap_or_default().chunks_exact(4).take(num).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect()
                } else {
                    Vec::new()
                };
                let mut morphs = Vec::new();
                for _ in 0..i16::from_le_bytes([ph[0x54], ph[0x55]]).max(0) {
                    let k = c.count(0xc)?;
                    let b = if k != 0 { c.take(k << 4)? } else { &[][..] };
                    let n = if k >= 3 { size_at(b, 0x10)?.min(k - 3) } else { 0 };
                    morphs.push(
                        b.get(0x30..0x30 + n * 16).unwrap_or_default()
                            .chunks_exact(16)
                            .map(|q| Ok((size_at(q, 12)?, [f32_at(q, 0), f32_at(q, 4), f32_at(q, 8)])))
                            .collect::<Result<_, Error>>()?,
                    );
                }
                let constant = if i32_at(ph, 0x4c) < 0 { Some(c.take(4)?) } else { None };
                if i32_at(bh, 0) == 1 {
                    collision(&mut m.collision, node, material, ph, vif, &bones, starts, bounds, constant)?;
                }
                let mut pk = decode_vif(vif)?;
                pk.bones = bones;
                pk.palette = palette.clone();
                pk.prim = bh[0x31];
                pk.uv_swap = ph[0x58] != 0;
                pk.morphs = morphs;
                pk.group = i16::from_le_bytes([bh[0x2a], bh[0x2b]]);
                pk.noise = noise;
                pk.bounds = [0, 0x10].map(|o| std::array::from_fn(|k| f32_at(ph, o + 4 * k)));
                packets.push(pk);
            }
        }
        m.materials.push(packets);
    }
    m.collision.sort_by_key(|t| t.node); // stable: material, then file order, within a node
    for _ in 0..c.count(4)? {
        let s = c.count(4)?;
        let name = c.take(s)?;
        m.morph_names.push(String::from_utf8_lossy(name.split(|&b| b == 0).next().unwrap_or_default()).into_owned());
    }
    Ok(m)
}

/// The packet's drawn triangles (strip slots `s, s+1, s+2` for every listed start `s`), skipping any whose
/// third vertex is a strip restart, as the game does.
#[allow(clippy::too_many_arguments)]
fn collision(
    out: &mut Vec<CollisionTri>,
    node: usize,
    material: usize,
    ph: &[u8],
    vif: &[u8],
    map: &[u8],
    starts: &[u8],
    bounds: &[u8],
    constant: Option<&[u8]>,
) -> Result<(), Error> {
    let at = |o: usize, n: usize| vif.get(o..o + n).ok_or_else(|| Error(format!("mdl: collision data past VIF block at {o:#x}")));
    let (pos, nrm, col, uv) = (size_at(ph, 0x44)? << 4, size_at(ph, 0x48)? << 4, i32_at(ph, 0x4c), size_at(ph, 0x50)? << 4);
    let qword = |b: &[u8]| -> [f32; 4] { std::array::from_fn(|k| f32_at(b, 4 * k)) };
    for (i, &s) in starts.iter().enumerate() {
        let s = s as usize;
        let idx = |k: usize| map.get(s + k).map(|&v| v as usize).ok_or_else(|| Error("mdl: strip slot past index map".into()));
        if at(nrm + idx(2)? * 8 + 6, 2)?[1] & 0x80 != 0 {
            continue;
        }
        let mut t = CollisionTri {
            node,
            material,
            pos: [[0.0; 4]; 3],
            uv: [[0.0; 4]; 3],
            color: [[0; 4]; 3],
            flags: [0; 3],
            bounds: [qword(&bounds[i * 32..]), qword(&bounds[i * 32 + 16..])],
        };
        for k in 0..3 {
            t.pos[k] = qword(at(pos + idx(k)? * 16, 16)?);
            t.flags[k] = at(nrm + idx(k)? * 8 + 6, 1)?[0] >> 3;
            t.uv[k] = qword(at(uv + (s + k) * 16, 16)?);
            let c = match constant {
                Some(c) => c,
                None => at(((col as usize) << 4) + (s + k) * 4, 4)?,
            };
            t.color[k] = c[..4].try_into().unwrap();
        }
        out.push(t);
    }
    Ok(())
}

/// Walk a VIF stream and rebuild triangle strips from the unpacked vertex attributes.
fn decode_vif(d: &[u8]) -> Result<Packet, Error> {
    let mut pk = Packet::default();
    let (mut cl, mut wl) = (4u32, 4u32);
    let mut row = [0u32; 4];
    // Attributes arrive in a fixed order per strip; `slot` counts V4-32 unpacks seen in the strip.
    let mut slot = 0;
    let mut strip_start = 0usize;
    let mut tri_start = 0usize;
    let mut p = 0;
    let rd = |p: usize| d.get(p..p + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    while let Some(w) = rd(p) {
        p += 4;
        let cmd = (w >> 24) & 0x7f;
        let num = (w >> 16) & 0xff;
        match cmd {
            0x00 | 0x10 | 0x11 | 0x13 | 0x14 | 0x15 | 0x17 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 => {}
            0x01 => (cl, wl) = (w & 0xff, (w >> 8) & 0xff),
            0x20 => p += 4,
            0x30 => {
                for (i, r) in row.iter_mut().enumerate() {
                    *r = rd(p + i * 4).ok_or_else(|| Error("vif: STROW truncated".into()))?;
                }
                p += 16;
            }
            0x31 => p += 16,
            0x50 | 0x51 => p += (w & 0xffff) as usize * 16,
            0x60..=0x7f => {
                let (vn, vl) = ((cmd >> 2) & 3, cmd & 3);
                let n = if num == 0 { 256 } else { num } as usize;
                let reads = if cl >= wl { n } else { (n / wl as usize) * cl as usize + (n % wl as usize).min(cl as usize) };
                let elem = if vl == 3 { 2 } else { [4, 2, 1][vl as usize] * (vn as usize + 1) };
                let size = (reads * elem + 3) & !3;
                let data = d.get(p..p + size).ok_or_else(|| Error("vif: unpack truncated".into()))?;
                p += size;
                match (vn, vl, cmd & 0x10 != 0) {
                    (3, 0, false) => {
                        match slot {
                            0 => {
                                strip_start = pk.vertices.len();
                                tri_start = pk.triangles.len();
                            }
                            1 => {
                                for v in data.chunks_exact(16) {
                                    pk.vertices.push(Vertex {
                                        pos: [f32_at(v, 0), f32_at(v, 4), f32_at(v, 8)],
                                        color: [0x80; 4],
                                        ..Default::default()
                                    });
                                    pk.entry_weight.push(f32_at(v, 12));
                                }
                            }
                            _ => {
                                for (v, uv) in pk.vertices[strip_start..].iter_mut().zip(data.chunks_exact(16)) {
                                    v.uv = [f32_at(uv, 0), f32_at(uv, 4)];
                                }
                                pk.uvs.extend(data.chunks_exact(16).map(|uv| [f32_at(uv, 0), f32_at(uv, 4)]));
                                // the third vertex's UV w is the triangle's winding (VU1 culls by it, as the
                                // collision sweep orients by it): ≥ 0 keeps strip order, else reversed
                                for t in &mut pk.triangles[tri_start..] {
                                    let i = t[0].max(t[1]).max(t[2]);
                                    let w = data.get((i as usize - strip_start) * 16 + 12..).map_or(1.0, |b| f32_at(b, 0));
                                    *t = if 0.0 <= w { [i - 2, i - 1, i] } else { [i, i - 1, i - 2] };
                                }
                                slot = 0;
                                continue;
                            }
                        }
                        slot += 1;
                    }
                    (3, 1, _) => {
                        let mut kick = Vec::new();
                        pk.entry_flags.extend(data.chunks_exact(8).map(|nb| u16::from_le_bytes([nb[6], nb[7]])));
                        for (i, (v, nb)) in pk.vertices[strip_start..].iter_mut().zip(data.chunks_exact(8)).enumerate() {
                            let s = |o: usize| i16::from_le_bytes([nb[o], nb[o + 1]]);
                            v.normal = [s(0) as f32 / 16384.0, s(2) as f32 / 16384.0, s(4) as f32 / 16384.0];
                            if s(6) as u16 & 0x8000 == 0 && i >= 2 {
                                kick.push(strip_start + i);
                            }
                        }
                        for i in kick {
                            let i = i as u32;
                            // alternate winding along the strip so all faces keep one orientation
                            pk.triangles.push(if (i - strip_start as u32) % 2 == 0 { [i - 2, i - 1, i] } else { [i - 1, i - 2, i] });
                        }
                    }
                    (3, 2, _) => {
                        for (v, c) in pk.vertices[strip_start..].iter_mut().zip(data.chunks_exact(4)) {
                            v.color = [c[0], c[1], c[2], c[3]];
                        }
                        pk.colors.extend(data.chunks_exact(4).take(n).map(|c| [c[0], c[1], c[2], c[3]]));
                    }
                    (3, 0, true) => {
                        let c = row.map(|r| r.min(255) as u8);
                        for v in &mut pk.vertices[strip_start..] {
                            v.color = c;
                        }
                        pk.colors.extend(std::iter::repeat_n(c, n));
                    }
                    _ => return Err(Error(format!("vif: unexpected unpack {cmd:#x}"))),
                }
            }
            _ => break, // end of VIF codes; the rest of the qword-padded block is filler
        }
    }
    Ok(pk)
}
