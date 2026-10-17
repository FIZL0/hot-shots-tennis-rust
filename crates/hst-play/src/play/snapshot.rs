//! The match state you can copy and hash (M2c), for rollback (save → re-simulate → restore) and desync checks.
//!
//! `Game` is the per-tick state; its per-match constants (trajectory tables, `CharacterData`, the collision world…)
//! sit in it behind [`Const`], shared between copies and left out of the hash. A [`Snapshot`] adds what the tick keeps
//! outside `Game`: the pads (`TickPads`, the `Pads` latches), every rig's `Motion` (`played_out`/`held_ball` read the
//! players'; the umpire, walkers and creatures play theirs), and the `play/` modules' own tick state.
//!
//! [`state_hash`] is a 64-bit FNV-1a over `Game` (constants left out), the players' motions and the tick's pads, fed by
//! their `Debug` text: Rust prints an f32 as its shortest round-trip decimal, so different bits print differently
//! (NaNs aside, which all read "NaN").

// the rollback (M2d) and desync checks (M2g) are their users; only the test calls them yet
#![cfg_attr(not(test), allow(dead_code))]

use super::{Figure, Game, Pads, tick_input::TickPads};
use crate::character::{Motion, Rig};
use bevy::prelude::*;
use std::fmt::{self, Write};
use std::sync::Arc;

/// A per-match constant inside `Game`: copies share it, the hash skips it. Only setup (and a mod's overrides) write
/// it, before any copy exists. A resync's state goes without them (`None` until `decode` puts the local ones in).
pub(super) struct Const<T>(Option<Arc<T>>);

impl<T: Default> Default for Const<T> {
    fn default() -> Self {
        Const(Some(Arc::default()))
    }
}

impl<T> Clone for Const<T> {
    fn clone(&self) -> Self {
        Const(self.0.clone())
    }
}

impl<T> std::ops::Deref for Const<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_deref().expect("a per-match constant left out of a resync (snapshot::decode)")
    }
}

impl<T> std::ops::DerefMut for Const<T> {
    fn deref_mut(&mut self) -> &mut T {
        Arc::get_mut(self.0.as_mut().expect("a per-match constant")).expect("a per-match constant written after a snapshot was taken")
    }
}

impl<T> fmt::Debug for Const<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("_")
    }
}

impl<T> From<T> for Const<T> {
    fn from(t: T) -> Self {
        Const(Some(Arc::new(t)))
    }
}

impl<T> serde::Serialize for Const<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_unit()
    }
}

impl<'de, T> serde::Deserialize<'de> for Const<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        <()>::deserialize(d).map(|()| Const(None))
    }
}

/// FNV-1a over everything written to it.
struct Fnv(u64);

impl Write for Fnv {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x100_0000_01b3);
        }
        Ok(())
    }
}

/// The match's 64-bit state hash this tick: `Game` without its constants, the players' motions, the tick's pads.
/// ponytail: Debug text through FNV (~0.1 ms a tick); a hand-written field walk if netplay ever measures it.
pub fn state_hash(w: &mut World) -> u64 {
    fnv(&state_text(w))
}

pub fn fnv(text: &str) -> u64 {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    let _ = h.write_str(text);
    h.0
}

/// What `state_hash` hashes (a desync's report keeps it, to diff the machines'). A motion's drawn clock goes as its
/// gameplay one (B556): it runs a mod's own clip, which a peer without the mod's looks doesn't have, and only draws.
pub fn state_text(w: &mut World) -> String {
    let mut figures: Vec<_> = w.query::<(&Figure, &Motion)>().iter(w).map(|(f, m)| (f.0, Motion { drawn: m.clock, drawn_prev: m.prev, ..*m })).collect();
    figures.sort_by_key(|f| f.0);
    format!("{:?}{:?}{:?}", w.get_resource::<Game>(), figures, w.get_resource::<TickPads>())
}

/// Everything a match tick reads and writes, copied.
pub struct Snapshot {
    game: Option<Game>,
    pads: Option<Pads>,
    tick: Option<TickPads>,
    motions: Vec<(Entity, Motion)>,
    modules: Vec<Box<dyn Fn(&mut World) + Send + Sync>>,
}

