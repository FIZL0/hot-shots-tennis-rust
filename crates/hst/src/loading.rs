//! The original's "Now Loading..." screen before a match (B43): the court's picture (`MENU/WALL/SUBnn.XB`, a
//! 640×448 BMP) full screen, the HST logo panel and a white pane from `Loading.tm2` (`MENU/MENU00B.XB0`) sliced
//! nine ways, one hint (`MENU/WALL/WALLnn.XB0` → `L_txtNN.tm2`, 512×256, text and pictures baked in) on the pane, and
//! "Now Loading..." bottom right. Rects are the original's, on its 640×448 screen; the screen is the centred 4:3
//! picture (`widescreen.rs`), the court picture fills the window's width. Shown only while the port loads (`mode.rs`).
//!
//! Which hint: the original keeps a "seen" bit per hint and game mode in the save (here `hints.txt` beside the disc).
//! Hint 0 (the welcome) comes first; then the mode's first list in order while any of it is unseen; then a random
//! unseen one of its second list; once all are seen, a random one of both.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use hst_data::{iso::Iso, tim2, xb::Archive};

/// Fun Time Tennis's hints (the port's only mode): the first list, shown in order, then the second.
const FIRST: [usize; 4] = [1, 2, 3, 7];
const SECOND: [usize; 13] = [4, 5, 6, 11, 12, 13, 15, 28, 29, 30, 31, 35, 37];
/// Its bit in a hint's seen flags (Challenge and Training use 2; 1 is "seen in any mode").
const BIT: u8 = 4;
const HINTS: usize = 38;

/// Picks a hint and marks it seen; `rand` is the original's random number (bits 16..30 taken, mod the list).
fn pick(seen: &mut [u8; HINTS], rand: u32) -> usize {
    let unseen = |l: &[usize]| l.iter().copied().filter(|&h| seen[h] & BIT == 0).collect::<Vec<_>>();
    let all: Vec<usize> = FIRST.iter().chain(&SECOND).copied().collect();
    let (any, first, second) = (unseen(&all), unseen(&FIRST), unseen(&SECOND));
    let r = (rand >> 16 & 0x7fff) as usize;
    let mut h = if any.is_empty() {
        all[r % all.len()]
    } else if first.is_empty() {
        second[r % second.len()]
    } else {
        first[0]
    };
    if seen[0] & BIT == 0 {
        h = 0;
    }
    seen[h] |= BIT | 1;
    if h == 0 {
        seen[0] |= 7;
    }
    h
}

fn path(iso: &str) -> std::path::PathBuf {
    std::path::Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("hints.txt")
}

/// The next hint, its seen flag saved (`hints.txt`: one hex byte per hint).
fn next_hint(iso: &str) -> usize {
    let mut seen = [0u8; HINTS];
    if let Ok(t) = crate::store::read_to_string(path(iso)) {
        for (s, b) in seen.iter_mut().zip(t.trim().as_bytes().chunks(2)) {
            *s = std::str::from_utf8(b).ok().and_then(|b| u8::from_str_radix(b, 16).ok()).unwrap_or(0);
        }
    }
    // ponytail: any random source does (front end, not the sim's RNG)
    let rand = std::hash::BuildHasher::hash_one(&std::collections::hash_map::RandomState::new(), 0u8) as u32;
    let h = pick(&mut seen, rand);
    let text: String = seen.iter().map(|b| format!("{b:02x}")).collect();
    if let Err(e) = crate::store::write(path(iso), text) {
        warn!("hints.txt: {e}");
    }
    h
}

fn entry(iso: &mut Iso, archive: &str, name: &str) -> Option<Vec<u8>> {
    let data = iso.read(archive).ok()?;
    let arc = Archive::parse(&data).ok()?;
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name))?;
    arc.read(e).ok()
}

fn image(rgba: Vec<u8>, w: u32, h: u32) -> Image {
    let size = Extent3d { width: w, height: h, depth_or_array_layers: 1 };
    Image::new(size, TextureDimension::D2, rgba, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)
}

/// A menu sheet (alpha authored 0–255, as the INPANE ones).
fn sheet(data: &[u8]) -> Option<Image> {
    let p = tim2::decode_alpha8(data).ok()?.into_iter().next()?;
    Some(image(p.rgba, p.width, p.height))
}

/// An 8-bit palettised Windows BMP (the court pictures).
fn bmp(d: &[u8]) -> Option<Image> {
    let u32_ = |o: usize| Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?));
    let (off, w, h) = (u32_(10)? as usize, u32_(18)?, u32_(22)?);
    if d.get(28) != Some(&8) {
        return None;
    }
    let pal = d.get(54..54 + 1024)?;
    let stride = (w as usize + 3) & !3;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in (0..h as usize).rev() {
        for &i in d.get(off + y * stride..off + y * stride + w as usize)? {
            let c = &pal[i as usize * 4..];
            rgba.extend([c[2], c[1], c[0], 255]);
        }
    }
    Some(image(rgba, w, h))
}

/// A piece of `Loading.tm2`: source rect, destination rect on the 640×448 screen, alpha.
type Piece = ([f32; 4], [f32; 4], f32);

