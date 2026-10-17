//! The main menu (remaster-only flow in the original menus' art): Local, Online, Settings and Quit (not on web or the phone apps). Local goes through
//! singles/doubles, controller assignment, the character select (a cursor per player, at once) and the match
//! confirmation (court, sets, games, umpire), then starts the match as a child process of this executable with the
//! choices as flags (`--chars`, `--outfits`, `--umpire`, `--pads`, …) and comes back here when it ends.
//!
//! Art and text come from the disc: walls, titles and labels from `MENU/MENUxx.XB0`, the font from `word.tm2`, the
//! descriptions and names from `message0.dat`, the grades from the MENU overlay (`exe::Menu`), the play style from
//! TParam.csv. Settings (each enhancement on or off, and the graphics settings, `graphics.rs`) live in `settings.txt`
//! beside the disc image.
//!
//! Who plays: the device that chose Local is player 1; in the assignment the other pads and the keyboard join with ✕
//! (the first free seat, ←/→ to change seat, ○ or its seat's ✕ to leave; on the character select ✕ holds the seat open
//! for the next device's ✕, B238), up to four humans, any device on any seat; 1P's ←→ and △ add a computer player
//! to a free seat (or take it off, B216), and the seats left empty are filled at Start like an online lobby's (the
//! first characters nobody has). Player slots in the match: singles 1P→0, 2P→1;
//! doubles 1P→0, 2P→2 (1P's partner), 3P→1, 4P→3. In the character select a costume someone has locked in (ready)
//! is closed to everyone else on that character (B66: landing on it moves on to the next free costume, L1 / R1 pass
//! it, and a character with none free is drawn dark and passed over by the cursor); the computer players added are
//! picked by player 1 after the humans. B368: the select is one column per player (`roster`), each browsing and
//! scrolling the roster on its own device, its stats card always up (△ opens its name picker, B371).
//! `HST_MENU=<screen>` (main, rematch = main with a Rematch card, settings, graphics, sound, controls, mode, assign, chars, coms = chars with 1P ready (the COM cards), coms-random / confirm-random = 2P's computer readied on Random, chars-random = 1P on the Random card, names / names-type / names-card = chars with 1P named and its name picker open on the list / typing / closed (B371, B541), attrs / mods-attrs = chars / mods (B368: the stats card is always up), roster1–roster4 = 1–4 humans (B368), roster2-favs / roster4-favs = them with four favorites, 1P in Favorites and 2P on the last section (B382), roster2-deep = 2P's column scrolled to a mod far down, lobby-bot = the host's lobby with a computer player, lobby-singles = a singles lobby (B490), lobby-wait = the host's lobby waiting on a player choosing about a mod (B281), lobby-fetching = the host's lobby with a player still downloading a mod (M122), confirm-court = confirm on Nakagawa River Courts (B192), umpires-mod / confirm-umpire-mod = the umpire select / confirm on the first umpire mod (M8f3), courts-mod / confirm-court-mod = the court select / confirm on the first court mod (M8f6), chars4 = four humans on one portrait, mods = chars with 1P on the first mod, main-mods = main with the Mods row highlighted, main-update = main with B317's New version card highlighted (with `HST_LATEST`), rematch-update = both cards (M97), main-extras / extras = main on Extras / the Extras screen (M97), confirm, sets / games = confirm with that pill picked, assign-moved = assign with 1P moved to 3P, pending (B437), online-custom / relay-url = the Relay row on Custom... / its URL entry, renderer = Display with the Renderer switch's confirm popup up (B327), takeover / takeover-toast = main with the pad takeover prompt up / its Yes's toast (M127), graphics-list / graphics-fps = Graphics with Preset's / Frame Limit's dropdown open (B498)) opens on that screen (for `--shot`).
//!
//! Custom characters (remaster-only): every standard mod under `mods/` beside the disc image (`mods::list`) is on a
//! second page of the select, switched for everyone by Tab / □: B133: a grid like the disc characters', as many
//! columns and rows a page as the window fits (B190, [`Grid`]) in one framed group, each a face icon (`inspect::face_icon`, cut from the mod's baked
//! costume-0 preview, blank until that's baked) with its name under it, the players' hands and tags on it as on the
//! disc grid (←→ one, ↑↓ a row). A card on a mod shows its 3D preview, its name in `word.tm2`, its TParam.csv row's
//! play style and its power and footwork grades; its costumes are `mod.json`'s. Picked, it plays that match slot
//! (`--slot-mod N DIR`, `--chars` the donor), and a costume readied on is closed to the others as on the disc.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::sound;
use std::sync::Arc;

use super::controls::{self, Action, Bindings, Seat};
use super::widescreen::Anchor;
use crate::Args;
use crate::graphics::Graphics;
use crate::mods_online::Ask;
use crate::audio::{Sound, SoundBank, Volumes};

pub use super::widescreen::ASPECT;
pub(crate) use previews::face;

pub(super) mod cursor;
pub(crate) mod flame;
pub(crate) mod legend;
mod child;
mod confirm;
mod extras;
mod frame;
mod home;
mod look;
mod names;
mod online;
mod options;
mod osk;
mod over;
mod mods_kind;
mod mods_screen;
mod pointer;
mod tabs;
mod trace;
mod umpire_mods;
mod umpire_picker;
mod court_mods;
mod court_picker;
pub(super) mod pause;
mod plate_emote;
mod assign;
mod assign_shake;
mod previews;
mod roster;
mod slide;
mod splash;
mod takeover;

/// Characters on the select screen.
const CHARS: usize = 14;
/// Costumes per character (`--outfits` 0..9).
const COSTUMES: usize = 10;
/// The select grid: character and face centre on the 640×448 screen (beginner row, intermediate cluster, expert row).
const GRID: [(usize, f32, f32); CHARS] = [
    (0, 232.0, 128.0),
    (1, 300.0, 128.0),
    (2, 368.0, 128.0),
    (5, 435.0, 128.0),
    (13, 232.0, 242.0),
    (4, 300.0, 209.0),
    (3, 368.0, 209.0),
    (6, 435.0, 242.0),
    (7, 300.0, 278.0),
    (12, 368.0, 278.0),
    (8, 232.0, 360.0),
    (9, 300.0, 360.0),
    (10, 368.0, 360.0),
    (11, 435.0, 360.0),
];

/// B217: the dice for a Random pick (std's per-call hasher keys, never the match's RNG).
fn dice() -> u64 {
    std::hash::BuildHasher::hash_one(&std::hash::RandomState::new(), 0u8)
}

/// B296 / B368: the menus' layout on the window's view, in the centred 640×448 space reaching past it on the sides
/// (and into a tall window's letterbox): the select's view (its player columns, `roster::columns`); `strip` and
/// `detail` are the lobby's and confirm screens' (`reach`, `split`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Grid {
    /// The select's room: left, top, right, bottom (down to the legend strip).
    view: [f32; 4],
    strip: [f32; 4],
    detail: [f32; 4],
    /// How far the view reaches past each side of the 4:3 screen (640 units; B297's main screen).
    extra: f32,
}

impl Default for Grid {
    fn default() -> Self {
        Grid::fit(4.0 / 3.0)
    }
}

impl Grid {
    /// The layout on a view `aspect` wide.
    fn fit(aspect: f32) -> Self {
        Grid::fit_at(aspect, 1.0)
    }

    /// The layout on a view `aspect` wide at Menu Size `k` (the view `1 / k` as many units across and down).
    fn fit_at(aspect: f32, k: f32) -> Self {
        let (share, vshare) = super::widescreen::menu_shares(aspect, k);
        // how far the view reaches past the 4:3 screen on each side, in its units
        let ex = (640.0 / share - 640.0) / 2.0;
        let ey = (448.0 / vshare - 448.0) / 2.0;
        let (left, right, top, bottom) = (12.0 - ex, 628.0 + ex, -ey, 398.0 + ey);
        let w = right - left;
        // the lobby's and confirm screen's (B296's strip and detail panel)
        let strip = [left, top + 46.0, w, 70.0];
        let y = top + 124.0;
        let detail = if aspect >= 1.0 {
            let dw = 200.0f32.min(w * 0.4);
            [right - dw, y, dw, bottom - y]
        } else {
            [left, y + 8.0, w, bottom - y - 8.0]
        };
        Grid { view: [-ex, -ey, 640.0 + ex, 432.0 + ey], strip, detail, extra: ex }
    }
}

/// Keeps `Menu::grid` fitted to the window and the roster, live on resize; applies the Aspect Ratio setting live
/// (B117); B306: keeps the select's scroll on the shown player.
fn fit_grid(window: Query<&Window, With<PrimaryWindow>>, mut menu: ResMut<Menu>) {
    super::widescreen::ASPECT.store(menu.settings.3, std::sync::atomic::Ordering::Relaxed);
    super::key_hints::ON.store(menu.settings.0[KEY_HINTS], std::sync::atomic::Ordering::Relaxed);
    legend::LAYOUT.store(menu.settings.9, std::sync::atomic::Ordering::Relaxed);
    let Ok(w) = window.single() else { return };
    let g = Grid::fit_at(super::widescreen::aspect(w), menu.settings.apply_menu_scale(w));
    let resized = menu.grid != g;
    if resized {
        menu.grid = g;
    }
    // B368: each column's grid scrolls to its player's tile when it moves (or the window changes), else stays where
    // the wheel or a drag left it
    if menu.screen == Screen::Chars {
        for p in 0..menu.players() {
            if resized || menu.followed[p] != Some(menu.cell(p)) {
                menu.follow(p);
            }
        }
    }
    // B297: the main screen's New in the store column, on a window wide enough for it
    // (and for the Mods row's Updates and Store facts while it's highlighted: the listing is cached an hour)
    let mods_row = menu.main_rows().get(menu.sel) == Some(&home::Row::Mods);
    if menu.screen == Screen::Main && (mods_row || home::columns(menu.grid.extra).3.is_some()) && mods_screen::store_len(&menu).is_none() {
        menu.want_store();
    }
}
/// The original's courts on the confirm screen (names `message0` 276 + 2k, descriptions 581 + 2k).
const COURTS: usize = 11;
/// Each court's surface on the confirm screen (0 grass, 1 clay, 2 hard; `hst_sim::ball::COURTS` bounces the same).
const SURFACE: [usize; COURTS] = [1, 1, 1, 2, 0, 0, 2, 0, 1, 0, 2];
/// The umpires; the value after the last court or umpire is a random pick (`Menu::venue`).
const UMPIRES: usize = 5;
/// The court picker's strip order (the original's tiles 1–11 as courts k), then our Random.
const STRIP: [usize; COURTS] = [0, 3, 5, 1, 7, 8, 10, 9, 2, 6, 4];
/// Sets on the confirm screen (sets played; to win: (n + 1) / 2).
const SETS: [i32; 3] = [1, 3, 5];
/// Games to win a set on the confirm screen: the original's 2, 4 or 6 (its option steps by 2 and stops at the ends, B212).
const GAMES: [i32; 3] = [2, 4, 6];
/// Player colours: 1P pink, 2P orange, 3P blue, 4P green (`CharacterSelect2_03`'s cards, same order).
const COLOURS: [[f32; 3]; 4] = [[128.0, 60.0, 90.0], [128.0, 80.0, 20.0], [40.0, 90.0, 128.0], [70.0, 115.0, 30.0]];
pub(super) use super::word_label::{FONT, glyphs};

/// P53: the Auto preset's pick for this device (`graphics::pick`), logged.
pub(super) fn auto_graphics() -> Graphics {
    let d = crate::graphics::Device::here(touch_page());
    let g = crate::graphics::pick(&d);
    info!("auto graphics: {d:?} → {} ({}% scale, shadows {}, msaa {})", g.preset(), g.scale, g.shadows.name(), crate::graphics::msaa_name(g.msaa));
    g
}

/// P53: the Auto preset's timing (`graphics::auto_tune`) saves what it stepped down to; the rest of settings.txt (the
/// window, Show FPS too) stays as it is.
pub(crate) fn save_graphics(iso: &str, g: Graphics) {
    let path = Settings::path(iso);
    let mut s = crate::store::read_to_string(&path).map_or_else(|_| Settings::default(), |t| Settings::parse(&t));
    s.1 = Graphics { windowed: s.1.windowed, show_fps: s.1.show_fps, ..g };
    if let Err(e) = crate::store::write(&path, s.text()) {
        warn!("{}: {e}", path.display());
    }
}

// ---------------------------------------------------------------- settings

/// The enhancements, each on or off (all on by default), the graphics settings, the volumes (B235) and the aspect
/// ratio (B117, a `widescreen::ASPECTS` index) and what to do with an online lobby's mods (B289: the lobby's
/// prompt asks, or its "Download, always" / "Never download" answer) and the browser build's renderer (B273, a
/// `RENDERERS` index; web/index.html reads it from settings.txt in localStorage) and the confirm screen's last umpire
/// pick (B191: `UMPIRES` is Random, the first-time and fallback choice) and the Menu Size (`MENU_SIZES`: 0 Auto,
/// else tenths) and how many store downloads run at once (B339: 0 Auto, [`Settings::downloads_at_once`]) and the
/// pads' button icons (B362: a `legend::LAYOUTS` index, 0 Auto), saved in `settings.txt`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Settings([bool; 9], Graphics, Volumes, u8, Ask, u8, u8, u8, u8, u8, Ask);
/// B273: the browser's renderer setting, each as its settings.txt / web/index.html word and its Settings row's word
/// (B316: the row's value and choices both come from here): Auto (WebGPU when the browser has an adapter), WebGPU,
/// WebGL2.
const RENDERERS: [(&str, &str); 3] = [("auto", "Auto"), ("webgpu", "WebGPU"), ("webgl2", "WebGL2")];
/// The Menu Size setting's values: Auto (0), then 50% to 150% in tenths.
const MENU_SIZES: [u8; 12] = [0, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
/// `music` is the match's (an older `settings.txt`'s one Music switch keeps meaning the match). Their rows are on the
/// Settings screen's pages (`options.rs`).
const SETTING_NAMES: [&str; 9] = ["upscaled_textures", "menu_music", "music", "upscaled_downloads", "key_hints", "chat", "vibration", "update_check", "emote_voices"];
/// Upscaled Downloads (B227): the mod store downloads mods' `textures/upscaled/` sets (≈ 90% of a mod's size). It
/// follows Texture Replacements (when settings.txt lacks it, and when that switch is flipped) unless switched itself.
const UPSCALED_DOWNLOADS: usize = 3;
/// B301: the match's keyboard hints (`play::key_hints`).
const KEY_HINTS: usize = 4;
/// M20: online chat shown (`crate::chat`).
const CHAT: usize = 5;
/// B118: the on-screen pad's buzz on each press (`web/touch.js`).
const VIBRATION: usize = 6;
/// B374: look for a newer release at start (`crate::update`).
const UPDATE_CHECK: usize = 7;
/// B474: the emotes' voice lines heard (`audio::EMOTE_VOICES`).
const EMOTE_VOICES: usize = 8;

/// Texture replacements and menu music on, match music off (user 2026-10-12); key hints on (B301); chat on (M20);
/// vibration on (B118); update check on (B374); emote voices on (B474).
const SETTING_DEFAULTS: [bool; 9] = [true, true, false, true, true, true, true, true, true];

/// B339: the most store downloads the Downloads at Once setting allows: 8 on desktop, 4 in a browser, 2 on a phone.
fn max_downloads() -> usize {
    if touch_page() {
        2
    } else if cfg!(target_arch = "wasm32") {
        4
    } else {
        8
    }
}

/// Auto, but 16:9 on a phone's browser (B117: played by touch). The phone apps fit Auto into the safe area (M106).
fn default_aspect() -> u8 {
    if touch_page() && !cfg!(any(target_os = "ios", target_os = "android")) { 2 } else { 0 }
}

impl Default for Settings {
    fn default() -> Self {
        Settings(SETTING_DEFAULTS, Graphics::default(), Volumes::default(), default_aspect(), Ask::Each, 0, UMPIRES as u8, 0, 0, 0, Ask::Each)
    }
}

impl Settings {
    fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        let (mut downloads, mut aspect, mut wide) = (None, None, None);
        crate::mods_textures::set_order(Default::default());
        for (k, v) in text.lines().filter_map(|l| l.split('#').next()?.split_once('=')) {
            if crate::mods_textures::setting(k, v) {
                // M5a: the texture packs' order, kept in `mods_textures`
            } else if k.trim() == "online_mods" {
                s.4 = Ask::parse(v).unwrap_or(s.4);
            } else if k.trim() == "keep_downloads" {
                s.10 = Ask::parse(v).unwrap_or(s.10);
            } else if k.trim() == "umpire" {
                s.6 = v.trim().parse().ok().filter(|&u| u as usize <= UMPIRES).unwrap_or(UMPIRES as u8);
            } else if k.trim() == "menu_size" {
                s.7 = v.trim().trim_end_matches('%').parse::<u8>().ok().map(|p| p / 10).filter(|t| MENU_SIZES.contains(t)).unwrap_or(0);
            } else if k.trim() == "downloads_at_once" {
                s.8 = v.trim().parse::<u8>().ok().filter(|&n| n as usize <= max_downloads()).unwrap_or(0);
            } else if k.trim() == "button_icons" {
                s.9 = legend::LAYOUTS.iter().position(|l| l.1 == v.trim()).unwrap_or(0) as u8;
            } else if k.trim() == "renderer" {
                s.5 = RENDERERS.iter().position(|r| r.0 == v.trim()).unwrap_or(0) as u8;
            } else if k.trim() == "aspect" {
                aspect = super::widescreen::ASPECTS[..super::widescreen::aspect_choices()].iter().position(|a| *a == v.trim()).map(|i| i as u8);
            } else if k.trim() == "widescreen" {
                // before B117's aspect ratio: on was Auto, off the 4:3 picture
                wide = Some(if v.trim() == "off" { 1 } else { 0 });
            } else if let Some(i) = SETTING_NAMES.iter().position(|n| *n == k.trim()) {
                s.0[i] = v.trim() != "off";
                downloads = downloads.or((i == UPSCALED_DOWNLOADS).then_some(s.0[i]));
            } else if k.trim() == "uncapped_fps" {
                // before the graphics settings: off waited for the display
                s.1.vsync = v.trim() == "off";
            } else {
                s.1.set(k, v);
            }
        }
        s.0[UPSCALED_DOWNLOADS] = downloads.unwrap_or(s.0[0]);
        s.3 = aspect.or(wide).unwrap_or(s.3);
        s.2 = Volumes::parse(text);
        crate::chat::HIDDEN.store(!s.0[CHAT], std::sync::atomic::Ordering::Relaxed);
        crate::update::CHECK.store(s.0[UPDATE_CHECK], std::sync::atomic::Ordering::Relaxed);
        crate::audio::EMOTE_VOICES.store(s.0[EMOTE_VOICES], std::sync::atomic::Ordering::Relaxed);
        s
    }

    fn text(&self) -> String {
        let on: String = SETTING_NAMES.iter().zip(self.0).map(|(n, on)| format!("{n} = {}\n", if on { "on" } else { "off" })).collect();
        format!("aspect = {}\n", super::widescreen::ASPECTS[self.3 as usize]) + &on + &format!("online_mods = {}\n", self.4.word()) + &format!("keep_downloads = {}\n", self.10.word()) + &self.1.text() + &self.2.text() + &self.renderer_line() + &format!("umpire = {}\n", self.6) + &format!("menu_size = {}\n", if self.7 == 0 { "auto".into() } else { format!("{}", self.7 as u32 * 10) }) + &format!("downloads_at_once = {}\n", if self.8 == 0 { "auto".into() } else { self.8.to_string() }) + &format!("button_icons = {}\n", legend::LAYOUTS[self.9 as usize].1) + &crate::mods_textures::settings_text()
    }

    /// B339: how many store installs run at once: the setting, or on Auto 3 on desktop, 2 in a browser, 1 on a
    /// phone (each install in a browser holds its mod in memory until the page has saved it).
    pub(super) fn downloads_at_once(&self) -> usize {
        match self.8 {
            0 if touch_page() => 1,
            0 if cfg!(target_arch = "wasm32") => 2,
            0 => 3,
            n => (n as usize).min(max_downloads()),
        }
    }

    /// The Menu Size factor on a window `physical_h` px tall: Auto is 100%, 80% on a window taller than 1440 px.
    fn menu_scale(&self, physical_h: u32) -> f32 {
        match self.7 {
            0 if physical_h > 1440 => 0.8,
            0 => 1.0,
            t => t as f32 / 10.0,
        }
    }

    /// Puts the Menu Size factor for window `w` in force (`widescreen::MENU_SCALE`, which the menus' drawing and
    /// pointer read) and returns it.
    pub(super) fn apply_menu_scale(&self, w: &Window) -> f32 {
        let k = self.menu_scale(w.physical_height());
        super::widescreen::MENU_SCALE.store(k.to_bits(), std::sync::atomic::Ordering::Relaxed);
        k
    }

    /// B273: the renderer is the browser build's alone.
    fn renderer_line(&self) -> String {
        if cfg!(target_arch = "wasm32") { format!("renderer = {}\n", RENDERERS[self.5 as usize].0) } else { String::new() }
    }

    pub(crate) fn path(iso: &str) -> std::path::PathBuf {
        crate::store::settings(iso)
    }

    /// The match's flags for these settings.
    fn flags(&self) -> Vec<String> {
        let [upscale, _, music, ..] = self.0;
        let on = [(!upscale, "--no-upscale"), (music, "--music")].into_iter().filter(|(on, _)| *on).map(|(_, f)| f.to_string());
        on.chain(self.1.flags()).collect()
    }
}

// ---------------------------------------------------------------- state

/// An input device: the keyboard or a gamepad.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dev {
    Keys,
    Pad(Entity),
    /// B238: a human's seat on the character select whose device was taken off (✕): the next device's ✕ takes it.
    Open,
}

/// One device's presses this frame (directions already repeated).
#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Press {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    ok: bool,
    back: bool,
    /// △: a computer player on the seat assignment, an umpire's call; on the select the favorite (B375).
    card: bool,
    /// L1 / R1: the costume.
    prev: bool,
    next: bool,
    /// □ / Tab: on the select 1P's Singles / Doubles swap (B375).
    page: bool,
    /// Start (keyboard: N): the select's name picker (B375); a pad's Start is also ✕ off the select.
    name: bool,
    /// Select / − (keyboard: the camera key, C): the hand toggle.
    hand: bool,
    /// B373: L2 / R2 (keyboard: 1 / 3, over Q / E): the Mods pages' kind tab.
    kind_prev: bool,
    kind_next: bool,
    /// B558: O / R3: the Settings pages over the select and the lobby (`over.rs`).
    settings: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    /// B516: the press-start splash a fresh start opens on (`splash.rs`): its first press takes 1P.
    Press,
    /// `bgmm_03` plays on `Press` (the original's title screen), `bgmm_04` on the first three, `bgmm_05` on the next three, `bgmm_06` on Confirm (the MENU overlay's track
    /// switch, measured per screen).
    Main,
    /// B290/B294: the Settings tabs (`options.rs`).
    Display,
    Graphics,
    /// B411: Graphics → Advanced, the speed-up switches (under the Graphics tab).
    Advanced,
    /// B235: the volume sliders.
    Sound,
    Controls,
    /// Online & Mods: what the mod store and online lobbies download.
    ModSettings,
    /// B118: phone-only options (on a touch screen).
    Mobile,
    Mode,
    Assign,
    Chars,
    Confirm,
    /// Online (`online.rs`): its menu, Find's list, the lobby.
    Online,
    Lobbies,
    Lobby,
    /// Join by Code's code entry (B104).
    Code,
    /// M16: add and remove mods (the browser build's; `mods_screen.rs`).
    Mods,
    /// M97: Extras (`extras.rs`): Donate.
    Extras,
    /// A match is running (the window is hidden).
    Playing,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Player {
    /// Grid index.
    cursor: usize,
    costume: usize,
    ready: bool,
    /// The custom list's entry, and whether the player is on it (else on the grid).
    pick: usize,
    modded: bool,
    /// Select / −: plays the character with the other hand (`hand_swap`), kept across characters.
    swap: bool,
    /// B217: on the Random card (the ? card until ready; readying draws the pick, `Menu::draw`).
    random: bool,
    /// B369 / B382: the roster section the cursor is in (`tabs.rs`: a favorite is in two).
    tab: u8,
}

impl Player {
    fn char(&self) -> usize {
        GRID[self.cursor].0
    }

    /// B346: a seat entering the select: its cursor on the Random card.
    fn fresh() -> Self {
        Player { random: true, ..default() }
    }
}

/// Who a player has on the select: a disc character or the custom roster's entry.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Who {
    Disc(usize),
    Mod(usize),
}

#[derive(Resource, Clone, Debug)]
struct Menu {
    screen: Screen,
    sel: usize,
    settings: Settings,
    doubles: bool,
    /// The device driving each player (menu order 1P..4P); `None` is the computer.
    seats: [Option<Dev>; 4],
    /// B216: the seats 1P added a computer player to (picked on the select by 1P); a seat with no device and no bot
    /// is a COM card on its automatic pick ([`Menu::preset_coms`]). Online, the room's computer seats (`align`).
    bots: [bool; 4],
    players: [Player; 4],
    court: usize,
    sets: usize,
    games: i32,
    umpire: u8,
    /// The dice for a random court or umpire, thrown at Start (`venue`).
    roll: u64,
    /// The action waiting for a new key or button (controls screen).
    rebinding: Option<usize>,
    /// B291: the Controls page's tab: Keyboard & Mouse (else Controller).
    keys_tab: bool,
    /// The custom roster.
    mods: Vec<crate::mods::Mod>,
    /// M8f3: the installed umpire mods, on the umpire select after the disc five (`umpire_mods`).
    umpire_mods: Vec<crate::mods_umpire::Umpire>,
    /// M8f6: the installed court mods, on the court select after the disc eleven (`court_mods`).
    court_mods: Vec<crate::mods_court::Court>,
    /// B400: their pictures, by the same index (`umpire_mods::load`; None: it ships none).
    umpire_pics: Vec<Option<umpire_mods::Pic>>,
    /// B451: whether each umpire mod ships its own voice preview (Preview Voice's calls), by the same index.
    umpire_voices: Vec<bool>,
    /// B451: the umpire picker's rows' places.
    umpire_pick: umpire_picker::Picker,
    /// B401: the court mods' `previews/court.png` sizes (`court_mods::Pics`), by `court_mods` index.
    court_pics: Vec<Option<[f32; 2]>>,
    /// B368: each player's column's first row on view ([`Menu::page`]), and the tile it was last scrolled to
    /// (`Menu::follow`).
    scroll: [usize; 4],
    followed: [Option<Option<Who>>; 4],
    /// The custom roster's indices, newest installed first (each `mod.json`'s time, read once at startup; folder
    /// order where there's no time, as in the browser): the main screen's Mods picture.
    newest: Vec<usize>,
    /// B296: the player whose pick the select's detail panel shows: the last one to press anything there.
    focus: usize,
    /// M1n: (custom roster entry, costume) whose model failed to load in the select: L1 / R1 pass it, it can't be
    /// readied on.
    broken: std::collections::HashSet<(usize, usize)>,
    /// The Online screens' state; `net.go` is set while the local screens pick this machine's players for Online.
    net: online::View,
    /// The confirmation screen's flame, from P1's pick.
    flame: flame::Flame,
    /// B64: the court or umpire picker is open (on row `sel`), with the value it opened on (○ puts it back).
    picking: Option<usize>,
    /// B190: the custom grid's fit to the window.
    grid: Grid,
    /// M16: the Mods screen's state.
    add: mods_screen::View,
    /// B254: the assignment screen over a paused match (`pause.rs`): `bots` are the match's computer players, fixed
    /// (no △, never swapped); ✕ takes any device off its seat (1P too, without moving the others up).
    paused: bool,
    /// B269: the roster cell the last pick readied on (`None` inside: the Random card); the next computer player's
    /// cursor starts there.
    last: Option<Option<Who>>,
    /// B428: the roster section that pick was readied in (`Player::tab`): the next computer player's cursor opens
    /// there too (Favorites stays Favorites).
    last_tab: u8,
    /// B297: a local match was started from this menu this session; its setup is still here (the Rematch card).
    rematch: bool,
    /// The umpire picker's Preview Voice presses so far (each plays the next of [`CALLS`]).
    calls: usize,
    /// B326: a Settings page's row is being changed (row `sel`), with the settings it opened on (○ puts them back).
    editing: Option<Settings>,
    /// B327: the Renderer switch's confirm popup is up: the renderer it would switch to and the chosen pill (0 Switch).
    switch: Option<(u8, usize)>,
    /// Online pressed while a newer version is out: the suggestion popup's chosen pill (0 Update, 1 Play anyway), and
    /// whether it has been shown this run (it asks once; Play anyway always goes on).
    stale: (Option<usize>, bool),
    /// B498: a many-choice Settings row's dropdown is open (row `sel`, `editing` set): the highlighted choice.
    listing: Option<usize>,
    /// B369: the select's favorite characters (`favorites.txt`).
    favs: tabs::Favorites,
    /// B371: the players' names (`names.rs`).
    names: names::Names,
    /// B450: the court picker's rows' places.
    court_pick: court_picker::Picker,
    /// B437: the device that drives the controller assignment once it has moved or taken off a seat there: it keeps
    /// driving it (its own seat change pending, shown on the screen) until it leaves, where the new 1P takes over.
    assign_lead: Option<Dev>,
    /// The assignment: 1P has picked up the focused card's controller (✕); ←→ carry it, ✕ puts it down.
    held: bool,
    /// The assignment's seats and computer players as it opened, and over a match the Keep / Undo popup's pill.
    assign_was: Option<([Option<Dev>; 4], [bool; 4])>,
    keep: Option<usize>,
    /// B516: the device that took 1P on the splash (`splash.rs`), the one that drives 1P's player.
    p1: Option<Dev>,
    splash: splash::Splash,
    /// P21a: each seat's card's sideways offset on the controller assignment while its pad's right stick moves
    /// (`assign_shake.rs`).
    shake: [f32; 4],
    /// B564: how bright each seat's press-to-find flash is (1 just pressed, fading to 0), `assign_shake.rs`.
    flash: [f32; 4],
    /// M124: the assignment over an online match's pause: the seats another machine plays. Only this machine's own
    /// seats (the rest are `bots`, fixed) take or swap its pads, and no device is taken off.
    net_pause: Option<[bool; 4]>,
    /// M127: the pad takeover prompt (`takeover.rs`).
    takeover: takeover::Takeover,
    /// B558: the Settings pages opened from the select or the lobby: the screen and row Back returns to (`over.rs`).
    over: Option<(Screen, usize)>,
}

