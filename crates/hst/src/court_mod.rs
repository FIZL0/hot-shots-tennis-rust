//! M8f6: a court mod (`mods_court`) seen and heard in place of its layout court. `load` sets the layout court up as
//! ever (collision, judging, weather, the court generators' draws, the people) but spawns its models under a hidden
//! [`DiscCourt`]; here the mod's `court.glb`, `sky.glb` (standing at the eye like the disc skies) and `gimmick.glb`
//! (each animation looping on its own clock) are drawn in the layout court's game space, its manifest light is baked
//! into their vertex colours, its sun casts the shadows, and its BGM and ambience loops play in place of the layout
//! court's music and creature sounds. Nothing here reads or writes the match: replays stay bit-exact.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageAddressMode, ImageSampler, ImageSamplerDescriptor, ImageType};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use hst_data::mtl;

use crate::audio::{CourtBank, Group, MusicBanks, Sound, SoundBank, Track};
use crate::gs::{GsMaterial, NO_FOG};
use crate::mods_court::{Court, Light, MatchCourt};

/// The layout court's own models, kept but not drawn.
#[derive(Component)]
pub struct DiscCourt;

/// The court mod's BGM bank while its match runs, for the pause menu's music toggle (`bgm`).
static BGM: Mutex<Option<Arc<SoundBank>>> = Mutex::new(None);

/// The match's court mod has no lens flare (`play::flare` draws none).
pub static FLARE_OFF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The PSP's lighting saturates at full colour (vertex colour 1.0 = ×1 here, as the GS's 0x80).
const MAX_LIGHT: f32 = 1.0;

/// The match's court mod, if any: its systems (the menu's and the command line's matches).
pub fn add(app: &mut App, court: Option<Court>) {
    *BGM.lock().unwrap() = None;
    FLARE_OFF.store(false, std::sync::atomic::Ordering::Relaxed);
    let Some(court) = court else { return };
    app.insert_resource(MatchCourt(court))
        .init_resource::<Kept>()
        .add_systems(PostStartup, setup)
        .add_systems(PreUpdate, sounds)
        .add_systems(Update, (keep.after(crate::weather::apply), animate));
}

/// The pause menu turned the match music back on: the court mod's BGM in place of the layout court's.
pub fn bgm(bgm: &mut Option<(Arc<SoundBank>, Vec<u8>)>) {
    if let Some(b) = BGM.lock().unwrap().clone() {
        *bgm = Some((b, Vec::new()));
    }
}

/// How a court mod material is lit: `Sky` unlit and unfogged (as the disc skies); `Baked` with the manifest's light
/// in its vertex colours and its fog; `Weather` lit and fogged as the layout court is (no manifest light).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Role {
    Sky,
    Baked,
    Weather,
}

/// The court mod's materials and how each is lit, re-applied over each weather change (`keep`).
#[derive(Resource, Default)]
struct Kept {
    materials: Vec<(AssetId<GsMaterial>, Role)>,
    fog: Option<(Vec4, Vec4)>,
}

/// A glTF animation looping on its own clock: per channel the node's entity, its times and its values.
#[derive(Component)]
struct Looped {
    length: f32,
    channels: Vec<Channel>,
}

struct Channel {
    node: Entity,
    times: Vec<f32>,
    values: Values,
    step: bool,
}

