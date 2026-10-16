//! Hit effects from `AZUMA/C_EFF/EFFCT.XB0`, played by `hst_sim::effect`: the racket impact (one model per shot
//! kind, the smash its own), started as a shot leaves the racket, at the ball, along its velocity; and the hit
//! sparks thrown off the ball with it (camera-facing quads, `*tubu00` by shot kind); and each player's swing trail
//! (a `zanzou` ribbon behind the racket); the ball's flight ribbon (`ballrolling`, coloured by shot kind) and the
//! glow spinning about it after a stroke (`impactef_*`); and the ball's bounce: a mark on the court, dust puffs in
//! the court's colours, the `ballbound` ring model (a smash landing: the court's `chakudan` crater and a dust cloud).
//! Also the landing markers from `PCDATA/PCCG0.XB`: the red one where a shot is aimed, the yellow one where a lob
//! can be smashed.

use bevy::mesh::{VertexAttributeValues, morph::MorphWeights};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;
use hst_data::{ani, iso::Iso, mdl, mor, mtl::{self, Blend}, xb::Archive};
use hst_sim::effect::{Bounce, Contact, Effect, Puff, Roll, SPARK_FADE, SPARKS, Sparks, Trail, impact_matrix, impact_scale, FLIGHT_POINTS, Flight, GLOW_FRAMES, Glow};
use hst_sim::rng::Mt;

const IMPACTS: [&str; 6] = ["top", "slice", "flat", "lob", "drop", "smash"];

/// The entities drawing one effect model.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Shown<M: Asset = StandardMaterial> {
    pub root: Entity,
    joints: Vec<Entity>,
    #[serde(skip)]
    pub materials: Vec<Handle<M>>,
}

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct Impacts {
    models: Vec<(Effect, Shown<Fx>)>,
    /// The impact playing (the game has one).
    playing: Option<usize>,
}

/// A shot leaving the racket: shot kind (0 top … 4 drop; a smash's own kind), whether it is a smash (its own
/// model), the ball's position and velocity.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Hit {
    pub kind: i32,
    pub smash: bool,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
}

impl Impacts {
    /// An impact is playing (tests).
    pub fn playing(&self) -> bool {
        self.playing.is_some()
    }

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
    materials: &mut Assets<Fx>,
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
pub fn model<M: EffectMaterial>(
    arc: &Archive,
    s: &str,
    commands: &mut Commands,
    parent: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<M>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<(Effect, Shown<M>), String> {
    let get = |name: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(name)).and_then(|e| arc.read(e).ok());
    let file = |ext: &str| get(&format!("{s}.{ext}")).ok_or(format!("{s}.{ext} missing"));
    let model = mdl::parse(&file("mdl")?).map_err(|e| e.0)?;
    let mtl = mtl::parse(&file("mtl")?, get(&format!("{s}.mti")).as_deref()).map_err(|e| e.0)?;
    let _pre = crate::textures::prefetch(&mtl);
    let effect = Effect::new(
        &model,
        // the landing markers have no ANI: rest pose
        &get(&format!("{s}.ani")).map_or(Ok(ani::Anim { ticks_per_frame: 1, tracks: vec![] }), |a| ani::parse(&a)).map_err(|e| e.0)?,
        // some of the umpire's call models have no MOR: no morphs
        &get(&format!("{s}.mor")).map_or(Ok(mor::Tracks { ticks_per_frame: 1, tracks: vec![] }), |m| mor::parse(&m, 1)).map_err(|e| e.0)?,
        // the ball's tornado has no MTA: alpha as the MTL
        &get(&format!("{s}.mta")).map_or(Ok(mor::Tracks { ticks_per_frame: 1, tracks: vec![] }), |m| mor::parse(&m, 1)).map_err(|e| e.0)?,
        &mtl.materials,
    );
    let tex: Vec<Handle<Image>> = mtl.textures.iter().map(|t| crate::textures::add_mtl(images, crate::character::texture_image(t), t)).collect();
    let mats: Vec<Handle<M>> = mtl
        .materials
        .iter()
        .map(|m| {
            let [r, g, b, a] = m.color;
            materials.add(M::new(StandardMaterial {
                base_color: Color::linear_rgba(r, g, b, a),
                base_color_texture: m.texture.map(|t| tex[t].clone()),
                unlit: true,
                double_sided: true,
                cull_mode: None,
                // ponytail: @sub (Cd − Cs·As) has no Bevy mode; drawn as Blend
                alpha_mode: if m.blend() == Blend::Add { AlphaMode::Add } else { AlphaMode::Blend },
                depth_bias: AFTER_SCENE,
                ..default()
            }))
        })
        .collect();
    let parts = crate::character::skinned_parts_whole(&model, &mats, meshes);
    let binds = bindposes.add(SkinnedMeshInverseBindposes::from(model.node_bind.iter().map(|b| Mat4::from_cols_array_2d(b).inverse()).collect::<Vec<_>>()));

    let weights = MorphWeights::new(vec![0.0; model.morph_names.len()], None).unwrap_or_default();
    let root = commands.spawn((Transform::default(), Visibility::Hidden, weights)).id();
    commands.entity(parent).add_child(root);
    let holder = commands.spawn((Transform::default(), Visibility::default(), Collapse(root))).id();
    commands.entity(root).add_child(holder);
    // the parts hang off the holder, so the joints carry no visibility (P41: the per-view checks don't walk them)
    let joints: Vec<Entity> = model.node_local.iter().map(|l| commands.spawn(Transform::from_matrix(Mat4::from_cols_array_2d(l))).id()).collect();
    for (i, p) in model.node_parent.iter().enumerate() {
        commands.entity(p.map_or(holder, |p| joints[p])).add_child(joints[i]);
    }
    let skin = SkinnedMesh { inverse_bindposes: binds, joints: joints.clone() };
    for (mesh, material, morphed) in parts {
        // culled by bounds that follow its joints (B345, `cull.rs`)
        let part = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material), skin.clone(), Transform::default(), crate::cull::Cullable)).id();
        if morphed {
            // always drawn (when in view), collapsed while hidden (`Collapse`)
            commands.entity(part).insert((cpu_morph(&mesh, root, meshes), Visibility::Visible));
        }
        commands.entity(holder).add_child(part);
    }
    Ok((effect, Shown { root, joints, materials: mats }))
}

