//! `HST_TRACE=1`: logs (stderr) every menu cursor change and the input around it: keys, pad buttons and sticks, the
//! window's pointer and focus events, the hidden-mouse flag. For input bugs only seen on someone's own desktop.

use std::sync::atomic::Ordering;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorEntered, CursorLeft, CursorMoved, WindowFocused};

use super::Menu;

pub(super) fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("HST_TRACE").is_some())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn trace(
    menu: Res<Menu>,
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<(Entity, &Gamepad)>,
    time: Res<Time>,
    mut moved: MessageReader<CursorMoved>,
    mut left: MessageReader<CursorLeft>,
    mut entered: MessageReader<CursorEntered>,
    mut focus: MessageReader<WindowFocused>,
    mut motion: MessageReader<MouseMotion>,
    mut was: Local<String>,
    mut idle: Local<bool>,
) {
    let t = time.elapsed_secs();
    let at = format!("{:?} sel {} cursors {:?}", menu.screen, menu.sel, menu.players.map(|p| (p.cursor, p.costume)));
    if *at != *was {
        eprintln!("{t:9.3} MENU {at}");
        *was = at;
    }
    let i = super::pointer::MOUSE_IDLE.load(Ordering::Relaxed);
    if i != *idle {
        eprintln!("{t:9.3} mouse idle {i}");
        *idle = i;
    }
    for k in keys.get_just_pressed() {
        eprintln!("{t:9.3} key down {k:?}");
    }
    for k in keys.get_just_released() {
        eprintln!("{t:9.3} key up {k:?}");
    }
    for (e, g) in &pads {
        for b in g.get_just_pressed() {
            eprintln!("{t:9.3} pad {e} down {b:?}");
        }
        for b in g.get_just_released() {
            eprintln!("{t:9.3} pad {e} up {b:?}");
        }
        let (l, r) = (g.left_stick(), g.right_stick());
        if l.length() > 0.3 || r.length() > 0.3 {
            eprintln!("{t:9.3} pad {e} sticks {l} {r}");
        }
    }
    for e in moved.read() {
        eprintln!("{t:9.3} cursor moved {} delta {:?}", e.position, e.delta);
    }
    for _ in left.read() {
        eprintln!("{t:9.3} cursor left");
    }
    for _ in entered.read() {
        eprintln!("{t:9.3} cursor entered");
    }
    for e in focus.read() {
        eprintln!("{t:9.3} focused {}", e.focused);
    }
    let m: Vec2 = motion.read().map(|m| m.delta).sum();
    if m != Vec2::ZERO {
        eprintln!("{t:9.3} mouse motion {m}");
    }
}
