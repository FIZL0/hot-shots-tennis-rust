//! Right-stick emotes (B145, ours, layered on top): a human flicks the right stick fully up / down / left / right
//! (keys 1–4) and their player plays one of four of the character's own motions with its voice line, standing: no
//! run, no stroke and no swing press until the motion is out. A ball that comes meanwhile goes by unhit — that's the
//! cost. In live play only: in a rally, waiting for a serve and serving before the toss, not mid-stroke. Never
//! started between points (B472: the post-point time, a fault's or let's call included, and the changes of ends);
//! one started in the rally plays on into the post-point time until the point's reaction ends it (B278), and
//! not at all once the point has won a game or set (B363). The flick is part of the match input (`tick_input`'s right stick bytes), so replays and online play see it alike; bots never emote. Nothing here draws from the match's generators.

use bevy::prelude::*;
use hst_sim::score::Event;
use hst_sim::motion::id;
use hst_sim::sound;

use super::{Game, Phase, controls::Action, set_motion};

/// The emotes by flick (up, down, left, right): the character's post-point reactions `gu_set` (the big win),
/// `di_set` (the big loss), `gu` (a won point), `di` (a lost one) — a mod character's from its pack.
pub const MOTIONS: [usize; 4] = [id::WON_GAME as usize, id::LOST_GAME as usize, id::WON as usize, id::LOST as usize];

/// Keys 1–4 by default, in flick order.
pub const KEYS: [Action; 4] = [Action::EmoteUp, Action::EmoteDown, Action::EmoteLeft, Action::EmoteRight];

/// A right stick (`Gamepad::right_stick`, +y up) pushed fully toward one side: its emote direction.
pub fn flick(v: Vec2) -> Option<u8> {
    // ponytail: "fully" is 0.9 of the way out, so worn sticks still reach it
    (v.length() >= 0.9).then(|| match v.y.abs() > v.x.abs() {
        true if v.y > 0.0 => 0,
        true => 1,
        false if v.x < 0.0 => 2,
        false => 3,
    })
}

/// B271: the browser's touch pad (web/touch.js) has no right stick, so its L1 / R1 stand in for the left / right
/// flick, held as a stick is. Only that pad: a real pad's shoulders never emote.
pub fn touch(g: &Gamepad, name: Option<&Name>) -> Option<u8> {
    name.is_some_and(|n| is_touch(n))
        .then(|| [(GamepadButton::LeftTrigger, 2), (GamepadButton::RightTrigger, 3)].into_iter().find(|(b, _)| g.pressed(*b)))
        .flatten()
        .map(|(_, d)| d)
}

/// A pad button bound to an emote in `controls.txt` (none by default), held: its direction.
pub fn bound(g: &Gamepad, bind: &super::controls::Bindings) -> Option<u8> {
    KEYS.iter().position(|a| bind.pad_pressed(g, *a)).map(|d| d as u8)
}

/// The touch pad's id (web/touch.js `pad.id`), as gilrs names it.
pub fn is_touch(name: &str) -> bool {
    name.starts_with("Touch controls")
}

/// Human player `i`'s turn while it may emote: `start` is this tick's new flick. True while the emote holds the
/// player (`control` skips its turn).
pub(super) fn tick(g: &mut Game, i: usize, start: Option<u8>) -> bool {
    // the server too, until its toss (B228: online the host is 1P, who serves first)
    let serving = g.phase == Phase::Serve && g.score.server == i as i32;
    // B278: one from the rally plays on into the post-point time; the reaction (`react`) ends it
    // B363: not once the point has won a game or set (the match too): the game's and set's reactions and show
    let open = match g.phase {
        Phase::Rally => true,
        Phase::Post => !ends_a_game(g),
        Phase::Serve => !serving || g.serving.toss.is_none(),
        Phase::ChangeEnds(_) => false,
    };
    if !open {
        g.players[i].emote = 0;
        return false;
    }
    // B472: only live play starts one, never the time between points
    if let Some(d) = start
        && g.phase != Phase::Post
        && g.players[i].emote == 0
        && idle(g, i)
        && let Some(clip) = g.data[i].motions.get(&MOTIONS[d as usize])
    {
        let ticks = clip.length.ceil() as u32;
        let p = &mut g.players[i];
        p.emote = ticks.max(1);
        p.body.running = false;
        p.body.vel = [0.0; 3];
        set_motion(p, MOTIONS[d as usize] as i32, 1.0, false, None);
        voice(g, i, MOTIONS[d as usize]);
    }
    let p = &mut g.players[i];
    if p.emote == 0 {
        return false;
    }
    p.emote -= 1;
    (p.prev, p.vel, p.aim, p.dpad) = (p.pos, Vec2::ZERO, Vec2::ZERO, 0);
    if serving {
        // the ball stays in hand (`held_ball` puts it at the emote's hand), as `serve_turn` holds it
        super::hold_ball(g);
    }
    true
}