/// A morphed effect part, morphed here rather than by Bevy: WebGPU in Safari on the iPhone drew the bounce ring's
/// morphed sprites as big flat panels (B137a; WebGL2 and desktop fine), and Bevy morphs through storage buffers only on
/// WebGPU. The effects' meshes are a few dozen vertices, so the positions are rewritten whenever the weights change.
#[derive(Component)]
pub struct CpuMorph {
    /// The entity with the `MorphWeights`.
    root: Entity,
    base: Vec<Vec3>,
    /// Per target, every vertex's offset.
    targets: Vec<Vec<Vec3>>,
    last: Vec<f32>,
}

/// Take `mesh`'s morph targets off it (rebuilt without them, kept in the main world) into a `CpuMorph`.
fn cpu_morph(mesh: &Handle<Mesh>, root: Entity, meshes: &mut Assets<Mesh>) -> CpuMorph {
    let old = meshes.get(mesh).expect("just added");
    let mut new = Mesh::new(old.primitive_topology(), bevy::asset::RenderAssetUsages::default());
    for (attr, values) in old.attributes() {
        new.insert_attribute(*attr, values.clone());
    }
    if let Some(i) = old.indices() {
        new.insert_indices(i.clone());
    }
    // its joints' boxes, widened by the targets' reach (`cull::bound`)
    new.set_skinned_mesh_bounds(old.skinned_mesh_bounds().cloned());
    let base: Vec<Vec3> = old.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|v| v.as_float3()).unwrap_or_default().iter().map(|p| Vec3::from(*p)).collect();
    let attrs = old.morph_targets().cloned().unwrap_or_default();
    let n = base.len().max(1);
    let targets = attrs.chunks(n).map(|t| t.iter().map(|a| a.position).collect()).collect();
    let _ = meshes.insert(mesh, new);
    CpuMorph { root, base, targets, last: vec![] }
}

/// Write each `CpuMorph` part's positions (base + Σ weight · target) when its root's weights changed.
pub fn morph_on_cpu(mut parts: Query<(&mut CpuMorph, &Mesh3d)>, weights: Query<&MorphWeights>, mut meshes: ResMut<Assets<Mesh>>) {
    for (mut m, mesh) in &mut parts {
        let Ok(w) = weights.get(m.root) else { continue };
        if m.last == w.weights() {
            continue;
        }
        if m.last.len() == w.weights().len() {
            m.last.copy_from_slice(w.weights());
        } else {
            m.last = w.weights().to_vec();
        }
        let Some(mut mesh) = meshes.get_mut(&mesh.0) else { continue };
        // Written in place (no buffers per frame): per vertex base, then each nonzero target's offset in target order.
        let Ok(Some(VertexAttributeValues::Float32x3(pos))) = mesh.try_attribute_mut_option(Mesh::ATTRIBUTE_POSITION) else { continue };
        pos.iter_mut().zip(&m.base).for_each(|(p, b)| *p = b.to_array());
        for (t, &k) in m.targets.iter().zip(w.weights()) {
            if k != 0.0 {
                pos.iter_mut().zip(t).for_each(|(p, d)| *p = (Vec3::from_array(*p) + *d * k).to_array());
            }
        }
    }
}

/// Holds an effect model's joints and parts under its root (the entity here), scaled to nothing while the root is
/// hidden. Bevy (0.19) draws a morphed mesh unmorphed on the first frame it is drawn again after being hidden (its morph
/// descriptor is built before the mesh instance it needs), so an impact's disc flashed at full size before shrinking
/// to its morphed size (B253): morphed parts stay drawn, collapsed to a point while their effect is hidden.
#[derive(Component)]
pub struct Collapse(Entity);

