//! Characters: a skeleton, skinned meshes and motions, independent of where they come from. [`load_disc`] builds
//! one from the game's files on the user's disc (`PC/PCnnCcc.XB` model and racket, `PCANI/PCnnANI.XB` motions);
//! a custom character only has to fill the same [`CharacterData`]: joints named like the 3ds Max Biped skeleton
//! the game uses (`Bip01Pelvis`, `Bip01RHand`, …, plus `Racket` where the racket sits) and clips keyed by the
//! game's motion numbers ([`hst_data::ani::MOTIONS`]). Built in game space (Y down, feet at the origin).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use hst_data::{ani, iso::Iso, mdl, mtl, xb::Archive};

/// One joint of the skeleton.
pub struct Joint {
    pub name: String,
    pub parent: Option<usize>,
    /// Local placement at rest.
    pub rest: Transform,
    /// Model space → joint space in the bind pose.
    pub inverse_bind: Mat4,
}

/// A motion: per joint, rotation and translation keys at game frames (60 Hz).
pub struct Clip {
    pub end: f32,
    pub tracks: Vec<(usize, Vec<(f32, Quat)>, Vec<(f32, Vec3)>)>,
}

pub struct CharacterData {
    pub joints: Vec<Joint>,
    /// Skinned body parts.
    pub parts: Vec<(Handle<Mesh>, Handle<StandardMaterial>)>,
    /// Rigid parts carried by the `Racket` joint.
    pub racket: Vec<(Handle<Mesh>, Handle<StandardMaterial>)>,
    /// Motions by the game's motion number.
    pub motions: HashMap<usize, Clip>,
    /// Every joint's inverse bind matrix, for the skinned parts.
    pub binds: Handle<SkinnedMeshInverseBindposes>,
    /// Per motion number (0..48): the forward row (x, z) of `Bip01Pelvis`'s model matrix at the motion's first
    /// frame — what the game's body turn compares (`hst_sim::player::turn`).
    pub pelvis: Vec<[f32; 2]>,
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

/// The motion a character plays: the game's motion number, time in game frames and its speed.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub id: usize,
    pub time: f32,
    pub speed: f32,
    pub looping: bool,
}

impl Default for Motion {
    fn default() -> Self {
        Motion { id: 0, time: 0.0, speed: 1.0, looping: true }
    }
}

impl Motion {
    /// Switch to motion `id` from its start (keeps going if it already plays).
    pub fn play(&mut self, id: usize, speed: f32, looping: bool) {
        if self.id != id {
            *self = Motion { id, time: 0.0, speed, looping };
        } else {
            self.speed = speed;
            self.looping = looping;
        }
    }
}

/// Row-vector game matrix → Bevy (column-vector) matrix: the rows become the columns.
fn mat(m: &[[f32; 4]; 4]) -> Mat4 {
    Mat4::from_cols_array_2d(m)
}

fn texture_image(t: &mtl::Texture) -> Image {
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
    let tex: Vec<Handle<Image>> = mats.textures.iter().map(|t| images.add(texture_image(t))).collect();
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

    let joints: Vec<Joint> = (0..model.node_count)
        .map(|i| Joint {
            name: model.node_names[i].clone(),
            parent: model.node_parent[i],
            rest: Transform::from_matrix(mat(&model.node_local[i])),
            inverse_bind: mat(&model.node_bind[i]).inverse(),
        })
        .collect();

    // skinned parts, one mesh per material
    let mut by_material: HashMap<usize, (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<[f32; 4]>, Vec<[u16; 4]>, Vec<[f32; 4]>, Vec<u32>)> = HashMap::new();
    for (material, verts, tris) in model.skinned() {
        let e = by_material.entry(material).or_default();
        let base = e.0.len() as u32;
        for v in &verts {
            e.0.push(v.pos);
            e.1.push(v.normal);
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
        let (pos, nrm, uv, col, joint, weight, idx) = by_material.remove(&m).unwrap();
        if idx.is_empty() {
            continue;
        }
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, bevy::mesh::VertexAttributeValues::Uint16x4(joint))
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weight)
            .with_inserted_indices(Indices::U32(idx));
        parts.push((meshes.add(mesh), handles.get(m).cloned().unwrap_or_default()));
    }

    // the racket: a rigid model in its own space, carried by the `Racket` joint
    let mut racket = Vec::new();
    let rk = format!("racket_{n:02}");
    if let (Some(rm), Some(rt)) = (find(&format!("{rk}.mdl")), find(&format!("{rk}.mtl"))) {
        let rmodel = mdl::parse(&rm).map_err(|e| e.0)?;
        let rmats = mtl::parse(&rt, find(&format!("{rk}.mti")).as_deref()).map_err(|e| e.0)?;
        let rh = materials_of(&rmats, images, materials);
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

    // motions by the game's numbers
    let anims = iso.read(&format!("PCANI/PC{n:02}ANI.XB")).map_err(|e| e.to_string())?;
    let aarc = Archive::parse(&anims).map_err(|e| e.0)?;
    let mut motions = HashMap::new();
    let skeleton = hst_sim::pose::Skeleton { names: model.node_names.clone(), parent: model.node_parent.clone(), rest: model.node_local.clone() };
    let hip = skeleton.names.iter().position(|n| n == "Bip01Pelvis");
    let mut pelvis = vec![[0.0, 1.0]; 48];
    for id in 0..ani::MOTIONS.len() {
        let stem = ani::motion_name(id, n).unwrap().to_ascii_lowercase();
        let Some(e) = aarc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))) else { continue };
        let Ok(a) = ani::parse(&aarc.read(e).map_err(|e| e.0)?) else { continue };
        if let (Some(h), Some(row)) = (hip, pelvis.get_mut(id)) {
            let r = hst_sim::pose::first_frame(&skeleton, &a)[h][2];
            *row = [r[0], r[2]];
        }
        let per_frame = a.ticks_per_frame.max(1) as f32;
        let tracks = a
            .tracks
            .iter()
            .filter_map(|t| {
                let j = joints.iter().position(|j| j.name == t.name)?;
                // a rotation key is the conjugate of the joint's local rotation
                let rot = t.rotation.iter().map(|&(tick, q)| (tick as f32 / per_frame, Quat::from_xyzw(-q[0], -q[1], -q[2], q[3]).normalize())).collect();
                let pos = t.position.iter().map(|&(tick, p)| (tick as f32 / per_frame, Vec3::new(p[0], p[1], p[2]))).collect();
                Some((j, rot, pos))
            })
            .collect();
        motions.insert(id, Clip { end: a.end_tick() as f32 / per_frame, tracks });
    }
    let binds = bindposes.add(SkinnedMeshInverseBindposes::from(joints.iter().map(|j| j.inverse_bind).collect::<Vec<_>>()));
    Ok(CharacterData { joints, parts, racket, motions, binds, pelvis })
}

