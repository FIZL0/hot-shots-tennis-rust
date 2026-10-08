//! Standard character mods (`modding/HST-CHARACTER-STANDARD.md` v1): `mods/<id>/mod.json` and its `.glb`
//! costumes, read into the same [`CharacterData`] as a disc character. The body comes from the mod; motions,
//! `.MOR` faces, arm table and (by `donor`) the game logic's tables come from the donor character on the disc.
//! A model that fails §7's conformance (`modding/tools/check.py`, ported) is rejected with the failed checks.
// ponytail: the match setup (M1c) and the character select (B40) read the manifest, TParam row and voices
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageAddressMode, ImageSampler, ImageSamplerDescriptor, ImageType};
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use hst_data::{iso::Iso, mtl, xb::Archive};
use hst_sim::pose::Skeleton;

use crate::character::{self, CharacterData, Joint, Motions};

/// The skeleton every mod is rigged to (names, parents, HST pc00's bind frames).
const SKELETON: &str = include_str!("../../../modding/hst_skeleton.json");
/// The face channels a morph face must carry (§4a).
const FACE: [&str; 8] = ["joy_eye", "joy_mouth", "anger_eye", "anger_mouth", "sorrow_eye", "sorrow_mouth", "doki_eye", "doki_mouth"];

/// A mod's manifest (`mod.json`, §6), checked.
#[derive(Debug, Clone)]
pub struct Mod {
    pub dir: PathBuf,
    pub id: String,
    pub name: String,
    /// Costume `.glb` paths, relative to `dir`.
    pub costumes: Vec<String>,
    /// The disc character (0..13) whose motions, faces, arm table, shot records and AI rows it uses.
    pub donor: usize,
    /// +1 right-handed, −1 left-handed (mirrored like Carol and Will).
    pub hand: f32,
    /// TParam.csv: the base row, and cells overridden by column header.
    pub base: usize,
    pub overrides: Vec<(String, String)>,
    pub ai_row: usize,
    /// `face.json` whole-face textures (§4b) instead of morph targets (`morph`), or `none` (a still face).
    pub texture_face: bool,
    pub no_face: bool,
    /// The voice folder, relative to `dir`.
    pub voice: String,
}

/// The mod playing in a match slot (`--mod DIR`, `--mod-slot N`) until the character select lists mods (B40a).
#[derive(Resource)]
pub struct MatchMod {
    pub slot: usize,
    pub m: Mod,
}

/// Reads and checks `dir/mod.json`.
pub fn read(dir: &Path) -> Result<Mod, String> {
    let at = dir.join("mod.json");
    let err = |m: String| format!("{}: {m}", at.display());
    let text = std::fs::read_to_string(&at).map_err(|e| err(e.to_string()))?;
    let j: serde_json::Value = serde_json::from_str(&text).map_err(|e| err(format!("not JSON: {e}")))?;
    let num = |v: &serde_json::Value, what: &str, max: u64| v.as_u64().filter(|&n| n <= max).map(|n| n as usize).ok_or_else(|| err(format!("`{what}` must be a number 0..={max}")));
    let text = |k: &str| j[k].as_str().map(str::to_string).ok_or_else(|| err(format!("`{k}` must be a string")));
    if j["standard"].as_u64() != Some(1) {
        return Err(err(format!("`standard` is {}, this game reads standard 1", j["standard"])));
    }
    let costumes: Vec<String> = j["costumes"].as_array().map(|a| a.iter().filter_map(|c| c.as_str().map(str::to_string)).collect()).unwrap_or_default();
    if costumes.is_empty() {
        return Err(err("`costumes` must list at least one .glb".into()));
    }
    if let Some(c) = costumes.iter().find(|c| !dir.join(c).is_file()) {
        return Err(err(format!("costume `{c}` not found")));
    }
    let donor = num(&j["donor"], "donor", 13)?;
    let hand = match j["hand"].as_str() {
        Some("right") => 1.0,
        Some("left") => -1.0,
        _ => return Err(err("`hand` must be \"right\" or \"left\"".into())),
    };
    let base = num(&j["params"]["base"], "params.base", 13)?;
    let mut overrides = Vec::new();
    for (k, v) in j["params"]["override"].as_object().into_iter().flatten() {
        let v = match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => return Err(err(format!("params.override `{k}` must be a number or a string"))),
        };
        column(k).ok_or_else(|| err(format!("params.override `{k}` is not a TParam.csv column")))?;
        overrides.push((k.clone(), v));
    }
    let texture_face = match j["face"].as_str() {
        Some("morph" | "none") => false,
        Some("texture") if dir.join("face.json").is_file() => true,
        Some("texture") => return Err(err("`face` \"texture\" needs a face.json".into())),
        _ => return Err(err("`face` must be \"morph\", \"texture\" or \"none\"".into())),
    };
    Ok(Mod {
        dir: dir.to_path_buf(),
        id: text("id")?,
        name: text("name")?,
        costumes,
        donor,
        hand,
        base,
        overrides,
        ai_row: num(&j["ai_row"], "ai_row", 13)?,
        texture_face,
        no_face: j["face"] == "none",
        voice: j["voice"].as_str().unwrap_or("voice/").to_string(),
    })
}

