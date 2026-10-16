//! B460: emotes in the online lobby and on the local Ready to play? screen (ours, layered on top). Each human there
//! flicks the right stick fully up / down / left / right (keys 1–4, the match's emote keys; the browser's and phone
//! apps' touch pad: L1 / R1 for left / right, as in a match, B271) and their card's 3D model (`previews.rs`) plays
//! that emote's motion (`play::emote::MOTIONS`) with its match voice line, then goes back to its pose. As in a match a
//! new one waits for the last to play out. The menus never read the right stick, so nothing clashes with moving about;
//! the touch pad's L1 / R1 there are the emote's, not the costume step's. Not while a picker (court, umpire), the
//! lobby's question covers the cards.
//!
//! Online a seat's emote goes to the other peers over the lobby channel (`net::Msg::Emote`, the host checking the seat
//! is the sender's) so every machine plays it on that seat's card. It is the menu's only: never the match's input,
//! nothing from the match's generators.
//!
//! B463: the character select too (`Screen::Chars`, `roster.rs`), on each seat's 3D preview: the keyboard's 1–4 all
//! four, a pad's right stick left / right the last two (its up / down stay B422's win / loss, `previews::buttons`,
//! the same two motions); the touch pad's L1 / R1 stay the costume there. Not for Random (no figure) or while that
//! seat names itself.
//!
//! `HST_EMOTE=0..3` (for `--shot`s): every card plays that emote as the screen opens.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bevy::prelude::*;
use hst_data::iso::Iso;
use hst_sim::sound;

use super::previews::{Card, Pending};
use super::{Dev, Menu, Screen, Who};
use crate::audio::{Sound, SoundBank};
use crate::character::{self, Motion};
use crate::inspect::{self, Preview, UvClock};
use crate::net::{Cmd, Net};
use crate::play::controls::Bindings;
use crate::play::emote::{self, KEYS, MOTIONS};

/// A flick waits this long (seconds) for its card's live model to load, then is dropped.
const WAIT: f32 = 3.0;

#[derive(Resource, Default)]
struct Plates {
    /// Each card's flick still to play (and when it came), its emote's end (elapsed seconds) once playing.
    queued: [Option<(u8, f32)>; 4],
    until: [Option<f32>; 4],
    /// Each pad's last flick (a new one starts an emote, as the match's `tick_input` does).
    held: HashMap<Entity, Option<u8>>,
    /// The voice banks heard (by bank path), and each card's last key per emote (the takes come round in turn, as
    /// the match's).
    banks: HashMap<String, Option<Arc<SoundBank>>>,
    keys: [[u8; 4]; 4],
    /// Each card's last emote line's play id (0 for none): a new one cuts it, as in a match (B473, B500).
    lines: [u64; 4],
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Plates>().add_systems(Update, (input.before(super::step), play.after(super::step).before(super::draw)));
}

fn on(menu: &Menu) -> bool {
    matches!(menu.screen, Screen::Lobby | Screen::Confirm | Screen::Chars)
}

/// The screens whose emotes go to (and come from) the other peers.
fn shared(menu: &Menu) -> bool {
    menu.screen == Screen::Lobby || menu.screen == Screen::Chars && menu.online()
}

