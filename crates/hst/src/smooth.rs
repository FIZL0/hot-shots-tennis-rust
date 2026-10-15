//! Graphics → Smooth Normals (G6, remaster-only): VU1 lights per vertex (`gs.wgsl`), and the disc splits a vertex
//! wherever its packets meet or a curved surface was exported faceted: the same position with a normal per face,
//! so a racket's frame, a post or a court's rounded prop lights in flat bands. With the setting on, every vertex
//! takes the average of the distinct normals at its position (in its mesh) that lie within [`CREASE`] of its own,
//! so soft splits close while roof ridges, a box's corners and a sheet's two sides stay apart. Only the normals change: the
//! vertex count, the triangles and every other attribute stay as they are.
//! A skinned vertex's normal is pre-weighted per bone (the first bone's in the normal slot, the rest in the tangent
//! slot, `character.rs`): the two are summed into its bind-pose normal, and each keeps its own length on the
//! averaged direction. Off (the default and the original mode), nothing runs.

use bevy::mesh::{Mesh, VertexAttributeValues};
use bevy::math::Vec3;
use std::collections::HashMap;

/// The cosine of the widest angle between two normals at one position that still get averaged: 45.6°. The disc's
/// rackets split at 10–40° and the courts' round props at up to an octagon's 45°; wider splits are mostly roof
/// ridges, bevels and box corners (a peak at 90°), whose flat faces would shade unevenly if averaged (G6 journal).
const CREASE: f32 = 0.70;

/// Positions closer than this (in model units, metres) are one: packets repeat a vertex where they meet.
const SAME: f32 = 1e-4;

/// `mesh`'s normals smoothed when Graphics → Smooth Normals is on (`graphics::SMOOTH`).
pub fn normals(mesh: &mut Mesh) {
    if crate::graphics::SMOOTH.load(std::sync::atomic::Ordering::Relaxed) {
        smooth(mesh);
    }
}

/// [`normals`] whatever the setting.
pub fn smooth(mesh: &mut Mesh) {
    let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return };
    let Some(VertexAttributeValues::Float32x3(nrm)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) else { return };
    // the second bone's share only on a skinned mesh (elsewhere the tangent slot isn't a normal)
    let tan: Option<Vec<Vec3>> = match (mesh.attribute(Mesh::ATTRIBUTE_TANGENT), mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)) {
        (Some(VertexAttributeValues::Float32x4(t)), Some(_)) => Some(t.iter().map(|t| Vec3::new(t[0], t[1], t[2])).collect()),
        _ => None,
    };
    let n0: Vec<Vec3> = nrm.iter().map(|&n| Vec3::from(n)).collect();
    let full: Vec<Vec3> = match &tan {
        Some(t) => n0.iter().zip(t).map(|(a, b)| *a + *b).collect(),
        None => n0.clone(),
    };
    let dir = average(pos, &full);
    let scaled = |v: &[Vec3]| v.iter().zip(&dir).map(|(n, d)| d.map_or(*n, |d| d * n.length())).collect::<Vec<_>>();
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, scaled(&n0).into_iter().map(|n| n.to_array()).collect::<Vec<_>>());
    if let Some(t) = tan {
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, scaled(&t).into_iter().map(|n| n.extend(0.0).to_array()).collect::<Vec<_>>());
    }
}

/// Per vertex, its averaged unit normal (`None` where it has none, or nothing changes).
fn average(pos: &[[f32; 3]], nrm: &[Vec3]) -> Vec<Option<Vec3>> {
    let mut at: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
    for (i, p) in pos.iter().enumerate() {
        at.entry(p.map(|x| (x / SAME).round() as i64)).or_default().push(i);
    }
    let mut out = vec![None; pos.len()];
    for group in at.values().filter(|g| g.len() > 1) {
        // the distinct directions here: a seam's repeats count once, so they don't outweigh a lone face
        let mut dirs: Vec<Vec3> = Vec::new();
        for &i in group {
            if let Some(u) = nrm[i].try_normalize() {
                if !dirs.iter().any(|d| d.dot(u) > 0.9999) {
                    dirs.push(u);
                }
            }
        }
        if dirs.len() < 2 {
            continue;
        }
        for &i in group {
            let Some(u) = nrm[i].try_normalize() else { continue };
            let sum: Vec3 = dirs.iter().filter(|d| d.dot(u) >= CREASE).sum();
            out[i] = sum.try_normalize();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::PrimitiveTopology;

    fn normals_of(m: &Mesh, a: bevy::mesh::MeshVertexAttribute) -> Vec<Vec3> {
        match m.attribute(a).unwrap() {
            VertexAttributeValues::Float32x3(v) => v.iter().map(|&n| Vec3::from(n)).collect(),
            VertexAttributeValues::Float32x4(v) => v.iter().map(|n| Vec3::new(n[0], n[1], n[2])).collect(),
            _ => unreachable!(),
        }
    }

    #[test]
    fn soft_splits_close_and_creases_stay() {
        // one position, three faces' normals: two 40° apart (a faceted curve), one at 90° to both's middle (a corner),
        // plus a seam repeat of the first; a lone vertex elsewhere
        let a = Vec3::new(-(20f32.to_radians().sin()), 20f32.to_radians().cos(), 0.0);
        let b = Vec3::new(20f32.to_radians().sin(), 20f32.to_radians().cos(), 0.0);
        let c = Vec3::Z;
        let pos = vec![[1.0, 2.0, 3.0], [1.0, 2.0, 3.0 + SAME * 0.2], [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], [5.0, 0.0, 0.0]];
        let nrm = vec![a, b, c, a, a];
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm.iter().map(|n| n.to_array()).collect::<Vec<_>>());
        smooth(&mut m);
        let out = normals_of(&m, Mesh::ATTRIBUTE_NORMAL);
        assert!(out[0].abs_diff_eq(Vec3::Y, 1e-6) && out[1].abs_diff_eq(Vec3::Y, 1e-6) && out[3].abs_diff_eq(Vec3::Y, 1e-6), "{out:?}");
        assert_eq!((out[2], out[4]), (c, a));
    }

    #[test]
    fn skinned_weights_keep_their_lengths() {
        // a seam between two skinned vertices: 0.7/0.3 of one direction, 1.0/0 of another 30° off
        let a = Vec3::new(-(15f32.to_radians().sin()), 15f32.to_radians().cos(), 0.0);
        let b = Vec3::new(15f32.to_radians().sin(), 15f32.to_radians().cos(), 0.0);
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 2])
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![(a * 0.7).to_array(), b.to_array()])
            .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, vec![(a * 0.3).extend(0.0).to_array(), [0.0; 4]])
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(vec![[0, 1, 0, 0], [0; 4]]));
        smooth(&mut m);
        let (n, t) = (normals_of(&m, Mesh::ATTRIBUTE_NORMAL), normals_of(&m, Mesh::ATTRIBUTE_TANGENT));
        assert!(n[0].abs_diff_eq(Vec3::Y * 0.7, 1e-6) && t[0].abs_diff_eq(Vec3::Y * 0.3, 1e-6), "{n:?} {t:?}");
        assert!(n[1].abs_diff_eq(Vec3::Y, 1e-6) && t[1] == Vec3::ZERO, "{n:?} {t:?}");
    }
}
