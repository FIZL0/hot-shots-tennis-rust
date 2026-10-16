//! Video settings (remaster-only; P53: the Auto preset, picked per device and timed in matches, below): borderless fullscreen or a window, frame limit, Show FPS, vsync, render resolution scale, shadow quality, MSAA and the original
//! 640×448 mode (`hud_gamma.rs` draws it; it also fixes the aspect ratio to 4:3 and turns upscaled textures off, `main.rs`).
//! The menu edits the [`Graphics`] resource (applied live) and passes it to a match as flags
//! (`--windowed`, `--fps N`, `--vsync`, `--scale N`, `--shadows off|low|high`, `--msaa off|2x|4x`, `--original`, `--mesh N`). The simulation stays a fixed
//! 60 Hz tick whatever these are. Defaults draw as before these settings existed.

use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shadows {
    Off,
    Low,
    High,
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Graphics {
    /// Frame limit in fps, 0 = uncapped.
    pub fps: u32,
    pub vsync: bool,
    /// The 3D scene's render size in percent of the window's (upscaled to it; the HUD stays at full size).
    pub scale: u32,
    pub shadows: Shadows,
    /// MSAA samples on the 3D cameras: 1 (off), 2 or 4 (Bevy's default, how it drew before this setting).
    pub msaa: u32,
    /// Draw as the PS2 did: the court at 640×448 and the HUD at 448 lines, scaled up unfiltered (overrides `scale`).
    pub original: bool,
    /// A window instead of borderless fullscreen on the current monitor (the default, B96). Not part of a preset.
    pub windowed: bool,
    /// Draw the frame rate in the window's bottom-left corner (B186; `fps_counter`): 0 off, 1 on, 2 detailed (where
    /// the frame's time goes, B328). Not part of a preset.
    pub show_fps: u8,
    /// B343: at Shadows Low only the players, the umpire and what shades the court cast (`shadow::Minor`, B351);
    /// off, Low casts everything High does. A speed-up a device can turn off; set by a preset (B412: High turns it off).
    pub fewer_shadows: bool,
    /// Leave the characters' and effects' meshes out of a view they're outside of (B345, `cull.rs`); off draws them
    /// all as before. Set by a preset (B412).
    pub cull: bool,
    /// B364: the share of the characters' triangles drawn, in percent (`lod.rs`; 100 = their own meshes). Applies to
    /// models loaded after a change. Set by a preset (B412).
    pub mesh: u32,
    /// P53: the Auto preset: picked from the device (`pick`) and, while on, the first seconds of each match's play
    /// timed and the scale and shadows stepped down (`cheaper`) until 60 fps holds (`auto_tune`). Any change to a
    /// graphics row turns it off. Not part of a preset, like the window.
    pub auto: bool,
}

/// `Graphics::mesh` for the loaders, which have no world (`character.rs`, `mods.rs`); kept in step by `mesh_detail`.
pub static MESH: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100);

/// Set for `--shot` and unattended (HST_BG) runs: their fixed-size window never goes fullscreen (`main.rs`).
pub static KEEP_WINDOWED: AtomicBool = AtomicBool::new(false);

/// P44: 240/360/480 for high-refresh monitors (the sim stays 60 Hz; drawing interpolates between ticks).
/// 90 for the OLED Steam Deck's 90 Hz screen.
pub const FPS: [u32; 9] = [30, 60, 90, 120, 144, 240, 360, 480, 0];
pub const SCALES: [u32; 5] = [50, 67, 75, 85, 100];
pub const SHADOWS: [Shadows; 3] = [Shadows::Off, Shadows::Low, Shadows::High];
pub const MSAA: [u32; 3] = [1, 2, 4];
pub const MESHES: [u32; 4] = [25, 50, 75, 100];
/// The speed-up switch's default (`Graphics::fewer_shadows`).
const FEWER_SHADOWS: bool = true;

impl Default for Graphics {
    fn default() -> Self {
        Graphics { fps: 0, vsync: false, scale: 100, shadows: Shadows::High, msaa: 4, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, cull: true, mesh: 100, auto: false }
    }
}

impl Graphics {
    /// The Steam Deck's: its native 1280×800, cheaper shadows, vsync on (picked on a Deck's first run).
    pub const STEAM_DECK: Graphics = Graphics { fps: 0, vsync: true, scale: 100, shadows: Shadows::Low, msaa: 4, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, cull: true, mesh: 100, auto: false };
    /// A phone's or tablet's (B351, picked on the browser build's first run on a touch screen and the iOS app's first
    /// run): Low shadows, which also drop the small props' and the spectators' (`shadow::Minor`), at 67% scale with 2x
    /// MSAA. M91: an iPhone 15 Pro at 100%/2x is GPU-bound on the Grand Indoor Palace (40 fps); a phone's dense screen
    /// hides the lower scale. B413: with Fewer Shadows and Skip Off-Screen on (the speed-ups that measurably help);
    /// Mesh stays 100%: 75% and 50% showed no gain native (dGPU, iGPU) or in a 4x-throttled phone browser, as the
    /// characters' few thousand triangles aren't what a phone's frame waits on.
    pub const PHONE: Graphics = Graphics { fps: 60, vsync: false, scale: 67, shadows: Shadows::Low, msaa: 2, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, cull: true, mesh: 100, auto: false };
    /// The cheapest that still looks like the game: for weak GPUs.
    pub const LOW: Graphics = Graphics { fps: 60, vsync: false, scale: 67, shadows: Shadows::Low, msaa: 1, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, cull: true, mesh: 100, auto: false };
    /// B414: below Low, for the weakest devices: half scale, no shadows or MSAA, the characters at a quarter of their
    /// triangles, every speed-up on. 60, like every preset: 30 is unplayable (user 2026-10-15), never a default.
    pub const LOWEST: Graphics = Graphics { fps: 60, vsync: false, scale: 50, shadows: Shadows::Off, msaa: 1, original: false, windowed: false, show_fps: 0, fewer_shadows: true, cull: true, mesh: 25, auto: false };
    /// B412: the best look, with every speed-up off (Fewer Shadows, Skip Off-Screen; full meshes). Default is the
    /// balanced one: the same picture with the speed-ups on.
    pub const HIGH: Graphics = Graphics { fps: 0, vsync: false, scale: 100, shadows: Shadows::High, msaa: 4, original: false, windowed: false, show_fps: 0, fewer_shadows: false, cull: false, mesh: 100, auto: false };
    pub const PRESETS: [(&str, Graphics); 6] = [("High", Self::HIGH), ("Default", Graphics { fps: 0, vsync: false, scale: 100, shadows: Shadows::High, msaa: 4, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, cull: true, mesh: 100, auto: false }), ("Steam Deck", Self::STEAM_DECK), ("Low", Self::LOW), ("Lowest", Self::LOWEST), ("Phone", Self::PHONE)];

    /// Read one `key = value` (settings.txt or a flag's name and value); false if the key isn't one of these.
    pub fn set(&mut self, key: &str, v: &str) -> bool {
        let v = v.trim();
        match key.trim() {
            "fps" | "frame_limit" => self.fps = if v == "uncapped" { 0 } else { v.parse().unwrap_or(self.fps) },
            "vsync" => self.vsync = v != "off",
            "scale" | "render_scale" => self.scale = v.trim_end_matches('%').parse::<u32>().map_or(self.scale, |s| s.clamp(25, 100)),
            "msaa" | "anti_aliasing" => self.msaa = MSAA.into_iter().find(|&m| msaa_name(m) == v).unwrap_or(self.msaa),
            "original" => self.original = v != "off",
            "windowed" => self.windowed = v != "off",
            "show_fps" => self.show_fps = ["off", "on", "detailed"].iter().position(|&n| n == v).unwrap_or(1) as u8,
            "cull" => self.cull = v != "off",
            "mesh" => self.mesh = v.trim_end_matches('%').trim().parse::<u32>().map_or(self.mesh, |m| m.clamp(10, 100)),
            "shadows" => self.shadows = SHADOWS.into_iter().find(|s| s.name() == v).unwrap_or(self.shadows),
            "fewer_shadows" | "fewer-shadows" => self.fewer_shadows = v != "off",
            "auto_graphics" => self.auto = v == "on",
            _ => return false,
        }
        true
    }

