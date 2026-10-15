//! The select cards' 3D previews (B40b1): each card a player is picking on shows its hovered character in its costume
//! as the original's inspect screen draws it (`inspect.rs`), in place of the portrait, under the card's text. That
//! player's R2 / L2 (keyboard: 1, 2) play the inspect screen's win and loss; △ is the stats, not a preview action.
//!
//! B86: a card shows a PNG of the preview (character × costume) while its player scrolls, and the live model only once
//! the hover has held [`SETTLE`] seconds (B99: or at once when there is no PNG yet; the PNG is read on the frame its
//! key is hovered, and stays over the live model, kept hidden under it, for [`COVER`] frames and [`SETTLE_TICKS`] fixed
//! ticks, B140: the live node shows a frame before the PNG goes, as its first shown frame draws nothing) (the live one is what R2/L2 act on). The PNGs are drawn here, once, by an
//! off-screen preview (the baker: [`BAKE`] pixels, the cards' crop and lighting) and read back into a local cache
//! beside settings.txt (`cache/previews/<disc boot file>-r<RENDER>/`): disc characters' images are made from the
//! user's disc, so they are never shipped, and are all baked in the background from the first launch on (the hovered
//! uncached one first); a later launch only reads them. A mod may ship its own as `previews/<costume>.png` (the mods
//! repo's standard); else its costumes bake in the background after the disc's (a hovered one first). `HST_BAKE=all`
//! quits once every mod's costumes are baked (the mods repo's packaging). Bump [`RENDER`] when the preview's drawing changes.
//! B120: in the browser the cache is OPFS `previews/` (`web.rs`; the page reads it all before the game starts), and
//! a held hover keeps the card (the live model loads only for a press); a live rig stays hidden until it's posed
//! (`inspect.rs`), never showing its rest T-pose.
//!
//! Characters load on a worker thread (each one decodes its texture replacements and motions, ~130 ms; a disc
//! character only the preview's motions, B102: ~8 ms, ~10 ms in the browser, where a load runs inside a frame), the newest ask
//! first; at most [`KEEP_MODELS`] stay loaded and [`KEEP_PNGS`] PNGs (M1m's budget), least recently used dropped, so
//! scrolling a big mods folder can't run the GPU out of memory.
//!
//! The cards stack by quad order: every slot's `ZIndex` is its index in the layout, the preview's its `Tex::Preview`'s.
//!
//! `HST_SELECT=c,c,c,c` (with `HST_MENU=chars`) seats all four players on the keyboard hovering those characters (`mN`:
//! the custom roster's entry N, the select on the custom page), to `--shot` every card at once. `HST_PREVIEW=png`
//! never draws a live model, `live` never a PNG (to compare the two). `HST_STRESS=<seconds>` scrolls every card at
//! random each frame, then quits (`tools/select_stress.sh`).

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::TextureUsages;
use bevy::window::PrimaryWindow;
use bevy::tasks::{IoTaskPool, Task};
use hst_data::iso::Iso;

use super::{Art, Dev, GRID, Menu, Screen, Slot, Tex, Who, layout};
use crate::mods::Mod;
use crate::audio::{Sound, SoundBank};
use hst_sim::sound;
use crate::character::{self, CharacterData, Motion, Staged};
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use crate::inspect::{self, Act, Preview, PreviewMaterial};
use crate::Args;
use crate::play::controls::Bindings;

/// The baked previews' drawing version: part of the cache folder's name.
const RENDER: u32 = 8;
/// B219: Ashley's face square, grown about its centre to sit like the rest of the cast's.
const ASHLEY_ZOOM: f32 = 1.25;
/// A baked preview's size: the card's 128×160 at 3× (sharp on a window up to 1344 lines); a multiple of 64 bytes a
/// row, so the read-back has no row padding.
const BAKE: UVec2 = UVec2::new(384, 480);
/// B160: a face icon's close-up (the preview's [`inspect::face_square`] drawn again on its own): the select grid's
/// 60-px face is ~290 px on a 2160-line window.
// ponytail: one fixed size; bake at the window's icon pixels if 8K windows show it soft
const FACE: UVec2 = UVec2::new(320, 320);
/// B187: this device's bake and face close-up sizes: [`BAKE`] and [`FACE`] natively; in the browser the card's 128×160
/// at the page's lines (its shorter side in CSS px: the canvas's pixels) over 448, rounded up to 1×–3× (a phone's 390-line
/// page bakes 1×: 9× fewer pixels to draw, read back, convert and encode), the face to match (a multiple of 32 px, so its
/// read-back rows have no padding either). Its own cache folder ([`cache_name`]), so a desktop's cards stay as they were.
fn sizes() -> (UVec2, UVec2) {
    static SIZES: std::sync::LazyLock<(UVec2, UVec2)> = std::sync::LazyLock::new(|| {
        #[cfg(target_arch = "wasm32")]
        let lines = crate::web::page_size().map_or(1080.0, |p| p.min_element());
        #[cfg(not(target_arch = "wasm32"))]
        let lines = 1344.0_f32;
        let s = ((lines / 448.0).ceil() as u32).clamp(1, 3);
        if s == 3 { (BAKE, FACE) } else { (BAKE / 3 * s, UVec2::splat((FACE.x * s / 3).div_ceil(32) * 32)) }
    });
    *SIZES
}
/// `FixedUpdate` ticks (60 Hz) a baked preview draws before its read-back.
const SETTLE_TICKS: u32 = 6;
/// The baker's render layer (the cards use 1–4).
const BAKE_LAYER: usize = 6;
/// Loaded characters kept (four live cards, the baker's, one spare) and PNGs kept.
const KEEP_MODELS: usize = 6;
const KEEP_PNGS: usize = 48;
/// Seconds a card's hover holds before its live model replaces the PNG.
const SETTLE: f32 = 0.3;
/// B402: a menu unit's width over its height on screen (the 640×448 screen shown 4:3: 448 / 480).
pub(super) const UNIT_W: f32 = 14.0 / 15.0;
/// B402: a preview's width over its height in menu units, for a picture that is the bake's 4:5 on screen (the PNG
/// and the live drawing then show the same proportions).
pub(super) const CARD_W: f32 = 0.8 / UNIT_W;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(inspect::plugin)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<Gone>().init_resource::<Pending>().init_resource::<Voices>().init_resource::<ModCard>()
        .add_systems(Startup, start_loader)
        .add_systems(PostStartup, (hover_all, start_baker))
        .add_systems(FixedUpdate, character::tick)
        // B140: before `inspect::place` (its clear and crop), so a camera spawned this frame renders cleared, not black
        .add_systems(Update, ((receive, sync, bake, buttons).chain().after(super::draw).before(inspect::place), character::animate, icons))
        .add_systems(Last, despawn);
    if let Some(secs) = std::env::var("HST_STRESS").ok().and_then(|s| s.parse::<f32>().ok()) {
        app.add_systems(Update, stress(secs).before(super::step));
    }
    if let Ok(dir) = std::env::var("HST_SHOTS") {
        let from = if std::env::var("HST_STRESS").is_ok() { 60 } else { 1 };
        app.add_systems(Last, shots(dir, from));
    }
}

type Key = (Who, usize);

/// What a card shows: its key, the live model's camera (else a PNG) and its UI node; a live one's PNG it replaced, kept
/// on top for its last frames (B99).
struct Shown {
    key: Key,
    cam: Option<Entity>,
    node: Entity,
    cover: Option<(Entity, u32, f32)>,
    /// B192: its rectangle's size (a new size, the select's card to the confirm screen's stage, draws it again)
    wh: [f32; 2],
}

/// Frames a PNG stays over the live model that replaced it, while that model's first frames draw (its textures and
/// pipelines can land a few frames late: a blank or part-drawn card otherwise), and at least [`SETTLE_TICKS`] (B140: its
/// face settles on the fixed clock, which a fast menu frame outruns: the mouth popped at the swap).
// ponytail: a fixed count; a check of the live target's first full draw if a slow GPU still shows a gap
const COVER: u32 = 10;

/// A worker's load: the character, its hand and its assets, to move into the world's (`Err("")`: the ask was dropped
/// as stale).
type Loaded = (Key, Result<(CharacterData, f32, Staged<Mesh>, Staged<StandardMaterial>, Staged<Image>, Staged<SkinnedMeshInverseBindposes>), String>);

