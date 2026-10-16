//! Off-screen culling of the characters' and effects' meshes (B345, remaster-only; `Graphics::cull`, Settings →
//! Graphics → Cull Off-Screen, `--cull on|off`, `?cull=off` in the browser). A skinned part's bounds follow its joints
//! (Bevy's `DynamicSkinnedMeshBounds` over per-joint boxes, `bound`), widened by its morph targets' reach and a
//! margin for the costume sway (`noise.rs` moves vertices off their skin); an effect's CPU-written mesh gets its box
//! from its positions on each write (Bevy's own). Off, every such mesh is drawn (and casts) wherever it is, as before.

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{DynamicSkinnedMeshBounds, NoAutoAabb, NoFrustumCulling};
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// The setting as of the last `Graphics` change (spawns read it; `apply` flips what's already spawned).
static ON: AtomicBool = AtomicBool::new(true);

/// `--cull on|off` (the browser's `?cull=off`): 0 not given, 1 off, 2 on; over settings.txt's, for this run.
static FORCED: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// A mesh this module culls or not as the setting says: on being added it brings bounds that follow the joints (a
/// skinned part, `SkinnedMesh` spawned with it or before) or Bevy's box of its positions (recomputed on each write),
/// or with the setting off none (never culled).
#[derive(Component)]
#[component(on_add = added)]
pub struct Cullable;

fn added(mut world: bevy::ecs::world::DeferredWorld, ctx: bevy::ecs::lifecycle::HookContext) {
    let skinned = world.entity(ctx.entity).contains::<SkinnedMesh>();
    let mut e = world.commands();
    let mut e = e.entity(ctx.entity);
    match (ON.load(Ordering::Relaxed), skinned) {
        (false, _) => _ = e.try_insert(NoFrustumCulling),
        (true, true) => _ = e.try_insert((Aabb::default(), NoAutoAabb, DynamicSkinnedMeshBounds)),
        (true, false) => {}
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, (apply, unbounded).chain().before(bevy::camera::visibility::VisibilitySystems::CalculateBounds));
    if checking() {
        app.add_systems(PostUpdate, check.after(bevy::camera::visibility::VisibilitySystems::CheckVisibility));
    }
}

/// Set the setting before anything spawns (the app's start).
pub fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Remember the command line's `--cull` value.
pub fn force(v: &str) {
    FORCED.store(if v.trim() == "off" { 1 } else { 2 }, Ordering::Relaxed);
}

/// The command line's `--cull`, if it had one.
pub fn forced() -> Option<bool> {
    match FORCED.load(Ordering::Relaxed) {
        0 => None,
        f => Some(f == 2),
    }
}

/// Give a skinned part's mesh its per-joint bind-pose boxes (Bevy's `SkinnedMeshBounds`): each joint's box holds its
/// vertices as far as their morph targets (`targets`, each a list of vertex offsets; weights 0..1) can carry them,
/// plus a margin for the sway. A mesh without skinned vertices keeps none: `unbounded` leaves its part unculled.
pub fn bound<'a>(mesh: &mut Mesh, targets: impl IntoIterator<Item = &'a [[f32; 3]]>) {
    use bevy::mesh::skinning::{InfluenceIterator, JointAabb, JointIndex, SkinnedMeshBounds};
    let Some(pos) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|p| p.as_float3()) else { return };
    // each vertex's reach: the sum of its targets' negative and positive offsets per axis
    let mut reach = vec![(Vec3::ZERO, Vec3::ZERO); pos.len()];
    for t in targets {
        for (r, d) in reach.iter_mut().zip(t) {
            r.0 += Vec3::from(*d).min(Vec3::ZERO);
            r.1 += Vec3::from(*d).max(Vec3::ZERO);
        }
    }
    let Ok(influences) = InfluenceIterator::new(mesh) else { return };
    let mut boxes: Vec<Option<(Vec3, Vec3)>> = Vec::new();
    for i in influences {
        let j = i.joint_index.0 as usize;
        if boxes.len() <= j {
            boxes.resize(j + 1, None);
        }
        let (Some(p), Some(r)) = (pos.get(i.vertex_index), reach.get(i.vertex_index)) else { continue };
        let (lo, hi) = (Vec3::from(*p) + r.0, Vec3::from(*p) + r.1);
        let b = boxes[j].get_or_insert((lo, hi));
        *b = (b.0.min(lo), b.1.max(hi));
    }
    let (lo, hi) = boxes.iter().flatten().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), b| (lo.min(b.0), hi.max(b.1)));
    // the sway (`noise::draw`) moves a costume's skirt/hair vertices off their skin: a tenth of the part's own size
    // covers it with room (HST_CULL_CHECK held every vertex over a match; with no margin they left by up to 0.9)
    let pad = Vec3::splat(0.1 * (hi - lo).max_element());
    let (joints, aabbs): (Vec<_>, Vec<_>) =
        boxes.iter().enumerate().filter_map(|(j, b)| b.map(|(lo, hi)| (JointIndex(j as u16), JointAabb { center: (lo + hi) / 2.0, half_size: (hi - lo) / 2.0 + pad }))).unzip();
    if !aabbs.is_empty() {
        mesh.set_skinned_mesh_bounds(Some(SkinnedMeshBounds { aabbs, aabb_index_to_joint_index: joints }));
    }
}

