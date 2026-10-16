//! M14 (ours, by the user's request): the button legend along the bottom of every menu screen: each action the screen
//! takes there and its button, in our own glyphs (keycaps, Xbox / PlayStation / Steam Deck face buttons, shoulders,
//! d-pad; drawn here with tiny_skia, not the disc's PS2 glyphs) for the device last used, switching live. Where
//! several devices drive a screen (the select, seat assignment) each one's buttons are shown; devices with different
//! actions get a group each, tagged with their player. A touch hides it (no tap hints) until a key or button is used.
//!
//! The screens fill [`Legend`] every frame (the main menus in `main_menu/legend.rs`; the pause menu in `menu.rs`, the result
//! screen in `match_stats.rs`); [`draw`] lays it out on the strip under the description bar (640×448 y 436..448,
//! spanning the window as the bar does) at the 4:3 screen's scale, shrinking the row to fit, never stretched.
//!
//! B362: a pad's style follows the pad (PlayStation by Sony's vendor id, Switch by Nintendo's, Steam Deck by Valve's,
//! else Xbox) unless the Button Icons setting (Settings → Display, `button_icons` in settings.txt) picks one for
//! every pad: PlayStation, Xbox, Switch or Generic (position dots, numbered shoulders). The same art stands in the
//! menus' "Press ✕ to join" hints ([`Buttons`], [`join_styles`]).
//!
//! `HST_LEGEND=keys|xbox|ps|deck|switch|generic` draws every device in that style (for `--shot`s without the pad).
//!
//! P61: the legend's types and its drawing are here, with the match (the key hints, the result screen); the main
//! menus' rows in `main_menu/legend.rs`.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

use super::controls::{self, Action, Bindings};
use super::menu_art::{BOLD, fill_rgb, rounded, stroke_rgb, text_path, unpremultiply};
use super::widescreen::{self, Anchor};

/// What a legend entry does with which button.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Act {
    /// ↑↓←→ / the d-pad.
    Move,
    UpDown,
    LeftRight,
    /// ✕ / Enter.
    Ok,
    /// ○ / Esc.
    Back,
    /// △: the attribute card; on the select the favorite (B375).
    Card,
    /// L1 / R1: the costume; the Settings tab (B294).
    Costume,
    /// □ / Tab: on the select 1P's Singles / Doubles swap (B375).
    Page,
    /// Select: the hand.
    Hand,
    Start,
    /// Start / N: the select's name picker (B375).
    Name,
    /// The keyboard types (Join by Code, the name); words only for a pad.
    Type,
    /// Esc only (the controls screen waiting for a key or button).
    Cancel,
    /// B301: a match key hint, keycaps only (`key_hints.rs`).
    Key,
    /// B320: text chat in an online lobby (`chat.rs`): T, a pad's R2 (phrases) and L2 (say it).
    Chat,
    /// B373: L2 / R2 (keyboard 1 / 3): the Mods pages' kind tab; B382: the character select's sections.
    Kind,
    /// B422: the right stick up / down (keyboard R / F): the select's preview plays its win / loss.
    Preview,
    /// B463: the match's emote keys (1–4) and the right stick left / right: the select's preview plays an emote.
    Emote,
}

/// A device's button art.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Style {
    Keys,
    Xbox,
    Ps,
    Deck,
    /// B362: Nintendo's letters (A right, B bottom, X top, Y left), L / R, ZL / ZR, − / +.
    Switch,
    /// B362: no maker's: the four face buttons as dots with the one meant filled, L1 / R1, L2 / R2, Select / Start.
    Generic,
}

impl Style {
    pub const ALL: [Style; 6] = [Style::Keys, Style::Xbox, Style::Ps, Style::Deck, Style::Switch, Style::Generic];
}

/// B362: the Button Icons setting's values (a [`LAYOUT`]): Auto (each pad's own), then one style for every pad.
pub const LAYOUTS: [(&str, &str, Option<Style>); 5] =
    [("Auto", "auto", None), ("PlayStation", "ps", Some(Style::Ps)), ("Xbox", "xbox", Some(Style::Xbox)), ("Switch", "switch", Some(Style::Switch)), ("Generic", "generic", Some(Style::Generic))];

/// B362: the Button Icons setting, as the main menu last had it (a [`LAYOUTS`] index).
pub static LAYOUT: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, PartialEq, Debug)]
pub struct Item {
    pub act: Act,
    pub label: String,
    /// The keycaps it shows for the keyboard.
    pub keys: Vec<String>,
    /// B462: its first / second glyph (a pair's L2 / R2) drawn faded: that press does nothing here.
    pub dim: [bool; 2],
}

/// B462: a dimmed L2 / R2 hint's alpha (of 128) where the menus draw the art themselves (the Mods page's tab row).
pub const DIM_ALPHA: f32 = 45.0;

/// Devices (None: the keyboard) with the same actions, and their player's tag when groups differ.
#[derive(Clone, PartialEq, Debug)]
pub struct Group {
    pub tag: Option<String>,
    pub devs: Vec<Option<Entity>>,
    pub items: Vec<Item>,
}

/// This frame's legend; `draw` takes it.
#[derive(Resource, Default)]
pub struct Legend(pub Vec<Group>);

