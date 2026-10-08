//! Characters: a skeleton, skinned meshes and motions, independent of where they come from. [`load_disc`] builds
//! one from the game's files on the user's disc (`PC/PCnnCcc.XB` model and racket, `PCANI/PCnnANI.XB` motions);
//! a custom character only has to fill the same [`CharacterData`]: joints named like the 3ds Max Biped skeleton
//! the game uses (`Bip01Pelvis`, `Bip01RHand`, …, plus `Racket` where the racket sits) and clips keyed by the
//! game's motion numbers ([`hst_data::ani::MOTIONS`]). Built in game space (Y down, feet at the origin).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use hst_data::{ani, iso::Iso, mdl, mor, mtl, xb::Archive};
use hst_sim::pose::{arm_table, ArmTable, Clip, Path, Skeleton};
use hst_sim::motion::{Clock, Fade};

/// One joint of the skeleton.
pub struct Joint {
    pub name: String,
    pub parent: Option<usize>,
    /// Local placement at rest.
    pub rest: Transform,
    /// Model space → joint space in the bind pose.
    pub inverse_bind: Mat4,
}

pub struct CharacterData {
    pub joints: Vec<Joint>,
    /// Skinned body parts.
    /// Skinned parts; `true` where the mesh carries the face morph targets.
    pub parts: Vec<(Handle<Mesh>, Handle<StandardMaterial>, bool)>,
    /// Rigid parts carried by the `Racket` joint.
    pub racket: Vec<(Handle<Mesh>, Handle<StandardMaterial>)>,
    /// Motions by the game's motion number, bound to the skeleton (the game's sampler, `hst_sim::pose`).
    pub motions: HashMap<usize, Clip>,
    /// Root paths of the motions that carry the player, by motion number: the post-point reactions (`gu_set` from
    /// the character's own `*_gu_set_dummy`, the team reactions co03–co05 from their shared dummies) and the
    /// dive (`receive_f` from `*_receive_f_dummy`).
    pub paths: HashMap<usize, Path>,
    /// Every joint's inverse bind matrix, for the skinned parts.
    pub binds: Handle<SkinnedMeshInverseBindposes>,
    /// Per motion number (0..48): the forward row (x, z) of `Bip01Pelvis`'s model matrix at the motion's first
    /// frame — what the game's body turn compares (`hst_sim::player::turn`).
    pub pelvis: Vec<[f32; 2]>,
    /// The arm table (strokes 0x10–0x19 and volleys 0x1a/0x1b at frame 8) the contact solve reaches from.
    pub arm: Option<Arc<ArmTable>>,
    /// Face morph targets of the skinned parts (`MorphWeights` on the rig root, one weight per target).
    pub morph_targets: usize,
    /// Faces by motion number (`.MOR`, `hst_sim::face`).
    pub faces: HashMap<usize, Face>,
    /// The ball's track through the serve stance (0x20; `*_serve_ad00_ball`).
    pub stance_ball: Option<Path>,
    /// The model's skeleton in the game's terms (bone-attached points such as the held serve ball).
    pub skeleton: Skeleton,
    /// The costume's noise deformers (`.NOI`), if it sways.
    pub noise: Option<Arc<crate::noise::Costume>>,
    /// Each part's material as the GS draws it (one, or two for TEST mode 20..29), VU1-lit: `shade` swaps them in,
    /// per rig.
    pub gs: HashMap<AssetId<StandardMaterial>, Vec<crate::gs::GsMaterial>>,
    /// A mod's texture face (standard §4b), if it swaps whole-face textures instead of morphing.
    pub texture_face: Option<TextureFace>,
}

/// A texture face (standard §4b): the face materials show the strongest channel's whole-face texture above 0.5,
/// else the neutral one. Each texture as (sRGB for a `StandardMaterial`, raw for the GS draws).
pub struct TextureFace {
    pub neutral: (Handle<Image>, Handle<Image>),
    /// Per channel: its weight in the rig's `MorphWeights` (the donor's `.MOR` tracks drive it) and its texture.
    pub channels: Vec<(usize, (Handle<Image>, Handle<Image>))>,
}

impl TextureFace {
    /// The texture shown at weights `w`: the first strongest channel above 0.5, else neutral.
    pub fn pick(&self, w: &[f32]) -> &(Handle<Image>, Handle<Image>) {
        strongest(self.channels.iter().map(|c| c.0), w).map_or(&self.neutral, |k| &self.channels[k].1)
    }

    fn holds(&self, h: &Handle<Image>) -> bool {
        std::iter::once(&self.neutral).chain(self.channels.iter().map(|c| &c.1)).any(|t| t.0 == *h || t.1 == *h)
    }
}

/// Of the channels (by weight index), the first strongest one above 0.5.
fn strongest(channels: impl Iterator<Item = usize>, w: &[f32]) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (k, t) in channels.enumerate() {
        let x = w.get(t).copied().unwrap_or(0.0);
        if x > 0.5 && best.is_none_or(|b| x > b.1) {
            best = Some((k, x));
        }
    }
    best.map(|b| b.0)
}

