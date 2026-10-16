//! M95: what a player plays on — the platform (browser, Android, iOS, desktop) and the input they last used (keyboard
//! and mouse, controller, touch) — for the icons beside their name in an online lobby. Sent in the lobby's Hello and
//! again when the input changes (`net::Cmd::Device`); never in the match or the handshake check (M2e): it changes
//! nothing that's played.

use bevy::input::gamepad::GamepadEvent;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::touch::TouchInput;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    #[default]
    Desktop,
    Browser,
    Android,
    Ios,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    #[default]
    Keyboard,
    Controller,
    Touch,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub platform: Platform,
    pub input: Input,
}

/// This build's platform (a browser on a phone is the browser).
pub const PLATFORM: Platform = if cfg!(target_arch = "wasm32") {
    Platform::Browser
} else if cfg!(target_os = "android") {
    Platform::Android
} else if cfg!(target_os = "ios") {
    Platform::Ios
} else {
    Platform::Desktop
};

impl Device {
    /// This machine before any input: a phone's touch, else the keyboard.
    pub const fn here() -> Device {
        let input = if matches!(PLATFORM, Platform::Android | Platform::Ios) { Input::Touch } else { Input::Keyboard };
        Device { platform: PLATFORM, input }
    }
}

/// The input a frame's events say was used (`None` with none).
pub fn used(keys: bool, mouse: bool, pad: bool, touch: bool) -> Option<Input> {
    // a tap also comes as a mouse click on some platforms (the browser's emulated mouse): touch first
    match () {
        _ if touch => Some(Input::Touch),
        _ if pad => Some(Input::Controller),
        _ if keys || mouse => Some(Input::Keyboard),
        _ => None,
    }
}

/// The `Device` resource's value, for a network started outside a system (`Net::start`'s Hello).
pub static CURRENT: std::sync::Mutex<Device> = std::sync::Mutex::new(Device::here());

/// Keeps the `Device` resource on the input last used; a change goes to the lobby (`Net`) if there is one.
pub fn watch(
    mut dev: ResMut<Device>,
    mut keys: MessageReader<KeyboardInput>,
    mut mouse: MessageReader<MouseButtonInput>,
    mut pads: MessageReader<GamepadEvent>,
    mut touches: MessageReader<TouchInput>,
    net: Option<Res<crate::net::Net>>,
    mut sent: Local<Option<Device>>,
) {
    // a pad's button or a stick pushed past a deadzone, not a connection or a resting stick's noise
    let pad = pads.read().any(|e| match e {
        GamepadEvent::Button(b) => b.value > 0.5,
        GamepadEvent::Axis(a) => a.value.abs() > 0.5,
        GamepadEvent::Connection(_) => false,
    });
    let input = used(keys.read().count() > 0, mouse.read().count() > 0, pad, touches.read().count() > 0);
    if let Some(input) = input
        && dev.input != input
    {
        dev.input = input;
    }
    *CURRENT.lock().unwrap() = *dev;
    // a new network (it starts with `CURRENT`) or a change: the lobby hears it
    if let Some(n) = net
        && (n.is_added() || *sent != Some(*dev))
    {
        n.send(crate::net::Cmd::Device(*dev));
        *sent = Some(*dev);
    }
}

/// The `Device` resource and its watcher.
pub fn plugin(app: &mut App) {
    app.insert_resource(Device::here()).add_systems(PreUpdate, watch.after(bevy::input::InputSystems));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_input_wins_touch_over_its_mouse() {
        assert_eq!(used(false, false, false, false), None);
        assert_eq!(used(true, false, false, false), Some(Input::Keyboard));
        assert_eq!(used(false, true, false, false), Some(Input::Keyboard));
        assert_eq!(used(false, false, true, false), Some(Input::Controller));
        assert_eq!(used(false, true, false, true), Some(Input::Touch));
        assert_eq!(Device::here().platform, Platform::Desktop);
    }

    #[test]
    fn small_on_the_wire() {
        let d = Device { platform: Platform::Android, input: Input::Touch };
        let s = serde_json::to_string(&d).unwrap();
        assert_eq!(s, r#"{"platform":"Android","input":"Touch"}"#);
        assert_eq!(serde_json::from_str::<Device>(&s).unwrap(), d);
    }
}
