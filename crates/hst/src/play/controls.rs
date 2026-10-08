//! Rebindable controls and controller slots that survive hot-plugging (remaster-only, never touches the 60 Hz sim).
//!
//! Bindings live in `controls.txt` beside the disc image, written with the defaults on first run: one line per
//! action, `action = Name Name …`, keys by their Bevy `KeyCode` name (`KeyJ` or just `J`, `Digit1` or `1`, `Space`,
//! `ArrowLeft`, …) and pad buttons with a `Pad` prefix (`PadSouth`, `PadLeftTrigger`, …). Unknown names are reported
//! and skipped; an action left without a binding keeps its default.
//!
//! Controllers keep their slot while connected: unplugging one frees its slot (the other pad stays where it was) and
//! the next pad plugged in takes the first free slot.

use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    /// ✕: topspin, serve.
    Normal,
    /// ○: slice.
    Cut,
    /// △: lob.
    Lob,
    /// Serve only (keyboard Space by default; ✕ serves too, as the original).
    Serve,
    Camera,
    TurnLeft,
    TurnRight,
}

const ACTIONS: [(Action, &str); 11] = [
    (Action::Up, "up"),
    (Action::Down, "down"),
    (Action::Left, "left"),
    (Action::Right, "right"),
    (Action::Normal, "topspin"),
    (Action::Cut, "slice"),
    (Action::Lob, "lob"),
    (Action::Serve, "serve"),
    (Action::Camera, "camera"),
    (Action::TurnLeft, "turn_left"),
    (Action::TurnRight, "turn_right"),
];

/// One binding: a key or a pad button.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Input {
    Key(KeyCode),
    Pad(GamepadButton),
}

#[derive(Resource, Clone, PartialEq, Debug)]
pub struct Bindings([Vec<Input>; 11]);

impl Default for Bindings {
    fn default() -> Self {
        use GamepadButton as P;
        use Input::{Key as K, Pad};
        use KeyCode as C;
        Bindings([
            vec![K(C::KeyW), Pad(P::DPadUp)],
            vec![K(C::KeyS), Pad(P::DPadDown)],
            vec![K(C::KeyA), Pad(P::DPadLeft)],
            vec![K(C::KeyD), Pad(P::DPadRight)],
            vec![K(C::KeyJ), Pad(P::South)],
            vec![K(C::KeyK), Pad(P::East)],
            vec![K(C::KeyL), Pad(P::North)],
            vec![K(C::Space)],
            vec![K(C::KeyC), Pad(P::Select)],
            vec![K(C::ArrowLeft)],
            vec![K(C::ArrowRight)],
        ])
    }
}

/// Every key and pad button a binding may name (Bevy's names, as `{:?}` prints them).
const KEYS: &[KeyCode] = {
    use KeyCode::*;
    &[
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS,
        KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7,
        Digit8, Digit9, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
        NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide, NumpadDecimal, NumpadEnter, ArrowUp, ArrowDown,
        ArrowLeft, ArrowRight, Space, Enter, Tab, Backspace, Insert, Delete, Home, End, PageUp, PageDown, ShiftLeft,
        ShiftRight, ControlLeft, ControlRight, AltLeft, AltRight, Comma, Period, Slash, Backslash, Semicolon, Quote,
        BracketLeft, BracketRight, Minus, Equal, Backquote, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    ]
};
const BUTTONS: &[GamepadButton] = {
    use GamepadButton::*;
    &[
        South, East, North, West, C, Z, LeftTrigger, LeftTrigger2, RightTrigger, RightTrigger2, Select, Start, Mode,
        LeftThumb, RightThumb, DPadUp, DPadDown, DPadLeft, DPadRight,
    ]
};

impl Input {
    fn parse(name: &str) -> Option<Input> {
        let eq = |a: String| a.eq_ignore_ascii_case(name);
        if let Some(b) = name.strip_prefix("Pad") {
            return BUTTONS.iter().find(|x| format!("{x:?}").eq_ignore_ascii_case(b)).map(|&x| Input::Pad(x));
        }
        KEYS.iter()
            .find(|k| {
                let s = format!("{k:?}");
                let short = s.strip_prefix("Key").or(s.strip_prefix("Digit")).map(str::to_string);
                eq(s.clone()) || short.is_some_and(eq)
            })
            .map(|&k| Input::Key(k))
    }

    fn name(self) -> String {
        match self {
            Input::Key(k) => format!("{k:?}"),
            Input::Pad(b) => format!("Pad{b:?}"),
        }
    }
}

impl Bindings {
    /// Read `text` over the defaults; returns the bindings and a message per name that wasn't understood.
    pub fn parse(text: &str) -> (Bindings, Vec<String>) {
        let mut b = Bindings::default();
        let mut errors = Vec::new();
        for line in text.lines().map(|l| l.split('#').next().unwrap_or("").trim()).filter(|l| !l.is_empty()) {
            let Some((action, names)) = line.split_once('=') else {
                errors.push(format!("not `action = keys`: {line}"));
                continue;
            };
            let Some(i) = ACTIONS.iter().position(|(_, n)| *n == action.trim()) else {
                errors.push(format!("unknown action: {}", action.trim()));
                continue;
            };
            let set: Vec<Input> = names
                .split_whitespace()
                .filter_map(|n| Input::parse(n).or_else(|| {
                    errors.push(format!("unknown key or button: {n}"));
                    None
                }))
                .collect();
            if !set.is_empty() {
                b.0[i] = set;
            }
        }
        (b, errors)
    }

