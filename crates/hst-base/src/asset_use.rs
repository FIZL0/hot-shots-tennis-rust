//! Z2: with `HST_ASSET_LOG=FILE` (`hst_data::uselog`), the textures actually drawn: each frame, the images of every
//! mesh some camera sees and every UI image shown, logged once as `tex-draw KEY` (the disc texture's content key,
//! `texhash::GsTex::key`, which `--dump-textures` file names end in). `textures::add` logs `tex-key KEY` when a
//! texture is made and `tex-pack FILE` when a pack's file replaces it. Images made from disc data some other way
//! (pop-ups, balloons, sheets rebuilt or composited) are tied to their entry with [`source`] and logged as
//! `img-draw ARCHIVE:entry` when drawn.

use crate::effects::{Fx, OnTop};
use crate::gs::GsMaterial;
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;

/// Image -> the key it was made from (only while the log is on).
static KEYS: Mutex<Option<HashMap<AssetId<Image>, String>>> = Mutex::new(None);

/// `id` was made from the texture `key`.
pub fn made(id: AssetId<Image>, key: &str) {
    if hst_data::uselog::on() {
        hst_data::uselog::note("tex-key", || key.to_string());
        KEYS.lock().unwrap().get_or_insert_default().insert(id, key.to_string());
    }
}

/// Images made from something other than a texture key (or composited): id -> name.
static NAMES: Mutex<Option<HashMap<AssetId<Image>, String>>> = Mutex::new(None);

/// `id` was made from the archive entry whose decoded bytes are `bytes` (or is named `name` if none is).
pub fn source(id: AssetId<Image>, bytes: &[u8], name: &str) {
    if hst_data::uselog::on() {
        let n = hst_data::uselog::name_of(bytes).unwrap_or_else(|| name.to_string());
        NAMES.lock().unwrap().get_or_insert_default().insert(id, n);
    }
}

pub fn plugin(app: &mut App) {
    if hst_data::uselog::on() {
        app.add_systems(Last, drawn);
    }
}

#[allow(clippy::too_many_arguments)]
fn drawn(
    gs: Query<(&ViewVisibility, &MeshMaterial3d<GsMaterial>)>,
    std: Query<(&ViewVisibility, &MeshMaterial3d<StandardMaterial>)>,
    fx: Query<(&ViewVisibility, &MeshMaterial3d<Fx>)>,
    on_top: Query<(&ViewVisibility, &MeshMaterial3d<OnTop>)>,
    ui: Query<(&InheritedVisibility, &ImageNode)>,
    gs_m: Res<Assets<GsMaterial>>,
    std_m: Res<Assets<StandardMaterial>>,
    fx_m: Res<Assets<Fx>>,
    on_top_m: Res<Assets<OnTop>>,
) {
    let mut seen: Vec<AssetId<Image>> = Vec::new();
    seen.extend(gs.iter().filter(|(v, _)| v.get()).filter_map(|(_, m)| gs_m.get(&m.0)?.texture.as_ref().map(|t| t.id())));
    let std_images = |m: &StandardMaterial| {
        [&m.base_color_texture, &m.emissive_texture, &m.normal_map_texture, &m.metallic_roughness_texture, &m.occlusion_texture]
            .into_iter()
            .flatten()
            .map(|h| h.id())
            .collect::<Vec<_>>()
    };
    seen.extend(std.iter().filter(|(v, _)| v.get()).filter_map(|(_, m)| std_m.get(&m.0)).flat_map(std_images));
    seen.extend(fx.iter().filter(|(v, _)| v.get()).filter_map(|(_, m)| fx_m.get(&m.0)).flat_map(|m| std_images(&m.base)));
    seen.extend(on_top.iter().filter(|(v, _)| v.get()).filter_map(|(_, m)| on_top_m.get(&m.0)).flat_map(|m| std_images(&m.base)));
    seen.extend(ui.iter().filter(|(v, _)| v.get()).map(|(_, i)| i.image.id()));
    let (keys, names) = (KEYS.lock().unwrap(), NAMES.lock().unwrap());
    for id in seen {
        if let Some(k) = keys.as_ref().and_then(|m| m.get(&id)) {
            hst_data::uselog::note("tex-draw", || k.clone());
        }
        if let Some(n) = names.as_ref().and_then(|m| m.get(&id)) {
            hst_data::uselog::note("img-draw", || n.clone());
        }
    }
}