/// Swap every texture-face rig's face texture to the one its weights pick (after `animate` sets them).
pub fn texture_faces(
    rigs: Query<(&Rig, &bevy::mesh::morph::MorphWeights, &Children)>,
    parts: Query<AnyOf<(&MeshMaterial3d<StandardMaterial>, &MeshMaterial3d<crate::gs::GsMaterial>)>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut gs: ResMut<Assets<crate::gs::GsMaterial>>,
) {
    for (rig, w, children) in &rigs {
        let Some(face) = &rig.data.texture_face else { continue };
        let (srgb, raw) = face.pick(w.weights());
        for (s, g) in parts.iter_many(children) {
            // only the face's draws carry a face texture; touch them only on a change
            if let Some(m) = s.filter(|m| std_materials.get(&m.0).and_then(|m| m.base_color_texture.as_ref()).is_some_and(|t| t != srgb && face.holds(t))) {
                std_materials.get_mut(&m.0).unwrap().base_color_texture = Some(srgb.clone());
            }
            if let Some(m) = g.filter(|m| gs.get(&m.0).and_then(|m| m.texture.as_ref()).is_some_and(|t| t != raw && face.holds(t))) {
                gs.get_mut(&m.0).unwrap().texture = Some(raw.clone());
            }
        }
    }
}

/// A motion's face: per bound track its morph target, ticks and weights, and the face clock's length.
pub struct Face {
    pub tracks: Vec<(usize, Vec<i32>, Vec<[f32; 4]>)>,
    pub length: f32,
}

impl CharacterData {
    pub fn joint(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name == name)
    }
}

/// A spawned character: its data and the entity of every joint.
#[derive(Component)]
pub struct Rig {
    pub data: Arc<CharacterData>,
    pub joints: Vec<Entity>,
}

/// The motion a character plays: the game's motion number and its clock (`hst_sim::motion::Clock`), with the
/// sampled time one tick earlier (drawing blends the two).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub id: usize,
    pub clock: Clock,
    pub prev: f32,
    /// Bumped by whoever restarts the motion (lets a driver restart the same motion).
    pub serial: u32,
    /// The contact IK's turn of right hand, right forearm, right upper arm and left upper arm, and its weight.
    pub arm: Option<([[f32; 4]; 4], f32)>,
    /// The face playing (`CharacterData::faces`) and its clock.
    pub face: usize,
    pub face_clock: Clock,
    /// The crossfade from the outgoing motion, and this frame's mix weight (`true`: the held fade, the new motion's
    /// frame 0 over the frozen outgoing pose; else the outgoing pose over the new one).
    pub fade: Fade,
    pub mix: Option<(f32, bool)>,
    /// Bumped by every [`Motion::set`], a restart of the same motion included (the costume deformers restart).
    pub sets: u32,
}

impl Default for Motion {
    fn default() -> Self {
        let clock = Clock::start(1.0, true, None);
        Motion { id: 0, clock, prev: 0.0, serial: 0, arm: None, face: 0, face_clock: clock, fade: Fade::default(), mix: None, sets: 0 }
    }
}

impl Motion {
    /// The game's motion setter: restart motion `id` at its start.
    pub fn set(&mut self, id: usize, speed: f32, looping: bool, hold: Option<i32>, serial: u32) {
        // the soft follow-throughs 0x1c/0x1d keep the stroke's face; the face clock restarts with the motion
        // ponytail: the face clock ignores the crossfade hold; team reactions (0x30..) use their own clip's face, not the co offset
        let face = if id == 0x1c || id == 0x1d { self.face } else { id };
        // crossfade length: the held count's frames, 8 for looping motions and whiffs, else a cut
        // ponytail: stand↔ready's track 6 (the game's longest-arc slerp) mixes like the others
        let frames = match hold {
            Some(n) => n + 1,
            None if looping || id == 0x27 || id == 0x28 => 8,
            None => 0,
        };
        let mut fade = self.fade;
        fade.start(frames, id as i32, hold.is_some(), self.id as i32, true, &self.clock);
        let face_clock = Clock::start(speed, looping, None);
        *self = Motion { id, clock: Clock::start(speed, looping, hold), prev: 0.0, serial, arm: self.arm, face, face_clock, fade, mix: None, sets: self.sets.wrapping_add(1) };
    }

    /// Switch to motion `id` from its start (keeps going if it already plays).
    pub fn play(&mut self, id: usize, speed: f32, looping: bool) {
        if self.id != id {
            self.set(id, speed, looping, None, self.serial);
        } else {
            self.clock.speed = speed;
            self.clock.looping = looping;
            self.face_clock.speed = speed;
            self.face_clock.looping = looping;
        }
    }
}

/// Row-vector game matrix → Bevy (column-vector) matrix: the rows become the columns.
fn mat(m: &[[f32; 4]; 4]) -> Mat4 {
    Mat4::from_cols_array_2d(m)
}

