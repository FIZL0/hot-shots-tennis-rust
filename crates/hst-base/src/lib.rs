//! The game's lower layer, below the match (`hst-play`) and the app (`hst`): disc assets and how they're drawn
//! (characters, the GS material, shadows, effects, weather), sound, mods and the mod store, the online lobby
//! transport, the platform glue (browser, iOS, Android) and the few app-wide types the rest share (P60).

#[cfg(any(target_os = "android", test))]
pub mod android_pad;
pub mod audio;
pub mod character;
pub mod court_anim;
pub mod cull;
#[cfg(not(target_arch = "wasm32"))]
pub mod disc;
#[cfg(target_os = "ios")]
pub mod ios;
#[cfg(target_os = "android")]
pub mod android;
pub mod effects;
pub mod graphics;
pub mod handshake;
pub mod gs;
pub mod modfs;
pub mod modstore;
pub mod mods;
pub mod mods_online;
pub mod mods_umpire;
pub mod mods_court;
pub mod mods_textures;
pub mod court_mod;
pub mod net;
pub mod chat;
pub mod hud_gamma;
pub mod inspect;
pub mod lod;
pub mod noise;
pub mod jobs;
pub mod seats;
pub mod shade;
pub mod shadow;
pub mod spring;
pub mod stall;
pub mod textures;
pub mod ui_census;
pub mod scene_census;
pub mod ui_pool;
pub mod vertex_patch;
pub mod weather;
#[cfg(target_arch = "wasm32")]
pub mod web;
pub mod widescreen;

use bevy::prelude::*;

/// settings.txt, controls.txt and online.txt: files beside the disc, or in the browser its localStorage.
pub mod store {
    #[cfg(not(target_arch = "wasm32"))]
    pub use std::fs::{read_to_string, write};
    #[cfg(target_arch = "wasm32")]
    pub use crate::web::{read_to_string, write};

    /// settings.txt beside the disc (`main_menu::Settings::path`).
    pub fn settings(iso: &str) -> std::path::PathBuf {
        std::path::Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("settings.txt")
    }
}

#[derive(Resource, Clone)]
pub struct Args {
    pub iso: String,
    pub archives: Vec<String>,
    pub shot: Option<String>,
    /// Seconds of game time before the shot (`--shot-at`, default 0.5).
    pub shot_at: f32,
    pub radius: Option<f32>,
    pub ball: bool,
    pub court: usize,
    pub stage: Option<u32>,
    pub play: bool,
    pub singles: bool,
    /// Characters on court (player order), `--chars 0,2,1,5`.
    pub chars: Vec<usize>,
    /// Their outfits (costumes 0..9), `--outfits 9,9,4,9`: the model, the panel face and the computer players' AI row.
    pub outfits: Vec<usize>,
    /// Character viewer: `--character N [--motion M]`.
    pub viewer: Option<(usize, usize)>,
    /// Sound audition: archive, bank header, program, key.
    pub sound: Option<(String, String, usize, usize)>,
    /// `--music`: the court's BGM in a match (off by default).
    pub music: bool,
    /// The umpire (`--umpire N`, the menu's order: 0 Chika, 1 Suzuki, 2 Lily, 3 Robot Tennis, 4 Anna; default 4).
    pub umpire: u8,
    /// Sets to win (`--sets`, default 1) and games to win a set (`--games`, default 4).
    pub sets: i32,
    pub games: i32,
    /// `--pads a,b,c,d`: what drives seats 1P..4P (`k` the keyboard, a gamepad's index into the sorted pad list, `-`
    /// the computer), as the main menu assigned them (`controls::PadSlots`).
    pub pads: Option<[Option<seats::Seat>; 4]>,
}

#[derive(Component)]
pub struct Orbit {
    pub focus: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Orbit {
    /// The camera's transform on this rig.
    pub fn transform(&self) -> Transform {
        let rot = Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        Transform::from_translation(self.focus + rot * Vec3::Z * self.radius).looking_at(self.focus, Vec3::Y)
    }
}

/// Parent for everything in game space: game space is Y-down, a half-turn about X maps it to Bevy's
/// Y-up without mirroring.
#[derive(Component)]
pub struct GameSpace;

/// A sky, `_clo`, `_acc` or `_bg` model: the court draw stands it at the camera's eye + (0, lift, 0), turned by yaw
/// about game y (`gs::backdrop`; slot 5's RAM has these matrices).
#[derive(Component)]
pub struct Backdrop {
    pub yaw: f32,
    pub lift: f32,
}

/// The backdrops' blended-draw sort offset (`GsMaterial::order`): before the hole's `-1e6`, so before everything.
pub const BACKDROP: f32 = -2.0e6;

/// The phone builds turn the frame-rate log on here: setting `HST_FPS` while the OS's own threads read the
/// environment is undefined behaviour.
pub static FPS_LOG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `HST_FPS=1` or `FPS_LOG`.
pub fn fps_log() -> bool {
    FPS_LOG.load(std::sync::atomic::Ordering::Relaxed) || std::env::var_os("HST_FPS").is_some()
}

/// A `play/` module's tick state for a resync (`play::snapshot::encode`): as bytes, then this machine's own entities
/// and handles put back (`local`; the handles go as their defaults). Here so `effects` and `weather` can be one.
pub trait Wire: Resource + Clone + serde::Serialize + serde::de::DeserializeOwned {
    fn local(&mut self, _local: &Self) {}
}
