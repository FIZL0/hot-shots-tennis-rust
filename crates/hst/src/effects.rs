//! Hit effects from `AZUMA/C_EFF/EFFCT.XB0`, played by `hst_sim::effect`: the racket impact (one model per shot
//! kind, the smash its own), started as a shot leaves the racket, at the ball, along its velocity; and the hit
//! sparks thrown off the ball with it (camera-facing quads, `*tubu00` by shot kind); and each player's swing trail
//! (a `zanzou` ribbon behind the racket).

use bevy::mesh::morph::{MeshMorphWeights, MorphWeights};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;
use hst_data::{ani, iso::Iso, mdl, mor, mtl::{self, Blend}, xb::Archive};
use hst_sim::effect::{Effect, Roll, SPARK_FADE, SPARKS, Sparks, Trail, impact_matrix, impact_scale};

const IMPACTS: [&str; 6] = ["top", "slice", "flat", "lob", "drop", "smash"];

/// One effect model on court: its player and the entities drawing it.
struct Shown {
    effect: Effect,
    root: Entity,
    joints: Vec<Entity>,
    materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Resource)]
pub struct Impacts {
    models: Vec<Shown>,
    /// The impact playing (the game has one).
    playing: Option<usize>,
}

/// A shot leaving the racket: shot kind (0 top … 4 drop; a smash's own kind), whether it is a smash (its own
/// model), the ball's position and velocity.
#[derive(Clone, Copy)]
pub struct Hit {
    pub kind: i32,
    pub smash: bool,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
}

impl Impacts {
    pub fn start(&mut self, h: Hit, transforms: &mut Query<&mut Transform>) {
        let k = if h.smash { 5 } else { h.kind.clamp(0, 4) as usize };
        self.playing = Some(k);
        let m = &mut self.models[k];
        m.effect.start();
        let world = Mat4::from_cols_array_2d(&impact_matrix([h.vel[0], h.vel[1], h.vel[2], 0.0], h.pos));
        if let Ok(mut t) = transforms.get_mut(m.root) {
            *t = Transform::from_matrix(world).with_scale(Vec3::splat(impact_scale(h.kind, h.vel)));
        }
    }
}

pub fn load(
    iso: &mut Iso,
    commands: &mut Commands,
    parent: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<Impacts, String> {
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let get = |name: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(name)).and_then(|e| arc.read(e).ok());
    let mut models = Vec::new();
    for k in IMPACTS {
        let s = format!("yumoto/impact_{k}_a");
        let file = |ext: &str| get(&format!("{s}.{ext}")).ok_or(format!("{s}.{ext} missing"));
        let model = mdl::parse(&file("mdl")?).map_err(|e| e.0)?;
        let mtl = mtl::parse(&file("mtl")?, get(&format!("{s}.mti")).as_deref()).map_err(|e| e.0)?;
        let effect = Effect::new(
            &model,
            &ani::parse(&file("ani")?).map_err(|e| e.0)?,
            &mor::parse(&file("mor")?, 1).map_err(|e| e.0)?,
            &mor::parse(&file("mta")?, 1).map_err(|e| e.0)?,
            &mtl.materials,
        );
        let tex: Vec<Handle<Image>> = mtl.textures.iter().map(|t| images.add(crate::character::texture_image(t))).collect();
        let mats: Vec<Handle<StandardMaterial>> = mtl
            .materials
            .iter()
            .map(|m| {
                let [r, g, b, a] = m.color;
                materials.add(StandardMaterial {
                    base_color: Color::linear_rgba(r, g, b, a),
                    base_color_texture: m.texture.map(|t| tex[t].clone()),
                    unlit: true,
                    double_sided: true,
                    cull_mode: None,
                    // ponytail: @sub (Cd − Cs·As) has no Bevy mode and no impact uses it; drawn as Blend
                    alpha_mode: if m.blend() == Blend::Add { AlphaMode::Add } else { AlphaMode::Blend },
                    ..default()
                })
            })
            .collect();
        let parts = crate::character::skinned_parts(&model, &mats, meshes);
        let binds = bindposes.add(SkinnedMeshInverseBindposes::from(model.node_bind.iter().map(|b| Mat4::from_cols_array_2d(b).inverse()).collect::<Vec<_>>()));

        let weights = MorphWeights::new(vec![0.0; model.morph_names.len()], None).unwrap_or_default();
        let root = commands.spawn((Transform::default(), Visibility::Hidden, weights)).id();
        commands.entity(parent).add_child(root);
        let joints: Vec<Entity> = model.node_local.iter().map(|l| commands.spawn((Transform::from_matrix(Mat4::from_cols_array_2d(l)), Visibility::default())).id()).collect();
        for (i, p) in model.node_parent.iter().enumerate() {
            commands.entity(p.map_or(root, |p| joints[p])).add_child(joints[i]);
        }
        let skin = SkinnedMesh { inverse_bindposes: binds, joints: joints.clone() };
        for (mesh, material, morphed) in parts {
            let part = commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), skin.clone(), Transform::default())).id();
            if morphed {
                commands.entity(part).insert(MeshMorphWeights::Reference(root));
            }
            commands.entity(root).add_child(part);
        }
        models.push(Shown { effect, root, joints, materials: mats });
    }
    Ok(Impacts { models, playing: None })
}

