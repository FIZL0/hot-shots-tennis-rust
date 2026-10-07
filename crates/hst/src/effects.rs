//! Hit effects from `AZUMA/C_EFF/EFFCT.XB0`, played by `hst_sim::effect`: the racket impact (one model per shot
//! kind, the smash its own), started as a shot leaves the racket, at the ball, along its velocity.

use bevy::mesh::morph::{MeshMorphWeights, MorphWeights};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;
use hst_data::{ani, iso::Iso, mdl, mor, mtl::{self, Blend}, xb::Archive};
use hst_sim::effect::{Effect, impact_matrix, impact_scale};

const IMPACTS: [&str; 6] = ["top", "slice", "flat", "lob", "drop", "smash"];

/// One effect model on court: its player and the entities drawing it.
struct Shown {
    effect: Effect,
    root: Entity,
    joints: Vec<Entity>,
    materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Resource)]
pub struct Impacts {
    models: Vec<Shown>,
    /// The impact playing (the game has one).
    playing: Option<usize>,
}

/// A shot leaving the racket: shot kind (0 top … 4 drop; a smash's own kind), whether it is a smash (its own
/// model), the ball's position and velocity.
#[derive(Clone, Copy)]
pub struct Hit {
    pub kind: i32,
    pub smash: bool,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
}

impl Impacts {
    pub fn start(&mut self, h: Hit, transforms: &mut Query<&mut Transform>) {
        let k = if h.smash { 5 } else { h.kind.clamp(0, 4) as usize };
        self.playing = Some(k);
        let m = &mut self.models[k];
        m.effect.start();
        let world = Mat4::from_cols_array_2d(&impact_matrix([h.vel[0], h.vel[1], h.vel[2], 0.0], h.pos));
        if let Ok(mut t) = transforms.get_mut(m.root) {
            *t = Transform::from_matrix(world).with_scale(Vec3::splat(impact_scale(h.kind, h.vel)));
        }
    }
}

pub fn load(
    iso: &mut Iso,
    commands: &mut Commands,
    parent: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<Impacts, String> {
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").map_err(|e| e.to_string())?;
    let arc = Archive::parse(&data).map_err(|e| e.0)?;
    let get = |name: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(name)).and_then(|e| arc.read(e).ok());
    let mut models = Vec::new();
    for k in IMPACTS {
        let s = format!("yumoto/impact_{k}_a");
        let file = |ext: &str| get(&format!("{s}.{ext}")).ok_or(format!("{s}.{ext} missing"));
        let model = mdl::parse(&file("mdl")?).map_err(|e| e.0)?;
        let mtl = mtl::parse(&file("mtl")?, get(&format!("{s}.mti")).as_deref()).map_err(|e| e.0)?;
        let effect = Effect::new(
            &model,
            &ani::parse(&file("ani")?).map_err(|e| e.0)?,
            &mor::parse(&file("mor")?, 1).map_err(|e| e.0)?,
            &mor::parse(&file("mta")?, 1).map_err(|e| e.0)?,
            &mtl.materials,
        );
        let tex: Vec<Handle<Image>> = mtl.textures.iter().map(|t| images.add(crate::character::texture_image(t))).collect();
        let mats: Vec<Handle<StandardMaterial>> = mtl
            .materials
            .iter()
            .map(|m| {
                let [r, g, b, a] = m.color;
                materials.add(StandardMaterial {
                    base_color: Color::linear_rgba(r, g, b, a),
                    base_color_texture: m.texture.map(|t| tex[t].clone()),
                    unlit: true,
                    double_sided: true,
                    cull_mode: None,
                    // ponytail: @sub (Cd − Cs·As) has no Bevy mode and no impact uses it; drawn as Blend
                    alpha_mode: if m.blend() == Blend::Add { AlphaMode::Add } else { AlphaMode::Blend },
                    ..default()
                })
            })
            .collect();
        let parts = crate::character::skinned_parts(&model, &mats, meshes);
        let binds = bindposes.add(SkinnedMeshInverseBindposes::from(model.node_bind.iter().map(|b| Mat4::from_cols_array_2d(b).inverse()).collect::<Vec<_>>()));

        let weights = MorphWeights::new(vec![0.0; model.morph_names.len()], None).unwrap_or_default();
        let root = commands.spawn((Transform::default(), Visibility::Hidden, weights)).id();
        commands.entity(parent).add_child(root);
        let joints: Vec<Entity> = model.node_local.iter().map(|l| commands.spawn((Transform::from_matrix(Mat4::from_cols_array_2d(l)), Visibility::default())).id()).collect();
        for (i, p) in model.node_parent.iter().enumerate() {
            commands.entity(p.map_or(root, |p| joints[p])).add_child(joints[i]);
        }
        let skin = SkinnedMesh { inverse_bindposes: binds, joints: joints.clone() };
        for (mesh, material, morphed) in parts {
            let part = commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), skin.clone(), Transform::default())).id();
            if morphed {
                commands.entity(part).insert(MeshMorphWeights::Reference(root));
            }
            commands.entity(root).add_child(part);
        }
        models.push(Shown { effect, root, joints, materials: mats });
    }
    Ok(Impacts { models, playing: None })
}

/// One game frame of the playing impact (before this frame's shots start new ones).
pub fn tick(mut fx: ResMut<Impacts>) {
    let fx = &mut *fx;
    if let Some(k) = fx.playing {
        fx.models[k].effect.tick();
        if !fx.models[k].effect.live {
            fx.playing = None;
        }
    }
}

/// Pose, morph, fade and show the playing impact; hide the others.
pub fn draw(fx: Res<Impacts>, mut q: Query<(&mut Visibility, &mut MorphWeights)>, mut joints: Query<&mut Transform>, mut materials: ResMut<Assets<StandardMaterial>>) {
    for (k, m) in fx.models.iter().enumerate() {
        let live = fx.playing == Some(k);
        if let Ok((mut v, mut w)) = q.get_mut(m.root) {
            *v = if live { Visibility::Visible } else { Visibility::Hidden };
            if live {
                w.weights_mut().copy_from_slice(&m.effect.weights);
            }
        }
        if !live {
            continue;
        }
        for (j, l) in m.joints.iter().zip(m.effect.locals()) {
            if let Ok(mut t) = joints.get_mut(*j) {
                *t = Transform::from_matrix(Mat4::from_cols_array_2d(&l));
            }
        }
        for (h, a) in m.materials.iter().zip(&m.effect.alphas) {
            if let Some(mut mat) = materials.get_mut(h) {
                mat.base_color.set_alpha(*a);
            }
        }
    }
}
