//! The select cards' 3D previews (B40b1): each card a player is picking on shows its hovered character in its costume
//! as the original's inspect screen draws it (`inspect.rs`), in place of the portrait, under the card's text. That
//! player's R2 / L2 / △ (keyboard: 1, 2 and the attributes key) play the inspect screen's win, loss and back.
//!
//! B86: a card shows a PNG of the preview (character × costume) while its player scrolls, and the live model only once
//! the hover has held [`SETTLE`] seconds (the live one is what R2/L2/△ act on). The PNGs are drawn here, once, by an
//! off-screen preview (the baker: [`BAKE`] pixels, the cards' crop and lighting) and read back into a local cache
//! beside settings.txt (`cache/previews/<disc boot file>-r<RENDER>/`): disc characters' images are made from the
//! user's disc, so they are never shipped, and are all baked in the background from the first launch on (the hovered
//! uncached one first); a later launch only reads them. A mod may ship its own as `previews/<costume>.png` (the mods
//! repo's standard); else its costumes bake on hover like the disc's. `HST_BAKE=all` also bakes every mod's
//! costumes and quits when done (the mods repo's packaging). Bump [`RENDER`] when the preview's drawing changes.
//!
//! Characters load on a worker thread (each one decodes its texture replacements and motions, ~130 ms), the newest ask
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
use hst_data::iso::Iso;

use super::{Art, Dev, GRID, Menu, Screen, Slot, Tex, Who, layout};
use crate::mods::Mod;
use crate::character::{self, CharacterData, Motion, Staged};
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use crate::inspect::{self, Act, Preview, PreviewMaterial};
use crate::Args;
use crate::play::controls::{Action, Bindings};

/// The baked previews' drawing version: part of the cache folder's name.
const RENDER: u32 = 1;
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
        .init_resource::<Gone>().init_resource::<Pending>()
        .add_systems(Startup, start_loader)
        .add_systems(PostStartup, (hover_all, start_baker))
        .add_systems(FixedUpdate, character::tick)
        .add_systems(Update, ((receive, sync, bake, buttons).chain().after(super::draw), character::animate))
        .add_systems(Last, despawn);
    if let Some(secs) = std::env::var("HST_STRESS").ok().and_then(|s| s.parse::<f32>().ok()) {
        app.add_systems(Update, stress(secs).before(super::step));
    }
}

type Key = (Who, usize);

