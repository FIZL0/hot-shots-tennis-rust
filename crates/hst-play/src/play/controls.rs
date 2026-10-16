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
    /// □: asks for the post-point's instant replay (`instant_replay`). There's no serve action: the shot buttons serve,
    /// as the original (B119; an older file's `serve = …` line is ignored).
    Replay,
    /// Select: swap the character's hand at character select (`camera`, its old name, kept for existing files).
    Camera,
    /// Emotes (`play::emote`), as the right stick flicked up, down, left, right.
    EmoteUp,
    EmoteDown,
    EmoteLeft,
    EmoteRight,
}

const ACTIONS: [(Action, &str); 13] = [
    (Action::Up, "up"),
    (Action::Down, "down"),
    (Action::Left, "left"),
    (Action::Right, "right"),
    (Action::Normal, "topspin"),
    (Action::Cut, "slice"),
    (Action::Lob, "lob"),
    (Action::Replay, "replay"),
    (Action::Camera, "camera"),
    (Action::EmoteUp, "emote_up"),
    (Action::EmoteDown, "emote_down"),
    (Action::EmoteLeft, "emote_left"),
    (Action::EmoteRight, "emote_right"),
];

/// One binding: a key or a pad button.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Input {
    Key(KeyCode),
    Pad(GamepadButton),
}

#[derive(Resource, Clone, PartialEq, Debug)]
pub struct Bindings([Vec<Input>; 13]);

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
            vec![K(C::Space), Pad(P::West)],
            vec![K(C::KeyC), Pad(P::Select)],
            vec![K(C::Digit1)],
            vec![K(C::Digit2)],
            vec![K(C::Digit3)],
            vec![K(C::Digit4)],
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
                // the free camera's turn keys (gone, B218) and the serve key (gone, B119) in an older file
                if matches!(action.trim(), "turn_left" | "turn_right" | "serve") {
                    continue;
                }
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
             # PadRightTrigger2, PadSelect, PadDPadUp, …). The left stick always moves; the right stick flicked\n\
             # fully up/down/left/right plays the emotes; Esc/Start pause; camera swaps the hand at character select.\n",
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
}

/// Every action's name in `controls.txt`, in order (the rebinding screen's rows).
pub fn action_names() -> impl Iterator<Item = &'static str> {
    ACTIONS.iter().map(|(_, n)| *n)
}

impl Bindings {
    /// Action `i`'s bindings by name, keys first (`KeyJ`, `PadSouth`).
    /// M14: action `a`'s first bound key as a keycap shows it ("J", "1", "Space", "Left"), if it has one.
    pub fn key(&self, a: Action) -> Option<String> {
        self.of(a).iter().find_map(|x| match x {
            Input::Key(k) => {
                let n = format!("{k:?}");
                Some(["Key", "Digit", "Arrow", "Numpad"].iter().find_map(|p| n.strip_prefix(p)).unwrap_or(&n).to_string())
            }
            Input::Pad(_) => None,
        })
    }

    pub fn names(&self, i: usize) -> Vec<String> {
        let mut v: Vec<_> = self.0[i].iter().map(|x| (matches!(x, Input::Pad(_)), x.name())).collect();
        v.sort_by_key(|(pad, _)| *pad);
        v.into_iter().map(|(_, n)| n).collect()
    }

    /// Rebind action `i` to key `k`: it replaces the action's keys, its pad buttons stay.
    pub fn rebind_key(&mut self, i: usize, k: KeyCode) {
        self.0[i].retain(|x| matches!(x, Input::Pad(_)));
        self.0[i].insert(0, Input::Key(k));
    }

    /// Rebind action `i` to pad button `b`: it replaces the action's pad buttons, its keys stay.
    pub fn rebind_pad(&mut self, i: usize, b: GamepadButton) {
        self.0[i].retain(|x| matches!(x, Input::Key(_)));
        self.0[i].push(Input::Pad(b));
    }

    /// B291: action `i`'s bound keys, in order.
    pub fn keys_of(&self, i: usize) -> Vec<KeyCode> {
        self.0[i].iter().filter_map(|x| if let Input::Key(k) = x { Some(*k) } else { None }).collect()
    }

    /// B291: action `i`'s bound pad buttons, in order.
    pub fn pads_of(&self, i: usize) -> Vec<GamepadButton> {
        self.0[i].iter().filter_map(|x| if let Input::Pad(b) = x { Some(*b) } else { None }).collect()
    }

    /// Whether `k` / `b` can be bound (one of the names `controls.txt` knows).
    pub fn bindable_key(k: KeyCode) -> bool {
        KEYS.contains(&k)
    }
    pub fn bindable_pad(b: GamepadButton) -> bool {
        BUTTONS.contains(&b)
    }

