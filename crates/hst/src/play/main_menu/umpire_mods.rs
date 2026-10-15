//! M8f3: umpire mods on the confirm screen's umpire select, after the disc five (`crate::mods_umpire`). A pick's
//! value past Random (`UMPIRES`) is mod `value - UMPIRES - 1` of `Menu::umpire_mods`; the match gets its donor
//! (`--umpire`, the timing) and the mod (`--umpire-mod DIR`), an online room the donor and the mod's id.
//!
//! B400: a mod's own picture (`previews/umpire.png`, the mods repo's standard §3) stands in for the disc's: its head
//! on the strip's card and the confirm screen's round face, the whole picture on the select's stage. Without one a
//! mod keeps its plain named card and its donor's face and still.

use std::path::PathBuf;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::{Draw, Menu, Tex, Text, UMPIRES, WHITE};
use crate::mods_umpire::Umpire;

/// B400: an umpire mod's picture: its size and the square round its head, in its pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Pic {
    pub size: [f32; 2],
    pub face: [f32; 4],
}

/// B400: the umpire mods' pictures (`Tex::Umpire`), by `Menu::umpire_mods` index, and the folders they were read for.
#[derive(Resource, Default)]
pub(super) struct Pics {
    dirs: Vec<PathBuf>,
    pub handles: Vec<Option<Handle<Image>>>,
}

/// Reads the umpire mods' pictures whenever the installed list isn't the one they were read for.
pub(super) fn load(mut menu: ResMut<Menu>, mut pics: ResMut<Pics>, mut images: ResMut<Assets<Image>>) {
    let dirs: Vec<PathBuf> = menu.umpire_mods.iter().map(|x| x.dir.clone()).collect();
    if pics.dirs == dirs && menu.umpire_pics.len() == dirs.len() {
        return;
    }
    let read: Vec<Option<(Pic, Image)>> = dirs.iter().map(|d| crate::modfs::read(&d.join("previews/umpire.png")).ok().and_then(|b| decode(&b))).collect();
    menu.umpire_pics = read.iter().map(|r| r.as_ref().map(|r| r.0)).collect();
    pics.handles = read.into_iter().map(|r| r.map(|r| images.add(r.1))).collect();
    pics.dirs = dirs;
}

/// A picture's PNG: its [`Pic`] and image.
fn decode(png: &[u8]) -> Option<(Pic, Image)> {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?.into_rgba8();
    let face = crate::mods_umpire::head(&img)?;
    let (w, h) = img.dimensions();
    let pic = Pic { size: [w as f32, h as f32], face };
    Some((pic, Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, img.into_raw(), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)))
}

/// B400: umpire mod `v`'s card on the strip in `r` when it has a picture: its head across the card (cut to the card's
/// shape), the name on its foot as Random's. False: no picture (the plain named card).
pub(super) fn card(d: &mut Draw, m: &Menu, v: usize, r: [f32; 4], on: bool, text: &Text) -> bool {
    let Some((i, pic)) = picture(m, v) else { return false };
    let [fx, fy, fs, _] = pic.face;
    // the head's square widened to the card's shape about its middle, its top kept
    let w = fs * r[2] / r[3];
    d.q(Tex::Solid, [0.0; 4], r, if on { [70.0, 66.0, 86.0] } else { [44.0, 40.0, 54.0] }, 128.0);
    d.q(Tex::Umpire(i), [fx + fs / 2.0 - w / 2.0, fy, w, fs], r, if on { WHITE } else { [100.0; 3] }, 128.0);
    let name = m.umpire_name(text, v);
    let rgb = if on { super::GOLD } else { WHITE };
    d.outlined(|d| d.label(&name, r[0] + r[2] / 2.0, r[1] + r[3] - 14.0, 12.0, r[2] - 4.0, rgb));
    true
}

/// B400: umpire mod `v`'s whole picture on the select's stage `[x, y, w, h]`, as big as fits. False: no picture.
pub(super) fn stage(d: &mut Draw, m: &Menu, v: usize, [x, y, w, h]: [f32; 4]) -> bool {
    let Some((i, pic)) = picture(m, v) else { return false };
    let [sw, sh] = pic.size;
    let ph = (h - 16.0).min((w - 16.0) * sh / sw);
    let pw = ph * sw / sh;
    d.q(Tex::Umpire(i), [0.0, 0.0, sw, sh], [x + (w - pw) / 2.0, y + (h - ph) / 2.0, pw, ph], WHITE, 128.0);
    true
}