/// What a card shows: its key, the live model's camera (else a PNG) and its UI node.
struct Shown {
    key: Key,
    cam: Option<Entity>,
    node: Entity,
}

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
    png_ask: Sender<(Key, PathBuf)>,
    png_done: Mutex<Receiver<(Key, Option<Image>)>>,
    /// PNGs by key: `None` while reading or where there is none.
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

    /// The PNG of `key` if it is read, asking for it if not; marked as just used.
    fn png(&mut self, key: Key, menu: &Menu) -> Option<Handle<Image>> {
        if let Some(i) = self.png_order.iter().position(|k| *k == key) {
            self.png_order.remove(i);
            self.png_order.push_back(key);
            return self.pngs.get(&key).cloned().flatten();
        }
        self.pngs.insert(key, None);
        self.png_order.push_back(key);
        if let Some(path) = self.path(key, menu).filter(|p| p.is_file()) {
            let _ = self.png_ask.send((key, path));
        }
        while self.png_order.len() > KEEP_PNGS {
            let old = self.png_order.pop_front().unwrap();
            self.pngs.remove(&old);
        }
        None
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

    /// Forget `key`'s PNG (a fresh bake), so it is read again.
    fn forget_png(&mut self, key: Key) {
        self.pngs.remove(&key);
        self.png_order.retain(|k| *k != key);
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

fn start_loader(mut commands: Commands, args: Res<Args>, meshes: Res<Assets<Mesh>>, materials: Res<Assets<StandardMaterial>>, images: Res<Assets<Image>>, binds: Res<Assets<SkinnedMeshInverseBindposes>>) {
    let (ask, asks) = std::sync::mpsc::channel::<(Key, Option<Mod>)>();
    let (send, done) = std::sync::mpsc::channel();
    let providers = (meshes.get_handle_provider(), materials.get_handle_provider(), images.get_handle_provider(), binds.get_handle_provider());
    let path = args.iso.clone();
    std::thread::spawn(move || {
        let Ok(mut iso) = Iso::open(&path) else { return };
        // newest ask first: the card just hovered loads before older hovers; a backlog past a few is stale
        let mut queue = Vec::new();
        loop {
            queue.extend(asks.try_iter());
            if queue.is_empty() {
                match asks.recv() {
                    Ok(a) => queue.push(a),
                    Err(_) => return,
                }
            }
            let stale = queue.len().saturating_sub(8);
            for (key, _) in queue.drain(..stale) {
                if send.send((key, Err(String::new()))).is_err() {
                    return;
                }
            }
            let ((who, costume), md) = queue.pop().unwrap();
            let (mut m, mut t, mut i, mut b) = (Staged::new(providers.0.clone()), Staged::new(providers.1.clone()), Staged::new(providers.2.clone()), Staged::new(providers.3.clone()));
            let r = match (who, md) {
                (Who::Mod(_), Some(md)) => crate::mods::load(&mut iso, &md, costume, &mut m, &mut t, &mut i, &mut b).map(|d| (d, md.hand)),
                (Who::Disc(c), _) => character::load_disc(&mut iso, c, costume, &mut m, &mut t, &mut i, &mut b).map(|d| (d, inspect::disc_hand(&mut iso, c))),
                (Who::Mod(_), None) => Err("no mod".into()),
            };
            let r = r.map(|(d, hand)| (d, hand, m, t, i, b)).map_err(|e| if e.is_empty() { "failed".into() } else { e });
            if send.send(((who, costume), r)).is_err() {
                return;
            }
        }
    });
    let (png_ask, png_asks) = std::sync::mpsc::channel::<(Key, PathBuf)>();
    let (png_send, png_done) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(first) = png_asks.recv() {
            // newest first, as the models
            let mut queue = vec![first];
            while let Some((key, path)) = {
                queue.extend(png_asks.try_iter());
                queue.pop()
            } {
                let image = std::fs::read(&path).ok().and_then(|b| {
                    Image::from_buffer(&b, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), RenderAssetUsages::RENDER_WORLD)
                        .map_err(|e| warn!("{}: {e}", path.display()))
                        .ok()
                });
                if png_send.send((key, image)).is_err() {
                    return;
                }
            }
        }
    });
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
        png_ask,
        png_done: Mutex::new(png_done),
        pngs: HashMap::new(),
        png_order: VecDeque::new(),
        dir,
        stamps: HashMap::new(),
    });
}

