//! B29: the human serve's frame timeline through the app (toss press → release → swing press → contact → back to
//! play), for comparing with the original's (`tools/record_serve.py`, `research/serve_timeline.py`).

#[cfg(test)]
mod tests {
    use super::super::tick_input::{IDLE, Replay};
    use super::super::*;
    use crate::{Args, GameSpace};
    use hst_sim::replay::Pad;
    use std::path::Path;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// A doubles match with 1P human (Carol, as slot 3), run to 1P's first serve.
    fn to_serve() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: false, chars: vec![6, 1, 2, 3], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { seats: Some([true, false, false, false]), autoplay: false, ..default() });
        app.insert_resource(Replay(Default::default()));
        app.init_resource::<super::super::tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, super::super::tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        for _ in 0..600 {
            let g = app.world().resource::<Game>();
            if g.phase == Phase::Serve && g.score.server == 0 && g.serving.t > 30 {
                return app;
            }
            app.world_mut().run_schedule(FixedUpdate);
        }
        panic!("1P never served");
    }

    fn pad(buttons: u16) -> [Pad; 4] {
        [Pad { buttons, ..IDLE }, IDLE, IDLE, IDLE]
    }

    /// One row per tick: the port's serve state, server motion and ball, after the press script `script`
    /// ((tick, buttons) held from that tick on).
    /// Also returns each tick's (tossed, swing (frames left, offset), served, server's motion speed).
    fn timeline(script: &[(usize, u16)], ticks: usize) -> (App, Vec<(bool, Option<(u32, i32)>, Option<u32>, f32)>) {
        let mut app = to_serve();
        let mut rows = vec![];
        let mut held = 0;
        for t in 0..ticks {
            if let Some(&(_, b)) = script.iter().find(|(k, _)| *k == t) {
                held = b;
            }
            app.world_mut().resource_mut::<Replay>().0.push_back(pad(held));
            app.world_mut().run_schedule(FixedUpdate);
            let w = app.world_mut();
            let m = w.query::<(&Figure, &Motion)>().iter(w).find(|(f, _)| f.0 == 0).map(|(_, m)| (m.id, m.clock.time, m.clock.speed)).unwrap();
            let g = w.resource::<Game>();
            let s = g.serving;
            let p = &g.players[0];
            eprintln!(
                "{t:4} {:?} faults {} btn {held:04x} {:?} t {} tossed {} swing {:?} whiff {} served {:?} x {:.4} motion {:#x} t {:.2} sp {:.3} ball y {:.7} vy {:.7} b {}",
                g.phase, g.rally.faults, s.toss, s.t, s.tossed as u8, s.swing.map(|sw| (sw.left, sw.offset)), s.whiffed as u8, p.served, p.pos[0], m.0, m.1, m.2,
                g.flight.ball.pos[1], g.flight.ball.vel[1], g.flight.bounces
            );
            rows.push((s.tossed, s.swing.map(|sw| (sw.left, sw.offset)), p.served, m.2));
        }
        (app, rows)
    }

    /// As `context/b29/run2/B_weak.bin`: ○ at 0 (held 5), ○ again 80 ticks later. The original releases the toss 45
    /// ticks after the press, the swing counts 16 frames down (contact offset 8, motion speed 0.5) and hits 16 ticks
    /// after its press.
    #[test]
    fn weak_toss_timeline() {
        if !Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let (_, rows) = timeline(&[(0, 0x4000), (5, 0), (80, 0x4000), (85, 0)], 100);
        let released = rows.iter().position(|r| r.0);
        let served = rows.iter().position(|r| r.2.is_some());
        assert_eq!((released, rows[80].1, rows[80].3, served), (Some(45), Some((16, 8)), 0.5, Some(96)));
    }

    /// A swing press with no contact left before the bounce (as `context/b29/run3/E2_late.bin`: press 119 ticks
    /// after the weak toss's): the original calls it a fault when the toss lands, then sets up the second serve.
    #[test]
    fn whiffed_serve_is_a_fault() {
        if !Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let (app, _) = timeline(&[(0, 0x4000), (5, 0), (119, 0x4000), (124, 0)], 140);
        let g = app.world().resource::<Game>();
        assert_eq!((g.phase.clone(), g.rally.faults), (Phase::Post, 1), "a whiffed serve is a fault");
    }
}
