//! M101: controllers on Android. gilrs has no Android backend, so the patched winit (`third_party/winit`,
//! `platform::android::gamepad`) takes pad key and motion events out of its event stream, and this turns them into the
//! raw gamepad messages gilrs writes on desktop: menus and matches read the pad through the same `Gamepad` path, with
//! the same bindings (`play/controls.rs`). Buttons map as Android's standard layout names them (`KEYCODE_BUTTON_A` = ✕/A,
//! `BUTTON_X` = □/X, …), the left stick is AXIS_X/Y, the right one Z/RZ, the d-pad either key events or the hat axes, the analog
//! triggers L2/R2.
//!
//! Hot-plug (USB or Bluetooth): a thread asks Android for its input devices (`InputDevice.getDeviceIds`, over JNI)
//! every second; a pad that shows up connects with its name and USB ids (Sony's get the ✕ ○ legend), one that's gone
//! disconnects. A pad's first event connects it too, if the list hasn't caught up with it yet (Android's own `input`
//! tool injects as device -1, which is never listed).

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use std::collections::HashMap;

use bevy::input::gamepad::{GamepadConnection, GamepadConnectionEvent, GamepadInput};
use bevy::prelude::*;

/// The pad button for an Android key code (`KeyEvent.KEYCODE_*`).
fn button(keycode: u32) -> Option<GamepadButton> {
    use GamepadButton::*;
    Some(match keycode {
        96 => South,
        97 => East,
        98 => C,
        99 => West,
        100 => North,
        101 => Z,
        102 => LeftTrigger,
        103 => RightTrigger,
        104 => LeftTrigger2,
        105 => RightTrigger2,
        106 => LeftThumb,
        107 => RightThumb,
        108 => Start,
        109 => Select,
        110 => Mode,
        19 => DPadUp,
        20 => DPadDown,
        21 => DPadLeft,
        22 => DPadRight,
        // some layouts give a pad's View/Back button KEYCODE_BACK
        4 => Select,
        _ => return None,
    })
}

/// What one motion event changed, from the pad's axes before (`was`) to now, in the patched winit's `AXES` order
/// (X, Y, Z, RZ, RX, RY, HAT_X, HAT_Y, LTRIGGER, RTRIGGER, BRAKE, GAS). Android's sticks point y down, bevy's up; the
/// hat is the d-pad, ±1 per direction; a trigger is the larger of its two axes (pads report one or the other).
fn motion(was: &[f32; 12], now: &[f32; 12]) -> Vec<(GamepadInput, f32)> {
    let read = |a: &[f32; 12]| {
        use GamepadAxis::*;
        use GamepadButton::*;
        [
            (GamepadInput::Axis(LeftStickX), a[0]),
            (GamepadInput::Axis(LeftStickY), -a[1]),
            (GamepadInput::Axis(RightStickX), a[2]),
            (GamepadInput::Axis(RightStickY), -a[3]),
            (GamepadInput::Button(DPadLeft), (a[6] < -0.5) as u8 as f32),
            (GamepadInput::Button(DPadRight), (a[6] > 0.5) as u8 as f32),
            (GamepadInput::Button(DPadUp), (a[7] < -0.5) as u8 as f32),
            (GamepadInput::Button(DPadDown), (a[7] > 0.5) as u8 as f32),
            (GamepadInput::Button(LeftTrigger2), a[8].max(a[10]).clamp(0.0, 1.0)),
            (GamepadInput::Button(RightTrigger2), a[9].max(a[11]).clamp(0.0, 1.0)),
        ]
    };
    read(was).into_iter().zip(read(now)).filter(|(w, n)| w.1 != n.1).map(|(_, n)| n).collect()
}

/// An input device Android lists as a gamepad or joystick.
#[derive(Clone, Debug, PartialEq)]
struct Device {
    id: i32,
    name: String,
    vendor: u16,
    product: u16,
}

#[derive(Default)]
struct Pad {
    entity: Option<Entity>,
    connected: bool,
    /// Seen in Android's device list (only those disconnect when they leave it).
    listed: bool,
    /// Left the list: events still queued from it (Android sends a last one as a pad goes) don't connect it again.
    gone: bool,
    axes: [f32; 12],
}

#[derive(Resource, Default)]
struct Pads(HashMap<i32, Pad>);

#[cfg(target_os = "android")]
pub fn plugin(app: &mut App) {
    std::thread::spawn(devices::watch);
    app.init_resource::<Pads>().add_systems(PreUpdate, read.before(bevy::input::InputSystems));
}

