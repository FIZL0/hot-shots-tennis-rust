//! B372: Browse as a picture grid. B383: the title band has one tab row, the kinds (`mods_kind::row`), then the
//! search box (✕ types into it from the keyboard, a pad's or a phone's opens an on-screen keyboard; it never takes the
//! keys by itself), the Upload button and the downloads' pill; a filter rail on the left (Game, Plays like, Hand,
//! Show, each option with how many mods it leaves; Show's Installed and My uploads are those grids, the pages the
//! old tab row had); the tiles in the middle under a heading and the Newest / A-Z / Uploader sort; the chosen mod's
//! panel on the right (its full picture, △ card, Install / More by / Save). The cursor is one number
//! (`Menu::sel`): the search box, Upload, the heading's words, the rail's options, then the tiles, so
//! `View::hovered` is the tile under it as on every other page.
//!
//! Pictures load by themselves: every tile on view, then one more screen the way the grid last scrolled, a few at a
//! time ([`plan`]); each is the store's small `thumb.png` (an older upload: its preview shrunk once), cached by the
//! zip's sha256 beside `mods/` (`modstore::thumb`), so a second visit reads the cache, not the network.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::super::pointer::{Act, OK, press};
use super::super::{CARD_FILL, CHAR_NAMES, Draw, Effect, GOLD, INK, Menu, Press, STYLE_NAMES, Tex, WHITE};
use super::{Entry, GREEN, Kind, Page, RED, View, mb, modstore};

/// The sort words, then (the whole store) Refresh, or (an author's mods) Download all and All mods.
const SORTS: [&str; 3] = ["Newest", "A-Z", "Uploader"];
/// The rail's groups' titles.
const GROUPS: [&str; 4] = ["GAME", "PLAYS LIKE", "HAND", "SHOW"];
/// The games the rail lists by name (the most mods first); the rest are Other.
const GAMES: usize = 6;
/// The tile pictures kept loaded at most (the ones on view and the next screen are never dropped).
const KEEP: usize = 160;
/// The gold of the chosen things, and the dark of the panels and tiles.
const PANEL: [f32; 3] = [34.0, 31.0, 38.0];
const TILE: [f32; 3] = [50.0, 46.0, 56.0];
const LIGHT: [f32; 3] = [100.0, 96.0, 104.0];
const BLUE: [f32; 3] = [30.0, 80.0, 125.0];
const PINK: [f32; 3] = [118.0, 60.0, 84.0];

/// The on-screen keyboard's keys: four rows of ten, then Space, Delete, Clear and Done.
const KEYS: [&str; 4] = ["abcdefghij", "klmnopqrst", "uvwxyz0123", "456789-'.&"];
const WIDE: [&str; 4] = ["Space", "Delete", "Clear", "Done"];

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(super) enum Sort {
    #[default]
    Newest,
    Az,
    Uploader,
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(super) enum Show {
    #[default]
    All,
    Installed,
    Updates,
    Mine,
    Wish,
}

/// B383: Show's options, in the rail's order.
const SHOWS: [Show; 5] = [Show::All, Show::Installed, Show::Updates, Show::Mine, Show::Wish];

/// B383: the heading's Add mod word as last drawn (the browser's file input follows it; `mods_screen::add_rect`).
pub(super) static ADD_AT: std::sync::Mutex<Option<[f32; 4]>> = std::sync::Mutex::new(None);

/// A Game filter: one game by its `source`, or every game the rail doesn't list.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Game {
    Src(String),
    Other,
}

/// A tile's picture: asked for, the store has none (or couldn't be reached), or in `Shelf::thumbs[.0]` (its width
/// over its height).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Pic {
    Asked,
    Missing,
    Shown(u32, f32),
}

/// Browse's own state on `View`.
#[derive(Default, Clone, Debug)]
pub(super) struct Browse {
    pub sort: Sort,
    pub game: Option<Game>,
    pub style: Option<u8>,
    pub left: bool,
    pub show: Show,
    pub search: String,
    /// B383: ✕ on the search box with the keyboard: the keys type into it until the cursor leaves it, Enter or Esc.
    pub focus: bool,
    /// The on-screen keyboard is up: its key under the cursor.
    pub osk: Option<usize>,
    /// The saved mods by [`Entry::key`] (`mods-wishlist.txt` beside settings.txt / localStorage, the old name kept so
    /// lists carry over; None: not read yet).
    pub wish: Option<HashSet<String>>,
    /// The tile pictures by zip sha256.
    pub pics: HashMap<String, Pic>,
    /// The grid last scrolled down (its next screen is fetched below, else above), and the top row it was on.
    pub down: bool,
    pub seen: usize,
    /// The tile and rail option the cursor left from (← into the rail, → back to the grid).
    pub tile_at: usize,
    pub rail_at: usize,
    /// The tile a tap chose: a tap on it again installs (a first tap only chooses).
    pub tapped: Option<usize>,
    /// Frames since the page opened (the loading tiles' shimmer, the downloads' spinner).
    pub tick: u32,
    /// The made-up store of a `sample` screen: no picture is fetched (installed ones show the select's).
    pub offline: bool,
}

/// A rail option.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Opt {
    Game(Option<Game>),
    Style(Option<u8>),
    Left(bool),
    Show(Show),
}

impl Opt {
    fn group(&self) -> usize {
        match self {
            Opt::Game(_) => 0,
            Opt::Style(_) => 1,
            Opt::Left(_) => 2,
            Opt::Show(_) => 3,
        }
    }

    fn label(&self) -> String {
        match self {
            Opt::Game(None) => "All games".into(),
            Opt::Game(Some(Game::Src(s))) => s.clone(),
            Opt::Game(Some(Game::Other)) => "Other".into(),
            Opt::Style(None) => "Any".into(),
            Opt::Style(Some(s)) => ["All-round", "Baseline", "Net", "Big Server"][(*s as usize).min(3)].into(),
            Opt::Left(false) => "Either".into(),
            Opt::Left(true) => "Left-handed".into(),
            Opt::Show(s) => match s {
                Show::All => "Everything",
                Show::Installed => "Installed",
                Show::Updates => "Updates",
                Show::Mine => "My uploads",
                Show::Wish => "Saved mods",
            }
            .into(),
        }
    }
}

/// Where the cursor is on Browse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Slot {
    Search,
    /// B383: the header's Upload button.
    Upload,
    Word(usize),
    Rail(usize),
    Tile(usize),
}