/// The device last pressed: the legend's when one device drives a screen.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub enum Last {
    Keys,
    Pad(Entity),
    Touch,
}

impl Last {
    /// The device as a legend's (None: the keyboard; Touch draws nothing anyway).
    pub fn dev(self) -> Option<Entity> {
        if let Last::Pad(e) = self { Some(e) } else { None }
    }
}

impl Legend {
    /// One group for the device last used.
    pub fn one(&mut self, last: Last, items: Vec<Item>) {
        self.0 = vec![Group { tag: None, devs: vec![last.dev()], items }];
    }
}

/// An item with keyboard caps.
pub fn item(act: Act, label: &str, keys: &[&str]) -> Item {
    Item { act, label: label.into(), keys: keys.iter().map(|s| s.to_string()).collect(), dim: [false; 2] }
}

/// The main menus' keys for `act` (`main_menu::step`): the bound ones, and the fixed Enter, Esc, Q/E, Tab.
/// `select`: the character select's (B463: Q / E its sections, Z / X the costume).
pub fn menu_keys(act: Act, bind: &Bindings, select: bool) -> Vec<String> {
    let k = |a: Action| bind.key(a).unwrap_or_default();
    match act {
        Act::Costume if select => vec!["Z".into(), "X".into()],
        Act::Kind if select => vec!["Q".into(), "E".into()],
        Act::Emote => emote_keys(bind),
        Act::Move => vec![k(Action::Up), k(Action::Left), k(Action::Down), k(Action::Right)],
        Act::UpDown => vec![k(Action::Up), k(Action::Down)],
        Act::LeftRight => vec![k(Action::Left), k(Action::Right)],
        Act::Ok => vec!["Enter".into(), k(Action::Normal)],
        Act::Back => vec!["Esc".into(), k(Action::Cut)],
        Act::Card => vec![k(Action::Lob)],
        Act::Costume => vec!["Q".into(), "E".into()],
        Act::Page => vec!["Tab".into()],
        Act::Hand => vec![k(Action::Camera)],
        Act::Start | Act::Cancel => vec!["Esc".into()],
        Act::Name => vec!["N".into()],
        Act::Type => vec!["A-Z".into()],
        Act::Chat => vec!["T".into()],
        Act::Kind => vec!["1".into(), "3".into()],
        Act::Preview => vec!["R".into(), "F".into()],
        Act::Key => vec![],
    }
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect()
}

/// B463: the emote keys' caps: "1-4" for the stock 1, 2, 3, 4, else each bound one.
pub fn emote_keys(bind: &Bindings) -> Vec<String> {
    let keys: Vec<String> = super::emote::KEYS.iter().map(|&a| bind.key(a).unwrap_or_default()).collect();
    if keys == ["1", "2", "3", "4"] { vec!["1-4".into()] } else { keys }
}

// ---------------------------------------------------------------- the last device used

/// `HST_LEGEND`'s style, if it names one.
fn forced() -> Option<Style> {
    match std::env::var("HST_LEGEND").as_deref() {
        Ok("keys") => Some(Style::Keys),
        Ok("xbox") => Some(Style::Xbox),
        Ok("ps") => Some(Style::Ps),
        Ok("deck") => Some(Style::Deck),
        Ok("switch") => Some(Style::Switch),
        Ok("generic") => Some(Style::Generic),
        _ => None,
    }
}

fn style_of(dev: Option<Entity>, pads: &Query<&Gamepad>) -> Style {
    if let Some(s) = forced() {
        return s;
    }
    let Some(e) = dev else { return Style::Keys };
    pad_style(pads.get(e).ok().and_then(|g| g.vendor_id()).unwrap_or(0), LAYOUT.load(Ordering::Relaxed), std::env::var("SteamDeck").as_deref() == Ok("1"))
}

/// A pad's style: the Button Icons setting's (`layout`, a [`LAYOUTS`] index) when it picks one, else by the pad's
/// USB vendor id. SteamOS sets `SteamDeck=1` (`deck`); its built-in pad comes through Steam's Valve-ID'd virtual pad.
fn pad_style(vendor: u16, layout: u8, deck: bool) -> Style {
    if let Some((_, _, Some(s))) = LAYOUTS.get(layout as usize) {
        return *s;
    }
    match vendor {
        _ if deck => Style::Deck,
        0x054c => Style::Ps,
        0x28de => Style::Deck,
        0x057e => Style::Switch,
        _ => Style::Xbox,
    }
}

/// B362: the connected pads' styles (each once, in pad order), as [`note_pads`] last saw them.
static PADS: Mutex<Vec<Style>> = Mutex::new(Vec::new());
/// B368: each connected pad's style, as [`note_pads`] last saw them.
static PAD_STYLES: Mutex<Vec<(Entity, Style)>> = Mutex::new(Vec::new());

/// B368: the button art device `dev` (None: the keyboard) shows, as its legend's.
pub fn dev_style(dev: Option<Entity>) -> Style {
    if let Some(s) = forced() {
        return s;
    }
    let Some(e) = dev else { return Style::Keys };
    let known = PAD_STYLES.lock().ok().and_then(|v| v.iter().find(|(p, _)| *p == e).map(|(_, s)| *s));
    known.unwrap_or_else(|| pad_style(0, LAYOUT.load(Ordering::Relaxed), std::env::var("SteamDeck").as_deref() == Ok("1")))
}

