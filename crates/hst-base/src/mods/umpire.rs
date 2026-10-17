//! An umpire mod's model (M8f4; the mods repo's `standard/HST-COURT-UMPIRE-STANDARD.md` §3b): `umpire.glb`, one
//! skinned model seated on the chair (the seat at the origin, facing +Z, game space under `game_space`) with the
//! animations `idle`, `turn_r`, `turn_l` in the disc umpire's motion slots 0, 1, 2 and an optional `wait`. It is
//! drawn in place of the disc umpire; her controller (which clip, which frame) is the disc one's, so the match
//! never reads it. Picked with `--umpire-mod DIR`; the match's umpire number (timing, voice) becomes its `donor`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use hst_data::ani::{Anim, Track};
use hst_sim::pose::{Clip, Skeleton};

use super::{Kind, joints_of, materials_of, open, prim, skeleton_of, worlds};
use crate::character::{CharacterData, Store};

/// The `wait` animation's clip number (after the disc's three).
pub const WAIT: usize = 3;
/// The clips by animation name, in the disc umpire's motion numbers.
const CLIPS: [(&str, usize); 4] = [("idle", 0), ("turn_r", 1), ("turn_l", 2), ("wait", WAIT)];

/// `--umpire-mod DIR` this run (reset by each argument parse).
pub static CHOSEN: Mutex<Option<PathBuf>> = Mutex::new(None);

/// An umpire mod's manifest (`mod.json`, §3), checked.
#[derive(Debug, Clone)]
pub struct Umpire {
    pub dir: PathBuf,
    pub id: String,
    pub name: String,
    /// The disc umpire (0..4) whose timing, calls and (without `umpire.glb`) model it uses.
    pub donor: u8,
}

pub fn read(dir: &Path) -> Result<Umpire, String> {
    let at = dir.join("mod.json");
    let err = |m: String| format!("{}: {m}", at.display());
    let text = crate::modfs::read_to_string(&at).map_err(|e| err(e.to_string()))?;
    let j: serde_json::Value = serde_json::from_str(&text).map_err(|e| err(format!("not JSON: {e}")))?;
    if Kind::of(&j) != Kind::Umpire {
        return Err(err(format!("`kind` is {}: not an umpire mod", Kind::of(&j))));
    }
    if j["standard"].as_u64() != Some(1) {
        return Err(err(format!("`standard` is {}, this game reads standard 1", j["standard"])));
    }
    let text = |k: &str| j[k].as_str().filter(|t| (1..=64).contains(&t.chars().count())).map(str::to_string).ok_or_else(|| err(format!("`{k}` must be a string of 1 to 64 characters")));
    let donor = j["donor"].as_u64().filter(|&n| n < 5).ok_or_else(|| err("`donor` must be a number 0..=4".into()))? as u8;
    Ok(Umpire { dir: dir.to_path_buf(), id: text("id")?, name: text("name")?, donor })
}

/// The `--umpire-mod` umpire, if one was given and reads.
pub fn chosen() -> Option<Umpire> {
    let dir = CHOSEN.lock().ok()?.clone()?;
    read(&dir).inspect(|u| info!("umpire mod {} ({}), timed and voiced as disc umpire {}", u.name, u.id, u.donor)).inspect_err(|e| warn!("{e}")).ok()
}