impl View {
    /// The heading's words: the sorts, then Refresh (an author's mods: Download all, All mods; B383: Installed's
    /// Add mod, Reload and Remove all), then Reload while mods changed since the start (B387: the way back to the
    /// Reload a Later put off).
    pub(super) fn words(&self) -> Vec<&'static str> {
        let mut w = SORTS.to_vec();
        w.extend(match (self.page, &self.by) {
            (Page::Installed, _) => &["Add mod", "Reload", "Remove all"][..],
            (_, Some(_)) => &["Download all", "All mods"][..],
            _ => &["Refresh"][..],
        });
        if self.changed && self.page != Page::Installed {
            w.push("Reload");
        }
        w
    }

    /// The games the rail names (of the kind tab's mods): the most mods first (at most [`GAMES`]; the chosen one
    /// always), and whether any mod is from another (Other).
    fn games(&self) -> (Vec<String>, bool) {
        let mut n: HashMap<&str, usize> = HashMap::new();
        for e in self.entries().iter().filter(|e| e.meta.kind == self.kind) {
            *n.entry(e.meta.source.as_str()).or_default() += 1;
        }
        let mut l: Vec<(&str, usize)> = n.into_iter().filter(|(s, _)| !s.is_empty()).collect();
        l.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let mut names: Vec<String> = l.iter().take(GAMES).map(|g| g.0.to_string()).collect();
        if let Some(Game::Src(s)) = &self.b.game
            && !names.contains(s)
        {
            names.push(s.clone());
        }
        let other = self.entries().iter().filter(|e| e.meta.kind == self.kind).any(|e| !names.contains(&e.meta.source));
        (names, other || self.b.game == Some(Game::Other))
    }

    /// The rail's options, group by group.
    pub(super) fn rail(&self) -> Vec<Opt> {
        let (games, other) = self.games();
        let mut l = vec![Opt::Game(None)];
        l.extend(games.into_iter().map(|g| Opt::Game(Some(Game::Src(g)))));
        if other {
            l.push(Opt::Game(Some(Game::Other)));
        }
        // M8f9: play style and handedness are a character's
        if self.kind == Kind::Character {
            l.push(Opt::Style(None));
            l.extend((0..4).map(|s| Opt::Style(Some(s))));
            l.extend([Opt::Left(false), Opt::Left(true)]);
        }
        l.extend(SHOWS.map(Opt::Show));
        l
    }

    /// The option chosen in group `g`.
    fn chosen(&self, g: usize) -> Opt {
        match g {
            0 => Opt::Game(self.b.game.clone()),
            1 => Opt::Style(self.b.style),
            2 => Opt::Left(self.b.left),
            _ => Opt::Show(self.b.show),
        }
    }

    /// Whether `e` is one option `o` leaves (`games`: the games the rail names).
    fn passes(&self, e: &Entry, o: &Opt, games: &[String]) -> bool {
        let m = &e.meta;
        match o {
            Opt::Game(None) | Opt::Style(None) | Opt::Left(false) | Opt::Show(Show::All) => true,
            Opt::Game(Some(Game::Src(s))) => m.source == *s,
            Opt::Game(Some(Game::Other)) => !games.contains(&m.source),
            // (a court or umpire has neither: those filters aren't on its tab's rail)
            Opt::Style(Some(_)) | Opt::Left(true) if m.kind != Kind::Character => true,
            Opt::Style(Some(s)) => m.style == Some(*s),
            Opt::Left(true) => m.hand.is_some_and(|h| h < 0),
            // (B383: Installed and My uploads are lists of their own, `count_of`'s and `entries`')
            Opt::Show(Show::Installed | Show::Mine) => true,
            Opt::Show(Show::Updates) => self.state(e) == Some("Update"),
            Opt::Show(Show::Wish) => self.b.wish.as_ref().is_some_and(|w| w.contains(&e.key())),
        }
    }

    /// Whether `e` matches the search (its name, uploader or game) and every group's chosen option but `skip`'s.
    fn keeps(&self, e: &Entry, skip: Option<usize>, games: &[String]) -> bool {
        // B373: the chosen kind's only
        e.meta.kind == self.kind && matches(e, &self.b.search) && (0..4).filter(|g| Some(*g) != skip).all(|g| self.passes(e, &self.chosen(g), games))
    }

    /// How many mods option `o` would leave with the rest of the filters as they are.
    pub(super) fn count_of(&self, o: &Opt) -> usize {
        let games = self.games().0;
        // B383: Show's options count their own list (the installed mods, your uploads, the store's or an author's)
        let list: &[Entry] = match o {
            Opt::Show(Show::Installed) => &self.inst,
            Opt::Show(Show::Mine) => self.mine.as_deref().unwrap_or_default(),
            Opt::Show(_) => match &self.by {
                Some(b) => &b.list,
                None => self.store.as_deref().unwrap_or_default(),
            },
            _ => self.entries(),
        };
        list.iter().filter(|e| self.keeps(e, Some(o.group()), &games) && self.passes(e, o, &games)).count()
    }

    /// The tiles: the store's mods the search and filters leave, sorted.
    pub(super) fn tiles(&self) -> Vec<&Entry> {
        let games = self.games().0;
        let mut l: Vec<&Entry> = self.entries().iter().filter(|e| self.keeps(e, None, &games)).collect();
        match self.b.sort {
            // the newest pull request first; merged ones (no number) after, A-Z as the store lists them
            Sort::Newest => l.sort_by_key(|e| std::cmp::Reverse(modstore::pr_of(&e.rev).unwrap_or(0))),
            Sort::Az => l.sort_by_key(|e| e.meta.name.to_lowercase()),
            Sort::Uploader => l.sort_by_key(|e| (e.meta.author.is_empty(), e.meta.author.to_lowercase(), e.meta.name.to_lowercase())),
        }
        l
    }

    /// The search box, Upload, the heading's words and the rail's options: the slots before the tiles.
    pub(super) fn browse_acts(&self) -> usize {
        2 + self.words().len() + self.rail().len()
    }

    pub(super) fn slot(&self, sel: usize) -> Slot {
        let (w, r) = (self.words().len(), self.rail().len());
        match sel {
            0 => Slot::Search,
            1 => Slot::Upload,
            s if s < 2 + w => Slot::Word(s - 2),
            s if s < 2 + w + r => Slot::Rail(s - 2 - w),
            s => Slot::Tile(s - 2 - w - r),
        }
    }

    /// The saved mods, read once.
    pub(super) fn wish(&mut self) -> &mut HashSet<String> {
        // (the tests start with none)
        self.b.wish.get_or_insert_with(|| if cfg!(test) { None } else { crate::store::read_to_string(WISH).ok() }.map(|s| s.lines().filter(|l| !l.is_empty()).map(String::from).collect()).unwrap_or_default())
    }

    /// □: `e` into or out of the saved mods (kept at once).
    fn wish_toggle(&mut self, e: &Entry) -> bool {
        let k = e.key();
        let w = self.wish();
        let on = !w.remove(&k);
        if on {
            w.insert(k);
        }
        let mut l: Vec<&String> = w.iter().collect();
        l.sort();
        let text: String = l.into_iter().map(|k| format!("{k}\n")).collect();
        // (the tests' lists aren't kept)
        #[cfg(not(test))]
        let _ = crate::store::write(WISH, text);
        #[cfg(test)]
        let _ = text;
        on
    }

    /// A tap on the tile at cursor slot `sel`: whether a tap chose it already (this one then presses ✕).
    pub(in super::super) fn tile_tap(&mut self, sel: usize) -> bool {
        let again = self.b.tapped == Some(sel);
        self.b.tapped = Some(sel);
        again
    }

    /// The mouse moved onto another tile: the next tap only chooses.
    pub(in super::super) fn b_untap(&mut self) {
        self.b.tapped = None;
    }
}

const WISH: &str = "mods-wishlist.txt";

/// Whether `e`'s name, uploader or game has `q` in it (any case; blank matches all).
fn matches(e: &Entry, q: &str) -> bool {
    let q = q.trim().to_lowercase();
    q.is_empty() || [&e.meta.name, &e.meta.author, &e.meta.source].iter().any(|s| s.to_lowercase().contains(&q))
}

/// The tiles to fetch pictures for, in order: every one on view (`rows` rows of `cols` from row `top`, of `n`), then
/// one more screen the way the grid last scrolled (nearest first).
pub(super) fn plan(n: usize, cols: usize, top: usize, rows: usize, down: bool) -> Vec<usize> {
    let cols = cols.max(1);
    let row = |r: usize| (r * cols..((r + 1) * cols).min(n)).collect::<Vec<_>>();
    let mut l: Vec<usize> = (top..top + rows).flat_map(row).collect();
    // (at the top nothing is above: the screen below)
    if down || top == 0 {
        l.extend((top + rows..top + 2 * rows).flat_map(row));
    } else {
        l.extend((top.saturating_sub(rows)..top).rev().flat_map(row));
    }
    l
}

// ---------------------------------------------------------------- layout

/// Where Browse's parts go on the view `g` fits (640×448 units, as every screen's).
#[derive(Clone, Copy, Debug)]
pub(super) struct Lay {
    /// The search box, Upload and the downloads' pill sit in the title band right of the kind tabs (a wide view),
    /// else in a row under it; the room the tabs keep.
    pub tabs_room: f32,
    pub search: [f32; 4],
    pub upload: [f32; 4],
    pub queue: [f32; 4],
    pub rail: [f32; 4],
    /// The grid's box (heading and tiles), the tiles' area in it, the chosen mod's panel.
    pub grid: [f32; 4],
    pub tiles: [f32; 4],
    pub detail: [f32; 4],
    pub cols: usize,
    pub rows: usize,
    /// A tile's width, its picture's height and the row pitch.
    pub tw: f32,
    pub ph: f32,
    pub pitch: f32,
    /// The footer's line.
    pub foot: f32,
}

const GAP: f32 = 7.0;
/// A tile's picture is this much wider than high; its name and line under it take `NAME_H`.
const PIC_ASPECT: f32 = 1.1;
const NAME_H: f32 = 22.0;
/// The rail's option and group title pitches.
const OPT_H: f32 = 14.5;
const TITLE_H: f32 = 17.0;

pub(super) fn lay(g: &super::super::Grid) -> Lay {
    let [l, y, w, _] = g.strip;
    let bottom = g.detail[1] + g.detail[3] + 32.0;
    let portrait = g.detail[2] >= g.strip[2];
    let right = l + w;
    // the title band: the kind tabs from the title's tab, then the search box, Upload and the pill
    // (`options::tab_row`'s top)
    let band = !portrait && w >= 700.0;
    let (qw, uw, sw) = (128.0, 72.0, if band { 190.0f32.min(w * 0.24) } else { (w * 0.5).min(260.0) });
    let top = y - 43.0;
    let (search, upload, queue, cy) = if band {
        let ux = right - qw - 8.0 - uw;
        ([ux - 8.0 - sw, top, sw, 24.0], [ux, top, uw, 24.0], [right - qw, top, qw, 24.0], y)
    } else {
        ([l, y, sw, 24.0], [l + sw + 8.0, y, uw, 24.0], [right - qw, y, qw, 24.0], y + 32.0)
    };
    let tabs_room = if band { search[0] - 10.0 - (l - 12.0) } else { w + 24.0 };
    let h = bottom - cy;
    let rw = (w * 0.15).clamp(92.0, 124.0);
    let (detail, gh) = if portrait {
        let gh = (h * 0.56).max(160.0);
        ([l, cy + gh + 8.0, w, h - gh - 8.0], gh)
    } else {
        let dw = (w * 0.235).clamp(176.0, 214.0);
        ([right - dw, cy, dw, h], h)
    };
    let rail = [l, cy, rw, gh];
    let gx = l + rw + 8.0;
    let gw = if portrait { right - gx } else { detail[0] - 8.0 - gx };
    let grid = [gx, cy, gw, gh];
    // the heading over the tiles; the scroll bar's lane right of them
    let tiles = [gx + 8.0, cy + 30.0, gw - 22.0, gh - 36.0];
    let cols = (((tiles[2] + GAP) / (96.0 + GAP)).round() as usize).clamp(2, 8);
    let tw = (tiles[2] - GAP * (cols - 1) as f32) / cols as f32;
    let ph = tw / PIC_ASPECT;
    let pitch = ph + NAME_H + GAP;
    let rows = (((tiles[3] + GAP) / pitch) as usize).max(1);
    Lay { tabs_room, search, upload, queue, rail, grid, tiles, detail, cols, rows, tw, ph, pitch, foot: bottom + 5.0 }
}

