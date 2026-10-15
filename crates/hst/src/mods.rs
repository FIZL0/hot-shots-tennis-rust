//! Standard character mods (HST-MODS `standard/HST-CHARACTER-STANDARD.md` v1): `mods/<id>/mod.json` and its `.glb`
//! costumes, read into the same [`CharacterData`] as a disc character. The body comes from the mod; motions,
//! `.MOR` faces, arm table and (by `donor`) the game logic's tables come from the donor character on the disc.
//! A model that fails §7's conformance (HST-MODS `tools/rerig/check.py`, ported) is rejected with the failed checks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageAddressMode, ImageSampler, ImageSamplerDescriptor, ImageType};
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use hst_data::{iso::Iso, mtl, xb::Archive};
use hst_sim::pose::{Clip, Skeleton};

use crate::character::{self, CharacterData, Joint, Motions, Store};

pub mod umpire;

/// The skeleton every mod is rigged to (names, parents, HST pc00's bind frames).
const SKELETON: &str = include_str!("../hst_skeleton.json");
/// The face channels a morph face must carry (§4a).
const FACE: [&str; 8] = ["joy_eye", "joy_mouth", "anger_eye", "anger_mouth", "sorrow_eye", "sorrow_mouth", "doki_eye", "doki_mouth"];

/// A mod's manifest (`mod.json`, §6), checked.
#[derive(Debug, Clone)]
pub struct Mod {
    pub dir: PathBuf,
    pub id: String,
    pub name: String,
    /// The game it comes from (`source` up to its platform, e.g. "Hot Shots Tennis: Get a Grip"); empty when unset.
    pub source: String,
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
    /// The mod's own motions `.glb` (§6 `motions`), relative to `dir`: drawn in place of the donor's, same timing.
    pub motions: Option<String>,
    /// §6 `costume_of`: a costume pack for this disc character (0..13): the select lists its costumes after the
    /// character's own, and it plays exactly as that character (`donor`, `params.base` and `ai_row` are it, no
    /// overrides).
    pub costume_of: Option<usize>,
    /// Online (M2i): the skeleton every peer simulates this mod on (its owner's costume's, from the lobby), in place
    /// of the one its looks load with. `None` offline.
    pub skeleton: Option<std::sync::Arc<Skeleton>>,
}

/// The mods playing in a match, by match slot (`--slot-mod N DIR` from the character select, or `--mod DIR
/// --mod-slot N`); `None` slots play their disc character.
#[derive(Resource, Default)]
pub struct MatchMod(pub Vec<Option<Mod>>);

impl MatchMod {
    pub fn get(&self, slot: usize) -> Option<&Mod> {
        self.0.get(slot)?.as_ref()
    }

    pub fn set(&mut self, slot: usize, m: Mod) {
        if self.0.len() <= slot {
            self.0.resize(slot + 1, None);
        }
        self.0[slot] = Some(m);
    }
}

impl Mod {
    /// B248: the mod's unique id among the installed ones: its folder's name (`modfs::folder_for` gives two mods
    /// sharing an `id` their own folders). Two mods in packs (`list`) may share one; tell those apart by `dir`.
    pub fn folder(&self) -> String {
        self.dir.file_name().map_or_else(|| self.id.clone(), |n| n.to_string_lossy().to_string())
    }

    /// B369: the select's category tab it goes under: its game (`source`), "Mods" when unset.
    pub fn game(&self) -> &str {
        if self.source.is_empty() { "Mods" } else { &self.source }
    }
}

/// Reads and checks `dir/mod.json`.
pub fn read(dir: &Path) -> Result<Mod, String> {
    let at = dir.join("mod.json");
    let err = |m: String| format!("{}: {m}", at.display());
    let text = crate::modfs::read_to_string(&at).map_err(|e| err(e.to_string()))?;
    let j: serde_json::Value = serde_json::from_str(&text).map_err(|e| err(format!("not JSON: {e}")))?;
    if let k @ (Kind::Court | Kind::Umpire | Kind::Other(_)) = Kind::of(&j) {
        return Err(err(format!("`kind` is {k}: not a character mod")));
    }
    let num = |v: &serde_json::Value, what: &str, max: u64| v.as_u64().filter(|&n| n <= max).map(|n| n as usize).ok_or_else(|| err(format!("`{what}` must be a number 0..={max}")));
    // shown as plain text, length-capped (M16)
    let text = |k: &str| j[k].as_str().filter(|t| (1..=64).contains(&t.chars().count())).map(str::to_string).ok_or_else(|| err(format!("`{k}` must be a string of 1 to 64 characters")));
    // a path in the manifest names a file inside the mod, never anything outside it (M16)
    let inside = |k: &str, f: &str| if crate::modfs::plain(f) { Ok(()) } else { Err(err(format!("`{k}` `{f}` must be a path inside the mod folder"))) };
    if j["standard"].as_u64() != Some(1) {
        return Err(err(format!("`standard` is {}, this game reads standard 1", j["standard"])));
    }
    let mut costumes: Vec<String> = j["costumes"].as_array().map(|a| a.iter().filter_map(|c| c.as_str().map(str::to_string)).collect()).unwrap_or_default();
    costumes.iter().try_for_each(|c| inside("costumes", c))?;
    // M1n: a listed costume with no .glb is left out (the select never offers an empty one), not the whole mod
    costumes.retain(|c| {
        let ok = crate::modfs::is_file(&dir.join(c));
        if !ok {
            warn!("{}: costume `{c}` not found, left out", at.display());
        }
        ok
    });
    if costumes.is_empty() {
        return Err(err("`costumes` must list at least one .glb that exists".into()));
    }
    let costume_of = if j["costume_of"].is_null() { None } else { Some(num(&j["costume_of"], "costume_of", 13)?) };
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
        override_ok(k, &v).map_err(err)?;
        overrides.push((k.clone(), v));
    }
    let texture_face = match j["face"].as_str() {
        Some("morph" | "none") => false,
        Some("texture") if crate::modfs::is_file(&dir.join("face.json")) => true,
        Some("texture") => return Err(err("`face` \"texture\" needs a face.json".into())),
        _ => return Err(err("`face` must be \"morph\", \"texture\" or \"none\"".into())),
    };
    if !j["eyes"].is_null() && !j["eyes"].as_array().is_some_and(|a| a.iter().all(|m| m.is_string())) {
        return Err(err("`eyes` must list the eyes' glTF material names".into()));
    }
    if let Some(f) = j["motions"].as_str() {
        inside("motions", f)?;
    }
    let voice = j["voice"].as_str().unwrap_or("voice/").to_string();
    inside("voice", voice.strip_suffix('/').unwrap_or(&voice))?;
    let motions = match &j["motions"] {
        serde_json::Value::Null => None,
        serde_json::Value::String(f) if crate::modfs::is_file(&dir.join(f)) => Some(f.clone()),
        v => return Err(err(format!("`motions` {v} must name a .glb in the mod folder"))),
    };
    let ai_row = num(&j["ai_row"], "ai_row", 13)?;
    if !j["disc_racket"].is_null() {
        num(&j["disc_racket"], "disc_racket", 13)?;
    }
    if let Some(c) = costume_of
        && (donor != c || base != c || ai_row != c || !overrides.is_empty())
    {
        return Err(err(format!("`costume_of` {c} needs `donor`, `params.base` and `ai_row` {c} and no `params.override`")));
    }
    Ok(Mod {
        skeleton: None,
        costume_of,
        motions,
        dir: dir.to_path_buf(),
        id: text("id")?,
        name: text("name")?,
        source: game(j["source"].as_str().unwrap_or("")).chars().take(64).collect(),
        costumes,
        donor,
        hand,
        base,
        overrides,
        ai_row,
        texture_face,
        no_face: j["face"] == "none",
        voice,
    })
}

/// A `source` up to its platform tag: "Hot Shots Golf Fore! (PS2) pc03" → "Hot Shots Golf Fore!".
fn game(source: &str) -> String {
    source.split(" (").next().unwrap_or("").trim().to_string()
}

/// B380: the first Open Tee's characters are dropped (Open Tee 2 has all of them, in better versions, and more):
/// the roster, the Mods pages and the store's listing all leave them out. `source` raw or already cut by [`game`].
pub fn retired(id: &str, source: &str) -> bool {
    id.starts_with("opentee_") || game(source) == "Hot Shots Golf: Open Tee"
}

/// What a mod is (`mod.json` `kind`, M8f1; the mods repo's `standard/HST-COURT-UMPIRE-STANDARD.md` §1): absent or
/// `character` a player, `court` a venue, `umpire` a chair umpire; any other value a kind this build doesn't know
/// yet (skipped quietly, not broken).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Character,
    Court,
    Umpire,
    Other(String),
}

impl Kind {
    pub fn of(manifest: &serde_json::Value) -> Kind {
        match manifest.get("kind") {
            None | Some(serde_json::Value::Null) => Kind::Character,
            Some(k) => match k.as_str() {
                Some("character") => Kind::Character,
                Some("court") => Kind::Court,
                Some("umpire") => Kind::Umpire,
                _ => Kind::Other(k.to_string().chars().take(64).collect()),
            },
        }
    }