/// B400: umpire mod `v`'s round face of `size` centred on `[x, y]` (the confirm card's): the disc faces' dark rim and
/// white ring, its head clipped round inside. False: no picture.
pub(super) fn face(d: &mut Draw, m: &Menu, v: usize, [x, y]: [f32; 2], size: f32) -> bool {
    let Some((i, pic)) = picture(m, v) else { return false };
    for (inset, rgb) in [(0.0, [36.0, 35.0, 34.0]), (0.7, WHITE)] {
        let r = size / 2.0 - inset;
        for (src, dst) in super::super::menu_art::pill_quads(0, [x - r, y - r, 2.0 * r, 2.0 * r]) {
            d.q(Tex::Pill, src, dst, rgb, 128.0);
        }
    }
    let r = size / 2.0 - 2.2;
    d.q(Tex::Solid, [0.0; 4], [x - r, y - r, 2.0 * r, 2.0 * r], [70.0, 66.0, 86.0], 128.0);
    d.out.last_mut().unwrap().round = [r; 4];
    d.q(Tex::Umpire(i), pic.face, [x - r, y - r, 2.0 * r, 2.0 * r], WHITE, 128.0);
    d.out.last_mut().unwrap().round = [r; 4];
    true
}

/// M8f9/B400: installed umpire mod `dir`'s head (`Tex::Umpire` and its square), for the Mods pages' cards and
/// tiles; None when it isn't an umpire mod the menu read at startup, or ships no picture.
pub(super) fn face_of(m: &Menu, dir: &std::path::Path) -> Option<(Tex, [f32; 4])> {
    // the menu's dirs keep `./mods`'s `./`, the Mods pages' drop it
    let bare = |p: &std::path::Path| p.strip_prefix(".").unwrap_or(p).to_path_buf();
    let i = m.umpire_mods.iter().position(|x| bare(&x.dir) == bare(dir))?;
    Some((Tex::Umpire(i), (*m.umpire_pics.get(i)?)?.face))
}

/// Umpire value `v`'s mod index and picture, if it's a mod with one.
fn picture(m: &Menu, v: usize) -> Option<(usize, Pic)> {
    let i = v.checked_sub(UMPIRES + 1)?;
    Some((i, (*m.umpire_pics.get(i)?)?))
}

/// The umpire mods' values on the select's strip, after the disc five.
pub(super) fn values(m: &Menu) -> impl Iterator<Item = usize> {
    UMPIRES + 1..UMPIRES + 1 + m.umpire_mods.len()
}

impl Menu {
    /// The umpire mod that umpire value `u` picks, if it's one.
    pub(super) fn umpire_mod_of(&self, u: usize) -> Option<&Umpire> {
        self.umpire_mods.get(u.checked_sub(UMPIRES + 1)?)
    }

    /// The disc umpire calling as `u` (a mod's donor; Random stays Random).
    pub(super) fn umpire_donor(&self, u: usize) -> usize {
        self.umpire_mod_of(u).map_or(u, |x| x.donor as usize)
    }

    /// Umpire `u`'s name: the disc's (`message0` 669 + 2u, "Umpire " dropped), a mod's own, or Random.
    pub(super) fn umpire_name(&self, text: &Text, u: usize) -> String {
        match self.umpire_mod_of(u) {
            Some(x) => x.name.clone(),
            None if u < UMPIRES => text.msg(669 + 2 * u).trim_start_matches("Umpire ").to_string(),
            None => "Random".into(),
        }
    }

    /// The picked umpire's mod flags for the match (`--umpire-mod DIR`), if it's a mod.
    pub(super) fn umpire_mod_flags(&self) -> Vec<String> {
        self.umpire_mod_of(self.umpire as usize).map(|x| vec!["--umpire-mod".to_string(), x.dir.display().to_string()]).unwrap_or_default()
    }

    /// A pick past the installed mods (one taken away since): Random.
    pub(super) fn umpire_in_range(&mut self) {
        if self.umpire as usize > UMPIRES + self.umpire_mods.len() {
            self.umpire = UMPIRES as u8;
        }
    }
}