impl Lay {
    /// The `k`th tile on view's box (its picture and its name under it).
    pub(super) fn cell(&self, k: usize) -> [f32; 4] {
        let [x, y, ..] = self.tiles;
        [x + (self.tw + GAP) * (k % self.cols) as f32, y + self.pitch * (k / self.cols) as f32, self.tw, self.ph + NAME_H]
    }
}

/// The rail's rows from its top: each group's title then its options (y, option index or None for a title).
fn rail_rows(v: &View) -> Vec<(f32, Option<usize>, usize)> {
    let mut out = Vec::new();
    let mut y = 8.0;
    let mut g = usize::MAX;
    for (i, o) in v.rail().iter().enumerate() {
        if o.group() != g {
            g = o.group();
            if i > 0 {
                y += 5.0;
            }
            out.push((y, None, g));
            y += TITLE_H;
        }
        out.push((y, Some(i), g));
        y += OPT_H;
    }
    out
}

/// How far the rail is scrolled: none, unless the cursor's option is past its foot.
fn rail_off(v: &View, rail: [f32; 4], on: Option<usize>) -> f32 {
    let rows = rail_rows(v);
    let end = rows.last().map_or(0.0, |r| r.0 + OPT_H + 8.0);
    let Some(y) = on.and_then(|i| rows.iter().find(|r| r.1 == Some(i))).map(|r| r.0) else { return 0.0 };
    (y + OPT_H + 30.0 - rail[3]).clamp(0.0, (end - rail[3]).max(0.0))
}

// ---------------------------------------------------------------- presses

impl Menu {
    /// Browse's move from the cursor: around the search box and Upload, the heading's words, the rail and the
    /// tiles' grid. Returns whether it moved.
    pub(super) fn browse_move(&mut self, k: Press) -> bool {
        let v = &self.add;
        let (nw, nr, n) = (v.words().len(), v.rail().len(), v.tiles().len());
        let cols = lay(&self.grid).cols;
        let (w0, r0, t0) = (2, 2 + nw, 2 + nw + nr);
        let sort_word = w0 + v.b.sort as usize;
        let back_to_grid = if n > 0 { t0 + v.b.tile_at.min(n - 1) } else { sort_word };
        let to = match v.slot(self.sel) {
            // B383: the header: search box, then Upload on its right; down into the heading's words under them
            Slot::Search if k.right => 1,
            Slot::Search if k.down => sort_word,
            Slot::Search if k.left => r0 + v.b.rail_at.min(nr - 1),
            Slot::Search => self.sel,
            Slot::Upload if k.left => 0,
            Slot::Upload if k.down => w0 + nw - 1,
            Slot::Upload => self.sel,
            Slot::Word(i) if k.left && i == 0 => r0 + v.b.rail_at.min(nr - 1),
            Slot::Word(_) if k.left => self.sel - 1,
            Slot::Word(i) if k.right && i + 1 < nw => self.sel + 1,
            Slot::Word(_) if k.up => 0,
            Slot::Word(_) if k.down && n > 0 => t0 + (v.top * cols).min(n - 1),
            Slot::Word(_) => self.sel,
            Slot::Rail(i) if k.up && i == 0 => 0,
            Slot::Rail(_) if k.up => self.sel - 1,
            Slot::Rail(i) if k.down && i + 1 < nr => self.sel + 1,
            Slot::Rail(_) if k.right => back_to_grid,
            Slot::Rail(_) => self.sel,
            Slot::Tile(i) if k.left && i % cols == 0 => r0 + v.b.rail_at.min(nr - 1),
            Slot::Tile(_) if k.left => self.sel - 1,
            Slot::Tile(i) if k.right && i % cols + 1 < cols && i + 1 < n => self.sel + 1,
            Slot::Tile(i) if k.up && i < cols => sort_word,
            Slot::Tile(_) if k.up => self.sel - cols,
            Slot::Tile(i) if k.down && i + cols < n => self.sel + cols,
            // the row under is short: its last tile
            Slot::Tile(i) if k.down && i / cols < (n - 1) / cols => t0 + n - 1,
            Slot::Tile(_) => self.sel,
        };
        // the grid and the rail remember where the cursor left them
        match v.slot(self.sel) {
            Slot::Tile(i) if !matches!(v.slot(to), Slot::Tile(_)) => self.add.b.tile_at = i,
            Slot::Rail(i) if !matches!(v.slot(to), Slot::Rail(_)) => self.add.b.rail_at = i,
            _ => {}
        }
        let moved = to != self.sel;
        self.sel = to;
        moved
    }

    /// Browse's ✕, △ and □ on the cursor's slot.
    pub(super) fn browse_press(&mut self, k: Press, out: &mut Vec<Effect>) {
        use Effect::Sound;
        let v = &self.add;
        match v.slot(self.sel) {
            // B383: ✕ types in place with the keyboard (the box doesn't take the keys before), else the keyboard on
            // screen (a pad, a phone's tap: the browser's own keyboard)
            Slot::Search if k.ok && !v.pad && !v.phone => {
                self.add.b.focus = !self.add.b.focus;
                out.push(Sound(0));
            }
            Slot::Search if k.ok => {
                self.add.b.osk = Some(0);
                out.push(Sound(0));
            }
            Slot::Search if k.page => {
                self.add.b.search.clear();
                out.push(Sound(1));
            }
            Slot::Upload if k.ok => self.mods_tab(2, out),
            Slot::Word(i) if k.ok => match v.words()[i] {
                "Refresh" if v.page == Page::Mine => self.mine_step(0, out),
                "Add mod" => self.installed_step(0, out),
                "Remove all" => self.installed_step(2, out),
                "Refresh" => {
                    if !self.add.busy {
                        self.add.busy = true;
                        self.add.status = "Reading the mod store...".into();
                        modstore::list(true);
                    }
                    out.push(Sound(2));
                }
                "Download all" => {
                    let b = v.by.clone().expect("an author's list");
                    self.download_all(&b.author, b.list, out);
                }
                "All mods" => self.mods_back(out),
                "Reload" => {
                    out.push(Sound(2));
                    if let Err(e) = super::reload(out) {
                        self.add.status = e;
                    }
                }
                _ => {
                    self.add.b.sort = [Sort::Newest, Sort::Az, Sort::Uploader][i];
                    self.add.top = 0;
                    self.add.b.tile_at = 0;
                    out.push(Sound(0));
                }
            },
            Slot::Rail(i) if k.ok => {
                let o = v.rail()[i].clone();
                let b = &mut self.add.b;
                match o {
                    Opt::Game(g) => b.game = g,
                    Opt::Style(s) => b.style = s,
                    Opt::Left(l) => b.left = l,
                    // B383: Installed and My uploads are pages of their own
                    Opt::Show(s) => return self.mods_show(s, out),
                }
                self.add.top = 0;
                self.add.b.tile_at = 0;
                out.push(Sound(0));
            }
            Slot::Tile(_) => {
                let Some(e) = v.hovered(self.sel).cloned() else { return };
                if k.card {
                    // △: every mod its uploader shared (B255's list)
                    if e.meta.author.is_empty() || self.add.by.is_some() {
                        self.add.status = if e.meta.author.is_empty() { "The store doesn't say who shared this one.".into() } else { format!("These are all {}'s mods.", e.meta.author) };
                        return out.push(Sound(1));
                    }
                    // (B383: from Installed or My uploads, in the store's grid)
                    let from = if self.add.page == Page::Browse { self.sel } else { 0 };
                    if self.add.page != Page::Browse {
                        self.mods_page(Page::Browse, 0);
                    }
                    self.add.narrow(Some(e.meta.author.clone()), from);
                    self.add.top = 0;
                    self.add.b.tile_at = 0;
                    self.sel = self.add.acts();
                    self.add.status = format!("{} mods by {}.", self.add.entries().len(), e.meta.author);
                    out.push(Sound(2));
                } else if k.page {
                    let on = self.add.wish_toggle(&e);
                    self.add.status = format!("{} {} your saved mods.", e.meta.name, if on { "added to" } else { "removed from" });
                    out.push(Sound(if on { 2 } else { 1 }));
                } else if k.ok && self.add.page == Page::Mine {
                    self.mine_step(self.sel, out);
                } else if k.ok && self.add.page == Page::Installed && self.add.state(&e) != Some("Update") {
                    self.installed_step(self.sel, out);
                } else if k.ok {
                    if self.add.busy && self.add.doing.is_empty() {
                        self.add.status = "Wait for the store to finish first.".into();
                    } else if self.add.state(&e) == Some("Installed") && !self.add.jobs.contains_key(&e.key()) {
                        self.open_child(&["Remove"], format!("{} is installed: remove it (Reload then takes it out of the select)?", e.meta.name), out);
                    } else if self.add.state(&e) == Some("Installed") {
                        self.add.status = format!("{} is installed already.", e.meta.name);
                    } else {
                        self.install(e, out);
                    }
                }
            }
            _ => {}
        }
    }

