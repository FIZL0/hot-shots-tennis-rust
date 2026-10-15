//! Video settings (remaster-only): borderless fullscreen or a window, frame limit, Show FPS, vsync, render resolution scale, shadow quality, MSAA, ambient occlusion (G1, beyond the original) and the original
//! 640×448 mode (`hud_gamma.rs` draws it; it also fixes the aspect ratio to 4:3 and turns upscaled textures off, `main.rs`).
//! The menu edits the [`Graphics`] resource (applied live) and passes it to a match as flags
//! (`--windowed`, `--fps N`, `--vsync`, `--scale N`, `--shadows off|low|high`, `--msaa off|2x|4x`, `--ao low|high`, `--original`, `--mesh N`). The simulation stays a fixed
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
    /// G1: screen-space ambient occlusion (Bevy's GTAO-based SSAO, `ao`): 0 off, 1 low, 2 high quality. Not in the
    /// original mode; the 3D cameras then draw without MSAA (SSAO needs it off), SMAA smoothing the edges instead
    /// when Anti-aliasing is on.
    pub ao: u8,
    /// Draw as the PS2 did: the court at 640×448 and the HUD at 448 lines, scaled up unfiltered (overrides `scale`).
    pub original: bool,
    /// A window instead of borderless fullscreen on the current monitor (the default, B96). Not part of a preset.
    pub windowed: bool,
    /// Draw the frame rate in the window's bottom-left corner (B186; `fps_counter`): 0 off, 1 on, 2 detailed (where
    /// the frame's time goes, B328). Not part of a preset.
    pub show_fps: u8,
    /// B343: at Shadows Low only the players, the umpire and what shades the court cast (`shadow::Minor`, B351);
    /// off, Low casts everything High does. A speed-up a device can turn off; not part of a preset.
    pub fewer_shadows: bool,
    /// B343: identical court creatures share their meshes and materials (each one's shade rides in its draw's
    /// `MeshTag`, `shade::Shared`) so the renderer can draw them as one instanced draw. Not part of a preset.
    pub batch_crowd: bool,
    /// B342: the court props' unblended parts drawn as one mesh per material (`merge.rs`) instead of one per part.
    /// Off by default: no gain measured (Bevy already batches the copies' draws). Not part of a preset.
    pub merge_meshes: bool,
    /// Leave the characters' and effects' meshes out of a view they're outside of (B345, `cull.rs`); off draws them
    /// all as before. Not part of a preset.
    pub cull: bool,
    /// B364: the share of the characters' triangles drawn, in percent (`lod.rs`; 100 = their own meshes). Applies to
    /// models loaded after a change. Not part of a preset.
    pub mesh: u32,
}

/// `Graphics::mesh` for the loaders, which have no world (`character.rs`, `mods.rs`); kept in step by `mesh_detail`.
pub static MESH: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100);

/// `--merge-meshes` given at launch: 0 not given, 1 off, 2 on; it holds over `Graphics::merge_meshes` (B342).
pub static MERGE_FORCED: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Set for `--shot` and unattended (HST_BG) runs: their fixed-size window never goes fullscreen (`main.rs`).
pub static KEEP_WINDOWED: AtomicBool = AtomicBool::new(false);

pub const FPS: [u32; 5] = [30, 60, 120, 144, 0];
pub const SCALES: [u32; 5] = [50, 67, 75, 85, 100];
pub const SHADOWS: [Shadows; 3] = [Shadows::Off, Shadows::Low, Shadows::High];
pub const MSAA: [u32; 3] = [1, 2, 4];
pub const MESHES: [u32; 4] = [25, 50, 75, 100];
/// `ambient_occlusion`'s values (`Graphics::ao`).
pub const AO: [&str; 3] = ["off", "low", "high"];
/// The speed-up switches' defaults (`Graphics::fewer_shadows`, `batch_crowd`, `merge_meshes`).
const FEWER_SHADOWS: bool = true;
const BATCH_CROWD: bool = false;
const MERGE_MESHES: bool = false;

impl Default for Graphics {
    fn default() -> Self {
        Graphics { fps: 0, vsync: false, scale: 100, shadows: Shadows::High, msaa: 4, ao: 0, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, batch_crowd: BATCH_CROWD, merge_meshes: MERGE_MESHES, cull: true, mesh: 100 }
    }
}