/// This machine's seats' flicks, and (online) the other peers'.
fn input(
    menu: Res<Menu>,
    mut plates: ResMut<Plates>,
    keys: Res<ButtonInput<KeyCode>>,
    bind: Res<Bindings>,
    mut pads: Query<(Entity, &mut Gamepad, Option<&Name>)>,
    net: Option<Res<Net>>,
    time: Res<Time>,
    mut shot: Local<bool>,
) {
    let heard = net.as_ref().map(|n| n.emotes()).unwrap_or_default();
    if !on(&menu) {
        plates.queued = [None; 4];
        return;
    }
    let now = time.elapsed_secs();
    let mut flicks = Vec::new();
    // a picker over the cards (the court's or umpire's: its L1 / R1 page), the lobby's question: no flicks
    let covered = menu.picking.is_some() || menu.lobby_asking();
    let select = menu.screen == Screen::Chars;
    for (e, mut g, name) in pads.iter_mut().filter(|_| !covered) {
        let touch = name.is_some_and(|n| emote::is_touch(n));
        // B463: on the select only the stick's left / right (up / down are the preview's win / loss), no touch L1 / R1
        let d = if select { emote::flick(g.right_stick()).filter(|d| *d >= 2) } else { emote::bound(&g, &bind).or(emote::flick(g.right_stick())).or(emote::touch(&g, name)) };
        let was = plates.held.insert(e, d).flatten();
        if touch && !select {
            // the touch pad's L1 / R1 are the emotes here, not the costume step (`step` reads them after this)
            let b = g.digital_mut();
            b.clear_just_pressed(GamepadButton::LeftTrigger);
            b.clear_just_pressed(GamepadButton::RightTrigger);
        }
        if !select {
            // buttons bound to emotes are emotes here, not what `step` would do with them
            let b = g.digital_mut();
            for i in 9..13 /* controls.txt rows emote_up..emote_right */ {
                for x in bind.pads_of(i) {
                    b.clear_just_pressed(x);
                }
            }
        }
        if let Some(d) = d.filter(|d| Some(*d) != was) {
            flicks.push((Dev::Pad(e), d));
        }
    }
    // the keyboard's 1–4 are letters while a name is typed (the chat box takes the keys itself)
    if !covered
        && !(0..4).any(|q| menu.naming(q))
        && let Some(d) = KEYS.iter().position(|a| bind.key_just_pressed(&keys, *a))
    {
        flicks.push((Dev::Keys, d as u8));
    }
    for (dev, d) in flicks {
        // B463: the select's Random has no figure to play it, a seat naming itself takes its presses
        let idle = |p: usize| !select || !menu.players[p].random && !menu.naming(p);
        for p in (0..4).filter(|&p| menu.seats[p] == Some(dev) && idle(p)) {
            if start(&mut plates, p, d, now) && shared(&menu)
                && let Some(net) = &net
            {
                net.send(Cmd::Emote(p as u8, d));
            }
        }
    }
    // `HST_EMOTE=0..3` (for `--shot`s): every card flicks that way once, as the screen opens
    if !std::mem::replace(&mut *shot, true)
        && let Some(d) = std::env::var("HST_EMOTE").ok().and_then(|v| v.parse::<u8>().ok()).filter(|d| *d < 4)
    {
        (0..4).for_each(|p| _ = start(&mut plates, p, d, now));
    }
    for (seat, d) in heard {
        let p = seat as usize;
        if shared(&menu) && p < 4 && d < 4 && menu.seats[p].is_none() {
            start(&mut plates, p, d, now);
        }
    }
}

/// Card `p` takes flick `d` unless its last emote is still playing (or waiting to): whether it did.
fn start(plates: &mut Plates, p: usize, d: u8, now: f32) -> bool {
    let busy = plates.queued[p].is_some_and(|(_, t)| now - t < WAIT) || plates.until[p].is_some_and(|t| now < t);
    if !busy {
        plates.queued[p] = Some((d, now));
    }
    !busy
}

/// The flicks onto the cards' live models (a card still on its picture loads its model first, `previews::Pending`),
/// with the voice; a played-out emote back to the pose.
#[allow(clippy::too_many_arguments)]
fn play(
    menu: Res<Menu>,
    mut plates: ResMut<Plates>,
    mut cams: Query<(&mut Preview, &Card)>,
    data: Query<&character::Rig>,
    mut rigs: Query<(&mut Motion, &mut Transform, &mut UvClock)>,
    mut pending: ResMut<Pending>,
    (args, sound, time): (Res<crate::Args>, Option<Res<Sound>>, Res<Time>),
) {
    let now = time.elapsed_secs();
    let mut live = [false; 4];
    for (mut p, Card(i)) in &mut cams {
        let i = *i;
        let Ok(rig) = data.get(p.rig) else { continue };
        live[i] = true;
        let d = rig.data.clone();
        if plates.until[i].is_some_and(|t| now >= t) {
            plates.until[i] = None;
            inspect::apply(&mut p, inspect::Act::Pose, &mut rigs, &d);
        }
        let Some((dir, _)) = plates.queued[i].take() else { continue };
        let id = MOTIONS[dir as usize];
        if !inspect::react(&mut p, id, &mut rigs, &d) {
            continue;
        }
        // the match's length: the motion's ticks at 60 a second
        plates.until[i] = Some(now + d.motions[&id].length.ceil().max(1.0) / 60.0);
        // B474's mute covers these too
        if let Some(s) = sound.as_ref().filter(|_| crate::audio::EMOTE_VOICES.load(std::sync::atomic::Ordering::Relaxed)) {
            let who = menu.pick(i).0;
            let Plates { keys, banks, lines, .. } = &mut *plates;
            let key = &mut keys[i][dir as usize];
            if let Some((bank, play)) = voice(id, who, &menu.mods, banks, key, Path::new(&args.iso)) {
                if lines[i] != 0 {
                    s.stop(std::mem::take(&mut lines[i]));
                }
                lines[i] = s.play_centre(&bank, play);
            }
        }
    }
    for (i, &up) in live.iter().enumerate() {
        if !up {
            plates.until[i] = None;
            match plates.queued[i] {
                Some((_, t)) if now - t >= WAIT || !on(&menu) => plates.queued[i] = None,
                // its picture gives way to the live model (`previews::sync` loads it for a press)
                Some(_) if pending.0[i].is_none() => pending.0[i] = Some(inspect::Act::Pose),
                _ => {}
            }
        }
    }
}