/// TParam.csv's column headers, line breaks as spaces.
const COLUMNS: [&str; 67] = [
    "#", "通称", "性別", "身長", "モデル", "モデルタイプ", "利き腕", "タイプ", "得意１", "得意２", "Special SHOT", "Special POW", "Serv POW",
    "Strk POW", "Voley POW", "Lob POW", "Lob POW2", "Top SPIN", "Slice SPIN", "Drop SPIN", "Strk CON", "Voley CON", "Serv CON", "Body ADJ",
    "Vbdy ADJ", "Back ADJ", "Rizing ADJ", "Run CON", "ショット ウサギIMP GI/NI/BI", "ショット カメIMP GI/NI/BI", "強トス ウサギIMP GI/NI/BI",
    "強トス カメIMP GI/NI/BI", "弱トス ウサギIMP GI/NI/BI", "弱トス カメIMP GI/NI/BI", "Strk GH", "Strk NH", "BH (参考値）", "Voley GH",
    "Voley NH", "BH (参考値）", "SPE", "STA", "飛びつき減少/バックハンド減少/スマッシュ減少", "Agili", "計", "Body DWN", "Back DWN",
    "Rizing DWN", "V DWN", "Strk UP", "LOW POW", "V LOW POW", "SM LOW POW", "Strk HI POW", "サーブ手前ブレ倍率", "リーチ基点（ｍ）",
    "リーチげた(cm)", "リーチ(cm)", "飛びつき（遠）開始", "飛びつき限界", "ボディショット切り替え(cm)", "衝突判定(cm)(縦/横)",
    "アンダーショット下限(cm)", "最適高度(cm) (ストローク/ボレー）", "スマッシュ高度 （最高/最適/最低）", "通常サーブ高度 （最高/最適/最低）",
    "アンダーサーブ高度 （最高/最適/最低）",
];

/// The column of header `name` (whitespace-blind; the first of a repeated header).
fn column(name: &str) -> Option<usize> {
    let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    COLUMNS.iter().position(|c| squash(c) == squash(name))
}

/// The mod's TParam.csv row as cells: row `base` with the overrides (`play.rs` reads rows the same way).
pub fn tparam(iso: &mut Iso, m: &Mod) -> Result<Vec<String>, String> {
    let data = iso.read("PCDATA/PCDATA.XB").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).ok_or("no TParam.csv")?;
    let csv = arc.read(e).map_err(|e| e.0)?;
    let tag = format!("{},", m.base);
    let row = csv.split(|&b| b == b'\n').find(|l| l.starts_with(tag.as_bytes())).ok_or("no TParam.csv row")?;
    let mut cells: Vec<String> = row.split(|&b| b == b',').map(|c| String::from_utf8_lossy(c).trim().to_string()).collect();
    for (k, v) in &m.overrides {
        let c = column(k).unwrap();
        *cells.get_mut(c).ok_or("short TParam.csv row")? = v.clone();
    }
    Ok(cells)
}

/// The mod's voice programs (§5), if it ships any `<program>_<key>.wav`.
pub fn voice(m: &Mod) -> Option<crate::audio::SoundBank> {
    crate::audio::SoundBank::wavs(&m.dir.join(&m.voice))
}

/// A `.glb`: its document and binary chunk.
fn open(path: &Path) -> Result<(gltf::Document, Vec<u8>), String> {
    // ponytail: glTF's own validation skipped (older rerig.py output lacks POSITION min/max); §7's checks gate the file
    let g = gltf::Gltf::from_slice_without_validation(&std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?).map_err(|e| format!("{}: {e}", path.display()))?;
    if g.buffers().any(|b| !matches!(b.source(), gltf::buffer::Source::Bin)) {
        return Err(format!("{}: buffers must be embedded (.glb)", path.display()));
    }
    Ok((g.document, g.blob.unwrap_or_default()))
}