    /// Browse's Remove (✕ on an installed tile): takes the installed copy out.
    pub(super) fn browse_remove(&mut self, sel: usize, out: &mut Vec<Effect>) {
        let Some(dir) = self.add.hovered(sel).and_then(|e| self.add.dir_of(e)).map(super::bare) else { return };
        if let Some(i) = self.add.locals().iter().position(|m| super::bare(&m.dir) == dir) {
            self.remove(i, out);
        }
    }

    /// Keeps the cursor's tile on view (`top`, the first row shown) and notes which way the grid scrolled.
    pub(super) fn browse_scroll(&mut self) {
        let l = lay(&self.grid);
        let n = self.add.tiles().len();
        let v = &mut self.add;
        if let Slot::Tile(i) = v.slot(self.sel) {
            let r = i / l.cols;
            v.top = v.top.min(r).max((r + 1).saturating_sub(l.rows));
        }
        v.top = v.top.min(n.div_ceil(l.cols).saturating_sub(l.rows));
        if v.top != v.b.seen {
            v.b.down = v.top > v.b.seen;
            v.b.seen = v.top;
        }
    }

    /// The on-screen keyboard's press (it takes them all while up): arrows move, ✕ presses the key, □ deletes, ○
    /// closes.
    pub(super) fn osk_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        let Some(at) = self.add.b.osk else { return };
        let to = osk_move(at, k);
        if to != at {
            self.add.b.osk = Some(to);
            out.push(Effect::Sound(0));
        }
        let s = &mut self.add.b.search;
        if k.back {
            self.add.b.osk = None;
            out.push(Effect::Sound(1));
        } else if k.page {
            s.pop();
            out.push(Effect::Sound(0));
        } else if k.ok {
            match at {
                40 => s.push(' '),
                41 => _ = s.pop(),
                42 => s.clear(),
                43 => self.add.b.osk = None,
                k => s.push(KEYS[k / 10].chars().nth(k % 10).unwrap_or(' ')),
            }
            self.add.top = 0;
            self.add.b.tile_at = 0;
            out.push(Effect::Sound(if at == 43 { 2 } else { 0 }));
        }
    }
}

/// The keyboard's cursor moved: around each row; up and down between the rows (the wide keys a quarter each).
fn osk_move(at: usize, k: Press) -> usize {
    let (r, c) = if at >= 40 { (4, at - 40) } else { (at / 10, at % 10) };
    match (r, k) {
        (4, k) if k.left => 40 + (c + 3) % 4,
        (4, k) if k.right => 40 + (c + 1) % 4,
        (4, k) if k.up => 30 + (c * 10 + 5) / 4,
        (4, k) if k.down => (c * 10 + 5) / 4,
        (_, k) if k.left => r * 10 + (c + 9) % 10,
        (_, k) if k.right => r * 10 + (c + 1) % 10,
        (0, k) if k.up => 40 + c * 4 / 10,
        (3, k) if k.down => 40 + c * 4 / 10,
        (_, k) if k.up => at - 10,
        (_, k) if k.down => at + 10,
        _ => at,
    }
}

/// The search box's typing (desktop keys, the browser's phone keyboard): while the box was pressed (B383: never just
/// for the cursor being on it) or the on-screen keyboard is up, letters and digits go into the search, Backspace
/// deletes, Enter (Esc) closes the keyboard or stops typing; the menu's key bindings don't see them.
pub(super) fn typing(mut menu: ResMut<Menu>, mut keys: ResMut<ButtonInput<KeyCode>>, mut typed: MessageReader<bevy::input::keyboard::KeyboardInput>, mut was: Local<bool>) {
    use bevy::input::keyboard::Key;
    let v = &menu.add;
    let on = menu.screen == super::Screen::Mods && v.grid() && v.child.is_none() && (v.b.osk.is_some() || (v.b.focus && menu.sel == 0 && v.nophone.is_none()));
    let osk = v.b.osk.is_some();
    if osk != *was {
        *was = osk;
        // a phone's own keyboard: the page's tap target that opens it (`web.rs`)
        #[cfg(target_arch = "wasm32")]
        crate::web::code_entry(osk, true);
    }
    if !on {
        typed.clear();
        return;
    }
    let mut changed = false;
    for e in typed.read().filter(|e| e.state.is_pressed()) {
        match &e.logical_key {
            Key::Character(c) if c.chars().all(|c| !c.is_control()) => {
                menu.add.b.search.push_str(c);
                keys.clear_just_pressed(e.key_code);
                changed = true;
            }
            Key::Space => {
                menu.add.b.search.push(' ');
                keys.clear_just_pressed(e.key_code);
                changed = true;
            }
            Key::Backspace => {
                menu.add.b.search.pop();
                keys.clear_just_pressed(e.key_code);
                changed = true;
            }
            Key::Enter if osk => {
                menu.add.b.osk = None;
                keys.clear_just_pressed(e.key_code);
            }
            Key::Enter | Key::Escape if !osk => {
                menu.add.b.focus = false;
                keys.clear_just_pressed(e.key_code);
            }
            _ => {}
        }
    }
    #[cfg(target_arch = "wasm32")]
    for c in crate::web::typed().chars() {
        match c {
            '\u{8}' => _ = menu.add.b.search.pop(),
            '\n' => menu.add.b.osk = None,
            c => menu.add.b.search.push(c),
        }
        changed = true;
    }
    if changed {
        let v = &mut menu.add;
        v.b.search = v.b.search.chars().take(30).collect();
        v.top = 0;
        v.b.tile_at = 0;
    }
}

// ---------------------------------------------------------------- drawing

/// A little rotated bar (the ✓'s and the spinner's strokes).
fn bar(d: &mut Draw, [cx, cy]: [f32; 2], len: f32, thick: f32, angle: f32, rgb: [f32; 3], alpha: f32) {
    d.q(Tex::Solid, [0.0; 4], [cx - len / 2.0, cy - thick / 2.0, len, thick], rgb, alpha);
    if let Some(q) = d.out.last_mut() {
        q.angle = angle;
    }
}

/// A round blue disc with a white ✓ (verified).
fn tick_disc(d: &mut Draw, cx: f32, cy: f32, r: f32) {
    for (src, dst) in super::super::super::menu_art::pill_quads(0, [cx - r, cy - r, 2.0 * r, 2.0 * r]) {
        d.q(Tex::Pill, src, dst, BLUE, 128.0);
    }
    bar(d, [cx - r * 0.3, cy + r * 0.18], r * 0.6, r * 0.28, 0.8, WHITE, 128.0);
    bar(d, [cx + r * 0.18, cy - r * 0.05], r * 1.05, r * 0.28, -0.95, WHITE, 128.0);
}

/// A thin frame `t` wide just inside `[x, y, w, h]`.
fn outline(d: &mut Draw, [x, y, w, h]: [f32; 4], t: f32, rgb: [f32; 3]) {
    for r in [[x, y, w, t], [x, y + h - t, w, t], [x, y + t, t, h - 2.0 * t], [x + w - t, y + t, t, h - 2.0 * t]] {
        d.q(Tex::Solid, [0.0; 4], r, rgb, 128.0);
    }
}

/// A small pill with words.
fn chip(d: &mut Draw, [x, y]: [f32; 2], words: &str, size: f32, fill: [f32; 3], ink: [f32; 3]) -> f32 {
    let w = d.width(words, size) + size * 1.1;
    d.pill_at([x, y, w, size + 6.0], [INK, fill, fill], 128.0);
    d.label(words, x + w / 2.0, y + 3.0, size, w - 4.0, ink);
    w
}

/// An install's progress on its tile: the bar's share (None: no bar) and its words.
fn progress(v: &View, e: &Entry) -> Option<(Option<f32>, String, [f32; 3])> {
    let (tag, what) = v.jobs.get(&e.key())?;
    Some(match *tag {
        "Queued" => (None, "Queued".into(), LIGHT),
        "Downloading" if what.starts_with("Checking") => (Some(1.0), "Checking...".into(), GOLD),
        "Downloading" => {
            let pct = what.rsplit('(').next().and_then(|s| s.strip_suffix("%)")).and_then(|s| s.parse::<f32>().ok());
            match pct {
                Some(p) => (Some(p / 100.0), format!("{p:.0}% - downloading"), GOLD),
                None => (Some(0.0), "Downloading...".into(), GOLD),
            }
        }
        "Error" => (None, "Not installed".into(), RED),
        _ => return None,
    })
}

/// The installed copy's face on the select (a mod with no store picture shows it instead of its initial).
fn face(m: &Menu, e: &Entry) -> Option<(Tex, [f32; 4])> {
    m.add.dir_of(e).and_then(|dir| super::face_of(m, dir))
}

