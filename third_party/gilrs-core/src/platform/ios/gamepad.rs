// HST M91: each next_event() with no queued events reads every connected GCController's extended gamepad and queues
// the changes since the last read (like the wasm backend). Controllers are told apart by object address.
use super::FfDevice;
use crate::{AxisInfo, Event, EventType, PlatformError, PowerInfo};
use objc2::rc::autoreleasepool;
use objc2::runtime::{AnyClass, AnyObject, MessageReceiver, Sel};
use std::collections::VecDeque;
use std::ffi::CStr;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::time::Duration;
use uuid::Uuid;

#[link(name = "GameController", kind = "framework")]
unsafe extern "C" {}

type Id = *mut AnyObject;

/// `[o sel]` for an object-returning property; nil in, nil out.
unsafe fn get(o: Id, sel: &CStr) -> Id {
    if o.is_null() { o } else { unsafe { o.send_message(Sel::register(sel), ()) } }
}

/// A GCControllerButtonInput / GCControllerAxisInput's `value`; 0 when the element is missing.
unsafe fn value(o: Id) -> f32 {
    if o.is_null() { 0.0 } else { unsafe { o.send_message(Sel::register(c"value"), ()) } }
}

use native_ev_codes as nec;

/// Element paths from the extended gamepad (Apple's A/B/X/Y are bottom/right/left/top).
const BUTTONS: [(EvCode, &[&CStr]); 15] = [
    (nec::BTN_SOUTH, &[c"buttonA"]),
    (nec::BTN_EAST, &[c"buttonB"]),
    (nec::BTN_WEST, &[c"buttonX"]),
    (nec::BTN_NORTH, &[c"buttonY"]),
    (nec::BTN_LT, &[c"leftShoulder"]),
    (nec::BTN_RT, &[c"rightShoulder"]),
    (nec::BTN_START, &[c"buttonMenu"]),
    (nec::BTN_SELECT, &[c"buttonOptions"]),
    (nec::BTN_MODE, &[c"buttonHome"]),
    (nec::BTN_LTHUMB, &[c"leftThumbstickButton"]),
    (nec::BTN_RTHUMB, &[c"rightThumbstickButton"]),
    (nec::BTN_DPAD_UP, &[c"dpad", c"up"]),
    (nec::BTN_DPAD_DOWN, &[c"dpad", c"down"]),
    (nec::BTN_DPAD_LEFT, &[c"dpad", c"left"]),
    (nec::BTN_DPAD_RIGHT, &[c"dpad", c"right"]),
];
/// Triggers are axes 0..1 (gilrs turns them into LeftTrigger2/RightTrigger2 buttons), sticks −1..1.
const AXES: [(EvCode, &[&CStr]); 6] = [
    (nec::AXIS_LT2, &[c"leftTrigger"]),
    (nec::AXIS_RT2, &[c"rightTrigger"]),
    (nec::AXIS_LSTICKX, &[c"leftThumbstick", c"xAxis"]),
    (nec::AXIS_LSTICKY, &[c"leftThumbstick", c"yAxis"]),
    (nec::AXIS_RSTICKX, &[c"rightThumbstick", c"xAxis"]),
    (nec::AXIS_RSTICKY, &[c"rightThumbstick", c"yAxis"]),
];
const BUTTON_CODES: [EvCode; 15] = {
    let mut c = [EvCode(0); 15];
    let mut i = 0;
    while i < 15 {
        c[i] = BUTTONS[i].0;
        i += 1;
    }
    c
};
const AXIS_CODES: [EvCode; 6] = [AXES[0].0, AXES[1].0, AXES[2].0, AXES[3].0, AXES[4].0, AXES[5].0];

unsafe fn element(pad: Id, path: &[&CStr]) -> Id {
    path.iter().fold(pad, |o, sel| unsafe { get(o, sel) })
}

#[derive(Debug)]
pub struct Gilrs {
    gamepads: Vec<Gamepad>,
    events: VecDeque<Event>,
}

impl Gilrs {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        if AnyClass::get(c"GCController").is_none() {
            return Err(PlatformError::NotImplemented(Gilrs { gamepads: Vec::new(), events: VecDeque::new() }));
        }
        Ok(Gilrs { gamepads: Vec::new(), events: VecDeque::new() })
    }

    pub(crate) fn next_event(&mut self) -> Option<Event> {
        if self.events.is_empty() {
            autoreleasepool(|_| unsafe { self.poll() });
        }
        self.events.pop_front()
    }

    unsafe fn poll(&mut self) {
        let cls = AnyClass::get(c"GCController").unwrap();
        let list: Id = unsafe { cls.send_message(Sel::register(c"controllers"), ()) };
        let n: usize = if list.is_null() { 0 } else { unsafe { list.send_message(Sel::register(c"count"), ()) } };
        let mut seen = Vec::with_capacity(n);
        for i in 0..n {
            let ctrl: Id = unsafe { list.send_message(Sel::register(c"objectAtIndex:"), (i,)) };
            let pad = unsafe { get(ctrl, c"extendedGamepad") };
            if pad.is_null() {
                continue; // no full gamepad profile (e.g. a Siri Remote)
            }
            let key = ctrl as usize;
            seen.push(key);
            let id = match self.gamepads.iter().position(|g| g.key == key) {
                Some(id) => id,
                None => {
                    let id = self.gamepads.iter().position(|g| !g.connected).unwrap_or(self.gamepads.len());
                    let g = Gamepad { key, name: unsafe { name(ctrl) }, connected: true, buttons: [false; 15], axes: [0.0; 6] };
                    if id == self.gamepads.len() { self.gamepads.push(g) } else { self.gamepads[id] = g }
                    self.events.push_back(Event::new(id, EventType::Connected));
                    id
                }
            };
            let g = &mut self.gamepads[id];
            for (k, (code, path)) in BUTTONS.iter().enumerate() {
                let el = unsafe { element(pad, path) };
                let down = !el.is_null() && unsafe { el.send_message::<_, objc2::runtime::Bool>(Sel::register(c"isPressed"), ()).as_bool() };
                if down != g.buttons[k] {
                    g.buttons[k] = down;
                    let ev = if down { EventType::ButtonPressed(crate::EvCode(*code)) } else { EventType::ButtonReleased(crate::EvCode(*code)) };
                    self.events.push_back(Event::new(id, ev));
                }
            }
            for (k, (code, path)) in AXES.iter().enumerate() {
                let v = unsafe { value(element(pad, path)) };
                if v != g.axes[k] {
                    g.axes[k] = v;
                    self.events.push_back(Event::new(id, EventType::AxisValueChanged((v as f64 * i32::MAX as f64) as i32, crate::EvCode(*code))));
                }
            }
        }
        for (id, g) in self.gamepads.iter_mut().enumerate() {
            if g.connected && !seen.contains(&g.key) {
                g.connected = false;
                self.events.push_back(Event::new(id, EventType::Disconnected));
            }
        }
    }

    pub(crate) fn next_event_blocking(&mut self, _timeout: Option<Duration>) -> Option<Event> {
        self.next_event() // ponytail: never blocks; bevy_gilrs only polls
    }

    pub fn gamepad(&self, id: usize) -> Option<&Gamepad> {
        self.gamepads.get(id)
    }

    pub fn last_gamepad_hint(&self) -> usize {
        self.gamepads.len()
    }
}