/// Scale each `Collapse` holder to nothing while its root is hidden (by itself or an ancestor), else back to 1.
pub fn collapse_hidden(mut holders: Query<(&Collapse, &mut Transform)>, vis: Query<(&Visibility, Option<&ChildOf>)>) {
    for (c, mut t) in &mut holders {
        let mut at = Some(c.0);
        let mut shown = true;
        while let Some(e) = at {
            match vis.get(e) {
                Ok((Visibility::Hidden, _)) => shown = false,
                Ok((Visibility::Inherited, p)) => {
                    at = p.map(ChildOf::parent);
                    continue;
                }
                _ => {}
            }
            break;
        }
        let s = if shown { 1.0 } else { 0.0 };
        if t.scale.x != s {
            t.scale = Vec3::splat(s);
        }
    }
}

/// Show `effect` on `view` this frame (posed, morphed, faded) when live, else hide it.
pub fn pose<F: bevy::ecs::query::QueryFilter, M: EffectMaterial>(effect: &Effect, view: &Shown<M>, q: &mut Query<(&mut Visibility, &mut MorphWeights)>, joints: &mut Query<&mut Transform, F>, materials: &mut Assets<M>) {
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
        if materials.get(h).is_some_and(|mat| mat.std_ref().base_color.alpha() != *a)
            && let Some(mut mat) = materials.get_mut(h)
        {
            mat.std().base_color.set_alpha(*a);
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
pub fn draw(fx: Res<Impacts>, mut q: Query<(&mut Visibility, &mut MorphWeights)>, mut joints: Query<&mut Transform>, mut materials: ResMut<Assets<Fx>>) {
    for (k, (effect, view)) in fx.models.iter().enumerate() {
        if fx.playing == Some(k) {
            pose(effect, view, &mut q, &mut joints, &mut materials);
        } else if let Ok((mut v, _)) = q.get_mut(view.root) {
            *v = Visibility::Hidden;
        }
    }
}

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct HitSparks {
    sparks: Sparks,
    #[serde(skip)]
    mesh: Handle<Mesh>,
    view: Entity,
    /// `*tubu00` for top … drop, then smash.
    #[serde(skip)]
    looks: [Handle<Fx>; 6],
}

/// `yumoto/<name>.tm2` (or `<dir>/<name>.tm2`) from the effect archive as an unlit, alpha-blended, two-sided material.
pub fn look<M: EffectMaterial>(iso: &mut Iso, name: &str, materials: &mut Assets<M>, images: &mut Assets<Image>) -> Result<Handle<M>, String> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    // ponytail: re-reads the archive per texture; a handful at load
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let path = if name.contains('/') { format!("{name}.tm2") } else { format!("yumoto/{name}.tm2") };
    let e = arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&path)).ok_or(format!("{path} missing"))?;
    let pic = hst_data::tim2::decode(&arc.read(e).map_err(|e| e.0)?).map_err(|e| e.0)?.remove(0);
    let img = Image::new(Extent3d { width: pic.width, height: pic.height, depth_or_array_layers: 1 }, TextureDimension::D2, pic.rgba.clone(), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    let image = crate::textures::add_tim2(images, img, &pic);
    Ok(materials.add(M::new(StandardMaterial { base_color_texture: Some(image), unlit: true, cull_mode: None, alpha_mode: AlphaMode::Blend, depth_bias: AFTER_SCENE, ..default() })))
}

/// The game draws its effects after the court's and the ball tornado's blended draws (GS dump: the tornado, then the
/// ball's ribbon, the hit sparks, the swing trail, a bounce's puff and mark): blended draws sort by their mesh's
/// centre, and these meshes sit at the court's origin or the ball, so this puts them after any of those.
/// The landing markers go after these still (`1e3 +`).
pub const AFTER_SCENE: f32 = 500.0;

/// The in-match effects' material: a `StandardMaterial` whose `depth_bias` only orders the blended draws. Bevy also
/// writes it into the pipeline as a hardware depth bias in units the GPU picks: nothing at court depths on desktop
/// Vulkan/D3D (float depth: 2^(exponent − 23)), but a fixed step on some phone GPUs (Metal, GLES drivers), where
/// 500–1e3 units pull the bounce marks, puffs and landing markers metres toward the camera, through the players and
/// the net (B137).
pub type Fx = bevy::pbr::ExtendedMaterial<StandardMaterial, SortOnly>;

/// Drops the hardware depth bias, keeps the sort offset (`Fx`).
#[derive(Asset, bevy::render::render_resource::AsBindGroup, Reflect, Clone, Default)]
pub struct SortOnly {}

impl bevy::pbr::MaterialExtension for SortOnly {
    fn specialize(
        _: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _: &bevy::mesh::MeshVertexBufferLayoutRef,
        _: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.bias.constant = 0;
        }
        Ok(())
    }
}

/// The umpire's call models' material (P33): the match camera draws them over the scene as the original's overlay
/// does, so no depth test, no depth write and no hardware depth bias; `depth_bias` only sorts them after the scene's
/// blended draws.
pub type OnTop = bevy::pbr::ExtendedMaterial<StandardMaterial, NoDepth>;

