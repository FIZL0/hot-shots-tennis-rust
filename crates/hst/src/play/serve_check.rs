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
        let mut app = set_up(vec![6, 1, 2, 3]);
        for _ in 0..600 {
            let g = app.world().resource::<Game>();
            if g.phase == Phase::Serve && g.score.server == 0 && g.serving.t > 30 {
                return app;
            }
            app.world_mut().run_schedule(FixedUpdate);
        }
        panic!("1P never served");
    }

    /// A doubles match of these characters with 1P human, set up.
    fn set_up(chars: Vec<usize>) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: false, chars, outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { seats: Some([true, false, false, false]), autoplay: false, ..default() });
        app.insert_resource(Replay(Default::default()));
        app.init_resource::<super::super::tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, super::super::tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        app
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

    /// As `context/b29/run3/D_early.bin`: ✕ (strong toss), ✕ again 50 ticks later, 12 frames from contact. The
    /// serve's contact IK steps Carol 0.0018936 along x over the countdown, a twelfth a tick from the press's tick
    /// (weight 0), bit for bit as the original's samples.
    #[test]
    fn early_serve_steps() {
        if !Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = to_serve();
        // the save's Carol is right-handed (the disc's is a lefty)
        app.world_mut().resource_mut::<Game>().players[0].hand = 1.0;
        let mut xs = vec![];
        for t in 0..63 {
            let b = if t < 5 || (50..55).contains(&t) { 0x2000 } else { 0 };
            app.world_mut().resource_mut::<Replay>().0.push_back(pad(b));
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            if t >= 50 {
                xs.push((g.serving.swing.map(|sw| sw.left), g.players[0].served.is_some(), g.players[0].pos[0], g.players[0].pos[2]));
            }
        }
        // the original's x per sample from the press (D_early @6504..6516); z stays −12.25
        let want: [f32; 13] = [3.0, 3.0001576, 3.0003154, 3.0004733, 3.000631, 3.000789, 3.0009468, 3.0011046, 3.0012622, 3.00142, 3.0015779, 3.0017357, 3.0018935];
        assert_eq!((xs[0].0, xs[12].1), (Some(12), true), "press at 50, contact 12 ticks on: {xs:?}");
        assert_eq!(xs.iter().map(|r| r.2).collect::<Vec<_>>(), want, "{xs:?}");
        assert!(xs.iter().all(|r| r.3 == -12.25));
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

    /// B26: each player's serve data is their own character's, as slot 3's RAM (characters 6, 4, 3, 11) holds it:
    /// contact windows, reach, powers and both tosses' timing tables (once character 0's for everyone).
    #[test]
    fn serve_data_per_character() {
        let ram = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/ram/s03.bin");
        let (Ok(ram), true) = (std::fs::read(ram), Path::new(ISO).is_file()) else {
            return eprintln!("no ISO or s03.bin, skipped");
        };
        let u = |a: usize| u32::from_le_bytes(ram[(a & 0x1ff_ffff)..][..4].try_into().unwrap()) as usize;
        let f = |a: usize| f32::from_bits(u(a) as u32);
        let chars: Vec<usize> = (0..4).map(|p| ram[0x2ef7f4 + 12 * p] as usize).collect();
        let app = set_up(chars.clone());
        let g = app.world().resource::<Game>();
        for p in 0..4 {
            let obj = u(u(0x422f80) + 0xa8 + 4 * p);
            let d = &g.serve_data[p];
            let table = |g: usize, b: usize, n: usize| -> (Vec<u8>, Vec<i32>) {
                let n = u(obj + n);
                (ram[(obj + g) & 0x1ff_ffff..][..n].to_vec(), (0..n).map(|k| u(obj + b + 4 * k) as i32).collect())
            };
            let at = |o: [usize; 3]| o.map(|o| f(obj + o));
            assert_eq!(
                (d.over, d.under, d.reach, d.power, d.low_power, d.short_miss),
                (at([0x13cc, 0x13d0, 0x13d4]), at([0x13d8, 0x13dc, 0x13e0]), [0x1364, 0x1368].map(|o| u(obj + o) as i32), u(obj + 0x12e4) as i32, u(obj + 0x1398) as i32, f(obj + 0x13a8)),
                "p{p} character {}",
                chars[p]
            );
            assert_eq!((d.strong_grades.clone(), d.strong_bias.clone()), table(0x1644, 0x1554, 0x1680), "p{p} strong");
            assert_eq!((d.weak_grades.clone(), d.weak_bias.clone()), table(0x1774, 0x1684, 0x17b0), "p{p} weak");
        }
    }

    /// B14: a left-handed server tosses with the mirrored hand: Carol's toss leaves her hand and peaks on the
    /// opposite side of her facing line from the right-handed Carol's, at the same height and depth (the sim's
    /// release and apex are bit-exact against the original's for the lefty in `hst-sim` `match_s05_serves`).
    #[test]
    fn lefty_toss_mirrored() {
        if !Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        // (across, up, ahead) of the ball from the server, in the server's frame, at release and at the apex
        let toss = |hand: f32| {
            let mut app = to_serve();
            app.world_mut().resource_mut::<Game>().players[0].hand = hand;
            let mut out = vec![];
            for t in 0..80 {
                app.world_mut().resource_mut::<Replay>().0.push_back(pad(if t < 5 { 0x4000 } else { 0 }));
                app.world_mut().run_schedule(FixedUpdate);
                let g = app.world().resource::<Game>();
                let (p, b) = (&g.players[0], g.flight.ball);
                let [fx, _, fz, _] = p.body.face.dir;
                let d = [b.pos[0] - p.pos[0], b.pos[1] - p.pos[1], b.pos[2] - p.pos[2]];
                // release, then the top of the toss (velocity's y turns: game space is y-down)
                if g.serving.tossed && (out.is_empty() || (out.len() == 1 && b.vel[1] >= 0.0)) {
                    out.push([d[0] * fz - d[2] * fx, d[1], d[0] * fx + d[2] * fz]);
                }
            }
            assert_eq!(out.len(), 2, "hand {hand}: no toss");
            out
        };
        let (left, right) = (toss(-1.0), toss(1.0));
        eprintln!("lefty {left:?}\nrighty {right:?}");
        assert!(right[0][0].abs() > 0.1, "the toss leaves from the body's side: {right:?}");
        for (l, r) in left.iter().zip(&right) {
            assert!((l[0] + r[0]).abs() < 1e-4 && (l[1] - r[1]).abs() < 1e-4 && (l[2] - r[2]).abs() < 1e-4, "lefty {l:?} vs righty {r:?}");
        }
    }
}
