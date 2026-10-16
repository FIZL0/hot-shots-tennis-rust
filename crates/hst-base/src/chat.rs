//! M20: text chat in online lobbies and matches (ours, layered on top). A line goes over the peer connection as
//! `net::Msg::Chat` through the host, which cleans it, rate-limits it and passes it on with the sender's lobby name;
//! it never enters the match's input, so the sim, its generators and the replays never see it.
//!
//! The box: T opens it on the keyboard (typed keys go to it, not to the menu or the match), Enter sends, Esc
//! closes; in a match a pad's R2 opens the preset phrases and steps through them, L2 says the one shown (they close by
//! themselves after a few seconds). B512: not in the menus (R2/L2 are the select's there): the lobby's Chat button
//! opens (and closes) the typed box on a pad or a touch screen too, and the menus' legend strip shows only T as Chat. On a touch screen a
//! speech-bubble button on the on-screen pad opens the box (the phone's keyboard): beside Start in the browser
//! (`web/touch.js`, B312), beside Select in the phone apps (`play/touch_pad.rs`, [`BUTTON`]). B531: one tap brings the box
//! and the keyboard up together (the page focuses its input in the tap's own lift; the apps' system keyboard follows
//! [`OPEN`]), and the button stays up wherever chat is on, the box open too: a tap then brings the keyboard back. The chat itself takes no
//! clicks or taps: nothing of it shows until a line is said, then the last few show at the bottom left, fading out,
//! over the menus and the HUD alike.
//! `/mute NAME` and `/unmute NAME` (or the phrases' Mute/Unmute rows) hide a player's lines on this machine. Settings
//! → Online & Mods → Chat off hides all of it.
//!
//! `HST_CHAT=lines|open|phrase` (for `--shot`s): chat shown outside a room, a few lines held at fixed ages (one
//! fading), with the box open on a draft / the pads' phrase up.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use n0_future::time::{Duration, Instant};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::net::{Cmd, Net};

/// A line's most characters.
pub const MAX: usize = 80;
/// At most this many lines per peer in any [`WINDOW`].
pub const BURST: usize = 4;
const WINDOW: Duration = Duration::from_secs(8);
/// How long a line stays up (seconds; its last [`FADE`] fading), and how many show.
const LIFE: f32 = 12.0;
const FADE: f32 = 2.0;
const SHOWN: usize = 6;
/// The pads' phrases close after this long with no R2/L2 (seconds).
const PICK: f32 = 4.0;
pub const PHRASES: [&str; 8] = ["Good luck!", "Nice shot!", "Oops!", "Well played!", "Good game!", "Thanks!", "Ready?", "Rematch?"];

/// Settings → Chat off (`main_menu::Settings`).
pub static HIDDEN: AtomicBool = AtomicBool::new(false);

/// B396: the lobby's Chat button asks for the box (1: the typed line, a second ask closes it) or the pads' phrases (2);
/// B531: the on-screen pad's chat button asks with 3 (opens the box, or brings the phone's keyboard back while it's open).
pub static ASK: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// B512: the menus showed this frame (`main_menu::online::poll` sets it, [`input`] takes it): no quick-chat phrases.
pub static MENUS: AtomicBool = AtomicBool::new(false);

/// The on-screen pad's chat button is up: wherever chat is on, the box open or not (B531); a tap on it stores 3 in
/// [`ASK`].
pub static BUTTON: AtomicBool = AtomicBool::new(false);

/// B531: the typed box is open: the phone apps' system keyboard is up while it is (`main_menu::osk`).
pub static OPEN: AtomicBool = AtomicBool::new(false);

/// B531: the chat button was tapped with the box already open: the phone apps' system keyboard comes back up (put
/// away meanwhile); `main_menu::osk` takes it.
pub static RAISE: AtomicBool = AtomicBool::new(false);

/// B517: played by touch with no controller (`osk::mobile`): while the box is open it sits at the top of the screen,
/// over the device's keyboard.
pub static TOUCH: AtomicBool = AtomicBool::new(false);

/// B449: the last two lines heard (who, words; a note's who is empty), kept after they fade, for the lobby's chat box.
pub static RECENT: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());