/// Nine pieces of a frame: `uv` the four u and v cuts (u0 v0 u1 v1 u2 v2 u3 v3), centred at `cx, cy`, `w × h`.
fn nine(uv: [f32; 8], cx: f32, cy: f32, w: f32, h: f32, alpha: f32) -> Vec<Piece> {
    let (u, v) = ([uv[0], uv[2], uv[4], uv[6]], [uv[1], uv[3], uv[5], uv[7]]);
    let x = [cx - w / 2.0, cx - w / 2.0 + u[1] - u[0], cx + w / 2.0 - (u[3] - u[2]), cx + w / 2.0];
    let y = [cy - h / 2.0, cy - h / 2.0 + v[1] - v[0], cy + h / 2.0 - (v[3] - v[2]), cy + h / 2.0];
    let mut out = vec![];
    for j in 0..3 {
        for i in 0..3 {
            out.push(([u[i], v[j], u[i + 1] - u[i], v[j + 1] - v[j]], [x[i], y[j], x[i + 1] - x[i], y[j + 1] - y[j]], alpha));
        }
    }
    out
}

/// The pane and logo panel, back to front: the white pane and its rim (the sheet's own alpha lets the court show
/// through at ~0.4, as in PCSX2), then the logo panel over them.
fn pieces() -> Vec<Piece> {
    let mut p = nine([184.0, 0.0, 200.0, 16.0, 216.0, 32.0, 232.0, 48.0], 320.0, 223.0, 524.0, 302.0, 1.0);
    p.extend(nine([184.0, 48.0, 200.0, 64.0, 216.0, 80.0, 232.0, 96.0], 320.0, 223.0, 516.0, 294.0, 1.0));
    p.extend(nine([0.0, 0.0, 144.0, 104.0, 160.0, 120.0, 184.0, 152.0], 312.0, 200.0, 560.0, 368.0, 1.0));
    p
}

const WORDS: Piece = ([0.0, 152.0, 256.0, 104.0], [416.0, 392.0, 256.0, 104.0], 1.0);
const HINT: [f32; 4] = [64.0, 96.0, 512.0, 256.0];

/// What to draw on the loading cover before a match on `court` (the disc's `COURT/nn`).
pub fn cover(iso: String, court: usize) -> impl FnOnce(&mut World, Entity) + Send + Sync {
    move |world, root| {
        let Ok(mut disc) = Iso::open(&iso) else { return };
        let hint = next_hint(&iso);
        let wall = entry(&mut disc, &format!("MENU/WALL/WALL{:02}.XB0", hint + 1), &format!("/l_txt{hint:02}.tm2"));
        let back = court.checked_sub(1).and_then(|c| entry(&mut disc, &format!("MENU/WALL/SUB{court:02}.XB"), &format!("/l_bg{c:02}.bmp")));
        let art = entry(&mut disc, "MENU/MENU00B.XB0", "/loading.tm2");
        let mut images = world.resource_mut::<Assets<Image>>();
        let [wall, back, art] = [wall.as_deref().and_then(sheet), back.as_deref().and_then(bmp), art.as_deref().and_then(sheet)].map(|i| i.map(|i| images.add(i)));
        let abs = |[x, y, w, h]: [f32; 4]| Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(x / 6.4),
            top: Val::Percent(y / 4.48),
            width: Val::Percent(w / 6.4),
            height: Val::Percent(h / 4.48),
            ..default()
        };
        world.entity_mut(root).insert(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            ..default()
        });
        world.entity_mut(root).with_children(|p| {
            // the court picture fills the window's width (cropped top and bottom when wider than 4:3)
            if let Some(back) = back {
                p.spawn((ImageNode::new(back), Node { width: Val::Percent(100.0), aspect_ratio: Some(4.0 / 3.0), flex_shrink: 0.0, ..default() }));
            }
            // the 640×448 screen as a centred 4:3 picture, as the HUD (`widescreen.rs`)
            let screen = Node { position_type: PositionType::Absolute, height: Val::Percent(100.0), aspect_ratio: Some(4.0 / 3.0), max_width: Val::Percent(100.0), ..default() };
            p.spawn(screen).with_children(|s| {
                let Some(art) = art else { return };
                let piece = |s: &mut ChildSpawner, (src, dst, a): Piece| {
                    let img = ImageNode { image: art.clone(), rect: Some(Rect::new(src[0], src[1], src[0] + src[2], src[1] + src[3])), color: Color::WHITE.with_alpha(a), image_mode: NodeImageMode::Stretch, ..default() };
                    s.spawn((img, abs(dst)));
                };
                for x in pieces() {
                    piece(s, x);
                }
                if let Some(wall) = wall {
                    s.spawn((ImageNode::new(wall), abs(HINT)));
                }
                piece(s, WORDS);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_order() {
        let mut seen = [0u8; HINTS];
        // the welcome, then the first list in order, then the second list's unseen ones at random
        assert_eq!(pick(&mut seen, 0), 0);
        assert_eq!(seen[0], 7);
        assert_eq!([1, 2, 3, 4].map(|_| pick(&mut seen, 0)), FIRST);
        let mut rest: Vec<usize> = (0..SECOND.len()).map(|k| pick(&mut seen, (k as u32 * 7) << 16)).collect();
        rest.sort();
        assert_eq!(rest, SECOND);
        // all seen: any of both lists
        assert_eq!(pick(&mut seen, 5 << 16), SECOND[1]);
        // another mode's welcome bit doesn't count
        let mut other = [0u8; HINTS];
        other[0] = 2 | 1;
        assert_eq!(pick(&mut other, 0), 0);
    }
}
