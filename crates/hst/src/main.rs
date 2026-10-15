//! `hst` alone imports the disc on first start and runs from that pack after (see disc.rs); `hst <iso> …` runs
//! straight from an ISO (or a pack).
//! Viewer / sandbox: `hst <iso> <archive.XB>... [--ball] [--court N] [--radius r] [--shot out.png] [--windowed]`
//! Loads every model in the given disc archives and shows them with an orbit camera
//! (drag: orbit, wheel: zoom). `--shot` saves one frame (after `--shot-at` seconds) and exits, for unattended checks;
//! `--radius r` orbits the origin at distance r instead of framing everything (skyboxes are huge).
//! `--stage NN` loads disc court NN (01..11) with every prop placed from its layout data.
//! `--ball` adds the game ball driven by the ported physics (Space: new shot); `--court` picks the
//! physics surface table (0..11). `--play` is a playable match against a simple AI (see play.rs).
//! `--sound <archive> <bank.hd> <program> <key>` plays one sound of a bank (see audio.rs); a BGM archive
//! (`--sound SND/BGM/BGMM_05.XB data/sound/BGM/Menu/bgmm_05.hd`) plays its music. `--music` turns on the court's
//! BGM in a `--play` match (off by default).
//! `--inspect all|N[:C],…|MODDIR[:C],…` shows characters (costume C) and mods posed as the original's inspect screen, side
//! by side (default: the disc roster; see inspect.rs).
//! Textures come from `mods/texture-replacements/` (key-named files over a PCSX2 pack's) over the disc (beside the
//! ISO; see textures.rs); `hst <iso> --dump-textures` writes every disc texture to `mods/textures-src/` to edit or
//! upscale into `mods/texture-replacements/`.

#[cfg(any(target_os = "android", test))]
mod android_pad;
mod audio;
mod character;
mod court_anim;
mod cull;
#[cfg(not(target_arch = "wasm32"))]
mod disc;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "android")]
mod android;
mod effects;
mod graphics;
mod handshake;
mod gs;
mod modfs;
mod modstore;
mod mods;
mod mods_online;
mod mods_umpire;
mod mods_court;
mod court_mod;
mod net;
mod chat;
mod hud_gamma;
mod inspect;
mod loading;
mod mash;
mod lod;
mod mode;
mod noise;
mod play;
mod jobs;
mod preload;
mod sandbox;
mod shade;
mod shadow;
mod spring;
mod stall;
mod textures;
mod ui_census;
mod scene_census;
mod ui_pool;
mod update;
mod vertex_patch;
mod weather;
#[cfg(target_arch = "wasm32")]
mod web;

/// settings.txt, controls.txt and online.txt: files beside the disc, or in the browser its localStorage.
mod store {
    #[cfg(not(target_arch = "wasm32"))]
    pub use std::fs::{read_to_string, write};
    #[cfg(target_arch = "wasm32")]
    pub use crate::web::{read_to_string, write};
}

use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use hst_data::{iso::Iso, layout, mdl, mtl, xb::Archive};

#[derive(Resource, Clone)]
pub struct Args {
    pub iso: String,
    archives: Vec<String>,
    shot: Option<String>,
    /// Seconds of game time before the shot (`--shot-at`, default 0.5).
    shot_at: f32,
    radius: Option<f32>,
    ball: bool,
    pub court: usize,
    stage: Option<u32>,
    play: bool,
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
    pub pads: Option<[Option<play::controls::Seat>; 4]>,
}

#[derive(Component)]
pub struct Orbit {
    pub focus: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}

/// The command line: the app's `Args` and the flags that only pick how it starts.
struct Cli {
    args: Args,
    gfx: graphics::Graphics,
    upscale: bool,
    viewer_mod: Option<String>,
    mod_slot: usize,
    slot_mods: Vec<(usize, String)>,
    roster: Option<inspect::Roster>,
    /// `--swap-hands 1,0`: the select's hand toggle by match slot (`play::hand_swap`).
    swap_hands: play::hand_swap::SwapHands,
    /// `--names a,,b`: the players' names by match slot (B371, `play::name_tags`).
    names: play::name_tags::Names,
    /// M8f3: `--umpire-mod DIR`, or `--umpire-mod-id ID` (online: the room's, if installed under `mods/`).
    umpire_mod: Option<mods_umpire::Umpire>,
    /// M8f6: `--court-mod DIR`, drawn and heard over its layout court.
    court_mod: Option<mods_court::Court>,
}

fn parse(mut a: impl Iterator<Item = String>) -> Cli {
    let iso = a.next().expect("usage: hst <iso> <archive.XB>... [--shot out.png]");
    let (mut archives, mut shot, mut radius, mut ball, mut court, mut stage, mut play, mut singles, mut chars) = (Vec::new(), None, None, false, 0, None, false, false, Vec::new());
    let (mut viewer_char, mut viewer_motion) = (None, 0);
    // drawing runs uncapped by default; the simulation stays a fixed 60 Hz tick either way
    let mut gfx = graphics::Graphics::default();
    let mut music = false;
    let mut shot_at = 0.5;
    let mut sound = None;
    let mut swap_hands = play::hand_swap::SwapHands::default();
    let mut names = play::name_tags::Names::default();
    let mut outfits = Vec::new();
    let mut viewer_mod = None;
    let mut mod_slot = 0;
    let mut slot_mods = Vec::new();
    let (mut umpire, mut sets, mut games, mut pads, mut upscale) = (4, 1, 4, None, true);
    let mut roster = None;
    let (mut umpire_given, mut umpire_mod, mut umpire_mod_id) = (false, None, None);
    let (mut court_mod, mut court_mod_id) = (None::<String>, None::<String>);
    while let Some(x) = a.next() {
        match x.as_str() {
            "--shot" => shot = a.next(),
            "--shot-at" => shot_at = a.next().and_then(|r| r.parse().ok()).unwrap_or(shot_at),
            "--radius" => radius = a.next().and_then(|r| r.parse().ok()),
            "--ball" => ball = true,
            "--court" => court = a.next().and_then(|r| r.parse().ok()).unwrap_or(0),
            "--stage" => stage = a.next().and_then(|r| r.parse().ok()),
            "--play" => play = true,
            "--singles" => singles = true,
            "--character" => viewer_char = a.next().and_then(|r| r.parse().ok()),
            "--mod" => (viewer_mod, viewer_char) = (a.next(), viewer_char.or(Some(0))),
            "--mod-slot" => mod_slot = a.next().and_then(|r| r.parse().ok()).unwrap_or(0),
            "--slot-mod" => {
                let (n, dir) = (a.next().and_then(|r| r.parse::<usize>().ok()), a.next());
                if let (Some(n), Some(dir)) = (n, dir) {
                    slot_mods.push((n, dir));
                }
            }
            "--motion" => viewer_motion = a.next().and_then(|r| r.parse().ok()).unwrap_or(0),
            "--inspect" => {
                let list = a.next().filter(|l| l != "all").unwrap_or_else(|| (0..14).map(|n| n.to_string()).collect::<Vec<_>>().join(","));
                let (mut disc, mut mods) = (Vec::new(), Vec::new());
                for e in list.split(',') {
                    let (n, c) = e.split_once(':').unwrap_or((e, "0"));
                    match (n.parse(), c.parse()) {
                        (Ok(n), Ok(c)) => disc.push((n, c)),
                        _ => mods.push(e.to_string()),
                    }
                }
                roster = Some(inspect::Roster(disc, mods));
            }
            "--vsync" => gfx.vsync = true,
            "--original" => gfx.original = true,
            "--show-fps" => gfx.show_fps = 1,
            "--show-fps-detailed" => gfx.show_fps = 2,
            "--fps" | "--scale" | "--shadows" | "--msaa" | "--fewer-shadows" | "--mesh" => _ = gfx.set(&x[2..], &a.next().unwrap_or_default()),
            "--cull" => {
                let v = a.next().unwrap_or_default();
                gfx.set("cull", &v);
                cull::force(&v);
            }
            // borderless fullscreen is the default (B96); `--fullscreen` stays for older launch scripts
            "--windowed" => gfx.windowed = true,
            "--fullscreen" => gfx.windowed = false,
            "--music" => music = true,
            "--umpire" => (umpire, umpire_given) = (a.next().and_then(|r| r.parse().ok()).filter(|&u| u < 5).unwrap_or(umpire), true),
            "--umpire-mod" => umpire_mod = a.next(),
            "--umpire-mod-id" => umpire_mod_id = a.next(),
            "--court-mod" => court_mod = a.next(),
            "--court-mod-id" => court_mod_id = a.next(),
            "--sets" => sets = a.next().and_then(|r| r.parse().ok()).unwrap_or(sets),
            "--games" => games = a.next().and_then(|r| r.parse().ok()).unwrap_or(games),
            "--pads" => {
                let v: Vec<_> = a.next().unwrap_or_default().split(',').map(play::controls::Seat::parse).collect();
                pads = Some(std::array::from_fn(|i| v.get(i).copied().flatten()));
            }
            "--4x3" => play::main_menu::ASPECT.store(1, std::sync::atomic::Ordering::Relaxed),
            // B117: Auto, 4:3, 16:9, 16:10, 21:9
            "--aspect" => {
                let a = a.next().unwrap_or_default();
                play::main_menu::ASPECT.store(play::widescreen::ASPECTS.iter().position(|n| *n == a).unwrap_or(0) as u8, std::sync::atomic::Ordering::Relaxed);
            }
            "--no-upscale" => upscale = false,
            // B374: no check for a newer release this run
            "--no-update-check" => update::NO_CHECK_FLAG.store(true, std::sync::atomic::Ordering::Relaxed),
            "--sound" => {
                let mut n = || a.next().unwrap_or_default();
                let (xb, hd, program, key) = (n(), n(), n(), n());
                sound = Some((xb, hd, program.parse().unwrap_or(0), key.parse().unwrap_or(0)));
            }
            "--dump-textures" => {
                let mut iso_ = Iso::open(&iso).expect("open iso");
                let o = hst_data::texhash::Overrides::scan(std::path::Path::new(&iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()));
                let n = o.dump(&mut iso_);
                println!("wrote {n} disc textures to {} (upscale them into mods/texture-replacements)", o.root.join("mods/textures-src").display());
                std::process::exit(0);
            }
            "--swap-hands" => swap_hands = play::hand_swap::SwapHands::parse(&a.next().unwrap_or_default()),
            "--names" => names = play::name_tags::Names::parse(&a.next().unwrap_or_default()),
            "--chars" => chars = a.next().map(|s| s.split(',').filter_map(|c| c.trim().parse().ok()).collect()).unwrap_or_default(),
            "--outfits" => outfits = a.next().map(|s| s.split(',').filter_map(|c| c.trim().parse().ok()).collect()).unwrap_or_default(),
            _ => archives.push(x),
        }
    }
    // the original's picture: 4:3 with the disc's own textures
    if gfx.original {
        play::main_menu::ASPECT.store(1, std::sync::atomic::Ordering::Relaxed);
        upscale = false;
    }
    // unattended runner slots (HST_BG=1) debug against the disc's own textures, never the upscaled pack
    if std::env::var_os("HST_BG").is_some() {
        upscale = false;
    }
    // per parse, so a match after an `--original` one gets the remastered menu sheets back
    play::menu_art::OFF.store(gfx.original, std::sync::atomic::Ordering::Relaxed);
    // M8f3: an umpire mod is heard over its donor; `--umpire` (the menu's and the online room's: the donor) keeps
    // the timing whatever the mod's manifest says
    let umpire_mod = match (umpire_mod, umpire_mod_id) {
        (Some(dir), _) => mods_umpire::read(dir.as_ref()).map_err(|e| warn!("{e}")).ok(),
        (None, Some(id)) => {
            let found = mods_umpire::find(&play::main_menu::Settings::path(&iso).with_file_name("mods"), &id);
            if found.is_none() {
                info!("umpire mod {id} not installed: umpire {umpire} calls the match");
            }
            found
        }
        (None, None) => None,
    };
    // M8f4: its model on the chair, whichever way it was picked
    *mods::umpire::CHOSEN.lock().unwrap() = umpire_mod.as_ref().map(|u| u.dir.clone());
    if let Some(u) = umpire_mod.as_ref().filter(|_| !umpire_given) {
        umpire = u.donor;
    }
    // M8f6: a court mod plays on its layout court (`--stage`/`--court` default to it)
    let court_mod = match (court_mod, court_mod_id) {
        (Some(dir), _) => mods_court::read(dir.as_ref()).map_err(|e| warn!("{e}")).ok(),
        // M8f8: an online room's, by id; not installed (or another layout) → the disc court the room plays on
        (None, Some(id)) => {
            let found = mods_court::find(&play::main_menu::Settings::path(&iso).with_file_name("mods"), &id).filter(|c| stage.is_none_or(|s| s == c.layout as u32 + 1));
            if found.is_none() {
                info!("court mod {id} not installed: the disc court is played");
            }
            found
        }
        (None, None) => None,
    };
    if let Some(c) = &court_mod {
        stage = stage.or(Some(c.layout as u32 + 1));
        if court == 0 {
            court = c.layout as usize + 1;
        }
    }
    let args = Args { iso, archives, shot, shot_at, radius, ball, court, stage, play, singles, chars, outfits, viewer: viewer_char.map(|c| (c, viewer_motion)), sound, music, umpire, sets, games, pads };
    Cli { args, gfx, upscale, viewer_mod, mod_slot, slot_mods, roster, swap_hands, names, umpire_mod, court_mod }
}

