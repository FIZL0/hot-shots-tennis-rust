//! M14 (ours, by the user's request): the button legend along the bottom of every menu screen: each action the screen
//! takes there and its button, in our own glyphs (keycaps, Xbox / PlayStation / Steam Deck face buttons, shoulders,
//! d-pad; drawn here with tiny_skia, not the disc's PS2 glyphs) for the device last used, switching live. Where
//! several devices drive a screen (the select, seat assignment) each one's buttons are shown; devices with different
//! actions get a group each, tagged with their player. A touch hides it (no tap hints) until a key or button is used.
//!
//! The screens fill [`Legend`] every frame (the main menus here, `items`; the pause menu in `menu.rs`, the result
//! screen in `match_stats.rs`); [`draw`] lays it out on the strip under the description bar (640×448 y 436..448,
//! spanning the window as the bar does) at the 4:3 screen's scale, shrinking the row to fit, never stretched.
//!
//! `HST_LEGEND=keys|xbox|ps|deck` draws every device in that style (for `--shot`s without the pad).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use std::collections::HashMap;
use tiny_skia::{Pixmap, PixmapPaint, Transform};

use super::super::controls::{self, Action, Bindings};
use super::super::menu_art::{BOLD, fill_rgb, rounded, stroke_rgb, text_path, unpremultiply};
use super::super::widescreen::{self, Anchor};
use super::{CONFIRM_ROWS, Dev, Menu, Screen, options};

/// What a legend entry does with which button.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Act {
    /// ↑↓←→ / the d-pad.
    Move,
    UpDown,
    LeftRight,
    /// ✕ / Enter.
    Ok,
    /// ○ / Esc.
    Back,
    /// △: the attribute card.
    Card,
    /// L1 / R1: the costume; the Settings tab (B294).
    Costume,
    /// □ / Tab: the disc or custom page.
    Page,
    /// Select: the hand.
    Hand,
    Start,
    /// The keyboard types (Join by Code, the name); words only for a pad.
    Type,
    /// Esc only (the controls screen waiting for a key or button).
    Cancel,
    /// B301: a match key hint, keycaps only (`key_hints.rs`).
    Key,
}

/// A device's button art.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Style {
    Keys,
    Xbox,
    Ps,
    Deck,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Item {
    pub act: Act,
    pub label: String,
    /// The keycaps it shows for the keyboard.
    pub keys: Vec<String>,
}

/// Devices (None: the keyboard) with the same actions, and their player's tag when groups differ.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Group {
    pub tag: Option<String>,
    pub devs: Vec<Option<Entity>>,
    pub items: Vec<Item>,
}

/// This frame's legend; `draw` takes it.
#[derive(Resource, Default)]
pub(crate) struct Legend(pub Vec<Group>);

/// The device last pressed: the legend's when one device drives a screen.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub(crate) enum Last {
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
pub(crate) fn item(act: Act, label: &str, keys: &[&str]) -> Item {
    Item { act, label: label.into(), keys: keys.iter().map(|s| s.to_string()).collect() }
}

/// The main menus' keys for `act` (`main_menu::step`): the bound ones, and the fixed Enter, Esc, Q/E, Tab.
fn menu_keys(act: Act, bind: &Bindings) -> Vec<String> {
    let k = |a: Action| bind.key(a).unwrap_or_default();
    match act {
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
        Act::Type => vec!["A-Z".into()],
        Act::Key => vec![],
    }
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect()
}

// ---------------------------------------------------------------- the main menus' entries

