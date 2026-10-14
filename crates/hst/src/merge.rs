//! B342: fewer court draws. Every placement of a prop model is its own entity with one mesh per material; a court
//! places the same tree or structure tens of times (court 1: 521 court meshes on screen, 222 distinct mesh +
//! material pairs). At load, each model's placements are baked into one mesh per material ([`build`]) and drawn
//! instead of the copies while Settings → Display → Merge Court Meshes is on (`Graphics::merged`).
//!
//! Only parts drawn without blending are merged: the blended ones are sorted back to front per copy, and one merged
//! mesh would sort as a whole. A copy that `net_hide` hides (in front of the lens, around a buried eye) brings its
//! whole model back to per-copy draws for as long as it is hidden ([`switch`]), so what shows is the same either way.

use crate::gs::GsMaterial;
use bevy::prelude::*;

/// A placed copy of a prop model, as `main::load` spawned it.
pub struct Placed {
    pub stem: String,
    /// The copy's entity (its parts are its children).
    pub entity: Entity,
    pub transform: Transform,
    /// It casts a shadow (`shadow::Caster`).
    pub caster: bool,
}

/// One model's merged parts (this entity's children) and the copies they stand in for.
#[derive(Component)]
pub struct Merged {
    copies: Vec<Entity>,
    /// The copies' parts the merged meshes replace.
    parts: Vec<(AssetId<Mesh>, AssetId<GsMaterial>)>,
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        switch.after(bevy::transform::TransformSystems::Propagate).before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
    );
}

/// Bakes the copies of each model (by stem and whether they cast) into one mesh per unblended material, children of
/// `root` (the copies' parent), hidden until [`switch`] shows them. A model placed once, or a mirrored copy (its
/// winding would flip), stays as it is.
pub fn build(
    commands: &mut Commands,
    root: Entity,
    placed: &[Placed],
    library: &std::collections::HashMap<String, Vec<(Handle<Mesh>, Handle<GsMaterial>)>>,
    meshes: &mut Assets<Mesh>,
    materials: &Assets<GsMaterial>,
) {
    let mut groups: Vec<((&str, bool), Vec<&Placed>)> = Vec::new();
    for p in placed.iter().filter(|p| p.transform.scale.min_element() > 0.0) {
        let key = (p.stem.as_str(), p.caster);
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(p),
            None => groups.push((key, vec![p])),
        }
    }
    for ((stem, caster), copies) in groups.into_iter().filter(|(_, c)| c.len() > 1) {
        let Some(parts) = library.get(stem) else { continue };
        let solid: Vec<_> = parts.iter().filter(|(_, m)| materials.get(m).is_some_and(|m| !matches!(Material::alpha_mode(m), AlphaMode::Blend))).collect();
        let mut made = Vec::new();
        for (mesh, mat) in &solid {
            let Some(src) = meshes.get(mesh) else { continue };
            let mut out: Option<Mesh> = None;
            for c in &copies {
                let m = src.clone().transformed_by(c.transform);
                match &mut out {
                    Some(o) => {
                        if o.merge(&m).is_err() {
                            out = None;
                            break;
                        }
                    }
                    None => out = Some(m),
                }
            }
            if let Some(o) = out {
                made.push(((mesh.id(), mat.id()), (Mesh3d(meshes.add(o)), MeshMaterial3d((*mat).clone()))));
            }
        }
        if made.is_empty() {
            continue;
        }
        let (parts, draws): (Vec<_>, Vec<_>) = made.into_iter().unzip();
        let e = commands.spawn((Name::new(format!("merged {stem}")), Transform::default(), Visibility::Hidden, Merged { copies: copies.iter().map(|c| c.entity).collect(), parts })).id();
        if caster {
            commands.entity(e).insert(crate::shadow::Caster);
        }
        for d in draws {
            commands.entity(e).with_child(d);
        }
        commands.entity(root).add_child(e);
    }
}

/// Draws each model merged while the setting is on and none of its copies is hidden, else copy by copy. Set every
/// frame: the loading screen's warm-up (`mode.rs`) puts back the visibilities it found when it began.
fn switch(
    g: Res<crate::graphics::Graphics>,
    mut merged: Query<(&Merged, &mut Visibility)>,
    copies: Query<(&Visibility, &Children), Without<Merged>>,
    mut parts: Query<(&Mesh3d, &MeshMaterial3d<GsMaterial>, &mut Visibility), (Without<Merged>, Without<Children>)>,
) {
    let on = g.merged();
    for (m, mut vis) in &mut merged {
        let show = on && m.copies.iter().all(|&c| copies.get(c).is_ok_and(|(v, _)| *v != Visibility::Hidden));
        vis.set_if_neq(if show { Visibility::Inherited } else { Visibility::Hidden });
        for &c in &m.copies {
            let Ok((_, children)) = copies.get(c) else { continue };
            for &p in children {
                if let Ok((mesh, mat, mut v)) = parts.get_mut(p)
                    && m.parts.contains(&(mesh.id(), mat.id()))
                {
                    v.set_if_neq(if show { Visibility::Hidden } else { Visibility::Inherited });
                }
            }
        }
    }
}
