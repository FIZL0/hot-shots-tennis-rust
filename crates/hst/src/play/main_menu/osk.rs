//! B372 / B441: the on-screen keyboard popup over the dimmed page, one for every short text the menus type: the Mods
//! page's search and the Online menu's name. Its field shows the text so far; four rows of ten keys, then Space,
//! Delete, Clear and Done (the name's: Save). A pad's arrows move round it, ✕ presses the key, □ deletes, ○ closes.
//!
//! Mobile keyboard mode (a phone or tablet played by touch, no controller): the popup sits at the top of the screen
//! with only its field and the wide keys, and the device's own keyboard comes up under it (the page's input in the
//! browser, `web::code_entry`; the system keyboard in the iPhone and Android apps). A tap on the field brings the
//! system keyboard back after it was put away; Return is Done.

use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use bevy::prelude::*;

use super::pointer::{Act, OK, press};
use super::{Draw, Effect, GOLD, INK, Menu, Press, Screen, Tex, WHITE};

/// The keys: four rows of ten, then the wide ones.
pub(super) const KEYS: [&str; 4] = ["abcdefghij", "klmnopqrst", "uvwxyz0123", "456789-'.&"];
const WIDE: [&str; 3] = ["Space", "Delete", "Clear"];
pub(super) const SPACE: usize = 40;
pub(super) const DONE: usize = 43;
/// The field (a tap there brings the system keyboard back in mobile keyboard mode).
pub(super) const FIELD: usize = 44;

const PANEL: [f32; 3] = [34.0, 31.0, 38.0];
const TILE: [f32; 3] = [50.0, 46.0, 56.0];
const LIGHT: [f32; 3] = [100.0, 96.0, 104.0];

/// What a press did to the text (the caller's to apply).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Did {
    Nothing,
    /// A letter key's character (lower case: the caller's case).
    Type(char),
    Space,
    Delete,
    Clear,
    Done,
    /// ○: the popup closes.
    Close,
}

/// One press while the popup is up (it takes them all): moves `at` (in mobile keyboard mode only along the wide keys)
/// and plays its sound; returns what it does to the text.
pub(super) fn step(at: &mut usize, k: Press, out: &mut Vec<Effect>) -> Did {
    let mobile = mobile();
    let was = if mobile && *at < SPACE { DONE } else { *at };
    let to = if mobile { wide_move(was, k) } else { osk_move(was, k) };
    if to != *at {
        *at = to;
        out.push(Effect::Sound(0));
    }
    if k.back {
        out.push(Effect::Sound(1));
        return Did::Close;
    }
    if k.page {
        out.push(Effect::Sound(0));
        return Did::Delete;
    }
    if !k.ok {
        return Did::Nothing;
    }
    out.push(Effect::Sound(if was == DONE { 2 } else { 0 }));
    match was {
        40 => Did::Space,
        41 => Did::Delete,
        42 => Did::Clear,
        DONE => Did::Done,
        FIELD => {
            RAISE.store(true, Relaxed);
            *at = DONE;
            Did::Nothing
        }
        k => KEYS[k / 10].chars().nth(k % 10).map_or(Did::Nothing, Did::Type),
    }
}

/// The cursor moved: around each row; up and down between the rows (the wide keys a quarter each).
fn osk_move(at: usize, k: Press) -> usize {
    let (r, c) = if at >= SPACE { (4, (at - SPACE).min(3)) } else { (at / 10, at % 10) };
    match (r, k) {
        (4, k) if k.left => 40 + (c + 3) % 4,
        (4, k) if k.right => 40 + (c + 1) % 4,
        (4, k) if k.up => 30 + (c * 10 + 5) / 4,
        (4, k) if k.down => (c * 10 + 5) / 4,
        (_, k) if k.left => r * 10 + (c + 9) % 10,
        (_, k) if k.right => r * 10 + (c + 1) % 10,
        (0, k) if k.up => 40 + c * 4 / 10,
        (3, k) if k.down => 40 + c * 4 / 10,
        (_, k) if k.up => at - 10,
        (_, k) if k.down => at + 10,
        _ => at,
    }
}