enum Values {
    Translation(Vec<Vec3>),
    Rotation(Vec<Quat>),
    Scale(Vec<Vec3>),
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    court: Res<MatchCourt>,
    args: Res<crate::Args>,
    root: Query<Entity, With<crate::GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GsMaterial>>,
    mut binds: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut kept: ResMut<Kept>,
    sun: Option<Res<crate::shadow::Sun>>,
) {
    let Ok(root) = root.single() else { return };
    let c = &court.0;
    let mut store = Store { meshes: &mut meshes, images: &mut images, materials: &mut materials, binds: &mut binds, kept: &mut kept };
    for (file, sky) in [("court.glb", false), ("gimmick.glb", false), ("sky.glb", true)] {
        let path = c.dir.join(file);
        if !crate::modfs::is_file(&path) {
            continue;
        }
        let parent = if sky {
            // stands at the eye like the disc skies (`backdrops`)
            commands.spawn((crate::Backdrop { yaw: 0.0, lift: 0.0 }, Transform::default(), Visibility::default(), ChildOf(root))).id()
        } else {
            root
        };
        match crate::mods::open(&path).and_then(|(doc, blob)| scene(&mut commands, &mut store, &doc, &blob, parent, args.singles, if sky { None } else { c.light.as_ref() }, sky)) {
            Ok(n) => info!("court mod {}: {file}, {n} draws", c.id),
            Err(e) => warn!("court mod {}: {}: {e}", c.id, path.display()),
        }
    }
    kept.fog = c.light.as_ref().map(|l| l.fog.as_ref().map_or((NO_FOG, Vec4::ONE), |f| (Vec4::new(255.0 * (1.0 - f.amount[0]), 255.0 * (1.0 - f.amount[1]), f.near, f.far), f.colour.extend(1.0))));
    // the layout court's ball and figure shading comes from its own (unseen) casters: none here
    commands.remove_resource::<crate::shade::Shade>();
    // the shadows fall from the manifest's sun (raised to at least 50°, as the game's)
    if let (Some(l), Some(sun)) = (&c.light, sun) {
        let light = -l.sun;
        let flat = Vec2::new(light.x, light.z);
        let lowest = 50f32.to_radians();
        let dir = if light.y >= lowest.sin() || flat.length() < 1e-6 { light } else { (flat.normalize() * lowest.cos()).extend(lowest.sin()).xzy() };
        commands.insert_resource(crate::shadow::Sun { dir, light, axes: hst_sim::shade::sun_axes([dir.x, dir.y, dir.z, 0.0]), ..*sun });
    }
    // the disc's flare stands for the mod's sprite at the manifest's sun; a court mod without one has none
    FLARE_OFF.store(c.lens_flare.is_none(), std::sync::atomic::Ordering::Relaxed);
}

struct Store<'a> {
    meshes: &'a mut Assets<Mesh>,
    images: &'a mut Assets<Image>,
    materials: &'a mut Assets<GsMaterial>,
    binds: &'a mut Assets<SkinnedMeshInverseBindposes>,
    kept: &'a mut Kept,
}

