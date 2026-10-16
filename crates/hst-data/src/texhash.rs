//! PCSX2's texture-replacement names for disc textures, so a PCSX2 pack (`mods/texture-replacements/`) can be applied.
//!
//! PCSX2 names a texture `<tex>-<clut>-<bits>.png` (`<tex>-<bits>.png` without a palette): `tex` is XXH3-64 of
//! the texture as it sits in GS memory, `clut` XXH3-64 of the palette as 16 or 256 RGBA32 entries (raw PS2 alpha,
//! logical order), `bits` PSM | TW << 6 | TH << 10 (| TEXA for 16/24-bit). How the texture bytes are hashed:
//! - 32-bit, 8-bit and 4-bit textures of at least one GS block: every block (256 bytes, in its swizzled layout),
//!   blocks in raster order;
//! - anything smaller than a block, and 16/24-bit: the texture expanded row by row (8-bit indices for palettes,
//!   RGBA32 otherwise).
//! With mipmapping on, each level of the drawn LOD range is hashed after the base into the same digest.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use twox_hash::XxHash3_64;

/// One disc texture as the GS sees it.
pub struct GsTex<'a> {
    pub psm: u8,
    /// log2 of the GS width/height of level 0.
    pub tw: u32,
    pub th: u32,
    /// Raw texel data per mip level, linear rows (4-bit: low nibble first).
    pub levels: Vec<&'a [u8]>,
    /// Logical palette, raw PS2 alpha; empty for direct colour.
    pub clut: &'a [[u8; 4]],
}

const PSMCT32: u8 = 0x00;
const PSMCT24: u8 = 0x01;
const PSMCT16: u8 = 0x02;
const PSMT8: u8 = 0x13;
const PSMT4: u8 = 0x14;

/// Block size in pixels and bits per pixel of the formats the game's textures use.
fn format(psm: u8) -> Option<((u32, u32), u32)> {
    Some(match psm {
        PSMCT32 => ((8, 8), 32),
        PSMCT24 => ((8, 8), 24),
        PSMCT16 => ((16, 8), 16),
        PSMT8 => ((16, 16), 8),
        PSMT4 => ((32, 16), 4),
        _ => return None,
    })
}

/// Position of pixel (x, y) inside its 256-byte GS block, in units of the format's pixel (u32, byte or nibble).
fn in_block(psm: u8, x: u32, y: u32) -> u32 {
    if psm == PSMCT32 {
        return (y >> 1) << 4 | (x >> 1) << 2 | (y & 1) << 1 | (x & 1);
    }
    // 8- and 4-bit blocks are 4 columns of 4 rows; in every other column the half rows swap 4 pixels
    let (c, r) = (y >> 2, y & 3);
    let x8 = if (r >> 1) ^ (c & 1) != 0 {
        (x + 4) & 7
    } else {
        x & 7
    };
    if psm == PSMT8 {
        c * 64 + (x8 >> 1) * 16 + (x8 & 1) * 4 + ((x >> 3) & 1) * 2 + (r & 1) * 8 + (r >> 1)
    } else {
        c * 128 + (x8 >> 1) * 32 + (x8 & 1) * 8 + ((x >> 3) & 3) * 2 + (r & 1) * 16 + (r >> 1)
    }
}

fn texel(psm: u8, d: &[u8], i: usize) -> u32 {
    let at = |o: usize, n: usize| {
        (0..n).fold(0u32, |v, k| {
            v | (d.get(o + k).copied().unwrap_or(0) as u32) << (8 * k)
        })
    };
    match psm {
        PSMT4 => at(i / 2, 1) >> (4 * (i & 1)) & 0xf,
        PSMT8 => at(i, 1),
        PSMCT16 => at(i * 2, 2),
        PSMCT24 => at(i * 3, 3),
        _ => at(i * 4, 4),
    }
}

