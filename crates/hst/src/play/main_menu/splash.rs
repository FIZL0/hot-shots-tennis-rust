//! B516: the press-start splash before the main menu (ours). Whichever device presses ✕ / Start (a pad) or Enter /
//! Space (the keyboard), or taps the screen (a phone: the keyboard's seat), is 1P ([`Menu::p1`]): the one device that
//! drives 1P's player from then on (a match's seats come from it, `Menu::step`'s Local / Online). The keyboard still
//! pauses and works the menus whoever has 1P (`Menu::hosts`), but only moves a player from a seat it took itself.
//! It fixes an idle connected pad taking a keyboard player's controls over.
//!
//! Behind it the confirm screen's court pictures (`MENU/WALL/COURTnn.XB`, read with the menus: nothing more to load)
//! fill the window, panning slowly and crossfading from court to court, dimmed and softened (568×256 is soft blown up
//! to a big window), court mods with their own picture among them; the site's logo (`web/logo.svg`) over them.
//! B524: a court that has been played here has a sharp backdrop baked in its match (`hst_play`'s `play/backdrop.rs`,
//! `store::read_backdrop`): it shows that instead, unsoftened (court mods too, with or without their own picture).
//! Only the court up and the next are decoded ([`backdrops`]), and nothing once the splash is gone.

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use std::collections::HashMap;

use super::{COURTS, Dev, Draw, Menu, Screen, Tex, WHITE, court_pic, pointer};
use hst_base::widescreen::Anchor;

/// `web/logo.svg` drawn 640 wide (`rsvg-convert -w 640 web/logo.svg -o crates/hst/assets/logo.png`).
const LOGO_PNG: &[u8] = include_bytes!("../../../assets/logo.png");
const LOGO: [f32; 2] = [640.0, 505.0];
/// Seconds a court is up, and the crossfade into the next at its end.
const SHOW: f32 = 8.0;
const FADE: f32 = 1.6;
/// How far the picture is zoomed in past filling the window: room for its pan.
const ZOOM: f32 = 1.12;
/// The softening: the picture drawn again this many picture pixels off each way, averaged.
const BLUR: f32 = 1.4;
/// The backdrop's brightness (128 = as is).
const DIM: f32 = 52.0;
/// A baked backdrop's (B524): sharp, so a little brighter.
const DIM_BAKED: f32 = 60.0;
/// Seconds the "Player 1" card stays up after the press, before the main menu.
const CLAIMED: f32 = 1.2;

/// The splash's clock and the window's shape (`tick`), for `layout`.
#[derive(Clone, Debug)]
pub(super) struct Splash {
    t: f32,
    /// When 1P was taken (`t` then).
    claimed: Option<f32>,
    aspect: f32,
    /// `HST_MENU=press-claimed` (for `--shot`s): the card stays up.
    held: bool,
    /// B524: the court values with a baked backdrop, its size and whether it's decoded ([`Backdrops`]).
    baked: HashMap<usize, ([f32; 2], bool)>,
}

impl Default for Splash {
    fn default() -> Self {
        Splash { t: 0.0, claimed: None, aspect: 4.0 / 3.0, held: false, baked: HashMap::new() }
    }
}

impl Splash {
    pub(super) fn hold(&mut self) {
        self.held = true;
    }
}

/// The logo's image.
#[derive(Resource)]
pub(super) struct Logo(pub Handle<Image>);

pub(super) fn load(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
    let Ok(i) = Image::from_buffer(LOGO_PNG, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), bevy::asset::RenderAssetUsages::RENDER_WORLD) else { return };
    commands.insert_resource(Logo(images.add(i)));
}