/// The worker threads: characters, and PNG files.
#[derive(Resource)]
struct Loader {
    ask: Sender<(Key, Option<Mod>)>,
    done: Mutex<Receiver<Loaded>>,
    asked: HashSet<Key>,
    /// Loaded characters (`None`: failed), least recently used first.
    models: HashMap<Key, Option<(Arc<CharacterData>, f32)>>,
    model_order: VecDeque<Key>,
    /// PNGs by key (`None`: the file didn't decode).
    pngs: HashMap<Key, Option<Handle<Image>>>,
    png_order: VecDeque<Key>,
    /// The cache folder, and per mod its folder name and newest file time (its PNGs' stamp).
    dir: PathBuf,
    stamps: HashMap<usize, String>,
    /// B229: mods the Mods page asked a picture of that aren't in the select's roster (installed since the game
    /// started): `Who::Mod(menu.mods.len() + n)`.
    extra: Vec<Mod>,
}

impl Loader {
    /// Load `key` (`m`: the mod, for a custom one).
    fn want(&mut self, key: Key, m: Option<&Mod>) {
        // B336: a browser mod's files are read in first (asked again each frame until they are)
        if m.is_some_and(|m| !crate::modfs::ready(&m.dir)) {
            return;
        }
        if self.asked.insert(key) {
            let _ = self.ask.send((key, m.cloned()));
        }
    }

    /// `key`'s loaded character, marked as just used.
    fn model(&mut self, key: Key) -> Option<&Option<(Arc<CharacterData>, f32)>> {
        if let Some(i) = self.model_order.iter().position(|k| *k == key) {
            self.model_order.remove(i);
            self.model_order.push_back(key);
        }
        self.models.get(&key)
    }

    /// The PNG of `key`, read now if it isn't kept (B99: a ~3 ms decode on the hovering frame, so a card never waits
    /// blank for a worker; None: not baked yet); marked as just used.
    fn png(&mut self, key: Key, menu: &Menu, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        if let Some(i) = self.png_order.iter().position(|k| *k == key) {
            self.png_order.remove(i);
            self.png_order.push_back(key);
            return self.pngs.get(&key).cloned().flatten();
        }
        let path = self.path(key, menu)?;
        let image = read(&path).and_then(|b| {
            Image::from_buffer(&b, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), RenderAssetUsages::RENDER_WORLD)
                .map_err(|e| warn!("{}: {e}", path.display()))
                .ok()
        });
        let handle = image.map(|i| images.add(i));
        self.pngs.insert(key, handle.clone());
        self.png_order.push_back(key);
        while self.png_order.len() > KEEP_PNGS {
            let old = self.png_order.pop_front().unwrap();
            self.pngs.remove(&old);
        }
        handle
    }

    /// Where `key`'s PNG is (a mod's own `previews/<costume>.png` if it ships one), or None for a mod index gone.
    fn path(&mut self, (w, k): Key, menu: &Menu) -> Option<PathBuf> {
        match w {
            Who::Disc(c) => Some(self.dir.join(format!("d{c:02}-{k}.png"))),
            Who::Mod(i) => {
                let m = menu.mods.get(i).or_else(|| self.extra.get(i.checked_sub(menu.mods.len())?))?;
                // B259: a mod installed since the start (the Mods page's) can be updated again: never kept
                let stamp = if i < menu.mods.len() { self.stamps.entry(i).or_insert_with(|| stamp(m)).clone() } else { stamp(m) };
                Some(mod_png(&self.dir, m, k, &stamp))
            }
        }
    }

    /// Mod index `i` (the roster's, else [`Loader::extra`]'s).
    fn mod_at<'a>(&'a self, menu: &'a Menu, i: usize) -> Option<&'a Mod> {
        menu.mods.get(i).or_else(|| self.extra.get(i.checked_sub(menu.mods.len())?))
    }

    /// Drop the least recently used characters over the budget, except `keep`'s.
    fn trim(&mut self, keep: &[Key]) {
        let mut i = 0;
        while self.models.len() > KEEP_MODELS && i < self.model_order.len() {
            let k = self.model_order[i];
            if keep.contains(&k) {
                i += 1;
                continue;
            }
            self.model_order.remove(i);
            self.models.remove(&k);
            self.asked.remove(&k);
        }
    }
}

/// Where mod `m`'s costume `k` preview is in cache folder `dir` (`stamp`: [`stamp`]'s), or its own
/// `previews/<costume's glb stem>.png` if it ships one.
fn mod_png(dir: &Path, m: &Mod, k: usize, stamp: &str) -> PathBuf {
    let own = m.costumes.get(k).and_then(|c| Path::new(c).file_stem()).map(|s| m.dir.join("previews").join(s).with_extension("png"));
    own.filter(|p| crate::modfs::is_file(p)).unwrap_or_else(|| dir.join(format!("m-{stamp}-{k}.png")))
}

/// The cache folder's name: the disc's boot file and [`RENDER`].
fn cache_name(iso: &str) -> String {
    let boot = Iso::open(iso).ok().as_mut().and_then(boot_file).unwrap_or_else(|| "disc".into());
    let size = sizes().0;
    if size == BAKE { format!("{boot}-r{RENDER}") } else { format!("{boot}-r{RENDER}-{}", size.y) }
}

/// The cache folder (natively beside settings.txt; in the browser the OPFS names' prefix, B120).
fn cache_dir(iso: &str, name: &str) -> PathBuf {
    #[cfg(target_arch = "wasm32")]
    return PathBuf::from(name);
    #[cfg(not(target_arch = "wasm32"))]
    super::Settings::path(iso).with_file_name("cache").join("previews").join(name)
}

/// B133: player face icon (INPANE's layout, [`inspect::face_icon`]) for the match panel, from the select preview of
/// mod `m`'s costume `k`, else (B160) disc character `c`'s; None until that's baked (the panel shows the disc face).
pub(crate) fn face(iso: &str, m: Option<&Mod>, c: usize, k: usize) -> Option<Image> {
    let dir = cache_dir(iso, &cache_name(iso));
    let path = match m {
        Some(m) => mod_png(&dir, m, k, &stamp(m)),
        None => dir.join(format!("d{c:02}-{k}.png")),
    };
    face_from(&path, 40)
}

/// B229: the Mods page's hovered mod without a picture (`want`), whose costume 0 the baker draws on that page as the
/// select would (the same cache: `card` reads it); `baked` counts finished drawings, so the page reads again.
#[derive(Resource, Default)]
pub(super) struct ModCard {
    pub want: Option<Mod>,
    pub baked: u32,
}

/// M17: mod `m`'s select picture (costume 0) as PNG bytes, once the select has baked it (or it ships its own).
pub(super) fn card(iso: &str, m: &Mod) -> Option<Vec<u8>> {
    read(&mod_png(&cache_dir(iso, &cache_name(iso)), m, 0, &stamp(m)))
}

/// B133: the select's face icons (each character's costume 0, as the disc grid shows each character's first face;
/// B160: the disc grid's too; the main screen's pictures' too), read from the face close-ups as they're baked (a mod's own preview PNG: cut from it);
/// [`ICON_READS`] new ones a frame so a page turn doesn't stall.
fn icons(menu: Res<Menu>, mut loader: ResMut<Loader>, mut icons: ResMut<super::Icons>, mut images: ResMut<Assets<Image>>, mut failed: Local<HashSet<Who>>) {
    let page: Vec<Who> = match menu.screen {
        // B368: every column's tiles on view
        Screen::Chars => menu.on_view(),
        // the main screen's pictures' faces (`home::faces`), as the baker draws them
        Screen::Main => super::home::faces(&menu),
        Screen::Mods => super::mods_screen::faces(&menu),
        _ => return,
    };
    let mut reads = 0;
    for w in page {
        if reads == ICON_READS || icons.0.contains_key(&w) || failed.contains(&w) {
            continue;
        }
        let Some(path) = loader.path((w, 0), &menu) else { continue };
        // a baked preview's close-up lands a few frames after it
        let own = !path.starts_with(&loader.dir);
        if !cached(&if own { path.clone() } else { face_path(&path) }) {
            continue;
        }
        reads += 1;
        match face_from(&path, 64) {
            Some(img) => {
                icons.0.insert(w, images.add(img));
            }
            None => {
                failed.insert(w);
            }
        }
    }
}

/// Icons read a frame (each a few ms: a PNG decode and a scale).
const ICON_READS: usize = 2;

/// A mod's PNG name stem: its folder's name and a hash of its files (B259: every file's path, size and time, and
/// `mod.json` / `store.json` whole: a store update's zip sha256 is in the latter), so an updated mod is baked again.
// ponytail: sizes and times, not contents; a browser zip import (no times, no store.json) that changes a file to one
// of the same size keeps its old card unless its mod.json changes too. Hash the in-memory bytes if that turns up.
fn stamp(m: &Mod) -> String {
    dir_stamp(&m.dir)
}