// A `play/` module's tick state for a resync (`encode`), in hst-base beside `effects` and `weather`'s
pub(crate) use hst_base::Wire;

/// A walk over the `play/` modules' tick state, a resource at a time (`save` copies, `encode`/`decode` bytes).
trait Each {
    fn one<R: Wire>(&mut self);
}

/// The `play/` modules' tick state.
fn modules(e: &mut impl Each) {
    // the instant replay's recorder and cameras
    e.one::<super::instant_replay::Recorder>();
    e.one::<super::replay_camera::ReplayCamera>();
    e.one::<super::cutaway::Director>();
    e.one::<super::foot_fx::FootFx>();
    e.one::<super::match_stats::Serving>();
    e.one::<super::panel::Clock>();
    e.one::<super::surprise::Pops>();
    e.one::<crate::weather::Weather>();
    e.one::<super::reaction_voices::Frames>();
    e.one::<super::bodyhit::Hits>();
    e.one::<super::npcs::Npcs>();
    e.one::<super::popups::Calls>();
    // the change-ends glide writes the camera (in the hash) from its last view: a rollback mid-glide desynced (B154)
    e.one::<super::change_ends::Swing>();
    e.one::<super::replay_pace::Pace>();
    e.one::<super::bodyhit::Posed>();
    // the effects: the hit sparks draw the sound generator while they live
    e.one::<crate::effects::Impacts>();
    e.one::<crate::effects::HitSparks>();
    e.one::<crate::effects::BallFlight>();
    e.one::<crate::effects::BallBounce>();
    e.one::<crate::effects::LandingMarks>();
    // the sounds' tick number and timers (what was played stays: `heard`)
    e.one::<super::heard::Heard>();
}

/// Each resource the match has, to be copied back as it was.
struct Keep<'a>(&'a World, Vec<Box<dyn Fn(&mut World) + Send + Sync>>);

impl Each for Keep<'_> {
    fn one<R: Wire>(&mut self) {
        if let Some(r) = self.0.get_resource::<R>().cloned() {
            self.1.push(Box::new(move |w| w.insert_resource(r.clone())));
        }
    }
}

pub fn save(w: &mut World) -> Snapshot {
    Snapshot {
        game: w.get_resource::<Game>().cloned(),
        pads: w.get_resource::<Pads>().cloned(),
        tick: w.get_resource::<TickPads>().cloned(),
        motions: w.query_filtered::<(Entity, &Motion), With<Rig>>().iter(w).map(|(e, m)| (e, *m)).collect(),
        modules: {
            let mut k = Keep(w, Vec::new());
            modules(&mut k);
            k.1
        },
    }
}

pub fn restore(w: &mut World, s: &Snapshot) {
    if let Some(g) = &s.game {
        w.insert_resource(g.clone());
    }
    if let Some(p) = &s.pads {
        // B196: who's human stays as it is now (a restart changes it: an older copy, e.g. the instant replay's serve
        // set-up, mustn't bring the old seats back)
        let seats = w.get_resource::<Pads>().map_or(p.seats, |now| now.seats);
        w.insert_resource(Pads { seats, ..p.clone() });
    }
    if let Some(t) = &s.tick {
        w.insert_resource(t.clone());
    }
    for (e, m) in &s.motions {
        if let Some(mut now) = w.get_mut::<Motion>(*e) {
            *now = *m;
        }
    }
    s.modules.iter().for_each(|put| put(w));
}

/// Each resource as bincode (`None`: the match hasn't it).
struct Enc<'a>(&'a World, Vec<Option<Vec<u8>>>);

impl Each for Enc<'_> {
    fn one<R: Wire>(&mut self) {
        self.1.push(self.0.get_resource::<R>().map(|r| bincode::serialize(r).unwrap()));
    }
}

/// `Enc`'s parts back into the world.
struct Dec<'a>(&'a mut World, std::vec::IntoIter<Option<Vec<u8>>>, Result<(), String>);

impl Each for Dec<'_> {
    fn one<R: Wire>(&mut self) {
        match self.1.next().flatten().map(|b| bincode::deserialize::<R>(&b)) {
            Some(Ok(mut r)) => {
                if let Some(l) = self.0.get_resource::<R>() {
                    r.local(l);
                }
                self.0.insert_resource(r)
            }
            Some(Err(e)) => self.2 = Err(format!("{}: {e}", std::any::type_name::<R>())),
            None => drop(self.0.remove_resource::<R>()),
        }
    }
}