    /// The kind of the mod in `dir`; a missing or unreadable `mod.json` counts as a character (so `read` names
    /// what's wrong with it).
    pub fn of_dir(dir: &Path) -> Kind {
        crate::modfs::read_to_string(&dir.join("mod.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).map_or(Kind::Character, |j| Kind::of(&j))
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Kind::Character => f.write_str("`character`"),
            Kind::Court => f.write_str("`court`"),
            Kind::Umpire => f.write_str("`umpire`"),
            Kind::Other(k) => f.write_str(k),
        }
    }
}

/// Every mod under `root` (the select's custom roster, sorted by folder); a broken one is left out with its
/// `<path>: <reason>` logged.
/// A folder without a `mod.json` is searched one level down (a pack, e.g. `mods/hst-mods` → the mods repo's `out/mods`);
/// what has none there either (`texture-replacements`, `textures-src`) is skipped quietly, and so are mods of
/// another `Kind` (courts, umpires), and the [`dropped`] ones.
pub fn list(root: &Path) -> Vec<Mod> {
    dirs(root).iter().filter(|d| Kind::of_dir(d) == Kind::Character).filter_map(|d| read(d).map_err(|e| warn!("{e}")).ok()).filter(|m| !dropped(&m.id) && !retired(&m.id, &m.source)).collect()
}

/// Every folder under `root` with a `mod.json`, any kind, sorted (`list`'s walk: packs one level down, dot folders
/// left out).
pub fn dirs(root: &Path) -> Vec<PathBuf> {
    let sub = crate::modfs::read_dir;
    // a folder without mod.json is a pack of mods (e.g. a link to the mods repo's out/mods); a dot folder is an
    // install still being unpacked (`modfs::install_with`'s stage, B339: one may be while another lists)
    let mut dirs: Vec<PathBuf> =
        sub(root).into_iter().filter(|d| !d.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))).flat_map(|d| if crate::modfs::is_file(&d.join("mod.json")) { vec![d] } else { sub(&d) }).filter(|d| crate::modfs::is_file(&d.join("mod.json"))).collect();
    dirs.sort();
    dirs
}

/// B381: mods left off the custom roster and the store even when installed or uploaded — HST's own chair umpires
/// Chika, Suzuki, Lily and Anna, whose stood-up meshes look wrong as players (Robot Tennis stays).
const DROPPED: [&str; 4] = ["hst_umpire_00_chika", "hst_umpire_01_suzuki", "hst_umpire_02_lily", "hst_umpire_04_anna"];

