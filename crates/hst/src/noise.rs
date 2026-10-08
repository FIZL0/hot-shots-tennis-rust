//! Costume noise (`hst_sim::noise`): a costume's `.NOI` deformers sway its hair and cloth with the wind. The
//! game moves each packet's position entries in their bones' spaces before skinning, so a moved entry carries its
//! offset d through its bone's posed matrix: the vertex moves by Σ R·d. Bevy skins the bind-pose mesh, so each frame
//! the drawn vertex gets the bind-space offset its skin maps onto that: (Σ w·R·bind⁻¹)⁻¹ · Σ R·d, in the rig's own
//! copy of the part's mesh.

use std::sync::{Arc, LazyLock};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use hst_data::{mdl, noi};
use hst_sim::noise::{self, Noise};

use crate::character::{Motion, Rig};
use crate::weather::Weather;

static TABLE: LazyLock<[f32; 32]> = LazyLock::new(noise::table);

/// A position entry a deformer moves: its bone-space position, its share of the noise and its bone (joint).
struct Entry {
    deformer: usize,
    p: [f32; 3],
    weight: f32,
    node: usize,
}

/// A moved drawn vertex: its index in the part's mesh, its bind-pose position, its skin (joints and weights as the
/// mesh carries them) and its position entries.
struct Moved {
    v: usize,
    base: [f32; 3],
    skin: [(usize, f32); 4],
    entries: Vec<Entry>,
}

/// A costume's deformers, and per swaying part (index into `CharacterData::parts`) its moved drawn vertices.
pub struct Costume {
    deformers: Vec<noi::Deformer>,
    parts: Vec<(usize, Vec<Moved>)>,
}

/// The costume's deformers over the parts `character::skinned_parts` built from `model` (one per material with
/// triangles, in material order). The swaying parts' meshes stay readable, so each rig can copy them.
pub fn costume(model: &mdl::Model, noi: &[u8], parts: &[(Handle<Mesh>, Handle<StandardMaterial>, bool)], meshes: &mut Assets<Mesh>) -> Option<Arc<Costume>> {
    let deformers = noi::parse(noi)?;
    let packets: Vec<_> = model.materials.iter().flatten().zip(model.skinned()).collect();
    let mut drawn: Vec<usize> = packets.iter().filter(|(_, s)| !s.2.is_empty()).map(|(_, s)| s.0).collect();
    drawn.sort();
    drawn.dedup();
    let mut base = vec![0usize; model.materials.len()];
    let mut out: Vec<(usize, Vec<_>)> = Vec::new();
    for (pk, (material, verts, ..)) in packets {
        let first = base[material];
        base[material] += verts.len();
        let (Some(d), Some(part)) = (deformers.iter().position(|d| d.node as i16 == pk.group), drawn.iter().position(|&m| m == material)) else { continue };
        if pk.noise.is_empty() {
            continue;
        }
        let moved = (0..verts.len()).filter_map(|v| {
            let entries: Vec<Entry> = (pk.bones[v] as usize..(pk.bones[v + 1] as usize).min(pk.vertices.len()))
                .filter(|&e| pk.noise.get(e).is_some_and(|&w| w != 0.0))
                .map(|e| {
                    let node = pk.palette.get((pk.entry_flags.get(e).copied().unwrap_or(0) >> 3 & 7) as usize).or(pk.palette.first()).copied().unwrap_or(0);
                    Entry { deformer: d, p: pk.vertices[e].pos, weight: pk.noise[e], node }
                })
                .collect();
            let skin = std::array::from_fn(|k| (verts[v].joints[k] as usize, verts[v].weights[k]));
            (!entries.is_empty()).then_some(Moved { v: first + v, base: verts[v].pos, skin, entries })
        });
        match out.iter_mut().find(|(p, _)| *p == part) {
            Some((_, m)) => m.extend(moved),
            None => out.push((part, moved.collect())),
        }
    }
    for (part, _) in &out {
        if let Some(mut mesh) = meshes.get_mut(&parts[*part].0) {
            mesh.asset_usage = RenderAssetUsages::default();
        }
    }
    (!out.is_empty()).then(|| Arc::new(Costume { deformers, parts: out }))
}