// the browser build starts from `web::run` once the page has the disc pack
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    #[cfg(target_os = "ios")]
    ios::log_to_documents(&disc::data_dir());
    // `hst net …`: the lobby flow in a terminal (`net::cli`)
    // `hst net … --play [disc] [flags]`: the online match once everyone's in
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.first().map(String::as_str) == Some("net") {
        let play = argv.iter().position(|x| x == "--play");
        let rest = play.map(|i| argv.split_off(i)[1..].to_vec());
        let Some(started) = net::cli(argv.split_off(1), rest.is_some()) else { return };
        *play::netplay::PENDING.lock().unwrap() = Some(started);
        argv = rest.unwrap_or_default();
    }
    // B374: where the game's file is, before an update swaps it; a Windows update's leftover opentennis.exe.old goes
    update::cleanup();
    disc::resolve(&mut argv);
    start(argv);
}

// Android (NativeActivity, `tools/apk.sh`): the disc is the pack imported into the app's external files dir
// (/sdcard/Android/data/<package>/files), where settings.txt etc. go too, else the first .iso there (adb push);
// with neither, the start page that imports one (`android.rs`, M99)
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: bevy::android::android_activity::AndroidApp) {
    let dir = app.external_data_path().expect("no external files dir");
    android::log_to_files(&dir);
    let _ = bevy::android::ANDROID_APP.set(app);
    // mods/ and the other paths the desktop build keeps beside the binary land in the same dir
    let _ = std::env::set_current_dir(&dir);
    let iso = std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("iso")));
    let Some(disc) = Some(dir.join("disc.pack")).filter(|p| p.exists()).or(iso) else { android::no_disc(dir) };
    start(vec![disc.to_string_lossy().into_owned()]);
}

/// The app for `argv` = disc, flags (see the module doc).
fn start(argv: Vec<String>) {
    let Cli { args, mut gfx, upscale, viewer_mod, mod_slot, slot_mods, roster, swap_hands, names, umpire_mod, court_mod } = parse(argv.into_iter());
    // nothing asked for: the main menu, which starts matches in this app (`mode.rs`) with the flags above
    // a bad archive name would otherwise panic inside `load` and take the renderer down with it
    if !args.archives.is_empty() && let Ok(mut iso) = Iso::open(&args.iso) {
        for path in &args.archives {
            if let Err(e) = iso.read(path) {
                eprintln!("hst: archive {path:?}: {e} (usage: hst <iso> [<archive.XB>...] [--flags])");
                std::process::exit(2);
            }
        }
    }
    let menu = args.archives.is_empty() && args.stage.is_none() && !args.play && !args.ball && args.viewer.is_none() && args.sound.is_none() && roster.is_none();
    // `--windowed` on the command line holds for the whole run, over the menu's setting
    let windowed = gfx.windowed;
    // the menu opens the window as settings.txt's graphics say, so it doesn't start fullscreen and then shrink
    if menu && let Ok(t) = store::read_to_string(play::main_menu::Settings::path(&args.iso)) {
        for (k, v) in t.lines().filter_map(|l| l.split('#').next()?.split_once('=')) {
            gfx.set(k, v);
        }
    }
    gfx.cull = cull::forced().unwrap_or(gfx.cull);
    cull::set(gfx.cull);
    // B222: settings.txt's `original = on` too: the disc's own textures, no texture pack (its 4× walls would tile 4× big)
    play::menu_art::OFF.store(gfx.original, std::sync::atomic::Ordering::Relaxed);
    if upscale && !gfx.original {
        textures::init(&args.iso);
    }
    let mut app = App::new();
    let present_mode = gfx.present_mode();
    // a `--shot` window, or any run under the unattended runners (HST_BG=1), has its own title so the desktop can keep
    // test windows out of the way (tools/shot.sh); those runs are also silent
    let bg = args.shot.is_some() || std::env::var_os("HST_BG").is_some();
    let title = if bg { "Hot Shots Tennis (shot)" } else { "OpenTennis" };
    // borderless fullscreen on the current monitor unless `--windowed`/settings say otherwise; test windows stay windows
    // (a browser only allows fullscreen from a click, so the web build stays in its page)
    graphics::KEEP_WINDOWED.store(windowed || bg || cfg!(target_arch = "wasm32"), std::sync::atomic::Ordering::Relaxed);
    let mode = gfx.window_mode();
    // `HST_WINDOW="3840 2160"`: the window's size in physical pixels (unattended shots at a set resolution)
    let mut resolution = bevy::window::WindowResolution::default();
    if let Some((w, h)) = std::env::var("HST_WINDOW").ok().and_then(|v| v.split_once(' ').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))) {
        resolution = bevy::window::WindowResolution::new(w, h).with_scale_factor_override(1.0);
    }
    // HST_WEBGL2=1: the web WebGL2 build's limits on the desktop (no storage buffers, CPU batching, uniform skins), with
    // the browser's usual 8k/2k (3D) textures, to reproduce WebGL2-only renderer bugs natively (B385)
    let mut render = bevy::render::settings::WgpuSettings::default();
    if std::env::var_os("HST_WEBGL2").is_some() {
        render.priority = bevy::render::settings::WgpuSettingsPriority::WebGL2;
        render.limits = bevy::render::settings::WgpuLimits { max_texture_dimension_2d: 8192, max_texture_dimension_3d: 2048, ..bevy::render::settings::WgpuLimits::downlevel_webgl2_defaults() };
    }
    let plugins = DefaultPlugins.set(bevy::render::RenderPlugin {
        render_creation: render.into(),
        // P47: `HST_SYNC_PIPELINES=1` compiles pipelines on the render thread, as the browser and macOS always do
        synchronous_pipeline_compilation: std::env::var_os("HST_SYNC_PIPELINES").is_some(),
        ..default()
    }).set(WindowPlugin { primary_window: Some(Window { title: title.into(), name: Some("opentennis".into()), present_mode, mode, resolution, ..default() }), ..default() });
    // iOS: the log goes to Documents/hst-log.txt too (`ios.rs`); winit logs every redraw at info there, a line a frame,
    // and warns of each event it hands over while redrawing (P49: 14.5k of a 4-minute log's 15.8k lines)
    #[cfg(target_os = "ios")]
    let plugins = plugins.set(bevy::log::LogPlugin { custom_layer: ios::log_layer, filter: format!("{},winit=warn,winit::platform_impl::ios::app_state=error", bevy::log::DEFAULT_FILTER), ..default() });
    // Android: into hst-log.txt too (`android.rs`)
    #[cfg(target_os = "android")]
    let plugins = plugins.set(bevy::log::LogPlugin { fmt_layer: android::log_layer, ..default() });
    app.add_plugins(plugins);
    #[cfg(target_os = "ios")]
    app.add_systems(Last, ios::heartbeat);
    #[cfg(target_os = "android")]
    app.add_systems(Last, android::heartbeat);
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(Update, window_icon);
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Last, (web::shown, web::frame_rate));
    if bg {
        app.insert_resource(GlobalVolume::new(bevy::audio::Volume::SILENT));
    }
    app.add_plugins((audio::plugin, gs::plugin, shadow::plugin, court_anim::plugin, effects::plugin, textures::plugin, hud_gamma::plugin, weather::plugin));
    app.add_plugins(shade::plugin);
    app.add_plugins(cull::plugin);
    // the match's effect material: render side, so here in the base app (`mode.rs`), not in `play::plugin` (B148)
    app.add_plugins(MaterialPlugin::<effects::Fx>::default());
    // the umpire's calls' (P33)
    app.add_plugins(MaterialPlugin::<effects::OnTop>::default());
    app.add_systems(PostUpdate, effects::morph_on_cpu);
    graphics::MESH.store(gfx.mesh, std::sync::atomic::Ordering::Relaxed);
    app.add_plugins(graphics::plugin).insert_resource(gfx);
    app.add_plugins((noise::plugin, mash::plugin, spring::plugin, chat::plugin, vertex_patch::plugin));
    // M91a1: the iPhone's on-screen pad, kept through mode switches
    app.add_plugins(play::touch_pad::plugin);
    // M101: Android's controllers, which gilrs can't read
    #[cfg(target_os = "android")]
    app.add_plugins(android_pad::plugin);
    app.add_systems(Update, (character::texture_faces.after(character::animate), character::uv_offsets));
    if std::env::var_os("HST_UI_CENSUS").is_some() {
        app.add_plugins(ui_census::plugin);
    }
    if std::env::var_os("HST_SCENE_CENSUS").is_some() {
        app.add_plugins(scene_census::plugin);
    }
    if std::env::var_os("HST_FPS").is_some() {
        app.add_systems(Last, (log_fps, log_assets)).add_plugins(stall::plugin);
    }
    if std::env::var_os("HST_KEYS").is_some() {
        app.add_systems(PreUpdate, scripted_keys.after(bevy::input::InputSystems));
    }
    if menu {
        // `hst net … --play`: this machine's players on the keyboard, then pads 0, 1
        let first = match play::netplay::PENDING.lock().unwrap().take() {
            Some((net, room, me)) => {
                use play::controls::Seat;
                let devices: Vec<_> = (0..4).map(|j| Some(if j == 0 { Seat::Keys } else { Seat::Pad(j - 1) })).collect();
                let flags = play::netplay::flags(&room, &me, &devices, &[]);
                play::netplay::mode(&args.iso, &flags, mode::Mode::new(|app| {
                    // no lobby screen to go back to from here (B279's hand-over): the connection ends
                    play::netplay::LOBBY.lock().unwrap().take();
                    menu_mode(app)
                }), net, room, me)
            }
            None => mode::Mode::new(menu_mode),
        };
        app.add_plugins((mode::plugin, inspect::render)).insert_resource(mode::Next(Some(first)));
    } else if args.play {
        // a mode too, so its quit (the pause menu's Return to the Menu, the result screen) lands in the main menu
        let menu_args = Args { play: false, ..args.clone() };
        let back = mode::Mode::new(move |app| {
            app.insert_resource(menu_args);
            menu_mode(app);
        });
        let a = args.clone();
        let (iso, early) = (args.iso.clone(), lineup(&args, &slot_mods, viewer_mod.is_some().then_some(mod_slot)));
        let first = mode::Mode::new(move |app| {
            app.insert_resource(a).insert_resource(swap_hands).insert_resource(names).add_systems(Startup, load);
            add_match(app, viewer_mod, mod_slot, slot_mods, umpire_mod);
            court_mod::add(app, court_mod);
        })
        .early(move |world| preload::characters(world, &iso, &early));
        app.add_plugins((mode::plugin, inspect::render)).insert_resource(mode::Next(Some(first.back(back))));
    } else if let Some(r) = roster {
        app.add_plugins(inspect::viewer).insert_resource(r);
    } else if args.viewer.is_some() {
        app.add_plugins(character::viewer).insert_resource(character::ViewerMod(viewer_mod));
    } else if args.ball {
        app.add_plugins(sandbox::plugin);
    }
    if !menu && !args.play {
        app.add_systems(Startup, load);
    }
    app.insert_resource(args).insert_resource(ClearColor(Color::srgb(0.25, 0.3, 0.35))).add_systems(Update, (orbit, auto_shot, clouds))
        .add_systems(PostUpdate, (fixed_eye, backdrops, hide_inside).chain().before(bevy::transform::TransformSystems::Propagate)).run();
}