/// The clock; once 1P is taken and its card has been up a moment, on to the main menu.
pub(super) fn tick(mut menu: ResMut<Menu>, time: Res<Time>, window: Query<&Window, With<bevy::window::PrimaryWindow>>) {
    if menu.screen != Screen::Press {
        return;
    }
    let m = &mut *menu;
    m.splash.t += time.delta_secs();
    m.splash.aspect = super::super::widescreen::window_aspect(&window);
    let t = m.splash.t;
    if m.p1.is_some() && m.splash.claimed.is_none() {
        m.splash.claimed = Some(t);
    }
    if m.splash.claimed.is_some_and(|c| t - c >= CLAIMED) && !m.splash.held {
        m.screen = Screen::Main;
        m.sel = 0;
    }
}

impl Menu {
    /// The splash's presses: the first ✕ / Start / Enter / Space (or tap) takes 1P.
    pub(super) fn splash_step(&mut self, dev: Dev, ok: bool) -> bool {
        if !ok || self.p1.is_some() || dev == Dev::Open {
            return false;
        }
        self.p1 = Some(dev);
        true
    }

    /// The courts the splash cycles: the disc's, then the court mods with a picture loaded or a backdrop baked.
    fn splash_courts(&self) -> Vec<usize> {
        (0..COURTS).chain(super::court_mods::values(self).filter(|&c| self.court_pic(c).is_some() || self.splash.baked.contains_key(&c))).collect()
    }

    /// The court shown at splash time `t` (`k`th of `splash_courts`) and the one fading in after it.
    fn splash_court_at(&self, k: usize) -> usize {
        let courts = self.splash_courts();
        courts[k % courts.len()]
    }

    /// B524: court value `c`'s backdrop key (`store::backdrop_key`).
    fn backdrop_key(&self, c: usize) -> Option<String> {
        match self.court_mod_of(c) {
            Some(x) => Some(crate::store::backdrop_key(None, Some(&x.id))),
            None => (c < COURTS).then(|| crate::store::backdrop_key(Some(c), None)),
        }
    }
}

/// The source rect of a `[pw, ph]` picture filling a window of `aspect`, zoomed in by [`ZOOM`] and panned `p` (0..1)
/// across the slack that leaves, left to right (`flip`: right to left), centred the other way.
fn pan([pw, ph]: [f32; 2], aspect: f32, p: f32, flip: bool) -> [f32; 4] {
    let [_, _, w, h] = super::cover([pw, ph], [aspect, 1.0]).map(|v| v / ZOOM);
    let p = if flip { 1.0 - p } else { p };
    [(pw - w) * p, (ph - h) / 2.0, w, h]
}

/// Court value `c`'s picture over the window at weight `w` (1: covers what's under it) and pan `p`: [`BLUR`]'s taps,
/// each at the alpha that leaves them all an equal share of `w` when drawn over one another in turn.
/// B524: a decoded baked backdrop is drawn once, sharp.
fn court(d: &mut Draw, m: &Menu, c: usize, w: f32, p: f32, flip: bool) {
    let blurred = [[0.0, 0.0], [BLUR, 0.0], [-BLUR, 0.0], [0.0, BLUR], [0.0, -BLUR]];
    let (tex, size, taps, dim): (_, _, &[[f32; 2]], _) = match (m.splash.baked.get(&c), m.court_pic(c)) {
        (Some(&(size, true)), _) => (Tex::Backdrop(c), size, &[[0.0, 0.0]], DIM_BAKED),
        (_, Some((i, size))) => (Tex::CourtMod(i), size, &blurred, DIM),
        _ => (court_pic(m.court_layout(c)), [568.0, 256.0], &blurred, DIM),
    };
    let [x, y, sw, sh] = pan(size, m.splash.aspect, p, flip);
    let n = taps.len() as f32;
    for (j, &[dx, dy]) in taps.iter().enumerate() {
        let a = (w / n) / (1.0 - w + (j + 1) as f32 * w / n);
        let src = [(x + dx).clamp(0.0, size[0] - sw), (y + dy).clamp(0.0, size[1] - sh), sw, sh];
        d.q(tex, src, [0.0, 0.0, 640.0, 448.0], [dim; 3], a * 128.0);
    }
}