/// What `dev`'s presses do on `m`'s screen as it stands (`Menu::step`): the legend lists exactly these (the test
/// probes `step` for them).
pub(super) fn acts(m: &Menu, dev: Dev) -> Vec<(Act, String)> {
    let mut v: Vec<(Act, &str)> = Vec::new();
    let host = m.seats[0] == Some(dev);
    let s = m.sel;
    match m.screen {
        Screen::Main if m.add.ask.is_some() => v.extend([(Act::Move, "Move"), (Act::Ok, "Choose"), (Act::Back, "Later")]),
        Screen::Main => v.extend([(Act::UpDown, "Move"), (Act::Ok, if m.main_rows().get(s) == Some(&super::home::Row::Quit) { "Quit" } else { "Choose" })]),
        Screen::Display | Screen::Graphics | Screen::Sound | Screen::ModSettings | Screen::Mobile => {
            v.push((Act::UpDown, "Option"));
            if let Some(&o) = options::page(m.screen).get(s).filter(|o| !options::locked(**o)) {
                v.push((Act::LeftRight, "Change"));
                if options::ok_changes(o) {
                    v.push((Act::Ok, "Change"));
                }
            }
            v.extend([(Act::Costume, "Page"), (Act::Back, "Back")]);
        }
        Screen::Controls if m.rebinding.is_some() => v.extend([(Act::Type, "Press the new key or button"), (Act::Cancel, "Cancel")]),
        Screen::Controls => {
            v.extend([(Act::UpDown, "Action"), (Act::Ok, "Rebind"), (Act::Costume, "Page"), (Act::Back, "Back")]);
        }
        Screen::Mode if host => v.extend([(Act::Move, "Singles / Doubles"), (Act::Ok, "Choose"), (Act::Back, "Back")]),
        Screen::Assign if host => {
            v.push((Act::UpDown, "Pick"));
            if m.host_swap(false).is_some() || m.host_swap(true).is_some() {
                v.push((Act::LeftRight, "Swap"));
            }
            if m.droppable(m.sel.min(m.players() - 1)) {
                v.push((Act::Page, "Remove"));
            }
            if m.can_bot(m.sel.min(m.players() - 1)) {
                let p = m.sel.min(m.players() - 1);
                v.push((Act::Card, if m.bots[p] { "Remove COM" } else { "Add COM" }));
            }
            v.extend([(Act::Ok, if m.paused { "Done" } else { "Start" }), (Act::Back, "Back")]);
        }
        Screen::Assign => {
            let n = m.players();
            let room = |q: usize| m.seats[q].is_none() && !(m.online() && q >= m.local_max());
            match m.seats[..n].iter().position(|d| *d == Some(dev)) {
                None if (0..if m.online() { m.local_max() } else { n }).any(|p| m.seats[p].is_none() && !m.bots[p]) => v.push((Act::Ok, "Join")),
                None => {}
                Some(p) => {
                    if (1..n).any(|q| q != p && room(q)) {
                        v.push((Act::Move, "Seat"));
                    }
                    v.push((Act::Back, "Leave"));
                }
            }
        }
        Screen::Chars => {
            let Some(p) = m.driven(dev) else { return vec![] };
            let me = m.players[p];
            if !me.ready {
                v.extend([(Act::Move, "Character"), (Act::Costume, "Costume"), (Act::Hand, "Hand")]);
            }
            if !me.ready && !m.blocked(p) {
                v.push((Act::Ok, "Ready"));
            }
            if me.ready {
                v.push((Act::Back, "Cancel"));
            } else if m.online() && m.net.edit == Some(p) {
                v.push((Act::Back, "Back"));
            } else if m.open(p) && !m.online() {
                v.push((Act::Back, "Leave COM"));
            } else if m.seats[p].is_none() && m.players.iter().take(m.players()).any(|q| q.ready) {
                v.push((Act::Back, "Back"));
            } else if m.seats[p] == Some(dev) && m.droppable(p) {
                v.push((Act::Back, "Leave"));
            } else if p == 0 && m.seats[p].is_some() {
                v.push((Act::Back, "Back"));
            }
        }
        Screen::Confirm if host && m.picking.is_some() => v.extend([if s == 1 { (Act::Move, "Court") } else { (Act::LeftRight, ["", "Court", "Sets", "Games", "Umpire"].get(s).copied().unwrap_or("Change")) }, (Act::Ok, "Keep"), (Act::Back, "Cancel")]),
        Screen::Confirm if host => {
            v.push((Act::UpDown, "Move"));
            match s {
                0 => v.push((Act::Ok, "Start")),
                _ if s < CONFIRM_ROWS => v.push((Act::Ok, "Choose")),
                _ => {}
            }
            v.push((Act::Back, "Back"));
        }
        Screen::Online => {
            moves(&mut v, m.pill_axes());
            v.extend([(Act::Ok, "Choose"), (Act::Back, "Back")]);
        }
        Screen::Lobbies => {
            let n = m.net.list.len();
            v.extend([(Act::UpDown, "Move"), (Act::Ok, if s < n { "Join" } else if s == n { "Refresh" } else { "Back" }), (Act::Back, "Back")]);
        }
        Screen::Code => {
            let keys = m.keys().len();
            let ok = match s.checked_sub(keys) {
                None => Some("Type"),
                Some(0) => Some("Delete"),
                Some(1) if m.net.naming => (!m.net.entry.is_empty()).then_some("Clear"),
                Some(1) => Some("Paste"),
                // Save a name that isn't blank, Join a whole short code
                Some(2) if m.net.naming => (!m.net.entry.trim().is_empty()).then_some("Save"),
                Some(2) => crate::net::is_short(&m.net.entry).then_some("Join"),
                _ => Some("Back"),
            };
            // B207: typing on the phone's keyboard: no grid to move on or type from
            let grid = !super::online::touch_typing();
            if grid {
                v.push((Act::Move, "Move"));
            }
            v.extend(ok.map(|ok| (Act::Ok, ok)));
            // B239: □ deletes, ○ leaves
            v.extend([(Act::Type, "Type"), (Act::Page, "Delete"), (Act::Back, "Back")].into_iter().filter(|a| grid || a.0 == Act::Back));
        }
        Screen::Lobby if m.net.ask.is_some() => v.extend([(Act::UpDown, "Move"), (Act::Ok, "Choose"), (Act::Back, "Leave")]),
        Screen::Lobby => {
            moves(&mut v, m.pill_axes());
            if m.lobby_bot() {
                v.extend([(Act::Costume, "Costume"), (Act::Card, "Remove")]);
            }
            if let Some(ok) = m.lobby_ok() {
                v.push((Act::Ok, ok));
            }
            v.push((Act::Back, "Leave"));
        }
        Screen::Mods => v.extend([(Act::UpDown, "Move"), (Act::Ok, "Choose"), (Act::Back, "Back")]),
        _ => {}
    }
    v.into_iter().map(|(a, l)| (a, l.to_string())).collect()
}

