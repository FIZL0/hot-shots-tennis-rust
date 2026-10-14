//! B301 (ours, by the user's request): the match's keyboard hints. While the keyboard plays a seat and was the last
//! device used, the menus' legend strip (`main_menu::legend`, bottom right under the 4:3 screen's description bar, in
//! keycaps) lists the match's keys from `controls.txt`: move, the three shots, serve, the emotes and pause. A pad or a
//! touch hides it (switching live, as the legend does), and so does the Key Hints setting (Settings → Display,
//! `key_hints` in settings.txt, on by default). Whatever else fills the legend (the pause menu, its Settings screens,
//! the result screen) takes the strip instead. Presentation only: nothing here reaches the sim.

use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

use super::controls::{Action, Bindings, PadSlots};
use super::main_menu::legend::{self, Act, Item, Last, Legend};

/// The Key Hints setting, as the main menu (and the pause menu's Settings) last had it.
pub static ON: AtomicBool = AtomicBool::new(true);

/// The match's keys, by the bindings (a key-less action drops its cap; an action with none drops out).
fn items(bind: &Bindings) -> Vec<Item> {
    let k = |a: Action| bind.key(a);
    let caps = |acts: &[Action]| acts.iter().filter_map(|&a| k(a)).collect::<Vec<_>>();
    let mut v = vec![
        (Act::Move, "Move", caps(&[Action::Up, Action::Left, Action::Down, Action::Right])),
        (Act::Key, "Topspin", caps(&[Action::Normal])),
        (Act::Key, "Slice", caps(&[Action::Cut])),
        (Act::Key, "Lob", caps(&[Action::Lob])),
        (Act::Key, "Serve", caps(&[Action::Serve])),
        (Act::Key, "Emotes", caps(&[Action::EmoteUp, Action::EmoteDown, Action::EmoteLeft, Action::EmoteRight])),
    ];
    // `menu.rs`: Esc pauses (fixed, not in controls.txt)
    v.push((Act::Key, "Pause", vec!["Esc".into()]));
    v.into_iter().filter(|(_, _, keys)| !keys.is_empty()).map(|(act, label, keys)| Item { act, label: label.into(), keys }).collect()
}

/// Whether the hints show: the setting on, the keyboard last used and playing a seat (no autoplay).
fn shown(on: bool, last: Last, keys_seat: bool, autoplay: bool) -> bool {
    on && last == Last::Keys && keys_seat && !autoplay
}

fn fill(bind: Res<Bindings>, last: Res<Last>, slots: Res<PadSlots>, pads: Option<Res<super::Pads>>, mut out: ResMut<Legend>) {
    let seat = slots.keys().is_some_and(|k| pads.as_ref().is_none_or(|p| p.seats.is_none_or(|h| h[k])));
    if !out.0.is_empty() || !shown(ON.load(Ordering::Relaxed), *last, seat, pads.as_ref().is_some_and(|p| p.autoplay)) {
        return;
    }
    out.one(*last, items(&bind));
}

pub fn plugin(app: &mut App) {
    legend::plugin(app);
    app.add_systems(PostUpdate, fill.before(legend::draw));
}

#[test]
fn hints_follow_the_bindings_and_the_keyboard() {
    let (b, _) = Bindings::parse("topspin = Z\nlob = PadNorth\n");
    let got: Vec<_> = items(&b).into_iter().map(|i| format!("{} {}", i.label, i.keys.join(" "))).collect();
    // lob has no key left: no entry
    assert_eq!(got, ["Move W A S D", "Topspin Z", "Slice K", "Serve Space", "Emotes 1 2 3 4", "Pause Esc"]);
    assert!(shown(true, Last::Keys, true, false));
    assert!(!shown(false, Last::Keys, true, false), "setting off");
    assert!(!shown(true, Last::Pad(Entity::PLACEHOLDER), true, false), "pad last used");
    assert!(!shown(true, Last::Touch, true, false), "touch");
    assert!(!shown(true, Last::Keys, false, false), "keyboard has no seat");
    assert!(!shown(true, Last::Keys, true, true), "autoplay");
}