/// B524: the decoded backdrops (`Tex::Backdrop`) by court value, and the one decoding.
#[derive(Resource, Default)]
pub(super) struct Backdrops {
    pub handles: HashMap<usize, Handle<Image>>,
    /// The court mods' ids the sizes were read for.
    read_for: Option<Vec<String>>,
    decoding: Option<(usize, Task<Option<Image>>)>,
    /// Court values whose PNG wouldn't decode, not tried again.
    broken: Vec<usize>,
}

/// B524: on the splash, which courts have a backdrop (their sizes, from the PNG headers: once, and when the court
/// mods change), and the court up and the next one decoded, one at a time off the frame; any other dropped. Off the
/// splash, every decoded one goes.
pub(super) fn backdrops(mut menu: ResMut<Menu>, mut b: ResMut<Backdrops>, args: Res<crate::Args>, mut images: ResMut<Assets<Image>>) {
    if menu.screen != Screen::Press {
        if !b.handles.is_empty() || b.decoding.is_some() {
            b.handles.clear();
            b.decoding = None;
            menu.splash.baked.values_mut().for_each(|v| v.1 = false);
        }
        return;
    }
    let ids: Vec<String> = menu.court_mods.iter().map(|x| x.id.clone()).collect();
    if b.read_for.as_ref() != Some(&ids) {
        let courts = (0..COURTS).chain(COURTS + 1..COURTS + 1 + ids.len());
        let baked: HashMap<usize, ([f32; 2], bool)> = courts
            .filter_map(|c| {
                let size = crate::store::backdrop_size(&args.iso, &menu.backdrop_key(c)?)?;
                Some((c, (size.map(|v| v as f32), b.handles.contains_key(&c))))
            })
            .collect();
        menu.splash.baked = baked;
        b.read_for = Some(ids);
    }
    let k = (menu.splash.t / SHOW) as usize;
    let want = [menu.splash_court_at(k), menu.splash_court_at(k + 1)];
    b.handles.retain(|c, _| want.contains(c));
    for (c, v) in menu.splash.baked.iter_mut() {
        v.1 = b.handles.contains_key(c);
    }
    if let Some((c, task)) = &mut b.decoding {
        let c = *c;
        if !task.is_finished() {
            return;
        }
        let Some((_, task)) = b.decoding.take() else { return };
        match bevy::tasks::block_on(task) {
            Some(i) if want.contains(&c) => {
                b.handles.insert(c, images.add(i));
                if let Some(v) = menu.splash.baked.get_mut(&c) {
                    v.1 = true;
                }
            }
            Some(_) => {}
            None => b.broken.push(c),
        }
    }
    let Some(&c) = want.iter().find(|c| menu.splash.baked.contains_key(c) && !b.handles.contains_key(c) && !b.broken.contains(c)) else { return };
    let Some(key) = menu.backdrop_key(c) else { return };
    let iso = args.iso.clone();
    b.decoding = Some((c, IoTaskPool::get().spawn(async move { decode(&crate::store::read_backdrop(&iso, &key)?) })));
}

fn decode(png: &[u8]) -> Option<Image> {
    use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
    // the PNG holds the scene's encoded values: sRGB, as the disc pictures are read
    Image::from_buffer(png, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), bevy::asset::RenderAssetUsages::RENDER_WORLD).ok()
}

