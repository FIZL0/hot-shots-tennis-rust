//! The pause screen's words as the game sets them: `word.tm2`'s glyphs (P61: from `main_menu.rs` and
//! `main_menu/pause.rs`, as the match's intro and `menu_art` use them too).

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};

/// `word.tm2`'s glyphs, 12 to a 32-px row from y 32.
pub const FONT: [&str; 15] = [
    "aàáâãäåæbcçd",
    "ðeèéêëfghiìí",
    "îïjklmnñoòóô",
    "õöøœpqrsštuù",
    "úûũüvwxyýÿz°",
    "AÀÁÂÃÄÅÆBCÇD",
    "ÐEÈÉÊËFGHIÌÍ",
    "ÎÏJKLMNÑOÒÓÔ",
    "ÕÖØŒPQRSŠTUÙ",
    "ÚÛŨÜVWXYÝŸZ°",
    "1234567890-=",
    "˜€?!\"#£%&`'^",
    "_[]().®©™¿¡„",
    ":;+*@{},<>/¥",
    "«»$ß",
];

/// `word.tm2`'s ink span per glyph (cell x + first inked column, cell y, width), from its alpha.
pub fn glyphs(font: &[u8]) -> std::collections::HashMap<char, (f32, f32, f32)> {
    let pic = hst_data::tim2::decode_alpha8(font).expect("TIM2").remove(0);
    let mut glyphs = std::collections::HashMap::new();
    for (row, chars) in FONT.iter().enumerate() {
        for (col, ch) in chars.chars().enumerate() {
            let (x0, y0) = (col * 32, 32 + row * 32);
            let ink: Vec<usize> = (0..32).filter(|x| (0..32).any(|y| pic.rgba[((y0 + y) * pic.width as usize + x0 + x) * 4 + 3] > 0x20)).collect();
            if let (Some(&a), Some(&b)) = (ink.first(), ink.last()) {
                glyphs.entry(ch).or_insert(((x0 + a) as f32, y0 as f32, (b - a + 1) as f32));
            }
        }
    }
    glyphs
}

/// One sheet out of a disc archive.
pub fn sheet(iso: &mut Iso, xb: &str, name: &str) -> Vec<u8> {
    let data = iso.read(xb).expect("menu archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name)).unwrap_or_else(|| panic!("{name} in {xb}"));
    arc.read(e).expect("archive bytes")
}

/// `s` in `word.tm2`, white, centred on a 256×32 sheet as one of `i_dialog_01`'s rows (the dialog has no "Settings"),
/// set again in the remastered font (`menu_art::relabel`) unless `--original`.
pub fn label(iso: &mut Iso, images: &mut Assets<Image>, s: &str) -> Handle<Image> {
    let font = sheet(iso, "MENU/MENU04.XB0", "/word.tm2");
    let glyphs = glyphs(&font);
    let pic = hst_data::tim2::decode_alpha8(&font).expect("TIM2").remove(0);
    let (w, h) = (256usize, 32usize);
    // the glyphs' coverage, then white over a 2-px dark outline as the dialog's own rows are drawn
    let width: f32 = s.chars().map(|c| glyphs.get(&c).map_or(10.0, |g| g.2 + 2.0)).sum::<f32>() - 2.0;
    // B171: words wider than the row are set on a wider one (room for the outline), then shrunk to fit it
    let wide = w.max(width.ceil() as usize + 4);
    let mut ink = vec![0f32; wide * h];
    let mut x = ((wide as f32 - width) / 2.0).max(0.0) as usize;
    for c in s.chars() {
        let Some(&(u, v, gw)) = glyphs.get(&c) else {
            x += 10;
            continue;
        };
        for dy in 0..h {
            for dx in 0..gw as usize {
                if x + dx < wide {
                    ink[dy * wide + x + dx] = pic.rgba[((v as usize + dy) * pic.width as usize + u as usize + dx) * 4 + 3] as f32 / 255.0;
                }
            }
        }
        x += gw as usize + 2;
    }
    let ink = shrink(&ink, wide, w, h);
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let near = (-2..=2i32).flat_map(|dy| (-2..=2i32).map(move |dx| (dx, dy))).filter(|(dx, dy)| dx * dx + dy * dy <= 5);
            let edge = near.filter_map(|(dx, dy)| (x + dx >= 0 && y + dy >= 0 && x + dx < w as i32 && y + dy < h as i32).then(|| ink[((y + dy) * w as i32 + x + dx) as usize])).fold(0f32, f32::max);
            let a = ink[(y * w as i32 + x) as usize];
            let c = (30.0 + 225.0 * a) as u8;
            rgba[(y as usize * w + x as usize) * 4..][..4].copy_from_slice(&[c, c, c, (edge.max(a) * 255.0) as u8]);
        }
    }
    if let Some(h) = super::menu_art::relabel(images, w, h, &rgba, s) {
        return h;
    }
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    images.add(Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, rgba, TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::RENDER_WORLD))
}

/// A `wide`×`h` coverage row scaled by `w / wide` both ways (box filter), centred on a `w`×`h` one.
pub fn shrink(ink: &[f32], wide: usize, w: usize, h: usize) -> Vec<f32> {
    if wide == w {
        return ink.to_vec();
    }
    let f = w as f32 / wide as f32;
    let y0 = h as f32 * (1.0 - f) / 2.0;
    let mut out = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            // 4×4 samples over the source square under this pixel
            let mut sum = 0.0;
            for k in 0..16 {
                let (sx, sy) = ((x as f32 + (k % 4) as f32 / 4.0 + 0.125) / f, (y as f32 + (k / 4) as f32 / 4.0 + 0.125 - y0) / f);
                if sy >= 0.0 && (sy as usize) < h {
                    sum += ink[sy as usize * wide + (sx as usize).min(wide - 1)];
                }
            }
            out[y * w + x] = sum / 16.0;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A full 512×32 row shrinks to half height, centred, edge to edge (B171).
    #[test]
    fn shrink_fits() {
        let out = shrink(&vec![1.0; 512 * 32], 512, 256, 32);
        let col: Vec<f32> = (0..32).map(|y| out[y * 256 + 128]).collect();
        assert!(col[..8].iter().chain(&col[24..]).all(|&a| a == 0.0) && col[8..24].iter().all(|&a| a == 1.0), "{col:?}");
        assert!(out[16 * 256] == 1.0 && out[16 * 256 + 255] == 1.0);
    }
}