/// Preview Voice's bank for umpire `u`: the disc's, or a mod's calls over its donor's.
pub(super) fn preview_bank(iso: &str, m: &Menu, u: u8) -> Option<crate::audio::SoundBank> {
    let mut iso = hst_data::iso::Iso::open(iso).ok()?;
    let disc = crate::audio::umpire_bank(&mut iso, m.umpire_donor(u as usize) as u8, 0)?;
    Some(match m.umpire_mod_of(u as usize) {
        Some(x) => crate::mods_umpire::bank(&disc, x),
        None => disc,
    })
}

#[cfg(test)]
mod tests {
    use super::super::*;

    fn press(f: impl FnOnce(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    /// M8f3: the umpire select's strip runs Random, the disc five, then the umpire mods; → past Anna picks the
    /// first mod, which shows by its name, plays as its donor (`--umpire`) with `--umpire-mod DIR`, isn't saved as
    /// the next run's pick, and goes back to Random when it's no longer installed.
    #[test]
    fn umpire_mods_follow_the_disc_five() {
        let ump = |n: u8, donor| crate::mods_umpire::Umpire { dir: format!("mods/gag_ump{n:02}").into(), id: format!("gag_ump{n:02}"), name: format!("Umpire {n:02}"), source: "Hot Shots Tennis: Get a Grip".into(), donor, voice: "voice/".into() };
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], court: 0, umpire: 4, umpire_mods: vec![ump(0, 2), ump(1, 0)], ..default() };
        m.sel = 4;
        assert_eq!(strip_order(&m), [UMPIRES, 0, 1, 2, 3, 4, UMPIRES + 1, UMPIRES + 2]);
        m.step(Dev::Keys, press(|p| &mut p.ok));
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.umpire as usize, UMPIRES + 1);
        // △: Preview Voice on a mod too
        assert!(m.step(Dev::Keys, press(|p| &mut p.card)).iter().any(|e| matches!(e, Effect::Hear(6, ..))));
        let saved = m.settings.6;
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!((m.umpire as usize, m.settings.6), (UMPIRES + 1, saved), "kept, not saved");
        let t = Text { msg: Vec::new(), glyphs: default(), grades: [[0; 5]; 14], style: [0; 14], hands: [1.0; 14], ranks: Vec::new(), mods: Vec::new() };
        assert_eq!((m.umpire_name(&t, UMPIRES + 2), m.umpire_name(&t, UMPIRES), m.umpire_donor(UMPIRES + 1)), ("Umpire 01".into(), "Random".into(), 2));
        let a = m.args(&[]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        assert_eq!((flag("--umpire"), flag("--umpire-mod")), ("2".into(), "mods/gag_ump00".into()));
        // a disc pick: no mod flag
        m.umpire = 3;
        assert!(!m.args(&[]).contains(&"--umpire-mod".to_string()));
        m.umpire = UMPIRES as u8 + 2;
        m.umpire_mods.pop();
        m.umpire_in_range();
        assert_eq!(m.umpire as usize, UMPIRES);
    }

    /// B400: a mod's `previews/umpire.png` gives its head's square (0.36 of the figure's height, centred on its top
    /// fifth); only mod values with a picture draw one, a blank picture is none.
    #[test]
    fn umpire_mod_pictures_find_the_head() {
        // a 100-tall figure from y 20: a 20-wide head at x 40..60 on a 60-wide body at x 10..70
        let mut img = image::RgbaImage::new(80, 130);
        for (x, y, p) in img.enumerate_pixels_mut() {
            if (20..40).contains(&y) && (40..60).contains(&x) || (40..120).contains(&y) && (10..70).contains(&x) {
                *p = image::Rgba([200, 100, 50, 255]);
            }
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let (pic, _) = super::decode(&png).unwrap();
        let s = 0.36 * 100.0;
        assert_eq!(pic, super::Pic { size: [80.0, 130.0], face: [50.0 - s / 2.0, 20.0 - 0.05 * s, s, s] });
        let mut blank = Vec::new();
        image::RgbaImage::new(8, 8).write_to(&mut std::io::Cursor::new(&mut blank), image::ImageFormat::Png).unwrap();
        assert!(super::decode(&blank).is_none());
        let m = Menu { umpire_pics: vec![None, Some(pic)], ..default() };
        assert_eq!(super::picture(&m, UMPIRES + 2), Some((1, pic)));
        for v in [0, UMPIRES, UMPIRES + 1, UMPIRES + 3] {
            assert_eq!(super::picture(&m, v), None);
        }
    }
}