impl Graphics {
    /// The Steam Deck's: its native 1280×800, cheaper shadows, vsync on (picked on a Deck's first run).
    pub const STEAM_DECK: Graphics = Graphics { fps: 0, vsync: true, scale: 100, shadows: Shadows::Low, msaa: 4, ao: 0, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, batch_crowd: BATCH_CROWD, merge_meshes: MERGE_MESHES, cull: true, mesh: 100 };
    /// A phone's or tablet's (B351, picked on the browser build's first run on a touch screen): full resolution, but
    /// Low shadows, which also drop the small props' and the spectators' (`shadow::Minor`).
    pub const PHONE: Graphics = Graphics { fps: 60, vsync: false, scale: 100, shadows: Shadows::Low, msaa: 4, ao: 0, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, batch_crowd: BATCH_CROWD, merge_meshes: MERGE_MESHES, cull: true, mesh: 100 };
    /// The cheapest that still looks like the game: for weak GPUs.
    pub const LOW: Graphics = Graphics { fps: 60, vsync: false, scale: 67, shadows: Shadows::Low, msaa: 1, ao: 0, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, batch_crowd: BATCH_CROWD, merge_meshes: MERGE_MESHES, cull: true, mesh: 100 };
    pub const PRESETS: [(&str, Graphics); 4] = [("Default", Graphics { fps: 0, vsync: false, scale: 100, shadows: Shadows::High, msaa: 4, ao: 0, original: false, windowed: false, show_fps: 0, fewer_shadows: FEWER_SHADOWS, batch_crowd: BATCH_CROWD, merge_meshes: MERGE_MESHES, cull: true, mesh: 100 }), ("Steam Deck", Self::STEAM_DECK), ("Low", Self::LOW), ("Phone", Self::PHONE)];

    /// Read one `key = value` (settings.txt or a flag's name and value); false if the key isn't one of these.
    pub fn set(&mut self, key: &str, v: &str) -> bool {
        let v = v.trim();
        match key.trim() {
            "fps" | "frame_limit" => self.fps = if v == "uncapped" { 0 } else { v.parse().unwrap_or(self.fps) },
            "vsync" => self.vsync = v != "off",
            "scale" | "render_scale" => self.scale = v.trim_end_matches('%').parse::<u32>().map_or(self.scale, |s| s.clamp(25, 100)),
            "msaa" | "anti_aliasing" => self.msaa = MSAA.into_iter().find(|&m| msaa_name(m) == v).unwrap_or(self.msaa),
            "original" => self.original = v != "off",
            "ao" | "ambient_occlusion" => self.ao = AO.iter().position(|&n| n == v).map_or(self.ao, |i| i as u8),
            "windowed" => self.windowed = v != "off",
            "show_fps" => self.show_fps = ["off", "on", "detailed"].iter().position(|&n| n == v).unwrap_or(1) as u8,
            "merge_meshes" | "merge-meshes" => self.merge_meshes = v != "off",
            "cull" => self.cull = v != "off",
            "mesh" => self.mesh = v.trim_end_matches('%').trim().parse::<u32>().map_or(self.mesh, |m| m.clamp(10, 100)),
            "shadows" => self.shadows = SHADOWS.into_iter().find(|s| s.name() == v).unwrap_or(self.shadows),
            "fewer_shadows" | "fewer-shadows" => self.fewer_shadows = v != "off",
            "batch_crowd" | "batch-crowd" => self.batch_crowd = v != "off",
            _ => return false,
        }
        true
    }

    /// settings.txt lines.
    pub fn text(&self) -> String {
        let fps = if self.fps == 0 { "uncapped".to_string() } else { self.fps.to_string() };
        let on = |b: bool| if b { "on" } else { "off" };
        format!(
            "frame_limit = {fps}\nvsync = {}\nrender_scale = {}\nshadows = {}\nanti_aliasing = {}\nambient_occlusion = {}\noriginal = {}\nwindowed = {}\nshow_fps = {}\nfewer_shadows = {}\nbatch_crowd = {}\nmerge_meshes = {}\ncull = {}\nmesh = {}\n",
            on(self.vsync),
            self.scale,
            self.shadows.name(),
            msaa_name(self.msaa),
            AO[self.ao.min(2) as usize],
            on(self.original),
            on(self.windowed),
            ["off", "on", "detailed"][self.show_fps.min(2) as usize],
            on(self.fewer_shadows),
            on(self.batch_crowd),
            on(self.merge_meshes),
            on(self.cull),
            self.mesh
        )
    }