/// P50: the players' disc characters and costumes (`play::setup`'s line-up), but a mod's slots.
fn lineup(args: &Args, slot_mods: &[(usize, String)], viewer_slot: Option<usize>) -> Vec<(usize, usize)> {
    let modded: Vec<usize> = slot_mods.iter().map(|m| m.0).chain(viewer_slot).collect();
    (0..if args.singles { 2 } else { 4 })
        .filter(|i| !modded.contains(i))
        .map(|i| (args.chars.get(i).copied().unwrap_or([6, 1, 2, 3][i]), args.outfits.get(i).copied().unwrap_or(0)))
        .collect()
}

/// The match: `play::plugin`; `--play --mod DIR [--mod-slot N]` or `--slot-mod N DIR` (any number): the mod plays
/// player N (default 1st), its costume by `--outfits`; `umpire_mod` is heard in the chair (M8f3).
fn add_match(app: &mut App, viewer_mod: Option<String>, mod_slot: usize, mut slot_mods: Vec<(usize, String)>, umpire_mod: Option<mods_umpire::Umpire>) {
    app.add_plugins(play::plugin);
    if let Some(u) = umpire_mod {
        app.insert_resource(mods_umpire::MatchUmpire(u));
    }
    slot_mods.extend(viewer_mod.map(|d| (mod_slot, d)));
    if !slot_mods.is_empty() {
        let mut mm = mods::MatchMod::default();
        for (n, dir) in slot_mods {
            mm.set(n, mods::read(dir.as_ref()).unwrap_or_else(|e| panic!("{e}")));
        }
        app.insert_resource(mm);
    }
}

/// The main menu as a mode (`mode.rs`).
pub fn menu_mode(app: &mut App) {
    app.add_systems(Startup, load).add_plugins(play::main_menu::plugin);
}

/// The menu's match in this app: `flags` read as on the command line (the settings' `--no-upscale` and
/// graphics flags take effect here); its quit comes back to `back`.
pub fn match_mode(iso: &str, flags: &[String], back: mode::Mode) -> mode::Mode {
    let Cli { mut args, gfx, upscale, viewer_mod, mod_slot, slot_mods, swap_hands, names, umpire_mod, court_mod, .. } = parse(std::iter::once(iso.to_string()).chain(flags.iter().cloned()));
    textures::set(iso, upscale);
    let cover = loading::cover(args.iso.clone(), args.court);
    let (iso, early) = (args.iso.clone(), lineup(&args, &slot_mods, viewer_mod.is_some().then_some(mod_slot)));
    graphics::MESH.store(gfx.mesh, std::sync::atomic::Ordering::Relaxed);
    mode::Mode::new(move |app| {
        app.insert_resource(gfx);
        // the app's own `--shot` goes on through the match
        let old = app.world().resource::<Args>();
        (args.shot, args.shot_at) = (old.shot.clone(), old.shot_at);
        app.insert_resource(args).insert_resource(swap_hands).insert_resource(names).add_systems(Startup, load);
        add_match(app, viewer_mod, mod_slot, slot_mods, umpire_mod);
        court_mod::add(app, court_mod);
    })
    .back(back)
    .cover(cover)
    .early(move |world| preload::characters(world, &iso, &early))
}

/// Court `n`'s entry list and hole-01 placement records.
fn court_layout(iso: &mut Iso, n: usize) -> Option<(Vec<layout::Entry>, Vec<layout::Placement>)> {
    let find = |data: &[u8], suffix: &str| {
        let arc = Archive::parse(data).ok()?;
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix))?;
        arc.read(e).ok()
    };
    let (cmn, hol) = (iso.read(&format!("COURT/{n:02}/CMN.XB")).ok()?, iso.read(&format!("COURT/{n:02}/HOL01.XB")).ok()?);
    let list = layout::entries(&String::from_utf8_lossy(&find(&cmn, &format!("entry_c{n:02}.txt"))?));
    Some((list, layout::plants(&find(&hol, &format!("plant_c{n:02}_h01_0.dat"))?).ok()?))
}

/// Parent for everything in game space: game space is Y-down, a half-turn about X maps it to Bevy's
/// Y-up without mirroring.
#[derive(Component)]
pub struct GameSpace;

/// How many clouds court `n` makes outside heavy rain (`layout::cloud_count`; singles' environment row 1, doubles' 0).
fn cloud_count(iso: &mut Iso, n: u32, envir: Option<&[u8]>, singles: bool) -> usize {
    let hole = iso.read(&format!("COURT/{n:02}/GRD01.XB")).ok().and_then(|d| {
        let arc = Archive::parse(&d).ok()?;
        arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("envir_c{n:02}_h01.dat")))?).ok()
    });
    envir.zip(hole).and_then(|(e, h)| layout::cloud_count(e, &h, singles as usize)).unwrap_or(20)
}

