//! P61: the match's court from the disc that `main.rs`' loader and the match share (moved from `main.rs`): layout
//! records, models as Bevy meshes, the sky's cloud draws on each new point; the debug orbit camera.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use hst_data::{iso::Iso, layout, mdl, mtl, xb::Archive};

use crate::{Orbit, gs, shade, textures};

/// Court `n`'s entry list and hole-01 placement records.
pub fn court_layout(iso: &mut Iso, n: usize) -> Option<(Vec<layout::Entry>, Vec<layout::Placement>)> {
    let find = |data: &[u8], suffix: &str| {
        let arc = Archive::parse(data).ok()?;
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix))?;
        arc.read(e).ok()
    };
    let (cmn, hol) = (iso.read(&format!("COURT/{n:02}/CMN.XB")).ok()?, iso.read(&format!("COURT/{n:02}/HOL01.XB")).ok()?);
    let list = layout::entries(&String::from_utf8_lossy(&find(&cmn, &format!("entry_c{n:02}.txt"))?));
    Some((list, layout::plants(&find(&hol, &format!("plant_c{n:02}_h01_0.dat"))?).ok()?))
}

/// Every model in an archive whose lower-cased name ends in `.mdl` and passes `keep`, keyed by its
/// lower-case file stem, as one Bevy mesh + material per original material.
pub fn models(data: &[u8], keep: impl Fn(&str) -> bool, images: &mut Assets<Image>) -> Vec<(String, Vec<(Mesh, StandardMaterial)>)> {
    let mut out = Vec::new();
    for_models(data, keep, |model_stem, model, mats| {
        let _pre = textures::prefetch(&mats);
        let tex: Vec<Handle<Image>> = mats.textures.iter().map(|t| textures::add_mtl(images, image(t), t)).collect();
        let mut parts = Vec::new();
        {
            for (mi, packets) in model.materials.iter().enumerate() {
                let Some(mesh) = mesh(packets.iter(), &model) else { continue };
                let mat = mats.materials.get(mi);
                let texture = mat.and_then(|m| m.texture).map(|t| tex[t].clone());
                if let (Some(m), Some(t), Some(h)) = (mat, mat.and_then(|m| m.texture), &texture) {
                    let mut raw = image(&mats.textures[t]);
                    raw.texture_descriptor.format = TextureFormat::Rgba8Unorm;
                    let mut draws = gs::GsMaterial::for_batch(m, packets.first().map_or(0x10, |p| p.prim), Some(textures::add_mtl(images, raw, &mats.textures[t])));
                    draws.iter_mut().for_each(|g| g.uniform.lod_k = model.lod_k.get(mi).copied().unwrap_or(0.0));
                    shade::BALL_GS.lock().unwrap().insert(h.id(), draws);
                }
                let [r, g, b, a] = mat.map_or([1.0; 4], |m| m.color);
                parts.push((
                    mesh,
                    StandardMaterial {
                        base_color: Color::linear_rgba(r, g, b, a),
                        base_color_texture: texture,
                        unlit: true,
                        double_sided: true,
                        cull_mode: None,
                        alpha_mode: AlphaMode::Mask(0.5),
                        ..default()
                    },
                ));
            }
        }
        out.push((model_stem, parts));
    });
    out
}

/// Every model in an archive whose lower-cased name ends in `.mdl` and passes `keep`, with its lower-case file
/// stem and parsed MTL.
pub fn for_models(data: &[u8], keep: impl Fn(&str) -> bool, mut f: impl FnMut(String, mdl::Model, mtl::Mtl)) {
    let arc = Archive::parse(data).expect("xb archive");
    for e in arc.entries.iter().filter(|e| { let n = e.name.to_ascii_lowercase(); n.ends_with(".mdl") && keep(&n) }) {
        let stem = &e.name[..e.name.len() - 4];
        let sib = |ext: &str| arc.find(&format!("{stem}.{ext}")).and_then(|x| arc.read(x).ok());
        let (Ok(model), Some(mtl_bytes)) = (mdl::parse(&arc.read(e).unwrap()), sib("MTL")) else {
            warn!("skipping {}", e.name);
            continue;
        };
        let mats = match mtl::parse(&mtl_bytes, sib("MTI").as_deref()) {
            Ok(m) => m,
            Err(err) => { warn!("{}: {err}", e.name); continue }
        };
        f(stem.rsplit(['\\', '/']).next().unwrap_or(stem).to_ascii_lowercase(), model, mats);
    }
}