/// `u`'s `umpire.glb` as a rig: `None` when it ships none (the donor's disc model is drawn).
pub fn load(
    u: &Umpire,
    meshes: &mut impl Store<Mesh>,
    materials: &mut impl Store<StandardMaterial>,
    images: &mut impl Store<Image>,
    bindposes: &mut impl Store<SkinnedMeshInverseBindposes>,
) -> Result<Option<CharacterData>, String> {
    let file = u.dir.join("umpire.glb");
    if !crate::modfs::is_file(&file) {
        return Ok(None);
    }
    let ctx = |e: String| format!("{}: {e}", file.display());
    let (doc, blob) = open(&file)?;
    let skin = doc.skins().next().ok_or_else(|| ctx("no skin".into()))?;
    let index: HashMap<usize, usize> = skin.joints().enumerate().map(|(k, n)| (n.index(), k)).collect();
    let mut up = vec![None; doc.nodes().len()];
    for n in doc.nodes() {
        n.children().for_each(|c| up[c.index()] = Some(n.index()));
    }
    let w = worlds(&doc);
    let space = doc.default_scene().or_else(|| doc.scenes().next()).and_then(|s| s.nodes().next()).map_or(Mat4::IDENTITY, |r| w[r.index()].inverse());
    let joints = joints_of(&doc, &blob);
    let skeleton = skeleton_of(&joints);
    let mut gs = HashMap::new();
    let handles = materials_of(&doc, &blob, super::upscaled_in(&u.dir, &file).as_deref(), images, materials, &mut gs).map_err(ctx)?;
    // a mesh without a skin rides the nearest joint at or above its node, whole (as a character mod's)
    let rides = |n: &gltf::Node| {
        let mut p = Some(n.index());
        while let Some(i) = p.filter(|i| !index.contains_key(i)) {
            p = up[i];
        }
        p.map(|i| index[&i])
    };
    let mut parts = Vec::new();
    for (n, rigid) in doc.nodes().filter_map(|n| match n.skin() {
        Some(_) => Some((n, None)),
        None => n.mesh().and(rides(&n)).map(|j| (n, Some(j))),
    }) {
        for p in n.mesh().unwrap().primitives() {
            let mut v = prim(&p, &blob, &[]).map_err(ctx)?;
            if let Some(j) = rigid {
                let to = space * w[n.index()];
                let nm = Mat3::from_mat4(to).inverse().transpose();
                v.pos.iter_mut().for_each(|q| *q = to.transform_point3(Vec3::from(*q)).into());
                v.nrm.iter_mut().for_each(|q| *q = (nm * Vec3::from(*q)).normalize_or_zero().into());
                v.joints.fill([j as u16, 0, 0, 0]);
                v.weights.fill([1.0, 0.0, 0.0, 0.0]);
            }
            // VU1's per-bone normals, split as a character mod's (`mods::load`)
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, crate::cull::usage(RenderAssetUsages::RENDER_WORLD))
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, v.pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, v.nrm.iter().zip(&v.weights).map(|(n, w)| (Vec3::from(*n) * w[0]).to_array()).collect::<Vec<_>>())
                .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, v.nrm.iter().zip(&v.weights).map(|(n, w)| (Vec3::from(*n) * (1.0 - w[0])).extend(0.0).to_array()).collect::<Vec<_>>())
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, v.uv)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, v.col)
                .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(v.joints))
                .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, v.weights)
                .with_inserted_indices(Indices::U32(v.idx));
            crate::lod::detail(&mut mesh, &[]);
            crate::cull::bound(&mut mesh, std::iter::empty::<&[[f32; 3]]>());
            parts.push((meshes.add(mesh), handles[p.material().index().unwrap_or(handles.len() - 1)].clone(), false));
        }
    }
    let motions = clips(&doc, &blob, &skeleton).map_err(ctx)?;
    if !motions.contains_key(&0) {
        return Err(ctx("no `idle` animation".into()));
    }
    let binds = crate::character::part_skins(&joints, &parts, meshes, bindposes);
    Ok(Some(CharacterData {
        joints,
        parts,
        racket: Vec::new(),
        motions,
        paths: HashMap::new(),
        binds,
        pelvis: Vec::new(),
        arm: None,
        morph_names: Vec::new(),
        faces: HashMap::new(),
        part_nodes: Vec::new(),
        stance_ball: None,
        skeleton,
        noise: None,
        gs,
        texture_face: None,
        visual: HashMap::new(),
        feet: None,
        frame_gameplay: false,
    }))
}