fn dir_stamp(dir: &Path) -> String {
    use std::hash::{Hash, Hasher};
    fn walk(d: &Path, h: &mut impl Hasher) {
        for p in crate::modfs::read_dir(d) {
            match crate::modfs::size(&p) {
                Some((len, t)) => (p.file_name(), len, t).hash(h),
                None => walk(&p, h),
            }
        }
    }
    // ponytail: SipHash with fixed keys; a Rust update that changes it only bakes the cards once more
    let mut h = std::hash::DefaultHasher::new();
    walk(dir, &mut h);
    for f in ["mod.json", "store.json"] {
        crate::modfs::read(&dir.join(f)).ok().hash(&mut h);
    }
    // the folder's name, as its PNGs' names start (`m-<name>-<hash>-<k>.png`)
    let name: String = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '-' }).collect();
    format!("{name}-{:016x}", h.finish())
}

/// B259: cache file `name` is a mod's drawing (`m-…`, its face close-up too) that none of `stamps` made: an updated or
/// removed mod's old one.
fn stale(name: &str, stamps: &[String]) -> bool {
    name.starts_with("m-") && !stamps.iter().any(|s| name.strip_prefix("m-").and_then(|n| n.strip_prefix(s.as_str())).is_some_and(|r| r.starts_with('-')))
}

/// The disc's boot file (`SCUS_976.10`), from SYSTEM.CNF.
fn boot_file(iso: &mut Iso) -> Option<String> {
    let cnf = iso.read("SYSTEM.CNF").ok()?;
    let line = String::from_utf8_lossy(&cnf).lines().find(|l| l.starts_with("BOOT2"))?.to_string();
    let name = line.rsplit(['\\', ':']).next()?.split(';').next()?.trim().to_string();
    Some(name.replace(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '_', "_"))
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static WORK: std::cell::RefCell<Option<Box<dyn FnMut() -> bool>>> = const { std::cell::RefCell::new(None) };
}

fn start_loader(mut commands: Commands, args: Res<Args>, meshes: Res<Assets<Mesh>>, materials: Res<Assets<StandardMaterial>>, images: Res<Assets<Image>>, binds: Res<Assets<SkinnedMeshInverseBindposes>>) {
    let (ask, asks) = std::sync::mpsc::channel::<(Key, Option<Mod>)>();
    let (send, done) = std::sync::mpsc::channel();
    let providers = (meshes.get_handle_provider(), materials.get_handle_provider(), images.get_handle_provider(), binds.get_handle_provider());
    let path = args.iso.clone();
    // newest ask first: the card just hovered loads before older hovers; a backlog past a few is stale
    let mut queue = Vec::new();
    // one load; false once the menu is gone. `block`: wait for an ask (the worker thread), else return when there's none
    let mut work = move |iso: &mut Iso, block: bool| -> bool {
        queue.extend(asks.try_iter());
        if queue.is_empty() {
            if !block {
                return true;
            }
            match asks.recv() {
                Ok(a) => queue.push(a),
                Err(_) => return false,
            }
        }
        let stale = queue.len().saturating_sub(8);
        for (key, _) in queue.drain(..stale) {
            if send.send((key, Err(String::new()))).is_err() {
                return false;
            }
        }
        let ((who, costume), md) = queue.pop().unwrap();
        let (mut m, mut t, mut i, mut b) = (Staged::new(providers.0.clone()), Staged::new(providers.1.clone()), Staged::new(providers.2.clone()), Staged::new(providers.3.clone()));
        let r = match (who, md) {
            (Who::Mod(_), Some(md)) => crate::mods::load(iso, &md, costume, &mut m, &mut t, &mut i, &mut b).map(|d| (d, md.hand)),
            (Who::Disc(c), _) => character::load_disc_with(iso, c, costume, &mut m, &mut t, &mut i, &mut b, Some(&inspect::MOTIONS)).map(|d| (d, inspect::disc_hand(iso, c))),
            (Who::Mod(_), None) => Err("no mod".into()),
        };
        let r = r.map(|(d, hand)| (d, hand, m, t, i, b)).map_err(|e| if e.is_empty() { "failed".into() } else { e });
        send.send(((who, costume), r)).is_ok()
    };
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        let Ok(mut iso) = Iso::open(&path) else { return };
        while work(&mut iso, true) {}
    });
    // no threads in the browser: `receive` runs a load a frame
    #[cfg(target_arch = "wasm32")]
    if let Ok(mut iso) = Iso::open(&path) {
        WORK.with(|w| *w.borrow_mut() = Some(Box::new(move || work(&mut iso, false))));
    }
    let name = cache_name(&args.iso);
    #[cfg(not(target_arch = "wasm32"))]
    let dir = {
        let root = super::Settings::path(&args.iso).with_file_name("cache").join("previews");
        // older drawings' folders go
        for e in std::fs::read_dir(&root).into_iter().flatten().flatten() {
            if e.file_name().to_string_lossy() != name.as_str() && e.path().is_dir() {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
        let dir = cache_dir(&args.iso, &name);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            warn!("{}: {e}", dir.display());
        }
        dir
    };
    // B120: the browser's cards are OPFS files named `<folder>~<file>` (`web.rs`); older drawings' go
    #[cfg(target_arch = "wasm32")]
    let dir = {
        crate::web::cards_keep(&format!("{name}~"));
        cache_dir(&args.iso, &name)
    };
    commands.insert_resource(Loader {
        ask,
        done: Mutex::new(done),
        asked: HashSet::new(),
        models: HashMap::new(),
        model_order: VecDeque::new(),
        pngs: HashMap::new(),
        png_order: VecDeque::new(),
        dir,
        stamps: HashMap::new(),
        extra: Vec::new(),
    });
}

/// Finished loads into the world: characters' assets, and PNGs.
fn receive(mut loader: ResMut<Loader>, mut menu: Option<ResMut<Menu>>, mut assets: (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>, ResMut<Assets<Image>>, ResMut<Assets<SkinnedMeshInverseBindposes>>)) {
    let (meshes, materials, images, binds) = &mut assets;
    let loader = &mut *loader;
    #[cfg(target_arch = "wasm32")]
    WORK.with(|w| w.borrow_mut().as_mut().map(|f| f()));
    for (key, r) in loader.done.get_mut().unwrap().try_iter() {
        let loaded = match r {
            Ok((d, hand, m, t, i, b)) => {
                (m.insert(meshes), t.insert(materials), i.insert(images), b.insert(binds));
                Some((Arc::new(d), hand))
            }
            Err(e) if e.is_empty() => {
                loader.asked.remove(&key);
                continue;
            }
            Err(e) => {
                warn!("select preview of {:?} costume {}: {e}", key.0, key.1);
                if let (Who::Mod(i), Some(menu)) = (key.0, menu.as_deref_mut()) {
                    broke(menu, i, key.1);
                }
                None
            }
        };
        loader.models.insert(key, loaded);
        loader.model_order.retain(|k| *k != key);
        loader.model_order.push_back(key);
    }
}

/// M1n: costume `k` of mod `i` failed to load: the select passes it, and a player still picking on it moves to its
/// next costume that hasn't failed.
fn broke(menu: &mut Menu, i: usize, k: usize) {
    menu.broken.insert((i, k));
    let n = menu.mods.get(i).map_or(0, |m| m.costumes.len());
    let next = (1..n).map(|d| (k + d) % n).find(|c| !menu.broken.contains(&(i, *c)));
    for pl in menu.players.iter_mut().filter(|pl| pl.modded && pl.pick == i && pl.costume == k && !pl.ready) {
        pl.costume = next.unwrap_or(k);
    }
}

/// Card-frames whose card showed another key or nothing though its PNG was read ([0]), the hovered one ([1]), or
/// nothing with no PNG baked yet ([2]); logged when `HST_STRESS` quits.
static CHECK: [std::sync::atomic::AtomicU32; 3] = [const { std::sync::atomic::AtomicU32::new(0) }; 3];

/// Which way the cards draw (`HST_PREVIEW`).
fn mode() -> &'static str {
    static MODE: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| std::env::var("HST_PREVIEW").unwrap_or_default());
    MODE.as_str()
}

#[allow(clippy::too_many_arguments)]
/// A preview camera and its rig's game space (the rig's parent), to despawn.
fn preview_entities(cam: Entity, cams: &Query<&Preview>, parents: &Query<&ChildOf, Without<Slot>>) -> Vec<Entity> {
    let space = cams.get(cam).ok().and_then(|c| parents.get(c.rig).ok()).map(|s| s.parent());
    space.into_iter().chain([cam]).collect()
}

