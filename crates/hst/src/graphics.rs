//! Video settings (remaster-only): frame limit, vsync, render resolution scale and shadow quality.
//! ponytail: no anti-aliasing setting: changing the scene cameras' MSAA blanks the court (TODO P23b).
//! The menu edits the [`Graphics`] resource (applied live) and passes it to a match as flags
//! (`--fps N`, `--vsync`, `--scale N`, `--shadows off|low|high`). The simulation stays a fixed
//! 60 Hz tick whatever these are. Defaults draw as before these settings existed.

use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
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
}

pub const FPS: [u32; 5] = [30, 60, 120, 144, 0];
pub const SCALES: [u32; 5] = [50, 67, 75, 85, 100];
pub const SHADOWS: [Shadows; 3] = [Shadows::Off, Shadows::Low, Shadows::High];

impl Default for Graphics {
    fn default() -> Self {
        Graphics { fps: 0, vsync: false, scale: 100, shadows: Shadows::High }
    }
}

impl Graphics {
    /// The Steam Deck's: its native 1280×800, cheaper shadows, vsync on (picked on a Deck's first run).
    pub const STEAM_DECK: Graphics = Graphics { fps: 0, vsync: true, scale: 100, shadows: Shadows::Low };
    /// The cheapest that still looks like the game: for weak GPUs.
    pub const LOW: Graphics = Graphics { fps: 60, vsync: false, scale: 67, shadows: Shadows::Low };
    pub const PRESETS: [(&str, Graphics); 3] = [("Default", Graphics { fps: 0, vsync: false, scale: 100, shadows: Shadows::High }), ("Steam Deck", Self::STEAM_DECK), ("Low", Self::LOW)];

    /// Read one `key = value` (settings.txt or a flag's name and value); false if the key isn't one of these.
    pub fn set(&mut self, key: &str, v: &str) -> bool {
        let v = v.trim();
        match key.trim() {
            "fps" | "frame_limit" => self.fps = if v == "uncapped" { 0 } else { v.parse().unwrap_or(self.fps) },
            "vsync" => self.vsync = v != "off",
            "scale" | "render_scale" => self.scale = v.trim_end_matches('%').parse::<u32>().map_or(self.scale, |s| s.clamp(25, 100)),
            "shadows" => self.shadows = SHADOWS.into_iter().find(|s| s.name() == v).unwrap_or(self.shadows),
            _ => return false,
        }
        true
    }

    /// settings.txt lines.
    pub fn text(&self) -> String {
        let fps = if self.fps == 0 { "uncapped".to_string() } else { self.fps.to_string() };
        format!("frame_limit = {fps}\nvsync = {}\nrender_scale = {}\nshadows = {}\n", if self.vsync { "on" } else { "off" }, self.scale, self.shadows.name())
    }

    /// The match's flags.
    pub fn flags(&self) -> Vec<String> {
        let mut f = vec!["--fps".into(), self.fps.to_string(), "--scale".into(), self.scale.to_string(), "--shadows".into(), self.shadows.name().into()];
        if self.vsync {
            f.push("--vsync".into());
        }
        f
    }

    pub fn preset(&self) -> &'static str {
        Self::PRESETS.iter().find(|(_, g)| g == self).map_or("Custom", |(n, _)| n)
    }

    pub fn present_mode(&self) -> PresentMode {
        if self.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync }
    }
}

impl Shadows {
    pub fn name(self) -> &'static str {
        ["off", "low", "high"][self as usize]
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Graphics>()
        .add_systems(PostUpdate, (window, lights))
        .add_systems(Last, limit);
}

fn window(g: Res<Graphics>, mut w: Query<&mut Window, With<PrimaryWindow>>) {
    if g.is_changed() {
        for mut w in &mut w {
            w.present_mode = g.present_mode();
        }
    }
}

/// Shadow quality: off, or the shadow map's size and cascades (high = Bevy's default 2048 and 4 cascades).
fn lights(mut commands: Commands, g: Res<Graphics>, mut map: ResMut<DirectionalLightShadowMap>, mut lights: Query<(Entity, Mut<DirectionalLight>)>) {
    if !g.is_changed() && !lights.iter().any(|(_, l)| l.is_added()) {
        return;
    }
    let (size, cascades) = if g.shadows == Shadows::Low { (1024, 2) } else { (2048, 4) };
    if map.size != size {
        map.size = size;
    }
    for (e, mut l) in &mut lights {
        l.shadow_maps_enabled = g.shadows != Shadows::Off;
        commands.entity(e).insert(CascadeShadowConfigBuilder { num_cascades: cascades, first_cascade_far_bound: 30.0, maximum_distance: 120.0, ..default() }.build());
    }
}

/// The frame limit: sleep away what's left of the frame's time.
// ponytail: thread::sleep overshoots by the OS timer's slack (≈0.1 ms on Linux); spin the last bit if that shows
fn limit(g: Res<Graphics>, mut next: Local<Option<Instant>>) {
    if g.fps == 0 {
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
            let mut g = Graphics { fps: 7, vsync: !p.vsync, scale: 30, shadows: Shadows::Off };
            for (k, v) in p.text().lines().filter_map(|l| l.split_once('=')) {
                assert!(g.set(k, v), "{k}");
            }
            assert_eq!(g, p);
        }
        assert_eq!(Graphics::default().preset(), "Default");
        assert_eq!(Graphics { scale: 75, ..default() }.preset(), "Custom");
        assert_eq!(Graphics::LOW.flags(), ["--fps", "60", "--scale", "67", "--shadows", "low"]);
    }
}
