//! The main menu (remaster-only flow in the original menus' art): Local, Online, Settings and Quit. Local goes through
//! singles/doubles, controller assignment, the character select (a cursor per player, at once) and the match
//! confirmation (court, sets, games, umpire), then starts the match as a child process of this executable with the
//! choices as flags (`--chars`, `--outfits`, `--umpire`, `--pads`, …) and comes back here when it ends.
//!
//! Art and text come from the disc: walls, titles and labels from `MENU/MENUxx.XB0`, the font from `word.tm2`, the
//! descriptions and names from `message0.dat`, the grades from the MENU overlay (`exe::Menu`), the play style from
//! TParam.csv. Settings (each enhancement on or off) live in `settings.txt` beside the disc image.
//!
//! Who plays: the device that chose Local is player 1; in the assignment the other pads and the keyboard join with ✕
//! (the first free seat, ←/→ to change seat, ○ to leave), up to four humans, any device on any seat; the rest are the
//! computer's. Player slots in the match: singles 1P→0, 2P→1;
//! doubles 1P→0, 2P→2 (1P's partner), 3P→1, 4P→3. In the character select a costume someone has locked in (ready)
//! is closed to everyone else on that character; the computer's players are picked by player 1 after the humans.
//! `HST_MENU=<screen>` (main, settings, controls, mode, assign, chars, confirm) opens on that screen (for `--shot`).
//!
//! Custom characters (remaster-only): every standard mod under `mods/` beside the disc image (`mods::list`) is on a
//! second page of the select, switched for everyone by Tab / Select: a list of names, 2 columns of [`LIST_ROWS`] a
//! page (↑↓ one, ←→ a column). A card on a mod shows its 3D preview, its name in `word.tm2`, its TParam.csv row's
//! play style and (△) its power and footwork numbers; its costumes are `mod.json`'s. Picked, it plays that match slot
//! (`--slot-mod N DIR`, `--chars` the donor), and a costume readied on is closed to the others as on the disc.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::sound;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::controls::{self, Action, Bindings, Seat};
use super::panel;
use crate::Args;
use crate::audio::{Sound, SoundBank};

pub use super::widescreen::WIDE;

mod previews;

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
/// Sets on the confirm screen (sets played; to win: (n + 1) / 2).
const SETS: [i32; 3] = [1, 3, 5];
/// Player colours: 1P pink, 2P orange, 3P blue, 4P green (`CharacterSelect2_03`'s cards, same order).
const COLOURS: [[f32; 3]; 4] = [[128.0, 60.0, 90.0], [128.0, 80.0, 20.0], [40.0, 90.0, 128.0], [70.0, 115.0, 30.0]];
const CARDS: [[f32; 2]; 4] = [[4.0, 4.0], [4.0, 180.0], [156.0, 4.0], [156.0, 180.0]];
/// `word.tm2`'s glyphs, 12 to a 32-px row from y 32.
const FONT: [&str; 15] = [
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

/// The enhancements, each on or off (all on by default), saved in `settings.txt`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Settings([bool; 4]);
const SETTING_NAMES: [&str; 4] = ["widescreen", "uncapped_fps", "upscaled_textures", "music"];
const SETTING_LABELS: [&str; 4] = ["Widescreen", "Uncapped Frame Rate", "Upscaled Textures", "Music"];

impl Default for Settings {
    fn default() -> Self {
        Settings([true; 4])
    }
}

impl Settings {
    fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for (k, v) in text.lines().filter_map(|l| l.split('#').next()?.split_once('=')) {
            if let Some(i) = SETTING_NAMES.iter().position(|n| *n == k.trim()) {
                s.0[i] = v.trim() != "off";
            }
        }
        s
    }

    fn text(&self) -> String {
        SETTING_NAMES.iter().zip(self.0).map(|(n, on)| format!("{n} = {}\n", if on { "on" } else { "off" })).collect()
    }

    fn path(iso: &str) -> std::path::PathBuf {
        std::path::Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("settings.txt")
    }

    /// The match's flags for these settings.
    fn flags(&self) -> Vec<String> {
        let [wide, uncapped, upscale, music] = self.0;
        [(!wide, "--4x3"), (!uncapped, "--vsync"), (!upscale, "--no-upscale"), (music, "--music")]
            .into_iter()
            .filter(|(on, _)| *on)
            .map(|(_, f)| f.to_string())
            .collect()
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
    /// Select / Tab: the disc or custom page.
    page: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    /// `bgmm_04` plays on the first three, `bgmm_05` on the next three, `bgmm_06` on Confirm (the MENU overlay's track
    /// switch, measured per screen).
    Main,
    Settings,
    Controls,
    Mode,
    Assign,
    Chars,
    Confirm,
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

#[derive(Resource, Debug)]
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
    /// The action waiting for a new key or button (controls screen).
    rebinding: Option<usize>,
    /// The custom roster, and whether the select shows it.
    mods: Vec<crate::mods::Mod>,
    custom: bool,
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
            rebinding: None,
            mods: Vec::new(),
            custom: false,
        }
    }
}

