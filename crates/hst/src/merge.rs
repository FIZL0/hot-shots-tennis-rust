//! B328: the court's placed props drawn as one mesh per material instead of an entity per model part. A court holds
//! ~400 prop parts; each one an entity costs the renderer (visibility, extract, queue, a draw) more than its triangles,
//! which on a phone's single wasm thread was most of the frame.
//!
//! - Merged: the parts that sort no blend order (opaque and alpha-tested materials), their vertices taken into game
//!   space with the plant's world matrix (normals by its inverse transpose, normalised as the shader does), grouped
//!   by material and by whether the plant casts a shadow.
//! - Kept apart: blended parts (drawn back to front by their model's place, which one mesh would lose), parts with
//!   another vertex layout and mirrored placements (the winding would flip).
//! - Hiding: each plant keeps its entity (and `Visibility`) for `net_hide`/`HideInside`; when one is hidden or shown
//!   again, the groups holding its parts are rebuilt without it (`rebuild`, only on a change).
//!
//! `HST_NO_MERGE=1` spawns every part on its own as before (for comparing).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use std::collections::HashMap;

use crate::gs::GsMaterial;

pub fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        rebuild.after(bevy::transform::TransformSystems::Propagate).before(bevy::camera::visibility::VisibilitySystems::CalculateBounds),
    );
}

/// A placed model whose mergeable parts went into [`Merged`] groups.
#[derive(Component)]
pub struct Plant;

/// One merged mesh: its pieces (one model part of one plant each, in game space) and which are drawn.
#[derive(Component)]
pub struct Merged {
    pieces: Vec<Piece>,
    shown: Vec<bool>,
}

struct Piece {
    plant: Entity,
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    col: Vec<[f32; 4]>,
    idx: Vec<u32>,
}

/// Collects the plants of a court load; [`Self::finish`] spawns the merged meshes.
#[derive(Default)]
pub struct Merger {
    groups: HashMap<(bool, AssetId<GsMaterial>), (Handle<GsMaterial>, Vec<Piece>)>,
    off: bool,
}

impl Merger {
    pub fn new() -> Self {
        Merger { off: std::env::var_os("HST_NO_MERGE").is_some(), ..default() }
    }

    /// Spawns the plant at `t` under `root` with the parts that can't be merged as children (as the court's own
    /// spawn does); the rest go to the groups. `cast`: the plant casts a shadow.
    pub fn add(
        &mut self,
        commands: &mut Commands,
        root: Entity,
        parts: &[(Handle<Mesh>, Handle<GsMaterial>)],
        t: Transform,
        cast: bool,
        meshes: &Assets<Mesh>,
        materials: &Assets<GsMaterial>,
    ) -> Entity {
        let e = commands.spawn((Plant, t, Visibility::default(), ChildOf(root))).id();
        let a = t.compute_affine();
        let mirrored = a.matrix3.determinant() < 0.0;
        for (m, mat) in parts {
            let blended = materials.get(mat).is_none_or(|g| matches!(Material::alpha_mode(g), AlphaMode::Blend));
            let piece = (!self.off && !blended && !mirrored).then(|| meshes.get(m).and_then(|mesh| piece(mesh, e, a))).flatten();
            match piece {
                Some(p) => self.groups.entry((cast, mat.id())).or_insert_with(|| (mat.clone(), Vec::new())).1.push(p),
                None => {
                    commands.spawn((Mesh3d(m.clone()), MeshMaterial3d(mat.clone()), ChildOf(e)));
                }
            }
        }
        e
    }

    /// Spawns the merged meshes under `root`: the shadow casters' under a [`crate::shadow::Caster`].
    pub fn finish(self, commands: &mut Commands, root: Entity, meshes: &mut Assets<Mesh>) {
        let parents = [false, true].map(|cast| {
            let p = commands.spawn((Transform::default(), Visibility::default(), ChildOf(root))).id();
            if cast {
                commands.entity(p).insert(crate::shadow::Caster);
            }
            p
        });
        for ((cast, _), (mat, pieces)) in self.groups {
            let shown = vec![true; pieces.len()];
            let mesh = meshes.add(build(&pieces, &shown));
            commands.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Merged { pieces, shown }, Transform::default(), Visibility::default(), ChildOf(parents[cast as usize])));
        }
    }
}