/// The bytes PCSX2 hashes for one level of `w`×`h` pixels (the data's own width is `dw`).
fn level_bytes(psm: u8, d: &[u8], w: u32, h: u32, dw: u32, texa: (u8, bool, u8)) -> Vec<u8> {
    let ((bw, bh), bpp) = format(psm).unwrap();
    let get = |x: u32, y: u32| {
        if x < dw {
            texel(psm, d, (y * dw + x) as usize)
        } else {
            0
        }
    };
    if w < bw || h < bh || psm == PSMCT24 || psm == PSMCT16 {
        let (ta0, aem, ta1) = texa;
        return (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let c = get(x, y);
                match psm {
                    PSMT8 | PSMT4 => vec![c as u8],
                    PSMCT32 => c.to_le_bytes().to_vec(),
                    PSMCT24 => vec![
                        c as u8,
                        (c >> 8) as u8,
                        (c >> 16) as u8,
                        if aem && c == 0 { 0 } else { ta0 },
                    ],
                    _ => {
                        let ch = |s: u32| ((c >> s & 31) << 3) as u8;
                        let a = if c & 0x8000 != 0 {
                            ta1
                        } else if aem && c & 0x7fff == 0 {
                            0
                        } else {
                            ta0
                        };
                        vec![ch(0), ch(5), ch(10), a]
                    }
                }
            })
            .collect();
    }
    let mut out = vec![0u8; (w * h * bpp / 8) as usize];
    for by in 0..h / bh {
        for bx in 0..w / bw {
            let block = &mut out[((by * (w / bw) + bx) * 256) as usize..][..256];
            for y in 0..bh {
                for x in 0..bw {
                    let (c, o) = (get(bx * bw + x, by * bh + y), in_block(psm, x, y) as usize);
                    match psm {
                        PSMT4 => block[o / 2] |= (c as u8) << (4 * (o & 1)),
                        PSMT8 => block[o] = c as u8,
                        _ => block[o * 4..o * 4 + 4].copy_from_slice(&c.to_le_bytes()),
                    }
                }
            }
        }
    }
    out
}

impl GsTex<'_> {
    /// PCSX2's file name (without `.png`) when it draws levels `first..=last`; `None` for formats it can't be.
    /// `texa` = (TA0, AEM, TA1), used by 16/24-bit textures only.
    pub fn pcsx2_name(
        &self,
        first: usize,
        last: usize,
        data_width: u32,
        texa: (u8, bool, u8),
    ) -> Option<String> {
        format(self.psm)?;
        let mut bytes = Vec::new();
        for k in first..=last {
            let (w, h) = (
                1 << self.tw.saturating_sub(k as u32),
                1 << self.th.saturating_sub(k as u32),
            );
            bytes.extend(level_bytes(
                self.psm,
                self.levels.get(k)?,
                w,
                h,
                (data_width >> k).max(1),
                texa,
            ));
        }
        let (tw, th) = (
            self.tw.saturating_sub(first as u32),
            self.th.saturating_sub(first as u32),
        );
        let mut bits = self.psm as u32 | tw << 6 | th << 10;
        if matches!(self.psm, PSMCT24 | PSMCT16) {
            bits |= (texa.0 as u32) << 15 | (texa.1 as u32) << 23 | (texa.2 as u32) << 24;
        }
        let tex = XxHash3_64::oneshot(&bytes);
        Some(if matches!(self.psm, PSMT8 | PSMT4) {
            let n = if self.psm == PSMT8 { 256 } else { 16 };
            let clut: Vec<u8> = (0..n)
                .flat_map(|i| self.clut.get(i).copied().unwrap_or([0; 4]))
                .collect();
            format!("{tex:x}-{:x}-{bits:08x}", XxHash3_64::oneshot(&clut))
        } else {
            format!("{tex:x}-{bits:08x}")
        })
    }

    /// Every name PCSX2 may give this texture: the base level alone and each mip range it may draw.
    pub fn pcsx2_names(&self, data_width: u32, texa: (u8, bool, u8)) -> Vec<String> {
        let n = self.levels.len();
        (0..n)
            .flat_map(|a| (a..n).map(move |b| (a, b)))
            .filter_map(|(a, b)| self.pcsx2_name(a, b, data_width, texa))
            .collect()
    }

    /// A content key for files that don't depend on PCSX2: XXH3-64 of the format, size, base level and palette.
    pub fn key(&self) -> String {
        let mut b = vec![self.psm, self.tw as u8, self.th as u8];
        b.extend_from_slice(self.levels.first().copied().unwrap_or_default());
        b.extend(self.clut.iter().flatten());
        format!("{:016x}", XxHash3_64::oneshot(&b))
    }
}