/// Finished loads into the world: characters' assets, and PNGs.
fn receive(mut loader: ResMut<Loader>, mut assets: (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>, ResMut<Assets<Image>>, ResMut<Assets<SkinnedMeshInverseBindposes>>)) {
    let (meshes, materials, images, binds) = &mut assets;
    let loader = &mut *loader;
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
                None
            }
        };
        loader.models.insert(key, loaded);
        loader.model_order.retain(|k| *k != key);
        loader.model_order.push_back(key);
    }
    for (key, image) in loader.png_done.get_mut().unwrap().try_iter() {
        // a PNG read after its entry was dropped is not kept
        if let (Some(slot), Some(image)) = (loader.pngs.get_mut(&key), image) {
            *slot = Some(images.add(image));
        }
    }
}

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
    mut nodes: Query<(&mut Node, &mut ZIndex), With<Shot>>,
    cams: Query<&Preview>,
    parents: Query<&ChildOf, Without<Slot>>,
    mut images: ResMut<Assets<Image>>,
    mut ui: ResMut<Assets<PreviewMaterial>>,
    mut cards: Local<([Option<Shown>; 4], [Option<(Key, f32)>; 4])>,
    mut loader: ResMut<Loader>,
    mut gone: ResMut<Gone>,
    (baker, mut pending): (Res<Baker>, ResMut<Pending>),
) {
    let Some(art) = art else { return };
    let quads = if menu.screen == Screen::Chars { layout(&menu, &art.2, &bind, &[], 0.0, 0).0 } else { Vec::new() };
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
        let key = want.map(|_| (menu.who(p), menu.players[p].costume));
        if since[p].map(|s| s.0) != key {
            since[p] = key.map(|k| (k, now));
            pending.0[p] = None;
        }
        // a press loads the live model straight away
        let settled = (since[p].is_some_and(|(_, t)| now - t >= SETTLE) || pending.0[p].is_some()) && mode() != "png";
        let modded = |w: Who| match w {
            Who::Mod(i) => menu.mods.get(i),
            Who::Disc(_) => None,
        };
        let png = match key {
            Some(k) if mode() != "live" => loader.png(k, &menu),
            _ => None,
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
        if !same || upgrade {
            if let Some(s) = slot.take() {
                if let Some(cam) = s.cam {
                    gone.0.extend(preview_entities(cam, &cams, &parents));
                }
                gone.0.push(s.node);
            }
            if let (Some((w, costume)), Some((_, q))) = (key, want) {
                let node = Node { position_type: PositionType::Absolute, ..default() };
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
                    *slot = Some(Shown { key: (w, costume), cam: Some(cam), node });
                } else if let Some(png) = png {
                    let node = commands.spawn((Shot, node, ImageNode::new(png))).id();
                    commands.entity(root).add_child(node);
                    *slot = Some(Shown { key: (w, costume), cam: None, node });
                }
            }
        }
        if let (Some(s), Some((i, q))) = (slot.as_ref(), want)
            && let Ok((mut node, mut z)) = nodes.get_mut(s.node)
        {
            let [x, y, w, h] = q.dst;
            let (left, width) = crate::play::widescreen::place(x, w, crate::play::widescreen::window_share(&window), q.anchor);
            node.left = Val::Percent(left);
            node.top = Val::Percent(y / 4.48);
            node.width = Val::Percent(width);
            node.height = Val::Percent(h / 4.48);
            z.0 = i as i32;
        }
    }
    let keep: Vec<Key> = shown.iter().flatten().map(|s| s.key).chain(since.iter().flatten().map(|s| s.0)).chain(baker.job.as_ref().map(|j| j.key)).collect();
    loader.trim(&keep);
}

/// The off-screen preview drawing uncached PNGs one at a time: hovered ones first (newest first), then every disc
/// character's costumes (and with `HST_BAKE=all` every mod's).
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
}

fn start_baker(mut commands: Commands, menu: Res<Menu>) {
    let mut queue = Vec::new();
    if std::env::var("HST_BAKE").is_ok_and(|v| v == "all") {
        for (i, m) in menu.mods.iter().enumerate().rev() {
            queue.extend((0..m.costumes.len()).rev().map(|k| (Who::Mod(i), k)));
        }
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
    if menu.screen != Screen::Chars && baker.job.is_none() {
        return;
    }
    // a hovered card without a PNG jumps the queue
    let now: Vec<Key> = (0..menu.players()).map(|p| (menu.who(p), menu.players[p].costume)).collect();
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
        if let Some(data) = data {
            let key = job.key;
            if let Some(r) = job.reading {
                gone.0.push(r);
            }
            gone.0.extend(preview_entities(job.cam, &cams, &parents));
            if save(&data, &job.path) {
                info!("baked {}", job.path.display());
                loader.forget_png(key);
            } else {
                baker.failed.insert(key);
            }
            baker.job = None;
            baker.jobs += 1;
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
                baker.job = Some(Job { key, path, cam, image, frames: 0, reading: None, done: default() });
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
        let d = rig.data.clone();
        inspect::apply(&mut p, act, &mut rigs, &d);
    }
}

/// Each player's R2/L2/△ pressed on a card not yet showing its live model.
#[derive(Resource, Default)]
struct Pending([Option<Act>; 4]);

/// `HST_SELECT=c,c,c,c`: every player seated on the keyboard, hovering those characters (grid order is not
/// character order: each is looked up).
fn hover_all(mut menu: ResMut<Menu>) {
    let Ok(list) = std::env::var("HST_SELECT") else { return };
    for (p, c) in list.split(',').map(str::trim).take(4).enumerate() {
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
