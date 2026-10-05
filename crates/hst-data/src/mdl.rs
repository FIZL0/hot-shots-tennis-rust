//! `.MDL` models. The file is a sequence of 16-byte-aligned reads:
//!
//! ```text
//! u8 version
//! node tree      : node = 0x40 header (+0x38 extra size), extra, 3 × mat4, u32 n, n × group
//!                  group = 0x40 header, extra, 3 × mat4, u32 n, n × node
//! u32 material count (== MTL material count)
//!   per material : 0xc header (+0 batch count)
//!     per batch  : 0x34 header (+0x1c packet count)
//!       per packet: 0x60 header, VIF stream (+0x34 qwords), bone list (+0x3c + 1 bytes),
//!                   +0x38 bytes, +0x38 × 32 bytes, [+0x57 qwords if +0x56], +0x54 × sub-blocks,
//!                   [4 bytes if +0x4c < 0]
//! u32 n, n × { u32 size, data }
//! ```
//! Geometry is the VU1 upload itself: per strip a GIF tag, V4-32 positions, V4-16 normals whose
//! `w` bit 15 is the GS no-kick flag (strip restart), a colour (STROW constant or V4-8), V4-32 UVs.

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
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<[u32; 3]>,
    /// Matrix palette indices used by this packet (skinned models).
    pub bones: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Model {
    pub node_count: usize,
    /// `materials[m]` = packets drawn with MTL material `m`.
    pub materials: Vec<Vec<Packet>>,
}

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
fn tree_item(c: &mut Cursor, depth: u32, count: &mut usize) -> Result<(), Error> {
    if depth > 64 {
        return Err(Error("mdl: node tree too deep".into()));
    }
    let h = c.take(0x40)?;
    c.take(size_at(h, 0x38)?)?;
    c.take(0xc0)?;
    *count += 1;
    for _ in 0..c.count(4)? {
        tree_item(c, depth + 1, count)?;
    }
    Ok(())
}

pub fn parse(d: &[u8]) -> Result<Model, Error> {
    let mut c = Cursor { d, p: 0 };
    let mut m = Model::default();
    c.take(1)?;
    tree_item(&mut c, 0, &mut m.node_count)?;
    for _ in 0..c.count(4)? {
        let mut packets = Vec::new();
        for _ in 0..c.count(0xc)? {
            let bh = c.take(0x34)?;
            for _ in 0..size_at(bh, 0x1c)? {
                let ph = c.take(0x60)?;
                let vif = c.take(size_at(ph, 0x34)? << 4)?;
                let bones = c.take(size_at(ph, 0x3c)? + 1)?.to_vec();
                let n = size_at(ph, 0x38)?;
                c.take(n)?;
                c.take(n << 5)?;
                if ph[0x56] != 0 {
                    c.take((ph[0x57] as usize) << 4)?;
                }
                for _ in 0..i16::from_le_bytes([ph[0x54], ph[0x55]]).max(0) {
                    let k = c.count(0xc)?;
                    if k != 0 {
                        c.take(k << 4)?;
                    }
                }
                if i32_at(ph, 0x4c) < 0 {
                    c.take(4)?;
                }
                let mut pk = decode_vif(vif)?;
                pk.bones = bones;
                packets.push(pk);
            }
        }
        m.materials.push(packets);
    }
    let n = c.count(4)?;
    for _ in 0..n {
        let s = c.count(4)?;
        c.take(s)?;
    }
    Ok(m)
}

/// Walk a VIF stream and rebuild triangle strips from the unpacked vertex attributes.
fn decode_vif(d: &[u8]) -> Result<Packet, Error> {
    let mut pk = Packet::default();
    let (mut cl, mut wl) = (4u32, 4u32);
    let mut row = [0u32; 4];
    // Attributes arrive in a fixed order per strip; `slot` counts V4-32 unpacks seen in the strip.
    let mut slot = 0;
    let mut strip_start = 0usize;
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
                            }
                            1 => {
                                for v in data.chunks_exact(16) {
                                    pk.vertices.push(Vertex {
                                        pos: [f32_at(v, 0), f32_at(v, 4), f32_at(v, 8)],
                                        color: [0x80; 4],
                                        ..Default::default()
                                    });
                                }
                            }
                            _ => {
                                for (v, uv) in pk.vertices[strip_start..].iter_mut().zip(data.chunks_exact(16)) {
                                    v.uv = [f32_at(uv, 0), f32_at(uv, 4)];
                                }
                                slot = 0;
                                continue;
                            }
                        }
                        slot += 1;
                    }
                    (3, 1, _) => {
                        let mut kick = Vec::new();
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
                    }
                    (3, 0, true) => {
                        let c = row.map(|r| r.min(255) as u8);
                        for v in &mut pk.vertices[strip_start..] {
                            v.color = c;
                        }
                    }
                    _ => return Err(Error(format!("vif: unexpected unpack {cmd:#x}"))),
                }
            }
            _ => break, // end of VIF codes; the rest of the qword-padded block is filler
        }
    }
    Ok(pk)
}