/// The messages gilrs writes for the same input (`bevy_gilrs`' event system).
#[derive(bevy::ecs::system::SystemParam)]
struct Out<'w> {
    raw: MessageWriter<'w, bevy::input::gamepad::RawGamepadEvent>,
    connections: MessageWriter<'w, GamepadConnectionEvent>,
    buttons: MessageWriter<'w, bevy::input::gamepad::RawGamepadButtonChangedEvent>,
    axes: MessageWriter<'w, bevy::input::gamepad::RawGamepadAxisChangedEvent>,
}

impl Out<'_> {
    fn connection(&mut self, event: GamepadConnectionEvent) {
        self.raw.write(event.clone().into());
        self.connections.write(event);
    }

    fn input(&mut self, gamepad: Entity, input: GamepadInput, value: f32) {
        use bevy::input::gamepad::{RawGamepadAxisChangedEvent as A, RawGamepadButtonChangedEvent as B};
        match input {
            GamepadInput::Button(b) => {
                self.raw.write(B::new(gamepad, b, value).into());
                self.buttons.write(B::new(gamepad, b, value));
            }
            GamepadInput::Axis(a) => {
                self.raw.write(A::new(gamepad, a, value).into());
                self.axes.write(A::new(gamepad, a, value));
            }
        }
    }
}

/// Connects a pad (again on the same entity, if a mode switch hasn't taken it).
fn connect(commands: &mut Commands, entities: &bevy::ecs::entity::Entities, out: &mut Out, pad: &mut Pad, device: Option<&Device>, id: i32) {
    let entity = match pad.entity.filter(|&e| entities.contains(e)) {
        Some(e) => e,
        None => *pad.entity.insert(commands.spawn_empty().id()),
    };
    let (name, vendor_id, product_id) = match device {
        Some(d) => (d.name.clone(), Some(d.vendor).filter(|&v| v != 0), Some(d.product).filter(|&p| p != 0)),
        None => (format!("Gamepad {id}"), None, None),
    };
    out.connection(GamepadConnectionEvent::new(entity, GamepadConnection::Connected { name, vendor_id, product_id }));
    pad.connected = true;
    pad.axes = [0.0; 12];
}

#[cfg(target_os = "android")]
fn read(mut commands: Commands, entities: &bevy::ecs::entity::Entities, mut pads: ResMut<Pads>, mut out: Out) {
    use winit::platform::android::gamepad::PadEvent;
    let listed = devices::DEVICES.lock().unwrap().take();
    if let Some(list) = &listed {
        for d in list {
            let pad = pads.0.entry(d.id).or_default();
            pad.listed = true;
            pad.gone = false;
            if !pad.connected {
                info!("gamepad connected: {} ({:04x}:{:04x})", d.name, d.vendor, d.product);
                connect(&mut commands, entities, &mut out, pad, Some(d), d.id);
            }
        }
        for (id, pad) in &mut pads.0 {
            if pad.listed && !list.iter().any(|d| d.id == *id) {
                info!("gamepad {id} disconnected");
                pad.listed = false;
                pad.gone = true;
                pad.connected = false;
                if let Some(e) = pad.entity.filter(|&e| entities.contains(e)) {
                    out.connection(GamepadConnectionEvent::new(e, GamepadConnection::Disconnected));
                }
            }
        }
    }
    for event in winit::platform::android::gamepad::take() {
        let (PadEvent::Key { device, .. } | PadEvent::Motion { device, .. }) = event;
        let pad = pads.0.entry(device).or_default();
        if pad.gone {
            continue;
        }
        if !pad.connected || !pad.entity.is_some_and(|e| entities.contains(e)) {
            connect(&mut commands, entities, &mut out, pad, devices::device(device).as_ref(), device);
        }
        let Some(entity) = pad.entity else { continue };
        match event {
            PadEvent::Key { keycode, pressed, .. } => {
                if let Some(b) = button(keycode) {
                    out.input(entity, GamepadInput::Button(b), if pressed { 1.0 } else { 0.0 });
                }
            }
            PadEvent::Motion { axes, .. } => {
                for (input, value) in motion(&pad.axes, &axes) {
                    out.input(entity, input, value);
                }
                pad.axes = axes;
            }
        }
    }
}

/// Android's input device list, over JNI.
#[cfg(target_os = "android")]
mod devices {
    use std::sync::Mutex;

    use jni::objects::{JIntArray, JString};
    use jni::{JNIEnv, JavaVM};

    use super::Device;

    /// The pads listed since the frame last looked (`None`: no new list).
    pub(super) static DEVICES: Mutex<Option<Vec<Device>>> = Mutex::new(None);