/// The setting changed: each `Cullable` mesh culled or not to match. A skinned part without per-joint bounds (an odd
/// mod mesh) stays unculled either way.
fn apply(
    mut commands: Commands,
    g: Res<crate::graphics::Graphics>,
    parts: Query<(Entity, &Mesh3d, Has<SkinnedMesh>, Has<NoFrustumCulling>), With<Cullable>>,
    meshes: Res<Assets<Mesh>>,
) {
    if !g.is_changed() || ON.swap(g.cull, Ordering::Relaxed) == g.cull {
        return;
    }
    for (e, mesh, skinned, unculled) in &parts {
        let mut e = commands.entity(e);
        if !g.cull {
            e.try_insert(NoFrustumCulling).try_remove::<DynamicSkinnedMeshBounds>();
        } else if !skinned {
            e.try_remove::<NoFrustumCulling>();
        } else if unculled && meshes.get(mesh).is_some_and(|m| m.skinned_mesh_bounds().is_some()) {
            e.try_insert((Aabb::default(), NoAutoAabb, DynamicSkinnedMeshBounds)).try_remove::<NoFrustumCulling>();
        }
    }
}

/// A new skinned part whose mesh has no per-joint bounds (`bound` found no skinned vertices): drawn unculled, as its
/// box would otherwise stay empty at the origin.
fn unbounded(mut commands: Commands, parts: Query<(Entity, &Mesh3d), Added<DynamicSkinnedMeshBounds>>, meshes: Res<Assets<Mesh>>) {
    for (e, mesh) in &parts {
        if meshes.get(mesh).is_none_or(|m| m.skinned_mesh_bounds().is_none()) {
            commands.entity(e).try_insert(NoFrustumCulling).try_remove::<DynamicSkinnedMeshBounds>();
        }
    }
}

/// `HST_CULL_CHECK=1`: the skinned parts' meshes stay readable (`usage`) and `check` skins each culled part's
/// vertices on the CPU every frame (morph weights and the sway's moved positions included) and reports, every 5 s,
/// how far the farthest one lay outside its part's box (0 = the bounds always held it).
pub fn checking() -> bool {
    static CHECK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CHECK.get_or_init(|| std::env::var_os("HST_CULL_CHECK").is_some())
}

/// A skinned part's mesh usage: `usual`, or readable in the main world too while `checking`.
pub fn usage(usual: bevy::asset::RenderAssetUsages) -> bevy::asset::RenderAssetUsages {
    if checking() { bevy::asset::RenderAssetUsages::default() } else { usual }
}

#[derive(Default)]
pub struct Checked {
    frames: u32,
    parts: usize,
    drawn: usize,
    worst: f32,
    next: f64,
}

#[allow(clippy::type_complexity)]
pub fn check(
    parts: Query<(&Mesh3d, &SkinnedMesh, &Aabb, &GlobalTransform, Option<&bevy::mesh::morph::MeshMorphWeights>, &ViewVisibility), With<DynamicSkinnedMeshBounds>>,
    joints: Query<&GlobalTransform>,
    weights: Query<&bevy::mesh::morph::MorphWeights>,
    meshes: Res<Assets<Mesh>>,
    binds: Res<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
    time: Res<Time<Real>>,
    mut c: Local<Checked>,
) {
    use bevy::mesh::VertexAttributeValues as V;
    c.frames += 1;
    for (mesh, skin, aabb, at, morph, shown) in &parts {
        let (Some(m), Some(b)) = (meshes.get(mesh), binds.get(&skin.inverse_bindposes)) else { continue };
        let (Some(V::Float32x3(pos)), Some(V::Uint16x4(js)), Some(V::Float32x4(ws))) =
            (m.attribute(Mesh::ATTRIBUTE_POSITION), m.attribute(Mesh::ATTRIBUTE_JOINT_INDEX), m.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT))
        else {
            continue;
        };
        let w: Vec<f32> = match morph {
            Some(bevy::mesh::morph::MeshMorphWeights::Reference(r)) => weights.get(*r).map(|w| w.weights().to_vec()).unwrap_or_default(),
            Some(bevy::mesh::morph::MeshMorphWeights::Value { weights }) => weights.clone(),
            None => Vec::new(),
        };
        let targets = m.morph_targets().map(|t| t.as_slice()).unwrap_or_default();
        let to_entity = at.affine().inverse();
        let (lo, hi) = (Vec3::from(aabb.min()), Vec3::from(aabb.max()));
        let mats: Vec<Mat4> = skin.joints.iter().zip(b.iter()).map(|(j, ib)| joints.get(*j).map_or(Mat4::IDENTITY, |g| g.to_matrix()) * *ib).collect();
        c.parts += 1;
        c.drawn += shown.get() as usize;
        for (v, p) in pos.iter().enumerate() {
            let mut p = Vec3::from(*p);
            for (t, k) in w.iter().enumerate() {
                if *k != 0.0 {
                    p += targets.get(t * pos.len() + v).map_or(Vec3::ZERO, |a| a.position) * *k;
                }
            }
            let world: Vec3 = (0..4).filter(|&k| ws[v][k] != 0.0).map(|k| mats.get(js[v][k] as usize).map_or(p, |m| m.transform_point3(p)) * ws[v][k]).sum();
            let q = to_entity.transform_point3(world);
            let out = (lo - q).max(q - hi).max_element();
            c.worst = c.worst.max(out);
        }
    }
    let now = time.elapsed_secs_f64();
    if now >= c.next {
        info!("cull check: {} frames, {} part-frames ({} drawn), farthest vertex outside its box {:.4}", c.frames, c.parts, c.drawn, c.worst);
        c.next = now + 5.0;
    }
}