pub fn texture_image(t: &mtl::Texture) -> Image {
    use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut img = Image::new(
        Extent3d { width: t.width.max(1), height: t.height.max(1), depth_or_array_layers: 1 },
        TextureDimension::D2,
        if t.rgba.is_empty() { vec![255; 4 * (t.width.max(1) * t.height.max(1)) as usize] } else { t.rgba.clone() },
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    img
}

/// Materials of an MTL with their textures uploaded.
fn materials_of(mats: &mtl::Mtl, images: &mut Assets<Image>, materials: &mut Assets<StandardMaterial>) -> Vec<Handle<StandardMaterial>> {
    let tex: Vec<Handle<Image>> = mats.textures.iter().map(|t| crate::textures::add_mtl(images, texture_image(t), t)).collect();
    mats.materials
        .iter()
        .map(|m| {
            let [r, g, b, a] = m.color;
            materials.add(StandardMaterial {
                base_color: Color::linear_rgba(r, g, b, a),
                base_color_texture: m.texture.map(|t| tex[t].clone()),
                unlit: true,
                double_sided: true,
                cull_mode: None,
                // character texture alpha is not coverage (it holds shading masks): draw opaque
                alpha_mode: AlphaMode::Opaque,
                ..default()
            })
        })
        .collect()
}

/// The GS draws of `mats` for `model`'s batches (by its first packet's PRIM; raw texels), by the material's handle.
fn gs_of(model: &mdl::Model, mats: &mtl::Mtl, handles: &[Handle<StandardMaterial>], images: &mut Assets<Image>) -> HashMap<AssetId<StandardMaterial>, Vec<crate::gs::GsMaterial>> {
    let tex: Vec<Handle<Image>> = mats
        .textures
        .iter()
        .map(|t| {
            let mut img = texture_image(t);
            img.texture_descriptor.format = bevy::render::render_resource::TextureFormat::Rgba8Unorm;
            crate::textures::add_mtl(images, img, t)
        })
        .collect();
    let mut out = HashMap::new();
    for (mi, (m, h)) in mats.materials.iter().zip(handles).enumerate() {
        let prim = model.materials.get(mi).and_then(|p| p.first()).map_or(0x10, |p| p.prim);
        let mut draws = crate::gs::GsMaterial::for_batch(m, prim, m.texture.map(|t| tex[t].clone()));
        draws.iter_mut().for_each(|g| g.uniform.lod_k = model.lod_k.get(mi).copied().unwrap_or(0.0));
        out.insert(h.id(), draws);
    }
    out
}

/// A skinned model's parts, one mesh per material (joints = the model's nodes), with its morph targets; `true`
/// where a part carries them.
pub fn skinned_parts(model: &mdl::Model, handles: &[Handle<StandardMaterial>], meshes: &mut Assets<Mesh>) -> Vec<(Handle<Mesh>, Handle<StandardMaterial>, bool)> {
    #[allow(clippy::type_complexity)]
    let mut by_material: HashMap<usize, (Vec<[f32; 3]>, Vec<[[f32; 3]; 2]>, Vec<[f32; 2]>, Vec<[f32; 4]>, Vec<[u16; 4]>, Vec<[f32; 4]>, Vec<u32>, Vec<Vec<[f32; 3]>>)> = HashMap::new();
    let targets = model.morph_names.len();
    for (material, verts, tris, morph) in model.skinned() {
        let e = by_material.entry(material).or_default();
        let base = e.0.len() as u32;
        // per target, every vertex of the material (0 where a packet has no morphs)
        e.7.resize(targets, Vec::new());
        for (t, offsets) in e.7.iter_mut().enumerate() {
            offsets.resize(base as usize, [0.0; 3]);
            offsets.extend(morph.get(t).cloned().unwrap_or_else(|| vec![[0.0; 3]; verts.len()]));
        }
        for v in &verts {
            e.0.push(v.pos);
            e.1.push(v.normals);
            e.2.push(v.uv);
            e.3.push(v.color.map(|c| c as f32 / 128.0));
            e.4.push(v.joints);
            e.5.push(v.weights);
        }
        e.6.extend(tris.iter().flatten().map(|i| i + base));
    }
    let mut parts = Vec::new();
    let mut keys: Vec<_> = by_material.keys().copied().collect();
    keys.sort();
    for m in keys {
        let (pos, nrm, uv, col, joint, weight, idx, morph) = by_material.remove(&m).unwrap();
        if idx.is_empty() {
            continue;
        }
        let n = pos.len();
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            // VU1's per-bone normals (gs.wgsl): the first joint's as the normal, the second's in the tangent slot
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm.iter().map(|n| n[0]).collect::<Vec<_>>())
            .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, nrm.iter().map(|n| [n[1][0], n[1][1], n[1][2], 0.0]).collect::<Vec<_>>())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, bevy::mesh::VertexAttributeValues::Uint16x4(joint))
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weight)
            .with_inserted_indices(Indices::U32(idx));
        let morphed = morph.iter().flatten().any(|d| *d != [0.0; 3]);
        if morphed {
            let attrs = morph.iter().flat_map(|t| (0..n).map(|v| bevy::mesh::morph::MorphAttributes::new(Vec3::from(t.get(v).copied().unwrap_or_default()), Vec3::ZERO, Vec3::ZERO)));
            mesh.set_morph_targets(attrs.collect());
        }
        parts.push((meshes.add(mesh), handles.get(m).cloned().unwrap_or_default(), morphed));
    }
    parts

}