    /// settings.txt lines.
    pub fn text(&self) -> String {
        let fps = if self.fps == 0 { "uncapped".to_string() } else { self.fps.to_string() };
        let on = |b: bool| if b { "on" } else { "off" };
        format!(
            "frame_limit = {fps}\nvsync = {}\nrender_scale = {}\nshadows = {}\nanti_aliasing = {}\noriginal = {}\nwindowed = {}\nshow_fps = {}\nfewer_shadows = {}\ncull = {}\nmesh = {}\nauto_graphics = {}\n",
            on(self.vsync),
            self.scale,
            self.shadows.name(),
            msaa_name(self.msaa),
            on(self.original),
            on(self.windowed),
            ["off", "on", "detailed"][self.show_fps.min(2) as usize],
            on(self.fewer_shadows),
            on(self.cull),
            self.mesh,
            on(self.auto)
        )
    }

    /// The match's flags.
    pub fn flags(&self) -> Vec<String> {
        let mut f = vec!["--fps".into(), self.fps.to_string(), "--scale".into(), self.scale.to_string(), "--shadows".into(), self.shadows.name().into(), "--msaa".into(), msaa_name(self.msaa).into()];
        if self.vsync {
            f.push("--vsync".into());
        }
        if self.original {
            f.push("--original".into());
        }
        if self.windowed {
            f.push("--windowed".into());
        }
        if !self.cull {
            f.extend(["--cull".into(), "off".into()]);
        }
        match self.show_fps {
            0 => {}
            1 => f.push("--show-fps".into()),
            _ => f.push("--show-fps-detailed".into()),
        }
        // Fewer Shadows only when not at its default (also the browser's `?fewer-shadows=off`)
        if self.fewer_shadows != FEWER_SHADOWS {
            f.extend(["--fewer-shadows".into(), if self.fewer_shadows { "on" } else { "off" }.into()]);
        }
        if self.mesh != 100 {
            f.extend(["--mesh".into(), self.mesh.to_string()]);
        }
        f
    }

    pub fn preset(&self) -> &'static str {
        Self::PRESETS.iter().find(|(_, g)| self.keep(*g) == *self).map_or("Custom", |(n, _)| n)
    }

    /// Preset `p` with this one's settings that no preset sets (the window, Show FPS, Auto).
    pub fn keep(&self, p: Graphics) -> Graphics {
        Graphics { windowed: self.windowed, show_fps: self.show_fps, auto: self.auto, ..p }
    }

    pub fn present_mode(&self) -> PresentMode {
        // `HST_PRESENT=fifo|mailbox|immediate` (P44): one present mode for frame-pacing benches, over Vsync
        match std::env::var("HST_PRESENT").as_deref() {
            Ok("fifo") => return PresentMode::Fifo,
            Ok("mailbox") => return PresentMode::Mailbox,
            Ok("immediate") => return PresentMode::Immediate,
            _ => {}
        }
        if self.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync }
    }

    pub fn window_mode(&self) -> WindowMode {
        if self.windowed || KEEP_WINDOWED.load(Ordering::Relaxed) { WindowMode::Windowed } else { WindowMode::BorderlessFullscreen(MonitorSelection::Current) }
    }
}

/// `anti_aliasing`'s value: off, 2x, 4x.
pub fn msaa_name(m: u32) -> &'static str {
    match m {
        1 => "off",
        2 => "2x",
        _ => "4x",
    }
}

impl Shadows {
    pub fn name(self) -> &'static str {
        ["off", "low", "high"][self as usize]
    }
}

pub fn plugin(app: &mut App) {
    if !app.is_plugin_added::<bevy::diagnostic::FrameTimeDiagnosticsPlugin>() {
        app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default());
    }
    app.init_resource::<Graphics>()
        .add_systems(PreStartup, mesh_detail)
        .add_systems(First, mesh_detail)
        .add_systems(PostUpdate, (window, lights, msaa.after(bevy::camera::CameraUpdateSystems), fps_counter))
        .add_systems(Last, limit)
        .add_systems(Last, auto_tune.run_if(crate::play::match_on()))
        .add_systems(PreStartup, cpu_clusters)
        .add_systems(PostUpdate, unclustered.before(bevy::light::SimulationLightSystems::AssignLightsToClusters));
    // Show FPS → Detailed: where the frame's time goes (`frame_start`)
    use bevy::app::RunFixedMainLoopSystems as F;
    app.init_resource::<Split>()
        .add_systems(First, frame_start)
        .add_systems(RunFixedMainLoop, (mark::<1>.in_set(F::BeforeFixedMainLoop), mark::<2>.in_set(F::AfterFixedMainLoop)))
        .add_systems(Last, mark::<3>.before(limit));
    // P49: ProMotion's refresh rate follows the frame limit (`ios::frame_rate`)
    #[cfg(target_os = "ios")]
    app.add_systems(PostUpdate, |g: Res<Graphics>| if g.is_changed() { crate::ios::frame_rate(g.fps) });
    if let Some(r) = app.get_sub_app_mut(bevy::render::RenderApp) {
        use bevy::render::renderer::{RenderGraph, RenderGraphSystems as G};
        r.add_systems(RenderGraph, (mark::<4>.in_set(G::Begin), mark::<5>.in_set(G::Render).after(bevy::core_pipeline::schedule::camera_driver)))
            .add_systems(bevy::render::Render, mark::<6>.in_set(bevy::render::RenderSystems::Cleanup));
        // the swapchain acquire: waits there while the GPU is still busy with earlier frames (P28)
        use bevy::render::{RenderSystems as R, view::prepare_windows};
        r.add_systems(bevy::render::Render, (mark::<7>.in_set(R::PrepareViews).before(prepare_windows), mark::<8>.in_set(R::PrepareViews).after(prepare_windows)));
        r.add_systems(RenderGraph, limit_present.in_set(G::Finish));
        #[cfg(target_os = "ios")]
        r.add_systems(RenderGraph, (gpu_time::<false>.in_set(G::Begin), gpu_time::<true>.after(G::Render).before(G::Submit)));
    }
    if crate::fps_log() {
        app.init_resource::<SplitLog>().add_systems(Last, (log_draws, log_split, |t: Res<Time<Real>>| NOW.store(t.elapsed_secs().to_bits(), Ordering::Relaxed)));
        if let Some(r) = app.get_sub_app_mut(bevy::render::RenderApp) {
            r.add_systems(bevy::render::Render, log_pipelines.in_set(bevy::render::RenderSystems::Cleanup));
        }
    }
    // `HST_PASSES=1` (P32): Bevy's per-pass render timings, with GPU time where the backend gives timestamps (Vulkan
    // does, Metal doesn't), logged every 5 s: which pass a heavy court spends its GPU time in
    if std::env::var_os("HST_PASSES").is_some() {
        app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin).add_systems(Last, log_passes);
        if let Some(r) = app.get_sub_app_mut(bevy::render::RenderApp) {
            r.add_systems(bevy::render::Render, log_unpacking.in_set(bevy::render::RenderSystems::Cleanup));
        }
    }
    // P48: direct draws for the 3D cameras; `HST_INDIRECT=1` keeps Bevy's GPU-built indirect ones (bench knob)
    if std::env::var("HST_INDIRECT").map_or(true, |v| v == "0") {
        app.add_systems(PostUpdate, direct_draws);
    }
}

/// P48: the 3D cameras draw directly instead of through Bevy's GPU-built indirect draws. Indirect drawing groups a
/// view's meshes into batch sets, one per pipeline and material bind group, and every frame runs one `bin_unpacking`
/// compute dispatch per batch set before the draws. `GsMaterial` isn't bindless, so each of the court's materials is a
/// batch set of its own: a match had ~350 dispatches of 1–2 meshes each (~440 meshes; `HST_PASSES=1` logs it), whose
/// fixed per-dispatch cost made the pass the biggest on the GPU (0.65 ms of ~1.3 on an RX 9070 XT, more than the
/// opaque draws' 0.21), plus ~0.3 ms CPU to record them and ~0.2 ms to rebuild their bind groups. With batch sets that
/// small there was nothing for a multi-draw to merge, so the draws themselves cost the same. Shadow cascades already
/// draw directly (Bevy only culls them on the GPU in its occlusion-culling mode), and where multi-draw-indirect-count
/// is missing (Metal, WebGPU, GL) Bevy draws directly anyway: the change is Vulkan/DX12 only (desktop, Steam Deck).
fn direct_draws(mut commands: Commands, q: Query<Entity, (With<Camera3d>, Without<bevy::render::view::NoIndirectDrawing>)>) {
    for e in &q {
        commands.entity(e).insert(bevy::render::view::NoIndirectDrawing);
    }
}

