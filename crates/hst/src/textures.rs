//! Replaced textures from `mods/texture-replacements/` beside the ISO: key-named files over a PCSX2 pack's, over the disc.
//! Mod files are re-read when they change while running.

use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use crate::character::Store;
use hst_data::texhash::{self, Overrides, TEXA};
use hst_data::{mtl, tim2};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

struct State {
    o: Overrides,
    /// key -> (PCSX2 names, the images made from it)
    used: HashMap<String, (Vec<String>, Vec<AssetId<Image>>)>,
    /// replaced images: replacement size / disc size, for the UI's pixel rects
    scale: HashMap<AssetId<Image>, Vec2>,
}

// ponytail: global, so the loaders that only hold `Assets<Image>` can use it without new system params
static STATE: Mutex<Option<State>> = Mutex::new(None);

pub fn init(iso: &str) {
    let root = Path::new(iso)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let o = Overrides::scan(root);
    eprintln!(
        "textures: {} PCSX2-named, {} key-named in mods/texture-replacements",
        o.pack.len(),
        o.mods.len()
    );
    *STATE.lock().unwrap() = Some(State {
        o,
        used: HashMap::new(),
        scale: HashMap::new(),
    });
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, reload)
        .add_systems(PostUpdate, ui_rects);
}

/// Add `img`, made from the texture with this key and these PCSX2 names, replaced if the mod folder or pack has it.
pub fn add(
    images: &mut impl Store<Image>,
    mut img: Image,
    key: String,
    names: Vec<String>,
) -> Handle<Image> {
    let mut s = STATE.lock().unwrap();
    let Some(s) = s.as_mut() else {
        return images.add(img);
    };
    let disc = img.size_f32();
    let replaced = s.o.load(&key, &names).map(|r| {
        debug!("textures: replaced {key} ({disc} -> {}x{})", r.0, r.1);
        apply(&mut img, r);
    });
    let size = img.size_f32();
    let h = images.add(img);
    if replaced.is_some() {
        s.scale.insert(h.id(), size / disc);
    }
    s.used.entry(key).or_insert((names, vec![])).1.push(h.id());
    h
}

pub fn add_mtl(images: &mut impl Store<Image>, img: Image, t: &mtl::Texture) -> Handle<Image> {
    if t.levels.is_empty() {
        return images.add(img);
    }
    let g = t.gs();
    add(images, img, g.key(), g.pcsx2_names(t.width, TEXA))
}

pub fn add_tim2(images: &mut impl Store<Image>, img: Image, p: &tim2::Picture) -> Handle<Image> {
    add(images, img, p.gs().key(), texhash::tim2_names(p))
}

/// Swap the pixels for a replacement of any size: one level (the pack has no mips), format and sampler kept.
fn apply(img: &mut Image, (w, h, rgba): (u32, u32, Vec<u8>)) {
    img.texture_descriptor.size = Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    img.texture_descriptor.mip_level_count = 1;
    img.data = Some(rgba);
}

/// Once a second, re-read mod files that are new or changed since the last look.
// ponytail: a deleted mod file keeps its image until restart (the disc pixels aren't kept around)
fn reload(time: Res<Time>, mut next: Local<f32>, mut images: ResMut<Assets<Image>>) {
    if time.elapsed_secs() < *next {
        return;
    }
    *next = time.elapsed_secs() + 1.0;
    let mut s = STATE.lock().unwrap();
    let Some(s) = s.as_mut() else { return };
    let mods = Overrides::scan_mods(&s.o.root);
    let changed: Vec<String> = mods
        .iter()
        .filter(|(k, v)| s.o.mods.get(*k) != Some(v))
        .map(|(k, _)| k.clone())
        .collect();
    s.o.mods = mods;
    for key in changed {
        let Some((names, ids)) = s.used.get(&key) else {
            continue;
        };
        let Some(r) = s.o.load(&key, names) else {
            continue;
        };
        info!("textures: reloaded {}", s.o.mods[&key].0.display());
        for &id in ids {
            if let Some(mut img) = images.get_mut(id) {
                let disc = img.size_f32() / s.scale.get(&id).copied().unwrap_or(Vec2::ONE);
                apply(&mut img, r.clone());
                s.scale.insert(id, img.size_f32() / disc);
            }
        }
    }
}

/// A UI rect as its system set it (disc pixels) and as scaled to the replacement.
#[derive(Component)]
struct Scaled {
    disc: Rect,
    out: Rect,
}

/// The HUD picks sheet pieces in disc pixels: scale `ImageNode::rect` to the replacement's size.
fn ui_rects(mut commands: Commands, mut q: Query<(Entity, &mut ImageNode, Option<&mut Scaled>)>) {
    let s = STATE.lock().unwrap();
    let Some(s) = s.as_ref() else { return };
    for (e, mut img, scaled) in &mut q {
        let Some(rect) = img.rect else { continue };
        // a rect we wrote is still the scaled disc rect (scale may have changed since: hot reload)
        let disc = match &scaled {
            Some(sc) if sc.out == rect => sc.disc,
            _ => rect,
        };
        let k = s.scale.get(&img.image.id()).copied().unwrap_or(Vec2::ONE);
        let out = Rect {
            min: disc.min * k,
            max: disc.max * k,
        };
        if out != rect {
            img.bypass_change_detection().rect = Some(out);
        }
        match scaled {
            Some(mut sc) => *sc = Scaled { disc, out },
            None => {
                commands.entity(e).insert(Scaled { disc, out });
            }
        }
    }
}
