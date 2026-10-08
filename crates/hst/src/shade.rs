//! The court's sun-shade map (`hst_sim::shade`) on court: built from the shadow casters at load (the game: a few
//! frames after), then each frame the ball's ([`Ball`]) and every NPC's light scale is looked up under it in clear and
//! cloudy weather (1.0 in rain and on court 7) and darkens it. Off the court the ball's height is a ray cast down at
//! the court's collision model; a miss keeps its last scale. The players are never shaded.
//!
//! `HST_SHADE_DUMP=<file>` writes the built map (the game's bit layout) for comparing with a capture.
//!
//! ponytail: models are drawn unlit, so the scale darkens [`DIRECT`] of their colour instead of their
//! directional light; exact once they are VU1-lit (P17s).

use bevy::prelude::*;
use hst_sim::mesh::World;
use hst_sim::shade::{self, Frame};

use crate::character::Rig;

/// The share of a character's colour its directional light gives.
const DIRECT: f32 = 0.5;

/// The court's map frame (from the hole model) and map, and the collision world whose court the ball's ground
/// ray hits.
#[derive(Resource)]
pub struct Shade {
    pub frame: Frame,
    pub map: Vec<u8>,
    pub world: World,
}

/// The ball model: shaded on the ground only.
#[derive(Component)]
pub struct Ball;

/// A rig's or the ball's own materials with their unshaded colours, and the scale last applied.
#[derive(Component)]
struct Lit(Vec<(Handle<StandardMaterial>, LinearRgba)>, f32);

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (own_materials, light).chain());
}

/// Model-space (min, max) over the packet boxes, the box the game frames the map with.
pub fn packet_box(model: &hst_data::mdl::Model) -> ([f32; 3], [f32; 3]) {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for pk in model.materials.iter().flatten() {
        for k in 0..3 {
            lo[k] = lo[k].min(pk.bounds[0][k]);
            hi[k] = hi[k].max(pk.bounds[1][k]);
        }
    }
    (lo, hi)
}

/// The map of the casters' shadows cast along `dir` onto the hole's `ground` triangles (game space).
pub fn build(frame: Frame, dir: Vec3, casters: &[shade::Caster], ground: &[[[f32; 3]; 3]], world: World) -> Shade {
    let map = shade::build(&frame, dir.to_array(), casters, ground);
    if let Ok(path) = std::env::var("HST_SHADE_DUMP") {
        _ = std::fs::write(path, &map);
    }
    Shade { frame, map, world }
}

/// Give every new NPC rig and the ball materials of their own, so each can be shaded apart.
fn own_materials(
    mut commands: Commands,
    rigs: Query<Entity, Or<(Added<Rig>, Added<Ball>)>>,
    children: Query<&Children>,
    mut parts: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for rig in &rigs {
        let mut own = Vec::new();
        for e in children.iter_descendants(rig) {
            let Ok(mut m) = parts.get_mut(e) else { continue };
            let Some(mat) = materials.get(&m.0).cloned() else { continue };
            let base = mat.base_color.to_linear();
            m.0 = materials.add(mat);
            own.push((m.0.clone(), base));
        }
        commands.entity(rig).insert(Lit(own, 1.0));
    }
}

fn light(
    shade: Option<Res<Shade>>,
    mut rigs: Query<(&GlobalTransform, &mut Lit, Has<Ball>), Without<crate::effects::SwingTrail>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(Shade { frame, map, world }) = shade.as_deref() else { return };
    let rain = crate::weather::now() >= 2;
    for (t, mut lit, ball) in &mut rigs {
        let p = t.translation();
        // Bevy (x, y, z) is game (x, −y, −z); the ball is shaded only on the ground, NPCs anywhere
        let s = if rain {
            1.0
        } else if ball {
            let Some(h) = shade::ball_height(&world.court, &world.models, [p.x, -p.y, -p.z, 1.0]) else { continue };
            shade::ball(map, frame, p.x, -p.z, h)
        } else {
            shade::lookup(map, frame, p.x, -p.z)
        };
        if s == lit.1 {
            continue;
        }
        lit.1 = s;
        let k = 1.0 - DIRECT * (1.0 - s);
        for (h, base) in &lit.0 {
            if let Some(mut m) = materials.get_mut(h) {
                m.base_color = Color::LinearRgba(LinearRgba { red: base.red * k, green: base.green * k, blue: base.blue * k, alpha: base.alpha });
            }
        }
    }
}
