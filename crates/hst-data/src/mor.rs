//! `.MOR` face-morph and `.UVA` UV-animation tracks, one file per motion. 16-byte-aligned reads:
//!
//! ```text
//! u32 ticks per frame (80), u32 track count
//! per track: u32 name size (with NUL), u32 key count
//!            name     (MOR: a morph target of the model, `face\x01blink_eye`; UVA: a texture)
//!            u32 tick × keys
//!            value × keys (MOR: f32 weight, UVA: f32 × 4, the UV offset in x, y)
//! ```

use crate::xb::Error;

#[derive(Debug, Default, PartialEq)]
pub struct Track {
    pub name: String,
    pub ticks: Vec<i32>,
    /// MOR: the weight in `x`, rest 0.
    pub values: Vec<[f32; 4]>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Tracks {
    pub ticks_per_frame: i32,
    pub tracks: Vec<Track>,
}

/// `width` values per key: 1 for `.MOR`, 4 for `.UVA`.
pub fn parse(d: &[u8], width: usize) -> Result<Tracks, Error> {
    let mut p = 0;
    let mut take = |n: usize| -> Result<&[u8], Error> {
        let b = d.get(p..p + n).ok_or_else(|| Error(format!("mor: need {n:#x} bytes at {p:#x} of {:#x}", d.len())))?;
        p = (p + n + 15) & !15;
        Ok(b)
    };
    let i32_at = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let size = |v: i32| usize::try_from(v).map_err(|_| Error(format!("mor: negative size {v}")));
    let h = take(8)?;
    let mut out = Tracks { ticks_per_frame: i32_at(h, 0), tracks: Vec::new() };
    for _ in 0..size(i32_at(h, 4))? {
        let th = take(8)?;
        let (name, n) = (size(i32_at(th, 0))?, size(i32_at(th, 4))?);
        let name = take(name)?;
        let name = String::from_utf8_lossy(name.split(|&b| b == 0).next().unwrap_or_default()).into_owned();
        let ticks = take(4 * n)?.chunks_exact(4).map(|b| i32_at(b, 0)).collect();
        let values = take(4 * width * n)?
            .chunks_exact(4 * width)
            .map(|b| std::array::from_fn(|k| if k < width { f32::from_le_bytes(b[4 * k..4 * k + 4].try_into().unwrap()) } else { 0.0 }))
            .collect();
        out.tracks.push(Track { name, ticks, values });
    }
    Ok(out)
}
