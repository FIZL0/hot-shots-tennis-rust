//! `.MTL` materials + `.MTI` texture pixels.
//!
//! ```text
//! MTL: u32 material count
//!      u32 n, n × texture header (0x48)          pixels/palettes for these live in the MTI
//!      u32 n, n × { texture header, pixels, palette, 0x10 nibble-usage map }   embedded
//!      materials × { 0x30 header, extra (+0x1c bytes) }    +0: f32 RGBA (128 = 1.0), +0x20: i16 texture index
//! texture header: +0 i16 palette entries, +2 u16 width, +4 u16 height, +0xc u8 GS PSM,
//!                 +0x10 7 × { u32 mip size, u16 w, u16 h }
//! MTI: per MTI texture, its mip levels then palette (u32 RGBA, 0x80 = opaque), each 16-aligned.
//! ```
//! The embedded textures double as collision attribute maps: a hit's texel (4-bit palette index) picks a ground
//! material id from the 16-byte map that follows the palette.

use crate::xb::Error;

pub struct Texture {
    pub width: u32,
    pub height: u32,
    /// Top mip level, RGBA8 with PS2 alpha expanded.
    pub rgba: Vec<u8>,
}

pub struct Material {
    /// Index into `Mtl::textures` (MTI textures first, then embedded).
    pub texture: Option<usize>,
    /// RGBA modulate colour, 1.0 = PS2 0x80.
    pub color: [f32; 4],
    /// Index into `Mtl::attributes`.
    pub attributes: Option<usize>,
    /// Collision tests both windings.
    pub two_sided: bool,
    /// The material's name from its extra bytes (3ds Max, e.g. `spark1`, `2 - Default@add`); `.MTA` tracks
    /// are matched to materials by it.
    pub name: String,
    pub header: [u8; 0x30],
}

/// An embedded texture as raw collision data.
pub struct AttributeMap {
    pub header: [u8; 0x48],
    /// Top mip level, unconverted.
    pub texels: Vec<u8>,
    /// Material id per palette index.
    pub table: [u8; 0x10],
}

pub struct Mtl {
    pub textures: Vec<Texture>,
    pub materials: Vec<Material>,
    pub attributes: Vec<AttributeMap>,
}

struct Cursor<'a> {
    d: &'a [u8],
    p: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let b = self.d.get(self.p..self.p + n).ok_or_else(|| Error(format!("mtl: need {n:#x} at {:#x}", self.p)))?;
        self.p = (self.p + n + 15) & !15;
        Ok(b)
    }
    fn count(&mut self) -> Result<usize, Error> {
        let v = i32::from_le_bytes(self.take(4)?.try_into().unwrap());
        usize::try_from(v).map_err(|_| Error(format!("mtl: negative count {v}")))
    }
}

fn texture(h: &[u8], src: &mut Cursor) -> Result<Texture, Error> {
    let u16_ = |o: usize| u16::from_le_bytes([h[o], h[o + 1]]);
    let (w, ht, psm) = (u16_(2) as u32, u16_(4) as u32, h[0xc]);
    let mut top = None;
    for i in 0..7 {
        let n = i32::from_le_bytes(h[0x10 + i * 8..0x14 + i * 8].try_into().unwrap());
        if n > 0 {
            let px = src.take(n as usize)?;
            top.get_or_insert(px);
        }
    }
    let entries = i16::from_le_bytes([h[0], h[1]]).max(0) as usize;
    let mut pal: Vec<[u8; 4]> = src.take(entries * 4)?.chunks_exact(4).map(|c| rgba32(c)).collect();
    if entries == 256 {
        // CSM1 layout: entries 8..15 and 16..23 of every 32 are swapped
        for block in pal.chunks_mut(32) {
            for j in 8..16 {
                block.swap(j, j + 8);
            }
        }
    }
    let px = top.unwrap_or(&[]);
    let n = (w * ht) as usize;
    let get = |i: usize| -> [u8; 4] {
        let idx = match psm {
            0x13 => px.get(i).copied().unwrap_or(0) as usize,
            0x14 => px.get(i / 2).map_or(0, |b| if i & 1 == 0 { b & 0xf } else { b >> 4 }) as usize,
            0x00 => return px.get(i * 4..i * 4 + 4).map_or([0; 4], rgba32),
            0x01 => return px.get(i * 3..i * 3 + 3).map_or([0; 4], |c| [c[0], c[1], c[2], 255]),
            _ => return [255, 0, 255, 255], // unknown format: loud magenta
        };
        pal.get(idx).copied().unwrap_or([255, 0, 255, 255])
    };
    Ok(Texture { width: w, height: ht, rgba: (0..n).flat_map(get).collect() })
}

fn rgba32(c: &[u8]) -> [u8; 4] {
    [c[0], c[1], c[2], (c[3] as u16 * 255 / 128).min(255) as u8]
}

/// `mti` is the sibling `.MTI` file; models without one only use embedded textures.
pub fn parse(mtl: &[u8], mti: Option<&[u8]>) -> Result<Mtl, Error> {
    let mut c = Cursor { d: mtl, p: 0 };
    let mut src = Cursor { d: mti.unwrap_or(&[]), p: 0 };
    let nmat = c.count()?;
    let mut textures = Vec::new();
    for _ in 0..c.count()? {
        let h = c.take(0x48)?;
        textures.push(texture(h, &mut src)?);
    }
    let mut attributes = Vec::new();
    for _ in 0..c.count()? {
        let h = c.take(0x48)?;
        let top = c.p;
        textures.push(texture(h, &mut c)?);
        let mip0 = i32::from_le_bytes(h[0x10..0x14].try_into().unwrap()).max(0) as usize;
        attributes.push(AttributeMap {
            header: h.try_into().unwrap(),
            texels: mtl[top..top + mip0].to_vec(),
            table: c.take(0x10)?.try_into().unwrap(),
        });
    }
    let mut materials = Vec::with_capacity(nmat);
    for _ in 0..nmat {
        let h: [u8; 0x30] = c.take(0x30)?.try_into().unwrap();
        let extra = i16::from_le_bytes([h[0x1c], h[0x1d]]);
        let name = if extra > 0 { c.take(extra as usize)? } else { &[] };
        let name = String::from_utf8_lossy(name.split(|&b| b == 0).next().unwrap_or_default()).into_owned();
        let t = i16::from_le_bytes([h[0x20], h[0x21]]);
        let a = i16::from_le_bytes([h[0x22], h[0x23]]);
        let color = std::array::from_fn(|i| f32::from_le_bytes(h[i * 4..i * 4 + 4].try_into().unwrap()) / 128.0);
        materials.push(Material {
            texture: usize::try_from(t).ok().filter(|&t| t < textures.len()),
            color,
            attributes: usize::try_from(a).ok().filter(|&a| a < attributes.len()),
            two_sided: h[0x24] != 0,
            name,
            header: h,
        });
    }
    Ok(Mtl { textures, materials, attributes })
}

/// How a material blends with the frame buffer (GS ALPHA), chosen by a tag in its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend {
    /// `(Cs - Cd) * As + Cd`
    Normal,
    /// `@add`: `Cs * As + Cd`
    Add,
    /// `@sub`: `Cd - Cs * As`
    Sub,
}

impl Material {
    pub fn blend(&self) -> Blend {
        // the game tests @add first, then @sub (which wins)
        if self.name.contains("@sub") {
            Blend::Sub
        } else if self.name.contains("@add") {
            Blend::Add
        } else {
            Blend::Normal
        }
    }
}
