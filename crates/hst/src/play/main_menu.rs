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
//! (the first free seat, ←/→ to change seat, ○ to leave), up to four humans, any device on any seat; the rest are the
//! computer's. Player slots in the match: singles 1P→0, 2P→1;
//! doubles 1P→0, 2P→2 (1P's partner), 3P→1, 4P→3. In the character select a costume someone has locked in (ready)
//! is closed to everyone else on that character; the computer's players are picked by player 1 after the humans.
//! `HST_MENU=<screen>` (main, settings, graphics, controls, mode, assign, chars, attrs = chars with 1P's △ card, coms = chars with 1P ready (the COM cards), mods = the custom page, confirm) opens on that screen (for `--shot`).
//!
//! Custom characters (remaster-only): every standard mod under `mods/` beside the disc image (`mods::list`) is on a
//! second page of the select, switched for everyone by Tab / □: a list of names, 2 columns of [`LIST_ROWS`] a
//! page (↑↓ one, ←→ a column). A card on a mod shows its 3D preview, its name in `word.tm2`, its TParam.csv row's
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
use crate::audio::{Sound, SoundBank};

pub use super::widescreen::WIDE;

mod cursor;
pub(crate) mod flame;
mod online;
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
/// Rows a column of the custom list; two columns a page.
const LIST_ROWS: usize = 12;
/// The custom list's columns (x) and first row (y), one row 22 px.
const LIST: [f32; 3] = [176.0, 324.0, 96.0];
/// The original's courts on the confirm screen (names `message0` 276 + 2k, descriptions 581 + 2k).
const COURTS: usize = 11;
/// Each court's surface on the confirm screen (0 grass, 1 clay, 2 hard; `hst_sim::ball::COURTS` bounces the same).
const SURFACE: [usize; COURTS] = [1, 1, 1, 2, 0, 0, 2, 0, 1, 0, 2];
/// The confirm screen's ball meters per surface: bounce height and ball speed, out of 4.
const METERS: [(usize, usize); 3] = [(2, 4), (3, 2), (4, 3)];
/// The umpires; the value after the last court or umpire is a random pick (`Menu::venue`).
const UMPIRES: usize = 5;
/// Sets on the confirm screen (sets played; to win: (n + 1) / 2).
const SETS: [i32; 3] = [1, 3, 5];
/// Player colours: 1P pink, 2P orange, 3P blue, 4P green (`CharacterSelect2_03`'s cards, same order).
const COLOURS: [[f32; 3]; 4] = [[128.0, 60.0, 90.0], [128.0, 80.0, 20.0], [40.0, 90.0, 128.0], [70.0, 115.0, 30.0]];
const CARDS: [[f32; 2]; 4] = [[4.0, 4.0], [4.0, 180.0], [156.0, 4.0], [156.0, 180.0]];
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

/// The enhancements, each on or off (all on by default), and the graphics settings, saved in `settings.txt`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Settings([bool; 4], Graphics);
/// `music` is the match's (an older `settings.txt`'s one Music switch keeps meaning the match).
const SETTING_NAMES: [&str; 4] = ["widescreen", "upscaled_textures", "menu_music", "music"];
const SETTING_LABELS: [&str; 4] = ["Widescreen", "Upscaled Textures", "Menu Music", "Match Music"];
/// The Settings screen's rows after the switches.
const GRAPHICS_ROW: usize = SETTING_NAMES.len();
const CONTROLS_ROW: usize = GRAPHICS_ROW + 1;
const GRAPHICS_LABELS: [&str; 7] = ["Preset", "Resolution", "Shadows", "Frame Limit", "VSync", "Original Mode", "Anti-aliasing"];

/// Widescreen, upscaled textures and menu music on, match music off (user 2026-10-12).
const SETTING_DEFAULTS: [bool; 4] = [true, true, true, false];

impl Default for Settings {
    fn default() -> Self {
        Settings(SETTING_DEFAULTS, Graphics::default())
    }
}

/// The next (`d` = 1) or previous (−1) of `list` after `cur` (the first if `cur` isn't in it).
fn cycle<T: PartialEq + Copy>(list: &[T], cur: T, d: isize) -> T {
    let i = list.iter().position(|x| *x == cur).map_or(0, |i| (i as isize + d).rem_euclid(list.len() as isize) as usize);
    list[i]
}

impl Settings {
    fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for (k, v) in text.lines().filter_map(|l| l.split('#').next()?.split_once('=')) {
            if let Some(i) = SETTING_NAMES.iter().position(|n| *n == k.trim()) {
                s.0[i] = v.trim() != "off";
            } else if k.trim() == "uncapped_fps" {
                // before the graphics settings: off waited for the display
                s.1.vsync = v.trim() == "off";
            } else {
                s.1.set(k, v);
            }
        }
        s
    }

    fn text(&self) -> String {
        let on: String = SETTING_NAMES.iter().zip(self.0).map(|(n, on)| format!("{n} = {}\n", if on { "on" } else { "off" })).collect();
        on + &self.1.text()
    }

    /// Change graphics row `row` (`GRAPHICS_LABELS`) one step in direction `d`.
    fn graphics_step(&mut self, row: usize, d: isize) {
        use crate::graphics::{FPS, MSAA, SCALES, SHADOWS};
        let g = &mut self.1;
        match row {
            0 => {
                let names: Vec<_> = Graphics::PRESETS.iter().map(|(n, _)| *n).collect();
                let n = cycle(&names, g.preset(), d);
                *g = Graphics::PRESETS.iter().find(|(m, _)| *m == n).unwrap().1;
            }
            1 => g.scale = cycle(&SCALES, g.scale, d),
            2 => g.shadows = cycle(&SHADOWS, g.shadows, d),
            3 => g.fps = cycle(&FPS, g.fps, d),
            4 => g.vsync = !g.vsync,
            5 => g.original = !g.original,
            _ => g.msaa = cycle(&MSAA, g.msaa, d),
        }
    }

    /// The value shown beside graphics row `row`.
    fn graphics_value(&self, row: usize) -> String {
        let g = &self.1;
        match row {
            0 => g.preset().into(),
            1 => format!("{} %", g.scale),
            2 => ["Off", "Low", "High"][g.shadows as usize].into(),
            3 if g.fps == 0 => "Uncapped".into(),
            3 => format!("{} fps", g.fps),
            4 => if g.vsync { "On" } else { "Off" }.into(),
            5 => if g.original { "On" } else { "Off" }.into(),
            _ => ["Off", "2x", "4x"][g.msaa.ilog2().min(2) as usize].into(),
        }
    }

    fn path(iso: &str) -> std::path::PathBuf {
        std::path::Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("settings.txt")
    }

    /// The match's flags for these settings.
    fn flags(&self) -> Vec<String> {
        let [wide, upscale, _, music] = self.0;
        let on = [(!wide, "--4x3"), (!upscale, "--no-upscale"), (music, "--music")].into_iter().filter(|(on, _)| *on).map(|(_, f)| f.to_string());
        on.chain(self.1.flags()).collect()
    }
}

// ---------------------------------------------------------------- state

/// An input device: the keyboard or a gamepad.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dev {
    Keys,
    Pad(Entity),
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
    /// □ / Tab: the disc or custom page.
    page: bool,
    /// Select / − (keyboard: the camera key, C): the hand toggle.
    hand: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    /// `bgmm_04` plays on the first three, `bgmm_05` on the next three, `bgmm_06` on Confirm (the MENU overlay's track
    /// switch, measured per screen).
    Main,
    Settings,
    Graphics,
    Controls,
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
    /// A match is running (the window is hidden).
    Playing,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Player {
    /// Grid index.
    cursor: usize,
    costume: usize,
    ready: bool,
    card: bool,
    /// The custom list's entry, and whether the player is on it (else on the grid).
    pick: usize,
    modded: bool,
    /// Select / −: plays the character with the other hand (`hand_swap`), kept across characters.
    swap: bool,
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
    players: [Player; 4],
    court: usize,
    sets: usize,
    games: i32,
    umpire: u8,
    /// The dice for a random court or umpire, thrown at Start (`venue`).
    roll: u64,
    /// The action waiting for a new key or button (controls screen).
    rebinding: Option<usize>,
    /// The custom roster, and whether the select shows it.
    mods: Vec<crate::mods::Mod>,
    custom: bool,
    /// M1n: (custom roster entry, costume) whose model failed to load in the select: L1 / R1 pass it, it can't be
    /// readied on.
    broken: std::collections::HashSet<(usize, usize)>,
    /// The Online screens' state; `net.go` is set while the local screens pick this machine's players for Online.
    net: online::View,
    /// The confirmation screen's flame, from P1's pick.
    flame: flame::Flame,
}