/// Whether the mod `id` is one of the [`DROPPED`] ones.
pub fn dropped(id: &str) -> bool {
    DROPPED.contains(&id)
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

/// B275: one `params.override` a mod may have — `read` and a peer's `mods_online::Wire::check` both ask this, so a
/// mod that loads here is never refused online. A TParam.csv column and a cell up to 64 characters (TParam has text
/// cells too, e.g. `タイプ` = `オール`); no comma or control character, as the row is re-joined with commas for parsing.
pub fn override_ok(k: &str, v: &str) -> Result<(), String> {
    if column(k).is_none() {
        return Err(format!("params.override `{k}` is not a TParam.csv column"));
    }
    if v.chars().count() > 64 || v.chars().any(|c| c == ',' || c.is_control()) {
        return Err(format!("params.override `{k}` = {v:?} must be up to 64 characters, no commas"));
    }
    Ok(())
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
pub(crate) fn open(path: &Path) -> Result<(gltf::Document, Vec<u8>), String> {
    // ponytail: glTF's own validation skipped (older rerig.py output lacks POSITION min/max); §7's checks gate the file
    let g = gltf::Gltf::from_slice_without_validation(&crate::modfs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?).map_err(|e| format!("{}: {e}", path.display()))?;
    if g.buffers().any(|b| !matches!(b.source(), gltf::buffer::Source::Bin)) {
        return Err(format!("{}: buffers must be embedded (.glb)", path.display()));
    }
    let blob = g.blob.unwrap_or_default();
    sound(&g.document, &blob).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((g.document, blob))
}

/// A `.glb`'s structure, checked before anything reads it (M16: a mod is hostile input until it passes): glTF's own
/// validation (indices in range, enums; POSITION's min/max may be missing), images embedded, every accessor dense,
/// inside its view and the view inside the binary chunk, of the type its reader takes, the node tree a tree no deeper
/// than 1024, each skin's inverse binds one per joint, and each primitive's attributes as long as its POSITION, its
/// indices and joints in range.
fn sound(doc: &gltf::Document, blob: &[u8]) -> Result<(), String> {
    use gltf::accessor::{DataType as D, Dimensions as N};
    use gltf::json::validation::Validate;
    let root = doc.as_json();
    // gltf-json's validation indexes POSITION's accessor unchecked (it would panic)
    let pos = gltf::json::validation::Checked::Valid(gltf::Semantic::Positions);
    if root.meshes.iter().flat_map(|m| &m.primitives).any(|p| p.attributes.get(&pos).is_some_and(|a| a.value() >= root.accessors.len())) {
        return Err("a POSITION accessor that isn't there".into());
    }
    // nor does it check an animation channel's target node
    if root.animations.iter().flat_map(|a| &a.channels).any(|c| c.target.node.value() >= root.nodes.len()) {
        return Err("an animation of a node that isn't there".into());
    }
    let mut bad = None;
    root.validate(root, gltf::json::Path::new, &mut |path, e| {
        let p = path().to_string();
        if !(p.ends_with(".min") || p.ends_with(".max")) {
            bad.get_or_insert(format!("not valid glTF: {p}: {e}"));
        }
    });
    if let Some(e) = bad {
        return Err(e);
    }
    if doc.images().any(|i| !matches!(i.source(), gltf::image::Source::View { .. })) {
        return Err("images must be embedded".into());
    }
    for a in doc.accessors() {
        let (Some(v), None) = (a.view(), a.sparse()) else { return Err(format!("accessor {} must be dense, in a buffer view", a.index())) };
        let stride = v.stride().unwrap_or(a.size());
        let end = (a.count().checked_sub(1)).and_then(|c| c.checked_mul(stride)).and_then(|n| n.checked_add(a.offset() + a.size()));
        let view_end = v.offset().checked_add(v.length());
        if stride < a.size() || end.is_none_or(|e| e > v.length()) || view_end.is_none_or(|e| e > blob.len()) {
            return Err(format!("accessor {} is outside its data", a.index()));
        }
    }
    let is = |a: &gltf::Accessor, dims: &[N], types: &[D]| -> Result<(), String> {
        if dims.contains(&a.dimensions()) && types.contains(&a.data_type()) { Ok(()) } else { Err(format!("accessor {} has the wrong type", a.index())) }
    };
    let ints = [D::U8, D::U16];
    let unorm = [D::F32, D::U8, D::U16];
    let n_nodes = doc.nodes().len();
    let mut parent = vec![None; n_nodes];
    for n in doc.nodes() {
        for c in n.children() {
            if parent[c.index()].replace(n.index()).is_some() {
                return Err(format!("node {} has two parents", c.index()));
            }
        }
    }
    for i in 0..n_nodes {
        let (mut p, mut depth) = (parent[i], 0);
        while let Some(q) = p {
            depth += 1;
            if depth > 1024 || q == i {
                return Err("the node tree has a loop or is too deep".into());
            }
            p = parent[q];
        }
    }
    let joints = doc.skins().map(|s| s.joints().len()).max().unwrap_or(0);
    for s in doc.skins() {
        if s.joints().len() == 0 || s.joints().len() > u16::MAX as usize {
            return Err("a skin with no or too many joints".into());
        }
        if let Some(a) = s.inverse_bind_matrices() {
            is(&a, &[N::Mat4], &[D::F32])?;
            if a.count() != s.joints().len() {
                return Err("a skin's inverse binds aren't one per joint".into());
            }
        }
    }
    for mesh in doc.meshes() {
        for p in mesh.primitives() {
            let pos = p.get(&gltf::Semantic::Positions).ok_or("a primitive without POSITION")?;
            let n = pos.count();
            for (sem, a) in p.attributes() {
                use gltf::Semantic as S;
                match sem {
                    S::Positions | S::Normals => is(&a, &[N::Vec3], &[D::F32])?,
                    S::Tangents => is(&a, &[N::Vec4], &[D::F32])?,
                    S::TexCoords(_) => is(&a, &[N::Vec2], &unorm)?,
                    S::Colors(_) => is(&a, &[N::Vec3, N::Vec4], &unorm)?,
                    S::Joints(_) => is(&a, &[N::Vec4], &ints)?,
                    S::Weights(_) => is(&a, &[N::Vec4], &unorm)?,
                    S::Extras(_) => is(&a, &[N::Vec2], &[D::F32])?,
                }
                if a.count() != n {
                    return Err(format!("accessor {} isn't one per vertex", a.index()));
                }
            }
            for t in p.morph_targets() {
                for a in [t.positions(), t.normals(), t.tangents()].into_iter().flatten() {
                    is(&a, &[N::Vec3], &[D::F32])?;
                    if a.count() != n {
                        return Err(format!("accessor {} isn't one per vertex", a.index()));
                    }
                }
            }
            let r = p.reader(|_| Some(blob));
            if let Some(a) = p.indices() {
                is(&a, &[N::Scalar], &[D::U8, D::U16, D::U32])?;
            }
            if r.read_indices().is_some_and(|i| i.into_u32().any(|i| i as usize >= n)) {
                return Err("a primitive's indices go past its vertices".into());
            }
            if p.get(&gltf::Semantic::Joints(0)).is_some() && r.read_joints(0).is_some_and(|j| j.into_u16().flatten().any(|j| j as usize >= joints.max(1))) {
                return Err("a vertex rides a joint the skin doesn't have".into());
            }
        }
    }
    for anim in doc.animations() {
        for c in anim.channels() {
            let (i, o) = (c.sampler().input(), c.sampler().output());
            is(&i, &[N::Scalar], &[D::F32])?;
            use gltf::animation::Property as P;
            match c.target().property() {
                P::Translation | P::Scale => is(&o, &[N::Vec3], &[D::F32])?,
                P::Rotation => is(&o, &[N::Vec4], &[D::F32, D::I8, D::U8, D::I16, D::U16])?,
                P::MorphTargetWeights => is(&o, &[N::Scalar], &[D::F32, D::I8, D::U8, D::I16, D::U16])?,
            }
        }
    }
    Ok(())
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

/// §7's checks (HST-MODS `tools/rerig/check.py`): the failures as `Err`, else the warnings.
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

/// The upscaled-image file name of a `.glb` image: its name, else `image<index>` (same as `upscale_mods.py`).
fn image_file(im: &gltf::Image) -> String {
    format!("{}.png", im.name().map_or_else(|| format!("image{}", im.index()), str::to_string))
}

/// `textures/upscaled/<glb stem>/` of a mod's `.glb`, when the "upscaled textures" setting is on.
fn upscaled_dir(m: &Mod, glb: &Path) -> Option<PathBuf> {
    crate::textures::on().then(|| m.dir.join("textures/upscaled").join(glb.file_stem().unwrap_or_default()))
}

/// The `.glb`'s materials as a disc MTL would give them: an unlit `StandardMaterial` and its GS draws. Images
/// with a PNG of the same name in `upscaled` are drawn from that instead.
#[allow(clippy::type_complexity)]
fn materials_of(
    doc: &gltf::Document,
    blob: &[u8],
    upscaled: Option<&Path>,
    images: &mut impl Store<Image>,
    materials: &mut impl Store<StandardMaterial>,
    gs: &mut HashMap<AssetId<StandardMaterial>, Vec<crate::gs::GsMaterial>>,
) -> Result<Vec<Handle<StandardMaterial>>, String> {
    let sampler = || ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: ImageAddressMode::Repeat, address_mode_v: ImageAddressMode::Repeat, ..ImageSamplerDescriptor::linear() });
    let mut tex = Vec::new();
    for im in doc.images() {
        let gltf::image::Source::View { view, mime_type } = im.source() else { return Err("images must be embedded".into()) };
        let bytes = blob.get(view.offset()..view.offset() + view.length()).ok_or("image outside the buffer")?;
        let up = upscaled.filter(|_| crate::modfs::plain(&image_file(&im))).and_then(|d| crate::modfs::read(&d.join(image_file(&im))).ok());
        let (bytes, ty) = match &up {
            Some(b) => (&b[..], ImageType::Extension("png")),
            None => (bytes, ImageType::MimeType(mime_type)),
        };
        let img = Image::from_buffer(bytes, ty, CompressedImageFormats::NONE, true, sampler(), RenderAssetUsages::RENDER_WORLD).map_err(|e| format!("image {}: {e}", im.index()))?;
        let mut raw = img.clone();
        raw.texture_descriptor.format = bevy::render::render_resource::TextureFormat::Rgba8Unorm;
        tex.push((images.add(img), images.add(raw)));
    }
    let mut out = Vec::new();
    for m in doc.materials() {
        let t = m.pbr_metallic_roughness().base_color_texture().map(|i| i.texture().source().index());
        let color = m.pbr_metallic_roughness().base_color_factor();
        let mask = m.alpha_mode() == gltf::material::AlphaMode::Mask;
        let blend = m.alpha_mode() == gltf::material::AlphaMode::Blend;
        let h = materials.add(StandardMaterial {
            base_color: Color::linear_rgba(color[0], color[1], color[2], color[3]),
            base_color_texture: t.and_then(|t| tex.get(t)).map(|t| t.0.clone()),
            unlit: true,
            double_sided: true,
            cull_mode: None,
            // texture alpha is not coverage on HST bodies; `MASK` marks the real cut-outs (§2)
            alpha_mode: if mask { AlphaMode::Mask(m.alpha_cutoff().unwrap_or(0.5)) } else if blend { AlphaMode::Blend } else { AlphaMode::Opaque },
            ..default()
        });
        // the MTL header a disc material would have: its lighting words from `extras.hst_mtl` (0, matte, when
        // absent), TEST mode 10 (A ≥ 0x40) for MASK, 25 (two passes split at A 0x70, blended) for BLEND; drawn
        // fogged like every disc character batch
        let extras: serde_json::Value = m.extras().as_ref().and_then(|r| serde_json::from_str(r.get()).ok()).unwrap_or_default();
        let word = |k: &str| (extras["hst_mtl"][k].as_f64().unwrap_or(0.0) as f32).to_le_bytes();
        let mut header = [0; 0x30];
        header[0x10..0x14].copy_from_slice(&word("shininess"));
        header[0x14..0x18].copy_from_slice(&word("highlight"));
        header[0x1e..0x20].copy_from_slice(&(if mask { 10i16 } else if blend { 25 } else { 0 }).to_le_bytes());
        let mtl = mtl::Material { texture: t, color: color.map(|c| c.min(1.0)), attributes: None, two_sided: m.double_sided(), name: m.name().unwrap_or("").to_string(), header };
        let prim = 0x20 | if t.is_some() { 0x10 } else { 0 } | if blend { 0x40 } else { 0 };
        gs.insert(h.id(), crate::gs::GsMaterial::for_batch(&mtl, prim, t.and_then(|t| tex.get(t)).map(|t| t.1.clone())));
        out.push(h);
    }
    // glTF's default material
    out.push(materials.add(StandardMaterial { unlit: true, double_sided: true, cull_mode: None, ..default() }));
    Ok(out)
}

/// A texture face (§4b) from `dir/face.json`: each channel with a texture becomes a morph weight (after the model's
/// own targets, so the donor's `.MOR` tracks drive it by name) and the listed glTF materials start on the neutral face.
fn texture_face(
    dir: &Path,
    doc: &gltf::Document,
    handles: &[Handle<StandardMaterial>],
    images: &mut impl Store<Image>,
    materials: &mut impl Store<StandardMaterial>,
    gs: &mut HashMap<AssetId<StandardMaterial>, Vec<crate::gs::GsMaterial>>,
    names: &mut Vec<String>,
) -> Result<character::TextureFace, String> {
    let at = dir.join("face.json");
    let err = |m: String| format!("{}: {m}", at.display());
    let text = crate::modfs::read_to_string(&at).map_err(|e| err(e.to_string()))?;
    let j: serde_json::Value = serde_json::from_str(&text).map_err(|e| err(format!("not JSON: {e}")))?;
    // one image pair per file (an emotion's eye and mouth channels share one)
    let mut loaded: HashMap<String, (Handle<Image>, Handle<Image>)> = HashMap::new();
    let mut image = |p: &str| -> Result<(Handle<Image>, Handle<Image>), String> {
        if let Some(t) = loaded.get(p) {
            return Ok(t.clone());
        }
        if !crate::modfs::plain(p) {
            return Err(err(format!("`{p}` must be a path inside the mod folder")));
        }
        let bytes = crate::modfs::read(&dir.join(p)).map_err(|e| err(format!("`{p}`: {e}")))?;
        let ext = Path::new(p).extension().and_then(|e| e.to_str()).unwrap_or("png");
        let sampler = ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: ImageAddressMode::Repeat, address_mode_v: ImageAddressMode::Repeat, ..ImageSamplerDescriptor::linear() });
        let img = Image::from_buffer(&bytes, ImageType::Extension(ext), CompressedImageFormats::NONE, true, sampler, RenderAssetUsages::RENDER_WORLD).map_err(|e| err(format!("`{p}`: {e}")))?;
        let mut raw = img.clone();
        raw.texture_descriptor.format = bevy::render::render_resource::TextureFormat::Rgba8Unorm;
        let t = (images.add(img), images.add(raw));
        loaded.insert(p.to_string(), t.clone());
        Ok(t)
    };
    let neutral = image(j["neutral"].as_str().ok_or_else(|| err("`neutral` must be an image path".into()))?)?;
    let mut channels = Vec::new();
    for (k, v) in j["channels"].as_object().into_iter().flatten() {
        // null: the channel has no face of its own (a mask), it stays neutral
        let Some(p) = v.as_str() else { continue };
        let t = names.iter().position(|n| n == k).unwrap_or_else(|| {
            names.push(k.clone());
            names.len() - 1
        });
        channels.push((t, image(p)?));
    }
    let wanted: Vec<&str> = j["materials"].as_array().into_iter().flatten().filter_map(|m| m.as_str()).collect();
    if wanted.is_empty() {
        return Err(err("`materials` must list the face's glTF material names".into()));
    }
    for name in wanted {
        let i = doc.materials().position(|m| m.name() == Some(name)).ok_or_else(|| err(format!("no material `{name}` in the costume")))?;
        if let Some(m) = materials.get_mut(&handles[i]) {
            m.base_color_texture = Some(neutral.0.clone());
        }
        for d in gs.get_mut(&handles[i].id()).into_iter().flatten() {
            d.texture = Some(neutral.1.clone());
        }
    }
    Ok(character::TextureFace { neutral, channels })
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
    /// `_HST_NOISE`: per vertex its noise deformer and share (0: not moved).
    noise: Vec<[f32; 2]>,
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
        noise: p.get(&gltf::Semantic::Extras("HST_NOISE".into())).and_then(|a| gltf::accessor::Iter::<[f32; 2]>::new(a, |_| Some(blob))).map_or_else(Vec::new, |i| i.collect()),
        morph: r.read_morph_targets().zip(names).map(|((p, ..), name)| (name.clone(), p.map_or_else(|| vec![[0.0; 3]; n], |p| p.collect()))).collect(),
        pos,
    })
}

/// The skin's joints in game space (below `game_space`): each joint's world there, its parent the nearest joint above it.
fn joints_of(doc: &gltf::Document, blob: &[u8]) -> Vec<Joint> {
    let Some(skin) = doc.skins().next() else { return Vec::new() };
    let nodes: Vec<_> = skin.joints().collect();
    let index: HashMap<usize, usize> = nodes.iter().enumerate().map(|(k, n)| (n.index(), k)).collect();
    let mut up = vec![None; doc.nodes().len()];
    for n in doc.nodes() {
        n.children().for_each(|c| up[c.index()] = Some(n.index()));
    }
    let w = worlds(&doc);
    let space = doc.default_scene().or_else(|| doc.scenes().next()).and_then(|s| s.nodes().next()).map_or(Mat4::IDENTITY, |r| w[r.index()].inverse());
    let ibm: Vec<Mat4> = skin.reader(|_| Some(blob)).read_inverse_bind_matrices().map(|i| i.map(|m| Mat4::from_cols_array_2d(&m)).collect()).unwrap_or_else(|| vec![Mat4::IDENTITY; nodes.len()]);
    nodes
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
        .collect()
}