/// The app's real time (s, f32 bits) for the render world's logs.
static NOW: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// `HST_FPS=1` (P30): each render pipeline as it's first queued, with when: one queued mid-match is a first-draw
/// compile the loading screen's warm-up (`mode.rs`) missed.
fn log_pipelines(cache: Res<bevy::render::render_resource::PipelineCache>, mut seen: Local<usize>) {
    use bevy::render::render_resource::PipelineDescriptor;
    let all: Vec<_> = cache.pipelines().collect();
    for p in &all[(*seen).min(all.len())..] {
        let PipelineDescriptor::RenderPipelineDescriptor(d) = &p.descriptor else { continue };
        let defs = |v: &[bevy::shader::ShaderDefVal]| v.iter().map(|d| format!("{d:?}").replace("Bool(\"", "").replace("\", true)", "")).collect::<Vec<_>>().join(" ");
        eprintln!(
            "pipeline at {:.2} s: {} | {} vertex buffers ({} attrs) | samples {} | vs [{}] | fs [{}]",
            f32::from_bits(NOW.load(Ordering::Relaxed)),
            d.label.as_deref().unwrap_or("?"),
            d.vertex.buffers.len(),
            d.vertex.buffers.iter().map(|b| b.attributes.len()).sum::<usize>(),
            d.multisample.count,
            defs(&d.vertex.shader_defs),
            d.fragment.as_ref().map_or(String::new(), |f| defs(&f.shader_defs)),
        );
    }
    *seen = all.len();
}

/// `HST_PASSES=1`: every 5 s, the ten render passes that took longest on the GPU (`elapsed_gpu`; CPU where none).
/// A pass that stopped running keeps its last average (the menus' `msaa_writeback` shows on through a match).
fn log_passes(d: Res<bevy::diagnostic::DiagnosticsStore>, time: Res<Time<Real>>, mut next: Local<f32>) {
    let t = time.elapsed_secs();
    if t < *next {
        return;
    }
    *next = t + 5.0;
    let gpu = d.iter().any(|g| g.path().as_str().ends_with("elapsed_gpu"));
    let kind = if gpu { "elapsed_gpu" } else { "elapsed_cpu" };
    let mut passes: Vec<(f64, String)> = d
        .iter()
        .filter(|g| g.path().as_str().starts_with("render/") && g.path().as_str().ends_with(kind))
        .filter_map(|g| Some((g.average()?, g.path().as_str().trim_start_matches("render/").trim_end_matches(kind).trim_end_matches('/').to_string())))
        .collect();
    passes.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top: Vec<String> = passes.iter().take(10).map(|(ms, p)| format!("{p} {ms:.2}")).collect();
    eprintln!("passes {kind} ms at {t:.0} s: {}", top.join(", "));
}

/// `HST_PASSES=1` (P48): every 5 s, what Bevy's `bin_unpacking` compute pass did last frame: one dispatch per batch
/// set (pipeline + material bind group + vertex/index slab) of each binned phase (opaque, alpha mask, prepass,
/// shadow) of each view (the cameras and every shadow cascade), and how many mesh instances they unpacked in all.
fn log_unpacking(b: Res<bevy::render::batching::gpu_preprocessing::BinUnpackingBuffers>, mut next: Local<f32>) {
    let t = f32::from_bits(NOW.load(Ordering::Relaxed));
    if t < *next {
        return;
    }
    *next = t + 5.0;
    let (mut views, mut jobs, mut instances, mut biggest) = (std::collections::HashSet::new(), 0, 0, 0);
    for (key, v) in &b.view_phase_buffers {
        let all = v.indexed_unpacking_jobs.iter().chain(&v.non_indexed_unpacking_jobs);
        if v.indexed_unpacking_jobs.is_empty() && v.non_indexed_unpacking_jobs.is_empty() {
            continue;
        }
        views.insert(key.view);
        for j in all {
            jobs += 1;
            instances += j.mesh_instance_count;
            biggest = biggest.max(j.mesh_instance_count);
        }
    }
    eprintln!("bin unpacking at {t:.0} s: {jobs} dispatches over {} (view, phase)s of {} views, {instances} instances (largest batch set {biggest})", b.view_phase_buffers.len(), views.len());
}

/// The Mesh setting where the loaders read it (`MESH`).
fn mesh_detail(g: Res<Graphics>) {
    if g.is_changed() {
        MESH.store(g.mesh, Ordering::Relaxed);
    }
}

/// `HST_FPS=1`: every 5 s, what the cameras and the shadow cascades draw: the meshes the active scene cameras see and
/// how many distinct (mesh, material) pairs they are (the most draws they can batch down to), the spectators' and
/// creatures' share (`shadow::Minor` rigs) and the meshes the sun's cascades draw.
#[allow(clippy::type_complexity)]
fn log_draws(
    time: Res<Time<Real>>,
    mut next: Local<f32>,
    cams: Query<(&bevy::camera::visibility::VisibleEntities, &Camera), With<Camera3d>>,
    cascades: Query<&bevy::camera::visibility::CascadesVisibleEntities>,
    parts: Query<(&Mesh3d, Option<&MeshMaterial3d<crate::gs::GsMaterial>>, Option<&MeshMaterial3d<StandardMaterial>>)>,
    parents: Query<&ChildOf>,
    npc: Query<(), (With<crate::character::Rig>, With<crate::shadow::Minor>)>,
) {
    let now = time.elapsed_secs();
    if now < *next {
        return;
    }
    *next = now + 5.0;
    let (mut seen, mut pairs, mut npcs, mut npc_pairs) = (0, std::collections::HashSet::new(), 0, std::collections::HashSet::new());
    for (v, c) in &cams {
        if !c.is_active {
            continue;
        }
        for &e in v.get(std::any::TypeId::of::<Mesh3d>()) {
            let Ok((mesh, gs, std)) = parts.get(e) else { continue };
            let pair = (mesh.0.id(), gs.map(|m| m.0.id().untyped()).or(std.map(|m| m.0.id().untyped())));
            seen += 1;
            pairs.insert(pair);
            if parents.iter_ancestors(e).any(|a| npc.contains(a)) {
                npcs += 1;
                npc_pairs.insert(pair);
            }
        }
    }
    let cast: Vec<Vec<usize>> = cascades.iter().flat_map(|c| c.entities.values()).map(|v| v.iter().map(|c| c.entities.len()).collect()).collect();
    eprintln!("draws at {now:.0} s: {} of {} cameras active, {seen} meshes seen ({} mesh/material pairs), npcs {npcs} ({} pairs), shadow casters per view and cascade {cast:?}", cams.iter().filter(|(_, c)| c.is_active).count(), cams.iter().count(), pairs.len(), npc_pairs.len());
}

/// When this frame reached: 0 its start (`First`), 1/2 the fixed-step simulation's start/end, 3 the end of the
/// game's update, 4 the render graph's start (extract, prepare and queue done), 5 pass encoding done, 6 submitted and
/// presented, 7/8 the swapchain acquire's start/end (`prepare_windows`, between 3 and 4). On the web all of it runs on
/// one thread in that order; natively rendering overlaps the next frame.
static MARKS: std::sync::Mutex<[Option<bevy::platform::time::Instant>; 9]> = std::sync::Mutex::new([None; 9]);

fn mark<const I: usize>() {
    MARKS.lock().unwrap()[I] = Some(bevy::platform::time::Instant::now());
}

/// Sums of the frames since the counter last showed them, in ms: frame, game (incl. sim), sim, prepare (extract,
/// prepare, queue; with the acquire), encode, submit (+ present), worst frame, frames, then the swapchain acquire (of
/// prepare) and the GPU's time from the frame's first command buffer starting to its last ending, over that many
/// frames (Metal only: `gpu_time`).
#[derive(Resource, Default)]
struct Split([f32; 11]);

/// The same for `HST_FPS`'s log, every 5 s.
#[derive(Resource, Default)]
struct SplitLog(Split);