/// Spawn a character under `parent` (game space); returns its root (carry `Transform` and `Motion` on it).
pub fn spawn(commands: &mut Commands, data: &Arc<CharacterData>, parent: Entity) -> Entity {
    let root = commands.spawn((Transform::default(), Visibility::default(), Motion::default())).id();
    commands.entity(parent).add_child(root);
    let joints: Vec<Entity> = data.joints.iter().map(|j| commands.spawn((j.rest, Visibility::default())).id()).collect();
    for (i, j) in data.joints.iter().enumerate() {
        let up = j.parent.map_or(root, |p| joints[p]);
        commands.entity(up).add_child(joints[i]);
    }
    let skin = SkinnedMesh { inverse_bindposes: data.binds.clone(), joints: joints.clone() };
    for (mesh, material) in &data.parts {
        let part = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), skin.clone(), Transform::default())).id();
        commands.entity(root).add_child(part);
    }
    if let Some(r) = data.joint("Racket") {
        for (mesh, material) in &data.racket {
            let part = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), Transform::default())).id();
            commands.entity(joints[r]).add_child(part);
        }
    }
    commands.entity(root).insert(Rig { data: data.clone(), joints });
    root
}

/// Sample every character's motion into its joints.
pub fn animate(rigs: Query<(&Rig, &Motion)>, mut joints: Query<&mut Transform>) {
    for (rig, motion) in &rigs {
        let Some(clip) = rig.data.motions.get(&motion.id) else { continue };
        let t = if motion.looping && clip.end > 0.0 { motion.time.rem_euclid(clip.end) } else { motion.time.min(clip.end) };
        for (j, rot, pos) in &clip.tracks {
            let Ok(mut tf) = joints.get_mut(rig.joints[*j]) else { continue };
            if let Some(q) = sample(rot, t, |a, b, u| a.slerp(b, u)) {
                tf.rotation = q;
            }
            if let Some(p) = sample(pos, t, |a, b, u| a.lerp(b, u)) {
                tf.translation = p;
            }
        }
    }
}

/// Keyed value at frame `t`, interpolated between the keys around it (held past the ends).
fn sample<T: Copy>(keys: &[(f32, T)], t: f32, mix: impl Fn(T, T, f32) -> T) -> Option<T> {
    let first = keys.first()?;
    if t <= first.0 {
        return Some(first.1);
    }
    let i = keys.partition_point(|k| k.0 <= t);
    if i >= keys.len() {
        return Some(keys[keys.len() - 1].1);
    }
    let (a, b) = (keys[i - 1], keys[i]);
    Some(mix(a.1, b.1, (t - a.0) / (b.0 - a.0).max(1e-6)))
}

/// Advance every motion by one game frame.
pub fn tick(mut q: Query<&mut Motion>) {
    for mut m in &mut q {
        m.time += m.speed;
    }
}

/// Character viewer (`--character N --motion M`): the character alone at the origin, playing motion M in a loop.
pub fn viewer(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0)).add_systems(PostStartup, spawn_viewer).add_systems(FixedUpdate, tick).add_systems(Update, animate);
}

fn spawn_viewer(
    mut commands: Commands,
    args: Res<crate::Args>,
    root: Query<Entity, With<crate::GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let (Some((n, motion)), Ok(root)) = (args.viewer, root.single()) else { return };
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = Arc::new(load_disc(&mut iso, n, 0, &mut meshes, &mut materials, &mut images, &mut bindposes).expect("character"));
    info!("character {n}: {} joints, {} parts, motions {:?}", data.joints.len(), data.parts.len(), { let mut k: Vec<_> = data.motions.keys().collect(); k.sort(); k });
    let c = spawn(&mut commands, &data, root);
    commands.entity(c).insert(Motion { id: motion, ..default() });
}
