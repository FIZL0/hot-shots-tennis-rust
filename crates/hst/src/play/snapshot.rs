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
/// it, before any copy exists.
#[derive(Default)]
pub(super) struct Const<T>(Arc<T>);

impl<T> Clone for Const<T> {
    fn clone(&self) -> Self {
        Const(self.0.clone())
    }
}

impl<T> std::ops::Deref for Const<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Const<T> {
    fn deref_mut(&mut self) -> &mut T {
        Arc::get_mut(&mut self.0).expect("a per-match constant written after a snapshot was taken")
    }
}

impl<T> fmt::Debug for Const<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("_")
    }
}

impl<T> From<T> for Const<T> {
    fn from(t: T) -> Self {
        Const(Arc::new(t))
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

/// What `state_hash` hashes (a desync's report keeps it, to diff the machines').
pub fn state_text(w: &mut World) -> String {
    let mut figures: Vec<_> = w.query::<(&Figure, &Motion)>().iter(w).map(|(f, m)| (f.0, *m)).collect();
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

/// The `play/` modules' tick state: each resource copied back as it was, if the match has it.
fn modules(w: &World) -> Vec<Box<dyn Fn(&mut World) + Send + Sync>> {
    fn keep<R: Resource + Clone>(w: &World, out: &mut Vec<Box<dyn Fn(&mut World) + Send + Sync>>) {
        if let Some(r) = w.get_resource::<R>().cloned() {
            out.push(Box::new(move |w| w.insert_resource(r.clone())));
        }
    }
    let mut out = Vec::new();
    // the instant replay's recorder and cameras
    keep::<super::instant_replay::Recorder>(w, &mut out);
    keep::<super::replay_camera::ReplayCamera>(w, &mut out);
    keep::<super::cutaway::Director>(w, &mut out);
    keep::<super::foot_fx::FootFx>(w, &mut out);
    keep::<super::match_stats::Serving>(w, &mut out);
    keep::<super::panel::Clock>(w, &mut out);
    keep::<super::surprise::Pops>(w, &mut out);
    keep::<crate::weather::Weather>(w, &mut out);
    keep::<super::reaction_voices::Frames>(w, &mut out);
    keep::<super::bodyhit::Hits>(w, &mut out);
    keep::<super::npcs::Npcs>(w, &mut out);
    keep::<super::popups::Calls>(w, &mut out);
    // the effects: the hit sparks draw the sound generator while they live
    keep::<crate::effects::Impacts>(w, &mut out);
    keep::<crate::effects::HitSparks>(w, &mut out);
    keep::<crate::effects::BallFlight>(w, &mut out);
    keep::<crate::effects::BallBounce>(w, &mut out);
    keep::<crate::effects::LandingMarks>(w, &mut out);
    // the sounds' tick number and timers (what was played stays: `heard`)
    keep::<super::heard::Heard>(w, &mut out);
    out
}

pub fn save(w: &mut World) -> Snapshot {
    Snapshot {
        game: w.get_resource::<Game>().cloned(),
        pads: w.get_resource::<Pads>().cloned(),
        tick: w.get_resource::<TickPads>().cloned(),
        motions: w.query_filtered::<(Entity, &Motion), With<Rig>>().iter(w).map(|(e, m)| (e, *m)).collect(),
        modules: modules(w),
    }
}

pub fn restore(w: &mut World, s: &Snapshot) {
    if let Some(g) = &s.game {
        w.insert_resource(g.clone());
    }
    if let Some(p) = &s.pads {
        w.insert_resource(p.clone());
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

    fn run(app: &mut App, pads: &[[hst_sim::replay::Pad; 4]]) -> Vec<u64> {
        pads.iter()
            .map(|p| {
                app.insert_resource(tick_input::Replay([*p].into()));
                app.world_mut().run_schedule(FixedUpdate);
                state_hash(app.world_mut())
            })
            .collect()
    }

    #[test]
    fn save_run_restore_rerun() {
        let (Ok(data), true) = (std::fs::read(FIXTURE), Path::new(ISO).is_file()) else {
            return eprintln!("no ISO or round1.bin, skipped");
        };
        let pads: Vec<_> = hst_sim::replay::frames(&data).iter().take(1200).map(|f| [f.pad(0), tick_input::IDLE, tick_input::IDLE, tick_input::IDLE]).collect();
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
        let pads: Vec<_> = hst_sim::replay::frames(&data).iter().take(900).map(|f| [f.pad(0), tick_input::IDLE, tick_input::IDLE, tick_input::IDLE]).collect();
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
}
