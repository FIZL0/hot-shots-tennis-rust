//! Costume noise (`hst_sim::noise`): a costume's `.NOI` deformers sway its hair and cloth with the wind. The
//! game moves each packet's position entries in their bones' spaces before skinning, so a moved entry carries its
//! offset d through its bone's posed matrix: the vertex moves by Σ R·d. Bevy skins the bind-pose mesh, so each frame
//! the drawn vertex gets the bind-space offset its skin maps onto that: (Σ w·R·bind⁻¹)⁻¹ · Σ R·d, in the rig's own
//! copy of the part's mesh.

use std::sync::{Arc, LazyLock};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use hst_data::{mdl, noi};
use hst_sim::noise::{self, Noise};

use crate::character::{Motion, Rig};
use crate::vertex_patch::{self, Patched, Queued};
use crate::weather::Weather;

static TABLE: LazyLock<[f32; 32]> = LazyLock::new(noise::table);

/// A position entry a deformer moves: its bone-space position, its share of the noise and its bone (joint).
pub struct Entry {
    pub deformer: usize,
    pub p: [f32; 3],
    pub weight: f32,
    pub node: usize,
}

/// A moved drawn vertex: its index in the part's mesh, its bind-pose position, its skin (joints and weights as the
/// mesh carries them) and its position entries.
pub struct Moved {
    pub v: usize,
    pub base: [f32; 3],
    pub skin: [(usize, f32); 4],
    pub entries: Vec<Entry>,
}

/// A costume's deformers, and per swaying part (index into `CharacterData::parts`) its moved drawn vertices.
pub struct Costume {
    pub deformers: Vec<noi::Deformer>,
    pub parts: Vec<(usize, Vec<Moved>)>,
    /// A mod's spring joints (standard §3b, `crate::spring`); none on the disc.
    pub springs: Vec<crate::spring::Mass>,
}

/// The costume's deformers over the parts `character::skinned_parts` built from `model` (one per material with
/// triangles, in material order). The swaying parts' meshes stay readable, so each rig can copy them.
pub fn costume(model: &mdl::Model, noi: &[u8], parts: &[(Handle<Mesh>, Handle<StandardMaterial>, bool)], meshes: &mut impl crate::character::Store<Mesh>) -> Option<Arc<Costume>> {
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
        if let Some(mesh) = meshes.get_mut(&parts[*part].0) {
            mesh.asset_usage = RenderAssetUsages::default();
        }
    }
    (!out.is_empty()).then(|| Arc::new(Costume { deformers, parts: out, springs: Vec::new() }))
}

/// A rig's deformers and its own copies of the swaying parts' meshes; the motion set last seen. B344: the deformers
/// step once a tick, the pose every drawn frame, and the drawn vertices of a strip share position entries, so each
/// distinct entry (deformer, position, weight) is moved once per step: per swaying part, each moved vertex's
/// entries' indices (in order) into `distinct`, and their bone-space offsets as the deformers stood at `offsets_at`.
#[derive(Component)]
pub struct Sway {
    noise: Vec<Noise>,
    /// Each copy's handle (kept alive) and its vertices as the GPU holds them (B357: patched, never `get_mut`).
    meshes: Vec<(Handle<Mesh>, Option<Patched>)>,
    sets: Option<u32>,
    slots: Vec<Vec<u32>>,
    distinct: Vec<(usize, [f32; 3], f32)>,
    offsets: Vec<Vec3>,
    offsets_at: Vec<Noise>,
    /// P40: the motion moved (a tick, or a driver set it) since the sway was last drawn. The noise only steps a
    /// tick, so between ticks (frames drawn above 60 Hz) the drawn vertices stay put: their bind-space offsets
    /// only move with the pose through the blend of a vertex's skin, under a millimetre over one tick's
    /// interpolation.
    stale: bool,
}

/// The costume's distinct entries, and per swaying part its moved vertices' entries' indices into them.
fn distinct(c: &Costume) -> (Vec<Vec<u32>>, Vec<(usize, [f32; 3], f32)>) {
    let mut at = std::collections::HashMap::new();
    let mut distinct = Vec::new();
    let slots = c.parts.iter().map(|(_, moved)| {
        moved.iter().flat_map(|m| &m.entries).map(|e| {
            *at.entry((e.deformer, e.p.map(f32::to_bits), e.weight.to_bits())).or_insert_with(|| {
                distinct.push((e.deformer, e.p, e.weight));
                distinct.len() as u32 - 1
            })
        }).collect()
    }).collect();
    (slots, distinct)
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (attach, draw.after(crate::character::animate))).add_systems(FixedUpdate, step.after(crate::character::tick));
}

