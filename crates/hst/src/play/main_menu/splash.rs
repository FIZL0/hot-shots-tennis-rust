//! B516: the press-start splash before the main menu (ours). Whichever device presses ✕ / Start (a pad) or Enter /
//! Space (the keyboard), or taps the screen (a phone: the keyboard's seat), is 1P ([`Menu::p1`]): the one device that
//! drives 1P's player from then on (a match's seats come from it, `Menu::step`'s Local / Online). The keyboard still
//! pauses and works the menus whoever has 1P (`Menu::hosts`), but only moves a player from a seat it took itself.
//! It fixes an idle connected pad taking a keyboard player's controls over.
//!
//! Behind it the confirm screen's court pictures (`MENU/WALL/COURTnn.XB`, read with the menus: nothing more to load)
//! fill the window, still, crossfading slowly from court to court, dimmed and softened (568×256 is soft blown up
//! to a big window), court mods with their own picture among them; the site's logo (`web/logo.svg`) over them.

use bevy::prelude::*;

use super::{COURTS, Dev, Draw, Menu, Screen, Tex, WHITE, court_pic, pointer};
use hst_base::widescreen::Anchor;

/// `web/logo.svg` drawn 640 wide (`rsvg-convert -w 640 web/logo.svg -o crates/hst/assets/logo.png`).
const LOGO_PNG: &[u8] = include_bytes!("../../../assets/logo.png");
const LOGO: [f32; 2] = [640.0, 505.0];
/// Seconds a court is up, and the crossfade into the next at its end.
/// Still, and slow: a moving or quickly changing backdrop is sickening (user 2026-10-16).
const SHOW: f32 = 20.0;
const FADE: f32 = 3.0;
/// The softening: the picture drawn again this many picture pixels off each way, averaged.
const BLUR: f32 = 1.4;
/// The backdrop's brightness (128 = as is).
const DIM: f32 = 52.0;
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
}

impl Default for Splash {
    fn default() -> Self {
        Splash { t: 0.0, claimed: None, aspect: 4.0 / 3.0, held: false }
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

    /// The courts the splash cycles: the disc's, then the court mods with a picture loaded.
    fn splash_courts(&self) -> Vec<usize> {
        (0..COURTS).chain(super::court_mods::values(self).filter(|&c| self.court_pic(c).is_some())).collect()
    }
}

/// The source rect of a `[pw, ph]` picture filling a window of `aspect`, centred.
fn fill([pw, ph]: [f32; 2], aspect: f32) -> [f32; 4] {
    let [_, _, w, h] = super::cover([pw, ph], [aspect, 1.0]);
    [(pw - w) / 2.0, (ph - h) / 2.0, w, h]
}

/// Court value `c`'s picture over the window at weight `w` (1: covers what's under it): [`BLUR`]'s taps,
/// each at the alpha that leaves them all an equal share of `w` when drawn over one another in turn.
fn court(d: &mut Draw, m: &Menu, c: usize, w: f32) {
    let (tex, size) = match m.court_pic(c) {
        Some((i, size)) => (Tex::CourtMod(i), size),
        None => (court_pic(c), [568.0, 256.0]),
    };
    let [x, y, sw, sh] = fill(size, m.splash.aspect);
    let taps = [[0.0, 0.0], [BLUR, 0.0], [-BLUR, 0.0], [0.0, BLUR], [0.0, -BLUR]];
    let n = taps.len() as f32;
    for (j, [dx, dy]) in taps.into_iter().enumerate() {
        let a = (w / n) / (1.0 - w + (j + 1) as f32 * w / n);
        let src = [(x + dx).clamp(0.0, size[0] - sw), (y + dy).clamp(0.0, size[1] - sh), sw, sh];
        d.q(tex, src, [0.0, 0.0, 640.0, 448.0], [DIM; 3], a * 128.0);
    }
}

/// The splash: the courts behind, the logo, and what to press (or who has 1P).
pub(super) fn layout(d: &mut Draw, m: &Menu, pads: &[Entity], controllers: usize) {
    let s = &m.splash;
    let courts = m.splash_courts();
    let k = (s.t / SHOW) as usize;
    let into = s.t - k as f32 * SHOW;
    let at = |i: usize| courts[i % courts.len()];
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 128.0);
    court(d, m, at(k), 1.0);
    if into > SHOW - FADE {
        let w = (into - (SHOW - FADE)) / FADE;
        court(d, m, at(k + 1), w * w * (3.0 - 2.0 * w));
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
    fn fills_inside_the_picture() {
        for aspect in [0.5, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
            let [x, y, w, h] = fill([568.0, 256.0], aspect);
            assert!(x >= 0.0 && y >= 0.0 && x + w <= 568.01 && y + h <= 256.01, "{aspect}");
            assert!((w / h - aspect).abs() < 1e-3);
        }
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