impl Default for Menu {
    fn default() -> Self {
        Menu {
            screen: Screen::Main,
            sel: 0,
            settings: Settings::default(),
            doubles: false,
            seats: [None; 4],
            bots: [false; 4],
            players: [Player::default(); 4],
            court: COURTS, // Random
            sets: 0,
            games: 4,
            umpire: UMPIRES as u8, // B191: Random until a pick is saved
            roll: 0,
            rebinding: None,
            keys_tab: false,
            mods: Vec::new(),
            umpire_mods: Vec::new(),
            court_mods: Vec::new(),
            umpire_pics: Vec::new(),
            umpire_voices: Vec::new(),
            umpire_pick: default(),
            court_pics: Vec::new(),
            scroll: [0; 4],
            followed: [None; 4],
            newest: Vec::new(),
            focus: 0,
            broken: default(),
            net: online::View::default(),
            flame: flame::Flame::default(),
            picking: None,
            grid: Grid::default(),
            add: default(),
            paused: false,
            last: None,
            last_tab: 0,
            rematch: false,
            calls: 0,
            editing: None,
            switch: None,
            stale: (None, false),
            listing: None,
            favs: default(),
            names: default(),
            court_pick: default(),
            assign_lead: None,
            held: false,
            assign_was: None,
            keep: None,
            p1: None,
            splash: default(),
            shake: [0.0; 4],
            flash: [0.0; 4],
            net_pause: None,
            takeover: default(),
            over: None,
        }
    }
}

/// Rows per screen (the main screen's: `Menu::main_rows`).
const CONFIRM_ROWS: usize = 5;

/// The system bank's cue for a device leaving the roster (joining plays the back cue, key 1).
const LEAVE_SOUND: u8 = 3;

/// What a step asks of the app.
#[derive(Debug, PartialEq)]
#[allow(clippy::large_enum_variant)] // `Online`'s network command, as `online::Fx`: a step's effects live for one frame
enum Effect {
    Sound(u8),
    SaveSettings,
    Launch,
    /// The game restarted itself after a self-update (no browser equivalent).
    #[cfg(not(target_arch = "wasm32"))]
    Quit,
    ResetControls,
    Online(online::Fx),
    /// The umpire picker's Preview Voice: umpire, voice program, key (`hst_sim::umpire`'s banks).
    Hear(u8, u8, u8),
    /// B369: `favorites.txt` written.
    SaveFavorites,
}

impl Menu {
    fn players(&self) -> usize {
        if self.doubles { 4 } else { 2 }
    }

    fn who(&self, p: usize) -> Who {
        let pl = self.players[p];
        if pl.modded { Who::Mod(pl.pick) } else { Who::Disc(pl.char()) }
    }

    /// Player `p`'s character and costume as drawn and played: a disc character's own costume, else (past its
    /// [`COSTUMES`]) the costume pack's (`Mod::costume_of`) it lands on.
    fn pick(&self, p: usize) -> (Who, usize) {
        self.resolve(self.who(p), self.players[p].costume)
    }

    fn resolve(&self, w: Who, costume: usize) -> (Who, usize) {
        if let Who::Disc(c) = w
            && costume >= COSTUMES
        {
            let mut k = costume - COSTUMES;
            for (i, m) in self.mods.iter().enumerate().filter(|(_, m)| m.costume_of == Some(c)) {
                if k < m.costumes.len() {
                    return (Who::Mod(i), k);
                }
                k -= m.costumes.len();
            }
        }
        (w, costume)
    }

    fn costumes(&self, w: Who) -> usize {
        match w {
            Who::Disc(c) => COSTUMES + self.mods.iter().filter(|m| m.costume_of == Some(c)).map(|m| m.costumes.len()).sum::<usize>(),
            Who::Mod(i) => self.mods[i].costumes.len(),
        }
    }

    /// Player `p`'s hand in the match: their pick's (a mod's own), the other one with the toggle on.
    fn hand(&self, text: &Text, p: usize) -> f32 {
        let h = match self.pick(p).0 {
            Who::Disc(c) => text.hands[c],
            Who::Mod(i) => self.mods[i].hand,
        };
        if self.players[p].swap { -h } else { h }
    }

    /// Whether player `p`'s character and costume are locked in by someone else.
    fn blocked(&self, p: usize) -> bool {
        !self.free(p, self.who(p), self.players[p].costume)
    }

    /// Whether `w`'s costume `c` is open to player `p`: loaded, and nobody else ready on it.
    fn free(&self, p: usize, w: Who, c: usize) -> bool {
        let pick = self.resolve(w, c);
        !matches!(pick, (Who::Mod(i), k) if self.broken.contains(&(i, k)))
            && !(0..self.players()).any(|q| q != p && self.players[q].ready && self.pick(q) == pick)
    }

    /// B66: `w`'s first costume open to player `p` from `from` on (wrapping, `back` the other way), if any.
    fn next_free(&self, p: usize, w: Who, from: usize, back: bool) -> Option<usize> {
        let n = self.costumes(w);
        (0..n).map(|j| if back { (from + n - j) % n } else { (from + j) % n }).find(|&c| self.free(p, w, c))
    }

    /// B66: every player not ready who sits on a costume that's taken moves on to that character's next free one.
    fn skip_taken(&mut self) {
        for p in 0..self.players() {
            if !self.players[p].ready && self.blocked(p) && let Some(c) = self.next_free(p, self.who(p), self.players[p].costume, false) {
                self.players[p].costume = c;
            }
        }
    }

    /// B217: a character open to player `p` (any disc character or custom one, a costume pack's costumes going with
    /// its disc character) and one of its open costumes, each at random by `seed`; `None` if nothing's open.
    /// B520: in an online lobby with mods off, only the disc characters and their own costumes.
    fn draw(&self, p: usize, seed: u64) -> Option<(Who, usize)> {
        let vanilla = self.vanilla_lobby();
        let mods = if vanilla { 0 } else { self.own_mods() };
        let whos = (0..CHARS).map(Who::Disc).chain((0..mods).filter(|&i| self.mods[i].costume_of.is_none()).map(Who::Mod));
        let n = |w: Who| if vanilla { COSTUMES } else { self.costumes(w) };
        let open: Vec<(Who, Vec<usize>)> = whos.map(|w| (w, (0..n(w)).filter(|&c| self.free(p, w, c)).collect::<Vec<_>>())).filter(|(_, c)| !c.is_empty()).collect();
        let n = open.len() as u64;
        let (w, cs) = open.get((seed % n.max(1)) as usize)?;
        Some((*w, cs[(seed / n % cs.len() as u64) as usize]))
    }

    /// B217: player `p` on Random readies: the drawn pick put on their cursor (still on Random, so ○ shows the ? card
    /// again). False if nothing's open.
    fn roll_random(&mut self, p: usize) -> bool {
        let Some((w, c)) = self.draw(p, dice()) else { return false };
        let me = &mut self.players[p];
        match w {
            Who::Disc(ch) => (me.modded, me.cursor) = (false, GRID.iter().position(|g| g.0 == ch).unwrap_or(0)),
            Who::Mod(i) => (me.modded, me.pick) = (true, i),
        }
        me.costume = c;
        true
    }

    /// Player `p` picking again (on the same character and costume).
    fn unready(&mut self, p: usize) {
        self.players[p].ready = false;
    }

    /// B296 / B306: the select's one roster: the Random card (`None`) first, then the disc characters in the
    /// original's grid order (beginner, intermediate, expert), then the custom ones in folder order.
    fn roster(&self) -> Vec<Option<Who>> {
        let disc = GRID.iter().map(|g| Who::Disc(g.0));
        std::iter::once(None).chain(disc.chain((0..self.own_mods()).map(Who::Mod)).map(Some)).collect()
    }

    /// B296: player `p`'s roster cell: their character, or `None` on the Random card.
    fn cell(&self, p: usize) -> Option<Who> {
        (!self.players[p].random).then(|| self.who(p))
    }

    /// B296: player `p` onto roster cell `c` (a new character starts on its first costume).
    fn put(&mut self, p: usize, c: Option<Who>) {
        let me = &mut self.players[p];
        match c {
            None => me.random = true,
            Some(Who::Disc(ch)) => (me.modded, me.cursor, me.random, me.costume) = (false, GRID.iter().position(|g| g.0 == ch).unwrap_or(0), false, 0),
            Some(Who::Mod(i)) => (me.modded, me.pick, me.random, me.costume) = (true, i, false, 0),
        }
    }

    /// B66 / B217: whether roster cell `c` has nothing open to player `p` (every costume taken or broken; the Random
    /// card when nothing at all is).
    fn full(&self, p: usize, c: Option<Who>) -> bool {
        match c {
            None => self.draw(p, 0).is_none(),
            Some(w) => self.next_free(p, w, 0, false).is_none(),
        }
    }

    /// B296: the player the select's detail panel and description follow: the one who pressed last while picking,
    /// else the first not ready (online: the host's computer player being edited, else this machine's first).
    #[cfg(test)]
    fn shown(&self) -> usize {
        let n = self.players();
        let first = (0..n).find(|&p| !self.players[p].ready).unwrap_or(0);
        let lead = if self.select_online() {
            self.net.edit.or((0..n).find(|&p| self.seats[p].is_some() && !self.players[p].ready)).or((0..n).find(|&p| self.seats[p].is_some())).unwrap_or(first)
        } else {
            first
        };
        if self.focus < n && !self.players[self.focus].ready && self.focus != lead && self.seats[self.focus].is_some() { self.focus } else { lead }
    }

    /// B368: player `p`'s column scrolled as little as shows their tile; B382: its section's heading too while it's on
    /// the section's first row.
    fn follow(&mut self, p: usize) {
        let c = self.columns();
        let secs = self.sections();
        let lines = tabs::lines(&secs, c.cols);
        let row = self.place(p, &secs).and_then(|k| tabs::unflat(&secs, k)).and_then(|(s, i)| lines.iter().position(|&l| l == tabs::Line::Tiles(s, i - i % c.cols)));
        if let Some(row) = row {
            let want = if row > 0 && matches!(lines[row - 1], tabs::Line::Head(_)) { row - 1 } else { row };
            let mut top = self.scroll[p].min(want);
            while top < row && c.fit(&lines, top) <= row - top {
                top += 1;
            }
            self.scroll[p] = top.min(c.last_top(&lines));
        }
        self.followed[p] = Some(self.cell(p));
    }

    /// B368: player `p`'s column's grid scrolled `d` lines (the wheel, a drag), its tile free to leave the view.
    fn scroll_by(&mut self, p: usize, d: i32) -> bool {
        let c = self.columns();
        let last = c.last_top(&tabs::lines(&self.sections(), c.cols));
        let to = (self.scroll[p].min(last) as i32 + d).clamp(0, last as i32) as usize;
        std::mem::replace(&mut self.scroll[p], to) != to
    }

    /// B368: the select's player columns on this window.
    fn columns(&self) -> roster::Columns {
        // B519: online, the relay's notice band under the top bar pushes the columns down
        let [l, t, r, b] = self.grid.view;
        let band = online::band_h(self, r - l - 2.0 * roster::MARGIN * roster::PX);
        // M121: online, the lobby's strip and bottom row under them
        let b = match self.online() {
            true => {
                let [_, y, _, h] = self.lobby_foot().stage;
                y + h
            }
            false => b,
        };
        roster::columns_mixed([l, t + band, r, b], self.players(), std::array::from_fn(|p| self.compact(p)))
    }

    /// B201: the select of an online room not started yet.
    fn select_online(&self) -> bool {
        self.net.room.is_some() && !self.net.me.is_empty() && !self.net.started
    }

    /// The player `dev` drives on the character select: its own until it is ready, then (player 1's device, once
    /// every human is ready) the first computer player not ready.
    fn driven(&self, dev: Dev) -> Option<usize> {
        let n = self.players();
        if let Some(p) = (0..n).find(|&p| self.seats[p] == Some(dev)) {
            let humans_ready = (0..n).all(|q| self.seats[q].is_none() || self.players[q].ready);
            if self.online() {
                return self.online_driven(dev, p);
            }
            // B258: any human, once every human is ready (not only 1P)
            if !self.players[p].ready || !humans_ready {
                return Some(p);
            }
            return (0..n).find(|&q| self.bot(q) && !self.players[q].ready).or(Some(p));
        }
        None
    }

    /// Seat `p` is a computer player on the select (no device on it): one 1P added (B216), or a seat left open,
    /// whose card starts on its automatic pick (B258, [`Menu::preset_coms`]).
    fn bot(&self, p: usize) -> bool {
        self.seats[p].is_none()
    }

    /// B216: seat `p` has nobody: no device, no computer player added; it fills at Start.
    fn open(&self, p: usize) -> bool {
        !self.bots[p] && self.seats[p].is_none()
    }

    /// B216: whether 1P's △ on the seat assignment adds or removes a computer player on seat `p` (local only).
    fn can_bot(&self, p: usize) -> bool {
        !self.online() && !self.paused && (1..self.players()).contains(&p) && self.seats[p].is_none()
    }

    /// B216: the pick of a seat nobody took and no computer player was added to, as an online lobby's at Start: the
    /// first disc character nobody else picked, in number order, on its first free costume (B66); not ready.
    fn auto_com(&self, q: usize) -> Player {
        let taken: Vec<usize> = (0..self.players())
            .filter(|&o| o != q && (self.players[o].ready || !self.open(o)))
            .map(|o| match self.pick(o).0 {
                Who::Disc(c) => c,
                Who::Mod(i) => self.mods[i].donor,
            })
            .collect();
        let c = (0..CHARS).find(|c| !taken.contains(c)).unwrap_or(0);
        let costume = self.next_free(q, Who::Disc(c), 0, false).unwrap_or(0);
        Player { cursor: GRID.iter().position(|g| g.0 == c).unwrap_or(0), costume, ..default() }
    }

    /// B258: once every human is ready, the open seats' COM cards (not ready) start on their automatic pick, for a
    /// human to keep (✕) or change; ○ goes back to the last pick readied (B355).
    fn preset_coms(&mut self) {
        let n = self.players();
        if (0..n).all(|q| self.seats[q].is_none() || self.players[q].ready) {
            for q in (0..n).filter(|&q| self.open(q) && !self.players[q].ready).collect::<Vec<_>>() {
                self.players[q] = self.auto_com(q);
            }
        }
    }

    /// B269 / B346: the computer player `dev` drives next (one 1P added, or an open seat's COM card) starts on the
    /// cell the last pick readied on, so ✕ over and over gives them all the same (e.g. all Random, or one character
    /// in its next free costumes).
    fn carry_pick(&mut self, dev: Dev) {
        if let (Some(q), Some(c)) = (self.driven(dev), self.last)
            && self.seats[q].is_none() && !self.players[q].ready
        {
            self.put(q, c);
            self.players[q].tab = self.last_tab;
            self.skip_taken();
        }
    }

    /// The local select done: on to the court and umpire once every card is ready.
    fn chars_done(&mut self, out: &mut Vec<Effect>) {
        self.preset_coms();
        if (0..self.players()).all(|q| self.players[q].ready) {
            self.go(Screen::Confirm, out);
        }
    }

    /// B247: on a phone 1P is the screen (taps, or the on-screen pad), and the on-screen pad hides once a real one is in:
    /// a real pad's first press takes 1P over (on any screen, in the match too) instead of only joining as another
    /// player from the controller screen.
    fn pad_takes_touch_seat(&mut self, dev: Dev, pads: &[Entity]) {
        let touch = |s: Option<Dev>| matches!(s, Some(Dev::Keys)) || matches!(s, Some(Dev::Pad(e)) if !pads.contains(&e));
        if matches!(dev, Dev::Pad(_)) && !self.seats.contains(&Some(dev)) && touch(self.seats[0]) {
            self.seats[0] = Some(dev);
        }
        // B516: and the splash's 1P with it
        if matches!(dev, Dev::Pad(_)) && touch(self.p1) {
            self.p1 = Some(dev);
        }
    }

    /// B516: the splash's 1P a pad that's gone (unplugged or flat): the next device to press anything has it.
    fn takes_lost_p1(&mut self, dev: Dev, pads: &[Entity]) {
        if matches!(self.p1, Some(Dev::Pad(e)) if !pads.contains(&e)) && dev != Dev::Open {
            self.p1 = Some(dev);
        }
    }

    /// B388: 1P's pad gone (unplugged or flat) leaves the screens only 1P drives stuck: the keyboard's first press
    /// takes 1P over (from its own seat, which the missing pad gets instead, so plugged back in it drives that one), as
    /// does a pad no seat holds.
    fn takes_lost_1p(&mut self, dev: Dev, pads: &[Entity]) {
        if !matches!(self.seats[0], Some(Dev::Pad(e)) if !pads.contains(&e)) {
            return;
        }
        match (dev, self.seats.iter().position(|s| *s == Some(dev))) {
            (Dev::Keys, Some(q)) => self.seats.swap(0, q),
            (Dev::Keys | Dev::Pad(_), None) => self.seats[0] = Some(dev),
            _ => {}
        }
    }

    /// B238: whether seat `p` has a device that ✕ may take off: not this machine's first (the menu's driver), except
    /// on the assignment screen while another device is seated, which then moves up into it (changing 1P).
    fn droppable(&self, p: usize) -> bool {
        let dev = |s: &Option<Dev>| matches!(s, Some(Dev::Keys | Dev::Pad(_)));
        let first = self.seats.iter().position(Option::is_some) == Some(p);
        self.net_pause.is_none() && dev(&self.seats[p]) && (!first || self.paused || self.screen == Screen::Assign && self.seats.iter().filter(|s| dev(s)).count() > 1)
    }

    /// The device driving the menu's 1P screens: 1P's, except on the controller assignment after it moved its own
    /// seat there (B437: still the one that came in, until it leaves).
    fn lead(&self) -> Option<Dev> {
        self.assign_lead.filter(|_| self.screen == Screen::Assign).or(self.seats[self.first_seat()])
    }

    /// M124: the seat whose device drives the assignment: 1P's, over an online match this machine's first.
    fn first_seat(&self) -> usize {
        self.net_pause.and_then(|_| (0..self.players()).find(|&p| !self.bots[p])).unwrap_or(0)
    }

    /// Whether `dev` works the screens only 1P drives: 1P's device ([`lead`](Self::lead)); B516: on the mode and
    /// court screens (no seat of its own to move) the keyboard too, whoever has 1P.
    fn hosts(&self, dev: Dev) -> bool {
        self.lead() == Some(dev) || dev == Dev::Keys && self.lead().is_some() && matches!(self.screen, Screen::Mode | Screen::Confirm)
    }

    /// B437: the assignment's driver has moved off 1P (or off every seat): where it sits now (`None`: taken off), once
    /// it leaves the screen.
    fn lead_pending(&self) -> Option<Option<usize>> {
        let lead = self.lead()?;
        (self.seats[self.first_seat()] != Some(lead)).then(|| self.seats[..self.players()].iter().position(|s| *s == Some(lead)))
    }

    /// The seats 1P's ←/→ (`right`) trades on the assignment screen: the focused one and its neighbour that way,
    /// unless that would leave 1P empty or changes nothing.
    fn host_swap(&self, right: bool) -> Option<(usize, usize)> {
        self.swap_at(self.sel.min(self.players() - 1), right)
    }

    /// The seat the keyboard's ✕ takes on the assignment when it isn't the driver: the focused one (the mouse's hover
    /// moves it) if it's free, or 1P's when a pad has it (the keyboard and mouse may always take 1P over).
    fn keys_target(&self, seated: Option<usize>) -> Option<usize> {
        let q = self.sel.min(self.players() - 1);
        let mine = !(self.online() && q >= self.local_max());
        let free = self.open(q) && mine;
        let pad_1p = q == self.first_seat() && matches!(self.seats[q], Some(Dev::Pad(_))) && self.net_pause.is_none_or(|r| !r[q]);
        (seated != Some(q) && (free || pad_1p)).then_some(q)
    }

    /// Whether 1P's ✕ picks up seat `p`'s controller: a device on it that can move at least one way.
    fn pickable(&self, p: usize) -> bool {
        matches!(self.seats[p], Some(Dev::Keys | Dev::Pad(_))) && (self.swap_at(p, false).is_some() || self.swap_at(p, true).is_some())
    }

    /// [`host_swap`](Self::host_swap) from seat `p`.
    fn swap_at(&self, p: usize, right: bool) -> Option<(usize, usize)> {
        let n = self.players();
        let next = |q: usize| if right { (q + 1) % n } else { (q + n - 1) % n };
        let mut q = next(p);
        // B254: over a match the computer's seats stay put
        if self.paused {
            if self.bots[p] {
                return None;
            }
            while self.bots[q] {
                q = next(q);
            }
        }
        let f = self.first_seat();
        let empties_1p = q == f && self.seats[p].is_none() || p == f && self.seats[q].is_none();
        (!empties_1p && self.seats[p] != self.seats[q]).then_some((p, q))
    }

    /// Leaves the assignment: `keep` on to the character select (over a match, back to it with the new seats),
    /// else back with the seats as they were (`host`, B437: the driver, as 1P).
    fn assign_leave(&mut self, keep: bool, host: Option<Dev>, out: &mut Vec<Effect>) {
        if keep {
            for p in &mut self.players {
                *p = Player::fresh();
            }
            self.scroll = [0; 4];
            if self.online() {
                // B195: into the lobby first (seats held), then its character select
                self.online_open(out);
            } else {
                self.go(Screen::Chars, out);
            }
            return;
        }
        self.seats = [host, None, None, None];
        self.bots = [false; 4];
        self.screen = match self.net.go {
            Some(online::Go::Find) => Screen::Lobbies,
            Some(online::Go::Code) => Screen::Code,
            _ => Screen::Mode,
        };
        out.push(Effect::Sound(1));
    }

    /// B238: seat `p`'s device off it: free in the assignment, held open on the select for another device to join.
    fn drop_seat(&mut self, p: usize) -> bool {
        if !self.droppable(p) {
            return false;
        }
        if p == 0 && self.screen == Screen::Assign && !self.paused {
            // the next seated device becomes 1P (the menu's driver)
            let n = self.players();
            self.seats[..n].rotate_left(1);
            self.seats[n - 1] = None;
            let q = self.seats[..n].iter().position(Option::is_some).unwrap_or(0);
            self.seats[..n].swap(0, q);
            return true;
        }
        // B331: on a local roster the seat goes back to the computer, open to the next ✕
        self.seats[p] = if self.screen == Screen::Chars && !self.local_roster() { Some(Dev::Open) } else { None };
        self.unready(p);
        // a local roster stays doubles when a device leaves: only a third device joining changes the mode on its own
        true
    }

    /// The seats with a device on them (this machine's humans).
    fn humans(&self) -> usize {
        self.seats[..self.players()].iter().filter(|s| matches!(s, Some(Dev::Keys | Dev::Pad(_)))).count()
    }

    /// B361: whether a device not seated can join the select with ✕: a free seat, or a local singles roster's third
    /// device, which turns it into doubles.
    fn can_join(&self) -> bool {
        self.join_seat().is_some() || self.local_roster() && !self.doubles
    }

    /// B375: whether `dev` may swap Singles ↔ Doubles by hand: 1P's device on a local roster (B361 took it from
    /// everyone; the other players still only change it by joining and leaving).
    fn can_swap(&self, dev: Dev) -> bool {
        self.local_roster() && self.seats[0] == Some(dev)
    }

    /// Local doubles: a seated human other than 1P switches team with □ / Tab, onto the other team's first computer
    /// seat (teams are seats 0–1 and 2–3); their pick goes with them and the computer's takes their old seat.
    fn team_switch(&self, dev: Dev) -> Option<(usize, usize)> {
        if !(self.local_roster() && self.doubles) {
            return None;
        }
        let p = (1..4).find(|&p| self.seats[p] == Some(dev))?;
        (if p < 2 { 2..4 } else { 0..2 }).find(|&q| self.seats[q].is_none()).map(|q| (p, q))
    }

    /// B449: 1P may swap Singles ↔ Doubles on the confirm screen: a local match, and singles only while at most two
    /// people play.
    fn confirm_swap(&self) -> bool {
        !self.online() && (!self.doubles || self.humans() <= 2)
    }

    /// B331: the local roster (not online): devices join it with ✕; B361: its mode follows them
    /// (singles for one or two devices, doubles from the third).
    fn local_roster(&self) -> bool {
        self.screen == Screen::Chars && !self.online()
    }

    /// The seat a device's ✕ on the select takes: one held open for a device (B238), else on a local roster the
    /// first computer player's (B331).
    fn join_seat(&self) -> Option<usize> {
        let n = self.players();
        (0..n).find(|&p| self.seats[p] == Some(Dev::Open)).or_else(|| (0..n).find(|&p| self.local_roster() && self.seats[p].is_none()))
    }

    /// B331: a local match goes straight to the roster (Smash style): `dev` is 1P, every other seat a computer player
    /// until a device joins with ✕; the Mods page's Play with puts its mod on 1P's cursor.
    fn local_start(&mut self, dev: Dev, out: &mut Vec<Effect>) {
        self.rematch = false;
        self.doubles = false;
        self.seats = [Some(dev), None, None, None];
        self.bots = [false; 4];
        self.players = [Player::fresh(); 4];
        self.scroll = [0; 4];
        self.go(Screen::Chars, out);
    }

    /// B331: Singles ↔ Doubles on the roster (B361: on its own, as devices join and leave). 1P's opponent stays
    /// across the net (singles' 2P is doubles' 3P); going to singles keeps 1P and the first other human (opponents
    /// first). Picks follow their seats; `keep`: a human's ready pick stays ready, else every card picks again.
    fn swap_mode(&mut self, keep: bool) {
        let (seats, players) = (self.seats, self.players);
        let from = if self.doubles {
            let o = [2, 3, 1].into_iter().find(|&q| seats[q].is_some()).unwrap_or(2);
            [Some(0), Some(o), None, None]
        } else {
            [Some(0), None, Some(1), None]
        };
        self.doubles = !self.doubles;
        for (p, q) in from.into_iter().enumerate() {
            self.seats[p] = q.and_then(|q| seats[q]);
            let ready = keep && q.is_some_and(|q| seats[q].is_some() && players[q].ready);
            self.players[p] = Player { ready, ..q.map_or_else(Player::fresh, |q| players[q]) };
        }
        self.focus = 0;
    }