/// "Move" on whichever of ↑↓ and ←→ go somewhere.
fn moves(v: &mut Vec<(Act, &str)>, axes: (bool, bool)) {
    match axes {
        (true, true) => v.push((Act::Move, "Move")),
        (true, false) => v.push((Act::UpDown, "Move")),
        (false, true) => v.push((Act::LeftRight, "Move")),
        _ => {}
    }
}

/// The devices driving `m`'s screen and each one's player (None: not seated): the seated humans on the select and
/// seat assignment (and the last-used one there if it isn't seated: it can join), the host on Mode and Confirm, else
/// the last used.
fn drivers(m: &Menu, last: Dev) -> Vec<(Dev, Option<usize>)> {
    let seated = |m: &Menu| (0..m.players()).filter_map(|p| m.seats[p].map(|d| (d, Some(p)))).collect::<Vec<_>>();
    let seat = |d: Dev| (0..m.players()).find(|&p| m.seats[p] == Some(d));
    match m.screen {
        Screen::Chars | Screen::Assign => {
            let mut v = seated(m);
            v.dedup_by_key(|x| x.0);
            if m.screen == Screen::Assign && !v.iter().any(|x| x.0 == last) {
                v.push((last, None));
            }
            v
        }
        Screen::Mode | Screen::Confirm if m.seats[0].is_some() => vec![(m.seats[0].unwrap(), Some(0))],
        _ => vec![(last, seat(last))],
    }
}

/// The main menus' legend: each driving device's entries, devices with the same ones in one group.
pub(super) fn groups(m: &Menu, last: Dev, bind: &Bindings) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    let mut seen = Vec::new();
    for (dev, p) in drivers(m, last) {
        if dev == Dev::Open || seen.contains(&dev) {
            continue;
        }
        seen.push(dev);
        let items: Vec<Item> = acts(m, dev).into_iter().map(|(act, label)| Item { act, keys: menu_keys(act, bind), label }).collect();
        let id = match dev {
            Dev::Keys => None,
            Dev::Pad(e) => Some(e),
            Dev::Open => unreachable!(),
        };
        match out.iter_mut().find(|g| g.items == items) {
            Some(g) => g.devs.push(id),
            None => out.push(Group { tag: p.map(|p| format!("{}P", p + 1)), devs: vec![id], items }),
        }
    }
    out.retain(|g| !g.items.is_empty());
    if out.len() == 1 {
        out[0].tag = None;
    }
    out
}