/// A rig's deformers and its own copies of the swaying parts' meshes; the motion set last seen.
#[derive(Component)]
pub struct Sway {
    noise: Vec<Noise>,
    meshes: Vec<Handle<Mesh>>,
    sets: Option<u32>,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (attach, draw.after(crate::character::animate))).add_systems(FixedUpdate, step.after(crate::character::tick));
}

/// A new rig with a costume deformer swaps its swaying parts for copies of its own.
fn attach(mut commands: Commands, rigs: Query<(Entity, &Rig, &Children), Added<Rig>>, mut parts: Query<&mut Mesh3d>, mut meshes: ResMut<Assets<Mesh>>) {
    for (root, rig, children) in &rigs {
        let Some(c) = &rig.data.noise else { continue };
        let mut own = Vec::new();
        for (part, _) in &c.parts {
            let shared = rig.data.parts[*part].0.clone();
            let Some(copy) = meshes.get(&shared).cloned() else { continue };
            let copy = meshes.add(copy);
            for &child in children {
                if let Ok(mut m) = parts.get_mut(child)
                    && m.0 == shared
                {
                    m.0 = copy.clone();
                }
            }
            own.push(copy);
        }
        let noise = c.deformers.iter().map(|d| Noise::new(d.period)).collect();
        commands.entity(root).insert(Sway { noise, meshes: own, sets: None });
    }
}

/// Every tick: a motion set (a restart of the same motion too) restarts the deformers, then they step with the
/// wind (1.0 off court, as the game boots).
fn step(weather: Option<Res<Weather>>, mut rigs: Query<(&Rig, &Motion, &mut Sway)>) {
    let wind = weather.map_or(1.0, |w| noise::wind(w.today().speed));
    for (rig, motion, mut s) in &mut rigs {
        let Some(c) = &rig.data.noise else { continue };
        let reset = s.sets != Some(motion.sets);
        s.sets = Some(motion.sets);
        for (n, d) in s.noise.iter_mut().zip(&c.deformers) {
            if reset {
                n.reset(0.0, d.rate, d.amp[0], wind);
            }
            n.step(1.0, d.rate, d.amp[0], wind);
        }
    }
}

/// Every drawn frame, after the pose: each moved vertex from the previous phase, its entries' offsets carried
/// through their bones as posed.
fn draw(rigs: Query<(&Rig, &Sway)>, joints: Query<&Transform>, mut meshes: ResMut<Assets<Mesh>>) {
    for (rig, s) in &rigs {
        let Some(c) = &rig.data.noise else { continue };
        // each joint's posed model matrix (its parents' first)
        let mut pose: Vec<Option<Mat4>> = vec![None; rig.joints.len()];
        fn posed(j: usize, rig: &Rig, joints: &Query<&Transform>, pose: &mut [Option<Mat4>]) -> Mat4 {
            if let Some(m) = pose[j] {
                return m;
            }
            let local = joints.get(rig.joints[j]).map_or(Mat4::IDENTITY, |t| t.to_matrix());
            let m = rig.data.joints[j].parent.map_or(Mat4::IDENTITY, |p| posed(p, rig, joints, pose)) * local;
            pose[j] = Some(m);
            m
        }
        let mut rot = |j: usize| Mat3::from_mat4(posed(j, rig, &joints, &mut pose));
        for ((_, moved), h) in c.parts.iter().zip(&s.meshes) {
            let Some(mut mesh) = meshes.get_mut(h) else { continue };
            let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else { continue };
            for m in moved {
                let mut off = Vec3::ZERO;
                for e in &m.entries {
                    let n = &s.noise[e.deformer];
                    let q = noise::deform(&TABLE, e.p, e.weight, n.freq, n.prev, n.amp);
                    off += rot(e.node) * (Vec3::from(q) - Vec3::from(e.p));
                }
                let skin: Mat3 = m.skin.iter().map(|&(j, w)| rot(j) * Mat3::from_mat4(rig.data.joints[j].inverse_bind) * w).sum();
                if skin.determinant() != 0.0 {
                    pos[m.v] = (Vec3::from(m.base) + skin.inverse() * off).into();
                }
            }
        }
    }
}