unsafe fn name(ctrl: Id) -> String {
    let s = unsafe { get(ctrl, c"vendorName") };
    if s.is_null() {
        return "Game Controller".into();
    }
    let p: *const std::ffi::c_char = unsafe { s.send_message(Sel::register(c"UTF8String"), ()) };
    if p.is_null() { "Game Controller".into() } else { unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned() }
}

#[derive(Debug)]
pub struct Gamepad {
    key: usize,
    name: String,
    connected: bool,
    buttons: [bool; 15],
    axes: [f32; 6],
}

impl Gamepad {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> Uuid {
        Uuid::nil() // no SDL mapping: gilrs's default mapping of the codes below applies
    }

    pub fn vendor_id(&self) -> Option<u16> {
        None
    }

    pub fn product_id(&self) -> Option<u16> {
        None
    }

    pub fn power_info(&self) -> PowerInfo {
        PowerInfo::Unknown
    }

    pub fn is_ff_supported(&self) -> bool {
        false
    }

    pub fn ff_device(&self) -> Option<FfDevice> {
        None
    }

    pub fn buttons(&self) -> &[EvCode] {
        &BUTTON_CODES
    }

    pub fn axes(&self) -> &[EvCode] {
        &AXIS_CODES
    }

    pub(crate) fn axis_info(&self, code: EvCode) -> Option<&AxisInfo> {
        const TRIGGER: AxisInfo = AxisInfo { min: 0, max: i32::MAX, deadzone: None };
        const STICK: AxisInfo = AxisInfo { min: -i32::MAX, max: i32::MAX, deadzone: None };
        Some(if code == nec::AXIS_LT2 || code == nec::AXIS_RT2 { &TRIGGER } else { &STICK })
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(feature = "serde-serialize")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde-serialize", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EvCode(u8);

impl EvCode {
    pub fn into_u32(self) -> u32 {
        self.0 as u32
    }
}

impl Display for EvCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.0.fmt(f)
    }
}

pub mod native_ev_codes {
    use super::EvCode;

    pub const AXIS_LSTICKX: EvCode = EvCode(0);
    pub const AXIS_LSTICKY: EvCode = EvCode(1);
    pub const AXIS_LEFTZ: EvCode = EvCode(2);
    pub const AXIS_RSTICKX: EvCode = EvCode(3);
    pub const AXIS_RSTICKY: EvCode = EvCode(4);
    pub const AXIS_RIGHTZ: EvCode = EvCode(5);
    pub const AXIS_DPADX: EvCode = EvCode(6);
    pub const AXIS_DPADY: EvCode = EvCode(7);
    pub const AXIS_RT: EvCode = EvCode(8);
    pub const AXIS_LT: EvCode = EvCode(9);
    pub const AXIS_RT2: EvCode = EvCode(10);
    pub const AXIS_LT2: EvCode = EvCode(11);

    pub const BTN_SOUTH: EvCode = EvCode(12);
    pub const BTN_EAST: EvCode = EvCode(13);
    pub const BTN_C: EvCode = EvCode(14);
    pub const BTN_NORTH: EvCode = EvCode(15);
    pub const BTN_WEST: EvCode = EvCode(16);
    pub const BTN_Z: EvCode = EvCode(17);
    pub const BTN_LT: EvCode = EvCode(18);
    pub const BTN_RT: EvCode = EvCode(19);
    pub const BTN_LT2: EvCode = EvCode(20);
    pub const BTN_RT2: EvCode = EvCode(21);
    pub const BTN_SELECT: EvCode = EvCode(22);
    pub const BTN_START: EvCode = EvCode(23);
    pub const BTN_MODE: EvCode = EvCode(24);
    pub const BTN_LTHUMB: EvCode = EvCode(25);
    pub const BTN_RTHUMB: EvCode = EvCode(26);

    pub const BTN_DPAD_UP: EvCode = EvCode(27);
    pub const BTN_DPAD_DOWN: EvCode = EvCode(28);
    pub const BTN_DPAD_LEFT: EvCode = EvCode(29);
    pub const BTN_DPAD_RIGHT: EvCode = EvCode(30);
}
