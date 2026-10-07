//! Hit effects from `AZUMA/C_EFF/EFFCT.XB0`, played by `hst_sim::effect`: the racket impact (one model per shot
//! kind, the smash its own), started as a shot leaves the racket, at the ball, along its velocity; and the hit
//! sparks thrown off the ball with it (camera-facing quads, `*tubu00` by shot kind); and each player's swing trail
//! (a `zanzou` ribbon behind the racket); the ball's flight ribbon (`ballrolling`, coloured by shot kind) and the
//! glow spinning about it after a stroke (`impactef_*`); and the ball's bounce: a mark on the court, dust puffs in
//! the court's colours, the `ballbound` ring model (a smash landing: the court's `chakudan` crater and a dust cloud).
//! Also the landing markers from `PCDATA/PCCG0.XB`: the red one where a shot is aimed, the yellow one where a lob
//! can be smashed.

use bevy::mesh::morph::{MeshMorphWeights, MorphWeights};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;
use hst_data::{ani, iso::Iso, mdl, mor, mtl::{self, Blend}, xb::Archive};
use hst_sim::effect::{Bounce, Contact, Effect, Puff, Roll, SPARK_FADE, SPARKS, Sparks, Trail, impact_matrix, impact_scale, FLIGHT_POINTS, Flight, GLOW_FRAMES, Glow};

const IMPACTS: [&str; 6] = ["top", "slice", "flat", "lob", "drop", "smash"];

/// The entities drawing one effect model.
struct Shown {
    root: Entity,
    joints: Vec<Entity>,
    materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Resource)]
