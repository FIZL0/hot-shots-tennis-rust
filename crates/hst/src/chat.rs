//! M20: text chat in online lobbies and matches (ours, layered on top). A line goes over the peer connection as
//! `net::Msg::Chat` through the host, which cleans it, rate-limits it and passes it on with the sender's lobby name;
//! it never enters the match's input, so the sim, its generators and the replays never see it.
//!
//! The box: Enter opens it on the keyboard (typed keys go to it, not to the menu or the match), Enter sends, Esc
//! closes; a pad's R2 opens the preset phrases and steps through them, L2 says the one shown (they close by
//! themselves after a few seconds); a tap or click on Chat opens it on a touch page (the phone's keyboard). The last
//! few lines show at the bottom left, fading out, over the menus and the HUD alike, taking no presses from the match.
//! `/mute NAME` and `/unmute NAME` (or the phrases' Mute/Unmute rows) hide a player's lines on this machine. Settings
//! → Online & Mods → Chat off hides all of it.

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
#[derive(Component)]
struct Tap;

pub fn plugin(app: &mut App) {
    app.init_resource::<Chat>()
        .add_systems(PreUpdate, input.after(bevy::input::InputSystems))
        .add_systems(PostUpdate, draw);
}

/// The lines heard, and this machine's typing: while the box is open the keyboard's keys are the box's alone.
#[allow(clippy::too_many_arguments)]
fn input(
    net: Option<Res<Net>>,
    mut chat: ResMut<Chat>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    pads: Query<&Gamepad>,
    tap: Query<&Interaction, (With<Tap>, Changed<Interaction>)>,
    time: Res<Time<Real>>,
    mut was: Local<bool>,
) {
    let dt = time.delta_secs();
    chat.lines.iter_mut().for_each(|l| l.age += dt);
    chat.lines.retain(|l| l.age < LIFE);
    let on = !HIDDEN.load(Ordering::Relaxed) && net.as_ref().is_some_and(|n| n.in_room());
    let Some(net) = net else { return phone(&mut was, false) };
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
    let enter = [KeyCode::Enter, KeyCode::NumpadEnter];
    if !chat.open && (keys.any_just_pressed(enter) || tap.iter().any(|i| *i == Interaction::Pressed)) {
        chat.open = true;
        typed.clear();
        keys.reset_all();
    } else if chat.open {
        let mut send = false;
        for e in typed.read().filter(|e| e.state.is_pressed()) {
            match &e.logical_key {
                Key::Enter => send = true,
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
    // the pads: R2 shows / steps the phrases, L2 says the one shown
    let pads: Vec<_> = pads.iter().filter(|g| crate::play::controls::readable(g)).collect();
    let next = pads.iter().any(|g| g.just_pressed(GamepadButton::RightTrigger2));
    let say = pads.iter().any(|g| g.just_pressed(GamepadButton::LeftTrigger2));
    let n = chat.phrases().len();
    chat.phrase = match chat.phrase {
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

/// The phone's keyboard (the browser's code-entry input) up while the box is open on a touch page.
fn phone(was: &mut bool, on: bool) {
    if std::mem::replace(was, on) != on {
        #[cfg(target_arch = "wasm32")]
        crate::web::code_entry(on);
    }
}

/// The lines (fading), the box or the phrase, and Chat to tap; sized on the 4:3 screen's height as the FPS counter
/// (`graphics::fps_counter`), so they scale uniformly at any window shape.
fn draw(
    mut commands: Commands,
    net: Option<Res<Net>>,
    chat: Res<Chat>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    root: Query<(Entity, &mut Visibility), With<Root>>,
    mut rows: Query<(&Row, &mut Text, &mut TextFont, &mut TextColor)>,
) {
    let on = !HIDDEN.load(Ordering::Relaxed) && net.is_some_and(|n| n.in_room());
    let mut root = root;
    let Ok((_, mut vis)) = root.single_mut() else {
        if on {
            spawn(&mut commands);
        }
        return;
    };
    let v = if on { Visibility::Inherited } else { Visibility::Hidden };
    if *vis != v {
        *vis = v;
    }
    if !on {
        return;
    }
    let tall = window.single().is_ok_and(|w| w.width() * 3.0 < w.height() * 4.0);
    let px = |n: f32| if tall { bevy::text::FontSize::Vw(n / 448.0 * 75.0) } else { bevy::text::FontSize::Vh(n / 448.0 * 100.0) };
    let start = chat.lines.len().saturating_sub(SHOWN);
    let shown = &chat.lines[start..];
    let busy = chat.open || chat.phrase.is_some();
    for (row, mut text, mut font, mut colour) in &mut rows {
        let (s, alpha, c) = match row.0 {
            i if i < SHOWN => match shown.get(i) {
                Some(l) => {
                    let a = if busy { 1.0 } else { ((LIFE - l.age) / FADE).clamp(0.0, 1.0) };
                    let s = if l.name.is_empty() { l.text.clone() } else { format!("{}: {}", l.name, l.text) };
                    (s, a, if l.name.is_empty() { Color::srgb(1.0, 0.85, 0.4) } else { Color::WHITE })
                }
                None => (String::new(), 0.0, Color::WHITE),
            },
            // the box, or the pads' phrase
            SHOWN => match (chat.open, chat.phrase) {
                (true, _) => (format!("Say: {}_", chat.draft), 1.0, Color::srgb(0.6, 1.0, 0.6)),
                (_, Some((i, _))) => (format!("R2 next · L2 say: {}", chat.phrases()[i]), 1.0, Color::srgb(0.6, 1.0, 0.6)),
                _ => (String::new(), 0.0, Color::WHITE),
            },
            // Chat to tap or click
            _ => ("Chat".into(), if busy { 0.0 } else { 0.6 }, Color::WHITE),
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
    }
}

fn spawn(commands: &mut Commands) {
    let node = Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(1.5),
        bottom: Val::Percent(18.0),
        max_width: Val::Percent(48.0),
        flex_direction: FlexDirection::Column,
        ..default()
    };
    let row = |i| (Row(i), Text::new(""), TextFont::default(), TextColor(Color::NONE), TextShadow::default());
    commands.spawn((Root, node, Visibility::Hidden, GlobalZIndex(i32::MAX - 1))).with_children(|p| {
        for i in 0..=SHOWN {
            p.spawn(row(i));
        }
        p.spawn((Tap, Button, Node { align_self: AlignSelf::FlexStart, ..default() })).with_children(|b| {
            b.spawn(row(SHOWN + 1));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_cleaned() {
        assert_eq!(clean("  hi\tthere\n "), Some("hi there".into()));
        assert_eq!(clean("a\u{7}b\u{202e}c\u{0}"), Some("abc".into()));
        assert_eq!(clean(" \u{1b} "), None);
        assert_eq!(clean(&"x".repeat(200)).map(|s| s.len()), Some(MAX));
    }

    #[test]
    fn a_burst_is_limited() {
        let t = Instant::now();
        let mut sent = Vec::new();
        assert_eq!((0..6).map(|_| allow(&mut sent, t)).collect::<Vec<_>>(), [true, true, true, true, false, false]);
        assert!(allow(&mut sent, t + WINDOW));
    }
}