/// The point over ended a game or a set (or the match): its transition, where nobody emotes (B363).
fn ends_a_game(g: &Game) -> bool {
    g.post.as_ref().is_some_and(|p| matches!(p.event, Some(Event::Game | Event::Set)))
}

/// While the server emotes before its toss, a mod character's ball (B252): its own emote clip is what's drawn
/// (`CharacterData::visual`), but `held_ball` places the ball on the donor's, so the ball would float off the hand
/// — it's hidden (with its shadow and outline) until the emote ends, then back in hand for the toss. A disc
/// character's ball stays in its hand, seen. Draw-side only.
pub(super) fn hide_ball(g: Res<Game>, mut q: Query<&mut Visibility, Or<(With<super::BallView>, With<crate::shade::Ball>)>>, mut hidden: Local<bool>) {
    let i = g.score.server as usize;
    let hide = g.phase == Phase::Serve
        && g.players.get(i).is_some_and(|p| p.emote > 0 && g.data[i].visual.contains_key(&p.cmd.id));
    // only on a change, so the ceremony's hiding (`match_stats::ceremony::hide`) stays
    if std::mem::replace(&mut *hidden, hide) != hide {
        for mut v in &mut q {
            *v = if hide { Visibility::Hidden } else { Visibility::Inherited };
        }
    }
}

/// Nothing under way: no stroke, dive, whiff, serve follow-through, reaction or body hit.
fn idle(g: &Game, i: usize) -> bool {
    let p = &g.players[i];
    p.swing.is_none()
        && p.after.is_none()
        && p.contact.is_none()
        && p.pending.is_none()
        && p.approach.is_none()
        && p.whiff.is_none()
        && p.dive.is_none()
        && p.served.is_none()
        && p.follow.is_none()
        && p.wait_swing.is_none()
        // a played-out reaction may be followed by an emote (its root has stopped moving the player)
        && (p.root.is_none() || p.played)
        && !super::bodyhit::standing(g, i)
}

/// The reaction's voice line for the emote's motion (`voice_keys`), its key the next of the player's emote takes:
/// B300 and B500, the key memory is its own, slot 2, which neither the shouts (0, 1) nor the reactions (3–5) use, so
/// the real reactions pick as before. The memory is match state (snapshots carry it), so every peer picks alike;
/// nothing draws from the match's generators.
fn voice(g: &mut Game, i: usize, motion: usize) {
    // B474: muted here only after the pick, so the key memory (match state) moves the same on every peer
    if let Some(play) = line(&mut g.voices[i], i, motion).filter(|_| crate::audio::EMOTE_VOICES.load(std::sync::atomic::Ordering::Relaxed)) {
        g.lines.push(Line::Emote(i, play));
    }
}

/// An emote's voice program and its takes `lo..=hi`, by motion: the reaction's (`sound::reaction_voice` as a
/// winner, never the losers' group groan, the singles keys whatever the player count: the character's bank has them
/// all), widened to every take of the program on the disc banks (B500: gu's 7 has 0–2 — key 0 the instant replay's —
/// di's 8 0–4 — 2 the replay's, 3–4 the losing side's groan); gu_set's 9 and di_set's 10 have only their two.
pub fn voice_keys(motion: usize) -> Option<(u8, i32, i32)> {
    let (program, lo, hi, _) = sound::reaction_voice(2, motion as i32, false, false, || 100)?;
    Some(match program {
        7 => (7, 0, 2),
        8 => (8, 0, 4),
        _ => (program, lo, hi),
    })
}