fn sync(
    mut commands: Commands,
    menu: Res<Menu>,
    art: Option<Res<Art>>,
    bind: Res<Bindings>,
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut slots: Query<(&Slot, &ChildOf, &mut ZIndex), Without<Shot>>,
    mut nodes: Query<(&mut Node, &mut ZIndex, &mut UiTransform), With<Shot>>,
    cams: Query<&Preview>,
    parents: Query<&ChildOf, Without<Slot>>,
    mut images: ResMut<Assets<Image>>,
    mut ui: ResMut<Assets<PreviewMaterial>>,
    mut cards: Local<([Option<Shown>; 4], [Option<(Key, f32)>; 4])>,
    mut loader: ResMut<Loader>,
    mut gone: ResMut<Gone>,
    (baker, mut pending, slide, fixed, mut shows): (Res<Baker>, ResMut<Pending>, Res<super::slide::Slide>, Res<Time<Fixed>>, Query<&mut Visibility, With<Shot>>),
) {
    let Some(art) = art else { return };
    // the cards follow the select's slide in (`slide.rs`; no fade on a 3D card), and wait while the last screen leaves
    let mut quads = match slide.incoming() {
        // B192: and the confirm screen's players
        // B370: and the lobby's seat cards
        Some(_) if matches!(menu.screen, Screen::Chars | Screen::Confirm | Screen::Lobby) => layout(&menu, &art.2, &bind, &[], 0.0).0,
        _ => Vec::new(),
    };
    let dx = slide.incoming().map_or(0.0, |s| s.0);
    quads.iter_mut().for_each(|q| q.dst[0] += dx);
    let mut root = None;
    for (Slot(i), parent, mut z) in &mut slots {
        root = Some(parent.parent());
        if z.0 != *i as i32 {
            z.0 = *i as i32;
        }
    }
    let Some(root) = root else { return };
    let scale = window.single().map_or(1.0, |w| crate::play::widescreen::unit_px(w) * w.scale_factor()) * crate::play::widescreen::menu_scale();
    let (share, vshare) = crate::play::widescreen::window_menu_shares(&window);
    let now = time.elapsed_secs();
    let (shown, since) = &mut *cards;
    for (p, slot) in shown.iter_mut().enumerate() {
        let want = quads.iter().enumerate().find(|(_, q)| q.tex == Tex::Preview(p));
        let key = want.map(|_| menu.pick(p));
        if since[p].map(|s| s.0) != key {
            since[p] = key.map(|k| (k, now));
            pending.0[p] = None;
        }
        let png = match key {
            Some(k) if mode() != "live" => loader.png(k, &menu, &mut images),
            _ => None,
        };
        // a press, or a key not baked yet (nothing to show until the live model), loads the live model straight away;
        // B120: in the browser (a load runs inside a frame, ~30 ms on a phone) a held hover keeps the card
        let held = since[p].is_some_and(|(_, t)| now - t >= SETTLE) && !cfg!(target_arch = "wasm32");
        let settled = (held || pending.0[p].is_some() || png.is_none()) && mode() != "png";
        let modded = |w: Who| match w {
            Who::Mod(i) => menu.mods.get(i),
            Who::Disc(_) => None,
        };
        if let Some(k) = key
            && settled
        {
            loader.want(k, modded(k.0));
        }
        let live = key.filter(|_| settled).and_then(|k| loader.model(k).cloned().flatten());
        // a new key: its live model, else its PNG, else nothing yet; a PNG gives way to the live model once loaded
        let same = slot.as_ref().is_some_and(|s| Some(s.key) == key && want.is_some_and(|(_, q)| s.wh == [q.dst[2], q.dst[3]]));
        let upgrade = live.is_some() && slot.as_ref().is_some_and(|s| s.cam.is_none());
        let place = |node: &mut Node, [x, y, w, h]: [f32; 4], anchor| {
            let (left, width) = crate::play::widescreen::place(x, w, share, anchor);
            node.left = Val::Percent(left);
            let (top, height) = crate::play::widescreen::vplace_in(y, h, vshare, anchor);
            node.top = Val::Percent(top);
            node.width = Val::Percent(width);
            node.height = Val::Percent(height);
        };
        if !same || upgrade {
            // the PNG a live model replaces stays a few frames, the live node hidden until then (B140: shown under the
            // PNG it darkened the PNG's see-through parts, the racket strings)
            let mut cover = None;
            if let Some(s) = slot.take() {
                gone.0.extend(s.cover.map(|c| c.0));
                if let Some(cam) = s.cam {
                    gone.0.extend(preview_entities(cam, &cams, &parents));
                }
                if same && s.cam.is_none() {
                    cover = Some((s.node, COVER, fixed.elapsed_secs()));
                } else {
                    gone.0.push(s.node);
                }
            }
            if let (Some((w, costume)), Some((i, q))) = (key, want) {
                // placed as it spawns (else it draws one frame at the image's own size, top left)
                let mut node = Node { position_type: PositionType::Absolute, ..default() };
                place(&mut node, q.dst, q.anchor);
                // B140: flipped as it spawns (else its first frame, and the PNG it covers, draws the other hand)
                let flip = UiTransform { scale: Vec2::new(if menu.players[p].swap { -1.0 } else { 1.0 }, 1.0), ..default() };
                let node = (node, ZIndex(i as i32), flip);
                if let Some((data, hand)) = live {
                    // a mod poses with its donor's timing and yaws
                    let c = match w {
                        Who::Disc(c) => c,
                        Who::Mod(i) => menu.mods[i].donor,
                    };
                    // B402: its on-screen pixels (a menu unit is narrower than tall), so it draws the shape it shows
                    let image = inspect::target(&mut images, (Vec2::new(q.dst[2] * UNIT_W, q.dst[3]) * scale).as_uvec2());
                    let cam = inspect::spawn_into(&mut commands, &data, c, hand, 1 + p, image.clone());
                    commands.entity(cam).insert(Card(p));
                    let shown = if cover.is_some() { Visibility::Hidden } else { Visibility::Inherited };
                    let node = commands.spawn((Shot, node, shown, MaterialNode(ui.add(PreviewMaterial { image })))).id();
                    commands.entity(root).add_child(node);
                    *slot = Some(Shown { key: (w, costume), cam: Some(cam), node, cover, wh: [q.dst[2], q.dst[3]] });
                } else if let Some(png) = png {
                    let node = commands.spawn((Shot, node, ImageNode { image: png, image_mode: NodeImageMode::Stretch, ..default() })).id();
                    commands.entity(root).add_child(node);
                    *slot = Some(Shown { key: (w, costume), cam: None, node, cover: None, wh: [q.dst[2], q.dst[3]] });
                }
            } else if let Some((c, ..)) = cover {
                gone.0.push(c);
            }
        }
        if let Some(Shown { cover: cover @ Some(_), node, .. }) = slot.as_mut() {
            let (c, n, t) = cover.as_mut().unwrap();
            if *n > 0 {
                *n -= 1;
            } else if fixed.elapsed_secs() - *t >= SETTLE_TICKS as f32 / 60.0
                && let Ok(mut v) = shows.get_mut(*node)
            {
                // the live node shows a frame before its cover goes (its first shown frame draws nothing)
                if *v == Visibility::Hidden {
                    *v = Visibility::Inherited;
                } else {
                    gone.0.push(*c);
                    *cover = None;
                }
            }
        }
        for e in slot.iter().flat_map(|s| [Some(s.node), s.cover.map(|c| c.0)]).flatten() {
            let Some((i, q)) = want else { break };
            let Ok((mut node, mut z, mut flip)) = nodes.get_mut(e) else { continue };
            place(&mut node, q.dst, q.anchor);
            z.0 = i as i32;
            // the hand toggle: the drawing mirrored (the model faces the camera, so its mirror is the picture's)
            let x = if menu.players[p].swap { -1.0 } else { 1.0 };
            if flip.scale.x != x {
                flip.scale.x = x;
            }
        }
    }
    // the stress run's check: is a card on screen showing its hovered key (else blank or another character; a key not
    // baked yet counted apart)
    for (p, s) in shown.iter().enumerate() {
        if quads.iter().any(|q| q.tex == Tex::Preview(p)) {
            let key = menu.pick(p);
            let i = if s.as_ref().is_some_and(|s| s.key == key) { 1 } else if loader.pngs.get(&key).is_some_and(Option::is_some) { 0 } else { 2 };
            CHECK[i].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    let keep: Vec<Key> = shown.iter().flatten().map(|s| s.key).chain(since.iter().flatten().map(|s| s.0)).chain(baker.job.as_ref().map(|j| j.key)).collect();
    loader.trim(&keep);
}

/// The off-screen preview drawing uncached PNGs one at a time: hovered ones first (newest first), then every disc
/// character's costumes, then every mod's (`HST_BAKE=all` quits when done).
#[derive(Resource, Default)]
struct Baker {
    queue: Vec<Key>,
    job: Option<Job>,
    /// Keys whose drawing came back empty this run, not tried again.
    failed: HashSet<Key>,
    /// Frames the first job waits (pipelines compiling), then each.
    jobs: u32,
    /// B187: a first drawing has started (the menu's start draws one under its loading screen, so the previews'
    /// pipelines compile there, not on the select's first frame: a 0.7–1.8 s freeze on a 6×-throttled browser).
    warmed: bool,
    /// B335: store mods whose listing's `preview.png` was asked for (once a run), the one on its way, its answer.
    tried: HashSet<Key>,
    fetching: bool,
    got: Arc<Mutex<Option<(Key, PathBuf, Option<Vec<u8>>)>>>,
}

struct Job {
    key: Key,
    path: PathBuf,
    cam: Entity,
    image: Handle<Image>,
    frames: u32,
    /// The fixed clock at its spawn: the pose and face settle on `FixedUpdate` ticks, which a fast menu frame
    /// outruns (B140: a read-back 4 frames in caught the face before its mouth).
    start: f32,
    /// The read-back's entity, once asked; the result lands in `done`.
    reading: Option<Entity>,
    done: Arc<Mutex<Option<Vec<u8>>>>,
    /// The PNGs being written (true: written): the preview's, then its face close-up's (B160).
    saving: Vec<Task<bool>>,
    /// Drawing the face close-up (B160): the camera retargeted onto [`FACE`] at the preview's head square.
    face: bool,
    /// Every drawing read back; waiting for `saving`.
    drawn: bool,
    /// B187: only drawn for its pipelines (every card was cached): thrown away, not saved.
    warm: bool,
}

impl Job {
    /// Draw `key` (`data`, its rig's `hand`, posed with disc character `c`'s timing) off screen.
    #[allow(clippy::too_many_arguments)]
    fn start(commands: &mut Commands, images: &mut Assets<Image>, key: Key, path: PathBuf, data: &Arc<CharacterData>, c: usize, hand: f32, start: f32, warm: bool) -> Job {
        let image = inspect::target(images, sizes().0);
        images.get_mut(&image).unwrap().texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let cam = inspect::spawn_into(commands, data, c, hand, BAKE_LAYER, image.clone());
        Job { key, path, cam, image, frames: 0, start, reading: None, done: default(), saving: Vec::new(), face: false, drawn: false, warm }
    }
}

fn start_baker(mut commands: Commands, menu: Res<Menu>, mut loader: ResMut<Loader>) {
    // B259: each mod's stamp now, and every other mod drawing goes (an updated mod's old one, a removed mod's)
    let stamps: Vec<String> = menu.mods.iter().map(stamp).collect();
    #[cfg(not(target_arch = "wasm32"))]
    for e in std::fs::read_dir(&loader.dir).into_iter().flatten().flatten() {
        if stale(&e.file_name().to_string_lossy(), &stamps) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let pre = format!("{}~", card_name(&loader.dir));
        crate::web::cards_drop(|n| n.strip_prefix(pre.as_str()).is_some_and(|n| stale(n, &stamps)));
    }
    loader.stamps.extend(stamps.into_iter().enumerate());
    // every mod's costumes after the disc's (B99: an unbaked one is blank until its live model loads)
    let mut queue = Vec::new();
    // B187: in the browser (a load and a drawing's read-back inside a frame, a hitch on a phone) only the icons'
    // costume 0 in the background; any other costume is drawn when it's hovered
    let costumes = if cfg!(target_arch = "wasm32") { 1 } else { super::COSTUMES };
    for (i, m) in menu.mods.iter().enumerate().rev().filter(|_| costumes > 1) {
        queue.extend((1..m.costumes.len()).rev().map(|k| (Who::Mod(i), k)));
    }
    // costume 0 of every character first (popped last-in first), then each mod's (B133: the custom page's icons)
    for k in (0..costumes).rev() {
        if k == 0 {
            queue.extend((0..menu.mods.len()).rev().map(|i| (Who::Mod(i), 0)));
        }
        queue.extend(GRID.iter().rev().map(|&(c, ..)| (Who::Disc(c), k)));
    }
    commands.insert_resource(Baker { queue, ..default() });
}

#[allow(clippy::too_many_arguments)]
fn bake(
    mut commands: Commands,
    menu: Res<Menu>,
    mut baker: ResMut<Baker>,
    mut loader: ResMut<Loader>,
    mut images: ResMut<Assets<Image>>,
    mut gone: ResMut<Gone>,
    mut exit: MessageWriter<AppExit>,
    mut hovered: Local<Vec<Key>>,
    fixed: Res<Time<Fixed>>,
    cams: Query<&Preview>,
    parents: Query<&ChildOf, Without<Slot>>,
    compiling: Option<Res<crate::mode::Compiling>>,
    rigs: Query<&crate::character::Rig>,
    globals: Query<&GlobalTransform>,
    mut card: ResMut<ModCard>,
    mut loaded: Local<Option<Key>>,
) {
    // B229: on the Mods page only its hovered mod's costume 0 draws
    let asked = card.want.as_ref().filter(|_| menu.screen == Screen::Mods).map(|m| {
        // B259: an updated one's newest files
        let i = menu.mods.iter().position(|o| o.dir == m.dir).or_else(|| loader.extra.iter().position(|o| o.dir == m.dir).inspect(|&n| loader.extra[n] = m.clone()).map(|n| menu.mods.len() + n));
        (Who::Mod(i.unwrap_or_else(|| {
            loader.extra.push(m.clone());
            menu.mods.len() + loader.extra.len() - 1
        })), 0)
    });
    // the main screen's faces (`home::faces`): drawn there, first shown first, so they fill in without a select visit
    // the Mods page's shown cards' too (`mods_screen::faces`)
    let home: Vec<Key> = match menu.screen {
        Screen::Main => super::home::faces(&menu),
        Screen::Mods => super::mods_screen::faces(&menu),
        _ => Vec::new(),
    }
    .into_iter()
    .map(|w| (w, 0))
    .collect();
    if menu.screen != Screen::Chars && asked.is_none() && home.is_empty() && baker.job.is_none() && baker.warmed {
        return;
    }
    // a hovered card without a PNG jumps the queue
    let now: Vec<Key> = match menu.screen {
        Screen::Chars => (0..menu.players()).map(|p| menu.pick(p)).collect(),
        Screen::Main => home.iter().rev().copied().collect(),
        Screen::Mods => home.iter().rev().copied().chain(asked).collect(),
        _ => asked.into_iter().collect(),
    };
    for k in &now {
        if !hovered.contains(k) && !baker.failed.contains(k) {
            baker.queue.push(*k);
        }
    }
    *hovered = now;
    let baker = &mut *baker;
    if let Some(job) = &mut baker.job {
        job.frames += 1;
        // B142: a part whose pipeline is still compiling isn't drawn (a mod's first MASK material: Emi's body and
        // hair), and an uncapped menu runs `wait` frames in milliseconds; count from the last compile
        if job.reading.is_none() && compiling.is_some_and(|c| c.busy()) {
            job.frames = 0;
        }
        let wait = if baker.jobs == 0 { 30 } else { 4 };
        let ticked = fixed.elapsed_secs() - job.start >= SETTLE_TICKS as f32 / 60.0;
        let data = job.done.lock().unwrap().take();
        if job.warm && data.is_some() {
            gone.0.extend(preview_entities(job.cam, &cams, &parents));
            gone.0.extend(job.reading.take());
            baker.job = None;
            baker.jobs += 1;
        } else if job.drawn {
            // Z1: the PNG encodes (~20 ms) run on the IO pool; the next job waits for the files
            if job.saving.iter().all(|t| t.is_finished()) {
                let ok: Vec<bool> = job.saving.drain(..).map(|t| bevy::tasks::block_on(t)).collect();
                if ok.first() == Some(&true) {
                    info!("baked {}", job.path.display());
                } else {
                    baker.failed.insert(job.key);
                }
                card.baked += 1;
                baker.job = None;
                baker.jobs += 1;
            }
        } else if let Some(data) = data {
            if let Some(r) = job.reading.take() {
                gone.0.push(r);
            }
            let (bake, face) = sizes();
            let size = if job.face { face } else { bake };
            let rgba = straight(&data, size);
            let path = if job.face { face_path(&job.path) } else { job.path.clone() };
            // B176: framed on the rig's head bone
            let head = cams.get(job.cam).ok().and_then(|p| inspect::head_px(p, bake.as_vec2(), &rigs, &globals));
            let square = (!job.face).then(|| image::RgbaImage::from_raw(size.x, size.y, rgba.clone())).flatten().and_then(|i| inspect::face_square(&i, head))
                .map(|sq| if job.key.0 == Who::Disc(0) { inspect::grow_square(sq, ASHLEY_ZOOM, size.x, size.y) } else { sq });
            job.saving.push(IoTaskPool::get().spawn(async move { save(rgba, size, &path) }));
            // B160: the same drawing's head square again at [`FACE`]
            if let (Some(sq), Ok(p)) = (square, cams.get(job.cam)) {
                let crop = inspect::sub_crop(p.crop, bake.as_vec2(), sq.as_rect());
                let image = inspect::target(&mut images, face);
                images.get_mut(&image).unwrap().texture_descriptor.usage |= TextureUsages::COPY_SRC;
                commands.entity(job.cam).insert((
                    Preview { rect: p.rect, crop, n: p.n, rig: p.rig, centre: p.centre },
                    bevy::camera::RenderTarget::Image(bevy::camera::ImageRenderTarget { handle: image.clone(), scale_factor: 1.0 }),
                ));
                (job.image, job.face, job.frames) = (image, true, 0);
            } else {
                gone.0.extend(preview_entities(job.cam, &cams, &parents));
                job.drawn = true;
            }
        } else if job.frames >= wait && ticked && job.reading.is_none() {
            let done = job.done.clone();
            let r = commands
                .spawn(Readback::texture(job.image.clone()))
                .observe(move |ev: On<ReadbackComplete>, mut commands: Commands| {
                    *done.lock().unwrap() = Some(ev.data.clone());
                    commands.entity(ev.entity).try_remove::<Readback>();
                })
                .id();
            job.reading = Some(r);
        }
        return;
    }
    // B335: a store mod's listing picture arrived: its drawing and close-up, else it's drawn as any other
    if baker.fetching {
        let Some((key, path, png)) = baker.got.lock().unwrap().take() else { return };
        baker.fetching = false;
        if png.is_some_and(|b| from_store(&b, &path)) {
            loader.pngs.remove(&key);
            loader.png_order.retain(|k| *k != key);
            card.baked += 1;
        }
    }
    // the next key without a PNG
    while let Some(&key) = baker.queue.last() {
        if menu.screen != Screen::Chars && Some(key) != asked && !home.contains(&key) && baker.warmed {
            return;
        }
        let path = loader.path(key, &menu);
        // B160: a baked one without its face close-up (a run quit between the two) is drawn again
        let done = |p: &PathBuf| cached(p) && (!p.starts_with(&loader.dir) || cached(&face_path(p)));
        let Some(path) = path.filter(|p| !done(p) && !baker.failed.contains(&key)) else {
            baker.queue.pop();
            continue;
        };
        let m = match key.0 {
            Who::Mod(i) => loader.mod_at(&menu, i).cloned(),
            Who::Disc(_) => None,
        };
        // B335: a store mod's costume 0 is its listing's picture (a phone ran out of memory drawing 40 mods)
        if let Some(m) = m.as_ref().filter(|_| key.1 == 0 && baker.tried.insert(key)) {
            let (got, p) = (baker.got.clone(), path.clone());
            if crate::modstore::installed_preview(&m.dir, &m.id, move |png| *got.lock().unwrap() = Some((key, p, png))) {
                baker.fetching = true;
                return;
            }
        }
        loader.want(key, m.as_ref());
        match loader.model(key).cloned() {
            None => return,
            Some(None) => {
                baker.failed.insert(key);
                baker.queue.pop();
            }
            Some(Some((data, hand))) => {
                // B187: drawn from the frame after its load (in the browser a load runs inside a frame), not on top
                if *loaded != Some(key) {
                    *loaded = Some(key);
                    return;
                }
                baker.queue.pop();
                let c = match key.0 {
                    Who::Disc(c) => c,
                    Who::Mod(i) => loader.mod_at(&menu, i).map_or(0, |m| m.donor),
                };
                baker.job = Some(Job::start(&mut commands, &mut images, key, path, &data, c, hand, fixed.elapsed_secs(), false));
                baker.warmed = true;
                return;
            }
        }
    }
    // B187: every card cached: the first one drawn anyway at the menu's start, for its pipelines
    if !baker.warmed {
        let key = (Who::Disc(GRID[0].0), 0);
        loader.want(key, None);
        match loader.model(key).cloned() {
            None => return,
            Some(None) => baker.warmed = true,
            Some(Some((data, hand))) => {
                baker.job = Some(Job::start(&mut commands, &mut images, key, PathBuf::new(), &data, GRID[0].0, hand, fixed.elapsed_secs(), true));
                baker.warmed = true;
                return;
            }
        }
    }
    if std::env::var("HST_BAKE").is_ok_and(|v| v == "all") {
        info!("baked every preview");
        exit.write(AppExit::Success);
    }
}

/// The baker's read-back (`size`, `Rgba16Float`: GS-encoded colour, alpha −1 where nothing drew, as `inspect.wgsl`
/// reads it) as sRGB bytes with straight alpha: what `inspect.wgsl` would draw.
fn straight(data: &[u8], size: UVec2) -> Vec<u8> {
    let half = |b: &[u8]| {
        let h = u16::from_le_bytes([b[0], b[1]]);
        let (s, e, m) = ((h >> 15) as u32, ((h >> 10) & 0x1f) as u32, (h & 0x3ff) as u32);
        let f = match e {
            0 => (m as f32) * 2f32.powi(-24),
            31 => f32::INFINITY,
            _ => f32::from_bits((e + 112) << 23 | m << 13),
        };
        if s == 1 { -f } else { f }
    };
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut rgba = Vec::with_capacity((size.x * size.y * 4) as usize);
    for px in data.chunks_exact(8).take((size.x * size.y) as usize) {
        rgba.extend([byte(half(&px[0..])), byte(half(&px[2..])), byte(half(&px[4..])), byte(half(&px[6..]) + 1.0)]);
    }
    rgba
}

/// [`straight`]'s bytes as a PNG at `path`. False if nothing drew.
fn save(rgba: Vec<u8>, size: UVec2, path: &Path) -> bool {
    if !rgba.chunks_exact(4).any(|p| p[3] > 0) {
        warn!("{}: the preview drew nothing", path.display());
        return false;
    }
    // B120: fast deflate (~4× quicker than the default, a bigger file): in the browser the encode runs inside a frame
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new_with_quality(&mut png, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::Adaptive);
    let r = image::ImageEncoder::write_image(encoder, &rgba, size.x, size.y, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string());
    match r.and_then(|()| write(path, png).map_err(|e| e.to_string())) {
        Ok(()) => true,
        Err(e) => {
            warn!("{}: {e}", path.display());
            false
        }
    }
}

/// B160: a baked preview's face close-up beside it (`d05-0.png` → `d05-0-face.png`).
fn face_path(preview: &Path) -> PathBuf {
    let stem = preview.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    preview.with_file_name(format!("{stem}-face.png"))
}

/// B335: a store mod's listing picture (`preview.png`, the uploader's baked card) as `path`'s drawing at this
/// device's size and its close-up (the head found by [`inspect::face_square`] without a head bone).
fn from_store(png: &[u8], path: &Path) -> bool {
    use image::imageops::{self, FilterType};
    let Ok(img) = image::load_from_memory_with_format(png, image::ImageFormat::Png) else { return false };
    let (bake, face) = sizes();
    let img = imageops::resize(&img.into_rgba8(), bake.x, bake.y, FilterType::Triangle);
    let Some(sq) = inspect::face_square(&img, None) else { return false };
    let head = imageops::resize(&imageops::crop_imm(&img, sq.min.x, sq.min.y, sq.width(), sq.height()).to_image(), face.x, face.y, FilterType::Triangle);
    save(head.into_raw(), face, &face_path(path)) && save(img.into_raw(), bake, path)
}

/// B160: a face icon (`inner` out of 64, [`inspect::face_icon`]) from `preview`'s close-up, else (a mod's own preview
/// PNG, which has none) cut from the preview itself; None until baked.
fn face_from(preview: &Path, inner: u32) -> Option<Image> {
    match read(&face_path(preview)) {
        Some(b) => inspect::icon(&b, inner),
        None => inspect::face_icon(&read(preview)?, inner),
    }
}

/// Whether the card cache has `path`: a file natively, an OPFS file in the browser (B120, `web.rs`).
fn cached(path: &Path) -> bool {
    #[cfg(target_arch = "wasm32")]
    return crate::web::card(&card_name(path)).is_some() || crate::modfs::is_file(path);
    #[cfg(not(target_arch = "wasm32"))]
    path.is_file()
}

fn read(path: &Path) -> Option<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    // a mod's own previews/ (its files are in modfs, not the card cache)
    return crate::web::card(&card_name(path)).map(|b| b.to_vec()).or_else(|| crate::modfs::read(path).ok());
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::read(path).ok()
}

fn write(path: &Path, png: Vec<u8>) -> std::io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    return crate::web::put_card(card_name(path), png);
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::write(path, png)
}