/// The match state for another machine (B154b's resync): what `save` keeps, as bytes, the entities by `Figure` (the
/// players' motions; the umpire's, walkers' and creatures' stay as they are) and the devices' latches (`Pads`) left
/// out. An instant replay's pads still to play go along (only while one plays: else each tick brings its own), and the
/// state its serve set-up kept (the replay puts it back), encoded the same way.
pub fn encode(w: &mut World) -> Vec<u8> {
    let serve = super::instant_replay::serve(w).map(|s| {
        let now = save(w);
        restore(w, &s);
        let b = encode_now(w);
        restore(w, &now);
        b
    });
    bincode::serialize(&(encode_now(w), serve)).unwrap()
}

/// `encode`'s state into this match: the per-match constants (`Const`) stay this machine's own.
pub fn decode(w: &mut World, bytes: &[u8]) -> Result<(), String> {
    let (now, serve): (Vec<u8>, Option<Vec<u8>>) = bincode::deserialize(bytes).map_err(|e| e.to_string())?;
    let serve = match serve {
        Some(b) => {
            let here = save(w);
            decode_now(w, &b)?;
            let s = save(w);
            restore(w, &here);
            Some(Arc::new(s))
        }
        None => None,
    };
    decode_now(w, &now)?;
    super::instant_replay::set_serve(w, serve);
    Ok(())
}

fn encode_now(w: &mut World) -> Vec<u8> {
    let mut figures: Vec<_> = w.query::<(&Figure, &Motion)>().iter(w).map(|(f, m)| (f.0, *m)).collect();
    figures.sort_by_key(|f| f.0);
    let replay = super::instant_replay::playing(w).then(|| w.get_resource::<super::tick_input::Replay>()).flatten();
    let mut e = Enc(w, vec![
        w.get_resource::<Game>().map(|g| bincode::serialize(g).unwrap()),
        w.get_resource::<TickPads>().map(|t| bincode::serialize(t).unwrap()),
        Some(bincode::serialize(&figures).unwrap()),
        replay.map(|r| bincode::serialize(r).unwrap()),
    ]);
    modules(&mut e);
    bincode::serialize(&e.1).unwrap()
}

fn decode_now(w: &mut World, bytes: &[u8]) -> Result<(), String> {
    let parts: Vec<Option<Vec<u8>>> = bincode::deserialize(bytes).map_err(|e| e.to_string())?;
    let mut parts = parts.into_iter();
    let mut next = || parts.next().flatten().ok_or("a part missing");
    let mut g: Game = bincode::deserialize(&next()?).map_err(|e| format!("Game: {e}"))?;
    let tick: TickPads = bincode::deserialize(&next()?).map_err(|e| format!("TickPads: {e}"))?;
    let figures: Vec<(usize, Motion)> = bincode::deserialize(&next()?).map_err(|e| format!("motions: {e}"))?;
    let replay = parts.next().flatten();
    let local = w.get_resource::<Game>().ok_or("no match")?;
    g.serve_tables = local.serve_tables.clone();
    g.specials = local.specials.clone();
    g.smash_tables = local.smash_tables.clone();
    g.rally_tables = local.rally_tables.clone();
    g.rally_records = local.rally_records.clone();
    g.timing = local.timing.clone();
    g.serve_data = local.serve_data.clone();
    g.world = local.world.clone();
    g.pelvis = local.pelvis.clone();
    g.data = local.data.clone();
    w.insert_resource(g);
    w.insert_resource(tick);
    for (f, mut m) in w.query::<(&Figure, &mut Motion)>().iter_mut(w) {
        if let Some((_, theirs)) = figures.iter().find(|t| t.0 == f.0) {
            *m = *theirs;
        }
    }
    let mut d = Dec(w, parts.collect::<Vec<_>>().into_iter(), Ok(()));
    modules(&mut d);
    d.2?;
    if let Some(r) = replay {
        w.insert_resource(bincode::deserialize::<super::tick_input::Replay>(&r).map_err(|e| format!("Replay: {e}"))?);
    }
    Ok(())
}

