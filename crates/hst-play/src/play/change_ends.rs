//! The change-ends camera move (B49): the players are already at their new ends on the phase's first tick; the
//! picture cuts to the first view of scripted shot 2 (behind the player at the match camera's new end) and glides
//! from it round that player to the match camera's view of the new set-up over the phase
//! (`hst_sim::camera::swing`), as the original's camera does on its change-ends message (B174).

use bevy::prelude::*;
use hst_sim::camera::{Scene, View, swing, swing_weight};
use hst_sim::cutaway::{IDENTITY, Shot};
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

pub(super) fn step(mut g: ResMut<Game>, mut s: ResMut<Swing>, d: Option<Res<super::cutaway::Director>>) {
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
        // the pivot: the game's court spot 0x76, the player at the match camera's end, on the ground facing the net
        // (both recorded glides fit it to 0.1 m: slot 5 and the Grand Palais with the camera on the human's side).
        // ponytail: the first player at that end; which of two (doubles) the spot follows wasn't recorded
        let near = g.players.iter().find(|p| p.pos[2] * cam.view.eye[2] > 0.0).unwrap_or(&g.players[g.score.server as usize]).pos;
        let pivot = [near[0], 0.0, near[2]];
        // the start: not the cut-away on screen but the first view of scripted shot 2, which orbits that spot from
        // behind it (frame 0x77 is the same spot at the other end), so the move stays behind the new end's baseline
        // instead of swinging round the court (out through the indoor court's walls)
        let b = if pivot[2] > 0.0 { -1.0 } else { 1.0 };
        let spot = |b: f32, p: [f32; 3]| [[b, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, b, 0.0], [p[0], 0.0, p[2], 1.0]];
        let (here, there) = (spot(b, pivot), spot(-b, [-pivot[0], 0.0, -pivot[2]]));
        let from = d.map_or(s.last.unwrap_or(cam.view), |d| {
            let mut shot = Shot::new(&d.shots, 2, false);
            shot.build(&|i| match i { 0x76 => here, 0x77 => there, _ => IDENTITY });
            shot.view()
        });
        s.run = Some((from, cam.view, pivot));
        g.prev_view = from;
        // the placement's cut doesn't reach the picture: the move is shown instead
        g.cam_cut = false;
    }
    let Some((from, to, pivot)) = s.run else { return };
    // the game counts its 80 down before use (79 on the first tick) and shows the match camera at 0
    g.cam.view = if n >= 2 { swing(&from, &to, swing_weight(n - 1), pivot) } else { to };
    s.last = Some(g.cam.view);
}

/// The player who serves next, through the change of ends (B262): the placement put everyone where the next point
/// has them, but the score only moves on when the phase ends. The original shows that player in the serve stance
/// (motion 0x20), bouncing the ball in hand, over the whole move.
pub(super) fn server(g: &Game) -> Option<usize> {
    let Phase::ChangeEnds(_) = g.phase else { return None };
    let mut s = g.score.clone();
    s.next_point(&g.rules);
    Some(s.server as usize)
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::Swing;
    use crate::{Args, GameSpace};
    use bevy::mesh::skinning::SkinnedMeshInverseBindposes;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// A bot singles match plays to its first change of ends: the players stand at their new ends from the
    /// phase's first tick, the picture glides (no cut after its first tick) to the match camera, which it reaches
    /// on the phase's last tick, staying behind the new end's baseline (skipped without the ISO).
    #[test]
    fn change_of_ends_glides() {
        glide(None, None);
    }

    /// B174: the same on the indoor Grand Palais with the camera following player 0 (the human's view), whose
    /// move ends on the other side of the court from where the post-point cut-away left it: it still starts
    /// behind the new end (the original's shot 2) instead of swinging round the court through the hall's walls.
    #[test]
    fn change_of_ends_glides_on_the_players_side() {
        glide(Some(7), Some(0));
    }

    fn glide(stage: Option<u32>, owner: Option<usize>) {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        // every player a bot
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, super::plugin));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, control, move |mut g: ResMut<Game>| g.cam_owner = owner.or(g.cam_owner), simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let (mut views, mut to, mut after, mut aimed) = (Vec::new(), None, None, false);
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
                // B73: the original's player markers are up through the whole change of ends
                assert!(super::super::markers::shown(g), "markers up at change-ends tick {n}");
                // B193: the last shot's landing markers went with the players' reaction (the original's 0x19)
                assert!(!landing_shown(g), "landing markers hidden at change-ends tick {n}");
                aimed |= g.marks.red.is_some();
                // B262: the next server plays the serve stance at the new end with the ball in hand
                let i = super::server(g).expect("a server through the change of ends");
                let (at, ball) = (g.players[i].pos, g.flight.ball.pos);
                // (y points down: the stance bounces it between about 0.06 and 0.6 m up)
                assert!(ball[1] < -0.05 && (ball[0] - at[0]).hypot(ball[2] - at[2]) < 1.0, "tick {n}: ball {ball:?} not in the hand of {at:?}");
                let w = app.world_mut();
                let m = w.query::<(&Figure, &Motion)>().iter(w).find(|(f, _)| f.0 == i).map(|(_, m)| m.id);
                assert_eq!(m, Some(0x20), "tick {n}: server's motion");
                let g = app.world().resource::<Game>();
                views.push((n, g.prev_view, g.cam.view));
                to = app.world().resource::<Swing>().run.map(|r| r.1);
            } else if !views.is_empty() {
                after = Some(g.cam.view);
                break;
            }
        }
        assert!(aimed, "the point before the change had a red landing marker to hide");
        // 80 down to 0, as the game's 81 change-ends ticks
        assert_eq!(views.len() as u32, hst_sim::flow::CHANGE_ENDS + 1, "a whole change-ends phase");
        let to = to.expect("the move ran to its end view");
        let eye = |v: &hst_sim::camera::View| Vec3::from(v.eye);
        // the eye stays behind the new end's baseline, between the side walls (the original's both recorded
        // glides: |x| under 6.2 m, |z| over 19 m)
        for v in &views[1..] {
            let e = eye(&v.2);
            assert!(e.z * to.eye[2] > 0.0 && e.z.abs() > 11.885 && e.x.abs() < 8.0, "tick {}: eye {e} off the new end", v.0);
        }
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