/// A cache path's OPFS file name (no folders there): `<folder>~<file>`.
#[cfg(target_arch = "wasm32")]
fn card_name(path: &Path) -> String {
    path.to_string_lossy().replace(['/', '\\'], "~")
}

/// Previews to despawn, in `Last`: by then every `Update`/`PostUpdate` system that reacted to a new rig (noise,
/// shade, …) has had its commands applied, so none lands on a despawned entity.
#[derive(Resource, Default)]
struct Gone(Vec<Entity>);

fn despawn(mut commands: Commands, mut gone: ResMut<Gone>) {
    for e in gone.0.drain(..) {
        commands.entity(e).try_despawn();
    }
}

/// A preview's UI node.
#[derive(Component)]
struct Shot;

/// A preview camera's player.
#[derive(Component)]
struct Card(usize);

/// Each seated player's R2 / L2 on their own preview.
fn buttons(
    menu: Res<Menu>,
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut cams: Query<(&mut Preview, &Card)>,
    data: Query<&character::Rig>,
    mut rigs: Query<(&mut Motion, &mut Transform, &mut inspect::UvClock)>,
    mut pending: ResMut<Pending>,
    (args, sound, mut voices): (Res<Args>, Option<Res<Sound>>, ResMut<Voices>),
) {
    if menu.screen != Screen::Chars {
        return;
    }
    for player in 0..4 {
        let Some(dev) = menu.seats[player] else { continue };
        let pressed = |b: GamepadButton, k: bool| match dev {
            Dev::Keys => k,
            Dev::Pad(e) => pads.get(e).is_ok_and(|g| g.just_pressed(b)),
            Dev::Open => false,
        };
        if pressed(GamepadButton::RightTrigger2, keys.just_pressed(KeyCode::Digit1)) {
            pending.0[player] = Some(Act::Win);
        } else if pressed(GamepadButton::LeftTrigger2, keys.just_pressed(KeyCode::Digit2)) {
            pending.0[player] = Some(Act::Loss);
        }
    }
    // a card still on its PNG keeps the press until `sync` has loaded and spawned its live model
    for (mut p, Card(player)) in &mut cams {
        let Some(act) = pending.0[*player] else { continue };
        let Ok(rig) = data.get(p.rig) else { continue };
        pending.0[*player] = None;
        // the original drops a press, motion and all, while that screen's R2/L2 voice still sounds
        let busy = |id: u64| id != 0 && sound.as_ref().is_some_and(|s| s.playing(id));
        if busy(voices.playing[*player]) {
            continue;
        }
        let d = rig.data.clone();
        inspect::apply(&mut p, act, &mut rigs, &d);
        let Some(s) = &sound else { continue };
        if let Some((bank, play)) = voice(act, menu.pick(*player).0, menu.pick(*player).1, &menu.mods, &mut voices.banks, Path::new(&args.iso)) {
            voices.playing[*player] = s.play_centre(&bank, play);
        }
    }
}