    /// Write these bindings to `controls.txt` beside `iso`.
    pub fn save(&self, iso: &str) -> std::io::Result<()> {
        crate::store::write(path(iso), self.text())
    }
}

/// B291: every pad button a binding may name, in `controls.txt`'s order (the legend bakes an icon for each).
pub fn pad_buttons() -> &'static [GamepadButton] {
    BUTTONS
}

/// B291: the stick that always does action `i`, unbindable: the left stick moves (up, down, left, right), the right
/// stick flicked plays the emotes. `(right stick, direction 0 up 1 down 2 left 3 right)`.
pub fn fixed_stick(i: usize) -> Option<(bool, usize)> {
    match ACTIONS.get(i)?.0 {
        Action::Up => Some((false, 0)),
        Action::Down => Some((false, 1)),
        Action::Left => Some((false, 2)),
        Action::Right => Some((false, 3)),
        Action::EmoteUp => Some((true, 0)),
        Action::EmoteDown => Some((true, 1)),
        Action::EmoteLeft => Some((true, 2)),
        Action::EmoteRight => Some((true, 3)),
        _ => None,
    }
}

/// `controls.txt` beside the disc image.
fn path(iso: &str) -> PathBuf {
    std::path::Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("controls.txt")
}

fn load(mut commands: Commands, args: Res<crate::Args>, pads: Option<ResMut<super::Pads>>) {
    let path = path(&args.iso);
    let b = match crate::store::read_to_string(&path) {
        Ok(text) => {
            let (b, errors) = Bindings::parse(&text);
            for e in errors {
                warn!("{}: {e}", path.display());
            }
            b
        }
        Err(_) => {
            let b = Bindings::default();
            if crate::store::write(&path, b.text()).is_ok() {
                info!("wrote default controls to {}", path.display());
            }
            b
        }
    };
    commands.insert_resource(b);
    if let Some(seats) = args.pads {
        commands.insert_resource(PadSlots([None; 4], Some(seats)));
        if let Some(mut p) = pads {
            p.seats = Some(seats.map(|s| s.is_some()));
        }
    }
}

// What drives a seat and which pads are read: in hst-base (`Args` and the chat use them)
pub use hst_base::seats::{Seat, readable};

/// Controller slots 1P..4P: the gamepad entity holding each. A slot is kept while its pad stays connected.
/// With seats (`--pads`) slot `s` first takes pad `n` of seat `Pad(n)` (an index into the sorted pad list), then any
/// free pad if that one is gone; a computer seat never takes a pad, a keyboard seat only in the browser (`update`).
#[derive(Resource, Default)]
pub struct PadSlots(pub [Option<Entity>; 4], pub Option<[Option<Seat>; 4]>);

impl PadSlots {
    /// Drop pads gone from `present`, then give each new one the first free slot (in connection order). With seats a
    /// pad seat prefers its own pad and the others stay empty.
    pub fn update(&mut self, present: &[Entity]) {
        self.update_with(present, &[]);
    }

    /// `update`; `touch`: the on-screen pads among `present` (`emote::is_touch`), which the keyboard seat also takes
    /// when no other seat wants them. B244: in the browser a phone's menus are tapped (the keyboard's seat), so the
    /// touch pad (web/touch.js; the iPhone app's `touch_pad.rs`) must drive that seat in the match. B516: only those:
    /// a real pad no seat holds (idle, or never pressed in the menus) never drives the keyboard's player.
    pub fn update_with(&mut self, present: &[Entity], touch: &[Entity]) {
        for s in &mut self.0 {
            if s.is_some_and(|e| !present.contains(&e)) {
                *s = None;
            }
        }
        let wants = |i: usize| self.1.map_or(Some(None), |seats| match seats[i] {
            Some(Seat::Pad(n)) => Some(Some(n)),
            Some(Seat::Keys) if !touch.is_empty() => Some(None),
            _ => None,
        });
        // seats' own pads first, so a seat whose pad is gone doesn't take one another empty seat is waiting for
        for i in 0..4 {
            if let (None, Some(Some(n))) = (self.0[i], wants(i))
                && let Some(&e) = present.get(n).filter(|e| !self.0.contains(&Some(**e)))
            {
                self.0[i] = Some(e);
            }
        }
        for i in 0..4 {
            if self.0[i].is_none() && wants(i).is_some() {
                let own: Vec<_> = (0..4).filter(|&j| j != i && self.0[j].is_none()).filter_map(|j| wants(j).flatten()).filter_map(|n| present.get(n)).collect();
                let keys = self.1.is_some_and(|seats| seats[i] == Some(Seat::Keys));
                let free = present.iter().find(|e| !self.0.contains(&Some(**e)) && (self.1.is_none() || !own.contains(e)) && (!keys || touch.contains(e)));
                self.0[i] = free.copied();
            }
        }
    }