    /// One device's presses on the current screen.
    fn step(&mut self, dev: Dev, k: Press) -> Vec<Effect> {
        use Effect::*;
        let mut out = Vec::new();
        // B437: left the assignment: its seats stand, 1P's device drives from here
        if self.screen != Screen::Assign {
            (self.assign_lead, self.held, self.assign_was, self.keep) = (None, false, None, None);
        } else if self.assign_was.is_none() {
            self.assign_was = Some((self.seats, self.bots));
        }
        // M127: the pad takeover prompt takes every press while it's up
        if let Some(fx) = self.takeover_step(dev, k) {
            return fx;
        }
        let rows = |n: usize, sel: &mut usize, out: &mut Vec<Effect>| {
            if k.up || k.down {
                *sel = if k.down { (*sel + 1) % n } else { (*sel + n - 1) % n };
                out.push(Sound(0));
            }
        };
        // a lobby link's start (`online::linked`): the first device to press anything is this machine's player
        if self.screen == Screen::Press {
            if self.splash_step(dev, k.ok) {
                out.push(Effect::Sound(2));
            }
            return out;
        }
        if self.screen == Screen::Assign && self.online() && self.seats[0].is_none() {
            self.seats[0] = Some(dev);
            self.p1 = Some(dev);
            // B392: and straight into the lobby, no controllers screen
            self.online_enter(&mut out);
            return out;
        }
        // B516: the splash's 1P (and the keyboard) works the menus; another pad only joins the screens that seat players
        // B558: the pages opened over them too, by any seated device
        let joins = matches!(self.screen, Screen::Assign | Screen::Chars | Screen::Confirm | Screen::Lobby) || self.over.is_some();
        if !self.paused && !joins && matches!(dev, Dev::Pad(_)) && self.p1.is_some_and(|p| p != dev) {
            return out;
        }
        let host = self.lead();
        if self.screen == Screen::Main && (self.ask_step(k, &mut out) || self.stale_step(dev, k, &mut out)) {
            return out;
        }
        // B484: the other side's mod asked about over the select / the confirm (drawn over everything)
        if self.mod_ask_step(k, &mut out) {
            return out;
        }
        if self.switch_step(k, &mut out) {
            return out;
        }
        if self.settings_step(dev, k, &mut out) {
            return out;
        }
        match self.screen {
            Screen::Main => {
                let main = self.main_rows();
                rows(main.len(), &mut self.sel, &mut out);
                // B374: ○ on the update notice: not again this run (the row above takes the hand)
                if k.back && main.get(self.sel) == Some(&home::Row::Update) {
                    crate::update::later();
                    if crate::update::latest().is_none() {
                        self.sel = self.sel.saturating_sub(1);
                        out.push(Sound(1));
                    }
                } else if k.back {
                    // B533: back to the splash, which gives 1P up to the next press
                    self.splash_back();
                    out.push(Sound(1));
                    return out;
                }
                if k.ok {
                    use home::Row;
                    match main.get(self.sel).copied().unwrap_or(Row::Local) {
                        Row::Rematch => {
                            // the same setup; a random court or umpire is drawn again, as Start does
                            self.roll = std::hash::BuildHasher::hash_one(&std::hash::RandomState::new(), 0u8);
                            out.extend([Sound(2), Launch]);
                            self.screen = Screen::Playing;
                        }
                        // B516: whoever chose it, 1P is the splash's
                        Row::Local => self.local_start(self.p1.unwrap_or(dev), &mut out),
                        Row::Online if !self.stale.1 && crate::update::latest().is_some() => {
                            self.stale = (Some(0), true);
                            out.push(Effect::Sound(2));
                        }
                        Row::Online => self.go_online(dev, &mut out),
                        Row::Settings => self.go(Screen::Display, &mut out),
                        Row::Mods => self.mods_open(&mut out),
                        Row::Extras => self.go(Screen::Extras, &mut out),
                        Row::Update => crate::update::act(),
                        #[cfg(target_arch = "wasm32")]
                        Row::Quit => crate::web::launcher(),
                        #[cfg(not(target_arch = "wasm32"))]
                        Row::Quit => out.push(Quit),
                    }
                }
            }
            Screen::Display | Screen::Graphics | Screen::Advanced | Screen::Sound | Screen::ModSettings | Screen::Mobile => {
                if self.editing.is_none() {
                    rows(options::page(self.screen).len(), &mut self.sel, &mut out);
                }
                self.step_page(k, &mut out);
            }
            Screen::Controls => {
                if self.rebinding.is_some() {
                    return out;
                }
                // the last row resets every binding (a stick on a phone's pad can't be bound back)
                let n = controls::action_names().count();
                rows(n + 1, &mut self.sel, &mut out);
                if k.kind_prev || k.kind_next {
                    // B291: L2 / R2 (1 / 3) the other tab, as the Mods page's kinds
                    self.controls_tab(!self.keys_tab, &mut out);
                } else if k.ok && self.sel == n {
                    out.extend([ResetControls, Sound(2)]);
                } else if k.ok {
                    self.rebinding = Some(self.sel);
                    out.push(Sound(2));
                } else {
                    self.step_tabs(k, &mut out);
                }
            }
            Screen::Mode if self.hosts(dev) => {
                if k.up || k.down || k.left || k.right {
                    self.doubles = !self.doubles;
                    out.push(Sound(0));
                }
                if k.ok {
                    self.bots = [false; 4];
                    self.go(Screen::Assign, &mut out);
                } else if k.back && self.online() {
                    self.back_to_online(&mut out);
                } else if k.back {
                    self.sel = self.main_at(home::Row::Local);
                    self.screen = Screen::Main;
                    out.push(Sound(1));
                }
            }
            Screen::Assign => {
                let n = self.players();
                let seated = self.seats[..n].iter().position(|s| *s == Some(dev));
                if host == Some(dev) {
                    let p = self.sel.min(n - 1);
                    self.held &= self.pickable(p);
                    if let Some(a) = self.keep {
                        // over a match, ○ after a change: Keep / Undo (○ again stays on the screen)
                        if k.up || k.down || k.left || k.right {
                            self.keep = Some(1 - a);
                            out.push(Sound(0));
                        } else if k.ok {
                            self.keep = None;
                            self.assign_leave(a == 0, host, &mut out);
                        } else if k.back {
                            self.keep = None;
                            out.push(Sound(1));
                        }
                    } else if k.ok && seated.is_none() && self.open(p) && !(self.online() && p >= self.local_max()) {
                        // a driver that gave up its seat takes the focused free one back
                        self.seats[p] = Some(dev);
                        out.push(Sound(2));
                    } else if k.ok && (self.held || self.pickable(p)) {
                        // ✕ picks up the focused seat's controller, or puts it down
                        self.held = !self.held;
                        out.push(Sound(if self.held { 2 } else { 1 }));
                    } else if k.ok && !self.paused || k.name {
                        // Start (a pad's Start, N), or ✕ on a seat with nothing to pick up before a match
                        self.assign_leave(true, host, &mut out);
                    } else if self.held && (k.left || k.right) {
                        // the held controller trades places with the next seat that way (1P is never left empty)
                        if let Some((p, q)) = self.host_swap(k.right) {
                            // B437: the swap is shown at once, but this device drives the screen until it leaves
                            self.assign_lead = host;
                            self.seats.swap(p, q);
                            if !self.paused {
                                self.bots.swap(p, q);
                            }
                            self.sel = q;
                            out.push(Sound(0));
                        }
                    } else if self.held && k.back {
                        self.held = false;
                        out.push(Sound(1));
                    } else if !self.held && (k.left || k.right || k.up || k.down) {
                        // the focus goes about the cards as they lie (two rows of two on a tall view)
                        let per = self.assign_per();
                        let (row, col) = (p / per, p % per);
                        self.sel = if k.left || k.right {
                            row * per + if k.right { (col + 1) % per } else { (col + per - 1) % per }
                        } else if per < n {
                            (p + per) % n
                        } else {
                            p
                        };
                        if self.sel != p {
                            out.push(Sound(0));
                        }
                    } else if k.page && self.drop_seat(p) {
                        out.push(Sound(1));
                    } else if k.card && self.can_bot(p) {
                        // B216: △ adds a computer player to the focused empty seat (or takes it off)
                        self.bots[p] = !self.bots[p];
                        out.push(Sound(if self.bots[p] { 2 } else { 1 }));
                    } else if k.back && self.paused && self.assign_was.is_some_and(|w| w != (self.seats, self.bots)) {
                        self.keep = Some(0);
                        out.push(Sound(2));
                    } else if k.back {
                        self.assign_leave(false, host, &mut out);
                    }
                } else if k.ok && dev == Dev::Keys && let Some(q) = self.keys_target(seated) {
                    // the keyboard's ✕ takes the seat the pointer / 1P's focus is on: a free one, or 1P's from a pad
                    // (which goes to the keyboard's old seat, else the first free one, else off)
                    let bumped = self.seats[q].take();
                    if let Some(s) = seated {
                        self.seats[s] = bumped;
                    } else if let Some(f) = (0..n).find(|&f| self.open(f) && f != q && !(self.online() && f >= self.local_max())) {
                        self.seats[f] = bumped;
                    }
                    self.seats[q] = Some(dev);
                    self.assign_lead = None;
                    self.held = false;
                    out.push(Sound(2));
                } else if k.ok && seated.is_none() {
                    let n = if self.online() { self.local_max() } else { n };
                    if let Some(p) = (0..n).find(|&p| self.seats[p].is_none() && !self.bots[p]) {
                        self.seats[p] = Some(dev);
                        out.push(Sound(2));
                    }
                } else if (k.left || k.right || k.up || k.down) && let Some(p) = seated.filter(|&p| p > 0) {
                    // the next free seat that way (1P stays the host's)
                    let d = if k.right || k.down { 1 } else { n - 2 };
                    let to = (1..n - 1).map(|j| 1 + (p - 1 + d * j) % (n - 1)).find(|&q| self.seats[q].is_none() && !self.bots[q] && !(self.online() && q >= self.local_max()));
                    if let Some(q) = to {
                        self.seats.swap(p, q);
                        out.push(Sound(0));
                    }
                } else if k.back && let Some(p) = seated.filter(|&p| p > 0) {
                    // B437: (the device the driver moved onto 1P stays there; the driver can take it off)
                    self.seats[p] = None;
                    out.push(Sound(1));
                }
            }
            Screen::Chars => {
                // B361: a third device on a local singles roster makes it doubles (and takes 1P's partner's seat)
                if k.ok && !self.seats.contains(&Some(dev)) && self.join_seat().is_none() && self.can_join() {
                    self.swap_mode(true);
                }
                // B238: a device not seated takes the first seat held open for one
                if k.ok && !self.seats.contains(&Some(dev)) && let Some(p) = self.join_seat() {
                    self.seats[p] = Some(dev);
                    self.unready(p);
                    out.push(Sound(1));
                    return out;
                }
                let Some(p) = self.driven(dev) else { return out };
                self.focus = p;
                // B375: a seated pad's Start is the name picker's, not ✕
                let k = Press { ok: k.ok && !k.name, ..k };
                // B371: Start the name picker for the device's own seat; while open it takes the presses
                let np = (0..self.players()).find(|&q| self.seats[q] == Some(dev) && self.naming(q)).unwrap_or(p);
                if self.name_step(np, dev, k, &mut out) {
                    return out;
                }
                // B392: online, the host's 1P swaps the lobby's
                if k.page && self.online_swap(dev, &mut out) {
                    return out;
                }
                // B522: a guest's moves its player to the other team
                if k.page && self.online_team_step(dev, &mut out) {
                    return out;
                }
                // B375: 1P's device (only) swaps Singles ↔ Doubles with □ / Tab (or a tap on the mode pill)
                if k.page && self.can_swap(dev) {
                    self.swap_mode(true);
                    out.push(Sound(0));
                    return out;
                }
                // the other humans' □ / Tab: the other team (doubles)
                if k.page && let Some((p, q)) = self.team_switch(dev) {
                    self.seats.swap(p, q);
                    self.players.swap(p, q);
                    self.scroll.swap(p, q);
                    out.push(Sound(0));
                    return out;
                }
                let costumes = self.costumes(self.who(p));
                let costume_to = |back: bool| self.next_free(p, self.who(p), (self.players[p].costume + if back { costumes - 1 } else { 1 }) % costumes, back);
                let (next_c, prev_c) = (costume_to(false), costume_to(true));
                if k.hand && !self.players[p].ready {
                    self.players[p].swap = !self.players[p].swap;
                    out.push(Sound(0));
                }
                // B369 / B375: △ a favorite (not Random's); B382: the moves and L2 / R2 on the grid
                if k.card && !self.players[p].ready && !self.players[p].random {
                    self.toggle_favorite(p, &mut out);
                }
                if !self.players[p].ready && self.grid_move(p, k) {
                    self.follow(p);
                    out.push(Sound(0));
                }
                if !self.players[p].ready && (k.prev || k.next) {
                    // M1n / B66: past a costume that failed to load or that someone else is ready on
                    if let Some(c) = if k.next { next_c } else { prev_c } {
                        self.players[p].costume = c;
                    }
                    out.push(Sound(0));
                }
                // B66: landing on a taken costume moves on to the next free one
                self.skip_taken();
                if k.ok && !self.players[p].ready {
                    if (self.players[p].random && !self.roll_random(p)) || self.blocked(p) {
                        out.push(Sound(1));
                    } else {
                        self.players[p].ready = true;
                        self.skip_taken();
                        out.push(Sound(2));
                        self.last = Some(self.cell(p));
                        self.last_tab = self.players[p].tab;
                        if self.online() && self.net.edit == Some(p) {
                            self.online_bot_ready(p, &mut out);
                        } else if self.online() && (0..self.players()).all(|q| self.seats[q].is_none() || self.players[q].ready) {
                            self.online_ready(&mut out);
                        } else {
                            self.chars_done(&mut out);
                            self.carry_pick(dev);
                        }
                    }
                } else if k.back {
                    if self.players[p].ready {
                        self.unready(p);
                        out.push(Sound(1));
                    } else if self.online() && self.net.edit == Some(p) {
                        // B216: the host leaves its computer player as it was in the lobby
                        self.net.edit = None;
                        self.go(Screen::Lobby, &mut out);
                        out.push(Sound(1));
                    } else if self.seats[p].is_none() {
                        // player 1 driving a computer player (one added or an open seat's COM card): back to the last
                        // one readied, never a pick for the computer (B355)
                        if let Some(q) = (0..self.players()).rev().find(|&q| self.players[q].ready) {
                            self.unready(q);
                        }
                        out.push(Sound(1));
                    } else if self.seats[p] == Some(dev) && self.drop_seat(p) {
                        // B238: a joined device's ○ takes it off its seat
                        out.push(Sound(LEAVE_SOUND));
                    } else if p == 0 && self.online() && self.net.first {
                        self.chars_back(&mut out);
                    } else if p == 0 && self.online() {
                        // online: back to the lobby, the seats still held
                        self.screen = Screen::Lobby;
                        out.push(Sound(1));
                    } else if p == 0 {
                        // B331: the local roster is the first screen past the main menu
                        self.seats[1..].fill(None);
                        self.sel = self.main_at(home::Row::Local);
                        self.screen = Screen::Main;
                        out.push(Sound(1));
                    }
                }
            }
            // B451: the umpire picker's rows
            Screen::Confirm if self.hosts(dev) && self.picking.is_some() && self.sel == 4 => umpire_picker::step(self, k, &mut out),
            Screen::Confirm if self.hosts(dev) && self.picking.is_some() => {
                // the pickers: the court and umpire strips' ←→ (Random first, stop at the ends); ✕ keeps, ○ puts back
                let d = k.right as i32 - k.left as i32;
                if self.sel == 1 {
                    // B450: the court picker's rows
                    court_picker::step(self, k, &mut out);
                } else if self.sel == 4 {
                    let order = strip_order(self);
                    let at = strip_at(self);
                    let to = (at as i32 + d).clamp(0, order.len() as i32 - 1) as usize;
                    if to != at {
                        self.umpire = order[to] as u8;
                        out.push(Sound(0));
                    }
                    if k.card && self.sel == 4 && self.umpire as usize != UMPIRES {
                        // the lob key (△): Preview Voice
                        let (program, key) = CALLS[self.calls % CALLS.len()];
                        self.calls += 1;
                        out.push(Hear(self.umpire, program, key));
                    }
                } else if d != 0 {
                    // B212: Sets and Games stop at the ends like the umpire cards (the original's ◀ ▶ go there)
                    let was = (self.umpire, self.sets, self.games);
                    let step = |v: usize, n: usize| (v as i32 + d).clamp(0, n as i32 - 1) as usize;
                    match self.sel {
                        2 => self.sets = step(self.sets, SETS.len()),
                        3 => self.games = GAMES[step(GAMES.iter().position(|&n| n >= self.games).unwrap_or(1), GAMES.len())],
                        _ => self.umpire = step(self.umpire as usize, UMPIRES + 1) as u8,
                    }
                    if was != (self.umpire, self.sets, self.games) {
                        out.push(Sound(0));
                    }
                }
                if k.ok {
                    self.picking = None;
                    out.push(Sound(2));
                    // B191: the umpire pick is the next confirm screen's (and the next run's) starting choice
                    if self.sel != 1 && self.settings.6 != self.umpire && self.umpire as usize <= UMPIRES {
                        self.settings.6 = self.umpire;
                        out.push(SaveSettings);
                    }
                } else if k.back {
                    let v = self.picking.take().unwrap_or_default();
                    match self.sel {
                        1 => self.court = v,
                        2 => self.sets = v,
                        3 => self.games = v as i32,
                        _ => self.umpire = v as u8,
                    }
                    out.push(Sound(1));
                }
                if self.picking.is_none() {
                    // opened from the lobby's settings row: back there
                    self.venue_picked(&mut out);
                }
            }
            Screen::Confirm if self.hosts(dev) => {
                // B449: the arrows go about the screen as its parts lie; Start (a pad's also sends ✕) starts
                let start = k.name || (k.ok && self.sel == 0);
                let k = Press { ok: k.ok && !k.name, ..k };
                let to = self.confirm_step(k);
                if to != self.sel {
                    self.sel = to;
                    out.push(Sound(0));
                }
                // B212: the court, Sets, Games and the umpire open their picker on ✕ (the value it puts back on ○)
                if k.ok && (1..CONFIRM_ROWS).contains(&self.sel) {
                    self.picking = Some([0, self.court, self.sets, self.games as usize, self.umpire as usize][self.sel]);
                    out.push(Sound(2));
                }
                if start {
                    // ponytail: std's per-call hasher keys as the dice (not the match's RNG, which this never touches)
                    self.roll = std::hash::BuildHasher::hash_one(&std::hash::RandomState::new(), 0u8);
                }
                if start && self.online() {
                    self.online_host(&mut out);
                } else if start {
                    out.extend([Sound(2), Launch]);
                    self.screen = Screen::Playing;
                    self.rematch = true;
                } else if k.page && self.confirm_swap() {
                    // B449: 1P's □ swaps Singles ↔ Doubles here too; new computer partners are picked on the select
                    self.swap_mode(true);
                    self.preset_coms();
                    self.sel = 0;
                    out.push(Sound(0));
                    if !(0..self.players()).all(|q| self.players[q].ready) {
                        self.screen = Screen::Chars;
                    }
                } else if k.page && !self.online() {
                    // doubles with more than two people here can't be singles
                    out.push(Sound(1));
                } else if (k.back || k.ok && self.sel >= CONFIRM_ROWS) && self.online() {
                    self.go(Screen::Lobby, &mut out);
                } else if k.back || k.ok && self.sel >= CONFIRM_ROWS {
                    // ○, or ✕ on a plate: back to the select
                    for p in 0..4 {
                        self.unready(p);
                    }
                    self.screen = Screen::Chars;
                    out.push(Sound(1));
                }
            }
            Screen::Online | Screen::Lobbies | Screen::Lobby | Screen::Code => self.online_step(k, &mut out),
            Screen::Mods => {
                // B383: the search box's ✕ types in place from the keyboard, a pad gets the keyboard on screen
                self.add.pad = matches!(dev, Dev::Pad(_));
                self.mods_step(k, &mut out)
            }
            Screen::Extras => self.extras_step(k, &mut out),
            _ => {}
        }
        out
    }

    fn go(&mut self, s: Screen, out: &mut Vec<Effect>) {
        self.screen = s;
        self.sel = 0;
        out.push(Effect::Sound(2));
    }

    /// The court and umpire the match gets: a random choice resolved by `roll`.
    fn venue(&self) -> (usize, u8) {
        (self.court_layout(self.court_pick()), self.umpire_donor(self.umpire_pick()) as u8)
    }

    /// The court value the match gets: Random drawn by `roll` from the disc courts and the court mods on the picker.
    fn court_pick(&self) -> usize {
        if self.court != COURTS {
            return self.court;
        }
        let all: Vec<usize> = (0..COURTS).chain(court_mods::values(self)).collect();
        all[(self.roll % all.len() as u64) as usize]
    }

    /// The umpire value the match gets: Random drawn by `roll` from the disc umpires and the umpire mods (B432: online
    /// only the disc ones, the umpires every peer has; peers don't announce their umpire mods as they do courts).
    fn umpire_pick(&self) -> usize {
        if self.umpire as usize != UMPIRES {
            return self.umpire as usize;
        }
        let all: Vec<usize> = (0..UMPIRES).chain(umpire_mods::values(self).filter(|_| !self.online())).collect();
        all[(self.roll / 0x1_0000 % all.len() as u64) as usize]
    }

    /// The match's flags (`pads`: each device's index among the sorted gamepads).
    fn args(&self, pads: &[Entity]) -> Vec<String> {
        let (court, umpire) = self.venue();
        let court = court + 1;
        // menu player → match player
        let order: &[usize] = if self.doubles { &[0, 2, 1, 3] } else { &[0, 1] };
        let mut chars = vec![0; order.len()];
        let mut outfits = vec![0; order.len()];
        let mut slot_mods = Vec::new();
        let mut swaps = vec!["0"; order.len()];
        for (p, &m) in order.iter().enumerate() {
            chars[m] = self.players[p].char();
            if self.players[p].swap {
                swaps[m] = "1";
            }
            let (who, costume) = self.pick(p);
            outfits[m] = costume;
            if let Who::Mod(i) = who {
                // the donor's number: the match's tables and voice draw go by it
                chars[m] = self.mods[i].donor;
                slot_mods.extend(["--slot-mod".to_string(), m.to_string(), self.mods[i].dir.display().to_string()]);
            }
        }
        let join = |v: &[usize]| v.iter().map(usize::to_string).collect::<Vec<_>>().join(",");
        let seat = |p: usize| match self.seats[p] {
            Some(Dev::Keys) => Some(Seat::Keys),
            Some(Dev::Pad(e)) => pads.iter().position(|x| *x == e).map(Seat::Pad),
            Some(Dev::Open) | None => None,
        };
        let seats: Vec<_> = (0..self.players()).map(seat).collect();
        let mut a = vec!["--play".to_string(), "--stage".into(), court.to_string(), "--court".into(), court.to_string()];
        if !self.doubles {
            a.push("--singles".into());
        }
        a.extend([
            "--chars".into(),
            join(&chars),
            "--outfits".into(),
            join(&outfits),
            "--umpire".into(),
            umpire.to_string(),
            "--sets".into(),
            ((SETS[self.sets] + 1) / 2).to_string(),
            "--games".into(),
            self.games.to_string(),
            "--pads".into(),
            Seat::flag(&seats),
        ]);
        a.extend(slot_mods);
        a.extend(self.umpire_mod_flags());
        a.extend(self.court_mod_flags());
        a.extend(self.name_flags(order));
        if swaps.contains(&"1") {
            a.extend(["--swap-hands".into(), swaps.join(",")]);
        }
        a.extend(self.settings.flags());
        a
    }
}

/// Direction repeat: the press, then after 0.4 s held every 0.1 s.
#[derive(Clone, Copy, Default)]
struct Repeat {
    held: f32,
    next: f32,
}

impl Repeat {
    fn step(&mut self, on: bool, dt: f32) -> bool {
        if !on {
            *self = Repeat::default();
            return false;
        }
        let first = self.held == 0.0;
        self.held += dt.max(1e-6);
        if first {
            self.next = 0.4;
            return true;
        }
        if self.held >= self.next {
            self.next += 0.1;
            return true;
        }
        false
    }
}

// ---------------------------------------------------------------- art

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    Solid,
    Font,
    /// The 3D pointing hand (`cursor.rs`).
    Cursor,
    /// `SHEETS[k]`.
    Sheet(usize),
    /// Player `p`'s 3D preview (`previews.rs`), in place of the portrait.
    Preview(usize),
    /// The hovered store mod's preview (`mods_screen::Shelf`).
    Shelf,
    /// B372: a Browse tile's picture (`mods_screen::Shelf::thumbs[slot]`).
    Thumb(u32),
    /// A character's face icon on the select ([`Icons`]; a disc one not baked yet draws its `FACES` square).
    Icon(Who),
    /// The main screen's face icons: [`Icons`]' only, nothing drawn until it's baked (no sheet square standing in).
    Face(Who),
    /// A plain fill (as `Solid`) drawn only once that face is baked: covers what stands in for it until then (the
    /// main screen's Mods tiles' initials).
    Cover(Who),
    /// Our button pill's layers (`menu_art::pill_layers`).
    Pill,
    /// B362: a style's ✕ (the keyboard's Enter) in the legend's art (`legend::Buttons`), whole.
    Button(legend::Style),
    /// B383: a style's kind-switch button `i` (0 back, 1 on: L2 / R2, the keyboard's 1 / 3) in the legend's art, whole.
    Glyph(legend::Style, u8),
    /// B400: umpire mod `k`'s picture (`umpire_mods::Pics`), in its pixels.
    Umpire(usize),
    /// B401: court mod `k`'s picture (`court_mods::Pics`), in its pixels.
    CourtMod(usize),
    /// B516: the splash's logo (`splash::Logo`), in its pixels.
    Logo,
    /// B541: seat `p`'s name as a tag (`names::NameArt`), whole.
    Name(usize),
}

/// B133: the select's face icons (B160: the disc grid's too), as `previews.rs` reads them.
#[derive(Resource, Default)]
struct Icons(std::collections::HashMap<Who, Handle<Image>>);

/// Z21: only the sheets something draws (our titles, rows and cards replaced the disc's title bars, option and select
/// sheets, the shots pictures, `info`, `Confirmation_01`/`04` and `KeyAssign_CharacterSelect2`; the umpire picker's
/// cards and tiles the umpire screen's wall, `Confirmation_03` and `item_umpireNN` stills).
const SHEETS: [(&str, &str); 3] = [
    ("MENU/MENU05.XB0", "wall_playerselect"),
    ("MENU/MENU12B.XB0", "characterselect2_01"),
    ("MENU/MENU12B.XB0", "characterselect2_04"),
];
/// Every screen's wall (`SHEETS` index).
const WALL: usize = 0;
/// The disc grid's faces sheet (`SHEETS` index): drawn for a face icon not baked yet.
const FACES: usize = 1;
/// The △ attribute card: title box, play-style titles, stat labels and the A–F letters.
const ATTRS: Tex = Tex::Sheet(2);
/// After `SHEETS`: `Confirmation_00`/`02`: round umpire faces and balls; court tiles (the "?" one).
const EXTRA: usize = SHEETS.len();
const C00: Tex = Tex::Sheet(EXTRA);
const C02: Tex = Tex::Sheet(EXTRA + 1);
/// B192: court `k`'s picture (`MENU/WALL/COURTnn.XB` → `Confirmation_Courtnn.tm2`, 568×256), the original's confirm
/// screen backdrop.
const fn court_pic(k: usize) -> Tex {
    Tex::Sheet(EXTRA + 2 + k)
}
/// The "?" court tile on `C02` (a random court; B217's Random card).
const RANDOM_TILE: [f32; 4] = [96.0, 360.0, 92.0, 68.0];
/// Its inside, within the 2-px dark rim; the "?" spans rows 11..55 of the tile, its middle at 33.
const RANDOM_INNER: [f32; 4] = [99.0, 363.0, 86.0, 62.0];

/// [`RANDOM_INNER`] cropped (never stretched) to `[w, h]`'s shape, kept on the "?" where it cuts rows.
fn random_cover([w, h]: [f32; 2]) -> [f32; 4] {
    let [u, v, iw, ih] = RANDOM_INNER;
    let [cx, _, cw, ch] = cover([iw, ih], [w, h]);
    let cy = (RANDOM_TILE[1] + 33.0 - ch / 2.0).clamp(v, v + ih - ch);
    [u + cx, cy, cw, ch]
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
    /// Tiled across `dst` at the screen's scale.
    tile: bool,
    /// Where it goes on a wide window.
    anchor: Anchor,
    /// Turned clockwise about its centre (radians).
    angle: f32,
    /// Its corners rounded off (in 640×448 units: top-left, top-right, bottom-right, bottom-left), a picture cut to
    /// the round box it sits in.
    round: [f32; 4],
    /// Drawn as an outline this thick (640×448 units) in its colour round an empty middle, not filled.
    rim: f32,
}

/// The disc's text and tables the screens show.
struct Text {
    msg: Vec<String>,
    /// Each glyph's cell and ink span in `word.tm2`.
    glyphs: std::collections::HashMap<char, (f32, f32, f32)>,
    grades: [[u8; 5]; 14],
    /// Play style (0 all-round, 1 baseline, 2 net, 3 big server) by character.
    style: [usize; 14],
    /// Hand by character: +1 right, −1 left (TParam.csv's 利き腕, right only when exactly 右).
    hands: [f32; 14],
    /// AIParam.csv's rank (0 "Tennis Lv 5" … 16 "Tennis Superstar") by row: a computer player's label.
    ranks: Vec<u8>,
    /// The custom roster's play style and △ grades (`mod_grades`).
    mods: Vec<(usize, [u8; 5])>,
}

/// The TParam.csv column each △ grade follows (Serve, Stroke, Volley, Footwork = speed); Impact has none.
const GRADE_COLS: [Option<usize>; 5] = [Some(12), Some(13), Some(14), None, Some(40)];

/// A mod's △ grades, so its card is the disc characters' card: its base character's, except where its row changes a
/// `GRADE_COLS` cell: there the mean grade of the disc characters nearest in that column.
/// ponytail: the disc's grades are hand-set (not a function of TParam), so nearest-neighbour is a stand-in.
fn mod_grades(row: &[String], base: usize, csv: &[u8], grades: &[[u8; 5]; 14]) -> [u8; 5] {
    let mut disc: [Vec<f32>; 14] = Default::default();
    for line in String::from_utf8_lossy(csv).lines() {
        let cells: Vec<&str> = line.split(',').map(str::trim).collect();
        if let Some(c) = cells[0].parse::<usize>().ok().filter(|&c| c < 14) {
            disc[c] = cells.iter().map(|v| v.parse().unwrap_or(f32::NAN)).collect();
        }
    }
    let cell = |r: &[f32], c: usize| r.get(c).copied().unwrap_or(f32::NAN);
    std::array::from_fn(|k| {
        let Some(col) = GRADE_COLS[k] else { return grades[base][k] };
        let Some(v) = row.get(col).and_then(|v| v.parse::<f32>().ok()) else { return grades[base][k] };
        if v == cell(&disc[base], col) {
            return grades[base][k];
        }
        let d = |c: usize| (cell(&disc[c], col) - v).abs();
        let best = (0..14).map(d).filter(|d| !d.is_nan()).fold(f32::INFINITY, f32::min);
        let near: Vec<u32> = (0..14).filter(|&c| d(c) == best).map(|c| grades[c][k] as u32).collect();
        if near.is_empty() { grades[base][k] } else { ((near.iter().sum::<u32>() * 2 + near.len() as u32) / (2 * near.len() as u32)) as u8 }
    })
}

/// A mod's play style: its base row's, or its `タイプ` override's.
fn mod_style(m: &crate::mods::Mod, base: &[usize; 14]) -> usize {
    match m.overrides.iter().find(|(k, _)| k == "タイプ") {
        Some((_, v)) => ["オール", "ベース", "ネット", "ビッグ"].iter().position(|p| v.starts_with(p)).unwrap_or(0),
        None => base[m.base],
    }
}

impl Text {
    fn msg(&self, i: usize) -> &str {
        self.msg.get(i).map_or("", String::as_str)
    }
}

#[derive(Resource)]
struct Art(Vec<Handle<Image>>, Option<Arc<SoundBank>>, Text);
#[derive(Component)]
struct Slot(usize);

// four human cards on the select take ~760 (B156); the confirm screen needs more than 700 (B141a)
// B296: a full roster page (64 cells) with four players' cards takes ~2.5k
const POOL: usize = 3200;