/// Every node's glTF world matrix.
fn worlds(doc: &gltf::Document) -> Vec<Mat4> {
    let mut parent = vec![None; doc.nodes().len()];
    for n in doc.nodes() {
        n.children().for_each(|c| parent[c.index()] = Some(n.index()));
    }
    fn world(i: usize, doc: &gltf::Document, parent: &[Option<usize>], w: &mut [Option<Mat4>]) -> Mat4 {
        if let Some(m) = w[i] {
            return m;
        }
        let local = Mat4::from_cols_array_2d(&doc.nodes().nth(i).unwrap().transform().matrix());
        let m = parent[i].map_or(Mat4::IDENTITY, |p| world(p, doc, parent, w)) * local;
        w[i] = Some(m);
        m
    }
    let mut w = vec![None; parent.len()];
    (0..parent.len()).map(|i| world(i, doc, &parent, &mut w)).collect()
}

/// §7's checks (`modding/tools/check.py`): the failures as `Err`, else the warnings.
fn conform(doc: &gltf::Document, blob: &[u8], morph_face: bool) -> Result<Vec<String>, String> {
    let (mut fail, mut warn) = (Vec::new(), Vec::new());
    let std: serde_json::Value = serde_json::from_str(SKELETON).unwrap();
    let core: Vec<(&str, &str, Mat4)> = std["joints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| {
            let f = |k: &str, i: usize| j[k][i].as_f64().unwrap() as f32;
            let bind = Mat4::from_rotation_translation(Quat::from_xyzw(f("bind_rotation", 0), f("bind_rotation", 1), f("bind_rotation", 2), f("bind_rotation", 3)), Vec3::new(f("bind_position", 0), f("bind_position", 1), f("bind_position", 2)));
            (j["name"].as_str().unwrap(), j["parent"].as_str().unwrap_or("game_space"), bind)
        })
        .collect();
    let root = doc.default_scene().or_else(|| doc.scenes().next()).and_then(|s| s.nodes().next());
    let rot = root.as_ref().map(|r| r.transform().decomposed().1);
    if root.as_ref().and_then(|r| r.name()) != Some("game_space") || !rot.is_some_and(|q| Quat::from_array(q).abs_diff_eq(Quat::from_xyzw(0.0, 0.0, 1.0, 0.0), 1e-5)) {
        fail.push("the scene root must be `game_space`, rotation [0,0,1,0]".to_string());
    }
    let skins: Vec<_> = doc.skins().collect();
    let [skin] = &skins[..] else { return Err(format!("needs exactly one skin (has {})", skins.len())) };
    let name = |n: &gltf::Node| n.name().unwrap_or("").replace(' ', "");
    let mut parent = vec![None; doc.nodes().len()];
    for n in doc.nodes() {
        n.children().for_each(|c| parent[c.index()] = Some(n.index()));
    }
    let nodes: Vec<_> = doc.nodes().collect();
    let joints: Vec<_> = skin.joints().collect();
    let ibm: Vec<Mat4> = skin.reader(|_| Some(blob)).read_inverse_bind_matrices().map(|m| m.map(|m| Mat4::from_cols_array_2d(&m)).collect()).unwrap_or_else(|| vec![Mat4::IDENTITY; joints.len()]);
    let at = |n: &str| joints.iter().position(|j| name(j) == n);
    // binds in game space as check.py takes them: G·K·IBM⁻¹, K the skin's mesh → world from the pelvis
    let w = worlds(doc);
    let g = Mat4::from_diagonal(Vec4::new(-1.0, -1.0, 1.0, 1.0));
    let k = at("Bip01Pelvis").unwrap_or(0);
    let kk = w[joints[k].index()] * ibm[k];
    let bind: Vec<Mat4> = ibm.iter().map(|m| g * kk * m.inverse()).collect();
    let missing: Vec<_> = core.iter().filter(|c| at(c.0).is_none()).map(|c| c.0).collect();
    if !missing.is_empty() {
        fail.push(format!("missing core joints: {}", missing.join(", ")));
    }
    let wrong: Vec<_> = core
        .iter()
        .filter_map(|&(n, p, _)| {
            let got = parent[joints[at(n)?].index()].map(|i| name(&nodes[i])).unwrap_or_default();
            (got != p).then(|| format!("{n} under {got} (want {p})"))
        })
        .collect();
    if !wrong.is_empty() {
        fail.push(format!("core parents: {}", wrong.join("; ")));
    }
    let err = joints.iter().zip(&bind).map(|(j, b)| (g * w[j.index()] - *b).to_cols_array().iter().fold(0f32, |a, x| a.max(x.abs()))).fold(0f32, f32::max);
    if err >= 1e-3 {
        fail.push(format!("rest pose must be the bind pose (max diff {err:.2e})"));
    }
    // bone axes: each core bone's direction in its own frame against HST's
    let hst = |n: &str| core.iter().find(|c| c.0 == n).map(|c| c.2);
    let mut worst = (0f32, String::new());
    for (n, c) in children() {
        let (Some(a), Some(b), Some(ha), Some(hb)) = (at(&n), at(&c), hst(&n), hst(&c)) else { continue };
        let d = bind[a].inverse().transform_vector3(bind[b].w_axis.truncate() - bind[a].w_axis.truncate());
        let dh = ha.inverse().transform_vector3(hb.w_axis.truncate() - ha.w_axis.truncate());
        let deg = d.angle_between(dh).to_degrees();
        if deg > worst.0 {
            worst = (deg, n);
        }
    }
    if worst.0 >= 10.0 {
        fail.push(format!("bone axes must match HST's (worst {} {:.1}°)", worst.1, worst.0));
    }
    let h = at("Bip01").map_or(0.0, |i| -bind[i].w_axis.y);
    if !(0.2 < h && h < 2.0) {
        fail.push(format!("Bip01 height {h:.3} m (HST 0.58–0.95)"));
    } else if !(0.4 < h && h < 1.3) {
        warn.push(format!("Bip01 height {h:.3} m (HST 0.58–0.95)"));
    }
    let (mut feet, mut over4, mut sums, mut targets) = (f32::NAN, false, 0, Vec::new());
    for n in doc.nodes() {
        let Some(mesh) = n.mesh() else { continue };
        targets.extend(target_names(&mesh));
        for p in mesh.primitives().filter(|_| n.skin().is_some()) {
            let r = p.reader(|_| Some(blob));
            feet = r.read_positions().into_iter().flatten().map(|v| v[1]).fold(feet, f32::max);
            sums += r.read_weights(0).map(|w| w.into_f32().filter(|w| (w.iter().sum::<f32>() - 1.0).abs() > 0.01).count()).unwrap_or(0);
            over4 |= p.get(&gltf::Semantic::Joints(1)).is_some();
        }
    }
    if !(feet.abs() < 0.06) {
        warn.push(format!("feet at y=0 (lowest point {feet:.3})"));
    }
    if over4 || sums > 0 {
        fail.push(format!("≤4 weights per vertex summing to 1 ({sums} bad sums{})", if over4 { ", JOINTS_1 present" } else { "" }));
    }
    let face: Vec<_> = FACE.iter().filter(|c| !targets.iter().any(|t| t == *c)).collect();
    if morph_face && !face.is_empty() {
        warn.push(format!("face channels missing: {face:?}"));
    }
    if morph_face && !targets.iter().any(|t| t == "blink_eye") {
        warn.push("no blink_eye".into());
    }
    if fail.is_empty() { Ok(warn) } else { Err(fail.join("\n")) }
}