fn frame_start(mut split: ResMut<Split>, log: Option<ResMut<SplitLog>>) {
    let now = bevy::platform::time::Instant::now();
    let mut m = MARKS.lock().unwrap();
    let (gpu, gpu_n) = std::mem::take(&mut *GPU_MS.lock().unwrap());
    for split in std::iter::once(&mut *split).chain(log.map(|l| &mut l.into_inner().0)) {
        if let [Some(t0), Some(t1), Some(t2), Some(t3), Some(t4), Some(t5), Some(t6), t7, t8] = *m {
            let ms = |a: bevy::platform::time::Instant, b: bevy::platform::time::Instant| b.saturating_duration_since(a).as_secs_f32() * 1000.0;
            let frame = ms(t0, now);
            let acquire = t7.zip(t8).map_or(0.0, |(a, b)| ms(a, b));
            // the frame limit's sleep (`limit_present`) is between 5 and 6: idle, not submit
            let slept = SLEPT_US.load(Ordering::Relaxed) as f32 / 1000.0;
            for (sum, v) in split.0.iter_mut().zip([frame, ms(t0, t3), ms(t1, t2), ms(t3, t4), ms(t4, t5), (ms(t5, t6) - slept).max(0.0)]) {
                *sum += v;
            }
            split.0[6] = split.0[6].max(frame);
            split.0[7] += 1.0;
            split.0[8] += acquire;
        }
        split.0[9] += gpu;
        split.0[10] += gpu_n;
    }
    *m = [Some(now), None, None, None, None, None, None, None, None];
}

/// `HST_FPS=1`: every 5 s, the frame split (`Split`) to stderr: prep without the swapchain acquire, the acquire (the
/// CPU waiting for a drawable, i.e. for the GPU), and the GPU's own time per frame where Metal gives it.
fn log_split(time: Res<Time<Real>>, mut log: ResMut<SplitLog>, mut next: Local<f32>) {
    let now = time.elapsed_secs();
    if now < *next {
        return;
    }
    *next = now + 5.0;
    let [frame, game, sim, prep, enc, sub, worst, n, acquire, gpu, gpu_n] = log.0.0;
    if n > 0.0 {
        let a = |v: f32| v / n;
        let gpu = if gpu_n > 0.0 { format!("{:.2}", gpu / gpu_n) } else { "n/a".into() };
        eprintln!(
            "frame split ms: frame {:.2} (worst {worst:.1}) game {:.2} sim {:.2} prep {:.2} acquire {:.2} enc {:.2} submit {:.2} idle {:.2} gpu {gpu}",
            a(frame),
            a(game),
            a(sim),
            a(prep - acquire),
            a(acquire),
            a(enc),
            a(sub),
            a((frame - game - prep - enc - sub).max(0.0))
        );
    }
    log.0.0 = [0.0; 11];
}

/// Metal's GPU time of the frames finished since `frame_start` last took it: ms summed, frames.
static GPU_MS: std::sync::Mutex<(f32, f32)> = std::sync::Mutex::new((0.0, 0.0));