impl Default for Menu {
    fn default() -> Self {
        Menu {
            screen: Screen::Main,
            sel: 0,
            settings: Settings::default(),
            doubles: false,
            seats: [None; 4],
            players: [Player::default(); 4],
            court: 0,
            sets: 0,
            games: 4,
            umpire: 4,
            roll: 0,
            rebinding: None,
            mods: Vec::new(),
            custom: false,
            broken: default(),
            net: online::View::default(),
            flame: flame::Flame::default(),
        }
    }
}

/// Rows per screen.
#[cfg(not(target_arch = "wasm32"))]
const MAIN: [&str; 4] = ["Local", "Online", "Settings", "Quit"];
/// A web page can't close itself (an `AppExit` there only freezes it), so the browser build has no Quit.
#[cfg(target_arch = "wasm32")]
const MAIN: [&str; 3] = ["Local", "Online", "Settings"];
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
        matches!(self.pick(p), (Who::Mod(i), k) if self.broken.contains(&(i, k)))
            || (0..self.players()).any(|q| q != p && self.players[q].ready && self.pick(q) == self.pick(p))
    }

    /// Player `p` picking again, on the page shown.
    fn unready(&mut self, p: usize) {
        let custom = self.custom;
        let me = &mut self.players[p];
        me.ready = false;
        if me.modded != custom {
            me.modded = custom;
            me.costume = 0;
        }
    }

    /// The player `dev` drives on the character select: its own until it is ready, then (player 1's device, once
    /// every human is ready) the first computer player not ready.
    fn driven(&self, dev: Dev) -> Option<usize> {
        let n = self.players();
        if let Some(p) = (0..n).find(|&p| self.seats[p] == Some(dev)) {
            let humans_ready = (0..n).all(|q| self.seats[q].is_none() || self.players[q].ready);
            if !self.players[p].ready || p != 0 || !humans_ready || self.online() {
                return Some(p);
            }
            return (0..n).find(|&q| self.seats[q].is_none() && !self.players[q].ready).or(Some(p));
        }
        None
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
        if self.screen == Screen::Chars && self.online() && self.seats[0].is_none() {
            self.seats[0] = Some(dev);
        }
        let host = self.seats[0];
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
                        2 => self.go(Screen::Settings, &mut out),
                        _ => out.push(Quit),
                    }
                }
            }
            Screen::Settings => {
                rows(CONTROLS_ROW + 2, &mut self.sel, &mut out);
                let s = self.sel;
                if s < SETTING_NAMES.len() && (k.ok || k.left || k.right) {
                    self.settings.0[s] = !self.settings.0[s];
                    out.extend([Sound(0), SaveSettings]);
                } else if k.ok && s == GRAPHICS_ROW {
                    self.go(Screen::Graphics, &mut out);
                } else if k.ok && s == CONTROLS_ROW {
                    self.go(Screen::Controls, &mut out);
                } else if (k.ok && s > CONTROLS_ROW) || k.back {
                    self.sel = 2;
                    self.screen = Screen::Main;
                    out.push(Sound(1));
                }
            }
            Screen::Graphics => {
                rows(GRAPHICS_LABELS.len() + 1, &mut self.sel, &mut out);
                let s = self.sel;
                if s < GRAPHICS_LABELS.len() && (k.ok || k.left || k.right) {
                    self.settings.graphics_step(s, if k.left { -1 } else { 1 });
                    out.extend([Sound(0), SaveSettings]);
                } else if k.ok || k.back {
                    self.sel = GRAPHICS_ROW;
                    self.screen = Screen::Settings;
                    out.push(Sound(1));
                }
            }
            Screen::Controls => {
                if self.rebinding.is_some() {
                    return out;
                }
                let n = controls::action_names().count();
                rows(n + 1, &mut self.sel, &mut out);
                if k.ok && self.sel < n {
                    self.rebinding = Some(self.sel);
                    out.push(Sound(2));
                } else if (k.ok && self.sel == n) || k.back {
                    self.sel = CONTROLS_ROW;
                    self.screen = Screen::Settings;
                    out.push(Sound(1));
                }
            }
            Screen::Mode if host == Some(dev) => {
                if k.up || k.down || k.left || k.right {
                    self.doubles = !self.doubles;
                    out.push(Sound(0));
                }
                if k.ok {
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
                        self.custom = false;
                        self.go(Screen::Chars, &mut out);
                    } else if k.back {
                        self.seats[1..].fill(None);
                        self.screen = Screen::Mode;
                        out.push(Sound(1));
                    }
                } else if k.ok && seated.is_none() {
                    let n = if self.online() { self.local_max() } else { n };
                    if let Some(p) = (0..n).find(|&p| self.seats[p].is_none()) {
                        self.seats[p] = Some(dev);
                        out.push(Sound(2));
                    }
                } else if (k.left || k.right || k.up || k.down) && let Some(p) = seated {
                    // the next free seat that way (1P stays the host's)
                    let d = if k.right || k.down { 1 } else { n - 2 };
                    let to = (1..n - 1).map(|j| 1 + (p - 1 + d * j) % (n - 1)).find(|&q| self.seats[q].is_none() && !(self.online() && q >= self.local_max()));
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
                let Some(p) = self.driven(dev) else { return out };
                if k.page {
                    if self.mods.is_empty() {
                        out.push(Sound(1));
                    } else {
                        self.custom = !self.custom;
                        for q in 0..self.players() {
                            if !self.players[q].ready {
                                self.unready(q);
                            }
                        }
                        out.push(Sound(0));
                    }
                }
                let costumes = self.costumes(self.who(p));
                let broken = |c: usize| matches!(self.resolve(self.who(p), c), (Who::Mod(i), k) if self.broken.contains(&(i, k)));
                let broken: Vec<bool> = (0..costumes).map(broken).collect();
                let n_mods = self.mods.len();
                let me = &mut self.players[p];
                if k.card {
                    me.card = !me.card;
                    out.push(Sound(0));
                }
                if k.hand && !me.ready {
                    me.swap = !me.swap;
                    out.push(Sound(0));
                }
                if !me.ready {
                    let (dx, dy) = ((k.right as i32 - k.left as i32) as f32, (k.down as i32 - k.up as i32) as f32);
                    if (dx != 0.0 || dy != 0.0) && me.modded {
                        let to = (me.pick as i32 + dy as i32 + dx as i32 * LIST_ROWS as i32).clamp(0, n_mods as i32 - 1) as usize;
                        if to != me.pick {
                            me.pick = to;
                            me.costume = 0;
                            out.push(Sound(0));
                        }
                    } else if dx != 0.0 || dy != 0.0 {
                        let to = grid_step(me.cursor, dx, dy);
                        if to != me.cursor {
                            me.cursor = to;
                            me.costume = 0;
                            out.push(Sound(0));
                        }
                    }
                    if k.prev || k.next {
                        // M1n: past a costume that failed to load
                        for _ in 0..costumes {
                            me.costume = (me.costume + if k.next { 1 } else { costumes - 1 }) % costumes;
                            if !broken[me.costume] {
                                break;
                            }
                        }
                        out.push(Sound(0));
                    }
                }
                if k.ok && !self.players[p].ready {
                    if self.blocked(p) {
                        out.push(Sound(1));
                    } else {
                        self.players[p].ready = true;
                        out.push(Sound(2));
                        if self.online() && (0..self.players()).all(|q| self.seats[q].is_none() || self.players[q].ready) {
                            self.online_ready(&mut out);
                        } else if (0..self.players()).all(|q| self.players[q].ready) {
                            self.go(Screen::Confirm, &mut out);
                        }
                    }
                } else if k.back {
                    if self.players[p].ready {
                        self.unready(p);
                        out.push(Sound(1));
                    } else if self.seats[p].is_none() {
                        // player 1 driving a computer player: back to the last one readied
                        if let Some(q) = (0..self.players()).rev().find(|&q| self.players[q].ready) {
                            self.unready(q);
                        }
                        out.push(Sound(1));
                    } else if p == 0 {
                        self.screen = Screen::Assign;
                        out.push(Sound(1));
                    }
                }
            }
            Screen::Confirm if host == Some(dev) => {
                rows(CONFIRM_ROWS, &mut self.sel, &mut out);
                let d = k.right as i32 - k.left as i32 + (k.ok && self.sel > 0) as i32;
                if d != 0 {
                    let wrap = |v: usize, n: usize| (v as i32 + d).rem_euclid(n as i32) as usize;
                    match self.sel {
                        1 => self.court = wrap(self.court, COURTS + 1),
                        2 => self.sets = wrap(self.sets, SETS.len()),
                        3 => self.games = wrap(self.games as usize - 1, 6) as i32 + 1,
                        4 => self.umpire = wrap(self.umpire as usize, UMPIRES + 1) as u8,
                        _ => {}
                    }
                    if self.sel > 0 {
                        out.push(Sound(0));
                    }
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
                } else if k.back {
                    for p in 0..4 {
                        self.unready(p);
                    }
                    self.screen = Screen::Chars;
                    out.push(Sound(1));
                }
            }
            Screen::Online | Screen::Lobbies | Screen::Lobby | Screen::Code => self.online_step(k, &mut out),
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
            None => None,
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