/// B368: the names on `dev`'s buttons for `act` as the legend draws them (a keyboard's keys, Q / E; a pad's L1 / R1
/// or LB / RB, its Select as "Create" on a PlayStation pad): the select's costume and hand pills label theirs.
pub fn button_names(dev: Option<Entity>, act: Act, bind: &Bindings) -> Vec<String> {
    let it = item(act, "", &[]);
    let it = Item { keys: menu_keys(act, bind, true), ..it };
    glyphs(dev_style(dev), &it)
        .into_iter()
        .filter_map(|g| match g {
            Glyph::Cap(s) => Some(s),
            Glyph::Shoulder(s) | Glyph::Small(s) => Some(s.to_string()),
            _ => None,
        })
        .collect()
}

/// B383: the style of the device last used (`fill` keeps it; `HST_LEGEND`'s for `--shot`s): the art the Mods page's
/// kind switch shows beside its tabs.
pub static LAST_STYLE: AtomicU8 = AtomicU8::new(0);

pub fn last_style() -> Style {
    forced().unwrap_or(Style::ALL[LAST_STYLE.load(Ordering::Relaxed) as usize % Style::ALL.len()])
}

/// B383: the Mods page's button art, as the legend draws it in `style`: 0 / 1 the kind switch (L2 / R2, ZL / ZR,
/// LT / RT; the keyboard's 1 / 3), 2 ✕ (Enter), 3 △ and 4 □ (their keys) for the detail panel's buttons; B448: 5 / 6
/// L1 / R1 (Q / E) round the Settings tabs and a stepped pill, 7 Select (C) on the Mods filter pill.
fn page_glyphs(style: Style) -> Vec<Glyph> {
    let it = |a: Act| Item { keys: menu_keys(a, &Bindings::default(), false), ..item(a, "", &[]) };
    let mut v = glyphs(style, &it(Act::Kind));
    v.truncate(2);
    for a in [Act::Ok, Act::Card, Act::Page] {
        v.extend(glyphs(style, &it(a)).into_iter().take(1));
    }
    let mut lr = glyphs(style, &it(Act::Costume));
    lr.truncate(2);
    v.extend(lr);
    v.extend(glyphs(style, &it(Act::Hand)).into_iter().take(1));
    v
}

/// The number of [`page_glyphs`] (`Tex::Glyph(style, i)` for `i` below it).
pub const PAGE_GLYPHS: usize = 8;

/// B383: [`page_glyphs`]' `i`th one's width / height (`Tex::Glyph(style, i)`'s art).
pub fn glyph_aspect(style: Style, i: u8) -> f32 {
    static W: std::sync::OnceLock<Vec<Vec<f32>>> = std::sync::OnceLock::new();
    let w = W.get_or_init(|| Style::ALL.iter().map(|&s| page_glyphs(s).iter().map(|g| draw_glyph(g).width() as f32 / H as f32).collect()).collect());
    w[Style::ALL.iter().position(|s| *s == style).unwrap_or(0)].get(i as usize).copied().unwrap_or(1.0)
}

/// B362: the styles a "press ✕ to join" hint shows: each connected pad's ✕ (`HST_LEGEND`'s pad style, if it names
/// one, for `--shot`s without the pad); B434: not the keyboard's Enter, which joins all the same.
pub fn join_styles() -> Vec<Style> {
    let v = PADS.lock().map(|p| p.clone()).unwrap_or_default();
    match forced().filter(|s| *s != Style::Keys) {
        Some(s) => vec![s],
        None => v,
    }
}

/// B481: the button art of each connected pad not in `seated` (in pad order), for the COM cards' join bars; with
/// `HST_LEGEND` naming a pad style and no pad plugged in, one pad of it (for `--shot`s). Not the keyboard, as
/// [`join_styles`].
pub fn unseated_styles(seated: &[Entity]) -> Vec<Style> {
    let v: Vec<(Entity, Style)> = PAD_STYLES.lock().map(|p| p.clone()).unwrap_or_default();
    let forced = forced().filter(|s| *s != Style::Keys);
    if v.is_empty() {
        return forced.into_iter().collect();
    }
    v.into_iter().filter(|(e, _)| !seated.contains(e)).map(|(_, s)| forced.unwrap_or(s)).collect()
}

/// A phone build (iOS / Android): no keyboard shortcuts anywhere in the menus.
pub const MOBILE: bool = cfg!(any(target_os = "ios", target_os = "android"));

fn note_pads(pads: Query<(Entity, &Gamepad)>, all: Query<&Gamepad>) {
    let mut ps: Vec<_> = pads.iter().filter(|(_, g)| controls::readable(g)).map(|(e, _)| e).collect();
    ps.sort();
    let each: Vec<(Entity, Style)> = ps.iter().map(|&e| (e, style_of(Some(e), &all))).collect();
    if let Ok(mut p) = PAD_STYLES.lock()
        && *p != each
    {
        *p = each;
    }
    let mut v: Vec<Style> = Vec::new();
    for s in ps.into_iter().map(|e| style_of(Some(e), &all)) {
        if !v.contains(&s) {
            v.push(s);
        }
    }
    if let Ok(mut p) = PADS.lock()
        && *p != v
    {
        *p = v;
    }
}