/// One tile: its picture (or the loading shimmer, or its initial), the verified disc and Installed / Update pill
/// on it, an install's progress along its foot; its name and line under it, gold while chosen.
fn tile(d: &mut Draw, m: &Menu, e: &Entry, [x, y, w, _]: [f32; 4], ph: f32, on: bool) {
    let v = &m.add;
    let mt = &e.meta;
    let pic = [x, y, w, ph];
    let fill = mt.style.map_or(TILE, |s| CARD_FILL[(s as usize).min(3)].map(|c| c * 0.6));
    // (B383: a mod installed by hand has none on the store: the select's drawing)
    let state = if e.rev.starts_with("local:") { Some(Pic::Missing) } else { v.b.pics.get(&mt.sha256).copied() };
    let shimmer = matches!(state, None | Some(Pic::Asked));
    let a = if shimmer { 96.0 + 24.0 * (v.b.tick as f32 * 0.12 + x * 0.02).sin() } else { 128.0 };
    d.q(Tex::Solid, [0.0; 4], pic, fill, a);
    match state {
        Some(Pic::Shown(slot, aspect)) => {
            // the whole picture, as big as it fits (never stretched)
            let (pw, phh) = if aspect > w / ph { (w, w / aspect) } else { (ph * aspect, ph) };
            d.q(Tex::Thumb(slot), [0.0, 0.0, 1.0, 1.0], [x + (w - pw) / 2.0, y + (ph - phh) / 2.0, pw, phh], WHITE, 128.0);
        }
        _ => match face(m, e).filter(|_| state == Some(Pic::Missing)) {
            Some(f) => super::super::face_frame(d, f, x + w / 2.0, y + ph / 2.0, ph * 0.8, WHITE),
            None => {
                let initial: String = mt.name.chars().take(1).collect::<String>().to_uppercase();
                d.centred(&initial, x + w / 2.0, y + ph / 2.0 - ph * 0.22, ph * 0.44, if shimmer { LIGHT } else { WHITE });
            }
        },
    }
    if e.verified {
        tick_disc(d, x + 9.0, y + 9.0, 6.0);
    }
    let state = v.state(e).or((v.page == Page::Installed).then_some("Installed"));
    if let Some(s) = state.filter(|_| !matches!(v.jobs.get(&e.key()).map(|j| j.0), Some("Downloading" | "Queued"))) {
        let (fill, ink) = if s == "Update" { (GOLD, INK) } else { (GREEN.map(|c| c * 1.6), WHITE) };
        let cw = d.width(s, 9.0) + 10.0;
        chip(d, [x + w - cw - 3.0, y + 3.0], s, 9.0, fill, ink);
    }
    if let Some((share, words, rgb)) = progress(v, e) {
        let sy = y + ph - 13.0;
        d.q(Tex::Solid, [0.0; 4], [x, sy, w, 13.0], [10.0; 3], 100.0);
        if let Some(k) = share {
            d.q(Tex::Solid, [0.0; 4], [x, sy + 10.5, w * k.clamp(0.0, 1.0), 2.5], rgb, 128.0);
        }
        d.label(&words, x + w / 2.0, sy + 1.5, 9.0, w - 6.0, if rgb == RED { [128.0, 70.0, 70.0] } else { WHITE });
    }
    if on {
        outline(d, pic, 3.0, GOLD);
    }
    // the name, the hand's letter at its right; the line under: the uploader when sorted by them, else the game
    let hand = mt.hand.map(|h| if h < 0 { "L" } else { "R" });
    let hw = if hand.is_some() { 10.0 } else { 0.0 };
    super::super::left(d, &mt.name, x + 1.0, y + ph + 2.0, 11.0, w - hw - 2.0, if on { GOLD } else { WHITE });
    if let Some(h) = hand {
        d.label(h, x + w - 4.0, y + ph + 3.0, 9.0, 9.0, LIGHT);
    }
    let sub = if v.b.sort == Sort::Uploader || mt.source.is_empty() { format!("by {}", super::who(&mt.author)) } else { mt.source.clone() };
    super::super::left(d, &sub, x + 1.0, y + ph + 13.5, 8.0, w - 2.0, LIGHT);
}

/// The downloads' pill: "2 downloading - 1 queued" by a spinner (nothing waiting: how many are installed).
fn queue_pill(d: &mut Draw, v: &View, [x, y, w, h]: [f32; 4]) {
    d.pill_at([x, y, w, h], [INK, if v.doing.is_empty() { LIGHT } else { GOLD }, PANEL], 128.0);
    let words = match (v.doing.len(), v.queue.len()) {
        (0, 0) => format!("{} installed", v.installed_len()),
        (n, 0) => format!("{n} downloading"),
        (n, q) => format!("{n} downloading - {q} queued"),
    };
    let busy = !v.doing.is_empty();
    let tx = if busy { x + 22.0 } else { x + 10.0 };
    super::super::left(d, &words, tx, y + (h - 11.0) / 2.0, 11.0, x + w - 8.0 - tx, WHITE);
    if busy {
        let (cx, cy) = (x + 12.0, y + h / 2.0);
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::FRAC_PI_4;
            let lit = (v.b.tick / 4 + i) % 8;
            bar(d, [cx + 4.5 * a.cos(), cy + 4.5 * a.sin()], 3.5, 1.6, a, GOLD, 30.0 + 12.0 * lit as f32);
        }
    }
}

/// The search box: a gold-ringed field with a magnifier, the words typed (or what it searches), "N found".
fn search_box(d: &mut Draw, v: &View, [x, y, w, h]: [f32; 4], on: bool, found: usize) {
    d.pill_at([x, y, w, h], [INK, if on { WHITE } else { GOLD }, [20.0, 18.0, 24.0]], 128.0);
    // the magnifier: a ring and its handle
    let (cx, cy) = (x + 13.0, y + h / 2.0 - 1.0);
    for (i, rgb) in [(0, GOLD), (1, [20.0, 18.0, 24.0])] {
        let r = 4.5 - 1.6 * i as f32;
        for (src, dst) in super::super::super::menu_art::pill_quads(0, [cx - r, cy - r, 2.0 * r, 2.0 * r]) {
            d.q(Tex::Pill, src, dst, rgb, 128.0);
        }
    }
    bar(d, [cx + 4.6, cy + 4.6], 4.5, 1.8, std::f32::consts::FRAC_PI_4, GOLD, 128.0);
    let tx = x + 24.0;
    let tail = if v.b.search.is_empty() { String::new() } else { format!("{found} found") };
    let fw = if tail.is_empty() { 0.0 } else { d.width(&tail, 10.0) + 8.0 };
    if v.b.search.is_empty() {
        super::super::left(d, "Search name, uploader, game", tx, y + (h - 11.0) / 2.0, 11.0, x + w - 10.0 - tx, LIGHT);
    } else {
        let caret = if on && v.b.tick / 30 % 2 == 0 { "_" } else { "" };
        super::super::left(d, &format!("{}{caret}", v.b.search), tx, y + (h - 12.0) / 2.0, 12.0, x + w - 10.0 - fw - tx, WHITE);
        super::super::left(d, &tail, x + w - fw - 4.0, y + (h - 10.0) / 2.0, 10.0, fw, GOLD);
    }
}

/// B383: the Upload button: gold while its page is up.
fn upload_button(d: &mut Draw, [x, y, w, h]: [f32; 4], up: bool, on: bool, phone: bool) {
    d.pill_at([x, y, w, h], [INK, if on { WHITE } else { GOLD }, if up { GOLD } else { PANEL }], 128.0);
    // an arrow up: its head and shaft
    let (ax, ink) = (x + 13.0, if up { INK } else { GOLD });
    bar(d, [ax - 2.2, y + h / 2.0 - 2.0], 6.0, 1.8, -std::f32::consts::FRAC_PI_4, ink, 128.0);
    bar(d, [ax + 2.2, y + h / 2.0 - 2.0], 6.0, 1.8, std::f32::consts::FRAC_PI_4, ink, 128.0);
    d.q(Tex::Solid, [0.0; 4], [ax - 0.9, y + h / 2.0 - 4.0, 1.8, 10.0], ink, 128.0);
    // (B208: greyed on a phone, which can't upload)
    super::super::left(d, "Upload", x + 22.0, y + (h - 12.0) / 2.0, 12.0, w - 28.0, if up { INK } else if phone { LIGHT } else { WHITE });
}