/// A voice line that cuts emote lines still sounding (B473): an emote's line its own player's last one, a post-point
/// reaction's every player's.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum Line {
    Emote(usize, sound::Play),
    Reaction(usize, sound::Play),
}

/// Plays the lines due this tick at their player, each once however often its tick runs (`heard`), cutting the emote
/// lines still sounding (`emotes`: each player's last emote line's play id, 0 for none).
pub(super) fn play_lines(
    mut g: ResMut<Game>,
    sound: Option<Res<crate::audio::Sound>>,
    voices: Option<Res<crate::audio::VoiceBanks>>,
    heard: Res<super::heard::Heard>,
    mut played: ResMut<super::heard::Played>,
    mut emotes: Local<[u64; 4]>,
) {
    let lines = std::mem::take(&mut g.lines);
    let (Some(sound), Some(voices)) = (sound, voices) else { return };
    for line in lines {
        let (Line::Emote(i, p) | Line::Reaction(i, p)) = line;
        let at = g.players[i].pos;
        if !played.first(heard.tick, ("line", p, at)) {
            continue;
        }
        let cut = match line {
            Line::Emote(..) => i..i + 1,
            Line::Reaction(..) => 0..emotes.len(),
        };
        for id in emotes[cut].iter_mut().filter(|id| **id != 0) {
            sound.stop(std::mem::take(id));
        }
        let Some(b) = voices.0.get(p.slot as usize - 1).and_then(|b| b.as_ref()) else { continue };
        let id = sound.play_at(b, p, at);
        if let Line::Emote(..) = line {
            emotes[i] = id;
        }
    }
}

/// Player `i`'s emote line for `motion` on its voice `v`: the take after its last.
fn line(v: &mut sound::Voice, i: usize, motion: usize) -> Option<sound::Play> {
    let (program, lo, hi) = voice_keys(motion)?;
    let k = v.cycle(lo, hi, EMOTE_SLOT);
    v.react(i, (program, k, k, None), || 0)
}

/// The emotes' key-memory slot in `sound::Voice`.
const EMOTE_SLOT: usize = 2;

#[cfg(test)]
mod tests {
    use super::super::tick_input::{IDLE, TickPads, decode};
    use super::*;

    /// Pad buttons rebound to emotes (bumpers, triggers) play them; unbound, a shoulder never does.
    #[test]
    fn rebound_pad_buttons_emote() {
        let (bind, errors) = super::super::controls::Bindings::parse("emote_left = PadLeftTrigger\nemote_up = PadRightTrigger2\n");
        assert!(errors.is_empty(), "{errors:?}");
        let mut g = Gamepad::default();
        assert_eq!(bound(&g, &bind), None);
        g.digital_mut().press(GamepadButton::LeftTrigger);
        assert_eq!(bound(&g, &bind), Some(2));
        let mut g = Gamepad::default();
        g.digital_mut().press(GamepadButton::RightTrigger2);
        assert_eq!(bound(&g, &bind), Some(0));
        let mut g = Gamepad::default();
        g.digital_mut().press(GamepadButton::LeftTrigger);
        assert_eq!(bound(&g, &Default::default()), None);
    }

    /// B300 / B500: repeated emotes of one direction go through every take of their program in turn, and leave the
    /// reactions' key memory alone.
    #[test]
    fn emote_lines_vary() {
        for (m, program, keys) in [(id::WON_GAME, 9, &[0, 1, 0][..]), (id::LOST_GAME, 10, &[0, 1, 0]), (id::WON, 7, &[0, 1, 2, 0]), (id::LOST, 8, &[0, 1, 2, 3, 4, 0])] {
            let mut v = sound::Voice::default();
            let got: Vec<_> = keys.iter().map(|_| line(&mut v, 1, m as usize).map(|p| (p.program, p.key)).unwrap()).collect();
            assert_eq!(got, keys.iter().map(|&k| (program, k)).collect::<Vec<_>>(), "motion {m:#x}");
            let rv = sound::reaction_voice(2, m as i32, false, false, || 0).unwrap();
            let mut fresh = sound::Voice::default();
            assert_eq!(v.react(1, rv, || 3).map(|p| p.key), fresh.react(1, rv, || 3).map(|p| p.key), "motion {m:#x}");
        }
    }