    /// The match's flags.
    pub fn flags(&self) -> Vec<String> {
        let mut f = vec!["--fps".into(), self.fps.to_string(), "--scale".into(), self.scale.to_string(), "--shadows".into(), self.shadows.name().into(), "--msaa".into(), msaa_name(self.msaa).into()];
        if self.ao > 0 {
            f.extend(["--ao".into(), AO[self.ao.min(2) as usize].into()]);
        }
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
        // the speed-up switches only when not at their defaults (also the browser's `?fewer-shadows=off` etc.)
        for (flag, on, default) in [("--fewer-shadows", self.fewer_shadows, FEWER_SHADOWS), ("--batch-crowd", self.batch_crowd, BATCH_CROWD), ("--merge-meshes", self.merge_meshes, MERGE_MESHES)] {
            if on != default {
                f.extend([flag.into(), if on { "on" } else { "off" }.into()]);
            }
        }
        if self.mesh != 100 {
            f.extend(["--mesh".into(), self.mesh.to_string()]);
        }
        f
    }

    pub fn preset(&self) -> &'static str {
        Self::PRESETS.iter().find(|(_, g)| self.keep(*g) == *self).map_or("Custom", |(n, _)| n)
    }

    /// Preset `p` with this one's settings that no preset sets (the window, Show FPS, the speed-up switches).
    pub fn keep(&self, p: Graphics) -> Graphics {
        Graphics { windowed: self.windowed, show_fps: self.show_fps, fewer_shadows: self.fewer_shadows, batch_crowd: self.batch_crowd, merge_meshes: self.merge_meshes, cull: self.cull, mesh: self.mesh, ..p }
    }

    /// Whether court meshes are drawn merged (`merge.rs`): the setting, unless `--merge-meshes` said otherwise.
    pub fn merged(&self) -> bool {
        match MERGE_FORCED.load(Ordering::Relaxed) {
            0 => self.merge_meshes,
            f => f == 2,
        }
    }

    /// Whether the scene draws with ambient occlusion: set, not in the original mode, and a GPU that runs it (WebGL2
    /// has no compute shaders for Bevy's SSAO).
    pub fn ao_on(&self) -> bool {
        self.ao > 0 && !self.original && !cfg!(all(target_arch = "wasm32", not(feature = "webgpu")))
    }

    pub fn present_mode(&self) -> PresentMode {
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
        .add_systems(PostUpdate, (window, lights, msaa.after(bevy::camera::CameraUpdateSystems), ao.after(msaa), fps_counter))
        .add_systems(Last, limit);
    // Show FPS → Detailed: where the frame's time goes (`frame_start`)
    use bevy::app::RunFixedMainLoopSystems as F;
    app.init_resource::<Split>()
        .add_systems(First, frame_start)
        .add_systems(RunFixedMainLoop, (mark::<1>.in_set(F::BeforeFixedMainLoop), mark::<2>.in_set(F::AfterFixedMainLoop)))
        .add_systems(Last, mark::<3>.before(limit));
    if let Some(r) = app.get_sub_app_mut(bevy::render::RenderApp) {
        use bevy::render::renderer::{RenderGraph, RenderGraphSystems as G};
        r.add_systems(RenderGraph, (mark::<4>.in_set(G::Begin), mark::<5>.in_set(G::Render).after(bevy::core_pipeline::schedule::camera_driver)))
            .add_systems(bevy::render::Render, mark::<6>.in_set(bevy::render::RenderSystems::Cleanup));
    }
    if std::env::var_os("HST_FPS").is_some() {
        app.add_systems(Last, log_draws);
    }
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
/// presented. On the web all of it runs on one thread in that order; natively rendering overlaps the next frame.
static MARKS: std::sync::Mutex<[Option<bevy::platform::time::Instant>; 7]> = std::sync::Mutex::new([None; 7]);

fn mark<const I: usize>() {
    MARKS.lock().unwrap()[I] = Some(bevy::platform::time::Instant::now());
}

/// Sums of the frames since the counter last showed them, in ms: frame, game (incl. sim), sim, prepare (extract,
/// prepare, queue), encode, submit (+ present), worst frame, frames.
#[derive(Resource, Default)]
struct Split([f32; 8]);

fn frame_start(mut split: ResMut<Split>) {
    let now = bevy::platform::time::Instant::now();
    let mut m = MARKS.lock().unwrap();
    if let [Some(t0), Some(t1), Some(t2), Some(t3), Some(t4), Some(t5), Some(t6)] = *m {
        let ms = |a: bevy::platform::time::Instant, b: bevy::platform::time::Instant| b.saturating_duration_since(a).as_secs_f32() * 1000.0;
        let frame = ms(t0, now);
        for (sum, v) in split.0.iter_mut().zip([frame, ms(t0, t3), ms(t1, t2), ms(t3, t4), ms(t4, t5), ms(t5, t6)]) {
            *sum += v;
        }
        split.0[6] = split.0[6].max(frame);
        split.0[7] += 1.0;
    }
    *m = [Some(now), None, None, None, None, None, None];
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
fn msaa(g: Res<Graphics>, mut q: Query<&mut Msaa, With<crate::hud_gamma::Scene>>, added: Query<(), Added<crate::hud_gamma::Scene>>) {
    if !g.is_changed() && added.is_empty() {
        return;
    }
    // Browser WebGPU only takes 1 or 4 samples (createTexture rejects 2), so 2x draws as 4x there.
    let m = Msaa::from_samples(if g.ao_on() { 1 } else if cfg!(all(target_arch = "wasm32", feature = "webgpu")) && g.msaa == 2 { 4 } else { g.msaa });
    for mut c in &mut q {
        c.set_if_neq(m);
    }
}

/// Ambient occlusion on the 3D cameras that clear the scene (the court's; not an overlay drawing over it, which
/// has its own depth): Bevy's SSAO (GTAO) from a depth + normal prepass (`gs_prepass.wgsl` draws the court
/// models', `gs.wgsl` darkens by it), and SMAA in place of the MSAA it can't run with. Inserted while on, removed
/// with what it brought when turned off, so an off camera draws exactly as before.
#[allow(clippy::type_complexity)]
fn ao(
    mut commands: Commands,
    g: Res<Graphics>,
    q: Query<(Entity, &Camera, Option<&bevy::pbr::ScreenSpaceAmbientOcclusion>, Has<bevy::anti_alias::smaa::Smaa>), With<crate::hud_gamma::Scene>>,
    added: Query<(), Added<crate::hud_gamma::Scene>>,
) {
    use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
    use bevy::pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel as Q};
    if !g.is_changed() && added.is_empty() {
        return;
    }
    let want = g.ao_on().then(|| ScreenSpaceAmbientOcclusion { quality_level: if g.ao == 1 { Q::Low } else { Q::High }, ..default() });
    for (e, cam, now, smaa) in &q {
        let mut c = commands.entity(e);
        match &want {
            Some(w) if !matches!(cam.clear_color, ClearColorConfig::None) => {
                if now != Some(w) {
                    c.insert(w.clone());
                }
                if g.msaa > 1 && !smaa {
                    c.insert(bevy::anti_alias::smaa::Smaa::default());
                } else if g.msaa == 1 && smaa {
                    c.remove::<bevy::anti_alias::smaa::Smaa>();
                }
            }
            _ if now.is_some() || smaa => _ = c.remove::<(ScreenSpaceAmbientOcclusion, DepthPrepass, NormalPrepass, bevy::anti_alias::smaa::Smaa)>(),
            _ => {}
        }
    }
}

/// Shadow quality: off, or the shadow map's size and cascades (high = Bevy's default 2048 and 4 cascades).
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
    // 1024-to-80 m first cascade): each cascade draws every caster again, too many draws for a phone (B328)
    #[cfg(not(all(target_arch = "wasm32", not(feature = "webgpu"))))]
    let (size, cascades, far) = if g.shadows == Shadows::Low { (2048, 1, 120.0) } else { (2048, 4, 120.0) };
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
    let [frame, game, sim, prep, enc, sub, worst, n] = split.0;
    if g.show_fps == 1 {
        let s = format!("{fps:.0} FPS");
        if text.0 != s {
            text.0 = s;
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
        info!("frame split ms: frame {:.2} game {:.2} sim {:.2} render {:.2} (prep {:.2} enc {:.2} submit {:.2}) idle {:.2} meshes {shown} cast {cast}", a(frame), a(game), a(sim), a(render), a(prep), a(enc), a(sub), a((frame - game - render).max(0.0)));
        text.0 = format!(
            "{fps:.0} FPS  {:.1} ms (worst {worst:.1})\ncpu {:.1}  idle {:.1}\n game {:.1} (sim {:.1})\n render {:.1}: prep {:.1} enc {:.1} submit {:.1}\nmeshes {shown} drawn, {cast} cast shadows\n{size} shadows {} msaa {} | {gpu}",
            a(frame),
            a(game + render),
            a((frame - game - render).max(0.0)),
            a(game),
            a(sim),
            a(render),
            a(prep),
            a(enc),
            a(sub),
            g.shadows.name(),
            msaa_name(g.msaa),
        );
    }
    if frame >= 500.0 || g.show_fps == 1 {
        split.0 = [0.0; 8];
    }
    // 16 lines of the PS2's 448 (detailed: 11, it has six)
    let tall = window.single().is_ok_and(|w| w.width() * 3.0 < w.height() * 4.0);
    let px = if g.show_fps == 2 { 11.0 } else { 16.0 };
    let size = if tall { bevy::text::FontSize::Vw(px / 448.0 * 75.0) } else { bevy::text::FontSize::Vh(px / 448.0 * 100.0) };
    if font.font_size != size {
        font.font_size = size;
    }
}

/// The frame limit: sleep away what's left of the frame's time.
// ponytail: thread::sleep overshoots by the OS timer's slack (≈0.1 ms on Linux); spin the last bit if that shows
fn limit(g: Res<Graphics>, mut next: Local<Option<Instant>>) {
    // the browser paces frames itself (and can't sleep)
    if g.fps == 0 || cfg!(target_arch = "wasm32") {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_flags_round_trip() {
        for (_, p) in Graphics::PRESETS {
            let mut g = Graphics { fps: 7, vsync: !p.vsync, scale: 30, shadows: Shadows::Off, msaa: 3, ao: 2, original: !p.original, windowed: true, show_fps: 1, fewer_shadows: !FEWER_SHADOWS, batch_crowd: !BATCH_CROWD, merge_meshes: !MERGE_MESHES, cull: !p.cull, mesh: 50 };
            for (k, v) in p.text().lines().filter_map(|l| l.split_once('=')) {
                assert!(g.set(k, v), "{k}");
            }
            assert_eq!(g, p);
        }
        assert_eq!(Graphics::default().preset(), "Default");
        assert_eq!(Graphics { scale: 75, ..default() }.preset(), "Custom");
        assert_eq!(Graphics { windowed: true, ..default() }.preset(), "Default");
        assert_eq!(Graphics { show_fps: 1, ..default() }.preset(), "Default");
        assert_eq!(Graphics { merge_meshes: true, ..default() }.preset(), "Default");
        assert_eq!(Graphics { merge_meshes: true, ..default() }.flags().last().unwrap(), "on");
        assert_eq!(Graphics { show_fps: 1, ..default() }.flags().last().unwrap(), "--show-fps");
        assert_eq!(Graphics { windowed: true, ..default() }.flags().last().unwrap(), "--windowed");
        assert_eq!(Graphics { cull: false, ..default() }.preset(), "Default");
        assert_eq!(Graphics { cull: false, ..default() }.flags()[8..], ["--cull", "off"]);
        assert_eq!(Graphics::PHONE.flags(), ["--fps", "60", "--scale", "100", "--shadows", "low", "--msaa", "4x"]);
        assert_eq!(Graphics::LOW.flags(), ["--fps", "60", "--scale", "67", "--shadows", "low", "--msaa", "off"]);
        // B343: the speed-up switches pass only off their defaults, keep through a preset and read back as flags
        let g = Graphics { fewer_shadows: !FEWER_SHADOWS, batch_crowd: !BATCH_CROWD, ..default() };
        assert_eq!(g.preset(), "Default");
        assert_eq!(g.keep(Graphics::PHONE), Graphics { fewer_shadows: !FEWER_SHADOWS, batch_crowd: !BATCH_CROWD, ..Graphics::PHONE });
        let f = g.flags();
        let mut back = Graphics::default();
        for kv in f[8..].chunks(2) {
            assert!(back.set(&kv[0][2..], &kv[1]), "{kv:?}");
        }
        assert_eq!(back, g);
        // B364: Mesh keeps through a preset, passes only below 100 and reads back
        let g = Graphics { mesh: 50, ..default() };
        assert_eq!((g.preset(), g.keep(Graphics::LOW).mesh), ("Default", 50));
        assert_eq!(g.flags()[8..], ["--mesh", "50"]);
        let mut back = Graphics::default();
        assert!(back.set("mesh", "50 %") && back == g);
        // G1: Ambient Occlusion is preset-set (off in every one) and passes only when on
        let g = Graphics { ao: 2, ..default() };
        assert_eq!((g.preset(), &g.flags()[8..]), ("Custom", &["--ao".to_string(), "high".into()][..]));
        let mut back = Graphics::default();
        assert!(back.set("ao", "high") && back == g);
    }
}
