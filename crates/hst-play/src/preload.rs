//! P50: a match's disc characters loaded on the async pool from the moment the menu asks for the match (its
//! `Mode::early`), while the menu fades out and the loading screen goes up, into [`Staged`] stores under handles
//! reserved from the world's. The match's start takes them ([`load_disc`]) instead of loading them in its own frame
//! (a character is ~0.1 s on desktop) and waits for any still loading before its next system ([`busy`]). Same loader,
//! same data: only where and when it runs changes. B550: mod characters' `.glb` costumes the same way ([`mods`],
//! [`load_mod`]).

use crate::character::{self, CharacterData, Staged};
use crate::mods::Mod;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use std::sync::Mutex;

/// A character loaded off the main thread with the assets it made.
type Loaded = Result<(CharacterData, Staged<Mesh>, Staged<StandardMaterial>, Staged<Image>, Staged<SkinnedMeshInverseBindposes>), String>;

/// Loads under way or done, by (character, costume).
static LOADS: Mutex<Vec<((usize, usize), crate::jobs::Job<Loaded>)>> = Mutex::new(Vec::new());

/// Starts loading each of `which` (character, costume) from `iso`, each once. Nothing without the asset stores or the
/// async pool (headless tests): the start loads them itself.
pub fn characters(world: &mut World, iso: &str, which: &[(usize, usize)]) {
    let (Some(m), Some(t), Some(i), Some(b)) = (
        world.get_resource::<Assets<Mesh>>(),
        world.get_resource::<Assets<StandardMaterial>>(),
        world.get_resource::<Assets<Image>>(),
        world.get_resource::<Assets<SkinnedMeshInverseBindposes>>(),
    ) else {
        return;
    };
    let providers = (m.get_handle_provider(), t.get_handle_provider(), i.get_handle_provider(), b.get_handle_provider());
    let mut loads = LOADS.lock().unwrap();
    for &(c, costume) in which {
        if loads.iter().any(|(k, _)| *k == (c, costume)) {
            continue;
        }
        let (iso, p) = (iso.to_string(), providers.clone());
        // B409: in the browser one a frame's budget, between frames (`jobs`)
        let Some(job) = crate::jobs::spawn(move || {
            let mut iso = hst_data::iso::Iso::open(&iso).map_err(|e| e.to_string())?;
            let (mut m, mut t, mut i, mut b) = (Staged::new(p.0), Staged::new(p.1), Staged::new(p.2), Staged::new(p.3));
            let d = character::load_disc(&mut iso, c, costume, &mut m, &mut t, &mut i, &mut b)?;
            Ok((d, m, t, i, b))
        }) else {
            return;
        };
        loads.push(((c, costume), job));
    }
}

/// Mod costume loads under way or done, by the costume's `.glb` file.
static MOD_LOADS: Mutex<Vec<(std::path::PathBuf, crate::jobs::Job<Loaded>)>> = Mutex::new(Vec::new());

/// The `.glb` costume `outfit` of mod `m` plays in (wrapping round its costume list, as the match's setup), if it
/// has any.
fn costume_file(m: &Mod, outfit: usize) -> Option<std::path::PathBuf> {
    (!m.costumes.is_empty()).then(|| m.dir.join(&m.costumes[outfit % m.costumes.len()]))
}

/// B550: starts loading each of `which` (mod, its slot's costume number) from its `.glb` (the donor's motions from
/// `iso`), each costume once. A mod without costumes (online stand-in) plays its donor: [`characters`] loads that.
pub fn mods(world: &mut World, iso: &str, which: &[(Mod, usize)]) {
    let (Some(m), Some(t), Some(i), Some(b)) = (
        world.get_resource::<Assets<Mesh>>(),
        world.get_resource::<Assets<StandardMaterial>>(),
        world.get_resource::<Assets<Image>>(),
        world.get_resource::<Assets<SkinnedMeshInverseBindposes>>(),
    ) else {
        return;
    };
    let providers = (m.get_handle_provider(), t.get_handle_provider(), i.get_handle_provider(), b.get_handle_provider());
    let stand_ins: Vec<_> = which.iter().filter(|(m, _)| m.costumes.is_empty()).map(|(m, o)| (m.donor, *o)).collect();
    {
        let mut loads = MOD_LOADS.lock().unwrap();
        for (md, outfit) in which {
            let Some(file) = costume_file(md, *outfit) else { continue };
            if loads.iter().any(|(k, _)| *k == file) {
                continue;
            }
            let (iso, p, md, costume) = (iso.to_string(), providers.clone(), md.clone(), outfit % md.costumes.len());
            let Some(job) = crate::jobs::spawn(move || {
                let mut iso = hst_data::iso::Iso::open(&iso).map_err(|e| e.to_string())?;
                let (mut m, mut t, mut i, mut b) = (Staged::new(p.0), Staged::new(p.1), Staged::new(p.2), Staged::new(p.3));
                let d = crate::mods::load(&mut iso, &md, costume, &mut m, &mut t, &mut i, &mut b)?;
                Ok((d, m, t, i, b))
            }) else {
                return;
            };
            loads.push((file, job));
        }
    }
    characters(world, iso, &stand_ins);
}