/// The named animations as clips (60 Hz key times → frames), keyed by joint name with their local rotation and
/// translation in game space, as a character mod's own motions (`mods::own_motions`); other names are skipped.
fn clips(doc: &gltf::Document, blob: &[u8], skeleton: &Skeleton) -> Result<HashMap<usize, Clip>, String> {
    use gltf::animation::util::ReadOutputs;
    let mut out = HashMap::new();
    for a in doc.animations() {
        let name = a.name().unwrap_or("");
        let Some(&(_, id)) = CLIPS.iter().find(|c| c.0 == name) else {
            warn!("umpire.glb: animation `{name}` is not one of idle, turn_r, turn_l, wait: skipped");
            continue;
        };
        let mut tracks: Vec<Track> = Vec::new();
        for c in a.channels() {
            let joint = c.target().node().name().unwrap_or("").replace(' ', "");
            if !skeleton.names.contains(&joint) {
                continue;
            }
            let r = c.reader(|_| Some(blob));
            let times: Vec<f32> = r.read_inputs().ok_or_else(|| format!("`{name}`: a channel has no times"))?.collect();
            let values: Vec<[f32; 4]> = match r.read_outputs() {
                // a rotation key is the conjugate of the local rotation (`ani`)
                Some(ReadOutputs::Rotations(q)) => q.into_f32().map(|[x, y, z, w]| [-x, -y, -z, w]).collect(),
                Some(ReadOutputs::Translations(t)) => t.map(|[x, y, z]| [x, y, z, 1.0]).collect(),
                _ => continue,
            };
            let unit = |v: &[f32; 4]| (v.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 0.01;
            let near = |v: &[f32; 4]| v.iter().all(|x| x.abs() < 1000.0);
            let rot = matches!(c.target().property(), gltf::animation::Property::Rotation);
            if !times.iter().all(|t| (0.0..=3600.0).contains(t)) || !values.iter().all(|v| if rot { unit(v) } else { near(v) }) {
                return Err(format!("`{name}`: a key out of range (times 0..=3600 s, rotations unit, positions within 1000 m)"));
            }
            let mut list: Vec<(i32, [f32; 4])> = Vec::new();
            for (t, v) in times.iter().zip(values) {
                let tick = (t * 60.0).round() as i32;
                match list.last_mut() {
                    Some(l) if l.0 == tick => l.1 = v,
                    _ => list.push((tick, v)),
                }
            }
            let k = tracks.iter().position(|t| t.name == joint).unwrap_or_else(|| {
                tracks.push(Track { name: joint.clone(), length: 0.0, rotation: vec![], position: vec![], parent: None });
                tracks.len() - 1
            });
            if rot {
                tracks[k].rotation = list;
            } else {
                tracks[k].position = list;
            }
        }
        out.insert(id, Clip::new(skeleton, &Anim { ticks_per_frame: 1, tracks }));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Get a Grip's umpires from the mods repo's `out/mods` (game data: never in git, so skipped where it isn't
    /// built) load as seated rigs with their four clips, the pelvis just above the seat in the idle.
    #[test]
    fn a_get_a_grip_umpire_loads() {
        // the mods repo beside this checkout (or beside the worktrees' folder)
        let at = |up: &str| Path::new(env!("CARGO_MANIFEST_DIR")).join(up).join("opentennis-mods/out/mods");
        let Some(root) = ["../../..", "../../../.."].map(at).into_iter().find(|d| d.join("gag_ump00_natasha/umpire.glb").is_file()) else {
            eprintln!("skipped: no Get a Grip umpire built in ../opentennis-mods/out/mods");
            return;
        };
        let mut app = App::new();
        app.add_plugins((AssetPlugin::default(), bevy::image::ImagePlugin::default())).init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<SkinnedMeshInverseBindposes>();
        let w = app.world_mut();
        let mut meshes = w.remove_resource::<Assets<Mesh>>().unwrap();
        let mut materials = w.remove_resource::<Assets<StandardMaterial>>().unwrap();
        let mut images = w.remove_resource::<Assets<Image>>().unwrap();
        let mut binds = w.remove_resource::<Assets<SkinnedMeshInverseBindposes>>().unwrap();
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.path()).filter(|p| p.join("umpire.glb").is_file()).collect();
        dirs.sort();
        assert_eq!(dirs.len(), 8, "Get a Grip's eight umpires");
        for dir in dirs {
            let u = read(&dir).unwrap();
            assert!(u.donor < 5);
            let d = load(&u, &mut meshes, &mut materials, &mut images, &mut binds).unwrap().unwrap();
            for id in [0, 1, 2, WAIT] {
                assert!(d.motions.get(&id).is_some_and(|c| c.length > 0.0), "clip {id}");
            }
            // idle 0.5 s, a turn 0.57 s, wait 2 s: frames at 60 Hz
            assert_eq!(d.motions[&0].length.round(), 30.0);
            assert_eq!(d.motions[&WAIT].length.round(), 120.0);
            // posed at the idle's first frame, as `character::animate` does
            let idle = &d.motions[&0];
            let local = |k: usize| {
                let mut tf = d.joints[k].rest;
                if let Some(t) = idle.tracks.iter().position(|t| t.node == k) {
                    let (rot, pos) = idle.sample(t, 0.0);
                    if let Some([x, y, z, w]) = rot {
                        tf.rotation = Quat::from_xyzw(-x, -y, -z, w).normalize();
                    }
                    if let Some(p) = pos {
                        tf.translation = Vec3::from(p);
                    }
                }
                tf.to_matrix()
            };
            let mut m = Mat4::IDENTITY;
            let mut j = d.joint("Bip01Pelvis");
            while let Some(k) = j {
                m = local(k) * m;
                j = d.joints[k].parent;
            }
            let y = m.w_axis.y;
            assert!((-0.15..0.0).contains(&y), "{}: pelvis {y} m (Y down) above the seat", u.id);
            assert!(!d.parts.is_empty());
        }
    }
}
