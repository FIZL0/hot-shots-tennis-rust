//! The ball's draw as the original's: the ball model's scale, its shadow and its outline.
//!
//! - The ball model (`ball1.mdl`) at the ball object's scale (2.0), grown with view depth in the serve, the rally
//!   and after the point so it keeps a least size on screen (`hst_sim::shade::ball_scale`).
//! - The shadow (`ballshadow.mdl`, a soft black disc) on the ground under the ball (`shade::ground`; a ray miss off
//!   the court leaves it where it was), turned to face the camera and stretched towards it with distance
//!   (`ball_shadow`, `ball_shadow_stretch`), at 2.5 × the ball object's scale, alpha 0.7 × the ball's.
//! - The outline: a second `ballshadow.mdl` standing at the ball facing the camera (`ball_outline`), alpha 1 × the
//!   ball's, drawn only at ball alpha 1; the ball model in front of it covers its middle.
//!
//! Both discs are GS draws (fog, ABE, TEST 25) like the ball's. The ball's alpha only drops below 1 in practice
//! mode (its fade-out), which the app doesn't have, so it is 1 here.

use bevy::prelude::*;
use hst_sim::shade;

use super::{BallView, Game, Phase, draw};
use crate::gs::GsMaterial;

/// The ball object's scale in the game.
const BALL_SCALE: f32 = 2.0;
/// The shadow's scale over the ball object's, and its alpha.
const SHADOW_SCALE: f32 = 2.5;
const SHADOW_ALPHA: f32 = 0.7;

/// The outline billboard.
#[derive(Component)]
struct Outline;

/// A disc's GS draw whose material alpha has been scaled by its draw alpha.
#[derive(Component)]
struct Faded;

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, outline.after(super::setup)).add_systems(Update, (fade, place.after(draw)));
}

/// Spawn the outline as a copy of the shadow's parts; both go through the ball's GS swap (`shade::Ball`), which also
/// gives them the court's fog.
// ponytail: `shade::Ball` also looks up the sun-shade light scale for them; it only scales the lit colour of a black disc
fn outline(mut commands: Commands, views: Query<(Entity, &BallView, &ChildOf)>, children: Query<&Children>, parts: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>) {
    for (e, v, up) in &views {
        if !v.0 {
            continue;
        }
        commands.entity(e).insert(crate::shade::Ball);
        let o = commands.spawn((Outline, crate::shade::Ball, Transform::default(), Visibility::default(), ChildOf(up.parent()))).id();
        for c in children.iter_descendants(e) {
            if let Ok((mesh, m)) = parts.get(c) {
                commands.spawn((mesh.clone(), m.clone(), ChildOf(o)));
            }
        }
    }
}

/// Scale each disc draw's material alpha by the shadow's or the outline's alpha, once it has its GS draws.
fn fade(
    mut commands: Commands,
    views: Query<(Entity, Option<&BallView>), Or<(With<BallView>, With<Outline>)>>,
    children: Query<&Children>,
    parts: Query<&MeshMaterial3d<GsMaterial>, Without<Faded>>,
    mut materials: ResMut<Assets<GsMaterial>>,
) {
    for (e, v) in &views {
        let alpha = match v {
            Some(BallView(true)) => SHADOW_ALPHA,
            Some(BallView(false)) => continue,
            None => 1.0,
        };
        for c in children.iter_descendants(e) {
            let Ok(h) = parts.get(c) else { continue };
            if let Some(mut m) = materials.get_mut(&h.0) {
                m.uniform.color.w *= alpha;
                // TFX as `GsMaterial::for_batch` picks it: HIGHLIGHT2 only at colour alpha 0x80
                m.key.modulate |= (m.uniform.color.w * 128.0) as i32 != 0x80;
                commands.entity(c).insert(Faded);
            }
        }
    }
}

/// Scale the ball and place the shadow and the outline (game space, under the court root) for the drawn ball and camera.
fn place(
    g: Res<Game>,
    time: Res<Time<Fixed>>,
    world: Option<Res<crate::shade::Shade>>,
    mut balls: Query<(&BallView, &mut Transform), Without<Outline>>,
    mut outlines: Query<&mut Transform, With<Outline>>,
) {
    let a = time.overstep_fraction();
    let mix = |x: [f32; 3], y: [f32; 3]| {
        let p = Vec3::from(x).lerp(Vec3::from(y), a);
        [p.x, p.y, p.z, 1.0]
    };
    let (pos, eye) = (mix(g.prev_ball, g.flight.ball.pos), mix(g.prev_view.eye, g.cam.view.eye));
    let view = &g.cam.view;
    let depth = view.local([pos[0], pos[1], pos[2]])[2];
    let t = view.fov.tan();
    let ball = shade::ball_scale(BALL_SCALE, depth, t, !matches!(g.phase, Phase::ChangeEnds(_)));
    let [dx, dy, dz] = view.rot[1];
    let (outline, m) = shade::ball_outline(ball, depth, t, pos, eye, [dx, dy, dz, 0.0]);
    let [x, y, z, p] = m.map(Vec4::from);
    for mut tr in &mut outlines {
        *tr = Transform::from_matrix(Mat4::from_cols(x * outline, y * outline, z * outline, p));
    }
    // the drawn court's collision world (the sun-shade's), else the match's
    let court = world.as_ref().map_or(&g.world.0, |s| &s.world);
    let shadow = shade::ground(&court.court, &court.models, pos).map(|(point, normal)| {
        let [x, y, z, t] = shade::ball_shadow(point, normal, eye).map(Vec4::from);
        let s = BALL_SCALE * SHADOW_SCALE;
        Mat4::from_cols(x * s, y * s, z * s * shade::ball_shadow_stretch(t.to_array(), eye), t)
    });
    for (v, mut tr) in &mut balls {
        match (v.0, shadow) {
            (false, _) => tr.scale = Vec3::splat(ball),
            (true, Some(m)) => *tr = Transform::from_matrix(m),
            (true, None) => {}
        }
    }
}