/// Rows per screen.
const MAIN: [&str; 4] = ["Local", "Online", "Settings", "Quit"];
const CONFIRM_ROWS: usize = 5;

/// What a step asks of the app.
#[derive(Debug, PartialEq)]
enum Effect {
    Sound(u8),
    SaveSettings,
    Launch,
    Quit,
}

impl Menu {
    fn players(&self) -> usize {
        if self.doubles { 4 } else { 2 }
    }

    fn who(&self, p: usize) -> Who {
        let pl = self.players[p];
        if pl.modded { Who::Mod(pl.pick) } else { Who::Disc(pl.char()) }
    }

    fn costumes(&self, w: Who) -> usize {
        match w {
            Who::Disc(_) => COSTUMES,
            Who::Mod(i) => self.mods[i].costumes.len(),
        }
    }

    /// Whether player `p`'s character and costume are locked in by someone else.
    fn blocked(&self, p: usize) -> bool {
        let me = self.players[p];
        (0..self.players()).any(|q| q != p && self.players[q].ready && self.who(q) == self.who(p) && self.players[q].costume == me.costume)
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
            if !self.players[p].ready || p != 0 || !humans_ready {
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
                        // ponytail: Online is M2's; the row is there, it does nothing yet
                        1 => out.push(Sound(1)),
                        2 => self.go(Screen::Settings, &mut out),
                        _ => out.push(Quit),
                    }
                }
            }
            Screen::Settings => {
                rows(SETTING_NAMES.len() + 2, &mut self.sel, &mut out);
                let s = self.sel;
                if s < SETTING_NAMES.len() && (k.ok || k.left || k.right) {
                    self.settings.0[s] = !self.settings.0[s];
                    out.extend([Sound(0), SaveSettings]);
                } else if k.ok && s == SETTING_NAMES.len() {
                    self.go(Screen::Controls, &mut out);
                } else if (k.ok && s > SETTING_NAMES.len()) || k.back {
                    self.sel = 2;
                    self.screen = Screen::Main;
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
                    self.sel = SETTING_NAMES.len();
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
                    if let Some(p) = (0..n).find(|&p| self.seats[p].is_none()) {
                        self.seats[p] = Some(dev);
                        out.push(Sound(2));
                    }
                } else if (k.left || k.right || k.up || k.down) && let Some(p) = seated {
                    // the next free seat that way (1P stays the host's)
                    let d = if k.right || k.down { 1 } else { n - 2 };
                    let to = (1..n - 1).map(|j| 1 + (p - 1 + d * j) % (n - 1)).find(|&q| self.seats[q].is_none());
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
                let n_mods = self.mods.len();
                let me = &mut self.players[p];
                if k.card {
                    me.card = !me.card;
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
                        me.costume = (me.costume + if k.next { 1 } else { costumes - 1 }) % costumes;
                        out.push(Sound(0));
                    }
                }
                if k.ok && !self.players[p].ready {
                    if self.blocked(p) {
                        out.push(Sound(1));
                    } else {
                        self.players[p].ready = true;
                        out.push(Sound(2));
                        if (0..self.players()).all(|q| self.players[q].ready) {
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
                        1 => self.court = wrap(self.court, COURTS),
                        2 => self.sets = wrap(self.sets, SETS.len()),
                        3 => self.games = wrap(self.games as usize - 1, 6) as i32 + 1,
                        4 => self.umpire = wrap(self.umpire as usize, 5) as u8,
                        _ => {}
                    }
                    if self.sel > 0 {
                        out.push(Sound(0));
                    }
                }
                if k.ok && self.sel == 0 {
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
            _ => {}
        }
        out
    }

    fn go(&mut self, s: Screen, out: &mut Vec<Effect>) {
        self.screen = s;
        self.sel = 0;
        out.push(Effect::Sound(2));
    }

    /// The match's flags (`pads`: each device's index among the sorted gamepads).
    fn args(&self, pads: &[Entity]) -> Vec<String> {
        let court = self.court + 1;
        // menu player → match player
        let order: &[usize] = if self.doubles { &[0, 2, 1, 3] } else { &[0, 1] };
        let mut chars = vec![0; order.len()];
        let mut outfits = vec![0; order.len()];
        let mut slot_mods = Vec::new();
        for (p, &m) in order.iter().enumerate() {
            chars[m] = self.players[p].char();
            outfits[m] = self.players[p].costume;
            if let Who::Mod(i) = self.who(p) {
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
            self.umpire.to_string(),
            "--sets".into(),
            ((SETS[self.sets] + 1) / 2).to_string(),
            "--games".into(),
            self.games.to_string(),
            "--pads".into(),
            Seat::flag(&seats),
        ]);
        a.extend(slot_mods);
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
    /// The description bar's font (`ascii.bmp`), the last of `Art`'s images.
    Bar,
    Hand,
    /// `SHEETS[k]`.
    Sheet(usize),
    /// A portrait sheet: costume 0 (characters 0–7, 8–13), costume 9 (same), costumes 1..8.
    Shots(usize),
    /// Player `p`'s 3D preview (`previews.rs`), in place of the portrait.
    Preview(usize),
}

const SHEETS: [(&str, &str); 14] = [
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
];
const OPTION: Tex = Tex::Sheet(9);
const CS0: Tex = Tex::Sheet(10);
const FACES: Tex = Tex::Sheet(11);
const NAMES: Tex = Tex::Sheet(12);
const CARD: Tex = Tex::Sheet(13);
const CONFIRM: Tex = Tex::Sheet(14);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
    /// Tiled across `dst` at the screen's scale.
    tile: bool,
}

