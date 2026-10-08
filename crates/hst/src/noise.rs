//! Costume noise (`hst_sim::noise`): a costume's `.NOI` deformers sway its hair and cloth with the wind. The
//! game moves each packet's position entries in their bones' spaces before skinning; here each moved entry's
//! offset is rotated into the bind pose and added to its drawn vertex, in the rig's own copy of the part's mesh.

use std::sync::{Arc, LazyLock};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use hst_data::{mdl, noi};
use hst_sim::noise::{self, Noise};

use crate::character::{Motion, Rig};
use crate::weather::Weather;

static TABLE: LazyLock<[f32; 32]> = LazyLock::new(noise::table);

/// A position entry a deformer moves: its bone-space position, its share of the noise and its bone's bind rotation.
struct Entry {
    deformer: usize,
    p: [f32; 3],
    weight: f32,
    rot: [[f32; 4]; 4],
}

/// A costume's deformers, and per swaying part (index into `CharacterData::parts`) its moved drawn vertices:
/// (vertex, bind-pose position, entries).
pub struct Costume {
    deformers: Vec<noi::Deformer>,
    parts: Vec<(usize, Vec<(usize, [f32; 3], Vec<Entry>)>)>,
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
                    Entry { deformer: d, p: pk.vertices[e].pos, weight: pk.noise[e], rot: model.node_bind[node] }
                })
                .collect();
            (!entries.is_empty()).then_some((first + v, verts[v].pos, entries))
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

/// A rig's deformers and its own copies of the swaying parts' meshes; the motion last seen.
#[derive(Component)]
pub struct Sway {
    noise: Vec<Noise>,
    meshes: Vec<Handle<Mesh>>,
    motion: Option<(usize, u32)>,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, attach).add_systems(FixedUpdate, sway);
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
        commands.entity(root).insert(Sway { noise, meshes: own, motion: None });
    }
}

/// Every tick: a motion change restarts the deformers, then they step with the wind (1.0 off court, as the game
/// boots) and the parts' moved vertices are redrawn from the previous phase.
/// ponytail: a restart of the same motion under the same serial doesn't reset them (the game resets on every set).
fn sway(weather: Option<Res<Weather>>, mut rigs: Query<(&Rig, &Motion, &mut Sway)>, mut meshes: ResMut<Assets<Mesh>>) {
    let wind = weather.map_or(1.0, |w| noise::wind(w.today().speed));
    for (rig, motion, mut s) in &mut rigs {
        let Some(c) = &rig.data.noise else { continue };
        let now = Some((motion.id, motion.serial));
        let reset = s.motion != now;
        s.motion = now;
        for (n, d) in s.noise.iter_mut().zip(&c.deformers) {
            if reset {
                n.reset(0.0, d.rate, d.amp[0], wind);
            }
            n.step(1.0, d.rate, d.amp[0], wind);
        }
        for ((_, moved), h) in c.parts.iter().zip(&s.meshes) {
            let Some(mut mesh) = meshes.get_mut(h) else { continue };
            let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else { continue };
            for (v, base, entries) in moved {
                let mut p = *base;
                for e in entries {
                    let n = &s.noise[e.deformer];
                    let q = noise::deform(&TABLE, e.p, e.weight, n.freq, n.prev, n.amp);
                    for k in 0..3 {
                        p[k] += (0..3).map(|j| (q[j] - e.p[j]) * e.rot[j][k]).sum::<f32>();
                    }
                }
                pos[*v] = p;
            }
        }
    }
}