/// log2 of the GS size the game gives a texture `n` pixels wide/high (the next power of two).
pub fn log2_size(n: u32) -> u32 {
    n.max(1).next_power_of_two().trailing_zeros()
}

/// The TEXA the game leaves set (TA0 0, AEM off, TA1 0x80).
pub const TEXA: (u8, bool, u8) = (0, false, 0x80);

/// A texture on the disc, decoded.
pub struct DiscTexture {
    /// Readable name: archive path without `.XB`/`.XB0`, file path in it without extension, `.<index>`
    /// (`COURT/05/CMN/c05_court.0`).
    pub path: String,
    pub width: u32,
    pub height: u32,
    /// RGBA8, PS2 alpha expanded (0x80 -> 0xff).
    pub rgba: Vec<u8>,
    /// [`GsTex::key`].
    pub key: String,
    /// [`GsTex::pcsx2_names`].
    pub names: Vec<String>,
}

/// PCSX2's names for a TIM2 picture: as stored, and as the game uploads the 2D sheets whose palette alpha is
/// authored 0–255 (halved, rounding up).
pub fn tim2_names(p: &crate::tim2::Picture) -> Vec<String> {
    let mut names = p.gs().pcsx2_names(p.width, TEXA);
    let half: Vec<[u8; 4]> = p
        .clut
        .iter()
        .map(|&[r, g, b, a]| [r, g, b, ((a as u16 + 1) >> 1) as u8])
        .collect();
    if !half.is_empty() {
        names.extend(
            GsTex {
                clut: &half,
                ..p.gs()
            }
            .pcsx2_names(p.width, TEXA),
        );
    }
    names
}

