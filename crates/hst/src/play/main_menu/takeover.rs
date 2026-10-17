//! M127: the pad takeover prompt. With 1P on the keyboard (and mouse), a button on a pad no seat holds pops
//! "Controller detected / Use it for 1P?" over the menu instead of going nowhere: only that pad answers it (the
//! keyboard's Esc dismisses it, a tap or click picks a pill), and the press that opened it does nothing else. Yes
//! makes the pad 1P (its seat and [`Menu::p1`]: the legend's hints follow it) and frees the keyboard to join as
//! another player, with a short toast saying so; No, Esc or [`WAIT`] seconds untouched keep the keyboard and that
//! pad isn't asked again until it's unplugged. Not on the splash (its first press takes 1P), the screens a press
//! joins a player on (the online lobby among them; the online menus before it do ask), over a paused match (its
//! controller assignment does it there), nor on a touch page (a pad's press takes the screen's 1P there, B247).

use bevy::prelude::*;

use super::{Dev, Draw, Effect, Menu, Press, Screen, Tex, INK, WHITE, frame, pointer};
use hst_base::widescreen::Anchor;

/// Seconds the prompt waits before it keeps the keyboard.
const WAIT: f32 = 4.0;
/// Seconds the toast stays (the last `FADE` of them fading).
const TOAST: f32 = 2.5;
const FADE: f32 = 0.5;