    /// Slots in use, as `Pads::connected` counts them: slot 2 counts while its pad is in, even with slot 1 empty.
    pub fn connected(&self) -> usize {
        self.0.iter().rposition(Option::is_some).map_or(0, |i| i + 1)
    }

    /// The slot the keyboard drives: 1P without seats (alongside 1P's pad), else its own seat, if it has one; B388:
    /// with none, 1P while 1P's pad is out (unplugged or flat), so the keyboard can play on or pause and reassign.
    pub fn keys(&self) -> Option<usize> {
        self.1.map_or(Some(0), |seats| {
            seats.iter().position(|s| *s == Some(Seat::Keys)).or((matches!(seats[0], Some(Seat::Pad(_))) && self.0[0].is_none()).then_some(0))
        })
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Bindings>().init_resource::<PadSlots>().add_systems(Startup, load);
    app.add_systems(PreUpdate, no_key_repeat.after(bevy::input::InputSystems));
}

/// B95: a held key presses once, like a held pad button. The OS's key repeat is dropped: Bevy ignores a repeat while
/// the key is down, but one arriving after a release it missed (focus lost and back while held) would press again. The
/// menus' held-direction repeat is their own timer, the same for the pad and the keyboard.
fn no_key_repeat(mut keys: ResMut<ButtonInput<KeyCode>>, mut events: MessageReader<bevy::input::keyboard::KeyboardInput>) {
    for e in events.read().filter(|e| e.repeat && e.state.is_pressed()) {
        keys.clear_just_pressed(e.key_code);
    }
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
        let mut r = d.clone();
        r.rebind_key(4, KeyCode::KeyZ);
        r.rebind_pad(4, GamepadButton::West);
        assert_eq!(r.names(4), ["KeyZ", "PadWest"]);
        assert_eq!(Bindings::parse(&r.text()).0, r);
    }

    #[test]
    fn seated_pads() {
        let [a, b, c] = [5, 7, 8].map(|n| Entity::from_raw_u32(n).unwrap());
        let (k, p) = (Some(Seat::Keys), |n| Some(Seat::Pad(n)));
        // 1P keyboard, 2P pad 1: slot 1 stays empty, slot 2 takes pad 1
        let mut s = PadSlots([None; 4], Some([k, p(1), None, None]));
        s.update(&[a, b]);
        assert_eq!((s.0, s.keys()), ([None, Some(b), None, None], Some(0)));
        // 2P the computer's: a second pad never makes it human
        let mut s = PadSlots([None; 4], Some([p(0), None, None, None]));
        s.update(&[a, b]);
        assert_eq!((s.0, s.connected(), s.keys()), ([Some(a), None, None, None], 1, None));
        // four humans, the keyboard as 3P; pad 0 gone: its seat waits instead of taking pad 2's (or 4P's pad 1)
        let mut s = PadSlots([None; 4], Some([p(0), p(2), k, p(1)]));
        s.update(&[a, b, c]);
        assert_eq!((s.0, s.keys()), ([Some(a), Some(c), None, Some(b)], Some(2)));
        s.update(&[b, c]);
        assert_eq!(s.0, [None, Some(c), None, Some(b)]);
        // a pad plugged back in fills it
        s.update(&[b, c, a]);
        assert_eq!(s.0, [Some(a), Some(c), None, Some(b)]);
        // B244, the browser: the keyboard's seat takes a pad no seat wants (the touch pad), never another seat's
        let mut s = PadSlots([None; 4], Some([k, p(1), None, None]));
        s.update_with(&[a, b], &[a]);
        assert_eq!((s.0, s.keys()), ([Some(a), Some(b), None, None], Some(0)));
        let mut s = PadSlots([None; 4], Some([k, p(0), None, None]));
        s.update_with(&[a], &[a]);
        assert_eq!(s.0, [None, Some(a), None, None]);
        // B516: a real pad no seat holds stays off the keyboard's seat (it took over a keyboard player online)
        let mut s = PadSlots([None; 4], Some([k, None, None, None]));
        s.update_with(&[a, b], &[b]);
        assert_eq!((s.0, s.keys()), ([Some(b), None, None, None], Some(0)));
        s.update_with(&[a], &[]);
        assert_eq!((s.0, s.keys()), ([None; 4], Some(0)));
        // no seats: connection order, the keyboard with 1P
        let mut s = PadSlots::default();
        s.update(&[a, b, c]);
        assert_eq!((s.0, s.keys()), ([Some(a), Some(b), Some(c), None], Some(0)));
        // B388: 1P's pad out and no keyboard seat: the keyboard drives 1P until a pad fills it again
        let mut s = PadSlots([None; 4], Some([p(0), p(1), None, None]));
        s.update(&[a, b]);
        assert_eq!(s.keys(), None);
        s.update(&[b]);
        assert_eq!((s.0, s.keys()), ([None, Some(b), None, None], Some(0)));
        s.update(&[a, b]);
        assert_eq!(s.keys(), None);
        // 2P's pad out instead: the keyboard stays off (it's 1P's stand-in only)
        s.update(&[a]);
        assert_eq!(s.keys(), None);
        assert_eq!(Seat::flag(&[k, None, p(1)]), "k,-,1");
        assert_eq!(["k", "-", "2", ""].map(Seat::parse), [k, None, p(2), None]);
    }