/// The core bones and the child each points at (`rerig.py`'s CHILD).
fn children() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = [("Spine", "Spine1"), ("Spine1", "Spine2"), ("Spine2", "Neck"), ("Neck", "Head")].map(|(a, b)| (format!("Bip01{a}"), format!("Bip01{b}"))).into();
    for s in ["L", "R"] {
        for (a, b) in [("Clavicle", "UpperArm"), ("UpperArm", "Forearm"), ("Forearm", "Hand"), ("Hand", "Finger2"), ("Thigh", "Calf"), ("Calf", "Foot"), ("Foot", "Toe0")] {
            v.push((format!("Bip01{s}{a}"), format!("Bip01{s}{b}")));
        }
        for f in 0..5 {
            v.push((format!("Bip01{s}Finger{f}"), format!("Bip01{s}Finger{f}1")));
            v.push((format!("Bip01{s}Finger{f}1"), format!("Bip01{s}Finger{f}2")));
        }
    }
    v
}

fn target_names(mesh: &gltf::Mesh) -> Vec<String> {
    let extras: Option<serde_json::Value> = mesh.extras().as_ref().and_then(|r| serde_json::from_str(r.get()).ok());
    extras.and_then(|e| e["targetNames"].as_array().map(|a| a.iter().filter_map(|t| t.as_str().map(str::to_string)).collect())).unwrap_or_default()
}