/// B362: each style's ✕ (the keyboard's Enter), for the menus' join hints, and its width / height.
#[derive(Resource, Default)]
pub struct Buttons(pub HashMap<Style, (Handle<Image>, f32)>, pub HashMap<(Style, u8), (Handle<Image>, f32)>);

/// `style`'s ✕ (bottom face button; the keyboard's Enter).
fn ok_glyph(style: Style) -> Glyph {
    if style == Style::Keys { Glyph::Cap("Enter".into()) } else { Glyph::Face(style, 0) }
}

/// [`ok_glyph`]'s width / height.
pub fn ok_aspect(style: Style) -> f32 {
    static W: std::sync::OnceLock<[f32; 6]> = std::sync::OnceLock::new();
    let w = W.get_or_init(|| Style::ALL.map(|s| draw_glyph(&ok_glyph(s)).width() as f32 / H as f32));
    w[Style::ALL.iter().position(|s| *s == style).unwrap_or(0)]
}

fn bake_buttons(mut images: ResMut<Assets<Image>>, mut buttons: ResMut<Buttons>) {
    for s in Style::ALL {
        let p = draw_glyph(&ok_glyph(s));
        let (w, h) = (p.width(), p.height());
        let img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, unpremultiply(p), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
        buttons.0.insert(s, (images.add(img), w as f32 / h as f32));
        // B383: the Mods page's kind switch and detail buttons
        for (i, g) in page_glyphs(s).iter().enumerate() {
            let p = draw_glyph(g);
            let (w, h) = (p.width(), p.height());
            let img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, unpremultiply(p), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
            buttons.1.insert((s, i as u8), (images.add(img), w as f32 / h as f32));
        }
    }
}

/// A key, a readable pad's button or stick, or a touch makes that the last device used (a mouse click doesn't). On a
/// Steam Deck the built-in pad is it until something is pressed.
fn track(keys: Res<ButtonInput<KeyCode>>, pads: Query<(Entity, &Gamepad)>, touches: Res<Touches>, mut last: ResMut<Last>, mut pressed: Local<bool>) {
    let deck = || std::env::var("SteamDeck").as_deref() == Ok("1");
    let now = if touches.any_just_pressed() {
        Some(Last::Touch)
    } else if !MOBILE && keys.get_just_pressed().next().is_some() {
        Some(Last::Keys)
    } else {
        pads.iter()
            .filter(|(_, g)| controls::readable(g))
            .find(|(_, g)| g.get_just_pressed().next().is_some() || g.left_stick().length() > 0.5)
            .map(|(e, _)| Last::Pad(e))
    };
    *pressed |= now.is_some();
    let now = now.or_else(|| (!*pressed && deck()).then(|| pads.iter().find(|(_, g)| controls::readable(g)).map(|(e, _)| Last::Pad(e))).flatten());
    if let Some(n) = now {
        last.set_if_neq(n);
    }
}

// ---------------------------------------------------------------- glyphs

