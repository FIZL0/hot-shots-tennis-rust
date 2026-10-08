//! The ball's shadow (`ballshadow.mdl`, a soft black disc) as the original places it each frame: on the ground under
//! the ball (`hst_sim::shade::ground`; a ray miss off the court leaves it where it was), turned to face the camera and
//! stretched towards it with distance (`ball_shadow`, `ball_shadow_stretch`), at 2.5 × the ball's scale (2.0),
//! alpha-blended at 0.7 × the model's (texel × vertex) alpha.

use bevy::prelude::*;
use hst_sim::shade;

use super::{BallView, Game, draw};

/// The ball object's scale in the game (its draw scale, without the distance growth the ball model gets).
const BALL_SCALE: f32 = 2.0;
/// The shadow's scale over the ball's, and its alpha.
const SHADOW_SCALE: f32 = 2.5;
const SHADOW_ALPHA: f32 = 0.7;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (blend, place.after(draw)));
}

/// Draw the shadow's material blended at the game's alpha instead of alpha-tested.
fn blend(views: Query<(Entity, &BallView), Added<BallView>>, children: Query<&Children>, parts: Query<&MeshMaterial3d<StandardMaterial>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    for (e, v) in &views {
        if !v.0 {
            continue;
        }
        for c in children.iter_descendants(e) {
            if let Some(mut m) = parts.get(c).ok().and_then(|m| materials.get_mut(&m.0)) {
                m.alpha_mode = AlphaMode::Blend;
                let alpha = m.base_color.alpha() * SHADOW_ALPHA;
                m.base_color.set_alpha(alpha);
            }
        }
    }
}

/// Place the shadow (game space, under the court root) for the drawn ball and camera.
fn place(g: Res<Game>, time: Res<Time<Fixed>>, world: Option<Res<crate::shade::Shade>>, mut q: Query<(&BallView, &mut Transform)>) {
    let a = time.overstep_fraction();
    let mix = |x: [f32; 3], y: [f32; 3]| {
        let p = Vec3::from(x).lerp(Vec3::from(y), a);
        [p.x, p.y, p.z, 1.0]
    };
    let (pos, eye) = (mix(g.prev_ball, g.flight.ball.pos), mix(g.prev_view.eye, g.cam.view.eye));
    // ponytail: courts without a sun-shade map have no collision world here; their ground is taken as flat
    let ground = match &world {
        Some(s) => shade::ground(&s.world.court, &s.world.models, pos),
        None => Some(([pos[0], 0.0, pos[2], 1.0], [0.0, -1.0, 0.0, 0.0])),
    };
    let Some((point, normal)) = ground else { return };
    let [x, y, z, t] = shade::ball_shadow(point, normal, eye).map(Vec4::from);
    let s = BALL_SCALE * SHADOW_SCALE;
    let m = Mat4::from_cols(x * s, y * s, z * s * shade::ball_shadow_stretch(t.to_array(), eye), t);
    for (v, mut tr) in &mut q {
        if v.0 {
            *tr = Transform::from_matrix(m);
        }
    }
}