/// B449: the lobby drew its chat box this frame: the lines show there, not at the bottom left (the box and the phrase
/// still do, while typing).
pub static BOXED: AtomicBool = AtomicBool::new(false);

/// `text` as a chat line: control and bidi-override characters out, spaces trimmed, at most [`MAX`] characters;
/// `None` if nothing is left.
pub fn clean(text: &str) -> Option<String> {
    let bidi = |c: char| matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}');
    let s: String = text.chars().map(|c| if c.is_whitespace() { ' ' } else { c }).filter(|&c| !c.is_control() && !bidi(c)).collect();
    let s: String = s.trim().chars().take(MAX).collect();
    (!s.is_empty()).then_some(s)
}

/// Whether one more line may go now, `sent` being that peer's recent lines' times (kept to the window; `now` added
/// when it may).
pub fn allow(sent: &mut Vec<Instant>, now: Instant) -> bool {
    sent.retain(|t| now.saturating_duration_since(*t) < WINDOW);
    let ok = sent.len() < BURST;
    if ok {
        sent.push(now);
    }
    ok
}

struct Line {
    name: String,
    text: String,
    age: f32,
}

#[derive(Resource, Default)]
struct Chat {
    lines: Vec<Line>,
    /// The keyboard's box is open, with what's typed so far.
    open: bool,
    draft: String,
    /// The pads' phrase shown, and how long until it closes.
    phrase: Option<(usize, f32)>,
    /// Who has said something (peer → name), and the peers muted.
    names: BTreeMap<String, String>,
    muted: BTreeSet<String>,
    sent: Vec<Instant>,
}

impl Chat {
    fn note(&mut self, text: &str) {
        self.lines.push(Line { name: String::new(), text: text.into(), age: 0.0 });
    }

    /// The phrases, then Mute/Unmute for each player heard.
    fn phrases(&self) -> Vec<String> {
        let mute = self.names.iter().map(|(p, n)| format!("{} {n}", if self.muted.contains(p) { "Unmute" } else { "Mute" }));
        PHRASES.iter().map(|p| p.to_string()).chain(mute).collect()
    }

    /// Mute (or unmute) peer `peer`.
    fn mute(&mut self, peer: &str, on: bool) {
        let name = self.names.get(peer).cloned().unwrap_or_default();
        if on { self.muted.insert(peer.into()) } else { self.muted.remove(peer) };
        self.note(&format!("{name} {}.", if on { "muted" } else { "unmuted" }));
    }

