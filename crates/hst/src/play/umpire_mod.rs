//! M8f3: an umpire mod in the chair (`mods_umpire`). The match sets up with its donor (`--umpire`), slot 5 holding
//! her disc bank; this puts the mod's calls over that bank, so each call plays the mod's clip where it ships one and
//! the donor's take where it doesn't. Sound only: the post-point timeline still waits for the donor's word lengths
//! (`Sound::centre_frames` leaves the mod's takes out), so nothing the match simulates changes.

use std::sync::Arc;

use bevy::prelude::*;

use crate::audio::VoiceBanks;
use crate::mods_umpire::{self, MatchUmpire};

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, over_donor.after(super::setup)).add_systems(Update, over_donor.run_if(not(resource_exists::<Over>)));
}

/// B487: the mod's calls are over the donor's (or there's nothing to put over them).
#[derive(Resource)]
struct Over;

/// Slot 5 (`VoiceBanks` index 4): the donor's bank with the mod's calls over it. B487: in the browser the mod's
/// voices may still be in its storage (an online peer's copy, never wanted before): asked for, and put over the
/// donor's on the frame they're in (the donor's are heard until then).
fn over_donor(mut commands: Commands, ump: Option<Res<MatchUmpire>>, args: Res<crate::Args>, voices: Option<ResMut<VoiceBanks>>, over: Option<Res<Over>>) {
    if over.is_some() {
        return;
    }
    let (Some(ump), Some(mut voices)) = (ump, voices) else { return commands.insert_resource(Over) };
    if !crate::modfs::ready(&ump.0.dir) {
        return;
    }
    commands.insert_resource(Over);
    if ump.0.donor != args.umpire {
        // online: this peer's copy names another donor than the room's; the room's umpire still times the calls
        warn!("umpire mod {}: donor {} here, umpire {} in the match; heard over umpire {}", ump.0.id, ump.0.donor, args.umpire, args.umpire);
    }
    if let Some(slot) = voices.0.get_mut(4)
        && let Some(disc) = slot.as_deref()
    {
        *slot = Some(Arc::new(mods_umpire::bank(disc, &ump.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{Args, GameSpace};
    use std::path::Path;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// A bot singles match on court 1 with umpire 4 heard (`snapshot.rs`'s sounding test app), with `ump` in the chair.
    fn app(ump: Option<&crate::mods_umpire::Umpire>) -> (App, crate::audio::Sound) {
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
        if let Some(u) = ump {
            app.insert_resource(crate::mods_umpire::MatchUmpire(u.clone()));
            app.add_plugins(super::plugin);
        }
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        (app, sound)
    }

    /// A mono PCM16 wav of `n` samples at 22.05 kHz.
    fn wav(n: usize) -> Vec<u8> {
        let samples: Vec<u8> = (0..n).map(|i| ((i as f32 * 0.3).sin() * 12000.0) as i16).flat_map(i16::to_le_bytes).collect();
        let mut w = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        w.extend([16u32.to_le_bytes(), [1, 0, 1, 0], 22050u32.to_le_bytes(), (22050u32 * 2).to_le_bytes(), [2, 0, 16, 0]].concat());
        w.extend(b"data");
        w.extend((samples.len() as u32).to_le_bytes());
        w.extend(samples);
        w
    }

    /// B487: in the browser an umpire mod's voices stay in its storage until wanted (even right after a store
    /// download): the match hears the donor until they're read in, then the mod's calls, never a bank built from the
    /// files it couldn't read yet (skipped without the ISO).
    #[test]
    fn an_umpire_mod_still_in_the_browsers_storage_is_heard_once_read_in() {
        if !Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let dir = std::env::temp_dir().join(format!("hst-umpire-lazy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.json"), r#"{"standard":1,"kind":"umpire","id":"lazy_ump","name":"Gloria","donor":4}"#).unwrap();
        let files = ["voice/1_1.wav", "voice/0_13.wav"].map(|f| (dir.join(f), wav(600)));
        for (f, w) in &files {
            crate::modfs::add_lazy(f, w.len() as u64);
        }
        let ump = crate::mods_umpire::read(&dir).unwrap();
        use bevy::ecs::system::RunSystemOnce;
        let (mut app, _) = app(Some(&ump));
        let takes = |app: &App| app.world().resource::<VoiceBanks>().0[4].as_ref().unwrap().takes.len();
        app.world_mut().run_system_once(super::over_donor).unwrap();
        assert_eq!(takes(&app), 0, "the donor's bank alone while the voices are out");
        // the page reads the folder in
        for (f, w) in files {
            crate::modfs::add(&f, w.into());
        }
        crate::modfs::loaded(&dir.display().to_string());
        app.world_mut().run_system_once(super::over_donor).unwrap();
        assert_eq!(takes(&app), 2, "the mod's calls over the donor's once they're in");
        app.world_mut().run_system_once(super::over_donor).unwrap();
        assert_eq!(takes(&app), 2, "put over once");
        crate::modfs::remove(&dir);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// M8f3: an umpire mod changes what's heard, never the match. Its calls (every key of the call table, the
    /// first-word ones far longer than any disc word, the rest far shorter, plus "deuce again", which no disc umpire
    /// has) play in place of the donor's, yet the match hashes the same as with the donor alone tick for tick through
    /// the first points, so the post-point timeline still runs on her word lengths (skipped without the ISO).
    #[test]
    fn an_umpire_mod_is_heard_and_never_moves_the_match() {
        if !Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let dir = std::env::temp_dir().join(format!("hst-umpire-match-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("voice")).unwrap();
        std::fs::write(dir.join("mod.json"), r#"{"standard":1,"kind":"umpire","id":"test_ump","name":"Test","donor":4}"#).unwrap();
        let calls = (0..14).map(|k| (0, k)).chain((0..5).map(|k| (1, k))).chain((0..5).map(|k| (2, k)));
        for (p, k) in calls {
            let long = p == 0 && k < 4 || p == 2;
            std::fs::write(dir.join(format!("voice/{p}_{k}.wav")), wav(if long { 22050 * 3 } else { 600 })).unwrap();
        }
        let ump = crate::mods_umpire::read(&dir).unwrap();
        let (mut disc, _) = app(None);
        let (mut modded, sound) = app(Some(&ump));
        std::fs::remove_dir_all(&dir).ok();
        let (mut points, mut heard, mut last) = (0, 0, String::new());
        for t in 0..60 * 60 * 3 {
            let hash = |app: &mut App| {
                app.insert_resource(tick_input::Replay([[tick_input::IDLE; 4]].into()));
                app.world_mut().run_schedule(FixedUpdate);
                super::super::snapshot::state_hash(app.world_mut())
            };
            assert_eq!(hash(&mut disc), hash(&mut modded), "tick {t}: the umpire mod moved the match");
            heard = heard.max(sound.takes_sounding());
            let g = modded.world().resource::<Game>();
            let now = format!("{:?} {:?}", g.score.points, g.score.games);
            if now != last {
                points += !last.is_empty() as u32;
                last = now;
            }
            // the third point's call heard out
            if points == 3 && g.phase == Phase::Serve {
                break;
            }
        }
        assert!(points >= 3, "only {points} points played");
        assert!(heard > 0, "the mod's calls were never heard");
    }
}