/// B383: the title band's one tab row, the kinds (L2 / R2 between them; a kind with no mods isn't shown), then
/// the search box (a grid's), Upload and the downloads' pill. Returns the cursor's box when it's up here.
pub(super) fn header(d: &mut Draw, m: &Menu, l: &Lay) -> Option<[f32; 4]> {
    let v = &m.add;
    let g = &m.grid;
    let x0 = d.width(super::super::TITLES[5].0, 30.0) + 88.0;
    super::mods_kind::row(d, [x0 + g.strip[0] - 12.0, g.strip[1] - 46.0, l.tabs_room - x0 - 8.0], v.kind, v.kind_counts(), &v.kinds_shown());
    // (a kind the store doesn't carry yet: nothing to search)
    let grid = v.grid() && !v.empty_kind();
    let slot = if grid { Some(v.slot(m.sel)) } else { None };
    let mut cursor = None;
    if grid {
        search_box(d, v, l.search, slot == Some(Slot::Search), v.tiles().len());
        d.hit(l.search, Act::Row(0, OK));
        if slot == Some(Slot::Search) {
            cursor = Some([l.search[0] + 12.0, l.search[1], l.search[2], l.search[3]]);
        }
    }
    let up = v.page == Page::Upload;
    upload_button(d, l.upload, up, slot == Some(Slot::Upload), v.phone);
    // (on Upload: a tap goes back to the grid)
    d.hit(l.upload, if up { Act::Tab(0) } else if grid { Act::Row(1, OK) } else { Act::Tab(2) });
    if slot == Some(Slot::Upload) {
        cursor = Some(l.upload);
    }
    queue_pill(d, v, l.queue);
    cursor
}

/// Browse's page, Installed's and My uploads' (the header too: the tabs' room depends on the search box's place).
pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let v = &m.add;
    let l = lay(&m.grid);
    let tiles = v.tiles();
    let slot = v.slot(m.sel);
    let hand = |d: &mut Draw, r: [f32; 4]| d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [r[0] - 18.0 + hand_x, r[1] + r[3] / 2.0 - 9.0, 36.0, 18.0], WHITE, 128.0);
    let mut cursor = header(d, m, &l);
    *ADD_AT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    // the rail
    d.box3(l.rail, 10.0, [Some(INK), Some(LIGHT), Some(PANEL)], 128.0);
    let opts = v.rail();
    let r0 = 2 + v.words().len();
    let on_rail = match slot {
        Slot::Rail(i) => Some(i),
        _ => None,
    };
    let off = rail_off(v, l.rail, on_rail);
    for (ry, i, g) in rail_rows(v) {
        let y = l.rail[1] + ry - off;
        if y < l.rail[1] + 2.0 || y + OPT_H > l.rail[1] + l.rail[3] - 2.0 {
            continue;
        }
        let Some(i) = i else {
            super::super::left(d, GROUPS[g], l.rail[0] + 8.0, y + 2.0, 10.0, l.rail[2] - 16.0, GOLD);
            continue;
        };
        let o = &opts[i];
        let chosen = v.chosen(g) == *o;
        let row = [l.rail[0] + 4.0, y, l.rail[2] - 8.0, OPT_H - 1.5];
        if chosen {
            d.box3(row, 5.0, [None, None, Some(GOLD)], 128.0);
        }
        let ink = if chosen { INK } else { WHITE };
        let count = v.count_of(o).to_string();
        let cw = d.width(&count, 9.0);
        super::super::left(d, &o.label(), row[0] + 5.0, y + 1.5, 10.5, row[2] - cw - 14.0, ink);
        super::super::left(d, &count, row[0] + row[2] - cw - 4.0, y + 2.5, 9.0, cw, if chosen { INK } else { LIGHT });
        d.hit(row, Act::Row(r0 + i, OK));
        if on_rail == Some(i) {
            outline(d, row, 1.5, if chosen { WHITE } else { GOLD });
            cursor = Some(row);
        }
    }
    // the grid's box, its heading and sort words
    d.box3(l.grid, 10.0, [Some(INK), Some(LIGHT), Some(PANEL)], 128.0);
    let title = match (&v.by, v.page) {
        (Some(b), _) => format!("Mods by {}", super::who(&b.author)),
        (_, Page::Installed) => "Installed".into(),
        (_, Page::Mine) => "My uploads".into(),
        _ => opts.iter().find(|o| o.group() == 0 && v.chosen(0) == **o).map_or("All games".into(), Opt::label),
    };
    let mut head = format!("{title} - {} mod{}", tiles.len(), if tiles.len() == 1 { "" } else { "s" });
    if !v.b.search.trim().is_empty() {
        head = format!("{head} match '{}'", v.b.search.trim());
    }
    let words = v.words();
    let ww: Vec<f32> = words.iter().map(|w| d.width(w, 10.5) + 14.0).collect();
    let mut wx = l.grid[0] + l.grid[2] - 8.0 - ww.iter().map(|w| w + 4.0).sum::<f32>();
    let hy = l.grid[1] + 7.0;
    super::super::left(d, &head, l.grid[0] + 10.0, hy + 1.0, 13.0, wx - l.grid[0] - 18.0, WHITE);
    for (i, (word, w)) in words.iter().zip(&ww).enumerate() {
        let r = [wx, hy, *w, 17.0];
        let sorted = i < 3 && v.b.sort as usize == i;
        let on = slot == Slot::Word(i);
        d.pill_at(r, [INK, if on { WHITE } else if sorted { GOLD } else { LIGHT }, if sorted { GOLD } else if i >= 3 { [70.0, 66.0, 76.0] } else { PANEL }], 128.0);
        // (B208: a phone can't pick a file)
        let ink = if sorted { INK } else if v.phone && *word == "Add mod" { LIGHT } else { WHITE };
        d.label(word, wx + w / 2.0, hy + 3.0, 10.5, w - 6.0, ink);
        d.hit(r, Act::Row(2 + i, OK));
        if *word == "Add mod" {
            *ADD_AT.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
        }
        if on {
            cursor = Some(r);
        }
        wx += w + 4.0;
    }
    // the tiles on view
    let t0 = v.browse_acts();
    let first = v.top * l.cols;
    for (k, e) in tiles.iter().enumerate().skip(first).take(l.cols * l.rows) {
        let r = l.cell(k - first);
        let on = slot == Slot::Tile(k);
        tile(d, m, e, r, l.ph, on);
        d.hit([r[0] - GAP / 2.0, r[1] - GAP / 2.0, r[2] + GAP, r[3] + GAP], Act::Tile(t0 + k));
        if on {
            cursor = Some([r[0], r[1], r[2], l.ph]);
        }
    }
    if tiles.is_empty() {
        let none = if v.page == Page::Mine && v.account.is_none() {
            "Sign in with Upload (top right) to see your uploads."
        } else if v.busy && (v.store.is_none() || v.page == Page::Mine) {
            if v.page == Page::Mine { "Reading your uploads..." } else { "Reading the mod store..." }
        } else if v.entries().is_empty() {
            match v.page {
                Page::Installed => "No mods installed yet: Add mod, or install one from Everything.",
                Page::Mine => "None of your mods are on the store.",
                _ => "Nobody has shared a mod yet.",
            }
        } else {
            "No mod matches: try another filter or search."
        };
        super::super::options::para(d, none, l.tiles[0] + 6.0, l.tiles[1] + 10.0, l.tiles[2] - 12.0, 13.0, 17.0, LIGHT);
    }
    let rows = tiles.len().div_ceil(l.cols);
    if rows > l.rows {
        // the scroll bar: a thin gold thumb
        let (tx, ty, th) = (l.grid[0] + l.grid[2] - 9.0, l.tiles[1], l.tiles[3]);
        d.box3([tx, ty, 3.0, th], 1.5, [None, None, Some([60.0, 56.0, 66.0])], 128.0);
        let (k, at) = (l.rows as f32 / rows as f32, v.top as f32 / rows as f32);
        d.box3([tx, ty + th * at, 3.0, (th * k).max(8.0)], 1.5, [None, None, Some(GOLD)], 128.0);
    }
    // the footer's line (the legend's on the right)
    let fw = (m.grid.strip[2] * 0.42).min(380.0);
    d.outlined(|d| super::super::left(d, &format!("{} - pictures load by themselves, cached for next time", super::mods_kind::online(v.kind)), m.grid.strip[0], l.foot, 9.0, fw, WHITE));
    detail(d, m, l.detail, hand_x);
    if let Some(r) = cursor.filter(|_| v.child.is_none() && v.b.osk.is_none()) {
        hand(d, r);
    }
    if v.b.osk.is_some() {
        keyboard(d, v, hand_x);
    }
}

