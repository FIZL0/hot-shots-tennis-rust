//! The court's sun-shade map (`hst_sim::shade`) on court: built from the shadow casters at load (the game: a few
//! frames after), then each frame the ball's ([`Ball`]) and every NPC's light scale is looked up under it in clear and
//! cloudy weather (1.0 in rain and on court 7) and darkens it. Off the court the ball's height is a ray cast down at
//! the court's collision model; a miss keeps its last scale. The players are never shaded.
//!
//! The ball and characters are drawn VU1-lit ([`GsMaterial`]): ambient + the light scale × the directional light
//! (the sun's direction) + a second light (opposite the sun; the players': along the court towards their end), from
//! the match's model light (`gs::model_light`; the players' own, `gs::player_light`, with their own fog near F).
//!
//! `HST_SHADE_DUMP=<file>` writes the built map (the game's bit layout) for comparing with a capture.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use bevy::prelude::*;
use hst_sim::mesh::World;
use hst_sim::shade::{self, Frame};

use crate::character::Rig;
use crate::gs::{self, GsMaterial};

/// The GS draws of `main::models`' textured materials, by their texture: the ball's swap reads them.
/// ponytail: a global keyed by texture, since the ball's spawn only has its StandardMaterial.
pub static BALL_GS: LazyLock<Mutex<HashMap<AssetId<Image>, Vec<GsMaterial>>>> = LazyLock::new(Default::default);

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

/// A rig lit by a fixed light instead (direction, colour, ambient, second direction, second colour; `inspect`).
#[derive(Component)]
pub struct FixedLight(pub [Vec4; 5]);

/// A rig's or the ball's own GS draws, and the (light scale, weather, player on the +z half) last applied.
#[derive(Component)]
struct Lit(Vec<Handle<GsMaterial>>, Option<(f32, Option<u8>, bool)>);

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (own_materials.after(crate::noise::attach), light.after(crate::weather::apply)).chain());
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

/// Give every new rig and the ball GS draws of their own (so each can be shaded apart) in place of their
/// StandardMaterials; a second draw (TEST mode 20..29) is a sibling with the same mesh.
fn own_materials(
    mut commands: Commands,
    rigs: Query<(Entity, Option<&Rig>), Or<(Added<Rig>, Added<Ball>)>>,
    children: Query<&Children>,
    parts: Query<(&MeshMaterial3d<StandardMaterial>, &Mesh3d, Option<&bevy::mesh::skinning::SkinnedMesh>, Option<&bevy::mesh::morph::MeshMorphWeights>, &Transform, &ChildOf)>,
    materials: Res<Assets<StandardMaterial>>,
    mut gs: ResMut<Assets<GsMaterial>>,
) {
    for (root, rig) in &rigs {
        let mut own = Vec::new();
        for e in children.iter_descendants(root) {
            let Ok((m, mesh, skin, morph, t, up)) = parts.get(e) else { continue };
            let draws = match rig {
                Some(r) => r.data.gs.get(&m.0.id()).cloned(),
                None => materials.get(&m.0).and_then(|s| s.base_color_texture.as_ref()).and_then(|t| BALL_GS.lock().unwrap().get(&t.id()).cloned()),
            };
            for (k, d) in draws.into_iter().flatten().enumerate() {
                let h = gs.add(d);
                own.push(h.clone());
                if k == 0 {
                    commands.entity(e).remove::<MeshMaterial3d<StandardMaterial>>().insert(MeshMaterial3d(h));
                    continue;
                }
                let mut c = commands.spawn((mesh.clone(), *t, MeshMaterial3d(h), ChildOf(up.parent())));
                if let Some(s) = skin {
                    c.insert(s.clone());
                }
                if let Some(w) = morph {
                    c.insert(w.clone());
                }
            }
        }
        commands.entity(root).insert(Lit(own, None));
    }
}

/// Light each rig's and the ball's draws when its light scale or the weather changes: the players are never shaded.
fn light(
    shade: Option<Res<Shade>>,
    look: Option<Res<crate::weather::CourtLook>>,
    sun: Option<Res<crate::shadow::Sun>>,
    mut rigs: Query<(&GlobalTransform, &mut Lit, Has<Ball>, Has<crate::effects::SwingTrail>, Option<&FixedLight>)>,
    mut materials: ResMut<Assets<GsMaterial>>,
) {
    let rain = crate::weather::now() >= 2;
    let w = look.as_ref().and_then(|l| l.last);
    for (t, mut lit, ball, player, fixed) in &mut rigs {
        if let Some(FixedLight([dir, colour, ambient, dir2, second])) = fixed {
            if lit.1.is_none() {
                lit.1 = Some((1.0, None, false));
                for h in &lit.0 {
                    let Some(mut m) = materials.get_mut(h) else { continue };
                    let u = &mut m.uniform;
                    (u.light_dir, u.light_color, u.ambient, u.light2_dir, u.light2_color) = (*dir, *colour, *ambient, *dir2, *second);
                }
            }
            continue;
        }
        let p = t.translation();
        // Bevy (x, y, z) is game (x, −y, −z); the ball is shaded only on the ground, NPCs anywhere
        let s = match shade.as_deref() {
            Some(Shade { frame, map, world }) if !rain && !player => {
                if ball {
                    match shade::ball_height(&world.court, &world.models, [p.x, -p.y, -p.z, 1.0]) {
                        Some(h) => shade::ball(map, frame, p.x, -p.z, h),
                        None => lit.1.map_or(1.0, |l| l.0),
                    }
                } else {
                    shade::lookup(map, frame, p.x, -p.z)
                }
            }
            _ => 1.0,
        };
        let key = (s, w, player && p.z < 0.0);
        if lit.1 == Some(key) {
            continue;
        }
        lit.1 = Some(key);
        let court = look.as_deref().zip(w).map(|(look, w)| {
            let l = look.looks.get(w as usize).map_or(hst_sim::weather::Look::CLEAR, |r| hst_sim::weather::look(w, *r));
            let light = if player { gs::player_light(&look.envir, look.season, &l, look.players) } else { gs::model_light(&look.envir, look.season, &l) };
            (light, gs::court_fog(&look.envir, look.season, &l))
        });
        let (mut dir, mut colour, mut ambient) = gs::DEFAULT_LIGHT;
        let (mut dir2, mut second) = (Vec4::ZERO, Vec4::ZERO);
        if let (Some((Some((c, a, t)), _)), Some(sun)) = (court, sun.as_deref()) {
            (dir, colour, ambient) = (sun.light.extend(0.0), c, a);
            // the players' second light runs along the court towards their end (Bevy z is −game z)
            (dir2, second) = (if player { gs::player_light2(-p.z) } else { -dir }, t);
        }
        for h in &lit.0 {
            let Some(mut m) = materials.get_mut(h) else { continue };
            (m.uniform.light_dir, m.uniform.light_color, m.uniform.ambient) = (dir, (colour.truncate() * s).extend(1.0), ambient);
            (m.uniform.light2_dir, m.uniform.light2_color, m.uniform.glare) = (dir2, second, !rain as u8 as f32);
            if let Some((_, Some((mut main, _, _, fog)))) = court {
                // the players' fog is the court's with their own near F (`exe::Game::player_light`[3])
                if player {
                    main.x = look.as_deref().unwrap().players[3];
                }
                (m.uniform.fog, m.uniform.fog_color) = (main, fog);
            }
        }
    }
}