/// `message0.dat`'s strings: a count, offsets from the end of the offsets, each string `07 7f` then Latin-1 to a NUL.
fn messages(d: &[u8]) -> Vec<String> {
    let u = |o: usize| d.get(o..o + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let base = 4 + 4 * u(0);
    (0..u(0))
        .map(|i| {
            let s = d.get(base + u(4 + 4 * i)..).unwrap_or(&[]);
            let s = s.strip_prefix(b"\x07\x7f").unwrap_or(s);
            s.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect()
        })
        .collect()
}

/// TParam.csv's type cell (column 7, Shift-JIS) as the select's label: オール, ベース, ネット, ビッグ (the mods' are
/// `mod_style`'s).
fn hands(csv: &[u8]) -> [f32; 14] {
    let mut out = [-1.0; 14];
    for line in csv.split(|&b| b == b'\n') {
        let cells: Vec<&[u8]> = line.split(|&b| b == b',').collect();
        let Some(c) = std::str::from_utf8(cells[0]).ok().and_then(|s| s.trim().parse::<usize>().ok()).filter(|&c| c < 14) else { continue };
        if cells.get(6).is_some_and(|h| h.trim_ascii() == [0x89, 0x45]) {
            out[c] = 1.0;
        }
    }
    out
}

fn styles(csv: &[u8]) -> [usize; 14] {
    let mut out = [0; 14];
    for line in csv.split(|&b| b == b'\n') {
        let cells: Vec<&[u8]> = line.split(|&b| b == b',').collect();
        let Some(c) = std::str::from_utf8(cells[0]).ok().and_then(|s| s.trim().parse::<usize>().ok()).filter(|&c| c < 14) else { continue };
        let t = cells.get(7).copied().unwrap_or(&[]);
        out[c] = [&b"\x83\x49"[..], b"\x83\x78", b"\x83\x6c", b"\x83\x72"].iter().position(|p| t.starts_with(p)).unwrap_or(0);
    }
    out
}

/// A computer player's rank label: its AIParam.csv row's rank, the row picked as the match picks it
/// (`play.rs ai_params`).
fn com_rank(ranks: &[u8], character: usize, costume: usize, doubles: bool) -> usize {
    let row = hst_sim::ai::Choice::new(hst_sim::ai::menu_row(character as u8, costume as u8) as u32, character as u8, doubles).row;
    ranks.get(row).map_or(0, |&r| r as usize)
}

/// `mods`' indices, newest installed first (its `mod.json`'s time; ties and no time keep folder order).
fn newest(mods: &[crate::mods::Mod]) -> Vec<usize> {
    let time = |m: &crate::mods::Mod| crate::modfs::size(&m.dir.join("mod.json")).and_then(|s| s.1);
    let mut v: Vec<(usize, Option<std::time::SystemTime>)> = mods.iter().map(time).enumerate().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.into_iter().map(|(i, _)| i).collect()
}

/// P57: the menu's start in three systems, each its own slice of the staged start (`mode.rs`), where one `setup`
/// took 410 ms of one frame in the browser: the sheets ([`read_sheets`]), the text tables, sounds and mod roster
/// ([`read_text`]), then the menu itself ([`setup`]).
#[derive(Resource)]
struct Sheets {
    art: Vec<Handle<Image>>,
    /// `word.tm2`, for its glyphs.
    font: Vec<u8>,
}

/// M113: the disc's TParam.csv, so the Mods screen can grade a mod the startup roster doesn't have ([`mod_card`]).
#[derive(Resource)]
struct TParam(Vec<u8>);

/// M113: mod `m`'s play style and △ grades as the select shows them (None: its TParam.csv row can't be read).
fn mod_card(m: &crate::mods::Mod, csv: &[u8], t: &Text) -> Option<(usize, [u8; 5])> {
    let row = crate::mods::tparam_row(csv, m).ok()?;
    Some((mod_style(m, &t.style), mod_grades(&row, m.base, csv, &t.grades)))
}

/// [`read_text`]'s mod roster and its folder, for [`setup`].
#[derive(Resource)]
struct Roster(Vec<crate::mods::Mod>, std::path::PathBuf);

/// The menu's sheets read from the disc (P57: drawn and decoded on the jobs, `menu_art::image`).
fn read_sheets(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let mut archives = std::collections::HashMap::new();
    let mut read = |iso: &mut Iso, xb: &str, name: &str| -> Vec<u8> {
        let arc = archives.entry(xb.to_string()).or_insert_with(|| iso.read(xb).expect("menu archive on disc"));
        let arc = Archive::parse(arc).expect("xb archive");
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name)).unwrap_or_else(|| panic!("{name} in {xb}"));
        arc.read(e).expect("archive bytes")
    };
    let mut art = vec![images.add(Image::new_fill(
        bevy::render::render_resource::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        &[255; 4],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ))];
    let font = read(&mut iso, "MENU/MENU04.XB0", "/word.tm2");
    art.push(super::menu_art::image(&mut images, &font, "word"));
    // Z1: read every sheet first so their replacements decode on all cores at once
    let mut sheets: Vec<(String, Vec<u8>)> = SHEETS
        .iter()
        .map(|(xb, name)| (name.to_string(), read(&mut iso, xb, &format!("/{name}.tm2"))))
        .collect();
    for name in ["confirmation_00", "confirmation_02"] {
        sheets.push((name.into(), read(&mut iso, "MENU/MENU03.XB0", &format!("/{name}.tm2"))));
    }
    // B192: each court's picture behind the confirm screen's players ([`court_pic`])
    for k in 0..COURTS {
        let name = format!("confirmation_court{k:02}");
        sheets.push((name.clone(), read(&mut iso, &format!("MENU/WALL/COURT{k:02}.XB"), &format!("/{name}.tm2"))));
    }
    let _pre = crate::textures::prefetch_tim2(sheets.iter().map(|s| s.1.as_slice()));
    art.extend(sheets.iter().map(|(name, d)| super::menu_art::image(&mut images, d, name)));
    commands.insert_resource(Sheets { art, font });
}

/// The menu's text tables, its sounds and the custom roster.
fn read_text(mut commands: Commands, args: Res<Args>, mut sheets: ResMut<Sheets>, mut images: ResMut<Assets<Image>>) {
    commands.remove_resource::<Sheets>();
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let read = |iso: &mut Iso, xb: &str, name: &str| -> Vec<u8> {
        let arc = iso.read(xb).expect("menu archive on disc");
        let arc = Archive::parse(&arc).expect("xb archive");
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name)).unwrap_or_else(|| panic!("{name} in {xb}"));
        arc.read(e).expect("archive bytes")
    };
    let mut art = std::mem::take(&mut sheets.art);
    let glyphs = glyphs(&sheets.font);
    let msg = messages(&read(&mut iso, "MENU/MENU00B.XB0", "/message0.dat"));
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/MENU.BIN").expect("MENU.BIN"));
    let grades = hst_data::exe::Menu::new(&cnf, &bin).expect("supported disc").grades();
    let pc = iso.read("PCDATA/PCDATA.XB").expect("character archive on disc");
    let pc = Archive::parse(&pc).expect("xb archive");
    let csv = pc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).and_then(|e| pc.read(e).ok()).unwrap_or_default();
    let bank = SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").map(Arc::new);
    let style = styles(&csv);
    let ai = pc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("aiparam.csv")).and_then(|e| pc.read(e).ok()).unwrap_or_default();
    let ranks = hst_sim::ai::AiParams::table(&ai).iter().map(|r| r.rank).collect();
    // the custom roster: a mod whose row can't be read is left out like a broken one
    let mods_dir = Settings::path(&args.iso).with_file_name("mods");
    mods_screen::set_root(mods_dir.clone());
    let roster: Vec<_> = crate::mods::list(&mods_dir)
        .into_iter()
        .filter_map(|m| match crate::mods::tparam(&mut iso, &m) {
            Ok(row) => Some(((mod_style(&m, &style), mod_grades(&row, m.base, &csv, &grades)), m)),
            Err(e) => {
                warn!("{}: {e}", m.dir.display());
                None
            }
        })
        .collect();
    info!("{} custom characters in {}", roster.len(), mods_dir.display());
    let (mod_text, mods) = roster.into_iter().unzip();
    art.push(super::menu_art::pill_layers(&mut images));
    commands.insert_resource(Art(art, bank, Text { msg, glyphs, grades, style, hands: hands(&csv), ranks, mods: mod_text }));
    commands.insert_resource(Roster(mods, mods_dir));
    commands.insert_resource(TParam(csv));
}

/// The menu: its settings, the screen it opens on, its quads.
fn setup(mut commands: Commands, args: Res<Args>, kept: Option<Res<Kept>>, mut roster: ResMut<Roster>) {
    commands.remove_resource::<Roster>();
    let (mods, mods_dir) = (std::mem::take(&mut roster.0), std::mem::take(&mut roster.1));
    let fresh = kept.is_none();
    // first run (no settings.txt): the Auto preset's pick for this device (P53; B351: a phone's is Phone or below it,
    // the browser on a touch screen too; a Steam Deck's its own)
    let first = || Settings(SETTING_DEFAULTS, auto_graphics(), Volumes::default(), default_aspect(), Ask::Each, 0, UMPIRES as u8, 0, 0, 0, Ask::Each);
    let saved = crate::store::read_to_string(Settings::path(&args.iso)).map(|t| {
        let mut s = Settings::parse(&t);
        // B289: an older build kept the lobby prompt's answer in the online mod cache
        if !t.contains("online_mods") {
            s.4 = Ask::load(&crate::mods_online::cache(&args.iso)).unwrap_or(s.4);
        }
        s
    });
    let mut menu = kept.map(|k| k.0.clone()).unwrap_or_else(|| Menu { settings: saved.as_ref().map_or_else(|_| first(), |s| *s), mods, ..default() });
    // M8f3: an umpire mod pick (not saved in settings.txt) stays for the menu a match comes back to
    let was = menu.umpire;
    menu.umpire = menu.settings.6;
    menu.umpire_mods = crate::mods_umpire::list(&mods_dir);
    if !fresh && was as usize > UMPIRES {
        menu.umpire = was;
    }
    menu.umpire_in_range();
    // M8f6: a court mod pick likewise (never saved)
    menu.court_mods = crate::mods_court::list(&mods_dir);
    // B425: `HST_NO_MODS=1` (for `--shot`s): the menu as with no mods installed (1P's Get more chip), whatever
    // `mods/` holds
    if std::env::var_os("HST_NO_MODS").is_some() {
        (menu.mods, menu.court_mods, menu.umpire_mods) = (Vec::new(), Vec::new(), Vec::new());
        menu.umpire_in_range();
    }
    menu.court_in_range();
    if fresh {
        menu.favs = tabs::load(&args.iso);
    }
    // B371: the names saved so far (another run may have added some)
    menu.names.saved = names::load(&args.iso);
    // back from a match: its pause menu may have changed them (B126)
    if let Ok(s) = saved {
        menu.settings = s;
    }
    menu.settings.1.cull = crate::cull::forced().unwrap_or(menu.settings.1.cull);
    commands.insert_resource(menu.settings.1);
    let screen = std::env::var("HST_MENU").unwrap_or_default();
    // B553: back from an online match into its lobby (`online.rs` `launch`'s way back): that lobby, as it was kept
    let lobby = !fresh && menu.screen == Screen::Lobby;
    menu.screen = match screen.as_str() {
        _ if lobby => Screen::Lobby,
        "settings" | "display" | "renderer" => Screen::Display,
        "graphics" | "graphics-list" | "graphics-fps" => Screen::Graphics,
        "advanced" => Screen::Advanced,
        "sound" | "sound-edit" => Screen::Sound,
        "mod-settings" => Screen::ModSettings,
        "mobile" => Screen::Mobile,
        "controls" | "controls-keys" | "controls-bind" => Screen::Controls,
        "mode" => Screen::Mode,
        "assign" | "assign-moved" => Screen::Assign,
        "chars" | "chars2" | "chars4" | "chars4-pads" | "roster1" | "roster2" | "roster3" | "roster4" | "roster2-favs" | "roster4-favs" | "roster2-deep" | "mods" | "attrs" | "mods-attrs" | "coms" | "coms-random" | "chars-online" | "chars-hosting" | "chars-joining" | "chars-relay" | "chars-random" | "names" | "names-type" | "names-card" => Screen::Chars,
        "confirm" | "confirm-random" | "confirm-court" | "courts" | "umpires" | "umpires-mod" | "confirm-umpire-mod" | "courts-mod" | "confirm-court-mod" | "sets" | "games" => Screen::Confirm,
        "online" | "online-custom" | "online-unread" => Screen::Online,
        "lobbies" | "lobbies-searching" | "lobbies-more" | "lobbies-none" | "lobbies-failed" => Screen::Lobbies,
        "lobby" | "lobby-ios" | "lobby-bot" | "lobby-move" | "lobby-kick" | "lobby-singles" | "lobby-courts" | "lobby-ask" | "lobby-wait" | "lobby-fetching" | "lobby-guest" | "lobby-hosting" | "lobby-joining" | "lobby-failed" | "lobby-relay" => Screen::Lobby,
        "code" | "name" | "relay-url" => Screen::Code,
        "addmods" | "browsemods" | "uploadmods" | "mymods" | "signinmods" | "phonemods" | "phonemods-ask" => Screen::Mods,
        "extras" => Screen::Extras,
        "press" | "press-claimed" => Screen::Press,
        // B516: a fresh start opens on the splash
        "" if fresh => Screen::Press,
        _ => Screen::Main,
    };
    if screen == "main-stale" {
        menu.stale = (Some(0), true);
    }
    if screen == "press-claimed" {
        menu.p1 = Some(Dev::Keys);
        menu.splash.hold();
    }
    if fresh && matches!(menu.screen, Screen::Press | Screen::Main) {
        online::linked(&mut menu, &args);
    }
    // B297: the main screen with a Rematch card (a singles match against the computer played)
    if fresh && (screen == "rematch" || screen == "rematch-update") {
        menu.rematch = true;
        menu.seats[0] = Some(Dev::Keys);
        menu.players[1].cursor = 9;
    }
    if fresh {
        menu.newest = newest(&menu.mods);
        mods_screen::prime(&mut menu);
    }
    if fresh && screen == "main-mods" {
        menu.sel = menu.main_at(home::Row::Mods);
    }
    // M97: Extras highlighted
    if fresh && screen == "main-extras" {
        menu.sel = menu.main_at(home::Row::Extras);
    }
    if fresh && (screen == "main-update" || screen == "rematch-update") {
        menu.sel = menu.main_at(home::Row::Update);
    }
    if fresh && screen == "reloadmods" {
        mods_screen::sample(&mut menu, &screen);
    }
    // M127: the pad takeover prompt up / its toast
    if fresh && (screen == "takeover" || screen == "takeover-toast") {
        takeover::sample(&mut menu, screen == "takeover-toast");
    }
    // B327: the Renderer switch's popup up, on WebGL2
    if fresh && screen == "renderer" {
        menu.switch = Some((2, 0));
    }
    // B498: Graphics with Preset's / Frame Limit's dropdown open
    if fresh && (screen == "graphics-list" || screen == "graphics-fps") {
        menu.sel = if screen == "graphics-fps" { 4 } else { 0 };
        menu.step(Dev::Keys, Press { ok: true, ..default() });
        menu.step(Dev::Keys, Press { down: true, ..default() });
    }
    if !fresh && menu.screen != Screen::Lobby {
        menu.after_match();
        pause::seats_back(&mut menu);
    } else if !lobby && menu.screen != Screen::Main {
        menu.seats[0] = Some(Dev::Keys);
        menu.doubles = true;
        // B216: 2P and 3P computer players added, 4P left open
        menu.bots = [false, true, true, false];
        menu.players[0].cursor = 6;
        menu.players[0].ready = screen.starts_with("coms");
        menu.players[0].random = screen == "chars-random";
        // B371: 1P named, its name picker open on the list or typing (`names`, `names-type`) or closed (`names-card`)
        if screen.starts_with("names") {
            menu.names.demo(0, Dev::Keys, screen == "names-type");
            // B541: picked, the picker closed: the name's tag on the card (`names-card`)
            if screen == "names-card" {
                menu.names.open[0] = None;
            }
        }
        // B437: doubles, the keyboard (1P) moved itself to 3P (pending until it leaves), a pad on 1P, 2P and 4P open
        if fresh && screen == "assign-moved" {
            menu.bots = [false; 4];
            menu.seats = [Some(Dev::Pad(Entity::from_raw_u32(1).unwrap())), None, Some(Dev::Keys), None];
            (menu.assign_lead, menu.sel) = (Some(Dev::Keys), 2);
        }
        // B264: 2P's computer readied on Random (`coms-random`, `confirm-random`)
        menu.players[1].random = matches!(screen.as_str(), "coms-random" | "confirm-random");
        menu.players[1].ready |= menu.players[1].random;
        // B64: the court / umpire picker open on Nakagawa River Courts / Lily, as the PCSX2 shots (`context/b64`)
        // B212: the Sets / Games pill picked
        if let Some(sel) = [("sets", 2), ("games", 3)].iter().find(|r| r.0 == screen).map(|r| r.1) {
            menu.sel = sel;
            menu.picking = Some(if sel == 2 { menu.sets } else { menu.games as usize });
        }
        // B291: the Controls page's Keyboard & Mouse tab; its third row waiting for a button
        menu.keys_tab = screen == "controls-keys";
        if screen == "controls-bind" {
            (menu.sel, menu.rebinding) = (2, Some(2));
        }
        // B326: a Settings row picked (Effects Volume)
        if screen == "sound-edit" {
            (menu.sel, menu.editing) = (1, Some(menu.settings));
        }
        if let Some(sel) = [("courts", 1), ("umpires", 4)].iter().find(|r| r.0 == screen).map(|r| r.1) {
            (menu.sel, menu.court, menu.umpire) = (sel, 8, 2);
            menu.picking = Some(if sel == 1 { 8 } else { 2 });
        }
        // M8f3: the umpire select / the confirm screen on the first umpire mod installed
        if matches!(screen.as_str(), "umpires-mod" | "confirm-umpire-mod") && !menu.umpire_mods.is_empty() {
            menu.umpire = UMPIRES as u8 + 1;
            if screen == "umpires-mod" {
                (menu.sel, menu.picking) = (4, Some(2));
            }
        }
        // M8f6: the court select / the confirm screen on the first court mod installed
        if matches!(screen.as_str(), "courts-mod" | "confirm-court-mod") && !menu.court_mods.is_empty() {
            menu.court = COURTS + 1;
            if screen == "courts-mod" {
                (menu.sel, menu.picking) = (1, Some(8));
            }
        }
        // B192: the confirm screen on Nakagawa River Courts and Lily, as the PCSX2 shots (`context/b192`)
        if screen == "confirm-court" {
            (menu.court, menu.umpire) = (8, 2);
        }
        if screen == "chars4" {
            // four humans on one portrait: their hands' offsets keep them apart (B156)
            menu.seats = [Some(Dev::Keys); 4];
            menu.players.iter_mut().for_each(|p| p.cursor = 6);
        }
        // B353: humans on the keyboard and pads (`chars2`: 1P keys, 2P a pad, ready; `chars4-pads`: 2P–4P pads), so
        // the shared legend shows both devices' glyphs
        if screen == "chars2" || screen == "chars4-pads" {
            let pad = |i: u32| Some(Dev::Pad(Entity::from_raw_u32(1_000_000 + i).unwrap()));
            let n = if screen == "chars2" { 2 } else { 4 };
            for p in 1..n {
                menu.seats[p] = pad(p as u32);
                menu.bots[p] = false;
                menu.players[p].cursor = 6;
            }
            // 2P readied on another character (Carol) than 1P's, so 1P's ✕ stays "Ready"
            if screen == "chars2" {
                menu.players[1].cursor = 7;
                menu.players[1].ready = true;
            }
        }
        // B368: 1 to 4 humans (`rosterN`: 1P keys, the rest pads; doubles past 2), a computer in the seats left,
        // each on their own character, 2P scrolled down its column
        if let Some(n) = screen.strip_prefix("roster").and_then(|n| n[..1].parse::<usize>().ok()) {
            let pad = |i: u32| Some(Dev::Pad(Entity::from_raw_u32(1_000_000 + i).unwrap()));
            menu.doubles = n > 2;
            menu.bots = [false; 4];
            for p in 0..menu.players() {
                menu.seats[p] = if p == 0 { Some(Dev::Keys) } else if p < n { pad(p as u32) } else { None };
                menu.bots[p] = p >= n;
                menu.players[p].cursor = [6, 9, 3, 12][p];
            }
            menu.players[menu.players() - 1].ready = n < menu.players();
        }
        // B382: `rosterN-favs`: four disc favorites (Favorites first), 1P on the second, 2P on the last section
        if screen.ends_with("-favs") {
            menu.favs = [0, 3, 5, 9].map(|c| format!("hst {c}")).into();
            let secs = menu.sections();
            menu.put_at(0, &secs, 1);
            menu.jump(1, &secs, secs.len() - 1);
        }
        mods_screen::sample(&mut menu, &screen);
        // B368: `roster2-deep`: 2P on a mod far down, its column scrolled there, 1P's at the top
        if screen == "roster2-deep" && menu.mods.len() > 40 {
            menu.put(1, Some(Who::Mod(40)));
        }
        // B306: the one roster, 1P on the first mod (scrolled to it)
        if screen.starts_with("mods") && menu.screen == Screen::Chars && menu.own_mods() > 0 {
            menu.put(0, Some(Who::Mod(0)));
        }
        if matches!(menu.screen, Screen::Online | Screen::Lobby | Screen::Lobbies | Screen::Code) || matches!(screen.as_str(), "chars-online" | "chars-hosting" | "chars-joining" | "chars-relay") {
            online::sample(&mut menu);
        }
    }
    commands.insert_resource(menu);
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    commands.spawn((full, GlobalZIndex(10))).with_children(|p| {
        for i in 0..POOL {
            p.spawn((Slot(i), ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, BorderColor::default(), Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
        }
        // the description bar's text over its bar (`ticker_text`)
        let clip = Node { position_type: PositionType::Absolute, overflow: Overflow::clip(), ..default() };
        p.spawn((TickerClip, clip, ZIndex(POOL as i32), Visibility::Hidden)).with_child((
            bevy::ui::widget::Text::new(""),
            TextColor(Color::srgb_u8(223, 223, 223)),
            TextLayout::no_wrap(),
            Node { position_type: PositionType::Absolute, ..default() },
        ));
    });
}

// ---------------------------------------------------------------- layout

struct Draw<'a> {
    out: Vec<Quad>,
    text: &'a Text,
    /// The description bar's text (`ticker_text` draws it).
    said: String,
    /// Where `q` puts its quads on a wide window.
    anchor: Anchor,
    /// B221: what the pointer can press (`pointer.rs`), and what a press outside them does.
    hits: Vec<pointer::Hit>,
    outside: Option<pointer::Act>,
}

const WHITE: [f32; 3] = [128.0; 3];
/// B141: our colours. Each screen's title and the colour of its bar (the wall's, as the disc's bars had it).
const TITLES: [(&str, [f32; 3]); 11] = [
    ("Main Menu", [102.0, 85.0, 25.0]),
    ("Settings", [110.0, 65.0, 75.0]),
    ("Game Selection", [75.0, 52.0, 85.0]),
    ("Character Selection", [102.0, 70.0, 40.0]),
    ("Confirm Settings", [55.0, 80.0, 42.0]),
    ("Mods", [110.0, 65.0, 75.0]),
    ("Lobby", [55.0, 80.0, 42.0]),
    ("Select Court", [55.0, 80.0, 42.0]),
    ("Online", [32.0, 67.0, 98.0]),
    ("Select Umpire", [55.0, 80.0, 42.0]),
    ("Extras", [102.0, 85.0, 25.0]),
];
/// Rims and outlines.
const INK: [f32; 3] = [15.0; 3];
/// B152: the titles' outline (the original's ring reads ~35..60 of 255 on a PCSX2 shot, the darkest blurred by the
/// upscale).
const RING: [f32; 3] = [20.0; 3];
/// Gold words: play styles, names, "Ready to go!".
const GOLD: [f32; 3] = [128.0, 112.0, 30.0];
/// The Singles and Doubles bars' fills.
const MODE_FILL: [[f32; 3]; 2] = [[110.0, 55.0, 80.0], [30.0, 92.0, 110.0]];
/// The player cards' fills (1P pink, 2P orange, 3P blue, 4P green), darker than [`COLOURS`]' words.
const CARD_FILL: [[f32; 3]; 4] = [[104.0, 52.0, 72.0], [104.0, 66.0, 26.0], [22.0, 46.0, 66.0], [38.0, 52.0, 18.0]];
const STYLE_NAMES: [&str; 4] = ["All-round Player", "Baseline Player", "Net Player", "Big Server"];
/// By character, as the disc's name sheet.
const CHAR_NAMES: [&str; CHARS] = ["Ashley", "Cody", "Jun", "Kaito", "Momoko", "JJ", "Carol", "Big Chief", "Miranda", "Kent", "Lola", "Will", "Gloria", "Suzuki"];
/// By rank, as `Confirmation_04`'s titles (nine to a column).
const RANK_NAMES: [&str; 17] = [
    "Tennis Lv 5",
    "Tennis Lv 4",
    "Tennis Lv 3",
    "Tennis Lv 2",
    "Tennis Lv 1",
    "Tennis Hopeful",
    "Tennis Tier II",
    "Tennis Tier III",
    "Tennis Tier IV",
    "Tennis Tier V",
    "Tennis Master",
    "Tennis Elite",
    "Tennis Guru",
    "Tennis Legend",
    "Tennis Hero",
    "Tennis King",
    "Tennis Superstar",
];

impl Draw<'_> {
    fn q(&mut self, tex: Tex, src: [f32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: f32) {
        self.out.push(Quad { tex, src, dst, rgb, alpha, tile: false, anchor: self.anchor, angle: 0.0, round: [0.0; 4], rim: 0.0 });
    }

    /// B221: `rect` (placed as `q` would place it) does `act` under the pointer.
    fn hit(&mut self, rect: [f32; 4], act: pointer::Act) {
        self.hits.push(pointer::Hit { rect, anchor: self.anchor, act });
    }

    /// B221: a popup is up: only what's drawn from here on takes the pointer, and a press outside it does `outside`.
    fn popup(&mut self, outside: Option<pointer::Act>) {
        self.hits.clear();
        self.outside = outside;
    }

    /// The screen's wall, tiled over the whole window.
    fn wall(&mut self, wall: usize) {
        self.out.push(Quad { tex: Tex::Sheet(wall), src: [0.0, 0.0, 32.0, 32.0], dst: [0.0, 0.0, 640.0, 448.0], rgb: WHITE, alpha: 128.0, tile: true, anchor: Anchor::Span, angle: 0.0, round: [0.0; 4], rim: 0.0 });
    }

    /// The screen's wall (tiled) and title bar.
    fn frame(&mut self, title: usize, wall: usize) {
        self.wall(wall);
        self.anchor = Anchor::Span;
        // B141, ours: the screen's colour along the top over a white and a dark rule, and a tab hanging from it with
        // the title in our font, its soft shadow down and right
        let (words, band) = TITLES[title];
        self.q(Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 14.0], band, 128.0);
        self.q(Tex::Solid, [0.0; 4], [0.0, 14.0, 640.0, 2.5], WHITE, 128.0);
        self.q(Tex::Solid, [0.0; 4], [0.0, 16.5, 640.0, 1.5], [15.0; 3], 128.0);
        let size = 30.0;
        let w = self.width(words, size);
        self.box3([-30.0, -30.0, w + 110.0, 70.0], 16.0, [Some([15.0; 3]), Some(WHITE), Some(band)], 128.0);
        let start = self.out.len();
        self.text(words, 42.0, 7.0, size, [0.0; 3]);
        self.out[start..].iter_mut().for_each(|q| q.alpha = 40.0);
        self.outlined(|d| d.text(words, 40.0, 5.0, size, WHITE));
        // B221: the title's tab is Back for the pointer
        self.hit([-30.0, -30.0, w + 110.0, 70.0], pointer::press(|k| k.back = true));
        self.anchor = Anchor::Centre;
    }

    fn width(&self, s: &str, size: f32) -> f32 {
        s.chars().map(|c| self.text.glyphs.get(&c).map_or(10.0, |g| g.2 + 2.0)).sum::<f32>() * size / 32.0
    }

    /// `s` in `word.tm2` at `size` px high from (x, y).
    fn text(&mut self, s: &str, x: f32, y: f32, size: f32, rgb: [f32; 3]) {
        let k = size / 32.0;
        let mut x = x;
        for c in s.chars() {
            match self.text.glyphs.get(&c) {
                Some(&(u, v, w)) => {
                    self.q(Tex::Font, [u, v, w, 32.0], [x, y, w * k, size], rgb, 128.0);
                    x += (w + 2.0) * k;
                }
                None => x += 10.0 * k,
            }
        }
    }

    fn centred(&mut self, s: &str, cx: f32, y: f32, size: f32, rgb: [f32; 3]) {
        let w = self.width(s, size);
        self.text(s, cx - w / 2.0, y, size, rgb);
    }

    /// `s` centred on `cx`, shrunk to fit `max` px wide.
    fn fitted(&mut self, s: &str, cx: f32, y: f32, size: f32, max: f32) {
        let k = (max / self.width(s, size).max(1.0)).min(1.0);
        self.centred(s, cx, y + size * (1.0 - k) / 2.0, size * k, WHITE);
    }

    /// A pill filled `rgb` in the mode pills' style: dark rim, white ring.
    fn pill(&mut self, x: f32, y: f32, w: f32, rgb: [f32; 3]) {
        self.pill3(x, y, w, [[30.0; 3], WHITE, rgb]);
    }

    /// A pill (our own, `menu_art::pill_layers`, three-sliced) with its rim, ring and fill each in its own colour.
    fn pill3(&mut self, x: f32, y: f32, w: f32, cols: [[f32; 3]; 3]) {
        self.pill_at([x, y, w, 38.0], cols, 128.0);
    }

    /// [`Self::pill3`] over any box (B141), at `alpha`.
    fn pill_at(&mut self, dst: [f32; 4], cols: [[f32; 3]; 3], alpha: f32) {
        for (i, rgb) in cols.into_iter().enumerate() {
            for (src, dst) in super::menu_art::pill_quads(i, dst) {
                self.q(Tex::Pill, src, dst, rgb, alpha);
            }
        }
    }

    /// Our rounded box (B141, `menu_art::pill_layers`' boxes, nine-sliced, `c`-px corners): rim, ring and fill each
    /// its own colour, or left out (`None`: a see-through fill, no ring).
    fn box3(&mut self, dst: [f32; 4], c: f32, cols: [Option<[f32; 3]>; 3], alpha: f32) {
        for (i, rgb) in cols.into_iter().enumerate() {
            let Some(rgb) = rgb else { continue };
            for (src, dst) in super::menu_art::box_quads(i, dst, c) {
                self.q(Tex::Pill, src, dst, rgb, alpha);
            }
        }
    }

    /// `s` centred on `cx` at `size`, squeezed to fit `max` px, tinted `rgb` (B141's labels in our font).
    fn label(&mut self, s: &str, cx: f32, y: f32, size: f32, max: f32, rgb: [f32; 3]) {
        let k = (max / self.width(s, size).max(1.0)).min(1.0);
        let w = self.width(s, size) * k;
        // squeezed across only: the words keep their height
        let start = self.out.len();
        self.text(s, cx - w / 2.0, y, size, rgb);
        let x0 = cx - w / 2.0;
        for q in &mut self.out[start..] {
            q.dst[0] = x0 + (q.dst[0] - x0) * k;
            q.dst[2] *= k;
        }
    }

    /// B152: the words `f` draws, over a dark ring as the original's titles (confirm screen's court and umpire, a
    /// PCSX2 shot): ~1.5 px round the glyphs, near black. Each glyph is stamped in [`RING`] eight ways round, then
    /// the fill on top; the glyphs are vector-set at `menu_art::K`×, so the ring stays sharp at any resolution.
    fn outlined(&mut self, f: impl FnOnce(&mut Self)) {
        let start = self.out.len();
        f(self);
        let fill: Vec<Quad> = self.out.drain(start..).collect();
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::FRAC_PI_4;
            for q in fill.iter().filter(|q| q.tex == Tex::Font) {
                let mut q = *q;
                q.dst[0] += 1.5 * a.cos();
                q.dst[1] += 1.5 * a.sin();
                q.rgb = RING;
                self.out.push(q);
            }
        }
        self.out.extend(fill);
    }

    /// A column of rows `pitch` px apart, the chosen one bright with the hand beside it, the rows `locked` says greyed out (B243): a faint dark pill and dim words, never the chosen
    /// row's gold even under the hand, so a switch the build keeps off reads as one that can't be pressed.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn rows_locked(&mut self, labels: &[String], sel: usize, x: f32, y: f32, w: f32, hand_x: f32, pitch: f32, locked: impl Fn(usize) -> bool) {
        for (i, l) in labels.iter().enumerate() {
            let yy = y + pitch * i as f32;
            let on = i == sel;
            if locked(i) {
                self.pill_at([x, yy, w, 38.0], [[30.0; 3], [60.0; 3], [40.0, 40.0, 44.0]], 64.0);
                self.centred(l, x + w / 2.0, yy + 7.0, 24.0, [50.0; 3]);
                self.hit([x, yy, w, 38.0], pointer::Act::Row(i, pointer::OK));
                continue;
            }
            self.pill(x, yy, w, if on { [128.0, 110.0, 40.0] } else { [70.0, 70.0, 80.0] });
            self.centred(l, x + w / 2.0, yy + 7.0, 24.0, if on { WHITE } else { [100.0; 3] });
            self.hit([x, yy, w, 38.0], pointer::Act::Row(i, pointer::OK));
        }
        self.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x - 60.0 + hand_x, y + pitch * sel as f32 + 3.0, 64.0, 32.0], WHITE, 128.0);
    }

    /// The description bar along the bottom.
    fn info(&mut self, s: &str) {
        self.anchor = Anchor::Span;
        self.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 404.0, 640.0, 32.0], [0.0; 3], 90.0);
        self.anchor = Anchor::Centre;
        self.said = s.into();
    }
}