/// Fills the legend from the main menu (and its Settings screens over a paused match).
pub(super) fn fill(menu: Res<Menu>, last: Res<Last>, bind: Res<Bindings>, mut legend: ResMut<Legend>) {
    let dev = match *last {
        Last::Pad(e) => Dev::Pad(e),
        _ => Dev::Keys,
    };
    legend.0 = groups(&menu, dev, &bind);
}

// ---------------------------------------------------------------- the last device used

fn style_of(dev: Option<Entity>, pads: &Query<&Gamepad>) -> Style {
    match std::env::var("HST_LEGEND").as_deref() {
        Ok("keys") => return Style::Keys,
        Ok("xbox") => return Style::Xbox,
        Ok("ps") => return Style::Ps,
        Ok("deck") => return Style::Deck,
        _ => {}
    }
    let Some(e) = dev else { return Style::Keys };
    // SteamOS sets `SteamDeck=1`; its built-in pad comes through Steam's Valve-ID'd virtual pad
    let vendor = pads.get(e).ok().and_then(|g| g.vendor_id()).unwrap_or(0);
    match vendor {
        _ if std::env::var("SteamDeck").as_deref() == Ok("1") => Style::Deck,
        0x054c => Style::Ps,
        0x28de => Style::Deck,
        _ => Style::Xbox,
    }
}