/// Spawns a court mod `.glb`'s scene under `parent` in the layout court's game space (the scene's `game_space` node
/// undone), leaving out the other match kind's `__singles`/`__doubles` nodes; its animations loop. Returns the draws.
#[allow(clippy::too_many_arguments)]
fn scene(commands: &mut Commands, s: &mut Store, doc: &gltf::Document, blob: &[u8], parent: Entity, singles: bool, light: Option<&Light>, sky: bool) -> Result<usize, String> {
    let worlds = worlds(doc);
    let space = doc.nodes().find(|n| n.name() == Some("game_space")).map_or(Mat4::IDENTITY, |n| worlds[n.index()]);
    let to_game = space.inverse();
    let role = if sky { Role::Sky } else if light.is_some() { Role::Baked } else { Role::Weather };
    let mats = materials(doc, blob, s, role)?;
    let anchor = commands.spawn((Transform::from_matrix(to_game), Visibility::default(), ChildOf(parent))).id();
    let skip = |n: &gltf::Node| n.name().is_some_and(|m| m.ends_with(if singles { "__doubles" } else { "__singles" }));
    // the node tree as entities (an animation or a skin moves them)
    let mut nodes = HashMap::new();
    let mut stack: Vec<(gltf::Node, Entity)> = doc.default_scene().or_else(|| doc.scenes().next()).ok_or("no scene")?.nodes().map(|n| (n, anchor)).collect();
    while let Some((n, up)) = stack.pop() {
        if skip(&n) {
            continue;
        }
        let (t, r, sc) = n.transform().decomposed();
        let e = commands.spawn((Transform { translation: Vec3::from(t), rotation: Quat::from_array(r), scale: Vec3::from(sc) }, Visibility::default(), ChildOf(up))).id();
        nodes.insert(n.index(), e);
        stack.extend(n.children().map(|c| (c, e)));
    }
    let mut draws = 0;
    for n in doc.nodes().filter(|n| nodes.contains_key(&n.index())) {
        let Some(mesh) = n.mesh() else { continue };
        let skin = n.skin().filter(|sk| sk.joints().all(|j| nodes.contains_key(&j.index())));
        // the skin's joints in game space at bind (to bake the light), and Bevy's skin
        let (rest, skinned) = match &skin {
            Some(sk) => {
                let ibm: Vec<Mat4> = sk.reader(|_| Some(blob)).read_inverse_bind_matrices().map_or_else(|| vec![Mat4::IDENTITY; sk.joints().count()], |i| i.map(|m| Mat4::from_cols_array_2d(&m)).collect());
                let rest: Vec<Mat4> = sk.joints().zip(&ibm).map(|(j, b)| to_game * worlds[j.index()] * *b).collect();
                let joints = sk.joints().map(|j| nodes[&j.index()]).collect();
                (rest, Some(SkinnedMesh { inverse_bindposes: s.binds.add(SkinnedMeshInverseBindposes::from(ibm)), joints }))
            }
            None => (vec![to_game * worlds[n.index()]], None),
        };
        for p in mesh.primitives() {
            let Some(m) = primitive(&p, blob, light, &rest, skinned.is_some()) else { continue };
            let m = s.meshes.add(m);
            for mat in &mats[p.material().index().unwrap_or(mats.len() - 1)] {
                let mut e = commands.spawn((Mesh3d(m.clone()), MeshMaterial3d(mat.clone()), Transform::default(), ChildOf(nodes[&n.index()])));
                if let Some(sk) = &skinned {
                    e.insert((sk.clone(), crate::cull::Cullable));
                }
                draws += 1;
            }
        }
    }
    for a in doc.animations() {
        let mut channels = Vec::new();
        for ch in a.channels() {
            let Some(&node) = nodes.get(&ch.target().node().index()) else { continue };
            let r = ch.reader(|_| Some(blob));
            let Some(times) = r.read_inputs().map(|i| i.collect::<Vec<f32>>()) else { continue };
            let interp = ch.sampler().interpolation();
            // cubic spline keys are (in-tangent, value, out-tangent): the values, linearly between
            let pick = |k: usize| interp != gltf::animation::Interpolation::CubicSpline || k % 3 == 1;
            let values = match r.read_outputs() {
                Some(gltf::animation::util::ReadOutputs::Translations(v)) => Values::Translation(v.enumerate().filter(|(k, _)| pick(*k)).map(|(_, v)| Vec3::from(v)).collect()),
                Some(gltf::animation::util::ReadOutputs::Rotations(v)) => Values::Rotation(v.into_f32().enumerate().filter(|(k, _)| pick(*k)).map(|(_, q)| Quat::from_array(q).normalize()).collect()),
                Some(gltf::animation::util::ReadOutputs::Scales(v)) => Values::Scale(v.enumerate().filter(|(k, _)| pick(*k)).map(|(_, v)| Vec3::from(v)).collect()),
                _ => continue,
            };
            let len = match &values {
                Values::Translation(v) | Values::Scale(v) => v.len(),
                Values::Rotation(v) => v.len(),
            };
            if len != times.len() || times.is_empty() {
                continue;
            }
            channels.push(Channel { node, times, values, step: interp == gltf::animation::Interpolation::Step });
        }
        let length = channels.iter().filter_map(|c| c.times.last().copied()).fold(0.0, f32::max);
        if !channels.is_empty() && length > 0.0 {
            commands.spawn(Looped { length, channels });
        }
    }
    Ok(draws)
}