/// Build a character from the disc: character `n` (0..9) in costume `costume`.
pub fn load_disc(
    iso: &mut Iso,
    n: usize,
    costume: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<CharacterData, String> {
    let data = iso.read(&format!("PC/PC{n:02}C{costume:02}.XB")).map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let find = |suffix: &str| arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix)).and_then(|e| arc.read(e).ok());
    // pcNN_tTT_cCC: TT is the character's body type (standard, short, tall, …)
    let body = arc
        .entries
        .iter()
        .map(|e| e.name.to_ascii_lowercase())
        .filter_map(|name| name.rsplit(['\\', '/']).next().map(str::to_string))
        .find(|f| f.starts_with(&format!("pc{n:02}_t")) && f.ends_with(&format!("_c{costume:02}.mdl")))
        .map(|f| f.trim_end_matches(".mdl").to_string())
        .ok_or("no body model")?;
    let model = mdl::parse(&find(&format!("{body}.mdl")).ok_or("no body model")?).map_err(|e| e.0)?;
    let mats = mtl::parse(&find(&format!("{body}.mtl")).ok_or("no body MTL")?, find(&format!("{body}.mti")).as_deref()).map_err(|e| e.0)?;
    let handles = materials_of(&mats, images, materials);
    let mut gs = gs_of(&model, &mats, &handles, images);

    let joints: Vec<Joint> = (0..model.node_count)
        .map(|i| Joint {
            name: model.node_names[i].clone(),
            parent: model.node_parent[i],
            rest: Transform::from_matrix(mat(&model.node_local[i])),
            inverse_bind: mat(&model.node_bind[i]).inverse(),
        })
        .collect();

    let parts = skinned_parts(&model, &handles, meshes);
    let noise = find(&format!("{body}.noi")).and_then(|d| crate::noise::costume(&model, &d, &parts, meshes));
    let targets = model.morph_names.len();

    let racket = disc_racket(&arc, n, meshes, materials, images, &mut gs)?;

    let skeleton = Skeleton { names: model.node_names.clone(), parent: model.node_parent.clone(), rest: model.node_local.clone() };
    // the game binds a face track to the target of the same name; others are dropped
    let Motions { motions, paths, pelvis, arm, faces, stance_ball } = disc_motions(iso, n, &skeleton, |name| model.morph_names.iter().position(|m| m == name))?;
    let binds = bindposes.add(SkinnedMeshInverseBindposes::from(joints.iter().map(|j| j.inverse_bind).collect::<Vec<_>>()));
    Ok(CharacterData { joints, parts, racket, motions, paths, binds, pelvis, arm, morph_targets: targets, faces, stance_ball, skeleton, noise, gs, texture_face: None })
}

/// What a character takes from its disc motion set: the clips, root paths, pelvis rows, arm table, faces and the
/// serve stance's ball (see [`CharacterData`]).
pub struct Motions {
    pub motions: HashMap<usize, Clip>,
    pub paths: HashMap<usize, Path>,
    pub pelvis: Vec<[f32; 2]>,
    pub arm: Option<Arc<ArmTable>>,
    pub faces: HashMap<usize, Face>,
    pub stance_ball: Option<Path>,
}

/// The menu's inspect pose (`MENU/PC/PCnn.XB` `pcNN_pose.ANI`/`.MOR`), keyed past the game's motion numbers.
pub const POSE: usize = 0x40;

/// The characters whose inspect pose shows its end rather than frame 0.
const POSE_AT_END: [bool; 16] = [true, true, false, true, false, true, false, true, false, false, true, true, false, true, false, false];

/// The pose frame character `n`'s inspect screen holds (its motion never advances there): the clip's whole length −
/// 1 for the characters that show the end (Ashley a fixed 69), else 0.
pub fn pose_frame(n: usize, length: f32) -> f32 {
    match POSE_AT_END.get(n) {
        Some(true) if n == 0 => 69.0,
        Some(true) => (length as i32 - 1) as f32,
        _ => 0.0,
    }
}