/// The disc's text and tables the screens show.
struct Text {
    msg: Vec<String>,
    /// Each glyph's cell and ink span in `word.tm2`.
    glyphs: std::collections::HashMap<char, (f32, f32, f32)>,
    grades: [[u8; 5]; 14],
    /// Play style (0 all-round, 1 baseline, 2 net, 3 big server) by character.
    style: [usize; 14],
    /// The custom roster's play style and △ numbers (`MOD_STATS`), from each one's TParam.csv row.
    mods: Vec<(usize, [String; 5])>,
}

/// The custom card's △ numbers: TParam.csv column and label.
const MOD_STATS: [(usize, &str); 5] = [(12, "Serve"), (13, "Stroke"), (14, "Volley"), (40, "Speed"), (41, "Stamina")];

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

fn setup(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>) {
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
    art.push(panel::image(&mut images, &font));
    let inpane = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let inpane = Archive::parse(&inpane).expect("xb archive");
    let hand = inpane.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("hsm_yubi.tm2")).expect("hsm_yubi");
    art.push(panel::image(&mut images, &inpane.read(hand).expect("INPANE bytes")));
    for (xb, name) in SHEETS.iter().chain([&("MENU/MENU03.XB0", "confirmation_03")]) {
        art.push(panel::image(&mut images, &read(&mut iso, xb, &format!("/{name}.tm2"))));
    }
    for u in 0..5 {
        art.push(panel::image(&mut images, &read(&mut iso, "MENU/MENU60.XB0", &format!("/item_umpire{u:02}.tm2"))));
    }
    let shots = ["cs2_shots_nomal_a", "cs2_shots_nomal_b", "cs2_shots_cos_a", "cs2_shots_cos_b"]
        .into_iter()
        .map(String::from)
        .chain((0..CHARS).map(|c| format!("cs2_shots_{c:02}")));
    for name in shots {
        art.push(panel::image(&mut images, &read(&mut iso, "MENU/MENU12B.XB0", &format!("/{name}.tm2"))));
    }
    // the font's ink span per glyph, from its alpha
    let pic = hst_data::tim2::decode_alpha8(&font).expect("TIM2").remove(0);
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
    art.push(images.add(bar_font(&read(&mut iso, "CMN/GFFONT.XB", "/ascii.bmp"))));
    let msg = messages(&read(&mut iso, "MENU/MENU00B.XB0", "/message0.dat"));
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/MENU.BIN").expect("MENU.BIN"));
    let grades = hst_data::exe::Menu::new(&cnf, &bin).expect("supported disc").grades();
    let pc = iso.read("PCDATA/PCDATA.XB").expect("character archive on disc");
    let pc = Archive::parse(&pc).expect("xb archive");
    let csv = pc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).and_then(|e| pc.read(e).ok()).unwrap_or_default();
    let bank = SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").map(Arc::new);
    let style = styles(&csv);
    // the custom roster: a mod whose row can't be read is left out like a broken one
    let mods_dir = Settings::path(&args.iso).with_file_name("mods");
    let roster: Vec<_> = crate::mods::list(&mods_dir)
        .into_iter()
        .filter_map(|m| match crate::mods::tparam(&mut iso, &m) {
            Ok(row) => Some(((mod_style(&m, &style), MOD_STATS.map(|(c, _)| row.get(c).cloned().unwrap_or_default())), m)),
            Err(e) => {
                warn!("{}: {e}", m.dir.display());
                None
            }
        })
        .collect();
    info!("{} custom characters in {}", roster.len(), mods_dir.display());
    let (mod_text, mods) = roster.into_iter().unzip();
    commands.insert_resource(Art(art, bank, Text { msg, glyphs, grades, style, mods: mod_text }));

    let mut menu = Menu { settings: std::fs::read_to_string(Settings::path(&args.iso)).map_or_else(|_| Settings::default(), |t| Settings::parse(&t)), mods, ..default() };
    let screen = std::env::var("HST_MENU").unwrap_or_default();
    menu.screen = match screen.as_str() {
        "settings" => Screen::Settings,
        "controls" => Screen::Controls,
        "mode" => Screen::Mode,
        "assign" => Screen::Assign,
        "chars" => Screen::Chars,
        "confirm" => Screen::Confirm,
        _ => Screen::Main,
    };
    if menu.screen != Screen::Main {
        menu.seats[0] = Some(Dev::Keys);
        menu.doubles = true;
        menu.players[0].cursor = 6;
    }
    commands.insert_resource(menu);
    commands.spawn((super::widescreen::screen_43(), GlobalZIndex(10))).with_children(|p| {
        for i in 0..POOL {
            p.spawn((Slot(i), ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
        }
    });
}

// ---------------------------------------------------------------- layout

struct Draw<'a> {
    out: Vec<Quad>,
    text: &'a Text,
    /// Frames (60 Hz) since the description bar's text changed, and that text.
    tick: u32,
    said: String,
}