fn load(
    mut commands: Commands,
    args: Res<Args>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gs_materials: ResMut<Assets<gs::GsMaterial>>,
    (seed, mods): (Option<Res<handshake::MatchSeed>>, Option<Res<mods::MatchMod>>),
    court_mod: Option<Res<mods_court::MatchCourt>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let root = commands
        .spawn((GameSpace, Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::PI)), Visibility::default()))
        .id();
    // M8f6: under a court mod the layout court is set up as ever (its sim, weather and generators) but not drawn
    let root = court_mod.map_or(root, |_| commands.spawn((court_mod::DiscCourt, Transform::default(), Visibility::Hidden, ChildOf(root))).id());
    let mut bounds = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    // HST_ONLY / HST_SKIP: substring filters on model paths, for bisecting what draws what
    let only = std::env::var("HST_ONLY").ok();
    let not = std::env::var("HST_SKIP").ok();
    let skip = move |n: &str| {
        n.contains("debug")
            || n.contains("holeend")
            || only.as_deref().is_some_and(|o| !n.contains(o))
            || not.as_deref().is_some_and(|o| n.contains(o))
    };
    let spawn = |commands: &mut Commands, parts: &[(Handle<Mesh>, Handle<gs::GsMaterial>)], t: Transform| {
        let e = commands.spawn((t, Visibility::default())).id();
        for (m, mat) in parts {
            commands.entity(e).with_child((Mesh3d(m.clone()), MeshMaterial3d(mat.clone())));
        }
        commands.entity(root).add_child(e);
        e
    };
    let add = |parts: Vec<(Mesh, Vec<gs::GsMaterial>)>, meshes: &mut Assets<Mesh>, materials: &mut Assets<gs::GsMaterial>| {
        let mut out = Vec::new();
        for (m, mats) in parts {
            let m = meshes.add(m);
            out.extend(mats.into_iter().map(|mat| (m.clone(), materials.add(mat))));
        }
        out
    };
    if let Some(n) = args.stage {
        let dir = format!("COURT/{n:02}");
        let mut library = std::collections::HashMap::new();
        let mut anims = std::collections::HashMap::new();
        // the game's season is 1 in singles (hole archive SSN1/HOL01.XB, which differs by the net), 0 in doubles
        let season = args.singles as usize;
        let mut sky_colour = std::collections::HashMap::new();
        let (mut hole_box, mut raw_models) = (std::collections::HashMap::new(), std::collections::HashMap::new());
        let mut model_shapes = std::collections::HashMap::new();
        for name in ["CMN.XB", "GRD01.XB", if args.singles { "SSN1/HOL01.XB" } else { "HOL01.XB" }] {
            let data = iso.read(&format!("{dir}/{name}")).expect("court archive on disc");
            for_models(&data, |n| n.contains("_sky"), |stem, model, mats| {
                sky_colour.insert(stem, sky_top(&model, &mats));
            });
            // the sun-shade map: the hole model's box frames it, the casters' triangles shade it
            for_models(&data, |n| !skip(n), |stem, model, mats| {
                hole_box.insert(stem.clone(), shade::packet_box(&model));
                model_shapes.insert(stem.clone(), hst_sim::shade::Shape::new(&model, &mats));
                raw_models.insert(stem, (model, mats));
            });
            for (stem, parts, anim) in gs_models_anim(&data, |n| !skip(n), &mut images) {
                let tags: Vec<_> = parts.iter().flat_map(|(_, g, t)| std::iter::repeat_n(*t, g.len())).collect();
                let parts = add(parts.into_iter().map(|(m, g, _)| (m, g)).collect(), &mut meshes, &mut gs_materials);
                anims.insert(stem.clone(), (anim, tags));
                library.insert(stem, parts);
            }
        }
        // the season's fog on every court material (FGE batches use it)
        let envir = iso.read(&format!("{dir}/CMN.XB")).ok().and_then(|d| {
            let arc = Archive::parse(&d).ok()?;
            arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("envir_c{n:02}.dat")))?).ok()
        });
        // the fogs, light, clear colour and ground shadow follow the weather (`weather::apply`)
        let sun = shadow::Sun::read(&mut iso, n as usize);
        let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
        let exe = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
        let today = {
            let o = exe.weather_odds(n);
            let odds = hst_sim::weather::Odds { cloudy: o[0], cloudy_len: [o[1], o[2]], rain: o[3], rain_len: [o[4], o[5]], no_border: o[7] != 0, heavy: o[8] != 0 };
            let ((directions, speed), (chance, kind)) = (exe.wind(n), exe.gusts(n));
            let wind = hst_sim::weather::Wind { chance, kind, directions, speed };
            // a match: `--games`, `--sets` (`play` keeps `Weather::game` on the games played)
            let players = if !args.play { 1 } else if args.singles { 2 } else { 4 };
            // the game seeds its MT19937 with a `rand()` output when it sets the match up; `HST_WEATHER_SEED` gives it
            // directly (slot 5's was 0x28c7c4a1)
            // the menus' `rand()` calls before the match are the confirmation screen's flame (`rng::MenuFlame`), drawing
            // every frame it's up from P1's pick (`main_menu::flame`); a match started without the menus takes a
            // clock-picked count of draws instead
            // online, the host's seed (`handshake::MatchSeed`, or `hst net`'s room seed) is taken as is
            let seed = seed.map(|s| s.0).unwrap_or_else(|| {
                std::env::var("HST_WEATHER_SEED").ok().and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok()).or_else(play::netplay::seed).map_or_else(
                    || {
                        play::main_menu::flame::rand().map_or_else(
                            || hst_sim::rng::Seed::after(web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.subsec_micros() % 65536)),
                            |mut rand| hst_sim::rng::Seed { shared: rand.next(), rand },
                        )
                    },
                    hst_sim::rng::Seed::shared,
                )
            });
            commands.insert_resource(handshake::Handshake::of(&args, iso.pack_hash, mods.as_deref(), seed));
            let r = seed.rand;
            let mut mt = hst_sim::weather::Mt::new(seed.shared);
            let schedule = hst_sim::weather::schedule(&odds, &wind, args.games, args.sets, players, || mt.next());
            // the match goes on drawing from the same generators (with a given seed, `rand()` is taken as at boot)
            let mut rngs = hst_sim::rng::Rngs::new(r, mt);
            // the setup's own `rand()` calls: the lens flare, the sound manager, the clouds and the effects seed. The
            // clouds are made only with in-season cloud models; at the setup in singles only
            let fixed = std::env::var("HST_WEATHER").ok().and_then(|w| w.parse().ok());
            let first = fixed.unwrap_or(schedule[0].weather);
            let models = court_layout(&mut iso, n as usize).is_some_and(|(l, _)| l.iter().any(|e| e.dir == "cloud" && layout::in_season(&e.stem, season)));
            let sky = Sky { weathers: schedule.map(|g| g.weather), fixed, made: first, count: cloud_count(&mut iso, n, envir.as_deref(), args.singles), rain: exe.rain_clouds().0, draws: None };
            let (effects, draws) = rngs.setup_rand(if models && args.singles { sky.made_count(first) } else { 0 });
            play::set_effects_seed(effects);
            commands.insert_resource(play::MatchRng(rngs));
            *SKY.lock().unwrap() = models.then_some(Sky { draws: Some((first, draws)), ..sky });
            let w = weather::Weather { schedule, game: 0, fixed };
            let today = w.today();
            commands.insert_resource(w);
            today
        };
        let mut look = weather::CourtLook {
            envir: envir.clone().unwrap_or_default(),
            sky: None,
            looks: exe.weather_looks(),
            players: exe.player_light(n),
            bg: Default::default(),
            skies: Default::default(),
            holes: Default::default(),
            decals: Default::default(),
            acc: Vec::new(),
            clo: Vec::new(),
            last: None,
            mix: None,
        };
        let (list, plants) = court_layout(&mut iso, n as usize).expect("court layout");
        // the court load shifts the hole model's vertex colours in HSL by the court's and the hole's envir
        let hole_envir = iso.read(&format!("{dir}/GRD01.XB")).ok().and_then(|d| {
            let arc = Archive::parse(&d).ok()?;
            arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("envir_c{n:02}_h01.dat")))?).ok()
        });
        let hole_hsl = envir.as_deref().zip(hole_envir.as_deref()).and_then(|(c, h)| gs::hole_hsl(c, h));
        // the ground stands at the origin, sky/clo/acc/bg follow the camera (`backdrops`); props are placed from the
        // plant records
        // the entry list names every hole variant; this layout is hole 01
        let this_hole = |e: &&layout::Entry| e.dir != "hole" || e.stem.contains("_h01");
        // the hole's ground model is the one shadow receiver
        // the game loads every in-season sky but draws only the first; its colour also clears the screen
        let mut sky = None;
        let (mut shade_frame, mut shade_casters) = (None, Vec::new());
        for e in list.iter().filter(|e| matches!(e.dir.as_str(), "hole" | "bg")).filter(this_hole).filter(|e| layout::in_season(&e.stem, season)) {
            if e.stem.contains("_sky") {
                if sky.is_some() {
                    continue;
                }
                sky = sky_colour.get(&e.stem).copied();
            }
            // the sun-shade map frames the hole model (court 7 has none)
            if let Some(&(lo, hi)) = hole_box.get(&e.stem).filter(|_| e.dir == "hole" && n != 7) {
                shade_frame = Some((hst_sim::shade::Frame::new(lo, hi, 1.0, [0.0; 3]), e.stem.clone()));
            }
            if let Some(parts) = library.get(&e.stem) {
                if let Some(hsl) = hole_hsl.filter(|_| e.dir == "hole") {
                    shift_colours(parts, &mut meshes, hsl);
                }
                let id = spawn(&mut commands, parts, Transform::default());
                if let Some(i) = (e.dir == "bg").then(|| ["_sky", "_clo", "_acc", "_bg"].iter().position(|k| e.stem.contains(k))).flatten() {
                    let (yaw, lift) = gs::backdrop(envir.as_deref().unwrap_or_default(), hole_envir.as_deref(), i);
                    commands.entity(id).insert(Backdrop { yaw, lift });
                    // B75: they stand at the eye (+ lift), so by mesh centre their blended draws could sort after the
                    // effects and billboards, which write no Z, and cover them; the game draws them first, at the far
                    // end of Z (GS dump, court 10). A constant keeps their order among themselves (by lift).
                    for (_, m) in parts {
                        if let Some(mut m) = gs_materials.get_mut(m) {
                            m.order = BACKDROP;
                        }
                    }
                }
                let set = if e.dir == "hole" { &mut look.holes } else if e.stem.contains("_sky") { &mut look.skies } else { &mut look.bg };
                set.extend(parts.iter().map(|(_, m)| m.id()));
                if e.stem.contains("_acc") {
                    look.acc.push(id);
                } else if e.stem.contains("_clo") {
                    look.clo.push(id);
                }
                // the hole model's own .UVA/.MTA play from the start (water, waterfalls)
                if let Some((anim, tags)) = anims.remove(&e.stem).filter(|(a, _)| e.dir == "hole" && !a.is_empty()) {
                    let parts = parts.iter().zip(tags).map(|((_, m), t)| (m.clone(), t)).collect();
                    commands.spawn(court_anim::Playing { anim, parts });
                }
            }
        }
        look.sky = sky;
        if let Some(sun) = sun {
            commands.insert_resource(sun);
        }
        for p in &plants {
            let Some(e) = layout::resolve(&list, p, season) else { continue };
            if !matches!(e.dir.as_str(), "tree" | "prop" | "structure" | "billboard") {
                continue; // creatures/gallery are animated NPCs, dmy are markers
            }
            let Some(parts) = library.get(&e.stem) else { continue };
            let stem = &e.stem;
            let s = if p.scale > 0.0 { p.scale } else { 1.0 };
            // the world matrix the game draws it with: yaw, plus the code's tilt and extra turn (`world::place`)
            let (m, post) = hst_sim::world::place(p.category, p.pos, p.yaw, p.code);
            let t = if post {
                Transform::from_translation(Vec3::from(p.pos)).with_rotation(Quat::from_rotation_y(p.yaw))
            } else {
                Transform::from_matrix(Mat4::from_cols_array_2d(&m))
            }
            .with_scale(Vec3::splat(s));
            let shape = model_shapes.get(&e.stem).cloned().unwrap_or_default();
            let near = matches!(e.dir.as_str(), "tree" | "prop");
            let cast = p.code[3] != b'0' && (17..=19).contains(&p.category);
            let e = spawn(&mut commands, parts, t);
            // B185: the Grand Palais end houses: the match camera behind your baseline sits inside one
            if stem.starts_with("w-house") {
                let b = parts.iter().filter_map(|(m, _)| bevy::camera::primitives::MeshAabb::compute_aabb(meshes.get(m)?)).fold((Vec3::MAX, Vec3::MIN), |(lo, hi), a| (lo.min(a.min().into()), hi.max(a.max().into())));
                commands.entity(e).insert(HideInside { lo: b.0, hi: b.1, at: t.compute_affine().inverse() });
            }
            // B303: trees and props that come right up in front of the lens are hidden (`net_hide::in_front`)
            if near && !play::net_hide::picked(p.category, p.pos) {
                let corners = parts.iter().filter_map(|(m, _)| bevy::camera::primitives::MeshAabb::compute_aabb(meshes.get(m)?)).flat_map(|a| (0..8).map(move |i| Vec3::from(a.center) + Vec3::from(a.half_extents) * Vec3::new([-1.0, 1.0][i & 1], [-1.0, 1.0][i >> 1 & 1], [-1.0, 1.0][i >> 2])));
                let (lo, hi) = corners.map(|c| t.transform_point(c)).fold((Vec3::MAX, Vec3::MIN), |(lo, hi), c| (lo.min(c), hi.max(c)));
                if lo.cmple(hi).all() {
                    commands.entity(e).insert(play::net_hide::NearBox { lo, hi });
                }
            }
            if play::net_hide::picked(p.category, p.pos) {
                commands.entity(e).insert(play::net_hide::HideNearCamera);
            } else if play::net_hide::tested(p.category, p.code) {
                commands.entity(e).insert(play::net_hide::HideAroundEye { pos: p.pos, prop: None });
            }
            if cast {
                commands.entity(e).insert(shadow::Caster);
                // B351: one whose shadow can't reach the court drops it at Shadows Low
                let (lo, hi) = parts.iter().filter_map(|(m, _)| bevy::camera::primitives::MeshAabb::compute_aabb(meshes.get(m)?)).flat_map(|a| (0..8).map(move |i| Vec3::from(a.center) + Vec3::from(a.half_extents) * Vec3::new([-1.0, 1.0][i & 1], [-1.0, 1.0][i >> 1 & 1], [-1.0, 1.0][i >> 2]))).map(|c| t.transform_point(c)).fold((Vec3::MAX, Vec3::MIN), |(lo, hi), c| (lo.min(c), hi.max(c)));
                if !sun.as_ref().is_some_and(|sun| lo.cmple(hi).all() && shadow::reaches_court(lo, hi, sun.dir)) {
                    commands.entity(e).insert(shadow::Minor);
                }
                if let Some(sun) = &sun {
                    shade_casters.push(hst_sim::shade::Caster::new(&shape, p, &sun.axes));
                }
            }
            // ponytail: the net's shadow is too thin for the sun's shadow map, so its triangles are flattened along
            // the sun onto the ground (game y 0) and drawn black at the shadow's strength through the texture's
            // alpha (`weather::apply`); not the game's shadow-texture pass (P17v7, dropped)
            if let Some(sun) = sun.filter(|_| stem.starts_with("znet")) {
                let mut done = std::collections::HashSet::new();
                let decal: Vec<_> = parts.iter().filter(|(m, _)| done.insert(m.id())).filter_map(|(m, g)| {
                    let mut mesh = meshes.get(m)?.clone();
                    let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else { return None };
                    for v in pos {
                        let w = t.transform_point(Vec3::from(*v));
                        *v = (w - sun.dir * (w.y / sun.dir.y) - Vec3::Y * 0.01).to_array();
                    }
                    let mut g = gs_materials.get(g)?.clone();
                    g.key = gs::GsKey { modulate: true, test: gs::Test::Never, blend: Some(hst_data::mtl::Blend::Normal), fog: false, cull: false, ..g.key };
                    (g.uniform.color, g.uniform.highlight, g.uniform.shadow) = (Vec4::ZERO, 0.0, 0.0);
                    let g = gs_materials.add(g);
                    look.decals.insert(g.id());
                    Some((meshes.add(mesh), g))
                }).collect();
                spawn(&mut commands, &decal, Transform::default());
            }
        }
        look.sky = sky;
        if envir.is_some() {
            commands.insert_resource(look);
        }
        if let (Some((frame, hole)), Some(_)) = (shade_frame, sun) {
            if let Some((model, mats)) = raw_models.get(&hole) {
                commands.insert_resource(shade::build(frame, &shade_casters, model, mats, hst_sim::court::world(&mut iso, n)));
            }
        }
        // clouds: singles only (the game makes them in doubles too but neither moves nor draws them)
        if args.singles {
            let models: Vec<_> = list.iter().filter(|e| e.dir == "cloud" && layout::in_season(&e.stem, season)).filter_map(|e| library.get(&e.stem)).collect();
            if !models.is_empty() {
                let models = models.into_iter().cloned().collect();
                // laid out from the setup's draws (`clouds`)
                commands.insert_resource(Clouds { list: Vec::new(), degrees: today.degrees, speed: today.speed, ticks: 0.0, models, root, rain: exe.rain_clouds().1 });
            }
        }
        // the background figures are `play::npcs`'s
        bounds = (Vec3::splat(-20.0), Vec3::splat(20.0));
    }
    for path in &args.archives {
        let data = iso.read(path).expect("archive on disc");
        for (_, parts) in gs_models(&data, |n| !skip(n), &mut images) {
            for (mesh, _) in &parts {
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    for v in p {
                        let p = Vec3::new(v[0], -v[1], -v[2]);
                        bounds = (bounds.0.min(p), bounds.1.max(p));
                    }
                }
            }
            let parts = add(parts, &mut meshes, &mut gs_materials);
            spawn(&mut commands, &parts, Transform::default());
        }
    }
    if args.ball && !args.play {
        let data = iso.read("CMN/GAME.XB").expect("ball archive on disc");
        let parts: Vec<_> = models(&data, |n| n.ends_with("ball1.mdl"), &mut images)
            .into_iter()
            .flat_map(|(_, p)| p)
            .map(|(mesh, material)| (Mesh3d(meshes.add(mesh)), MeshMaterial3d(materials.add(material))))
            .collect();
        let ball = commands.spawn((sandbox::BallView, Transform::default(), Visibility::default())).id();
        for part in parts {
            commands.entity(ball).with_child(part);
        }
        commands.entity(root).add_child(ball);
    }
    let (focus, radius) = if bounds.0.x <= bounds.1.x {
        ((bounds.0 + bounds.1) / 2.0, (bounds.1 - bounds.0).length().max(1.0))
    } else {
        (Vec3::ZERO, 10.0)
    };
    info!("bounds {bounds:?}");
    let (focus, radius) = args.radius.map_or((focus, radius), |r| (Vec3::ZERO, r));
    commands.spawn((
        Camera3d::default(),
        // the PS2 writes its colours as they are
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Projection::Perspective(PerspectiveProjection { far: 20_000.0, ..default() }),
        Transform::default(),
        // play mode: broadcast view from behind the near (your) baseline
        if args.play { Orbit { focus: Vec3::new(0.0, 0.0, 2.0), radius: 24.0, yaw: std::f32::consts::PI, pitch: -0.38 } } else { Orbit { focus, radius, yaw: std::env::var("HST_VIEW_YAW").ok().and_then(|y| y.parse().ok()).unwrap_or(0.6), pitch: -0.4 } },
    ));
}