/// Each card's last R2/L2 voice (play id), and the banks it came from.
#[derive(Resource, Default)]
struct Voices {
    playing: [u64; 4],
    banks: HashMap<String, Option<Arc<SoundBank>>>,
}

/// The voice a press on a card plays as its motion starts (B86a), as the original's Data → Items → Characters screen,
/// whose inspect the cards copy: R2 / L2 key 0 / 1 (the happy / upset take; pad bits 0x200 / 0x100) of program 0 of
/// the character's `itgNN_n` bank (bank slot 0x16; `_c` from costume 5 on, the Costumes tab). △ shows the stats, no
/// voice (user 2026-10-12). A mod: R2 / L2 its own `voice/` program 9 / 10 key 0 (the match's voices
/// for the same motions, 0x2e / 0x2f); a take it lacks, or a mod with no voice at all: silent, never its donor's
/// (B181). A costume pack (`costume_of`) is its disc character: that character's costume-0 R2 / L2.
fn voice(act: Act, who: Who, costume: usize, mods: &[Mod], banks: &mut HashMap<String, Option<Arc<SoundBank>>>, iso: &Path) -> Option<(Arc<SoundBank>, sound::Play)> {
    let (c, costume) = match who {
        Who::Disc(c) => (c, costume),
        Who::Mod(i) => {
            let m = mods.get(i).filter(|m| crate::modfs::ready(&m.dir))?;
            let own = banks.entry(format!("mod {}", m.dir.join(&m.voice).display())).or_insert_with(|| crate::mods::voice(m).map(Arc::new)).clone();
            let program: u8 = match act { Act::Win => 9, Act::Loss => 10, Act::Pose => return None };
            if let Some(own) = own.filter(|o| o.wav.contains_key(&(program as usize))) {
                return Some((own, sound::Play { slot: 0x16, program, key: 0, volume: 0x80, speed: 1.0 }));
            }
            (m.costume_of?, 0)
        }
    };
    if act == Act::Pose {
        return None;
    }
    let v = if costume >= 5 { 'c' } else { 'n' };
    let (xb, hd) = (format!("SND/VOICE/MENU/ITG{c:02}_{}.XB0", v.to_ascii_uppercase()), format!("data/sound/VOICE/MENU/itg{c:02}_{v}.hd"));
    let play = sound::Play { slot: 0x16, program: 0, key: (act == Act::Loss) as u8, volume: 0x80, speed: 1.0 };
    let bank = banks.entry(hd.clone()).or_insert_with(|| Iso::open(iso).ok().and_then(|mut iso| SoundBank::load(&mut iso, &xb, &hd)).map(Arc::new)).clone()?;
    Some((bank, play))
}

