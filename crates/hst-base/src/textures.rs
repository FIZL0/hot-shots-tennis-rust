//! Replaced textures: the texture packs switched on (M5a, `mods_textures.rs`; the first in their order wins), then
//! `mods/texture-replacements/` beside the ISO (key-named files over a PCSX2 pack's), over the disc. The Texture
//! Replacements setting switches all of it. The folders are read once; with `HST_TEXTURE_RELOAD=1`
//! `texture-replacements/` files are re-read when they change while running.

use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};
use crate::character::Store;
use hst_data::texhash::{self, Overrides, TEXA};
use hst_data::{mtl, tim2};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

struct State {
    o: Overrides,
    /// The packs' textures, the first pack's of a key (M5a), and the packs' folders, top first.
    packs: HashMap<String, PathBuf>,
    pack_dirs: Vec<PathBuf>,
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

fn root(iso: &str) -> &Path {
    Path::new(iso)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

pub fn init(iso: &str) {
    let root = root(iso);
    // B227: the browser has no texture-replacements folder (its files are the mods' in OPFS), only packs
    let o = if cfg!(target_arch = "wasm32") { Overrides { root: root.into(), ..default() } } else { Overrides::scan(root) };
    let (packs, pack_dirs) = packs(root);
    eprintln!(
        "textures: {} from {} texture packs, {} PCSX2-named, {} key-named in mods/texture-replacements",
        packs.len(),
        pack_dirs.len(),
        o.pack.len(),
        o.mods.len()
    );
    *STATE.lock().unwrap() = Some(State {
        o,
        packs,
        pack_dirs,
        used: HashMap::new(),
        scale: HashMap::new(),
    });
}

/// The texture packs on under `root/mods` (`mods_textures::active`): each key's file from the first pack that has it,
/// and their folders, top first.
fn packs(root: &Path) -> (HashMap<String, PathBuf>, Vec<PathBuf>) {
    packs_in(root, &crate::mods_textures::order())
}

fn packs_in(root: &Path, order: &crate::mods_textures::Order) -> (HashMap<String, PathBuf>, Vec<PathBuf>) {
    let mods = root.join("mods");
    let active = crate::mods_textures::arrange(&mods, crate::mods_textures::list(&mods), order);
    let mut by_key = HashMap::new();
    for p in &active {
        for (k, f) in &p.textures {
            by_key.entry(k.clone()).or_insert_with(|| f.clone());
        }
    }
    (by_key, active.into_iter().map(|p| p.dir).collect())
}

/// Replacements on or off (the menu's "texture replacements" setting, per match); on keeps what an earlier `init`
/// read but re-reads the texture packs, whose order or set may have changed since (it applies to what's added from
/// then on: the next match's textures).
pub fn set(iso: &str, on: bool) {
    if !on {
        *STATE.lock().unwrap() = None;
    } else if STATE.lock().unwrap().is_none() {
        init(iso);
    } else {
        let (packs, dirs) = packs(root(iso));
        if let Some(s) = STATE.lock().unwrap().as_mut() {
            (s.packs, s.pack_dirs) = (packs, dirs);
        }
    }
}

/// M5d: the texture packs read again (one was installed or its order changed) and every texture already made whose
/// pack file changed, or comes from the pack in `fresh` (an update's files keep their names), swapped for it at once:
/// the menu's sheets show a new pack without a restart. A texture a pack no longer covers keeps its picture until the
/// next start (the disc's pixels aren't kept). How many images it swapped; none while the setting is off.
pub fn repack(images: &mut Assets<Image>, fresh: Option<&Path>) -> usize {
    // (decoded under the lock: it's a menu's handful of sheets, once per install)
    STATE.lock().unwrap().as_mut().map_or(0, |s| s.repack(images, fresh, &crate::mods_textures::order()))
}

impl State {
    fn repack(&mut self, images: &mut Assets<Image>, fresh: Option<&Path>, order: &crate::mods_textures::Order) -> usize {
        let (packs, dirs) = packs_in(&self.o.root, order);
        let new_from = |f: &PathBuf| fresh.is_some_and(|d| f.starts_with(d));
        let keys: Vec<String> = self.used.keys().filter(|k| packs.get(*k).is_some_and(|f| self.packs.get(*k) != Some(f) || new_from(f))).cloned().collect();
        (self.packs, self.pack_dirs) = (packs, dirs);
        let want: Vec<Files> = keys.iter().map(|k| self.files(k, &self.used[k].0)).collect();
        let got = parallel(&want, Files::load);
        let mut n = 0;
        for (k, r) in keys.iter().zip(got) {
            let Some(r) = r else { continue };
            for &id in &self.used[k].1 {
                if let Some(mut img) = images.get_mut(id) {
                    let disc = img.size_f32() / self.scale.get(&id).copied().unwrap_or(Vec2::ONE);
                    apply(&mut img, r.clone());
                    self.scale.insert(id, img.size_f32() / disc);
                    n += 1;
                }
            }
        }
        n
    }
}

/// Whether the "texture replacements" setting is on outside the browser (mods read their `textures/upscaled/` then;
/// the browser never has those, B227).
pub fn on() -> bool {
    !cfg!(target_arch = "wasm32") && STATE.lock().unwrap().is_some()
}

/// The texture packs in use (on, top first); empty while the setting is off.
pub fn packs_in_use() -> Vec<PathBuf> {
    STATE.lock().unwrap().as_ref().map(|s| s.pack_dirs.clone()).unwrap_or_default()
}

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, ui_rects);
    // B83: a full re-walk every second stutters on slow storage (microSD), so live reload is for texture authors only
    if std::env::var_os("HST_TEXTURE_RELOAD").is_some() {
        app.add_systems(Update, reload);
    }
}

