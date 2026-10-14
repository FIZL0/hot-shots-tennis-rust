//! Video settings (remaster-only): borderless fullscreen or a window, frame limit, Show FPS, vsync, render resolution scale, shadow quality, MSAA and the original
//! 640×448 mode (`hud_gamma.rs` draws it; it also fixes the aspect ratio to 4:3 and turns upscaled textures off, `main.rs`).
//! The menu edits the [`Graphics`] resource (applied live) and passes it to a match as flags
//! (`--windowed`, `--fps N`, `--vsync`, `--scale N`, `--shadows off|low|high`, `--msaa off|2x|4x`, `--original`). The simulation stays a fixed
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
    /// Draw the frame rate in the window's top-right corner (B186; `fps_counter`). Not part of a preset.
    pub show_fps: bool,
}

/// Set for `--shot` and unattended (HST_BG) runs: their fixed-size window never goes fullscreen (`main.rs`).
pub static KEEP_WINDOWED: AtomicBool = AtomicBool::new(false);

pub const FPS: [u32; 5] = [30, 60, 120, 144, 0];
pub const SCALES: [u32; 5] = [50, 67, 75, 85, 100];
pub const SHADOWS: [Shadows; 3] = [Shadows::Off, Shadows::Low, Shadows::High];
pub const MSAA: [u32; 3] = [1, 2, 4];

impl Default for Graphics {
    fn default() -> Self {
        Graphics { fps: 0, vsync: false, scale: 100, shadows: Shadows::High, msaa: 4, original: false, windowed: false, show_fps: false }
    }
}

impl Graphics {
    /// The Steam Deck's: its native 1280×800, cheaper shadows, vsync on (picked on a Deck's first run).
    pub const STEAM_DECK: Graphics = Graphics { fps: 0, vsync: true, scale: 100, shadows: Shadows::Low, msaa: 4, original: false, windowed: false, show_fps: false };
    /// The cheapest that still looks like the game: for weak GPUs.
    pub const LOW: Graphics = Graphics { fps: 60, vsync: false, scale: 67, shadows: Shadows::Low, msaa: 1, original: false, windowed: false, show_fps: false };
    pub const PRESETS: [(&str, Graphics); 3] = [("Default", Graphics { fps: 0, vsync: false, scale: 100, shadows: Shadows::High, msaa: 4, original: false, windowed: false, show_fps: false }), ("Steam Deck", Self::STEAM_DECK), ("Low", Self::LOW)];

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
            "show_fps" => self.show_fps = v != "off",
            "shadows" => self.shadows = SHADOWS.into_iter().find(|s| s.name() == v).unwrap_or(self.shadows),
            _ => return false,
        }
        true
    }

    /// settings.txt lines.
    pub fn text(&self) -> String {
        let fps = if self.fps == 0 { "uncapped".to_string() } else { self.fps.to_string() };
        format!("frame_limit = {fps}\nvsync = {}\nrender_scale = {}\nshadows = {}\nanti_aliasing = {}\noriginal = {}\nwindowed = {}\nshow_fps = {}\n", if self.vsync { "on" } else { "off" }, self.scale, self.shadows.name(), msaa_name(self.msaa), if self.original { "on" } else { "off" }, if self.windowed { "on" } else { "off" }, if self.show_fps { "on" } else { "off" })
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
        if self.show_fps {
            f.push("--show-fps".into());
        }
        f
    }

    pub fn preset(&self) -> &'static str {
        Self::PRESETS.iter().find(|(_, g)| Graphics { windowed: self.windowed, show_fps: self.show_fps, ..*g } == *self).map_or("Custom", |(n, _)| n)
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
        .add_systems(PostUpdate, (window, lights, msaa.after(bevy::camera::CameraUpdateSystems), fps_counter))
        .add_systems(Last, limit);
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
    let m = Msaa::from_samples(g.msaa);
    for mut c in &mut q {
        c.set_if_neq(m);
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
    // shadows on the far half faded out
    #[cfg(not(all(target_arch = "wasm32", not(feature = "webgpu"))))]
    let (size, cascades, far) = if g.shadows == Shadows::Low { (1024, 2, 120.0) } else { (2048, 4, 120.0) };
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

/// Show FPS: the smoothed frame rate at the window's top-right corner, over menus and matches alike. Sized on the
/// 4:3 screen's height (the window's, or its width × 3/4 when taller than 4:3, `widescreen::vplace`) so it scales
/// uniformly with the HUD.
fn fps_counter(mut commands: Commands, g: Res<Graphics>, d: Res<bevy::diagnostic::DiagnosticsStore>, window: Query<&Window, With<PrimaryWindow>>, mut q: Query<(Entity, &mut Text, &mut TextFont), With<FpsText>>) {
    let Ok((e, mut text, mut font)) = q.single_mut() else {
        if g.show_fps {
            // bottom-left: the menus' header holds pads/disc/version top-right, the legend bar's left end is empty
            let node = Node { position_type: PositionType::Absolute, bottom: Val::Percent(1.0), left: Val::Percent(1.0), ..default() };
            commands.spawn((FpsText, Text::new(""), TextFont::default(), TextColor(Color::srgb(1.0, 1.0, 0.3)), TextShadow::default(), node, GlobalZIndex(i32::MAX)));
        }
        return;
    };
    if !g.show_fps {
        commands.entity(e).despawn();
        return;
    }
    let fps = d.get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS).and_then(|f| f.smoothed()).unwrap_or(0.0);
    let s = format!("{fps:.0} FPS");
    if text.0 != s {
        text.0 = s;
    }
    // 16 lines of the PS2's 448
    let tall = window.single().is_ok_and(|w| w.width() * 3.0 < w.height() * 4.0);
    let size = if tall { bevy::text::FontSize::Vw(16.0 / 448.0 * 75.0) } else { bevy::text::FontSize::Vh(16.0 / 448.0 * 100.0) };
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
            let mut g = Graphics { fps: 7, vsync: !p.vsync, scale: 30, shadows: Shadows::Off, msaa: 3, original: !p.original, windowed: true, show_fps: true };
            for (k, v) in p.text().lines().filter_map(|l| l.split_once('=')) {
                assert!(g.set(k, v), "{k}");
            }
            assert_eq!(g, p);
        }
        assert_eq!(Graphics::default().preset(), "Default");
        assert_eq!(Graphics { scale: 75, ..default() }.preset(), "Custom");
        assert_eq!(Graphics { windowed: true, ..default() }.preset(), "Default");
        assert_eq!(Graphics { show_fps: true, ..default() }.preset(), "Default");
        assert_eq!(Graphics { show_fps: true, ..default() }.flags().last().unwrap(), "--show-fps");
        assert_eq!(Graphics { windowed: true, ..default() }.flags().last().unwrap(), "--windowed");
        assert_eq!(Graphics::LOW.flags(), ["--fps", "60", "--scale", "67", "--shadows", "low", "--msaa", "off"]);
    }
}
