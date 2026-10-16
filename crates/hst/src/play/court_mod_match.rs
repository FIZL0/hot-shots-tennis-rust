//! M8f6: a court mod (`crate::court_mod`) changes what's seen and heard, never the match.

use super::*;
use crate::{Args, GameSpace};
use std::path::Path;

const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
/// The mods repo's build of Get a Grip's first court (`mods/hst-mods`, the mods repo's `out/mods`).
const MOD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/gag_c01_yamate");

/// A bot singles match on `court` (`umpire_mod.rs`'s test app), with `mod_` drawn and heard over it.
fn app(court: usize, mod_: Option<&crate::mods_court::Court>) -> (App, crate::audio::Sound) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
    app.init_asset::<crate::gs::GsMaterial>();
    app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: true, umpire: 4, sets: 1, games: 1, pads: None });
    app.insert_resource(Pads { seats: Some([true, false, false, false]), autoplay: true, ..default() });
    app.init_resource::<tick_input::TickPads>();
    app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
    app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, weather::plugin, heard::plugin));
    app.add_systems(PostStartup, setup);
    app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
    let mut iso = hst_data::iso::Iso::open(ISO).unwrap();
    let sound = crate::audio::Sound::test(&mut iso);
    app.insert_resource(crate::audio::court_bank(&mut iso, court));
    app.insert_resource(crate::audio::GalleryBank(crate::audio::gallery_bank(&mut iso, court, 0).map(std::sync::Arc::new)));
    app.insert_resource(crate::audio::MusicBanks { jingles: None, bgm: None });
    app.insert_resource(sound.clone());
    app.add_systems(FixedUpdate, play_sounds.after(held_ball));
    crate::court_mod::add(&mut app, mod_.cloned());
    app.finish();
    app.cleanup();
    app.world_mut().run_schedule(Startup);
    app.world_mut().run_schedule(PostStartup);
    (app, sound)
}

/// The mod's court, sky and gimmicks spawn (`Update`'s gimmick loops and relighting only move the mod's own
/// nodes and materials; the test app's other `Update` systems want the full app, so it's left out), its BGM goes in as the match music and its ambience loops, yet the
/// match hashes the same as on its bare layout court tick for tick through the first points (skipped without the
/// ISO or the mods repo's build).
#[test]
fn a_court_mod_is_seen_and_heard_and_never_moves_the_match() {
    if !Path::new(ISO).is_file() || !Path::new(MOD).is_dir() {
        return eprintln!("no ISO or no {MOD}, skipped");
    }
    let court = crate::mods_court::read(Path::new(MOD)).unwrap();
    let n = court.layout as usize + 1;
    let (mut disc, _) = app(n, None);
    let (mut modded, sound) = app(n, Some(&court));
    let meshes = |app: &mut App| app.world_mut().query::<&Mesh3d>().iter(app.world()).count();
    assert!(meshes(&mut modded) > meshes(&mut disc) + 400, "the mod's court wasn't drawn");
    // B436: the layout court's trigger creatures (court 6's six fences and its bear) run but stand under the hidden
    // disc court, not on the mod's
    let mut q = modded.world_mut().query_filtered::<&ChildOf, With<Motion>>();
    let kept = q.iter(modded.world()).filter(|c| modded.world().get::<crate::court_mod::DiscCourt>(c.parent()).is_some()).count();
    assert!(n != 6 || kept == 7, "{kept} of court 6's 7 trigger creatures hidden");
    let (mut points, mut last) = (0, String::new());
    for t in 0..60 * 60 * 2 {
        let hash = |app: &mut App| {
            app.insert_resource(tick_input::Replay([[tick_input::IDLE; 4]].into()));
            app.world_mut().run_schedule(PreUpdate);
            app.world_mut().run_schedule(FixedUpdate);
            super::snapshot::state_hash(app.world_mut())
        };
        assert_eq!(hash(&mut disc), hash(&mut modded), "tick {t}: the court mod moved the match");
        if t == 0 {
            let music = modded.world().resource::<crate::audio::MusicBanks>();
            assert!(music.bgm.as_ref().is_some_and(|b| b.0.stream.is_some()), "the mod's BGM isn't the match music");
            assert!(sound.looping() > 0, "the mod's ambience isn't looping");
        }
        let g = modded.world().resource::<Game>();
        let now = format!("{:?} {:?}", g.score.points, g.score.games);
        if now != last {
            points += !last.is_empty() as u32;
            last = now;
        }
        if points == 2 && g.phase == Phase::Serve {
            break;
        }
    }
    assert!(points >= 2, "only {points} points played");
    // M8f7: the mod's people stand there, reacted to those points and have their cue sounds (a mods repo build
    // without its `npcs.json` skips this part)
    if court.people.is_none() {
        return eprintln!("{MOD} has no npcs.json: its people unchecked");
    }
    let (placed, reacted, sounds) = npcs::court_people_seen(modded.world_mut());
    let want = court.people.as_ref().map_or(0, |p| p.people.len());
    assert!(want > 0 && placed == want, "{placed} of the mod's {want} people placed");
    assert_eq!((reacted, sounds), (placed, true), "the people didn't all react, or have no sounds");
}