/// Add `img`, made from the texture with this key and these PCSX2 names, replaced if a texture pack, the mod folder
/// or a PCSX2 pack has it.
pub fn add(
    images: &mut impl Store<Image>,
    mut img: Image,
    key: String,
    names: Vec<String>,
) -> Handle<Image> {
    // the PNG decode runs unlocked: other threads' textures and the UI's `ui_rects` don't wait on it
    let Some(files) = STATE.lock().unwrap().as_ref().map(|s| s.files(&key, &names)) else {
        let h = images.add(img);
        crate::asset_use::made(h.id(), &key);
        return h;
    };
    let disc = img.size_f32();
    let r = READY.lock().unwrap().as_ref().and_then(|r| r.get(&key).cloned()).or_else(|| files.load());
    let replaced = r.map(|r| {
        debug!("textures: replaced {key} ({disc} -> {}x{})", r.0, r.1);
        apply(&mut img, r);
    });
    let size = img.size_f32();
    let h = images.add(img);
    crate::asset_use::made(h.id(), &key);
    if let (Some(_), Some(f)) = (&replaced, &files.pack) {
        hst_data::uselog::note("tex-pack", || crate::modfs::key(f));
    }
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

/// Where a texture's replacement is read from: a texture pack's file (on disk, or a browser mod file in memory), else
/// `texture-replacements/`'s.
struct Files {
    pack: Option<PathBuf>,
    rest: texhash::Files,
}

impl Files {
    fn load(&self) -> Option<Pixels> {
        // a pack file that won't read or decode falls through, as a broken `texture-replacements/` file does
        let packed = self.pack.as_deref().and_then(|p| crate::modfs::read(p).map_err(|e| warn!("textures: {}: {e}", p.display())).ok());
        packed.and_then(|b| texhash::decode_png(std::io::Cursor::new(b))).or_else(|| self.rest.load())
    }
}

impl State {
    fn files(&self, key: &str, names: &[String]) -> Files {
        Files { pack: self.packs.get(key).cloned(), rest: self.o.files(key, names) }
    }
}

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
    let want: Vec<(String, Files)> = {
        let s = STATE.lock().unwrap();
        let Some(s) = s.as_ref() else { return Prefetch(vec![]) };
        let mut keys = std::collections::HashSet::new();
        mats.textures
            .iter()
            .filter(|t| !t.levels.is_empty())
            .map(|t| (t.gs(), t.width))
            .filter(|(g, _)| keys.insert(g.key()))
            .map(|(g, w)| (g.key(), s.files(&g.key(), &g.pcsx2_names(w, TEXA))))
            .collect()
    };
    fetch(want)
}

/// [`prefetch`] for TIM2 sheets added through [`add_tim2`] (the menu's ~50 sheets, Z1).
pub fn prefetch_tim2<'a>(sheets: impl IntoIterator<Item = &'a [u8]>) -> Prefetch {
    // P57: no replacements (the usual case in the browser): nothing to look up, so nothing to decode for it
    if STATE.lock().unwrap().as_ref().is_none_or(|s| s.packs.is_empty() && s.o.pack.is_empty() && s.o.mods.is_empty()) {
        return Prefetch(vec![]);
    }
    let pics: Vec<tim2::Picture> = sheets.into_iter().filter_map(|d| tim2::decode_alpha8(d).ok()?.into_iter().next()).collect();
    let want = {
        let s = STATE.lock().unwrap();
        let Some(s) = s.as_ref() else { return Prefetch(vec![]) };
        let mut keys = std::collections::HashSet::new();
        pics.iter()
            .filter(|p| keys.insert(p.gs().key()))
            .map(|p| (p.gs().key(), s.files(&p.gs().key(), &texhash::tim2_names(p))))
            .collect()
    };
    fetch(want)
}