/// The prompt's state, kept in [`Menu`].
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Takeover {
    /// Up: the asking pad, its name, the highlighted pill (0 Yes) and the seconds it has been up (0 on the frame it
    /// opened: that frame's presses are the opening one's).
    ask: Option<Ask>,
    /// The pads that said No: not asked again while they stay plugged in.
    declined: Vec<Entity>,
    /// The toast after Yes, and its seconds left.
    toast: Option<(String, f32)>,
    /// `HST_MENU=takeover` / `takeover-toast` (for `--shot`s, which read no pad): held up, no clock.
    held: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct Ask {
    pad: Entity,
    name: String,
    row: usize,
    age: f32,
}

impl Menu {
    /// Whether a pad's button may ask for 1P here: 1P on the keyboard, a local menu with nothing else popped up.
    fn takeover_here(&self) -> bool {
        let screens = matches!(
            self.screen,
            Screen::Main | Screen::Display | Screen::Graphics | Screen::Advanced | Screen::Sound | Screen::Controls | Screen::ModSettings | Screen::Mobile | Screen::Mode | Screen::Mods | Screen::Extras | Screen::Online | Screen::Lobbies | Screen::Code
        );
        screens
            && self.p1 == Some(Dev::Keys)
            && matches!(self.seats[0], None | Some(Dev::Keys))
            && !self.paused
            && self.net_pause.is_none()
            && self.rebinding.is_none()
            && self.switch.is_none()
            && self.stale.0.is_none()
            && self.listing.is_none()
            && self.add.ask.is_none()
    }

    /// Pad `pad` (named `name`) pressed a button: the prompt opens if it may ask. Returns whether it did.
    fn takeover_open(&mut self, pad: Entity, name: String) -> bool {
        let t = &self.takeover;
        if t.ask.is_some() || t.declined.contains(&pad) || self.seats.contains(&Some(Dev::Pad(pad))) || !self.takeover_here() {
            return false;
        }
        self.takeover.ask = Some(Ask { pad, name, row: 0, age: 0.0 });
        true
    }

    /// Whether the prompt is up.
    pub(super) fn takeover_asking(&self) -> bool {
        self.takeover.ask.is_some()
    }

    /// Whether `dev` is the pad being asked.
    pub(super) fn takeover_asked(&self, dev: Dev) -> bool {
        matches!((dev, &self.takeover.ask), (Dev::Pad(e), Some(a)) if a.pad == e)
    }

    /// One device's press while the prompt is up (it takes every one): the asked pad's ↑ ↓ ← → move between Yes and
    /// No, ✕ chooses, ○ is No; the keyboard's Back (Esc) is No; anything else does nothing. `None`: no prompt up.
    pub(super) fn takeover_step(&mut self, dev: Dev, k: Press) -> Option<Vec<Effect>> {
        let a = self.takeover.ask.as_mut()?;
        let mut out = Vec::new();
        if a.age == 0.0 {
            return Some(out);
        }
        if self.takeover_asked(dev) {
            let a = self.takeover.ask.as_mut().unwrap();
            if k.up || k.down || k.left || k.right {
                a.row = 1 - a.row;
                out.push(Effect::Sound(0));
            } else if k.ok {
                let yes = a.row == 0;
                self.takeover_answer(yes, &mut out);
            } else if k.back {
                self.takeover_answer(false, &mut out);
            }
        } else if dev == Dev::Keys && k.back {
            self.takeover_answer(false, &mut out);
        }
        Some(out)
    }

    /// A pointer act while the prompt is up: hovering a pill highlights it, a tap or click on it chooses it, a tap
    /// off the popup is No (`Draw::popup`'s outside act). `None`: no prompt up.
    pub(super) fn takeover_point(&mut self, act: pointer::Act, tap: bool) -> Option<Vec<Effect>> {
        let a = self.takeover.ask.as_mut()?;
        let mut out = Vec::new();
        if a.age == 0.0 {
            return Some(out);
        }
        if let pointer::Act::Row(i, _) = act {
            if a.row != i {
                a.row = i;
                out.push(Effect::Sound(0));
            }
            if tap {
                self.takeover_answer(i == 0, &mut out);
            }
        }
        Some(out)
    }

    /// The answer: Yes gives 1P to the pad and frees the keyboard; No keeps it and the pad isn't asked again.
    fn takeover_answer(&mut self, yes: bool, out: &mut Vec<Effect>) {
        let Some(a) = self.takeover.ask.take() else { return };
        if yes {
            let pad = Dev::Pad(a.pad);
            self.p1 = Some(pad);
            if self.seats[0] == Some(Dev::Keys) {
                self.seats[0] = Some(pad);
            }
            self.takeover.toast = Some((format!("1P: {} · keyboard is free", a.name), TOAST));
            out.push(Effect::Sound(2));
        } else {
            self.takeover.declined.push(a.pad);
            out.push(Effect::Sound(1));
        }
    }

    /// The clock: the prompt's wait (No once it runs out, or gone with its pad), the toast's, and the pads that said
    /// No forgotten once unplugged (`pads`: the connected ones).
    fn takeover_tick(&mut self, dt: f32, pads: &[Entity]) {
        if self.takeover.held {
            return;
        }
        let t = &mut self.takeover;
        t.declined.retain(|e| pads.contains(e));
        if let Some((_, left)) = &mut t.toast {
            *left -= dt;
            if *left <= 0.0 {
                t.toast = None;
            }
        }
        // a screen change under it (the match starting from elsewhere, a lobby link) takes it down unanswered
        if t.ask.as_ref().is_some_and(|a| !pads.contains(&a.pad)) || t.ask.is_some() && !self.takeover_here() {
            self.takeover.ask = None;
            return;
        }
        let Some(a) = &mut self.takeover.ask else { return };
        a.age += dt.max(f32::MIN_POSITIVE);
        if a.age >= WAIT {
            let mut out = Vec::new();
            self.takeover_answer(false, &mut out);
        }
    }
}

/// `HST_MENU=takeover`: the prompt up on the main menu, half its wait gone, for a pad named "Xbox Wireless
/// Controller"; `takeover-toast`: its Yes's toast.
pub(super) fn sample(m: &mut Menu, toast: bool) {
    let name = "Xbox Wireless Controller".to_string();
    m.p1 = Some(Dev::Keys);
    m.takeover.held = true;
    if toast {
        m.takeover.toast = Some((format!("1P: {name} · keyboard is free"), TOAST));
    } else {
        m.takeover.ask = Some(Ask { pad: Entity::PLACEHOLDER, name, row: 0, age: WAIT / 2.0 });
    }
}

/// Runs before the menu's `step`: the clock, then a button (any, d-pad included; not the sticks, which drift) on an
/// unseated pad opens the prompt. The press is then swallowed by [`Menu::takeover_step`] (the prompt's first frame).
pub(super) fn watch(mut menu: ResMut<Menu>, gamepads: Query<(Entity, &Gamepad, Option<&Name>)>, time: Res<Time>) {
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g, _)| super::controls::readable(g)).map(|(e, _, _)| e).collect();
    pads.sort();
    let busy = menu.takeover != Takeover::default();
    if busy {
        menu.takeover_tick(time.delta_secs(), &pads);
    }
    if menu.takeover.ask.is_some() || super::touch_page() {
        return;
    }
    for (i, &e) in pads.iter().enumerate() {
        let Ok((_, g, name)) = gamepads.get(e) else { continue };
        if g.get_just_pressed().next().is_none() {
            continue;
        }
        let name = name.map(|n| n.as_str().trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("Controller {}", i + 1));
        if menu.bypass_change_detection().takeover_open(e, name) {
            menu.set_changed();
            return;
        }
    }
}