/// The `.glb`'s materials as a disc MTL would give them: an unlit `StandardMaterial` and its GS draws.
#[allow(clippy::type_complexity)]
fn materials_of(
    doc: &gltf::Document,
    blob: &[u8],
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    gs: &mut HashMap<AssetId<StandardMaterial>, Vec<crate::gs::GsMaterial>>,
) -> Result<Vec<Handle<StandardMaterial>>, String> {
    let sampler = || ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: ImageAddressMode::Repeat, address_mode_v: ImageAddressMode::Repeat, ..ImageSamplerDescriptor::linear() });
    let mut tex = Vec::new();
    for im in doc.images() {
        let gltf::image::Source::View { view, mime_type } = im.source() else { return Err("images must be embedded".into()) };
        let bytes = blob.get(view.offset()..view.offset() + view.length()).ok_or("image outside the buffer")?;
        let img = Image::from_buffer(bytes, ImageType::MimeType(mime_type), CompressedImageFormats::NONE, true, sampler(), RenderAssetUsages::RENDER_WORLD).map_err(|e| format!("image {}: {e}", im.index()))?;
        let mut raw = img.clone();
        raw.texture_descriptor.format = bevy::render::render_resource::TextureFormat::Rgba8Unorm;
        tex.push((images.add(img), images.add(raw)));
    }
    let mut out = Vec::new();
    for m in doc.materials() {
        let t = m.pbr_metallic_roughness().base_color_texture().map(|i| i.texture().source().index());
        let color = m.pbr_metallic_roughness().base_color_factor();
        let mask = m.alpha_mode() == gltf::material::AlphaMode::Mask;
        let h = materials.add(StandardMaterial {
            base_color: Color::linear_rgba(color[0], color[1], color[2], color[3]),
            base_color_texture: t.and_then(|t| tex.get(t)).map(|t| t.0.clone()),
            unlit: true,
            double_sided: true,
            cull_mode: None,
            // texture alpha is not coverage on HST bodies; `MASK` marks the real cut-outs (§2)
            alpha_mode: if mask { AlphaMode::Mask(m.alpha_cutoff().unwrap_or(0.5)) } else { AlphaMode::Opaque },
            ..default()
        });
        // ponytail: a glTF material has none of the MTL header's lighting words (shininess, highlight): zero, as a
        // matte disc material; MASK is TEST mode 10 (A ≥ 0x40)
        let mut header = [0; 0x30];
        header[0x1e..0x20].copy_from_slice(&(if mask { 10i16 } else { 0 }).to_le_bytes());
        let mtl = mtl::Material { texture: t, color: color.map(|c| c.min(1.0)), attributes: None, two_sided: true, name: m.name().unwrap_or("").to_string(), header };
        gs.insert(h.id(), crate::gs::GsMaterial::for_batch(&mtl, if t.is_some() { 0x10 } else { 0 }, t.and_then(|t| tex.get(t)).map(|t| t.1.clone())));
        out.push(h);
    }
    // glTF's default material
    out.push(materials.add(StandardMaterial { unlit: true, double_sided: true, cull_mode: None, ..default() }));
    Ok(out)
}

/// One primitive's vertices.
#[derive(Default)]
struct Prim {
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    col: Vec<[f32; 4]>,
    joints: Vec<[u16; 4]>,
    weights: Vec<[f32; 4]>,
    idx: Vec<u32>,
    /// Per target name, the position offsets.
    morph: Vec<(String, Vec<[f32; 3]>)>,
}

fn prim(p: &gltf::Primitive, blob: &[u8], names: &[String]) -> Result<Prim, String> {
    if p.mode() != gltf::mesh::Mode::Triangles {
        return Err("primitives must be triangle lists".into());
    }
    let r = p.reader(|_| Some(blob));
    let pos: Vec<[f32; 3]> = r.read_positions().ok_or("primitive without POSITION")?.collect();
    let n = pos.len();
    Ok(Prim {
        nrm: r.read_normals().map_or_else(|| vec![[0.0, -1.0, 0.0]; n], |i| i.collect()),
        uv: r.read_tex_coords(0).map_or_else(|| vec![[0.0; 2]; n], |i| i.into_f32().collect()),
        // vertex colour 1.0 is the GS's 0x80 (`hst-gltf`)
        col: r.read_colors(0).map_or_else(|| vec![[1.0; 4]; n], |i| i.into_rgba_f32().collect()),
        joints: r.read_joints(0).map_or_else(|| vec![[0; 4]; n], |i| i.into_u16().collect()),
        weights: r.read_weights(0).map_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; n], |i| i.into_f32().collect()),
        idx: r.read_indices().map_or_else(|| (0..n as u32).collect(), |i| i.into_u32().collect()),
        morph: r.read_morph_targets().zip(names).map(|((p, ..), name)| (name.clone(), p.map_or_else(|| vec![[0.0; 3]; n], |p| p.collect()))).collect(),
        pos,
    })
}