/// Brackets the frame's command buffers with two of its own (`END` = false: before the render graph's passes, true:
/// after them, both submitted with them in order) and reads Metal's GPUStartTime of the first and GPUEndTime of the
/// last when they complete: the GPU's time on the frame's rendering (plus any gap the GPU sat idle inside it, none
/// when GPU-bound). Each carries a 4-byte fill so Metal schedules it. Bevy's RenderDiagnosticsPlugin only gets CPU
/// times on Metal (M91 build 2014).
#[cfg(target_os = "ios")]
fn gpu_time<const END: bool>(mut ctx: bevy::render::renderer::RenderContext, mut buf: Local<Option<bevy::render::render_resource::Buffer>>) {
    use wgpu::hal;
    use objc2_metal::MTLCommandBuffer;
    static START: std::sync::Mutex<Option<f64>> = std::sync::Mutex::new(None);
    let device = ctx.render_device().clone();
    let buf = buf.get_or_insert_with(|| device.create_buffer(&wgpu::BufferDescriptor { label: Some("gpu_time"), size: 4, usage: wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("gpu_time") });
    let done = block2::RcBlock::new(|cb: std::ptr::NonNull<objc2::runtime::ProtocolObject<dyn MTLCommandBuffer>>| {
        let cb = unsafe { cb.as_ref() };
        if !END {
            *START.lock().unwrap() = Some(cb.GPUStartTime());
        } else if let Some(start) = START.lock().unwrap().take() {
            let ms = ((cb.GPUEndTime() - start) * 1000.0) as f32;
            if ms > 0.0 && ms < 1000.0 {
                let mut g = GPU_MS.lock().unwrap();
                *g = (g.0 + ms, g.1 + 1.0);
            }
        }
    });
    // SAFETY: only the raw API records on this encoder; the buffer lives in the Local for the app's life
    unsafe {
        let Some(hal_buf) = buf.as_hal::<hal::api::Metal>() else { return };
        enc.as_hal_mut::<hal::api::Metal, _, _>(|e| {
            let Some(e) = e else { return };
            hal::CommandEncoder::clear_buffer(e, &*hal_buf, 0..4);
            if let Some(cb) = e.raw_command_buffer() {
                cb.addCompletedHandler(&*done as *const _ as *mut _);
            }
        });
    }
    ctx.add_command_buffer(enc.finish());
}

fn window(g: Res<Graphics>, mut w: Query<&mut Window, With<PrimaryWindow>>) {
    if g.is_changed() {
        for mut w in &mut w {
            w.present_mode = g.present_mode();
            let mode = g.window_mode();
            if w.mode != mode {
                w.mode = mode;
            }
        }
    }
}

/// Every scene camera (`hud_gamma::Scene`) at the same sample count: cameras sharing a target with different counts
/// get separate main textures, and the later one (`popups.rs`'s overlay, which doesn't clear) covers the court blank.
fn msaa(g: Res<Graphics>, mut q: Query<&mut Msaa, With<crate::hud_gamma::Scene>>, added: Query<(), Added<crate::hud_gamma::Scene>>, adapter: Option<Res<bevy::render::renderer::RenderAdapter>>) {
    if !g.is_changed() && added.is_empty() {
        return;
    }
    // Browser WebGPU only takes 1 or 4 samples (createTexture rejects 2), so 2x draws as 4x there; M102: so does a GPU
    // that can't take 2 for the main and depth textures (4 is WebGPU's guarantee; Android's SwiftShader, Phone's 2x)
    let no_2x = adapter.is_some_and(|a| {
        use bevy::render::render_resource::TextureFormat;
        [TextureFormat::Rgba16Float, TextureFormat::Depth32Float].iter().any(|&f| !a.get_texture_format_features(f).flags.sample_count_supported(2))
    });
    let m = Msaa::from_samples(if (cfg!(all(target_arch = "wasm32", feature = "webgpu")) || no_2x) && g.msaa == 2 { 4 } else { g.msaa });
    for mut c in &mut q {
        c.set_if_neq(m);
    }
}

/// P42: the game lights everything with one directional light and ambient; no point or spot light, light probe or
/// clustered decal is ever spawned (not by effects either), so Bevy's light clustering only costs time: its GPU path
/// built each 3D camera's cluster buffers anew every frame (`prepare_clusters_for_gpu_clustering`, ~250 µs traced)
/// and ran five clustering passes. CPU clustering (what Android and WebGL2 use anyway) with `ClusterConfig::None`
/// on the cameras does nothing per frame. The setting is read every frame, so switching it before the first is enough.
fn cpu_clusters(settings: Option<ResMut<bevy::light::cluster::GlobalClusterSettings>>) {
    if let Some(mut s) = settings {
        s.gpu_clustering = None;
    }
}

/// A 3D camera whose clustering `unclustered` turned off.
#[derive(Component)]
struct Unclustered;

/// No clusters for the 3D cameras while nothing clusterable exists; if a mod or later feature spawns a point/spot
/// light, probe or decal, the cameras get Bevy's default clustering back (and lose it again when it's gone).
fn unclustered(
    mut commands: Commands,
    cams: Query<(Entity, Has<Unclustered>), With<Camera3d>>,
    lit: Query<(), Or<(With<PointLight>, With<SpotLight>, With<bevy::light::LightProbe>, With<bevy::light::ClusteredDecal>)>>,
) {
    let none = lit.is_empty();
    for (e, off) in &cams {
        if none && !off {
            commands.entity(e).insert((bevy::light::cluster::ClusterConfig::None, Unclustered));
        } else if !none && off {
            commands.entity(e).insert(bevy::light::cluster::ClusterConfig::default()).remove::<Unclustered>();
        }
    }
}

/// Shadow quality: off, or the shadow map's size and cascades (High: 2048, two cascades).
fn lights(mut commands: Commands, g: Res<Graphics>, mut map: ResMut<DirectionalLightShadowMap>, mut lights: Query<(Entity, Mut<DirectionalLight>)>) {
    if !g.is_changed() && !lights.iter().any(|(_, l)| l.is_added()) {
        return;
    }
    // WebGL2 samples only a light's first cascade (Bevy's limit), which ends 30 m out, short of the court: one
    // cascade over the court instead, at WebGL2's largest texture (2048) for both qualities
    // ponytail: 60 m covers the match camera's view of the far baseline; raise it if a cut-away looks past that
    #[cfg(all(target_arch = "wasm32", not(feature = "webgpu")))]
    let (size, cascades, far) = (2048, 1, 60.0);
    // B287: the first cascade covers the whole court from either baseline camera; past it the jungle's tall canopy
    // shadows on the far half faded out. Low is one 2048 cascade to 120 m (still more texels a metre than the old
    // 1024-to-80 m first cascade): each cascade draws every caster again, too many draws for a phone (B328).
    // P36: High is the same first cascade to 80 m and one more to 120 m. Bevy's default 4 split 80–120 m into three
    // slivers that each drew every caster again; A/B shots on court 1 show no change on the court (one cascade to
    // 120 m like Low does: tree and net shadow edges move)
    #[cfg(not(all(target_arch = "wasm32", not(feature = "webgpu"))))]
    let (size, cascades, far) = if g.shadows == Shadows::Low { (2048, 1, 120.0) } else { (2048, 2, 120.0) };
    if map.size != size {
        map.size = size;
    }
    for (e, mut l) in &mut lights {
        l.shadow_maps_enabled = g.shadows != Shadows::Off;
        commands.entity(e).insert(CascadeShadowConfigBuilder { num_cascades: cascades, first_cascade_far_bound: 80.0, maximum_distance: far, ..default() }.build());
    }
}

#[derive(Component)]
struct FpsText;

/// Show FPS: the smoothed frame rate at the window's bottom-left corner, over menus and matches alike. Sized on the
/// 4:3 screen's height (the window's, or its width × 3/4 when taller than 4:3, `widescreen::vplace`) so it scales
/// uniformly with the HUD. Detailed (B328) adds, averaged over half a second: the frame's time split into the game's
/// update (of it the simulation), rendering (prepare, encode, submit) and idle (waiting on the browser/GPU: near 0 =
/// CPU-bound), the meshes drawn and the renderer.
#[allow(clippy::too_many_arguments)]
fn fps_counter(
    mut commands: Commands,
    g: Res<Graphics>,
    d: Res<bevy::diagnostic::DiagnosticsStore>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut q: Query<(Entity, &mut Text, &mut TextFont), With<FpsText>>,
    mut split: ResMut<Split>,
    meshes: Query<(&ViewVisibility, Has<bevy::light::NotShadowCaster>), With<Mesh3d>>,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    (time, mut since): (Res<Time<Real>>, Local<f32>),
) {
    let Ok((e, mut text, mut font)) = q.single_mut() else {
        if g.show_fps > 0 {
            // bottom-left: the menus' header holds pads/disc/version top-right, the legend bar's left end is empty (under the hint line)
            // detailed: top-left on a dark box, clear of the match's team panels
            let mut node = Node { position_type: PositionType::Absolute, bottom: Val::Px(0.0), left: Val::Percent(1.0), ..default() };
            let mut bg = BackgroundColor::DEFAULT;
            if g.show_fps == 2 {
                (node.bottom, node.top, bg) = (Val::Auto, Val::Px(0.0), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)));
            }
            commands.spawn((FpsText, Text::new(""), TextFont::default(), TextColor(Color::srgb(1.0, 1.0, 0.3)), TextShadow::default(), node, bg, GlobalZIndex(i32::MAX)));
        }
        return;
    };
    // off, or switched between On and Detailed (they sit in different corners): respawned next frame
    if g.show_fps == 0 || g.is_changed() {
        commands.entity(e).despawn();
        return;
    }
    let fps = d.get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS).and_then(|f| f.smoothed()).unwrap_or(0.0);
    let [frame, game, sim, prep, enc, sub, worst, n, acquire, ..] = split.0;
    *since += time.delta_secs();
    // P39: four times a second, not every frame: a new string is a text layout and a UI layout pass
    if g.show_fps == 1 {
        if *since >= 0.25 {
            *since = 0.0;
            let s = format!("{fps:.0} FPS");
            if text.0 != s {
                text.0 = s;
            }
        }
    } else if frame >= 500.0 {
        let a = |v: f32| v / n;
        let render = prep + enc + sub;
        let (mut shown, mut cast) = (0, 0);
        for (v, no_cast) in &meshes {
            shown += v.get() as u32;
            cast += (v.get() && !no_cast) as u32;
        }
        let size = window.single().map_or(String::new(), |w| {
            let k = if g.original { 0.0 } else { g.scale as f32 / 100.0 };
            if k > 0.0 { format!("{:.0}x{:.0}", w.physical_width() as f32 * k, w.physical_height() as f32 * k) } else { "640x448".into() }
        });
        let gpu = adapter.map_or(String::new(), |a| format!("{:?} {}", a.backend, a.name));
        // and to the log (the browser's console: Safari's Web Inspector on a phone)
        info!("frame split ms: frame {:.2} game {:.2} sim {:.2} render {:.2} (prep {:.2} acquire {:.2} enc {:.2} submit {:.2}) idle {:.2} meshes {shown} cast {cast}", a(frame), a(game), a(sim), a(render), a(prep - acquire), a(acquire), a(enc), a(sub), a((frame - game - render).max(0.0)));
        text.0 = format!(
            "{fps:.0} FPS  {:.1} ms (worst {worst:.1})\ncpu {:.1}  idle {:.1}\n game {:.1} (sim {:.1})\n render {:.1}: prep {:.1} acquire {:.1} enc {:.1} submit {:.1}\nmeshes {shown} drawn, {cast} cast shadows\n{size} shadows {} msaa {} | {gpu}",
            a(frame),
            a(game + render),
            a((frame - game - render).max(0.0)),
            a(game),
            a(sim),
            a(render),
            a(prep - acquire),
            a(acquire),
            a(enc),
            a(sub),
            g.shadows.name(),
            msaa_name(g.msaa),
        );
    }
    if frame >= 500.0 || g.show_fps == 1 {
        split.0 = [0.0; 11];
    }
    // 16 lines of the PS2's 448 (detailed: 11, it has six)
    let tall = window.single().is_ok_and(|w| w.width() * 3.0 < w.height() * 4.0);
    let px = if g.show_fps == 2 { 11.0 } else { 16.0 };
    let size = if tall { bevy::text::FontSize::Vw(px / 448.0 * 75.0) } else { bevy::text::FontSize::Vh(px / 448.0 * 100.0) };
    if font.font_size != size {
        font.font_size = size;
    }
}

/// The frame limit for `limit_present`; `HST_PACE=main` (P44): sleep here instead, at the main world's frame end (the
/// old limiter: up to a frame less input lag, uneven presents).
fn limit(g: Res<Graphics>, mut next: Local<Option<Instant>>) {
    LIMIT.store(g.fps, Ordering::Relaxed);
    // the browser paces frames itself (and can't sleep)
    if g.fps == 0 || cfg!(target_arch = "wasm32") || !PACE_MAIN.load(Ordering::Relaxed) {
        *next = None;
        return;
    }
    let frame = Duration::from_secs_f64(1.0 / g.fps as f64);
    let now = Instant::now();
    // fell behind by more than a frame: start over from now rather than rushing to catch up
    let at = next.filter(|&t| t + frame > now).unwrap_or(now);
    if at > now {
        std::thread::sleep(at - now);
    }
    *next = Some(at + frame);
}