/// Mobile keyboard mode's cursor: round the wide keys only.
fn wide_move(at: usize, k: Press) -> usize {
    let c = at.clamp(SPACE, DONE) - SPACE;
    if k.left {
        40 + (c + 3) % 4
    } else if k.right {
        40 + (c + 1) % 4
    } else {
        40 + c
    }
}

/// A controller other than the touch pad is connected (the native apps; the browser asks the page).
static PAD: AtomicBool = AtomicBool::new(false);
/// The field was tapped: the system keyboard comes back up.
static RAISE: AtomicBool = AtomicBool::new(false);

/// Mobile keyboard mode: played by touch with no controller, so typing is the device's own keyboard's.
pub(super) fn mobile() -> bool {
    #[cfg(target_arch = "wasm32")]
    return crate::web::touched() && !crate::web::real_pad();
    #[cfg(not(target_arch = "wasm32"))]
    return super::super::touch_pad::on() && !PAD.load(Relaxed);
}

/// The popup's words.
#[derive(Clone, Copy)]
pub(super) struct Look<'a> {
    /// Over the field (none: the field alone).
    pub title: Option<&'a str>,
    /// The last wide key's word (Done, Save).
    pub done: &'a str,
    /// The text is as long as it may be: no cursor after it.
    pub full: bool,
}

/// The popup over the dimmed page: the title, the text so far in its field, the keys (in mobile keyboard mode only
/// the wide ones, at the top of the screen, the device's keyboard below), the cursor's gold.
pub(super) fn draw(d: &mut Draw, text: &str, at: usize, hand_x: f32, look: Look) {
    use super::Anchor;
    let mobile = mobile();
    // mobile keyboard mode: the cursor on the wide keys only (`step`)
    let at = if mobile && at < SPACE { DONE } else { at };
    d.popup(Some(press(|k| k.back = true)));
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 70.0);
    d.anchor = Anchor::Centre;
    let (kw, kh, g) = (34.0, 28.0, 4.0);
    let w = 10.0 * kw + 9.0 * g + 24.0;
    let head = if look.title.is_some() { 24.0 } else { 0.0 };
    let rows = if mobile { 1.0 } else { 5.0 };
    // mobile keyboard mode: under the touch pad's top row (L1, Select, Start, R1), above the device's keyboard
    let (x, y) = (320.0 - w / 2.0, if mobile { 58.0 } else { 110.0 - head });
    d.box3([x, y, w, head + 50.0 + rows * (kh + g) + 10.0], 14.0, [Some(INK), Some(GOLD), Some(PANEL)], 128.0);
    if let Some(t) = look.title {
        d.outlined(|d| super::left(d, t, x + 14.0, y + 10.0, 14.0, w - 28.0, WHITE));
    }
    let field = [x + 12.0, y + head + 12.0, w - 24.0, 28.0];
    d.pill_at(field, [INK, if at == FIELD { WHITE } else { GOLD }, [20.0, 18.0, 24.0]], 128.0);
    let cursor = if look.full { "" } else { "_" };
    super::left(d, &format!("{text}{cursor}"), field[0] + 12.0, field[1] + 7.0, 14.0, field[2] - 24.0, WHITE);
    if mobile {
        d.hit(field, Act::Row(FIELD, OK));
    }
    let ky = y + head + 50.0;
    let mut hand = None;
    for (r, row) in KEYS.iter().enumerate().filter(|_| !mobile) {
        for (c, ch) in row.chars().enumerate() {
            let k = r * 10 + c;
            let rect = [x + 12.0 + c as f32 * (kw + g), ky + r as f32 * (kh + g), kw, kh];
            d.box3(rect, 6.0, [Some(INK), Some(if k == at { WHITE } else { LIGHT }), Some(if k == at { GOLD } else { TILE })], 128.0);
            d.centred(&ch.to_uppercase().to_string(), rect[0] + kw / 2.0, rect[1] + 6.0, 16.0, if k == at { INK } else { WHITE });
            d.hit(rect, Act::Row(k, OK));
            if k == at {
                hand = Some(rect);
            }
        }
    }
    let ww = (10.0 * kw + 9.0 * g - 3.0 * g) / 4.0;
    let wy = ky + (rows - 1.0) * (kh + g);
    for (i, word) in WIDE.iter().chain([&look.done]).enumerate() {
        let k = SPACE + i;
        let rect = [x + 12.0 + i as f32 * (ww + g), wy, ww, kh];
        d.box3(rect, 6.0, [Some(INK), Some(if k == at { WHITE } else { LIGHT }), Some(if k == at { GOLD } else { [70.0, 66.0, 76.0] })], 128.0);
        d.label(word, rect[0] + ww / 2.0, rect[1] + 7.0, 13.0, ww - 8.0, if k == at { INK } else { WHITE });
        d.hit(rect, Act::Row(k, OK));
        if k == at {
            hand = Some(rect);
        }
    }
    if let Some(r) = hand {
        d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [r[0] - 18.0 + hand_x, r[1] + r[3] / 2.0 - 9.0, 36.0, 18.0], WHITE, 128.0);
    }
}

