//! B12: the yellow smash marker belongs to the shot being struck, never the one after it.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{Args, GameSpace};

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// A bot match: at every strike, the search `strike` sets up for the receiver (as if human) over the ball's own
    /// flight places a point for each lob and for no other shot (B12's report: a lob's marker on the next hit).
    #[test]
    fn lob_places_its_own_marker() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 2, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: true, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.init_resource::<instant_replay::Recorder>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, weather::plugin, heard::plugin));
        app.add_systems(PostStartup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball, instant_replay::run).chain());
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        let (mut shots, mut lobs) = (0, 0);
        for _ in 0..60 * 60 * 2 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            if g.shots == shots || g.shots < 2 {
                shots = g.shots;
                continue;
            }
            shots = g.shots;
            // the path as `strike` and `smash_frame` grow it, up to the first bounce
            let mut f = g.flight;
            let mut path = vec![path_entry(&f)];
            while path.last().unwrap().bounces < 1 && path.len() < PATH_MAX {
                f.step(&g.shot, &COURTS[g.court]);
                path.push(path_entry(&f));
            }
            let mut s = SmashSearch::new(&[g.smash_heights[(g.last_hitter as usize + 1) & 1]]);
            s.search(&path);
            let lob = g.shot.class == 1 && g.shot.kind == 3;
            assert_eq!(s.at().is_some(), lob, "shot {shots} (class {}, kind {}) marker {:?}", g.shot.class, g.shot.kind, s.at());
            lobs += lob as u32;
        }
        assert!(lobs >= 2, "only {lobs} lobs");
    }
}