/// Every model in an archive whose lower-cased name ends in `.mdl` and passes `keep`, keyed by its
/// lower-case file stem, as one Bevy mesh + material per original material.
fn models(data: &[u8], keep: impl Fn(&str) -> bool, images: &mut Assets<Image>) -> Vec<(String, Vec<(Mesh, StandardMaterial)>)> {
    let mut out = Vec::new();
    for_models(data, keep, |model_stem, model, mats| {
        let _pre = textures::prefetch(&mats);
        let tex: Vec<Handle<Image>> = mats.textures.iter().map(|t| textures::add_mtl(images, image(t), t)).collect();
        let mut parts = Vec::new();
        {
            for (mi, packets) in model.materials.iter().enumerate() {
                let Some(mesh) = mesh(packets.iter(), &model) else { continue };
                let mat = mats.materials.get(mi);
                let texture = mat.and_then(|m| m.texture).map(|t| tex[t].clone());
                if let (Some(m), Some(t), Some(h)) = (mat, mat.and_then(|m| m.texture), &texture) {
                    let mut raw = image(&mats.textures[t]);
                    raw.texture_descriptor.format = TextureFormat::Rgba8Unorm;
                    let mut draws = gs::GsMaterial::for_batch(m, packets.first().map_or(0x10, |p| p.prim), Some(textures::add_mtl(images, raw, &mats.textures[t])));
                    draws.iter_mut().for_each(|g| g.uniform.lod_k = model.lod_k.get(mi).copied().unwrap_or(0.0));
                    shade::BALL_GS.lock().unwrap().insert(h.id(), draws);
                }
                let [r, g, b, a] = mat.map_or([1.0; 4], |m| m.color);
                parts.push((
                    mesh,
                    StandardMaterial {
                        base_color: Color::linear_rgba(r, g, b, a),
                        base_color_texture: texture,
                        unlit: true,
                        double_sided: true,
                        cull_mode: None,
                        alpha_mode: AlphaMode::Mask(0.5),
                        ..default()
                    },
                ));
            }
        }
        out.push((model_stem, parts));
    });
    out
}

/// Every model in an archive whose lower-cased name ends in `.mdl` and passes `keep`, with its lower-case file
/// stem and parsed MTL.
fn for_models(data: &[u8], keep: impl Fn(&str) -> bool, mut f: impl FnMut(String, mdl::Model, mtl::Mtl)) {
    let arc = Archive::parse(data).expect("xb archive");
    for e in arc.entries.iter().filter(|e| { let n = e.name.to_ascii_lowercase(); n.ends_with(".mdl") && keep(&n) }) {
        let stem = &e.name[..e.name.len() - 4];
        let sib = |ext: &str| arc.find(&format!("{stem}.{ext}")).and_then(|x| arc.read(x).ok());
        let (Ok(model), Some(mtl_bytes)) = (mdl::parse(&arc.read(e).unwrap()), sib("MTL")) else {
            warn!("skipping {}", e.name);
            continue;
        };
        let mats = match mtl::parse(&mtl_bytes, sib("MTI").as_deref()) {
            Ok(m) => m,
            Err(err) => { warn!("{}: {err}", e.name); continue }
        };
        f(stem.rsplit(['\\', '/']).next().unwrap_or(stem).to_ascii_lowercase(), model, mats);
    }
}

/// A sky's colour where the game reads it: the top-most vertex (smallest game y, the last of equals)
/// colour × its material colour / 128, in 0..255.
fn sky_top(model: &mdl::Model, mats: &mtl::Mtl) -> [f32; 3] {
    let mut top = (f32::MAX, [0.0; 3]);
    for (mi, pk) in model.materials.iter().enumerate() {
        let m = mats.materials.get(mi).map_or([1.0; 4], |m| m.color);
        for v in pk.iter().flat_map(|p| &p.vertices) {
            if v.pos[1] <= top.0 {
                top = (v.pos[1], std::array::from_fn(|i| v.color[i] as f32 * m[i]));
            }
        }
    }
    top.1
}

/// One triangle-list mesh of these packets; vertex colours stay in PS2 units (0x80 = 1.0).
/// B211: packets are stored in their nodes' spaces and drawn at the model's bind pose: a packet bound to one node by
/// its bind matrix, a weighted one skinned as [`mdl::Model::skinned`] (Queen's ball machine's launcher wheels and
/// balls sat under the grass at the origin; court 6's horses were torn apart).
fn mesh<'a>(packets: impl Iterator<Item = &'a mdl::Packet>, model: &mdl::Model) -> Option<Mesh> {
    let (mut pos, mut nrm, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![], vec![]);
    for pk in packets {
        let base = pos.len() as u32;
        if pk.palette.len() > 1 {
            let (verts, tris, _) = model.skin_packet(pk);
            for v in &verts {
                pos.push(v.pos);
                nrm.push((Vec3::from(v.normals[0]) + Vec3::from(v.normals[1])).to_array());
                uv.push(v.uv);
                col.push(v.color.map(|c| c as f32 / 128.0));
            }
            idx.extend(tris.iter().flatten().map(|i| i + base));
            continue;
        }
        let m = pk.palette.first().and_then(|&n| model.node_bind.get(n)).map_or(Mat4::IDENTITY, Mat4::from_cols_array_2d);
        for v in &pk.vertices {
            pos.push(m.transform_point3(Vec3::from(v.pos)).to_array());
            nrm.push(m.transform_vector3(Vec3::from(v.normal)).to_array());
            uv.push(v.uv);
            col.push(v.color.map(|c| c as f32 / 128.0)); // PS2 modulate: 0x80 = 1.0
        }
        idx.extend(pk.triangles.iter().flatten().map(|i| i + base));
    }
    if idx.is_empty() {
        return None;
    }
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx));
    Some(mesh)
}

/// Like [`models`], drawn as the GS draws them ([`gs`]): one mesh per material and batch PRIM, its texture raw and
/// wrapped per the MDL's material header.
fn gs_models(data: &[u8], keep: impl Fn(&str) -> bool, images: &mut Assets<Image>) -> Vec<(String, Vec<(Mesh, Vec<gs::GsMaterial>)>)> {
    gs_models_anim(data, keep, images).into_iter().map(|(stem, parts, _)| (stem, parts.into_iter().map(|(m, g, _)| (m, g)).collect())).collect()
}