/// Loads costume `costume` of mod `m` (motions, faces and arm table from the donor on the disc).
pub fn load(
    iso: &mut Iso,
    m: &Mod,
    costume: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<CharacterData, String> {
    let file = m.dir.join(m.costumes.get(costume).ok_or_else(|| format!("{}: no costume {costume}", m.id))?);
    let ctx = |e: String| format!("{}: {e}", file.display());
    let (doc, blob) = open(&file)?;
    for w in conform(&doc, &blob, !m.texture_face && !m.no_face).map_err(|e| ctx(format!("does not conform to the HST character standard:\n{e}")))? {
        warn!("{}: {w}", file.display());
    }
    let skin = doc.skins().next().unwrap();
    let nodes: Vec<_> = skin.joints().collect();
    let index: HashMap<usize, usize> = nodes.iter().enumerate().map(|(k, n)| (n.index(), k)).collect();
    let mut up = vec![None; doc.nodes().len()];
    for n in doc.nodes() {
        n.children().for_each(|c| up[c.index()] = Some(n.index()));
    }
    // game space is below `game_space`: each joint's world there, its parent the nearest joint above it
    let w = worlds(&doc);
    let space = doc.default_scene().or_else(|| doc.scenes().next()).and_then(|s| s.nodes().next()).map_or(Mat4::IDENTITY, |r| w[r.index()].inverse());
    let ibm: Vec<Mat4> = skin.reader(|_| Some(&blob[..])).read_inverse_bind_matrices().map(|i| i.map(|m| Mat4::from_cols_array_2d(&m)).collect()).unwrap_or_else(|| vec![Mat4::IDENTITY; nodes.len()]);
    let joints: Vec<Joint> = nodes
        .iter()
        .enumerate()
        .map(|(k, n)| {
            let mut p = up[n.index()];
            while let Some(i) = p.filter(|i| !index.contains_key(i)) {
                p = up[i];
            }
            let parent = p.map(|i| index[&i]);
            let world = space * w[n.index()];
            let local = parent.map_or(world, |q| (space * w[nodes[q].index()]).inverse() * world);
            Joint { name: n.name().unwrap_or("").replace(' ', ""), parent, rest: Transform::from_matrix(local), inverse_bind: ibm[k] }
        })
        .collect();
    let skeleton = Skeleton {
        names: joints.iter().map(|j| j.name.clone()).collect(),
        parent: joints.iter().map(|j| j.parent).collect(),
        rest: joints.iter().map(|j| j.rest.to_matrix().to_cols_array_2d()).collect(),
    };

    let mut gs = HashMap::new();
    let handles = materials_of(&doc, &blob, images, materials, &mut gs).map_err(ctx)?;
    // the skinned parts, one per primitive, every morphed one carrying all the model's targets
    let mut names: Vec<String> = Vec::new();
    for mesh in doc.meshes() {
        names.extend(target_names(&mesh).into_iter().filter(|t| !names.contains(t)).collect::<Vec<_>>());
    }
    let mut parts = Vec::new();
    for n in doc.nodes().filter(|n| n.skin().is_some()) {
        let mesh = n.mesh().unwrap();
        let tn = target_names(&mesh);
        for p in mesh.primitives() {
            let v = prim(&p, &blob, &tn).map_err(ctx)?;
            let count = v.pos.len();
            // ponytail: VU1's per-bone normals from one glTF normal — the first joint's share in the normal slot, the
            // rest in the second joint's tangent slot (gs.wgsl); exact for one or two influences
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, v.pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, v.nrm.iter().zip(&v.weights).map(|(n, w)| Vec3::from(*n) * w[0]).map(<[f32; 3]>::from).collect::<Vec<_>>())
                .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, v.nrm.iter().zip(&v.weights).map(|(n, w)| (Vec3::from(*n) * (1.0 - w[0])).extend(0.0).to_array()).collect::<Vec<_>>())
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, v.uv)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, v.col)
                .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(v.joints))
                .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, v.weights)
                .with_inserted_indices(Indices::U32(v.idx));
            let morphed = v.morph.iter().any(|t| t.1.iter().any(|d| *d != [0.0; 3]));
            if morphed {
                let zero = vec![[0.0; 3]; count];
                let attrs = names.iter().flat_map(|t| v.morph.iter().find(|m| m.0 == *t).map_or(&zero, |m| &m.1).iter().map(|d| bevy::mesh::morph::MorphAttributes::new(Vec3::from(*d), Vec3::ZERO, Vec3::ZERO)).collect::<Vec<_>>());
                mesh.set_morph_targets(attrs.collect());
            }
            let mat = handles[p.material().index().unwrap_or(handles.len() - 1)].clone();
            parts.push((meshes.add(mesh), mat, morphed));
        }
    }

    // the racket: racket.glb in the Racket joint's space, else the donor's
    let racket = match m.dir.join("racket.glb") {
        f if f.is_file() => {
            let (rdoc, rblob) = open(&f)?;
            let rh = materials_of(&rdoc, &rblob, images, materials, &mut gs)?;
            let mut out = Vec::new();
            for mesh in rdoc.meshes() {
                for p in mesh.primitives() {
                    let v = prim(&p, &rblob, &[]).map_err(|e| format!("{}: {e}", f.display()))?;
                    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, v.pos)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, v.nrm)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, v.uv)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, v.col)
                        .with_inserted_indices(Indices::U32(v.idx));
                    out.push((meshes.add(mesh), rh[p.material().index().unwrap_or(rh.len() - 1)].clone()));
                }
            }
            out
        }
        _ => {
            let data = iso.read(&format!("PC/PC{:02}C00.XB", m.donor)).map_err(|e| e.to_string())?;
            character::disc_racket(&Archive::parse(&data).map_err(|e| e.0)?, m.donor, meshes, materials, images, &mut gs)?
        }
    };

    // the donor's motions; its face tracks (`face\x01joy_eye`) bind by the name after the object prefix
    let Motions { motions, paths, pelvis, arm, faces, stance_ball } = character::disc_motions(iso, m.donor, &skeleton, |t| names.iter().position(|n| Some(n.as_str()) == t.rsplit('\x01').next()))?;
    let binds = bindposes.add(SkinnedMeshInverseBindposes::from(joints.iter().map(|j| j.inverse_bind).collect::<Vec<_>>()));
    Ok(CharacterData { joints, parts, racket, motions, paths, binds, pelvis, arm, morph_targets: names.len(), faces, stance_ball, skeleton, noise: None, gs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hst_sim::pose::{model, M4};

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
    /// Test mods (local, `research/journal/2026-10-08-m1a-mod-loader`): `test_pc00` is HST's pc00 through
    /// `modding/tools/rerig.py`, `test_broken` the same with `Bip01LThigh` renamed, `fore_pc00_phoebe` a packaged mod.
    const MODS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/mods");

    struct Stores(Assets<Mesh>, Assets<StandardMaterial>, Assets<Image>, Assets<SkinnedMeshInverseBindposes>);

    fn load_mod(iso: &mut Iso, s: &mut Stores, id: &str) -> Result<(Mod, CharacterData), String> {
        let m = read(&Path::new(MODS).join(id))?;
        let c = load(iso, &m, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3)?;
        Ok((m, c))
    }

    /// Model-space joint positions of `c` playing motion `id` at time `t`, by joint name.
    fn pose(c: &CharacterData, id: usize, t: f32) -> HashMap<String, Vec3> {
        let m: Vec<M4> = model(&c.skeleton, &c.motions[&id].locals(&c.skeleton, t));
        c.skeleton.names.iter().zip(m).map(|(n, m)| (n.clone(), Vec3::new(m[3][0], m[3][1], m[3][2]))).collect()
    }

    /// A rerigged pc00 loads like the disc's pc00: rest = bind, and its forehand and run put every core joint where
    /// the disc character's go; the manifest's hand, TParam overrides and face bindings come through.
    #[test]
    fn rerigged_mod_plays_forehand_and_run() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        if !Path::new(MODS).join("test_pc00").is_dir() {
            return eprintln!("no context/mods/test_pc00, skipped");
        }
        let mut s = Stores(default(), default(), default(), default());
        let (m, c) = load_mod(&mut iso, &mut s, "test_pc00").unwrap();
        let disc = character::load_disc(&mut iso, 0, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3).unwrap();
        // rest = bind: each joint's model matrix at rest undoes its inverse bind
        let rest = model(&c.skeleton, &c.skeleton.rest);
        let worst = rest.iter().zip(&c.joints).map(|(r, j)| (Mat4::from_cols_array_2d(r) * j.inverse_bind - Mat4::IDENTITY).to_cols_array().iter().fold(0f32, |a, x| a.max(x.abs()))).fold(0f32, f32::max);
        assert!(worst < 1e-3, "rest·inverse bind off by {worst}");
        for (id, ts) in [(0x10, [0.0, 0.3, 0.6]), (3, [0.0, 0.25, 0.5])] {
            for t in ts.map(|f| f * c.motions[&id].length) {
                let (a, b) = (pose(&c, id, t), pose(&disc, id, t));
                let (err, joint) = b.iter().filter_map(|(n, p)| Some(((a.get(n)? - *p).length(), n.clone()))).fold((0f32, String::new()), |w, x| if x.0 > w.0 { x } else { w });
                assert!(err < 0.01, "motion {id:#x} t {t}: {joint} off by {err} m");
            }
        }
        assert!(c.arm.is_some() && c.joint("Racket").is_some() && !c.racket.is_empty());
        assert!(c.faces.values().any(|f| !f.tracks.is_empty()), "no face track bound");
        assert_eq!(c.faces.values().map(|f| f.tracks.len()).sum::<usize>(), disc.faces.values().map(|f| f.tracks.len()).sum::<usize>());
        assert_eq!(m.hand, -1.0);
        let row = tparam(&mut iso, &m).unwrap();
        assert_eq!((&row[12][..], &row[40][..], &row[57][..], &row[13][..]), ("4", "10", "170", "4"));
    }

    /// A packaged rerig.py mod (Fore!'s Phoebe on donor 6) plays the forehand and run without breaking up.
    #[test]
    fn packaged_mod_plays() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        if !Path::new(MODS).join("fore_pc00_phoebe").is_dir() {
            return eprintln!("no context/mods/fore_pc00_phoebe, skipped");
        }
        let mut s = Stores(default(), default(), default(), default());
        let (m, c) = load_mod(&mut iso, &mut s, "fore_pc00_phoebe").unwrap();
        assert_eq!(m.donor, 6);
        for id in [0x10, 3] {
            let clip = &c.motions[&id];
            let hand: Vec<Vec3> = (0..8).map(|k| pose(&c, id, clip.length * k as f32 / 8.0)["Bip01RHand"]).collect();
            assert!(hand.iter().all(|p| p.is_finite() && p.length() < 2.5), "{hand:?}");
            assert!(hand.iter().any(|p| (*p - hand[0]).length() > 0.05), "motion {id:#x} doesn't move the hand");
        }
    }

    /// A model off the standard is refused, naming what is wrong.
    #[test]
    fn rejects_non_conforming_mod() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        if !Path::new(MODS).join("test_broken").is_dir() {
            return eprintln!("no context/mods/test_broken, skipped");
        }
        let mut s = Stores(default(), default(), default(), default());
        let e = load_mod(&mut iso, &mut s, "test_broken").err().unwrap();
        eprintln!("{e}");
        assert!(e.contains("missing core joints: Bip01LThigh") && e.contains("Bip01LCalf under LeftThigh"), "{e}");
    }

    /// Bad manifests are refused with the field named.
    #[test]
    fn checks_the_manifest() {
        let dir = std::env::temp_dir().join(format!("hst-mod-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("model")).unwrap();
        std::fs::write(dir.join("model/c00.glb"), b"").unwrap();
        let try_ = |json: &str| {
            std::fs::write(dir.join("mod.json"), json).unwrap();
            read(&dir)
        };
        let ok = r#"{"standard":1,"id":"x","name":"X","costumes":["model/c00.glb"],"donor":3,"hand":"right","params":{"base":3,"override":{"Strk POW":5}},"ai_row":3,"face":"morph","voice":"voice/"}"#;
        let m = try_(ok).unwrap();
        assert_eq!((m.donor, m.hand, m.overrides.clone()), (3, 1.0, vec![("Strk POW".to_string(), "5".to_string())]));
        for (bad, field) in [("\"standard\":1", "\"standard\":2"), ("\"donor\":3", "\"donor\":14"), ("\"right\"", "\"ambi\""), ("Strk POW", "Strk PWR"), ("\"morph\"", "\"texture\""), ("model/c00.glb", "model/c01.glb")].map(|(a, b)| (ok.replace(a, b), b)) {
            let e = try_(&bad).err().unwrap_or_default();
            assert!(!e.is_empty(), "accepted {field}");
            eprintln!("{e}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Every mod under `$HST_MODS` (e.g. the mods repo's `out/mods`) loads costume 0; prints the failures.
    #[test]
    #[ignore]
    fn loads_every_mod() {
        let (Ok(mut iso), Ok(root)) = (Iso::open(ISO), std::env::var("HST_MODS")) else { return eprintln!("no ISO or HST_MODS, skipped") };
        let mut dirs: Vec<_> = std::fs::read_dir(root).unwrap().flatten().map(|e| e.path()).filter(|d| d.join("mod.json").is_file()).collect();
        dirs.sort();
        let mut bad = 0;
        for d in &dirs {
            let mut s = Stores(default(), default(), default(), default());
            if let Err(e) = read(d).and_then(|m| load(&mut iso, &m, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3)) {
                bad += 1;
                eprintln!("FAIL {e}");
            }
        }
        eprintln!("{} of {} mods load", dirs.len() - bad, dirs.len());
        assert_eq!(bad, 0);
    }
}
