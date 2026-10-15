//! P50: a match's disc characters loaded on the async pool from the moment the menu asks for the match (its
//! `Mode::early`), while the menu fades out and the loading screen goes up, into [`Staged`] stores under handles
//! reserved from the world's. The match's start takes them ([`load_disc`]) instead of loading them in its own frame
//! (a character is ~0.1 s on desktop) and waits for any still loading before its next system ([`busy`]). Same loader,
//! same data: only where and when it runs changes.

use crate::character::{self, CharacterData, Staged};
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

/// Whether a load is still under way.
pub fn busy() -> bool {
    LOADS.lock().unwrap().iter().any(|(_, t)| !t.is_finished())
}

/// Drops whatever wasn't taken (the start is through).
pub fn clear() {
    LOADS.lock().unwrap().clear();
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