/// Each player's R2/L2 pressed on a card not yet showing its live model.
#[derive(Resource, Default)]
struct Pending([Option<Act>; 4]);

/// `HST_SELECT=c,c,c,c`: every player seated on the keyboard, hovering those characters (grid order is not
/// character order: each is looked up); `c/k` on costume `k`; a trailing `!` swaps the hand (B140).
fn hover_all(mut menu: ResMut<Menu>) {
    let Ok(list) = std::env::var("HST_SELECT") else { return };
    for (p, c) in list.split(',').map(str::trim).take(4).enumerate() {
        let (c, swap) = c.strip_suffix('!').map_or((c, false), |c| (c, true));
        menu.players[p].swap = swap;
        let (c, k) = c.split_once('/').unwrap_or((c, "0"));
        menu.players[p].costume = k.parse().unwrap_or(0);
        if let Some(i) = c.strip_prefix('m').and_then(|i| i.parse::<usize>().ok()).filter(|&i| i < menu.mods.len()) {
            menu.players[p].pick = i;
            menu.players[p].modded = true;
            menu.seats[p] = Some(Dev::Keys);
        } else if let Some(cursor) = c.parse::<usize>().ok().and_then(|c| super::GRID.iter().position(|g| g.0 == c)) {
            menu.players[p].cursor = cursor;
            menu.seats[p] = Some(Dev::Keys);
        }
    }
}

/// `HST_STRESS=<seconds>` (with `HST_MENU=chars`): every frame, player 1's keyboard and the other three cards jump
/// to a random character, costume, page or mod, as fast as the select can go, then the app quits (B86's crash hunt).
fn stress(secs: f32) -> impl FnMut(ResMut<Menu>, Res<Time>, MessageWriter<AppExit>, Local<u64>) {
    let mut hold = 0.0;
    move |mut menu, time, mut exit, mut seed| {
        if time.elapsed_secs() > secs {
            let [bad, good, unbaked] = CHECK.each_ref().map(|c| c.load(std::sync::atomic::Ordering::Relaxed));
            info!("stress: {bad} card-frames not showing the hovered key, {good} showing it, {unbaked} waiting for a bake");
            exit.write(AppExit::Success);
            return;
        }
        let mut rnd = |n: usize| {
            // xorshift
            *seed = if *seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { *seed };
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            (*seed % n.max(1) as u64) as usize
        };
        // its own presses never leave the select (nor save settings: their effects are dropped)
        menu.screen = Screen::Chars;
        let m = &mut *menu;
        // now and then the cards hold still long enough for their live models
        if time.elapsed_secs() < hold {
            return;
        }
        if rnd(120) == 0 {
            hold = time.elapsed_secs() + 0.4 + rnd(10) as f32 * 0.1;
            return;
        }
        let press = match rnd(6) {
            0 => super::Press { right: true, ..default() },
            1 => super::Press { down: true, ..default() },
            2 => super::Press { next: true, ..default() },
            3 => super::Press { prev: true, ..default() },
            4 if rnd(20) == 0 => super::Press { page: true, ..default() },
            _ => super::Press { left: true, ..default() },
        };
        m.step(Dev::Keys, press);
        m.screen = Screen::Chars;
        for p in 1..m.players() {
            m.seats[p] = Some(Dev::Keys);
            let pl = &mut m.players[p];
            pl.modded = !m.mods.is_empty() && rnd(2) == 1;
            if pl.modded {
                pl.pick = rnd(m.mods.len());
            } else {
                pl.cursor = rnd(GRID.len());
            }
            let n = if pl.modded { m.mods[pl.pick].costumes.len() } else { super::COSTUMES };
            pl.costume = rnd(n);
        }
    }
}