fn skeleton_of(joints: &[Joint]) -> Skeleton {
    Skeleton {
        names: joints.iter().map(|j| j.name.clone()).collect(),
        parent: joints.iter().map(|j| j.parent).collect(),
        rest: joints.iter().map(|j| j.rest.to_matrix().to_cols_array_2d()).collect(),
    }
}

/// The skeleton costume `costume` of `m` plays on, as [`load`] builds it (the online lobby sends it: M2i).
pub fn skeleton(m: &Mod, costume: usize) -> Result<Skeleton, String> {
    let file = m.dir.join(m.costumes.get(costume).ok_or_else(|| format!("{}: no costume {costume}", m.id))?);
    let (doc, blob) = open(&file)?;
    conform(&doc, &blob, false).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(skeleton_of(&joints_of(&doc, &blob)))
}

/// §7's checks on every costume of `m` (a mod that came over the network, before it's used).
pub fn check(m: &Mod) -> Result<(), String> {
    m.costumes.iter().try_for_each(|c| {
        let (doc, blob) = open(&m.dir.join(c))?;
        conform(&doc, &blob, false).map(drop).map_err(|e| format!("{c}: {e}"))
    })
}

/// Loads costume `costume` of mod `m` (motions, faces and arm table from the donor on the disc).
pub fn load(
    iso: &mut Iso,
    m: &Mod,
    costume: usize,
    meshes: &mut impl Store<Mesh>,
    materials: &mut impl Store<StandardMaterial>,
    images: &mut impl Store<Image>,
    bindposes: &mut impl Store<SkinnedMeshInverseBindposes>,
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
    let w = worlds(&doc);
    let space = doc.default_scene().or_else(|| doc.scenes().next()).and_then(|s| s.nodes().next()).map_or(Mat4::IDENTITY, |r| w[r.index()].inverse());
    let joints = joints_of(&doc, &blob);
    let skeleton = skeleton_of(&joints);

    let mut gs = HashMap::new();
    let handles = materials_of(&doc, &blob, upscaled_dir(m, &file).as_deref(), images, materials, &mut gs).map_err(ctx)?;
    // the skinned parts, one per primitive, every morphed one carrying all the model's targets
    let mut names: Vec<String> = Vec::new();
    for mesh in doc.meshes() {
        names.extend(target_names(&mesh).into_iter().filter(|t| !names.contains(t)).collect::<Vec<_>>());
    }
    let texture_face = if m.texture_face { Some(texture_face(&m.dir, &doc, &handles, images, materials, &mut gs, &mut names)?) } else { None };
    let mut parts = Vec::new();
    let mut part_nodes = Vec::new();
    // §4c: the eye materials the donor's `.UVA` eye track moves
    let eyes = eye_materials(&m.dir);
    for e in eyes.iter().filter(|e| !doc.materials().any(|x| x.name() == Some(e.as_str()))) {
        warn!("{}: `eyes` names no material `{e}` in the costume", file.display());
    }
    let mut deformers = Vec::new();
    let mut swaying = Vec::new();
    // a mesh without a skin rides the nearest joint at or above its node (Get a Grip's heads and faces), whole
    let rides = |n: &gltf::Node| {
        let mut p = Some(n.index());
        while let Some(i) = p.filter(|i| !index.contains_key(i)) {
            p = up[i];
        }
        p.map(|i| index[&i])
    };
    for (n, rigid) in doc.nodes().filter_map(|n| match n.skin() {
        Some(_) => Some((n, None)),
        None => n.mesh().and(rides(&n)).map(|j| (n, Some(j))),
    }) {
        let mesh = n.mesh().unwrap();
        let tn = target_names(&mesh);
        // the mesh's noise deformers (`extras.noise`, a disc `.NOI`'s), numbered across meshes
        let first = deformers.len();
        deformers.extend(noise_deformers(&mesh));
        for p in mesh.primitives() {
            let mut v = prim(&p, &blob, &tn).map_err(ctx)?;
            if let Some(j) = rigid {
                // into game space at bind, all on joint j
                let to = space * w[n.index()];
                let nm = Mat3::from_mat4(to).inverse().transpose();
                v.pos.iter_mut().for_each(|q| *q = to.transform_point3(Vec3::from(*q)).into());
                v.nrm.iter_mut().for_each(|q| *q = (nm * Vec3::from(*q)).normalize_or_zero().into());
                v.morph.iter_mut().flat_map(|t| t.1.iter_mut()).for_each(|d| *d = to.transform_vector3(Vec3::from(*d)).into());
                v.joints.fill([j as u16, 0, 0, 0]);
                v.weights.fill([1.0, 0.0, 0.0, 0.0]);
            }
            let count = v.pos.len();
            // the joints the part's vertices ride (a disc packet's palette): a `.UVA` track on one of them moves it
            let mut nodes: Vec<usize> = v.joints.iter().zip(&v.weights).flat_map(|(j, w)| (0..4).filter(|&k| w[k] > 0.0).map(|k| j[k] as usize)).collect();
            let moved = moved(&v, first, deformers.len(), &joints);
            // VU1's per-bone normals from one glTF normal: the first joint's share in the normal slot, the rest in the
            // tangent slot, which gs.wgsl turns by the other joints' weight blend
            // a swaying part stays readable: each rig sways its own copy (`noise`)
            let usage = if moved.is_empty() { crate::cull::usage(RenderAssetUsages::RENDER_WORLD) } else { RenderAssetUsages::default() };
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, usage)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, v.pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, v.nrm.iter().zip(&v.weights).map(|(n, w)| Vec3::from(*n) * w[0]).map(<[f32; 3]>::from).collect::<Vec<_>>())
                .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, v.nrm.iter().zip(&v.weights).map(|(n, w)| (Vec3::from(*n) * (1.0 - w[0])).extend(0.0).to_array()).collect::<Vec<_>>())
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, v.uv)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, v.col)
                .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(v.joints))
                .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, v.weights)
                .with_inserted_indices(Indices::U32(v.idx));
            // B364: fewer triangles at Graphics → Mesh below 100 %, the morphing (face) vertices kept
            let moving: Vec<bool> = (0..count).map(|i| v.morph.iter().any(|t| t.1.get(i).is_some_and(|d| *d != [0.0; 3]))).collect();
            crate::lod::detail(&mut mesh, &moving);
            let morphed = v.morph.iter().any(|t| t.1.iter().any(|d| *d != [0.0; 3]));
            if morphed {
                let zero = vec![[0.0; 3]; count];
                let attrs = names.iter().flat_map(|t| v.morph.iter().find(|m| m.0 == *t).map_or(&zero, |m| &m.1).iter().map(|d| bevy::mesh::morph::MorphAttributes::new(Vec3::from(*d), Vec3::ZERO, Vec3::ZERO)).collect::<Vec<_>>());
                mesh.set_morph_targets(attrs.collect());
            }
            crate::cull::bound(&mut mesh, v.morph.iter().map(|t| &t.1[..]));
            let mat = handles[p.material().index().unwrap_or(handles.len() - 1)].clone();
            if !moved.is_empty() {
                swaying.push((parts.len(), moved));
            }
            if p.material().name().is_some_and(|n| eyes.iter().any(|e| e == n)) {
                nodes.push(character::EYES);
            }
            nodes.sort();
            nodes.dedup();
            part_nodes.push((nodes, false));
            parts.push((meshes.add(mesh), mat, morphed));
        }
    }

    // the racket: racket.glb in the Racket joint's space, else disc character `disc_racket`'s (M1r), else the donor's
    let racket = match m.dir.join("racket.glb") {
        f if crate::modfs::is_file(&f) => {
            let (rdoc, rblob) = open(&f)?;
            let rh = materials_of(&rdoc, &rblob, upscaled_dir(m, &f).as_deref(), images, materials, &mut gs)?;
            let mut out = Vec::new();
            for mesh in rdoc.meshes() {
                for p in mesh.primitives() {
                    let v = prim(&p, &rblob, &[]).map_err(|e| format!("{}: {e}", f.display()))?;
                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, v.pos)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, v.nrm)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, v.uv)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, v.col)
                        .with_inserted_indices(Indices::U32(v.idx));
                    crate::lod::detail(&mut mesh, &[]);
                    out.push((meshes.add(mesh), rh[p.material().index().unwrap_or(rh.len() - 1)].clone()));
                }
            }
            out
        }
        _ => {
            let n = disc_racket(&m.dir).unwrap_or(m.donor);
            let data = iso.read(&format!("PC/PC{n:02}C00.XB")).map_err(|e| e.to_string())?;
            character::disc_racket(&Archive::parse(&data).map_err(|e| e.0)?, n, meshes, materials, images, &mut gs)?
        }
    };

    // the donor's motions; its face tracks (`face\x01joy_eye`) bind by the name after the object prefix
    let Motions { motions, paths, pelvis, arm, mut faces, stance_ball } = character::disc_motions(iso, m.donor, &skeleton, |t| names.iter().position(|n| Some(n.as_str()) == t.rsplit('\x01').next()))?;
    // the donor's eye tracks on nodes the costume lacks drive its §4c eye materials (those the donor's own model
    // binds: a track naming no node of it plays on neither)
    if !eyes.is_empty() {
        let donor = character::disc_nodes(iso, m.donor)?;
        for f in faces.values_mut() {
            f.uv.extend(f.uv_unbound.drain(..).filter(|t| donor.contains(&t.0)).map(|(_, ticks, values)| (character::EYES, ticks, values)));
        }
    }
    let visual = match &m.motions {
        Some(f) => own_motions(&m.dir.join(f), &skeleton, &motions)?,
        None => HashMap::new(),
    };
    let binds = bindposes.add(SkinnedMeshInverseBindposes::from(joints.iter().map(|j| j.inverse_bind).collect::<Vec<_>>()));
    let springs = crate::spring::masses(&nodes, &joints);
    let noise = (!swaying.is_empty() || !springs.is_empty()).then(|| std::sync::Arc::new(crate::noise::Costume { deformers, parts: swaying, springs }));
    Ok(CharacterData { joints, parts, racket, motions, paths, binds, pelvis, arm, morph_names: names, faces, part_nodes, stance_ball, skeleton, noise, gs, texture_face, visual })
}