/// The frame limit (fps, 0 = none), from the main world's `Graphics` to the render world's `limit_present`.
static LIMIT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// `HST_PACE=main`: `limit` sleeps instead of `limit_present`.
static PACE_MAIN: std::sync::LazyLock<AtomicBool> = std::sync::LazyLock::new(|| AtomicBool::new(std::env::var("HST_PACE").is_ok_and(|v| v == "main")));

/// The frame limit (P44): sleep away what's left of the frame's time after its submit, right before its present, so
/// presents come evenly spaced. Sleeping at the main world's frame end instead (`HST_PACE=main`) paced when rendering
/// started, and the render world's varying time then spread the presents: at a 60 fps cap, frame-to-frame jitter
/// 1.4–3.0 ms there vs 0.5–0.7 ms here (journal 2026-10-15-p35-high-fps/p44-pacing.md). Like vsync, the main world
/// then waits on the render world, up to a frame more input lag than sleeping at its own frame end.
// ponytail: thread::sleep overshoots by the OS timer's slack (≈0.06 ms on Linux); spin the last bit if that shows
fn limit_present(mut next: Local<Option<Instant>>) {
    let fps = LIMIT.load(Ordering::Relaxed);
    if fps == 0 || cfg!(target_arch = "wasm32") || PACE_MAIN.load(Ordering::Relaxed) {
        *next = None;
        SLEPT_US.store(0, Ordering::Relaxed);
        return;
    }
    let frame = Duration::from_secs_f64(1.0 / fps as f64);
    let now = Instant::now();
    let at = next.filter(|&t| t + frame > now).unwrap_or(now);
    SLEPT_US.store(at.saturating_duration_since(now).as_micros() as u32, Ordering::Relaxed);
    if at > now {
        std::thread::sleep(at - now);
    }
    *next = Some(at + frame);
}

/// `limit_present`'s last sleep, µs (for Show FPS → Detailed's split).
static SLEPT_US: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

// ---------------------------------------------------------------- P53: Auto

/// What the Auto preset picks from (`pick`). iOS Safari hides the model and the GPU, so the browser gives the screen
/// (CSS px and DPR), the cores, `deviceMemory` (Chromium only) and WebGPU's `maxBufferSize` (`web::device`); the apps
/// give the model and the memory.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Device {
    /// A phone or tablet: the browser on a touch screen, the iOS or Android app.
    pub touch: bool,
    pub steam_deck: bool,
    /// The app's device model: iOS's `hw.machine` (`iPhone14,5`), Android's `Build.MODEL`.
    pub model: Option<String>,
    /// The screen in CSS px, portrait (narrow side first); 0 unknown.
    pub css: [u32; 2],
    /// Device pixels per CSS px; 0 unknown.
    pub dpr: f32,
    /// CPU cores; 0 unknown.
    pub cores: u32,
    /// Memory in MB; 0 unknown (Safari, Firefox).
    pub ram_mb: u32,
    /// WebGPU's `maxBufferSize` in MB; 0 unknown (WebGL2, the apps).
    pub gpu_buffer_mb: u32,
}

impl Device {
    /// This device (`touch`: the page or app is played by touch).
    pub fn here(touch: bool) -> Device {
        let mut d = Device { touch: touch || cfg!(any(target_os = "ios", target_os = "android")), steam_deck: std::env::var("SteamDeck").is_ok_and(|v| v == "1"), ..default() };
        #[cfg(target_arch = "wasm32")]
        crate::web::device(&mut d);
        #[cfg(not(target_arch = "wasm32"))]
        {
            d.cores = std::thread::available_parallelism().map_or(0, |n| n.get() as u32);
        }
        #[cfg(target_os = "ios")]
        {
            d.model = sysctl_string(c"hw.machine");
            d.ram_mb = sysctl_u64(c"hw.memsize").map_or(0, |b| (b >> 20) as u32);
        }
        #[cfg(target_os = "android")]
        {
            d.model = crate::android::model();
            // SAFETY: plain sysconf queries
            let (pages, size) = unsafe { (libc::sysconf(libc::_SC_PHYS_PAGES), libc::sysconf(libc::_SC_PAGESIZE)) };
            d.ram_mb = if pages > 0 && size > 0 { ((pages as u64 * size as u64) >> 20) as u32 } else { 0 };
        }
        d
    }

    /// How many `cheaper` steps below Phone a phone or tablet starts (0–5). Biased high: the timing in a match steps
    /// down from there but never up, so only signs of a weaker device lower it.
    fn steps(&self) -> u32 {
        let mut n = match self.model.as_deref().and_then(ios_tier) {
            Some(n) => n,
            None => {
                let mut n = 0;
                // Chromium's deviceMemory or the app's: a cores count only means something beside it (Safari's isn't the
                // device's)
                if self.ram_mb > 0 {
                    if self.ram_mb <= 3 * 1024 || (1..=4).contains(&self.cores) {
                        n = 2;
                    } else if self.ram_mb <= 4 * 1024 {
                        n = 1;
                    }
                } else if self.model.is_none() {
                    // the browser on an iPhone: its screen size narrows down the generation (an iPhone 13 ran ~30 fps
                    // at Phone in Safari)
                    n = iphone_screen_tier(self.css, self.dpr);
                }
                if (1..=256).contains(&self.gpu_buffer_mb) {
                    n = n.max(1);
                }
                n
            }
        };
        // a tablet's many pixels: one more
        let px = self.css[0] as f32 * self.css[1] as f32 * self.dpr * self.dpr;
        if px > 4.5e6 {
            n += 1;
        }
        n.min(5)
    }
}

/// An iOS model identifier's steps below Phone: an iPhone 15 Pro (`iPhone16,1`) or newer, or an M1 iPad or newer, 0
/// (M91: a 15 Pro holds 60 at Phone in the app); an iPhone 13/14 (A15/A16, `iPhone14`/`15`) 1; older 2.
fn ios_tier(model: &str) -> Option<u32> {
    let (kind, rest) = model.split_at(model.find(|c: char| c.is_ascii_digit())?);
    let major: u32 = rest.split(',').next()?.parse().ok()?;
    match kind {
        "iPhone" => Some(if major >= 16 { 0 } else if major >= 14 { 1 } else { 2 }),
        "iPad" => Some(if major >= 13 { 0 } else { 2 }),
        _ => None,
    }
}

/// The browser on an iPhone, by its screen (CSS px at DPR 2 or 3): the sizes only iPhones up to the 14 and 16e have
/// (A15/A16 and older) 1, the pre-X ones and the XR/11 2, anything else (14 Pro and newer, not an iPhone) 0.
fn iphone_screen_tier([w, h]: [u32; 2], dpr: f32) -> u32 {
    if dpr < 2.0 {
        return 0;
    }
    match (w, h) {
        (375, 667) | (414, 736) | (414, 896) => 2,
        (375, 812) | (360, 780) | (390, 844) | (428, 926) => 1,
        _ => 0,
    }
}

/// The Auto preset's first pick on `d`: Steam Deck's on a Deck (without Auto: its preset is tuned by hand and SteamOS
/// may cap it below 60), Default elsewhere off a touch screen, Phone stepped down by `Device::steps` on a phone or tablet.
pub fn pick(d: &Device) -> Graphics {
    if d.steam_deck {
        return Graphics::STEAM_DECK;
    }
    if !d.touch {
        return Graphics { auto: true, ..default() };
    }
    let mut g = Graphics::PHONE;
    for _ in 0..d.steps() {
        g = cheaper(g).unwrap_or(g);
    }
    Graphics { auto: true, ..g }
}

/// One step cheaper, every preset's 60 fps kept (30 is unplayable, user 2026-10-15): the speed-ups on, then the scale
/// down to 67 %, Low shadows, MSAA halved to off, half scale, no shadows, quarter meshes (from Phone: Low, then down to
/// Lowest). `None` at the bottom.
pub fn cheaper(g: Graphics) -> Option<Graphics> {
    let mut c = g;
    if !g.fewer_shadows || !g.cull {
        (c.fewer_shadows, c.cull) = (true, true);
    } else if g.scale > 67 && !g.original {
        c.scale = SCALES.into_iter().rev().find(|&s| s < g.scale)?;
    } else if g.shadows == Shadows::High {
        c.shadows = Shadows::Low;
    } else if g.msaa > 1 {
        c.msaa = g.msaa / 2;
    } else if g.scale > 50 && !g.original {
        c.scale = 50;
    } else if g.shadows == Shadows::Low {
        c.shadows = Shadows::Off;
    } else if g.mesh > 25 {
        c.mesh = 25;
    } else {
        return None;
    }
    if (1..60).contains(&c.fps) {
        c.fps = 60;
    }
    Some(c)
}