/// One glyph of an entry.
#[derive(Clone, PartialEq, Debug)]
pub enum Glyph {
    Cap(String),
    /// A face button: 0 bottom, 1 right, 2 left, 3 top.
    Face(Style, u8),
    Shoulder(&'static str),
    /// Select / Start.
    Small(&'static str),
    /// B422: the right stick, up / down (B463: `false` left / right).
    Stick(bool),
    /// The d-pad, arms lit: up, down, left, right.
    Dpad([bool; 4]),
    Slash,
}

pub fn glyphs(style: Style, it: &Item) -> Vec<Glyph> {
    if style == Style::Keys || matches!(it.act, Act::Cancel | Act::Key) {
        return it.keys.iter().cloned().map(Glyph::Cap).collect();
    }
    let ps = style == Style::Ps;
    // Switch: L / R, ZL / ZR, − / +; Generic: numbered shoulders, Select / Start
    let (sw, generic) = (style == Style::Switch, style == Style::Generic);
    match it.act {
        Act::Move => vec![Glyph::Dpad([true; 4])],
        Act::UpDown => vec![Glyph::Dpad([true, true, false, false])],
        Act::LeftRight => vec![Glyph::Dpad([false, false, true, true])],
        Act::Ok => vec![Glyph::Face(style, 0)],
        Act::Back => vec![Glyph::Face(style, 1)],
        Act::Page => vec![Glyph::Face(style, 2)],
        Act::Card => vec![Glyph::Face(style, 3)],
        Act::Costume if sw => vec![Glyph::Shoulder("L"), Glyph::Shoulder("R")],
        Act::Costume if ps || generic => vec![Glyph::Shoulder("L1"), Glyph::Shoulder("R1")],
        Act::Costume if style == Style::Deck => vec![Glyph::Shoulder("L1"), Glyph::Shoulder("R1")],
        Act::Costume => vec![Glyph::Shoulder("LB"), Glyph::Shoulder("RB")],
        Act::Chat if sw => vec![Glyph::Shoulder("ZR"), Glyph::Shoulder("ZL")],
        Act::Chat if ps || generic || style == Style::Deck => vec![Glyph::Shoulder("R2"), Glyph::Shoulder("L2")],
        Act::Chat => vec![Glyph::Shoulder("RT"), Glyph::Shoulder("LT")],
        Act::Kind if sw => vec![Glyph::Shoulder("ZL"), Glyph::Shoulder("ZR")],
        Act::Kind if ps || generic || style == Style::Deck => vec![Glyph::Shoulder("L2"), Glyph::Shoulder("R2")],
        Act::Kind => vec![Glyph::Shoulder("LT"), Glyph::Shoulder("RT")],
        Act::Preview => vec![Glyph::Stick(true)],
        Act::Emote => vec![Glyph::Stick(false)],
        Act::Hand => vec![Glyph::Small(if ps { "Create" } else if sw { "-" } else { "Select" })],
        Act::Start | Act::Name => vec![Glyph::Small(if ps { "Options" } else if sw { "+" } else if generic { "Start" } else { "Menu" })],
        Act::Type | Act::Cancel | Act::Key => vec![],
    }
}

/// The art's height in px (the strip is drawn at whatever size the window gives it).
const H: u32 = 64;

const WHITE: [u8; 3] = [240, 240, 244];
const DARK: [u8; 3] = [52, 52, 60];
const RIM: [u8; 3] = [150, 150, 160];

fn font() -> ab_glyph::FontRef<'static> {
    ab_glyph::FontRef::try_from_slice(BOLD).expect("menu font")
}

/// `s`'s width at `px`.
fn text_w(s: &str, px: f32) -> f32 {
    use ab_glyph::Font;
    let f = font();
    let k = px / f.units_per_em().unwrap_or(1000.0);
    s.chars().map(|c| f.h_advance_unscaled(f.glyph_id(c))).sum::<f32>() * k
}

/// `s` at `px` with its capitals' middle at `cy`, starting at `x`.
fn words(out: &mut Pixmap, s: &str, px: f32, x: f32, cy: f32, c: [u8; 3]) {
    use ab_glyph::Font;
    let f = font();
    let k = px / f.units_per_em().unwrap_or(1000.0);
    let (Some(path), Some(caps)) = (text_path(&f, s), text_path(&f, "H")) else { return };
    let base = cy + caps.bounds().height() * k / 2.0;
    fill_rgb(out, &path, c, Transform::from_row(k, 0.0, 0.0, k, x, base));
}

fn pixmap(w: f32) -> Pixmap {
    Pixmap::new(w.ceil().max(1.0) as u32, H).expect("legend art")
}

fn draw_glyph(g: &Glyph) -> Pixmap {
    let cy = H as f32 / 2.0;
    match g {
        Glyph::Cap(s) => {
            let px = 30.0;
            let w = (text_w(s, px) + 20.0).max(44.0);
            let mut out = pixmap(w + 2.0);
            fill_rgb(&mut out, &rounded(1.0, 7.0, w + 1.0, 62.0, 9.0), [110, 110, 120], Transform::identity());
            fill_rgb(&mut out, &rounded(1.0, 2.0, w + 1.0, 57.0, 9.0), [236, 236, 240], Transform::identity());
            words(&mut out, s, px, 1.0 + (w - text_w(s, px)) / 2.0, 29.5, [30, 30, 36]);
            out
        }
        Glyph::Face(style, i) => {
            let mut out = pixmap(52.0);
            let disc = |out: &mut Pixmap, c: [u8; 3]| {
                let p = tiny_skia::PathBuilder::from_circle(26.0, cy, 25.0).expect("circle");
                fill_rgb(out, &p, c, Transform::identity());
                stroke_rgb(out, &tiny_skia::PathBuilder::from_circle(26.0, cy, 24.0).expect("circle"), RIM, 2.0, Transform::identity());
            };
            match style {
                Style::Ps => {
                    disc(&mut out, DARK);
                    let mut pb = tiny_skia::PathBuilder::new();
                    let (x, y) = (26.0, cy);
                    let c = match i {
                        0 => {
                            pb.move_to(x - 9.0, y - 9.0);
                            pb.line_to(x + 9.0, y + 9.0);
                            pb.move_to(x - 9.0, y + 9.0);
                            pb.line_to(x + 9.0, y - 9.0);
                            [124, 178, 255]
                        }
                        1 => {
                            pb.push_circle(x, y, 10.0);
                            [255, 102, 110]
                        }
                        2 => {
                            pb.push_rect(tiny_skia::Rect::from_xywh(x - 9.0, y - 9.0, 18.0, 18.0).expect("square"));
                            [255, 120, 230]
                        }
                        _ => {
                            pb.move_to(x, y - 12.0);
                            pb.line_to(x + 11.0, y + 7.0);
                            pb.line_to(x - 11.0, y + 7.0);
                            pb.close();
                            [64, 226, 170]
                        }
                    };
                    stroke_rgb(&mut out, &pb.finish().expect("shape"), c, 4.0, Transform::identity());
                }
                Style::Generic => {
                    // the four buttons' places (bottom, right, left, top), the meant one filled
                    disc(&mut out, DARK);
                    for (k, (dx, dy)) in [(0.0, 11.0), (11.0, 0.0), (-11.0, 0.0), (0.0, -11.0)].into_iter().enumerate() {
                        let p = tiny_skia::PathBuilder::from_circle(26.0 + dx, cy + dy, 5.5).expect("dot");
                        if k == *i as usize {
                            fill_rgb(&mut out, &p, WHITE, Transform::identity());
                        } else {
                            stroke_rgb(&mut out, &p, RIM, 2.0, Transform::identity());
                        }
                    }
                }
                _ => {
                    let letter = if *style == Style::Switch { ["B", "A", "Y", "X"] } else { ["A", "B", "X", "Y"] }[*i as usize];
                    let (fill, ink) = if matches!(style, Style::Deck | Style::Switch) {
                        (DARK, WHITE)
                    } else {
                        ([[70, 170, 60], [215, 60, 60], [50, 115, 220], [230, 180, 30]][*i as usize], WHITE)
                    };
                    disc(&mut out, fill);
                    words(&mut out, letter, 28.0, 26.0 - text_w(letter, 28.0) / 2.0, cy, ink);
                }
            }
            out
        }
        Glyph::Shoulder(s) => {
            let px = 28.0;
            let w = text_w(s, px) + 22.0;
            let mut out = pixmap(w + 2.0);
            fill_rgb(&mut out, &rounded(1.0, 8.0, w + 1.0, 56.0, 12.0), DARK, Transform::identity());
            stroke_rgb(&mut out, &rounded(2.0, 9.0, w, 55.0, 11.0), RIM, 2.0, Transform::identity());
            words(&mut out, s, px, 1.0 + (w - text_w(s, px)) / 2.0, cy, WHITE);
            out
        }
        Glyph::Small(s) => {
            let px = 25.0;
            let w = text_w(s, px) + 24.0;
            let mut out = pixmap(w + 2.0);
            fill_rgb(&mut out, &rounded(1.0, 12.0, w + 1.0, 52.0, 20.0), DARK, Transform::identity());
            stroke_rgb(&mut out, &rounded(2.0, 13.0, w, 51.0, 19.0), RIM, 2.0, Transform::identity());
            words(&mut out, s, px, 1.0 + (w - text_w(s, px)) / 2.0, cy, WHITE);
            out
        }
        Glyph::Dpad(lit) => {
            let mut out = pixmap(52.0);
            let (x, a, r) = (26.0, 9.5, 25.0);
            // the cross, then each arm lit or not
            let cross = |l: f32, t: f32, rr: f32, b: f32| rounded(l, t, rr, b, 4.0);
            let arms = [(x - a, cy - r, x + a, cy - a), (x - a, cy + a, x + a, cy + r), (x - r, cy - a, x - a, cy + a), (x + a, cy - a, x + r, cy + a)];
            fill_rgb(&mut out, &cross(x - a - 1.5, cy - r - 1.5, x + a + 1.5, cy + r + 1.5), RIM, Transform::identity());
            fill_rgb(&mut out, &cross(x - r - 1.5, cy - a - 1.5, x + r + 1.5, cy + a + 1.5), RIM, Transform::identity());
            fill_rgb(&mut out, &rounded(x - a, cy - a, x + a, cy + a, 1.0), DARK, Transform::identity());
            for (&(l, t, rr, b), &on) in arms.iter().zip(lit) {
                fill_rgb(&mut out, &cross(l, t, rr, b), if on { WHITE } else { DARK }, Transform::identity());
            }
            out
        }
        Glyph::Stick(up) => {
            // a thumbstick's cap with an R on it, arrows above and below (or either side)
            let mut out = pixmap(if *up { 52.0 } else { 76.0 });
            let x = if *up { 26.0 } else { 38.0 };
            let p = tiny_skia::PathBuilder::from_circle(x, cy, 17.0).expect("cap");
            fill_rgb(&mut out, &p, DARK, Transform::identity());
            stroke_rgb(&mut out, &tiny_skia::PathBuilder::from_circle(x, cy, 16.0).expect("cap"), RIM, 2.0, Transform::identity());
            words(&mut out, "R", 20.0, x - text_w("R", 20.0) / 2.0, cy, WHITE);
            for dir in [-1.0, 1.0] {
                let mut pb = tiny_skia::PathBuilder::new();
                if *up {
                    let tip = cy + dir * 31.0;
                    pb.move_to(x, tip);
                    pb.line_to(x + 7.0, tip - dir * 9.0);
                    pb.line_to(x - 7.0, tip - dir * 9.0);
                } else {
                    let tip = x + dir * 36.0;
                    pb.move_to(tip, cy);
                    pb.line_to(tip - dir * 9.0, cy + 7.0);
                    pb.line_to(tip - dir * 9.0, cy - 7.0);
                }
                pb.close();
                fill_rgb(&mut out, &pb.finish().expect("arrow"), WHITE, Transform::identity());
            }
            out
        }
        Glyph::Slash => {
            let mut out = pixmap(text_w("/", 28.0) + 2.0);
            words(&mut out, "/", 28.0, 1.0, cy, RIM);
            out
        }
    }
}

/// One entry's art: each style's glyphs (a slash between styles), then its words.
fn chip(styles: &[Style], it: &Item) -> Option<Pixmap> {
    let mut parts: Vec<Pixmap> = Vec::new();
    for gs in styles.iter().map(|&s| glyphs(s, it)).filter(|g| !g.is_empty()).fold(Vec::<Vec<Glyph>>::new(), |mut v, g| {
        if !v.contains(&g) {
            v.push(g);
        }
        v
    }) {
        if !parts.is_empty() {
            parts.push(draw_glyph(&Glyph::Slash));
        }
        parts.extend(gs.iter().enumerate().map(|(i, g)| {
            let mut p = draw_glyph(g);
            if it.dim.get(i) == Some(&true) {
                fade(&mut p);
            }
            p
        }));
    }
    // a pad can't type: no entry for it
    if parts.is_empty() && it.act == Act::Type && !styles.contains(&Style::Keys) {
        return None;
    }
    Some(row(parts, &it.label, WHITE))
}

/// B462: a glyph's art at [`DIM_ALPHA`]'s share of its opacity (premultiplied: every channel scales).
fn fade(p: &mut Pixmap) {
    let k = DIM_ALPHA / 128.0;
    for b in p.data_mut() {
        *b = (*b as f32 * k).round() as u8;
    }
}

fn row(parts: Vec<Pixmap>, label: &str, c: [u8; 3]) -> Pixmap {
    let px = 40.0;
    let gap = 4.0;
    let tw = text_w(label, px);
    let glyphs_w: f32 = parts.iter().map(|p| p.width() as f32 + gap).sum();
    let mut out = pixmap(glyphs_w + if parts.is_empty() { 0.0 } else { 6.0 } + tw + 2.0);
    let mut x = 0.0;
    for p in &parts {
        out.draw_pixmap(x as i32, 0, p.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        x += p.width() as f32 + gap;
    }
    if !parts.is_empty() {
        x += 6.0;
    }
    words(&mut out, label, px, x, H as f32 / 2.0, c);
    out
}

// ---------------------------------------------------------------- the strip

#[derive(Component)]
pub struct Strip;

/// The art made so far, by its description: (image, width / height).
#[derive(Resource, Default)]
pub struct Chips(HashMap<String, (Handle<Image>, f32)>);

/// The strip's top and height on the 640×448 screen.
const TOP: f32 = 436.0;
const TALL: f32 = 12.0;

fn setup(mut commands: Commands) {
    let node = Node {
        position_type: PositionType::Absolute,
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        overflow: Overflow::clip(),
        ..default()
    };
    commands.spawn((Strip, node, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)), GlobalZIndex(30), Visibility::Hidden));
}