const WHITE: [f32; 3] = [128.0; 3];
const DIMMED: [f32; 3] = [80.0; 3];

impl Draw<'_> {
    fn q(&mut self, tex: Tex, src: [f32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: f32) {
        self.out.push(Quad { tex, src, dst, rgb, alpha, tile: false });
    }

    /// The screen's wall (tiled) and title bar.
    fn frame(&mut self, title: usize, wall: usize) {
        self.out.push(Quad { tex: Tex::Sheet(wall), src: [0.0, 0.0, 32.0, 32.0], dst: [0.0, 0.0, 640.0, 448.0], rgb: WHITE, alpha: 128.0, tile: true });
        self.q(Tex::Sheet(title), [0.0, 0.0, 512.0, 64.0], [0.0, 0.0, 512.0, 64.0], WHITE, 128.0);
        self.q(Tex::Sheet(title), [480.0, 0.0, 32.0, 64.0], [512.0, 0.0, 128.0, 64.0], WHITE, 128.0);
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
        for (i, l) in labels.iter().enumerate() {
            let yy = y + 44.0 * i as f32;
            let on = i == sel;
            self.pill(x, yy, w, if on { [128.0, 110.0, 40.0] } else { [70.0, 70.0, 80.0] });
            self.centred(l, x + w / 2.0, yy + 7.0, 24.0, if on { WHITE } else { [100.0; 3] });
        }
        self.q(Tex::Hand, [0.0, 0.0, 64.0, 32.0], [x - 60.0 + hand_x, y + 44.0 * sel as f32 + 3.0, 64.0, 32.0], WHITE, 128.0);
    }

    /// The description bar along the bottom.
    fn info(&mut self, s: &str) {
        self.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 404.0, 640.0, 32.0], [0.0; 3], 90.0);
        self.said = s.into();
        // the boot program's text printer: `ascii.bmp` cell char - 0x1f (21 a row, 12×23), 11×22 texels drawn
        // 11×22 at an 11 px pitch, from y 408; a glyph is skipped once it is off either edge
        let mut x = ticker_x(self.tick, s.chars().count());
        for c in s.chars() {
            let g = (c as u32).wrapping_sub(0x1f).min(224);
            if x + 11.0 > 0.0 && x < 640.0 {
                let (u, v) = ((g % 21 * 12) as f32, (g / 21 * 23) as f32);
                self.q(Tex::Bar, [u, v, 11.0, 22.0], [x, 408.0, 11.0, 22.0], WHITE, 128.0);
            }
            x += 11.0;
        }
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
fn ticker_x(f: u32, n: usize) -> f32 {
    let w = 11 * n as i32;
    let x = if w <= 616 { 16 } else { 656 - 2 * ((f as i32 + 160) % ((w + 656) >> 1)) };
    if f < 160 { (656 - 32 * f as i32).max(16) as f32 } else { x as f32 }
}