/// A model part's triangles in its plant's parent space, if it has the court meshes' layout.
fn piece(mesh: &Mesh, plant: Entity, a: bevy::math::Affine3A) -> Option<Piece> {
    use VertexAttributeValues as V;
    // (a render-world-only mesh's data is gone from here once extracted: the court load merges in its own frame)
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList || mesh.try_attributes().ok()?.count() != 4 {
        return None;
    }
    let a_ = |id| mesh.try_attribute_option(id).ok().flatten();
    let (Some(V::Float32x3(pos)), Some(V::Float32x3(nrm)), Some(V::Float32x2(uv)), Some(V::Float32x4(col))) =
        (a_(Mesh::ATTRIBUTE_POSITION), a_(Mesh::ATTRIBUTE_NORMAL), a_(Mesh::ATTRIBUTE_UV_0), a_(Mesh::ATTRIBUTE_COLOR))
    else {
        return None;
    };
    let idx = match mesh.try_indices_option().ok()?? {
        Indices::U32(i) => i.clone(),
        Indices::U16(i) => i.iter().map(|&i| i as u32).collect(),
    };
    let normal = a.matrix3.inverse().transpose();
    Some(Piece {
        plant,
        pos: pos.iter().map(|p| a.transform_point3(Vec3::from(*p)).to_array()).collect(),
        nrm: nrm.iter().map(|n| if *n == [0.0; 3] { *n } else { (normal * Vec3A::from(*n)).normalize_or_zero().to_array() }).collect(),
        uv: uv.clone(),
        col: col.clone(),
        idx,
    })
}

fn build(pieces: &[Piece], shown: &[bool]) -> Mesh {
    let (mut pos, mut nrm, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![], vec![]);
    for p in pieces.iter().zip(shown).filter(|(_, s)| **s).map(|(p, _)| p) {
        let base = pos.len() as u32;
        pos.extend_from_slice(&p.pos);
        nrm.extend_from_slice(&p.nrm);
        uv.extend_from_slice(&p.uv);
        col.extend_from_slice(&p.col);
        idx.extend(p.idx.iter().map(|i| i + base));
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx))
}

/// A plant hidden or shown again: rebuild the groups its parts are in (a group with nothing left is hidden).
fn rebuild(changed: Query<(), (With<Plant>, Changed<Visibility>)>, plants: Query<&Visibility, With<Plant>>, mut groups: Query<(&mut Merged, &Mesh3d, &mut Visibility), Without<Plant>>, mut meshes: ResMut<Assets<Mesh>>) {
    if changed.is_empty() {
        return;
    }
    for (mut g, mesh, mut vis) in &mut groups {
        let shown: Vec<bool> = g.pieces.iter().map(|p| plants.get(p.plant).is_ok_and(|v| *v != Visibility::Hidden)).collect();
        if shown == g.shown {
            continue;
        }
        vis.set_if_neq(if shown.contains(&true) { Visibility::Inherited } else { Visibility::Hidden });
        if shown.contains(&true) {
            let _ = meshes.insert(&mesh.0, build(&g.pieces, &shown));
        }
        g.shown = shown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_land_where_the_plant_draws_them() {
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; 3])
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0; 4]; 3])
            .with_inserted_indices(Indices::U16(vec![0, 1, 2]));
        let t = Transform::from_xyz(5.0, 0.0, 0.0).with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)).with_scale(Vec3::splat(2.0));
        let p = piece(&mesh, Entity::PLACEHOLDER, t.compute_affine()).unwrap();
        for (i, v) in p.pos.iter().enumerate() {
            let want = t.transform_point(Vec3::from(mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap()[i]));
            assert!(Vec3::from(*v).distance(want) < 1e-5, "{v:?} {want}");
        }
        // turned with the plant, unit length, a zero normal (unlit) kept zero
        assert!(Vec3::from(p.nrm[0]).distance(Vec3::NEG_Z) < 1e-6 && p.nrm[1] == [0.0; 3], "{:?}", p.nrm);
        // two copies, the first hidden: only the second's triangle, its indices rebased
        let two = [p, piece(&mesh, Entity::PLACEHOLDER, bevy::math::Affine3A::IDENTITY).unwrap()];
        let m = build(&two, &[true, true]);
        assert_eq!(m.indices().unwrap().iter().collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5]);
        let m = build(&two, &[false, true]);
        assert_eq!((m.count_vertices(), m.indices().unwrap().iter().collect::<Vec<_>>()), (3, vec![0, 1, 2]));
    }
}