/// Lays the frame's legend out on the strip (rebuilt when it or the window's size changes) and empties it.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    mut commands: Commands,
    mut legend: ResMut<Legend>,
    last: Res<Last>,
    pads: Query<&Gamepad>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut strip: Query<(Entity, &mut Node, &mut Visibility), With<Strip>>,
    mut images: ResMut<Assets<Image>>,
    mut chips: ResMut<Chips>,
    mut shown: Local<String>,
) {
    let mut groups = std::mem::take(&mut legend.0);
    if MOBILE {
        // a phone build shows no keyboard shortcuts: only the pads' groups
        groups.iter_mut().for_each(|g| g.devs.retain(Option::is_some));
        groups.retain(|g| !g.devs.is_empty());
    }
    let (Ok(win), Ok((root, mut node, mut vis))) = (window.single(), strip.single_mut()) else { return };
    let hide = groups.is_empty() || (*last == Last::Touch && std::env::var_os("HST_LEGEND").is_none());
    vis.set_if_neq(if hide { Visibility::Hidden } else { Visibility::Inherited });
    if hide {
        return;
    }
    let vw = widescreen::view_size(win).x;
    let share = widescreen::menu_shares(widescreen::aspect(win), widescreen::menu_scale()).0;
    let (left, width) = widescreen::place(0.0, 640.0, share, Anchor::Span);
    let unit = share * vw / 640.0;
    // each chip: (key, words or the item, styles)
    let mut want: Vec<(String, Pixmap)> = Vec::new();
    // B325: bottom-pinned at the 4:3 screen's scale, as the description bar over it (`ticker_text`), so the two
    // stay stacked at any aspect
    let vshare = widescreen::menu_shares(widescreen::aspect(win), widescreen::menu_scale()).1;
    let (top, height) = widescreen::vplace_in(TOP, TALL, vshare, Anchor::Span);
    let mut key = format!("{left} {width} {top} {height} {unit} {vw}|");
    let mut plan: Vec<(String, bool)> = Vec::new();
    for g in &groups {
        let mut styles: Vec<Style> = g.devs.iter().map(|&d| style_of(d, &pads)).collect();
        styles.dedup();
        if let Some(t) = &g.tag {
            let k = format!("tag {t}");
            plan.push((k.clone(), true));
            if !chips.0.contains_key(&k) {
                want.push((k, row(vec![], t, [255, 214, 90])));
            }
        }
        for it in &g.items {
            let k = format!("{styles:?} {it:?}");
            if !chips.0.contains_key(&k) {
                match chip(&styles, it) {
                    Some(p) => want.push((k.clone(), p)),
                    None => continue,
                }
            }
            plan.push((k, false));
        }
    }
    for (k, p) in want {
        let (w, h) = (p.width(), p.height());
        let img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, unpremultiply(p), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
        chips.0.insert(k, (images.add(img), w as f32 / h as f32));
    }
    plan.iter().for_each(|(k, _)| key.push_str(k));
    if *shown == key {
        return;
    }
    *shown = key;
    (node.left, node.width) = (Val::Percent(left), Val::Percent(width));
    (node.top, node.height) = (Val::Percent(top), Val::Percent(height));
    // 11 of the strip's 12 units tall, 8 apart (a group's tag 14 before it), centred with 8 clear at each end; the
    // row shrinks as one if it's wider than the strip
    let room = width / 100.0 * vw - 16.0 * unit;
    let gap = |i: usize, tag: bool| if i == 0 { 0.0 } else if tag { 14.0 } else { 8.0 };
    let used: f32 = plan.iter().enumerate().map(|(i, (k, tag))| chips.0[k].1 * 11.0 + gap(i, *tag)).sum();
    let k = unit * (room / (used * unit)).min(1.0);
    node.padding = UiRect::horizontal(Val::Px(8.0 * unit));
    commands.entity(root).despawn_related::<Children>();
    commands.entity(root).with_children(|p| {
        for (i, (key, tag)) in plan.iter().enumerate() {
            let (img, aspect) = chips.0[key].clone();
            let h = 11.0 * k;
            let margin = UiRect::left(Val::Px(gap(i, *tag) * k));
            p.spawn((ImageNode::new(img), Node { width: Val::Px(h * aspect), height: Val::Px(h), flex_shrink: 0.0, margin, ..default() }));
        }
    });
}

