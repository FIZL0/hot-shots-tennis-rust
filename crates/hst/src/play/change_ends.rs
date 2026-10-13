//! The change-ends camera move (B49): the players are already at their new ends on the phase's first tick; the
//! picture glides from the view shown when the phase starts (the post-point cut-away) to the match camera's
//! view of the new set-up over the phase (`hst_sim::camera::swing`), as the original's camera does on its
//! change-ends message, instead of cutting.

use bevy::prelude::*;
use hst_sim::camera::{Scene, View, swing, swing_weight};
use hst_sim::flow::CHANGE_ENDS;

use super::{Game, Phase};

#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Swing {
    /// The view shown at the end of the last tick.
    last: Option<View>,
    /// The move's start and end views and its pivot.
    run: Option<(View, View, [f32; 3])>,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Swing>()
        .add_systems(FixedUpdate, step.after(super::replay_camera::step).before(super::start_effects));
}

fn step(mut g: ResMut<Game>, mut s: ResMut<Swing>) {
    let Phase::ChangeEnds(n) = g.phase else {
        s.run = None;
        s.last = Some(g.cam.view);
        return;
    };
    if n == CHANGE_ENDS {
        // the end view: the match camera cut to the new set-up (its cut is pending from the placement)
        let mut cam = g.cam;
        let players: Vec<[f32; 3]> = g.players.iter().map(|p| p.pos).collect();
        cam.step(&Scene { players: &players, ball: g.flight.ball.pos });
        let from = s.last.unwrap_or(cam.view);
        // ponytail: pivots on the server's spot; the game's reference frame for it (0x76) wasn't read at the
        // phase's start, only later (where it sat near the server)
        let server = g.players[g.score.server as usize].pos;
        s.run = Some((from, cam.view, [server[0], 0.0, server[2]]));
        g.prev_view = from;
        // the placement's cut doesn't reach the picture: the move is shown instead
        g.cam_cut = false;
    }
    let Some((from, to, pivot)) = s.run else { return };
    // the game counts its 80 down before use (79 on the first tick) and shows the match camera at 0
    g.cam.view = if n >= 2 { swing(&from, &to, swing_weight(n - 1), pivot) } else { to };
    s.last = Some(g.cam.view);
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::Swing;
    use crate::{Args, GameSpace};
    use bevy::mesh::skinning::SkinnedMeshInverseBindposes;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// A bot singles match plays to its first change of ends: the players stand at their new ends from the
    /// phase's first tick, the picture starts from the cut-away shown before it and glides (no cut) to the
    /// match camera, which it reaches on the phase's last tick (skipped without the ISO).
    #[test]
    fn change_of_ends_glides() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        // SAFETY: only `Pads::slot_of` reads it; every player is a bot
        unsafe { std::env::set_var("HST_AUTOPLAY", "1") };
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, super::plugin));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let (mut views, mut to, mut after) = (Vec::new(), None, None);
        for frame in 0..60 * 600 {
            let ends = app.world().resource::<Game>().score.swapped;
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            if let Phase::ChangeEnds(n) = g.phase {
                if views.is_empty() {
                    eprintln!("change of ends at tick {frame}");
                    assert_ne!(ends, g.score.swapped, "ends swapped on the phase's first tick");
                    let at = hst_sim::flow::serve_placement(0, &{ let mut s = g.score.clone(); s.next_point(&g.rules); s }, 0, g.players[0].stance, g.players[0].formation, g.score.swapped);
                    assert_eq!(g.players[0].end, at.facing, "player 0 at the new end");
                }
                views.push((n, g.prev_view, g.cam.view));
                to = app.world().resource::<Swing>().run.map(|r| r.1);
            } else if !views.is_empty() {
                after = Some(g.cam.view);
                break;
            }
        }
        // 80 down to 0, as the game's 81 change-ends ticks
        assert_eq!(views.len() as u32, hst_sim::flow::CHANGE_ENDS + 1, "a whole change-ends phase");
        let to = to.expect("the move ran to its end view");
        let eye = |v: &hst_sim::camera::View| Vec3::from(v.eye);
        // each tick moves on from the last (no cut), and the last shows the match camera
        for w in views.windows(2) {
            assert_eq!(w[1].1, w[0].2, "tick {} starts where {} ended", w[1].0, w[0].0);
        }
        let step = views.windows(2).map(|w| eye(&w[0].2).distance(eye(&w[1].2))).fold(0.0, f32::max);
        let span = eye(&views[0].1).distance(eye(&to));
        eprintln!("glide {span:.2} m, biggest step {step:.2} m");
        assert!(step < span.max(0.5) / 4.0, "no jump: step {step} of {span}");
        assert!(views[views.len() - 2..].iter().all(|v| v.2 == to), "the last two ticks show the match camera");
        let after = after.expect("the serve's first tick");
        assert!(eye(&after).distance(eye(&to)) < 0.05, "the serve set-up continues from it: {after:?} vs {to:?}");
    }
}

impl super::snapshot::Wire for Swing {}