    #[test]
    fn pad_slots_survive_unplug() {
        let (a, b, c) = (Entity::from_raw_u32(1).unwrap(), Entity::from_raw_u32(2).unwrap(), Entity::from_raw_u32(3).unwrap());
        let mut s = PadSlots::default();
        s.update(&[a, b]);
        assert_eq!(s.0, [Some(a), Some(b), None, None]);
        s.update(&[b]); // pad 1 unplugged: pad 2 stays slot 2
        assert_eq!((s.0, s.connected()), ([None, Some(b), None, None], 2));
        s.update(&[b, c]); // a new pad fills slot 1
        assert_eq!(s.0, [Some(c), Some(b), None, None]);
        s.update(&[c]);
        assert_eq!((s.0, s.connected()), ([Some(c), None, None, None], 1));
    }

    #[test]
    fn held_key_presses_once() {
        use bevy::input::{ButtonState, InputPlugin, keyboard::{Key, KeyboardFocusLost, KeyboardInput}};
        let mut app = App::new();
        app.add_plugins(InputPlugin).add_systems(PreUpdate, no_key_repeat.after(bevy::input::InputSystems));
        let key = |app: &mut App, repeat| {
            let e = KeyboardInput { key_code: KeyCode::KeyJ, logical_key: Key::Character("j".into()), state: ButtonState::Pressed, text: None, repeat, window: Entity::PLACEHOLDER };
            app.world_mut().write_message(e);
            app.update();
            app.world().resource::<ButtonInput<KeyCode>>().just_pressed(KeyCode::KeyJ)
        };
        assert!(key(&mut app, false));
        assert!(!key(&mut app, true));
        // focus lost while held releases it; the OS repeat coming back mustn't press again
        app.world_mut().write_message(KeyboardFocusLost);
        app.update();
        assert!(!key(&mut app, true));
        assert!(app.world().resource::<ButtonInput<KeyCode>>().pressed(KeyCode::KeyJ));
        assert!(!key(&mut app, true));
    }

    /// B119: no serve binding. ✕ (keyboard J, pad South) latches the topspin press the serve tosses on (press 0 →
    /// `Toss::Strong`), never the □ replay request; an older file's `serve = …` line loads without an error.
    #[test]
    fn cross_serves_on_keyboard_and_pad() {
        let (b, errors) = Bindings::parse("serve = Space\ntopspin = J PadSouth\n");
        assert_eq!((b, errors), (Bindings::default(), vec![]));
        assert!(!action_names().any(|n| n == "serve"));
        let slot = |key: Option<KeyCode>, pad: Option<GamepadButton>| {
            let mut w = World::new();
            let mut keys = ButtonInput::<KeyCode>::default();
            key.map(|k| keys.press(k));
            w.insert_resource(keys);
            w.insert_resource(super::super::Pads::default());
            w.init_resource::<Bindings>();
            w.init_resource::<PadSlots>();
            if let Some(p) = pad {
                let mut g = Gamepad::default();
                g.digital_mut().press(p);
                w.spawn(g);
            }
            w.run_system_cached(super::super::read_input).unwrap();
            let s = w.resource::<super::super::Pads>().slots[0];
            (s.shot, s.serve)
        };
        assert_eq!(slot(Some(KeyCode::KeyJ), None), (Some(0), false));
        assert_eq!(slot(None, Some(GamepadButton::South)), (Some(0), false));
        assert_eq!(slot(Some(KeyCode::Space), None), (None, true), "Space is □, the replay request");
    }
}