/// Draws whatever the depth buffer holds (`OnTop`).
#[derive(Asset, bevy::render::render_resource::AsBindGroup, Reflect, Clone, Default)]
pub struct NoDepth {}

impl bevy::pbr::MaterialExtension for NoDepth {
    fn specialize(
        _: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _: &bevy::mesh::MeshVertexBufferLayoutRef,
        _: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.bias.constant = 0;
            depth.depth_compare = Some(bevy::render::render_resource::CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

/// A material `model`/`look` can build: the menus' and NPCs' plain `StandardMaterial` or the match's `Fx`.
pub trait EffectMaterial: Material {
    fn new(m: StandardMaterial) -> Self;
    fn std(&mut self) -> &mut StandardMaterial;
    fn std_ref(&self) -> &StandardMaterial;
}

impl EffectMaterial for StandardMaterial {
    fn new(m: StandardMaterial) -> Self {
        m
    }
    fn std(&mut self) -> &mut StandardMaterial {
        self
    }
    fn std_ref(&self) -> &StandardMaterial {
        self
    }
}

impl EffectMaterial for Fx {
    fn new(base: StandardMaterial) -> Self {
        Fx { base, extension: SortOnly {} }
    }
    fn std(&mut self) -> &mut StandardMaterial {
        &mut self.base
    }
    fn std_ref(&self) -> &StandardMaterial {
        &self.base
    }
}

impl EffectMaterial for OnTop {
    fn new(base: StandardMaterial) -> Self {
        OnTop { base, extension: NoDepth {} }
    }
    fn std(&mut self) -> &mut StandardMaterial {
        &mut self.base
    }
    fn std_ref(&self) -> &StandardMaterial {
        &self.base
    }
}

/// Effect meshes [`fill`] has emptied (true) or filled again since [`hide_empty`] last ran.
static EMPTIED: std::sync::Mutex<Vec<(AssetId<Mesh>, bool)>> = std::sync::Mutex::new(Vec::new());

/// Shown on no camera while its mesh is [`fill`]'s empty placeholder.
#[derive(Component)]
pub struct Emptied;

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, hide_empty.before(bevy::camera::visibility::VisibilitySystems::CheckVisibility));
}

/// B328: an effect with nothing to draw (most of them, most of the time) still cost a draw of its degenerate
/// triangle; on no render layer it's skipped by every view, shadows included. Its `Visibility` stays the effect's own.
fn hide_empty(mut commands: Commands, q: Query<(Entity, &Mesh3d, Has<Emptied>, Has<bevy::camera::visibility::RenderLayers>)>) {
    let changes = std::mem::take(&mut *EMPTIED.lock().unwrap());
    if changes.is_empty() {
        return;
    }
    let empty: std::collections::HashMap<_, _> = changes.into_iter().collect();
    for (e, m, ours, layers) in &q {
        match empty.get(&m.id()) {
            Some(true) if !layers => {
                commands.entity(e).insert((Emptied, bevy::camera::visibility::RenderLayers::none()));
            }
            Some(false) if ours => {
                commands.entity(e).remove::<(Emptied, bevy::camera::visibility::RenderLayers)>();
            }
            _ => {}
        }
    }
}

/// Write a frame's quads into a dynamic mesh. Nothing to draw still writes one invisible degenerate triangle: Bevy's
/// mesh allocator skips a mesh with no vertices but then copies it anyway, logging a use-after-free every frame.
/// Sets an effect mesh's quads; one that's unchanged (mostly empty → empty) isn't touched, since a `get_mut` makes
/// Bevy repack and re-upload the whole mesh (B328).
pub fn fill(meshes: &mut Assets<Mesh>, h: &Handle<Mesh>, mut pos: Vec<[f32; 3]>, mut uv: Vec<[f32; 2]>, mut colour: Vec<[f32; 4]>, mut index: Vec<u32>) {
    use bevy::mesh::{Indices, VertexAttributeValues as V};
    if pos.is_empty() {
        (pos, uv, colour, index) = (vec![[0.0; 3]; 3], vec![[0.0; 2]; 3], vec![[0.0; 4]; 3], vec![0, 0, 0]);
    }
    if let Some(m) = meshes.get(h)
        && matches!(m.attribute(Mesh::ATTRIBUTE_POSITION), Some(V::Float32x3(v)) if *v == pos)
        && matches!(m.attribute(Mesh::ATTRIBUTE_UV_0), Some(V::Float32x2(v)) if *v == uv)
        && matches!(m.attribute(Mesh::ATTRIBUTE_COLOR), Some(V::Float32x4(v)) if *v == colour)
        && matches!(m.indices(), Some(Indices::U32(v)) if *v == index)
    {
        return;
    }
    let Some(mut mesh) = meshes.get_mut(h) else { return };
    EMPTIED.lock().unwrap().push((h.id(), index == [0, 0, 0]));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colour);
    mesh.insert_indices(bevy::mesh::Indices::U32(index));
}

