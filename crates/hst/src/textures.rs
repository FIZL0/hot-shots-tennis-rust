//! Replaced textures from `mods/texture-replacements/` beside the ISO: key-named files over a PCSX2 pack's, over the disc.
//! The folder is read once; with `HST_TEXTURE_RELOAD=1` mod files are re-read when they change while running.

use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};
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

/// The menus' own rebuilt sheets (`play/menu_art.rs`): their size / the disc's, whatever the replacement setting.
static OURS: Mutex<Option<HashMap<AssetId<Image>, Vec2>>> = Mutex::new(None);

/// `id` is a sheet drawn `scale`× the disc's size: the UI's pixel rects on it are scaled to match.
pub fn ours(id: AssetId<Image>, scale: Vec2) {
    OURS.lock().unwrap().get_or_insert_default().insert(id, scale);
}

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

/// Replacements on or off (the menu's "upscaled textures" setting, per match; always off in the browser); on keeps
/// what an earlier `init` read.
pub fn set(iso: &str, on: bool) {
    // B227: never in the browser (a phone's tab runs out of memory on 4K textures; mods there have no upscaled set)
    if !on || cfg!(target_arch = "wasm32") {
        *STATE.lock().unwrap() = None;
    } else if STATE.lock().unwrap().is_none() {
        init(iso);
    }
}

/// Whether the "upscaled textures" setting is on (mods read their `textures/upscaled/` then).
pub fn on() -> bool {
    STATE.lock().unwrap().is_some()
}

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, ui_rects);
    // B83: a full re-walk every second stutters on slow storage (microSD), so live reload is for texture authors only
    if std::env::var_os("HST_TEXTURE_RELOAD").is_some() {
        app.add_systems(Update, reload);
    }
}

/// Add `img`, made from the texture with this key and these PCSX2 names, replaced if the mod folder or pack has it.
pub fn add(
    images: &mut impl Store<Image>,
    mut img: Image,
    key: String,
    names: Vec<String>,
) -> Handle<Image> {
    // the PNG decode runs unlocked: other threads' textures and the UI's `ui_rects` don't wait on it
    let Some(files) = STATE.lock().unwrap().as_ref().map(|s| s.o.files(&key, &names)) else {
        return images.add(img);
    };
    let disc = img.size_f32();
    let r = READY.lock().unwrap().as_ref().and_then(|r| r.get(&key).cloned()).or_else(|| files.load());
    let replaced = r.map(|r| {
        debug!("textures: replaced {key} ({disc} -> {}x{})", r.0, r.1);
        apply(&mut img, r);
    });
    let size = img.size_f32();
    let h = images.add(img);
    let mut s = STATE.lock().unwrap();
    // switched off meanwhile: nothing to track
    let Some(s) = s.as_mut() else { return h };
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

type Pixels = (u32, u32, Vec<u8>);

/// Replacements [`prefetch`]ed, by key.
static READY: Mutex<Option<HashMap<String, Pixels>>> = Mutex::new(None);

/// Keeps a [`prefetch`]'s decoded replacements until dropped.
pub struct Prefetch(Vec<String>);

impl Drop for Prefetch {
    fn drop(&mut self) {
        if let Some(r) = READY.lock().unwrap().as_mut() {
            self.0.iter().for_each(|k| _ = r.remove(k));
        }
    }
}

/// Decode the replacements of `mats`' textures on all cores now (Z1: one at a time they were most of a match's load,
/// and most are added twice, lit and raw), for the [`add`]s while the guard lives; same pixels as decoding in `add`.
pub fn prefetch(mats: &mtl::Mtl) -> Prefetch {
    let want: Vec<(String, texhash::Files)> = {
        let s = STATE.lock().unwrap();
        let Some(s) = s.as_ref() else { return Prefetch(vec![]) };
        let mut keys = std::collections::HashSet::new();
        mats.textures
            .iter()
            .filter(|t| !t.levels.is_empty())
            .map(|t| (t.gs(), t.width))
            .filter(|(g, _)| keys.insert(g.key()))
            .map(|(g, w)| (g.key(), s.o.files(&g.key(), &g.pcsx2_names(w, TEXA))))
            .collect()
    };
    fetch(want)
}

/// [`prefetch`] for TIM2 sheets added through [`add_tim2`] (the menu's ~50 sheets, Z1).
pub fn prefetch_tim2<'a>(sheets: impl IntoIterator<Item = &'a [u8]>) -> Prefetch {
    let pics: Vec<tim2::Picture> = sheets.into_iter().filter_map(|d| tim2::decode_alpha8(d).ok()?.into_iter().next()).collect();
    let want = {
        let s = STATE.lock().unwrap();
        let Some(s) = s.as_ref() else { return Prefetch(vec![]) };
        let mut keys = std::collections::HashSet::new();
        pics.iter()
            .filter(|p| keys.insert(p.gs().key()))
            .map(|p| (p.gs().key(), s.o.files(&p.gs().key(), &texhash::tim2_names(p))))
            .collect()
    };
    fetch(want)
}