/// Every MTL/MTI and TIM2 texture in the disc's archives.
pub fn disc_textures(iso: &mut crate::iso::Iso, mut f: impl FnMut(DiscTexture)) {
    let paths: Vec<String> = iso
        .files
        .keys()
        .filter(|p| p.ends_with(".XB") || p.ends_with(".XB0"))
        .cloned()
        .collect();
    for path in paths {
        let Ok(data) = iso.read(&path) else { continue };
        let Ok(arc) = crate::xb::Archive::parse(&data) else {
            continue;
        };
        for e in &arc.entries {
            let lower = e.name.to_ascii_lowercase();
            let (stem, ext) = lower.rsplit_once('.').unwrap_or((&lower, ""));
            let rel: Vec<&str> = e.name[..stem.len()]
                .split(['\\', '/'])
                .filter(|c| !matches!(*c, "" | "." | ".."))
                .collect();
            let base = format!(
                "{}/{}",
                path.trim_end_matches(".XB0").trim_end_matches(".XB"),
                rel.join("/")
            );
            let mut emit = |i: usize, w: u32, h: u32, rgba: Vec<u8>, g: GsTex, dw: u32| {
                f(DiscTexture {
                    path: format!("{base}.{i}"),
                    width: w,
                    height: h,
                    rgba,
                    key: g.key(),
                    names: g.pcsx2_names(dw, TEXA),
                })
            };
            match ext {
                "mtl" => {
                    let mti = arc
                        .find(&format!("{}.MTI", &e.name[..stem.len()]))
                        .and_then(|m| arc.read(m).ok());
                    let Ok(m) = arc
                        .read(e)
                        .map_err(|_| ())
                        .and_then(|b| crate::mtl::parse(&b, mti.as_deref()).map_err(|_| ()))
                    else {
                        continue;
                    };
                    for (i, t) in m.textures.into_iter().enumerate() {
                        emit(i, t.width, t.height, t.rgba.clone(), t.gs(), t.width);
                    }
                }
                "tm2" => {
                    let Ok(pics) = arc
                        .read(e)
                        .map_err(|_| ())
                        .and_then(|b| crate::tim2::decode(&b).map_err(|_| ()))
                    else {
                        continue;
                    };
                    for (i, p) in pics.into_iter().enumerate() {
                        let names = tim2_names(&p);
                        f(DiscTexture {
                            path: format!("{base}.{i}"),
                            width: p.width,
                            height: p.height,
                            key: p.gs().key(),
                            rgba: p.rgba,
                            names,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

/// A picture's pixels that aren't fully transparent.
fn opaque(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4).filter(|p| p[3] != 0).count()
}

/// M5c: what an upload's pack check keeps of a disc texture: its size and [`pixels_hash`].
pub type Print = (u32, u32, u64);

/// M5c: the disc textures under `keys`, in one pass over the disc ([`disc_textures`]): each one's [`Print`], and
/// the one with the most opaque pixels whole (its path breaks a tie), for the pack's before/after picture: by area
/// alone it was a font sheet, mostly see-through.
pub fn disc_prints(
    iso: &mut crate::iso::Iso,
    keys: &std::collections::HashSet<String>,
) -> (HashMap<String, Print>, Option<DiscTexture>) {
    let (mut prints, mut big) = (HashMap::new(), None::<DiscTexture>);
    disc_textures(iso, |t| {
        if !keys.contains(&t.key) || t.rgba.len() != (t.width * t.height * 4) as usize {
            return;
        }
        prints.insert(
            t.key.clone(),
            (t.width, t.height, pixels_hash(t.width, t.height, &t.rgba)),
        );
        let size = |t: &DiscTexture| (opaque(&t.rgba), std::cmp::Reverse(t.path.clone()));
        if big.as_ref().is_none_or(|b| size(&t) > size(b)) {
            big = Some(t);
        }
    });
    (prints, big)
}

/// M5c: XXH3-64 of a picture's size and straight RGBA8 pixels, every fully transparent pixel counted as 0 (an
/// image editor may keep or clear their colour).
pub fn pixels_hash(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let mut b = Vec::with_capacity(8 + rgba.len());
    b.extend_from_slice(&w.to_le_bytes());
    b.extend_from_slice(&h.to_le_bytes());
    b.extend(rgba.chunks(4).flat_map(|p| if p[3] == 0 { [0; 4] } else { [p[0], p[1], p[2], p[3]] }));
    XxHash3_64::oneshot(&b)
}

/// M5c: whether replacement `rgba` (`w`×`h`) is the disc texture printed `disc` unchanged: the same pixels, or
/// them scaled up by whole factors with nearest-neighbour (each pixel a solid block).
pub fn is_disc_copy(w: u32, h: u32, rgba: &[u8], disc: Print) -> bool {
    let (dw, dh, hash) = disc;
    if dw == 0 || dh == 0 || w % dw != 0 || h % dh != 0 || rgba.len() != (w * h * 4) as usize {
        return false;
    }
    let (fx, fy) = ((w / dw) as usize, (h / dh) as usize);
    let px = |x: usize, y: usize| {
        let p = &rgba[(y * w as usize + x) * 4..][..4];
        if p[3] == 0 { [0; 4] } else { [p[0], p[1], p[2], p[3]] }
    };
    let mut small = Vec::with_capacity((dw * dh * 4) as usize);
    for y in 0..dh as usize {
        for x in 0..dw as usize {
            let p = px(x * fx, y * fy);
            let solid = (0..fy).all(|j| (0..fx).all(|i| px(x * fx + i, y * fy + j) == p));
            if !solid {
                return false;
            }
            small.extend_from_slice(&p);
        }
    }
    pixels_hash(dw, dh, &small) == hash
}

/// User textures that replace disc ones, all in `mods/texture-replacements/` beside the ISO: `**/<path>-<key>.png`
/// (straight alpha, found by [`GsTex::key`], e.g. upscaled `--dump-textures` output) over a PCSX2 pack's
/// hash-named `<name>.png` at its top level (PS2 alpha, 0x80 opaque).
#[derive(Default)]
pub struct Overrides {
    pub root: PathBuf,
    /// key -> (file, modified)
    pub mods: HashMap<String, (PathBuf, SystemTime)>,
    pub pack: HashMap<String, PathBuf>,
}

impl Overrides {
    pub fn scan(root: &Path) -> Self {
        let pack = std::fs::read_dir(root.join("mods/texture-replacements"))
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let p = e.ok()?.path();
                Some((p.file_name()?.to_str()?.strip_suffix(".png")?.to_owned(), p))
            })
            .collect();
        Overrides {
            root: root.into(),
            mods: Self::scan_mods(root),
            pack,
        }
    }

    /// The mod folder's files by key.
    pub fn scan_mods(root: &Path) -> HashMap<String, (PathBuf, SystemTime)> {
        fn walk(d: &Path, out: &mut HashMap<String, (PathBuf, SystemTime)>) {
            for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if let Some(key) = p
                    .file_name()
                    .and_then(|n| n.to_str()?.strip_suffix(".png")?.rsplit_once('-'))
                    .map(|(_, k)| k.to_owned())
                {
                    out.insert(
                        key,
                        (
                            p.clone(),
                            e.metadata()
                                .and_then(|m| m.modified())
                                .unwrap_or(SystemTime::UNIX_EPOCH),
                        ),
                    );
                }
            }
        }
        let mut out = HashMap::new();
        walk(&root.join("mods/texture-replacements"), &mut out);
        out
    }

    /// The replacement for a texture (width, height, straight RGBA8): mod file, then pack, else `None` (disc).
    pub fn load(&self, key: &str, names: &[String]) -> Option<(u32, u32, Vec<u8>)> {
        self.files(key, names).load()
    }

    /// Where [`load`](Self::load) reads from, to read it without holding `self` (the decode is the slow part).
    pub fn files(&self, key: &str, names: &[String]) -> Files {
        Files {
            m: self.mods.get(key).map(|(p, _)| p.clone()),
            pack: names.iter().find_map(|n| self.pack.get(n)).cloned(),
        }
    }

    /// The pack's PNG under any of PCSX2's names, alpha expanded.
    pub fn from_pack(&self, names: &[String]) -> Option<(u32, u32, Vec<u8>)> {
        Files { m: None, pack: names.iter().find_map(|n| self.pack.get(n)).cloned() }.load()
    }

    /// Write every disc texture as it is on the disc (native size, straight 0..255 alpha) to
    /// `mods/textures-src/<path>-<key>.png`: the input for upscaling, whose output goes in `mods/texture-replacements/` under
    /// the same names. Not the pack's PNGs, so every file is the same kind of input.
    pub fn dump(&self, iso: &mut crate::iso::Iso) -> usize {
        let mut n = 0;
        disc_textures(iso, |t| {
            let p = self
                .root
                .join(format!("mods/textures-src/{}-{}.png", t.path, t.key));
            if t.rgba.len() == (t.width * t.height * 4) as usize && t.width * t.height > 0 {
                let _ = std::fs::create_dir_all(p.parent().unwrap());
                n += write_png(&p, t.width, t.height, &t.rgba).is_ok() as usize;
            }
        });
        n
    }
}

/// A texture's replacement files: the mod folder's, then the pack's.
pub struct Files {
    m: Option<PathBuf>,
    pack: Option<PathBuf>,
}

impl Files {
    /// The replacement (width, height, straight RGBA8), the pack's alpha expanded, else `None` (disc).
    pub fn load(&self) -> Option<(u32, u32, Vec<u8>)> {
        if let Some(t) = self.m.as_deref().and_then(read_png) {
            return Some(t);
        }
        let (w, h, mut rgba) = read_png(self.pack.as_deref()?)?;
        rgba.iter_mut()
            .skip(3)
            .step_by(4)
            .for_each(|a| *a = (*a as u32 * 255 / 128).min(255) as u8);
        Some((w, h, rgba))
    }
}

/// A PNG as straight RGBA8.
pub fn read_png(p: &Path) -> Option<(u32, u32, Vec<u8>)> {
    decode_png(std::io::BufReader::new(std::fs::File::open(p).ok()?))
}

/// [`read_png`] of a PNG already read in (a browser mod file).
pub fn decode_png(r: impl std::io::BufRead + std::io::Seek) -> Option<(u32, u32, Vec<u8>)> {
    let mut d = png::Decoder::new(r);
    d.set_transformations(png::Transformations::normalize_to_color8());
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .chunks(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&c| [c, c, c, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some((info.width, info.height, rgba))
}

pub fn write_png(p: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), png::EncodingError> {
    let mut enc = png::Encoder::new(std::io::BufWriter::new(std::fs::File::create(p)?), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swizzle_columns() {
        // first rows of the GS manual's PSMT8/PSMT4 column layouts
        assert_eq!(
            (0..8).map(|x| in_block(PSMT8, x, 2)).collect::<Vec<_>>(),
            [33, 37, 49, 53, 1, 5, 17, 21]
        );
        assert_eq!(
            (0..8).map(|x| in_block(PSMT8, x, 4)).collect::<Vec<_>>(),
            [96, 100, 112, 116, 64, 68, 80, 84]
        );
        assert_eq!(
            (24..32).map(|x| in_block(PSMT4, x, 3)).collect::<Vec<_>>(),
            [87, 95, 119, 127, 23, 31, 55, 63]
        );
        assert_eq!(
            (0..8).map(|x| in_block(PSMCT32, x, 1)).collect::<Vec<_>>(),
            [2, 3, 6, 7, 10, 11, 14, 15]
        );
    }

    #[test]
    fn mods_then_pack() {
        let root = std::env::temp_dir().join(format!("hst-texhash-{}", std::process::id()));
        std::fs::create_dir_all(root.join("mods/texture-replacements/A")).unwrap();
        std::fs::create_dir_all(root.join("mods/texture-replacements")).unwrap();
        write_png(&root.join("mods/texture-replacements/abc-1.png"), 1, 1, &[1, 2, 3, 0x40]).unwrap();
        let o = Overrides::scan(&root);
        let names = ["abc-1".to_string()];
        // pack: PS2 alpha expanded
        assert_eq!(o.load("k1", &names), Some((1, 1, vec![1, 2, 3, 127])));
        write_png(
            &root.join("mods/texture-replacements/A/x.0-k1.png"),
            1,
            1,
            &[9, 9, 9, 200],
        )
        .unwrap();
        let o = Overrides::scan(&root);
        // mod file: straight alpha, over the pack
        assert_eq!(o.load("k1", &names), Some((1, 1, vec![9, 9, 9, 200])));
        assert_eq!(o.load("k2", &[]), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn disc_copies() {
        let disc = [[1, 2, 3, 255], [4, 5, 6, 0], [7, 8, 9, 128], [0, 0, 0, 255]].concat();
        let print = (2, 2, pixels_hash(2, 2, &disc));
        assert!(is_disc_copy(2, 2, &disc, print));
        // a transparent pixel's colour doesn't count
        let mut cleared = disc.clone();
        cleared[4..7].copy_from_slice(&[0, 0, 0]);
        assert!(is_disc_copy(2, 2, &cleared, print));
        // 2x wide, 3x high, nearest-neighbour
        let big: Vec<u8> = (0..6).flat_map(|y| (0..4).map(move |x| (x / 2, y / 3))).flat_map(|(x, y)| disc[(y * 2 + x) * 4..][..4].to_vec()).collect();
        assert!(is_disc_copy(4, 6, &big, print));
        // one pixel off, a smoothed block, a size that isn't a whole multiple
        let mut off = disc.clone();
        off[0] = 2;
        assert!(!is_disc_copy(2, 2, &off, print));
        let mut smooth = big.clone();
        smooth[0] = 9;
        assert!(!is_disc_copy(4, 6, &smooth, print));
        assert!(!is_disc_copy(3, 2, &[0; 24], print));
    }

    /// M5c: the disc pass a pack upload makes (the whole disc, keys of a few). Skips without the ISO.
    #[test]
    fn prints_from_the_disc() {
        let Ok(mut iso) = crate::iso::Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let mut all = Vec::new();
        let t = std::time::Instant::now();
        disc_textures(&mut iso, |d| all.push((d.key, d.width, d.height, d.rgba)));
        eprintln!("{} disc textures in {:?}, {} MB of pixels", all.len(), t.elapsed(), all.iter().map(|a| a.3.len()).sum::<usize>() >> 20);
        let keys: std::collections::HashSet<String> = all.iter().step_by(97).map(|a| a.0.clone()).collect();
        let (prints, big) = disc_prints(&mut iso, &keys);
        assert_eq!(prints.len(), keys.len());
        let big = big.unwrap();
        assert!(all.iter().filter(|a| keys.contains(&a.0)).all(|a| opaque(&a.3) <= opaque(&big.rgba)));
        let (_, w, h, rgba) = all.iter().find(|a| a.0 == big.key).unwrap();
        assert!(is_disc_copy(*w, *h, rgba, prints[&big.key]));
    }
}