pub fn load_sparks(iso: &mut Iso, commands: &mut Commands, parent: Entity, meshes: &mut Assets<Mesh>, materials: &mut Assets<Fx>, images: &mut Assets<Image>) -> Result<HitSparks, String> {
    let mut looks = Vec::new();
    for k in IMPACTS {
        looks.push(look(iso, &format!("{k}tubu00"), materials, images)?);
    }
    let mesh = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
    let view = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(looks[0].clone()), Transform::default(), Visibility::Hidden, crate::cull::Cullable)).id();
    commands.entity(parent).add_child(view);
    // the table is rolled by the match start's reseed (`HitSparks::restart`)
    let rolls = [Roll::new(0.0, [0.0; 3]); SPARKS];
    Ok(HitSparks { sparks: Sparks::new(rolls), mesh, view, looks: looks.try_into().unwrap() })
}

impl HitSparks {
    /// A burst is playing (tests).
    pub fn live(&self) -> bool {
        self.sparks.live
    }

    /// A reseed message: the whole table rolled from `rng` (the sound generator before the reseed); the burst stops.
    pub fn restart(&mut self, rng: &mut Mt) {
        self.sparks.restart(rng);
    }

    /// Throw a burst for this hit, then play the frame (the game moves a burst the frame it starts); a burst dying
    /// out rerolls from `rng`, the sound generator.
    pub fn frame(&mut self, hit: Option<Hit>, rng: &mut Mt, commands: &mut Commands) {
        if let Some(h) = hit {
            let look = if h.smash { 5 } else { h.kind.clamp(0, 4) as usize };
            commands.entity(self.view).insert(MeshMaterial3d(self.looks[look].clone()));
            self.sparks.start(h.kind, h.smash, [h.pos[0], h.pos[1], h.pos[2], 1.0], [h.vel[0], h.vel[1], h.vel[2], 0.0]);
        }
        self.sparks.tick(rng);
    }
}

/// The sparks as quads facing the camera (game space: the camera's right and screen-down axes), fading over their
/// last frames.
pub fn draw_sparks(fx: Res<HitSparks>, cam: Query<&Transform, With<crate::Orbit>>, mut vis: Query<&mut Visibility>, mut meshes: ResMut<Assets<Mesh>>) {
    let (Ok(cam), Ok(mut v)) = (cam.single(), vis.get_mut(fx.view)) else { return };
    *v = if fx.sparks.live { Visibility::Visible } else { Visibility::Hidden };
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
    fill(&mut meshes, &fx.mesh, pos, uv, colour, index);
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

pub fn load_trails(iso: &mut Iso, commands: &mut Commands, parent: Entity, meshes: &mut Assets<Mesh>, materials: &mut Assets<Fx>, images: &mut Assets<Image>) -> Result<Trails, String> {
    let mesh = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
    let view = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(look(iso, "zanzou", materials, images)?), Transform::default(), crate::cull::Cullable)).id();
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
    fill(&mut meshes, &fx.mesh, pos, uv, colour, index);
}

/// The ball's flight ribbon and glow, each one mesh.
#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct BallFlight {
    flight: Flight,
    glow: Glow,
    #[serde(skip)]
    ribbon: Handle<Mesh>,
    disc: Entity,
    /// The ball at the last frame, and where the ball's own draw interpolates it from (B504).
    pos: Vec3,
    vel: Vec3,
    prev: Vec3,
    /// The glow textures for top … drop: the game's list has `impactef_slice` (the ✕) first and `impactef_top`
    /// (the ○) second, so topspin shows the ✕ and slice the ○, the buttons pressed.
    #[serde(skip)]
    looks: [Handle<Fx>; 5],
}

pub fn load_flight(iso: &mut Iso, commands: &mut Commands, parent: Entity, meshes: &mut Assets<Mesh>, materials: &mut Assets<Fx>, images: &mut Assets<Image>) -> Result<BallFlight, String> {
    let ribbon = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
    let view = commands.spawn((Mesh3d(ribbon.clone()), MeshMaterial3d(look(iso, "ballrolling", materials, images)?), Transform::default(), crate::cull::Cullable)).id();
    // Z21: `Glow::start` shows slice's for flat and top's for drop, so `impactef_flat` and `impactef_drop` are never read
    let (slice, top) = (look(iso, "impactef_slice", materials, images)?, look(iso, "impactef_top", materials, images)?);
    let looks = vec![slice.clone(), top.clone(), slice, look(iso, "impactef_lob", materials, images)?, top];
    let disc = commands.spawn((Mesh3d(meshes.add(Rectangle::new(2.0 * GLOW_SIZE, 2.0 * GLOW_SIZE))), MeshMaterial3d(looks[0].clone()), Transform::default(), Visibility::Hidden)).id();
    commands.entity(parent).add_children(&[view, disc]);
    Ok(BallFlight { flight: Flight::default(), glow: Glow::default(), ribbon, disc, pos: Vec3::ZERO, vel: Vec3::ZERO, prev: Vec3::ZERO, looks: looks.try_into().unwrap() })
}

