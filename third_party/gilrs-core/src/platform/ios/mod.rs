// HST M91: iOS gamepads through Apple's GameController framework (GCController's extended gamepad profile), polled.
#[path = "../default/ff.rs"]
mod ff;
mod gamepad;

pub use self::ff::Device as FfDevice;
pub use self::gamepad::{native_ev_codes, EvCode, Gamepad, Gilrs};

// GCController's sticks already point up.
pub const IS_Y_AXIS_REVERSED: bool = false;