/// Each node's world matrix in the file.
fn worlds(doc: &gltf::Document) -> Vec<Mat4> {
    let mut parent = vec![None; doc.nodes().len()];
    for n in doc.nodes() {
        n.children().for_each(|c| parent[c.index()] = Some(n.index()));
    }
    let local: Vec<Mat4> = doc.nodes().map(|n| Mat4::from_cols_array_2d(&n.transform().matrix())).collect();
    let mut w: Vec<Option<Mat4>> = vec![None; local.len()];
    for i in 0..local.len() {
        // the chain up to a node already known (the tree is checked at most 1024 deep, `mods::open`)
        let mut chain = vec![i];
        while let Some(p) = parent[*chain.last().unwrap()].filter(|p| w[*p].is_none() && chain.len() <= 1024) {
            chain.push(p);
        }
        for &k in chain.iter().rev() {
            if w[k].is_none() {
                w[k] = Some(parent[k].and_then(|p| w[p]).unwrap_or(Mat4::IDENTITY) * local[k]);
            }
        }
    }
    w.into_iter().map(|m| m.unwrap_or(Mat4::IDENTITY)).collect()
}

/// The `.glb`'s materials as GS draws (as a character mod's, `mods::materials_of`; the last one glTF's default), lit
/// as `role` says.
fn materials(doc: &gltf::Document, blob: &[u8], s: &mut Store, role: Role) -> Result<Vec<Vec<Handle<GsMaterial>>>, String> {
    let sampler = || ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: ImageAddressMode::Repeat, address_mode_v: ImageAddressMode::Repeat, ..ImageSamplerDescriptor::linear() });
    let mut tex = Vec::new();
    for im in doc.images() {
        let gltf::image::Source::View { view, mime_type } = im.source() else { return Err("images must be embedded".into()) };
        let bytes = blob.get(view.offset()..view.offset() + view.length()).ok_or("image outside the buffer")?;
        let mut img = Image::from_buffer(bytes, ImageType::MimeType(mime_type), CompressedImageFormats::NONE, true, sampler(), RenderAssetUsages::RENDER_WORLD).map_err(|e| format!("image {}: {e}", im.index()))?;
        // the GS works on the stored values
        img.texture_descriptor.format = bevy::render::render_resource::TextureFormat::Rgba8Unorm;
        tex.push(s.images.add(img));
    }
    let mut out = Vec::new();
    for m in doc.materials().map(Some).chain([None]) {
        let t = m.as_ref().and_then(|m| m.pbr_metallic_roughness().base_color_texture()).map(|i| i.texture().source().index());
        let color = m.as_ref().map_or([1.0; 4], |m| m.pbr_metallic_roughness().base_color_factor());
        let alpha = m.as_ref().map_or(gltf::material::AlphaMode::Opaque, |m| m.alpha_mode());
        let (mask, blend) = (alpha == gltf::material::AlphaMode::Mask, alpha == gltf::material::AlphaMode::Blend);
        let mut header = [0; 0x30];
        header[0x1e..0x20].copy_from_slice(&(if mask { 10i16 } else if blend { 25 } else { 0 }).to_le_bytes());
        let two_sided = m.as_ref().is_none_or(|m| m.double_sided());
        let mtl = mtl::Material { texture: t, color: color.map(|c| c.min(1.0)), attributes: None, two_sided, name: m.as_ref().and_then(|m| m.name()).unwrap_or("").to_string(), header };
        let prim = 0x20 | if t.is_some() { 0x10 } else { 0 } | if blend { 0x40 } else { 0 };
        let mut handles = Vec::new();
        for mut g in GsMaterial::for_batch(&mtl, prim, t.and_then(|t| tex.get(t)).cloned()) {
            style(&mut g, role, None, 0.0);
            if role == Role::Sky {
                g.order = crate::BACKDROP;
            }
            let h = s.materials.add(g);
            s.kept.materials.push((h.id(), role));
            handles.push(h);
        }
        out.push(handles);
    }
    Ok(out)
}

