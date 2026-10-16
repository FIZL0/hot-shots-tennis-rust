//! B12: the yellow smash marker belongs to the shot being struck, never the one after it. B507: it is only for a
//! human on the receiving team, so a human's own lobs never show it (as in the original).

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{Args, GameSpace};

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// A singles match (chars 6, 1, court 0) with these seats human, the rest bots; set up, not ticked yet.
    fn bot_match(seats: [bool; 4]) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 2, pads: None });
        app.insert_resource(Pads { seats: Some(seats), autoplay: false, ..default() });
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
        app
    }

    /// A bot match: at every strike, the search `strike` sets up for the receiver (as if human) over the ball's own
    /// flight places a point for each lob and for no other shot (B12's report: a lob's marker on the next hit).
    #[test]
    fn lob_places_its_own_marker() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = bot_match([false; 4]);
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

    /// B507 (the user's report: their own lobs show no yellow marker): 1P human, pressing only △ (and ✕ to serve),
    /// so it moves to the ball by itself and lobs; the bot opponent hits all kinds. No strike of 1P's sets up a
    /// smash search, its lobs included; every shot of the opponent's from the return on does, and a lob of theirs
    /// places the marker before it bounces: the original searches only for the humans on the receiving team (the hitter's slot & 1 picks the
    /// team; its own team's shots get no candidates, checked live on slot 4 in the B507 journal).
    #[test]
    fn own_lobs_show_no_marker() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = bot_match([true, false, false, false]);
        let (mut shots, mut own, mut own_lobs, mut their_lobs) = (0, 0, 0, 0);
        // the opponent's lob in the air: whether its marker has started yet
        let mut waiting: Option<bool> = None;
        for t in 0..60 * 60 * 2 {
            // a fresh △ press every 8 ticks, ✕ in between (the serve's toss and hit)
            let buttons = match t % 16 {
                0..4 => 0x1000,
                8..12 => 0x2000,
                _ => 0,
            };
            let pad = hst_sim::replay::Pad { buttons, ..tick_input::IDLE };
            app.insert_resource(tick_input::Replay([[pad, tick_input::IDLE, tick_input::IDLE, tick_input::IDLE]].into()));
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            if let Some(started) = &mut waiting {
                *started |= g.marks.start;
                if g.flight.bounces > 0 || g.shots != shots {
                    assert!(*started, "shot {shots}: the opponent's lob bounced without a marker");
                    waiting = None;
                }
            }
            if g.shots == shots || g.shots < 2 {
                shots = g.shots;
                continue;
            }
            shots = g.shots;
            let lob = g.shot.class == 1 && g.shot.kind == 3;
            if g.last_hitter == 0 {
                assert!(g.marks.smash.is_none(), "shot {shots}: 1P's own shot set up a smash search");
                own += 1;
                own_lobs += lob as u32;
            } else {
                assert!(g.marks.smash.is_some(), "shot {shots}: no smash search for the human receiver");
                if lob {
                    their_lobs += 1;
                    waiting = Some(g.marks.start);
                }
            }
        }
        // the bot rarely lobs back at a lobber (`lob_places_its_own_marker` covers the opponent's lobs)
        assert!(own_lobs >= 2, "1P: {own} shots, {own_lobs} lobs; the opponent's lobs: {their_lobs}");
    }
}