/// The glow's half-size and turn over its life.
const GLOW_SIZE: f32 = 0.4;
const GLOW_TURN: f32 = std::f32::consts::FRAC_PI_2;

impl BallFlight {
    /// The ribbon is drawn (tests).
    pub fn live(&self) -> bool {
        self.flight.live
    }

    /// One game frame: a hit restarts the ribbon (and, bar a smash, the glow); a dead ball ends both. `prev` is
    /// where the drawn ball comes from this frame (the match's `prev_ball`).
    pub fn frame(&mut self, hit: Option<Hit>, dead: bool, pos: [f32; 3], vel: [f32; 3], prev: [f32; 3], commands: &mut Commands) {
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
        (self.pos, self.vel, self.prev) = (Vec3::from(pos), Vec3::from(vel), Vec3::from(prev));
        self.glow.tick();
    }
}

/// The kept samples newest first, the newest moved to where the ball is drawn (`ball`). The game draws one picture
/// per frame, with the ribbon's head on the ball (its newest sample is the ball's position, B504); the app draws the
/// ball between the last two frames' positions, so a head left on this frame's position ran up to a frame's flight
/// (0.6–0.9 m on a serve) ahead of the ball. The ball lies on the newest kept segment (the last frame's position is
/// the second newest sample or within the collinear test of it), so the ribbon keeps to the ball's path.
fn ribbon_points(f: &Flight, ball: Vec3) -> Vec<Vec3> {
    let mut p: Vec<Vec3> = (0..f.count).map(|k| Vec3::from_slice(&f.points[(f.head + 2 * FLIGHT_POINTS - 1 - k) % FLIGHT_POINTS][..3])).collect();
    if let Some(head) = p.first_mut() {
        *head = ball;
    }
    p
}

