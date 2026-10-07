//! A disc court's collision world, built as the game builds it at court load (see `world` and `mesh`).

use crate::ball::Material;
use crate::mesh::{self, World};
use crate::world::{self, IDENTITY};
use hst_data::iso::Iso;
use hst_data::xb::Archive;
use hst_data::{exe, layout, mdl, mtl};

/// The game's collision material table as ball responses, by material id.
pub fn materials(game: &exe::Game) -> Vec<Material> {
    game.surfaces().iter().zip(0..=255).map(|(s, id)| Material { court: s.court, special: s.special, restitution: s.restitution, spin_loss: s.spin_loss, id, effect: s.effect, sound: s.sound }).collect()
}

/// Every node's drawing matrix: the root's `root`, each child its own placement through its parent's.
// ponytail: children follow the game's node walk (local · parent), checked only on court 10 where every prop is one node
fn nodes(m: &mdl::Model, root: world::M4) -> Vec<world::M4> {
    let mut out: Vec<world::M4> = Vec::with_capacity(m.node_count);
    for (k, parent) in m.node_parent.iter().enumerate() {
        out.push(match parent {
            None => root,
            Some(p) => world::mat_mul(&m.node_local[k], &out[*p]),
        });
    }
    out
}

/// A disc model as collision geometry.
fn collision_model(m: &mdl::Model, mt: &mtl::Mtl) -> mesh::Model {
    let u16_at = |h: &[u8], o: usize| u16::from_le_bytes([h[o], h[o + 1]]) as i32;
    mesh::Model {
        tris: m
            .colliding(mt)
            .map(|t| mesh::Tri { node: t.node, material: t.material, pos: t.pos, uv: t.uv, bounds: t.bounds })
            .collect(),
        materials: mt
            .materials
            .iter()
            .zip(&m.wrap)
            .map(|(mat, &wrap)| mesh::Material {
                two_sided: mat.two_sided,
                textured: mat.texture.is_some(),
                wrap,
                map: mat.attributes.map(|a| {
                    let a = &mt.attributes[a];
                    mesh::AttributeMap {
                        width: u16_at(&a.header, 2),
                        height: u16_at(&a.header, 4),
                        stride: u16_at(&a.header, 0x14),
                        texels: a.texels.clone(),
                        table: a.table,
                    }
                }),
            })
            .collect(),
        node_from_model: m.node_inverse.clone(),
    }
}

/// The umpire's chair on disc court `n` (1..11): her plant record (creature `npc15`), hole 1.
pub fn umpire_chair(iso: &mut Iso, n: u32) -> Option<[f32; 3]> {
    let data = iso.read(&format!("COURT/{n:02}/HOL01.XB")).ok()?;
    let a = Archive::parse(&data).ok()?;
    let e = a.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("plant_c{n:02}_h01_0.dat")))?;
    layout::plants(&a.read(e).ok()?).ok()?.into_iter().find(|p| p.category == 23 && p.index == 15).map(|p| p.pos)
}

/// Disc court `n` (1..11), hole 1: the court model at the origin and the colliding props in the game's list order
/// and grid.
pub fn world(iso: &mut Iso, n: u32) -> World {
    let mut files = std::collections::HashMap::new();
    let (mut list, mut plants, mut court) = (Vec::new(), Vec::new(), None);
    for arc in ["CMN.XB", "GRD01.XB", "HOL01.XB"] {
        let data = iso.read(&format!("COURT/{n:02}/{arc}")).expect("court archive on disc");
        let a = Archive::parse(&data).unwrap();
        let read = |n: &str| a.entries.iter().find(|e| e.name.eq_ignore_ascii_case(n)).map(|e| a.read(e).unwrap());
        for e in &a.entries {
            let name = e.name.to_ascii_lowercase();
            if name.ends_with(&format!("entry_c{n:02}.txt")) {
                list = layout::entries(&String::from_utf8_lossy(&a.read(e).unwrap()));
            } else if name.ends_with(&format!("plant_c{n:02}_h01_0.dat")) {
                plants = layout::plants(&a.read(e).unwrap()).unwrap();
            } else if let Some(stem) = name.strip_suffix(".mdl") {
                let base = &e.name[..e.name.len() - 4];
                let m = mdl::parse(&a.read(e).unwrap()).unwrap();
                let mt = mtl::parse(&read(&format!("{base}.MTL")).unwrap(), read(&format!("{base}.MTI")).as_deref()).unwrap();
                if arc == "GRD01.XB" && m.collides(&mt) {
                    assert!(court.is_none(), "one collision model in the court archive");
                    court = Some((m, mt));
                } else {
                    files.insert(stem.rsplit(['/', '\\']).next().unwrap().to_string(), (m, mt));
                }
            }
        }
    }
    let (cm, cmt) = court.expect("court collision model");
    let mut models = vec![collision_model(&cm, &cmt)];
    // the court stands at the origin, unscaled
    let court = mesh::Object { model: 0, scale: 1.0, to_model: IDENTITY, nodes: nodes(&cm, world::mat_mul(&cm.node_local[0], &IDENTITY)), center: cm.center, radius: cm.radius };

    let mut created = Vec::new();
    for cat in [15, 17, 18, 19, 20] {
        for p in plants.iter().filter(|p| p.category == cat && p.scale != 0.0) {
            let Some(e) = layout::resolve(&list, p, 0) else { continue };
            let (m, _) = &files[&e.stem];
            let (at, _) = world::place(p.category, p.pos, p.yaw, p.code);
            created.push((e.stem.clone(), world::instance(at, p.scale, &m.node_local[0], m.center, m.radius), p.scale));
        }
    }
    let (mut props, mut stems, mut spheres) = (Vec::new(), Vec::<String>::new(), Vec::new());
    for k in world::list_order(created.len()) {
        let (stem, prop, scale) = &created[k];
        let (m, mt) = &files[stem];
        if !m.collides(mt) {
            continue;
        }
        let model = match stems.iter().position(|s| s == stem) {
            Some(i) => i + 1,
            None => {
                stems.push(stem.clone());
                models.push(collision_model(m, mt));
                models.len() - 1
            }
        };
        spheres.push((props.len(), prop.center, prop.radius));
        props.push(mesh::Object { model, scale: *scale, to_model: prop.to_model, nodes: nodes(m, prop.node), center: prop.center, radius: m.radius });
    }
    World { models, court, props, grid: world::grid(&spheres) }
}