/// A court mod material's light, fog and shadow for its role (`fog`: the manifest's, `None` the weather's).
fn style(m: &mut GsMaterial, role: Role, fog: Option<(Vec4, Vec4)>, darken: f32) {
    let u = &mut m.uniform;
    if role != Role::Weather {
        // colour = vertex colour × material colour (the light baked in, or none on the sky)
        (u.unlit, u.ambient, u.light_color, u.light2_color, u.glare, u.highlight) = (1.0, Vec4::ONE, Vec4::ZERO, Vec4::ZERO, 0.0, 0.0);
    }
    if role == Role::Sky {
        (u.fog, u.shadow) = (NO_FOG, 0.0);
        return;
    }
    if let Some((f, c)) = fog.filter(|_| role == Role::Baked) {
        (u.fog, u.fog_color) = (f, c);
    }
    u.shadow = darken;
}

/// One triangle-list primitive as a GS mesh: the manifest light baked into its vertex colours (normals turned to
/// game space by `rest`: the node's matrix, or the skin's joints at bind), joints and weights for a skin.
fn primitive(p: &gltf::Primitive, blob: &[u8], light: Option<&Light>, rest: &[Mat4], skinned: bool) -> Option<Mesh> {
    if p.mode() != gltf::mesh::Mode::Triangles {
        return None;
    }
    let r = p.reader(|_| Some(blob));
    let pos: Vec<[f32; 3]> = r.read_positions()?.collect();
    let n = pos.len();
    let nrm: Vec<[f32; 3]> = r.read_normals().map_or_else(|| vec![[0.0, -1.0, 0.0]; n], |i| i.collect());
    let uv: Vec<[f32; 2]> = r.read_tex_coords(0).map_or_else(|| vec![[0.0; 2]; n], |i| i.into_f32().collect());
    let mut col: Vec<[f32; 4]> = r.read_colors(0).map_or_else(|| vec![[1.0; 4]; n], |i| i.into_rgba_f32().collect());
    let joints: Vec<[u16; 4]> = r.read_joints(0).map_or_else(|| vec![[0; 4]; n], |i| i.into_u16().collect());
    let weights: Vec<[f32; 4]> = r.read_weights(0).map_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; n], |i| i.into_f32().collect());
    let idx: Vec<u32> = r.read_indices().map_or_else(|| (0..n as u32).collect(), |i| i.into_u32().collect());
    if [nrm.len(), uv.len(), col.len(), joints.len(), weights.len()].iter().any(|&l| l != n) || idx.iter().any(|&i| i as usize >= n) {
        return None;
    }
    if let Some(l) = light {
        for (k, c) in col.iter_mut().enumerate() {
            let m = if skinned { (0..4).map(|j| *rest.get(joints[k][j] as usize).unwrap_or(&Mat4::IDENTITY) * weights[k][j]).fold(Mat4::ZERO, |a, b| a + b) } else { rest[0] };
            let w = m.inverse().transpose().transform_vector3(Vec3::from(nrm[k])).normalize_or_zero();
            let lit = (l.ambient + l.sun_colour * w.dot(l.sun).max(0.0) + l.fill_colour * w.dot(l.fill).max(0.0)).min(Vec3::splat(MAX_LIGHT));
            let rgb = Vec3::new(c[0], c[1], c[2]) * lit;
            *c = [rgb.x, rgb.y, rgb.z, c[3]];
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, crate::cull::usage(RenderAssetUsages::RENDER_WORLD))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx));
    if skinned {
        // gs.wgsl's skinned normal: the first joint's share in the normal slot, the rest in the tangent slot
        mesh = mesh
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm.iter().zip(&weights).map(|(n, w)| (Vec3::from(*n) * w[0]).to_array()).collect::<Vec<_>>())
            .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, nrm.iter().zip(&weights).map(|(n, w)| (Vec3::from(*n) * (1.0 - w[0])).extend(0.0).to_array()).collect::<Vec<_>>())
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(joints))
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
    } else {
        mesh = mesh.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm);
    }
    Some(mesh)
}

