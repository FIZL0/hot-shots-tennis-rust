//! P38: what one 60 Hz tick of `control` costs in a 4-bot doubles match, and a hash of the whole match state over the
//! same ticks, so a speed-up can show it changed nothing. Run before and after a change and compare the hash:
//! `tools/check.sh -p hst four_bot_control_cost -- --ignored --nocapture` (needs the ISO at the repository root).
#[cfg(test)]
mod tests {
    use super::super::*;
    use super::super::snapshot::{fnv, state_hash};
    use crate::{Args, GameSpace};
    use std::path::Path;
    use std::time::{Duration, Instant};

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    #[derive(Resource, Default)]
    struct Spent(Duration);

    fn timed(g: ResMut<Game>, pads: Res<Pads>, tick: Res<tick_input::TickPads>, mut spent: ResMut<Spent>) {
        let t = Instant::now();
        control(g, pads, tick);
        spent.0 += t.elapsed();
    }

    #[test]
    #[ignore]
    fn four_bot_control_cost() {
        if !Path::new(ISO).is_file() {
            return;
        }
        let ticks: usize = std::env::var("HST_TICKS").ok().and_then(|s| s.parse().ok()).unwrap_or(20_000);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: false, chars: vec![6, 1, 2, 3], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 6, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: false, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.init_resource::<Spent>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, weather::plugin, heard::plugin));
        app.add_systems(PostStartup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), timed, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        let mut all = String::new();
        let total = Instant::now();
        for _ in 0..ticks {
            app.insert_resource(tick_input::Replay([[tick_input::IDLE; 4]].into()));
            app.world_mut().run_schedule(FixedUpdate);
            all += &format!("{:x}\n", state_hash(app.world_mut()));
        }
        let g = app.world().resource::<Game>();
        let spent = app.world().resource::<Spent>().0;
        println!(
            "{ticks} ticks, score {:?} {:?}, shots {}: control {:.1} µs/tick, whole tick {:.1} µs (with hashing), match hash {:016x}",
            g.score.games,
            g.score.points,
            g.shots,
            spent.as_secs_f64() * 1e6 / ticks as f64,
            total.elapsed().as_secs_f64() * 1e6 / ticks as f64,
            fnv(&all)
        );
    }
}