/// `HST_SHOTS=<dir>`: every frame's window as `<dir>/NNNN.png`, frames 1–299 (with `HST_STRESS` 60–299: after the
/// stress run's first second; B140: alone, a card's PNG → live swap).
fn shots(dir: String, from: u32) -> impl FnMut(Commands, Local<u32>) {
    move |mut commands, mut n| {
        *n += 1;
        if (from..300).contains(&*n) {
            let path = format!("{dir}/{:04}.png", *n);
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B335: a store picture (any size) becomes this device's drawing and a close-up of its head.
    #[test]
    fn a_store_picture_stands_in_for_a_drawing() {
        let mut img = image::RgbaImage::new(200, 250);
        for (x, y, p) in img.enumerate_pixels_mut() {
            if (90..110).contains(&x) && y > 40 {
                *p = image::Rgba([200, 100, 50, 255]);
            }
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let dir = std::env::temp_dir().join(format!("hst-b335-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("m-x-0.png");
        assert!(from_store(&png, &path) && !from_store(b"not a png", &dir.join("m-y-0.png")));
        let dims = |p: &Path| image::open(p).unwrap().into_rgba8().dimensions();
        assert_eq!(dims(&path), (BAKE.x, BAKE.y));
        assert_eq!(dims(&face_path(&path)), (FACE.x, FACE.y));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A card's load (B102) has the preview's three motions, the same as a full load's, for every disc character.
    #[test]
    fn preview_loads_only_its_motions() {
        let iso = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"));
        let Ok(mut disc) = Iso::open(iso) else { return eprintln!("no ISO, skipped") };
        let (mut m, mut t, mut i, mut b) = (Assets::<Mesh>::default(), Assets::<StandardMaterial>::default(), Assets::<Image>::default(), Assets::<SkinnedMeshInverseBindposes>::default());
        for (c, ..) in GRID {
            let part = character::load_disc_with(&mut disc, c, 0, &mut m, &mut t, &mut i, &mut b, Some(&inspect::MOTIONS)).unwrap();
            let full = character::load_disc(&mut disc, c, 0, &mut m, &mut t, &mut i, &mut b).unwrap();
            let mut ids: Vec<usize> = part.motions.keys().copied().collect();
            ids.sort();
            assert_eq!(ids, [0x2e, 0x2f, character::POSE], "character {c}");
            for id in ids {
                assert_eq!(part.motions[&id].length, full.motions[&id].length, "character {c} motion {id:#x}");
                assert_eq!(part.motions[&id].locals(&part.skeleton, 3.0), full.motions[&id].locals(&full.skeleton, 3.0), "character {c} motion {id:#x}");
            }
            assert_eq!(part.faces.keys().collect::<HashSet<_>>(), full.faces.keys().filter(|k| inspect::MOTIONS.contains(k)).collect(), "character {c} faces");
        }
    }

    /// B259: a mod's stamp follows any file's change, a store update (store.json) and an added file; its old
    /// drawings (and only its) are stale.
    #[test]
    fn updated_mod_gets_a_new_stamp() {
        let dir = Path::new("t_b259/anya");
        let put = |f: &str, b: &[u8]| crate::modfs::add(&dir.join(f), b.to_vec().into());
        put("mod.json", b"{}");
        put("textures/body.png", b"one");
        let a = dir_stamp(dir);
        assert_eq!(a, dir_stamp(dir));
        put("textures/body.png", b"three");
        let b = dir_stamp(dir);
        put("store.json", br#"{"sha256":"1"}"#);
        let c = dir_stamp(dir);
        put("store.json", br#"{"sha256":"2"}"#);
        let d = dir_stamp(dir);
        put("voice/win.ogg", b"x");
        let e = dir_stamp(dir);
        let all = [&a, &b, &c, &d, &e];
        assert!(all.iter().enumerate().all(|(i, x)| x.starts_with("anya-") && all[..i].iter().all(|y| y != x)), "{all:?}");
        let now = [e.clone(), "zed-00".into()];
        assert!(stale(&format!("m-{a}-0.png"), &now) && stale(&format!("m-{d}-1-face.png"), &now));
        assert!(!stale(&format!("m-{e}-0.png"), &now) && !stale(&format!("m-{e}-2-face.png"), &now) && !stale("d05-0.png", &now));
        crate::modfs::remove(Path::new("t_b259"));
    }

    /// R2 / L2 on a disc character and a mod start the matching voice (B86a).
    #[test]
    fn presses_start_voices() {
        let iso = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"));
        let Ok(mut disc) = Iso::open(iso) else { return eprintln!("no ISO, skipped") };
        let sound = Sound::test(&mut disc);
        let dir = std::env::temp_dir().join(format!("hst-b86a-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("voice")).unwrap();
        // a 0.1 s silent PCM16 mono wav as the mod's R2 voice (program 9); no L2 take, so L2 is silent
        let mut wav = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x22\x56\0\0\x44\xac\0\0\x02\0\x10\0data".to_vec();
        wav.extend((4410u32 * 2).to_le_bytes());
        wav.extend(vec![0u8; 4410 * 2]);
        std::fs::write(dir.join("voice/9_0.wav"), &wav).unwrap();
        let m = |voice: &str| Mod {
            dir: dir.clone(), id: "t".into(), name: "t".into(), source: String::new(), costumes: vec![], donor: 3, hand: 1.0, base: 3,
            overrides: vec![], ai_row: 3, texture_face: false, no_face: false, voice: voice.into(), motions: None, costume_of: None, skeleton: None,
        };
        let mods = [m("voice/"), m("none/")];
        let mut banks = HashMap::new();
        let mut go = |act, who, costume| {
            let (bank, play) = voice(act, who, costume, &mods, &mut banks, iso).unwrap_or_else(|| panic!("no voice {act:?} {who:?} {costume}"));
            let id = sound.play_centre(&bank, play);
            assert!(sound.playing(id), "{act:?} {who:?} {costume}");
            (Arc::as_ptr(&bank), play.slot, play.program, play.key)
        };
        let (win, s, p, k) = go(Act::Win, Who::Disc(2), 0);
        // B105: R2 the happy take (key 0), L2 the upset one (key 1), as the original's 0x200 / 0x100 handlers
        assert_eq!((s, p, k), (0x16, 0, 0));
        assert_eq!(go(Act::Loss, Who::Disc(2), 0), (win, 0x16, 0, 1));
        assert_ne!(go(Act::Win, Who::Disc(2), 5).0, win, "costume 5 on: the _c bank");
        let tail = |(_, s, p, k)| (s, p, k);
        assert!(voice(Act::Pose, Who::Disc(2), 0, &mods, &mut HashMap::new(), iso).is_none(), "△ is the stats, no voice");
        // the mod: its own R2 take; L2 silent (B181: it lacks that take, never its donor's); no voice → silent
        assert_eq!(tail(go(Act::Win, Who::Mod(0), 7)), (0x16, 9, 0));
        let mut none = |act, who| voice(act, who, 7, &mods, &mut HashMap::new(), iso).is_none();
        assert!(none(Act::Loss, Who::Mod(0)));
        assert!(none(Act::Pose, Who::Mod(0)));
        assert!(none(Act::Win, Who::Mod(1)) && none(Act::Loss, Who::Mod(1)));
        // a costume pack is its disc character: that character's takes
        let donor = go(Act::Win, Who::Disc(3), 0).0;
        let mods = [Mod { costume_of: Some(3), ..m("none/") }];
        let (bank, play) = voice(Act::Loss, Who::Mod(0), 7, &mods, &mut banks, iso).unwrap();
        assert_eq!((Arc::as_ptr(&bank), play.program, play.key), (donor, 0, 1));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
