//! B479: in doubles a player may take a ball its partner is going for. The original's contact search looks only at
//! which team hit last, never at the partner, and every hit sets each player's contact-cancel flag; recorded in
//! `research/b479_partner_ball.py` (slot 4, P1 driven into a dive falling short while its CPU partner volleys).

#[cfg(test)]
mod tests {
    use super::super::tick_input::{IDLE, Replay};
    use super::super::*;
    use crate::{Args, GameSpace};
    use std::path::Path;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// A doubles match of computer players.
    fn set_up() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: false, chars: vec![6, 1, 2, 3], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: false, ..default() });
        app.insert_resource(Replay(Default::default()));
        app.init_resource::<super::super::tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, super::super::tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        app
    }

    /// Puts player `i` at (x, z), running toward (tx, tz) at run speed.
    fn put(g: &mut Game, i: usize, x: f32, z: f32, to: Option<(f32, f32)>) {
        let p = &mut g.players[i];
        p.pos = [x, p.pos[1], z];
        p.body.pos = p.pos;
        if let Some((tx, tz)) = to {
            let (dx, dz) = (tx - x, tz - z);
            let n = (dx * dx + dz * dz).sqrt();
            let dir = [dx / n, 0.0, dz / n, 0.0];
            p.body.face.dir = dir;
            p.body.target = dir;
            p.body.vel = [dir[0] * 0.11, 0.0, dir[2] * 0.11];
            p.body.running = true;
        }
    }

    /// The other team's ball comes over; player 0 dives for it and falls short, player 2 stands where its path
    /// passes and presses: as in the original, player 2 still locks onto it and strikes it while player 0 slides.
    /// Also checks that a hit cancels a dive that would reach the ball.
    #[test]
    fn a_partner_takes_the_ball_a_dive_falls_short_of() {
        if !Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = set_up();
        let tick = |app: &mut App| {
            app.world_mut().resource_mut::<Replay>().0.push_back([IDLE; 4]);
            app.world_mut().run_schedule(FixedUpdate);
        };
        // the other team's first return, just struck toward team 0
        let (mut ok, mut before) = (false, -1);
        for _ in 0..3000 {
            tick(&mut app);
            let g = app.world().resource::<Game>();
            let fresh = g.last_hitter != before;
            before = g.last_hitter;
            if g.phase == Phase::Rally && g.last_hitter & 1 == 1 && fresh {
                ok = true;
                break;
            }
        }
        assert!(ok, "no ball coming to team 0");
        // each tick as the ball comes in: player 2 where the path is k frames on (its stand-off from the ball as the
        // bot's goal) so its search finds it, player 0 beside it running at the same point, too far to reach it
        let place = |g: &mut Game| -> Option<(usize, swing::Dive)> {
            let path = predicted_path(g, 20);
            let end = g.players[2].end;
            for (k, step) in path.iter().enumerate().take(16).skip(6) {
                let q = step.pos;
                let near = [0.0f32, 0.3, -0.3, 0.6, -0.6].into_iter().any(|dx| {
                    put(g, 2, q[0] + dx, q[2] - end * g.reach.ahead, None);
                    find_contact(g, 2).is_some()
                });
                if !near {
                    continue;
                }
                for side in [1.0f32, -1.0] {
                    for d in 8..40 {
                        let (x, z) = (q[0] + side * 0.25 * d as f32, q[2] - end * g.reach.ahead);
                        put(g, 0, x, z, Some((q[0], q[2])));
                        if let Some(dv) = find_dive(g, 0).filter(|dv| !dv.contact) {
                            return Some((k, dv));
                        }
                    }
                }
            }
            None
        };
        let mut found = None;
        for _ in 0..60 {
            let g = &mut *app.world_mut().resource_mut::<Game>();
            let idle = |p: &Player| p.contact.is_none() && p.pending.is_none() && p.swing.is_none() && p.dive.is_none();
            if g.last_hitter & 1 != 1 || !idle(&g.players[0]) || !idle(&g.players[2]) {
                break;
            }
            found = place(g);
            if found.is_some() {
                break;
            }
            tick(&mut app);
        }
        let Some((k, dive)) = found else {
            panic!("no place found for a short dive beside a partner's stroke");
        };
        let g = &mut *app.world_mut().resource_mut::<Game>();
        // a hit cancels every other lock, a dive's too: one that would reach the ball slides on without striking
        let mut reaching = dive;
        reaching.contact = true;
        g.players[0].dive = Some(reaching);
        cancel_locks(g, Some(2));
        assert_eq!(g.players[0].dive.map(|d| d.contact), Some(false));
        let p = &mut g.players[0];
        p.dive = Some(dive);
        p.body.running = false;
        p.vel = Vec2::ZERO;
        set_motion(p, 0x1e, 1.0, false, None);
        // the old rule: a partner's dive left player 2 nothing to hit
        assert!(find_contact(g, 2).is_some(), "player 2 can't lock while player 0 dives (frame {k})");
        press(g, 2, 0);
        let mut struck = None;
        for t in 0..60 {
            tick(&mut app);
            let g = app.world().resource::<Game>();
            if g.last_hitter == 2 {
                struck = Some((t, g.players[0].dive.is_some()));
                break;
            }
            assert!(g.last_hitter & 1 == 1, "player {} hit it", g.last_hitter);
        }
        let (t, diving) = struck.expect("player 2 never struck");
        assert!(diving, "player 0's dive was over before player 2's strike ({t} ticks)");
    }

}
