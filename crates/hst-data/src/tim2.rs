//! TIM2 (`.tm2`) textures, decoded to straight RGBA8.

use crate::xb::Error;

pub struct Picture {
    pub width: u32,
    pub height: u32,
    /// RGBA8, PS2 alpha already expanded (0x80 -> 0xff).
    pub rgba: Vec<u8>,
    /// Raw GS TEX0 register as authored; the renderer needs its wrap/function bits later.
    pub gs_tex0: u64,
    /// Raw texels (linear rows, 4-bit low nibble first).
    pub texels: Vec<u8>,
    /// Logical palette as the GS expands it (RGBA32, raw PS2 alpha; 16-bit entries with TA0 = 0, TA1 = 0x80).
    pub clut: Vec<[u8; 4]>,
}

impl Picture {
    /// The texture as the GS holds it, by its TEX0 (PCSX2 replacement names, content key).
    pub fn gs(&self) -> crate::texhash::GsTex<'_> {
        let t = self.gs_tex0;
        crate::texhash::GsTex { psm: (t >> 20 & 0x3f) as u8, tw: (t >> 26 & 15) as u32, th: (t >> 30 & 15) as u32, levels: vec![&self.texels], clut: &self.clut }
    }
}

fn rd<const N: usize>(d: &[u8], o: usize) -> Result<[u8; N], Error> {
    d.get(o..o + N).map(|b| b.try_into().unwrap()).ok_or_else(|| Error(format!("tim2 truncated at {o:#x}")))
}

pub fn decode(d: &[u8]) -> Result<Vec<Picture>, Error> {
    decode_with(d, true)
}

/// For the standalone sheets (INPANE, menus) whose alpha is authored 0–255 (0x80 is half): kept as is.
pub fn decode_alpha8(d: &[u8]) -> Result<Vec<Picture>, Error> {
    decode_with(d, false)
}

fn decode_with(d: &[u8], expand: bool) -> Result<Vec<Picture>, Error> {
    if d.get(..4) != Some(b"TIM2") {
        return Err(Error("not TIM2".into()));
    }
    let count = u16::from_le_bytes(rd(d, 6)?) as usize;
    let mut p = if d[5] == 1 { 0x80 } else { 0x10 }; // format 1 = 128-byte aligned
    let mut pics = Vec::with_capacity(count);
    for _ in 0..count {
        let h = rd::<0x30>(d, p)?;
        let u32_ = |o: usize| u32::from_le_bytes(h[o..o + 4].try_into().unwrap());
        let u16_ = |o: usize| u16::from_le_bytes(h[o..o + 2].try_into().unwrap());
        let (total, clut_size, image_size, header_size) = (u32_(0) as usize, u32_(4) as usize, u32_(8) as usize, u16_(12) as usize);
        let (clut_type, image_type) = (h[18], h[19]);
        let (w, ht) = (u16_(20) as u32, u16_(22) as u32);
        let gs_tex0 = u64::from_le_bytes(h[24..32].try_into().unwrap());
        let img = d.get(p + header_size..p + header_size + image_size).ok_or_else(|| Error("tim2 image truncated".into()))?;
        let clut = d
            .get(p + header_size + image_size..p + header_size + image_size + clut_size)
            .ok_or_else(|| Error("tim2 clut truncated".into()))?;
        let n = (w * ht) as usize;
        let rgba = match image_type {
            1..=3 => (0..n).map(|i| color(img, i, image_type, expand)).collect::<Result<Vec<_>, _>>()?,
            4 | 5 => {
                let palette = palette(clut, clut_type, image_type, expand)?;
                (0..n)
                    .map(|i| {
                        let idx = if image_type == 5 {
                            img.get(i).copied()
                        } else {
                            img.get(i / 2).map(|b| if i & 1 == 0 { b & 0xf } else { b >> 4 })
                        };
                        idx.and_then(|ix| palette.get(ix as usize).copied())
                            .ok_or_else(|| Error("tim2 index out of range".into()))
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            t => return Err(Error(format!("tim2 image type {t}"))),
        };
        let mut raw_clut = if matches!(image_type, 4 | 5) { palette(clut, clut_type, image_type, false)? } else { vec![] };
        if clut_type & 0x3f == 1 {
            // 16-bit entries: the GS widens 5 bits to 8 by shifting, alpha from TEXA
            let n = raw_clut.len();
            raw_clut = clut.chunks_exact(2).take(n).map(|c| u16::from_le_bytes([c[0], c[1]])).map(|v| {
                [((v & 31) << 3) as u8, ((v >> 5 & 31) << 3) as u8, ((v >> 10 & 31) << 3) as u8, if v & 0x8000 != 0 { 0x80 } else { 0 }]
            }).collect();
            if clut_type & 0x80 == 0 && image_type == 5 {
                for block in raw_clut.chunks_mut(32).filter(|c| c.len() == 32) {
                    for j in 8..16 {
                        block.swap(j, j + 8);
                    }
                }
            }
        }
        pics.push(Picture { width: w, height: ht, rgba: rgba.concat(), gs_tex0, texels: img.to_vec(), clut: raw_clut });
        p += total;
    }
    Ok(pics)
}

/// Pixel `i` of a direct-color buffer; `kind` 1 = RGBA5551, 2 = RGB888, 3 = RGBA8888.
fn color(b: &[u8], i: usize, kind: u8, expand: bool) -> Result<[u8; 4], Error> {
    let a = |x: u8| if expand { (x as u16 * 255 / 128).min(255) as u8 } else { x };
    let e = || Error("tim2 pixel out of range".into());
    Ok(match kind {
        1 => {
            let v = u16::from_le_bytes(b.get(i * 2..i * 2 + 2).ok_or_else(e)?.try_into().unwrap());
            let c = |s: u16| ((v >> s & 31) * 255 / 31) as u8;
            [c(0), c(5), c(10), if v & 0x8000 != 0 { 255 } else { 0 }]
        }
        2 => {
            let p = b.get(i * 3..i * 3 + 3).ok_or_else(e)?;
            [p[0], p[1], p[2], 255]
        }
        _ => {
            let p = b.get(i * 4..i * 4 + 4).ok_or_else(e)?;
            [p[0], p[1], p[2], a(p[3])]
        }
    })
}

fn palette(clut: &[u8], clut_type: u8, image_type: u8, expand: bool) -> Result<Vec<[u8; 4]>, Error> {
    let kind = clut_type & 0x3f;
    let size = [0, 2, 3, 4][kind.min(3) as usize];
    let mut pal = (0..clut.len() / size.max(1)).map(|i| color(clut, i, kind, expand)).collect::<Result<Vec<_>, _>>()?;
    // CSM1 8-bit CLUTs are stored with entries 8..15 and 16..23 of every 32 swapped.
    if clut_type & 0x80 == 0 && image_type == 5 {
        for block in pal.chunks_mut(32).filter(|c| c.len() == 32) {
            for j in 8..16 {
                block.swap(j, j + 8);
            }
        }
    }
    Ok(pal)
}