/// No Auto timing in `--shot` and unattended (HST_BG) runs (`main.rs`).
pub static NO_AUTO: AtomicBool = AtomicBool::new(false);

/// `auto_tune`: the play before timing starts (the match's loading hitches), one timed window, the wait after a step
/// for it to take hold, the most timed play per match.
const AUTO_SKIP: f32 = 3.0;
const AUTO_WINDOW: f32 = 2.0;
const AUTO_SETTLE: f32 = 1.0;
const AUTO_SPAN: f32 = 16.0;

#[derive(Default)]
struct Tune {
    /// The frame this ran last (a gap: a new match).
    frame: u32,
    /// The match's play time so far, s.
    played: f32,
    /// Timed play this match, s.
    timed: f32,
    /// Waiting after a step until this play time.
    settle: f32,
    /// The window's frame times.
    window: Vec<f32>,
    /// Windows in a row that held.
    held: u32,
    /// Through for this match.
    done: bool,
    /// What it last saw or set (another change, e.g. the pause menu's save, starts its timing over).
    seen: Option<Graphics>,
}

/// The rate a window held: frames over its time without its slowest 5 % (a shader compile or a GC pause isn't the
/// device's pace).
fn held_fps(window: &mut [f32]) -> f32 {
    window.sort_by(f32::total_cmp);
    let keep = &window[..window.len() - window.len() / 20];
    keep.len() as f32 / keep.iter().sum::<f32>().max(1e-6)
}

/// The Auto preset (`Graphics::auto`) in a match: after its first seconds, times 2 s windows and steps one notch
/// cheaper (`cheaper`, saved to settings.txt so the next match starts there) after each that falls under 93 % of 60
/// fps (or of the display's rate when that's lower: a browser in Low Power Mode at 30 Hz isn't the game's fault),
/// until two in a row hold, the bottom is reached, or 16 s have been timed. Never steps up; each match checks again
/// (a heavier court). Drawing only: the simulation's tick and RNG don't see it.
fn auto_tune(mut g: ResMut<Graphics>, time: Res<Time<Real>>, frames: Res<bevy::diagnostic::FrameCount>, monitors: Query<&bevy::window::Monitor>, args: Option<Res<crate::Args>>, mut t: Local<Tune>) {
    let new_match = frames.0 != t.frame.wrapping_add(1);
    t.frame = frames.0;
    if !g.auto || NO_AUTO.load(Ordering::Relaxed) {
        return;
    }
    if new_match {
        *t = Tune { frame: frames.0, ..default() };
    }
    #[cfg(target_arch = "wasm32")]
    let hz = crate::web::refresh().unwrap_or(60.0) as f32;
    #[cfg(not(target_arch = "wasm32"))]
    let hz = monitors.iter().filter_map(|m| m.refresh_rate_millihertz).max().map_or(60.0, |m| m as f32 / 1000.0);
    let _ = &monitors;
    if let Some(c) = t.frame_time(*g, time.delta_secs(), hz.min(60.0)) {
        *g = c;
        if let Some(a) = &args {
            crate::play::main_menu::save_graphics(&a.iso, c);
        }
    }
}

impl Tune {
    /// One frame of a match's play, `dt` s, at the `target` rate: the cheaper settings to switch to, if any.
    fn frame_time(&mut self, g: Graphics, dt: f32, target: f32) -> Option<Graphics> {
        self.played += dt;
        let mine = |g: &Graphics| Graphics { windowed: false, show_fps: 0, ..*g };
        if self.seen.is_some_and(|s| s != mine(&g)) {
            (self.window, self.held, self.settle) = (Vec::new(), 0, self.played + AUTO_SETTLE);
        }
        self.seen = Some(mine(&g));
        if self.done || self.played < AUTO_SKIP.max(self.settle) {
            return None;
        }
        // a hidden page or a suspended app: its gap isn't a frame
        if dt > 0.25 {
            self.window.clear();
            return None;
        }
        self.window.push(dt);
        self.timed += dt;
        if self.window.iter().sum::<f32>() < AUTO_WINDOW {
            return None;
        }
        let fps = held_fps(&mut self.window);
        self.window.clear();
        self.done = self.timed >= AUTO_SPAN;
        if fps >= target * 0.93 {
            self.held += 1;
            self.done |= self.held >= 2;
            if self.held >= 2 {
                info!("auto graphics: {fps:.0} fps holds at {} ({}% scale, shadows {}, msaa {})", g.preset(), g.scale, g.shadows.name(), msaa_name(g.msaa));
            }
            return None;
        }
        let Some(c) = cheaper(g) else {
            info!("auto graphics: {fps:.0} fps of {target:.0} at the cheapest");
            self.done = true;
            return None;
        };
        info!("auto graphics: {fps:.0} fps of {target:.0}: {}% scale, shadows {}, msaa {}, mesh {}", c.scale, c.shadows.name(), msaa_name(c.msaa), c.mesh);
        (self.held, self.settle, self.seen) = (0, self.played + AUTO_SETTLE, Some(mine(&c)));
        Some(c)
    }
}

#[cfg(target_os = "ios")]
fn sysctl_string(name: &std::ffi::CStr) -> Option<String> {
    let mut buf = [0u8; 64];
    let mut len = buf.len();
    // SAFETY: `buf` holds `len` bytes; sysctlbyname writes at most that and sets `len`
    let r = unsafe { libc::sysctlbyname(name.as_ptr(), buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) };
    (r == 0).then(|| String::from_utf8_lossy(&buf[..len]).trim_end_matches('\0').to_string())
}