/// One game frame of the playing impact (before this frame's shots start new ones).
pub fn tick(mut fx: ResMut<Impacts>) {
    let fx = &mut *fx;
    if let Some(k) = fx.playing {
        fx.models[k].effect.tick();
        if !fx.models[k].effect.live {
            fx.playing = None;
        }
    }
}

/// Pose, morph, fade and show the playing impact; hide the others.
pub fn draw(fx: Res<Impacts>, mut q: Query<(&mut Visibility, &mut MorphWeights)>, mut joints: Query<&mut Transform>, mut materials: ResMut<Assets<StandardMaterial>>) {
    for (k, m) in fx.models.iter().enumerate() {
        let live = fx.playing == Some(k);
        if let Ok((mut v, mut w)) = q.get_mut(m.root) {
            *v = if live { Visibility::Visible } else { Visibility::Hidden };
            if live {
                w.weights_mut().copy_from_slice(&m.effect.weights);
            }
        }
        if !live {
            continue;
        }
        for (j, l) in m.joints.iter().zip(m.effect.locals()) {
            if let Ok(mut t) = joints.get_mut(*j) {
                *t = Transform::from_matrix(Mat4::from_cols_array_2d(&l));
            }
        }
        for (h, a) in m.materials.iter().zip(&m.effect.alphas) {
            if let Some(mut mat) = materials.get_mut(h) {
                mat.base_color.set_alpha(*a);
            }
        }
    }
}

#[derive(Resource)]
pub struct HitSparks {
    sparks: Sparks,
    rng: u32,
    mesh: Handle<Mesh>,
    view: Entity,
    /// `*tubu00` for top … drop, then smash.
    looks: [Handle<StandardMaterial>; 6],
}

// ponytail: the port's own generator, not the game's MT19937; the rolls' ranges are the game's
fn rand(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / (1 << 24) as f32
}

fn roll(rng: &mut u32) -> Roll {
    Roll::new(rand(rng) * std::f32::consts::PI, [rand(rng), rand(rng), rand(rng)])
}

pub fn load_sparks(iso: &mut Iso, commands: &mut Commands, parent: Entity, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, images: &mut Assets<Image>) -> Result<HitSparks, String> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let mut looks = Vec::new();
    for k in IMPACTS {
        let name = format!("yumoto/{k}tubu00.tm2");
        let e = arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&name)).ok_or(format!("{name} missing"))?;
        let pic = hst_data::tim2::decode(&arc.read(e).map_err(|e| e.0)?).map_err(|e| e.0)?.remove(0);
        let image = images.add(Image::new(Extent3d { width: pic.width, height: pic.height, depth_or_array_layers: 1 }, TextureDimension::D2, pic.rgba, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
        looks.push(materials.add(StandardMaterial { base_color_texture: Some(image), unlit: true, cull_mode: None, alpha_mode: AlphaMode::Blend, ..default() }));
    }
    let mesh = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, RenderAssetUsages::default()));
    let view = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(looks[0].clone()), Transform::default(), Visibility::Hidden, bevy::camera::visibility::NoFrustumCulling)).id();
    commands.entity(parent).add_child(view);
    let mut rng = 0x5eed_5a4c;
    let rolls = std::array::from_fn(|_| roll(&mut rng));
    Ok(HitSparks { sparks: Sparks::new(rolls), rng, mesh, view, looks: looks.try_into().unwrap() })
}

impl HitSparks {
    /// Throw a burst for this hit, then play the frame (the game moves a burst the frame it starts).
    pub fn frame(&mut self, hit: Option<Hit>, commands: &mut Commands) {
        if let Some(h) = hit {
            let look = if h.smash { 5 } else { h.kind.clamp(0, 4) as usize };
            commands.entity(self.view).insert(MeshMaterial3d(self.looks[look].clone()));
            self.sparks.start(h.kind, h.smash, [h.pos[0], h.pos[1], h.pos[2], 1.0], [h.vel[0], h.vel[1], h.vel[2], 0.0]);
        }
        let rng = &mut self.rng;
        self.sparks.tick(|rolls| {
            // every fourth roll anew, from one of the first four
            for r in rolls.iter_mut().skip((rand(rng) * 4.0) as usize).step_by(4) {
                *r = roll(rng);
            }
        });
    }
}