/// One triangle-list mesh of these packets; vertex colours stay in PS2 units (0x80 = 1.0).
/// B211: packets are stored in their nodes' spaces and drawn at the model's bind pose: a packet bound to one node by
/// its bind matrix, a weighted one skinned as [`mdl::Model::skinned`] (Queen's ball machine's launcher wheels and
/// balls sat under the grass at the origin; court 6's horses were torn apart).
pub fn mesh<'a>(packets: impl Iterator<Item = &'a mdl::Packet>, model: &mdl::Model) -> Option<Mesh> {
    let (mut pos, mut nrm, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![], vec![]);
    for pk in packets {
        let base = pos.len() as u32;
        if pk.palette.len() > 1 {
            let (verts, tris, _) = model.skin_packet(pk);
            for v in &verts {
                pos.push(v.pos);
                nrm.push((Vec3::from(v.normals[0]) + Vec3::from(v.normals[1])).to_array());
                uv.push(v.uv);
                col.push(v.color.map(|c| c as f32 / 128.0));
            }
            idx.extend(tris.iter().flatten().map(|i| i + base));
            continue;
        }
        let m = pk.palette.first().and_then(|&n| model.node_bind.get(n)).map_or(Mat4::IDENTITY, Mat4::from_cols_array_2d);
        for v in &pk.vertices {
            pos.push(m.transform_point3(Vec3::from(v.pos)).to_array());
            nrm.push(m.transform_vector3(Vec3::from(v.normal)).to_array());
            uv.push(v.uv);
            col.push(v.color.map(|c| c as f32 / 128.0)); // PS2 modulate: 0x80 = 1.0
        }
        idx.extend(pk.triangles.iter().flatten().map(|i| i + base));
    }
    if idx.is_empty() {
        return None;
    }
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx));
    Some(mesh)
}

pub fn image(t: &mtl::Texture) -> Image {
    let mut img = Image::new(
        Extent3d { width: t.width.max(1), height: t.height.max(1), depth_or_array_layers: 1 },
        TextureDimension::D2,
        if t.rgba.is_empty() { vec![255; 4 * (t.width.max(1) * t.height.max(1)) as usize] } else { t.rgba.clone() },
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // PS2 UVs tile freely; Bevy's default sampler clamps
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    img
}

pub fn orbit(
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut q: Query<(&mut Orbit, &mut Transform)>,
) {
    for (mut o, mut t) in &mut q {
        if buttons.pressed(MouseButton::Left) {
            o.yaw -= motion.delta.x * 0.005;
            o.pitch = (o.pitch - motion.delta.y * 0.005).clamp(-1.5, 1.5);
        }
        o.radius *= 1.0 - scroll.delta.y * 0.1;
        *t = o.transform();
    }
}

/// The match's cloud spawner between the setup and the points: the schedule's weathers (`HST_WEATHER` fixes one),
/// the weather the clouds were last made for, the court's count and the heavy-rain one, and the last layout's
/// weather and `rand()` draws until `clouds` lays them out. None when the court has no in-season cloud models.
pub struct Sky {
    pub weathers: [u8; hst_sim::weather::GAMES],
    pub fixed: Option<u8>,
    pub made: u8,
    pub count: usize,
    pub rain: usize,
    pub draws: Option<(u8, Vec<u32>)>,
}

impl Sky {
    pub fn made_count(&self, weather: u8) -> usize {
        if matches!(weather, 3 | 5) { self.rain } else { self.count }
    }
}

pub static SKY: std::sync::Mutex<Option<Sky>> = std::sync::Mutex::new(None);

/// A new point on game `game` of the schedule (`Rngs::new_point`): a weather change remakes the clouds
/// (`weather::clouds_remade`), their draws between the shared and the court generator's seeds, in doubles too.
pub fn point_clouds(rng: &mut hst_sim::rng::Rngs, game: usize) {
    let mut sky = SKY.lock().unwrap();
    let Some(s) = sky.as_mut().filter(|s| {
        let w = s.fixed.unwrap_or(s.weathers[game % hst_sim::weather::GAMES]);
        hst_sim::weather::clouds_remade(s.made, w)
    }) else {
        return rng.new_point();
    };
    let w = s.fixed.unwrap_or(s.weathers[game % hst_sim::weather::GAMES]);
    s.made = w;
    s.draws = Some((w, rng.new_point_clouds(s.made_count(w))));
}