    #[test]
    fn flicks() {
        assert!(is_touch("Touch controls (STANDARD GAMEPAD)") && !is_touch("Xbox Controller"));
        assert_eq!([Vec2::Y, -Vec2::Y, -Vec2::X, Vec2::X, Vec2::new(0.5, 0.5), Vec2::ZERO].map(flick), [Some(0), Some(1), Some(2), Some(3), None, None]);
    }

    /// A flick goes over the wire as the right stick and comes back once, on the tick it starts.
    #[test]
    fn flick_on_the_wire() {
        for d in 0..4 {
            let mut s = super::super::SlotPad { emote: Some(d), ..default() };
            let p = super::super::tick_input::take(&mut s);
            assert_eq!(s.emote, None);
            assert_eq!((decode(p, IDLE).emote, decode(p, p).emote, decode(IDLE, p).emote), (Some(d), None, None));
            let mut t = TickPads::default();
            t.push([p, IDLE, IDLE, IDLE]);
            assert_eq!(t.slot(0).emote, Some(d));
        }
        assert_eq!(decode(super::super::tick_input::recorded(hst_sim::replay::Pad { rx: 0x80, ry: 0, ..IDLE }), IDLE).emote, None);
    }
}

/// An emote in the app's singles match: 2P (a disc character, 1P a bot that serves first) flicks the right stick during the rally; it
/// stands, its motion plays and its presses do nothing for the motion's length, then it runs and swings again.
/// Needs the ISO at the repository root (skipped without).
#[cfg(test)]
mod match_tests {
    use super::super::tick_input::{IDLE, Replay, TickPads};
    use super::{MOTIONS, id};
    use super::super::*;
    use crate::{Args, GameSpace};
    use hst_sim::replay::Pad;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    fn app(pads: Vec<[Pad; 4]>) -> App {
        seated([false, true, false, false], pads)
    }

    fn seated(seats: [bool; 4], pads: Vec<[Pad; 4]>) -> App {
        modded(seats, pads, None)
    }