/// Save → 300 ticks → restore → the same 300 ticks: the same hash every tick, on the app's singles match driven by
/// a PCSX2 recording's 1P pads (`context/fixtures/round1.bin`; needs the ISO at the repository root, skipped without).
#[cfg(test)]
mod tests {
    use super::super::*;
    use super::{restore, save, state_hash};
    use crate::{Args, GameSpace};
    use std::path::Path;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
    const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures/round1.bin");

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { seats: Some([true, false, false, false]), autoplay: false, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        // the modules keeping their own tick state; only the setup and fixed schedules run (no window, no sound)
        app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, weather::plugin, heard::plugin));
        app.add_systems(PostStartup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        app
    }

    /// B71: every tiebreak point plays court, crowd and racket sounds like any other point.
    #[test]
    fn tiebreak_points_sound() {
        if !Path::new(ISO).is_file() {
            return;
        }
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 1, pads: None });
        app.insert_resource(Pads { seats: Some([true, false, false, false]), autoplay: true, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, weather::plugin, heard::plugin));
        app.add_systems(PostStartup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
        let mut iso = hst_data::iso::Iso::open(ISO).unwrap();
        let sound = crate::audio::Sound::test(&mut iso);
        app.insert_resource(crate::audio::court_bank(&mut iso, 1));
        app.insert_resource(crate::audio::GalleryBank(crate::audio::gallery_bank(&mut iso, 1, 0).map(std::sync::Arc::new)));
        app.insert_resource(sound.clone());
        app.add_systems(FixedUpdate, play_sounds.after(held_ball));
        app.add_plugins(instant_replay::plugin);
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        app.world_mut().resource_mut::<Game>().humans[0] = true;
        let (mut last, mut plays, mut tiebreak_points) = (String::new(), 0, 0);
        for _ in 0..60 * 60 * 25 {
            run(&mut app, &[[tick_input::IDLE; 4]]);
            let mut g = app.world_mut().resource_mut::<Game>();
            // the first game goes to the other side so the one-game match reaches a tiebreak at 1-1
            if g.score.games == [1, 0] && g.phase == Phase::Serve {
                g.score.games = [0, 1];
            }
            let now = format!("{:?} {:?}", g.score.points, g.score.games);
            if now != last {
                if g.score.tiebreak && !last.is_empty() {
                    assert!(sound.plays() > plays, "tiebreak point {now} played no sound");
                    tiebreak_points += 1;
                }
                (last, plays) = (now, sound.plays());
                if g.score.match_over {
                    break;
                }
            }
        }
        assert!(tiebreak_points > 3, "reached the tiebreak ({tiebreak_points} points)");
    }

    fn run(app: &mut App, pads: &[[hst_sim::replay::Pad; 4]]) -> Vec<u64> {
        pads.iter()
            .map(|p| {
                app.insert_resource(tick_input::Replay([*p].into()));
                app.world_mut().run_schedule(FixedUpdate);
                state_hash(app.world_mut())
            })
            .collect()
    }

    /// The drawing entities and materials of the landing markers and the call pop-ups.
    fn views(w: &bevy::prelude::World) -> Vec<(Entity, usize)> {
        let marks = w.resource::<effects::LandingMarks>().views().map(|v| (v.root, v.materials.len()));
        marks.into_iter().chain(w.resource::<popups::Calls>().views()).collect()
    }

    /// B556: a resync's state keeps this machine's own marker and call-model entities and materials (they went as
    /// the host's entities with none: no red or yellow landing marker after a resync).
    #[test]
    fn a_resync_keeps_the_local_marker_views() {
        if !Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = app();
        run(&mut app, &[[tick_input::IDLE; 4]; 30]);
        let before = views(app.world());
        assert!(before.iter().all(|&(_, m)| m > 0), "{before:?}");
        // the state as a machine sends it (its materials never go on the wire)
        let data = super::encode(app.world_mut());
        super::decode(app.world_mut(), &data).unwrap();
        assert_eq!(views(app.world()), before);
    }

    #[test]
    fn save_run_restore_rerun() {
        let (Ok(data), true) = (std::fs::read(FIXTURE), Path::new(ISO).is_file()) else {
            return eprintln!("no ISO or round1.bin, skipped");
        };
        let pads: Vec<_> = hst_sim::replay::frames(&data).iter().take(1200).map(|f| [tick_input::recorded(f.pad(0)), tick_input::IDLE, tick_input::IDLE, tick_input::IDLE]).collect();
        let mut app = app();
        // into the rally, then save, 300 ticks, restore, the same 300 again
        let mut at = 0;
        let mut last = None;
        for start in [300, 600, 900] {
            run(&mut app, &pads[at..start]);
            let s = save(app.world_mut());
            let before = state_hash(app.world_mut());
            let first = run(&mut app, &pads[start..start + 300]);
            restore(app.world_mut(), &s);
            assert_eq!(state_hash(app.world_mut()), before, "restore didn't bring the state back");
            let again = run(&mut app, &pads[start..start + 300]);
            assert_eq!(first, again, "re-running from the snapshot diverged");
            assert!(first.windows(2).any(|w| w[0] != w[1]), "the state never changed");
            at = start + 300;
            last = Some((s, first));
        }
        // other pads from the same snapshot: another match
        let (s, a) = last.unwrap();
        restore(app.world_mut(), &s);
        let b = run(&mut app, &[[tick_input::IDLE; 4]; 300]);
        assert_ne!(a, b, "the hash didn't see the input");
    }

    /// M2d: a rollback re-running the last 8 ticks every tick starts exactly the sounds a straight run starts, tick
    /// for tick, and ends in the same state.
    #[test]
    fn rerun_ticks_play_nothing_twice() {
        let (Ok(data), true) = (std::fs::read(FIXTURE), Path::new(ISO).is_file()) else {
            return eprintln!("no ISO or round1.bin, skipped");
        };
        let pads: Vec<_> = hst_sim::replay::frames(&data).iter().take(900).map(|f| [tick_input::recorded(f.pad(0)), tick_input::IDLE, tick_input::IDLE, tick_input::IDLE]).collect();
        let sounding = || {
            let mut app = app();
            let mut iso = hst_data::iso::Iso::open(ISO).unwrap();
            let sound = crate::audio::Sound::test(&mut iso);
            // the test's court 0 has no sounds: court 1's
            app.insert_resource(crate::audio::court_bank(&mut iso, 1));
            app.insert_resource(crate::audio::GalleryBank(crate::audio::gallery_bank(&mut iso, 1, 0).map(std::sync::Arc::new)));
            app.insert_resource(sound.clone());
            app.add_systems(FixedUpdate, play_sounds.after(held_ball));
            (app, sound)
        };
        let (mut a, sa) = sounding();
        let (mut b, sb) = sounding();
        let mut want = Vec::new();
        for p in &pads {
            run(&mut a, &[*p]);
            want.push(sa.plays());
        }
        // into the point, then every tick: back to 8 ticks ago and those 8 again
        run(&mut b, &pads[..300]);
        let mut snaps = std::collections::VecDeque::new();
        for t in 300..pads.len() {
            snaps.push_back(save(b.world_mut()));
            run(&mut b, &pads[t..=t]);
            if snaps.len() == 8 {
                restore(b.world_mut(), &snaps.pop_front().unwrap());
                run(&mut b, &pads[t - 7..=t]);
            }
            assert_eq!(sb.plays(), want[t], "tick {t}: sounds started {} times, a straight run {}", sb.plays(), want[t]);
        }
        assert!(want[pads.len() - 1] - want[299] > 15, "too few sounds to tell");
        assert_eq!(state_hash(a.world_mut()), state_hash(b.world_mut()), "the re-runs ended somewhere else");
    }

    /// B132: a press's approach runs exactly its planned frames, the search frame doesn't move (as the original,
    /// `context/b132`: N runs, then the stroke state on a frame of its own), and a planned approach locks.
    #[test]
    fn approach_runs_its_frames_then_locks() {
        let (Ok(data), true) = (std::fs::read(FIXTURE), Path::new(ISO).is_file()) else {
            return eprintln!("no ISO or round1.bin, skipped");
        };
        let frames = hst_sim::replay::frames(&data);
        let mut app = app();
        let (mut planned, mut runs, mut approaches) = (None::<u32>, 0, 0);
        for f in &frames {
            let before = app.world().resource::<Game>().players[0];
            run(&mut app, &[[tick_input::recorded(f.pad(0)), tick_input::IDLE, tick_input::IDLE, tick_input::IDLE]]);
            let p = app.world().resource::<Game>().players[0];
            let moved = p.pos != before.pos;
            match (before.pending, p.pending) {
                (None, Some(n)) if p.approach.is_some() => (planned, runs) = (Some(n + 1), moved as u32),
                (Some(0), None) if planned.is_some() => {
                    assert!(!moved, "the search frame moved the player");
                    assert_eq!(Some(runs), planned, "the approach ran {runs} frames, planned {planned:?}");
                    assert!(p.contact.is_some(), "an approach to a reached ball didn't lock");
                    (planned, approaches) = (None, approaches + 1);
                }
                (Some(_), Some(_)) => runs += moved as u32,
                _ => {}
            }
        }
        assert!(approaches > 0, "round1.bin ran no approach");
        eprintln!("{approaches} approaches");
    }

    /// B59: a whiff ends the run, so the first run frame after it counts the run from 0 (no dash carried over),
    /// as the original (`context/b59`: run 30 → 0 after the whiff; every freed whiff in the recordings, human and
    /// CPU, runs off from 0).
    #[test]
    fn run_restarts_after_a_whiff() {
        let (Ok(data), true) = (std::fs::read(FIXTURE), Path::new(ISO).is_file()) else {
            return eprintln!("no ISO or round1.bin, skipped");
        };
        let mut app = app();
        let mut freed = 0;
        for f in &hst_sim::replay::frames(&data) {
            let before = app.world().resource::<Game>().players.clone();
            run(&mut app, &[[tick_input::recorded(f.pad(0)), tick_input::IDLE, tick_input::IDLE, tick_input::IDLE]]);
            for (b, p) in before.iter().zip(&app.world().resource::<Game>().players) {
                if b.whiff.is_some() && p.whiff.is_none() && p.body.running {
                    assert_eq!(p.body.run, 0, "a run after a whiff carried its count on");
                    freed += 1;
                }
            }
        }
        assert!(freed > 0, "round1.bin ran off no whiff");
        eprintln!("{freed} runs off a whiff");
    }

    /// B173: the human's serve touches the net and drops on its own half: the computer receiver gives the ball up
    /// (no path for it, and the net-touch message holds it) and stands still until the next serve, as the original,
    /// instead of running on to the stand spot and locking a contact. The human's pads are a fixed random walk; its
    /// first net-touched serve comes within the 40000 ticks (seed 7: P4's per-player reach and the merge with main moved
    /// the old seeds').
    #[test]
    fn receiver_gives_up_a_net_touched_serve() {
        if !Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = app();
        let mut r = 7u64;
        let mut next = move || {
            r ^= r << 13; r ^= r >> 7; r ^= r << 17;
            r
        };
        let (mut stick, mut held) = ((0x80, 0x80), None);
        for _ in 0..40000 {
            let n = next();
            if n % 30 == 0 {
                stick = ((next() % 256) as u8, (next() % 256) as u8);
            }
            if n % 5 == 0 {
                stick = (0x80, 0x80);
            }
            let buttons = match next() % 12 {
                0 | 1 => 0x2000,
                2 => 0x8000,
                _ => 0,
            };
            let pad = hst_sim::replay::Pad { buttons, lx: stick.0, ly: stick.1, ..tick_input::IDLE };
            run(&mut app, &[[pad, tick_input::IDLE, tick_input::IDLE, tick_input::IDLE]]);
            let g = app.world().resource::<Game>();
            let b = g.players[1];
            if held.is_none() && g.shots == 1 && g.last_hitter == 0 && g.flight.special_contacts > 0 {
                held = Some(b.pos);
            }
            let Some(at) = held else { continue };
            // a let's re-serve, or a fault that ended the game (the players placed at the change of ends)
            if g.shots == 0 || matches!(g.phase, Phase::ChangeEnds(_)) {
                return;
            }
            assert_eq!(b.pos, at, "the receiver chased a net-touched serve");
            assert!(b.contact.is_none() && b.pending.is_none() && b.swing.is_none(), "the receiver went for it");
        }
        panic!("no net-touched serve by the human");
    }
}