/// Character `n`'s motions (`PCANI/PCnnANI.XB`), the shared team reactions (`PCDATA/PCCG0.XB`) and its inspect
/// pose ([`POSE`]), bound to `skeleton`; `target` binds a `.MOR` track name to a morph target.
pub fn disc_motions(iso: &mut Iso, n: usize, skeleton: &Skeleton, target: impl Fn(&str) -> Option<usize>) -> Result<Motions, String> {
    // motions by the game's numbers
    let anims = iso.read(&format!("PCANI/PC{n:02}ANI.XB")).map_err(|e| e.to_string())?;
    let aarc = Archive::parse(&anims).map_err(|e| e.0)?;
    let mut motions = HashMap::new();
    let hip = skeleton.names.iter().position(|n| n == "Bip01Pelvis");
    let mut pelvis = vec![[0.0, 1.0]; 48];
    let mut paths = HashMap::new();
    let mut faces = HashMap::new();
    let mut stance_ball = None;
    for id in 0..ani::MOTIONS.len() {
        let stem = ani::motion_name(id, n).unwrap().to_ascii_lowercase();
        if let Some(t) = aarc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.mor"))).and_then(|e| mor::parse(&aarc.read(e).ok()?, 1).ok()) {
            let tracks: Vec<_> = t.tracks.into_iter().filter_map(|tr| Some((target(&tr.name)?, tr.ticks, tr.values))).collect();
            let length = hst_sim::face::length(tracks.iter().map(|t| &t.1[..]), t.ticks_per_frame);
            faces.insert(id, Face { tracks, length });
        }
        let Some(e) = aarc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))) else { continue };
        let Ok(a) = ani::parse(&aarc.read(e).map_err(|e| e.0)?) else { continue };
        if let (Some(h), Some(row)) = (hip, pelvis.get_mut(id)) {
            let r = hst_sim::pose::first_frame(skeleton, &a)[h][2];
            *row = [r[0], r[2]];
        }
        // the motion numbers 0x30.. are the team reactions (below); the files of those names are ball paths
        if id < 0x30 {
            motions.insert(id, Clip::new(skeleton, &a));
        }
        // gu_set's root path
        if id == 0x35 {
            paths.extend(Path::new(&a).map(|p| (0x2e, p)));
        }
        // the dive's (receive_f) root path
        if id == 51 {
            paths.extend(Path::new(&a).map(|p| (0x1e, p)));
        }
        if id == 52 {
            stance_ball = Path::new(&a);
        }
    }
    // doubles team reactions (motions 0x30..0x34): one skeletal clip each, shared by every character
    if let Ok(cg) = iso.read("PCDATA/PCCG0.XB") {
        let carc = Archive::parse(&cg).map_err(|e| e.0)?;
        for (k, stem) in ["co01_f", "co02_f", "co03", "co04", "co05"].iter().enumerate() {
            let name = format!("mtgrl/re_pc00_{stem}.ani2");
            let Some(e) = carc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(&name)) else { continue };
            let Ok(a) = ani::parse(&carc.read(e).map_err(|e| e.0)?) else { continue };
            motions.insert(0x30 + k, Clip::new(skeleton, &a));
            let name = format!("mtgrl/re_pc00_{stem}_dummy.ani2");
            let Some(e) = carc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(&name)) else { continue };
            let Ok(a) = ani::parse(&carc.read(e).map_err(|e| e.0)?) else { continue };
            paths.extend(Path::new(&a).map(|p| (0x30 + k, p)));
        }
    }
    // the menu's inspect pose (hierarchical: the scene's own nodes bind to nothing and drop out)
    if let Ok(d) = iso.read(&format!("MENU/PC/PC{n:02}.XB")) {
        let marc = Archive::parse(&d).map_err(|e| e.0)?;
        let file = |ext: &str| marc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("pc{n:02}_pose.{ext}"))).and_then(|e| marc.read(e).ok());
        if let Some(a) = file("ani").and_then(|d| ani::parse(&d).ok()) {
            motions.insert(POSE, Clip::new(skeleton, &a));
        }
        if let Some(t) = file("mor").and_then(|d| mor::parse(&d, 1).ok()) {
            let tracks: Vec<_> = t.tracks.into_iter().filter_map(|tr| Some((target(&tr.name)?, tr.ticks, tr.values))).collect();
            let length = hst_sim::face::length(tracks.iter().map(|t| &t.1[..]), t.ticks_per_frame);
            faces.insert(POSE, Face { tracks, length });
        }
    }
    let strokes: Option<Vec<Clip>> = (0x10..0x1c).map(|m| motions.get(&m).cloned()).collect();
    let arm = strokes.map(|c| Arc::new(arm_table(skeleton, &c, n as i32)));
    Ok(Motions { motions, paths, pelvis, arm, faces, stance_ball })
}