/// The grid cell from `from` in direction (dx, dy): the nearest in that half-plane, sideways distance counted double.
fn grid_step(from: usize, dx: f32, dy: f32) -> usize {
    let (_, x0, y0) = GRID[from];
    (0..CHARS)
        .filter_map(|i| {
            let (along, side) = ((GRID[i].1 - x0) * dx + (GRID[i].2 - y0) * dy, ((GRID[i].1 - x0) * dy - (GRID[i].2 - y0) * dx).abs());
            (along > 1.0).then_some((i, along + 2.0 * side))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(from, |(i, _)| i)
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
    /// A portrait sheet: costume 0 (characters 0–7, 8–13), costume 9 (same), costumes 1..8.
    Shots(usize),
    /// Player `p`'s 3D preview (`previews.rs`), in place of the portrait.
    Preview(usize),
    /// The lobby code's QR (`online::QrImage`).
    Qr,
}

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
const OPTION: Tex = Tex::Sheet(9);
const CS0: Tex = Tex::Sheet(10);
const FACES: Tex = Tex::Sheet(11);
const NAMES: Tex = Tex::Sheet(12);
const CARD: Tex = Tex::Sheet(13);
/// The △ attribute card: title box, play-style titles, stat labels and the A–F letters.
const ATTRS: Tex = Tex::Sheet(14);
const CONFIRM: Tex = Tex::Sheet(15);
/// After the portrait sheets: `info.tm2` (button icons), `Confirmation_04` (the rank labels and their pill),
/// `KeyAssign_CharacterSelect2` (the select's key hint words), then `Confirmation_00`/`01`/`02`: round umpire faces and
/// balls; court names, surfaces and meter icons; court tiles (the "?" one).
const EXTRA: usize = SHEETS.len() + 2 + UMPIRES + 4 + CHARS;
const INFO: Tex = Tex::Sheet(EXTRA);
const RANKS: Tex = Tex::Sheet(EXTRA + 1);
const KEYS: Tex = Tex::Sheet(EXTRA + 2);
const C00: Tex = Tex::Sheet(EXTRA + 3);
const C01: Tex = Tex::Sheet(EXTRA + 4);
const C02: Tex = Tex::Sheet(EXTRA + 5);
const C04: Tex = RANKS;

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
const POOL: usize = 700;

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
    commands.insert_resource(Art(art, bank, Text { msg, glyphs, grades, style, hands: hands(&csv), ranks, mods: mod_text }));

    let fresh = kept.is_none();
    // first run (no settings.txt) on a Steam Deck: its graphics preset
    let first = || Settings(SETTING_DEFAULTS, if std::env::var("SteamDeck").is_ok_and(|v| v == "1") { Graphics::STEAM_DECK } else { Graphics::default() });
    let saved = crate::store::read_to_string(Settings::path(&args.iso)).map(|t| Settings::parse(&t));
    let mut menu = kept.map(|k| k.0.clone()).unwrap_or_else(|| Menu { settings: saved.as_ref().map_or_else(|_| first(), |s| *s), mods, ..default() });
    // back from a match: its pause menu may have changed them (B126)
    if let Ok(s) = saved {
        menu.settings = s;
    }
    commands.insert_resource(menu.settings.1);
    let screen = std::env::var("HST_MENU").unwrap_or_default();
    menu.screen = match screen.as_str() {
        "settings" => Screen::Settings,
        "graphics" => Screen::Graphics,
        "controls" => Screen::Controls,
        "mode" => Screen::Mode,
        "assign" => Screen::Assign,
        "chars" | "mods" | "attrs" | "coms" => Screen::Chars,
        "confirm" => Screen::Confirm,
        "online" => Screen::Online,
        "lobbies" => Screen::Lobbies,
        "lobby" | "lobby-ask" => Screen::Lobby,
        "code" => Screen::Code,
        _ => Screen::Main,
    };
    if fresh && menu.screen == Screen::Main {
        online::linked(&mut menu);
    }
    if !fresh {
        (menu.screen, menu.sel) = (Screen::Main, 0);
    } else if menu.screen != Screen::Main {
        menu.seats[0] = Some(Dev::Keys);
        menu.doubles = true;
        menu.players[0].cursor = 6;
        menu.custom = screen == "mods" && !menu.mods.is_empty();
        menu.players[0].card = screen == "attrs";
        menu.players[0].ready = screen == "coms";
        menu.players.iter_mut().for_each(|p| p.modded = menu.custom);
        if matches!(menu.screen, Screen::Lobby | Screen::Lobbies) {
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
}

const WHITE: [f32; 3] = [128.0; 3];
const DIMMED: [f32; 3] = [80.0; 3];

impl Draw<'_> {
    fn q(&mut self, tex: Tex, src: [f32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: f32) {
        self.out.push(Quad { tex, src, dst, rgb, alpha, tile: false, anchor: self.anchor, angle: 0.0 });
    }

    /// The screen's wall (tiled) and title bar.
    fn frame(&mut self, title: usize, wall: usize) {
        self.out.push(Quad { tex: Tex::Sheet(wall), src: [0.0, 0.0, 32.0, 32.0], dst: [0.0, 0.0, 640.0, 448.0], rgb: WHITE, alpha: 128.0, tile: true, anchor: Anchor::Span, angle: 0.0 });
        self.anchor = Anchor::Span;
        self.q(Tex::Sheet(title), [0.0, 0.0, 512.0, 64.0], [0.0, 0.0, 512.0, 64.0], WHITE, 128.0);
        self.q(Tex::Sheet(title), [480.0, 0.0, 32.0, 64.0], [512.0, 0.0, 128.0, 64.0], WHITE, 128.0);
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

    /// A pill (`Option_00`'s white one, three-sliced) tinted.
    fn pill(&mut self, x: f32, y: f32, w: f32, rgb: [f32; 3]) {
        self.q(OPTION, [66.0, 2.0, 19.0, 38.0], [x, y, 19.0, 38.0], rgb, 128.0);
        self.q(OPTION, [85.0, 2.0, 24.0, 38.0], [x + 19.0, y, w - 38.0, 38.0], rgb, 128.0);
        self.q(OPTION, [109.0, 2.0, 19.0, 38.0], [x + w - 19.0, y, 19.0, 38.0], rgb, 128.0);
    }

    /// A column of rows, the chosen one bright with the hand beside it.
    fn rows(&mut self, labels: &[String], sel: usize, x: f32, y: f32, w: f32, hand_x: f32) {
        self.rows_at(labels, sel, x, y, w, hand_x, 44.0);
    }

    /// `rows` `pitch` px apart.
    #[allow(clippy::too_many_arguments)]
    fn rows_at(&mut self, labels: &[String], sel: usize, x: f32, y: f32, w: f32, hand_x: f32, pitch: f32) {
        for (i, l) in labels.iter().enumerate() {
            let yy = y + pitch * i as f32;
            let on = i == sel;
            self.pill(x, yy, w, if on { [128.0, 110.0, 40.0] } else { [70.0, 70.0, 80.0] });
            self.centred(l, x + w / 2.0, yy + 7.0, 24.0, if on { WHITE } else { [100.0; 3] });
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
    }
}

/// The description bar's first glyph x `f` frames after its text (`n` glyphs) changed, as the MENU overlay moves it:
/// in from x 656 at 32 px a frame to x 16; a text wider than 616 px (`n` × 11) then scrolls left 2 px a frame from
/// frame 160, round a loop of `(n × 11 + 656) / 2` frames that re-enters at x 656.
#[cfg(test)]
fn ticker_x(f: u32, n: usize) -> f32 {
    ticker_x_in(f, n, 656)
}

/// `ticker_x` on a bar whose right edge is `right - 16` (656 on the 4:3 screen, further on a wide one): the text
/// enters and re-enters at `right`, the same speeds, the same 616 px test.
fn ticker_x_in(f: u32, n: usize, right: i32) -> f32 {
    let w = 11 * n as i32;
    let x = if w <= 616 { 16 } else { right - 2 * ((f as i32 + (right - 16) / 2 - 160) % ((w + right) >> 1)) };
    if f < 160 { (right - 32 * f as i32).max(16) as f32 } else { x as f32 }
}

/// `ticker_x_in` at a fractional frame count: between the original's 60 Hz steps, a glide from one to the next (not
/// across a wrap, which jumps).
fn ticker_glide(f: f32, n: usize, right: i32) -> f32 {
    let (a, b) = (ticker_x_in(f as u32, n, right), ticker_x_in(f as u32 + 1, n, right));
    if (b - a).abs() > 32.0 { a } else { a + (b - a) * f.fract() }
}

/// The description bar's clip rect; its child is the text.
#[derive(Component)]
struct TickerClip;

/// Frames (60 Hz, fractional) since the description bar's text changed, and that text.
#[derive(Resource, Default)]
struct Ticker(String, f32);

/// The description bar's text, at the window's resolution in Fira Mono (Bevy's built-in font) sized so its advance
/// is the original's 11 px pitch on the 640-wide screen, in its tint (223), centred on the bar (y 404..436). Its bar
/// spans the window (`Anchor::Span`), so a wide window's text enters and wraps at the window's right edge.
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
    (cnode.top, cnode.height) = (Val::Percent(404.0 / 4.48), Val::Percent(32.0 / 4.48));
    // one 640-space unit in logical px; the font's advance is 0.6 em
    let unit = share * win.width() / 640.0;
    let em = 11.0 / 0.6 * unit;
    font.font_size = bevy::text::FontSize::Px(em);
    let right = (width / 100.0 * win.width() / unit).round() as i32 + 16;
    node.left = Val::Px(ticker_glide(ticker.1, ticker.0.chars().count(), right) * unit);
    node.top = Val::Px((32.0 / 448.0 * win.height() - 1.2 * em) / 2.0);
    if t.0 != ticker.0 {
        t.0.clone_from(&ticker.0);
    }
}

fn layout(m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity], hand_x: f32) -> (Vec<Quad>, String) {
    let mut d = Draw { out: Vec::new(), text, said: String::new(), anchor: Anchor::Centre };
    match m.screen {
        Screen::Main => {
            d.frame(0, 5);
            let labels: Vec<String> = MAIN.iter().map(|s| s.to_string()).collect();
            d.rows(&labels, m.sel, 200.0, 120.0, 240.0, hand_x);
            let info = match m.sel {
                0 => text.msg(188).to_string(),
                1 => "Play over the internet (coming later).".into(),
                2 => text.msg(190).to_string(),
                _ => "Close the game.".into(),
            };
            d.info(&info);
        }
        Screen::Settings => {
            d.frame(1, 6);
            let mut labels: Vec<String> = SETTING_LABELS.iter().map(|s| s.to_string()).collect();
            labels.extend(["Graphics".into(), "Controls".into(), "Back".into()]);
            d.rows(&labels, m.sel, 120.0, 90.0, 300.0, hand_x);
            for (i, on) in m.settings.0.iter().enumerate() {
                let src = if *on { [429.0, 5.0, 39.0, 23.0] } else { [423.0, 37.0, 51.0, 23.0] };
                d.q(OPTION, src, [440.0, 90.0 + 44.0 * i as f32 + 8.0, src[2], src[3]], WHITE, 128.0);
            }
            d.info(&match m.sel {
                0 => "Fill wide windows with the court; off shows the original 4:3 picture.".to_string(),
                1 => "Use the upscaled textures in mods/texture-replacements.".into(),
                2 => "Play music in the menus.".into(),
                3 => "Play each court's music during a match.".into(),
                4 => "Resolution, shadows, frame limit and vsync.".into(),
                5 => "Change the keys and buttons (controls.txt).".into(),
                _ => text.msg(190).to_string(),
            });
        }
        Screen::Graphics => {
            d.frame(1, 6);
            let mut labels: Vec<String> = GRAPHICS_LABELS.iter().map(|s| s.to_string()).collect();
            labels.push("Back".into());
            // eight rows: closer than the other screens' to clear the description bar (y 404)
            d.rows_at(&labels, m.sel, 120.0, 66.0, 300.0, hand_x, 42.0);
            for i in 0..GRAPHICS_LABELS.len() {
                d.text(&m.settings.graphics_value(i), 440.0, 66.0 + 42.0 * i as f32 + 7.0, 24.0, if i == m.sel { WHITE } else { [40.0; 3] });
            }
            d.info(match m.sel {
                0 => "Default draws at full quality; Steam Deck and Low trade detail for frame rate.",
                1 => "Draw the court at this share of the window's resolution, scaled up to fill it (the HUD stays sharp).",
                2 => "Off, or the shadow map's detail (Low: smaller and fewer cascades).",
                3 => "The most frames drawn a second (the game itself always runs at 60 Hz).",
                4 => "Wait for the display between frames: no tearing, at most its refresh rate.",
                5 => "Draw at the PS2's 640x448, 4:3, with the disc's textures, scaled up unfiltered.",
                6 => "Smooth the court's jagged edges (MSAA): off, 2x or 4x samples per pixel.",
                _ => "Back to the settings.",
            });
        }
        Screen::Controls => {
            d.frame(1, 6);
            let names: Vec<_> = controls::action_names().collect();
            for (i, n) in names.iter().chain(&["back"]).enumerate() {
                let y = 72.0 + 28.0 * i as f32;
                let on = i == m.sel;
                let rgb = if on { WHITE } else { [40.0; 3] };
                d.text(&n.replace('_', " "), 120.0, y, 22.0, rgb);
                if i < names.len() {
                    let s = if on && m.rebinding.is_some() { "press a key or button...".to_string() } else { bind.names(i).join("  ") };
                    d.text(&s, 300.0, y, 22.0, if on && m.rebinding.is_some() { [128.0, 110.0, 40.0] } else { rgb });
                }
            }
            d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [56.0 + hand_x, 68.0 + 28.0 * m.sel as f32, 64.0, 32.0], WHITE, 128.0);
            d.info(if m.rebinding.is_some() { "Press the new key or button (Escape cancels)." } else { "Choose an action to rebind." });
        }
        Screen::Mode => {
            d.frame(2, 5);
            for (i, src) in [[2.0, 2.0, 252.0, 30.0], [2.0, 34.0, 252.0, 30.0]].into_iter().enumerate() {
                let on = (i == 1) == m.doubles;
                d.q(CS0, src, [134.0, 150.0 + 70.0 * i as f32, 372.0, 44.0], if on { WHITE } else { DIMMED }, if on { 128.0 } else { 90.0 });
            }
            d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [76.0 + hand_x, 156.0 + 70.0 * m.doubles as u8 as f32, 64.0, 32.0], WHITE, 128.0);
            d.info(text.msg(192));
        }
        Screen::Assign => {
            d.frame(2, 5);
            for p in 0..m.players() {
                let y = 90.0 + 70.0 * p as f32;
                d.q(CARD, [CARDS[p][0], CARDS[p][1], 140.0, 40.0], [120.0, y, 400.0, 56.0], WHITE, 128.0);
                d.text(&format!("{}P", p + 1), 140.0, y + 10.0, 32.0, WHITE);
                d.text(&dev_name(m.seats[p], pads), 240.0, y + 12.0, 28.0, if m.seats[p].is_some() { WHITE } else { [60.0, 110.0, 40.0] });
            }
            d.info(&format!("{} Others: X or Enter joins, Left/Right changes seat, O leaves.", text.msg(193)));
        }
        Screen::Chars => chars(&mut d, m, text),
        // the match starts from the confirm screen, which stays up under the switch's zoom-out
        Screen::Confirm | Screen::Playing => confirm(&mut d, m, text, hand_x),
        Screen::Online | Screen::Lobbies | Screen::Lobby | Screen::Code => online::layout(&mut d, m, hand_x),
    }
    (d.out, d.said)
}

fn portrait(c: usize, costume: usize) -> (Tex, [f32; 4]) {
    let cell = match costume {
        0 | 9 => c % 8,
        _ => costume - 1,
    };
    let sheet = match costume {
        0 => c / 8,
        9 => 2 + c / 8,
        _ => 4 + c,
    };
    (Tex::Shots(sheet), [(cell % 4) as f32 * 128.0, (cell / 4 % 2) as f32 * 168.0, 128.0, 168.0])
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

fn chars(d: &mut Draw, m: &Menu, text: &Text) {
    d.frame(3, 7);
    hints(d);
    let n = m.players();
    d.q(CS0, if m.doubles { [2.0, 34.0, 252.0, 30.0] } else { [2.0, 2.0, 252.0, 30.0] }, [194.0, 56.0, 252.0, 30.0], WHITE, 128.0);
    let picking = |p: usize| m.seats[p].is_some() || (0..n).filter(|&q| m.seats[q].is_none()).all(|q| q >= p || m.players[q].ready) && (0..n).all(|q| m.seats[q].is_none() || m.players[q].ready);
    let lead = (0..n).find(|&p| !m.players[p].ready).unwrap_or(0);
    if m.custom {
        custom_list(d, m, (0..n).filter(|&p| picking(p) && !m.players[p].ready), m.players[lead].pick);
    } else {
        groups(d);
        // the faces, darkened where every costume of it is ... (all open in the remaster)
        for &(c, x, y) in &GRID {
            d.q(Tex::Solid, [0.0; 4], [x - 32.0, y - 32.0, 64.0, 64.0], [110.0, 100.0, 90.0], 128.0);
            d.q(FACES, [(c % 8) as f32 * 64.0, (c / 8) as f32 * 64.0, 64.0, 64.0], [x - 30.0, y - 30.0, 60.0, 60.0], WHITE, 128.0);
        }
        let hint = if m.mods.is_empty() { String::new() } else { format!("Tab / Square: {} custom characters   ", m.mods.len()) };
        d.centred(&format!("{hint}Select / C: left or right hand"), 320.0, 159.0, 12.0, WHITE);
    }
    let panels: [[f32; 2]; 4] = if m.doubles { [[16.0, 56.0], [16.0, 240.0], [480.0, 56.0], [480.0, 240.0]] } else { [[16.0, 56.0], [480.0, 56.0], [0.0; 2], [0.0; 2]] };
    for p in 0..n {
        let pl = m.players[p];
        let c = pl.char();
        let [x, y] = panels[p];
        let [u, v] = CARDS[p];
        let human = m.seats[p].is_some();
        let picking = picking(p);
        d.anchor = Anchor::Sides;
        let modded = match m.who(p) {
            Who::Mod(i) => Some(i),
            Who::Disc(_) => None,
        };
        // a card nobody is picking on yet is dark
        d.q(CARD, [u, v, 140.0, 164.0], [x, y, 144.0, 168.0], if picking || pl.ready { WHITE } else { [55.0; 3] }, 128.0);
        if picking || pl.ready {
            if pl.card {
                let (style, grades) = modded.map_or((text.style[c], text.grades[c]), |i| text.mods[i]);
                attribute_card(d, p, style, &grades, x, y);
            } else {
                d.q(Tex::Preview(p), [0.0; 4], [x + 8.0, y + 4.0, 128.0, 160.0], WHITE, 128.0);
                let style = modded.map_or(text.style[c], |i| text.mods[i].0);
                let style = [[320.0, 112.0, 144.0, 24.0], [324.0, 136.0, 136.0, 24.0], [346.0, 160.0, 94.0, 24.0], [346.0, 184.0, 94.0, 24.0]][style];
                // a computer's label sits higher, over its rank pill
                let sy = if human { y + 118.0 } else { y + 94.0 };
                d.q(CS0, style, [x + 72.0 - style[2] / 2.0 * 0.9, sy, style[2] * 0.9, 22.0], WHITE, 128.0);
                if !human {
                    let (who, costume) = m.pick(p);
                    let c = match who {
                        Who::Mod(i) => m.mods[i].donor,
                        Who::Disc(c) => c,
                    };
                    rank_pill(d, com_rank(&text.ranks, c, costume, m.doubles), COLOURS[p], x + 17.0, y + 114.0);
                } else {
                    let src = if m.hand(text, p) < 0.0 { [60.0, 184.0, 76.0, 22.0] } else { [60.0, 160.0, 76.0, 22.0] };
                    d.q(CS0, src, [x + 70.0, y + 5.0, 66.0, 19.0], if pl.swap { [128.0, 110.0, 30.0] } else { WHITE }, 128.0);
                }
                match modded {
                    Some(i) => {
                        d.fitted(&m.mods[i].name, x + 72.0, y + 142.0, 20.0, 128.0);
                        // M1m: the game it comes from, under the hand badge beside the player tag
                        d.fitted(&m.mods[i].source, x + 92.0, y + 27.0, 11.0, 88.0);
                    }
                    None => d.q(NAMES, [23.0, 24.0 * c as f32, 105.0, 24.0], [x + 20.0, y + 140.0, 105.0, 24.0], WHITE, 128.0),
                }
            }
            if pl.ready {
                d.q(CS0, [2.0, 208.0, 108.0, 24.0], [x + 18.0, y + 96.0, 108.0, 24.0], WHITE, 128.0);
            } else if m.blocked(p) {
                d.q(CS0, [2.0, 232.0, 110.0, 24.0], [x + 17.0, y + 96.0, 110.0, 24.0], WHITE, 128.0);
            }
            if !pl.ready && !pl.modded {
                d.anchor = Anchor::Centre;
                let [fx, fy] = [GRID[pl.cursor].1, GRID[pl.cursor].2];
                d.q(Tex::Hand, [0.0, 0.0, 64.0, 32.0], [fx - 40.0 + 10.0 * (p % 2) as f32, fy - 8.0 + 12.0 * (p / 2) as f32, 48.0, 24.0], COLOURS[p], 128.0);
                d.text(&format!("{}P", p + 1), fx - 4.0 + 14.0 * (p % 2) as f32, fy - 32.0 + 14.0 * (p / 2) as f32, 16.0, COLOURS[p]);
                d.anchor = Anchor::Sides;
            }
            // the card's title box takes the top: no player/costume tag over it
            if !(pl.card && modded.is_none()) {
                d.text(&format!("{}P", p + 1), x + 6.0, y + 4.0, 22.0, WHITE);
                // the costume number on a shirt in the player's colour, under the COM tag on a computer's card
                let sy = if human { y + 26.0 } else { y + 50.0 };
                d.q(CS0, [471.0, 66.0, 38.0, 28.0], [x + 6.0, sy, 38.0, 28.0], COLOURS[p], 128.0);
                d.centred(&format!("{}", pl.costume + 1), x + 25.0, sy + 6.0, 18.0, WHITE);
            }
        } else {
            d.text(&format!("{}P", p + 1), x + 6.0, y + 4.0, 22.0, WHITE);
        }
        if !human {
            d.text("COM", x + 8.0, y + 30.0, 20.0, [60.0, 120.0, 30.0]);
        }
        d.anchor = Anchor::Centre;
    }
    match m.who(lead) {
        Who::Mod(i) => {
            let md = &m.mods[i];
            let s = if md.costumes.len() == 1 { "" } else { "s" };
            let from = if md.source.is_empty() { String::new() } else { format!(" from {}", md.source) };
            d.info(&format!("{}: custom character{from}, {} costume{s}. Tab / Square: the disc characters.", md.name, md.costumes.len()));
        }
        Who::Disc(c) if !m.mods.is_empty() => d.info(&format!("{} (Tab / Square: custom characters)", text.msg(308 + c))),
        Who::Disc(c) => d.info(text.msg(308 + c)),
    }
}

/// The header's key hints as the original's: △ the attribute card, L1 / R1 the costume (`info.tm2`'s buttons,
/// `KeyAssign_CharacterSelect2`'s words; places off a PCSX2 shot). The original swaps one shoulder button for the
/// other in turn; both are shown here.
fn hints(d: &mut Draw) {
    d.anchor = Anchor::Sides;
    d.q(INFO, [228.0, 4.0, 24.0, 24.0], [314.0, 26.0, 22.0, 22.0], WHITE, 128.0);
    d.q(KEYS, [0.0, 4.0, 104.0, 16.0], [338.0, 29.0, 98.0, 15.0], WHITE, 128.0);
    d.q(INFO, [160.0, 9.0, 32.0, 19.0], [440.0, 27.0, 30.0, 18.0], WHITE, 128.0);
    d.q(INFO, [160.0, 73.0, 32.0, 19.0], [472.0, 27.0, 30.0, 18.0], WHITE, 128.0);
    d.q(KEYS, [0.0, 27.0, 132.0, 18.0], [506.0, 28.0, 128.0, 17.0], WHITE, 128.0);
    d.anchor = Anchor::Centre;
}

/// `src` drawn over `dst` with its `c`-px corners unstretched.
fn nine(d: &mut Draw, tex: Tex, [u, v, w, h]: [f32; 4], c: f32, [x, y, dw, dh]: [f32; 4]) {
    let cols = [(u, c, x, c), (u + c, w - 2.0 * c, x + c, dw - 2.0 * c), (u + w - c, c, x + dw - c, c)];
    let rows = [(v, c, y, c), (v + c, h - 2.0 * c, y + c, dh - 2.0 * c), (v + h - c, c, y + dh - c, c)];
    for (su, sw, dx, ww) in cols {
        for (sv, sh, dy, hh) in rows {
            d.q(tex, [su, sv, sw, sh], [dx, dy, ww, hh], WHITE, 128.0);
        }
    }
}

/// The grid's BEG / INT / EXP groups as the original frames them: an outlined box round each, a coloured bar down
/// its left with the letters (`CharacterSelect2_00`; boxes and colours off a PCSX2 shot).
fn groups(d: &mut Draw) {
    const GROUPS: [(f32, f32, [f32; 3], [f32; 4]); 3] = [
        (92.0, 72.0, [40.0, 100.0, 50.0], [470.0, 401.0, 13.0, 38.0]),
        (171.0, 146.0, [40.0, 80.0, 128.0], [494.0, 401.0, 13.0, 38.0]),
        (324.0, 74.0, [120.0, 45.0, 45.0], [470.0, 440.0, 13.0, 39.0]),
    ];
    for (y, h, rgb, letters) in GROUPS {
        nine(d, CS0, [154.0, 274.0, 60.0, 60.0], 12.0, [166.0, y, 308.0, h]);
        // the bar: the tall pill filled in the group's colour under its outline, the round ends kept
        for (u, tint) in [(392.0, rgb), (424.0, WHITE)] {
            d.q(CS0, [u, 400.0, 28.0, 14.0], [167.0, y + 1.0, 28.0, 14.0], tint, 128.0);
            d.q(CS0, [u, 414.0, 28.0, 48.0], [167.0, y + 15.0, 28.0, h - 30.0], tint, 128.0);
            d.q(CS0, [u, 462.0, 28.0, 14.0], [167.0, y + h - 15.0, 28.0, 14.0], tint, 128.0);
        }
        d.q(CS0, letters, [174.5, y + (h - letters[3]) / 2.0, letters[2], letters[3]], WHITE, 128.0);
    }
}

/// A computer's "Tennis Lv N" pill (`Confirmation_04`: rank `r`'s label, 9 to a column, on its pill in the card's
/// colour).
fn rank_pill(d: &mut Draw, r: usize, rgb: [f32; 3], x: f32, y: f32) {
    d.q(RANKS, [0.0, 224.0, 112.0, 24.0], [x, y, 110.0, 22.0], rgb, 128.0);
    d.q(RANKS, [112.0, 224.0, 112.0, 24.0], [x, y, 110.0, 22.0], rgb.map(|c| c * 0.45), 128.0);
    d.q(RANKS, [96.0 * (r / 9) as f32, 24.0 * (r % 9) as f32, 96.0, 24.0], [x + 7.0, y - 1.0, 96.0, 24.0], WHITE, 128.0);
}

/// The custom page: the roster's names, the page holding `at`, a hand per player picking on it.
fn custom_list(d: &mut Draw, m: &Menu, players: impl Iterator<Item = usize>, at: usize) {
    let per = 2 * LIST_ROWS;
    let page = at / per;
    let cell = |i: usize| {
        let k = i % per;
        (LIST[k / LIST_ROWS], LIST[2] + 22.0 * (k % LIST_ROWS) as f32)
    };
    for i in page * per..(page * per + per).min(m.mods.len()) {
        let (x, y) = cell(i);
        d.q(Tex::Solid, [0.0; 4], [x, y, 140.0, 20.0], [40.0, 40.0, 50.0], 100.0);
        d.fitted(&m.mods[i].name, x + 70.0, y + 2.0, 16.0, 132.0);
    }
    for p in players.filter(|&p| m.players[p].modded && m.players[p].pick / per == page) {
        let (x, y) = cell(m.players[p].pick);
        d.q(Tex::Hand, [0.0, 0.0, 64.0, 32.0], [x - 34.0 + 6.0 * (p % 2) as f32, y - 2.0 + 4.0 * (p / 2) as f32, 40.0, 20.0], COLOURS[p], 128.0);
    }
    let pages = m.mods.len().div_ceil(per);
    d.centred(&format!("Custom characters  {}/{pages}", page + 1), 320.0, LIST[2] + 22.0 * LIST_ROWS as f32 + 8.0, 18.0, WHITE);
}

fn confirm(d: &mut Draw, m: &Menu, text: &Text, hand_x: f32) {
    d.frame(4, 8);
    let order: &[usize] = if m.doubles { &[0, 1, 3, 2] } else { &[0, 1] };
    let w = 600.0 / order.len() as f32;
    for (k, &p) in order.iter().enumerate() {
        let x = 20.0 + w * k as f32 + (w - 112.0) / 2.0;
        match m.pick(p) {
            (Who::Mod(i), costume) => {
                // ponytail: no 3D preview on this screen; the name and costume on a dark card
                d.q(Tex::Solid, [0.0; 4], [x, 64.0, 112.0, 147.0], [30.0, 30.0, 40.0], 110.0);
                d.fitted(&m.mods[i].name, x + 56.0, 120.0, 20.0, 104.0);
                d.centred(&format!("{}", costume + 1), x + 56.0, 146.0, 18.0, WHITE);
            }
            (Who::Disc(c), costume) => {
                let (sheet, src) = portrait(c, costume);
                d.q(sheet, src, [x, 64.0, 112.0, 147.0], WHITE, 128.0);
            }
        }
        // play style over the card's foot (`Confirmation_03`'s titles)
        let (style, ai) = match m.pick(p) {
            (Who::Mod(i), costume) => (text.mods[i].0, (m.mods[i].ai_row, costume)),
            (Who::Disc(c), costume) => (text.style[c], (c, costume)),
        };
        d.q(CONFIRM, [412.0, 168.0 + 24.0 * style as f32, 90.0, 24.0], [x + 11.0, 186.0, 90.0, 24.0], WHITE, 128.0);
        if m.seats[p].is_none() {
            d.text(&format!("{}P", p + 1), x + 10.0, 214.0, 24.0, COLOURS[p]);
            d.text("COM", x + 56.0, 218.0, 18.0, [60.0, 120.0, 30.0]);
            // the computer's Tennis Lv: its AIParam row's rank (`Confirmation_04`'s titles, nine to a column)
            let row = hst_sim::ai::Choice::new(hst_sim::ai::menu_row(ai.0 as u8, ai.1 as u8) as u32, ai.0 as u8, m.doubles).row;
            let r = text.ranks.get(row).copied().unwrap_or(0) as f32;
            d.q(C04, [0.0, 224.0, 110.0, 24.0], [x + 1.0, 238.0, 110.0, 20.0], COLOURS[p], 128.0);
            d.q(C04, [96.0 * (r / 9.0).floor(), 24.0 * (r % 9.0), 96.0, 24.0], [x + 10.0, 237.0, 92.0, 22.0], WHITE, 128.0);
        } else {
            d.centred(&format!("{}P", p + 1), x + 56.0, 214.0, 24.0, COLOURS[p]);
        }
    }
    flame::draw(d, m);
    d.q(CONFIRM, [448.0, 264.0, 64.0, 48.0], [288.0, 110.0, 64.0, 48.0], WHITE, 128.0);
    // "Umpire", the umpire's round face (the "?" tile for a random one) and name at the top right
    d.anchor = Anchor::Sides;
    let u = m.umpire as usize;
    d.q(CONFIRM, [286.0, 194.0, 78.0, 28.0], [404.0, 12.0, 78.0, 28.0], WHITE, 128.0);
    let (face, name) = match u {
        UMPIRES => ([96.0, 360.0, 92.0, 68.0], "Random".to_string()),
        _ => ([63.0 + 48.0 * u as f32, 272.0, 46.0, 44.0], text.msg(669 + 2 * u).trim_start_matches("Umpire ").to_string()),
    };
    d.q(if u == UMPIRES { C02 } else { C00 }, face, [486.0, 4.0, 46.0, 44.0], WHITE, 128.0);
    d.text(&name, 538.0, 16.0, 22.0, WHITE);
    d.anchor = Anchor::Centre;
    // court name, its surface and the surface's ball meters (bounce, speed); none for a random court
    let gold = [128.0, 110.0, 40.0];
    if m.court == COURTS {
        d.text("Random Court", 30.0, 258.0, 24.0, gold);
    } else {
        let court = format!("{} {}", text.msg(276 + 2 * m.court), text.msg(277 + 2 * m.court));
        d.text(&court, 30.0, 258.0, 24.0, gold);
        let s = SURFACE[m.court];
        let tint = [[50.0, 128.0, 30.0], [128.0, 80.0, 30.0], [40.0, 100.0, 128.0]][s];
        d.q(C01, [68.0, 286.0 + 24.0 * s as f32, 124.0, 24.0], [400.0, 256.0, 124.0, 24.0], tint, 128.0);
        for (g, (icon, n)) in [([2.0, 490.0, 45.0, 22.0], METERS[s].0), ([49.0, 492.0, 46.0, 17.0], METERS[s].1)].into_iter().enumerate() {
            let x = 344.0 + 140.0 * g as f32;
            d.q(C01, icon, [x, 296.0, icon[2] * 0.8, icon[3] * 0.8], WHITE, 128.0);
            for b in 0..4 {
                d.q(C00, [72.0, if b < n { 192.0 } else { 216.0 }, 24.0, 24.0], [x + 40.0 + 22.0 * b as f32, 292.0, 22.0, 22.0], WHITE, 128.0);
            }
        }
    }
    // the rows: Start, Court, Sets, Games, Umpire
    let rows: [(&str, String); CONFIRM_ROWS] = [
        ("", String::new()),
        ("Select Court", String::new()),
        ("Sets", SETS[m.sets].to_string()),
        ("Games", m.games.to_string()),
        ("Select Umpire", String::new()),
    ];
    // the original's 3×2 grid under Start (its Set Handicap and Offbeat Rules aren't here: B40g)
    let pos = [[60.0, 286.0, 260.0], [40.0, 334.0, 180.0], [230.0, 334.0, 180.0], [420.0, 334.0, 180.0], [40.0, 372.0, 180.0]];
    for (i, ((label, value), [x, y, w])) in rows.iter().zip(pos).enumerate() {
        let on = i == m.sel;
        if i == 0 {
            d.pill(x, y, w, if on { [128.0, 40.0, 60.0] } else { [80.0, 40.0, 50.0] });
            d.q(CONFIRM, [232.0, 166.0, 160.0, 28.0], [x + w / 2.0 - 80.0, y + 5.0, 160.0, 28.0], WHITE, 128.0);
            continue;
        }
        d.pill(x, y - 4.0, w, if on { [128.0, 110.0, 40.0] } else { [90.0; 3] });
        d.text(label, x + 20.0, y + 4.0, 20.0, if on { WHITE } else { [60.0; 3] });
        let vw = d.width(value, 20.0);
        d.text(value, x + w - 18.0 - vw, y + 4.0, 20.0, if on { WHITE } else { [60.0; 3] });
    }
    let [hx, hy, _] = pos[m.sel];
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [hx - 50.0 + hand_x, hy + 2.0, 64.0, 32.0], WHITE, 128.0);
    let info = match m.sel {
        0 => text.msg(323).to_string(),
        1 if m.court == COURTS => "A court picked at random when the match starts.".to_string(),
        1 => format!("{} {}", text.msg(581 + 2 * m.court), text.msg(582 + 2 * m.court)),
        2 => text.msg(324).to_string(),
        3 => text.msg(325).to_string(),
        _ => text.msg(328).to_string(),
    };
    d.info(&info);
}

// ---------------------------------------------------------------- systems

/// The menu as a match left it (picks, seats, settings), back on its main screen.
#[derive(Resource)]
struct Kept(Menu);

#[allow(clippy::too_many_arguments)]
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
    (net, mut clipboard, mut click): (Option<Res<crate::net::Net>>, ResMut<bevy::clipboard::Clipboard>, Option<ResMut<pause::Click>>),
) {
    let play = |key: u8| {
        if let (Some(s), Some(Art(_, Some(b), _))) = (&sound, art.as_deref()) {
            s.play_centre(b, sound::Play { slot: 9, program: 0, key, volume: 0x80, speed: 1.0 });
        }
    };
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g)| controls::readable(g)).map(|(e, _)| e).collect();
    pads.sort();
    // the controls screen waiting for a key or button
    if let Some(i) = menu.rebinding {
        if keys.just_pressed(KeyCode::Escape) {
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
    let mut devices = vec![(Dev::Keys, None)];
    devices.extend(pads.iter().map(|&e| (Dev::Pad(e), Some(e))));
    for (dev, id) in devices {
        let held: [bool; 4];
        let mut k = Press::default();
        match id {
            None => {
                let any = |a: Action, extra: KeyCode| bind.key_pressed(&keys, a) || keys.pressed(extra);
                held = [any(Action::Up, KeyCode::ArrowUp), any(Action::Down, KeyCode::ArrowDown), any(Action::Left, KeyCode::ArrowLeft), any(Action::Right, KeyCode::ArrowRight)];
                k.ok = bind.key_just_pressed(&keys, Action::Normal) || keys.any_just_pressed([KeyCode::Enter, KeyCode::Space]);
                k.back = bind.key_just_pressed(&keys, Action::Cut) || keys.any_just_pressed([KeyCode::Escape, KeyCode::Backspace]);
                k.card = bind.key_just_pressed(&keys, Action::Lob);
                k.prev = keys.any_just_pressed([KeyCode::KeyQ, KeyCode::PageUp]);
                k.next = keys.any_just_pressed([KeyCode::KeyE, KeyCode::PageDown]);
                k.page = keys.just_pressed(KeyCode::Tab);
                k.hand = bind.key_just_pressed(&keys, Action::Camera);
                // a click or tap (`pause::pointer`) comes in as the keyboard's
                if let Some(c) = click.as_mut().and_then(|c| c.0.take()) {
                    (k.ok, k.back) = (k.ok || c.ok, k.back || c.back);
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
            }
        }
        let mut dirs = [false; 4];
        for (i, h) in held.into_iter().enumerate() {
            dirs[i] = repeat.entry((id, i)).or_default().step(h, dt);
        }
        [k.up, k.down, k.left, k.right] = dirs;
        if k == Press::default() {
            continue;
        }
        for e in menu.step(dev, k) {
            match e {
                Effect::Sound(s) => play(s),
                Effect::SaveSettings => {
                    // applied live: the menu draws with them too
                    gfx.set_if_neq(menu.settings.1);
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
) {
    let Some(art) = art else { return };
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g)| controls::readable(g)).map(|(e, _)| e).collect();
    pads.sort();
    // the original's menu hand wags in place (`cursor.rs`), it doesn't swing out and back as the pause menu's
    let hand_x = 0.0;
    // the description ticker counts the original's 60 Hz frames from its text's change
    ticker.1 += time.delta_secs() * 60.0;
    let (quads, said) = layout(&menu, &art.2, &bind, &pads, hand_x);
    if said != ticker.0 {
        *ticker = Ticker(said, 0.0);
    }
    let quads = slide.step(menu.screen, quads, time.delta_secs() * 60.0);
    let scale = window.single().map_or(1.0, |w| w.physical_height() as f32 / 448.0 / w.scale_factor());
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
            Tex::Shots(k) => art.0[3 + SHEETS.len() + 2 + 5 + k].clone(),
            Tex::Preview(_) => continue,
            Tex::Qr => match &qr {
                Some(qr) => qr.0.clone(),
                None => {
                    *vis = Visibility::Hidden;
                    continue;
                }
            },
        };
        let [u, v, w, h] = quad.src;
        img.rect = (quad.tex != Tex::Solid && quad.tex != Tex::Qr && (quad.tex != Tex::Cursor || cursor.is_none())).then(|| Rect::new(u, v, u + w, v + h));
        img.image_mode = if quad.tile { NodeImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: scale } } else { NodeImageMode::Stretch };
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        img.color = Color::srgba(r, g, b, quad.alpha / 128.0);
        let [x, y, w, h] = quad.dst;
        let (mut left, mut width) = super::widescreen::place(x, w, share, quad.anchor);
        let (mut top, mut height) = (y / 4.48, h / 4.48);
        if quad.tex == Tex::Qr
            && let Ok(win) = window.single()
        {
            // a QR stays square (scannable) at any aspect: the largest square in its rect, centred
            let side = (width * win.width()).min(height * win.height());
            (left, top) = (left + (width - side / win.width()) / 2.0, top + (height - side / win.height()) / 2.0);
            (width, height) = (side / win.width(), side / win.height());
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
            Screen::Main | Screen::Settings | Screen::Graphics | Screen::Controls => Some(0),
            Screen::Mode | Screen::Assign | Screen::Chars => Some(1),
            Screen::Confirm => Some(2),
            Screen::Online | Screen::Lobbies | Screen::Lobby | Screen::Code => Some(0),
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
    let want = menu.screen.bgm().filter(|_| menu.settings.0[2]); // Menu Music; Match Music is the match's `--music`
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
        // `HST_STRESS` (previews.rs) drives the select itself: no real key or pad reaches the menus or settings
        .init_resource::<Ticker>()
        .init_resource::<slide::Slide>()
        .add_systems(Update, ((step.run_if(|| std::env::var_os("HST_STRESS").is_none()), flame::tick, draw, ticker_text).chain(), bgm, cursor::tick));
    app.add_systems(Update, (online::poll.before(draw), online::typing.before(step)));
    previews::plugin(app);
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

    #[test]
    fn ticker_as_measured() {
        // Options' sound line (81 glyphs) on the original, frame by frame from its change
        let x = |f| ticker_x(f, 81);
        assert_eq!([x(0), x(1), x(19), x(20), x(160), x(161)], [656.0, 624.0, 48.0, 16.0, 16.0, 14.0]);
        // last shown at -888 (last glyph at -8), then back in at 656; 773 frames a run after that
        assert_eq!([x(160 + 452), x(160 + 453), x(160 + 453 + 772), x(160 + 453 + 773)], [-888.0, 656.0, -888.0, 656.0]);
        // music's line (37 glyphs) fits and holds
        assert_eq!(ticker_x(1000, 37), 16.0);
        // the overlay's test: 616 px (56 glyphs) still holds, 57 scroll
        assert_eq!([ticker_x(10, 56), ticker_x(1000, 56), ticker_x(161, 57)], [336.0, 16.0, 14.0]);
    }

    #[test]
    fn ticker_glides() {
        // whole frames land on the 60 Hz positions; between them, the share of the step
        for f in [0, 1, 19, 20, 160, 161, 612, 613, 1385] {
            assert_eq!(ticker_glide(f as f32, 81, 656), ticker_x(f, 81));
        }
        assert_eq!([ticker_glide(0.25, 81, 656), ticker_glide(19.5, 81, 656), ticker_glide(160.5, 81, 656)], [648.0, 32.0, 15.0]);
        // a wrap jumps (no glide from -888 back across the bar)
        assert_eq!(ticker_glide(612.5, 81, 656), -888.0);
        // a wide bar (right edge at 1120): enters there, scrolls on from 16 at the same 2 px a frame, re-enters there
        let x = |f| ticker_x_in(f, 81, 1136);
        assert_eq!([x(0), x(1), x(35), x(160), x(161)], [1136.0, 1104.0, 16.0, 16.0, 14.0]);
        assert_eq!([x(160 + 452), x(160 + 453)], [-888.0, 1136.0]);
    }

    #[test]
    fn bgm_per_screen() {
        let t = [Screen::Main, Screen::Settings, Screen::Controls, Screen::Mode, Screen::Assign, Screen::Chars, Screen::Confirm, Screen::Playing].map(Screen::bgm);
        assert_eq!(t, [Some(0), Some(0), Some(0), Some(1), Some(1), Some(1), Some(2), None]);
    }

    fn press(f: impl FnOnce(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    #[test]
    fn settings_round_trip() {
        let s = Settings([true, true, false, false], Graphics::LOW);
        assert_eq!(Settings::parse(&s.text()), s);
        let d = Graphics::default();
        assert_eq!(Settings::parse("music = off\nbogus = off\n"), Settings([true, true, true, false], d));
        // a fresh file: menu music on, court music off; an old file's `music` is the match's
        assert_eq!(Settings::parse(""), Settings([true, true, true, false], d));
        assert_eq!(Settings::parse("music = on\n"), Settings([true, true, true, true], d));
        assert_eq!(Settings::parse("menu_music = off\n"), Settings([true, true, false, false], d));
        assert!(!Settings::default().flags().contains(&"--music".to_string()));
        assert!(!Settings([true, true, true, false], d).flags().contains(&"--music".to_string()));
        assert!(Settings([true, true, false, true], d).flags().contains(&"--music".to_string()));
        assert_eq!(Settings::parse("uncapped_fps = off\n").1, Graphics { vsync: true, ..d });
        assert_eq!(s.flags()[..1], ["--fps"]);
        assert_eq!(Settings([false, false, false, true], d).flags()[..3], ["--4x3", "--no-upscale", "--music"]);
        // the preset row steps through the presets; changing any other row makes it Custom
        let mut s = Settings::default();
        s.graphics_step(0, 1);
        assert_eq!(s.1, Graphics::STEAM_DECK);
        s.graphics_step(0, -1);
        assert_eq!(s.1, d);
        s.graphics_step(1, -1);
        assert_eq!((s.1.scale, s.graphics_value(0).as_str()), (85, "Custom"));
        s.graphics_step(5, 1);
        assert!(s.1.original && Settings::parse(&s.text()) == s && s.flags().contains(&"--original".into()));
    }

    #[test]
    fn repeat_after_hold() {
        let mut r = Repeat::default();
        let fired: Vec<bool> = (0..60).map(|_| r.step(true, 1.0 / 60.0)).collect();
        // the press, then at 0.4 s and every 0.1 s after
        assert_eq!(fired.iter().filter(|&&f| f).count(), 1 + 6);
        assert!(fired[0] && !fired[1]);
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
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Chars);
        // both on Ashley costume 1: 1P locks it in, 2P can't ready on it, then can on costume 2
        m.step(Dev::Keys, ok);
        assert!(m.players[0].ready && m.blocked(1));
        m.step(pad, ok);
        assert!(!m.players[1].ready);
        m.step(pad, press(|p| &mut p.next));
        m.step(pad, ok);
        assert!(m.players[1].ready);
        // 1P now picks the computer's 3P and 4P: Cody, then Jun
        m.step(Dev::Keys, press(|p| &mut p.right));
        m.step(Dev::Keys, ok);
        m.step(Dev::Keys, press(|p| &mut p.right));
        m.step(Dev::Keys, press(|p| &mut p.right));
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
    }

    #[test]
    fn random_court_and_umpire() {
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], ..default() };
        let left = press(|p| &mut p.left);
        m.step(Dev::Keys, press(|p| &mut p.down));
        m.step(Dev::Keys, left); // court 0 → Random, after the last court
        assert_eq!(m.court, COURTS);
        for _ in 0..3 {
            m.step(Dev::Keys, press(|p| &mut p.down));
        }
        m.step(Dev::Keys, press(|p| &mut p.right)); // umpire 4 → Random
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
        m.step(Dev::Keys, ok);
        assert_eq!(m.screen, Screen::Chars);
        m.step(Dev::Keys, press(|p| &mut p.page));
        assert!(m.custom && m.players[0].modded && m.players[1].modded);
        m.step(Dev::Keys, press(|p| &mut p.next));
        m.step(Dev::Keys, press(|p| &mut p.next));
        assert_eq!(m.players[0].costume, 0, "mod a has 2 costumes");
        m.step(Dev::Keys, ok); // 1P: a, costume 1
        // the computer's 2P on a costume 1 too: closed; down to b, then back up and on to costume 2
        assert!(m.blocked(1));
        m.step(Dev::Keys, ok);
        assert!(!m.players[1].ready);
        m.step(Dev::Keys, press(|p| &mut p.down));
        assert_eq!(m.who(1), Who::Mod(1));
        m.step(Dev::Keys, press(|p| &mut p.down)); // the list ends
        assert_eq!(m.who(1), Who::Mod(1));
        m.step(Dev::Keys, press(|p| &mut p.up));
        m.step(Dev::Keys, press(|p| &mut p.prev));
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
        assert!(!m.custom);
        // M1n: a costume whose model failed to load is passed by L1 / R1 and can't be readied on
        let mut m = Menu { mods: vec![fake("c", 3, 3)], ..default() };
        for _ in 0..3 {
            m.step(Dev::Keys, ok);
        }
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
        for _ in 0..3 {
            m.step(Dev::Keys, ok);
        }
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
        for _ in 0..3 {
            m.step(Dev::Keys, ok);
        }
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
    fn grid_moves() {
        // Ashley → right Cody; Cody → down Momoko; Kaito → down Gloria; Will → left Lola
        assert_eq!(GRID[grid_step(0, 1.0, 0.0)].0, 1);
        assert_eq!(GRID[grid_step(1, 0.0, 1.0)].0, 4);
        assert_eq!(GRID[grid_step(6, 0.0, 1.0)].0, 12);
        assert_eq!(GRID[grid_step(13, -1.0, 0.0)].0, 10);
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