/// `mod.json`'s `disc_racket` (§6, M1r): the disc character (0..13) whose racket it holds when it ships no
/// `racket.glb` (`read` checked the range). Looks only: gameplay never reads the racket mesh.
fn disc_racket(dir: &Path) -> Option<usize> {
    let j: serde_json::Value = crate::modfs::read_to_string(&dir.join("mod.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    j["disc_racket"].as_u64().filter(|&n| n <= 13).map(|n| n as usize)
}

/// `mod.json`'s `eyes` (§4c): the glTF materials the donor's `.UVA` eye track moves (`read` checked the list).
fn eye_materials(dir: &Path) -> Vec<String> {
    let j: serde_json::Value = crate::modfs::read_to_string(&dir.join("mod.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    j["eyes"].as_array().into_iter().flatten().filter_map(|m| m.as_str().map(str::to_string)).collect()
}

/// A mod's own motions (§6 `motions`): each animation, named by its motion (`re_gu`, `sh_f_t`: the disc name
/// without `_pc%02d`), keys the skeleton's joints by name with their local rotation and translation in game space.
/// Its key times are seconds at 60 Hz (the source's own speed); it may run past the donor clip (drawn on its own
/// clock, `Motion::drawn`) but not stop short of it. Gameplay keeps reading the donor's clip.
fn own_motions(path: &Path, skeleton: &Skeleton, donor: &HashMap<usize, Clip>) -> Result<HashMap<usize, Clip>, String> {
    use hst_data::ani::{Anim, MOTIONS, Track};
    use gltf::animation::util::ReadOutputs;
    let (doc, blob) = open(path)?;
    let ctx = |e: String| format!("{}: {e}", path.display());
    let mut out = HashMap::new();
    for a in doc.animations() {
        let name = a.name().unwrap_or("");
        let id = MOTIONS.iter().position(|m| m.replace("_pc%02d", "") == name).ok_or_else(|| ctx(format!("animation `{name}` is not a motion name")))?;
        let d = donor.get(&id).ok_or_else(|| ctx(format!("`{name}`: the donor has no clip {id:#x} to time it by")))?;
        let mut keys: Vec<(&gltf::animation::Channel, Vec<f32>)> = Vec::new();
        let channels: Vec<_> = a.channels().collect();
        for c in &channels {
            keys.push((c, c.reader(|_| Some(&blob[..])).read_inputs().ok_or_else(|| ctx(format!("`{name}`: a channel has no times")))?.collect()));
        }
        let mut tracks: Vec<Track> = Vec::new();
        for (c, times) in keys {
            let joint = c.target().node().name().unwrap_or("").replace(' ', "");
            // one motions file serves every costume, and costumes' extra joints differ: a joint this one lacks is skipped
            // (B99: costumes 4+ of many Fore! mods failed to load over it)
            if !skeleton.names.contains(&joint) {
                continue;
            }
            let tick = |t: f32| (t * 60.0).round() as i32 * d.ticks_per_frame as i32;
            let values: Vec<[f32; 4]> = match c.reader(|_| Some(&blob[..])).read_outputs() {
                // a rotation key is the conjugate of the local rotation (`ani`)
                Some(ReadOutputs::Rotations(r)) => r.into_f32().map(|[x, y, z, w]| [-x, -y, -z, w]).collect(),
                Some(ReadOutputs::Translations(t)) => t.map(|[x, y, z]| [x, y, z, 1.0]).collect(),
                _ => continue,
            };
            // hostile input (M16): the game's squad and sinf take unit quaternions, ticks fit an i32
            let unit = |v: &[f32; 4]| (v.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 0.01;
            let near = |v: &[f32; 4]| v.iter().all(|x| x.abs() < 1000.0);
            let rot = matches!(c.target().property(), gltf::animation::Property::Rotation);
            if !times.iter().all(|t| (0.0..=3600.0).contains(t)) || !values.iter().all(|v| if rot { unit(v) } else { near(v) }) {
                return Err(ctx(format!("`{name}`: a key out of range (times 0..=3600 s, rotations unit, positions within 1000 m)")));
            }
            let mut list: Vec<(i32, [f32; 4])> = Vec::new();
            for (t, v) in times.iter().zip(values) {
                match list.last_mut() {
                    Some(l) if l.0 == tick(*t) => l.1 = v,
                    _ => list.push((tick(*t), v)),
                }
            }
            let k = tracks.iter().position(|t| t.name == joint).unwrap_or_else(|| {
                tracks.push(Track { name: joint.clone(), length: 0.0, rotation: vec![], position: vec![], parent: None });
                tracks.len() - 1
            });
            match c.target().property() {
                gltf::animation::Property::Rotation => tracks[k].rotation = list,
                _ => tracks[k].position = list,
            }
        }
        let clip = Clip::new(skeleton, &Anim { ticks_per_frame: d.ticks_per_frame as i32, tracks });
        if clip.length < d.length {
            return Err(ctx(format!("`{name}` spans {} frames, short of the donor's {}", clip.length, d.length)));
        }
        out.insert(id, clip);
    }
    Ok(out)
}

/// A mesh's `extras.noise` deformers (period, rate, amplitudes), as a `.NOI` gives them.
fn noise_deformers(mesh: &gltf::Mesh) -> Vec<hst_data::noi::Deformer> {
    let extras: serde_json::Value = mesh.extras().as_ref().and_then(|r| serde_json::from_str(r.get()).ok()).unwrap_or_default();
    let f = |d: &serde_json::Value, k: &str| d[k].as_f64().unwrap_or(0.0) as f32;
    extras["noise"].as_array().map_or_else(Vec::new, |a| {
        a.iter()
            .enumerate()
            .map(|(i, d)| hst_data::noi::Deformer { node: i, period: f(d, "period"), rate: f(d, "rate"), amp: std::array::from_fn(|k| d["amp"][k].as_f64().unwrap_or(0.0) as f32), name: d["name"].as_str().unwrap_or("").to_string() })
            .collect()
    })
}

/// A primitive's vertices its deformers (`first..end`) move: as on the disc, one position entry per influence, in its
/// joint's space times its weight (`w · inverse bind · p`), all with the vertex's share.
fn moved(v: &Prim, first: usize, end: usize, joints: &[Joint]) -> Vec<crate::noise::Moved> {
    v.noise
        .iter()
        .enumerate()
        .filter_map(|(i, &[d, share])| {
            let d = first + d as usize;
            if share == 0.0 || d >= end {
                return None;
            }
            let skin: [(usize, f32); 4] = std::array::from_fn(|k| (v.joints[i][k] as usize, v.weights[i][k]));
            let entries = skin
                .iter()
                .filter(|&&(j, w)| w > 0.0 && j < joints.len())
                .map(|&(j, w)| crate::noise::Entry { deformer: d, p: (joints[j].inverse_bind.transform_point3(Vec3::from(v.pos[i])) * w).into(), weight: share, node: j })
                .collect();
            Some(crate::noise::Moved { v: i, base: v.pos[i], skin, entries })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hst_sim::pose::{model, M4};

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
    /// Test mods (local, `research/journal/2026-10-08-m1a-mod-loader`): `test_pc00` is HST's pc00 through
    /// HST-MODS `tools/rerig/rerig.py`, `test_broken` the same with `Bip01LThigh` renamed, `fore_pc00_phoebe` a packaged mod.
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
        // M1d: the disc's sway (as many deformers and swaying vertices) and MTL words (the same draws, as a set)
        let sway = |c: &CharacterData| c.noise.as_ref().map(|n| (n.deformers.len(), n.parts.iter().map(|p| p.1.len()).sum::<usize>()));
        assert_eq!(sway(&c), sway(&disc));
        let draws = |c: &CharacterData| {
            let mut d: Vec<String> = c.gs.values().flatten().map(|g| format!("{:?} {} {}", g.key, g.uniform.shininess, g.uniform.highlight)).collect();
            d.sort();
            d.dedup();
            d
        };
        assert_eq!(draws(&c), draws(&disc));
        let row = tparam(&mut iso, &m).unwrap();
        assert_eq!((&row[12][..], &row[40][..], &row[57][..], &row[13][..]), ("4", "10", "170", "4"));
    }

    /// B380: Open Tee 1's characters are left out, by id or by source game; Open Tee 2's and the rest stay.
    #[test]
    fn open_tee_1_is_retired() {
        assert!(retired("opentee_pc00_mika", "Hot Shots Golf: Open Tee (PSP) pc00"));
        assert!(retired("mika_fixed", "Hot Shots Golf: Open Tee"));
        assert!(retired("opentee_pc09_roger", ""));
        assert!(!retired("opentee2_pc00_mika", "Hot Shots Golf: Open Tee 2 (PSP) pc00"));
        assert!(!retired("getagrip_pc00_emi", "Hot Shots Tennis: Get a Grip"));
        assert!(!retired("fore_pc00_phoebe", "Hot Shots Golf Fore! (PS2) pc00"));
    }

    /// B381: an installed Lily (one of the [`dropped`] chair umpires) stays off the roster, beside a mod that lists
    /// (skipped without `context/mods/test_pc00`).
    #[test]
    fn dropped_umpires_stay_off_the_roster() {
        let src = Path::new(MODS).join("test_pc00");
        if !src.is_dir() {
            return eprintln!("no context/mods/test_pc00, skipped");
        }
        let root = std::env::temp_dir().join(format!("hst-b381-{}", std::process::id()));
        for id in ["test_pc00", "hst_umpire_02_lily"] {
            let d = root.join(id);
            std::fs::create_dir_all(d.join("model")).unwrap();
            std::fs::copy(src.join("model/c00.glb"), d.join("model/c00.glb")).unwrap();
            let mut j: serde_json::Value = serde_json::from_slice(&std::fs::read(src.join("mod.json")).unwrap()).unwrap();
            j["id"] = id.into();
            std::fs::write(d.join("mod.json"), serde_json::to_vec(&j).unwrap()).unwrap();
        }
        let ids: Vec<String> = list(&root).into_iter().map(|m| m.id).collect();
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(ids, ["test_pc00"]);
    }

    /// M1f: the PSP mods' heads and faces are unskinned mesh nodes under `Bip01Head`; every costume of every Get a
    /// Grip and Open Tee 2 mod in `context/mods` carries vertices on the head joint, at rest above its neck.
    #[test]
    fn psp_mods_keep_their_heads() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        let Ok(dir) = std::fs::read_dir(MODS) else { return eprintln!("no context/mods, skipped") };
        let mut s = Stores(default(), default(), default(), default());
        let mut seen = 0;
        for e in dir.flatten().filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("getagrip_") || n.starts_with("opentee2_"))) {
            let m = read(&e.path()).unwrap();
            for k in 0..m.costumes.len() {
                let c = load(&mut iso, &m, k, &mut s.0, &mut s.1, &mut s.2, &mut s.3).unwrap();
                let (head, neck) = (c.joint("Bip01Head").unwrap(), c.joint("Bip01Neck").unwrap());
                let rest = model(&c.skeleton, &c.skeleton.rest);
                // game space is Y down: the head's vertices sit above (below in y) the neck joint
                let top = c.parts.iter().filter_map(|p| s.0.get(&p.0)).flat_map(|mesh| {
                    let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return Vec::new() };
                    let Some(VertexAttributeValues::Uint16x4(j)) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX) else { return Vec::new() };
                    pos.iter().zip(j).filter(|(_, j)| j[0] as usize == head).map(|(p, _)| p[1]).collect()
                });
                let top = top.fold(f32::INFINITY, f32::min);
                assert!(top < rest[neck][3][1], "{} costume {k}: no head (top {top}, neck {})", m.id, rest[neck][3][1]);
                seen += 1;
            }
        }
        eprintln!("{seen} PSP costumes have heads");
    }

    /// Get a Grip's Emi (a texture face, §4b): her face material starts neutral, the donor's `.MOR` tracks drive
    /// face.json's channels by name, and over the donor's faces some frames show an expression texture.
    #[test]
    fn texture_face_follows_the_donors_faces() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        if !Path::new(MODS).join("getagrip_pc00_emi").is_dir() {
            return eprintln!("no context/mods/getagrip_pc00_emi, skipped");
        }
        let mut s = Stores(default(), default(), default(), default());
        let (_, c) = load_mod(&mut iso, &mut s, "getagrip_pc00_emi").unwrap();
        let f = c.texture_face.as_ref().expect("texture face");
        assert_eq!(f.channels.len(), 9);
        assert_eq!(c.morph_names.len(), 9);
        // joy_eye and joy_mouth (weights 5, 6: channels by name) share zero_face5.png: one image
        let joy: Vec<_> = f.channels.iter().filter(|(t, _)| [5, 6].contains(t)).map(|c| c.1.0.id()).collect();
        assert!(joy.len() == 2 && joy[0] == joy[1], "{joy:?}");
        let face = c.parts.iter().filter_map(|p| s.1.get(&p.1)?.base_color_texture.clone()).filter(|t| *t == f.neutral.0).count();
        assert!(face > 0, "no part starts on the neutral face");
        assert!(c.gs.values().flatten().any(|d| d.texture.as_ref() == Some(&f.neutral.1)));
        // sample every face the way `character::animate` does; some frames must leave neutral
        let mut shown = 0;
        for face in c.faces.values() {
            for k in 0..(face.length as i32) {
                let mut w = vec![0.0; c.morph_names.len()];
                let t = hst_sim::ps2::mul(k as f32, 80.0);
                for (target, ticks, values) in &face.tracks {
                    w[*target] = hst_sim::face::sample(ticks, values, t, &mut 0, false)[0];
                }
                shown += (f.pick(&w).0 != f.neutral.0) as usize;
            }
        }
        assert!(shown > 0, "the donor's faces never show an expression");
    }

    /// A motions file keying a joint only some costumes have still loads the others (B99: Phoebe's costumes 4–7 lack
    /// `Body`, which her motions key).
    #[test]
    fn motions_skip_joints_a_costume_lacks() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/hst-mods/fore_pc00_phoebe");
        let Ok(m) = read(&dir) else { return eprintln!("no mods/hst-mods/fore_pc00_phoebe, skipped") };
        let mut s = Stores(default(), default(), default(), default());
        for k in 0..m.costumes.len() {
            let c = load(&mut iso, &m, k, &mut s.0, &mut s.1, &mut s.2, &mut s.3).unwrap_or_else(|e| panic!("costume {k}: {e}"));
            assert!(!c.visual.is_empty(), "costume {k} lost its own motions");
        }
    }

    /// An Out of Bounds mod's hair and earring chains (§3b `extras.spring`) reach the costume as spring joints. The
    /// rig whips 0.2 m sideways over 10 frames, then stands: the hair's joints swing off where they hang at rest
    /// (the pose plus gravity's sag), then settle back there; the head they hang on is never turned.
    #[test]
    fn oob_hair_swings_and_settles() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/hst-mods/oob_pc00_jasmine");
        let Ok(m) = read(&dir) else { return eprintln!("no mods/hst-mods/oob_pc00_jasmine, skipped") };
        let mut s = Stores(default(), default(), default(), default());
        let c = load(&mut iso, &m, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3).unwrap();
        // 23 records, 5 of them chain anchors (pinned)
        assert_eq!(c.noise.as_ref().map_or(0, |n| n.springs.len()), 18);
        use bevy::ecs::system::RunSystemOnce;
        let data = std::sync::Arc::new(c);
        let run = |whip: f32| {
            let mut w = World::new();
            w.insert_resource(Time::<()>::default());
            let joints: Vec<Entity> = data.joints.iter().map(|j| w.spawn(j.rest).id()).collect();
            let root = w.spawn((crate::character::Rig { data: data.clone(), joints: joints.clone() }, GlobalTransform::IDENTITY, crate::spring::Spring::default())).id();
            let mut frames = Vec::new();
            for f in 0..240 {
                for (e, j) in joints.iter().zip(&data.joints) {
                    *w.get_mut::<Transform>(*e).unwrap() = j.rest;
                }
                *w.get_mut::<GlobalTransform>(root).unwrap() = GlobalTransform::from_translation(Vec3::X * whip * f.min(10) as f32);
                w.resource_mut::<Time>().advance_by(std::time::Duration::from_micros(16_667));
                w.run_system_once(crate::spring::swing).unwrap();
                frames.push(joints.iter().map(|e| w.get::<Transform>(*e).unwrap().rotation).collect::<Vec<_>>());
            }
            frames
        };
        let (calm, whipped) = (run(0.0), run(0.02));
        let head = data.joint("Bip01Head").unwrap();
        let apart = |f: usize| calm[f].iter().zip(&whipped[f]).map(|(a, b)| a.angle_between(*b)).fold(0f32, f32::max).to_degrees();
        eprintln!("whipped vs calm, largest joint difference every 10 frames: {:?}", (0..240).step_by(10).map(|f| apart(f).round()).collect::<Vec<_>>());
        assert!((0..12).any(|f| apart(f) > 5.0), "the hair never trailed");
        assert!(apart(239) < 0.5, "the hair never settled: {}", apart(239));
        assert!(whipped.iter().all(|f| f[head] == data.joints[head].rest.rotation), "the hair turned the head");
    }

    /// A Get a Grip mod's own reactions (§6 `motions`) are drawn in place of the donor's at the source's own
    /// length (M1q: no longer squeezed into the donor's), on their own clock past the donor clip's end, while
    /// everything gameplay reads (the donor's clips, as on the disc) is what it is without them.
    #[test]
    fn own_motions_are_visual_only() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        // Fore!'s Phoebe: reactions up to 3x the donor's length
        for name in ["getagrip_pc00_emi", "fore_pc00_phoebe"] {
            own_motions_of(&mut iso, name);
        }
    }

    fn own_motions_of(iso: &mut Iso, name: &str) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/hst-mods").join(name);
        let Ok(m) = read(&dir) else { return eprintln!("no mods/hst-mods/{name}, skipped") };
        assert_eq!(m.motions.as_deref(), Some("motions.glb"));
        let mut s = Stores(default(), default(), default(), default());
        let c = load(iso, &m, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3).unwrap();
        let plain = load(iso, &Mod { motions: None, ..m.clone() }, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3).unwrap();
        assert!(plain.visual.is_empty());
        let mut ids: Vec<usize> = c.visual.keys().copied().collect();
        ids.sort();
        assert_eq!(ids, [0x2c, 0x2d, 0x2e, 0x2f]);
        for (id, clip) in &c.motions {
            let p = &plain.motions[id];
            assert_eq!(clip.length, p.length);
            for k in 0..4 {
                let t = p.length * k as f32 / 4.0;
                assert!(clip.locals(&c.skeleton, t) == p.locals(&plain.skeleton, t), "gameplay clip {id:#x} changed");
            }
        }
        assert_eq!((c.pelvis.clone(), c.paths.len()), (plain.pelvis.clone(), plain.paths.len()));
        // donor motions on a mod: the disc donor's clip lengths and timing
        let disc = character::disc_motions(iso, m.donor, &c.skeleton, |_| None).unwrap();
        for (id, clip) in &c.motions {
            assert_eq!((clip.length, clip.ticks_per_frame), (disc.motions[id].length, disc.motions[id].ticks_per_frame), "{id:#x}");
        }
        // each own clip lasts as long as its glTF (seconds at 60 Hz), at least the donor's
        let (doc, _) = open(&dir.join("motions.glb")).unwrap();
        for a in doc.animations() {
            let id = hst_data::ani::MOTIONS.iter().position(|n| n.replace("_pc%02d", "") == a.name().unwrap()).unwrap();
            let secs = a.channels().filter_map(|c| c.sampler().input().max()).filter_map(|m| m.as_array()?.first()?.as_f64()).fold(0.0, f64::max);
            assert_eq!(c.visual[&id].length, (secs * 60.0).round() as f32, "{id:#x}");
        }
        let longer = c.visual.iter().find(|(id, v)| v.length > c.motions[id].length).map(|(&id, _)| id);
        assert_eq!(longer.is_some(), name.starts_with("fore"), "Fore!'s reactions outlast the donor's, Emi's don't");
        if let Some(longer) = longer {
            let mut mo = character::Motion::default();
            mo.set(longer, 1.0, false, None, 0);
            for _ in 0..1000 {
                mo.clock.tick(c.motions[&longer].length);
                mo.drawn.tick(c.drawn(longer).unwrap().length);
            }
            assert_eq!((mo.clock.sampled, mo.drawn.sampled), (c.motions[&longer].length, c.visual[&longer].length), "each clock holds its own clip's end");
        }
        for id in ids {
            let (v, d) = (&c.visual[&id], &c.motions[&id]);
            assert!(v.length >= d.length);
            assert!(std::ptr::eq(c.drawn(id).unwrap(), v));
            let at = |clip: &Clip, t: f32| -> Vec<Vec3> {
                model(&c.skeleton, &clip.locals(&c.skeleton, t)).iter().map(|m| Vec3::new(m[3][0], m[3][1], m[3][2])).collect()
            };
            let mut moved = 0.0f32;
            for k in 0..=8 {
                let t = v.length * k as f32 / 8.0;
                let (a, b) = (at(v, t), at(d, t));
                assert!(a.iter().all(|p| p.is_finite() && p.length() < 2.5), "{id:#x} at {t}: {a:?}");
                moved = moved.max(a.iter().zip(&b).map(|(a, b)| (*a - *b).length()).fold(0.0, f32::max));
            }
            assert!(moved > 0.1, "{id:#x} draws like the donor's");
        }
    }

    /// The eye offset each part of `c` draws on the inspect pose of donor `donor` (the held frame, as `inspect`).
    fn pose_eyes(c: &CharacterData, donor: usize) -> Vec<[u32; 2]> {
        let t = character::pose_frame(donor, c.motions[&character::POSE].length);
        let t = hst_sim::pose::wrap(t, c.faces[&character::POSE].uv_length, true);
        crate::inspect::part_uv(c, character::POSE, t).iter().map(|o| [o.x.to_bits(), o.y.to_bits()]).collect()
    }

    /// M1i: a rerigged pc00 (eye node `s_me` kept as a joint) draws the disc pc00's eye offset on the inspect pose,
    /// on its eye material only; a costume without the donor's eye node takes it through `eyes` (§4c), and a donor
    /// whose track names no node of its own model (7) moves nothing.
    #[test]
    fn mod_eyes_follow_the_donors_uva() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        if !["test_pc00", "fore_pc00_phoebe"].iter().all(|m| Path::new(MODS).join(m).is_dir()) {
            return eprintln!("no context/mods/test_pc00 or fore_pc00_phoebe, skipped");
        }
        let mut s = Stores(default(), default(), default(), default());
        let disc = character::load_disc(&mut iso, 0, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3).unwrap();
        let held: Vec<_> = pose_eyes(&disc, 0).into_iter().filter(|o| *o != [0, 0]).collect();
        assert!(!held.is_empty() && held.iter().all(|o| *o == [0xbca3_d70a, 0x3ca3_d70a]), "{held:?}");
        let (_, c) = load_mod(&mut iso, &mut s, "test_pc00").unwrap();
        let moved: Vec<_> = pose_eyes(&c, 0).into_iter().filter(|o| *o != [0, 0]).collect();
        assert_eq!(moved, held, "the mod's moved parts vs the disc's");

        // Phoebe's rig has no eye node of donor 6's: only `eyes` binds the track, on that material's parts
        let dir = std::env::temp_dir().join(format!("hst_m1i_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // costumes must sit inside the mod folder (M16): copy Phoebe's in
        std::fs::copy(Path::new(MODS).join("fore_pc00_phoebe/model/pc00_phoebe_c00.glb"), dir.join("c00.glb")).unwrap();
        let glb = "c00.glb";
        let disc6 = character::load_disc(&mut iso, 6, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3).unwrap();
        let want6: Vec<_> = pose_eyes(&disc6, 6).into_iter().filter(|o| *o != [0, 0]).collect();
        assert!(!want6.is_empty());
        for (donor, eyes) in [(6, "[]"), (6, r#"["eye"]"#), (7, r#"["eye"]"#)] {
            let json = format!(r#"{{"standard":1,"id":"m1i","name":"m1i","costumes":[{:?}],"donor":{donor},"hand":"right","params":{{"base":{donor}}},"ai_row":{donor},"face":"morph","eyes":{eyes}}}"#, glb);
            std::fs::write(dir.join("mod.json"), json).unwrap();
            let m = read(&dir).unwrap();
            let c = load(&mut iso, &m, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3).unwrap();
            let o = pose_eyes(&c, donor);
            let moved: Vec<_> = o.iter().filter(|o| **o != [0, 0]).collect();
            match (donor, eyes) {
                (6, "[]") | (7, _) => assert!(moved.is_empty(), "donor {donor} eyes {eyes}: {moved:?}"),
                _ => {
                    assert!(!moved.is_empty() && moved.iter().all(|o| **o == want6[0]), "{moved:?} vs {want6:?}");
                    // only the parts of the `eye` material
                    let eye_parts = c.part_nodes.iter().filter(|p| p.0.contains(&character::EYES)).count();
                    assert_eq!(moved.len(), eye_parts);
                }
            }
        }
        std::fs::remove_dir_all(&dir).ok();
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

    /// M1r: a mod with `disc_racket` and no racket.glb holds that disc character's racket, not its donor's
    /// (Get a Grip's Suzuki: HST's Suzuki's, donor Cody).
    #[test]
    fn disc_racket_is_the_named_characters() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        let dir = Path::new(MODS).join("getagrip_pc12_suzuki");
        if disc_racket(&dir) != Some(13) || dir.join("racket.glb").exists() {
            return eprintln!("no context/mods/getagrip_pc12_suzuki packaged with disc_racket 13, skipped");
        }
        let mut s = Stores(default(), default(), default(), default());
        let (m, c) = load_mod(&mut iso, &mut s, "getagrip_pc12_suzuki").unwrap();
        // every disc racket has the same vertex counts: compare where the vertices are
        let verts = |s: &Stores, r: &[(Handle<Mesh>, Handle<StandardMaterial>)]| format!("{:?}", r.iter().map(|p| s.0.get(&p.0).unwrap().attribute(Mesh::ATTRIBUTE_POSITION)).collect::<Vec<_>>());
        let mut gs = HashMap::new();
        let disc = |iso: &mut Iso, s: &mut Stores, gs: &mut _, n: usize| {
            let data = iso.read(&format!("PC/PC{n:02}C00.XB")).unwrap();
            character::disc_racket(&Archive::parse(&data).unwrap(), n, &mut s.0, &mut s.1, &mut s.2, gs).unwrap()
        };
        let (own, donor) = (disc(&mut iso, &mut s, &mut gs, 13), disc(&mut iso, &mut s, &mut gs, m.donor));
        assert_ne!(m.donor, 13);
        assert_eq!(verts(&s, &c.racket), verts(&s, &own));
        assert_ne!(verts(&s, &c.racket), verts(&s, &donor));
    }

    /// Seeded mutations of a real mod's costume and motions `.glb` (numbers in its glTF JSON, bytes in its binary
    /// chunk): `read`, `check`, `skeleton` and (with the ISO) `load` return errors, never panic (M16's fuzz run;
    /// `HST_FUZZ=n` runs n instead of 400).
    #[test]
    fn mutated_glbs_never_panic() {
        let dir = Path::new(MODS).join("opentee2_pc16_lauryn");
        if !dir.is_dir() {
            return eprintln!("no context/mods/opentee2_pc16_lauryn, skipped");
        }
        let mut iso = Iso::open(ISO).ok();
        let at = Path::new("t_fuzz/lauryn");
        let files = crate::modfs::tests::dir_files(&dir);
        for (n, b) in files.iter().filter(|f| !f.0.starts_with("textures/") && !f.0.starts_with("voice/")) {
            crate::modfs::add(&at.join(n), b.clone().into());
        }
        let mut s = 0x2545_f491_4f6c_dd1du64;
        let mut rnd = move || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        let numbers = [0.0, 1.0, -1.0, 2.0, 3.0, 4.0, 255.0, 65535.0, 4294967295.0, 1e30, -1e30, 0.5];
        let n = std::env::var("HST_FUZZ").ok().and_then(|n| n.parse().ok()).unwrap_or(400);
        let (mut loaded, mut checked, mut panics) = (0, 0, Vec::new());
        for i in 0..n {
            let name = if i % 3 == 2 { "motions.glb" } else { "model/pc16_lauryn_own.glb" };
            let glb = &files.iter().find(|f| f.0 == name).unwrap().1;
            let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
            let (mut j, mut bin): (serde_json::Value, Vec<u8>) = (serde_json::from_slice(&glb[20..20 + len]).unwrap(), glb[20 + len..].to_vec());
            if i % 2 == 0 {
                // 1..=3 numbers anywhere in the JSON (indices, counts, offsets, strides, types, values)
                fn nth(v: &mut serde_json::Value, k: &mut usize, to: f64) -> bool {
                    match v {
                        serde_json::Value::Number(_) if *k == 0 => {
                            *v = serde_json::Number::from_f64(to).map_or(serde_json::Value::Null, |n| if to.fract() == 0.0 && to.abs() < 1e19 { (to as i64).into() } else { n.into() });
                            true
                        }
                        serde_json::Value::Number(_) => {
                            *k -= 1;
                            false
                        }
                        serde_json::Value::Array(a) => a.iter_mut().any(|v| nth(v, k, to)),
                        serde_json::Value::Object(o) => o.values_mut().any(|v| nth(v, k, to)),
                        _ => false,
                    }
                }
                fn count(v: &serde_json::Value) -> usize {
                    match v {
                        serde_json::Value::Number(_) => 1,
                        serde_json::Value::Array(a) => a.iter().map(count).sum(),
                        serde_json::Value::Object(o) => o.values().map(count).sum(),
                        _ => 0,
                    }
                }
                let leaves = count(&j);
                for _ in 0..1 + rnd() % 3 {
                    let to = if rnd() % 2 == 0 { numbers[rnd() as usize % numbers.len()] } else { (rnd() % 100_000) as f64 };
                    nth(&mut j, &mut (rnd() as usize % leaves), to);
                }
            } else {
                // bytes of the binary chunk (vertex data, indices, joints, weights, inverse binds, keys)
                for _ in 0..1 + rnd() % 64 {
                    let k = 8 + rnd() as usize % (bin.len() - 8);
                    bin[k] = rnd() as u8;
                }
            }
            let mut text = serde_json::to_vec(&j).unwrap();
            text.resize(text.len().next_multiple_of(4), b' ');
            let mut out = b"glTF".to_vec();
            out.extend(2u32.to_le_bytes());
            out.extend(((20 + text.len() + bin.len()) as u32).to_le_bytes());
            out.extend((text.len() as u32).to_le_bytes());
            out.extend(b"JSON");
            out.extend(text);
            out.extend(bin);
            crate::modfs::add(&at.join(name), out.into());
            let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if let Ok(m) = read(at)
                    && check(&m).is_ok()
                    && skeleton(&m, 0).is_ok()
                {
                    checked += 1;
                    if let Some(iso) = iso.as_mut() {
                        let mut s = Stores(default(), default(), default(), default());
                        loaded += load(iso, &m, 0, &mut s.0, &mut s.1, &mut s.2, &mut s.3).is_ok() as usize;
                    }
                }
            }));
            if run.is_err() {
                panics.push(i);
            }
            crate::modfs::add(&at.join(name), glb.clone().into());
        }
        eprintln!("{n} mutations: {checked} passed the checks, {loaded} loaded");
        crate::modfs::remove(Path::new("t_fuzz"));
        assert!(panics.is_empty(), "mutations {panics:?} panicked");
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
        // B275: TParam text cells load (and pass a peer's `Wire::check`, both `override_ok`); a comma is refused at read
        assert_eq!(try_(&ok.replace("\"Strk POW\":5", "\"タイプ\":\"オール\"")).unwrap().overrides, [("タイプ".to_string(), "オール".to_string())]);
        assert!(try_(&ok.replace("\"Strk POW\":5", "\"Strk POW\":\"1,2\"")).is_err());
        // M1n: a listed costume with no file is left out, the rest kept
        assert_eq!(try_(&ok.replace("[\"model/c00.glb\"]", "[\"model/c00.glb\",\"model/c01.glb\"]")).unwrap().costumes, ["model/c00.glb"]);
        assert_eq!((m.source.as_str(), try_(&ok.replace("\"id\"", "\"source\":\"Hot Shots Tennis: Get a Grip (PSP) pc03\",\"id\"")).unwrap().source.as_str()), ("", "Hot Shots Tennis: Get a Grip"));
        for (bad, field) in [("\"standard\":1", "\"standard\":2"), ("\"donor\":3", "\"donor\":14"), ("\"right\"", "\"ambi\""), ("Strk POW", "Strk PWR"), ("\"morph\"", "\"texture\""), ("model/c00.glb", "model/c01.glb"), ("\"ai_row\"", "\"disc_racket\":14,\"ai_row\"")].map(|(a, b)| (ok.replace(a, b), b)) {
            let e = try_(&bad).err().unwrap_or_default();
            assert!(!e.is_empty(), "accepted {field}");
            eprintln!("{e}");
        }
        // M1r: a disc character's racket by number
        try_(&ok.replace("\"ai_row\"", "\"disc_racket\":12,\"ai_row\"")).unwrap();
        assert_eq!((disc_racket(&dir), m.donor), (Some(12), 3));
        // a costume pack plays exactly as its disc character: donor, base and AI row it, no overrides
        let pack = ok.replace("\"donor\"", "\"costume_of\":3,\"donor\"").replace(",\"override\":{\"Strk POW\":5}", "");
        assert_eq!(try_(&pack).unwrap().costume_of, Some(3));
        assert_eq!(try_(ok).unwrap().costume_of, None);
        for bad in [ok.replace("\"donor\"", "\"costume_of\":3,\"donor\""), pack.replace("\"ai_row\":3", "\"ai_row\":4"), pack.replace("\"costume_of\":3", "\"costume_of\":14")] {
            assert!(try_(&bad).is_err(), "accepted {bad}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// M8f1: courts, umpires and kinds this build doesn't know stay off the character roster, quietly; `read`
    /// refuses them as characters; no `kind` is a character.
    #[test]
    fn other_kinds_stay_off_the_roster() {
        let dir = std::env::temp_dir().join(format!("hst-kind-test-{}", std::process::id()));
        for (k, want) in [("court", Kind::Court), ("umpire", Kind::Umpire), ("textures", Kind::Other("\"textures\"".into()))] {
            std::fs::create_dir_all(dir.join(k)).unwrap();
            std::fs::write(dir.join(k).join("mod.json"), format!(r#"{{"standard": 1, "kind": "{k}", "id": "x_{k}", "name": "X"}}"#)).unwrap();
            assert_eq!(Kind::of_dir(&dir.join(k)), want);
            assert!(read(&dir.join(k)).unwrap_err().contains("not a character mod"), "{k}");
        }
        std::fs::create_dir_all(dir.join("plain")).unwrap();
        std::fs::write(dir.join("plain/mod.json"), r#"{"standard": 1, "id": "x", "name": "X"}"#).unwrap();
        assert_eq!(Kind::of_dir(&dir.join("plain")), Kind::Character);
        assert!(read(&dir.join("plain")).unwrap_err().contains("costumes"));
        assert_eq!(Kind::of(&serde_json::json!({"kind": "character"})), Kind::Character);
        assert!(list(&dir).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The custom roster lists every packaged mod (all four games) and leaves a broken folder out.
    #[test]
    fn lists_the_custom_roster() {
        let Ok(root) = std::env::var("HST_MODS") else { return eprintln!("no HST_MODS, skipped") };
        let all = list(Path::new(&root));
        let n = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .map(|d| if d.join("mod.json").is_file() { vec![d] } else { std::fs::read_dir(&d).into_iter().flatten().flatten().map(|e| e.path()).collect() })
            .flatten()
            .filter(|d| d.join("mod.json").is_file() && Kind::of_dir(d) == Kind::Character && !d.file_name().unwrap().to_string_lossy().starts_with("opentee_"))
            .count();
        assert_eq!(all.len(), n);
        assert!(all.iter().all(|m| !m.source.is_empty() && !m.source.contains('(')), "a mod without its source game");
        for game in ["fore_", "getagrip_", "opentee2_", "oob_"] {
            assert!(all.iter().any(|m| m.id.starts_with(game)), "no {game} mod");
        }
        assert!(!all.iter().any(|m| retired(&m.id, &m.source)), "an Open Tee 1 mod listed (B380)");
        let dir = std::env::temp_dir().join(format!("hst-roster-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("broken")).unwrap();
        std::fs::write(dir.join("broken/mod.json"), "{}").unwrap();
        assert!(list(&dir).is_empty());
        // a pack folder (no mod.json) is searched one level down
        let one = all.iter().find(|m| m.id.starts_with("fore_")).unwrap();
        std::fs::create_dir_all(dir.join("pack")).unwrap();
        std::os::unix::fs::symlink(&one.dir, dir.join("pack/m")).unwrap();
        assert_eq!(list(&dir).iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), [one.id.as_str()]);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Every costume of every mod under `$HST_MODS` (e.g. the mods repo's `out/mods`) loads and has a body to draw
    /// (M1n: no costume in the select's list draws nothing); prints the failures.
    #[test]
    #[ignore]
    fn loads_every_mod() {
        let (Ok(mut iso), Ok(root)) = (Iso::open(ISO), std::env::var("HST_MODS")) else { return eprintln!("no ISO or HST_MODS, skipped") };
        let mut dirs: Vec<_> = std::fs::read_dir(root).unwrap().flatten().map(|e| e.path()).filter(|d| d.join("mod.json").is_file()).collect();
        dirs.sort();
        let (mut bad, mut n) = (0, 0);
        for d in &dirs {
            let m = match read(d) {
                Ok(m) => m,
                Err(e) => {
                    bad += 1;
                    eprintln!("FAIL {e}");
                    continue;
                }
            };
            for k in 0..m.costumes.len() {
                n += 1;
                let mut s = Stores(default(), default(), default(), default());
                let r = load(&mut iso, &m, k, &mut s.0, &mut s.1, &mut s.2, &mut s.3).and_then(|c| {
                    let verts: usize = c.parts.iter().filter_map(|p| s.0.get(&p.0)).map(Mesh::count_vertices).sum();
                    (verts > 0).then_some(()).ok_or_else(|| format!("{} costume {k}: nothing to draw ({} parts)", m.id, c.parts.len()))
                });
                if let Err(e) = r {
                    bad += 1;
                    eprintln!("FAIL {e}");
                }
            }
        }
        eprintln!("{} of {n} costumes of {} mods load", n - bad, dirs.len());
        assert_eq!(bad, 0);
    }
}