/// After each weather change (`weather::apply` relights every material): the court mod's own light and fog back,
/// and the ground shadow's strength the layout court's ground has now.
fn keep(look: Option<Res<crate::weather::CourtLook>>, kept: Res<Kept>, mut materials: ResMut<Assets<GsMaterial>>, mut last: Local<Option<(Option<u8>, Option<(u8, f32)>)>>) {
    let Some(look) = look else { return };
    if *last == Some((look.last, look.mix)) {
        return;
    }
    *last = Some((look.last, look.mix));
    let darken = look.holes.iter().find_map(|id| materials.get(*id)).map_or(0.0, |m| m.uniform.shadow);
    for &(id, role) in &kept.materials {
        if let Some(mut m) = materials.get_mut(id) {
            style(&mut m, role, kept.fog, darken);
        }
    }
}

/// Each gimmick animation on its own loop.
fn animate(time: Res<Time>, clips: Query<&Looped>, mut nodes: Query<&mut Transform>) {
    let now = time.elapsed_secs();
    for clip in &clips {
        let t = now % clip.length;
        for ch in &clip.channels {
            let Ok(mut tf) = nodes.get_mut(ch.node) else { continue };
            let k = ch.times.partition_point(|&x| x <= t).saturating_sub(1);
            let next = (k + 1).min(ch.times.len() - 1);
            let f = if ch.step || next == k { 0.0 } else { ((t - ch.times[k]) / (ch.times[next] - ch.times[k])).clamp(0.0, 1.0) };
            match &ch.values {
                Values::Translation(v) => tf.translation = v[k].lerp(v[next], f),
                Values::Rotation(v) => tf.rotation = v[k].slerp(v[next], f),
                Values::Scale(v) => tf.scale = v[k].lerp(v[next], f),
            }
        }
    }
}

/// Once the match's banks are loaded: the mod's BGM in place of the layout court's (with `--music`), and its
/// ambience layers looping in place of the layout court's creature and emitter sounds (court bank program 7, kept
/// silent: its plays still come and go on the same ticks).
fn sounds(court: Res<MatchCourt>, args: Res<crate::Args>, music: Option<ResMut<MusicBanks>>, bank: Option<ResMut<CourtBank>>, sound: Option<Res<Sound>>, mut done: Local<bool>) {
    let (false, Some(mut music), Some(mut bank), Some(sound)) = (*done, music, bank, sound) else { return };
    *done = true;
    let c = &court.0;
    let read = |l: &crate::mods_court::Loop| crate::modfs::read(&l.path).ok().and_then(|d| Track::read(&d, l.at, Track::BGM_RMS)).map(Arc::new);
    if let Some(track) = c.bgm.as_ref().and_then(read) {
        let b = Arc::new(SoundBank { hd: Vec::new(), bd: Vec::new(), wav: Default::default(), takes: Default::default(), stream: Some(track) });
        *BGM.lock().unwrap() = Some(b.clone());
        if args.music {
            music.bgm = Some((b, Vec::new()));
        }
    }
    let layers: Vec<_> = c.ambience.iter().filter_map(read).collect();
    if layers.is_empty() {
        return;
    }
    if let Some(b) = &bank.0 {
        let mut wav = b.wav.clone();
        wav.insert(7, vec![(Arc::from([0i16]), 48_000)]);
        bank.0 = Some(Arc::new(SoundBank { hd: b.hd.clone(), bd: b.bd.clone(), wav, takes: b.takes.clone(), stream: None }));
    }
    // several layers at once: each a little under the BGM's loudness
    let level = 0.7 / (layers.len() as f32).sqrt();
    for t in &layers {
        sound.play_loop(t, Group::Sfx, level);
    }
}