/// `ascii.bmp` (4-bit, bottom-up) as the bar's palette draws it: ink 200 under the text's 0x8f tint (223), its
/// alpha by pixel index from a ramp (index 15 opaque, 0–4 clear).
fn bar_font(bmp: &[u8]) -> Image {
    const ALPHA: [u8; 16] = [0, 0, 0, 0, 0, 0x1a, 0x1a, 0x1a, 0x1a, 0x33, 0x33, 0x33, 0x4c, 0x4c, 0x66, 0x80];
    let off = u32::from_le_bytes(bmp[10..14].try_into().unwrap()) as usize;
    let mut rgba = Vec::with_capacity(256 * 256 * 4);
    for y in 0..256 {
        for x in 0..256 {
            let b = bmp[off + (255 - y) * 128 + x / 2];
            let i = if x % 2 == 0 { b >> 4 } else { b & 15 };
            rgba.extend([223, 223, 223, (ALPHA[i as usize] as u32 * 255 / 128) as u8]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d { width: 256, height: 256, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

fn layout(m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity], hand_x: f32, tick: u32) -> (Vec<Quad>, String) {
    let mut d = Draw { out: Vec::new(), text, tick, said: String::new() };
    match m.screen {
        Screen::Main | Screen::Playing => {
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
            labels.extend(["Controls".into(), "Back".into()]);
            d.rows(&labels, m.sel, 120.0, 90.0, 300.0, hand_x);
            for (i, on) in m.settings.0.iter().enumerate() {
                let src = if *on { [429.0, 5.0, 39.0, 23.0] } else { [423.0, 37.0, 51.0, 23.0] };
                d.q(OPTION, src, [440.0, 90.0 + 44.0 * i as f32 + 8.0, src[2], src[3]], WHITE, 128.0);
            }
            d.info(&match m.sel {
                0 => "Fill wide windows with the court; off shows the original 4:3 picture.".to_string(),
                1 => "Draw as fast as the PC allows; off waits for the display (the game itself always runs at 60 Hz).".into(),
                2 => "Use the upscaled textures in mods/texture-replacements.".into(),
                3 => "Play each court's music in matches.".into(),
                4 => "Change the keys and buttons (controls.txt).".into(),
                _ => text.msg(190).to_string(),
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
            d.q(Tex::Hand, [0.0, 0.0, 64.0, 32.0], [56.0 + hand_x, 68.0 + 28.0 * m.sel as f32, 64.0, 32.0], WHITE, 128.0);
            d.info(if m.rebinding.is_some() { "Press the new key or button (Escape cancels)." } else { "Choose an action to rebind." });
        }
        Screen::Mode => {
            d.frame(2, 5);
            for (i, src) in [[2.0, 2.0, 252.0, 30.0], [2.0, 34.0, 252.0, 30.0]].into_iter().enumerate() {
                let on = (i == 1) == m.doubles;
                d.q(CS0, src, [134.0, 150.0 + 70.0 * i as f32, 372.0, 44.0], if on { WHITE } else { DIMMED }, if on { 128.0 } else { 90.0 });
            }
            d.q(Tex::Hand, [0.0, 0.0, 64.0, 32.0], [76.0 + hand_x, 156.0 + 70.0 * m.doubles as u8 as f32, 64.0, 32.0], WHITE, 128.0);
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
        Screen::Confirm => confirm(&mut d, m, text, hand_x),
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

fn chars(d: &mut Draw, m: &Menu, text: &Text) {
    d.frame(3, 7);
    let n = m.players();
    d.q(CS0, if m.doubles { [2.0, 34.0, 252.0, 30.0] } else { [2.0, 2.0, 252.0, 30.0] }, [194.0, 56.0, 252.0, 30.0], WHITE, 128.0);
    let picking = |p: usize| m.seats[p].is_some() || (0..n).filter(|&q| m.seats[q].is_none()).all(|q| q >= p || m.players[q].ready) && (0..n).all(|q| m.seats[q].is_none() || m.players[q].ready);
    let lead = (0..n).find(|&p| !m.players[p].ready).unwrap_or(0);
    if m.custom {
        custom_list(d, m, (0..n).filter(|&p| picking(p) && !m.players[p].ready), m.players[lead].pick);
    } else {
        // the faces, darkened where every costume of it is ... (all open in the remaster)
        for &(c, x, y) in &GRID {
            d.q(Tex::Solid, [0.0; 4], [x - 32.0, y - 32.0, 64.0, 64.0], [110.0, 100.0, 90.0], 128.0);
            d.q(FACES, [(c % 8) as f32 * 64.0, (c / 8) as f32 * 64.0, 64.0, 64.0], [x - 30.0, y - 30.0, 60.0, 60.0], WHITE, 128.0);
        }
    }
    let panels: [[f32; 2]; 4] = if m.doubles { [[16.0, 56.0], [16.0, 240.0], [480.0, 56.0], [480.0, 240.0]] } else { [[16.0, 56.0], [480.0, 56.0], [0.0; 2], [0.0; 2]] };
    for p in 0..n {
        let pl = m.players[p];
        let c = pl.char();
        let [x, y] = panels[p];
        let [u, v] = CARDS[p];
        let human = m.seats[p].is_some();
        let picking = picking(p);
        let modded = match m.who(p) {
            Who::Mod(i) => Some(i),
            Who::Disc(_) => None,
        };
        // a card nobody is picking on yet is dark
        d.q(CARD, [u, v, 140.0, 164.0], [x, y, 144.0, 168.0], if picking || pl.ready { WHITE } else { [55.0; 3] }, 128.0);
        if picking || pl.ready {
            d.q(Tex::Preview(p), [0.0; 4], [x + 8.0, y + 4.0, 128.0, 160.0], WHITE, 128.0);
            if pl.card && let Some(i) = modded {
                d.q(Tex::Solid, [0.0; 4], [x + 6.0, y + 36.0, 132.0, 96.0], [0.0; 3], 90.0);
                for (k, ((_, label), v)) in MOD_STATS.iter().zip(&text.mods[i].1).enumerate() {
                    let yy = y + 40.0 + 18.0 * k as f32;
                    d.text(label, x + 10.0, yy, 16.0, WHITE);
                    let w = d.width(v, 16.0);
                    d.text(v, x + 130.0 - w, yy, 16.0, WHITE);
                }
            } else if pl.card {
                d.q(Tex::Solid, [0.0; 4], [x + 6.0, y + 36.0, 132.0, 96.0], [0.0; 3], 90.0);
                for (k, g) in text.grades[c].iter().enumerate() {
                    let yy = y + 40.0 + 18.0 * k as f32;
                    d.q(CS0, [280.0, 16.0 * k as f32 + 2.0, 112.0, 18.0], [x + 8.0, yy, 100.0, 16.0], WHITE, 128.0);
                    d.q(NAMES, [0.0, 24.0 * (5 - *g as usize).min(5) as f32, 22.0, 24.0], [x + 112.0, yy - 2.0, 18.0, 20.0], WHITE, 128.0);
                }
            }
            let style = modded.map_or(text.style[c], |i| text.mods[i].0);
            let style = [[320.0, 112.0, 144.0, 24.0], [324.0, 136.0, 136.0, 24.0], [346.0, 160.0, 94.0, 24.0], [346.0, 184.0, 94.0, 24.0]][style];
            d.q(CS0, style, [x + 72.0 - style[2] / 2.0 * 0.9, y + 118.0, style[2] * 0.9, 22.0], WHITE, 128.0);
            match modded {
                Some(i) => d.fitted(&m.mods[i].name, x + 72.0, y + 142.0, 20.0, 128.0),
                None => d.q(NAMES, [23.0, 24.0 * c as f32, 105.0, 24.0], [x + 20.0, y + 140.0, 105.0, 24.0], WHITE, 128.0),
            }
            if pl.ready {
                d.q(CS0, [2.0, 208.0, 108.0, 24.0], [x + 18.0, y + 96.0, 108.0, 24.0], WHITE, 128.0);
            } else if m.blocked(p) {
                d.q(CS0, [2.0, 232.0, 110.0, 24.0], [x + 17.0, y + 96.0, 110.0, 24.0], WHITE, 128.0);
            }
            if !pl.ready && !pl.modded {
                let [fx, fy] = [GRID[pl.cursor].1, GRID[pl.cursor].2];
                d.q(Tex::Hand, [0.0, 0.0, 64.0, 32.0], [fx - 40.0 + 10.0 * (p % 2) as f32, fy - 8.0 + 12.0 * (p / 2) as f32, 48.0, 24.0], COLOURS[p], 128.0);
                d.text(&format!("{}P", p + 1), fx - 4.0 + 14.0 * (p % 2) as f32, fy - 32.0 + 14.0 * (p / 2) as f32, 16.0, COLOURS[p]);
            }
            d.text(&format!("{}P", p + 1), x + 6.0, y + 4.0, 22.0, WHITE);
            d.text(&format!("{}", pl.costume + 1), x + 6.0, y + 26.0, 18.0, WHITE);
        } else {
            d.text(&format!("{}P", p + 1), x + 6.0, y + 4.0, 22.0, WHITE);
        }
        if !human {
            d.text("COM", x + 8.0, y + 30.0, 20.0, [60.0, 120.0, 30.0]);
        }
    }
    match m.who(lead) {
        Who::Mod(i) => {
            let md = &m.mods[i];
            let s = if md.costumes.len() == 1 { "" } else { "s" };
            d.info(&format!("{}: custom character, {} costume{s}. Tab / Select: the disc characters.", md.name, md.costumes.len()));
        }
        Who::Disc(c) if !m.mods.is_empty() => d.info(&format!("{} (Tab / Select: custom characters)", text.msg(308 + c))),
        Who::Disc(c) => d.info(text.msg(308 + c)),
    }
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
        let pl = m.players[p];
        let x = 20.0 + w * k as f32 + (w - 112.0) / 2.0;
        match m.who(p) {
            Who::Mod(i) => {
                // ponytail: no 3D preview on this screen; the name and costume on a dark card
                d.q(Tex::Solid, [0.0; 4], [x, 64.0, 112.0, 147.0], [30.0, 30.0, 40.0], 110.0);
                d.fitted(&m.mods[i].name, x + 56.0, 120.0, 20.0, 104.0);
                d.centred(&format!("{}", pl.costume + 1), x + 56.0, 146.0, 18.0, WHITE);
            }
            Who::Disc(c) => {
                let (sheet, src) = portrait(c, pl.costume);
                d.q(sheet, src, [x, 64.0, 112.0, 147.0], WHITE, 128.0);
            }
        }
        d.centred(&format!("{}P", p + 1), x + 56.0, 214.0, 24.0, COLOURS[p]);
        if m.seats[p].is_none() {
            d.centred("COM", x + 56.0, 236.0, 18.0, [60.0, 120.0, 30.0]);
        }
    }
    d.q(CONFIRM, [448.0, 264.0, 64.0, 48.0], [288.0, 110.0, 64.0, 48.0], WHITE, 128.0);
    // the umpire's face at the top right
    d.q(CONFIRM, [10.0 + 88.0 * m.umpire as f32, 394.0, 76.0, 60.0], [540.0, 2.0, 76.0, 60.0], WHITE, 128.0);
    let umpire = text.msg(669 + 2 * m.umpire as usize).to_string();
    let w = d.width(&umpire, 22.0);
    d.text(&umpire, 532.0 - w, 20.0, 22.0, WHITE);
    // court name
    let court = format!("{} {}", text.msg(276 + 2 * m.court), text.msg(277 + 2 * m.court));
    d.text(&court, 30.0, 258.0, 24.0, [128.0, 110.0, 40.0]);
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
    d.q(Tex::Hand, [0.0, 0.0, 64.0, 32.0], [hx - 50.0 + hand_x, hy + 2.0, 64.0, 32.0], WHITE, 128.0);
    let info = match m.sel {
        0 => text.msg(323).to_string(),
        1 => format!("{} {}", text.msg(581 + 2 * m.court), text.msg(582 + 2 * m.court)),
        2 => text.msg(324).to_string(),
        3 => text.msg(325).to_string(),
        _ => text.msg(328).to_string(),
    };
    d.info(&info);
}

// ---------------------------------------------------------------- systems

/// A running match: set when its process ends.
#[derive(Resource)]
struct Child(Arc<AtomicBool>);

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
    child: Option<Res<Child>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    mut repeat: Local<std::collections::HashMap<(Option<Entity>, usize), Repeat>>,
    mut exit: MessageWriter<AppExit>,
) {
    let play = |key: u8| {
        if let (Some(s), Some(Art(_, Some(b), _))) = (&sound, art.as_deref()) {
            s.play_centre(b, sound::Play { slot: 9, program: 0, key, volume: 0x80, speed: 1.0 });
        }
    };
    if let Some(c) = child {
        if c.0.load(Ordering::Relaxed) {
            commands.remove_resource::<Child>();
            if let Ok(mut w) = window.single_mut() {
                w.visible = true;
            }
            menu.screen = Screen::Main;
            menu.sel = 0;
        }
        return;
    }
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g)| g.vendor_id() != Some(0x28de)).map(|(e, _)| e).collect();
    pads.sort();
    // the controls screen waiting for a key or button
    if let Some(i) = menu.rebinding {
        if keys.just_pressed(KeyCode::Escape) {
            menu.rebinding = None;
            play(1);
            return;
        }
        let key = keys.get_just_pressed().find(|k| Bindings::bindable_key(**k)).copied();
        let button = gamepads.iter().flat_map(|(_, g)| g.get_just_pressed().copied().collect::<Vec<_>>()).find(|b| Bindings::bindable_pad(*b));
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
                k.page = g.just_pressed(GamepadButton::Select);
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
                    let path = Settings::path(&args.iso);
                    if let Err(e) = std::fs::write(&path, menu.settings.text()) {
                        warn!("{}: {e}", path.display());
                    }
                }
                Effect::Quit => {
                    exit.write(AppExit::Success);
                }
                Effect::Launch => {
                    let a = menu.args(&pads);
                    info!("starting the match: {}", a.join(" "));
                    let exe = std::env::current_exe().expect("own executable");
                    match std::process::Command::new(exe).arg(&args.iso).args(&a).spawn() {
                        Ok(mut c) => {
                            let done = Arc::new(AtomicBool::new(false));
                            let flag = done.clone();
                            std::thread::spawn(move || {
                                let _ = c.wait();
                                flag.store(true, Ordering::Relaxed);
                            });
                            commands.insert_resource(Child(done));
                            if let Ok(mut w) = window.single_mut() {
                                w.visible = false;
                            }
                        }
                        Err(e) => {
                            warn!("starting the match: {e}");
                            menu.screen = Screen::Confirm;
                        }
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
    mut ticker: Local<(String, f32)>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let Some(art) = art else { return };
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g)| g.vendor_id() != Some(0x28de)).map(|(e, _)| e).collect();
    pads.sort();
    // the hand swings out and back (the pause menu's `Hand` is the original's; here a plain sine)
    let hand_x = -8.0 + 8.0 * (time.elapsed_secs() * 5.5).cos();
    // the description ticker counts the original's 60 Hz frames from its text's change
    ticker.1 += time.delta_secs() * 60.0;
    let (mut quads, said) = layout(&menu, &art.2, &bind, &pads, hand_x, ticker.1 as u32);
    if said != ticker.0 {
        *ticker = (said, 0.0);
        quads = layout(&menu, &art.2, &bind, &pads, hand_x, 0).0;
    }
    let scale = window.single().map_or(1.0, |w| w.physical_height() as f32 / 448.0 / w.scale_factor());
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i).filter(|q| !matches!(q.tex, Tex::Preview(_))) else {
            *vis = Visibility::Hidden;
            continue;
        };
        img.image = match quad.tex {
            Tex::Solid => art.0[0].clone(),
            Tex::Font => art.0[1].clone(),
            Tex::Bar => art.0[art.0.len() - 1].clone(),
            Tex::Hand => art.0[2].clone(),
            Tex::Sheet(k) => art.0[3 + k].clone(),
            Tex::Shots(k) => art.0[3 + SHEETS.len() + 1 + 5 + k].clone(),
            Tex::Preview(_) => continue,
        };
        let [u, v, w, h] = quad.src;
        img.rect = (quad.tex != Tex::Solid).then(|| Rect::new(u, v, u + w, v + h));
        img.image_mode = if quad.tile { NodeImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: scale } } else { NodeImageMode::Stretch };
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        img.color = Color::srgba(r, g, b, quad.alpha / 128.0);
        let [x, y, w, h] = quad.dst;
        node.left = Val::Percent(x / 6.4);
        node.top = Val::Percent(y / 4.48);
        node.width = Val::Percent(w / 6.4);
        node.height = Val::Percent(h / 4.48);
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
            Screen::Main | Screen::Settings | Screen::Controls => Some(0),
            Screen::Mode | Screen::Assign | Screen::Chars => Some(1),
            Screen::Confirm => Some(2),
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
    let want = menu.screen.bgm();
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
        .add_systems(Startup, (setup, load_bgm))
        .add_systems(Update, ((step, draw).chain(), bgm));
    previews::plugin(app);
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let s = Settings([true, false, true, false]);
        assert_eq!(Settings::parse(&s.text()), s);
        assert_eq!(Settings::parse("music = off\nbogus = off\n"), Settings([true, true, true, false]));
        assert_eq!(s.flags(), ["--vsync"]);
        assert_eq!(Settings([false, true, false, true]).flags(), ["--4x3", "--no-upscale", "--music"]);
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
            costumes: (0..costumes).map(|c| format!("model/c{c:02}.glb")).collect(),
            donor,
            hand: 1.0,
            base: donor,
            overrides: vec![],
            ai_row: donor,
            texture_face: false,
            no_face: false,
            voice: "voice/".into(),
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
}