pub struct Impacts {
    models: Vec<(Effect, Shown)>,
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
        let (effect, view) = &mut self.models[k];
        effect.start();
        let world = Mat4::from_cols_array_2d(&impact_matrix([h.vel[0], h.vel[1], h.vel[2], 0.0], h.pos));
        if let Ok(mut t) = transforms.get_mut(view.root) {
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
    let mut models = Vec::new();
    for k in IMPACTS {
        models.push(model(&arc, &format!("yumoto/impact_{k}_a"), commands, parent, meshes, materials, images, bindposes)?);
    }
    Ok(Impacts { models, playing: None })
}

/// Effect model `s` (archive path without extension): its player and its hidden entities under `parent`.
#[allow(clippy::too_many_arguments)]
fn model(
    arc: &Archive,
    s: &str,
    commands: &mut Commands,
    parent: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<(Effect, Shown), String> {
    let get = |name: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(name)).and_then(|e| arc.read(e).ok());
    let file = |ext: &str| get(&format!("{s}.{ext}")).ok_or(format!("{s}.{ext} missing"));
    let model = mdl::parse(&file("mdl")?).map_err(|e| e.0)?;
    let mtl = mtl::parse(&file("mtl")?, get(&format!("{s}.mti")).as_deref()).map_err(|e| e.0)?;
    let effect = Effect::new(
        &model,
        // the landing markers have no ANI: rest pose
        &get(&format!("{s}.ani")).map_or(Ok(ani::Anim { ticks_per_frame: 1, tracks: vec![] }), |a| ani::parse(&a)).map_err(|e| e.0)?,
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
                // ponytail: @sub (Cd − Cs·As) has no Bevy mode; drawn as Blend
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
    Ok((effect, Shown { root, joints, materials: mats }))
}

/// Show `effect` on `view` this frame (posed, morphed, faded) when live, else hide it.
fn pose<F: bevy::ecs::query::QueryFilter>(effect: &Effect, view: &Shown, q: &mut Query<(&mut Visibility, &mut MorphWeights)>, joints: &mut Query<&mut Transform, F>, materials: &mut Assets<StandardMaterial>) {
    let live = effect.live;
    if let Ok((mut v, mut w)) = q.get_mut(view.root) {
        *v = if live { Visibility::Visible } else { Visibility::Hidden };
        if live {
            w.weights_mut().copy_from_slice(&effect.weights);
        }
    }
    if !live {
        return;
    }
    for (j, l) in view.joints.iter().zip(effect.locals()) {
        if let Ok(mut t) = joints.get_mut(*j) {
            *t = Transform::from_matrix(Mat4::from_cols_array_2d(&l));
        }
    }
    for (h, a) in view.materials.iter().zip(&effect.alphas) {
        if let Some(mut mat) = materials.get_mut(h) {
            mat.base_color.set_alpha(*a);
        }
    }
}

/// One game frame of the playing impact (before this frame's shots start new ones).
pub fn tick(mut fx: ResMut<Impacts>) {
    let fx = &mut *fx;
    if let Some(k) = fx.playing {
        fx.models[k].0.tick();
        if !fx.models[k].0.live {
            fx.playing = None;
        }
    }
}

/// Pose, morph, fade and show the playing impact; hide the others.
pub fn draw(fx: Res<Impacts>, mut q: Query<(&mut Visibility, &mut MorphWeights)>, mut joints: Query<&mut Transform>, mut materials: ResMut<Assets<StandardMaterial>>) {
    for (k, (effect, view)) in fx.models.iter().enumerate() {
        if fx.playing == Some(k) {
            pose(effect, view, &mut q, &mut joints, &mut materials);
        } else if let Ok((mut v, _)) = q.get_mut(view.root) {
            *v = Visibility::Hidden;
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

/// `yumoto/<name>.tm2` (or `<dir>/<name>.tm2`) from the effect archive as an unlit, alpha-blended, two-sided material.
fn look(iso: &mut Iso, name: &str, materials: &mut Assets<StandardMaterial>, images: &mut Assets<Image>) -> Result<Handle<StandardMaterial>, String> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    // ponytail: re-reads the archive per texture; a handful at load
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let path = if name.contains('/') { format!("{name}.tm2") } else { format!("yumoto/{name}.tm2") };
    let e = arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&path)).ok_or(format!("{path} missing"))?;
    let pic = hst_data::tim2::decode(&arc.read(e).map_err(|e| e.0)?).map_err(|e| e.0)?.remove(0);
    let image = images.add(Image::new(Extent3d { width: pic.width, height: pic.height, depth_or_array_layers: 1 }, TextureDimension::D2, pic.rgba, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
    Ok(materials.add(StandardMaterial { base_color_texture: Some(image), unlit: true, cull_mode: None, alpha_mode: AlphaMode::Blend, ..default() }))
}

/// Write a frame's quads into a dynamic mesh. Nothing to draw still writes one invisible degenerate triangle: Bevy's
/// mesh allocator skips a mesh with no vertices but then copies it anyway, logging a use-after-free every frame.
fn fill(mesh: &mut Mesh, mut pos: Vec<[f32; 3]>, mut uv: Vec<[f32; 2]>, mut colour: Vec<[f32; 4]>, mut index: Vec<u32>) {
    if pos.is_empty() {
        (pos, uv, colour, index) = (vec![[0.0; 3]; 3], vec![[0.0; 2]; 3], vec![[0.0; 4]; 3], vec![0, 0, 0]);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colour);
    mesh.insert_indices(bevy::mesh::Indices::U32(index));
}

pub fn load_sparks(iso: &mut Iso, commands: &mut Commands, parent: Entity, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, images: &mut Assets<Image>) -> Result<HitSparks, String> {
    let mut looks = Vec::new();
    for k in IMPACTS {
        looks.push(look(iso, &format!("{k}tubu00"), materials, images)?);
    }
    let mesh = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
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
    fill(&mut mesh, pos, uv, colour, index);
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
    let mesh = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
    let view = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(look(iso, "zanzou", materials, images)?), Transform::default(), bevy::camera::visibility::NoFrustumCulling)).id();
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
    fill(&mut mesh, pos, uv, colour, index);
}

/// The ball's flight ribbon and glow, each one mesh.
#[derive(Resource)]
pub struct BallFlight {
    flight: Flight,
    glow: Glow,
    ribbon: Handle<Mesh>,
    disc: Entity,
    /// The ball at the last frame.
    pos: Vec3,
    vel: Vec3,
    /// `impactef_*` for top … drop.
    looks: [Handle<StandardMaterial>; 5],
}

pub fn load_flight(iso: &mut Iso, commands: &mut Commands, parent: Entity, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, images: &mut Assets<Image>) -> Result<BallFlight, String> {
    let ribbon = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
    let view = commands.spawn((Mesh3d(ribbon.clone()), MeshMaterial3d(look(iso, "ballrolling", materials, images)?), Transform::default(), bevy::camera::visibility::NoFrustumCulling)).id();
    let mut looks = Vec::new();
    for k in &IMPACTS[..5] {
        looks.push(look(iso, &format!("impactef_{k}"), materials, images)?);
    }
    let disc = commands.spawn((Mesh3d(meshes.add(Rectangle::new(2.0 * GLOW_SIZE, 2.0 * GLOW_SIZE))), MeshMaterial3d(looks[0].clone()), Transform::default(), Visibility::Hidden)).id();
    commands.entity(parent).add_children(&[view, disc]);
    Ok(BallFlight { flight: Flight::default(), glow: Glow::default(), ribbon, disc, pos: Vec3::ZERO, vel: Vec3::ZERO, looks: looks.try_into().unwrap() })
}

/// The glow's half-size and turn over its life.
const GLOW_SIZE: f32 = 0.4;
const GLOW_TURN: f32 = std::f32::consts::FRAC_PI_2;

impl BallFlight {
    /// One game frame: a hit restarts the ribbon (and, bar a smash, the glow); a dead ball ends both.
    pub fn frame(&mut self, hit: Option<Hit>, dead: bool, pos: [f32; 3], vel: [f32; 3], commands: &mut Commands) {
        if let Some(h) = hit {
            self.flight.start(h.kind, h.smash);
            if !h.smash {
                self.glow.start(h.kind);
                commands.entity(self.disc).insert(MeshMaterial3d(self.looks[self.glow.texture].clone()));
            }
        }
        if dead {
            self.flight.live = false;
            self.glow.life = 0;
        }
        self.flight.tick([pos[0], pos[1], pos[2], 1.0], [vel[0], vel[1], vel[2], 0.0]);
        (self.pos, self.vel) = (Vec3::from(pos), Vec3::from(vel));
        self.glow.tick();
    }
}

/// The ribbon from the ball back along its samples, facing the camera, `speed`·20 long, fading to its end; the glow
/// a disc across the flight at the ball, spinning a quarter turn and fading over its last 15 frames.
// ponytail: the game's tilt of a disc flying along the court, its texture flip by facing and its halved ribbon
// length under one of its mode counts are left out
pub fn draw_flight(fx: Res<BallFlight>, cam: Query<(&Transform, &Projection), With<Camera3d>>, mut disc: Query<(&mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>), Without<Camera3d>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let Ok((cam, proj)) = cam.single() else { return };
    let fov = if let Projection::Perspective(p) = proj { p.fov } else { 0.8 };
    // world → game space: (x, −y, −z)
    let eye = Vec3::new(cam.translation.x, -cam.translation.y, -cam.translation.z);
    if let Ok((mut t, mut v, look)) = disc.get_mut(fx.disc) {
        *v = if fx.glow.life > 0 { Visibility::Visible } else { Visibility::Hidden };
        let turn = GLOW_TURN * fx.glow.life as f32 / GLOW_FRAMES as f32;
        *t = Transform::from_translation(fx.pos).looking_to(fx.vel.try_normalize().unwrap_or(Vec3::Z), Vec3::Y) * Transform::from_rotation(Quat::from_rotation_z(turn));
        if let Some(mut m) = materials.get_mut(&look.0) {
            m.base_color = Color::srgba(1.0, 1.0, 1.0, (fx.glow.life as f32 / 15.0).min(1.0));
        }
    }
    let Some(mut mesh) = meshes.get_mut(&fx.ribbon) else { return };
    let f = &fx.flight;
    let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
    if f.live && f.count >= 2 {
        let p: Vec<Vec3> = (0..f.count).map(|k| Vec3::from_slice(&f.points[(f.head + 2 * FLIGHT_POINTS - 1 - k) % FLIGHT_POINTS][..3])).collect();
        let c = f.colour.to_be_bytes().map(|b| b as f32 / 128.0);
        let (len, tan) = (f.speed * 20.0, (fov / 2.0).tan());
        let mut run = 0.0;
        for i in 0..p.len() {
            let mut at = p[i];
            let mut last = i == p.len() - 1;
            if i > 0 {
                let d = at.distance(p[i - 1]);
                if run + d >= len {
                    at = p[i - 1].lerp(at, (len - run) / d.max(1e-6));
                    (run, last) = (len, true);
                } else {
                    run += d;
                }
            }
            let along = p[(i + 1).min(p.len() - 1)] - p[i.saturating_sub(1)];
            let side = along.cross(eye - at).normalize_or_zero() * (tan * at.distance(eye) * 0.005).max(0.08);
            let n = pos.len() as u32;
            let a = if last { 0.0 } else { c[3] * (1.0 - run / len.max(1e-6)).max(0.0) };
            pos.extend([(at - side).to_array(), (at + side).to_array()]);
            uv.extend([[0.0, 1.0 - run / len.max(1e-6)], [1.0, 1.0 - run / len.max(1e-6)]]);
            colour.extend([[c[0], c[1], c[2], a]; 2]);
            if i > 0 {
                index.extend([n - 2, n - 1, n, n, n - 1, n + 1]);
            }
            if last {
                break;
            }
        }
    }
    fill(&mut mesh, pos, uv, colour, index);
}

/// The ball's bounce effects: the sim, the ring and crater models, and one mesh each for marks and dust.
#[derive(Resource)]
pub struct BallBounce {
    bounce: Bounce,
    ring: Shown,
    crater: Shown,
    /// This flight's contacts so far (the first two bounces).
    contacts: Vec<Contact>,
    marks: Handle<Mesh>,
    puffs: Handle<Mesh>,
    dust: Handle<Mesh>,
    /// The court's dust and mark colours (1 = the game's 128).
    look: ([f32; 3], [f32; 3]),
}

/// Bounce effects for disc court `court` (1..11; others borrow court 10's crater).
#[allow(clippy::too_many_arguments)]
pub fn load_bounce(
    iso: &mut Iso,
    court: usize,
    commands: &mut Commands,
    parent: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<BallBounce, String> {
    let (cnf, bin) = (iso.read("SYSTEM.CNF").map_err(|e| e.to_string())?, iso.read("ZZBIN/GAME.BIN").map_err(|e| e.to_string())?);
    let looks = hst_data::exe::Game::new(&cnf, &bin).map_err(|e| e.0)?.bounce_looks();
    let colours = looks.get(court).copied().unwrap_or(looks[10]);
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let (ring, ring_view) = model(&arc, "bnd/ballbound_00", commands, parent, meshes, materials, images, bindposes)?;
    let c = if (1..=11).contains(&court) { court } else { 10 };
    let (crater, crater_view) = model(&arc, &format!("smash_bnd/c{c:02}/c{c:02}_chakudan"), commands, parent, meshes, materials, images, bindposes)?;
    let mut mesh = |tex: &str, commands: &mut Commands| -> Result<Handle<Mesh>, String> {
        let m = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
        let view = commands.spawn((Mesh3d(m.clone()), MeshMaterial3d(look(iso, tex, materials, images)?), Transform::default(), bevy::camera::visibility::NoFrustumCulling)).id();
        commands.entity(parent).add_child(view);
        Ok(m)
    };
    let marks = mesh("bnd/ballkon_00", commands)?;
    let puffs = mesh("run/kemuri_01", commands)?;
    let dust = mesh("run/kemuri_00", commands)?;
    Ok(BallBounce {
        bounce: Bounce::new(ring, crater, colours.puffs),
        ring: ring_view,
        crater: crater_view,
        contacts: vec![],
        marks,
        puffs,
        dust,
        look: (colours.dust.map(|v| v / 128.0), colours.mark.map(|v| v / 128.0)),
    })
}

impl BallBounce {
    /// One game frame of the ball (`bounces` so far, the last landing `at` on `court` ground moving at `vel`);
    /// `smash` a smash landing at 85 km/h or more; `fading` the marks age (a point is on).
    // ponytail: every contact's surface is the flat court (normal up); the stage mesh's own normal would tilt the ring
    pub fn frame(&mut self, bounces: i32, at: [f32; 3], vel: [f32; 3], court: bool, smash: bool, fading: bool, transforms: &mut Query<&mut Transform>) {
        if bounces == 0 {
            self.contacts.clear();
        } else if bounces as usize > self.contacts.len() && self.contacts.len() < 3 {
            self.contacts.push(Contact { point: [at[0], at[1], at[2], 1.0], vel: [vel[0], vel[1], vel[2], 0.0], normal: [0.0, -1.0, 0.0, 0.0], court });
        }
        self.bounce.fading = fading;
        self.bounce.tick(bounces, &self.contacts, smash);
        for (view, at) in [(&self.ring, self.bounce.ring_at), (&self.crater, self.bounce.crater_at)] {
            if let Ok(mut t) = transforms.get_mut(view.root) {
                *t = Transform::from_matrix(Mat4::from_cols_array_2d(&at));
            }
        }
    }
}

/// The ring and crater models; the marks as ground quads (a spot stretched along the bounce by the ball's speed),
/// the puffs and dust cloud as camera-facing quads rising from their foot, in the court's colours.
#[allow(clippy::type_complexity)]
pub fn draw_bounce(
    fx: Res<BallBounce>,
    cam: Query<&Transform, With<Camera3d>>,
    mut q: Query<(&mut Visibility, &mut MorphWeights)>,
    mut joints: Query<&mut Transform, Without<Camera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let b = &fx.bounce;
    pose(&b.ring, &fx.ring, &mut q, &mut joints, &mut materials);
    pose(&b.crater, &fx.crater, &mut q, &mut joints, &mut materials);
    let Ok(cam) = cam.single() else { return };
    let game = |v: Vec3| Vec3::new(v.x, -v.y, -v.z);
    let (right, up) = (game(cam.rotation * Vec3::X), game(cam.rotation * Vec3::Y));
    let put = |mesh: &mut Mesh, quads: Vec<([Vec3; 4], [f32; 2], [f32; 4])>| {
        let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
        for (corners, [v0, v1], c) in quads {
            let n = pos.len() as u32;
            pos.extend(corners.map(|v| v.to_array()));
            uv.extend([[0.0, v0], [1.0, v0], [0.0, v1], [1.0, v1]]);
            colour.extend([c; 4]);
            index.extend([n, n + 1, n + 2, n + 2, n + 1, n + 3]);
        }
        fill(mesh, pos, uv, colour, index);
    };
    let v3 = |r: [f32; 4]| Vec3::new(r[0], r[1], r[2]);
    let mut quads = vec![];
    let [r, g, bl] = fx.look.1;
    for m in &b.marks {
        let (c, s, f) = (v3(m.m[3]), v3(m.m[0]) * 0.05, v3(m.m[2]));
        let colour = [r, g, bl, m.alpha / 128.0];
        // ponytail: the game's spot halves; a cap, the stretch, a cap
        let rows = [c - f * 0.05, c, c + f * m.len, c + f * (m.len + 0.05)];
        for (k, v) in [(0, [0.0, 0.5]), (1, [0.5, 0.5]), (2, [0.5, 1.0])] {
            quads.push(([rows[k] - s, rows[k] + s, rows[k + 1] - s, rows[k + 1] + s], v, colour));
        }
    }
    if let Some(mut mesh) = meshes.get_mut(&fx.marks) {
        put(&mut mesh, quads);
    }
    let [r, g, bl] = fx.look.0;
    let billboard = |p: &Puff| {
        let (foot, w, h) = (v3(p.pos), right * p.size, up * 2.0 * p.size);
        ([foot - w + h, foot + w + h, foot - w, foot + w], [0.0, 1.0], [r, g, bl, p.alpha / 128.0])
    };
    if let Some(mut mesh) = meshes.get_mut(&fx.puffs) {
        put(&mut mesh, b.puffs.iter().filter(|p| p.life > 0).map(billboard).collect());
    }
    if let Some(mut mesh) = meshes.get_mut(&fx.dust) {
        put(&mut mesh, b.dust.iter().map(|(p, _, _)| billboard(p)).collect());
    }
}

/// The landing markers: `chakudan_p` (red, a looping pulse) at the shot's aim and `smash_p` (yellow, one 120-frame
/// play) at the smash point; each placed on the court at (x, z) or hidden.
#[derive(Resource)]
pub struct LandingMarks {
    red: (Effect, Shown),
    smash: (Effect, Shown),
    pub red_at: Option<[f32; 2]>,
    pub smash_at: Option<[f32; 2]>,
}

#[allow(clippy::too_many_arguments)]
pub fn load_marks(
    iso: &mut Iso,
    commands: &mut Commands,
    parent: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<LandingMarks, String> {
    let data = iso.read("PCDATA/PCCG0.XB").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let mut red = model(&arc, "taguchi/other/chakudan_p", commands, parent, meshes, materials, images, bindposes)?;
    // never restarted: the pulse runs on from wherever it was, ticked only while shown
    red.0.looping = true;
    red.0.start();
    let smash = model(&arc, "taguchi/other/smash_p", commands, parent, meshes, materials, images, bindposes)?;
    Ok(LandingMarks { red, smash, red_at: None, smash_at: None })
}

impl LandingMarks {
    /// One game frame: `start_smash` (the first smash point was just found) restarts the yellow marker's play.
    pub fn tick(&mut self, start_smash: bool) {
        if self.red_at.is_some() {
            self.red.0.tick();
        }
        if start_smash {
            self.smash.0.start();
        } else {
            self.smash.0.tick();
        }
    }
}

pub fn draw_marks(fx: Res<LandingMarks>, mut q: Query<(&mut Visibility, &mut MorphWeights)>, mut joints: Query<&mut Transform>, mut materials: ResMut<Assets<StandardMaterial>>) {
    for ((effect, view), at) in [(&fx.red, fx.red_at), (&fx.smash, fx.smash_at.filter(|_| fx.smash.0.live))] {
        match at {
            Some([x, z]) => {
                pose(effect, view, &mut q, &mut joints, &mut materials);
                if let Ok(mut t) = joints.get_mut(view.root) {
                    *t = Transform::from_xyz(x, 0.0, z);
                }
            }
            None => {
                if let Ok((mut v, _)) = q.get_mut(view.root) {
                    *v = Visibility::Hidden;
                }
            }
        }
    }
}