/// B405: the controllers among the readable pads' names (gilrs' `Name`): the on-screen pad (web/touch.js,
/// `touch_pad.rs`) isn't one, the home screen shows it as the Touch seat.
fn controllers<'a>(names: impl Iterator<Item = Option<&'a str>>) -> usize {
    names.filter(|n| !n.is_some_and(super::emote::is_touch)).count()
}

/// A device's name on the assignment screen.
fn dev_name(d: Option<Dev>, pads: &[Entity]) -> String {
    match d {
        None => "COM".into(),
        Some(Dev::Keys) => "Keyboard".into(),
        Some(Dev::Pad(e)) => format!("Controller {}", pads.iter().position(|x| *x == e).map_or(0, |i| i + 1)),
        Some(Dev::Open) => "OK or Enter joins".into(),
    }
}

/// B237: `s` broken at spaces into lines of at most `cols` characters (a longer word gets a line to itself).
fn wrap(s: &str, cols: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for w in s.split_whitespace() {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + w.chars().count() <= cols => {
                l.push(' ');
                l.push_str(w);
            }
            _ => lines.push(w.into()),
        }
    }
    lines
}

/// B237: the description bar's glyph pitch (px) and lines for `s` on a `w` × `h` px bar: up to `cap` px a glyph,
/// shrunk 5% at a time until its word-wrapped lines (1.2 em each, the advance 0.6 em) fit the bar. No marquee.
fn fit_lines(s: &str, w: f32, h: f32, cap: f32) -> (f32, Vec<String>) {
    let mut p = cap;
    loop {
        let lines = wrap(s, (w / p).max(1.0) as usize);
        let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32;
        if (lines.len() as f32 * 2.0 * p <= h && longest * p <= w) || p < 1.0 {
            return (p, lines);
        }
        p *= 0.95;
    }
}

/// The description bar's first glyph x (640 units) `f` frames after its page opened: in from the bar's right edge
/// `right` at 32 a frame to x 16, as the MENU overlay brings it in; then it holds.
fn ticker_x_in(f: f32, right: f32) -> f32 {
    (right - 32.0 * f).max(16.0)
}

/// The description bar's clip rect; its child is the text.
#[derive(Component)]
struct TickerClip;

/// The description bar's text, frames (60 Hz, fractional) since its page opened, and that page.
#[derive(Resource, Default)]
struct Ticker(String, f32, Option<(Screen, mods_screen::Page)>);

/// The description bar's text, at the window's resolution in Fira Mono (Bevy's built-in font), in its tint (223), on
/// the bar (y 404..436). Its bar spans the window (`Anchor::Span`). B237: at most 10 units a glyph (the original's
/// was 11, and scrolled when it didn't fit); a longer text word-wraps and shrinks until it fits the bar. B521: the
/// glyphs' width and line height are taken as Fira Mono's (0.6 and 1.2 em) times `grow`, which goes up whenever the
/// text as laid out comes back bigger than that (another platform's font rendering), so it's never cut off.
fn ticker_text(
    ticker: Res<Ticker>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut clip: Query<(&mut Node, &mut Visibility, &Children), With<TickerClip>>,
    mut text: Query<(&mut bevy::ui::widget::Text, &mut TextFont, &mut Node, &ComputedNode), Without<TickerClip>>,
    mut grow: Local<Option<Grow>>,
) {
    let (Ok(win), Ok((mut cnode, mut vis, kids))) = (window.single(), clip.single_mut()) else { return };
    let Some(Ok((mut t, mut font, mut node, laid))) = kids.first().map(|&k| text.get_mut(k)) else { return };
    let g = grow.get_or_insert_with(Grow::default);
    g.measure(&t.0, laid.size() * laid.inverse_scale_factor());
    *vis = if ticker.0.is_empty() { Visibility::Hidden } else { Visibility::Inherited };
    let (share, vshare) = super::widescreen::window_menu_shares(&window);
    let (left, width) = super::widescreen::place(0.0, 640.0, share, Anchor::Span);
    (cnode.left, cnode.width) = (Val::Percent(left), Val::Percent(width));
    let (top, height) = super::widescreen::vplace_in(404.0, 32.0, vshare, Anchor::Span);
    (cnode.top, cnode.height) = (Val::Percent(top), Val::Percent(height));
    // one 640-space unit in logical px
    let vs = super::widescreen::view_size(win);
    let unit = share * vs.x / 640.0;
    let (bar_w, bar_h) = (width / 100.0 * vs.x, height / 100.0 * vs.y);
    // a tall narrow window (a phone held upright) has the height for a bigger glyph than its width's units give
    let cap = (10.0 * unit).max((7.0f32).min(bar_h / 4.0));
    // B325: 3 units clear above and below, so even two lines' descenders stay off the legend strip under the bar
    let (p, lines) = fit_lines(&ticker.0, (bar_w - 32.0 * unit) / g.w, (bar_h - 6.0 * unit) / g.h, cap / g.w.max(g.h));
    let em = p / 0.6;
    font.font_size = bevy::text::FontSize::Px(em);
    node.left = Val::Px(ticker_x_in(ticker.1, bar_w / unit + 16.0) * unit);
    node.top = Val::Px((bar_h - 1.2 * g.h * em * lines.len() as f32) / 2.0);
    let s = lines.join("\n");
    // what the layout should come back as, at Fira Mono's metrics
    let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32;
    g.sent = (s.clone(), Vec2::new(longest * p, 1.2 * em * lines.len() as f32));
    if t.0 != s {
        t.0 = s;
    }
}

/// B521: how much bigger than Fira Mono's metrics the description bar's text lays out (width, height; never below 1),
/// and the text last set with its size at those metrics (logical px).
struct Grow {
    w: f32,
    h: f32,
    sent: (String, Vec2),
}

impl Default for Grow {
    fn default() -> Self {
        Grow { w: 1.0, h: 1.0, sent: (String::new(), Vec2::ZERO) }
    }
}

impl Grow {
    /// The text node's laid-out `size` (logical px) for `shown`: once it's the text last set, the factors grow to
    /// cover it (a hair over, so rounding can't cut the last glyph).
    fn measure(&mut self, shown: &str, size: Vec2) {
        let (ref s, want) = self.sent;
        if shown != s || s.is_empty() || want.x <= 0.0 || want.y <= 0.0 || size.x <= 0.0 {
            return;
        }
        let (w, h) = (size.x / want.x, size.y / want.y);
        if w > 1.001 {
            self.w = self.w.max(w * 1.01).min(3.0);
        }
        if h > 1.001 {
            self.h = self.h.max(h * 1.01).min(3.0);
        }
    }
}

fn layout(m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity], hand_x: f32) -> (Vec<Quad>, String) {
    let d = drawn(m, text, bind, pads, pads.len(), hand_x);
    (d.out, d.said)
}

/// The screen's quads, words and hit rects. `controllers`: the pads plugged in, the on-screen pad not counted (B405).
fn drawn<'a>(m: &Menu, text: &'a Text, bind: &Bindings, pads: &[Entity], controllers: usize, hand_x: f32) -> Draw<'a> {
    let mut d = Draw { out: Vec::new(), text, said: String::new(), anchor: Anchor::Centre, hits: Vec::new(), outside: None };
    match m.screen {
        Screen::Press => splash::layout(&mut d, m, pads, controllers),
        Screen::Main => home::layout(&mut d, m, text, controllers, hand_x),
        Screen::Display | Screen::Graphics | Screen::Advanced | Screen::Sound | Screen::ModSettings | Screen::Mobile => options::page_layout(&mut d, m, hand_x),
        Screen::Controls => options::controls_layout(&mut d, m, bind, hand_x),
        Screen::Mode => {
            d.frame(2, WALL);
            // the whole pill, its dark rim included (it runs to the sheet's edges), at the same scale and place as before
            for (i, words) in ["Singles", "Doubles"].into_iter().enumerate() {
                let on = (i == 1) == m.doubles;
                mode_bar(&mut d, i, on, [131.0, 147.0 + 70.0 * i as f32, 378.0, 47.0], words);
                d.hit([131.0, 147.0 + 70.0 * i as f32, 378.0, 47.0], pointer::Act::Row(i, pointer::OK));
            }
            d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [76.0 + hand_x, 156.0 + 70.0 * m.doubles as u8 as f32, 64.0, 32.0], WHITE, 128.0);
            d.info(text.msg(192));
        }
        // M124: the match roster's cards (`assign.rs`)
        Screen::Assign => assign::layout(&mut d, m, text, bind, pads),
        Screen::Chars => roster::chars(&mut d, m, text, bind, pads),
        // M121: the lobby is the online select's columns with the lobby around them (`lobby::over`), and its prompts
        Screen::Lobby if m.net.room.is_some() => {
            roster::chars(&mut d, m, text, bind, pads);
            online::layout(&mut d, m, hand_x);
        }
        // the match starts from the confirm screen, which stays up under the switch's zoom-out
        // B451: the umpire picker over the players
        Screen::Confirm if m.picking.is_some() && m.sel == 4 => umpire_picker::draw_over(&mut d, m, text, pads, hand_x),
        Screen::Confirm | Screen::Playing => confirm(&mut d, m, text, pads, hand_x),
        Screen::Online | Screen::Lobbies | Screen::Lobby | Screen::Code => online::layout(&mut d, m, hand_x),
        Screen::Mods => mods_screen::layout(&mut d, m, hand_x),
        Screen::Extras => extras::layout(&mut d, m, hand_x),
    }
    online::draw_ask_over(&mut d, m);
    takeover::layout(&mut d, m, hand_x);
    // B221: the screens with no Back row show the title tab's Back (B449: the confirm screen has its own)
    if matches!(m.screen, Screen::Main | Screen::Mode | Screen::Assign | Screen::Chars) && m.picking.is_none() && m.add.ask.is_none() && m.stale.0.is_none() {
        d.anchor = Anchor::Span;
        d.outlined(|d| d.text("<", 10.0, 5.0, 30.0, WHITE));
        d.anchor = Anchor::Centre;
    }
    d
}

/// A Singles / Doubles bar (B141, ours): pill `i`'s colour, dimmed and see-through when not chosen, its word in our
/// font as tall as the bar allows.
fn mode_bar(d: &mut Draw, i: usize, on: bool, [x, y, w, h]: [f32; 4], words: &str) {
    let fill = if on { MODE_FILL[i] } else { MODE_FILL[i].map(|c| c * 0.7) };
    d.pill_at([x, y, w, h], [INK, if on { WHITE } else { [90.0; 3] }, fill], if on { 128.0 } else { 90.0 });
    let size = h * 0.8;
    let start = d.out.len();
    d.label(words, x + w / 2.0, y + h * 0.09, size, w - h, if on { WHITE } else { [100.0; 3] });
    if !on {
        d.out[start..].iter_mut().for_each(|q| q.alpha = 90.0);
    }
}

/// B347: a pick's △ grades: the disc character's from the MENU overlay, a mod's from its TParam.csv row (`mod_grades`).
fn grades_of(text: &Text, w: Who) -> [u8; 5] {
    match w {
        Who::Mod(i) => text.mods[i].1,
        Who::Disc(c) => text.grades[c],
    }
}

/// B296: the roster panel's cream (ours, after the redesign's mockup).
const CREAM: [f32; 3] = [126.0, 122.0, 114.0];
/// Words on the cream panels.
const BROWN: [f32; 3] = [22.0, 15.0, 8.0];

/// A roster cell's face: the character's icon, or the Random card's "?" tile.
fn cell_face(c: Option<Who>) -> (Tex, [f32; 4]) {
    match c {
        None => (C02, RANDOM_TILE),
        Some(Who::Disc(c)) => (Tex::Icon(Who::Disc(c)), [(c % 8) as f32 * 64.0, (c / 8) as f32 * 64.0, 64.0, 64.0]),
        Some(w) => (Tex::Icon(w), [0.0, 0.0, 64.0, 64.0]),
    }
}

/// B306: a roster tile's picture cropped to fill a square: a face icon zoomed past its round frame (a little above
/// its middle, where the face is), the Random tile's middle square.
fn tile_face(c: Option<Who>) -> (Tex, [f32; 4]) {
    let (tex, [x, y, w, h]) = cell_face(c);
    let src = match c {
        None => random_cover([1.0, 1.0]),
        Some(_) => [x + w * 0.13, y + h * 0.09, w * 0.74, h * 0.74],
    };
    (tex, src)
}

/// B306: a T-shirt drawn from quads in `rgb` with an ink outline, filling `[x, y, w, h]`: the body, two sleeves
/// tilted out, a neck cut out in the card's colour `bg`.
fn shirt(d: &mut Draw, [x, y, w, h]: [f32; 4], rgb: [f32; 3], bg: [f32; 3]) {
    let parts = [
        ([x + w * 0.24, y + h * 0.1, w * 0.52, h * 0.9], 0.0),
        ([x + w * 0.02, y + h * 0.14, w * 0.36, h * 0.3], -0.6),
        ([x + w * 0.62, y + h * 0.14, w * 0.36, h * 0.3], 0.6),
    ];
    for (k, rgb) in [(1.5, INK), (0.0, rgb)] {
        for ([px, py, pw, ph], a) in parts {
            d.q(Tex::Solid, [0.0; 4], [px - k, py - k, pw + 2.0 * k, ph + 2.0 * k], rgb, 128.0);
            d.out.last_mut().unwrap().angle = a;
        }
    }
    for (r, rgb) in [(w * 0.14 + 1.5, INK), (w * 0.14, bg)] {
        for (src, dst) in super::menu_art::pill_quads(0, [x + w / 2.0 - r, y + h * 0.1 - r, 2.0 * r, 2.0 * r]) {
            d.q(Tex::Pill, src, dst, rgb, 128.0);
        }
    }
}

/// B238: the ✕ that takes seat `p`'s device off it (tapped or clicked; the device's own ○ does it too).
fn drop_button(d: &mut Draw, p: usize, x: f32, y: f32) {
    d.pill_at([x, y, 28.0, 28.0], [INK, WHITE, [120.0, 30.0, 30.0]], 128.0);
    d.centred("X", x + 14.0, y + 4.0, 20.0, WHITE);
    d.hit([x, y, 28.0, 28.0], pointer::Act::Drop(p));
}

/// B263: the warm fill behind a face icon.
const FACE_FILL: [f32; 3] = [110.0, 100.0, 90.0];

/// B263: a round face icon `icon` (a `Tex`, its source rect) in a frame `size` across centred on (x, y), in the
/// cards' style: a thin ink rim and white ring (dark with the face where every costume is taken) round a warm fill,
/// stacked discs of our pill's outer stadium.
fn face_frame(d: &mut Draw, (tex, src): (Tex, [f32; 4]), x: f32, y: f32, size: f32, tint: [f32; 3]) {
    let ring = if tint == WHITE { WHITE } else { [60.0; 3] };
    // B296: a small one (the roster's cells) without the ring, to spare quads
    let layers: &[(f32, [f32; 3])] = if size < 40.0 { &[(0.0, INK), (1.5, FACE_FILL)] } else { &[(0.0, INK), (1.5, ring), (3.0, FACE_FILL)] };
    for &(inset, rgb) in layers {
        let r = size / 2.0 - inset;
        for (src, dst) in super::menu_art::pill_quads(0, [x - r, y - r, 2.0 * r, 2.0 * r]) {
            d.q(Tex::Pill, src, dst, rgb, 128.0);
        }
    }
    let f = size / 2.0 - 3.5;
    if src == RANDOM_TILE {
        // the square "?" tile: its middle square, inscribed in the round frame (a face icon is round already)
        let [u, v, w, h] = RANDOM_TILE;
        let f = f * std::f32::consts::FRAC_1_SQRT_2;
        return d.q(tex, [u + (w - h) / 2.0, v, h, h], [x - f, y - f, 2.0 * f, 2.0 * f], tint, 128.0);
    }
    d.q(tex, src, [x - f, y - f, 2.0 * f, 2.0 * f], tint, 128.0);
    // M8f9/B400: an umpire mod's head is a square of its picture: clipped round
    if matches!(tex, Tex::Umpire(_)) {
        d.out.last_mut().unwrap().round = [f; 4];
    }
}

impl Grid {
    /// B295: the view's reach in 640×448 units (left, top, right, bottom), down to the legend strip: the lobby and
    /// confirm screens put their help under the seats, not in the description bar.
    fn reach(&self) -> [f32; 4] {
        [self.strip[0], self.strip[1] - 46.0, self.strip[0] + self.strip[2], self.detail[1] + self.detail[3] + 32.0]
    }
}

/// `s` from `x` at `size`, squeezed to fit `max`.
fn left(d: &mut Draw, s: &str, x: f32, y: f32, size: f32, max: f32, rgb: [f32; 3]) {
    let w = d.width(s, size).min(max);
    d.label(s, x + w / 2.0, y, size, max, rgb);
}

/// The confirm screen (B397, `confirm.rs`); the court picker over its players (B450; opened from the lobby: over the
/// lobby's), the umpire select as its own screen.
fn confirm(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity], hand_x: f32) {
    match m.sel {
        1 if m.picking.is_some() => {
            if m.net.venue {
                online::lobby_behind(d, &m.venue_under());
            } else {
                confirm::draw(d, m, text, pads, hand_x);
            }
            court_picker::draw(d, m, text);
        }
        _ => confirm::draw(d, m, text, pads, hand_x),
    }
}

/// B356: the random umpire's icon `size` across centred on `[x, y]`, round like the disc's umpire faces: their dark
/// rim and white ring (stacked discs of our pill's outer stadium), the "?" tile's square `src` clipped to the circle
/// inside them.
fn random_face(d: &mut Draw, src: [f32; 4], [x, y]: [f32; 2], size: f32) {
    for (inset, rgb) in [(0.0, [36.0, 35.0, 34.0]), (0.7, WHITE)] {
        let r = size / 2.0 - inset;
        for (src, dst) in super::menu_art::pill_quads(0, [x - r, y - r, 2.0 * r, 2.0 * r]) {
            d.q(Tex::Pill, src, dst, rgb, 128.0);
        }
    }
    let r = size / 2.0 - 2.2;
    d.q(C02, src, [x - r, y - r, 2.0 * r, 2.0 * r], WHITE, 128.0);
    d.out.last_mut().unwrap().round = [r; 4];
}

/// The part of a `[pw, ph]` picture that covers a `w`×`h` box, cut about its middle (never stretched).
fn cover([pw, ph]: [f32; 2], [w, h]: [f32; 2]) -> [f32; 4] {
    if w / h > pw / ph { let sh = pw * h / w; [0.0, (ph - sh) / 2.0, pw, sh] } else { let sw = ph * w / h; [(pw - sw) / 2.0, 0.0, sw, ph] }
}

/// B192: court value `c`'s picture over `dst`, cropped (never stretched) to its shape: a court mod's own (B401) else
/// its layout court's, Random's "?" tile filling it.
fn court_picture(d: &mut Draw, m: &Menu, c: usize, [x, y, w, h]: [f32; 4]) {
    if let Some((i, size)) = m.court_pic(c) {
        return d.q(Tex::CourtMod(i), cover(size, [w, h]), [x, y, w, h], WHITE, 128.0);
    }
    let k = m.court_layout(c);
    if k >= COURTS {
        return d.q(C02, random_cover([w, h]), [x, y, w, h], WHITE, 128.0);
    }
    d.q(court_pic(k), cover([568.0, 256.0], [w, h]), [x, y, w, h], WHITE, 128.0);
}

/// B295: a side card button: the big one (Start, Ready) yellow with dark words, the rest grey; the chosen one gold.
fn side_button(d: &mut Draw, [x, y, w, h]: [f32; 4], label: &str, on: bool, big: bool) {
    let (fill, words) = match (big, on) {
        (true, true) => ([128.0, 112.0, 40.0], INK),
        (true, false) => ([110.0, 92.0, 30.0], INK),
        (false, true) => ([128.0, 110.0, 40.0], WHITE),
        (false, false) => ([56.0, 56.0, 67.0], WHITE),
    };
    d.pill_at([x, y, w, h], [INK, if on { WHITE } else { [100.0; 3] }, fill], 128.0);
    let size = (h * 0.55).min(24.0);
    d.label(label, x + w / 2.0, y + (h - size) / 2.0, size, w - h * 0.6, words);
}

/// The picker strip's values (court or umpire, by `m.sel`): Random first, then the courts in the original's strip
/// order or the umpires in theirs, then the umpire mods (M8f3).
fn strip_order(m: &Menu) -> Vec<usize> {
    if m.sel == 1 { std::iter::once(COURTS).chain(STRIP).chain(court_mods::values(m)).collect() } else { std::iter::once(UMPIRES).chain(0..UMPIRES).chain(umpire_mods::values(m)).collect() }
}

/// Where the picked value sits on [`strip_order`].
fn strip_at(m: &Menu) -> usize {
    let v = if m.sel == 1 { m.court } else { m.umpire as usize };
    strip_order(m).iter().position(|&k| k == v).unwrap_or(0)
}

/// Preview Voice's lines in turn, (umpire voice program, key): Out, Deuce, Game, Advantage, Game and match.
const CALLS: [(u8, u8); 5] = [(1, 1), (0, 9), (2, 1), (0, 10), (2, 4)];

// ---------------------------------------------------------------- systems

/// The menu as a match left it (picks, seats, settings), back on its main screen.
#[derive(Resource)]
struct Kept(Menu);

#[allow(clippy::too_many_arguments)]
/// B247: the page is played by touch (a phone or tablet, `web/touch.js`; M91a1: the iPhone app, `touch_pad.rs`): its
/// 1P seat is the screen's.
fn touch_page() -> bool {
    #[cfg(target_arch = "wasm32")]
    return crate::web::touched();
    #[cfg(not(target_arch = "wasm32"))]
    super::touch_pad::on()
}

fn step(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    time: Res<Time>,
    mut menu: ResMut<Menu>,
    mut bind: ResMut<Bindings>,
    args: Res<Args>,
    art: Option<Res<Art>>,
    sound: Option<Res<Sound>>,
    mut repeat: Local<std::collections::HashMap<(Option<Entity>, usize), Repeat>>,
    #[cfg(not(target_arch = "wasm32"))] mut exit: MessageWriter<AppExit>,
    mut gfx: ResMut<Graphics>,
    (net, mut clipboard, mut pending): (Option<Res<crate::net::Net>>, ResMut<bevy::clipboard::Clipboard>, Option<ResMut<pointer::Pending>>),
    // Preview Voice: each umpire's voice bank once read, the call playing, and (B487) one waiting for the browser
    mut voice: Local<(std::collections::HashMap<u8, Option<Arc<crate::audio::SoundBank>>>, Option<u64>, Option<(u8, u8, u8)>)>,
    // B336: a launch waiting for the browser to read its mods in
    mut launching: Local<bool>,
) {
    let play = |key: u8| {
        if let (Some(s), Some(Art(_, Some(b), _))) = (&sound, art.as_deref()) {
            s.play_menu(b, key);
        }
    };
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g)| controls::readable(g)).map(|(e, _)| e).collect();
    pads.sort();
    // the controls screen waiting for a key or button
    if let Some(i) = menu.rebinding {
        // a tap or click cancels too (a phone has no key to bind)
        if keys.just_pressed(KeyCode::Escape) || pending.as_mut().is_some_and(|p| !std::mem::take(&mut p.0).is_empty()) {
            menu.rebinding = None;
            play(1);
            return;
        }
        // B291: the Keyboard & Mouse tab takes a key, the Controller tab a pad button
        let key = keys.get_just_pressed().find(|k| Bindings::bindable_key(**k)).copied().filter(|_| menu.keys_tab);
        let button = gamepads.iter().filter(|(_, g)| controls::readable(g)).flat_map(|(_, g)| g.get_just_pressed().copied().collect::<Vec<_>>()).find(|b| Bindings::bindable_pad(*b)).filter(|_| !menu.keys_tab);
        if key.is_some() || button.is_some() {
            if let Some(k) = key {
                bind.rebind_key(i, k);
            }
            if let Some(b) = button {
                bind.rebind_pad(i, b);
            }
            if let Err(e) = bind.save(&args.iso) {
                warn!("controls.txt: {e}");
            }
            menu.rebinding = None;
            play(2);
        }
        return;
    }
    #[cfg(target_arch = "wasm32")]
    crate::web::touch_buzz(menu.settings.0[VIBRATION]);
    #[cfg(not(target_arch = "wasm32"))]
    super::touch_pad::BUZZ.store(menu.settings.0[VIBRATION], std::sync::atomic::Ordering::Relaxed);
    let dt = time.delta_secs();
    // B221: the pointer's acts (`pointer.rs`), as its device's
    let mut acts = pending.as_mut().map(|p| std::mem::take(&mut p.0)).unwrap_or_default();
    if trace::on() && !acts.is_empty() {
        eprintln!("{:9.3} pointer acts {acts:?}", time.elapsed_secs());
    }
    let pointed = pointer::device(&menu, &pads);
    let mut devices = vec![(Dev::Keys, None)];
    devices.extend(pads.iter().map(|&e| (Dev::Pad(e), Some(e))));
    for (dev, id) in devices {
        let held: [bool; 4];
        let mut k = Press::default();
        match id {
            None => {
                let any = |a: Action, extra: KeyCode| bind.key_pressed(&keys, a) || keys.pressed(extra);
                held = [any(Action::Up, KeyCode::ArrowUp), any(Action::Down, KeyCode::ArrowDown), any(Action::Left, KeyCode::ArrowLeft), any(Action::Right, KeyCode::ArrowRight)];
                k.ok = bind.key_just_pressed(&keys, Action::Normal) || keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space]);
                k.back = bind.key_just_pressed(&keys, Action::Cut) || keys.any_just_pressed([KeyCode::Escape, KeyCode::Backspace]);
                k.card = bind.key_just_pressed(&keys, Action::Lob);
                k.prev = keys.any_just_pressed([KeyCode::KeyQ, KeyCode::PageUp]);
                k.next = keys.any_just_pressed([KeyCode::KeyE, KeyCode::PageDown]);
                k.page = keys.just_pressed(KeyCode::Tab);
                k.hand = bind.key_just_pressed(&keys, Action::Camera);
                k.name = keys.just_pressed(KeyCode::KeyN);
                k.kind_prev = keys.just_pressed(KeyCode::Digit1);
                k.kind_next = keys.just_pressed(KeyCode::Digit3);
                k.settings = keys.just_pressed(KeyCode::KeyO);
                // B463: on the select Q / E are the sections (Settings' tab keys), Z / X the costume; 1–4 its emotes
                if menu.screen == Screen::Chars {
                    (k.kind_prev, k.kind_next) = (k.prev, k.next);
                    k.prev = keys.just_pressed(KeyCode::KeyZ);
                    k.next = keys.just_pressed(KeyCode::KeyX);
                }
            }
            Some(e) => {
                let Ok((_, g)) = gamepads.get(e) else { continue };
                let s = g.left_stick();
                held = [g.pressed(GamepadButton::DPadUp) || s.y > 0.5, g.pressed(GamepadButton::DPadDown) || s.y < -0.5, g.pressed(GamepadButton::DPadLeft) || s.x < -0.5, g.pressed(GamepadButton::DPadRight) || s.x > 0.5];
                k.ok = g.just_pressed(GamepadButton::South) || g.just_pressed(GamepadButton::Start);
                k.back = g.just_pressed(GamepadButton::East);
                k.card = g.just_pressed(GamepadButton::North);
                k.prev = g.just_pressed(GamepadButton::LeftTrigger);
                k.next = g.just_pressed(GamepadButton::RightTrigger);
                k.page = g.just_pressed(GamepadButton::West);
                k.hand = g.just_pressed(GamepadButton::Select);
                k.name = g.just_pressed(GamepadButton::Start);
                k.kind_prev = g.just_pressed(GamepadButton::LeftTrigger2);
                k.kind_next = g.just_pressed(GamepadButton::RightTrigger2);
                k.settings = g.just_pressed(GamepadButton::RightThumb);
            }
        }
        let mut dirs = [false; 4];
        for (i, h) in held.into_iter().enumerate() {
            dirs[i] = repeat.entry((id, i)).or_default().step(h, dt);
        }
        [k.up, k.down, k.left, k.right] = dirs;
        if k != Press::default() && touch_page() {
            menu.pad_takes_touch_seat(dev, &pads);
        }
        // a click or tap is the keyboard's while 1P's pad is out (`pointer::device`)
        if k != Press::default() || dev == pointed && !acts.is_empty() {
            menu.takes_lost_1p(dev, &pads);
            menu.takes_lost_p1(dev, &pads);
        }
        let mut fx = Vec::new();
        if dev == pointed {
            for (a, tap) in std::mem::take(&mut acts) {
                fx.extend(menu.point(dev, a, tap));
            }
        }
        if trace::on() && (k != Press::default() || !fx.is_empty()) {
            eprintln!("{:9.3} step {dev:?} {k:?} pointer fx {fx:?}", time.elapsed_secs());
        }
        if dev == Dev::Keys && std::mem::take(&mut *launching) {
            fx.push(Effect::Launch);
        }
        if dev == Dev::Keys && let Some((u, program, key)) = voice.2.take().filter(|h| h.0 == menu.umpire) {
            fx.push(Effect::Hear(u, program, key));
        }
        // B526: Paste's clipboard read answered (the browser's is async)
        #[cfg(target_arch = "wasm32")]
        if dev == Dev::Keys && let Some((text, again)) = crate::web::clipboard_read() {
            fx.push(Effect::Online(online::Fx::Clip(text, again)));
        }
        if k == Press::default() && fx.is_empty() {
            continue;
        }
        if k != Press::default() {
            fx.extend(menu.step(dev, k));
        }
        for e in fx {
            match e {
                Effect::Sound(s) => play(s),
                Effect::SaveSettings => {
                    // applied live: the menu draws with them too
                    gfx.set_if_neq(menu.settings.1);
                    if let Some(s) = &sound {
                        s.set_volumes(menu.settings.2);
                    }
                    let path = Settings::path(&args.iso);
                    if let Err(e) = crate::store::write(&path, menu.settings.text()) {
                        warn!("{}: {e}", path.display());
                    }
                    #[cfg(target_arch = "wasm32")]
                    crate::web::renderer(RENDERERS[menu.settings.5 as usize].0);
                }
                #[cfg(not(target_arch = "wasm32"))]
                Effect::Quit => {
                    exit.write(AppExit::Success);
                }
                Effect::ResetControls => {
                    *bind = Bindings::default();
                    if let Err(e) = bind.save(&args.iso) {
                        warn!("controls.txt: {e}");
                    }
                }
                Effect::Launch => {
                    let a = menu.args(&pads);
                    // every one asked at once (no short-circuit), then tried again next frame
                    if !a.windows(3).filter(|w| w[0] == "--slot-mod").fold(true, |ok, w| crate::modfs::ready(std::path::Path::new(&w[2])) & ok) | !umpire_mods::launch_ready(&a) {
                        *launching = true;
                        continue;
                    }
                    info!("starting the match: {}", a.join(" "));
                    // the match in this app; its quit comes back to this menu as it is now
                    let (base, kept) = (args.clone(), menu.clone());
                    let back = crate::mode::Mode::new(move |app| {
                        app.insert_resource(base).insert_resource(Kept(kept));
                        crate::menu_mode(app);
                    });
                    commands.insert_resource(crate::mode::Next(Some(crate::match_mode(&args.iso, &a, back))));
                }
                Effect::SaveFavorites => {
                    let path = tabs::path(&args.iso);
                    if let Err(e) = crate::store::write(&path, tabs::text(&menu.favs)) {
                        warn!("{}: {e}", path.display());
                    }
                }
                Effect::Hear(u, program, key) => {
                    let Some(s) = &sound else { continue };
                    // B487: a mod's calls still in the browser's storage: asked for, heard once they're in
                    if !voice.0.contains_key(&u) && !umpire_mods::voice_ready(&menu, u) {
                        voice.2 = Some((u, program, key));
                        continue;
                    }
                    let bank = voice.0.entry(u).or_insert_with(|| umpire_mods::preview_bank(&args.iso, &menu, u).map(Arc::new)).clone();
                    if let Some(bank) = bank {
                        if let Some(id) = voice.1.take() {
                            s.stop(id);
                        }
                        voice.1 = Some(s.play_centre(&bank, sound::Play { slot: 5, program, key, volume: 0x80, speed: 1.0 }));
                    }
                }
                Effect::Online(fx) => {
                    for s in online::apply(fx, &mut menu, &mut commands, net.as_deref(), &mut clipboard, &args.iso) {
                        play(s);
                    }
                }
            }
        }
        return;
    }
}