/// Character `n`'s racket from its costume archive `arc`: a rigid model in its own space, carried by the `Racket`
/// joint; its GS draws go into `gs`.
pub fn disc_racket(
    arc: &Archive,
    n: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    gs: &mut HashMap<AssetId<StandardMaterial>, Vec<crate::gs::GsMaterial>>,
) -> Result<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>, String> {
    let find = |suffix: &str| arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix)).and_then(|e| arc.read(e).ok());
    let mut racket = Vec::new();
    let rk = format!("racket_{n:02}");
    if let (Some(rm), Some(rt)) = (find(&format!("{rk}.mdl")), find(&format!("{rk}.mtl"))) {
        let rmodel = mdl::parse(&rm).map_err(|e| e.0)?;
        let rmats = mtl::parse(&rt, find(&format!("{rk}.mti")).as_deref()).map_err(|e| e.0)?;
        let rh = materials_of(&rmats, images, materials);
        gs.extend(gs_of(&rmodel, &rmats, &rh, images));
        // the gut (TEST mode 25 in an ABE batch): A ≥ 0x70 writes colour and Z, the rest colour only, both blended
        // (Cs − Cd)·As + Cd; its texels are 0 or 0x80, so the holes between the strings show what is behind
        // ponytail: drawn without Z for the A ≥ 0x70 half too; nothing in the racket sits behind the strings but the frame
        for ((m, h), packets) in rmats.materials.iter().zip(&rh).zip(&rmodel.materials) {
            let mode = i16::from_le_bytes([m.header[0x1e], m.header[0x1f]]);
            if (20..30).contains(&mode) && packets.iter().any(|p| p.prim & 0x40 != 0) {
                if let Some(mut mat) = materials.get_mut(h) {
                    mat.alpha_mode = AlphaMode::Blend;
                }
            }
        }
        for (mi, packets) in rmodel.materials.iter().enumerate() {
            let (mut pos, mut nrm, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![], vec![]);
            for pk in packets {
                let base = pos.len() as u32;
                for v in &pk.vertices {
                    pos.push(v.pos);
                    nrm.push(v.normal);
                    uv.push(v.uv);
                    col.push(v.color.map(|c| c as f32 / 128.0));
                }
                idx.extend(pk.triangles.iter().flatten().map(|i| i + base));
            }
            if idx.is_empty() {
                continue;
            }
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
                .with_inserted_indices(Indices::U32(idx));
            racket.push((meshes.add(mesh), rh.get(mi).cloned().unwrap_or_default()));
        }
    }

    Ok(racket)
}

/// Build a background figure (umpire, spectator, creature) from a court archive: the model `{stem}.MDL` with its
/// MTL/MTI and the clips `anims` (motion k = the kth file; a missing file leaves k unset). Paths are matched
/// case-blind against the archive's names, ending in `stem`.
pub fn load_npc(
    arc: &Archive,
    stem: &str,
    anims: &[String],
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Option<CharacterData> {
    let find = |name: &str| {
        let name = name.to_ascii_lowercase();
        arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(&name)).and_then(|e| arc.read(e).ok())
    };
    let model = mdl::parse(&find(&format!("{stem}.mdl"))?).ok()?;
    let mats = mtl::parse(&find(&format!("{stem}.mtl"))?, find(&format!("{stem}.mti")).as_deref()).ok()?;
    let handles = materials_of(&mats, images, materials);
    let gs = gs_of(&model, &mats, &handles, images);
    // their texture alpha is coverage: TEST mode 10..19 keeps A ≥ 0x40, 20..29 blends (the ground shadow quad)
    // ponytail: mode 20..29's A ≥ 0x70 half writes Z on the PS2; drawn blended without it
    for (m, h) in mats.materials.iter().zip(&handles) {
        let mode = i16::from_le_bytes([m.header[0x1e], m.header[0x1f]]);
        if let (10..=29, Some(mut mat)) = (mode, materials.get_mut(h)) {
            mat.alpha_mode = if mode < 20 { AlphaMode::Mask(0.5) } else { AlphaMode::Blend };
        }
    }
    let joints: Vec<Joint> = (0..model.node_count)
        .map(|i| Joint {
            name: model.node_names[i].clone(),
            parent: model.node_parent[i],
            rest: Transform::from_matrix(mat(&model.node_local[i])),
            inverse_bind: mat(&model.node_bind[i]).inverse(),
        })
        .collect();
    let skeleton = Skeleton { names: model.node_names.clone(), parent: model.node_parent.clone(), rest: model.node_local.clone() };
    let motions = anims
        .iter()
        .enumerate()
        .filter_map(|(k, a)| Some((k, Clip::new(&skeleton, &ani::parse(&find(a)?).ok()?))))
        .collect();
    let binds = bindposes.add(SkinnedMeshInverseBindposes::from(joints.iter().map(|j| j.inverse_bind).collect::<Vec<_>>()));
    Some(CharacterData {
        parts: skinned_parts(&model, &handles, meshes),
        joints,
        racket: Vec::new(),
        motions,
        paths: HashMap::new(),
        binds,
        pelvis: Vec::new(),
        arm: None,
        morph_targets: model.morph_names.len(),
        faces: HashMap::new(),
        stance_ball: None,
        skeleton,
        noise: None,
        gs,
        texture_face: None,
    })
}

