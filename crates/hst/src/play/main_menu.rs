//! The main menu (remaster-only flow in the original menus' art): Local, Online, Settings and Quit (not on web). Local goes through
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
//! picked by player 1 after the humans.
//! `HST_MENU=<screen>` (main, settings, graphics, sound, controls, mode, assign, chars, coms = chars with 1P ready (the COM cards), coms-random / confirm-random = 2P's computer readied on Random, chars-random = 1P on the Random card, lobby-bot = the host's lobby with a computer player, lobby-wait = the host's lobby waiting on a player choosing about a mod (B281), chars4 = four humans on one portrait, mods = the roster filtered to the mods, confirm) opens on that screen (for `--shot`).
//!
//! Custom characters (remaster-only): every standard mod under `mods/` beside the disc image (`mods::list`) is on a
//! second page of the select, switched for everyone by Tab / □: B133: a grid like the disc characters', as many
//! columns and rows a page as the window fits (B190, [`Grid`]) in one framed group, each a face icon (`inspect::face_icon`, cut from the mod's baked
//! costume-0 preview, blank until that's baked) with its name under it, the players' hands and tags on it as on the
//! disc grid (←→ one, ↑↓ a row). A card on a mod shows its 3D preview, its name in `word.tm2`, its TParam.csv row's
//! play style and (△) its power and footwork numbers; its costumes are `mod.json`'s. Picked, it plays that match slot
//! (`--slot-mod N DIR`, `--chars` the donor), and a costume readied on is closed to the others as on the disc.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::sound;
use std::sync::Arc;

use super::controls::{self, Action, Bindings, Seat};
use super::panel;
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
mod online;
mod options;
mod mods_screen;
mod pointer;
mod trace;
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_imports))]
pub(crate) use mods_screen::root as mods_root;
pub(super) mod pause;
mod previews;
mod slide;

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

/// Height of a roster cell's row on the select (B296), and its box.
const CELL_PITCH: f32 = 60.0;
const CELL_MIN_W: f32 = 54.0;
const MAX_CELLS: usize = 64;

/// B296: the character select's layout on the window's view, in the centred 640×448 space reaching past it on the
/// sides (and into a tall window's letterbox): the seat strip across the top, the roster under it with the detail
/// panel beside it (below it in a portrait window), the roster's columns and rows as many as fit.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Grid {
    cols: usize,
    rows: usize,
    strip: [f32; 4],
    roster: [f32; 4],
    detail: [f32; 4],
}

impl Default for Grid {
    fn default() -> Self {
        Grid::fit(4.0 / 3.0)
    }
}

impl Grid {
    /// The layout on a view `aspect` wide.
    fn fit(aspect: f32) -> Self {
        use super::widescreen::{share, vshare};
        // how far the view reaches past the 4:3 screen on each side, in its units
        let ex = (640.0 / share(aspect) - 640.0) / 2.0;
        let ey = (448.0 / vshare(aspect) - 448.0) / 2.0;
        let (left, right, top, bottom) = (12.0 - ex, 628.0 + ex, -ey, 398.0 + ey);
        let w = right - left;
        // under the title tab, over the description bar (pinned to the window's bottom)
        let strip = [left, top + 46.0, w, 70.0];
        let y = top + 124.0;
        let (mut roster, mut detail) = if aspect >= 1.0 {
            let dw = 200.0f32.min(w * 0.4);
            ([left, y, w - dw - 8.0, bottom - y], [right - dw, y, dw, bottom - y])
        } else {
            ([left, y, w, bottom - y - 178.0], [left, bottom - 170.0, w, 170.0])
        };
        // ponytail: at most MAX_CELLS a page (wider cells past 16 columns) so a page's quads fit the menu's `POOL`
        let cols = (((roster[2] - 16.0 + 4.0) / CELL_MIN_W) as usize).clamp(1, 16);
        let rows = (((roster[3] - 60.0 + 4.0) / CELL_PITCH) as usize).clamp(1, (MAX_CELLS / cols).max(1));
        if aspect < 1.0 {
            // a portrait window's leftover height goes to the detail panel
            roster[3] = 60.0 + CELL_PITCH * rows as f32;
            detail = [left, y + roster[3] + 8.0, w, bottom - y - roster[3] - 8.0];
        }
        Grid { cols, rows, strip, roster, detail }
    }

    fn per(&self) -> usize {
        self.cols * self.rows
    }

    /// The box of the roster's `k`th cell on its page.
    fn cell(&self, k: usize) -> [f32; 4] {
        let [x, y, w, _] = self.roster;
        let pitch = (w - 16.0) / self.cols as f32;
        let k = k % self.per();
        [x + 8.0 + pitch * (k % self.cols) as f32 + 2.0, y + 36.0 + CELL_PITCH * (k / self.cols) as f32, pitch - 4.0, CELL_PITCH - 4.0]
    }
}

/// Keeps `Menu::grid` fitted to the window, live on resize; applies the Aspect Ratio setting live (B117).
fn fit_grid(window: Query<&Window, With<PrimaryWindow>>, mut menu: ResMut<Menu>) {
    super::widescreen::ASPECT.store(menu.settings.3, std::sync::atomic::Ordering::Relaxed);
    super::key_hints::ON.store(menu.settings.0[KEY_HINTS], std::sync::atomic::Ordering::Relaxed);
    let Ok(w) = window.single() else { return };
    let g = Grid::fit(super::widescreen::aspect(w));
    if menu.grid != g {
        menu.grid = g;
    }
}
/// The original's courts on the confirm screen (names `message0` 276 + 2k, descriptions 581 + 2k).
const COURTS: usize = 11;
/// Each court's surface on the confirm screen (0 grass, 1 clay, 2 hard; `hst_sim::ball::COURTS` bounces the same).
const SURFACE: [usize; COURTS] = [1, 1, 1, 2, 0, 0, 2, 0, 1, 0, 2];
/// The confirm screen's ball meters per surface: bounce height and ball speed, out of 4.
const METERS: [(usize, usize); 3] = [(2, 4), (3, 2), (4, 3)];
/// The umpires; the value after the last court or umpire is a random pick (`Menu::venue`).
const UMPIRES: usize = 5;
/// The court picker's strip order (the original's tiles 1–11 as courts k), then our Random.
const STRIP: [usize; COURTS] = [0, 3, 5, 1, 7, 8, 10, 9, 2, 6, 4];
/// Sets on the confirm screen (sets played; to win: (n + 1) / 2).
const SETS: [i32; 3] = [1, 3, 5];
/// Games to win a set on the confirm screen: the original's 2, 4 or 6 (its option steps by 2; ours wraps round like Sets).
const GAMES: [i32; 3] = [2, 4, 6];
/// Player colours: 1P pink, 2P orange, 3P blue, 4P green (`CharacterSelect2_03`'s cards, same order).
const COLOURS: [[f32; 3]; 4] = [[128.0, 60.0, 90.0], [128.0, 80.0, 20.0], [40.0, 90.0, 128.0], [70.0, 115.0, 30.0]];
/// `word.tm2`'s glyphs, 12 to a 32-px row from y 32.
pub(super) const FONT: [&str; 15] = [
    "aàáâãäåæbcçd",
    "ðeèéêëfghiìí",
    "îïjklmnñoòóô",
    "õöøœpqrsštuù",
    "úûũüvwxyýÿz°",
    "AÀÁÂÃÄÅÆBCÇD",
    "ÐEÈÉÊËFGHIÌÍ",
    "ÎÏJKLMNÑOÒÓÔ",
    "ÕÖØŒPQRSŠTUÙ",
    "ÚÛŨÜVWXYÝŸZ°",
    "1234567890-=",
    "˜€?!\"#£%&`'^",
    "_[]().®©™¿¡„",
    ":;+*@{},<>/¥",
    "«»$ß",
];

// ---------------------------------------------------------------- settings

/// The enhancements, each on or off (all on by default), the graphics settings, the volumes (B235) and the aspect
/// ratio (B117, a `widescreen::ASPECTS` index) and what to do with an online lobby's mods (B289: the lobby's
/// prompt asks, or its "Download, always" / "Never download" answer), saved in `settings.txt`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Settings([bool; 6], Graphics, Volumes, u8, Ask);
/// `music` is the match's (an older `settings.txt`'s one Music switch keeps meaning the match). Their rows are on the
/// Settings screen's pages (`options.rs`).
const SETTING_NAMES: [&str; 6] = ["upscaled_textures", "menu_music", "music", "upscaled_downloads", "key_hints", "chat"];
/// Upscaled Downloads (B227): the mod store downloads mods' `textures/upscaled/` sets (≈ 90% of a mod's size). It
/// follows Upscaled Textures (when settings.txt lacks it, and when that switch is flipped) unless switched itself.
const UPSCALED_DOWNLOADS: usize = 3;
/// B301: the match's keyboard hints (`play::key_hints`).
const KEY_HINTS: usize = 4;
/// M20: online chat shown (`crate::chat`).
const CHAT: usize = 5;

/// Upscaled textures and menu music on, match music off (user 2026-10-12); key hints on (B301); chat on (M20).
const SETTING_DEFAULTS: [bool; 6] = [true, true, false, true, true, true];

/// Auto, but 16:9 on a phone (B117: the browser played by touch).
fn default_aspect() -> u8 {
    if touch_page() { 2 } else { 0 }
}

impl Default for Settings {
    fn default() -> Self {
        Settings(SETTING_DEFAULTS, Graphics::default(), Volumes::default(), default_aspect(), Ask::Each)
    }
}