    /// Send `text`, or carry out a `/mute NAME` or `/unmute NAME`.
    fn say(&mut self, net: &Net, text: &str) {
        let Some(text) = clean(text) else { return };
        let lower = text.to_lowercase();
        let cmd = lower.strip_prefix("/mute ").map(|n| (n, true)).or(lower.strip_prefix("/unmute ").map(|n| (n, false)));
        if let Some((who, on)) = cmd {
            let who = who.trim();
            return match self.names.iter().find(|(_, n)| n.to_lowercase() == who).map(|(p, _)| p.clone()) {
                Some(peer) => self.mute(&peer, on),
                None => self.note(&format!("No one called {who} has chatted.")),
            };
        }
        if !allow(&mut self.sent, Instant::now()) {
            return self.note("Slow down: a few lines every few seconds.");
        }
        net.send(Cmd::Chat(text.clone()));
        self.lines.push(Line { name: "You".into(), text, age: 0.0 });
    }
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Row(usize);

pub fn plugin(app: &mut App) {
    app.init_resource::<Chat>()
        .add_systems(Startup, sample)
        .add_systems(PreUpdate, input.after(bevy::input::InputSystems))
        .add_systems(PostUpdate, draw);
}

/// `HST_CHAT` set (a shot of the chat).
fn sampled() -> bool {
    std::env::var_os("HST_CHAT").is_some()
}

/// `HST_CHAT`'s made-up lines and box.
fn sample(mut chat: ResMut<Chat>) {
    let Ok(how) = std::env::var("HST_CHAT") else { return };
    let line = |name: &str, text: &str, age| Line { name: name.into(), text: text.into(), age };
    chat.lines = vec![
        line("Sunny Day", "gl hf!", LIFE - FADE / 3.0),
        line("ACE-42", "Picking Kaoru, give me a sec", 6.0),
        line("You", "Good luck!", 4.0),
        line("", "ACE-42 muted.", 2.0),
        line("Sunny Day", "Rematch after this one?", 0.0),
    ];
    chat.open = how == "open";
    chat.draft = if chat.open { "sure, best of three".into() } else { String::new() };
    chat.phrase = (how == "phrase").then_some((3, PICK));
}

/// The lines heard, and this machine's typing: while the box is open the keyboard's keys are the box's alone.
#[allow(clippy::too_many_arguments)]
fn input(
    net: Option<Res<Net>>,
    mut chat: ResMut<Chat>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    pads: Query<&Gamepad>,
    time: Res<Time<Real>>,
    mut was: Local<bool>,
) {
    if sampled() {
        return;
    }
    let dt = time.delta_secs();
    chat.lines.iter_mut().for_each(|l| l.age += dt);
    chat.lines.retain(|l| l.age < LIFE);
    let on = !HIDDEN.load(Ordering::Relaxed) && net.as_ref().is_some_and(|n| n.in_room());
    let Some(net) = net else {
        return phone(&mut was, false);
    };
    if !on {
        (chat.open, chat.phrase) = (false, None);
        chat.draft.clear();
        net.chats();
        typed.clear();
        return phone(&mut was, false);
    }
    for (from, name, text) in net.chats() {
        let name = clean(&name).unwrap_or_default();
        chat.names.insert(from.clone(), name.clone());
        if !chat.muted.contains(&from) {
            chat.lines.push(Line { name, text, age: 0.0 });
        }
    }
    let ask = ASK.swap(0, Ordering::Relaxed);
    let menus = MENUS.swap(false, Ordering::Relaxed);
    #[cfg(target_arch = "wasm32")]
    let tapped = crate::web::chat_tapped() && !chat.open;
    #[cfg(not(target_arch = "wasm32"))]
    let tapped = false;
    if chat.open && ask == 3 {
        // B531: the pad's chat button with the box open: the keyboard again, the draft kept
        RAISE.store(true, Ordering::Relaxed);
    } else if !chat.open && (keys.just_pressed(KeyCode::KeyT) || tapped || ask == 1 || ask == 3) {
        chat.open = true;
        typed.clear();
        keys.reset_all();
    } else if chat.open && ask == 1 {
        // B512: the lobby's Chat button again (a pad or a tap can't press Esc)
        chat.open = false;
        chat.draft.clear();
        typed.clear();
    } else if chat.open {
        let mut send = false;
        for e in typed.read().filter(|e| e.state.is_pressed()) {
            match &e.logical_key {
                Key::Enter => send = true,
                // B531: the iPhone's system keyboard's Return comes as a character
                Key::Character(c) if c.contains(['\n', '\r']) => send = true,
                Key::Escape => chat.open = false,
                Key::Backspace => _ = chat.draft.pop(),
                Key::Character(c) if chat.draft.chars().count() < MAX => chat.draft.push_str(c),
                Key::Space if chat.draft.chars().count() < MAX => chat.draft.push(' '),
                _ => {}
            }
        }
        #[cfg(target_arch = "wasm32")]
        for c in crate::web::typed().chars() {
            match c {
                '\u{8}' => _ = chat.draft.pop(),
                '\n' => send = true,
                c if chat.draft.chars().count() < MAX => chat.draft.push(c),
                _ => {}
            }
        }
        if send {
            let text = std::mem::take(&mut chat.draft);
            chat.say(&net, &text);
            chat.open = false;
        }
        keys.reset_all();
    } else {
        typed.clear();
    }
    let open = chat.open;
    phone(&mut was, open);
    // the pads: R2 shows / steps the phrases, L2 says the one shown (in a match only, B512)
    let pads: Vec<_> = pads.iter().filter(|g| crate::seats::readable(g) && !menus).collect();
    let next = pads.iter().any(|g| g.just_pressed(GamepadButton::RightTrigger2)) || ask == 2;
    let say = pads.iter().any(|g| g.just_pressed(GamepadButton::LeftTrigger2));
    let n = chat.phrases().len();
    chat.phrase = match chat.phrase {
        _ if menus => None,
        _ if next => Some((chat.phrase.map_or(0, |(i, _)| (i + 1) % n), PICK)),
        Some((i, _)) if say => {
            match chat.names.keys().nth(i.wrapping_sub(PHRASES.len())).cloned() {
                Some(peer) => {
                    let on = !chat.muted.contains(&peer);
                    chat.mute(&peer, on);
                }
                None => chat.say(&net, PHRASES[i]),
            }
            None
        }
        Some((i, left)) => (left > dt).then_some((i, left - dt)),
        None => None,
    };
}

/// The phone's keyboard (the browser's code-entry input; the apps' system keyboard, [`OPEN`]) up while the box is
/// open on a touch screen.
fn phone(was: &mut bool, on: bool) {
    OPEN.store(on, Ordering::Relaxed);
    if std::mem::replace(was, on) != on {
        #[cfg(target_arch = "wasm32")]
        crate::web::code_entry(on, false);
    }
}

/// The lines (fading), and the box or the phrase; sized on the 4:3 screen's height as the FPS counter
/// (`graphics::fps_counter`), so they scale uniformly at any window shape.
fn draw(
    mut commands: Commands,
    net: Option<Res<Net>>,
    chat: Res<Chat>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    root: Query<(Entity, &mut Visibility, &mut Node), (With<Root>, Without<Row>)>,
    mut rows: Query<(&Row, &mut Text, &mut TextFont, &mut TextColor, &mut TextShadow, &mut Node, &mut BackgroundColor)>,
) {
    let on = !HIDDEN.load(Ordering::Relaxed) && (net.is_some_and(|n| n.in_room()) || sampled());
    let boxed = BOXED.swap(false, Ordering::Relaxed);
    if let Ok(mut recent) = RECENT.lock() {
        if !on {
            recent.clear();
        } else if let Some(l) = chat.lines.last().map(|l| (l.name.clone(), l.text.clone())) {
            // the older one may have faded already: kept until a newer line pushes it out
            if chat.lines.len() >= 2 {
                *recent = chat.lines[chat.lines.len() - 2..].iter().map(|l| (l.name.clone(), l.text.clone())).collect();
            } else if recent.last() != Some(&l) {
                recent.push(l);
                let n = recent.len().saturating_sub(2);
                recent.drain(..n);
            }
        }
    }
    let mut root = root;
    let Ok((_, mut vis, mut place)) = root.single_mut() else {
        if on {
            spawn(&mut commands);
        }
        return;
    };
    let v = if on { Visibility::Inherited } else { Visibility::Hidden };
    if *vis != v {
        *vis = v;
    }
    // B312: in the browser the on-screen pad's own chat button stands in for the label; B531: up wherever chat is on,
    // the box open or not (a tap then brings the keyboard back)
    let button = on;
    BUTTON.store(button, Ordering::Relaxed);
    #[cfg(target_arch = "wasm32")]
    crate::web::touch_chat(button);
    if !on {
        return;
    }
    // B517: typing by touch, the box at the top (under the on-screen pad's top row), clear of the device's keyboard
    let (top, bottom) = if chat.open && TOUCH.load(Ordering::Relaxed) { (Val::Percent(16.0), Val::Auto) } else { (Val::Auto, Val::Percent(18.0)) };
    if place.top != top || place.bottom != bottom {
        (place.top, place.bottom) = (top, bottom);
    }
    let tall = window.single().is_ok_and(|w| w.width() * 3.0 < w.height() * 4.0);
    let px = |n: f32| if tall { bevy::text::FontSize::Vw(n / 448.0 * 75.0) } else { bevy::text::FontSize::Vh(n / 448.0 * 100.0) };
    let val = |n: f32| if tall { Val::Vw(n / 448.0 * 75.0) } else { Val::Vh(n / 448.0 * 100.0) };
    let start = chat.lines.len().saturating_sub(SHOWN);
    let shown = &chat.lines[start..];
    let busy = chat.open || chat.phrase.is_some();
    for (row, mut text, mut font, mut colour, mut shadow, mut node, mut back) in &mut rows {
        let (s, alpha, c) = match row.0 {
            i if i < SHOWN => match shown.get(i) {
                Some(_) if boxed && !busy => (String::new(), 0.0, Color::WHITE),
                Some(l) => {
                    let a = if busy { 1.0 } else { ((LIFE - l.age) / FADE).clamp(0.0, 1.0) };
                    let s = if l.name.is_empty() { l.text.clone() } else { format!("{}: {}", l.name, l.text) };
                    (s, a, if l.name.is_empty() { Color::srgb(1.0, 0.85, 0.4) } else { Color::WHITE })
                }
                None => (String::new(), 0.0, Color::WHITE),
            },
            // the box, or the pads' phrase
            _ => match (chat.open, chat.phrase) {
                (true, _) => (format!("Say: {}_", chat.draft), 1.0, Color::srgb(0.6, 1.0, 0.6)),
                (_, Some((i, _))) => (format!("R2 next, L2 say: {}", chat.phrases()[i]), 1.0, Color::srgb(0.6, 1.0, 0.6)),
                _ => (String::new(), 0.0, Color::WHITE),
            },
        };
        if text.0 != s {
            text.0 = s;
        }
        let size = px(15.0);
        if font.font_size != size {
            font.font_size = size;
        }
        let c = c.with_alpha(alpha);
        if colour.0 != c {
            colour.0 = c;
        }
        // the shadow fades with its line: an old line leaves no dark copy behind
        let sc = TextShadow::default().color.with_alpha(TextShadow::default().color.alpha() * alpha);
        if shadow.color != sc {
            shadow.color = sc;
        }
        // B398: each line on a slight dark rounded backing, fading with it, so it reads over any menu or the HUD;
        // an empty or faded-out row takes no room
        let bc = BACKING.with_alpha(BACKING.alpha() * alpha);
        if back.0 != bc {
            back.0 = bc;
        }
        let display = if text.0.is_empty() || alpha <= 0.0 { Display::None } else { Display::Flex };
        let (pad, radius, gap) = (UiRect::axes(val(7.0), val(2.0)), BorderRadius::all(val(7.0)), UiRect::bottom(val(3.0)));
        if node.display != display || node.padding != pad || node.border_radius != radius || node.margin != gap {
            (node.display, node.padding, node.border_radius, node.margin) = (display, pad, radius, gap);
        }
    }
}

/// The lines' backing at full strength (B398): dark, translucent enough to keep the screen under it.
const BACKING: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);