/// The chosen mod's panel: its name and ✓ Verified, the game, uploader, version and size, its costumes and hand,
/// its full picture with its play style and stats card, a line on how it plays, then Install (Update, or Remove),
/// More by its uploader and Save. On the search box, a word or a filter: what it does.
fn detail(d: &mut Draw, m: &Menu, b: [f32; 4], hand_x: f32) {
    let v = &m.add;
    let [x, y, w, h] = b;
    d.box3(b, 12.0, [Some(INK), Some(GOLD), Some(PANEL)], 128.0);
    let (tx, tw) = (x + 12.0, w - 24.0);
    let Some(e) = v.hovered(m.sel) else {
        let what = match v.slot(m.sel) {
            Slot::Search => "Search".to_string(),
            Slot::Upload => "Upload".to_string(),
            Slot::Word(i) => v.words()[i].to_string(),
            Slot::Rail(i) => v.rail().get(i).map_or(String::new(), Opt::label),
            Slot::Tile(_) => String::new(),
        };
        super::super::left(d, &what, tx, y + 12.0, 20.0, tw, GOLD);
        super::super::options::para(d, &info(m), tx, y + 40.0, tw, 12.0, 16.0, WHITE);
        return;
    };
    let mt = &e.meta;
    let mut ty = y + 10.0;
    let vw = if e.verified { d.width("Verified", 10.0) + 22.0 } else { 0.0 };
    let ns = 18.0 * ((tw - vw) / d.width(&mt.name, 18.0).max(1.0)).min(1.0);
    super::super::left(d, &mt.name, tx, ty + (18.0 - ns) / 2.0, ns, tw - vw, GOLD);
    if e.verified {
        let vx = tx + d.width(&mt.name, ns) + 8.0;
        tick_disc(d, vx + 6.0, ty + 9.0, 6.0);
        super::super::left(d, "Verified", vx + 15.0, ty + 4.0, 10.0, vw, [80.0, 128.0, 128.0]);
    }
    ty += 22.0;
    let line = [
        (!mt.source.is_empty()).then(|| mt.source.clone()),
        (!mt.author.is_empty()).then(|| format!("by {}", mt.author)),
        Some(format!("v{}", if mt.version.is_empty() { "1" } else { &mt.version })),
        (mt.size > 0).then(|| mb(mt.size)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" - ");
    super::super::left(d, &line, tx, ty, 10.0, tw, LIGHT);
    ty += 15.0;
    let mut cx = tx;
    let pills: Vec<String> = [
        mt.costumes.map(|c| format!("{c} costume{}", if c == 1 { "" } else { "s" })),
        mt.hand.map(|h| if h < 0 { "Left-handed".into() } else { "Right-handed".into() }),
        v.b.wish.as_ref().filter(|w| w.contains(&e.key())).map(|_| "Saved".into()),
    ]
    .into_iter()
    .flatten()
    .collect();
    for p in &pills {
        cx += chip(d, [cx, ty], p, 9.0, [70.0, 66.0, 76.0], WHITE) + 4.0;
    }
    ty += 20.0;
    // the buttons at the foot, the picture and card filling what's between
    let bh = 22.0;
    let foot = y + h - 10.0 - 3.0 * (bh + 5.0);
    let blurb = blurb(e);
    let blurb_h = if blurb.is_empty() { 0.0 } else { 30.0 };
    let ph = (foot - blurb_h - 6.0 - ty).max(40.0);
    let pic = [tx, ty, tw, ph];
    d.q(Tex::Solid, [0.0; 4], pic, [20.0, 18.0, 24.0], 128.0);
    let stats = mt.style.map(usize::from).zip(mt.grades);
    // B384: the roster's stats card (Serve...Footwork, the original's grade letters) on the right of the picture when
    // there's room
    let card_w = if stats.is_some() && tw >= 100.0 && ph >= 70.0 { (tw * 0.44).min(110.0) } else { 0.0 };
    let shot = [tx, ty, tw - card_w, ph];
    if v.shot {
        // the full preview, the select's portrait shape (0.8 wide), as big as fits
        let aspect = v.shot_aspect.unwrap_or(super::super::previews::CARD_W);
        let pw = (shot[3] * aspect).min(shot[2]);
        let phh = pw / aspect;
        d.q(Tex::Shelf, [0.0, 0.0, 1.0, 1.0], [shot[0] + (shot[2] - pw) / 2.0, shot[1] + (shot[3] - phh) / 2.0, pw, phh], WHITE, 128.0);
    } else if let Some(Pic::Shown(slot, aspect)) = v.b.pics.get(&mt.sha256).copied().filter(|_| !e.rev.starts_with("local:")) {
        let (pw, phh) = if aspect > shot[2] / shot[3] { (shot[2], shot[2] / aspect) } else { (shot[3] * aspect, shot[3]) };
        d.q(Tex::Thumb(slot), [0.0, 0.0, 1.0, 1.0], [shot[0] + (shot[2] - pw) / 2.0, shot[1] + (shot[3] - phh) / 2.0, pw, phh], WHITE, 128.0);
    } else if let Some(f) = face(m, e) {
        super::super::face_frame(d, f, shot[0] + shot[2] / 2.0, shot[1] + shot[3] / 2.0, shot[3] * 0.8, WHITE);
    } else {
        d.centred("No picture", shot[0] + shot[2] / 2.0, ty + ph / 2.0 - 6.0, 11.0, LIGHT);
    }
    if let Some((_, grades)) = stats.filter(|_| card_w > 0.0) {
        let chh = ph.min(100.0);
        super::super::roster::stats(d, Some(grades), [tx + tw - card_w + 4.0, ty + (ph - chh) / 2.0, card_w - 4.0, chh]);
    } else if let Some(s) = mt.style {
        let sx = shot[0] + 4.0;
        let sw = chip(d, [sx, ty + ph - 18.0], STYLE_NAMES[(s as usize).min(3)], 9.0, PINK, WHITE);
        let _ = sw;
    }
    ty += ph + 6.0;
    if !blurb.is_empty() {
        super::super::options::para(d, &blurb, tx, ty, tw, 10.0, 13.0, WHITE);
    }
    // ✕ Install (Update / Remove), △ More by, □ Save
    let installed = v.state(e).or((v.page == Page::Installed).then_some("Installed"));
    // B383: Installed's own mods (one added by hand stays: the game didn't put it in), My uploads' take down
    let by_hand = v.page == Page::Installed && v.local(m.sel).is_some_and(|md| !super::ours(md));
    let first = match (v.jobs.get(&e.key()).map(|j| j.0), installed) {
        (Some("Downloading"), _) => "Downloading...".to_string(),
        (Some("Queued"), _) => "Queued".to_string(),
        _ if v.page == Page::Mine => if e.verified { "Ask to delete" } else { "Take down" }.to_string(),
        (_, Some("Installed")) if by_hand => "Added by hand".to_string(),
        (_, Some("Installed")) => "Installed - Remove".to_string(),
        (_, Some(_)) => format!("Update - {}", mb(mt.size)),
        _ => format!("Install - {}", mb(mt.size)),
    };
    let wished = v.b.wish.as_ref().is_some_and(|w| w.contains(&e.key()));
    let more = if mt.author.is_empty() || v.by.is_some() { "More by uploader".to_string() } else { format!("More by {}", mt.author) };
    let buttons: [(String, Press, bool); 3] = [
        (first, OK, true),
        (more, Press { card: true, ..Press::default() }, false),
        ((if wished { "Unsave" } else { "Save" }).to_string(), Press { page: true, ..Press::default() }, false),
    ];
    let _ = hand_x;
    for (k, (word, p, big)) in buttons.iter().enumerate() {
        let r = [tx, foot + 3.0 + k as f32 * (bh + 5.0), tw, bh];
        let red = installed == Some("Installed") || v.page == Page::Mine;
        let fill = if *big { if red { [80.0, 40.0, 40.0] } else { GOLD } } else { [70.0, 66.0, 76.0] };
        d.pill_at(r, [INK, if *big { WHITE } else { LIGHT }, fill], 128.0);
        // the button's own pad / key art, as the legend draws it
        let (style, i) = (super::super::legend::last_style(), 2 + k as u8);
        let gh = bh - 7.0;
        let gw = (gh * super::super::legend::glyph_aspect(style, i)).min(tw * 0.3);
        d.q(Tex::Glyph(style, i), [0.0; 4], [r[0] + 7.0, r[1] + 3.5, gw, gh], WHITE, 128.0);
        let ink = if *big && !red { INK } else { WHITE };
        d.label(word, r[0] + (tw + gw) / 2.0 + 4.0, r[1] + 5.0, 12.0, tw - gw - 22.0, ink);
        d.hit(r, Act::Press(*p));
    }
}

/// How a mod plays, from its record: "Plays like Kent: a Baseline Player."
fn blurb(e: &Entry) -> String {
    let mt = &e.meta;
    let like = mt.like.and_then(|c| CHAR_NAMES.get(c as usize));
    let style = mt.style.map(|s| STYLE_NAMES[(s as usize).min(3)]);
    match (like, style) {
        (Some(c), Some(s)) => format!("Plays like {c}: a {s}."),
        (Some(c), None) => format!("Plays like {c}."),
        (None, Some(s)) => format!("A {s}."),
        _ => String::new(),
    }
}

/// What the cursor's slot does (the description bar, and the panel off a tile).
pub(super) fn info(m: &Menu) -> String {
    let v = &m.add;
    match v.slot(m.sel) {
        Slot::Search => "Search the mods' names, uploaders and games: X, then type (a pad: the keyboard on screen).".into(),
        Slot::Upload if v.phone => "Not available in the browser on phones: use the desktop app.".into(),
        Slot::Upload => "Share your installed mods on the store: sign in to Hugging Face once, then upload.".into(),
        Slot::Word(i) => match v.words()[i] {
            "Newest" => "The newest uploads first.".into(),
            "A-Z" => "By name.".into(),
            "Uploader" => "By who shared them, each tile saying who.".into(),
            "Refresh" if v.page == Page::Mine => "Read your uploads from the store again.".into(),
            "Refresh" => "Read the store's list again.".into(),
            "Add mod" if v.phone => "Not available in the browser on phones: use the desktop app.".into(),
            "Add mod" => "Install a mod .zip from this computer (a character, court or umpire from the mods repo).".into(),
            "Remove all" => "Remove every mod the game installed (ones added by hand stay).".into(),
            "Reload" => "Restart the game with the mods as they are now: new ones join the select, removed ones leave it.".into(),
            "Download all" => format!("Install every mod by {} you don't have yet.", v.by.as_ref().map_or("", |b| super::who(&b.author))),
            _ => "Back to every mod in the store.".into(),
        },
        Slot::Rail(i) => match v.rail().get(i) {
            Some(Opt::Show(Show::Installed)) => format!("The mods on this machine ({}): remove them, add one from a .zip.", v.count_of(&Opt::Show(Show::Installed))),
            Some(Opt::Show(Show::Mine)) if v.account.is_none() => "The mods you shared on the store (sign in with Upload first).".into(),
            Some(Opt::Show(Show::Mine)) => "The mods you shared on the store: take one down.".into(),
            Some(o) => format!("Show {} ({} mods).", o.label().to_lowercase(), v.count_of(o)),
            None => String::new(),
        },
        Slot::Tile(_) => match v.hovered(m.sel) {
            Some(e) if v.jobs.contains_key(&e.key()) => v.jobs[&e.key()].1.clone(),
            Some(e) if v.page == Page::Mine && e.verified => format!("{} is on the store (merged): press to ask for it to be deleted.", e.meta.name),
            Some(e) if v.page == Page::Mine => format!("{} is live from your pull request: press to take it down.", e.meta.name),
            Some(e) if v.page == Page::Installed && v.state(e) != Some("Update") => match v.local(m.sel) {
                Some(md) if !super::ours(md) => format!("{} was added by hand: delete {} to remove it.", md.name, md.dir.display()),
                _ => format!("{} is installed: press to remove it.", e.meta.name),
            },
            Some(e) => match v.state(e) {
                Some("Installed") => format!("{} is installed: press to remove it.", e.meta.name),
                Some(_) => format!("Press to update {}.", e.meta.name),
                _ => format!("Press to install {}.", e.meta.name),
            },
            None => String::new(),
        },
    }
}

/// The on-screen keyboard over the dimmed page: the search typed so far, the keys, the cursor's gold.
fn keyboard(d: &mut Draw, v: &View, hand_x: f32) {
    use super::super::Anchor;
    d.popup(Some(press(|k| k.back = true)));
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 70.0);
    d.anchor = Anchor::Centre;
    let (kw, kh, g) = (34.0, 28.0, 4.0);
    let w = 10.0 * kw + 9.0 * g + 24.0;
    let (x, y) = (320.0 - w / 2.0, 110.0);
    d.box3([x, y, w, 50.0 + 5.0 * (kh + g) + 10.0], 14.0, [Some(INK), Some(GOLD), Some(PANEL)], 128.0);
    let field = [x + 12.0, y + 12.0, w - 24.0, 28.0];
    d.pill_at(field, [INK, GOLD, [20.0, 18.0, 24.0]], 128.0);
    super::super::left(d, &format!("{}_", v.b.search), field[0] + 12.0, field[1] + 7.0, 14.0, field[2] - 24.0, WHITE);
    let at = v.b.osk.unwrap_or(0);
    let ky = y + 50.0;
    let mut cursor = None;
    for (r, row) in KEYS.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            let k = r * 10 + c;
            let rect = [x + 12.0 + c as f32 * (kw + g), ky + r as f32 * (kh + g), kw, kh];
            d.box3(rect, 6.0, [Some(INK), Some(if k == at { WHITE } else { LIGHT }), Some(if k == at { GOLD } else { TILE })], 128.0);
            d.centred(&ch.to_uppercase().to_string(), rect[0] + kw / 2.0, rect[1] + 6.0, 16.0, if k == at { INK } else { WHITE });
            d.hit(rect, Act::Row(k, OK));
            if k == at {
                cursor = Some(rect);
            }
        }
    }
    let ww = (10.0 * kw + 9.0 * g - 3.0 * g) / 4.0;
    for (i, word) in WIDE.iter().enumerate() {
        let k = 40 + i;
        let rect = [x + 12.0 + i as f32 * (ww + g), ky + 4.0 * (kh + g), ww, kh];
        d.box3(rect, 6.0, [Some(INK), Some(if k == at { WHITE } else { LIGHT }), Some(if k == at { GOLD } else { [70.0, 66.0, 76.0] })], 128.0);
        d.label(word, rect[0] + ww / 2.0, rect[1] + 7.0, 13.0, ww - 8.0, if k == at { INK } else { WHITE });
        d.hit(rect, Act::Row(k, OK));
        if k == at {
            cursor = Some(rect);
        }
    }
    if let Some(r) = cursor {
        d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [r[0] - 18.0 + hand_x, r[1] + r[3] / 2.0 - 9.0, 36.0, 18.0], WHITE, 128.0);
    }
}