fn draw(
    menu: Res<Menu>,
    art: Option<Res<Art>>,
    bind: Res<Bindings>,
    gamepads: Query<(Entity, &Gamepad, Option<&Name>)>,
    window: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut ticker: ResMut<Ticker>,
    mut slide: ResMut<slide::Slide>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility, &mut UiTransform, &mut BorderColor)>,
    cursor: Option<Res<cursor::Cursor>>,
    icons: Res<Icons>,
    shelf: Res<mods_screen::Shelf>,
    mut hits: ResMut<pointer::Hits>,
    images: Res<Assets<Image>>,
    (buttons, umpire_pics, court_pics, logo, name_art): (Option<Res<legend::Buttons>>, Option<Res<umpire_mods::Pics>>, Option<Res<court_mods::Pics>>, Option<Res<splash::Logo>>, Option<Res<names::NameArt>>),
) {
    let Some(art) = art else { return };
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g, _)| controls::readable(g)).map(|(e, _, _)| e).collect();
    pads.sort();
    // B405: the on-screen pad (web/touch.js, `touch_pad.rs`) is the home screen's Touch seat, not a controller
    let controllers = controllers(gamepads.iter().filter(|(_, g, _)| controls::readable(g)).map(|(_, _, n)| n.map(|n| n.as_str())));
    // the original's menu hand wags in place (`cursor.rs`), it doesn't swing out and back as the pause menu's
    let hand_x = 0.0;
    // the description ticker counts the original's 60 Hz frames from its text's change
    ticker.1 += time.delta_secs() * 60.0;
    let d = drawn(&menu, &art.2, &bind, &pads, controllers, hand_x);
    *hits = pointer::Hits(d.hits, d.outside);
    let (quads, said) = (d.out, d.said);
    // the text flies in when a page opens, not on every new selection or status line
    let page = Some((menu.screen, menu.add.page));
    if page != ticker.2 {
        *ticker = Ticker(said, 0.0, page);
    } else {
        ticker.0 = said;
    }
    let quads = slide.step(menu.screen, quads, time.delta_secs() * 60.0);
    let scale = window.single().map_or(1.0, super::widescreen::unit_px) * super::widescreen::menu_scale();
    let (share, vshare) = super::widescreen::window_menu_shares(&window);
    // the quads' parent (the view) in physical pixels
    let px = window.single().map_or(Vec2::ZERO, |w| super::widescreen::view_size(w) * w.scale_factor());
    for (Slot(i), mut img, mut node, mut vis, mut turn, mut rim) in &mut q {
        let Some(quad) = quads.get(*i).filter(|q| !matches!(q.tex, Tex::Preview(_))) else {
            *vis = Visibility::Hidden;
            continue;
        };
        img.image = match quad.tex {
            Tex::Solid => art.0[0].clone(),
            Tex::Font => art.0[1].clone(),
            // Z21: always there (`cursor::setup` at Startup, from the disc the game can't start without), so no
            // `hsm_yubi` standing in for it
            Tex::Cursor => match &cursor {
                Some(c) => c.image.clone(),
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::Sheet(k) => art.0[2 + k].clone(),
            Tex::Pill => art.0.last().expect("pill").clone(),
            Tex::Logo => match &logo {
                Some(l) => l.0.clone(),
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::Preview(_) => continue,
            Tex::Icon(w) => match (icons.0.get(&w), w) {
                (Some(h), _) => h.clone(),
                (None, Who::Disc(_)) => art.0[2 + FACES].clone(),
                (None, Who::Mod(_)) => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::Cover(w) if icons.0.contains_key(&w) => art.0[0].clone(),
            Tex::Cover(_) => {
                *vis = Visibility::Hidden;
                continue;
            }
            Tex::Face(w) => match icons.0.get(&w) {
                Some(h) => h.clone(),
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::Shelf => match &shelf.shown {
                Some(h) => h.clone(),
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::Thumb(k) => match shelf.thumbs.get(k as usize).cloned().flatten() {
                Some(h) => h,
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::Button(s) => match buttons.as_ref().and_then(|b| b.0.get(&s)) {
                Some((h, _)) => h.clone(),
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::Glyph(s, i) => match buttons.as_ref().and_then(|b| b.1.get(&(s, i))) {
                Some((h, _)) => h.clone(),
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::Umpire(k) => match umpire_pics.as_ref().and_then(|p| p.handles.get(k).cloned().flatten()) {
                Some(h) => h,
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::CourtMod(k) => match court_pics.as_ref().and_then(|p| p.handles.get(k).cloned().flatten()) {
                Some(h) => h,
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
            Tex::Name(p) => match name_art.as_ref().and_then(|a| a.0.get(p).cloned().flatten()) {
                Some((_, h)) => h,
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
        };
        let [u, v, w, h] = quad.src;
        img.rect = (quad.tex != Tex::Solid && quad.tex != Tex::Shelf && !matches!(quad.tex, Tex::Button(_) | Tex::Glyph(..) | Tex::Thumb(_) | Tex::Name(_)) && !matches!(quad.tex, Tex::Icon(w) if icons.0.contains_key(&w)) && !matches!(quad.tex, Tex::Face(_) | Tex::Cover(_)) && (quad.tex != Tex::Cursor || cursor.is_none())).then(|| Rect::new(u, v, u + w, v + h));
        // a face's source is out of 64 whatever the icon's size (a part of it: the Mods tiles' square crop)
        if matches!(quad.tex, Tex::Face(_)) && [u, v, w, h] != [0.0, 0.0, 64.0, 64.0] {
            let k = images.get(&img.image).map_or(1.0, |i| i.width() as f32 / 64.0);
            img.rect = Some(Rect::new(u * k, v * k, (u + w) * k, (v + h) * k));
        }
        // M5d: a tile picture's source is a share of it (0..1): a part of it (a pack's before/after halves)
        if matches!(quad.tex, Tex::Thumb(_)) && [u, v, w, h] != [0.0, 0.0, 1.0, 1.0]
            && let Some(s) = images.get(&img.image).map(|i| i.size().as_vec2())
        {
            img.rect = Some(Rect::new(u * s.x, v * s.y, (u + w) * s.x, (v + h) * s.y));
        }
        // an icon with its own image ignores `src`, but a tile cut at the grid's foot (roster `cut`) keeps its top part,
        // not the whole face squeezed
        // ponytail: icons are square, so the cut is src's height over its width; a real src mapping if one isn't
        if matches!(quad.tex, Tex::Icon(w) if icons.0.contains_key(&w)) && h < w
            && let Some(size) = images.get(&img.image).map(|i| i.size().as_vec2())
        {
            img.rect = Some(Rect::new(0.0, 0.0, size.x, size.y * h / w));
        }
        img.image_mode = if quad.tile { NodeImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: scale / super::menu_art::wall_k() } } else { NodeImageMode::Stretch };
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        img.color = Color::srgba(r, g, b, quad.alpha / 128.0);
        let [mut x, mut y, mut w, mut h] = quad.dst;
        if quad.tex == Tex::Cursor && cursor.is_some() {
            // the hand's image covers `PAD` times its quad, around the same centre (B146)
            (x, y, w, h) = (x - w * (cursor::PAD - 1.0) / 2.0, y - h * (cursor::PAD - 1.0) / 2.0, w * cursor::PAD, h * cursor::PAD);
        }
        let (left, width) = super::widescreen::place(x, w, share, quad.anchor);
        let (top, height) = super::widescreen::vplace_in(y, h, vshare, quad.anchor);
        // edges on whole physical pixels: abutting quads (a box's nine slices) computed apart could round to either
        // side of an edge on a half pixel and leave a one-pixel seam showing what's behind
        let [(left, width), (top, height)] = [(left, width, px.x), (top, height, px.y)].map(|(a, len, p)| snap(a, len, p));
        node.left = Val::Percent(left);
        node.top = Val::Percent(top);
        node.width = Val::Percent(width);
        node.height = Val::Percent(height);
        turn.rotation = Rot2::radians(quad.angle);
        // the corners: units to logical pixels at the quad's placed width
        let k = window.single().map_or(0.0, |win| width / 100.0 * super::widescreen::view_size(win).x / quad.dst[2].max(1e-3));
        let [tl, tr, br, bl] = quad.round.map(|r| Val::Px(r * k));
        node.border_radius = BorderRadius { top_left: tl, top_right: tr, bottom_right: br, bottom_left: bl };
        // an outline: the node's own border (a true rounded stroke) in the quad's colour, the image clear
        node.border = UiRect::all(Val::Px(quad.rim * k));
        if quad.rim > 0.0 {
            *rim = BorderColor::all(img.color);
            img.color = Color::NONE;
        }
        *vis = Visibility::Inherited;
    }
}

/// A quad's `start, len` (percent of a `p`-pixel parent) with both edges rounded to whole pixels, each edge from its
/// own position so two quads sharing an edge share its pixel. `p` 0: as given.
fn snap(start: f32, len: f32, p: f32) -> (f32, f32) {
    if p <= 0.0 {
        return (start, len);
    }
    let edge = |e: f32| (e / 100.0 * p).round() / p * 100.0;
    let a = edge(start);
    (a, edge(start + len) - a)
}

/// The menu's BGM tracks, `bgmm_03`..`bgmm_06` (`bgmm_03`: the original's title screen, on our press-start splash).
#[derive(Resource)]
struct MenuBgm(Vec<Option<(Arc<SoundBank>, Vec<u8>)>>);

impl Screen {
    /// Which of `MenuBgm`'s tracks plays on this screen.
    fn bgm(self) -> Option<usize> {
        match self {
            Screen::Press => Some(0),
            Screen::Main | Screen::Display | Screen::Graphics | Screen::Advanced | Screen::Sound | Screen::Controls | Screen::ModSettings | Screen::Mobile => Some(1),
            Screen::Mode | Screen::Assign | Screen::Chars => Some(2),
            Screen::Confirm => Some(3),
            Screen::Online | Screen::Lobbies | Screen::Lobby | Screen::Code => Some(1),
            Screen::Mods | Screen::Extras => Some(1),
            Screen::Playing => None,
        }
    }
}

fn load_bgm(mut commands: Commands, args: Res<Args>) {
    let Ok(mut iso) = Iso::open(&args.iso) else { return };
    let tracks = (3..=6).map(|n| SoundBank::music(&mut iso, "Menu", &format!("bgmm_{n:02}")).map(|(b, m)| (Arc::new(b), m))).collect();
    commands.insert_resource(MenuBgm(tracks));
}

/// The overlay switches track only when the screen's track changes: the new one from its start at volume 55, no fade.
fn bgm(menu: Res<Menu>, tracks: Option<Res<MenuBgm>>, sound: Option<Res<Sound>>, mut on: Local<Option<usize>>) {
    let (Some(tracks), Some(sound)) = (tracks, sound) else { return };
    let want = menu.screen.bgm().filter(|_| menu.settings.0[1]); // Menu Music; Match Music is the match's `--music`
    if want == *on {
        return;
    }
    *on = want;
    match want.and_then(|t| tracks.0[t].as_ref()) {
        Some((bank, mid)) => sound.music(bank, mid, 55),
        // the match is its own process with its own music
        None => sound.music_volume(0),
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins((controls::plugin, super::widescreen::plugin))
        .add_systems(Startup, ((read_sheets, read_text, setup).chain(), load_bgm, cursor::setup))
        .add_systems(PreUpdate, pointer::mouse_idle.after(bevy::input::InputSystems))
        // `HST_STRESS` (previews.rs) drives the select itself: no real key or pad reaches the menus or settings
        .init_resource::<Ticker>()
        .init_resource::<Icons>()
        .init_resource::<names::NameArt>()
        .init_resource::<slide::Slide>()
        .add_systems(Update, ((fit_grid, step.run_if(|| std::env::var_os("HST_STRESS").is_none()), flame::tick, splash::tick, draw, ticker_text).chain(), bgm, cursor::tick))
        .add_systems(Startup, splash::load)
        .add_systems(Last, cursor::show);
    // M14: the button legend
    legend::plugin(app);
    app.add_systems(Update, legend::fill.after(step));
    app.add_systems(Update, takeover::watch.before(step).after(pointer::pointer));
    app.init_resource::<umpire_mods::Pics>().add_systems(Update, umpire_mods::load.before(draw));
    app.init_resource::<court_mods::Pics>().add_systems(Update, court_mods::load.before(draw));
    app.add_systems(Update, trace::trace.after(online::poll).run_if(trace::on));
    app.init_resource::<pointer::Pending>().init_resource::<pointer::Hits>().add_systems(
        Update,
        (online::poll.after(step).before(draw), online::typing.before(step), names::typing.before(step), names::persist.after(step), names::name_art.after(step).before(draw), pointer::pointer.before(step).run_if(|| std::env::var_os("HST_STRESS").is_none())),
    );
    // B531: a finger on the lobby's Chat button readies the phone's keyboard for its lift
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Update, online::chat_arm.before(step));
    previews::plugin(app);
    plate_emote::plugin(app);
    assign_shake::plugin(app);
    mods_screen::plugin(app);
    osk::plugin(app);
    // B221/B244: the menus take taps, so the on-screen pad's stick keeps to its corner there; the match frees it
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Startup, || crate::web::touch_menu(true));
}

#[cfg(test)]
mod tests {
    use super::*;
    use Effect::*;

    /// B553: back from an online match (`online.rs` `launch`'s way back keeps the menu on its lobby): the menu starts
    /// on that lobby with its seats, picks and connection state, not on the home screen or the sample lobby.
    #[test]
    fn an_online_match_comes_back_to_its_lobby() {
        use crate::net::{Pick, Room, Seat};
        let seats = vec![Some(Seat { peer: "host".into(), pick: Pick::disc(3, 0) }), Some(Seat { peer: "g".into(), pick: Pick::disc(8, 1) })];
        let room = Room { name: "ROOM".into(), seats, names: [("host".into(), "H".into()), ("g".into(), "G".into())].into(), ..Default::default() };
        let mut kept = Menu { screen: Screen::Lobby, sel: 0, ..default() };
        (kept.net.room, kept.net.me, kept.net.status) = (Some(room.clone()), "g".into(), "Back in the lobby.".into());
        let run = |kept: Option<Menu>| {
            let mut app = App::new();
            app.insert_resource(crate::parse(std::iter::once("no-disc.iso".to_string())).args).insert_resource(Roster(Vec::new(), "no-mods".into()));
            if let Some(m) = kept {
                app.insert_resource(Kept(m));
            }
            app.add_systems(Startup, setup);
            app.update();
            app.world_mut().remove_resource::<Menu>().unwrap()
        };
        let m = run(Some(kept.clone()));
        assert_eq!(m.screen, Screen::Lobby);
        assert_eq!((m.net.room.as_ref(), m.net.me.as_str(), m.net.status.as_str()), (Some(&room), "g", "Back in the lobby."));
        assert_eq!((m.seats, m.doubles, m.bots), (kept.seats, kept.doubles, kept.bots), "no sample seats");
        // a local match's way back still lands on the home screen
        kept.screen = Screen::Confirm;
        assert_eq!(run(Some(kept)).screen, Screen::Main);
    }

    /// B405: the browser's and the iPhone's on-screen pads aren't counted as controllers on the home screen.
    #[test]
    fn touch_pad_is_no_controller() {
        let names = [Some("Touch controls (STANDARD GAMEPAD)"), Some("Touch controls (iPhone)"), Some("Xbox Controller"), None];
        assert_eq!(controllers(names.into_iter()), 2);
        assert_eq!(controllers(names[..2].iter().copied()), 0);
    }

    /// Writes into the store the play style and grades of characters uploaded before Browse's card went up with
    /// them, as the select works them out (`read_text`), and with `HST_BAKED=<cache/previews/…-rN>` (an
    /// `HST_MENU=chars HST_BAKE=all` run over the same mod folders) their costume 0's bake as the listing's picture.
    /// Network, writes to the store: `HST_CARDS="<mod dir>@<PR> …"` `HST_TOKEN=<hf-token.txt>` `HST_ISO=<iso>`
    /// `tools/check.sh -p hst store_cards -- --ignored`.
    #[test]
    #[ignore]
    fn store_cards() {
        let (Ok(list), Ok(t), Ok(iso)) = (std::env::var("HST_CARDS"), std::env::var("HST_TOKEN"), std::env::var("HST_ISO")) else { return };
        let t = std::fs::read_to_string(t).unwrap();
        let token = t.trim().split_once('\n').unwrap().1.trim();
        let mut iso = Iso::open(&iso).unwrap();
        let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/MENU.BIN").unwrap());
        let grades = hst_data::exe::Menu::new(&cnf, &bin).unwrap().grades();
        let pc = iso.read("PCDATA/PCDATA.XB").unwrap();
        let pc = Archive::parse(&pc).unwrap();
        let csv = pc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).and_then(|e| pc.read(e).ok()).unwrap();
        let style = styles(&csv);
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        // one commit per pull request, not per mod: the Hub caps commits per repo
        let mut by_pr: std::collections::BTreeMap<u32, Vec<_>> = Default::default();
        for item in list.split_whitespace() {
            let (dir, pr) = item.rsplit_once('@').unwrap();
            let m = crate::mods::read_any(std::path::Path::new(dir)).unwrap();
            let row = crate::mods::tparam(&mut iso, &m).unwrap();
            let (st, g) = (mod_style(&m, &style), mod_grades(&row, m.base, &csv, &grades));
            let baked = std::env::var("HST_BAKED").ok().map(|b| {
                let name = super::previews::dir_stamp(&m.dir);
                std::fs::read(std::path::Path::new(&b).join(format!("m-{name}-0.png"))).unwrap_or_else(|e| panic!("{}: no bake: {e}", m.id))
            });
            eprintln!("{} style {st} grades {g:?}", m.id);
            by_pr.entry(pr.parse().unwrap()).or_default().push((m.id.clone(), st as u8, g, baked));
        }
        let mut failed = 0;
        for (pr, cards) in by_pr {
            let got = rt.block_on(crate::modstore::put_cards(token, pr, cards));
            failed += got.is_err() as usize;
            eprintln!("PR #{pr}: {got:?}");
        }
        assert_eq!(failed, 0);
    }

    /// A mod's △ grades: its base's where its row matches the base, the nearest disc characters' where it doesn't.
    #[test]
    fn mod_grades_follow_nearest() {
        let csv = b"#,x\n0,a,,,,,,,,,,,4,4,3\n1,b,,,,,,,,,,,7,7,8\n2,c,,,,,,,,,,,9,9,9\n3,d,,,,,,,,,,,9,9,9\n";
        let mut g = [[0u8; 5]; 14];
        (g[0], g[1], g[2], g[3]) = ([1, 1, 1, 5, 2], [2, 2, 3, 3, 3], [4, 4, 4, 1, 4], [3, 3, 3, 1, 3]);
        let row = |s: &str| s.split(',').map(String::from).collect::<Vec<_>>();
        assert_eq!(mod_grades(&row("0,a,,,,,,,,,,,4,4,3"), 0, csv, &g), g[0]);
        // serve 7 → character 1's grade; stroke 9 → mean of 4 and 3 rounds to 4; volley 3 is the base's; Impact, Footwork base
        assert_eq!(mod_grades(&row("0,a,,,,,,,,,,,7,9,3"), 0, csv, &g), [2, 4, 1, 5, 2]);
    }

    /// B243: a locked row under the hand stays a dim grey pill, never the chosen row's gold; it can still be tapped.
    #[test]
    fn locked_rows_greyed() {
        let t = Text { msg: Vec::new(), glyphs: default(), grades: [[0; 5]; 14], style: [0; 14], hands: [1.0; 14], ranks: Vec::new(), mods: Vec::new() };
        let gold = |sel| {
            let mut d = Draw { out: Vec::new(), text: &t, said: String::new(), anchor: Anchor::Centre, hits: Vec::new(), outside: None };
            d.rows_locked(&["a".into(), "b".into()], sel, 0.0, 0.0, 100.0, 0.0, 40.0, |i| i == 1);
            assert_eq!(d.hits.len(), 2);
            d.out.iter().any(|q| q.rgb == [128.0, 110.0, 40.0])
        };
        assert!(gold(0) && !gold(1));
    }

    /// A 2160-pixel-high view: a box's middle and bottom slices on the 3P row meet on a half pixel (238 units); snapped,
    /// the middle slice ends on the pixel the bottom one starts on, so no seam shows the team box through the row.
    #[test]
    fn abutting_quads_share_a_pixel() {
        let pct = |y: f32| y / 4.48;
        let (p, y) = (2160.0, 216.0);
        let (mid, bottom) = (snap(pct(y + 12.0), pct(10.0), p), snap(pct(y + 22.0), pct(12.0), p));
        let px = |e: f32| e / 100.0 * p;
        assert_eq!(px(mid.0 + mid.1).round(), px(bottom.0).round());
        assert!((px(mid.0 + mid.1) - px(mid.0 + mid.1).round()).abs() < 1e-3, "on a whole pixel");
    }

    #[test]
    fn description_wraps_and_fits() {
        assert_eq!(wrap("a bb ccc dddd", 6), ["a bb", "ccc", "dddd"]);
        assert_eq!(wrap("toolongword x", 4), ["toolongword", "x"]);
        // a short line keeps the cap; a long one wraps to fit the bar, never wider than it
        let (p, l) = fit_lines("Play music in the menus.", 600.0, 30.0, 10.0);
        assert_eq!((p, l.len()), (10.0, 1));
        let long = "Only upload what you have the right to share: you're responsible for it. It goes live for everyone at once. Press again to upload Anya.";
        let (p, l) = fit_lines(long, 600.0, 30.0, 10.0);
        assert!(l.len() >= 2 && l.len() as f32 * 2.0 * p <= 30.0 && l.iter().all(|l| l.chars().count() as f32 * p <= 600.0));
        assert_eq!(l.join(" "), long);
        // flies in from the right edge at 32 a frame, then holds at 16
        assert_eq!([ticker_x_in(0.0, 656.0), ticker_x_in(1.0, 656.0), ticker_x_in(20.0, 656.0), ticker_x_in(1000.0, 656.0)], [656.0, 624.0, 16.0, 16.0]);
    }

    /// B521: a platform whose glyphs lay out wider than Fira Mono's 0.6 em: the next fit takes them as that wide, so
    /// the line wraps or shrinks inside the bar instead of running off its right edge.
    #[test]
    fn description_grows_to_the_laid_out_size() {
        let long = "Open a lobby for players over the internet or on this network; start when you like: open seats are played by the computer.";
        let mut g = Grow { sent: ("a".into(), Vec2::new(100.0, 20.0)), ..Grow::default() };
        g.measure("b", Vec2::new(300.0, 60.0));
        g.measure("a", Vec2::new(100.0, 20.0));
        assert_eq!((g.w, g.h), (1.0, 1.0), "another text, or as expected: no change");
        g.measure("a", Vec2::new(115.0, 20.0));
        assert!(g.w > 1.15 && g.h == 1.0);
        let (w, h) = (600.0, 30.0);
        let (p, l) = fit_lines(long, w / g.w, h / g.h, 10.0 / g.w);
        let longest = l.iter().map(|l| l.chars().count()).max().unwrap() as f32;
        assert!(longest * p * 1.15 <= w, "at the platform's width it still fits");
        g.measure("a", Vec2::new(105.0, 20.0));
        assert!(g.w > 1.15, "never shrinks back");
    }

    #[test]
    fn bgm_per_screen() {
        let t = [Screen::Press, Screen::Main, Screen::Display, Screen::Controls, Screen::Mode, Screen::Assign, Screen::Chars, Screen::Confirm, Screen::Playing].map(Screen::bgm);
        assert_eq!(t, [Some(0), Some(1), Some(1), Some(1), Some(2), Some(2), Some(2), Some(3), None]);
    }

    fn press(f: impl FnOnce(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    #[test]
    fn settings_round_trip() {
        let s = Settings([true, false, false, true, true, true, true, true, true], Graphics::LOW, Volumes::default(), 3, Ask::Each, 0, UMPIRES as u8, 0, 4, 0, Ask::Each);
        assert_eq!(Settings::parse(&s.text()), s);
        // B339: Downloads at Once: Auto (3 on desktop) unless set, at most 8
        assert_eq!((s.downloads_at_once(), Settings::default().downloads_at_once()), (4, 3));
        assert_eq!((Settings::parse("downloads_at_once = 8\n").8, Settings::parse("downloads_at_once = 9\n").8), (8, 0));
        let d = Graphics::default();
        assert_eq!(Settings::parse("music = off\nbogus = off\n"), Settings([true, true, false, true, true, true, true, true, true], d, Volumes::default(), 0, Ask::Each, 0, UMPIRES as u8, 0, 0, 0, Ask::Each));
        // a fresh file: menu music on, court music off; an old file's `music` is the match's
        assert_eq!(Settings::parse(""), Settings([true, true, false, true, true, true, true, true, true], d, Volumes::default(), 0, Ask::Each, 0, UMPIRES as u8, 0, 0, 0, Ask::Each));
        assert_eq!(Settings::parse("music = on\n"), Settings([true, true, true, true, true, true, true, true, true], d, Volumes::default(), 0, Ask::Each, 0, UMPIRES as u8, 0, 0, 0, Ask::Each));
        // B117: an old file's widescreen on is Auto, off 4:3; `aspect` wins
        assert_eq!(Settings::parse("widescreen = on\n").3, 0);
        assert_eq!(Settings::parse("widescreen = off\n").3, 1);
        assert_eq!(Settings::parse("widescreen = off\naspect = 21:9\n").3, 4);
        // Upscaled Downloads follows Upscaled Textures unless it's set itself
        assert!(!Settings::parse("upscaled_textures = off\n").0[UPSCALED_DOWNLOADS]);
        assert!(Settings::parse("upscaled_textures = off\nupscaled_downloads = on\n").0[UPSCALED_DOWNLOADS]);
        assert_eq!(Settings::parse("menu_music = off\n"), Settings([true, false, false, true, true, true, true, true, true], d, Volumes::default(), 0, Ask::Each, 0, UMPIRES as u8, 0, 0, 0, Ask::Each));
        assert!(!Settings::default().flags().contains(&"--music".to_string()));
        assert!(!Settings([true, true, false, true, true, true, true, true, true], d, Volumes::default(), 0, Ask::Each, 0, UMPIRES as u8, 0, 0, 0, Ask::Each).flags().contains(&"--music".to_string()));
        assert!(Settings([true, false, true, true, true, true, true, true, true], d, Volumes::default(), 0, Ask::Each, 0, UMPIRES as u8, 0, 0, 0, Ask::Each).flags().contains(&"--music".to_string()));
        // B289: the online mod answer round-trips; an unknown word keeps Ask
        let a = Settings { 4: Ask::Never, ..Settings::default() };
        assert_eq!(Settings::parse(&a.text()), a);
        assert_eq!(Settings::parse("online_mods = maybe\n").4, Ask::Each);
        // M20: chat shown unless turned off
        assert!(!Settings::parse("chat = off\n").0[CHAT] && Settings::parse("").0[CHAT]);
        // B474: emote voices default on, round-trip off
        let e = Settings::parse("emote_voices = off\n");
        assert!(!e.0[EMOTE_VOICES] && Settings::parse("").0[EMOTE_VOICES] && !Settings::parse(&e.text()).0[EMOTE_VOICES]);
        assert_eq!(Settings::parse("uncapped_fps = off\n").1, Graphics { vsync: true, ..d });
        assert_eq!(s.flags()[..1], ["--fps"]);
        assert_eq!(Settings([false, false, true, false, true, true, true, true, true], d, Volumes::default(), 1, Ask::Each, 0, UMPIRES as u8, 0, 0, 0, Ask::Each).flags()[..2], ["--no-upscale", "--music"]);
    }

    #[test]
    fn repeat_after_hold() {
        let mut r = Repeat::default();
        let fired: Vec<bool> = (0..60).map(|_| r.step(true, 1.0 / 60.0)).collect();
        // the press, then at 0.4 s and every 0.1 s after
        assert_eq!(fired.iter().filter(|&&f| f).count(), 1 + 6);
        assert!(fired[0] && !fired[1]);
    }

    fn fake_mod(donor: usize) -> crate::mods::Mod {
        crate::mods::Mod {
            dir: "mods/x".into(),
            id: "x".into(),
            name: "x".into(),
            source: String::new(),
            costumes: vec![],
            donor,
            hand: 1.0,
            base: donor,
            overrides: vec![],
            ai_row: donor,
            texture_face: false,
            no_face: false,
            voice: "voice/".into(),
            motions: None,
            costume_of: None,
            skeleton: None,
            upscale: true,
        }
    }

    /// B216: 2P a computer player 1P picks for (as an online lobby's), not an open seat's automatic pick.
    fn add_com(m: &mut Menu) {
        assert_eq!(m.screen, Screen::Chars);
        m.bots[1] = true;
    }

    /// B216 / B258 / B346: a singles match with nobody on 2P: once 1P is ready, 2P's COM card starts on 1P's
    /// character (its next free costume); ✕ keeps it, or 1P picks another; ○ (B355) undoes 1P's pick instead of
    /// picking for the computer.
    #[test]
    fn an_open_seat_is_a_com_card() {
        let (ok, back) = (press(|p| &mut p.ok), press(|p| &mut p.back));
        let mut m = Menu::default();
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Chars);
        assert_eq!(m.driven(Dev::Keys), Some(0));
        m.step(Dev::Keys, press(|p| &mut p.right)); // from Random onto Ashley
        m.step(Dev::Keys, ok);
        assert_eq!((m.screen, m.driven(Dev::Keys)), (Screen::Chars, Some(1)));
        assert!(!m.players[1].ready);
        assert_eq!((m.players[1].char(), m.players[1].costume), (0, 1));
        // 1P moves the COM's cursor, then backs out: 1P's own pick is undone, the COM's stays open (B355)
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_ne!(m.players[1].char(), 0);
        m.step(Dev::Keys, back);
        assert_eq!((m.screen, m.driven(Dev::Keys)), (Screen::Chars, Some(0)));
        assert!(!m.players[0].ready && !m.players[1].ready && m.open(1));
        // 1P readies again; this time 1P picks the COM's character
        m.step(Dev::Keys, ok);
        m.step(Dev::Keys, press(|p| &mut p.right));
        let c = m.players[1].char();
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Confirm);
        assert_eq!(m.players[1].char(), c);
        // doubles, two humans: either one can pick each COM card
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let mut m = Menu { screen: Screen::Chars, doubles: true, seats: [Some(Dev::Keys), Some(pad), None, None], ..default() };
        m.step(Dev::Keys, ok);
        assert_eq!(m.driven(Dev::Keys), Some(0), "2P not ready yet");
        m.step(pad, ok);
        assert_eq!((m.driven(Dev::Keys), m.driven(pad)), (Some(2), Some(2)));
        m.step(pad, ok);
        assert_eq!(m.driven(Dev::Keys), Some(3));
        // ○ on 4P's card: back to 3P's, not a pick for the computer (B355)
        m.step(Dev::Keys, back);
        assert_eq!((m.screen, m.driven(Dev::Keys)), (Screen::Chars, Some(2)));
        assert!(!m.players[2].ready && !m.players[3].ready);
    }

    /// B355: on a phone (touch only, the keyboard's device) the title tab's Back after 1P's pick undoes it, never
    /// readies the computer's card; a second one leaves the select.
    #[test]
    fn touch_back_after_1p_picks_undoes_it() {
        let mut m = Menu::default();
        m.step(Dev::Keys, press(|p| &mut p.ok));
        m.step(Dev::Keys, press(|p| &mut p.right));
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert!(m.players[0].ready && m.driven(Dev::Keys) == Some(1));
        let tab = pointer::press(|k| k.back = true);
        assert!(pointer::tests::hits(&m).0.iter().any(|h| h.act == tab), "the title tab is drawn");
        m.point(Dev::Keys, tab, true);
        assert_eq!(m.screen, Screen::Chars);
        assert!(!m.players[0].ready && !m.players[1].ready);
        m.point(Dev::Keys, tab, true);
        assert_eq!(m.screen, Screen::Main);
    }

    /// B385: the same with a pad (○) that chose Local: its pick undone, still on the roster with 1P picking; a
    /// second ○ leaves for the main menu.
    #[test]
    fn pad_back_after_1p_picks_undoes_it() {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let mut m = Menu::default();
        m.step(pad, press(|p| &mut p.ok));
        assert_eq!((m.screen, m.seats), (Screen::Chars, [Some(pad), None, None, None]));
        m.step(pad, press(|p| &mut p.right));
        m.step(pad, press(|p| &mut p.ok));
        assert!(m.players[0].ready && m.driven(pad) == Some(1));
        m.step(pad, press(|p| &mut p.back));
        assert_eq!((m.screen, m.driven(pad)), (Screen::Chars, Some(0)));
        assert!(!m.players[0].ready && !m.players[1].ready);
        m.step(pad, press(|p| &mut p.back));
        assert_eq!(m.screen, Screen::Main);
    }

    /// B388: 1P's pad unplugged on the select: the keyboard takes 1P over (from its 2P seat, which the pad gets back)
    /// and backs out; with 1P's pad still in, the keyboard keeps its own seat.
    #[test]
    fn keyboard_takes_over_a_lost_1p() {
        let [a, b] = [9, 10].map(|n| Entity::from_raw_u32(n).unwrap());
        let pad = Dev::Pad(a);
        let mut m = Menu::default();
        m.step(pad, press(|p| &mut p.ok));
        assert_eq!(m.seats[0], Some(pad));
        m.takes_lost_1p(Dev::Keys, &[a]);
        assert_eq!(m.seats[0], Some(pad), "1P's pad is in");
        m.takes_lost_1p(Dev::Keys, &[]);
        assert_eq!(m.seats, [Some(Dev::Keys), None, None, None]);
        m.step(Dev::Keys, press(|p| &mut p.back));
        assert_eq!(m.screen, Screen::Main);
        // the keyboard seated as 2P trades seats; a seated pad never takes 1P, a spare one does
        m.seats = [Some(pad), Some(Dev::Keys), Some(Dev::Pad(b)), None];
        m.takes_lost_1p(Dev::Pad(b), &[b]);
        assert_eq!(m.seats[0], Some(pad));
        m.takes_lost_1p(Dev::Keys, &[b]);
        assert_eq!(m.seats, [Some(Dev::Keys), Some(pad), Some(Dev::Pad(b)), None]);
        m.seats = [Some(pad), None, None, None];
        m.takes_lost_1p(Dev::Pad(b), &[b]);
        assert_eq!(m.seats[0], Some(Dev::Pad(b)));
    }

    #[test]
    fn locked_costumes_and_match_order() {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let mut m = Menu::default();
        let ok = press(|p| &mut p.ok);
        m.step(Dev::Keys, ok); // Local: straight to the roster (B331)
        assert_eq!(m.screen, Screen::Chars);
        m.step(pad, ok); // the pad joins as 2P
        assert_eq!((m.doubles, m.seats), (false, [Some(Dev::Keys), Some(pad), None, None]));
        // B361: a third device makes it doubles (1P's partner, the pad across the net); leaving keeps it doubles
        let pad2 = Dev::Pad(Entity::from_raw_u32(11).unwrap());
        m.step(pad2, ok);
        assert_eq!((m.doubles, m.seats), (true, [Some(Dev::Keys), Some(pad2), Some(pad), None]));
        m.step(pad2, press(|p| &mut p.back));
        assert_eq!((m.doubles, m.seats), (true, [Some(Dev::Keys), None, Some(pad), None]));
        m.seats = [Some(Dev::Keys), Some(pad), None, None];
        // doubles with the pad as 1P's partner (a paused match's seats); B216: a computer player 1P picks for on 3P;
        // 4P stays open (its automatic pick)
        m.doubles = true;
        m.bots[2] = true;
        // B346: both from Random onto Ashley
        m.step(Dev::Keys, press(|p| &mut p.right));
        m.step(pad, press(|p| &mut p.right));
        // B66: both on Ashley costume 1: 1P locks it in, 2P moves on to costume 2; leaving Ashley and coming back
        // lands on costume 2 again, and L1 / R1 pass costume 1
        m.step(Dev::Keys, ok);
        assert!(m.players[0].ready && !m.blocked(1));
        assert_eq!(m.players[1].costume, 1);
        m.step(pad, press(|p| &mut p.right));
        assert_eq!((m.players[1].char(), m.players[1].costume), (1, 0));
        m.step(pad, press(|p| &mut p.left));
        assert_eq!((m.players[1].char(), m.players[1].costume), (0, 1));
        m.step(pad, press(|p| &mut p.prev));
        assert_eq!(m.players[1].costume, COSTUMES - 1);
        m.step(pad, press(|p| &mut p.next));
        assert_eq!(m.players[1].costume, 1, "10 → 2, past 1");
        m.step(pad, ok);
        assert!(m.players[1].ready);
        // 1P now picks the computer's 3P: Cody; the open 4P starts on Cody too (B346), his next free costume
        assert_eq!(m.driven(Dev::Keys), Some(2));
        m.step(Dev::Keys, press(|p| &mut p.right));
        m.step(Dev::Keys, ok);
        // B258: then the open 4P's COM card
        assert_eq!(m.driven(Dev::Keys), Some(3));
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Confirm);
        let a = m.args(&[Entity::from_raw_u32(3).unwrap(), Entity::from_raw_u32(9).unwrap()]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        // match order: 1P, 3P, 2P, 4P
        assert_eq!(flag("--chars"), "0,1,0,1");
        assert_eq!(flag("--outfits"), "0,0,1,1");
        assert_eq!(flag("--pads"), "k,1,-,-");
        assert_eq!((flag("--sets"), flag("--games")), ("1".into(), "4".into()));
        assert!(flag("--umpire").parse::<usize>().unwrap() < UMPIRES, "B191: Random by default, thrown at Start");
        assert!(!a.contains(&"--singles".to_string()));
        // B66: a character with no free costume is passed over by left/right/up/down (one-costume mods, 4 a row:
        // 1 and 4 taken), never stopped on
        let one = |i: usize| crate::mods::Mod { id: format!("m{i}"), costumes: vec!["model/c00.glb".into()], ..fake_mod(i) };
        let mut m = Menu { screen: Screen::Chars, doubles: true, mods: (0..9).map(one).collect(), seats: [Some(Dev::Keys), Some(pad), None, None], ..default() };
        m.players = [Player { modded: true, ..default() }; 4];
        for (q, i) in [(1, 1), (2, 4)] {
            (m.players[q].pick, m.players[q].ready) = (i, true);
        }
        let go = |m: &mut Menu, f: fn(&mut Press) -> &mut bool| {
            m.step(Dev::Keys, press(f));
            m.cell(0)
        };
        // B382: the one grid (8 a row here): the mods' section under Hot Shots Tennis' two rows
        m.grid = Grid::fit(4.0 / 3.0);
        while m.columns().cols < 8 {
            m.grid.view[2] += 1.0;
        }
        assert_eq!(m.columns().cols, 8);
        assert_eq!(go(&mut m, |p| &mut p.right), Some(Who::Mod(2)));
        assert_eq!(go(&mut m, |p| &mut p.left), Some(Who::Mod(0)));
        assert_eq!(go(&mut m, |p| &mut p.left), Some(Who::Disc(GRID[CHARS - 1].0)), "back past the section's start");
        assert_eq!(go(&mut m, |p| &mut p.right), Some(Who::Mod(0)));
        assert_eq!(go(&mut m, |p| &mut p.down), Some(Who::Mod(8)));
        assert_eq!(go(&mut m, |p| &mut p.down), Some(Who::Mod(8)), "the grid's end");
        assert_eq!(go(&mut m, |p| &mut p.up), Some(Who::Mod(0)));
        assert_eq!(go(&mut m, |p| &mut p.up), Some(Who::Disc(GRID[7].0)), "↑ over the heading");
        assert_eq!(go(&mut m, |p| &mut p.up), None, "the Random card");
    }

    /// B269: with 3 computer players added in doubles, 1P picking Random leaves each next COM's cursor on Random, so
    /// ✕ ✕ ✕ gives them all Random; a character carries over too (on its next free costume).
    #[test]
    fn the_next_com_starts_on_the_last_pick() {
        let ok = press(|p| &mut p.ok);
        let mut m = Menu { screen: Screen::Chars, doubles: true, seats: [Some(Dev::Keys), None, None, None], bots: [false, true, true, true], ..default() };
        m.step(Dev::Keys, press(|p| &mut p.left));
        assert!(m.players[0].random);
        for q in 1..4 {
            m.step(Dev::Keys, ok);
            assert_eq!(m.driven(Dev::Keys), Some(q));
            assert!(m.players[q].random && !m.players[q].ready, "COM {q} on Random");
        }
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Confirm);
        assert!(m.players.iter().all(|p| p.random && p.ready));
        // a character: the next COM lands on it, on the costume after the taken one
        let mut m = Menu { screen: Screen::Chars, doubles: true, seats: [Some(Dev::Keys), None, None, None], bots: [false, true, true, true], ..default() };
        m.players[0].cursor = 3;
        m.step(Dev::Keys, ok);
        assert_eq!((m.players[1].cursor, m.players[1].random, m.players[1].costume), (3, false, 1));
        m.step(Dev::Keys, ok);
        assert_eq!((m.players[2].cursor, m.players[2].costume), (3, 2));
    }

    /// B346: a local select opens with 1P on the Random card (not Ashley); in doubles the open seats' COM cards then
    /// take the character 1P readied on, each on its next free costume, not the next character.
    #[test]
    fn select_opens_on_random_and_coms_keep_the_character() {
        let ok = press(|p| &mut p.ok);
        let mut m = Menu::default();
        let mut out = Vec::new();
        m.local_start(Dev::Keys, &mut out);
        assert_eq!((m.screen, m.doubles), (Screen::Chars, false));
        // three COM cards (B361: a local roster is doubles only from a third device; the COM cards work alike)
        m.doubles = true;
        assert!(m.players.iter().all(|p| p.random && !p.ready), "every cursor on Random");
        assert_eq!(m.cell(0), None);
        m.step(Dev::Keys, press(|p| &mut p.right));
        let ch = m.players[0].char();
        m.step(Dev::Keys, ok);
        for q in 1..4 {
            assert!(m.open(q) && m.driven(Dev::Keys) == Some(q));
            assert_eq!((m.players[q].char(), m.players[q].random, m.players[q].costume), (ch, false, q), "COM {q}");
            m.step(Dev::Keys, ok);
        }
        assert_eq!(m.screen, Screen::Confirm);
    }

    /// B217: Random locked in many times lands on disc characters (a costume pack's costumes too), custom ones and
    /// their costumes beyond the first, never on a costume someone's ready on; picking again goes back to the ? card; a computer's
    /// Random draws the same way.
    #[test]
    fn random_card_draws_disc_and_mods() {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let three = |i: usize| crate::mods::Mod { id: format!("m{i}"), costumes: vec!["a".into(), "b".into(), "c".into()], ..fake_mod(i) };
        let pack = crate::mods::Mod { id: "pack".into(), costumes: vec!["a".into(), "b".into()], costume_of: Some(0), ..fake_mod(0) };
        let mut m = Menu { screen: Screen::Chars, doubles: true, mods: vec![three(1), three(2), pack], seats: [Some(Dev::Keys), Some(pad), None, None], bots: [false, false, true, true], ..default() };
        // B216: 3P and 4P are added computer players (a seat with nobody would be filled at Start)
        // taken: 2P ready on Jun's costume 4, the computer's 3P on the first mod's costume 2
        m.players[1] = Player { cursor: GRID.iter().position(|g| g.0 == 2).unwrap(), costume: 3, ready: true, ..default() };
        m.players[2] = Player { modded: true, pick: 0, costume: 1, ready: true, ..default() };
        let taken = [(Who::Disc(2), 3), (Who::Mod(0), 1)];
        // B296: ← from the first character reaches the Random card, the roster's first cell
        m.players[0].cursor = 0;
        m.step(Dev::Keys, press(|p| &mut p.left));
        assert!(m.players[0].random);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..3000 {
            m.step(Dev::Keys, press(|p| &mut p.ok));
            assert!(m.players[0].ready);
            let pick = m.pick(0);
            assert!(!taken.contains(&pick), "{pick:?} is taken");
            seen.insert(pick);
            // ○'s picking again (here 1P's ○ would go to the computers', the humans all ready)
            m.unready(0);
            assert!(m.players[0].random);
        }
        assert!(seen.iter().any(|&(w, c)| matches!(w, Who::Disc(_)) && c > 0), "disc costumes beyond the first");
        assert!(seen.contains(&(Who::Mod(2), 1)), "the costume pack's");
        assert!(seen.contains(&(Who::Mod(1), 2)) && seen.contains(&(Who::Mod(0), 2)), "custom characters' costumes");
        assert_eq!(seen.len(), 14 * 10 + 2 + 3 + 3 - taken.len(), "every open pick comes up");
        // the computer's 4P on Random, driven by 1P once the humans are ready
        m.step(Dev::Keys, press(|p| &mut p.ok));
        m.players[3].random = true;
        assert_eq!(m.driven(Dev::Keys), Some(3));
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert!(m.players[3].ready && m.screen == Screen::Confirm);
        let picks: Vec<_> = (0..4).map(|p| m.pick(p)).collect();
        assert!((0..4).all(|p| (0..4).all(|q| p == q || picks[p] != picks[q])), "{picks:?}");
    }

    /// B297: Start puts a Rematch card at the top of the main menu; it launches the same match flags, Local takes it
    /// off again, and Back from Local lands on the Local row.
    #[test]
    fn rematch_replays_the_setup() {
        // a fixed umpire: Random (B191's default) draws again for the rematch
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], court: 3, umpire: 2, ..default() };
        assert!(!m.main_rows().contains(&home::Row::Rematch));
        assert_eq!(m.step(Dev::Keys, press(|p| &mut p.ok)), [Effect::Sound(2), Effect::Launch]);
        let first = m.args(&[]);
        (m.screen, m.sel) = (Screen::Main, 0);
        assert_eq!(m.main_rows()[0], home::Row::Rematch);
        assert_eq!(m.step(Dev::Keys, press(|p| &mut p.ok)), [Effect::Sound(2), Effect::Launch]);
        assert_eq!((m.screen, m.args(&[])), (Screen::Playing, first));
        (m.screen, m.sel) = (Screen::Main, m.main_at(home::Row::Local));
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!(m.screen, Screen::Chars);
        m.step(Dev::Keys, press(|p| &mut p.back));
        assert_eq!((m.screen, m.sel, m.rematch), (Screen::Main, 0, false));
        assert_eq!(m.main_rows()[0], home::Row::Local);
    }

    #[test]
    fn random_court_and_umpire() {
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], court: 0, ..default() };
        let (right, ok) = (press(|p| &mut p.right), press(|p| &mut p.ok));
        m.sel = 1;
        m.step(Dev::Keys, ok);
        m.step(Dev::Keys, press(|p| &mut p.left)); // court 0, the strip's first after Random
        m.step(Dev::Keys, ok);
        assert_eq!(m.court, COURTS);
        m.sel = 4;
        m.step(Dev::Keys, ok);
        m.step(Dev::Keys, right); // B191: Random by default (the strip's first) → umpire 0
        assert_eq!(m.step(Dev::Keys, ok), [Sound(2), SaveSettings]);
        assert_eq!((m.umpire, Settings::parse(&m.settings.text()).6), (0, 0), "the pick is saved");
        m.step(Dev::Keys, ok);
        m.step(Dev::Keys, press(|p| &mut p.left)); // umpire 0 → Random
        m.step(Dev::Keys, ok);
        assert_eq!(m.umpire as usize, UMPIRES);
        assert_eq!(Settings::parse(&m.settings.text()).6 as usize, UMPIRES);
        // an older settings.txt (no umpire line) or an unknown value: Random
        for t in ["", "umpire = 9\n", "umpire = lily\n"] {
            assert_eq!(Settings::parse(t).6 as usize, UMPIRES, "{t:?}");
        }
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            (m.screen, m.sel) = (Screen::Confirm, 0);
            m.step(Dev::Keys, press(|p| &mut p.ok));
            let a = m.args(&[]);
            let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].parse::<usize>().unwrap();
            assert!((1..=COURTS).contains(&flag("--court")) && flag("--umpire") < UMPIRES);
            seen.insert((flag("--court"), flag("--umpire")));
        }
        assert!(seen.len() > 20, "the dice should vary: {seen:?}");
        // the pick stays Random for the next match
        assert_eq!((m.court, m.umpire as usize), (COURTS, UMPIRES));
    }

    /// B371 / B375: Start (keyboard N) on the select opens the device's own seat's name picker; a pad's Start is ✕ too,
    /// but there it doesn't ready the pick.
    #[test]
    fn start_opens_the_name_picker_on_the_select() {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let mut m = Menu { screen: Screen::Chars, seats: [Some(Dev::Keys), Some(pad), None, None], ..default() };
        m.players[0].cursor = 6;
        m.step(Dev::Keys, press(|p| &mut p.name));
        assert!(m.naming(0) && !m.naming(1), "Start names the device's own player");
        m.step(pad, Press { ok: true, name: true, ..default() });
        assert!(m.naming(1) && !m.players[1].ready);
    }

    /// B64: ✕ on Select Court / Select Umpire opens its picker; B257: ←→↑↓ walk the court grid (three to a row, no wrap),
    /// ←→ the umpire cards; ○ puts the value back.
    #[test]
    fn court_and_umpire_pickers() {
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], court: 0, umpire: 0, ..default() };
        let [down, left, right, ok, back] = [press(|p| &mut p.down), press(|p| &mut p.left), press(|p| &mut p.right), press(|p| &mut p.ok), press(|p| &mut p.back)];
        m.sel = 1;
        assert_eq!(m.court, 0);
        m.step(Dev::Keys, ok);
        let walk: Vec<usize> = [right, right, down, left, left, left, right].iter().map(|&k| (m.step(Dev::Keys, k), m.court).1).collect();
        // the strip: Random, then the original's 0 3 5 1 7 8 10 9 2 6 4; ↑↓ do nothing, the ends stop
        assert_eq!(walk, [3, 5, 5, 3, 0, COURTS, 0]);
        for _ in 0..12 {
            m.step(Dev::Keys, right);
        }
        assert_eq!(m.court, 4);
        m.step(Dev::Keys, back);
        assert_eq!((m.court, m.picking, m.screen), (0, None, Screen::Confirm));
        // B449: → from the court's cell is the umpire's
        m.step(Dev::Keys, right);
        assert_eq!(m.sel, 4);
        m.step(Dev::Keys, ok);
        m.step(Dev::Keys, left);
        assert_eq!(m.step(Dev::Keys, left), []); // the umpire cards don't wrap: Random first
        assert_eq!(m.umpire as usize, UMPIRES);
        // Preview Voice: not for Random; one of the picked umpire's lines in turn
        assert_eq!(m.step(Dev::Keys, press(|p| &mut p.card)), []);
        for _ in 0..9 {
            m.step(Dev::Keys, right);
        }
        assert_eq!(m.step(Dev::Keys, press(|p| &mut p.card)), [Hear(4, CALLS[0].0, CALLS[0].1)]);
        assert_eq!(m.step(Dev::Keys, press(|p| &mut p.card)), [Hear(4, CALLS[1].0, CALLS[1].1)]);
        m.step(Dev::Keys, ok);
        assert_eq!((m.umpire, m.picking), (4, None));
    }

    /// B212: Sets and Games are picked like the court and umpire: ←→ on the unpicked row do nothing, ✕ picks the pill,
    /// ←→ step it and stop at the ends (the original's 1/3/5 and 2/4/6), ✕ keeps, ○ puts back; a tap picks it too,
    /// its thirds step it and keep it, a tap off puts it back.
    #[test]
    fn sets_and_games_pickers() {
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], sel: 2, sets: 0, games: 4, ..default() };
        let [left, right, ok, back] = [press(|p| &mut p.left), press(|p| &mut p.right), press(|p| &mut p.ok), press(|p| &mut p.back)];
        assert_eq!((m.sets, m.picking), (0, None));
        m.step(Dev::Keys, ok);
        assert_eq!(m.picking, Some(0));
        assert_eq!(m.step(Dev::Keys, left), [], "1 set is the end");
        let walk: Vec<usize> = (0..3).map(|_| (m.step(Dev::Keys, right), m.sets).1).collect();
        assert_eq!(walk, [1, 2, 2]);
        m.step(Dev::Keys, ok);
        assert_eq!((m.sets, m.picking, m.screen), (2, None, Screen::Confirm));
        // B449: → from Sets is Games (the strip's next cell)
        m.step(Dev::Keys, right);
        assert_eq!(m.sel, 3);
        m.step(Dev::Keys, ok);
        let walk: Vec<i32> = (0..3).map(|_| (m.step(Dev::Keys, right), m.games).1).collect();
        assert_eq!(walk, [6, 6, 6]);
        let walk: Vec<i32> = (0..3).map(|_| (m.step(Dev::Keys, left), m.games).1).collect();
        assert_eq!(walk, [4, 2, 2]);
        m.step(Dev::Keys, back);
        assert_eq!((m.games, m.picking, m.screen), (4, None, Screen::Confirm));
        // pointer: a tap on Sets picks it, its left third steps down, a tap off puts it back; the middle keeps
        let act = |m: &Menu, x: f32| {
            let [rx, ry, rw, rh] = confirm::rects(m)[2];
            pointer::tests::act_at(m, rx + rw * x, ry + rh / 2.0)
        };
        let tap = |m: &mut Menu, x: f32| {
            let a = act(m, x).unwrap_or(pointer::press(|k| k.back = true));
            m.point(Dev::Keys, a, true)
        };
        tap(&mut m, 0.1);
        assert_eq!((m.sel, m.picking, m.sets), (2, Some(2), 2), "the first tap only picks it");
        tap(&mut m, 0.1);
        assert_eq!(m.sets, 1);
        tap(&mut m, 2.0); // off the pill
        assert_eq!((m.sets, m.picking), (2, None));
        tap(&mut m, 0.5);
        tap(&mut m, 0.1);
        tap(&mut m, 0.5);
        assert_eq!((m.sets, m.picking), (1, None));
    }

    #[test]
    fn a_dropped_device_frees_its_seat() {
        let [pa, pb] = [5, 6].map(|n| Dev::Pad(Entity::from_raw_u32(n).unwrap()));
        let ok = press(|p| &mut p.ok);
        let mut m = Menu { screen: Screen::Assign, doubles: true, seats: [Some(Dev::Keys), None, None, None], ..default() };
        m.step(pa, ok);
        m.step(pb, ok);
        assert_eq!(m.seats, [Some(Dev::Keys), Some(pa), Some(pb), None]);
        // B238: the ✕ on 2P (tapped, played as 1P's device): its seat is free, another device joins it
        m.point(Dev::Keys, pointer::Act::Drop(1), true);
        assert_eq!(m.seats, [Some(Dev::Keys), None, Some(pb), None]);
        m.step(pa, ok);
        assert_eq!(m.seats[1], Some(pa));
        // on the local roster (B331): back to the computer, the next device's ✕ takes it
        m.step(Dev::Keys, press(|p| &mut p.name));
        assert_eq!(m.screen, Screen::Chars);
        m.step(pb, ok);
        assert!(m.players[2].ready);
        m.step(pb, press(|p| &mut p.back)); // ready → picking again
        m.step(pb, press(|p| &mut p.back)); // ○ off its seat
        // down to two devices the roster stays doubles: pb's seat goes back to the computer
        assert_eq!((m.doubles, m.seats), (true, [Some(Dev::Keys), Some(pa), None, None]));
        assert!(!m.players[2].ready);
        assert_eq!(m.driven(pb), None);
        m.step(pb, ok); // back into the free seat
        assert_eq!((m.doubles, m.seats), (true, [Some(Dev::Keys), Some(pa), Some(pb), None]));
        m.point(Dev::Keys, pointer::Act::Drop(1), true);
        assert_eq!((m.doubles, m.seats), (true, [Some(Dev::Keys), None, Some(pb), None]));
        m.step(pb, ok); // seated already: readies its own, doesn't take a seat
        assert!(m.players[2].ready);
        // 1P backing out of the roster: the main menu, the other seats freed
        m.step(Dev::Keys, press(|p| &mut p.back));
        assert_eq!((m.screen, m.seats), (Screen::Main, [Some(Dev::Keys), None, None, None]));
    }

    /// The keyboard (and the mouse, its pointer) takes 1P over from a pad, and the free seat 1P's focus or the mouse
    /// is on, not only the first free one; a driver that gave up its seat takes a free one back.
    #[test]
    fn the_keyboard_takes_1p_or_the_focused_free_seat() {
        let [pa, pb] = [5, 6].map(|n| Dev::Pad(Entity::from_raw_u32(n).unwrap()));
        let ok = press(|p| &mut p.ok);
        let mut m = Menu { screen: Screen::Assign, doubles: true, seats: [Some(pa), Some(pb), None, None], ..default() };
        m.step(Dev::Keys, ok); // focus on 1P: the keyboard takes it, the pad goes to the first free seat
        assert_eq!(m.seats, [Some(Dev::Keys), Some(pb), Some(pa), None]);
        assert_eq!(m.lead(), Some(Dev::Keys));
        // pb taken off 2P
        m.sel = 1;
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert_eq!(m.seats, [Some(Dev::Keys), None, Some(pa), None]);
        // a pad drives again: the mouse hovers 4P and clicks, the keyboard sits there (not on the first free, 2P)
        let mut m = Menu { screen: Screen::Assign, doubles: true, seats: [Some(pa), None, Some(pb), None], ..default() };
        m.point(pa, pointer::Act::Row(3, Press::default()), true);
        assert_eq!(m.seats, [Some(pa), None, Some(pb), Some(Dev::Keys)]);
        // 1P's focus on 2P, the keyboard's Enter moves it there
        m.sel = 1;
        m.step(Dev::Keys, ok);
        assert_eq!(m.seats, [Some(pa), Some(Dev::Keys), Some(pb), None]);
        // the driver moved itself off and was taken off: its ✕ on a free seat sits it back down
        let mut m = Menu { screen: Screen::Assign, doubles: true, seats: [Some(Dev::Keys), Some(pa), None, None], ..default() };
        m.step(Dev::Keys, ok); // pick up
        m.step(Dev::Keys, press(|p| &mut p.right)); // carry to 2P (pa up to 1P), the keyboard still drives
        m.step(Dev::Keys, ok); // put down
        m.step(Dev::Keys, press(|p| &mut p.page)); // take itself off 2P
        assert_eq!((m.seats, m.lead()), ([Some(pa), None, None, None], Some(Dev::Keys)));
        m.sel = 2;
        m.step(Dev::Keys, ok);
        assert_eq!(m.seats, [Some(pa), None, Some(Dev::Keys), None]);
    }

    #[test]
    fn dropping_1p_moves_the_next_device_up() {
        let pa = Dev::Pad(Entity::from_raw_u32(5).unwrap());
        let mut m = Menu { screen: Screen::Assign, seats: [Some(Dev::Keys), None, None, None], ..default() };
        assert!(!m.droppable(0)); // alone: stays
        m.step(pa, press(|p| &mut p.ok));
        assert!(m.point(Dev::Keys, pointer::Act::Drop(0), true).contains(&Effect::Sound(1)));
        assert_eq!(m.seats, [Some(pa), None, None, None]);
        m.sel = 1;
        m.step(Dev::Keys, press(|p| &mut p.ok)); // the keyboard rejoins as 2P (1P's focus on it)
        m.sel = 0;
        assert_eq!(m.seats[..2], [Some(pa), Some(Dev::Keys)]);
        // 1P (now the pad) focuses 2P and swaps it back up; B437: the pad drives on, its move pending, until it leaves
        m.step(pa, press(|p| &mut p.right));
        m.step(pa, press(|p| &mut p.ok));
        m.step(pa, press(|p| &mut p.left));
        assert_eq!((&m.seats[..2], m.sel, m.lead(), m.lead_pending()), (&[Some(Dev::Keys), Some(pa)][..], 0, Some(pa), Some(Some(1))));
        m.step(Dev::Keys, press(|p| &mut p.down));
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert_eq!((&m.seats[..2], m.sel), (&[Some(Dev::Keys), Some(pa)][..], 0), "the keyboard doesn't drive yet");
        // the pad takes itself off; an empty seat never goes to 1P
        m.step(pa, press(|p| &mut p.ok)); // put down
        m.step(pa, press(|p| &mut p.right));
        m.step(pa, press(|p| &mut p.page));
        assert_eq!((&m.seats[..2], m.lead_pending()), (&[Some(Dev::Keys), None][..], Some(None)));
        m.step(pa, press(|p| &mut p.right));
        assert_eq!(m.seats[0], Some(Dev::Keys));
        m.step(pa, press(|p| &mut p.ok));
        assert_eq!((m.screen, m.lead()), (Screen::Chars, Some(Dev::Keys)));
        assert!(!m.droppable(0)); // the select's driver stays
    }

    #[test]
    fn keyboard_p1_ignores_other_pads_in_the_menus() {
        let pad = Dev::Pad(Entity::from_raw_u32(3).unwrap());
        let mut m = Menu { screen: Screen::Press, ..default() };
        m.step(Dev::Keys, press(|p| &mut p.ok));
        m.screen = Screen::Main;
        // the pad neither moves the main menu nor picks its row
        m.step(pad, press(|p| &mut p.down));
        assert_eq!(m.sel, 0);
        m.step(pad, press(|p| &mut p.ok));
        assert_eq!(m.screen, Screen::Main);
        // the keyboard starts Local as 1P; on the select the pad still joins as 2P
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!((m.screen, m.seats[0]), (Screen::Chars, Some(Dev::Keys)));
        m.step(pad, press(|p| &mut p.ok));
        assert_eq!(m.seats[..2], [Some(Dev::Keys), Some(pad)]);
    }

    #[test]
    fn four_humans_any_device_any_seat() {
        let [p0, p1, p2] = [3, 9, 11].map(|n| Dev::Pad(Entity::from_raw_u32(n).unwrap()));
        let ok = press(|p| &mut p.ok);
        let mut m = Menu::default();
        m.step(p1, ok); // a pad chose Local: it is 1P, on the roster (B331)
        assert_eq!(m.screen, Screen::Chars);
        // each device's ✕ joins the next free seat; B361: the third makes it doubles, 2P going across the net to 3P
        m.step(p2, ok);
        assert_eq!((m.doubles, m.seats), (false, [Some(p1), Some(p2), None, None]));
        m.step(p0, ok);
        assert_eq!((m.doubles, m.seats), (true, [Some(p1), Some(p0), Some(p2), None]));
        m.step(Dev::Keys, ok);
        assert_eq!(m.seats, [Some(p1), Some(p0), Some(p2), Some(Dev::Keys)]);
        // every seat is a human: each readies its own player, 1P never drives another
        for (i, d) in [p1, p0, p2, Dev::Keys].into_iter().enumerate() {
            assert_eq!(m.driven(d), Some(i));
            m.step(d, press(|p| &mut p.next));
            for _ in 0..i {
                m.step(d, press(|p| &mut p.right));
            }
            m.step(d, ok);
        }
        assert_eq!(m.screen, Screen::Confirm);
        let a = m.args(&[3, 9, 11].map(|n| Entity::from_raw_u32(n).unwrap()));
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        assert_eq!(flag("--pads"), "1,0,2,k");
        // the match reads them back
        let seats: Vec<_> = flag("--pads").split(',').map(Seat::parse).collect();
        assert_eq!(seats, [Some(Seat::Pad(1)), Some(Seat::Pad(0)), Some(Seat::Pad(2)), Some(Seat::Keys)]);
        // singles: the keyboard can be 2P
        let mut m = Menu::default();
        m.step(p0, ok);
        m.step(Dev::Keys, ok);
        assert_eq!((m.doubles, m.seats), (false, [Some(p0), Some(Dev::Keys), None, None]));
        assert_eq!(m.args(&[]).iter().skip_while(|x| *x != "--pads").nth(1).unwrap(), "-,k");
    }

    /// B368: the select's player columns at any aspect, 1 to 4 of them: side by side in the view, apart, over the
    /// legend strip; each part of a column inside its card, in order down it; tiles square on screen, filling the grid's
    /// width, at least a row on view; at 16:9 13 / 6 / 4 / 3 a row (the mockup's).
    #[test]
    fn select_fits_the_window() {
        use super::super::widescreen::{share, vplace, vshare};
        let inside = |r: [f32; 4], c: [f32; 4]| r[0] >= c[0] - 1e-3 && r[1] >= c[1] - 1e-3 && r[0] + r[2] <= c[0] + c[2] + 1e-3 && r[1] + r[3] <= c[1] + c[3] + 1e-3;
        for a in [0.4, 9.0 / 16.0, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
            let (ex, ey) = ((640.0 / share(a) - 640.0) / 2.0, (448.0 / vshare(a) - 448.0) / 2.0);
            let legend = vplace(436.0, 12.0, a, Anchor::Span).0;
            let g = Grid::fit(a);
            for n in 1..=4 {
                let c = roster::columns(g.view, n);
                assert!(c.rows >= 1 && c.cols >= 1, "{a} {n} {c:?}");
                assert!(((c.tile[0] / roster::PX) - (c.tile[1] / roster::PY)).abs() < 1e-3, "{a} {n}: square tiles");
                for p in 0..n {
                    let at = c.parts(p);
                    let card = at.card;
                    assert!(card[0] >= -ex && card[0] + card[2] <= 640.0 + ex + 1e-3 && card[1] >= -ey, "{a} {n} {card:?}");
                    let (t, h) = vplace(card[1], card[3], a, Anchor::Centre);
                    assert!(t + h <= legend + 1e-3, "{a} {n} {card:?}");
                    let rows = [at.head, at.pills, at.preview, at.grid, at.foot];
                    for (i, r) in rows.iter().enumerate() {
                        assert!(inside(*r, card) && r[3] > 0.0, "{a} {n} {p} part {i} {r:?} in {card:?}");
                        if i > 0 {
                            assert!(rows[i - 1][1] + rows[i - 1][3] <= r[1] + 1e-3, "{a} {n} {p} part {i} overlaps");
                        }
                    }
                    assert!(inside(at.bar, card) && at.grid[0] + at.grid[2] < at.bar[0], "{a} {n} {p}");
                    // B382: every heading and tile on view inside the grid, a full row across its width
                    let secs = vec![tabs::Section { id: 2, name: "A".into(), cells: vec![None; 3] }, tabs::Section { id: 3, name: "B".into(), cells: vec![None; 60] }];
                    let lines = tabs::lines(&secs, c.cols);
                    let v = roster::View { end: c.fit(&lines, 0), secs, lines, top: 0 };
                    let (heads, tiles) = c.laid(p, &v);
                    assert!(!tiles.is_empty() && !heads.is_empty(), "{a} {n} {p}");
                    for r in heads.iter().map(|h| h.1).chain(tiles.iter().map(|t| t.2)) {
                        assert!(inside(r, at.grid), "{a} {n} {p}: {r:?} in {:?}", at.grid);
                    }
                    let right = tiles.iter().filter(|t| t.0 >= 3).nth(c.cols - 1).map(|t| t.2);
                    if let Some(right) = right {
                        assert!((right[0] + right[2] - (at.grid[0] + at.grid[2])).abs() < 1e-2, "{a} {n}: the tiles fill the width");
                    }
                    if p > 0 {
                        let prev = c.parts(p - 1).card;
                        assert!(prev[0] + prev[2] < card[0], "{a} {n} {p}: apart");
                    }
                }
            }
            if (a - 16.0 / 9.0).abs() < 1e-3 {
                assert_eq!((1..=4).map(|n| roster::columns(g.view, n).cols).collect::<Vec<_>>(), [13, 6, 4, 3]);
            }
        }
    }

    /// B306: the roster scrolls only as far as keeps the shown player's tile in view; B368: each column on its own, the
    /// wheel / a drag moving one column's view and no cursor.
    #[test]
    fn roster_scrolls_to_the_cursor() {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let mut m = Menu { screen: Screen::Chars, mods: (0..100).map(fake_mod).map(|m| crate::mods::Mod { costumes: vec!["a".into()], ..m }).collect(), seats: [Some(Dev::Keys), Some(pad), None, None], grid: Grid::fit(4.0 / 3.0), ..default() };
        // B382: both on the first mod, its section's heading the top line on view
        for p in 0..2 {
            m.put(p, Some(Who::Mod(0)));
            m.follow(p);
        }
        let c = m.columns();
        let v = m.view(0);
        let r = v.lines.iter().position(|&l| l == tabs::Line::Head(1)).unwrap();
        let t = v.top;
        assert!(t <= r && r + 1 < v.end && m.scroll[1] == t, "{v:?}");
        assert!(v.end < v.lines.len());
        let down = |m: &mut Menu| {
            m.step(Dev::Keys, press(|p| &mut p.down));
        };
        for _ in r + 2..v.end {
            down(&mut m);
        }
        assert_eq!(m.scroll, [t, t, 0, 0], "still on the first view");
        down(&mut m);
        assert_eq!(m.scroll, [t + 1, t, 0, 0], "1P's column only");
        let start = c.laid(0, &m.view(0)).1[0].0;
        let acts = hits_chars(&m);
        assert!(acts.contains(&pointer::Act::Cell(0, start)) && !acts.contains(&pointer::Act::Cell(0, start - 1)));
        assert!(acts.contains(&pointer::Act::Section(1, 1)), "2P's heading still on view");
        m.step(Dev::Keys, press(|p| &mut p.up));
        assert_eq!(m.scroll[0], t + 1, "moving back up inside the view doesn't scroll");
        // 2P's column scrolled by the wheel: its own view moves, its pick and 1P's don't
        let (was, mine) = (m.cell(1), m.scroll[0]);
        assert_eq!(m.point(pad, pointer::Act::Scroll(1, 3), false), [Sound(0)]);
        assert_eq!((m.scroll[1], m.cell(1), m.scroll[0]), (t + 3, was, mine));
        assert_eq!(m.point(Dev::Keys, pointer::Act::Scroll(1, 1), false), [], "1P's device doesn't scroll 2P's column");
        assert!(m.scroll_by(1, -100) && m.scroll[1] == 0 && !m.scroll_by(1, -1), "not past the top");
        let last = c.last_top(&v.lines);
        m.scroll_by(1, 1000);
        assert_eq!(m.scroll[1], last, "not past the bottom");
        assert_eq!(m.view(1).end, v.lines.len());
        // a tap on a tile there picks it
        let (k, cell, _) = c.laid(1, &m.view(1)).1[1];
        m.point(pad, pointer::Act::Cell(1, k), false);
        assert_eq!(m.cell(1), cell);
    }

    fn hits_chars(m: &Menu) -> Vec<pointer::Act> {
        pointer::tests::hits(m).0.into_iter().map(|h| h.act).collect()
    }

    /// B295: the lobby and confirm screens at any aspect: the seats column and side card in the view, apart, over the
    /// legend strip; every button on the side card, none overlapping (B370: the lobby's items in the view, apart).
    #[test]
    fn lobby_and_confirm_fit_the_window() {
        use super::super::widescreen::{share, vplace, vshare};
        let inside = |r: [f32; 4], c: [f32; 4]| r[0] >= c[0] - 1e-3 && r[1] >= c[1] - 1e-3 && r[0] + r[2] <= c[0] + c[2] + 1e-3 && r[1] + r[3] <= c[1] + c[3] + 1e-3;
        let overlap = |a: [f32; 4], b: [f32; 4]| a[0] < b[0] + b[2] - 1e-3 && b[0] < a[0] + a[2] - 1e-3 && a[1] < b[1] + b[3] - 1e-3 && b[1] < a[1] + a[3] - 1e-3;
        for a in [0.4, 9.0 / 16.0, 0.99, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
            let g = Grid::fit(a);
            let (ex, ey) = ((640.0 / share(a) - 640.0) / 2.0, (448.0 / vshare(a) - 448.0) / 2.0);
            let legend = vplace(436.0, 12.0, a, Anchor::Span).0;
            let mut m = Menu { screen: Screen::Confirm, grid: g, ..default() };
            // B397: the confirm screen's rows in the view, over the legend strip, apart
            let rows = confirm::rects(&m);
            for &r in &rows {
                assert!(r[0] >= -ex && r[0] + r[2] <= 640.0 + ex + 1e-3 && r[1] >= -ey && r[2] > 0.0, "{a} {r:?}");
                let (t, h) = vplace(r[1], r[3], a, Anchor::Centre);
                assert!(t + h <= legend + 1e-3, "{a} {r:?}");
            }
            assert!((0..5).all(|i| (0..i).all(|j| !overlap(rows[i], rows[j]))), "{a} {rows:?}");
            // the lobby, hosting with a computer player on 4P, and as a guest
            m.screen = Screen::Lobby;
            online::sample(&mut m);
            m.net.room.as_mut().unwrap().seats[3] = Some(crate::net::Seat { peer: crate::net::CPU.into(), pick: crate::net::Pick::disc(9, 4) });
            for hosting in [true, false] {
                m.net.hosting = hosting;
                let rects = m.lobby_rects();
                // M121: the lobby spans the view like the select it holds
                let [l, t, r, b] = g.view;
                assert!(rects.iter().all(|&x| inside(x, [l, t, r - l, b - t])), "{a} {hosting} {rects:?}");
                for i in 0..rects.len() {
                    for j in 0..i {
                        // B396: a seat's ✕ sits on its own card
                        // B449: a seat's X sits on its own plate
                        assert!(!overlap(rects[i], rects[j]) || inside(rects[i], rects[j]) || inside(rects[j], rects[i]), "{a} {hosting} {i} {j} {rects:?}");
                    }
                }
            }
        }
    }

    /// B296: the busiest select (four humans, a hundred mods, every aspect) draws within the menu's quad pool.
    #[test]
    fn select_fits_the_pool() {
        let t = pointer::tests::text_with_mods(100);
        for a in [4.0, 21.0 / 9.0, 16.0 / 9.0, 4.0 / 3.0, 9.0 / 16.0, 0.4] {
            let mut m = Menu { screen: Screen::Chars, doubles: true, mods: (0..100).map(|i| crate::mods::Mod { name: "Longer Name".into(), source: format!("Game {}", i % 7), ..fake_mod(i % 14) }).collect(), seats: [Some(Dev::Keys); 4], grid: Grid::fit(a), ..default() };
            m.players[3].ready = true;
            let n = drawn(&m, &t, &Bindings::default(), &[], 0, 0.0).out.len();
            assert!(n < POOL * 9 / 10, "{a}: {n} quads");
        }
    }

    /// B306: one roster, no filters: Random, the disc's 14, every mod. B375: 1P's □ / Tab swaps Singles ↔ Doubles,
    /// 2P going to 3P (across the net) and back; nobody else's does (B361: they change it by joining and leaving).
    #[test]
    fn one_roster() {
        let from = |i: usize, s: &str| crate::mods::Mod { id: format!("m{i}"), source: s.into(), costumes: vec!["a".into()], ..fake_mod(i) };
        let mut m = Menu { screen: Screen::Chars, mods: vec![from(0, "Golf"), from(1, ""), from(2, "Get a Grip"), from(3, "Golf")], seats: [Some(Dev::Keys), Some(Dev::Keys), None, None], ..default() };
        let r = m.roster();
        assert_eq!(r.len(), 1 + CHARS + 4);
        assert_eq!((r[0], r[1], r[CHARS + 1]), (None, Some(Who::Disc(GRID[0].0)), Some(Who::Mod(0))));
        let was = m.players;
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert_eq!((m.doubles, m.seats), (true, [Some(Dev::Keys), None, Some(Dev::Keys), None]));
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert_eq!((m.doubles, m.seats), (false, [Some(Dev::Keys), Some(Dev::Keys), None, None]));
        assert_eq!(m.players[..2], was[..2]);
        // only 1P's device swaps: 2P's □ doesn't
        let pad = Dev::Pad(Entity::from_raw_u32(5).unwrap());
        m.seats[1] = Some(pad);
        m.step(pad, press(|p| &mut p.page));
        assert_eq!((m.doubles, m.seats), (false, [Some(Dev::Keys), Some(pad), None, None]));
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert_eq!((m.doubles, m.seats), (true, [Some(Dev::Keys), None, Some(pad), None]));
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert_eq!((m.doubles, m.seats), (false, [Some(Dev::Keys), Some(pad), None, None]));
        // the shown player: the one who pressed last while picking
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.shown(), 0);
    }

    /// The custom page: Tab switches it, a mod's costumes wrap at its own count, a readied mod costume is closed to
    /// the others, and the match gets the donor in `--chars` and the mod's folder per slot.
    #[test]
    fn custom_page_picks_a_mod() {
        let fake = |id: &str, donor: usize, costumes: usize| crate::mods::Mod {
            dir: format!("mods/{id}").into(),
            id: id.into(),
            name: id.into(),
            source: String::new(),
            costumes: (0..costumes).map(|c| format!("model/c{c:02}.glb")).collect(),
            donor,
            hand: 1.0,
            base: donor,
            overrides: vec![],
            ai_row: donor,
            texture_face: false,
            no_face: false,
            voice: "voice/".into(),
            motions: None,
            costume_of: None,
            skeleton: None,
            upscale: true,
        };
        let ok = press(|p| &mut p.ok);
        let mut m = Menu { mods: vec![fake("a", 5, 2), fake("b", 9, 1)], ..default() };
        m.step(Dev::Keys, ok); // Local, singles
        add_com(&mut m);
        // B306: the mods follow the disc roster on the one grid; everyone onto the first mod
        m.put(0, Some(Who::Mod(0)));
        m.put(1, Some(Who::Mod(0)));
        assert!(m.players[0].modded && m.players[1].modded);
        m.step(Dev::Keys, press(|p| &mut p.next));
        m.step(Dev::Keys, press(|p| &mut p.next));
        assert_eq!(m.players[0].costume, 0, "mod a has 2 costumes");
        m.step(Dev::Keys, ok); // 1P: a, costume 1
        // the computer's 2P on a costume 1 too: moved on to costume 2; down to b, then back up: costume 2 again
        assert!(!m.blocked(1));
        assert_eq!(m.players[1].costume, 1);
        m.step(Dev::Keys, press(|p| &mut p.down));
        assert_eq!(m.who(1), Who::Mod(1));
        m.step(Dev::Keys, press(|p| &mut p.down)); // the list ends
        assert_eq!(m.who(1), Who::Mod(1));
        m.step(Dev::Keys, press(|p| &mut p.left));
        m.step(Dev::Keys, press(|p| &mut p.prev));
        assert_eq!(m.players[1].costume, 1, "L1 passes the taken costume 1");
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Confirm);
        let a = m.args(&[]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        assert_eq!((flag("--chars"), flag("--outfits")), ("5,5".into(), "0,1".into()));
        let mods: Vec<_> = a.windows(3).filter(|w| w[0] == "--slot-mod").map(|w| (w[1].clone(), w[2].clone())).collect();
        assert_eq!(mods, [("0".into(), "mods/a".into()), ("1".into(), "mods/a".into())]);
        // back from the confirm: everyone picks again where they were
        m.step(Dev::Keys, press(|p| &mut p.back));
        assert!(m.players[0].modded && !m.players[0].ready);
        // M1n: a costume whose model failed to load is passed by L1 / R1 and can't be readied on
        let mut m = Menu { mods: vec![fake("c", 3, 3)], ..default() };
        m.step(Dev::Keys, ok);
        m.put(0, Some(Who::Mod(0)));
        m.broken.insert((0, 1));
        m.step(Dev::Keys, press(|p| &mut p.next));
        assert_eq!(m.players[0].costume, 2);
        m.step(Dev::Keys, press(|p| &mut p.next));
        m.step(Dev::Keys, press(|p| &mut p.next));
        assert_eq!(m.players[0].costume, 2, "0 → 2 → 0 → 2");
        m.players[0].costume = 1;
        assert!(m.blocked(0));
    }

    /// M1: a costume pack (`costume_of`) adds its costumes after the disc character's own on L1 / R1; picked, the match
    /// plays that disc character in the pack's model (`--slot-mod`, the pack's costume number).
    #[test]
    fn costume_pack_extends_a_disc_character() {
        let pack = |id: &str, of: usize, n: usize| crate::mods::Mod {
            dir: format!("mods/{id}").into(),
            id: id.into(),
            name: id.into(),
            source: String::new(),
            costumes: (0..n).map(|c| format!("model/c{c:02}.glb")).collect(),
            donor: of,
            hand: 1.0,
            base: of,
            overrides: vec![],
            ai_row: of,
            texture_face: false,
            no_face: false,
            voice: "voice/".into(),
            motions: None,
            costume_of: Some(of),
            skeleton: None,
            upscale: true,
        };
        // Ashley (grid 0, character 0) gets 2 + 1 more; Cody none
        let m = Menu { mods: vec![pack("x", 0, 2), pack("y", 1, 3), pack("z", 0, 1)], ..default() };
        assert_eq!((m.costumes(Who::Disc(0)), m.costumes(Who::Disc(1)), m.costumes(Who::Disc(2))), (13, 13, 10));
        assert_eq!(m.resolve(Who::Disc(0), 9), (Who::Disc(0), 9));
        assert_eq!(m.resolve(Who::Disc(0), 11), (Who::Mod(0), 1));
        assert_eq!(m.resolve(Who::Disc(0), 12), (Who::Mod(2), 0));
        let ok = press(|p| &mut p.ok);
        let mut m = m;
        m.step(Dev::Keys, ok);
        add_com(&mut m);
        m.step(Dev::Keys, press(|p| &mut p.right)); // B346: from Random onto Ashley
        m.step(Dev::Keys, press(|p| &mut p.prev)); // 0 → 12: pack z's only costume
        assert_eq!(m.pick(0), (Who::Mod(2), 0));
        m.step(Dev::Keys, ok);
        // the computer's 2P on the same pack costume: closed
        m.players[1].costume = 12;
        assert!(m.blocked(1));
        m.players[1].costume = 10;
        m.step(Dev::Keys, ok);
        let a = m.args(&[]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        assert_eq!((flag("--chars"), flag("--outfits")), ("0,0".into(), "0,0".into()));
        let mods: Vec<_> = a.windows(3).filter(|w| w[0] == "--slot-mod").map(|w| (w[1].clone(), w[2].clone())).collect();
        assert_eq!(mods, [("0".into(), "mods/z".into()), ("1".into(), "mods/x".into())]);
    }

    /// A third device makes the local roster doubles with 2P across the net; □ moves a non-1P human to the other
    /// team (onto a computer's seat, picks following), 1P's □ stays Singles / Doubles.
    #[test]
    fn doubles_team_switch() {
        let (ok, page) = (press(|p| &mut p.ok), press(|p| &mut p.page));
        let (a, b) = (Dev::Pad(Entity::from_raw_u32(1).unwrap()), Dev::Pad(Entity::from_raw_u32(2).unwrap()));
        let mut m = Menu::default();
        m.local_start(Dev::Keys, &mut Vec::new());
        m.step(a, ok);
        m.step(b, ok);
        assert!(m.doubles);
        assert_eq!(m.seats, [Some(Dev::Keys), Some(b), Some(a), None]);
        m.players[2].cursor = 5;
        m.step(a, page);
        assert_eq!(m.seats, [Some(Dev::Keys), Some(b), Some(a), None], "no computer seat on 1P's team: stays");
        m.step(b, page);
        assert_eq!(m.seats, [Some(Dev::Keys), None, Some(a), Some(b)], "b to the other team");
        m.step(a, page);
        assert_eq!((m.seats, m.players[1].cursor), ([Some(Dev::Keys), Some(a), None, Some(b)], 5), "a joins 1P, pick with it");
        m.step(Dev::Keys, page);
        assert!(!m.doubles, "1P's □ still swaps the mode");
    }

    /// B79: Select flips the hand. Ashley (right) + Select and Will (left) + Select go to the match as
    /// `--swap-hands 1,1`, and the match starts with her left-handed and him right-handed (the AI's strong side stays
    /// TParam's). The match half needs the ISO at the repository root (skipped without it).
    #[test]
    fn select_swaps_the_hand() {
        let ok = press(|p| &mut p.ok);
        let hand = press(|p| &mut p.hand);
        let mut m = Menu::default();
        m.step(Dev::Keys, ok);
        add_com(&mut m);
        m.step(Dev::Keys, press(|p| &mut p.right)); // B346: from Random onto Ashley
        assert_eq!(m.who(0), Who::Disc(0), "Ashley");
        m.step(Dev::Keys, hand);
        m.step(Dev::Keys, hand);
        assert!(!m.players[0].swap, "a second press flips it back");
        m.step(Dev::Keys, hand);
        m.step(Dev::Keys, ok);
        assert!(m.players[0].swap && m.players[0].ready);
        // 1P drives the computer's 2P now: Will
        m.players[1].cursor = 13;
        assert_eq!(m.who(1), Who::Disc(11));
        m.step(Dev::Keys, hand);
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Confirm);
        let a = m.args(&[]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        assert_eq!((flag("--chars"), flag("--swap-hands")), ("0,11".into(), "1,1".into()));
        assert!(!Menu::default().args(&[]).contains(&"--swap-hands".to_string()));

        const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, match half skipped");
        }
        let cli = crate::parse(std::iter::once(ISO.to_string()).chain(a));
        // every player a bot
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(cli.args).insert_resource(cli.swap_hands);
        app.init_resource::<super::super::Pads>().init_resource::<super::super::tick_input::TickPads>();
        app.world_mut().spawn((crate::GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, super::super::setup);
        app.update();
        let g = app.world().resource::<super::super::Game>();
        assert_eq!(g.chars[..2], [0, 11]);
        assert_eq!([g.players[0].hand, g.players[1].hand], [-1.0, 1.0], "Ashley left-handed, Will right-handed");
        assert_eq!([g.reaches[0].hand, g.reaches[1].hand], [-1.0, 1.0], "the contact search's forehand side follows");
        assert_eq!(g.strong_hand[..2], [1.0, -1.0], "the AI's strong side is TParam's");
    }

    #[test]
    fn real_pad_takes_touch_seat() {
        let [t, a, b] = [4, 5, 6].map(|n| Entity::from_raw_u32(n).unwrap());
        let mut m = Menu { seats: [Some(Dev::Keys), None, None, None], ..default() };
        m.pad_takes_touch_seat(Dev::Pad(a), &[a]);
        assert_eq!(m.seats[..2], [Some(Dev::Pad(a)), None]);
        // a second real pad doesn't take it from the first
        m.pad_takes_touch_seat(Dev::Pad(b), &[a, b]);
        assert_eq!(m.seats[..2], [Some(Dev::Pad(a)), None]);
        // 1P on the on-screen pad, hidden once a real one is in
        m.seats = [Some(Dev::Pad(t)), None, None, None];
        m.pad_takes_touch_seat(Dev::Pad(a), &[a]);
        assert_eq!(m.seats[0], Some(Dev::Pad(a)));
        // a pad already on 2P stays there
        m.seats = [Some(Dev::Keys), Some(Dev::Pad(b)), None, None];
        m.pad_takes_touch_seat(Dev::Pad(b), &[b]);
        assert_eq!(m.seats[..2], [Some(Dev::Keys), Some(Dev::Pad(b))]);
    }

    #[test]
    fn messages_parse() {
        let mut d = vec![2, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0];
        d.extend(b"\x07\x7fA\0\x07\x7f\xe9\0");
        assert_eq!(messages(&d), ["A", "é"]);
    }

    /// The COM cards' rank pill and the hand tag, against the original's doubles select in PCSX2 (2026-10-12, slot
    /// 2): JJ in costume 1 and 2 "Tennis Lv 4", Cody in costume 1 "Tennis Lv 5"; Carol and Will left-handed.
    #[test]
    fn com_ranks_and_hands() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("disc missing, skipped");
        };
        let pc = iso.read("PCDATA/PCDATA.XB").unwrap();
        let pc = Archive::parse(&pc).unwrap();
        let csv = |name: &str| pc.read(pc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(name)).unwrap()).unwrap();
        let ranks: Vec<u8> = hst_sim::ai::AiParams::table(&csv("aiparam.csv")).iter().map(|r| r.rank).collect();
        assert_eq!([(5, 1), (5, 0), (1, 0)].map(|(c, k)| com_rank(&ranks, c, k, true)), [1, 1, 0]);
        let hands = hands(&csv("tparam.csv"));
        assert_eq!((0..14).filter(|&c| hands[c] < 0.0).collect::<Vec<_>>(), [6, 11]);
    }
}