/// Whether a load is still under way.
pub fn busy() -> bool {
    LOADS.lock().unwrap().iter().any(|(_, t)| !t.is_finished()) || MOD_LOADS.lock().unwrap().iter().any(|(_, t)| !t.is_finished())
}

/// Drops whatever wasn't taken (the start is through).
pub fn clear() {
    LOADS.lock().unwrap().clear();
    MOD_LOADS.lock().unwrap().clear();
}

/// [`crate::mods::load`] of `m`'s costume `costume`, taken from a load started early ([`mods`]) if there is one.
#[allow(clippy::too_many_arguments)]
pub fn load_mod(
    iso: &mut hst_data::iso::Iso,
    m: &Mod,
    costume: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<CharacterData, String> {
    let task = costume_file(m, costume).and_then(|file| {
        let mut loads = MOD_LOADS.lock().unwrap();
        loads.iter().position(|(k, _)| *k == file).map(|at| loads.swap_remove(at).1)
    });
    match task.map(crate::jobs::Job::wait) {
        Some(Ok((d, mm, t, i, b))) => {
            mm.insert(meshes);
            t.insert(materials);
            i.insert(images);
            b.insert(bindposes);
            Ok(d)
        }
        _ => crate::mods::load(iso, m, costume, meshes, materials, images, bindposes),
    }
}

/// [`character::load_disc`], taken from a load started early if there is one (its assets moved into the stores).
pub fn load_disc(
    iso: &mut hst_data::iso::Iso,
    c: usize,
    costume: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Result<CharacterData, String> {
    let task = {
        let mut loads = LOADS.lock().unwrap();
        loads.iter().position(|(k, _)| *k == (c, costume)).map(|at| loads.swap_remove(at).1)
    };
    // still loading: wait for it (in the browser: still queued, so loaded now)
    let done = task.map(crate::jobs::Job::wait);
    match done {
        Some(Ok((d, m, t, i, b))) => {
            m.insert(meshes);
            t.insert(materials);
            i.insert(images);
            b.insert(bindposes);
            Ok(d)
        }
        _ => character::load_disc(iso, c, costume, meshes, materials, images, bindposes),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
    const MOD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/mods/getagrip_pc00_emi");

    /// B550: a mod's costume loaded on the pool and taken by the start is the one the start would load itself, its
    /// assets in the world's stores (needs the ISO at the repository root and a converted mod; skipped without).
    #[test]
    fn a_mod_loaded_early_is_the_one_the_start_loads() {
        if !Path::new(ISO).is_file() || !Path::new(MOD).is_dir() {
            return eprintln!("no ISO or context/mods/getagrip_pc00_emi, skipped");
        }
        let m = crate::mods::read(Path::new(MOD)).unwrap();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        mods(app.world_mut(), ISO, &[(m.clone(), 0)]);
        assert_eq!(MOD_LOADS.lock().unwrap().len(), 1, "no load started");
        let mut iso = hst_data::iso::Iso::open(ISO).unwrap();
        let w = app.world_mut();
        let (mut me, mut ma, mut im, mut bp) = (Assets::default(), Assets::default(), Assets::default(), Assets::default());
        let direct = crate::mods::load(&mut iso, &m, 0, &mut me, &mut ma, &mut im, &mut bp).unwrap();
        let early = w
            .resource_scope(|w, mut me: Mut<Assets<Mesh>>| {
                w.resource_scope(|w, mut ma: Mut<Assets<StandardMaterial>>| {
                    w.resource_scope(|w, mut im: Mut<Assets<Image>>| {
                        let mut bp = w.resource_mut::<Assets<SkinnedMeshInverseBindposes>>();
                        load_mod(&mut iso, &m, 0, &mut me, &mut ma, &mut im, &mut bp)
                    })
                })
            })
            .unwrap();
        assert!(MOD_LOADS.lock().unwrap().is_empty(), "the early load wasn't taken");
        assert_eq!((early.joints.len(), early.parts.len(), early.motions.len()), (direct.joints.len(), direct.parts.len(), direct.motions.len()));
        let meshes = w.resource::<Assets<Mesh>>();
        let materials = w.resource::<Assets<StandardMaterial>>();
        assert!(early.parts.iter().all(|(mesh, mat, _)| meshes.get(mesh).is_some() && materials.get(mat).is_some()));
        assert_eq!(meshes.len(), me.len());
        assert_eq!(w.resource::<Assets<Image>>().len(), im.len());
    }
}