impl Settings {
    fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        let (mut downloads, mut aspect, mut wide) = (None, None, None);
        for (k, v) in text.lines().filter_map(|l| l.split('#').next()?.split_once('=')) {
            if k.trim() == "online_mods" {
                s.4 = Ask::parse(v).unwrap_or(s.4);
            } else if k.trim() == "aspect" {
                aspect = super::widescreen::ASPECTS.iter().position(|a| *a == v.trim()).map(|i| i as u8);
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
        s
    }

    fn text(&self) -> String {
        let on: String = SETTING_NAMES.iter().zip(self.0).map(|(n, on)| format!("{n} = {}\n", if on { "on" } else { "off" })).collect();
        format!("aspect = {}\n", super::widescreen::ASPECTS[self.3 as usize]) + &on + &format!("online_mods = {}\n", self.4.word()) + &self.1.text() + &self.2.text()
    }

    pub(crate) fn path(iso: &str) -> std::path::PathBuf {
        std::path::Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("settings.txt")
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
    /// △: the attribute card.
    card: bool,
    /// L1 / R1: the costume.
    prev: bool,
    next: bool,
    /// □ / Tab: the select roster's next filter (B296).
    page: bool,
    /// Select / − (keyboard: the camera key, C): the hand toggle.
    hand: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    /// `bgmm_04` plays on the first three, `bgmm_05` on the next three, `bgmm_06` on Confirm (the MENU overlay's track
    /// switch, measured per screen).
    Main,
    /// B290/B294: the Settings tabs (`options.rs`).
    Display,
    Graphics,
    /// B235: the volume sliders.
    Sound,
    Controls,
    /// Online & Mods: what the mod store and online lobbies download.
    ModSettings,
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
}

impl Player {
    fn char(&self) -> usize {
        GRID[self.cursor].0
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
    /// The custom roster.
    mods: Vec<crate::mods::Mod>,
    /// B296: the select's roster filter ([`Menu::filters`]): 0 everyone, 1 the disc's, 2 the mods.
    filter: usize,
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
            umpire: 4,
            roll: 0,
            rebinding: None,
            mods: Vec::new(),
            filter: 0,
            focus: 0,
            broken: default(),
            net: online::View::default(),
            flame: flame::Flame::default(),
            picking: None,
            grid: Grid::default(),
            add: default(),
            paused: false,
        }
    }
}

/// Rows per screen.
#[cfg(not(target_arch = "wasm32"))]
const MAIN: &[&str] = &["Local", "Online", "Settings", "Mods", "Quit"];
/// A web page can't close itself (an `AppExit` there only freezes it), so the browser build has no Quit.
#[cfg(target_arch = "wasm32")]
const MAIN: &[&str] = &["Local", "Online", "Settings", "Mods"];
const CONFIRM_ROWS: usize = 5;

/// What a step asks of the app.
#[derive(Debug, PartialEq)]
enum Effect {
    Sound(u8),
    SaveSettings,
    Launch,
    Quit,
    Online(online::Fx),
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
    fn draw(&self, p: usize, seed: u64) -> Option<(Who, usize)> {
        let whos = (0..CHARS).map(Who::Disc).chain((0..self.mods.len()).filter(|&i| self.mods[i].costume_of.is_none()).map(Who::Mod));
        let open: Vec<(Who, Vec<usize>)> =
            whos.map(|w| (w, (0..self.costumes(w)).filter(|&c| self.free(p, w, c)).collect::<Vec<_>>())).filter(|(_, c)| !c.is_empty()).collect();
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

    /// B296: how many filters □ / Tab cycles: everyone, the disc's, the mods (B304: no per-game ones; one with no
    /// mods).
    fn filters(&self) -> usize {
        if self.mods.is_empty() { 1 } else { 3 }
    }

    /// B296: the select's roster under the filter: the Random card (`None`) first, then the disc characters in the
    /// original's grid order (beginner, intermediate, expert), then the custom ones in folder order.
    fn roster(&self) -> Vec<Option<Who>> {
        let disc = self.filter <= 1 || self.mods.is_empty();
        let mods = (0..self.mods.len()).filter(|_| self.filter != 1);
        let disc = GRID.iter().map(|g| Who::Disc(g.0)).filter(|_| disc);
        std::iter::once(None).chain(disc.chain(mods.map(Who::Mod)).map(Some)).collect()
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

    /// B296: the roster filter `f`: a player not ready whose character it hides moves to its first open one.
    fn set_filter(&mut self, f: usize) {
        self.filter = f % self.filters();
        let roster = self.roster();
        for p in 0..self.players() {
            if !self.players[p].ready && !roster.contains(&self.cell(p)) && let Some(&c) = roster[1..].iter().find(|&&c| !self.full(p, c)) {
                self.put(p, c);
            }
        }
        self.skip_taken();
    }

    /// B296: the player the select's detail panel and description follow: the one who pressed last while picking,
    /// else the first not ready (online: the host's computer player being edited, else this machine's first).
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

    /// B296: the roster under the filter and where the shown player's page of it starts.
    fn page(&self) -> (usize, Vec<Option<Who>>) {
        let roster = self.roster();
        let at = roster.iter().position(|c| *c == self.cell(self.shown())).unwrap_or(0);
        (at / self.grid.per() * self.grid.per(), roster)
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
    /// human to keep (✕), change, or leave to the computer (○).
    fn preset_coms(&mut self) {
        let n = self.players();
        if (0..n).all(|q| self.seats[q].is_none() || self.players[q].ready) {
            for q in (0..n).filter(|&q| self.open(q) && !self.players[q].ready).collect::<Vec<_>>() {
                self.players[q] = self.auto_com(q);
            }
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
    }

    /// B238: whether seat `p` has a device that ✕ may take off: not this machine's first (the menu's driver), except
    /// on the assignment screen while another device is seated, which then moves up into it (changing 1P).
    fn droppable(&self, p: usize) -> bool {
        let dev = |s: &Option<Dev>| matches!(s, Some(Dev::Keys | Dev::Pad(_)));
        let first = self.seats.iter().position(Option::is_some) == Some(p);
        dev(&self.seats[p]) && (!first || self.paused || self.screen == Screen::Assign && self.seats.iter().filter(|s| dev(s)).count() > 1)
    }

    /// The seats 1P's ←/→ (`right`) trades on the assignment screen: the focused one and its neighbour that way,
    /// unless that would leave 1P empty or changes nothing.
    fn host_swap(&self, right: bool) -> Option<(usize, usize)> {
        let n = self.players();
        let p = self.sel.min(n - 1);
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
        let empties_1p = q == 0 && self.seats[p].is_none() || p == 0 && self.seats[q].is_none();
        (!empties_1p && self.seats[p] != self.seats[q]).then_some((p, q))
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
        self.seats[p] = if self.screen == Screen::Chars { Some(Dev::Open) } else { None };
        self.unready(p);
        true
    }

    /// One device's presses on the current screen.
    fn step(&mut self, dev: Dev, k: Press) -> Vec<Effect> {
        use Effect::*;
        let mut out = Vec::new();
        let rows = |n: usize, sel: &mut usize, out: &mut Vec<Effect>| {
            if k.up || k.down {
                *sel = if k.down { (*sel + 1) % n } else { (*sel + n - 1) % n };
                out.push(Sound(0));
            }
        };
        // a lobby link's start (`online::linked`): the first device to press anything is this machine's player
        if self.screen == Screen::Assign && self.online() && self.seats[0].is_none() {
            self.seats[0] = Some(dev);
        }
        let host = self.seats[0];
        if self.screen == Screen::Main && self.ask_step(k, &mut out) {
            return out;
        }
        match self.screen {
            Screen::Main => {
                rows(MAIN.len(), &mut self.sel, &mut out);
                if k.ok {
                    match self.sel {
                        0 => {
                            self.seats = [Some(dev), None, None, None];
                            self.go(Screen::Mode, &mut out);
                        }
                        1 => {
                            self.seats = [Some(dev), None, None, None];
                            self.net.status.clear();
                            out.push(Effect::Online(online::Fx::Load));
                            self.go(Screen::Online, &mut out);
                        }
                        2 => self.go(Screen::Display, &mut out),
                        3 => self.mods_open(&mut out),
                        _ => out.push(Quit),
                    }
                }
            }
            Screen::Display | Screen::Graphics | Screen::Sound | Screen::ModSettings => {
                rows(options::page(self.screen).len(), &mut self.sel, &mut out);
                self.step_page(k, &mut out);
            }
            Screen::Controls => {
                if self.rebinding.is_some() {
                    return out;
                }
                rows(controls::action_names().count(), &mut self.sel, &mut out);
                if k.ok {
                    self.rebinding = Some(self.sel);
                    out.push(Sound(2));
                } else {
                    self.step_tabs(k, &mut out);
                }
            }
            Screen::Mode if host == Some(dev) => {
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
                    self.sel = 0;
                    self.screen = Screen::Main;
                    out.push(Sound(1));
                }
            }
            Screen::Assign => {
                let n = self.players();
                let seated = self.seats[..n].iter().position(|s| *s == Some(dev));
                if host == Some(dev) {
                    if k.ok {
                        for p in &mut self.players {
                            *p = Player::default();
                        }
                        self.filter = 0;
                        if self.online() {
                            // B195: into the lobby first (seats held), then its character select
                            self.online_open(&mut out);
                        } else {
                            self.go(Screen::Chars, &mut out);
                        }
                    } else if k.up || k.down {
                        // 1P focuses a seat to swap or take off
                        rows(n, &mut self.sel, &mut out);
                    } else if k.left || k.right {
                        // the focused seat's device trades places with the next seat that way (1P is never left empty)
                        if let Some((p, q)) = self.host_swap(k.right) {
                            self.seats.swap(p, q);
                            if !self.paused {
                                self.bots.swap(p, q);
                            }
                            self.sel = q;
                            out.push(Sound(0));
                        }
                    } else if k.page && self.drop_seat(self.sel.min(n - 1)) {
                        out.push(Sound(1));
                    } else if k.card && self.can_bot(self.sel.min(n - 1)) {
                        // B216: △ adds a computer player to the focused empty seat (or takes it off)
                        let p = self.sel.min(n - 1);
                        self.bots[p] = !self.bots[p];
                        out.push(Sound(if self.bots[p] { 2 } else { 1 }));
                    } else if k.back {
                        self.seats[1..].fill(None);
                        self.bots = [false; 4];
                        self.screen = match self.net.go {
                            Some(online::Go::Find) => Screen::Lobbies,
                            Some(online::Go::Code) => Screen::Code,
                            _ => Screen::Mode,
                        };
                        out.push(Sound(1));
                    }
                } else if k.ok && seated.is_none() {
                    let n = if self.online() { self.local_max() } else { n };
                    if let Some(p) = (0..n).find(|&p| self.seats[p].is_none() && !self.bots[p]) {
                        self.seats[p] = Some(dev);
                        out.push(Sound(2));
                    }
                } else if (k.left || k.right || k.up || k.down) && let Some(p) = seated {
                    // the next free seat that way (1P stays the host's)
                    let d = if k.right || k.down { 1 } else { n - 2 };
                    let to = (1..n - 1).map(|j| 1 + (p - 1 + d * j) % (n - 1)).find(|&q| self.seats[q].is_none() && !self.bots[q] && !(self.online() && q >= self.local_max()));
                    if let Some(q) = to {
                        self.seats.swap(p, q);
                        out.push(Sound(0));
                    }
                } else if k.back && let Some(p) = seated {
                    self.seats[p] = None;
                    out.push(Sound(1));
                }
            }
            Screen::Chars => {
                // B238: a device not seated takes the first seat held open for one
                if k.ok && !self.seats.contains(&Some(dev)) && let Some(p) = self.seats.iter().position(|s| *s == Some(Dev::Open)) {
                    self.seats[p] = Some(dev);
                    out.push(Sound(2));
                    return out;
                }
                let Some(p) = self.driven(dev) else { return out };
                self.focus = p;
                if k.page {
                    // B296: □ / Tab cycles the roster's filters (none without mods)
                    if self.filters() == 1 {
                        out.push(Sound(1));
                    } else {
                        self.set_filter(self.filter + 1);
                        out.push(Sound(0));
                    }
                }
                let costumes = self.costumes(self.who(p));
                let costume_to = |back: bool| self.next_free(p, self.who(p), (self.players[p].costume + if back { costumes - 1 } else { 1 }) % costumes, back);
                let (next_c, prev_c) = (costume_to(false), costume_to(true));
                if k.hand && !self.players[p].ready {
                    self.players[p].swap = !self.players[p].swap;
                    out.push(Sound(0));
                }
                if !self.players[p].ready {
                    let d = (k.right as i32 - k.left as i32) + (k.down as i32 - k.up as i32) * self.grid.cols as i32;
                    if d != 0 {
                        let roster = self.roster();
                        let at = roster.iter().position(|c| *c == self.cell(p));
                        // B66: past a full one the same way, staying put at the edge (the grid doesn't wrap)
                        let to = match at {
                            None => roster.iter().position(|&c| !self.full(p, c)),
                            Some(at) => {
                                let mut to = at as i32;
                                loop {
                                    to += d;
                                    if !(0..roster.len() as i32).contains(&to) {
                                        let edge = to.clamp(0, roster.len() as i32 - 1) as usize;
                                        break Some(if self.full(p, roster[edge]) { at } else { edge });
                                    }
                                    if !self.full(p, roster[to as usize]) {
                                        break Some(to as usize);
                                    }
                                }
                            }
                        };
                        if let Some(to) = to && Some(to) != at {
                            self.put(p, roster[to]);
                            out.push(Sound(0));
                        }
                    }
                    if k.prev || k.next {
                        // M1n / B66: past a costume that failed to load or that someone else is ready on
                        if let Some(c) = if k.next { next_c } else { prev_c } {
                            self.players[p].costume = c;
                        }
                        out.push(Sound(0));
                    }
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
                        if self.online() && self.net.edit == Some(p) {
                            self.online_bot_ready(p, &mut out);
                        } else if self.online() && (0..self.players()).all(|q| self.seats[q].is_none() || self.players[q].ready) {
                            self.online_ready(&mut out);
                        } else {
                            self.chars_done(&mut out);
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
                    } else if self.open(p) && !self.online() {
                        // B258: a human leaves an open seat's COM card to the computer: its automatic pick
                        self.players[p] = Player { ready: true, ..self.auto_com(p) };
                        out.push(Sound(1));
                        self.chars_done(&mut out);
                    } else if self.seats[p].is_none() {
                        // player 1 driving a computer player: back to the last one readied
                        if let Some(q) = (0..self.players()).rev().find(|&q| self.players[q].ready) {
                            self.unready(q);
                        }
                        out.push(Sound(1));
                    } else if self.seats[p] == Some(dev) && self.drop_seat(p) {
                        // B238: a joined device's ○ takes it off its seat
                        out.push(Sound(1));
                    } else if p == 0 {
                        // online: back to the lobby, the seats still held
                        self.screen = if self.online() { Screen::Lobby } else { Screen::Assign };
                        if !self.online() {
                            self.seats.iter_mut().filter(|s| **s == Some(Dev::Open)).for_each(|s| *s = None);
                        }
                        out.push(Sound(1));
                    }
                }
            }
            Screen::Confirm if host == Some(dev) && self.picking.is_some() => {
                // the original's pickers: ←→ hover (court strip wraps, umpire cards stop at the ends), ✕ keeps, ○ puts back
                let d = k.right as i32 - k.left as i32;
                if d != 0 && self.sel == 1 {
                    let at = STRIP.iter().position(|&c| c == self.court).unwrap_or(COURTS) as i32;
                    self.court = STRIP.get((at + d).rem_euclid(COURTS as i32 + 1) as usize).copied().unwrap_or(COURTS);
                    out.push(Sound(0));
                } else if d != 0 {
                    let u = (self.umpire as i32 + d).clamp(0, UMPIRES as i32) as u8;
                    if u != self.umpire {
                        self.umpire = u;
                        out.push(Sound(0));
                    }
                }
                if k.ok {
                    self.picking = None;
                    out.push(Sound(2));
                } else if k.back {
                    let v = self.picking.take().unwrap_or_default();
                    if self.sel == 1 { self.court = v } else { self.umpire = v as u8 }
                    out.push(Sound(1));
                }
            }
            Screen::Confirm if host == Some(dev) => {
                rows(CONFIRM_ROWS, &mut self.sel, &mut out);
                if k.ok && matches!(self.sel, 1 | 4) {
                    self.picking = Some(if self.sel == 1 { self.court } else { self.umpire as usize });
                    out.push(Sound(2));
                }
                let d = if matches!(self.sel, 2 | 3) { k.right as i32 - k.left as i32 + k.ok as i32 } else { 0 };
                if d != 0 {
                    let wrap = |v: usize, n: usize| (v as i32 + d).rem_euclid(n as i32) as usize;
                    match self.sel {
                        2 => self.sets = wrap(self.sets, SETS.len()),
                        3 => self.games = GAMES[wrap(GAMES.iter().position(|&n| n >= self.games).unwrap_or(1), GAMES.len())],
                        _ => {}
                    }
                    out.push(Sound(0));
                }
                if k.ok && self.sel == 0 {
                    // ponytail: std's per-call hasher keys as the dice (not the match's RNG, which this never touches)
                    self.roll = std::hash::BuildHasher::hash_one(&std::hash::RandomState::new(), 0u8);
                }
                if k.ok && self.sel == 0 && self.online() {
                    self.online_host(&mut out);
                } else if k.ok && self.sel == 0 {
                    out.extend([Sound(2), Launch]);
                    self.screen = Screen::Playing;
                } else if k.back && self.online() {
                    self.go(Screen::Lobby, &mut out);
                } else if k.back {
                    for p in 0..4 {
                        self.unready(p);
                    }
                    self.screen = Screen::Chars;
                    out.push(Sound(1));
                }
            }
            Screen::Online | Screen::Lobbies | Screen::Lobby | Screen::Code => self.online_step(k, &mut out),
            Screen::Mods => self.mods_step(k, &mut out),
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
        let court = if self.court == COURTS { (self.roll % COURTS as u64) as usize } else { self.court };
        let umpire = if self.umpire as usize == UMPIRES { (self.roll / COURTS as u64 % UMPIRES as u64) as u8 } else { self.umpire };
        (court, umpire)
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
    Hand,
    /// The 3D pointing hand (`cursor.rs`).
    Cursor,
    /// `SHEETS[k]`.
    Sheet(usize),
    /// Player `p`'s 3D preview (`previews.rs`), in place of the portrait.
    Preview(usize),
    /// The lobby code's QR (`online::QrImage`).
    Qr,
    /// The hovered store mod's preview (`mods_screen::Shelf`).
    Shelf,
    /// A character's face icon on the select ([`Icons`]; a disc one not baked yet draws its `FACES` square).
    Icon(Who),
    /// Our button pill's layers (`menu_art::pill_layers`).
    Pill,
}

/// B133: the select's face icons (B160: the disc grid's too), as `previews.rs` reads them.
#[derive(Resource, Default)]
struct Icons(std::collections::HashMap<Who, Handle<Image>>);

const SHEETS: [(&str, &str); 15] = [
    ("MENU/MENU10A.XB0", "title_mainmenu"),
    ("MENU/MENU70.XB0", "title_option"),
    ("MENU/MENU05.XB0", "title_playerselect"),
    ("MENU/MENU12A.XB0", "title_characterselect2"),
    ("MENU/MENU03.XB0", "title_confirmation_01"),
    ("MENU/MENU05.XB0", "wall_playerselect"),
    ("MENU/MENU70.XB0", "wall_option"),
    ("MENU/MENU12A.XB0", "wall_characterselect2"),
    ("MENU/MENU03.XB0", "wall_confirmation"),
    ("MENU/MENU70.XB0", "option_00"),
    ("MENU/MENU12B.XB0", "characterselect2_00"),
    ("MENU/MENU12B.XB0", "characterselect2_01"),
    ("MENU/MENU12B.XB0", "characterselect2_02"),
    ("MENU/MENU12B.XB0", "characterselect2_03"),
    ("MENU/MENU12B.XB0", "characterselect2_04"),
];
/// The disc grid's faces sheet (`SHEETS` index): drawn for a face icon not baked yet.
const FACES: usize = 11;
/// The △ attribute card: title box, play-style titles, stat labels and the A–F letters.
const ATTRS: Tex = Tex::Sheet(14);
const CONFIRM: Tex = Tex::Sheet(15);
/// After the portrait sheets: `info.tm2` (button icons), `Confirmation_04` (the rank labels and their pill),
/// `KeyAssign_CharacterSelect2` (the select's key hint words), then `Confirmation_00`/`01`/`02`: round umpire faces and
/// balls; court names, surfaces and meter icons; court tiles (the "?" one).
const EXTRA: usize = SHEETS.len() + 2 + UMPIRES + 4 + CHARS;
const C00: Tex = Tex::Sheet(EXTRA + 3);
const C02: Tex = Tex::Sheet(EXTRA + 5);
/// The "?" court tile on `C02` (a random court; B217's Random card).
const RANDOM_TILE: [f32; 4] = [96.0, 360.0, 92.0, 68.0];

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

/// `word.tm2`'s ink span per glyph (cell x + first inked column, cell y, width), from its alpha.
fn glyphs(font: &[u8]) -> std::collections::HashMap<char, (f32, f32, f32)> {
    let pic = hst_data::tim2::decode_alpha8(font).expect("TIM2").remove(0);
    let mut glyphs = std::collections::HashMap::new();
    for (row, chars) in FONT.iter().enumerate() {
        for (col, ch) in chars.chars().enumerate() {
            let (x0, y0) = (col * 32, 32 + row * 32);
            let ink: Vec<usize> = (0..32).filter(|x| (0..32).any(|y| pic.rgba[((y0 + y) * pic.width as usize + x0 + x) * 4 + 3] > 0x20)).collect();
            if let (Some(&a), Some(&b)) = (ink.first(), ink.last()) {
                glyphs.entry(ch).or_insert(((x0 + a) as f32, y0 as f32, (b - a + 1) as f32));
            }
        }
    }
    glyphs
}

fn setup(mut commands: Commands, args: Res<Args>, kept: Option<Res<Kept>>, mut images: ResMut<Assets<Image>>) {
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
    let inpane = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let inpane = Archive::parse(&inpane).expect("xb archive");
    let hand = inpane.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("hsm_yubi.tm2")).expect("hsm_yubi");
    art.push(panel::image(&mut images, &inpane.read(hand).expect("INPANE bytes")));
    // Z1: read every sheet first so their replacements decode on all cores at once
    let mut sheets: Vec<(String, Vec<u8>)> = SHEETS
        .iter()
        .chain([&("MENU/MENU03.XB0", "confirmation_03"), &("MENU/MENU03.XB0", "f_s")])
        .map(|(xb, name)| (name.to_string(), read(&mut iso, xb, &format!("/{name}.tm2"))))
        .collect();
    for u in 0..5 {
        let name = format!("item_umpire{u:02}");
        sheets.push((name.clone(), read(&mut iso, "MENU/MENU60.XB0", &format!("/{name}.tm2"))));
    }
    let shots = ["cs2_shots_nomal_a", "cs2_shots_nomal_b", "cs2_shots_cos_a", "cs2_shots_cos_b"]
        .into_iter()
        .map(String::from)
        .chain((0..CHARS).map(|c| format!("cs2_shots_{c:02}")));
    for name in shots {
        sheets.push((name.clone(), read(&mut iso, "MENU/MENU12B.XB0", &format!("/{name}.tm2"))));
    }
    for (xb, name) in [
        ("MENU/MENU12A.XB0", "info"),
        ("MENU/MENU12B.XB0", "confirmation_04"),
        ("MENU/MENU12A.XB0", "keyassign_characterselect2"),
        ("MENU/MENU03.XB0", "confirmation_00"),
        ("MENU/MENU03.XB0", "confirmation_01"),
        ("MENU/MENU03.XB0", "confirmation_02"),
    ] {
        sheets.push((name.into(), read(&mut iso, xb, &format!("/{name}.tm2"))));
    }
    let _pre = crate::textures::prefetch_tim2(sheets.iter().map(|s| s.1.as_slice()));
    art.extend(sheets.iter().map(|(name, d)| super::menu_art::image(&mut images, d, name)));
    let glyphs = glyphs(&font);
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

    let fresh = kept.is_none();
    // first run (no settings.txt) on a Steam Deck: its graphics preset
    let first = || Settings(SETTING_DEFAULTS, if std::env::var("SteamDeck").is_ok_and(|v| v == "1") { Graphics::STEAM_DECK } else { Graphics::default() }, Volumes::default(), default_aspect(), Ask::Each);
    let saved = crate::store::read_to_string(Settings::path(&args.iso)).map(|t| {
        let mut s = Settings::parse(&t);
        // B289: an older build kept the lobby prompt's answer in the online mod cache
        if !t.contains("online_mods") {
            s.4 = Ask::load(&crate::mods_online::cache(&args.iso)).unwrap_or(s.4);
        }
        s
    });
    let mut menu = kept.map(|k| k.0.clone()).unwrap_or_else(|| Menu { settings: saved.as_ref().map_or_else(|_| first(), |s| *s), mods, ..default() });
    // back from a match: its pause menu may have changed them (B126)
    if let Ok(s) = saved {
        menu.settings = s;
    }
    commands.insert_resource(menu.settings.1);
    let screen = std::env::var("HST_MENU").unwrap_or_default();
    menu.screen = match screen.as_str() {
        "settings" | "display" => Screen::Display,
        "graphics" => Screen::Graphics,
        "sound" => Screen::Sound,
        "mod-settings" => Screen::ModSettings,
        "controls" => Screen::Controls,
        "mode" => Screen::Mode,
        "assign" => Screen::Assign,
        "chars" | "chars4" | "mods" | "attrs" | "mods-attrs" | "coms" | "coms-random" | "chars-online" | "chars-random" => Screen::Chars,
        "confirm" | "confirm-random" | "courts" | "umpires" => Screen::Confirm,
        "online" => Screen::Online,
        "lobbies" | "lobbies-searching" | "lobbies-more" | "lobbies-none" | "lobbies-failed" => Screen::Lobbies,
        "lobby" | "lobby-bot" | "lobby-ask" | "lobby-wait" | "lobby-guest" | "lobby-hosting" | "lobby-joining" | "lobby-failed" => Screen::Lobby,
        "code" | "name" => Screen::Code,
        "addmods" | "browsemods" | "uploadmods" | "mymods" | "signinmods" | "phonemods" | "phonemods-ask" => Screen::Mods,
        _ => Screen::Main,
    };
    if fresh && menu.screen == Screen::Main {
        online::linked(&mut menu);
    }
    if fresh && screen == "reloadmods" {
        mods_screen::sample(&mut menu, &screen);
    }
    if !fresh && menu.screen != Screen::Lobby {
        (menu.screen, menu.sel) = (Screen::Main, 0);
    } else if menu.screen != Screen::Main {
        menu.seats[0] = Some(Dev::Keys);
        menu.doubles = true;
        // B216: 2P and 3P computer players added, 4P left open
        menu.bots = [false, true, true, false];
        menu.players[0].cursor = 6;
        menu.players[0].ready = screen.starts_with("coms");
        menu.players[0].random = screen == "chars-random";
        // B264: 2P's computer readied on Random (`coms-random`, `confirm-random`)
        menu.players[1].random = matches!(screen.as_str(), "coms-random" | "confirm-random");
        menu.players[1].ready |= menu.players[1].random;
        // B64: the court / umpire picker open on Nakagawa River Courts / Lily, as the PCSX2 shots (`context/b64`)
        if let Some(sel) = [("courts", 1), ("umpires", 4)].iter().find(|r| r.0 == screen).map(|r| r.1) {
            (menu.sel, menu.court, menu.umpire) = (sel, 8, 2);
            menu.picking = Some(if sel == 1 { 8 } else { 2 });
        }
        if screen == "chars4" {
            // four humans on one portrait: their hands' offsets keep them apart (B156)
            menu.seats = [Some(Dev::Keys); 4];
            menu.players.iter_mut().for_each(|p| p.cursor = 6);
        }
        mods_screen::sample(&mut menu, &screen);
        if screen.starts_with("mods") && menu.screen == Screen::Chars {
            menu.set_filter(2);
        }
        if matches!(menu.screen, Screen::Lobby | Screen::Lobbies | Screen::Code) || screen == "chars-online" {
            online::sample(&mut menu);
        }
    }
    commands.insert_resource(menu);
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    commands.spawn((full, GlobalZIndex(10))).with_children(|p| {
        for i in 0..POOL {
            p.spawn((Slot(i), ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
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
const TITLES: [(&str, [f32; 3]); 7] = [
    ("Main Menu", [102.0, 85.0, 25.0]),
    ("Settings", [110.0, 65.0, 75.0]),
    ("Game Selection", [75.0, 52.0, 85.0]),
    ("Character Selection", [102.0, 70.0, 40.0]),
    ("Confirm Settings", [55.0, 80.0, 42.0]),
    ("Mods", [110.0, 65.0, 75.0]),
    ("Lobby", [55.0, 80.0, 42.0]),
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
        self.out.push(Quad { tex, src, dst, rgb, alpha, tile: false, anchor: self.anchor, angle: 0.0 });
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

    /// The screen's wall (tiled) and title bar.
    fn frame(&mut self, title: usize, wall: usize) {
        self.out.push(Quad { tex: Tex::Sheet(wall), src: [0.0, 0.0, 32.0, 32.0], dst: [0.0, 0.0, 640.0, 448.0], rgb: WHITE, alpha: 128.0, tile: true, anchor: Anchor::Span, angle: 0.0 });
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

    /// A column of rows, the chosen one bright with the hand beside it.
    fn rows(&mut self, labels: &[String], sel: usize, x: f32, y: f32, w: f32, hand_x: f32) {
        self.rows_at(labels, sel, x, y, w, hand_x, 44.0);
    }

    /// `rows` `pitch` px apart.
    #[allow(clippy::too_many_arguments)]
    fn rows_at(&mut self, labels: &[String], sel: usize, x: f32, y: f32, w: f32, hand_x: f32, pitch: f32) {
        self.rows_locked(labels, sel, x, y, w, hand_x, pitch, |_| false);
    }

    /// `rows_at` with the rows `locked` says greyed out (B243): a faint dark pill and dim words, never the chosen
    /// row's gold even under the hand, so a switch the build keeps off reads as one that can't be pressed.
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

/// A device's name on the assignment screen.
fn dev_name(d: Option<Dev>, pads: &[Entity]) -> String {
    match d {
        None => "COM".into(),
        Some(Dev::Keys) => "Keyboard".into(),
        Some(Dev::Pad(e)) => format!("Controller {}", pads.iter().position(|x| *x == e).map_or(0, |i| i + 1)),
        Some(Dev::Open) => "X or Enter joins".into(),
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
/// was 11, and scrolled when it didn't fit); a longer text word-wraps and shrinks until it fits the bar.
fn ticker_text(
    ticker: Res<Ticker>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut clip: Query<(&mut Node, &mut Visibility, &Children), With<TickerClip>>,
    mut text: Query<(&mut bevy::ui::widget::Text, &mut TextFont, &mut Node), Without<TickerClip>>,
) {
    let (Ok(win), Ok((mut cnode, mut vis, kids))) = (window.single(), clip.single_mut()) else { return };
    let Some(Ok((mut t, mut font, mut node))) = kids.first().map(|&k| text.get_mut(k)) else { return };
    *vis = if ticker.0.is_empty() { Visibility::Hidden } else { Visibility::Inherited };
    let share = super::widescreen::window_share(&window);
    let (left, width) = super::widescreen::place(0.0, 640.0, share, Anchor::Span);
    (cnode.left, cnode.width) = (Val::Percent(left), Val::Percent(width));
    let (top, height) = super::widescreen::vplace(404.0, 32.0, super::widescreen::window_aspect(&window), Anchor::Span);
    (cnode.top, cnode.height) = (Val::Percent(top), Val::Percent(height));
    // one 640-space unit in logical px
    let vs = super::widescreen::view_size(win);
    let unit = share * vs.x / 640.0;
    let (bar_w, bar_h) = (width / 100.0 * vs.x, height / 100.0 * vs.y);
    // a tall narrow window (a phone held upright) has the height for a bigger glyph than its width's units give
    let cap = (10.0 * unit).max((7.0f32).min(bar_h / 4.0));
    let (p, lines) = fit_lines(&ticker.0, bar_w - 32.0 * unit, bar_h - 2.0 * unit, cap);
    let em = p / 0.6;
    font.font_size = bevy::text::FontSize::Px(em);
    node.left = Val::Px(ticker_x_in(ticker.1, bar_w / unit + 16.0) * unit);
    node.top = Val::Px((bar_h - 1.2 * em * lines.len() as f32) / 2.0);
    let s = lines.join("\n");
    if t.0 != s {
        t.0 = s;
    }
}

fn layout(m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity], hand_x: f32) -> (Vec<Quad>, String) {
    let d = drawn(m, text, bind, pads, hand_x);
    (d.out, d.said)
}

/// The screen's quads, words and hit rects.
fn drawn<'a>(m: &Menu, text: &'a Text, bind: &Bindings, pads: &[Entity], hand_x: f32) -> Draw<'a> {
    let mut d = Draw { out: Vec::new(), text, said: String::new(), anchor: Anchor::Centre, hits: Vec::new(), outside: None };
    match m.screen {
        Screen::Main => {
            d.frame(0, 5);
            let labels: Vec<String> = MAIN.iter().map(|s| s.to_string()).collect();
            d.rows(&labels, m.sel, 200.0, 120.0, 240.0, hand_x);
            let info = match m.sel {
                0 => text.msg(188).to_string(),
                1 => "Play over the internet or on this network: host a lobby, find one, or join by code.".into(),
                2 => text.msg(190).to_string(),
                3 => "Add, remove, browse and share custom characters (mods).".into(),
                _ => "Close the game.".into(),
            };
            d.info(&info);
            mods_screen::ask_layout(&mut d, m, hand_x);
        }
        Screen::Display | Screen::Graphics | Screen::Sound | Screen::ModSettings => options::page_layout(&mut d, m, hand_x),
        Screen::Controls => options::controls_layout(&mut d, m, bind, hand_x),
        Screen::Mode => {
            d.frame(2, 5);
            // the whole pill, its dark rim included (it runs to the sheet's edges), at the same scale and place as before
            for (i, words) in ["Singles", "Doubles"].into_iter().enumerate() {
                let on = (i == 1) == m.doubles;
                mode_bar(&mut d, i, on, [131.0, 147.0 + 70.0 * i as f32, 378.0, 47.0], words);
                d.hit([131.0, 147.0 + 70.0 * i as f32, 378.0, 47.0], pointer::Act::Row(i, pointer::OK));
            }
            d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [76.0 + hand_x, 156.0 + 70.0 * m.doubles as u8 as f32, 64.0, 32.0], WHITE, 128.0);
            d.info(text.msg(192));
        }
        Screen::Assign => {
            d.frame(2, 5);
            for p in 0..m.players() {
                let y = 90.0 + 70.0 * p as f32;
                d.box3([120.0, y, 400.0, 56.0], 10.0, [Some(INK), Some(WHITE), Some(CARD_FILL[p])], 128.0);
                d.hit([120.0, y, 400.0, 56.0], pointer::press(|k| k.ok = true));
                d.text(&format!("{}P", p + 1), 140.0, y + 10.0, 32.0, WHITE);
                // B216: a seat with nobody is open (filled at Start) until 1P adds a computer player to it
                let name = if m.open(p) && !m.online() { "Open".to_string() } else { dev_name(m.seats[p], pads) };
                d.text(&name, 240.0, y + 12.0, 28.0, if m.seats[p].is_some() { WHITE } else if m.bots[p] { [60.0, 110.0, 40.0] } else { [70.0; 3] });
                if m.droppable(p) {
                    drop_button(&mut d, p, 482.0, y + 14.0);
                }
                if m.can_bot(p) {
                    let (label, fill) = if m.bots[p] { ("- COM", [120.0, 30.0, 30.0]) } else { ("+ COM", [40.0, 100.0, 40.0]) };
                    d.pill_at([430.0, y + 14.0, 76.0, 28.0], [INK, WHITE, fill], 128.0);
                    d.centred(label, 468.0, y + 18.0, 20.0, WHITE);
                    d.hit([430.0, y + 14.0, 76.0, 28.0], pointer::Act::Row(p, Press { card: true, ..default() }));
                }
            }
            d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [60.0 + hand_x, 102.0 + 70.0 * m.sel.min(m.players() - 1) as f32, 64.0, 32.0], WHITE, 128.0);
            if m.paused {
                d.info("1P: Up/Down picks a seat, Left/Right swaps its device with the next player's, Tab / Square takes a device off its seat, X or Enter keeps the change, O or Esc leaves it. Others: X or Enter takes a free seat, Left/Right changes seat, O or its red X leaves.");
            } else {
            d.info(&format!("{} 1P: Up/Down picks a seat, Left/Right swaps it, Tab / Square takes it off, Triangle adds or removes a computer player (empty seats are filled at Start). Others: X or Enter joins, Left/Right changes seat, O or its red X leaves.", text.msg(193)));
            }
        }
        Screen::Chars => chars(&mut d, m, text, pads),
        // the match starts from the confirm screen, which stays up under the switch's zoom-out
        Screen::Confirm | Screen::Playing => confirm(&mut d, m, text, pads, hand_x),
        Screen::Online | Screen::Lobbies | Screen::Lobby | Screen::Code => online::layout(&mut d, m, hand_x),
        Screen::Mods => mods_screen::layout(&mut d, m, hand_x),
    }
    // B221: the screens with no Back row show the title tab's Back
    if matches!(m.screen, Screen::Mode | Screen::Assign | Screen::Chars | Screen::Confirm) && m.picking.is_none() {
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

/// The △ card as the original draws it over panel `p`: the title box in the player's colour with the play style,
/// the five stat labels and each grade's letter (A at grade 5, F at 0).
fn attribute_card(d: &mut Draw, p: usize, style: usize, grades: &[u8; 5], x: f32, y: f32) {
    const TINT: [[f32; 3]; 4] = [[100.0, 39.0, 39.0], [82.0, 63.0, 27.0], [23.0, 69.0, 79.0], [67.0, 72.0, 25.0]];
    d.q(ATTRS, [0.0, 0.0, 144.0, 40.0], [x, y + 2.0, 144.0, 40.0], TINT[p], 128.0);
    d.q(ATTRS, [0.0, 40.0, 144.0, 40.0], [x, y + 2.0, 144.0, 40.0], WHITE, 128.0);
    d.q(ATTRS, [0.0, 80.0 + 40.0 * style as f32, 144.0, 40.0], [x, y + 2.0, 144.0, 40.0], WHITE, 128.0);
    d.q(ATTRS, [0.0, 248.0, 112.0, 120.0], [x + 14.0, y + 40.0, 112.0, 120.0], WHITE, 128.0);
    for (k, &g) in grades.iter().enumerate() {
        d.q(ATTRS, [232.0, 24.0 * (5 - g.min(5)) as f32, 24.0, 24.0], [x + 102.0, y + 40.0 + 24.0 * k as f32, 24.0, 24.0], WHITE, 128.0);
    }
}

/// B296: the roster panel's cream and its cells' fills (ours, after the redesign's mockup).
const CREAM: [f32; 3] = [126.0, 122.0, 114.0];
const CELL_FILL: [f32; 3] = [128.0, 126.0, 120.0];
const CELL_ON: [f32; 3] = [128.0, 108.0, 38.0];
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

/// B296: the character select (ours, `mockups/CharSelect`): the seat strip, the roster with its filters, and the
/// detail panel of the player picking.
fn chars(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity]) {
    d.frame(3, 7);
    let g = m.grid;
    let n = m.players();
    let online = m.select_online();
    let picking = |p: usize| m.seats[p].is_some() || m.bot(p) && (0..n).filter(|&q| m.bot(q)).all(|q| q >= p || m.players[q].ready) && (0..n).all(|q| m.seats[q].is_none() || m.players[q].ready);
    // B201: online, every taken seat shows its player's place, an open one stays dark
    let picking = |p: usize| if online { m.seats[p].is_some() || m.remote(p).is_some() || m.net.edit == Some(p) } else { picking(p) };
    let shown = m.shown();
    let style_of = |w: Who| match w {
        Who::Mod(i) => text.mods[i].0,
        Who::Disc(c) => text.style[c],
    };
    let name_of = |p: usize| match m.who(p) {
        _ if m.players[p].random => "Random".to_string(),
        Who::Mod(i) => m.mods[i].name.clone(),
        // B234: another machine's mod this one doesn't have: its own name over the donor's looks
        Who::Disc(c) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[c]).to_string(),
    };
    let [sx, sy, sw, sh] = g.strip;
    mode_bar(d, m.doubles as usize, true, [sx + sw - 120.0, sy - 32.0, 120.0, 26.0], if m.doubles { "Doubles" } else { "Singles" });

    // the seat strip: a card per player, the teams either side of "VS"
    let (team, vs) = (if m.doubles { 2 } else { 1 }, 34.0);
    let gaps = vs + 8.0 * (n - 2) as f32;
    let cw = ((sw - gaps) / n as f32).min(240.0);
    let x0 = sx + (sw - cw * n as f32 - gaps) / 2.0;
    d.outlined(|d| d.centred("VS", x0 + cw * team as f32 + 8.0 * (team - 1) as f32 + vs / 2.0, sy + 22.0, 26.0, GOLD));
    for p in 0..n {
        let pl = m.players[p];
        let x = x0 + (cw + 8.0) * p as f32 + if p >= team { vs - 8.0 } else { 0.0 };
        let human = m.seats[p].is_some() || m.remote(p).is_some_and(|s| s.human());
        let on = picking(p) || pl.ready;
        let fill = if on { CARD_FILL[p] } else { CARD_FILL[p].map(|c| c * 0.45) };
        d.box3([x, sy, cw, sh], 12.0, [Some(INK), Some(if on { WHITE } else { [60.0; 3] }), Some(fill)], 128.0);
        // B221: a tap on a card readies its pick, on a ready one picks again
        d.hit([x, sy, cw, sh], pointer::press(|k| if pl.ready { k.back = true } else { k.ok = true }));
        let fs = sh - 16.0;
        if on {
            face_frame(d, cell_face(m.cell(p)), x + 8.0 + fs / 2.0, sy + sh / 2.0, fs, WHITE);
        } else {
            // an empty seat: just the dimmed ring
            let r = fs / 2.0;
            for (src, dst) in super::menu_art::pill_quads(0, [x + 8.0, sy + sh / 2.0 - r, fs, fs]) {
                d.q(Tex::Pill, src, dst, [40.0; 3], 128.0);
            }
        }
        let (tx, tw) = (x + fs + 14.0, cw - fs - 20.0);
        let cx = tx + tw / 2.0;
        // whose seat: the device, the online player, or a computer with its rank
        let whose = match m.remote(p).filter(|s| s.human() && online) {
            Some(s) => m.net.room.as_ref().and_then(|r| r.names.get(&s.peer)).map_or("Online".to_string(), |n| n.clone()),
            None if human => dev_name(m.seats[p], pads),
            None if online && !picking(p) => "Open".into(),
            None => {
                let (who, costume) = m.pick(p);
                let c = match who {
                    Who::Mod(i) => m.mods[i].donor,
                    Who::Disc(c) => c,
                };
                format!("COM · {}", RANK_NAMES.get(com_rank(&text.ranks, c, costume, m.doubles)).copied().unwrap_or(""))
            }
        };
        d.label(&format!("{}P · {whose}", p + 1), cx, sy + 6.0, 14.0, tw, WHITE);
        let ready = pl.ready && (human || pl.random);
        if on {
            d.label(&name_of(p), cx, sy + 22.0, 24.0, tw, GOLD);
        }
        let detail = if m.seats[p] == Some(Dev::Open) {
            "X or Enter joins".to_string()
        } else if !on {
            "Waiting".into()
        } else if ready {
            "Ready to go!".into()
        } else if m.blocked(p) {
            "Not selectable".into()
        } else if pl.random {
            "Drawn when ready".into()
        } else {
            format!("{} · Costume {}", STYLE_NAMES[style_of(m.who(p))], pl.costume + 1)
        };
        d.label(&detail, cx, sy + 48.0, 14.0, tw, if ready { GOLD } else { WHITE });
        // B238: a joined device's ✕ on its card's corner
        if m.droppable(p) {
            drop_button(d, p, x + cw - 20.0, sy - 8.0);
        }
    }

    // the roster: its filters, a page of cells, where the page is
    let [rx, ry, rw, rh] = g.roster;
    d.box3([rx, ry, rw, rh], 14.0, [Some(INK), Some(WHITE), Some(CREAM)], 128.0);
    let mut px = rx + 10.0;
    let mut pills = vec![("All".to_string(), CHARS + m.mods.len()), ("Hot Shots Tennis".into(), CHARS), ("Mods".into(), m.mods.len())];
    if m.mods.is_empty() {
        pills.truncate(1);
        pills[0].0 = "Hot Shots Tennis".into();
    }
    for (i, (words, k)) in pills.iter().enumerate() {
        let words = format!("{words} {k}");
        let w = d.width(&words, 14.0) + 20.0;
        if px + w > rx + rw - 10.0 {
            break;
        }
        let on = i == m.filter;
        d.pill_at([px, ry + 9.0, w, 22.0], [INK, if on { INK } else { [90.0; 3] }, if on { BROWN } else { CELL_FILL }], 128.0);
        d.centred(&words, px + w / 2.0, ry + 13.0, 14.0, if on { WHITE } else { BROWN });
        d.hit([px, ry + 9.0, w, 22.0], pointer::Act::Filter(i));
        px += w + 6.0;
    }
    let (start, roster) = m.page();
    for (k, &c) in roster.iter().enumerate().skip(start).take(g.per()) {
        let [x, y, w, h] = g.cell(k);
        let on: Vec<usize> = (0..n).filter(|&q| (picking(q) || m.players[q].ready) && m.cell(q) == c).collect();
        let full = m.full(shown, c);
        let fill = if full { [70.0; 3] } else if on.iter().any(|&q| !m.players[q].ready) { CELL_ON } else { CELL_FILL };
        // ponytail: square cells (two quads, not a rounded box's eighteen) so a page fits the quad pool
        d.q(Tex::Solid, [0.0; 4], [x, y, w, h], INK, 128.0);
        d.q(Tex::Solid, [0.0; 4], [x + 1.5, y + 1.5, w - 3.0, h - 3.0], fill, 128.0);
        face_frame(d, cell_face(c), x + w / 2.0, y + 20.0, 34.0, if full { [50.0; 3] } else { WHITE });
        let name = match c {
            None => "Random".to_string(),
            Some(Who::Disc(ch)) => CHAR_NAMES[ch].to_string(),
            Some(Who::Mod(i)) => m.mods[i].name.clone(),
        };
        d.label(&name, x + w / 2.0, y + 40.0, 12.0, w - 4.0, BROWN);
        // every player on it: their tag in the corner, and the hand of one still picking
        for (j, &q) in on.iter().enumerate() {
            d.pill_at([x + w - 21.0 - 12.0 * j as f32, y - 5.0, 22.0, 13.0], [INK, WHITE, COLOURS[q]], 128.0);
            d.centred(&format!("{}P", q + 1), x + w - 10.0 - 12.0 * j as f32, y - 4.0, 11.0, WHITE);
            if !m.players[q].ready && picking(q) {
                d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x - 22.0 + 6.0 * (q % 2) as f32, y + 14.0 + 8.0 * (q / 2) as f32, 36.0, 18.0], COLOURS[q], 128.0);
            }
        }
        d.hit([x, y, w, h], pointer::Act::Cell(k));
    }
    let total = roster.len().div_ceil(g.cols);
    let first = start / g.cols + 1;
    let fy = ry + rh - 22.0;
    d.text(&format!("Rows {first}-{} of {total}", (first + g.rows - 1).min(total)), rx + 12.0, fy, 13.0, BROWN);
    let bar = [rx + rw - 110.0, fy + 4.0, 96.0, 7.0];
    d.box3(bar, 3.5, [Some([90.0; 3]), None, Some([100.0; 3])], 128.0);
    let thumb = (g.rows as f32 / total.max(1) as f32).min(1.0);
    d.box3([bar[0] + bar[2] * (first - 1) as f32 / total.max(1) as f32, bar[1], (bar[2] * thumb).max(7.0), bar[3]], 3.5, [Some(BROWN), None, Some(BROWN)], 128.0);
    // B221: the bar's halves step the picking player a row up or down (onto the next page past its edge)
    d.hit([bar[0] - 4.0, fy - 4.0, bar[2] / 2.0 + 4.0, 20.0], pointer::press(|k| k.up = true));
    d.hit([bar[0] + bar[2] / 2.0, fy - 4.0, bar[2] / 2.0 + 4.0, 20.0], pointer::press(|k| k.down = true));

    // the detail panel: the shown player's figure, hand, style, △ grades and costume
    let [x, y, w, h] = g.detail;
    let pl = m.players[shown];
    let who = m.who(shown);
    d.box3([x, y, w, h], 14.0, [Some(INK), Some(WHITE), Some(CREAM)], 128.0);
    let wide = w > h * 1.5;
    // a tall panel (a portrait window's) gives its spare height to the figure
    let band = if wide { [x + 8.0, y + 8.0, 130.0, h - 16.0] } else { [x + 8.0, y + 8.0, w - 16.0, (h * 0.42).min(150.0).max(h - 186.0)] };
    d.box3(band, 10.0, [Some(INK), None, Some(CARD_FILL[shown])], 128.0);
    let fh = band[3] - 4.0;
    if pl.random {
        let fw = (band[2] - 12.0).min(fh * 92.0 / 68.0);
        d.q(C02, RANDOM_TILE, [band[0] + (band[2] - fw) / 2.0, band[1] + (band[3] - fw * 68.0 / 92.0) / 2.0, fw, fw * 68.0 / 92.0], WHITE, 128.0);
    } else {
        d.q(Tex::Preview(shown), [0.0; 4], [band[0] + (band[2] - fh * 0.8) / 2.0, band[1] + 2.0, fh * 0.8, fh], WHITE, 128.0);
        // the hand tag: Select / − (or a tap) swaps it
        let words = if m.hand(text, shown) < 0.0 { "Left hander" } else { "Right hander" };
        let hb = [band[0] + band[2] - 70.0, band[1] + 5.0, 66.0, 18.0];
        d.pill_at(hb, [INK, WHITE, if pl.swap { [128.0, 100.0, 20.0] } else { [90.0, 72.0, 48.0] }], 128.0);
        d.label(words, hb[0] + 33.0, hb[1] + 3.0, 12.0, 56.0, WHITE);
        d.hit(hb, pointer::press(|k| k.hand = true));
    }
    d.text(&format!("{}P", shown + 1), band[0] + 6.0, band[1] + 4.0, 18.0, WHITE);
    let (tx, ty, tw) = if wide { (x + 146.0, y + 8.0, w - 154.0) } else { (x + 10.0, band[1] + band[3] + 6.0, w - 20.0) };
    d.label(&name_of(shown), tx + tw / 2.0, ty, 22.0, tw, BROWN);
    if pl.random {
        d.label("A character and costume", tx + tw / 2.0, ty + 30.0, 14.0, tw, BROWN);
        d.label("drawn when you're ready", tx + tw / 2.0, ty + 46.0, 14.0, tw, BROWN);
        d.info("Random: a character and costume drawn from the disc and custom characters when you're ready.");
        return;
    }
    let from = match who {
        Who::Mod(i) if !m.mods[i].source.is_empty() => m.mods[i].source.clone(),
        Who::Mod(_) => "Custom".into(),
        Who::Disc(_) => "Hot Shots Tennis".into(),
    };
    d.label(&format!("{} · {from}", STYLE_NAMES[style_of(who)]), tx + tw / 2.0, ty + 24.0, 13.0, tw, [60.0, 42.0, 24.0]);
    let grades = match who {
        Who::Mod(i) => text.mods[i].1,
        Who::Disc(c) => text.grades[c],
    };
    let row = ((y + h - 34.0 - (ty + 42.0)) / 5.0).clamp(10.0, 17.0);
    for (k, (stat, &gr)) in ["Serve", "Stroke", "Volley", "Impact", "Footwork"].iter().zip(&grades).enumerate() {
        let sy = ty + 42.0 + row * k as f32;
        let size = row - 3.0;
        d.text(stat, tx, sy, size, BROWN);
        let bx = tx + size * 4.4;
        let bw = tw - size * 4.4 - size * 1.2;
        d.box3([bx, sy + size * 0.3, bw, size * 0.5], size * 0.25, [Some([90.0; 3]), None, Some([100.0; 3])], 128.0);
        if gr > 0 {
            d.box3([bx, sy + size * 0.3, bw * gr.min(5) as f32 / 5.0, size * 0.5], size * 0.25, [Some(INK), None, Some(CARD_FILL[shown])], 128.0);
        }
        d.centred(&["F", "E", "D", "C", "B", "A"][gr.min(5) as usize].to_string(), tx + tw - size * 0.5, sy, size, BROWN);
    }
    // the costume: L1 / R1 (or a tap on the chip's halves)
    let costumes = m.costumes(who);
    let chip = [tx + (tw - 130.0) / 2.0, y + h - 30.0, 130.0, 22.0];
    d.pill_at(chip, [INK, WHITE, BROWN], 128.0);
    d.centred(&format!("<  Costume {} / {costumes}  >", pl.costume + 1), chip[0] + 65.0, chip[1] + 4.0, 14.0, WHITE);
    d.hit([chip[0], chip[1], 65.0, 22.0], pointer::press(|k| k.prev = true));
    d.hit([chip[0] + 65.0, chip[1], 65.0, 22.0], pointer::press(|k| k.next = true));
    let filter = if m.mods.is_empty() { "" } else { " Tab / Square: the roster's filter." };
    match who {
        Who::Mod(i) => {
            let md = &m.mods[i];
            let s = if md.costumes.len() == 1 { "" } else { "s" };
            let from = if md.source.is_empty() { String::new() } else { format!(" from {}", md.source) };
            d.info(&format!("{}: custom character{from}, {} costume{s}.{filter}", md.name, md.costumes.len()));
        }
        Who::Disc(c) => d.info(&format!("{}{filter}", text.msg(308 + c))),
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
    d.q(tex, src, [x - f, y - f, 2.0 * f, 2.0 * f], tint, 128.0);
}

/// B295: the side card's width, a seat row's height and pitch, and the gap between the teams' groups.
const SIDE_W: f32 = 190.0;
const ROW_H: f32 = 34.0;
const ROW_PITCH: f32 = 38.0;
const TEAM_GAP: f32 = 18.0;
/// B295: the status chips' words on white (the mockup's): Host pink, Ready orange, a computer green.
const CHIP_HOST: [f32; 3] = [88.0, 24.0, 53.0];
const CHIP_READY: [f32; 3] = [90.0, 48.0, 3.0];
const CHIP_COM: [f32; 3] = [30.0, 61.0, 15.0];

impl Grid {
    /// B295: the view's reach in 640×448 units (left, top, right, bottom), down to the legend strip: the lobby and
    /// confirm screens put their help under the seats, not in the description bar.
    fn reach(&self) -> [f32; 4] {
        [self.strip[0], self.strip[1] - 46.0, self.strip[0] + self.strip[2], self.detail[1] + self.detail[3] + 32.0]
    }

    /// B295: the lobby's and confirm screen's seats column (the teams' groups, the help line under them) and side card
    /// (beside it; under it in a portrait window, as the select's detail panel).
    fn split(&self) -> ([f32; 4], [f32; 4]) {
        let [l, t, r, b] = self.reach();
        let top = t + 76.0;
        // under it only when the window is tall enough for the side card's buttons (a near-square one keeps it beside)
        if self.detail[0] > self.strip[0] || b - t < 670.0 {
            ([l, top, r - l - SIDE_W - 10.0, b - top], [r - SIDE_W, t + 48.0, SIDE_W, b - t - 48.0])
        } else {
            let w = (r - l).min(280.0);
            ([l, top, r - l, 268.0], [(l + r - w) / 2.0, top + 276.0, w, (b - top - 276.0).min(340.0)])
        }
    }
}

/// B295: team `k`'s group (of `per` seats) in the seats column `col`.
fn team_box(col: [f32; 4], per: usize, k: usize) -> [f32; 4] {
    let h = 24.0 + ROW_PITCH * per as f32;
    [col[0], col[1] + (h + TEAM_GAP) * k as f32, col[2], h]
}

/// B295: seat `i`'s row (1P, 2P on the first team; doubles 3P, 4P on the second).
fn seat_rect(col: [f32; 4], doubles: bool, i: usize) -> [f32; 4] {
    let per = 1 + doubles as usize;
    let [x, y, w, _] = team_box(col, per, i / per);
    [x + 6.0, y + 22.0 + ROW_PITCH * (i % per) as f32, w - 12.0, ROW_H]
}

/// B295: the context help under the seats.
fn help_rect(col: [f32; 4], doubles: bool) -> [f32; 4] {
    let [x, y, w, h] = team_box(col, 1 + doubles as usize, 1);
    [x, y + h + 8.0, w, 40.0]
}

/// B295: the teams' groups ("Team 1", "Team 2") behind the seat rows.
fn team_boxes(d: &mut Draw, col: [f32; 4], doubles: bool) {
    for k in 0..2 {
        let [x, y, w, h] = team_box(col, 1 + doubles as usize, k);
        d.box3([x, y, w, h], 12.0, [Some(INK), Some(WHITE), Some(CREAM)], 120.0);
        d.text(&format!("TEAM {}", k + 1), x + 10.0, y + 5.0, 12.0, [38.0, 51.0, 32.0]);
    }
}

/// B295: the help line's box under the seats, `s` word-wrapped onto its two lines (shrunk to fit).
fn help_line(d: &mut Draw, col: [f32; 4], doubles: bool, s: &str) {
    let [x, y, w, h] = help_rect(col, doubles);
    d.box3([x, y, w, h], 10.0, [None, None, Some(WHITE)], 96.0);
    let mut size = 13.0;
    let lines = loop {
        let lines = wrap(s, ((w - 16.0) / (size * 0.55)) as usize);
        if lines.len() <= 2 || size < 8.0 {
            break lines;
        }
        size -= 1.0;
    };
    let top = y + (h - 1.25 * size * lines.len() as f32) / 2.0;
    for (i, l) in lines.iter().enumerate() {
        let lw = d.width(l, size).min(w - 16.0);
        d.label(l, x + 8.0 + lw / 2.0, top + 1.25 * size * i as f32, size, w - 16.0, INK);
    }
}

/// `s` from `x` at `size`, squeezed to fit `max`.
fn left(d: &mut Draw, s: &str, x: f32, y: f32, size: f32, max: f32, rgb: [f32; 3]) {
    let w = d.width(s, size).min(max);
    d.label(s, x + w / 2.0, y, size, max, rgb);
}

/// B295: what a seat row shows.
struct SeatRow<'a> {
    /// the player (its colour and tag)
    p: usize,
    face: Option<(Tex, [f32; 4])>,
    name: &'a str,
    /// the name dimmed (still picking)
    faint: bool,
    who: &'a str,
    /// the status chip's words and their colour (None: see-through, white words)
    chip: (&'a str, Option<[f32; 3]>),
    /// taken (not an open seat)
    filled: bool,
    /// the cursor's
    on: bool,
}

/// B295: a seat's row (`mockups/Lobby`): the player's colour, "NP", the face, the character over the player, the
/// status chip at the right end (`end` units in from it, for a ✕), gold-ringed when chosen.
fn seat_row(d: &mut Draw, [x, y, w, h]: [f32; 4], s: &SeatRow, end: f32) {
    let fill = if s.filled { CARD_FILL[s.p] } else { CARD_FILL[s.p].map(|c| c * 0.45) };
    let ring = if s.on { GOLD } else if s.filled { WHITE } else { [60.0; 3] };
    d.box3([x, y, w, h], 12.0, [Some(INK), Some(ring), Some(fill)], 128.0);
    d.text(&format!("{}P", s.p + 1), x + 8.0, y + 8.0, 18.0, WHITE);
    let fx = x + 50.0;
    match s.face {
        Some(f) => face_frame(d, f, fx, y + h / 2.0, h - 6.0, WHITE),
        None => {
            let r = (h - 6.0) / 2.0;
            for (src, dst) in super::menu_art::pill_quads(0, [fx - r, y + h / 2.0 - r, 2.0 * r, 2.0 * r]) {
                d.q(Tex::Pill, src, dst, [40.0; 3], 128.0);
            }
        }
    }
    let (words, fill) = s.chip;
    let cw = d.width(words, 12.0).min(90.0) + 18.0;
    let cx = x + w - end - 6.0 - cw;
    d.pill_at([cx, y + 8.0, cw, 18.0], [fill.map_or([40.0; 3], |_| [60.0; 3]), fill.map_or([40.0; 3], |_| WHITE), fill.map_or([40.0; 3], |_| WHITE)], if fill.is_some() { 128.0 } else { 70.0 });
    d.label(words, cx + cw / 2.0, y + 10.0, 12.0, cw - 6.0, fill.unwrap_or(WHITE));
    let (tx, tw) = (fx + h / 2.0 + 4.0, cx - fx - h / 2.0 - 10.0);
    left(d, s.name, tx, y + 2.0, 17.0, tw, if s.faint { [100.0; 3] } else { WHITE });
    left(d, s.who, tx, y + 20.0, 11.0, tw, [118.0; 3]);
}

/// The confirm screen (B295, ours after `mockups/Lobby`): the seats as rows by team with the original's VS between
/// them, the side card with the court (its surface and ball meters) and umpire, the court / sets / games / umpire
/// rows and Start under it. The original's pickers (`picker`) take the bottom as before.
fn confirm(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity], hand_x: f32) {
    d.frame(4, 8);
    let (col, [sx, sy, sw, _]) = m.grid.split();
    let [l, t, ..] = m.grid.reach();
    let mode = format!("{} · {} set{}, {} games", if m.doubles { "Doubles" } else { "Singles" }, SETS[m.sets], if SETS[m.sets] == 1 { "" } else { "s" }, m.games);
    d.outlined(|d| d.text(&mode, l + 8.0, t + 48.0, 20.0, GOLD));
    // the flame behind the teams' groups, the VS over the gap between them
    let gap = team_box(col, 1 + m.doubles as usize, 1)[1] - TEAM_GAP / 2.0;
    let vs = [col[0] + col[2] / 2.0, gap];
    flame::draw(d, m, vs);
    team_boxes(d, col, m.doubles);
    d.q(CONFIRM, [448.0, 264.0, 64.0, 48.0], [vs[0] - 32.0, vs[1] - 24.0, 64.0, 48.0], WHITE, 128.0);
    for p in 0..m.players() {
        let pl = m.players[p];
        let remote = m.remote(p).filter(|s| s.human());
        let human = m.seats[p].is_some() || remote.is_some();
        let name = match m.who(p) {
            _ if pl.random => "Random".to_string(),
            Who::Mod(i) => m.mods[i].name.clone(),
            Who::Disc(c) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[c]).to_string(),
        };
        let (who, costume) = m.pick(p);
        let (style, c) = match who {
            Who::Mod(i) => (text.mods[i].0, m.mods[i].donor),
            Who::Disc(c) => (text.style[c], c),
        };
        let owner = match remote {
            Some(s) => m.net.room.as_ref().and_then(|r| r.names.get(&s.peer)).map_or("Online".to_string(), |n| n.clone()),
            None if human => dev_name(m.seats[p], pads),
            None => "COM".into(),
        };
        // B264: a Random pick stays hidden until the match starts
        let line = if pl.random { owner } else { format!("{owner} · {} · Costume {}", STYLE_NAMES[style], costume + 1) };
        let rank = RANK_NAMES.get(com_rank(&text.ranks, c, costume, m.doubles)).copied().unwrap_or("COM");
        let chip = if human { ("Ready", Some(CHIP_READY)) } else { (rank, Some(CHIP_COM)) };
        let row = SeatRow { p, face: Some(cell_face(m.cell(p))), name: &name, faint: false, who: &line, chip, filled: true, on: false };
        seat_row(d, seat_rect(col, m.doubles, p), &row, 0.0);
    }

    // the side card: the court's band, its surface and meters, the umpire
    d.box3([sx, sy, sw, 122.0], 14.0, [Some(INK), Some(WHITE), Some(CREAM)], 128.0);
    let court = if m.court == COURTS { "Random Court".to_string() } else { format!("{} {}", text.msg(276 + 2 * m.court), text.msg(277 + 2 * m.court)) };
    court_band(d, [sx, sy, sw], &court);
    if m.court < COURTS {
        let s = SURFACE[m.court];
        let tint = [[50.0, 128.0, 30.0], [128.0, 80.0, 30.0], [40.0, 100.0, 128.0]][s];
        d.outlined(|d| left(d, ["Grass Court", "Clay Court", "Hard Court"][s], sx + 10.0, sy + 52.0, 16.0, sw - 20.0, tint));
        // B141a: the icons and balls are ours (`menu_art::pill_layers`)
        for (g, (icon, n)) in [(super::menu_art::BOUNCE, METERS[s].0), (super::menu_art::SPEED, METERS[s].1)].into_iter().enumerate() {
            let x = sx + 10.0 + (sw - 20.0) / 2.0 * g as f32;
            let k = 18.0 / icon[3];
            d.q(Tex::Pill, icon, [x, sy + 72.0, icon[2] * k, 18.0], WHITE, 128.0);
            for b in 0..4 {
                d.q(Tex::Pill, if b < n { super::menu_art::BALL } else { super::menu_art::BALL_OFF }, [x + icon[2] * k + 2.0 + 13.0 * b as f32, sy + 74.0, 13.0, 13.0], WHITE, 128.0);
            }
        }
    }
    // the umpire's round face (the "?" tile for a random one) and name
    let u = m.umpire as usize;
    let (face, uname) = match u {
        UMPIRES => ((C02, RANDOM_TILE), "Random".to_string()),
        _ => ((C00, [63.0 + 48.0 * u as f32, 272.0, 46.0, 44.0]), text.msg(669 + 2 * u).trim_start_matches("Umpire ").to_string()),
    };
    face_frame(d, face, sx + 22.0, sy + 106.0, 24.0, WHITE);
    left(d, &format!("Umpire {uname}"), sx + 40.0, sy + 99.0, 14.0, sw - 50.0, BROWN);
    if m.picking.is_some() {
        return picker(d, m, text, hand_x);
    }
    let rects = confirm_rects(m);
    let rows: [String; CONFIRM_ROWS] = ["Start the match!".into(), "Select Court".into(), format!("Sets  {}", SETS[m.sets]), format!("Games  {}", m.games), "Select Umpire".into()];
    for (i, (label, r)) in rows.iter().zip(rects).enumerate() {
        let on = i == m.sel;
        d.hit(r, pointer::Act::Row(i, pointer::OK));
        side_button(d, r, label, on, i == 0);
    }
    let [hx, hy, _, hh] = rects[m.sel];
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [hx - 22.0 + hand_x, hy + hh / 2.0 - 9.0, 36.0, 18.0], WHITE, 128.0);
    let info = match m.sel {
        0 => text.msg(323),
        1 => text.msg(326),
        2 => text.msg(324),
        3 => text.msg(325),
        _ => text.msg(328),
    };
    help_line(d, col, m.doubles, info);
}

/// B295: the confirm screen's rows on the side card: Start (big, at its foot), then Court, Sets, Games, Umpire under
/// the court and umpire.
fn confirm_rects(m: &Menu) -> [[f32; 4]; CONFIRM_ROWS] {
    let (_, [sx, sy, sw, sh]) = m.grid.split();
    let row = |k: f32| [sx, sy + 130.0 + 32.0 * k, sw, 28.0];
    [[sx, sy + sh - 46.0, sw, 44.0], row(0.0), row(1.0), row(2.0), row(3.0)]
}

/// B295: the side card's court band (`[x, y, w]`): grass stripes, the court's name along its foot.
fn court_band(d: &mut Draw, [x, y, w]: [f32; 3], name: &str) {
    let n = 8;
    let sw = (w - 6.0) / n as f32;
    for i in 0..n {
        d.q(Tex::Solid, [0.0; 4], [x + 3.0 + sw * i as f32, y + 3.0, sw, 42.0], if i % 2 == 0 { [40.0, 77.0, 30.0] } else { [45.0, 84.0, 35.0] }, 128.0);
    }
    d.q(Tex::Solid, [0.0; 4], [x + 3.0, y + 45.0, w - 6.0, 2.0], INK, 128.0);
    d.outlined(|d| left(d, name, x + 10.0, y + 24.0, 18.0, w - 20.0, WHITE));
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

/// B64: the original's court picker (a strip of `Confirmation_02` court tiles, the hovered one large and lit, its
/// number from the sheet's digits, ◀ ▶ at the ends) or umpire picker (`Confirmation_03`'s five face cards, the hovered
/// one large and raised), in place of the rows. Positions measured off PCSX2 shots (`context/b64`).
fn picker(d: &mut Draw, m: &Menu, text: &Text, hand_x: f32) {
    // B221: a tap off the picker puts it back, as ○
    d.popup(Some(pointer::press(|k| k.back = true)));
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 326.0, 640.0, 78.0], [40.0, 90.0, 60.0], 90.0);
    d.anchor = Anchor::Centre;
    let (title, hand) = if m.sel == 1 {
        d.q(CONFIRM, [2.0, 132.0, 124.0, 22.0], [30.0, 300.0, 124.0, 22.0], WHITE, 128.0);
        // the strip: two tiles either side of the hovered one, wrapping (Random is the 12th, the "?" tile)
        let at = STRIP.iter().position(|&c| c == m.court).unwrap_or(COURTS) as i32;
        for off in [-2, -1, 1, 2, 0] {
            let p = (at + off).rem_euclid(COURTS as i32 + 1) as usize;
            // the sheet holds courts 0–6 down its left column, 7–10 down its right, the "?" tile at 11
            let tile = STRIP.get(p).map_or(11, |&k| if k < 7 { 2 * k } else { 2 * (k - 7) + 1 });
            let (k, lit) = (if off == 0 { 0.9 } else { 0.7 }, if off == 0 { WHITE } else { [70.0; 3] });
            let (w, h) = (92.0 * k, 68.0 * k);
            let (x, y) = (320.0 + 112.0 * off as f32 - w / 2.0, 366.0 - h / 2.0);
            d.q(C02, [96.0 * (tile % 2) as f32, 72.0 * (tile / 2) as f32, 92.0, 68.0], [x, y, w, h], lit, 128.0);
            d.hit([x, y, w, h], pointer::Act::Pick(STRIP.get(p).copied().unwrap_or(COURTS)));
            if p < COURTS {
                let n = (p + 1).to_string();
                for (i, c) in n.bytes().enumerate() {
                    let dg = (c - b'0') as f32;
                    d.q(C02, [192.0, 56.0 + 20.0 * dg, 20.0, 20.0], [x + 6.0 + 14.0 * k * i as f32, y - 6.0, 20.0 * k, 20.0 * k], lit, 128.0);
                }
            }
        }
        d.q(C02, [192.0, 0.0, 28.0, 56.0], [27.0, 341.0, 25.0, 50.0], WHITE, 128.0);
        d.q(C02, [228.0, 0.0, 24.0, 56.0], [589.0, 341.0, 22.0, 50.0], WHITE, 128.0);
        d.hit([15.0, 336.0, 45.0, 60.0], pointer::press(|k| k.left = true));
        d.hit([580.0, 336.0, 45.0, 60.0], pointer::press(|k| k.right = true));
        (349, [226.0, 330.0])
    } else {
        d.q(CONFIRM, [2.0, 260.0, 140.0, 25.0], [30.0, 300.0, 140.0, 25.0], WHITE, 128.0);
        let u = m.umpire as usize;
        for c in 0..=UMPIRES {
            let (k, lift) = if c == u { (1.0, 4.0) } else { (0.8, 0.0) };
            let (w, h) = (80.0 * k, 64.0 * k);
            let dst = [62.0 + 96.0 * c as f32 - w / 2.0, 369.0 - lift - h / 2.0, w, h];
            // our Random: the court sheet's "?" tile after the five cards
            let src = if c == UMPIRES { [96.0, 360.0, 92.0, 68.0] } else { [8.0 + 88.0 * c as f32, 392.0, 80.0, 64.0] };
            d.q(if c == UMPIRES { C02 } else { CONFIRM }, src, dst, WHITE, 128.0);
            d.hit(dst, pointer::Act::Pick(c));
        }
        (333, [62.0 + 96.0 * u as f32 - 92.0, 330.0])
    };
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [hand[0] + hand_x, hand[1], 64.0, 32.0], WHITE, 128.0);
    d.info(text.msg(title));
}

// ---------------------------------------------------------------- systems

/// The menu as a match left it (picks, seats, settings), back on its main screen.
#[derive(Resource)]
struct Kept(Menu);

#[allow(clippy::too_many_arguments)]
/// B247: the page is played by touch (a phone or tablet, `web/touch.js`): its 1P seat is the screen's.
fn touch_page() -> bool {
    #[cfg(target_arch = "wasm32")]
    return crate::web::touched();
    #[cfg(not(target_arch = "wasm32"))]
    false
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
    mut exit: MessageWriter<AppExit>,
    mut gfx: ResMut<Graphics>,
    (net, mut clipboard, mut pending): (Option<Res<crate::net::Net>>, ResMut<bevy::clipboard::Clipboard>, Option<ResMut<pointer::Pending>>),
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
        let key = keys.get_just_pressed().find(|k| Bindings::bindable_key(**k)).copied();
        let button = gamepads.iter().filter(|(_, g)| controls::readable(g)).flat_map(|(_, g)| g.get_just_pressed().copied().collect::<Vec<_>>()).find(|b| Bindings::bindable_pad(*b));
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
        let mut fx = Vec::new();
        if dev == pointed {
            for (a, tap) in std::mem::take(&mut acts) {
                fx.extend(menu.point(dev, a, tap));
            }
        }
        if trace::on() && (k != Press::default() || !fx.is_empty()) {
            eprintln!("{:9.3} step {dev:?} {k:?} pointer fx {fx:?}", time.elapsed_secs());
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
                }
                Effect::Quit => {
                    exit.write(AppExit::Success);
                }
                Effect::Launch => {
                    let a = menu.args(&pads);
                    info!("starting the match: {}", a.join(" "));
                    // the match in this app; its quit comes back to this menu as it is now
                    let (base, kept) = (args.clone(), menu.clone());
                    let back = crate::mode::Mode::new(move |app| {
                        app.insert_resource(base).insert_resource(Kept(kept));
                        crate::menu_mode(app);
                    });
                    commands.insert_resource(crate::mode::Next(Some(crate::match_mode(&args.iso, &a, back))));
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
    gamepads: Query<(Entity, &Gamepad)>,
    window: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut ticker: ResMut<Ticker>,
    mut slide: ResMut<slide::Slide>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility, &mut UiTransform)>,
    qr: Option<Res<online::QrImage>>,
    cursor: Option<Res<cursor::Cursor>>,
    icons: Res<Icons>,
    shelf: Res<mods_screen::Shelf>,
    mut hits: ResMut<pointer::Hits>,
) {
    let Some(art) = art else { return };
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g)| controls::readable(g)).map(|(e, _)| e).collect();
    pads.sort();
    // the original's menu hand wags in place (`cursor.rs`), it doesn't swing out and back as the pause menu's
    let hand_x = 0.0;
    // the description ticker counts the original's 60 Hz frames from its text's change
    ticker.1 += time.delta_secs() * 60.0;
    let d = drawn(&menu, &art.2, &bind, &pads, hand_x);
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
    let scale = window.single().map_or(1.0, |w| super::widescreen::unit_px(w));
    let share = super::widescreen::window_share(&window);
    for (Slot(i), mut img, mut node, mut vis, mut turn) in &mut q {
        let Some(quad) = quads.get(*i).filter(|q| !matches!(q.tex, Tex::Preview(_))) else {
            *vis = Visibility::Hidden;
            continue;
        };
        img.image = match quad.tex {
            Tex::Solid => art.0[0].clone(),
            Tex::Font => art.0[1].clone(),
            Tex::Hand => art.0[2].clone(),
            Tex::Cursor => match &cursor {
                Some(c) => c.image.clone(),
                None => art.0[2].clone(),
            },
            Tex::Sheet(k) => art.0[3 + k].clone(),
            Tex::Pill => art.0.last().expect("pill").clone(),
            Tex::Preview(_) => continue,
            Tex::Icon(w) => match (icons.0.get(&w), w) {
                (Some(h), _) => h.clone(),
                (None, Who::Disc(_)) => art.0[3 + FACES].clone(),
                (None, Who::Mod(_)) => {
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
            Tex::Qr => match &qr {
                Some(qr) => qr.0.clone(),
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
        };
        let [u, v, w, h] = quad.src;
        img.rect = (quad.tex != Tex::Solid && quad.tex != Tex::Qr && quad.tex != Tex::Shelf && !matches!(quad.tex, Tex::Icon(w) if icons.0.contains_key(&w)) && (quad.tex != Tex::Cursor || cursor.is_none())).then(|| Rect::new(u, v, u + w, v + h));
        img.image_mode = if quad.tile { NodeImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: scale / super::menu_art::wall_k() } } else { NodeImageMode::Stretch };
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        img.color = Color::srgba(r, g, b, quad.alpha / 128.0);
        let [mut x, mut y, mut w, mut h] = quad.dst;
        if quad.tex == Tex::Cursor && cursor.is_some() {
            // the hand's image covers `PAD` times its quad, around the same centre (B146)
            (x, y, w, h) = (x - w * (cursor::PAD - 1.0) / 2.0, y - h * (cursor::PAD - 1.0) / 2.0, w * cursor::PAD, h * cursor::PAD);
        }
        let (mut left, mut width) = super::widescreen::place(x, w, share, quad.anchor);
        let (mut top, mut height) = super::widescreen::vplace(y, h, super::widescreen::window_aspect(&window), quad.anchor);
        if quad.tex == Tex::Qr
            && let Ok(win) = window.single()
        {
            // a QR stays square (scannable) at any aspect: the largest square in its rect, centred
            let vs = super::widescreen::view_size(win);
            let side = (width * vs.x).min(height * vs.y);
            (left, top) = (left + (width - side / vs.x) / 2.0, top + (height - side / vs.y) / 2.0);
            (width, height) = (side / vs.x, side / vs.y);
        }
        node.left = Val::Percent(left);
        node.top = Val::Percent(top);
        node.width = Val::Percent(width);
        node.height = Val::Percent(height);
        turn.rotation = Rot2::radians(quad.angle);
        *vis = Visibility::Inherited;
    }
}

/// The menu's BGM tracks, `bgmm_04`..`bgmm_06`.
#[derive(Resource)]
struct MenuBgm(Vec<Option<(Arc<SoundBank>, Vec<u8>)>>);

impl Screen {
    /// Which of `MenuBgm`'s tracks plays on this screen.
    fn bgm(self) -> Option<usize> {
        match self {
            Screen::Main | Screen::Display | Screen::Graphics | Screen::Sound | Screen::Controls | Screen::ModSettings => Some(0),
            Screen::Mode | Screen::Assign | Screen::Chars => Some(1),
            Screen::Confirm => Some(2),
            Screen::Online | Screen::Lobbies | Screen::Lobby | Screen::Code => Some(0),
            Screen::Mods => Some(0),
            Screen::Playing => None,
        }
    }
}

fn load_bgm(mut commands: Commands, args: Res<Args>) {
    let Ok(mut iso) = Iso::open(&args.iso) else { return };
    let tracks = (4..=6).map(|n| SoundBank::music(&mut iso, "Menu", &format!("bgmm_{n:02}")).map(|(b, m)| (Arc::new(b), m))).collect();
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
        .add_systems(Startup, (setup, load_bgm, cursor::setup))
        .add_systems(PreUpdate, pointer::mouse_idle.after(bevy::input::InputSystems))
        // `HST_STRESS` (previews.rs) drives the select itself: no real key or pad reaches the menus or settings
        .init_resource::<Ticker>()
        .init_resource::<Icons>()
        .init_resource::<slide::Slide>()
        .add_systems(Update, ((fit_grid, step.run_if(|| std::env::var_os("HST_STRESS").is_none()), flame::tick, draw, ticker_text).chain(), bgm, cursor::tick));
    // M14: the button legend
    legend::plugin(app);
    app.add_systems(Update, legend::fill.after(step));
    app.add_systems(Update, trace::trace.after(online::poll).run_if(trace::on));
    app.init_resource::<pointer::Pending>().init_resource::<pointer::Hits>().add_systems(
        Update,
        (online::poll.after(step).before(draw), online::typing.before(step), pointer::pointer.before(step).run_if(|| std::env::var_os("HST_STRESS").is_none())),
    );
    previews::plugin(app);
    mods_screen::plugin(app);
    // B221/B244: the menus take taps, so the on-screen pad's stick keeps to its corner there; the match frees it
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Startup, || crate::web::touch_menu(true));
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn bgm_per_screen() {
        let t = [Screen::Main, Screen::Display, Screen::Controls, Screen::Mode, Screen::Assign, Screen::Chars, Screen::Confirm, Screen::Playing].map(Screen::bgm);
        assert_eq!(t, [Some(0), Some(0), Some(0), Some(1), Some(1), Some(1), Some(2), None]);
    }

    fn press(f: impl FnOnce(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    #[test]
    fn settings_round_trip() {
        let s = Settings([true, false, false, true, true, true], Graphics::LOW, Volumes::default(), 3, Ask::Each);
        assert_eq!(Settings::parse(&s.text()), s);
        let d = Graphics::default();
        assert_eq!(Settings::parse("music = off\nbogus = off\n"), Settings([true, true, false, true, true, true], d, Volumes::default(), 0, Ask::Each));
        // a fresh file: menu music on, court music off; an old file's `music` is the match's
        assert_eq!(Settings::parse(""), Settings([true, true, false, true, true, true], d, Volumes::default(), 0, Ask::Each));
        assert_eq!(Settings::parse("music = on\n"), Settings([true, true, true, true, true, true], d, Volumes::default(), 0, Ask::Each));
        // B117: an old file's widescreen on is Auto, off 4:3; `aspect` wins
        assert_eq!(Settings::parse("widescreen = on\n").3, 0);
        assert_eq!(Settings::parse("widescreen = off\n").3, 1);
        assert_eq!(Settings::parse("widescreen = off\naspect = 21:9\n").3, 4);
        // Upscaled Downloads follows Upscaled Textures unless it's set itself
        assert!(!Settings::parse("upscaled_textures = off\n").0[UPSCALED_DOWNLOADS]);
        assert!(Settings::parse("upscaled_textures = off\nupscaled_downloads = on\n").0[UPSCALED_DOWNLOADS]);
        assert_eq!(Settings::parse("menu_music = off\n"), Settings([true, false, false, true, true, true], d, Volumes::default(), 0, Ask::Each));
        assert!(!Settings::default().flags().contains(&"--music".to_string()));
        assert!(!Settings([true, true, false, true, true, true], d, Volumes::default(), 0, Ask::Each).flags().contains(&"--music".to_string()));
        assert!(Settings([true, false, true, true, true, true], d, Volumes::default(), 0, Ask::Each).flags().contains(&"--music".to_string()));
        // B289: the online mod answer round-trips; an unknown word keeps Ask
        let mut a = Settings::default();
        a.4 = Ask::Never;
        assert_eq!(Settings::parse(&a.text()), a);
        assert_eq!(Settings::parse("online_mods = maybe\n").4, Ask::Each);
        // M20: chat shown unless turned off
        assert!(!Settings::parse("chat = off\n").0[CHAT] && Settings::parse("").0[CHAT]);
        assert_eq!(Settings::parse("uncapped_fps = off\n").1, Graphics { vsync: true, ..d });
        assert_eq!(s.flags()[..1], ["--fps"]);
        assert_eq!(Settings([false, false, true, false, true, true], d, Volumes::default(), 1, Ask::Each).flags()[..2], ["--no-upscale", "--music"]);
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
        }
    }

    /// B216: on the seat assignment, 1P adds a computer player on 2P.
    fn add_com(m: &mut Menu) {
        assert_eq!(m.screen, Screen::Assign);
        m.step(Dev::Keys, press(|p| &mut p.down));
        m.step(Dev::Keys, press(|p| &mut p.card));
        assert!(m.bots[1]);
    }

    /// B216 / B258: a singles match with nobody on 2P: once 1P is ready, 2P's COM card starts on the first character
    /// nobody has (first costume); ✕ keeps it, ○ leaves it to the computer, or 1P picks another.
    #[test]
    fn an_open_seat_is_a_com_card() {
        let (ok, back) = (press(|p| &mut p.ok), press(|p| &mut p.back));
        let mut m = Menu::default();
        for _ in 0..3 {
            m.step(Dev::Keys, ok);
        }
        assert_eq!(m.screen, Screen::Chars);
        assert_eq!(m.driven(Dev::Keys), Some(0));
        m.step(Dev::Keys, ok);
        assert_eq!((m.screen, m.driven(Dev::Keys)), (Screen::Chars, Some(1)));
        assert!(!m.players[1].ready);
        assert_eq!((m.players[1].char(), m.players[1].costume), (1, 0));
        // 1P moves the COM's cursor, then backs out: the computer's own pick again
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_ne!(m.players[1].char(), 1);
        m.step(Dev::Keys, back);
        assert_eq!(m.screen, Screen::Confirm);
        assert!(m.players[1].ready);
        assert_eq!((m.players[1].char(), m.players[1].costume), (1, 0));
        // back to the select: everyone picks again; this time 1P picks the COM's character
        m.step(Dev::Keys, back);
        assert!(m.screen == Screen::Chars && !m.players[1].ready && m.open(1));
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
        m.step(Dev::Keys, back);
        assert_eq!(m.screen, Screen::Confirm);
        let chars: Vec<usize> = m.players.iter().map(|p| p.char()).collect();
        assert!(chars[2] != chars[3] && !chars[..2].contains(&chars[2]) && !chars[..2].contains(&chars[3]), "{chars:?}");
    }

    #[test]
    fn locked_costumes_and_match_order() {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let mut m = Menu::default();
        let ok = press(|p| &mut p.ok);
        m.step(Dev::Keys, ok); // Local
        assert_eq!(m.screen, Screen::Mode);
        m.step(Dev::Keys, press(|p| &mut p.down)); // doubles
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Assign);
        m.step(pad, ok); // the pad joins as 2P
        assert_eq!(m.seats[..2], [Some(Dev::Keys), Some(pad)]);
        // B216: 1P adds a computer player on 3P (△ on its seat, twice takes it off, three times back on); 4P stays open
        m.step(Dev::Keys, press(|p| &mut p.down));
        m.step(Dev::Keys, press(|p| &mut p.card));
        assert!(!m.bots[1], "2P has a device");
        m.step(Dev::Keys, press(|p| &mut p.down));
        for _ in 0..3 {
            m.step(Dev::Keys, press(|p| &mut p.card));
        }
        assert_eq!(m.bots, [false, false, true, false]);
        // a device joining passes over the computer's seat
        let pad2 = Dev::Pad(Entity::from_raw_u32(11).unwrap());
        m.step(pad2, ok);
        assert_eq!(m.seats[3], Some(pad2));
        m.step(pad2, press(|p| &mut p.back));
        assert_eq!(m.seats[3], None);
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Chars);
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
        // 1P now picks the computer's 3P: Cody; the open 4P starts on the first character nobody has (Jun)
        assert_eq!(m.driven(Dev::Keys), Some(2));
        m.step(Dev::Keys, press(|p| &mut p.right));
        m.step(Dev::Keys, ok);
        // B258: then the open 4P's COM card, on its automatic pick
        assert_eq!(m.driven(Dev::Keys), Some(3));
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Confirm);
        let a = m.args(&[Entity::from_raw_u32(3).unwrap(), Entity::from_raw_u32(9).unwrap()]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        // match order: 1P, 3P, 2P, 4P
        assert_eq!(flag("--chars"), "0,1,0,2");
        assert_eq!(flag("--outfits"), "0,0,1,0");
        assert_eq!(flag("--pads"), "k,1,-,-");
        assert_eq!((flag("--umpire"), flag("--sets"), flag("--games")), ("4".into(), "1".into(), "4".into()));
        assert!(!a.contains(&"--singles".to_string()));
        // B66: a character with no free costume is passed over by left/right/up/down (one-costume mods, 4 a row:
        // 1 and 4 taken), never stopped on
        let one = |i: usize| crate::mods::Mod { id: format!("m{i}"), costumes: vec!["model/c00.glb".into()], ..fake_mod(i) };
        let mut m = Menu { screen: Screen::Chars, doubles: true, filter: 2, mods: (0..9).map(one).collect(), seats: [Some(Dev::Keys), Some(pad), None, None], ..default() };
        m.players = [Player { modded: true, ..default() }; 4];
        for (q, i) in [(1, 1), (2, 4)] {
            (m.players[q].pick, m.players[q].ready) = (i, true);
        }
        let go = |m: &mut Menu, f: fn(&mut Press) -> &mut bool| {
            m.step(Dev::Keys, press(f));
            m.cell(0)
        };
        // B296: the roster under the Mods filter: the Random card, then the mods, 7 a row at 4:3
        assert_eq!(go(&mut m, |p| &mut p.right), Some(Who::Mod(2)));
        assert_eq!(go(&mut m, |p| &mut p.left), Some(Who::Mod(0)));
        assert_eq!(go(&mut m, |p| &mut p.left), None, "the Random card");
        assert_eq!(go(&mut m, |p| &mut p.left), None, "the roster's start");
        assert_eq!(go(&mut m, |p| &mut p.right), Some(Who::Mod(0)));
        assert_eq!(go(&mut m, |p| &mut p.down), Some(Who::Mod(7)));
        assert_eq!(go(&mut m, |p| &mut p.up), Some(Who::Mod(0)));
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

    #[test]
    fn random_court_and_umpire() {
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], court: 0, ..default() };
        let (left, ok) = (press(|p| &mut p.left), press(|p| &mut p.ok));
        m.step(Dev::Keys, press(|p| &mut p.down));
        m.step(Dev::Keys, ok);
        m.step(Dev::Keys, left); // court 0 (the strip's first tile) → Random, after the last tile
        m.step(Dev::Keys, ok);
        assert_eq!(m.court, COURTS);
        for _ in 0..3 {
            m.step(Dev::Keys, press(|p| &mut p.down));
        }
        m.step(Dev::Keys, ok);
        m.step(Dev::Keys, press(|p| &mut p.right)); // umpire 4 → Random
        m.step(Dev::Keys, ok);
        assert_eq!(m.umpire as usize, UMPIRES);
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

    /// B64: ✕ on Select Court / Select Umpire opens the original's picker; ←→ walk its strip, ○ puts the value back.
    #[test]
    fn court_and_umpire_pickers() {
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], court: 0, umpire: 0, ..default() };
        let [down, left, right, ok, back] = [press(|p| &mut p.down), press(|p| &mut p.left), press(|p| &mut p.right), press(|p| &mut p.ok), press(|p| &mut p.back)];
        m.step(Dev::Keys, down);
        m.step(Dev::Keys, right); // a closed row ignores ←→
        assert_eq!((m.court, m.sel), (0, 1));
        m.step(Dev::Keys, ok);
        let walk: Vec<usize> = (0..COURTS + 1).map(|_| (m.step(Dev::Keys, right), m.court).1).collect();
        assert_eq!(walk, [3, 5, 1, 7, 8, 10, 9, 2, 6, 4, COURTS, 0]); // the original's tiles 2…11, Random, 1 again
        m.step(Dev::Keys, right);
        m.step(Dev::Keys, back);
        assert_eq!((m.court, m.picking, m.screen), (0, None, Screen::Confirm));
        for _ in 0..3 {
            m.step(Dev::Keys, down);
        }
        m.step(Dev::Keys, ok);
        assert_eq!(m.step(Dev::Keys, left), []); // the umpire cards don't wrap
        for _ in 0..9 {
            m.step(Dev::Keys, right);
        }
        m.step(Dev::Keys, ok);
        assert_eq!((m.umpire as usize, m.picking), (UMPIRES, None));
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
        // on the select: held open (still a human's), the next device's ✕ takes it, the dropped one's does nothing
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Chars);
        m.step(pb, ok);
        assert!(m.players[2].ready);
        m.step(pb, press(|p| &mut p.back)); // ready → picking again
        m.step(pb, press(|p| &mut p.back)); // ○ off its seat
        assert_eq!((m.seats[2], m.players[2].ready), (Some(Dev::Open), false));
        assert_eq!(m.driven(pb), None);
        m.step(pb, ok);
        assert_eq!(m.seats[2], Some(pb));
        m.point(Dev::Keys, pointer::Act::Drop(2), true);
        m.step(pa, ok); // seated already: readies its own, doesn't take the open seat
        assert_eq!((m.seats[2], m.players[1].ready), (Some(Dev::Open), true));
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Chars); // not to the confirm screen with a seat open
        let keys2 = Dev::Pad(Entity::from_raw_u32(7).unwrap());
        m.step(keys2, ok);
        assert_eq!(m.seats[2], Some(keys2));
        // 1P backing out to the assignment frees seats still open
        m.point(Dev::Keys, pointer::Act::Drop(2), true);
        m.step(Dev::Keys, press(|p| &mut p.back));
        m.step(Dev::Keys, press(|p| &mut p.back));
        assert_eq!((m.screen, m.seats), (Screen::Assign, [Some(Dev::Keys), Some(pa), None, None]));
    }

    #[test]
    fn dropping_1p_moves_the_next_device_up() {
        let pa = Dev::Pad(Entity::from_raw_u32(5).unwrap());
        let mut m = Menu { screen: Screen::Assign, seats: [Some(Dev::Keys), None, None, None], ..default() };
        assert!(!m.droppable(0)); // alone: stays
        m.step(pa, press(|p| &mut p.ok));
        assert!(m.point(Dev::Keys, pointer::Act::Drop(0), true).contains(&Effect::Sound(1)));
        assert_eq!(m.seats, [Some(pa), None, None, None]);
        m.step(Dev::Keys, press(|p| &mut p.ok)); // the keyboard rejoins as 2P
        assert_eq!(m.seats[..2], [Some(pa), Some(Dev::Keys)]);
        // 1P focuses 2P and swaps it back up, then takes the pad off
        m.step(pa, press(|p| &mut p.down));
        m.step(pa, press(|p| &mut p.left));
        assert_eq!((&m.seats[..2], m.sel), (&[Some(Dev::Keys), Some(pa)][..], 0));
        m.step(Dev::Keys, press(|p| &mut p.down));
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert_eq!(m.seats[..2], [Some(Dev::Keys), None]);
        m.step(Dev::Keys, press(|p| &mut p.right)); // an empty seat never goes to 1P
        assert_eq!(m.seats[0], Some(Dev::Keys));
        m.step(pa, press(|p| &mut p.ok));
        m.step(pa, press(|p| &mut p.ok));
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!(m.screen, Screen::Chars);
        assert!(!m.droppable(0)); // the select's driver stays
    }

    #[test]
    fn four_humans_any_device_any_seat() {
        let [p0, p1, p2] = [3, 9, 11].map(|n| Dev::Pad(Entity::from_raw_u32(n).unwrap()));
        let ok = press(|p| &mut p.ok);
        let mut m = Menu::default();
        m.step(p1, ok); // a pad chose Local: it is 1P
        m.step(p1, press(|p| &mut p.down)); // doubles
        m.step(p1, ok);
        m.step(Dev::Keys, ok); // the keyboard joins as 2P
        m.step(p0, ok); // 3P
        m.step(Dev::Keys, press(|p| &mut p.right)); // the keyboard moves to the free 4P (other team)
        assert_eq!(m.seats, [Some(p1), None, Some(p0), Some(Dev::Keys)]);
        m.step(p2, ok); // the last pad takes 2P
        m.step(Dev::Keys, press(|p| &mut p.left)); // no free seat: stays
        assert_eq!(m.seats, [Some(p1), Some(p2), Some(p0), Some(Dev::Keys)]);
        m.step(p1, ok);
        assert_eq!(m.screen, Screen::Chars);
        // every seat is a human: each readies its own player, 1P never drives another
        for (i, d) in [p1, p2, p0, Dev::Keys].into_iter().enumerate() {
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
        assert_eq!(flag("--pads"), "1,2,0,k");
        // the match reads them back
        let seats: Vec<_> = flag("--pads").split(',').map(Seat::parse).collect();
        assert_eq!(seats, [Some(Seat::Pad(1)), Some(Seat::Pad(2)), Some(Seat::Pad(0)), Some(Seat::Keys)]);
        // singles: the keyboard can be 2P
        let mut m = Menu::default();
        m.step(p0, ok);
        m.step(p0, ok);
        m.step(Dev::Keys, ok);
        m.step(p1, ok); // no seat left
        assert_eq!(m.seats, [Some(p0), Some(Dev::Keys), None, None]);
        assert_eq!(m.args(&[]).iter().skip_while(|x| *x != "--pads").nth(1).unwrap(), "-,k");
    }

    /// B296: the select's layout at 4:3 holds the whole disc roster on one page; wider windows add columns, tall
    /// ones rows; at any aspect its strip, roster and detail panel stay in the view, apart, over the description bar.
    #[test]
    fn select_fits_the_window() {
        use super::super::widescreen::{share, vplace, vshare};
        let fit = |a: f32| (Grid::fit(a).cols, Grid::fit(a).rows);
        assert_eq!(fit(4.0 / 3.0), (7, 3));
        assert!(Grid::fit(4.0 / 3.0).per() > CHARS);
        assert_eq!(fit(16.0 / 9.0), (11, 3));
        assert!(fit(21.0 / 9.0).0 > 11 && fit(9.0 / 16.0).1 > 3);
        for a in [0.4, 9.0 / 16.0, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
            let g = Grid::fit(a);
            let (ex, ey) = ((640.0 / share(a) - 640.0) / 2.0, (448.0 / vshare(a) - 448.0) / 2.0);
            let ticker = vplace(404.0, 32.0, a, Anchor::Span).0;
            let last = g.cell(g.per() - 1);
            for r in [g.strip, g.roster, g.detail] {
                assert!(r[0] >= -ex && r[0] + r[2] <= 640.0 + ex + 1e-3 && r[1] >= -ey, "{a} {r:?}");
                let (t, h) = vplace(r[1], r[3], a, Anchor::Centre);
                assert!(t + h <= ticker + 1e-3, "{a} {r:?}");
            }
            assert!(g.strip[1] + g.strip[3] < g.roster[1] && g.strip[1] + g.strip[3] < g.detail[1], "{a}");
            let apart = g.roster[0] + g.roster[2] < g.detail[0] || g.roster[1] + g.roster[3] < g.detail[1];
            assert!(apart, "{a}");
            assert!(last[0] + last[2] <= g.roster[0] + g.roster[2] && last[1] + last[3] <= g.roster[1] + g.roster[3] - 20.0, "{a} {last:?}");
        }
    }

    /// B295: the lobby and confirm screens at any aspect: the seats column and side card in the view, apart, over the
    /// legend strip; the seats and help line in their column; every button on the side card, none overlapping.
    #[test]
    fn lobby_and_confirm_fit_the_window() {
        use super::super::widescreen::{share, vplace, vshare};
        let inside = |r: [f32; 4], c: [f32; 4]| r[0] >= c[0] - 1e-3 && r[1] >= c[1] - 1e-3 && r[0] + r[2] <= c[0] + c[2] + 1e-3 && r[1] + r[3] <= c[1] + c[3] + 1e-3;
        let overlap = |a: [f32; 4], b: [f32; 4]| a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3];
        for a in [0.4, 9.0 / 16.0, 0.99, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
            let g = Grid::fit(a);
            let (ex, ey) = ((640.0 / share(a) - 640.0) / 2.0, (448.0 / vshare(a) - 448.0) / 2.0);
            let legend = vplace(436.0, 12.0, a, Anchor::Span).0;
            let (col, side) = g.split();
            for r in [col, side] {
                assert!(r[0] >= -ex && r[0] + r[2] <= 640.0 + ex + 1e-3 && r[1] >= -ey && r[2] > 0.0, "{a} {r:?}");
                let (t, h) = vplace(r[1], r[3], a, Anchor::Centre);
                assert!(t + h <= legend + 1e-3, "{a} {r:?}");
            }
            assert!(!overlap(col, side), "{a}");
            for doubles in [false, true] {
                assert!(inside(help_rect(col, doubles), col), "{a} {doubles}");
            }
            let mut m = Menu { screen: Screen::Confirm, grid: g, ..default() };
            let rows = confirm_rects(&m);
            assert!(rows.iter().all(|&r| inside(r, side)) && (0..5).all(|i| (0..i).all(|j| !overlap(rows[i], rows[j]))), "{a} {rows:?}");
            // the lobby, hosting with a computer player on 4P, and as a guest
            m.screen = Screen::Lobby;
            online::sample(&mut m);
            m.net.room.as_mut().unwrap().seats[3] = Some(crate::net::Seat { peer: crate::net::CPU.into(), pick: crate::net::Pick::disc(9, 4) });
            for hosting in [true, false] {
                m.net.hosting = hosting;
                let rects = m.lobby_rects();
                let [l, t, r, b] = g.reach();
                assert!(rects.iter().all(|&x| inside(x, [l, t, r - l, b - t])), "{a} {hosting} {rects:?}");
                for i in 0..rects.len() {
                    for j in 0..i {
                        // a computer's ✕ sits on its own seat's row
                        assert!(!overlap(rects[i], rects[j]) || rects[i][2] == 26.0, "{a} {hosting} {i} {j} {rects:?}");
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
            let n = drawn(&m, &t, &Bindings::default(), &[], 0.0).out.len();
            assert!(n < POOL * 9 / 10, "{a}: {n} quads");
        }
    }

    /// B296: the roster's filters: everyone, the disc's, the mods (B304: no per-game ones, whatever the mods' games);
    /// a player whose character a filter hides moves to its first, a ready one stays.
    #[test]
    fn roster_filters() {
        let from = |i: usize, s: &str| crate::mods::Mod { id: format!("m{i}"), source: s.into(), costumes: vec!["a".into()], ..fake_mod(i) };
        let mut m = Menu { screen: Screen::Chars, mods: vec![from(0, "Golf"), from(1, ""), from(2, "Get a Grip"), from(3, "Golf")], seats: [Some(Dev::Keys), Some(Dev::Keys), None, None], ..default() };
        assert_eq!(m.filters(), 3);
        assert_eq!(m.roster().len(), 1 + CHARS + 4);
        m.players[1] = Player { cursor: 3, ready: true, ..default() };
        m.set_filter(2);
        assert_eq!(m.roster(), [None, Some(Who::Mod(0)), Some(Who::Mod(1)), Some(Who::Mod(2)), Some(Who::Mod(3))], "the mods");
        assert_eq!((m.cell(0), m.cell(1)), (Some(Who::Mod(0)), Some(Who::Disc(GRID[3].0))));
        m.set_filter(3);
        assert_eq!((m.filter, m.roster().len()), (0, 1 + CHARS + 4), "the filters wrap");
        m.set_filter(1);
        assert_eq!(m.cell(0), Some(Who::Disc(GRID[0].0)));
        // the shown player: the one who pressed last while picking
        m.players[1].ready = false;
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
        };
        let ok = press(|p| &mut p.ok);
        let mut m = Menu { mods: vec![fake("a", 5, 2), fake("b", 9, 1)], ..default() };
        m.step(Dev::Keys, ok); // Local, singles
        m.step(Dev::Keys, ok);
        add_com(&mut m);
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Chars);
        // B296: □ / Tab: All → Hot Shots Tennis → Mods, everyone moving onto the first mod
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert_eq!((m.filter, m.players[0].modded), (1, false));
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert!(m.filter == 2 && m.players[0].modded && m.players[1].modded);
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
        // back from the confirm: everyone picks again on the page shown; Tab with no mods is refused
        m.step(Dev::Keys, press(|p| &mut p.back));
        assert!(m.players[0].modded && !m.players[0].ready);
        let mut m = Menu::default();
        m.screen = Screen::Chars;
        m.seats[0] = Some(Dev::Keys);
        assert_eq!(m.step(Dev::Keys, press(|p| &mut p.page)), [Effect::Sound(1)]);
        assert_eq!(m.filter, 0);
        // M1n: a costume whose model failed to load is passed by L1 / R1 and can't be readied on
        let mut m = Menu { mods: vec![fake("c", 3, 3)], ..default() };
        for _ in 0..3 {
            m.step(Dev::Keys, ok);
        }
        m.step(Dev::Keys, press(|p| &mut p.page));
        m.step(Dev::Keys, press(|p| &mut p.page));
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
        };
        // Ashley (grid 0, character 0) gets 2 + 1 more; Cody none
        let m = Menu { mods: vec![pack("x", 0, 2), pack("y", 1, 3), pack("z", 0, 1)], ..default() };
        assert_eq!((m.costumes(Who::Disc(0)), m.costumes(Who::Disc(1)), m.costumes(Who::Disc(2))), (13, 13, 10));
        assert_eq!(m.resolve(Who::Disc(0), 9), (Who::Disc(0), 9));
        assert_eq!(m.resolve(Who::Disc(0), 11), (Who::Mod(0), 1));
        assert_eq!(m.resolve(Who::Disc(0), 12), (Who::Mod(2), 0));
        let ok = press(|p| &mut p.ok);
        let mut m = m;
        for _ in 0..2 {
            m.step(Dev::Keys, ok);
        }
        add_com(&mut m);
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Chars);
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

    /// B79: Select flips the hand. Ashley (right) + Select and Will (left) + Select go to the match as
    /// `--swap-hands 1,1`, and the match starts with her left-handed and him right-handed (the AI's strong side stays
    /// TParam's). The match half needs the ISO at the repository root (skipped without it).
    #[test]
    fn select_swaps_the_hand() {
        let ok = press(|p| &mut p.ok);
        let hand = press(|p| &mut p.hand);
        let mut m = Menu::default();
        for _ in 0..2 {
            m.step(Dev::Keys, ok);
        }
        add_com(&mut m);
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Chars);
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
        // SAFETY: only `Pads::slot_of` reads it; every player is a bot
        unsafe { std::env::set_var("HST_AUTOPLAY", "1") };
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