/// The prompt's popup: a dimmed menu, the question, Yes and No (the hand on the highlighted one), and the seconds
/// left as a bar running down; pinned to the centre like the other popups, so it keeps its shape at any aspect.
pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    toast(d, m);
    let Some(a) = &m.takeover.ask else { return };
    use super::super::menu::{CHOSEN, PANEL, UNCHOSEN};
    // the page's own hand goes; the popup has its own
    if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
        d.out.remove(i);
    }
    // a tap off the popup is No
    d.popup(Some(pointer::Act::Row(1, pointer::OK)));
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 80.0);
    d.anchor = Anchor::Centre;
    let top = 156.0;
    d.box3([160.0, top, 320.0, 158.0], 16.0, [Some(INK), Some(WHITE), Some(PANEL)], 128.0);
    d.outlined(|d| d.label("Controller detected", 320.0, top + 14.0, 22.0, 288.0, WHITE));
    d.outlined(|d| d.label(&format!("Use {} for 1P?", a.name), 320.0, top + 38.0, 18.0, 288.0, WHITE));
    for (k, word) in ["Yes", "No, keep the keyboard"].into_iter().enumerate() {
        let y = top + 70.0 + 40.0 * k as f32;
        let on = k == a.row;
        d.pill_at([176.0, y, 288.0, 32.0], [INK, WHITE, if on { CHOSEN } else { UNCHOSEN }], 128.0);
        d.hit([176.0, y, 288.0, 32.0], pointer::Act::Row(k, pointer::OK));
        d.label(word, 320.0, y + 6.0, 20.0, 256.0, if on { WHITE } else { UNCHOSEN.map(|c| c * 1.4) });
    }
    // the time left before it keeps the keyboard
    let left = (1.0 - a.age / WAIT).clamp(0.0, 1.0);
    d.q(Tex::Solid, [0.0; 4], [192.0, top + 146.0, 256.0 * left, 3.0], WHITE, 90.0);
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [116.0 + hand_x, top + 70.0 + 40.0 * a.row as f32, 64.0, 32.0], WHITE, 128.0);
}

/// The toast after Yes, a pill at the top of the screen, fading out at the end; it takes no pointer.
fn toast(d: &mut Draw, m: &Menu) {
    let Some((s, left)) = &m.takeover.toast else { return };
    let alpha = 128.0 * (left / FADE).clamp(0.0, 1.0);
    d.anchor = Anchor::Centre;
    let w = (frame_width(d, s) + 40.0).min(560.0);
    d.pill_at([320.0 - w / 2.0, 60.0, w, 32.0], [INK, WHITE, super::super::menu::PANEL], alpha);
    let start = d.out.len();
    frame::dotted(d, s, 320.0, 66.0, 18.0, w - 32.0, WHITE, true);
    for q in &mut d.out[start..] {
        q.alpha = alpha;
    }
}