/// The legend's words on Browse.
pub(super) fn legend(m: &Menu) -> Vec<(super::super::legend::Act, &'static str)> {
    use super::super::legend::Act as L;
    let v = &m.add;
    if v.b.osk.is_some() {
        return vec![(L::Move, "Move"), (L::Ok, "Type"), (L::Page, "Delete"), (L::Back, "Close")];
    }
    let mut l = vec![(L::Move, "Move")];
    match v.slot(m.sel) {
        Slot::Search if v.b.focus => l.extend([(L::Type, "Type"), (L::Ok, "Done")]),
        Slot::Search if !v.pad && !v.phone => l.push((L::Ok, "Type")),
        Slot::Search => l.push((L::Ok, "Keyboard")),
        Slot::Upload => l.push((L::Ok, "Upload")),
        Slot::Tile(_) => {
            if let Some(e) = v.hovered(m.sel) {
                let state = v.state(e).or((v.page == Page::Installed).then_some("Installed"));
                l.push((L::Ok, match state {
                    _ if v.page == Page::Mine => "Take down",
                    Some("Installed") => "Remove",
                    Some(_) => "Update",
                    None => "Install",
                }));
                l.extend([(L::Card, "More by"), (L::Page, "Save")]);
            }
        }
        _ => l.push((L::Ok, "Choose")),
    }
    l.extend(v.kind_switch().into_iter().chain([(L::Back, "Back")]));
    l
}

// ---------------------------------------------------------------- pictures

/// The tiles' pictures: fetches what's on view and the next screen (at most `cap` at once; cached ones are read at
/// once), and drops loaded ones far off view past [`KEEP`]. Returns the fetches to start (the caller's, so the
/// tests can see them without the network).
pub(super) fn want(v: &mut View, l: &Lay, cap: usize) -> Vec<Entry> {
    let tiles: Vec<Entry> = v.tiles().into_iter().cloned().collect();
    let order = plan(tiles.len(), l.cols, v.top, l.rows, v.b.down);
    let asked = v.b.pics.values().filter(|p| **p == Pic::Asked).count();
    let mut go = Vec::new();
    for &i in &order {
        let e = &tiles[i];
        if e.meta.sha256.is_empty() || e.rev.starts_with("local:") || v.b.pics.contains_key(&e.meta.sha256) {
            continue;
        }
        if asked + go.len() >= cap {
            break;
        }
        v.b.pics.insert(e.meta.sha256.clone(), Pic::Asked);
        go.push(e.clone());
    }
    go
}

/// The loaded pictures to drop: past [`KEEP`], the ones not on view or the next screen.
pub(super) fn evict(v: &mut View, l: &Lay) -> Vec<u32> {
    let shown = v.b.pics.values().filter(|p| matches!(p, Pic::Shown(..))).count();
    if shown <= KEEP {
        return Vec::new();
    }
    let tiles = v.tiles();
    let near: HashSet<String> = plan(tiles.len(), l.cols, v.top, l.rows, v.b.down).into_iter().chain(plan(tiles.len(), l.cols, v.top, l.rows, !v.b.down)).map(|i| tiles[i].meta.sha256.clone()).collect();
    let mut free = Vec::new();
    let far: Vec<String> = v.b.pics.iter().filter(|(k, p)| matches!(p, Pic::Shown(..)) && !near.contains(*k)).map(|(k, _)| k.clone()).take(shown - KEEP).collect();
    for k in far {
        if let Some(Pic::Shown(slot, _)) = v.b.pics.remove(&k) {
            free.push(slot);
        }
    }
    free
}

/// A picture came back for the zip `sha`: shown in slot `slot` (its width over height), or none.
pub(super) fn got(v: &mut View, sha: String, pic: Option<(u32, f32)>) {
    v.b.pics.insert(sha, pic.map_or(Pic::Missing, |(s, a)| Pic::Shown(s, a)));
}

/// The installed mods' faces on the tiles on view (what `previews.rs` bakes first while Browse is up).
pub(super) fn faces(m: &Menu) -> Vec<super::super::Who> {
    let v = &m.add;
    let l = lay(&m.grid);
    v.tiles()
        .into_iter()
        .skip(v.top * l.cols)
        .take(l.cols * l.rows)
        .filter_map(|e| v.dir_of(e))
        .filter_map(|dir| super::mod_of(m, dir))
        .map(super::super::Who::Mod)
        .collect()
}

/// The cache folder of the tiles' pictures: `mod-pictures/` beside `mods/`.
pub(super) fn cache_dir() -> std::path::PathBuf {
    super::root().with_file_name("mod-pictures")
}