/// Spawn a character under `parent` (game space); returns its root (carry `Transform` and `Motion` on it).
pub fn spawn(commands: &mut Commands, data: &Arc<CharacterData>, parent: Entity) -> Entity {
    let weights = bevy::mesh::morph::MorphWeights::new(vec![0.0; data.morph_targets], None).unwrap_or_default();
    let root = commands.spawn((Transform::default(), Visibility::default(), Motion::default(), weights)).id();
    commands.entity(parent).add_child(root);
    let joints: Vec<Entity> = data.joints.iter().map(|j| commands.spawn((j.rest, Visibility::default())).id()).collect();
    for (i, j) in data.joints.iter().enumerate() {
        let up = j.parent.map_or(root, |p| joints[p]);
        commands.entity(up).add_child(joints[i]);
    }
    let skin = SkinnedMesh { inverse_bindposes: data.binds.clone(), joints: joints.clone() };
    // a skinned part's bounds are its bind pose's: posed, a small part (the eyes, on the `eye` joint alone) leaves
    // them and was culled in close-ups (the sockets showed through, black at a distance). The GS draws every packet.
    // ponytail: no culling per part; posed bounds from the joints if the draw count ever matters
    for (mesh, material, morphed) in &data.parts {
        let part = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), skin.clone(), Transform::default(), NoFrustumCulling)).id();
        if *morphed {
            commands.entity(part).insert(bevy::mesh::morph::MeshMorphWeights::Reference(root));
        }
        commands.entity(root).add_child(part);
    }
    if let Some(r) = data.joint("Racket") {
        for (mesh, material) in &data.racket {
            let part = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), Transform::default(), NoFrustumCulling)).id();
            commands.entity(joints[r]).add_child(part);
        }
    }
    commands.entity(root).insert(Rig { data: data.clone(), joints });
    root
}

/// Sample every character's motion into its joints (unkeyed joints at rest, as the game binds a motion).
pub fn animate(time: Res<Time<Fixed>>, mut rigs: Query<(&Rig, &Motion, &mut bevy::mesh::morph::MorphWeights)>, mut joints: Query<&mut Transform>) {
    for (rig, motion, mut weights) in &mut rigs {
        let fading = rig.data.motions.get(&(motion.fade.id as usize)).filter(|_| motion.fade.clip);
        // the face: each bound track's weight at the face clock's last sampled time (unbound targets 0)
        // ponytail: the key cursor restarts from 0 each sample; the game walks from the last key, ≤1 ulp apart after a wrap
        let w = weights.weights_mut();
        w.fill(0.0);
        if let Some(face) = rig.data.faces.get(&motion.face) {
            let t = hst_sim::ps2::mul(motion.face_clock.sampled, 80.0);
            for (target, ticks, values) in &face.tracks {
                w[*target] = hst_sim::face::sample(ticks, values, t, &mut 0, false)[0];
            }
        }
        let Some(new) = rig.data.motions.get(&motion.id) else { continue };
        // between the last two ticks' sampled times (across a loop's wrap)
        let (a, b) = (motion.prev, motion.clock.sampled);
        let b = if b < a && motion.clock.looping { b + new.length } else { b };
        let t = new.wrap(a + (b - a) * time.overstep_fraction(), motion.clock.looping);
        // the base pose, and the pose mixed over its first 23 tracks (the player's blend count) by the fade
        let (clip, t, over) = match (motion.mix, fading) {
            (Some((w, true)), Some(old)) => (old, motion.fade.sampled, Some((new, 0.0, w))),
            (Some((w, false)), Some(old)) => (new, t, Some((old, motion.fade.sampled, w))),
            _ => (new, t, None),
        };
        for (j, joint) in rig.data.joints.iter().enumerate() {
            if let Ok(mut tf) = joints.get_mut(rig.joints[j]) {
                *tf = joint.rest;
            }
        }
        for (k, track) in clip.tracks.iter().enumerate() {
            let Ok(mut tf) = joints.get_mut(rig.joints[track.node]) else { continue };
            let (rot, pos) = clip.sample(k, t);
            // a rotation key is the conjugate of the joint's local rotation
            if let Some([x, y, z, w]) = rot {
                tf.rotation = Quat::from_xyzw(-x, -y, -z, w).normalize();
            }
            if let Some(p) = pos {
                tf.translation = Vec3::from(p);
            }
        }
        // ponytail: an unkeyed base track mixes from the rest pose (the game's from whatever its node last held)
        if let Some((other, t, w)) = over {
            for (k, track) in other.tracks.iter().enumerate().take(23) {
                let Ok(mut tf) = joints.get_mut(rig.joints[track.node]) else { continue };
                let (rot, pos) = other.sample(k, t);
                let r = tf.rotation;
                let (q, p) = hst_sim::motion::mix(([-r.x, -r.y, -r.z, r.w], tf.translation.into()), (rot.unwrap_or([-r.x, -r.y, -r.z, r.w]), pos.unwrap_or(tf.translation.into())), w);
                if rot.is_some() {
                    tf.rotation = Quat::from_xyzw(-q[0], -q[1], -q[2], q[3]).normalize();
                }
                if pos.is_some() {
                    tf.translation = Vec3::from(p);
                }
            }
        }
        // the contact IK turns the arm joints in their parents' frames (the game's local·Q, rows; translation kept)
        if let Some((quats, w)) = motion.arm {
            for (name, q) in ["Bip01RHand", "Bip01RForearm", "Bip01RUpperArm", "Bip01LUpperArm"].iter().zip(quats) {
                let Some(j) = rig.data.joint(name) else { continue };
                let Ok(mut tf) = joints.get_mut(rig.joints[j]) else { continue };
                let [x, y, z, w] = hst_sim::quat::slerp([0.0, 0.0, 0.0, 1.0], q, w);
                tf.rotation = (Quat::from_xyzw(-x, -y, -z, w).normalize() * tf.rotation).normalize();
            }
        }
    }
}