/// The legend's words while the popup is up with the cursor on `at`: ✕ there types, or (Done) says `done` (none: it
/// does nothing yet, as Save on a blank name).
pub(super) fn legend(at: usize, done: Option<&'static str>) -> Vec<(super::legend::Act, &'static str)> {
    use super::legend::Act as L;
    let ok = if at == DONE { done } else { Some("Type") };
    [(L::Move, "Move")].into_iter().chain(ok.map(|w| (L::Ok, w))).chain([(L::Page, "Delete"), (L::Back, "Close")]).collect()
}

/// A popup is up: the Mods page's keyboard or the Online menu's name.
fn up(m: &Menu) -> bool {
    m.screen == Screen::Mods && m.add.osk_up() || m.screen == Screen::Code && m.net.naming && !m.net.url
}

/// The native apps' system keyboard: up while a popup is in mobile keyboard mode (iOS and Android show it for an
/// allowed IME); a tap on the field puts it down for a frame and up again. While the chat's typed box is open the
/// chat holds it (`chat::system_keyboard`, which runs in matches too). Also notes a real controller (`mobile`).
fn system_keyboard(menu: Res<Menu>, pads: Query<(&Gamepad, Option<&Name>)>, mut window: Query<&mut Window, With<bevy::window::PrimaryWindow>>) {
    PAD.store(pads.iter().any(|(_, n)| !n.is_some_and(|n| super::super::emote::is_touch(n))), Relaxed);
    crate::chat::TOUCH.store(mobile(), Relaxed);
    let raise = RAISE.swap(false, Relaxed);
    if !cfg!(any(target_os = "ios", target_os = "android")) || crate::chat::OPEN.load(Relaxed) {
        return;
    }
    let want = mobile() && (up(&menu) || menu.screen == Screen::Code);
    let Ok(mut w) = window.single_mut() else { return };
    let want = want && !(raise && w.ime_enabled);
    if w.ime_enabled != want {
        w.ime_enabled = want;
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, system_keyboard.after(super::step));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key and wide key is reached by the arrows, ✕ on a letter types it, the wide keys do their thing.
    #[test]
    fn keys_and_moves() {
        let p = |f: fn(&mut Press)| {
            let mut k = Press::default();
            f(&mut k);
            k
        };
        let mut seen = std::collections::BTreeSet::new();
        let mut at = 0;
        let mut out = vec![];
        for n in 0..44 * 2 {
            seen.insert(at);
            let end = if at < SPACE { at % 10 == 9 } else { at == DONE };
            step(&mut at, p(|k| k.right = true), &mut out);
            if end {
                step(&mut at, p(|k| k.down = true), &mut out);
            }
            assert!(at <= DONE, "{n}");
        }
        assert_eq!(seen, (0..=DONE).collect());
        at = 12;
        assert_eq!(step(&mut at, OK, &mut out), Did::Type('m'));
        at = 41;
        assert_eq!(step(&mut at, OK, &mut out), Did::Delete);
        assert_eq!(step(&mut at, p(|k| k.back = true), &mut out), Did::Close);
        at = DONE;
        out.clear();
        assert_eq!(step(&mut at, OK, &mut out), Did::Done);
        assert_eq!(out, [Effect::Sound(2)]);
    }
}