/// `s`'s width at the toast's size, its "·" drawn as `frame::dotted` draws it.
fn frame_width(d: &Draw, s: &str) -> f32 {
    let parts: Vec<&str> = s.split('·').collect();
    parts.iter().map(|p| d.width(p, 18.0)).sum::<f32>() + 9.0 * (parts.len() - 1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(n: u32) -> Entity {
        Entity::from_raw_u32(n).unwrap()
    }

    /// The main menu with 1P on the keyboard.
    fn keys_menu() -> Menu {
        Menu { screen: Screen::Main, p1: Some(Dev::Keys), ..default() }
    }

    fn press(f: impl FnOnce(&mut Press)) -> Press {
        let mut k = Press::default();
        f(&mut k);
        k
    }

    #[test]
    fn yes_gives_1p_to_the_pad_and_frees_the_keyboard() {
        let (p, other) = (pad(5), pad(6));
        let mut m = Menu { seats: [Some(Dev::Keys), None, None, None], ..keys_menu() };
        assert!(m.takeover_open(p, "Pad".into()));
        // the opening press goes nowhere, nor the menu's
        let sel = m.sel;
        assert_eq!(m.takeover_step(Dev::Pad(p), press(|k| k.ok = true)), Some(vec![]));
        m.takeover_tick(0.016, &[p, other]);
        // only that pad answers: another pad's and the keyboard's ✕ / arrows do nothing
        for dev in [Dev::Pad(other), Dev::Keys] {
            assert_eq!(m.takeover_step(dev, press(|k| k.ok = true)), Some(vec![]));
            assert_eq!(m.takeover_step(dev, press(|k| k.down = true)), Some(vec![]));
        }
        assert!(m.takeover_asking());
        assert_eq!(m.takeover_step(Dev::Pad(p), press(|k| k.ok = true)), Some(vec![Effect::Sound(2)]));
        assert_eq!((m.p1, m.seats[0], m.sel), (Some(Dev::Pad(p)), Some(Dev::Pad(p)), sel));
        assert!(!m.seats.contains(&Some(Dev::Keys)));
        assert_eq!(m.takeover.toast.as_ref().map(|t| t.0.as_str()), Some("1P: Pad · keyboard is free"));
        // the toast goes, and with 1P on a pad nothing asks again
        m.takeover_tick(TOAST, &[p, other]);
        assert_eq!(m.takeover.toast, None);
        assert!(!m.takeover_open(other, "Other".into()));
        assert_eq!(m.takeover_step(Dev::Keys, press(|k| k.back = true)), None);
    }

    #[test]
    fn no_esc_and_the_wait_keep_the_keyboard_until_replugged() {
        let p = pad(5);
        for how in 0..4 {
            let mut m = keys_menu();
            assert!(m.takeover_open(p, "Pad".into()));
            m.takeover_tick(0.016, &[p]);
            match how {
                0 => drop(m.takeover_step(Dev::Pad(p), press(|k| k.back = true))),
                1 => {
                    m.takeover_step(Dev::Pad(p), press(|k| k.down = true));
                    m.takeover_step(Dev::Pad(p), press(|k| k.ok = true));
                }
                2 => drop(m.takeover_step(Dev::Keys, press(|k| k.back = true))),
                _ => m.takeover_tick(WAIT, &[p]),
            }
            assert!(!m.takeover_asking(), "{how}");
            assert_eq!(m.p1, Some(Dev::Keys));
            // not again while plugged in; unplugged and back, it asks again
            assert!(!m.takeover_open(p, "Pad".into()));
            m.takeover_tick(0.016, &[]);
            assert!(m.takeover_open(p, "Pad".into()));
        }
    }

    #[test]
    fn only_menus_with_1p_on_keys() {
        let p = pad(5);
        let may = |m: Menu| m.clone().takeover_open(p, "Pad".into());
        assert!(may(keys_menu()));
        assert!(may(Menu { screen: Screen::Graphics, ..keys_menu() }));
        // a pad has 1P, or this pad has a seat already
        assert!(!may(Menu { p1: Some(Dev::Pad(pad(9))), ..keys_menu() }));
        assert!(!may(Menu { seats: [Some(Dev::Keys), Some(Dev::Pad(p)), None, None], ..keys_menu() }));
        // the splash, the screens a press joins on, the pause menu, mid-match
        for screen in [Screen::Press, Screen::Assign, Screen::Chars, Screen::Confirm, Screen::Lobby, Screen::Playing] {
            assert!(!may(Menu { screen, ..keys_menu() }), "{screen:?}");
        }
        // the online menus before the lobby ask too
        assert!(may(Menu { screen: Screen::Online, ..keys_menu() }));
        assert!(!may(Menu { paused: true, ..keys_menu() }));
    }

    #[test]
    fn taps_choose() {
        let p = pad(5);
        let mut m = keys_menu();
        m.takeover_open(p, "Pad".into());
        m.takeover_tick(0.016, &[p]);
        // a hover highlights, a tap on Yes chooses it
        assert_eq!(m.takeover_point(pointer::Act::Row(1, pointer::OK), false), Some(vec![Effect::Sound(0)]));
        assert_eq!(m.takeover_point(pointer::Act::Row(0, pointer::OK), true), Some(vec![Effect::Sound(0), Effect::Sound(2)]));
        assert_eq!(m.p1, Some(Dev::Pad(p)));
        // drawn with its two pills to tap, over everything, at any aspect (centre-pinned)
        let mut m = keys_menu();
        m.takeover_open(p, "Pad".into());
        let (hits, outside) = super::super::pointer::tests::hits(&m);
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|h| h.anchor == Anchor::Centre));
        assert_eq!(outside, Some(pointer::Act::Row(1, pointer::OK)));
    }
}
