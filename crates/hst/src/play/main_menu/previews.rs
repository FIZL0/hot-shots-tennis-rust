//! The select cards' 3D previews (B40b1): each card a player is picking on shows its hovered character in its costume
//! as the original's inspect screen draws it (`inspect.rs`), in place of the portrait, under the card's text. That
//! player's R2 / L2 / △ (keyboard: 1, 2 and the attributes key) play the inspect screen's win, loss and back.
//!
//! B86: a card shows a PNG of the preview (character × costume) while its player scrolls, and the live model only once
//! the hover has held [`SETTLE`] seconds (B99: or at once when there is no PNG yet; the PNG is read on the frame its
//! key is hovered, and stays over the live model its first [`COVER`] frames) (the live one is what R2/L2/△ act on). The PNGs are drawn here, once, by an
//! off-screen preview (the baker: [`BAKE`] pixels, the cards' crop and lighting) and read back into a local cache
//! beside settings.txt (`cache/previews/<disc boot file>-r<RENDER>/`): disc characters' images are made from the
//! user's disc, so they are never shipped, and are all baked in the background from the first launch on (the hovered
//! uncached one first); a later launch only reads them. A mod may ship its own as `previews/<costume>.png` (the mods
//! repo's standard); else its costumes bake in the background after the disc's (a hovered one first). `HST_BAKE=all`
//! quits once every mod's costumes are baked (the mods repo's packaging). Bump [`RENDER`] when the preview's drawing changes.
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
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::PrimaryWindow;
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};
use hst_data::iso::Iso;

use super::{Art, Dev, GRID, Menu, Screen, Slot, Tex, Who, layout};
use crate::mods::Mod;
use crate::audio::{Sound, SoundBank};
use hst_sim::sound;
use crate::character::{self, CharacterData, Motion, Staged};
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use crate::inspect::{self, Act, Preview, PreviewMaterial};
use crate::Args;
use crate::play::controls::{Action, Bindings};

/// The baked previews' drawing version: part of the cache folder's name.
const RENDER: u32 = 3;
/// A baked preview's size: the card's 128×160 at 3× (sharp on a window up to 1344 lines); a multiple of 64 bytes a
/// row, so the read-back has no row padding.
const BAKE: UVec2 = UVec2::new(384, 480);
/// The baker's render layer (the cards use 1–4).
const BAKE_LAYER: usize = 6;
/// Loaded characters kept (four live cards, the baker's, one spare) and PNGs kept.
const KEEP_MODELS: usize = 6;
const KEEP_PNGS: usize = 48;
/// Seconds a card's hover holds before its live model replaces the PNG.
const SETTLE: f32 = 0.3;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(inspect::plugin)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<Gone>().init_resource::<Pending>().init_resource::<Voices>()
        .add_systems(Startup, start_loader)
        .add_systems(PostStartup, (hover_all, start_baker))
        .add_systems(FixedUpdate, character::tick)
        .add_systems(Update, ((receive, sync, bake, buttons).chain().after(super::draw), character::animate))
        .add_systems(Last, despawn);
    if let Some(secs) = std::env::var("HST_STRESS").ok().and_then(|s| s.parse::<f32>().ok()) {
        app.add_systems(Update, stress(secs).before(super::step));
        if let Ok(dir) = std::env::var("HST_SHOTS") {
            app.add_systems(Last, shots(dir));
        }
    }
}

type Key = (Who, usize);

/// What a card shows: its key, the live model's camera (else a PNG) and its UI node; a live one's PNG it replaced, kept
/// on top for its last frames (B99).
struct Shown {
    key: Key,
    cam: Option<Entity>,
    node: Entity,
    cover: Option<(Entity, u32)>,
}

/// Frames a PNG stays over the live model that replaced it, while that model's first frames draw (its textures and
/// pipelines can land a few frames late: a blank or part-drawn card otherwise).
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
}