/// [`gs_models`] with each model's `.UVA`/`.MTA` ([`court_anim`]): meshes are further split per UV track, and each
/// part carries the (material, packet) its animation is read from.
fn gs_models_anim(
    data: &[u8],
    keep: impl Fn(&str) -> bool,
    images: &mut Assets<Image>,
) -> Vec<(String, Vec<(Mesh, Vec<gs::GsMaterial>, (usize, usize))>, hst_sim::court_anim::CourtAnim)> {
    let arc = Archive::parse(data).expect("xb archive");
    let tracks = |stem: &str, ext: &str, width| {
        let suffix = format!("{stem}.{ext}");
        let e = arc.entries.iter().find(|e| e.name.rsplit(['\\', '/']).next().is_some_and(|n| n.eq_ignore_ascii_case(&suffix)))?;
        hst_data::mor::parse(&arc.read(e).ok()?, width).ok()
    };
    let mut out = Vec::new();
    for_models(data, keep, |stem, model, mats| {
        let _pre = textures::prefetch(&mats);
        let anim = hst_sim::court_anim::CourtAnim::new(&model, tracks(&stem, "uva", 4).as_ref(), tracks(&stem, "mta", 1).as_ref(), &mats.materials);
        // P32: a hole material whose alpha the `.MTA` animates, or whose texture a live reload can change, keeps its
        // blended draws whole
        let fixed_alpha = tracks(&stem, "mta", 1).is_none() && std::env::var_os("HST_TEXTURE_RELOAD").is_none();
        let mut parts = Vec::new();
        for (mi, packets) in model.materials.iter().enumerate() {
            let Some(mat) = mats.materials.get(mi) else { continue };
            let wrap = model.wrap.get(mi).copied().unwrap_or([0, 0]);
            // B161: see-through texels blended rather than cut (the hole, TEST 20..29) and no GS levels: our own
            let padded = mat.texture.and_then(|t| pow2(&mats.textures[t]));
            let tex = |t: usize| padded.as_ref().unwrap_or(&mats.textures[t]);
            let own_mips = (stem.contains("_h01_") || (20..=29).contains(&i16::from_le_bytes([mat.header[0x1e], mat.header[0x1f]])))
                .then(|| mat.texture.and_then(|t| cutout_mips(tex(t))))
                .flatten();
            let texture = mat.texture.map(|i| {
                let mut img = image(tex(i));
                img.texture_descriptor.format = TextureFormat::Rgba8Unorm;
                // the mip levels TEX1 MXL lets the GS use; gs.wgsl picks one from the view depth
                let t = tex(i);
                if !t.mips.is_empty() && !t.rgba.is_empty() {
                    img.texture_descriptor.mip_level_count = 1 + t.mips.len() as u32;
                    img.data.as_mut().unwrap().extend(t.mips.iter().flatten());
                } else if let Some(levels) = &own_mips {
                    img.texture_descriptor.mip_level_count = 1 + levels.len() as u32;
                    img.data.as_mut().unwrap().extend(levels.iter().flatten());
                }
                // GS CLAMP: 0 repeat; 1 clamp; 2 region clamp over the whole texture (as the game sets it) = clamp
                let mode = |w: u8| if w == 0 { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
                img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: mode(wrap[0]),
                    address_mode_v: mode(wrap[1]),
                    ..ImageSamplerDescriptor::linear()
                });
                textures::add_mtl(images, img, &mats.textures[i])
            });
            let key = |pi: usize| (packets[pi].prim & 0x70, anim.uv_track(mi, pi), packets[pi].uv_swap, packets[pi].flags & 2);
            let mut keys: Vec<_> = (0..packets.len()).map(|pi| (key(pi), pi)).collect();
            keys.sort();
            keys.dedup_by_key(|k| k.0);
            for (k, first) in keys {
                let Some(mesh) = mesh((0..packets.len()).filter(|&pi| key(pi) == k).map(|pi| &packets[pi]), &model) else { continue };
                // the court's hole model (`hole/<court>_h01_s….mdl`) is the one drawn faded
                let mut draws = gs::GsMaterial::for_court_batch(mat, k.0, texture.clone(), stem.contains("_h01_"));
                // ponytail: 1 km per material outweighs any view-depth gap on a court
                if stem.contains("_h01_") { draws.iter_mut().for_each(|g| g.order = -1.0e6 + mi as f32 * 1.0e3); }
                draws.iter_mut().for_each(|g| (g.uniform.lod_k, g.uniform.unlit) = (model.lod_k.get(mi).copied().unwrap_or(0.0), (k.3 != 0) as u8 as f32));
                if own_mips.is_some() {
                    draws.iter_mut().for_each(|g| g.uniform.screen_lod = 1.0);
                }
                // P32: each of the hole's two draws keeps only the triangles with a pixel it can draw (the indoor
                // court's floor is faded, its walls solid); a scrolling texture (`.UVA`) keeps them all
                let mut split = Vec::new();
                if stem.contains("_h01_") && fixed_alpha && k.1.is_none() {
                    let see = texture.as_ref().and_then(|t| images.get(t)).and_then(gs::SeeThrough::new);
                    if let (Some(Indices::U32(idx)), true) = (mesh.indices(), texture.is_none() || see.is_some()) {
                        let alpha: Vec<f32> = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
                            Some(bevy::mesh::VertexAttributeValues::Float32x4(c)) => c.iter().map(|c| c[3]).collect(),
                            _ => Vec::new(),
                        };
                        let uv: &[[f32; 2]] = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
                            Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) => uv,
                            _ => &[],
                        };
                        let can = |test| gs::hole_triangles(test, idx, &alpha, uv, mat.color[3], see.as_ref());
                        let (ge, lt) = (can(gs::Test::Ge80), can(gs::Test::Lt80));
                        // where every pixel passes A ≥ 0x80 the test is left out (an opaque draw without discard:
                        // tile GPUs' hidden-surface removal works on it)
                        let mut whole = Vec::new();
                        for g in std::mem::take(&mut draws) {
                            let pieces: Vec<(gs::GsMaterial, Vec<bool>)> = match g.key.test {
                                gs::Test::Lt80 => vec![(g, lt.clone())],
                                gs::Test::Ge80 => {
                                    let always = gs::GsMaterial { key: gs::GsKey { test: gs::Test::Always, ..g.key }, ..g.clone() };
                                    vec![(always, lt.iter().map(|&l| !l).collect()), (g, ge.iter().zip(&lt).map(|(&g, &l)| g && l).collect())]
                                }
                                _ => vec![(g, vec![true; idx.len() / 3])],
                            };
                            for (g, keep) in pieces {
                                if keep.iter().all(|&k| k) {
                                    whole.push(g);
                                } else if keep.iter().any(|&k| k) {
                                    let kept: Vec<u32> = idx.chunks(3).zip(&keep).filter(|(_, k)| **k).flat_map(|(t, _)| t.iter().copied()).collect();
                                    let mut part = mesh.clone();
                                    part.insert_indices(Indices::U32(kept));
                                    split.push((part, vec![g], (mi, first)));
                                }
                            }
                        }
                        draws = whole;
                    }
                }
                if !draws.is_empty() {
                    parts.push((mesh, draws, (mi, first)));
                }
                parts.extend(split);
            }
        }
        out.push((stem, parts, anim));
    });
    out
}

/// [`gs::hsl_shift`] on these parts' vertex colours (PS2 units, 0x80 = 1.0), each mesh once.
fn shift_colours(parts: &[(Handle<Mesh>, Handle<gs::GsMaterial>)], meshes: &mut Assets<Mesh>, hsl: [i32; 3]) {
    let mut done = std::collections::HashSet::new();
    for (m, _) in parts.iter().filter(|(m, _)| done.insert(m.id())) {
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(col)) = meshes.get_mut(m).and_then(|m| m.into_inner().attribute_mut(Mesh::ATTRIBUTE_COLOR)) else { continue };
        for c in col {
            let rgb = gs::hsl_shift(std::array::from_fn(|i| (c[i] * 128.0) as u8), hsl);
            c[..3].copy_from_slice(&rgb.map(|x| x as f32 / 128.0));
        }
    }
}

/// B161: a see-through texture the GS samples at its top level only (no MXL levels), such as Academy Grounds'
/// chain-link fence (1-texel wires): its own mip chain, colour weighted by alpha, so wires thinner than a pixel fade
/// out evenly instead of popping in and out as the camera moves (gs.wgsl `screen_lod`). `None` for opaque textures
/// and ones that have levels.
/// ponytail: only for draws that blend their see-through texels (an averaged alpha just reads as fainter); a pure
/// cut-out (TEST 10..19, the trees' leaves) would thin out, so those keep the GS's single level
fn cutout_mips(t: &mtl::Texture) -> Option<Vec<Vec<u8>>> {
    if !t.mips.is_empty() || t.rgba.is_empty() || t.rgba.chunks(4).all(|c| c[3] == 255) {
        return None;
    }
    let (mut w, mut h, mut prev) = (t.width as usize, t.height as usize, t.rgba.clone());
    let mut out = Vec::new();
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                let mut acc = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let p = &prev[(((2 * y + dy).min(h - 1)) * w + (2 * x + dx).min(w - 1)) * 4..][..4];
                    (0..3).for_each(|i| acc[i] += p[i] as u32 * p[3] as u32);
                    acc[3] += p[3] as u32;
                }
                let o = &mut next[(y * nw + x) * 4..][..4];
                (0..3).for_each(|i| o[i] = if acc[3] == 0 { 0 } else { ((acc[i] + acc[3] / 2) / acc[3]) as u8 });
                o[3] = ((acc[3] + 2) / 4) as u8;
            }
        }
        out.push(next.clone());
        (w, h, prev) = (nw, nh, next);
    }
    Some(out)
}

/// The texture padded to the power-of-two size the GS samples it over (TEX0 TW/TH) by repeating its last column and
/// row, or `None` if it already is one. B287: court 8's midground hedge is 256×320 with UVs over 512 rows; stretched
/// over 320 its cut-out tree tops fell off the top of the strip, leaving a flat-topped wall against the sky (court 3's
/// backdrop, 192 of 256 rows, too).
/// ponytail: the GS reads whatever VRAM holds past the stored rows; the UVs seen stay inside them, so edge texels do
fn pow2(t: &mtl::Texture) -> Option<mtl::Texture> {
    let (w, h) = (t.width.max(1).next_power_of_two(), t.height.max(1).next_power_of_two());
    if (w, h) == (t.width, t.height) || t.rgba.is_empty() {
        return None;
    }
    let pad = |px: &[u8], (sw, sh): (u32, u32), (dw, dh): (u32, u32)| {
        let mut out = Vec::with_capacity((4 * dw * dh) as usize);
        for y in 0..dh {
            for x in 0..dw {
                out.extend_from_slice(&px[4 * (y.min(sh - 1) * sw + x.min(sw - 1)) as usize..][..4]);
            }
        }
        out
    };
    let level = |l: usize| ((t.width >> l).max(1), (t.height >> l).max(1));
    let to = |l: usize| ((w >> l).max(1), (h >> l).max(1));
    Some(mtl::Texture {
        width: w,
        height: h,
        rgba: pad(&t.rgba, level(0), to(0)),
        mips: t.mips.iter().enumerate().map(|(l, m)| pad(m, level(l + 1), to(l + 1))).collect(),
        psm: t.psm,
        levels: Vec::new(),
        clut: Vec::new(),
    })
}