    /// `seated` with 2P playing mod `m`.
    fn modded(seats: [bool; 4], pads: Vec<[Pad; 4]>, m: Option<crate::mods::Mod>) -> App {
        let mut app = App::new();
        if let Some(m) = m {
            app.insert_resource(crate::mods::MatchMod(vec![None, Some(m)]));
        }
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { seats: Some(seats), autoplay: false, ..default() });
        app.insert_resource(Replay(pads.into_iter().collect()));
        app.init_resource::<TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, super::super::tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        for i in 0..2 {
            app.world_mut().spawn((Figure(i), character::Motion::default()));
        }
        app
    }

    /// 2P's position, emote ticks left, whether a press is under way (search, whiff or swing), motion; the phase.
    fn step(app: &mut App) -> (V3, u32, bool, usize, Phase) {
        app.world_mut().run_schedule(FixedUpdate);
        let g = app.world().resource::<Game>();
        let p = &g.players[1];
        (p.pos, p.emote, p.pending.is_some() || p.whiff.is_some() || p.swing.is_some(), p.cmd.id, g.phase.clone())
    }

    #[test]
    fn an_emote_locks_then_plays_on() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let left = Pad { lx: 0, ..IDLE };
        let flick = Pad { rx: 0x80, ry: 0, ..left };
        let cross = Pad { buttons: 0x2000, ..left };
        let n = 600;
        let rally = {
            let mut a = app(vec![]);
            (0..n).find(|_| step(&mut a).4 == Phase::Rally).expect("a rally")
        };
        let len = app(vec![]).world().resource::<Game>().data[1].motions[&MOTIONS[0]].length.ceil() as usize;
        // waiting for 1P's serve: 2P emotes from tick `at`, stick held left and ✕ pressed meanwhile, then ✕ in the rally
        let at = 10;
        assert!(at + len + 20 < rally, "the emote ({len} ticks) outlasts the serve's wait ({rally})");
        let mut pads = vec![[IDLE, left, IDLE, IDLE]; n];
        pads[at][1] = flick;
        pads[at + 5][1] = cross;
        pads[rally + 10][1] = cross;
        let mut a = app(pads);
        let mut last = None;
        for t in 0..rally + 12 {
            let (pos, emote, pressed, motion, phase) = step(&mut a);
            if (at..at + len).contains(&t) {
                assert_eq!((emote as usize, motion, pressed), (at + len - 1 - t, MOTIONS[0], false), "tick {t}");
                assert!(last.is_none_or(|l| l == pos), "moved while emoting, tick {t}");
                last = Some(pos);
            }
            if t == at + len + 2 {
                assert_ne!(Some(pos), last, "didn't run after the emote");
            }
            if t == rally + 10 {
                assert!(phase == Phase::Rally && pressed, "the press after the emote did nothing");
            }
        }
        // the same flick as the serve comes: it goes by unhit, and the point's end doesn't cut it (B278): the
        // emote plays on into the post-point time until it's out or the reaction takes over
        let mut pads = vec![[IDLE; 4]; n];
        pads[rally + 2][1] = flick;
        pads[rally + 10][1] = cross;
        let mut a = app(pads);
        let (mut emoted, mut ended, mut last) = (false, None, 0);
        for t in 0..n {
            let (_, emote, pressed, motion, phase) = step(&mut a);
            assert!(!pressed, "pressed while emoting, tick {t}");
            emoted |= emote > 0;
            if let Some(e) = ended {
                let reacted = a.world().resource::<Game>().post.as_ref().is_some_and(|p| p.reacted);
                if emote == 0 {
                    assert!(last == 1 || reacted, "the emote was cut at tick {t} ({last} left, point ended {e})");
                    return eprintln!("emote {len} ticks, rally from {rally}, the point ended at {e}, emote out at {t}, reacted {reacted}");
                }
                assert_eq!((emote, motion), (last - 1, MOTIONS[0]), "tick {t}");
            } else if emoted && phase != Phase::Rally {
                assert!(emote > 0, "the emote was out before the point ended");
                ended = Some(t);
            }
            last = emote;
        }
        panic!("the point never ended");
    }

    /// B228: the server emotes before its toss (online the host is 1P and serves first), the ball in its hand
    /// all the while, then tosses as usual.
    #[test]
    fn the_server_emotes_before_the_toss() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let flick = Pad { rx: 0, ry: 0x80, ..IDLE };
        let cross = Pad { buttons: 0x2000, ..IDLE };
        let len = app(vec![]).world().resource::<Game>().data[0].motions[&MOTIONS[2]].length.ceil() as usize;
        let (at, n) = (10, 600);
        let mut pads = vec![[IDLE; 4]; n];
        pads[at][0] = flick;
        pads[at + len + 5][0] = cross;
        let mut a = seated([true, false, false, false], pads);
        for t in 0..n {
            a.world_mut().run_schedule(FixedUpdate);
            let drawn = a.world_mut().query::<(&Figure, &character::Motion)>().iter(a.world()).find(|(f, _)| f.0 == 0).unwrap().1.id;
            let g = a.world().resource::<Game>();
            let p = &g.players[0];
            if (at..at + len).contains(&t) {
                assert_eq!((p.emote as usize, p.cmd.id, g.phase.clone()), (at + len - 1 - t, MOTIONS[2], Phase::Serve), "tick {t}");
                let d = (0..3).map(|k| (g.flight.ball.pos[k] - p.pos[k]).powi(2)).sum::<f32>().sqrt();
                assert!(d < 2.5 && g.flight.ball.vel == [0.0; 3], "the ball left the hand, tick {t}: {d} m");
            }
            // B231: the drawn body plays the emote, not the serve stance, then goes back to the stance
            if (at..at + len - 1).contains(&t) {
                assert_eq!(drawn, MOTIONS[2], "the body kept the stance, tick {t}");
            }
            if t == at + len + 2 {
                assert!((id::SERVE_STANCE as usize..=id::SERVE_RIGHT as usize).contains(&drawn), "no stance after the emote, tick {t}: {drawn:#x}");
            }
            // the press tosses (a second would swing)
            if g.serving.toss.is_some() {
                assert_eq!(t, at + len + 5, "tossed at tick {t}");
                return;
            }
        }
        panic!("never tossed");
    }

    /// B252: a disc server's ball stays seen through its emote; a mod's (its own emote clip drawn, here the donor's
    /// copied in as one) is hidden for the emote's length and back for the toss.
    #[test]
    fn a_mods_ball_hides_while_it_emotes() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let flick = Pad { rx: 0, ry: 0x80, ..IDLE };
        for own_clip in [false, true] {
            let (at, n) = (10, 120);
            let mut pads = vec![[IDLE; 4]; n];
            pads[at][0] = flick;
            let mut a = seated([true, false, false, false], pads);
            a.add_systems(FixedUpdate, super::hide_ball.after(held_ball));
            let ball = a.world_mut().spawn((BallView(false), Visibility::Inherited)).id();
            // the rigs share the data: dropped, so this test may change it
            let rigs: Vec<_> = a.world_mut().query_filtered::<Entity, With<character::Rig>>().iter(a.world()).collect();
            for e in rigs {
                a.world_mut().entity_mut(e).remove::<character::Rig>();
            }
            let len = {
                let mut g = a.world_mut().resource_mut::<Game>();
                let d = std::sync::Arc::get_mut(&mut g.data[0]).expect("the match's own character data");
                let clip = d.motions[&MOTIONS[2]].clone();
                let len = clip.length.ceil() as usize;
                if own_clip {
                    d.visual.insert(MOTIONS[2], clip);
                }
                len
            };
            for t in 0..at + len + 5 {
                a.world_mut().run_schedule(FixedUpdate);
                let hidden = a.world().get::<Visibility>(ball) == Some(&Visibility::Hidden);
                assert_eq!(hidden, own_clip && (at..at + len - 1).contains(&t), "own clip {own_clip}, tick {t}");
            }
        }
    }

    /// B473: 2P's second emote cuts its first one's voice line, and a post-point reaction's line cuts the second.
    #[test]
    fn a_new_line_cuts_the_last_emote_line() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let flick = Pad { rx: 0x80, ry: 0, ..IDLE };
        let mut a = app(vec![]);
        let len = a.world().resource::<Game>().data[1].motions[&MOTIONS[0]].length.ceil() as usize;
        let (at, again) = (10, 10 + len + 3);
        let mut pads = vec![[IDLE; 4]; again + 2];
        (pads[at][1], pads[again][1]) = (flick, flick);
        a.insert_resource(Replay(pads.into_iter().collect()));
        let sound = crate::audio::Sound::test(&mut hst_data::iso::Iso::open(ISO).unwrap());
        a.insert_resource(sound.clone());
        a.add_plugins(heard::plugin);
        a.add_systems(FixedUpdate, super::play_lines.after(held_ball));
        let mut lines = Vec::new();
        for t in 0..=again {
            let before = sound.plays();
            step(&mut a);
            if t == at || t == again {
                assert_eq!(sound.plays(), before + 1, "tick {t}: no emote line");
                lines.push(sound.plays());
            }
        }
        assert!(!sound.playing(lines[0]) && sound.playing(lines[1]), "the second emote didn't cut the first's line");
        // a reaction line, here 1P's, cuts every emote line
        let p = sound::Play { slot: 1, program: 7, key: 1, volume: 0x80, speed: 1.0 };
        a.world_mut().resource_mut::<Game>().lines.push(super::Line::Reaction(0, p));
        step(&mut a);
        assert!(!sound.playing(lines[1]) && sound.playing(sound.plays()), "the reaction didn't cut the emote line");
    }

    /// A mod character's emotes speak its own takes (programs 9, 10, 7, 8): each of the four flicks plays a line.
    /// Needs the ISO and the mods repo's packaged mods in `mods/hst-mods` (skipped without).
    #[test]
    fn a_mod_emote_speaks() {
        const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/");
        if !std::path::Path::new(ISO).is_file() || !std::path::Path::new(ROOT).is_dir() {
            return eprintln!("no ISO or mods/hst-mods, skipped");
        }
        for id in ["opentee2_pc10_carly", "opentee2_pc03_shu", "oob_pc12_anya"] {
            let m = crate::mods::read(&std::path::Path::new(ROOT).join(id)).unwrap();
            let mut a = modded([false, true, false, false], vec![], Some(m));
            let len = (0..4).map(|d| a.world().resource::<Game>().data[1].motions.get(&MOTIONS[d]).map_or(0, |c| c.length.ceil() as usize)).max().unwrap();
            let ats: Vec<usize> = (0..4).map(|d| 10 + d * (len + 5)).collect();
            let mut pads = vec![[IDLE; 4]; ats[3] + 2];
            for (d, &at) in ats.iter().enumerate() {
                pads[at][1] = [Pad { rx: 0x80, ry: 0, ..IDLE }, Pad { rx: 0x80, ry: 0xff, ..IDLE }, Pad { rx: 0, ry: 0x80, ..IDLE }, Pad { rx: 0xff, ry: 0x80, ..IDLE }][d];
            }
            a.insert_resource(Replay(pads.into_iter().collect()));
            let sound = crate::audio::Sound::test(&mut hst_data::iso::Iso::open(ISO).unwrap());
            a.insert_resource(sound.clone());
            a.add_plugins(heard::plugin);
            a.add_systems(FixedUpdate, super::play_lines.after(held_ball));
            for t in 0..=ats[3] {
                let before = sound.plays();
                let (_, emote, ..) = step(&mut a);
                if let Some(d) = ats.iter().position(|&at| at == t) {
                    assert!(emote > 0, "{id}: flick {d} started no emote");
                    assert_eq!(sound.plays(), before + 1, "{id}: flick {d}'s emote spoke no line");
                }
            }
        }
    }

    /// B472 (B363 before it): 1P tosses and swings as the ball drops past (a whiff: a fault); the same flick in the
    /// call's post-point time starts nothing, nor in one that won a game (the fault's post-point turned into one) or
    /// a set; once the next serve's wait is on, it does.
    #[test]
    fn no_emote_between_points() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let flick = Pad { rx: 0, ry: 0x80, ..IDLE };
        let cross = Pad { buttons: 0x2000, ..IDLE };
        let n = 900;
        let mut pads = vec![[IDLE; 4]; n];
        (pads[10][0], pads[134][0]) = (cross, cross);
        let post = {
            let mut a = seated([true, false, false, false], pads.clone());
            (0..n).find(|_| {
                a.world_mut().run_schedule(FixedUpdate);
                a.world().resource::<Game>().phase == Phase::Post
            })
            .expect("the fault's call")
        };
        let at = post + 3;
        pads[at][0] = flick;
        for event in [Some(Event::Game), Some(Event::Set), None] {
            let mut a = seated([true, false, false, false], pads.clone());
            for t in 0..=at {
                if t == at {
                    a.world_mut().resource_mut::<Game>().post.as_mut().expect("post-point").event = event;
                }
                a.world_mut().run_schedule(FixedUpdate);
            }
            let g = a.world().resource::<Game>();
            assert_eq!((g.phase.clone(), g.players[0].emote), (Phase::Post, 0), "{event:?}");
        }
        // the same flick held through the post-point time, then let go and flicked again in the second serve's wait
        let mut a = seated([true, false, false, false], pads.clone());
        let mut serve = None;
        for t in 0..n {
            let phase = a.world().resource::<Game>().phase.clone();
            if serve.is_none() && t > at && phase == Phase::Serve {
                serve = Some(t);
            }
            if let Some(p) = a.world_mut().resource_mut::<Replay>().0.front_mut() {
                p[0] = match serve {
                    Some(s) if t == s + 2 => flick,
                    Some(_) => IDLE,
                    None if t >= at => flick,
                    None => p[0],
                };
            }
            a.world_mut().run_schedule(FixedUpdate);
            let g = a.world().resource::<Game>();
            if serve.is_none() {
                assert_eq!(g.players[0].emote, 0, "emoted between points, tick {t}");
            } else if serve.is_some_and(|s| t == s + 2) {
                assert!(g.players[0].emote > 0, "no emote waiting to serve, tick {t}");
                return;
            }
        }
        panic!("no second serve");
    }
}