/// A new rig with a costume deformer swaps its swaying parts for copies of its own.
pub fn attach(mut commands: Commands, rigs: Query<(Entity, &Rig, &Children), Added<Rig>>, mut parts: Query<&mut Mesh3d>, mut meshes: ResMut<Assets<Mesh>>) {
    for (root, rig, children) in &rigs {
        let Some(c) = &rig.data.noise else { continue };
        let mut own = Vec::new();
        for (part, _) in &c.parts {
            let shared = rig.data.parts[*part].0.clone();
            let Some(copy) = meshes.get(&shared).cloned() else { continue };
            let patched = Patched::new(&copy);
            let copy = meshes.add(copy);
            let patched = patched.map(|p| p.of(copy.id()));
            for &child in children {
                if let Ok(mut m) = parts.get_mut(child)
                    && m.0 == shared
                {
                    m.0 = copy.clone();
                }
            }
            own.push((copy, patched));
        }
        let noise = c.deformers.iter().map(|d| Noise::new(d.period)).collect();
        // the select may have despawned a preview rig in the same frame (B84)
        let (slots, distinct) = distinct(c);
        commands.entity(root).try_insert(Sway { noise, meshes: own, sets: None, slots, distinct, offsets: Vec::new(), offsets_at: Vec::new(), stale: true });
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
pub fn draw(mut rigs: Query<(&Rig, Ref<Motion>, &mut Sway)>, joints: Query<&Transform>, mut queued: ResMut<Queued>) {
    let mut changed = Vec::new();
    for (rig, motion, mut s) in &mut rigs {
        let Some(c) = &rig.data.noise else { continue };
        if motion.is_changed() {
            s.stale = true;
        }
        if !s.stale && s.offsets_at == s.noise {
            continue;
        }
        // the entries' offsets from the previous phase, again only after a step
        if s.offsets_at != s.noise {
            let s = &mut *s;
            s.offsets = s.distinct.iter().map(|&(d, p, weight)| {
                let n = &s.noise[d];
                Vec3::from(noise::deform(&TABLE, p, weight, n.freq, n.prev, n.amp)) - Vec3::from(p)
            }).collect();
            s.offsets_at.clone_from(&s.noise);
        }
        s.stale = false;
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
        // each joint's posed rotation, and that times its inverse bind's (a skin's term), once per joint
        let mut rots: Vec<Option<(Mat3, Mat3)>> = vec![None; rig.joints.len()];
        let mut rot = |j: usize| {
            *rots[j].get_or_insert_with(|| {
                let r = Mat3::from_mat4(posed(j, rig, &joints, &mut pose));
                (r, r * Mat3::from_mat4(rig.data.joints[j].inverse_bind))
            })
        };
        let s = &mut *s;
        for (((_, moved), (_, patched)), slots) in c.parts.iter().zip(&mut s.meshes).zip(&s.slots) {
            // only the moved vertices go to the GPU, written over the mesh's place in the vertex buffer: `get_mut`
            // re-uploads the whole mesh (B328, B357)
            let Some(patched) = patched else { continue };
            changed.clear();
            let mut slots = slots.iter();
            // the last skin's inverse (a strip's neighbours mostly share their skin), None if singular
            let mut last: Option<([(usize, f32); 4], Option<Mat3>)> = None;
            for m in moved {
                let mut off = Vec3::ZERO;
                for (e, &k) in m.entries.iter().zip(&mut slots) {
                    off += rot(e.node).0 * s.offsets[k as usize];
                }
                let inverse = match last {
                    Some((skin, inverse)) if skin == m.skin => inverse,
                    _ => {
                        let skin: Mat3 = m.skin.iter().map(|&(j, w)| rot(j).1 * w).sum();
                        let inverse = (skin.determinant() != 0.0).then(|| skin.inverse());
                        last = Some((m.skin, inverse));
                        inverse
                    }
                };
                if let Some(inverse) = inverse
                    && patched.set(m.v, (Vec3::from(m.base) + inverse * off).into())
                {
                    changed.push(m.v);
                }
            }
            changed.sort_unstable();
            vertex_patch::send(patched, &changed, &mut queued);
        }
    }
}