    const SOURCE_GAMEPAD: i32 = 0x0000_0401;
    const SOURCE_JOYSTICK: i32 = 0x0100_0010;

    fn vm() -> Option<JavaVM> {
        let app = bevy::android::ANDROID_APP.get()?;
        unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()
    }

    /// Lists the pads every second, for as long as the app runs.
    pub(super) fn watch() {
        let Some(vm) = vm() else { return };
        let Ok(mut env) = vm.attach_current_thread_permanently() else { return };
        let mut last = None;
        loop {
            let list = env.with_local_frame(64, |env| Ok::<_, jni::errors::Error>(pads(env))).ok().flatten();
            if list.is_some() && list != last {
                *DEVICES.lock().unwrap() = list.clone();
                last = list;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    /// One device by its id, when an event comes before the list has it.
    pub(super) fn device(id: i32) -> Option<Device> {
        let vm = vm()?;
        let mut env = vm.attach_current_thread().ok()?;
        env.with_local_frame(16, |env| Ok::<_, jni::errors::Error>(info(env, id))).ok().flatten().map(|(d, _)| d)
    }

    fn pads(env: &mut JNIEnv) -> Option<Vec<Device>> {
        let ids = caught(env, |env| {
            let ids: JIntArray = env.call_static_method("android/view/InputDevice", "getDeviceIds", "()[I", &[])?.l()?.into();
            let mut buf = vec![0; env.get_array_length(&ids)? as usize];
            env.get_int_array_region(&ids, 0, &mut buf)?;
            Ok(buf)
        })?;
        Some(ids.into_iter().filter_map(|id| info(env, id)).filter(|(_, pad)| *pad).map(|(d, _)| d).collect())
    }

    /// A device's name and USB ids, and whether it's a (non-virtual) gamepad or joystick.
    fn info(env: &mut JNIEnv, id: i32) -> Option<(Device, bool)> {
        caught(env, |env| {
            let dev = env.call_static_method("android/view/InputDevice", "getDevice", "(I)Landroid/view/InputDevice;", &[id.into()])?.l()?;
            if dev.is_null() {
                return Ok(None);
            }
            let sources = env.call_method(&dev, "getSources", "()I", &[])?.i()?;
            let virt = env.call_method(&dev, "isVirtual", "()Z", &[])?.z()?;
            let name: JString = env.call_method(&dev, "getName", "()Ljava/lang/String;", &[])?.l()?.into();
            let name: String = if name.is_null() { String::new() } else { env.get_string(&name)?.into() };
            let vendor = env.call_method(&dev, "getVendorId", "()I", &[])?.i()? as u16;
            let product = env.call_method(&dev, "getProductId", "()I", &[])?.i()? as u16;
            let pad = !virt && (sources & SOURCE_GAMEPAD == SOURCE_GAMEPAD || sources & SOURCE_JOYSTICK == SOURCE_JOYSTICK);
            Ok(Some((Device { id, name, vendor, product }, pad)))
        })
        .flatten()
    }

    /// `f`'s result; a Java exception it raised is cleared and gives `None`.
    fn caught<T>(env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv) -> jni::errors::Result<T>) -> Option<T> {
        let r = f(env);
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
        r.ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_buttons_map_to_the_desktop_pads() {
        assert_eq!(button(96), Some(GamepadButton::South));
        assert_eq!(button(97), Some(GamepadButton::East));
        assert_eq!(button(100), Some(GamepadButton::North));
        assert_eq!(button(99), Some(GamepadButton::West));
        assert_eq!(button(108), Some(GamepadButton::Start));
        assert_eq!(button(109), Some(GamepadButton::Select));
        assert_eq!(button(21), Some(GamepadButton::DPadLeft));
        assert_eq!(button(29), None);
    }

    #[test]
    fn motion_sends_only_what_changed() {
        let rest = [0.0; 12];
        let mut a = rest;
        a[0] = 0.5;
        a[1] = 0.25; // stick pushed down-right
        a[7] = -1.0; // hat up
        a[11] = 0.75; // GAS
        let out = motion(&rest, &a);
        assert_eq!(
            out,
            vec![
                (GamepadInput::Axis(GamepadAxis::LeftStickX), 0.5),
                (GamepadInput::Axis(GamepadAxis::LeftStickY), -0.25),
                (GamepadInput::Button(GamepadButton::DPadUp), 1.0),
                (GamepadInput::Button(GamepadButton::RightTrigger2), 0.75),
            ]
        );
        assert!(motion(&a, &a).is_empty());
        assert_eq!(motion(&a, &rest).len(), 4);
    }
}