pub fn plugin(app: &mut App) {
    if app.world().contains_resource::<Legend>() {
        return;
    }
    app.init_resource::<Legend>()
        .init_resource::<Chips>()
        .init_resource::<Buttons>()
        .insert_resource(Last::Keys)
        .add_systems(Startup, (setup, bake_buttons))
        .add_systems(PreUpdate, (track, note_pads).after(bevy::input::InputSystems))
        .add_systems(PostUpdate, draw.before(bevy::ui::UiSystems::Layout));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hand swap's pill names the pad's Select button: "Select" (not Xbox's "View"), "Create" on a PlayStation pad.
    #[test]
    fn hand_hint_says_select() {
        let it = Item { act: Act::Hand, label: "Hand".into(), keys: vec![], dim: [false; 2] };
        for (style, want) in [(Style::Xbox, "Select"), (Style::Deck, "Select"), (Style::Ps, "Create")] {
            assert_eq!(glyphs(style, &it), vec![Glyph::Small(want)], "{style:?}");
        }
    }

    /// B362: a pad's style by its vendor unless Button Icons picks one; Switch and Generic name their own buttons.
    #[test]
    fn button_icons_pick_the_style() {
        assert_eq!([0x054c, 0x057e, 0x28de, 0x045e, 0].map(|v| pad_style(v, 0, false)), [Style::Ps, Style::Switch, Style::Deck, Style::Xbox, Style::Xbox]);
        assert_eq!(pad_style(0x054c, 0, true), Style::Deck, "on a Steam Deck");
        for (k, (_, _, s)) in LAYOUTS.iter().enumerate().skip(1) {
            assert_eq!([0x054c, 0x057e, 0x28de].map(|v| pad_style(v, k as u8, true)), [s.unwrap(); 3], "{k}: every pad");
        }
        let it = |act| Item { act, label: String::new(), keys: vec![], dim: [false; 2] };
        assert_eq!(glyphs(Style::Switch, &it(Act::Costume)), [Glyph::Shoulder("L"), Glyph::Shoulder("R")]);
        assert_eq!(glyphs(Style::Switch, &it(Act::Chat)), [Glyph::Shoulder("ZR"), Glyph::Shoulder("ZL")]);
        assert_eq!(glyphs(Style::Switch, &it(Act::Kind)), [Glyph::Shoulder("ZL"), Glyph::Shoulder("ZR")]);
        assert_eq!(glyphs(Style::Ps, &it(Act::Kind)), [Glyph::Shoulder("L2"), Glyph::Shoulder("R2")]);
        assert_eq!(glyphs(Style::Switch, &it(Act::Hand)), [Glyph::Small("-")]);
        assert_eq!(glyphs(Style::Generic, &it(Act::Start)), [Glyph::Small("Start")]);
        // the font draws Switch's - and +
        use ab_glyph::Font;
        assert!(['-', '+'].iter().all(|&c| font().glyph_id(c).0 != 0));
        // every style's ✕ is drawn, the keyboard's Enter wider than a face button
        assert!(Style::ALL.iter().all(|&s| ok_aspect(s) > 0.5) && ok_aspect(Style::Keys) > ok_aspect(Style::Ps));
    }
}