/// The sparks as quads facing the camera (game space: the camera's right and screen-down axes), fading over their
/// last frames.
pub fn draw_sparks(fx: Res<HitSparks>, cam: Query<&Transform, With<Camera3d>>, mut vis: Query<&mut Visibility>, mut meshes: ResMut<Assets<Mesh>>) {
    use bevy::mesh::{Indices, Mesh};
    let (Ok(cam), Ok(mut v)) = (cam.single(), vis.get_mut(fx.view)) else { return };
    *v = if fx.sparks.live { Visibility::Visible } else { Visibility::Hidden };
    let Some(mut mesh) = meshes.get_mut(&fx.mesh) else { return };
    // world → game space: (x, −y, −z)
    let game = |v: Vec3| Vec3::new(v.x, -v.y, -v.z);
    let (right, down) = (game(cam.rotation * Vec3::X), game(cam.rotation * Vec3::NEG_Y));
    let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
    for s in fx.sparks.sparks.iter().filter(|s| s.life > 0) {
        let p = Vec3::new(s.pos[0], s.pos[1], s.pos[2]);
        let (r, d) = (right * s.size, down * s.size);
        let a = s.life.min(SPARK_FADE) * 128 / SPARK_FADE;
        let n = pos.len() as u32;
        pos.extend([p - r - d, p + r - d, p - r + d, p + r + d].map(|v| v.to_array()));
        uv.extend([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0f32]]);
        colour.extend([[1.0, 1.0, 1.0, a as f32 / 128.0]; 4]);
        index.extend([n, n + 1, n + 2, n + 2, n + 1, n + 3]);
    }
    debug_assert!(pos.len() <= 4 * SPARKS);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colour);
    mesh.insert_indices(Indices::U32(index));
}

/// A player's swing trail and the motion (id, serial) last seen, to catch a swing's start.
#[derive(Component, Default)]
pub struct SwingTrail {
    trail: Trail,
    seen: (usize, u32),
}

/// The trails' one mesh (every player's ribbon).
#[derive(Resource)]
pub struct Trails {
    mesh: Handle<Mesh>,
}

pub fn load_trails(iso: &mut Iso, commands: &mut Commands, parent: Entity, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, images: &mut Assets<Image>) -> Result<Trails, String> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let e = arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with("yumoto/zanzou.tm2")).ok_or("zanzou.tm2 missing")?;
    let pic = hst_data::tim2::decode(&arc.read(e).map_err(|e| e.0)?).map_err(|e| e.0)?.remove(0);
    let image = images.add(Image::new(Extent3d { width: pic.width, height: pic.height, depth_or_array_layers: 1 }, TextureDimension::D2, pic.rgba, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
    let look = materials.add(StandardMaterial { base_color_texture: Some(image), unlit: true, cull_mode: None, alpha_mode: AlphaMode::Blend, ..default() });
    let mesh = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, RenderAssetUsages::default()));
    let view = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(look), Transform::default(), bevy::camera::visibility::NoFrustumCulling)).id();
    commands.entity(parent).add_child(view);
    Ok(Trails { mesh })
}

/// The swings the game draws a trail for: the strokes (bar the soft follow-throughs 0x1c/0x1d) and the serves.
fn swing(id: usize) -> bool {
    matches!(id, 0x10..=0x1b | 0x1e | 0x1f | 0x25 | 0x26)
}

/// One game frame of every trail: a swing starts one with the frames its motion has left, then the racket (its
/// joint in game space) is sampled.
// ponytail: the joint's transform is the last drawn pose (one frame behind the motion); a game-space racket from
// the sim's pose would be exact
pub fn tick_trails(mut q: Query<(&crate::character::Rig, &crate::character::Motion, &mut SwingTrail)>, joints: Query<&GlobalTransform>, root: Query<&GlobalTransform, With<crate::GameSpace>>) {
    let Ok(root) = root.single() else { return };
    let to_game = root.affine().inverse();
    for (rig, m, mut t) in &mut q {
        if (m.id, m.serial) != t.seen && swing(m.id) {
            let length = rig.data.motions.get(&m.id).map_or(0.0, |c| c.length);
            t.trail.start(m.id as i32, ((length - m.clock.sampled) / m.clock.speed.max(0.5)) as i32);
        }
        t.seen = (m.id, m.serial);
        let Some(j) = rig.data.joint("Racket").and_then(|j| joints.get(rig.joints[j]).ok()) else { continue };
        let w = Mat4::from(to_game * j.affine());
        t.trail.tick(m.id as i32, &[w.x_axis, w.y_axis, w.z_axis, w.w_axis].map(|c| c.to_array()));
    }
}

/// Every live trail as one ribbon mesh: inner edge u = 1, outer u = 0, v along the swing, alpha fading to the tail.
pub fn draw_trails(fx: Res<Trails>, q: Query<&SwingTrail>, mut meshes: ResMut<Assets<Mesh>>) {
    use bevy::mesh::Indices;
    let Some(mut mesh) = meshes.get_mut(&fx.mesh) else { return };
    let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
    for t in &q {
        let rows = t.trail.ribbon();
        for (i, (a, b, v, alpha)) in rows.iter().enumerate() {
            let n = pos.len() as u32;
            pos.extend([*a, *b]);
            uv.extend([[1.0, *v], [0.0, *v]]);
            colour.extend([[1.0, 1.0, 1.0, (alpha / 128.0).min(1.0)]; 2]);
            if i > 0 {
                index.extend([n - 2, n - 1, n, n, n - 1, n + 1]);
            }
        }
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colour);
    mesh.insert_indices(Indices::U32(index));
}