/// Advance every motion by one game frame, as the game's motion player.
pub fn tick(mut q: Query<(&Rig, &mut Motion)>) {
    for (rig, mut m) in &mut q {
        let length = rig.data.motions.get(&m.id).map_or(0.0, |c| c.length);
        let old = rig.data.motions.get(&(m.fade.id as usize)).map_or(0.0, |c| c.length);
        m.prev = m.clock.sampled;
        m.mix = m.fade.tick(old).map(|w| (w, false));
        m.clock.tick(length);
        if let Some(w) = m.fade.tick_hold() {
            m.mix = Some((w, true));
        }
        let face = rig.data.faces.get(&m.face).map_or(0.0, |f| f.length);
        m.face_clock.tick(face);
    }
}

/// Character viewer (`--character N --motion M`, or `--mod DIR` for a mod's first costume): the character alone at
/// the origin, playing motion M in a loop.
pub fn viewer(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0)).add_systems(PostStartup, spawn_viewer).add_systems(FixedUpdate, tick).add_systems(Update, animate);
}

/// The viewer's mod folder (`--mod`).
#[derive(Resource)]
pub struct ViewerMod(pub Option<String>);

#[allow(clippy::too_many_arguments)]
fn spawn_viewer(
    mut commands: Commands,
    args: Res<crate::Args>,
    view_mod: Res<ViewerMod>,
    root: Query<Entity, With<crate::GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let (Some((n, motion)), Ok(root)) = (args.viewer, root.single()) else { return };
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = match &view_mod.0 {
        Some(dir) => crate::mods::read(dir.as_ref()).and_then(|m| crate::mods::load(&mut iso, &m, 0, &mut meshes, &mut materials, &mut images, &mut bindposes)),
        None => load_disc(&mut iso, n, args.outfits.first().copied().unwrap_or(0), &mut meshes, &mut materials, &mut images, &mut bindposes),
    };
    let data = Arc::new(data.unwrap_or_else(|e| panic!("character: {e}")));
    info!("character {n}: {} joints, {} parts, motions {:?}", data.joints.len(), data.parts.len(), { let mut k: Vec<_> = data.motions.keys().collect(); k.sort(); k });
    let c = spawn(&mut commands, &data, root);
    // `set`, so the motion's face plays too
    let mut m = Motion::default();
    m.set(motion, 1.0, true, None, 0);
    commands.entity(c).insert(m);
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The strongest channel strictly above 0.5 wins, the first on a tie; none above 0.5 is neutral.
    #[test]
    fn texture_face_picks_the_strongest_channel() {
        let w = [0.9, 0.5, 0.95, 0.95, 0.2];
        assert_eq!(strongest([0, 1, 2, 3].into_iter(), &w), Some(2));
        assert_eq!(strongest([1, 4].into_iter(), &w), None);
        assert_eq!(strongest([4, 0, 9].into_iter(), &w), Some(1));
    }

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// Every character's inspect pose binds to its skeleton, and the held frames are the ones the game's inspect
    /// screen held (read live in its motion player for characters 0..7, 12 and 13).
    #[test]
    fn inspect_poses_bind_and_hold_the_games_frames() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        let mut held = Vec::new();
        for n in 0..14 {
            let data = iso.read(&format!("PC/PC{n:02}C00.XB")).unwrap();
            let arc = Archive::parse(&data).unwrap();
            let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("_c00.mdl")).unwrap();
            let model = mdl::parse(&arc.read(e).unwrap()).unwrap();
            let skeleton = Skeleton { names: model.node_names.clone(), parent: model.node_parent.clone(), rest: model.node_local.clone() };
            let m = disc_motions(&mut iso, n, &skeleton, |name| model.morph_names.iter().position(|m| m == name)).unwrap();
            let pose = &m.motions[&POSE];
            assert!(pose.tracks.len() > 40, "character {n}: {} pose tracks", pose.tracks.len());
            assert!(m.faces.get(&POSE).is_some_and(|f| !f.tracks.is_empty()), "character {n}: no pose face");
            held.push(pose_frame(n, pose.length));
        }
        assert_eq!(held[..8], [69.0, 61.0, 0.0, 62.0, 0.0, 66.0, 0.0, 57.0]);
        assert_eq!((held[12], held[13]), (0.0, 64.0));
    }
}