fn image(t: &mtl::Texture) -> Image {
    let mut img = Image::new(
        Extent3d { width: t.width.max(1), height: t.height.max(1), depth_or_array_layers: 1 },
        TextureDimension::D2,
        if t.rgba.is_empty() { vec![255; 4 * (t.width.max(1) * t.height.max(1)) as usize] } else { t.rgba.clone() },
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // PS2 UVs tile freely; Bevy's default sampler clamps
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    img
}

fn orbit(
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut q: Query<(&mut Orbit, &mut Transform)>,
) {
    for (mut o, mut t) in &mut q {
        if buttons.pressed(MouseButton::Left) {
            o.yaw -= motion.delta.x * 0.005;
            o.pitch = (o.pitch - motion.delta.y * 0.005).clamp(-1.5, 1.5);
        }
        o.radius *= 1.0 - scroll.delta.y * 0.1;
        *t = o.transform();
    }
}

impl Orbit {
    /// The camera's transform on this rig.
    fn transform(&self) -> Transform {
        let rot = Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        Transform::from_translation(self.focus + rot * Vec3::Z * self.radius).looking_at(self.focus, Vec3::Y)
    }
}

/// The sky's clouds and the wind they drift on (`--stage` in singles).
#[derive(Resource)]
struct Clouds {
    list: Vec<hst_sim::clouds::Cloud>,
    degrees: f32,
    speed: f32,
    /// 60 Hz ticks owed.
    ticks: f32,
    /// The court's cloud models and where they hang.
    models: Vec<Vec<(Handle<Mesh>, Handle<gs::GsMaterial>)>>,
    root: Entity,
    /// The heavy-rain layout (`exe::Game::rain_clouds`).
    rain: [f32; 3],
}

/// The match's cloud spawner between the setup and the points: the schedule's weathers (`HST_WEATHER` fixes one),
/// the weather the clouds were last made for, the court's count and the heavy-rain one, and the last layout's
/// weather and `rand()` draws until `clouds` lays them out. None when the court has no in-season cloud models.
pub struct Sky {
    weathers: [u8; hst_sim::weather::GAMES],
    fixed: Option<u8>,
    made: u8,
    count: usize,
    rain: usize,
    draws: Option<(u8, Vec<u32>)>,
}

impl Sky {
    fn made_count(&self, weather: u8) -> usize {
        if matches!(weather, 3 | 5) { self.rain } else { self.count }
    }
}

static SKY: std::sync::Mutex<Option<Sky>> = std::sync::Mutex::new(None);

/// A new point on game `game` of the schedule (`Rngs::new_point`): a weather change remakes the clouds
/// (`weather::clouds_remade`), their draws between the shared and the court generator's seeds, in doubles too.
pub fn point_clouds(rng: &mut hst_sim::rng::Rngs, game: usize) {
    let mut sky = SKY.lock().unwrap();
    let Some(s) = sky.as_mut().filter(|s| {
        let w = s.fixed.unwrap_or(s.weathers[game % hst_sim::weather::GAMES]);
        hst_sim::weather::clouds_remade(s.made, w)
    }) else {
        return rng.new_point();
    };
    let w = s.fixed.unwrap_or(s.weathers[game % hst_sim::weather::GAMES]);
    s.made = w;
    s.draws = Some((w, rng.new_point_clouds(s.made_count(w))));
}

#[derive(Component)]
/// Index into `Clouds::list` and each part's own material colour.
struct CloudView(usize, Vec<Vec4>);

/// Drifts the clouds at 60 Hz on the game's wind; each stands at its place relative to the camera, fades out near
/// the edge and darkens in rain. Laid out from the setup's and each weather change's draws (`point_clouds`): heavy
/// rain makes them low and many.
fn clouds(
    mut commands: Commands,
    time: Res<Time>,
    mut sky: Option<ResMut<Clouds>>,
    weather: Option<Res<weather::Weather>>,
    camera: Query<&GlobalTransform, With<Orbit>>,
    mut views: Query<(Entity, &CloudView, &mut Transform, &Children)>,
    parts: Query<&MeshMaterial3d<gs::GsMaterial>>,
    mut materials: ResMut<Assets<gs::GsMaterial>>,
) {
    let (Some(sky), Ok(camera)) = (sky.as_mut(), camera.single()) else { return };
    let today = weather.map(|w| w.today()).unwrap_or_default();
    (sky.degrees, sky.speed) = (today.degrees, today.speed);
    let made = SKY.lock().unwrap().as_mut().and_then(|s| s.draws.take());
    if let Some((w, draws)) = made {
        for (e, ..) in &views {
            commands.entity(e).despawn();
        }
        let (_, y0, y1, _) = hst_sim::weather::cloud_layout(w, (0, sky.rain));
        sky.list = hst_sim::clouds::spawn(&draws, sky.models.len(), [y0 * 40.0, y1 * 40.0]);
        for (i, k) in sky.list.iter().enumerate() {
            let rot = Quat::from_rotation_x(std::f32::consts::PI) * Quat::from_rotation_y(k.yaw);
            let bases = sky.models[k.model].iter().map(|(_, m)| materials.get(m).map_or(Vec4::ONE, |m| m.uniform.color)).collect();
            let e = commands.spawn((CloudView(i, bases), Transform::from_rotation(rot).with_scale(Vec3::splat(40.0)), Visibility::default())).id();
            for (m, mat) in &sky.models[k.model] {
                // its own material: the fade scales its alpha; clouds draw without fog
                let mut mat = materials.get(mat).expect("cloud material").clone();
                mat.key.modulate = true;
                mat.uniform.fog = gs::NO_FOG;
                // the game builds each cloud with object flags 3: alpha and VU1's unlit path
                mat.uniform.unlit = 1.0;
                // B75: with the backdrops, before everything else
                mat.order = BACKDROP;
                // and sets its TEST to GEQUAL 0x60, FB_ONLY on fail (mode 25's split at 0x70 moved down)
                mat.key.test = match mat.key.test {
                    gs::Test::Ge70 => gs::Test::Ge60,
                    gs::Test::Lt70 => gs::Test::Lt60,
                    t => t,
                };
                commands.entity(e).with_child((Mesh3d(m.clone()), MeshMaterial3d(materials.add(mat))));
            }
            commands.entity(sky.root).add_child(e);
        }
        return;
    }
    sky.ticks += time.delta_secs() * 60.0;
    while sky.ticks >= 1.0 {
        sky.ticks -= 1.0;
        let (d, s) = (sky.degrees, sky.speed);
        hst_sim::clouds::tick(&mut sky.list, d, s);
    }
    // game space is Bevy's turned a half-turn about X
    let eye = camera.translation() * Vec3::new(1.0, -1.0, -1.0);
    let tint = hst_sim::weather::cloud_tint(today.weather);
    for (_, v, mut t, children) in &mut views {
        let k = &sky.list[v.0];
        t.translation = Vec3::from(k.pos) + eye;
        let fade = hst_sim::clouds::fade(k);
        for (c, base) in children.iter().zip(&v.1) {
            // VU1 (flag bit 0): A = vc.a·mat.a·fade, highlight 0; the weather tint scales the light colour, the same
            // product (court 10 GS dump, research/p17s7_cloud_gs.py)
            if let Some(mut m) = parts.get(c).ok().and_then(|m| materials.get_mut(&m.0)) {
                m.uniform.color = (base.truncate() * tint).extend(base.w * fade);
            }
        }
    }
}

/// A sky, `_clo`, `_acc` or `_bg` model: the court draw stands it at the camera's eye + (0, lift, 0), turned by yaw
/// about game y (`gs::backdrop`; slot 5's RAM has these matrices).
#[derive(Component)]
struct Backdrop {
    yaw: f32,
    lift: f32,
}

/// A model not drawn while the eye is inside its box (model space `lo..hi`, `at` = game space → model space).
/// From inside a Grand Palais end house (B185) its walls are all back faces, but the clock on its end façade pokes
/// through the wall, so its casing's back half drew a ∧ in mid-court; the original shows only the court there (its
/// GS output has no prim nearer than the floor at that spot).
/// ponytail: by eye, not the original's rule (not found: its VU1 clip pass accepts those triangles); only the houses.
#[derive(Component)]
struct HideInside {
    lo: Vec3,
    hi: Vec3,
    at: bevy::math::Affine3A,
}

fn hide_inside(camera: Query<&Transform, With<Orbit>>, mut models: Query<(&HideInside, &mut Visibility)>) {
    let Ok(camera) = camera.single() else { return };
    let eye = camera.translation * Vec3::new(1.0, -1.0, -1.0);
    for (h, mut v) in &mut models {
        let e = h.at.transform_point3(eye);
        v.set_if_neq(if e.cmpge(h.lo).all() && e.cmple(h.hi).all() { Visibility::Hidden } else { Visibility::Inherited });
    }
}

/// The backdrops' blended-draw sort offset (`GsMaterial::order`): before the hole's `-1e6`, so before everything.
const BACKDROP: f32 = -2.0e6;

/// Runs after every Update camera move, so the backdrop never lags the camera by a frame.
fn backdrops(camera: Query<&Transform, (With<Orbit>, Without<Backdrop>)>, mut models: Query<(&Backdrop, &mut Transform)>) {
    let Ok(camera) = camera.single() else { return };
    // game space is Bevy's turned a half-turn about X
    let eye = camera.translation * Vec3::new(1.0, -1.0, -1.0);
    for (b, mut t) in &mut models {
        *t = Transform::from_translation(eye + Vec3::Y * b.lift).with_rotation(Quat::from_rotation_y(b.yaw));
    }
}

/// `HST_EYE=ex,ey,ez,fx,fy,fz,dx,dy,dz,t`: hold the camera at a view read from the original's RAM (game space: eye,
/// forward, down rows, t = tan of the horizontal half field of view), shown like the match camera (`play` keeps
/// 4:3's height), for `--shot` checks against `tools/screenshot.sh` from the same camera.
fn fixed_eye(mut camera: Query<(&mut Transform, &mut Projection), With<Orbit>>, window: Query<&Window>) {
    let Some(v) = std::env::var("HST_EYE").ok().map(|e| e.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 10) else { return };
    let Ok((mut t, mut proj)) = camera.single_mut() else { return };
    // game space → Bevy: (x, -y, -z)
    let b = |i: usize| Vec3::new(v[i], -v[i + 1], -v[i + 2]);
    *t = Transform::from_translation(b(0)).looking_to(b(3), -b(6));
    let aspect = window.single().map_or(16.0 / 9.0, crate::play::widescreen::scene_aspect);
    if let Projection::Perspective(p) = &mut *proj {
        p.fov = 2.0 * (v[9] * 0.75f32.max(1.0 / aspect)).atan();
    }
}

/// `HST_FPS=1`: every 5 s, the frame rate and the slowest frame to stderr, and each frame over 33 ms as it happens
/// (for profiling, P23, Z1).
fn log_fps(time: Res<Time<Real>>, mut acc: Local<(u32, f32, f32, f32)>, mut dts: Local<Vec<f32>>) {
    let dt = time.delta_secs();
    if time.elapsed_secs() > 0.0 {
        dts.push(dt);
    }
    // a stutter: any frame over two 60 Hz frames, with when it happened
    if dt > 1.0 / 30.0 {
        eprintln!("slow frame {:.1} ms at {:.2} s", dt * 1e3, time.elapsed_secs());
    }
    *acc = (acc.0 + 1, acc.1 + dt, acc.2.max(dt), acc.3);
    if acc.1 >= 5.0 {
        eprintln!("fps {:.1} (slowest frame {:.1} ms) at {:.0} s", acc.0 as f32 / acc.1, acc.2 * 1e3, time.elapsed_secs());
        eprintln!("{}", frame_spread(&mut dts));
        dts.clear();
        // the process's CPU time (all threads) per frame: steadier than fps on a busy machine
        let cpu = std::fs::read_to_string("/proc/self/stat").ok().and_then(|s| {
            let f: Vec<u64> = s.rsplit_once(')')?.1.split_whitespace().skip(11).take(2).filter_map(|v| v.parse().ok()).collect();
            Some(f.iter().sum::<u64>() as f32 * 10.0)
        });
        #[cfg(target_os = "ios")]
        let cpu = Some(ios::cpu_ms());
        #[cfg(target_os = "ios")]
        eprintln!("thermal {}", ios::thermal());
        if let Some(cpu) = cpu {
            eprintln!("cpu {:.2} ms/frame", (cpu - acc.3) / acc.0 as f32);
            acc.3 = cpu;
        }
        *acc = (0, 0.0, 0.0, acc.3);
    }
}

/// P44: the spread of a run of frame times (s): percentiles, the 1 % / 0.1 % lows (the fps of the slowest 1 % / 0.1 %
/// of frames, averaged) and the mean change from one frame to the next (pacing: an even 4 ms/4 ms/4 ms run is 0, a
/// 2/6/2/6 one 4 at the same mean fps).
fn frame_spread(dts: &mut [f32]) -> String {
    if dts.len() < 2 {
        return "frames: too few".into();
    }
    let n = dts.len();
    let jitter = dts.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / (n - 1) as f32;
    let mean = dts.iter().sum::<f32>() / n as f32;
    let sd = (dts.iter().map(|d| (d - mean) * (d - mean)).sum::<f32>() / n as f32).sqrt();
    dts.sort_by(|a, b| a.total_cmp(b));
    let pct = |p: f32| dts[((n as f32 * p) as usize).min(n - 1)] * 1e3;
    let low = |p: f32| {
        let k = ((n as f32 * p).ceil() as usize).max(1);
        k as f32 / dts[n - k..].iter().sum::<f32>()
    };
    format!(
        "frames ms: mean {:.2} sd {:.2} p50 {:.2} p90 {:.2} p99 {:.2} p99.9 {:.2} max {:.2} jitter {:.2} | 1% low {:.0} fps, 0.1% low {:.0} fps ({n} frames)",
        mean * 1e3,
        sd * 1e3,
        pct(0.5),
        pct(0.9),
        pct(0.99),
        pct(0.999),
        dts[n - 1] * 1e3,
        jitter * 1e3,
        low(0.01),
        low(0.001)
    )
}

/// `HST_FPS=1` too (B276): every 5 s, how many images (and their bytes), meshes, materials and entities there are,
/// to see a leak grow over a match.
fn log_assets(w: &World, mut next: Local<f32>) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    if now < *next {
        return;
    }
    *next = now + 5.0;
    let images = w.get_resource::<Assets<Image>>();
    let bytes: usize = images.iter().flat_map(|a| a.iter()).map(|(_, i)| i.data.as_ref().map_or(0, Vec::len)).sum();
    let n = |k: Option<usize>| k.unwrap_or(0);
    eprintln!(
        "assets at {now:.0} s: {} images ({} kB on CPU), {} meshes, {} std materials, {} fx materials, {} entities",
        n(images.map(|a| a.len())),
        bytes / 1024,
        n(w.get_resource::<Assets<Mesh>>().map(|a| a.len())),
        n(w.get_resource::<Assets<StandardMaterial>>().map(|a| a.len())),
        n(w.get_resource::<Assets<effects::Fx>>().map(|a| a.len())),
        w.entities().len(),
    );
    // the GPU's own allocations (Vulkan/DX12)
    if let Some(r) = w.get_resource::<bevy::render::renderer::RenderDevice>().and_then(|d| d.wgpu_device().generate_allocator_report()) {
        eprintln!("gpu at {now:.0} s: {} kB in {} allocations, {} kB reserved", r.total_allocated_bytes / 1024, r.allocations.len(), r.total_reserved_bytes / 1024);
    }
}