fn spawn(commands: &mut Commands) {
    let node = Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(1.5),
        bottom: Val::Percent(18.0),
        max_width: Val::Percent(48.0),
        flex_direction: FlexDirection::Column,
        ..default()
    };
    let row = |i| (Row(i), Text::new(""), TextFont::default(), TextColor(Color::NONE), TextShadow::default(), Node { align_self: AlignSelf::FlexStart, display: Display::None, ..default() }, BackgroundColor(Color::NONE));
    commands.spawn((Root, node, Visibility::Hidden, GlobalZIndex(i32::MAX - 1))).with_children(|p| {
        for i in 0..=SHOWN {
            p.spawn(row(i));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests that run the plugin share its statics (`ASK`, `MENUS`, `BUTTON`...): one at a time.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn lines_are_cleaned() {
        assert_eq!(clean("  hi\tthere\n "), Some("hi there".into()));
        assert_eq!(clean("a\u{7}b\u{202e}c\u{0}"), Some("abc".into()));
        assert_eq!(clean(" \u{1b} "), None);
        assert_eq!(clean(&"x".repeat(200)).map(|s| s.len()), Some(MAX));
    }

    /// In a lobby, T opens the box, typed keys go into it (not to the menus' keys), Enter sends the line.
    #[test]
    fn enter_types_and_sends() {
        use bevy::input::{ButtonState, InputPlugin};
        let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin)).add_plugins(plugin);
        let (net, mut rx) = Net::wired();
        net.enter_room();
        app.insert_resource(net);
        app.update();
        let key = |app: &mut App, code, logical_key| {
            for state in [ButtonState::Pressed, ButtonState::Released] {
                app.world_mut().write_message(KeyboardInput { key_code: code, logical_key: Key::clone(&logical_key), state, text: None, repeat: false, window: Entity::PLACEHOLDER });
                app.update();
            }
        };
        key(&mut app, KeyCode::KeyT, Key::Character("t".into()));
        assert!(app.world().resource::<Chat>().open, "T opens the box");
        key(&mut app, KeyCode::KeyH, Key::Character("h".into()));
        key(&mut app, KeyCode::KeyI, Key::Character("i".into()));
        assert!(!app.world().resource::<ButtonInput<KeyCode>>().get_just_pressed().any(|_| true), "the menus see no key");
        key(&mut app, KeyCode::Enter, Key::Enter);
        let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).filter_map(|c| if let Cmd::Chat(t) = c { Some(t) } else { None }).collect();
        assert_eq!(sent, ["hi"]);
        assert!(!app.world().resource::<Chat>().open);
    }

    /// B512: in the menus a pad's R2 brings up no quick-chat phrase (in a match it does), and the lobby's Chat button
    /// opens the typed box and, pressed again, closes it.
    #[test]
    fn no_quick_chat_in_the_menus() {
        let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_message::<KeyboardInput>().init_resource::<ButtonInput<KeyCode>>().add_plugins(plugin);
        let (net, _rx) = Net::wired();
        net.enter_room();
        app.insert_resource(net);
        // no InputPlugin: the press stays just pressed every frame
        let mut g = Gamepad::default();
        g.digital_mut().press(GamepadButton::RightTrigger2);
        app.world_mut().spawn(g);
        MENUS.store(true, Ordering::Relaxed);
        app.update();
        assert_eq!(app.world().resource::<Chat>().phrase, None, "the lobby: no phrase");
        app.update();
        assert!(app.world().resource::<Chat>().phrase.is_some(), "a match: R2 shows a phrase");
        for want in [true, false] {
            MENUS.store(true, Ordering::Relaxed);
            ASK.store(1, Ordering::Relaxed);
            app.update();
            let chat = app.world().resource::<Chat>();
            assert_eq!((chat.open, chat.phrase), (want, None), "the Chat button toggles the typed box");
        }
    }

    /// B531: the on-screen pad's chat button stays up with the box open (wherever chat is on); a tap opens the box (the
    /// phone's keyboard follows `OPEN`), a second brings the keyboard back with the draft kept, and the iPhone
    /// keyboard's Return (a "\n" character) sends the line.
    #[test]
    fn the_chat_button_stays_and_raises_the_keyboard() {
        use bevy::input::ButtonState;
        let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_message::<KeyboardInput>().init_resource::<ButtonInput<KeyCode>>().add_plugins(plugin);
        let (net, mut rx) = Net::wired();
        net.enter_room();
        app.insert_resource(net);
        // the first frame spawns the lines' root
        app.update();
        app.update();
        assert!(BUTTON.load(Ordering::Relaxed), "up in a room");
        ASK.store(3, Ordering::Relaxed);
        app.update();
        assert!(app.world().resource::<Chat>().open && OPEN.load(Ordering::Relaxed), "a tap opens the box");
        assert!(BUTTON.load(Ordering::Relaxed), "the button stays up with the box open");
        let key = |app: &mut App, c: &str| {
            for state in [ButtonState::Pressed, ButtonState::Released] {
                let logical_key = Key::Character(c.into());
                app.world_mut().write_message(KeyboardInput { key_code: KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified), logical_key, state, text: None, repeat: false, window: Entity::PLACEHOLDER });
            }
            app.update();
        };
        key(&mut app, "h");
        RAISE.store(false, Ordering::Relaxed);
        ASK.store(3, Ordering::Relaxed);
        app.update();
        let chat = app.world().resource::<Chat>();
        assert!(chat.open && chat.draft == "h" && RAISE.load(Ordering::Relaxed), "a second tap brings the keyboard back");
        key(&mut app, "i");
        key(&mut app, "\n");
        let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).filter_map(|c| if let Cmd::Chat(t) = c { Some(t) } else { None }).collect();
        assert_eq!(sent, ["hi"]);
        assert!(!app.world().resource::<Chat>().open && !OPEN.load(Ordering::Relaxed));
        assert!(BUTTON.load(Ordering::Relaxed));
    }

    #[test]
    fn a_burst_is_limited() {
        let t = Instant::now();
        let mut sent = Vec::new();
        assert_eq!((0..6).map(|_| allow(&mut sent, t)).collect::<Vec<_>>(), [true, true, true, true, false, false]);
        assert!(allow(&mut sent, t + WINDOW));
    }
}