impl Loader {
    /// Load `key` (`m`: the mod, for a custom one).
    fn want(&mut self, key: Key, m: Option<&Mod>) {
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
        let path = self.path(key, menu).filter(|p| p.is_file())?;
        let image = std::fs::read(&path).ok().and_then(|b| {
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
                let m = menu.mods.get(i)?;
                // a mod may ship its own: `previews/<costume's glb stem>.png`
                let own = m.costumes.get(k).and_then(|c| Path::new(c).file_stem()).map(|s| m.dir.join("previews").join(s).with_extension("png"));
                if let Some(own) = own.filter(|p| p.is_file()) {
                    return Some(own);
                }
                let stamp = self.stamps.entry(i).or_insert_with(|| stamp(m));
                Some(self.dir.join(format!("m-{stamp}-{k}.png")))
            }
        }
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

/// A mod's PNG name stem: its folder's name and its newest file's time, so an edited mod is baked again.
fn stamp(m: &Mod) -> String {
    let files = [Some("mod.json"), m.motions.as_deref()].into_iter().flatten().map(String::from).chain(m.costumes.iter().cloned());
    let t = files.filter_map(|f| std::fs::metadata(m.dir.join(f)).and_then(|d| d.modified()).ok()).max();
    let secs = t.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    let name: String = m.dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '-' }).collect();
    format!("{name}-{secs:x}")
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
    let mut iso = Iso::open(&args.iso).ok();
    let boot = iso.as_mut().and_then(boot_file).unwrap_or_else(|| "disc".into());
    let root = super::Settings::path(&args.iso).with_file_name("cache").join("previews");
    let name = format!("{boot}-r{RENDER}");
    // older drawings' folders go
    for e in std::fs::read_dir(&root).into_iter().flatten().flatten() {
        if e.file_name().to_string_lossy() != name.as_str() && e.path().is_dir() {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
    let dir = root.join(name);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        warn!("{}: {e}", dir.display());
    }
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
    (baker, mut pending, slide): (Res<Baker>, ResMut<Pending>, Res<super::slide::Slide>),
) {
    let Some(art) = art else { return };
    // the cards follow the select's slide in (`slide.rs`; no fade on a 3D card), and wait while the last screen leaves
    let mut quads = match slide.incoming() {
        Some(_) if menu.screen == Screen::Chars => layout(&menu, &art.2, &bind, &[], 0.0).0,
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
    let scale = window.single().map_or(1.0, |w| w.physical_height() as f32 / 448.0);
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
        // a press, or a key not baked yet (nothing to show until the live model), loads the live model straight away
        let settled = (since[p].is_some_and(|(_, t)| now - t >= SETTLE) || pending.0[p].is_some() || png.is_none()) && mode() != "png";
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
        let same = slot.as_ref().is_some_and(|s| Some(s.key) == key);
        let upgrade = live.is_some() && slot.as_ref().is_some_and(|s| s.cam.is_none());
        let place = |node: &mut Node, [x, y, w, h]: [f32; 4], anchor| {
            let (left, width) = crate::play::widescreen::place(x, w, crate::play::widescreen::window_share(&window), anchor);
            node.left = Val::Percent(left);
            node.top = Val::Percent(y / 4.48);
            node.width = Val::Percent(width);
            node.height = Val::Percent(h / 4.48);
        };
        if !same || upgrade {
            // the PNG a live model replaces stays on top of it a few frames
            let mut cover = None;
            if let Some(s) = slot.take() {
                if let Some(cam) = s.cam {
                    gone.0.extend(preview_entities(cam, &cams, &parents));
                }
                if same && s.cam.is_none() {
                    cover = Some((s.node, COVER));
                } else {
                    gone.0.push(s.node);
                }
            }
            if let (Some((w, costume)), Some((i, q))) = (key, want) {
                // placed as it spawns (else it draws one frame at the image's own size, top left)
                let mut node = Node { position_type: PositionType::Absolute, ..default() };
                place(&mut node, q.dst, q.anchor);
                let node = (node, ZIndex(i as i32));
                if let Some((data, hand)) = live {
                    // a mod poses with its donor's timing and yaws
                    let c = match w {
                        Who::Disc(c) => c,
                        Who::Mod(i) => menu.mods[i].donor,
                    };
                    let image = inspect::target(&mut images, (Vec2::new(q.dst[2], q.dst[3]) * scale).as_uvec2());
                    let cam = inspect::spawn_into(&mut commands, &data, c, hand, 1 + p, image.clone());
                    commands.entity(cam).insert(Card(p));
                    let node = commands.spawn((Shot, node, MaterialNode(ui.add(PreviewMaterial { image })))).id();
                    commands.entity(root).add_child(node);
                    if let Some((c, _)) = cover {
                        let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
                        commands.entity(c).insert((full, ZIndex(0)));
                        commands.entity(node).add_child(c);
                    }
                    *slot = Some(Shown { key: (w, costume), cam: Some(cam), node, cover });
                } else if let Some(png) = png {
                    let node = commands.spawn((Shot, node, ImageNode::new(png))).id();
                    commands.entity(root).add_child(node);
                    *slot = Some(Shown { key: (w, costume), cam: None, node, cover: None });
                }
            } else if let Some((c, _)) = cover {
                gone.0.push(c);
            }
        }
        if let Some(Shown { cover: cover @ Some(_), .. }) = slot.as_mut() {
            let (c, n) = cover.as_mut().unwrap();
            *n -= 1;
            if *n == 0 {
                gone.0.push(*c);
                *cover = None;
            }
        }
        if let (Some(s), Some((i, q))) = (slot.as_ref(), want)
            && let Ok((mut node, mut z, mut flip)) = nodes.get_mut(s.node)
        {
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
}

struct Job {
    key: Key,
    path: PathBuf,
    cam: Entity,
    image: Handle<Image>,
    frames: u32,
    /// The read-back's entity, once asked; the result lands in `done`.
    reading: Option<Entity>,
    done: Arc<Mutex<Option<Vec<u8>>>>,
    /// The PNG being written (true: written).
    saving: Option<Task<bool>>,
}

fn start_baker(mut commands: Commands, menu: Res<Menu>) {
    // every mod's costumes after the disc's (B99: an unbaked one is blank until its live model loads)
    let mut queue = Vec::new();
    for (i, m) in menu.mods.iter().enumerate().rev() {
        queue.extend((0..m.costumes.len()).rev().map(|k| (Who::Mod(i), k)));
    }
    // costume 0 of every character first (popped last-in first)
    for k in (0..super::COSTUMES).rev() {
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
    cams: Query<&Preview>,
    parents: Query<&ChildOf, Without<Slot>>,
) {
    // ponytail: no PNG cache in the browser (no files): its cards always draw the live model; an OPFS cache if
    // scrolling there hitches
    if cfg!(target_arch = "wasm32") || menu.screen != Screen::Chars && baker.job.is_none() {
        return;
    }
    // a hovered card without a PNG jumps the queue
    let now: Vec<Key> = (0..menu.players()).map(|p| menu.pick(p)).collect();
    for k in &now {
        if !hovered.contains(k) && !baker.failed.contains(k) {
            baker.queue.push(*k);
        }
    }
    *hovered = now;
    let baker = &mut *baker;
    if let Some(job) = &mut baker.job {
        job.frames += 1;
        let wait = if baker.jobs == 0 { 30 } else { 4 };
        let data = job.done.lock().unwrap().take();
        if let Some(saving) = &mut job.saving {
            // Z1: the PNG encode (~20 ms) runs on the IO pool; the next job waits for the file
            if let Some(ok) = check_ready(saving) {
                if ok {
                    info!("baked {}", job.path.display());
                } else {
                    baker.failed.insert(job.key);
                }
                baker.job = None;
                baker.jobs += 1;
            }
        } else if let Some(data) = data {
            if let Some(r) = job.reading {
                gone.0.push(r);
            }
            gone.0.extend(preview_entities(job.cam, &cams, &parents));
            let path = job.path.clone();
            job.saving = Some(IoTaskPool::get().spawn(async move { save(&data, &path) }));
        } else if job.frames >= wait && job.reading.is_none() {
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
    // the next key without a PNG
    while let Some(&key) = baker.queue.last() {
        let path = loader.path(key, &menu);
        let Some(path) = path.filter(|p| !p.is_file() && !baker.failed.contains(&key)) else {
            baker.queue.pop();
            continue;
        };
        let m = match key.0 {
            Who::Mod(i) => Some(&menu.mods[i]),
            Who::Disc(_) => None,
        };
        loader.want(key, m);
        match loader.model(key).cloned() {
            None => return,
            Some(None) => {
                baker.failed.insert(key);
                baker.queue.pop();
            }
            Some(Some((data, hand))) => {
                baker.queue.pop();
                let c = match key.0 {
                    Who::Disc(c) => c,
                    Who::Mod(i) => menu.mods[i].donor,
                };
                let image = inspect::target(&mut images, BAKE);
                images.get_mut(&image).unwrap().texture_descriptor.usage |= TextureUsages::COPY_SRC;
                let cam = inspect::spawn_into(&mut commands, &data, c, hand, BAKE_LAYER, image.clone());
                baker.job = Some(Job { key, path, cam, image, frames: 0, reading: None, done: default(), saving: None });
                return;
            }
        }
    }
    if std::env::var("HST_BAKE").is_ok_and(|v| v == "all") {
        info!("baked every preview");
        exit.write(AppExit::Success);
    }
}

/// The baker's read-back (`BAKE`, `Rgba16Float`: GS-encoded colour, alpha −1 where nothing drew, as
/// `inspect.wgsl` reads it) as an sRGB PNG with straight alpha: what `inspect.wgsl` would draw. False if nothing drew.
fn save(data: &[u8], path: &Path) -> bool {
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
    let mut rgba = Vec::with_capacity((BAKE.x * BAKE.y * 4) as usize);
    for px in data.chunks_exact(8).take((BAKE.x * BAKE.y) as usize) {
        rgba.extend([byte(half(&px[0..])), byte(half(&px[2..])), byte(half(&px[4..])), byte(half(&px[6..]) + 1.0)]);
    }
    if !rgba.chunks_exact(4).any(|p| p[3] > 0) {
        warn!("{}: the preview drew nothing", path.display());
        return false;
    }
    let image = Image::new(Extent3d { width: BAKE.x, height: BAKE.y, depth_or_array_layers: 1 }, TextureDimension::D2, rgba, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::MAIN_WORLD);
    match image.try_into_dynamic().map_err(|e| e.to_string()).and_then(|d| d.to_rgba8().save(path).map_err(|e| e.to_string())) {
        Ok(()) => true,
        Err(e) => {
            warn!("{}: {e}", path.display());
            false
        }
    }
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

/// Each seated player's R2 / L2 / △ on their own preview.
fn buttons(
    menu: Res<Menu>,
    keys: Res<ButtonInput<KeyCode>>,
    bind: Res<Bindings>,
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
        };
        if pressed(GamepadButton::RightTrigger2, keys.just_pressed(KeyCode::Digit1)) {
            pending.0[player] = Some(Act::Win);
        } else if pressed(GamepadButton::LeftTrigger2, keys.just_pressed(KeyCode::Digit2)) {
            pending.0[player] = Some(Act::Loss);
        } else if pressed(GamepadButton::North, bind.key_just_pressed(&keys, Action::Lob)) {
            pending.0[player] = Some(Act::Pose);
        }
    }
    // a card still on its PNG keeps the press until `sync` has loaded and spawned its live model
    for (mut p, Card(player)) in &mut cams {
        let Some(act) = pending.0[*player] else { continue };
        let Ok(rig) = data.get(p.rig) else { continue };
        pending.0[*player] = None;
        // the original drops a press, motion and all, while that screen's R2/L2 voice still sounds
        let busy = |id: u64| id != 0 && sound.as_ref().is_some_and(|s| s.playing(id));
        let [itg, sample] = voices.playing[*player];
        if busy(itg) {
            continue;
        }
        let d = rig.data.clone();
        inspect::apply(&mut p, act, &mut rigs, &d);
        let Some(s) = &sound else { continue };
        if act == Act::Pose && busy(sample) {
            continue;
        }
        if let Some((bank, play)) = voice(act, menu.pick(*player).0, menu.pick(*player).1, &menu.mods, &mut voices.banks, Path::new(&args.iso)) {
            voices.playing[*player][(act == Act::Pose) as usize] = s.play_centre(&bank, play);
        }
    }
}

/// Each card's last R2/L2 voice and △ sample (play ids), and the banks they came from.
#[derive(Resource, Default)]
struct Voices {
    playing: [[u64; 2]; 4],
    banks: HashMap<String, Option<Arc<SoundBank>>>,
}

/// The voice a press on a card plays as its motion starts (B86a), as the original's Data → Items → Characters screen,
/// whose inspect the cards copy: R2 / L2 key 0 / 1 (the happy / upset take; pad bits 0x200 / 0x100) of program 0 of
/// the character's `itgNN_n` bank (bank slot 0x16; `_c` from costume 5 on, the Costumes tab), △ the "Voice Sample",
/// key = character of `mvc_it0` (slot 0x13; `mvc_it1`, slot 0x14, from costume 5 on). A mod: R2 / L2 its own `voice/` program 9 / 10 key 0 (the match's voices
/// for the same motions, 0x2e / 0x2f), nothing for a take it lacks (B105: another person's voice would stand in); a mod
/// with no voice at all: its donor's costume-0 R2 / L2 and sample.
fn voice(act: Act, who: Who, costume: usize, mods: &[Mod], banks: &mut HashMap<String, Option<Arc<SoundBank>>>, iso: &Path) -> Option<(Arc<SoundBank>, sound::Play)> {
    let (c, costume) = match who {
        Who::Disc(c) => (c, costume),
        Who::Mod(i) => {
            let m = mods.get(i)?;
            let own = banks.entry(format!("mod {}", m.dir.join(&m.voice).display())).or_insert_with(|| crate::mods::voice(m).map(Arc::new)).clone();
            if let Some(own) = own {
                // ponytail: no mod take stands in for the sample (the standard has none); a mod with voices gives none
                let program: u8 = match act { Act::Win => 9, Act::Loss => 10, Act::Pose => return None };
                return own.wav.contains_key(&(program as usize)).then(|| (own, sound::Play { slot: 0x16, program, key: 0, volume: 0x80, speed: 1.0 }));
            }
            (m.donor, 0)
        }
    };
    let alt = costume >= 5;
    let (xb, hd, play) = match act {
        Act::Win | Act::Loss => {
            let v = if alt { 'c' } else { 'n' };
            (format!("SND/VOICE/MENU/ITG{c:02}_{}.XB0", v.to_ascii_uppercase()), format!("data/sound/VOICE/MENU/itg{c:02}_{v}.hd"), sound::Play { slot: 0x16, program: 0, key: (act == Act::Loss) as u8, volume: 0x80, speed: 1.0 })
        }
        Act::Pose => {
            let n = alt as u8;
            (format!("SND/VOICE/MENU/MVC_IT{n}.XB0"), format!("data/sound/VOICE/MENU/mvc_it{n}.hd"), sound::Play { slot: 0x13 + n, program: 0, key: c as u8, volume: 0x80, speed: 1.0 })
        }
    };
    let bank = banks.entry(hd.clone()).or_insert_with(|| Iso::open(iso).ok().and_then(|mut iso| SoundBank::load(&mut iso, &xb, &hd)).map(Arc::new)).clone()?;
    Some((bank, play))
}

/// Each player's R2/L2/△ pressed on a card not yet showing its live model.
#[derive(Resource, Default)]
struct Pending([Option<Act>; 4]);

/// `HST_SELECT=c,c,c,c`: every player seated on the keyboard, hovering those characters (grid order is not
/// character order: each is looked up); `c/k` on costume `k`.
fn hover_all(mut menu: ResMut<Menu>) {
    let Ok(list) = std::env::var("HST_SELECT") else { return };
    for (p, c) in list.split(',').map(str::trim).take(4).enumerate() {
        let (c, k) = c.split_once('/').unwrap_or((c, "0"));
        menu.players[p].costume = k.parse().unwrap_or(0);
        if let Some(i) = c.strip_prefix('m').and_then(|i| i.parse::<usize>().ok()).filter(|&i| i < menu.mods.len()) {
            menu.custom = true;
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
            pl.modded = m.custom && !m.mods.is_empty();
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

/// `HST_SHOTS=<dir>` (with `HST_STRESS`): every frame's window as `<dir>/NNNN.png`, frames 60–299 (after the stress
/// run's first second).
fn shots(dir: String) -> impl FnMut(Commands, Local<u32>) {
    move |mut commands, mut n| {
        *n += 1;
        if (60..300).contains(&*n) {
            let path = format!("{dir}/{:04}.png", *n);
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// R2 / L2 / △ on a disc character and a mod start the matching voice (B86a).
    #[test]
    fn presses_start_voices() {
        let iso = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"));
        let Ok(mut disc) = Iso::open(iso) else { return eprintln!("no ISO, skipped") };
        let sound = Sound::test(&mut disc);
        let dir = std::env::temp_dir().join(format!("hst-b86a-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("voice")).unwrap();
        // a 0.1 s silent PCM16 mono wav as the mod's R2 voice (program 9); no L2 take, so L2 plays nothing
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
        assert_eq!(tail(go(Act::Pose, Who::Disc(2), 0)), (0x13, 0, 2));
        assert_eq!(go(Act::Pose, Who::Disc(2), 7).1, 0x14);
        // the mod: its own R2 take; L2 and △ none (it has a voice, not the donor's); no voice → all the donor's
        assert_eq!(tail(go(Act::Win, Who::Mod(0), 7)), (0x16, 9, 0));
        let donor = go(Act::Win, Who::Disc(3), 0).0;
        assert!(voice(Act::Loss, Who::Mod(0), 7, &mods, &mut HashMap::new(), iso).is_none());
        assert!(voice(Act::Pose, Who::Mod(0), 0, &mods, &mut HashMap::new(), iso).is_none());
        assert_eq!(go(Act::Win, Who::Mod(1), 7), (donor, 0x16, 0, 0));
        assert_eq!(go(Act::Loss, Who::Mod(1), 7), (donor, 0x16, 0, 1));
        assert_eq!(tail(go(Act::Pose, Who::Mod(1), 7)), (0x13, 0, 3));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