/// The splash: the courts behind, the logo, and what to press (or who has 1P).
pub(super) fn layout(d: &mut Draw, m: &Menu, pads: &[Entity], controllers: usize) {
    let s = &m.splash;
    let k = (s.t / SHOW) as usize;
    let into = s.t - k as f32 * SHOW;
    let at = |i: usize| m.splash_court_at(i);
    // a court pans over the whole time it shows, its fade in and out included
    let p = |t: f32| (t / (SHOW + FADE)).clamp(0.0, 1.0);
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 128.0);
    court(d, m, at(k), 1.0, p(into + FADE), k % 2 == 1);
    if into > SHOW - FADE {
        let w = (into - (SHOW - FADE)) / FADE;
        court(d, m, at(k + 1), w * w * (3.0 - 2.0 * w), p(into - (SHOW - FADE)), k % 2 == 0);
    }
    d.anchor = Anchor::Centre;
    d.q(Tex::Logo, [0.0, 0.0, LOGO[0], LOGO[1]], [180.0, 34.0, 280.0, 280.0 * LOGO[1] / LOGO[0]], [128.0; 3], 128.0);
    let (line, sub) = match m.p1 {
        Some(Dev::Keys) if super::touch_page() => ("Player 1: Touch screen".to_string(), "Other players join on the character select".to_string()),
        Some(dev) => (format!("Player 1: {}", super::dev_name(Some(dev), pads)), "Other players join on the character select".to_string()),
        None if super::touch_page() => ("Tap to start".to_string(), "or press X / Start on a controller".to_string()),
        None if controllers > 0 => ("Press X / Start or Enter".to_string(), "Whoever presses it plays as Player 1".to_string()),
        None => ("Press Enter".to_string(), "or plug in a controller and press X / Start".to_string()),
    };
    // the prompt breathes until it's answered
    let a = if m.p1.is_some() { 128.0 } else { 128.0 * (0.7 + 0.3 * (s.t * 2.6).cos().abs()) };
    let start = d.out.len();
    d.outlined(|d| d.centred(&line, 320.0, 334.0, 30.0, if m.p1.is_some() { [128.0, 112.0, 30.0] } else { WHITE }));
    d.out[start..].iter_mut().for_each(|q| q.alpha = q.alpha.min(a));
    d.outlined(|d| d.centred(&sub, 320.0, 374.0, 17.0, [100.0; 3]));
    // a click or tap anywhere is the keyboard's press (`pointer::device`: 1P has no device yet)
    d.popup(Some(pointer::press(|k| k.ok = true)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pans_inside_the_picture() {
        for aspect in [0.5, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
            for p in [0.0, 0.5, 1.0] {
                let [x, y, w, h] = pan([568.0, 256.0], aspect, p, false);
                assert!(x >= 0.0 && y >= 0.0 && x + w <= 568.01 && y + h <= 256.01, "{aspect} {p}");
                assert!((w / h - aspect).abs() < 1e-3);
            }
        }
        assert_eq!(pan([568.0, 256.0], 2.0, 0.0, true), pan([568.0, 256.0], 2.0, 1.0, false));
    }

    /// Every device's first press takes 1P, only the first, and the card then hands over to the main menu.
    #[test]
    fn first_press_takes_1p() {
        let pad = Dev::Pad(Entity::from_raw_u32(3).unwrap());
        let mut m = Menu { screen: Screen::Press, ..default() };
        assert!(!m.splash_step(pad, false));
        assert!(m.splash_step(pad, true));
        assert!(!m.splash_step(Dev::Keys, true));
        assert_eq!(m.p1, Some(pad));
    }

    /// The keyboard choosing Local while a pad has 1P: the pad is 1P on the select, the keyboard not seated (it took
    /// over a keyboard player's match from an idle pad the other way round); a 1P pad unplugged hands 1P on.
    #[test]
    fn p1_seats_the_match() {
        let e = Entity::from_raw_u32(3).unwrap();
        let mut m = Menu { screen: Screen::Press, ..default() };
        m.step(Dev::Pad(e), super::super::Press { ok: true, ..default() });
        m.screen = Screen::Main;
        m.sel = m.main_at(super::super::home::Row::Local);
        m.step(Dev::Keys, super::super::Press { ok: true, ..default() });
        assert_eq!((m.screen, m.seats), (Screen::Chars, [Some(Dev::Pad(e)), None, None, None]));
        m.takes_lost_p1(Dev::Keys, &[e]);
        assert_eq!(m.p1, Some(Dev::Pad(e)));
        m.takes_lost_p1(Dev::Keys, &[]);
        assert_eq!(m.p1, Some(Dev::Keys));
    }
}