/// A key, a readable pad's button or stick, or a touch makes that the last device used (a mouse click doesn't). On a
/// Steam Deck the built-in pad is it until something is pressed.
fn track(keys: Res<ButtonInput<KeyCode>>, pads: Query<(Entity, &Gamepad)>, touches: Res<Touches>, mut last: ResMut<Last>, mut pressed: Local<bool>) {
    let deck = || std::env::var("SteamDeck").as_deref() == Ok("1");
    let now = if touches.any_just_pressed() {
        Some(Last::Touch)
    } else if keys.get_just_pressed().next().is_some() {
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
enum Glyph {
    Cap(String),
    /// A face button: 0 bottom, 1 right, 2 left, 3 top.
    Face(Style, u8),
    Shoulder(&'static str),
    /// Select / Start.
    Small(&'static str),
    /// The d-pad, arms lit: up, down, left, right.
    Dpad([bool; 4]),
    Slash,
}

fn glyphs(style: Style, it: &Item) -> Vec<Glyph> {
    if style == Style::Keys || matches!(it.act, Act::Cancel | Act::Key) {
        return it.keys.iter().cloned().map(Glyph::Cap).collect();
    }
    let ps = style == Style::Ps;
    match it.act {
        Act::Move => vec![Glyph::Dpad([true; 4])],
        Act::UpDown => vec![Glyph::Dpad([true, true, false, false])],
        Act::LeftRight => vec![Glyph::Dpad([false, false, true, true])],
        Act::Ok => vec![Glyph::Face(style, 0)],
        Act::Back => vec![Glyph::Face(style, 1)],
        Act::Page => vec![Glyph::Face(style, 2)],
        Act::Card => vec![Glyph::Face(style, 3)],
        Act::Costume if ps => vec![Glyph::Shoulder("L1"), Glyph::Shoulder("R1")],
        Act::Costume if style == Style::Deck => vec![Glyph::Shoulder("L1"), Glyph::Shoulder("R1")],
        Act::Costume => vec![Glyph::Shoulder("LB"), Glyph::Shoulder("RB")],
        Act::Hand => vec![Glyph::Small(if ps { "Create" } else { "View" })],
        Act::Start => vec![Glyph::Small(if ps { "Options" } else { "Menu" })],
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
                _ => {
                    let letter = ["A", "B", "X", "Y"][*i as usize];
                    let (fill, ink) = if *style == Style::Deck {
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
        parts.extend(gs.iter().map(draw_glyph));
    }
    // a pad can't type: no entry for it
    if parts.is_empty() && it.act == Act::Type && !styles.contains(&Style::Keys) {
        return None;
    }
    Some(row(parts, &it.label, WHITE))
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
pub(crate) struct Strip;

/// The art made so far, by its description: (image, width / height).
#[derive(Resource, Default)]
pub(crate) struct Chips(HashMap<String, (Handle<Image>, f32)>);

/// The strip's top and height on the 640×448 screen.
const TOP: f32 = 436.0;
const TALL: f32 = 12.0;

fn setup(mut commands: Commands) {
    let node = Node {
        position_type: PositionType::Absolute,
        top: Val::Percent(TOP / 4.48),
        height: Val::Percent(TALL / 4.48),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::FlexEnd,
        align_items: AlignItems::Center,
        overflow: Overflow::clip(),
        ..default()
    };
    commands.spawn((Strip, node, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)), GlobalZIndex(30), Visibility::Hidden));
}

/// Lays the frame's legend out on the strip (rebuilt when it or the window's size changes) and empties it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw(
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
    let groups = std::mem::take(&mut legend.0);
    let (Ok(win), Ok((root, mut node, mut vis))) = (window.single(), strip.single_mut()) else { return };
    let hide = groups.is_empty() || (*last == Last::Touch && std::env::var_os("HST_LEGEND").is_none());
    vis.set_if_neq(if hide { Visibility::Hidden } else { Visibility::Inherited });
    if hide {
        return;
    }
    let vw = widescreen::view_size(win).x;
    let share = widescreen::share(widescreen::aspect(win));
    let (left, width) = widescreen::place(0.0, 640.0, share, Anchor::Span);
    let unit = share * vw / 640.0;
    // each chip: (key, words or the item, styles)
    let mut want: Vec<(String, Pixmap)> = Vec::new();
    let mut key = format!("{left} {width} {unit} {vw}|");
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
    // 11 of the strip's 12 units tall, 8 apart (a group's tag 14 before it), 8 in from the right; the row shrinks
    // as one if it's wider than the strip
    let room = width / 100.0 * vw - 16.0 * unit;
    let used: f32 = plan.iter().map(|(k, tag)| chips.0[k].1 * 11.0 + if *tag { 14.0 } else { 8.0 }).sum();
    let k = unit * (room / (used * unit)).min(1.0);
    node.padding = UiRect::right(Val::Px(8.0 * unit));
    commands.entity(root).despawn_related::<Children>();
    commands.entity(root).with_children(|p| {
        for (key, tag) in &plan {
            let (img, aspect) = chips.0[key].clone();
            let h = 11.0 * k;
            let margin = UiRect::left(Val::Px(if *tag { 14.0 } else { 8.0 } * k));
            p.spawn((ImageNode::new(img), Node { width: Val::Px(h * aspect), height: Val::Px(h), flex_shrink: 0.0, margin, ..default() }));
        }
    });
}

pub(crate) fn plugin(app: &mut App) {
    if app.world().contains_resource::<Legend>() {
        return;
    }
    app.init_resource::<Legend>()
        .init_resource::<Chips>()
        .insert_resource(Last::Keys)
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, track.after(bevy::input::InputSystems))
        .add_systems(PostUpdate, draw.before(bevy::ui::UiSystems::Layout));
}

#[cfg(test)]
mod tests {
    use super::super::{Effect, Press, online};
    use super::*;

    const DIRS: [fn(&mut Press) -> &mut bool; 4] = [|p| &mut p.up, |p| &mut p.down, |p| &mut p.left, |p| &mut p.right];

    fn press(f: fn(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    /// Whether `dev`'s press does anything on `m`: the menu ends up unlike a step with nothing pressed (which fills
    /// in what a screen sets up lazily), or it asks for more than the refusal sound.
    fn handled(m: &Menu, dev: Dev, f: fn(&mut Press) -> &mut bool) -> bool {
        let (mut idle, mut n) = (m.clone(), m.clone());
        idle.step(dev, Press::default());
        let fx = n.step(dev, press(f));
        format!("{n:?}") != format!("{idle:?}") || fx.iter().any(|e| *e != Effect::Sound(1))
    }

    /// Every screen's legend (each state reachable with the d-pad from a start, on that screen) lists exactly the
    /// presses `Menu::step` takes there: a direction axis, ✕, ○, △, L1/R1, □, Select.
    #[test]
    fn legend_lists_what_each_screen_handles() {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let base = |screen| {
            let mut m = Menu { screen, doubles: true, seats: [Some(Dev::Keys), None, None, None], ..default() };
            m.players[0].cursor = 6;
            m
        };
        let online = |screen| {
            let mut m = base(screen);
            online::sample(&mut m);
            m
        };
        let mut starts: Vec<(Menu, Dev)> = vec![(Menu::default(), Dev::Keys)];
        for s in [Screen::Display, Screen::Graphics, Screen::Sound, Screen::ModSettings, Screen::Controls, Screen::Mode, Screen::Assign, Screen::Chars, Screen::Confirm, Screen::Online, Screen::Mods] {
            starts.push((base(s), Dev::Keys));
        }
        let mut m = base(Screen::Main);
        m.add.ask = Some(0);
        starts.push((m, Dev::Keys));
        let mut m = base(Screen::Controls);
        m.rebinding = Some(0);
        starts.push((m, Dev::Keys));
        // seat assignment: a pad not yet in, and in
        starts.push((base(Screen::Assign), pad));
        let mut m = base(Screen::Assign);
        m.seats[1] = Some(pad);
        starts.push((m.clone(), pad));
        // the select: two humans, one or both ready; the court / umpire pickers
        m.screen = Screen::Chars;
        starts.push((m.clone(), pad));
        m.players[1].ready = true;
        starts.push((m.clone(), pad));
        m.players[0].ready = true;
        starts.push((m, pad));
        for sel in [1, 4] {
            let mut m = base(Screen::Confirm);
            (m.sel, m.picking) = (sel, Some(0));
            starts.push((m, Dev::Keys));
        }
        for s in [Screen::Lobbies, Screen::Lobby, Screen::Code] {
            starts.push((online(s), Dev::Keys));
        }
        let mut m = online(Screen::Lobby);
        m.net.ask = Some(7);
        starts.push((m, Dev::Keys));
        let mut m = online(Screen::Lobby);
        m.net.hosting = false;
        starts.push((m, Dev::Keys));
        let mut m = online(Screen::Code);
        (m.net.naming, m.net.entry) = (true, String::new());
        starts.push((m, Dev::Keys));

        let mut checked = 0;
        for (mut start, last) in starts {
            // as every step leaves it: nobody not ready on a taken costume
            start.skip_taken();
            let screen = start.screen;
            let mut seen = std::collections::HashSet::new();
            let mut todo = vec![start];
            while let Some(m) = todo.pop() {
                if !seen.insert(format!("{m:?}")) || seen.len() > 400 {
                    continue;
                }
                for f in DIRS {
                    let mut n = m.clone();
                    n.step(last, press(f));
                    if n.screen == screen {
                        todo.push(n);
                    }
                }
                for (dev, _) in drivers(&m, last) {
                    let acts = acts(&m, dev);
                    let has = |a: &[Act]| acts.iter().any(|(x, _)| a.contains(x));
                    let h = |f| handled(&m, dev, f);
                    // ✕ there reloads the game, opens a file picker or reaches the store: taken as handled
                    let ok = matches!(m.screen, Screen::Mods) || m.screen == Screen::Main && m.add.ask.is_some() || h(|p| &mut p.ok);
                    let got = [
                        h(|p| &mut p.up) || h(|p| &mut p.down),
                        h(|p| &mut p.left) || h(|p| &mut p.right),
                        ok,
                        h(|p| &mut p.back),
                        h(|p| &mut p.card),
                        h(|p| &mut p.prev) || h(|p| &mut p.next),
                        h(|p| &mut p.page),
                        h(|p| &mut p.hand),
                    ];
                    let want = [
                        has(&[Act::Move, Act::UpDown]),
                        has(&[Act::Move, Act::LeftRight]),
                        has(&[Act::Ok]),
                        has(&[Act::Back]),
                        has(&[Act::Card]),
                        has(&[Act::Costume]),
                        has(&[Act::Page]),
                        has(&[Act::Hand]),
                    ];
                    assert_eq!(got, want, "{:?} sel {} {dev:?}: [↕ ↔ ✕ ○ △ L1R1 □ Select] handled vs legend {acts:?}", m.screen, m.sel);
                    checked += 1;
                }
            }
        }
        assert!(checked > 200, "{checked}");
    }
}
