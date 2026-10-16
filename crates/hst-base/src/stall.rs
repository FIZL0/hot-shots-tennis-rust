//! `HST_FPS=1` (P31): what a stalled frame spent its time on, one line per stall into the log. The game side: its
//! update, the gap to the next frame (extract, window events, present back-pressure), disc bytes read and entities
//! spawned. The render side, per render frame: asset upload (`PrepareAssets`), specialisation, the swapchain
//! acquire, queue and prepare, the pipeline queue, graph + encode + submit; with the textures and meshes uploaded
//! and the pipelines finished that frame. Pipeline compiles run on the async pool off the render thread (not on
//! macOS or the web), so a compile shows as `ready +N`, not as render time, unless it holds a lock the frame needs.

use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{CachedPipelineState, PipelineCache};
use bevy::render::{Render, RenderApp, RenderSystems as R};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

/// `eprintln!` for the `HST_FPS` log: on the web (no stderr there) the browser's console (B453: `?fps` in the URL).
#[macro_export]
macro_rules! diag {
    ($($t:tt)*) => {{
        #[cfg(target_arch = "wasm32")]
        bevy::log::info!($($t)*);
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!($($t)*);
    }};
}
pub use crate::diag;

/// A frame or a stage of one this long counts as a stall (ms).
const STALL: f32 = 50.0;

pub fn plugin(app: &mut App) {
    app.add_systems(First, game_start).add_systems(Last, game_end);
    let Some(r) = app.get_sub_app_mut(RenderApp) else { return };
    use bevy::render::view::prepare_windows;
    use bevy::render::renderer::{RenderGraph, RenderGraphSystems as G};
    r.init_resource::<Uploaded>().add_systems(RenderGraph, mark::<5>.in_set(G::Begin)).add_systems(
        Render,
        (
            mark::<0>.in_set(R::ExtractCommands),
            mark::<1>.in_set(R::PrepareMeshes),
            mark::<2>.in_set(R::PrepareViews).before(prepare_windows),
            mark::<3>.in_set(R::PrepareViews).after(prepare_windows),
            mark::<4>.in_set(R::Prepare).after(R::PrepareBindGroups),
            render_end.in_set(R::Cleanup),
        ),
    );
}

/// The render frame's stage marks (`plugin`'s order).
static MARKS: Mutex<[Option<Instant>; 6]> = Mutex::new([None; 6]);

fn mark<const I: usize>() {
    MARKS.lock().unwrap()[I] = Some(Instant::now());
}

/// What the render world held last frame: textures, their bytes, meshes, finished pipelines.
#[derive(Resource, Default)]
struct Uploaded {
    images: usize,
    bytes: u64,
    meshes: usize,
    ready: usize,
}

fn render_end(images: Res<RenderAssets<bevy::render::texture::GpuImage>>, meshes: Res<RenderAssets<bevy::render::mesh::RenderMesh>>, cache: Res<PipelineCache>, mut last: ResMut<Uploaded>) {
    let end = Instant::now();
    let m = std::mem::take(&mut *MARKS.lock().unwrap());
    let bytes = images
        .iter()
        .map(|(_, i)| {
            let d = &i.texture_descriptor;
            let texels = d.size.width as u64 * d.size.height as u64 * d.size.depth_or_array_layers as u64;
            texels * d.format.block_copy_size(None).unwrap_or(4) as u64 / (d.format.block_dimensions().0 * d.format.block_dimensions().1) as u64
        })
        .sum();
    let (mut ready, mut creating) = (0, 0);
    for p in cache.pipelines() {
        match p.state {
            CachedPipelineState::Ok(_) => ready += 1,
            CachedPipelineState::Creating(_) | CachedPipelineState::Queued => creating += 1,
            CachedPipelineState::Err(_) => {}
        }
    }
    let now = Uploaded { images: images.iter().count(), bytes, meshes: meshes.iter().count(), ready };
    let [Some(t0), Some(t1), Some(t2), Some(t3), Some(t4), Some(t5)] = m else {
        *last = now;
        return;
    };
    let ms = |a: Instant, b: Instant| b.saturating_duration_since(a).as_secs_f32() * 1000.0;
    // when each pipeline lands: one that lands after the loading screen is down was compiled mid-match
    let watched = watched(1);
    if now.ready > last.ready && ms(t0, end) <= STALL && !watched {
        diag!("pipelines ready +{} ({} in all, compiling {creating}) at {:.2} s", now.ready - last.ready, now.ready, since_start());
    }
    if ms(t0, end) > STALL || watched {
        diag!(
            "render{} {:.1} ms: assets {:.1} (images +{} {:.1} MB, meshes +{}) specialize {:.1} acquire {:.1} queue+prepare {:.1} pipeline queue {:.1} (ready +{}, compiling {creating}) graph+submit {:.1} at {:.2} s",
            if watched { "" } else { " stall" },
            ms(t0, end),
            ms(t0, t1),
            now.images.saturating_sub(last.images),
            now.bytes.saturating_sub(last.bytes) as f32 / 1e6,
            now.meshes.saturating_sub(last.meshes),
            ms(t1, t2),
            ms(t2, t3),
            ms(t3, t4),
            ms(t4, t5),
            now.ready.saturating_sub(last.ready),
            ms(t5, end),
            since_start(),
        );
    }
    *last = now;
}

/// The process's first frame (the log's clock, close to `Time<Real>`'s).
fn start() -> Instant {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// Frames left to log in full whatever their length, game side and render side (`watch`).
static WATCH: [AtomicU32; 2] = [AtomicU32::new(0), AtomicU32::new(0)];

/// The next frames get logged in full: the loading screen just came down (`mode::loaded`).
pub fn watch() {
    for w in &WATCH {
        w.store(10, Ordering::Relaxed);
    }
}

/// Whether this frame is one of `watch`'s (side 0 game, 1 render).
fn watched(side: usize) -> bool {
    WATCH[side].fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok()
}

/// Seconds since the first frame, as the stall lines give them.
pub fn since_start() -> f32 {
    start().elapsed().as_secs_f32()
}

/// The game side's last frame: its start, its update's end, disc bytes read and entities by then.
static GAME: Mutex<(Option<Instant>, Option<Instant>, u64, u32)> = Mutex::new((None, None, 0, 0));

fn game_start(world: &mut World) {
    let now = Instant::now();
    start();
    let read = hst_data::iso::READ.load(std::sync::atomic::Ordering::Relaxed);
    let entities = world.entities().count_spawned();
    let mut g = GAME.lock().unwrap();
    if let (Some(start), Some(end)) = (g.0, g.1) {
        let ms = |a: Instant, b: Instant| b.saturating_duration_since(a).as_secs_f32() * 1000.0;
        let (game, gap) = (ms(start, end), ms(end, now));
        let watched = watched(0);
        if game > STALL || gap > STALL || watched {
            diag!("game{}: update {game:.1} ms, then {gap:.1} ms to the next frame (extract, events, present); disc +{:.1} MB, entities {entities} ({:+}) at {:.2} s", if watched { "" } else { " stall" }, (read - g.2) as f32 / 1e6, entities as i64 - g.3 as i64, since_start());
        }
    }
    *g = (Some(now), None, read, entities);
}

fn game_end() {
    GAME.lock().unwrap().1 = Some(Instant::now());
}