/// `HST_KEYS="2:Enter 9:Escape 10:ArrowDown 11:Enter 15:shot=a.png"`: at each real-time second a key is pressed for one
/// frame (Enter, Space, Escape, Arrow*, Digit1–4) or the window saved, for unattended runs across modes (`--shot` takes one and quits).
fn scripted_keys(mut commands: Commands, time: Res<Time<Real>>, mut keys: ResMut<ButtonInput<KeyCode>>, mut done: Local<usize>, mut pressed_at: Local<f32>) {
    let script = std::env::var("HST_KEYS").unwrap_or_default();
    let steps: Vec<_> = script.split_whitespace().filter_map(|s| s.split_once(':')).collect();
    for k in [KeyCode::Enter, KeyCode::Space, KeyCode::Escape, KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4] {
        // held through at least one 60 Hz menu tick (a fast display's frame may fall between two)
        if keys.pressed(k) && !keys.just_pressed(k) && time.elapsed_secs() - *pressed_at >= 1.0 / 30.0 {
            keys.release(k);
        }
    }
    let Some(&(at, what)) = steps.get(*done) else { return };
    if time.elapsed_secs() < at.parse::<f32>().unwrap_or(f32::MAX) {
        return;
    }
    *done += 1;
    info!("HST_KEYS: {what}");
    let key = match what {
        "Enter" => KeyCode::Enter,
        "Space" => KeyCode::Space,
        "Escape" => KeyCode::Escape,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        // the default emote keys (B252)
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        _ => {
            if let Some(path) = what.strip_prefix("shot=") {
                commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.to_string()));
            }
            return;
        }
    };
    keys.press(key);
    *pressed_at = time.elapsed_secs();
}

fn auto_shot(mut commands: Commands, args: Res<Args>, time: Res<Time>, mut taken: Local<bool>, mut exit: MessageWriter<AppExit>) {
    let Some(path) = &args.shot else { return };
    if !*taken && time.elapsed_secs() >= args.shot_at {
        *taken = true;
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
    }
    // the save is asynchronous: wait for the file
    if *taken && std::path::Path::new(path).exists() {
        commands.insert_resource(mode::Quit);
        exit.write(AppExit::Success);
    }
}

/// The window's icon (taskbar, title bar; Wayland takes it from `hst.desktop` by the window's name instead), set once
/// the window exists. `icon/hst-128.rgba` is web/icon.svg at 128×128.
#[cfg(not(target_arch = "wasm32"))]
fn window_icon(mut done: Local<bool>, _main_thread: bevy::ecs::system::NonSendMarker) {
    if *done {
        return;
    }
    bevy::winit::WINIT_WINDOWS.with_borrow(|w| {
        for win in w.windows.values() {
            let icon = winit::window::Icon::from_rgba(include_bytes!("../icon/hst-128.rgba").to_vec(), 128, 128).ok();
            win.set_window_icon(icon);
            *done = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P44: an even run has no jitter and its lows at its fps; a 2/6 ms alternation has the same mean but 4 ms jitter,
    /// and one 20 ms hitch in 1000 frames is the 0.1 % low.
    #[test]
    fn frame_spread_lows_and_jitter() {
        let even = frame_spread(&mut [0.004; 1000]);
        assert!(even.contains("mean 4.00 sd 0.00") && even.contains("jitter 0.00") && even.contains("1% low 250 fps, 0.1% low 250 fps"), "{even}");
        let alt: Vec<f32> = (0..1000).map(|i| if i % 2 == 0 { 0.002 } else { 0.006 }).collect();
        let alt = frame_spread(&mut alt.clone());
        assert!(alt.contains("mean 4.00") && alt.contains("jitter 4.00") && alt.contains("1% low 167 fps"), "{alt}");
        let mut hitch = vec![0.004; 1000];
        hitch[500] = 0.020;
        let hitch = frame_spread(&mut hitch);
        assert!(hitch.contains("max 20.00") && hitch.contains("0.1% low 50 fps"), "{hitch}");
    }

    /// M8f3: `--umpire-mod DIR` plays the mod over its donor (the donor in the chair unless `--umpire` says
    /// otherwise, as the menu's and the online room's always do); `--umpire-mod-id` of a mod not installed: none.
    #[test]
    fn umpire_mod_flags() {
        let dir = std::env::temp_dir().join(format!("hst-umpire-flag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.json"), r#"{"standard":1,"kind":"umpire","id":"u","name":"U","donor":1}"#).unwrap();
        let cli = |f: &[&str]| parse(["x.iso", "--play"].iter().chain(f).map(|s| s.to_string()));
        let d = dir.display().to_string();
        let c = cli(&["--umpire-mod", &d]);
        assert_eq!((c.args.umpire, c.umpire_mod.map(|u| u.id)), (1, Some("u".into())));
        let c = cli(&["--umpire", "3", "--umpire-mod", &d]);
        assert_eq!((c.args.umpire, c.umpire_mod.is_some()), (3, true));
        let c = cli(&["--umpire", "2", "--umpire-mod-id", "not-installed-anywhere"]);
        assert_eq!((c.args.umpire, c.umpire_mod.is_none()), (2, true));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// M8f6: `--court-mod DIR` plays on the mod's layout court unless `--stage`/`--court` say otherwise; a broken
    /// one is dropped (the disc court plays).
    #[test]
    fn court_mod_flags() {
        let dir = std::env::temp_dir().join(format!("hst-court-flag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.json"), r#"{"standard":1,"kind":"court","id":"c","name":"C","layout":4}"#).unwrap();
        std::fs::write(dir.join("court.glb"), b"x").unwrap();
        let cli = |f: &[&str]| parse(["x.iso", "--play"].iter().chain(f).map(|s| s.to_string()));
        let d = dir.display().to_string();
        let c = cli(&["--court-mod", &d]);
        assert_eq!((c.args.stage, c.args.court, c.court_mod.map(|c| c.id)), (Some(5), 5, Some("c".into())));
        let c = cli(&["--stage", "2", "--court", "2", "--court-mod", &d]);
        assert_eq!((c.args.stage, c.args.court, c.court_mod.is_some()), (Some(2), 2, true));
        std::fs::remove_file(dir.join("court.glb")).unwrap();
        let c = cli(&["--court-mod", &d]);
        assert_eq!((c.args.stage, c.court_mod.is_none()), (None, true));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// M8f8: an online room's `--court-mod-id` is looked up under `mods/` beside the disc: installed, it's drawn over
    /// the room's layout court; not installed (or on another layout here), the disc court plays as the room says.
    #[test]
    fn court_mod_id_flags() {
        let root = std::env::temp_dir().join(format!("hst-court-id-{}", std::process::id()));
        let dir = root.join("mods/pack/c");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.json"), r#"{"standard":1,"kind":"court","id":"c","name":"C","layout":4}"#).unwrap();
        std::fs::write(dir.join("court.glb"), b"x").unwrap();
        let iso = root.join("x.iso").display().to_string();
        let cli = |f: &[&str]| parse([iso.as_str(), "--play"].iter().chain(f).map(|s| s.to_string()));
        let c = cli(&["--stage", "5", "--court", "5", "--court-mod-id", "c"]);
        assert_eq!((c.args.stage, c.args.court, c.court_mod.map(|c| c.id)), (Some(5), 5, Some("c".into())));
        let c = cli(&["--stage", "5", "--court", "5", "--court-mod-id", "not-installed-anywhere"]);
        assert_eq!((c.args.stage, c.args.court, c.court_mod.is_none()), (Some(5), 5, true));
        let c = cli(&["--stage", "2", "--court", "2", "--court-mod-id", "c"]);
        assert_eq!((c.args.stage, c.court_mod.is_none()), (Some(2), true), "another layout here: the room's court");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn pow2_pads_with_edge_texels() {
        let t = |w: u32, h: u32| mtl::Texture { width: w, height: h, rgba: (0..w * h * 4).map(|i| (i / 4) as u8).collect(), mips: Vec::new(), psm: 0, levels: Vec::new(), clut: Vec::new() };
        assert!(pow2(&t(4, 4)).is_none());
        // 3×3 → 4×4: texel (x, y) = 3y + x, the last column and row repeated
        let p = pow2(&t(3, 3)).unwrap();
        let px: Vec<u8> = p.rgba.chunks(4).map(|c| c[0]).collect();
        assert_eq!((p.width, p.height), (4, 4));
        assert_eq!(px, [0, 1, 2, 2, 3, 4, 5, 5, 6, 7, 8, 8, 6, 7, 8, 8]);
    }
}