/// The ribbon from the ball back along its samples, facing the camera, `speed`·20 long, fading to its end; the glow
/// a disc at the ball facing the flight, spinning a quarter turn about it and fading over its last 15 frames. Both
/// at the ball as drawn (`draw` interpolates it between frames by the fixed clock's overstep).
// ponytail: the game's flag switch on the glow by flight facing (towards/away from the camera) and its halved ribbon
// length under one of its mode counts are left out
pub fn draw_flight(fx: Res<BallFlight>, time: Res<Time<Fixed>>, cam: Query<(&Transform, &Projection), With<crate::Orbit>>, mut disc: Query<(&mut Transform, &mut Visibility, &MeshMaterial3d<Fx>), Without<crate::Orbit>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<Fx>>) {
    let Ok((cam, proj)) = cam.single() else { return };
    let fov = if let Projection::Perspective(p) = proj { p.fov } else { 0.8 };
    // world → game space: (x, −y, −z)
    let eye = Vec3::new(cam.translation.x, -cam.translation.y, -cam.translation.z);
    let ball = fx.prev.lerp(fx.pos, time.overstep_fraction());
    if let Ok((mut t, mut v, look)) = disc.get_mut(fx.disc) {
        *v = if fx.glow.life > 0 { Visibility::Visible } else { Visibility::Hidden };
        let turn = GLOW_TURN * fx.glow.life as f32 / GLOW_FRAMES as f32;
        *t = Transform::from_translation(ball).with_rotation(glow_turn(fx.vel, turn));
        // (written only when it changes: a written material is prepared and uploaded again, B328)
        let colour = Color::srgba(1.0, 1.0, 1.0, (fx.glow.life as f32 / 15.0).min(1.0));
        if materials.get(&look.0).is_some_and(|m| m.base.base_color != colour)
            && let Some(mut m) = materials.get_mut(&look.0)
        {
            m.base.base_color = colour;
        }
    }
    let f = &fx.flight;
    let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
    if f.live && f.count >= 2 {
        let p = ribbon_points(f, ball);
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
    fill(&mut meshes, &fx.ribbon, pos, uv, colour, index);
}

/// The glow's orientation (game space): a frame facing the flight (side = up × flight, then flight × side); a flight
/// within 60° of the court's length is turned about up away from it (by its angle off the length under 30°, easing to
/// none at 60°: the game's row-vector `RotMatrixY` by sign(x)·sign(z)·angle); then the disc spins about the flight by
/// `turn` (`Rz · M`, so it keeps facing the flight).
fn glow_turn(vel: Vec3, turn: f32) -> Quat {
    let d = vel.try_normalize().unwrap_or(Vec3::Z);
    let side = Vec3::Y.cross(d).try_normalize().unwrap_or(Vec3::X);
    let mut basis = Quat::from_mat3(&Mat3::from_cols(side, d.cross(side), d));
    let a = d.x.abs().atan2(d.z.abs());
    let limit = 60f32.to_radians();
    if a < limit {
        basis = Quat::from_rotation_y((limit - a).min(a) * d.x.signum() * d.z.signum()) * basis;
    }
    basis * Quat::from_rotation_z(turn)
}

/// The ball's bounce effects: the sim, the ring and crater models, and one mesh each for marks and dust.
#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct BallBounce {
    bounce: Bounce,
    ring: Shown<Fx>,
    crater: Shown<Fx>,
    /// This flight's contacts so far (the first two bounces).
    contacts: Vec<Contact>,
    #[serde(skip)]
    marks: Handle<Mesh>,
    #[serde(skip)]
    puffs: Handle<Mesh>,
    #[serde(skip)]
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
    materials: &mut Assets<Fx>,
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
        let view = commands.spawn((Mesh3d(m.clone()), MeshMaterial3d(look(iso, tex, materials, images)?), Transform::default(), crate::cull::Cullable)).id();
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
    /// `smash` a smash landing at 85 km/h or more; `fading` the marks age (`play::marks_age`); `recorded` a bounce
    /// now leaves a contact record (`play::bounce_recorded`): without one it makes no mark, ring or puff.
    // ponytail: every contact's surface is the flat court (normal up); the stage mesh's own normal would tilt the ring
    pub fn frame(&mut self, bounces: i32, at: [f32; 3], vel: [f32; 3], court: bool, smash: bool, fading: bool, recorded: bool, transforms: &mut Query<&mut Transform>) {
        if bounces == 0 {
            self.contacts.clear();
        } else if recorded && bounces as usize > self.contacts.len() && self.contacts.len() < 3 {
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
    cam: Query<&Transform, With<crate::Orbit>>,
    mut q: Query<(&mut Visibility, &mut MorphWeights)>,
    mut joints: Query<&mut Transform, Without<crate::Orbit>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<Fx>>,
) {
    let b = &fx.bounce;
    pose(&b.ring, &fx.ring, &mut q, &mut joints, &mut materials);
    pose(&b.crater, &fx.crater, &mut q, &mut joints, &mut materials);
    let Ok(cam) = cam.single() else { return };
    let game = |v: Vec3| Vec3::new(v.x, -v.y, -v.z);
    let (right, up) = (game(cam.rotation * Vec3::X), game(cam.rotation * Vec3::Y));
    let mut put = |h: &Handle<Mesh>, quads: Vec<([Vec3; 4], [f32; 2], [f32; 4])>| {
        let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
        for (corners, [v0, v1], c) in quads {
            let n = pos.len() as u32;
            pos.extend(corners.map(|v| v.to_array()));
            uv.extend([[0.0, v0], [1.0, v0], [0.0, v1], [1.0, v1]]);
            colour.extend([c; 4]);
            index.extend([n, n + 1, n + 2, n + 2, n + 1, n + 3]);
        }
        fill(&mut meshes, h, pos, uv, colour, index);
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
    put(&fx.marks, quads);
    let [r, g, bl] = fx.look.0;
    let billboard = |p: &Puff| {
        let (foot, w, h) = (v3(p.pos), right * p.size, up * 2.0 * p.size);
        ([foot - w + h, foot + w + h, foot - w, foot + w], [0.0, 1.0], [r, g, bl, p.alpha / 128.0])
    };
    put(&fx.puffs, b.puffs.iter().filter(|p| p.life > 0).map(billboard).collect());
    put(&fx.dust, b.dust.iter().map(|(p, _, _)| billboard(p)).collect());
}

/// The landing markers: `chakudan_p` (red, a looping pulse) at the shot's aim and `smash_p` (yellow, one 120-frame
/// play, then held) at the smash point; each placed on the court at (x, z) or hidden.
#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct LandingMarks {
    red: (Effect, Shown<Fx>),
    smash: (Effect, Shown<Fx>),
    pub red_at: Option<[f32; 2]>,
    pub smash_at: Option<[f32; 2]>,
}

#[allow(clippy::too_many_arguments)]
pub fn load_marks(
    iso: &mut Iso,
    commands: &mut Commands,
    parent: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<Fx>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<LandingMarks, String> {
    let data = iso.read("PCDATA/PCCG0.XB").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let mut red = model(&arc, "taguchi/other/chakudan_p", commands, parent, meshes, materials, images, bindposes)?;
    // never restarted: the pulse runs on from wherever it was, ticked only while shown
    red.0.looping = true;
    red.0.start();
    let mut smash = model(&arc, "taguchi/other/smash_p", commands, parent, meshes, materials, images, bindposes)?;
    // held on its last frame, as the game: the blob fades out over 120 frames, past the 105-frame morph
    smash.0.hold = true;
    for (s, (_, view)) in [("taguchi/other/chakudan_p", &red), ("taguchi/other/smash_p", &smash)] {
        layer(&arc, s, view, materials)?;
    }
    Ok(LandingMarks { red, smash, red_at: None, smash_at: None })
}

/// Blended draws carry no depth, so Bevy orders them by mesh centre, and a marker's flat layers all share one centre
/// (an unstable tie: the rings and the blob swapped from frame to frame) and lose to the huge court-line mesh whenever
/// its centre is nearer (the lines then cut through the marker). Draw them after the court and bottom layer first, as
/// the PS2's Z resolves the layers stacked a millimetre apart, the marker always over the lines (court 10).
fn layer(arc: &Archive, s: &str, view: &Shown<Fx>, materials: &mut Assets<Fx>) -> Result<(), String> {
    let e = arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&format!("{s}.mdl"))).ok_or(format!("{s}.mdl missing"))?;
    let model = mdl::parse(&arc.read(e).map_err(|e| e.to_string())?).map_err(|e| e.0)?;
    let y = |packets: &Vec<mdl::Packet>| {
        let v: Vec<f32> = packets.iter().flat_map(|p| p.vertices.iter().map(|v| v.pos[1])).collect();
        v.iter().sum::<f32>() / v.len().max(1) as f32
    };
    let mut order: Vec<usize> = (0..model.materials.len().min(view.materials.len())).collect();
    order.sort_by(|&a, &b| y(&model.materials[a]).total_cmp(&y(&model.materials[b])));
    for (rank, &i) in order.iter().enumerate() {
        if let Some(mut m) = materials.get_mut(&view.materials[i]) {
            // past any court mesh's centre (tens of metres), short of the head markers' LAST
            m.base.depth_bias = 1e3 + rank as f32;
        }
    }
    Ok(())
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

pub fn draw_marks(fx: Res<LandingMarks>, mut q: Query<(&mut Visibility, &mut MorphWeights)>, mut joints: Query<&mut Transform>, mut materials: ResMut<Assets<Fx>>) {
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

// a resync's state (`play::snapshot::encode`) keeps this machine's drawing entities and handles
impl crate::Wire for Impacts {
    fn local(&mut self, l: &Self) {
        self.models.iter_mut().zip(&l.models).for_each(|(m, l)| m.1 = l.1.clone());
    }
}

impl crate::Wire for HitSparks {
    fn local(&mut self, l: &Self) {
        (self.mesh, self.view, self.looks) = (l.mesh.clone(), l.view, l.looks.clone());
    }
}

impl crate::Wire for BallFlight {
    fn local(&mut self, l: &Self) {
        (self.ribbon, self.disc, self.looks) = (l.ribbon.clone(), l.disc, l.looks.clone());
    }
}

impl crate::Wire for BallBounce {
    fn local(&mut self, l: &Self) {
        (self.ring, self.crater) = (l.ring.clone(), l.crater.clone());
        (self.marks, self.puffs, self.dust) = (l.marks.clone(), l.puffs.clone(), l.dust.clone());
    }
}

impl crate::Wire for LandingMarks {}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn cpu_morph_adds_the_weighted_targets() {
        let mut world = World::new();
        let mut meshes = Assets::<Mesh>::default();
        let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        let t = |p: [f32; 3]| bevy::mesh::morph::MorphAttributes::new(Vec3::from(p), Vec3::ZERO, Vec3::ZERO);
        mesh.set_morph_targets(vec![t([1.0, 0.0, 0.0]), t([0.0; 3]), t([0.0; 3]), t([0.0; 3]), t([0.0, 2.0, 0.0]), t([0.0, 0.0, 4.0])]);
        let h = meshes.add(mesh);
        let root = world.spawn(MorphWeights::new(vec![0.5, 0.25], None).unwrap()).id();
        let m = cpu_morph(&h, root, &mut meshes);
        assert!(meshes.get(&h).unwrap().morph_targets().is_none());
        world.spawn((m, Mesh3d(h.clone())));
        world.insert_resource(meshes);
        world.run_system_once(morph_on_cpu).unwrap();
        let pos = world.resource::<Assets<Mesh>>().get(&h).unwrap().attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap().to_vec();
        assert_eq!(pos, vec![[0.5, 0.0, 0.0], [1.0, 0.5, 0.0], [0.0, 1.0, 1.0]]);
    }

    /// B504: a curving serve's samples (Lola's slice serve, as the original keeps them); the drawn ball halfway
    /// between two frames heads the ribbon, which then runs back along the kept samples.
    #[test]
    fn ribbon_starts_at_the_drawn_ball() {
        let mut f = Flight::default();
        f.start(1, false);
        let path = [[2.95, -2.70, -11.93], [2.62, -2.66, -11.38], [2.31, -2.62, -10.84], [2.01, -2.58, -10.31], [1.71, -2.54, -9.79]];
        for (i, p) in path.iter().enumerate() {
            let v = if i > 0 { std::array::from_fn(|k| p[k] - path[i - 1][k]) } else { [0.0; 3] };
            f.tick([p[0], p[1], p[2], 1.0], [v[0], v[1], v[2], 0.0]);
        }
        let (prev, now) = (Vec3::from(path[3]), Vec3::from(path[4]));
        let ball = prev.lerp(now, 0.5);
        let p = ribbon_points(&f, ball);
        assert_eq!(p[0], ball);
        assert_eq!(p.len(), f.count);
        // the rest are the samples the game keeps, older than the last frame's ball
        assert!(p[1..].iter().all(|q| path.iter().any(|s| Vec3::from(*s) == *q)));
        assert!(p[1].distance(now) >= prev.distance(now) - 1e-6);
    }
}