    /// The file text for these bindings.
    pub fn text(&self) -> String {
        let mut s = String::from(
            "# Hot Shots Tennis remaster controls. One line per action: action = Key Key PadButton …\n\
             # Keys by Bevy KeyCode name (J or KeyJ, 1 or Digit1, Space, ArrowLeft, ShiftLeft, …),\n\
             # pad buttons with a Pad prefix (PadSouth = ✕/A, PadEast = ○/B, PadNorth = △/Y, PadWest, PadLeftTrigger,\n\
             # PadRightTrigger2, PadSelect, PadDPadUp, …). The left stick always moves; the right stick turns the free\n\
             # camera; Esc/Start pause.\n",
        );
        for (i, (_, name)) in ACTIONS.iter().enumerate() {
            let names: Vec<_> = self.0[i].iter().map(|x| x.name()).collect();
            s += &format!("{name} = {}\n", names.join(" "));
        }
        s
    }

    fn of(&self, a: Action) -> &[Input] {
        &self.0[ACTIONS.iter().position(|(x, _)| *x == a).unwrap()]
    }

    pub fn key_pressed(&self, keys: &ButtonInput<KeyCode>, a: Action) -> bool {
        self.of(a).iter().any(|x| matches!(x, Input::Key(k) if keys.pressed(*k)))
    }

    pub fn key_just_pressed(&self, keys: &ButtonInput<KeyCode>, a: Action) -> bool {
        self.of(a).iter().any(|x| matches!(x, Input::Key(k) if keys.just_pressed(*k)))
    }

    pub fn pad_pressed(&self, g: &Gamepad, a: Action) -> bool {
        self.of(a).iter().any(|x| matches!(x, Input::Pad(b) if g.pressed(*b)))
    }

    pub fn pad_just_pressed(&self, g: &Gamepad, a: Action) -> bool {
        self.of(a).iter().any(|x| matches!(x, Input::Pad(b) if g.just_pressed(*b)))
    }

    /// `J/PadSouth`-style list of an action's bindings (keys only unless `pads`), for the help line.
    pub fn names(&self, a: Action, pads: bool) -> String {
        let n: Vec<_> = self.of(a)
            .iter()
            .filter(|x| pads || matches!(x, Input::Key(_)))
            .map(|x| x.name().trim_start_matches("Key").to_string())
            .collect();
        n.join("/")
    }
}

/// `controls.txt` beside the disc image.
fn path(iso: &str) -> PathBuf {
    std::path::Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("controls.txt")
}

fn load(mut commands: Commands, args: Res<crate::Args>) {
    let path = path(&args.iso);
    let b = match std::fs::read_to_string(&path) {
        Ok(text) => {
            let (b, errors) = Bindings::parse(&text);
            for e in errors {
                warn!("{}: {e}", path.display());
            }
            b
        }
        Err(_) => {
            let b = Bindings::default();
            if std::fs::write(&path, b.text()).is_ok() {
                info!("wrote default controls to {}", path.display());
            }
            b
        }
    };
    commands.insert_resource(b);
}

/// Controller slots 1 and 2: the gamepad entity holding each. A slot is kept while its pad stays connected.
#[derive(Resource, Default)]
pub struct PadSlots(pub [Option<Entity>; 2]);

impl PadSlots {
    /// Drop pads gone from `present`, then give each new one the first free slot (in connection order).
    pub fn update(&mut self, present: &[Entity]) {
        for s in &mut self.0 {
            if s.is_some_and(|e| !present.contains(&e)) {
                *s = None;
            }
        }
        for &e in present {
            if !self.0.contains(&Some(e))
                && let Some(free) = self.0.iter_mut().find(|s| s.is_none())
            {
                *free = Some(e);
            }
        }
    }

    /// Slots in use, as `Pads::connected` counts them: slot 2 counts while its pad is in, even with slot 1 empty.
    pub fn connected(&self) -> usize {
        self.0.iter().rposition(Option::is_some).map_or(0, |i| i + 1)
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Bindings>().init_resource::<PadSlots>().add_systems(Startup, load);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_round_trip_and_parse() {
        let d = Bindings::default();
        assert_eq!(Bindings::parse(&d.text()), (d.clone(), vec![]));
        let (b, errors) = Bindings::parse("topspin = Z 1 PadWest # comment\nlob = nope\nbogus = J\n");
        assert_eq!(b.of(Action::Normal), [Input::Key(KeyCode::KeyZ), Input::Key(KeyCode::Digit1), Input::Pad(GamepadButton::West)]);
        assert_eq!(b.of(Action::Lob), d.of(Action::Lob)); // nothing understood: default kept
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn pad_slots_survive_unplug() {
        let (a, b, c) = (Entity::from_raw_u32(1).unwrap(), Entity::from_raw_u32(2).unwrap(), Entity::from_raw_u32(3).unwrap());
        let mut s = PadSlots::default();
        s.update(&[a, b]);
        assert_eq!(s.0, [Some(a), Some(b)]);
        s.update(&[b]); // pad 1 unplugged: pad 2 stays slot 2
        assert_eq!((s.0, s.connected()), ([None, Some(b)], 2));
        s.update(&[b, c]); // a new pad fills slot 1
        assert_eq!(s.0, [Some(c), Some(b)]);
        s.update(&[c]);
        assert_eq!((s.0, s.connected()), ([Some(c), None], 1));
    }
}