fn fetch(want: Vec<(String, texhash::Files)>) -> Prefetch {
    let got: Vec<Option<Pixels>> = parallel(&want, |(_, f)| f.load());
    let mut r = READY.lock().unwrap();
    let r = r.get_or_insert_default();
    let keys = want.into_iter().zip(got).filter_map(|((k, _), p)| Some(k).zip(p)).map(|(k, p)| {
        r.insert(k.clone(), p);
        k
    });
    Prefetch(keys.collect())
}

/// `f` of each item, on scoped threads (one per core), in order.
#[cfg(not(target_arch = "wasm32"))]
fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = std::thread::available_parallelism().map_or(4, |n| n.get()).min(items.len()).max(1);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut out: Vec<(usize, R)> = std::thread::scope(|sc| {
        let workers: Vec<_> = (0..n)
            .map(|_| {
                sc.spawn(|| {
                    let mut mine = vec![];
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(t) = items.get(i) else { break mine };
                        mine.push((i, f(t)));
                    }
                })
            })
            .collect();
        workers.into_iter().flat_map(|w| w.join().unwrap()).collect()
    });
    out.sort_by_key(|(i, _)| *i);
    out.into_iter().map(|(_, r)| r).collect()
}

#[cfg(target_arch = "wasm32")]
fn parallel<T, R>(items: &[T], f: impl Fn(&T) -> R) -> Vec<R> {
    items.iter().map(f).collect()
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

type Scan = HashMap<String, (std::path::PathBuf, std::time::SystemTime)>;

/// `HST_TEXTURE_RELOAD` only: once a second, re-read mod files that are new or changed since the last look. The folder walk runs on the IO pool
/// (it's a big folder: tens of ms on the main thread).
// ponytail: a deleted mod file keeps its image until restart (the disc pixels aren't kept around)
fn reload(time: Res<Time>, mut next: Local<f32>, mut scan: Local<Option<Task<Scan>>>, mut images: ResMut<Assets<Image>>) {
    let Some(mods) = scan.as_mut().and_then(check_ready) else {
        if scan.is_none() && time.elapsed_secs() >= *next {
            *next = time.elapsed_secs() + 1.0;
            let root = STATE.lock().unwrap().as_ref().map(|s| s.o.root.clone());
            *scan = root.map(|r| IoTaskPool::get().spawn(async move { Overrides::scan_mods(&r) }));
        }
        return;
    };
    *scan = None;
    let mut s = STATE.lock().unwrap();
    let Some(s) = s.as_mut() else { return };
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
    let ours = OURS.lock().unwrap();
    if s.is_none() && ours.is_none() {
        return;
    }
    for (e, mut img, scaled) in &mut q {
        let Some(rect) = img.rect else { continue };
        // a rect we wrote is still the scaled disc rect (scale may have changed since: hot reload)
        let disc = match &scaled {
            Some(sc) if sc.out == rect => sc.disc,
            _ => rect,
        };
        let id = img.image.id();
        let k = ours.as_ref().and_then(|o| o.get(&id)).or_else(|| s.as_ref().and_then(|s| s.scale.get(&id))).copied().unwrap_or(Vec2::ONE);
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