/// The emote's match voice line (`play::emote::voice_keys`), from the character's singles voice bank (set a); a mod's
/// own `voice/` program of the same number (its takes wrap by key), a costume pack its disc character's, a mod with
/// none silent. `key` is the card's last key for this emote: the takes come round in turn.
fn voice(id: usize, who: Who, mods: &[crate::mods::Mod], banks: &mut HashMap<String, Option<Arc<SoundBank>>>, key: &mut u8, iso: &Path) -> Option<(Arc<SoundBank>, sound::Play)> {
    let (program, lo, hi) = emote::voice_keys(id)?;
    let next = if (*key as i32) < hi && *key as i32 >= lo { *key + 1 } else { lo as u8 };
    let c = match who {
        Who::Disc(c) => c,
        Who::Mod(i) => {
            let m = mods.get(i).filter(|m| crate::modfs::ready(&m.dir))?;
            let own = banks.entry(format!("mod {}", m.dir.join(&m.voice).display())).or_insert_with(|| crate::mods::voice(m).map(Arc::new)).clone();
            if let Some(own) = own.filter(|o| o.wav.contains_key(&(program as usize))) {
                *key = next;
                return Some((own, sound::Play { slot: 0x16, program, key: next, volume: 0x80, speed: 1.0 }));
            }
            m.costume_of?
        }
    };
    *key = next;
    let bank = banks.entry(format!("disc {c}")).or_insert_with(|| Iso::open(iso).ok().and_then(|mut iso| crate::audio::voice_bank(&mut iso, c, 2, 0, false)).map(Arc::new)).clone()?;
    Some((bank, sound::Play { slot: 0x16, program, key: *key, volume: 0x80, speed: 1.0 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keyboard's 3 on the lobby flicks left for the seat it holds; a second flick while that emote waits or plays
    /// is dropped, one after it has played out goes; another screen takes none.
    #[test]
    fn a_seats_flick_waits_for_its_last() {
        let mut app = App::new();
        app.init_resource::<Plates>().init_resource::<ButtonInput<KeyCode>>().init_resource::<Bindings>().init_resource::<Time>();
        app.insert_resource(Menu { screen: Screen::Lobby, seats: [None, Some(Dev::Keys), None, None], ..default() });
        app.add_systems(Update, input);
        let press = |app: &mut App, k: KeyCode| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(k);
            app.update();
            app.world().resource::<Plates>().queued
        };
        let q = press(&mut app, KeyCode::Digit3);
        assert_eq!(q.map(|q| q.map(|q| q.0)), [None, Some(2), None, None]);
        assert_eq!(press(&mut app, KeyCode::Digit1)[1].map(|q| q.0), Some(2), "the waiting one stays");
        // played (`play` took it), still playing, then out
        let mut plates = app.world_mut().resource_mut::<Plates>();
        (plates.queued[1], plates.until[1]) = (None, Some(1.0));
        assert_eq!(press(&mut app, KeyCode::Digit1)[1], None, "while it plays");
        app.world_mut().resource_mut::<Plates>().until[1] = Some(-1.0);
        assert_eq!(press(&mut app, KeyCode::Digit1)[1].map(|q| q.0), Some(0));
        app.world_mut().resource_mut::<Menu>().screen = Screen::Mods;
        assert_eq!(press(&mut app, KeyCode::Digit2), [None; 4], "not on the Mods page");
        app.world_mut().resource_mut::<Menu>().screen = Screen::Lobby;
        app.world_mut().resource_mut::<Menu>().picking = Some(0);
        assert_eq!(press(&mut app, KeyCode::Digit2), [None; 4], "not under the court picker");
    }

    /// B463: on the select the keyboard's 1–4 play on its seat's preview (not Random's, not while it names itself), a
    /// pad's right stick only left / right (up / down are the preview's win / loss); the touch pad's L1 stays the
    /// costume's.
    #[test]
    fn the_selects_emote_keys() {
        let mut app = App::new();
        app.init_resource::<Plates>().init_resource::<ButtonInput<KeyCode>>().init_resource::<Bindings>().init_resource::<Time>();
        let pad = app.world_mut().spawn((Gamepad::default(), Name::new("Xbox Controller"))).id();
        let touch = app.world_mut().spawn((Gamepad::default(), Name::new("Touch controls (STANDARD GAMEPAD)"))).id();
        app.insert_resource(Menu { screen: Screen::Chars, doubles: true, seats: [Some(Dev::Keys), Some(Dev::Pad(pad)), Some(Dev::Pad(touch)), None], ..default() });
        app.add_systems(Update, input);
        let queued = |app: &mut App| std::mem::take(&mut app.world_mut().resource_mut::<Plates>().queued).map(|q| q.map(|q| q.0));
        let keys = |app: &mut App, k: KeyCode| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(k);
            app.update();
            queued(app)
        };
        for (k, d) in [(KeyCode::Digit1, 0), (KeyCode::Digit2, 1), (KeyCode::Digit3, 2), (KeyCode::Digit4, 3)] {
            assert_eq!(keys(&mut app, k), [Some(d), None, None, None], "{k:?}");
        }
        app.world_mut().resource_mut::<Menu>().players[0].random = true;
        assert_eq!(keys(&mut app, KeyCode::Digit1), [None; 4], "Random has no figure");
        app.world_mut().resource_mut::<Menu>().players[0].random = false;
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
        let stick = |app: &mut App, e: Entity, x: f32, y: f32| {
            let mut g = app.world_mut().get_mut::<Gamepad>(e).unwrap();
            let a = g.analog_mut();
            a.set(GamepadAxis::RightStickX, x);
            a.set(GamepadAxis::RightStickY, y);
            app.update();
            queued(app)
        };
        assert_eq!(stick(&mut app, pad, 0.0, 1.0), [None; 4], "up is the preview's win");
        assert_eq!(stick(&mut app, pad, 0.0, 0.0), [None; 4]);
        assert_eq!(stick(&mut app, pad, -1.0, 0.0), [None, Some(2), None, None]);
        assert_eq!(stick(&mut app, pad, 0.0, 0.0), [None; 4]);
        assert_eq!(stick(&mut app, pad, 1.0, 0.0), [None, Some(3), None, None]);
        app.world_mut().get_mut::<Gamepad>(touch).unwrap().digital_mut().press(GamepadButton::LeftTrigger);
        app.update();
        assert_eq!(queued(&mut app), [None; 4], "the touch pad's L1 is the costume");
        assert!(app.world().get::<Gamepad>(touch).unwrap().just_pressed(GamepadButton::LeftTrigger));
    }

    /// B271's touch pad: its L1 is the left flick here, and the menu's costume step no longer sees the press; a real
    /// pad's L1 stays the menu's.
    #[test]
    fn the_touch_pads_l1_emotes() {
        for (name, touch) in [("Touch controls (STANDARD GAMEPAD)", true), ("Xbox Controller", false)] {
            let mut app = App::new();
            app.init_resource::<Plates>().init_resource::<ButtonInput<KeyCode>>().init_resource::<Bindings>().init_resource::<Time>();
            let pad = app.world_mut().spawn((Gamepad::default(), Name::new(name))).id();
            app.insert_resource(Menu { screen: Screen::Confirm, seats: [Some(Dev::Pad(pad)), None, None, None], ..default() });
            app.add_systems(Update, input);
            app.world_mut().get_mut::<Gamepad>(pad).unwrap().digital_mut().press(GamepadButton::LeftTrigger);
            app.update();
            let g = app.world().get::<Gamepad>(pad).unwrap();
            assert_eq!(app.world().resource::<Plates>().queued[0].map(|q| q.0), touch.then_some(2), "{name}");
            assert_eq!(g.just_pressed(GamepadButton::LeftTrigger), !touch, "{name}");
        }
    }

    /// The emotes' voice lines: the match's programs (gu_set 9, di_set 10, gu 7, di 8), every take in turn (B500).
    #[test]
    fn emote_voices_take_turns() {
        let iso = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"));
        if !iso.is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut banks = HashMap::new();
        for (id, program, keys) in [(0x2e, 9, &[1, 0, 1][..]), (0x2f, 10, &[1, 0, 1]), (0x2c, 7, &[1, 2, 0, 1]), (0x2d, 8, &[1, 2, 3, 4, 0, 1])] {
            let mut key = 0;
            let got: Vec<_> = keys.iter().map(|_| voice(id, Who::Disc(2), &[], &mut banks, &mut key, iso).map(|(_, p)| (p.program, p.key)).unwrap()).collect();
            assert_eq!(got, keys.iter().map(|&k| (program, k)).collect::<Vec<_>>(), "motion {id:#x}");
        }
    }
}