#[cfg(target_os = "ios")]
fn sysctl_u64(name: &std::ffi::CStr) -> Option<u64> {
    let mut v = 0u64;
    let mut len = std::mem::size_of::<u64>();
    // SAFETY: as above, into a u64
    let r = unsafe { libc::sysctlbyname(name.as_ptr(), (&mut v as *mut u64).cast(), &mut len, std::ptr::null_mut(), 0) };
    (r == 0).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_flags_round_trip() {
        for (_, p) in Graphics::PRESETS {
            let mut g = Graphics { fps: 7, vsync: !p.vsync, scale: 30, shadows: Shadows::Off, msaa: 3, original: !p.original, windowed: true, show_fps: 1, fewer_shadows: !FEWER_SHADOWS, cull: !p.cull, mesh: 50, auto: !p.auto };
            for (k, v) in p.text().lines().filter_map(|l| l.split_once('=')) {
                assert!(g.set(k, v), "{k}");
            }
            assert_eq!(g, p);
        }
        assert_eq!(Graphics::default().preset(), "Default");
        assert_eq!(Graphics { scale: 75, ..default() }.preset(), "Custom");
        assert_eq!(Graphics { windowed: true, ..default() }.preset(), "Default");
        assert_eq!(Graphics { show_fps: 1, ..default() }.preset(), "Default");
        assert_eq!(Graphics { show_fps: 1, ..default() }.flags().last().unwrap(), "--show-fps");
        assert_eq!(Graphics { windowed: true, ..default() }.flags().last().unwrap(), "--windowed");
        assert_eq!(Graphics { cull: false, ..default() }.preset(), "Custom");
        assert_eq!(Graphics { cull: false, ..default() }.flags()[8..], ["--cull", "off"]);
        assert_eq!(Graphics::PHONE.flags(), ["--fps", "60", "--scale", "67", "--shadows", "low", "--msaa", "2x"]);
        assert_eq!(Graphics::LOW.flags(), ["--fps", "60", "--scale", "67", "--shadows", "low", "--msaa", "off"]);
        // B413: the cheap presets run with the speed-ups that help, at full Mesh
        for p in [Graphics::PHONE, Graphics::LOW, Graphics::STEAM_DECK] {
            assert!(p.fewer_shadows && p.cull && p.mesh == 100);
        }
        // B343: Fewer Shadows passes only off its default and reads back as a flag; B412: a preset sets it
        let g = Graphics { fewer_shadows: !FEWER_SHADOWS, ..default() };
        assert_eq!(g.preset(), "Custom");
        assert_eq!(g.keep(Graphics::PHONE), Graphics::PHONE);
        let f = g.flags();
        let mut back = Graphics::default();
        for kv in f[8..].chunks(2) {
            assert!(back.set(&kv[0][2..], &kv[1]), "{kv:?}");
        }
        assert_eq!(back, g);
        // B364: Mesh passes only below 100 and reads back; B412: a preset sets it
        let g = Graphics { mesh: 50, ..default() };
        assert_eq!((g.preset(), g.keep(Graphics::LOW).mesh), ("Custom", 100));
        assert_eq!(g.flags()[8..], ["--mesh", "50"]);
        let mut back = Graphics::default();
        assert!(back.set("mesh", "50 %") && back == g);
        // B412: High is Default with the speed-ups off; the window and Show FPS keep through it
        assert_eq!(Graphics::HIGH, Graphics { fewer_shadows: false, cull: false, ..default() });
        assert_eq!(Graphics::HIGH.preset(), "High");
        assert_eq!(Graphics::HIGH.flags()[8..], ["--cull", "off", "--fewer-shadows", "off"]);
        let w = Graphics { windowed: true, show_fps: 1, ..default() };
        assert_eq!((w.keep(Graphics::HIGH).preset(), w.keep(Graphics::HIGH).windowed, w.keep(Graphics::HIGH).show_fps), ("High", true, 1));
        // B414: Lowest, below Low, with every speed-up on
        assert_eq!(Graphics::LOWEST.flags(), ["--fps", "60", "--scale", "50", "--shadows", "off", "--msaa", "off", "--mesh", "25"]);
        assert_eq!((Graphics::LOWEST.preset(), w.keep(Graphics::LOWEST).preset()), ("Lowest", "Lowest"));
        assert!(Graphics::LOWEST.fewer_shadows && Graphics::LOWEST.cull && Graphics::LOWEST.mesh == MESHES[0] && Graphics::LOWEST.scale == SCALES[0]);
        let names: Vec<_> = Graphics::PRESETS.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ["High", "Default", "Steam Deck", "Low", "Lowest", "Phone"]);
    }

    /// P53: the Auto preset's device pick and its step-down ladder never cap at 30, and the timing steps down until
    /// 60 holds.
    #[test]
    fn auto_picks_and_steps_down_at_60() {
        // the ladder: Default down to Phone's look, Low, then Lowest; every rung at 60 or uncapped
        let mut g = Graphics::default();
        let mut rungs = vec![g];
        while let Some(c) = cheaper(g) {
            assert!(c.fps == 0 || c.fps >= 60, "{c:?}");
            g = c;
            rungs.push(g);
        }
        let look = |g: &Graphics| (g.scale, g.shadows, g.msaa, g.mesh);
        let scales: Vec<_> = rungs.iter().map(|g| g.scale).collect();
        assert_eq!(scales, [100, 85, 75, 67, 67, 67, 67, 50, 50, 50]);
        for p in [Graphics::PHONE, Graphics::LOW, Graphics::LOWEST] {
            assert!(rungs.iter().any(|r| look(r) == look(&p)), "{p:?}");
        }
        assert_eq!(cheaper(Graphics::PHONE), Some(Graphics::LOW));
        assert_eq!((cheaper(Graphics::LOW).unwrap().scale, cheaper(Graphics::LOWEST)), (50, None));
        assert_eq!(cheaper(Graphics::HIGH), Some(Graphics::default()));
        assert_eq!(cheaper(Graphics { fps: 30, ..Graphics::LOW }).unwrap().fps, 60);
        // the pick: a Deck its own (no timing), a desktop Default, a phone Phone or below; never a 30 cap
        let phone = Device { touch: true, ..default() };
        assert_eq!(pick(&Device { steam_deck: true, ..default() }), Graphics::STEAM_DECK);
        assert_eq!(pick(&Device::default()), Graphics { auto: true, ..default() });
        assert_eq!(pick(&phone), Graphics { auto: true, ..Graphics::PHONE });
        let steps = |d: Device| d.steps();
        assert_eq!(steps(Device { model: Some("iPhone16,1".into()), ..phone.clone() }), 0);
        assert_eq!(steps(Device { model: Some("iPhone14,5".into()), ..phone.clone() }), 1);
        assert_eq!(steps(Device { model: Some("iPhone12,1".into()), ..phone.clone() }), 2);
        assert_eq!(steps(Device { model: Some("iPad8,1".into()), ..phone.clone() }), 2);
        // Safari on an iPhone 13 (390×844 @3): one down, Low; a 15 Pro's 393×852 stays at Phone
        assert_eq!(pick(&Device { css: [390, 844], dpr: 3.0, cores: 4, ..phone.clone() }), Graphics { auto: true, ..Graphics::LOW });
        assert_eq!(steps(Device { css: [393, 852], dpr: 3.0, ..phone.clone() }), 0);
        // Chromium's memory and cores; a tablet's pixels one more
        assert_eq!(steps(Device { ram_mb: 8192, cores: 8, ..phone.clone() }), 0);
        assert_eq!(steps(Device { ram_mb: 4096, cores: 8, ..phone.clone() }), 1);
        assert_eq!(steps(Device { ram_mb: 8192, cores: 4, ..phone.clone() }), 2);
        assert_eq!(steps(Device { css: [1024, 1366], dpr: 2.0, ..phone.clone() }), 1);
        for d in [phone.clone(), Device { ram_mb: 2048, css: [1024, 1366], dpr: 2.0, gpu_buffer_mb: 256, ..phone.clone() }] {
            let g = pick(&d);
            assert!(g.auto && g.fps == 60, "{g:?}");
        }
        assert!(ios_tier("Android").is_none() && ios_tier("iPhone").is_none());
    }

    /// P53: `Tune` waits out the match's first seconds, steps down after a slow window, waits for the step to take,
    /// and is through after two windows that hold (a hitch or a hidden page's gap doesn't count).
    #[test]
    fn auto_timing() {
        let run = |t: &mut Tune, g: &mut Graphics, dt: f32, secs: f32| {
            let mut steps = 0;
            for _ in 0..(secs / dt).round() as u32 {
                if let Some(c) = t.frame_time(*g, dt, 60.0) {
                    *g = c;
                    steps += 1;
                }
            }
            steps
        };
        // 30 fps: one step per window plus the wait after it, from Phone to the bottom within the 16 s
        let (mut t, mut g) = (Tune::default(), Graphics { auto: true, ..Graphics::PHONE });
        assert_eq!(run(&mut t, &mut g, 1.0 / 30.0, AUTO_SKIP - 0.1), 0);
        assert_eq!(run(&mut t, &mut g, 1.0 / 30.0, 2.2), 1);
        assert_eq!(g, Graphics { auto: true, ..Graphics::LOW });
        assert_eq!(run(&mut t, &mut g, 1.0 / 30.0, 30.0), 3);
        assert_eq!((g, t.done), (Graphics { auto: true, ..Graphics::LOWEST }, true));
        // 60 fps with a hitch: no step, through after two windows
        let (mut t, mut g) = (Tune::default(), Graphics { auto: true, ..Graphics::PHONE });
        let mut steps = run(&mut t, &mut g, 1.0 / 60.0, 4.0);
        assert!(t.frame_time(g, 0.2, 60.0).is_none() && t.frame_time(g, 5.0, 60.0).is_none());
        steps += run(&mut t, &mut g, 1.0 / 60.0, 3.0);
        assert!(!t.done);
        steps += run(&mut t, &mut g, 1.0 / 60.0, 2.0);
        assert_eq!((steps, t.done, g.scale), (0, true, 67));
        // a 30 Hz display (Low Power Mode) holds at 30
        let (mut t, g) = (Tune::default(), Graphics::PHONE);
        assert!((0..300).all(|_| t.frame_time(g, 1.0 / 30.0, 30.0).is_none()) && t.done);
        // the window's slowest 5 % left out
        let mut w = vec![1.0 / 60.0; 95];
        w.extend([0.2; 5]);
        assert!((held_fps(&mut w) - 60.0).abs() < 0.01);
    }
}