fn fetch(want: Vec<(String, Files)>) -> Prefetch {
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
pub(crate) fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
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
pub(crate) fn parallel<T, R>(items: &[T], f: impl Fn(&T) -> R) -> Vec<R> {
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
        .filter(|(k, v)| s.o.mods.get(*k) != Some(v) && !s.packs.contains_key(*k))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods_textures::Order;
    use texhash::write_png;

    /// M5a: per texture the first pack in the order wins, then `texture-replacements/`, then the disc (`None`); a
    /// pack switched off is skipped; the browser's in-memory pack files read as the disc's do.
    #[test]
    fn packs_over_replacements_over_disc() {
        let root = std::env::temp_dir().join(format!("hst-texlayers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let png = |p: PathBuf, v: u8| {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            write_png(&p, 2, 1, &[v; 8]).unwrap();
        };
        for (n, keys) in [("hi", &["0000000000000001"][..]), ("lo", &["0000000000000001", "0000000000000002"][..])] {
            std::fs::create_dir_all(root.join("mods").join(n)).unwrap();
            std::fs::write(root.join("mods").join(n).join("mod.json"), format!(r#"{{"standard": 1, "kind": "textures", "id": "{n}", "name": "{n}"}}"#)).unwrap();
            for k in keys {
                png(root.join(format!("mods/{n}/textures/COURT/t.0-{k}.png")), if n == "hi" { 10 } else { 20 });
            }
        }
        for k in ["0000000000000001", "0000000000000002", "0000000000000003"] {
            png(root.join(format!("mods/texture-replacements/A/t.0-{k}.png")), 30);
        }
        let state = |o: &Order| {
            let (packs, pack_dirs) = packs_in(&root, o);
            State { o: Overrides::scan(&root), packs, pack_dirs, used: HashMap::new(), scale: HashMap::new() }
        };
        let px = |s: &State, k: &str| s.files(k, &[]).load().map(|p| p.2[0]);
        let s = state(&Order { order: vec!["hi".into(), "lo".into()], off: vec![] });
        assert_eq!(s.pack_dirs, [root.join("mods/hi"), root.join("mods/lo")]);
        assert_eq!([1, 2, 3, 4].map(|k| px(&s, &format!("{k:016x}"))), [Some(10), Some(20), Some(30), None]);
        let s = state(&Order { order: vec!["lo".into(), "hi".into()], off: vec![] });
        assert_eq!(px(&s, "0000000000000001"), Some(20), "the higher pack wins");
        let s = state(&Order { order: vec![], off: vec!["lo".into()] });
        assert_eq!([1, 2].map(|k| px(&s, &format!("{k:016x}"))), [Some(10), Some(30)]);
        // M5d: a pack put on top swaps the textures already made at once; an update's (`fresh`) files too
        let mut s = state(&Order { order: vec![], off: vec!["hi".into()] });
        let mut images = Assets::<Image>::default();
        let blank = || Image::new_fill(Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, bevy::render::render_resource::TextureDimension::D2, &[0; 4], bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::all());
        let (a, b) = (images.add(blank()).id(), images.add(blank()).id());
        s.used.insert("0000000000000001".into(), (vec![], vec![a]));
        s.used.insert("0000000000000002".into(), (vec![], vec![b]));
        let top = Order { order: vec!["hi".into(), "lo".into()], off: vec![] };
        assert_eq!(s.repack(&mut images, None, &top), 1, "only the texture whose pack changed");
        let img = images.get(a).unwrap();
        assert_eq!((img.width(), img.data.as_ref().unwrap()[0], s.scale[&a]), (2, 10, Vec2::new(2.0, 1.0)));
        assert_eq!(images.get(b).unwrap().width(), 1);
        assert_eq!(s.repack(&mut images, None, &top), 0, "nothing new");
        assert_eq!(s.repack(&mut images, Some(&root.join("mods/lo")), &top), 1, "an updated pack's own");
        assert_eq!(images.get(b).unwrap().data.as_ref().unwrap()[0], 20);
        // a browser pack: its files only in memory
        let mem = Path::new("t_texlayers/mods/web");
        crate::modfs::add(&mem.join("mod.json"), br#"{"standard": 1, "kind": "textures", "id": "web", "name": "Web"}"#.to_vec().into());
        write_png(&root.join("web.png"), 1, 1, &[40; 4]).unwrap();
        crate::modfs::add(&mem.join("textures/x.0-0000000000000004.png"), std::fs::read(root.join("web.png")).unwrap().into());
        let (packs, _) = packs_in(Path::new("t_texlayers"), &Order::default());
        let s = State { o: Overrides::default(), packs, pack_dirs: vec![], used: HashMap::new(), scale: HashMap::new() };
        assert_eq!(s.files("0000000000000004", &[]).load(), Some((1, 1, vec![40; 4])));
        crate::modfs::remove(Path::new("t_texlayers"));
        std::fs::remove_dir_all(&root).ok();
    }
}
