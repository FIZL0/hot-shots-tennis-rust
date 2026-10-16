//! B372: Browse as a picture grid. B383: the top bar has one tab row, the kinds (`mods_kind::row`), then the search
//! box (✕ types into it from the keyboard, a pad's or a phone's opens an on-screen keyboard; it never takes the keys
//! by itself), the Upload button and the downloads' pill. B448: the page in the character select's look (`look.rs`,
//! `roster.rs`), in the chosen kind's colours (`mods_kind::seat_of`): the left column has a header row (where the
//! mods are, what they are, the last action's outcome), the Show pills (Browse, Installed, Updates, My uploads,
//! Saved; Installed and My uploads are those grids, the pages the old tab row had) with the sort and the one filter
//! pill on their right (Select picks Game, Plays like or Hand, ✕ / L1 / R1 its value), the tiles under a heading per
//! game (per uploader when sorted by them) and the scroll bar, the page's own words in the footer; the chosen mod's
//! column on the right (its name and uploader, costumes and hand, picture and stats card, facts, then Install / More
//! by / Save, or an open question's box and buttons). Upload is the same frame over the installed mods. The cursor
//! is one number (`Menu::sel`): the search box, Upload, the footer's words, the pills, then the tiles, so
//! `View::hovered` is the tile under it as on every other page.
//!
//! Pictures load by themselves: every tile on view, then one more screen the way the grid last scrolled, a few at a
//! time ([`plan`]); each is the store's small `thumb.png` (an older upload: its preview shrunk once), cached by the
//! zip's sha256 beside `mods/` (`modstore::thumb`), so a second visit reads the cache, not the network.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use super::super::look::{self, BAR, HEADING, LEGEND, MARGIN, PAD, PILL, RIM, ROW_GAP, TOP_BAR};
use super::super::osk::{self, Did};
use super::super::pointer::{Act, OK};
use super::super::roster::{self, PX, PY, ellipsis, hex, rounded};
use super::super::{CHAR_NAMES, Draw, Effect, Menu, Press, STYLE_NAMES, Tex, WHITE};
use super::{Entry, Kind, Mod, Page, RULE, UP_TOP, Up, View, mb, modstore, mods_kind};

/// The sorts, in the sort pill's order.
const SORTS: [&str; 3] = ["Newest", "A-Z", "Uploader"];
/// The filter pill's groups.
const GROUPS: [&str; 3] = ["Game", "Plays like", "Hand"];
/// The games the filter names (the most mods first); the rest are Other.
const GAMES: usize = 6;
/// The tile pictures kept loaded at most (the ones on view and the next screen are never dropped).
const KEEP: usize = 160;
/// Mockup px: a tile's least width and the gap between tiles, the least of the next row that peeks under the last
/// whole line.
const TILE: f32 = 96.0;
const TILE_GAP: f32 = 6.0;
const PEEK: f32 = 12.0;
/// The badges' colours (B471: small outlined pills): Update yellow, Installed light green, New light blue, Failed
/// light red; the verified line's green.
const BADGE_UPDATE: u32 = look::YELLOW;
const BADGE_INSTALLED: u32 = 0xa8e2b4;
const BADGE_NEW: u32 = 0xa9cbf6;
const BADGE_FAILED: u32 = 0xf6aaae;
const VERIFIED: u32 = 0x2f8a52;
/// The headings of mods from no named game, and of an uploader the store doesn't name.
const OTHER: &str = "Other games";
const UNKNOWN: &str = "Unknown uploader";

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

/// B383: Show's options, in the pills' order.
pub(super) const SHOWS: [Show; 5] = [Show::All, Show::Installed, Show::Updates, Show::Mine, Show::Wish];

/// B383: the footer's Add mod word as last drawn (the browser's file input follows it; `mods_screen::add_rect`).
pub(super) static ADD_AT: std::sync::Mutex<Option<[f32; 4]>> = std::sync::Mutex::new(None);

/// A Game filter: one game by its `source`, or every game the filter doesn't name.
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
    /// B448: the filter pill's group (0 Game, 1 Plays like, 2 Hand).
    pub filter: usize,
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
    /// The grid last scrolled down (its next screen is fetched below, else above), and the top line it was on.
    pub down: bool,
    pub seen: usize,
    /// The tile and pill the cursor left from (↑ out of the grid, ↓ back into it).
    pub tile_at: usize,
    pub pill_at: usize,
    /// The tile a tap chose: a tap on it again installs (a first tap only chooses).
    pub tapped: Option<usize>,
    /// Frames since the page opened (the loading tiles' shimmer, the downloads' spinner).
    pub tick: u32,
    /// B444: seconds the cursor has been on `Menu::sel` `.1` (a long title's slide starts over on every move).
    pub roll: (f32, usize),
    /// The made-up store of a `sample` screen: no picture is fetched (installed ones show the select's).
    pub offline: bool,
}

/// A filter's or Show's option.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Opt {
    Game(Option<Game>),
    Style(Option<u8>),
    Left(bool),
    Show(Show),
}

impl Opt {
    fn label(&self) -> String {
        match self {
            Opt::Game(None) => "All games".into(),
            Opt::Game(Some(Game::Src(s))) => s.clone(),
            Opt::Game(Some(Game::Other)) => "Other".into(),
            Opt::Style(None) => "Any".into(),
            Opt::Style(Some(s)) => ["All-round", "Baseline", "Net", "Big Server"][(*s as usize).min(3)].into(),
            Opt::Left(false) => "Either".into(),
            Opt::Left(true) => "Left-handed".into(),
            Opt::Show(s) => show_name(*s).into(),
        }
    }

    /// No filtering (the group's first option).
    fn any(&self) -> bool {
        matches!(self, Opt::Game(None) | Opt::Style(None) | Opt::Left(false) | Opt::Show(Show::All))
    }
}

/// B471: which of L1 / R1 do nothing (the first Show tab's L1, the last's R2): their hints dim.
pub(super) fn show_ends(v: &View) -> [bool; 2] {
    let at = SHOWS.iter().position(|s| *s == v.b.show).unwrap_or(0);
    [at == 0, at + 1 == SHOWS.len()]
}

fn show_name(s: Show) -> &'static str {
    match s {
        Show::All => "Browse",
        Show::Installed => "Installed",
        Show::Updates => "Updates",
        Show::Mine => "My uploads",
        Show::Wish => "Saved",
    }
}

/// B448: a pill in the column's pills row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Pill {
    Show(Show),
    Sort,
    Filter,
}

/// Where the cursor is on Browse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Slot {
    Search,
    /// B383: the top bar's Upload button.
    Upload,
    /// B448: the footer's words.
    Word(usize),
    Pill(usize),
    Tile(usize),
}

/// B448: a run of tiles under one heading: its name, its first tile and how many.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Sec {
    pub name: String,
    pub start: usize,
    pub len: usize,
}

/// B448: a line of the grid: a section's heading, or a row of tiles (`start..end`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Line {
    Head(usize),
    Row(usize, usize),
}

/// The grid's lines: each section's heading, then its tiles `cols` a row.
pub(super) fn lines(secs: &[Sec], cols: usize) -> Vec<Line> {
    let cols = cols.max(1);
    let mut l = Vec::new();
    for (k, s) in secs.iter().enumerate() {
        l.push(Line::Head(k));
        let mut i = s.start;
        while i < s.start + s.len {
            let e = (i + cols).min(s.start + s.len);
            l.push(Line::Row(i, e));
            i = e;
        }
    }
    l
}

/// The line tile `i` is on.
fn row_line(lines: &[Line], i: usize) -> Option<usize> {
    lines.iter().position(|l| matches!(*l, Line::Row(s, e) if (s..e).contains(&i)))
}

/// A move from tile `i`: ←→ the tile before or after (over the rows' and headings' ends), ↑↓ the row above or under
/// in the same column (its last on a short row). None: off the grid's edge.
fn tile_step(lines: &[Line], i: usize, k: Press) -> Option<usize> {
    let at = row_line(lines, i)?;
    let Line::Row(s, _) = lines[at] else { return None };
    let n = lines.iter().rev().find_map(|l| if let Line::Row(_, e) = *l { Some(e) } else { None }).unwrap_or(0);
    let col = i - s;
    let row = |l: &Line| if let Line::Row(s, e) = *l { Some((s + col).min(e - 1)) } else { None };
    if k.left {
        i.checked_sub(1)
    } else if k.right {
        (i + 1 < n).then_some(i + 1)
    } else if k.up {
        lines[..at].iter().rev().find_map(row)
    } else if k.down {
        lines[at + 1..].iter().find_map(row)
    } else {
        Some(i)
    }
}

impl View {
    /// B448: the footer's words: Refresh (an author's mods: Download all, All mods; Installed's Add mod, Reload and
    /// Remove all; My uploads' Refresh and Remove all uploads), then Reload while mods changed since the start (B387:
    /// the way back to the Reload a Later put off).
    pub(super) fn words(&self) -> Vec<&'static str> {
        let mut w: Vec<&'static str> = match (self.page, &self.by) {
            (Page::Installed, _) => vec!["Add mod", "Reload", "Remove all"],
            (_, Some(_)) => vec!["Download all", "All mods"],
            // B445
            (Page::Mine, _) => vec!["Refresh", "Remove all uploads"],
            _ => vec!["Refresh"],
        };
        if self.changed && self.page != Page::Installed {
            w.push("Reload");
        }
        w
    }

    /// B448: the pills row: Show's five, the sort, the filter.
    pub(super) fn pills(&self) -> Vec<Pill> {
        // M5b: the texture packs' list keeps its own order and has no games: no sort or filter
        let strip: &[Pill] = if self.packs_page() { &[] } else { &[Pill::Sort, Pill::Filter] };
        SHOWS.iter().map(|s| Pill::Show(*s)).chain(strip.iter().copied()).collect()
    }

    /// The games the filter names (of the kind tab's mods): the most mods first (at most [`GAMES`]; the chosen one
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

    /// The filter's groups on this kind tab (M8f9: play style and handedness are a character's).
    pub(super) fn groups(&self) -> usize {
        if self.kind == Kind::Character { 3 } else { 1 }
    }

    /// Filter group `g`'s options, the no-filter one first.
    pub(super) fn options(&self, g: usize) -> Vec<Opt> {
        match g {
            0 => {
                let (games, other) = self.games();
                let mut l = vec![Opt::Game(None)];
                l.extend(games.into_iter().map(|g| Opt::Game(Some(Game::Src(g)))));
                if other {
                    l.push(Opt::Game(Some(Game::Other)));
                }
                l
            }
            1 => [None, Some(0), Some(1), Some(2), Some(3)].map(Opt::Style).into(),
            _ => vec![Opt::Left(false), Opt::Left(true)],
        }
    }

    /// The option chosen in group `g` (3: Show).
    fn chosen(&self, g: usize) -> Opt {
        match g {
            0 => Opt::Game(self.b.game.clone()),
            1 => Opt::Style(self.b.style),
            2 => Opt::Left(self.b.left),
            _ => Opt::Show(self.b.show),
        }
    }

    /// Whether `e` is one option `o` leaves (`games`: the games the filter names).
    fn passes(&self, e: &Entry, o: &Opt, games: &[String]) -> bool {
        let m = &e.meta;
        match o {
            Opt::Game(None) | Opt::Style(None) | Opt::Left(false) | Opt::Show(Show::All) => true,
            Opt::Game(Some(Game::Src(s))) => m.source == *s,
            Opt::Game(Some(Game::Other)) => !games.contains(&m.source),
            // (a court or umpire has neither: those filters aren't on its tab)
            Opt::Style(Some(_)) | Opt::Left(true) if m.kind != Kind::Character => true,
            Opt::Style(Some(s)) => m.style == Some(*s),
            Opt::Left(true) => m.hand.is_some_and(|h| h < 0),
            // (B383: Installed and My uploads are lists of their own, `entries`')
            Opt::Show(Show::Installed | Show::Mine) => true,
            Opt::Show(Show::Updates) => self.state(e) == Some("Update"),
            Opt::Show(Show::Wish) => self.b.wish.as_ref().is_some_and(|w| w.contains(&e.key())),
        }
    }

    /// Show tab `s`'s mods of the chosen kind, before search and filters (None: its list isn't loaded yet).
    fn show_count(&self, s: Show) -> Option<usize> {
        let kind = |l: &[Entry]| l.iter().filter(|e| e.meta.kind == self.kind).count();
        let store = self.store.as_deref();
        match s {
            Show::All => store.map(kind),
            Show::Installed => Some(kind(&self.inst)),
            Show::Mine => self.mine.as_deref().map(kind),
            Show::Updates => store.map(|l| l.iter().filter(|e| e.meta.kind == self.kind && self.state(e) == Some("Update")).count()),
            Show::Wish => self.b.wish.as_ref().zip(store).map(|(w, l)| l.iter().filter(|e| e.meta.kind == self.kind && w.contains(&e.key())).count()),
        }
    }

    /// Whether `e` matches the search (its name, uploader or game) and every group's chosen option.
    fn keeps(&self, e: &Entry, games: &[String]) -> bool {
        // B373: the chosen kind's only
        e.meta.kind == self.kind && matches(e, &self.b.search) && (0..4).all(|g| self.passes(e, &self.chosen(g), games))
    }

    /// B448: the filter pill's words: its group and the option chosen in it, "+N" for the other groups filtering.
    pub(super) fn filter_words(&self) -> String {
        let g = self.b.filter.min(self.groups() - 1);
        let more = (0..self.groups()).filter(|&o| o != g && !self.chosen(o).any()).count();
        let s = format!("{}: {}", GROUPS[g], self.chosen(g).label());
        if more > 0 { format!("{s} +{more}") } else { s }
    }

    /// B448: the filter pill's group's next option (`back`: the one before), round.
    fn filter_step(&mut self, back: bool) {
        let g = self.b.filter.min(self.groups() - 1);
        let l = self.options(g);
        let n = l.len();
        let at = l.iter().position(|o| *o == self.chosen(g)).unwrap_or(0);
        match l[if back { (at + n - 1) % n } else { (at + 1) % n }].clone() {
            Opt::Game(x) => self.b.game = x,
            Opt::Style(x) => self.b.style = x,
            Opt::Left(x) => self.b.left = x,
            Opt::Show(_) => {}
        }
    }

    /// B448: the heading tile `e` goes under: its uploader when sorted by them, else its game.
    fn heading_of(&self, e: &Entry) -> String {
        let s = if self.b.sort == Sort::Uploader { &e.meta.author } else { &e.meta.source };
        if !s.is_empty() {
            s.clone()
        } else if self.b.sort == Sort::Uploader {
            UNKNOWN.into()
        } else {
            OTHER.into()
        }
    }

    /// The tiles: the store's mods the search and filters leave, sorted, then (B448) under a heading per game (per
    /// uploader when sorted by them): A-Z by heading when sorted A-Z (the unnamed last), else in the order their
    /// first mod comes.
    pub(super) fn tiles(&self) -> Vec<&Entry> {
        let games = self.games().0;
        let mut l: Vec<&Entry> = self.entries().iter().filter(|e| self.keeps(e, &games)).collect();
        // M5b: the texture packs in their order (the top wins), the sort left out
        if self.packs_page() {
            l.sort_by_key(|e| self.pack_of(e));
            return l;
        }
        match self.b.sort {
            // the newest pull request first; merged ones (no number) after, A-Z as the store lists them
            Sort::Newest => l.sort_by_key(|e| std::cmp::Reverse(modstore::pr_of(&e.rev).unwrap_or(0))),
            Sort::Az => l.sort_by_key(|e| e.meta.name.to_lowercase()),
            Sort::Uploader => l.sort_by_key(|e| (e.meta.author.is_empty(), e.meta.author.to_lowercase(), e.meta.name.to_lowercase())),
        }
        let mut keys: Vec<String> = Vec::new();
        for e in &l {
            let k = self.heading_of(e);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        if self.b.sort == Sort::Az {
            keys.sort_by_key(|k| (k == OTHER || k == UNKNOWN, k.to_lowercase()));
        }
        l.sort_by_cached_key(|e| {
            let k = self.heading_of(e);
            keys.iter().position(|x| *x == k)
        });
        l
    }

    /// B448: the grid's sections: the tiles' headings (Upload: the installed mods under one).
    pub(super) fn secs(&self) -> Vec<Sec> {
        if self.page == Page::Upload {
            let n = if self.empty_kind() { 0 } else { self.locals().len() };
            return if n == 0 { Vec::new() } else { vec![Sec { name: "On this machine".into(), start: 0, len: n }] };
        }
        if self.packs_page() {
            let n = self.items().len();
            return if n == 0 { Vec::new() } else { vec![Sec { name: "In order: the top pack wins".into(), start: 0, len: n }] };
        }
        let mut out: Vec<Sec> = Vec::new();
        for (i, e) in self.items().iter().enumerate() {
            let k = self.heading_of(e);
            match out.last_mut() {
                Some(s) if s.name == k => s.len += 1,
                _ => out.push(Sec { name: k, start: i, len: 1 }),
            }
        }
        out
    }

    /// The search box, Upload, the footer's words and the pills: the slots before the tiles.
    pub(super) fn browse_acts(&self) -> usize {
        2 + self.words().len() + self.pills().len()
    }

    pub(super) fn slot(&self, sel: usize) -> Slot {
        let (w, p) = (self.words().len(), self.pills().len());
        match sel {
            0 => Slot::Search,
            1 => Slot::Upload,
            s if s < 2 + w => Slot::Word(s - 2),
            s if s < 2 + w + p => Slot::Pill(s - 2 - w),
            s => Slot::Tile(s - 2 - w - p),
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

    /// B441: the on-screen keyboard is up (`osk`).
    pub(in super::super) fn osk_up(&self) -> bool {
        self.grid() && self.child.is_none() && self.b.osk.is_some()
    }

    /// B448: whether `e` is new: on the store, not reviewed yet, not installed.
    fn new_one(&self, e: &Entry) -> bool {
        !e.verified && !e.rev.starts_with("local:") && self.state(e).is_none() && self.page == Page::Browse
    }
}

const WISH: &str = "mods-wishlist.txt";

/// Whether `e`'s name, uploader or game has `q` in it (any case; blank matches all).
fn matches(e: &Entry, q: &str) -> bool {
    let q = q.trim().to_lowercase();
    q.is_empty() || [&e.meta.name, &e.meta.author, &e.meta.source].iter().any(|s| s.to_lowercase().contains(&q))
}

/// The tiles to fetch pictures for, in order: every one on the `fit` lines on view from line `top` (and the row
/// peeking under them), then one more screen the way the grid last scrolled (nearest first).
pub(super) fn plan(lines: &[Line], top: usize, fit: usize, down: bool) -> Vec<usize> {
    let tiles = |r: std::ops::Range<usize>| -> Vec<usize> { lines[r].iter().flat_map(|l| if let Line::Row(s, e) = *l { (s..e).collect() } else { Vec::new() }).collect() };
    let n = lines.len();
    let top = top.min(n);
    let end = (top + fit + 1).min(n);
    let mut l = tiles(top..end);
    // (at the top nothing is above: the screen below)
    if down || top == 0 {
        l.extend(tiles(end..(end + fit).min(n)));
    } else {
        let mut up = tiles(top.saturating_sub(fit)..top);
        up.reverse();
        l.extend(up);
    }
    l
}

// ---------------------------------------------------------------- layout

/// Where the page's parts go on the view `g` fits (`Grid::view`).
#[derive(Clone, Copy, Debug)]
pub(super) struct Lay {
    /// The zoom a portrait view draws with (the chosen mod's panel under the grid's there).
    pub zoom: look::Zoom,
    /// B471: the Show tabs' row: its left (None: right of the title tab, where `look::top` says), top and right end;
    /// the search box at the top bar's right (portrait: a row of its own under the tabs).
    pub shows: (Option<f32>, f32, f32),
    pub search: [f32; 4],
    /// The grid's panel, its header strip (`stack`: the kind tabs over the sort and filter, on two rows), the tiles'
    /// area (the scroll bar's lane right of it), the footer's row of pills (Upload at its right) and the room for its
    /// words (`note`: the footer's own row left of the pills, or a row over them on a narrow panel); the chosen
    /// mod's panel.
    pub card: [f32; 4],
    pub strip: [f32; 4],
    pub stack: bool,
    pub grid: [f32; 4],
    pub bar: [f32; 4],
    pub foot: [f32; 4],
    pub note: [f32; 4],
    pub upload: [f32; 4],
    pub detail: [f32; 4],
    /// Tiles a row, a tile's width and height (square on screen), the gaps across and down.
    pub cols: usize,
    pub tile: [f32; 2],
    pub gap: [f32; 2],
}

/// B471: the Upload button's width and a footer row of words on a narrow panel (mockup px).
const UPLOAD_W: f32 = 104.0;
const NOTE: f32 = 16.0;

pub(super) fn lay(g: &super::super::Grid) -> Lay {
    let portrait = g.detail[0] == g.strip[0];
    let zoom = look::Zoom::of(g.view, portrait);
    let view = zoom.inner(g.view);
    let [l, t, r, b] = view;
    let (x0, x1) = (l + MARGIN * PX, r - MARGIN * PX);
    let w = x1 - x0;
    let gap = look::GAP;
    let ph = PILL * PY;
    let (shows, search, cy) = if portrait {
        // the title tab fills the top bar: the tabs and the search box in rows under it
        let y = t + TOP_BAR * PY;
        ((Some(x0), y, x1), [x0, y + ph + ROW_GAP * PY, w, ph], y + 2.0 * (ph + ROW_GAP * PY))
    } else {
        let y = look::bar_y(t);
        let sw = (240.0 * PX).min(w * 0.24);
        ((None, y, x1 - sw - gap * PX), [x1 - sw, y, sw, ph], t + TOP_BAR * PY + 4.0 * PY)
    };
    let h = b - LEGEND - cy;
    let (card, detail) = if portrait {
        let ch = (h * 0.56).max(220.0 * PY);
        ([x0, cy, w, ch], [x0, cy + ch + gap * PY, w, h - ch - gap * PY])
    } else {
        let dw = (360.0 * PX).min(w * 0.42);
        ([x0, cy, w - dw - gap * PX, h], [x1 - dw, cy, dw, h])
    };
    let (ix, iw) = (card[0] + (RIM + PAD) * PX, card[2] - 2.0 * (RIM + PAD) * PX);
    let stack = card[2] < 640.0 * PX;
    let sh = if stack { look::STRIP + PILL + ROW_GAP } else { look::STRIP } * PY;
    let strip = [card[0] + RIM * PX, card[1] + RIM * PY, card[2] - 2.0 * RIM * PX, sh];
    let gy0 = strip[1] + sh + 2.5 * PY + PAD * PY;
    let foot = [ix, card[1] + card[3] - (RIM + PAD) * PY - ph, iw, ph];
    let upload = [ix + iw - UPLOAD_W * PX, foot[1], UPLOAD_W * PX, ph];
    // (a narrow panel: the footer's words get a row of their own over its pills)
    let note = if iw < 560.0 * PX { [ix, foot[1] - ROW_GAP * PY - NOTE * PY, iw, NOTE * PY] } else { foot };
    let gh = (note[1].min(foot[1]) - (ROW_GAP + 4.0) * PY - gy0).max(20.0 * PY);
    let (gx, gy) = (TILE_GAP * PX, TILE_GAP * PY);
    let gw = iw - (BAR + TILE_GAP) * PX;
    let cols = (((gw + gx) / (TILE * PX + gx)) as usize).max(2);
    let tw = ((gw - gx * (cols - 1) as f32) / cols as f32).max(1.0);
    let grid = [ix, gy0, gw, gh];
    let bar = [ix + iw - BAR * PX, gy0, BAR * PX, gh];
    Lay { zoom, shows, search, card, strip, stack, grid, bar, foot, note, upload, detail, cols, tile: [tw, tw * PY / PX], gap: [gx, gy] }
}

/// M5b: the page's layout: [`lay`], one row a line on the Texture packs list (`packs.rs`).
pub(super) fn lay_of(m: &Menu) -> Lay {
    let mut l = lay(&m.grid);
    if m.add.packs_page() {
        (l.cols, l.tile) = (1, [l.grid[2], super::packs::ROW * PY]);
        // no sort or filter pills: a narrow panel's strip keeps one line, the list gets the room
        if l.stack {
            let dy = (PILL + ROW_GAP) * PY;
            l.stack = false;
            l.strip[3] -= dy;
            for r in [&mut l.grid, &mut l.bar] {
                (r[1], r[3]) = (r[1] - dy, r[3] + dy);
            }
        }
    }
    l
}

impl Lay {
    /// A line's height (without the gap under it).
    fn line_h(&self, l: &Line) -> f32 {
        match l {
            Line::Head(_) => HEADING * PY,
            Line::Row(..) => self.tile[1],
        }
    }

    /// How many of `lines` from `top` the grid shows whole (at least one).
    pub(super) fn fit(&self, lines: &[Line], top: usize) -> usize {
        let room = self.grid[3] + self.gap[1] + 0.01;
        let mut used = 0.0;
        lines
            .iter()
            .skip(top)
            .take_while(|l| {
                used += self.line_h(l) + self.gap[1];
                used <= room
            })
            .count()
            .max(1)
    }

    /// The furthest the grid scrolls: the first line that shows the last.
    pub(super) fn last_top(&self, lines: &[Line]) -> usize {
        (0..lines.len()).find(|&t| t + self.fit(lines, t) >= lines.len()).unwrap_or(0)
    }

    /// The lines on view from `top`: each heading's section and box, each tile and its box (B426: the row peeking
    /// under the last whole line cut at the grid's foot).
    pub(super) fn laid(&self, lines: &[Line], top: usize) -> (Vec<(usize, [f32; 4])>, Vec<(usize, [f32; 4])>) {
        let [x, y0, w, h] = self.grid;
        let [tw, th] = self.tile;
        let mut y = y0;
        let (mut heads, mut tiles) = (Vec::new(), Vec::new());
        let end = (top + self.fit(lines, top)).min(lines.len());
        let row = |tiles: &mut Vec<(usize, [f32; 4])>, s: usize, e: usize, y: f32, h: f32| {
            for (j, i) in (s..e).enumerate() {
                tiles.push((i, [x + (tw + self.gap[0]) * j as f32, y, tw, h]));
            }
        };
        for l in &lines[top.min(end)..end] {
            match *l {
                Line::Head(k) => heads.push((k, [x, y, w, HEADING * PY])),
                Line::Row(s, e) => row(&mut tiles, s, e, y, th),
            }
            y += self.line_h(l) + self.gap[1];
        }
        let left = y0 + h - y;
        if let Some(&Line::Row(s, e)) = lines.get(end)
            && left >= PEEK * PY
        {
            row(&mut tiles, s, e, y, th.min(left));
        }
        (heads, tiles)
    }
}

// ---------------------------------------------------------------- presses

impl Menu {
    /// Browse's move from the cursor: the search box and Upload, the pills, the tiles' grid and the footer's words.
    /// Returns whether it moved.
    pub(super) fn browse_move(&mut self, k: Press) -> bool {
        let v = &self.add;
        let (nw, np, n) = (v.words().len(), v.pills().len(), v.tiles().len());
        let lines = lines(&v.secs(), lay_of(self).cols);
        let (w0, p0, t0) = (2, 2 + nw, 2 + nw + np);
        let tile = (n > 0).then(|| t0 + v.b.tile_at.min(n - 1));
        let ns = SHOWS.len();
        let shown = p0 + SHOWS.iter().position(|s| *s == v.b.show).unwrap_or(0);
        let strip = if np > ns { p0 + v.b.pill_at.clamp(ns, np - 1) } else { shown };
        let under = if nw > 0 { w0 } else { 1 };
        let to = match v.slot(self.sel) {
            // B471: the top bar: the Show tabs, then the search box on their right; ↓ to the sort and filter in the
            // panel's strip, ↓ again to the tiles
            Slot::Pill(i) if i < ns && k.left && i > 0 => self.sel - 1,
            Slot::Pill(i) if i < ns && k.right => if i + 1 < ns { self.sel + 1 } else { 0 },
            Slot::Pill(i) if i < ns && k.down => p0 + ns,
            Slot::Search if k.left => p0 + ns - 1,
            Slot::Search if k.down => if np > ns { p0 + np - 1 } else { tile.unwrap_or(under) },
            Slot::Pill(i) if k.left && i > ns => self.sel - 1,
            Slot::Pill(i) if k.right && i >= ns && i + 1 < np => self.sel + 1,
            Slot::Pill(i) if k.up && i >= ns && i + 1 == np => 0,
            Slot::Pill(_) if k.up => shown,
            Slot::Pill(i) if k.down && i >= ns => tile.unwrap_or(under),
            Slot::Tile(i) => match tile_step(&lines, i, k) {
                Some(j) => t0 + j,
                None if k.up => strip,
                None if k.down => under,
                None => self.sel,
            },
            // the footer: the page's words, then Upload at its right
            Slot::Word(i) if k.left && i > 0 => self.sel - 1,
            Slot::Word(i) if k.right => if i + 1 < nw { self.sel + 1 } else { 1 },
            Slot::Word(_) | Slot::Upload if k.up => tile.unwrap_or(strip),
            Slot::Upload if k.left && nw > 0 => w0 + nw - 1,
            _ => self.sel,
        };
        // the grid and the pills remember where the cursor left them
        match v.slot(self.sel) {
            Slot::Tile(i) if !matches!(v.slot(to), Slot::Tile(_)) => self.add.b.tile_at = i,
            Slot::Pill(i) if i >= SHOWS.len() && !matches!(v.slot(to), Slot::Pill(_)) => self.add.b.pill_at = i,
            _ => {}
        }
        let moved = to != self.sel;
        self.sel = to;
        moved
    }

    /// B448: Upload's move: Sign in and Upload all side by side over the installed mods' grid. Returns the slot
    /// it went to (the cursor stays where it is when nothing's there).
    pub(super) fn upload_move(&mut self, k: Press) -> usize {
        let v = &self.add;
        let n = v.locals().len();
        let lines = lines(&v.secs(), lay_of(self).cols);
        let to = match self.sel.checked_sub(UP_TOP) {
            None if k.left => 0,
            None if k.right => 1,
            None if k.down && n > 0 => UP_TOP + v.b.tile_at.min(n - 1),
            None => self.sel,
            Some(i) => match tile_step(&lines, i, k) {
                Some(j) => UP_TOP + j,
                None if k.up => v.b.pill_at.min(1),
                None => self.sel,
            },
        };
        match (self.sel.checked_sub(UP_TOP), to.checked_sub(UP_TOP)) {
            (Some(i), None) => self.add.b.tile_at = i,
            (None, Some(_)) => self.add.b.pill_at = self.sel,
            _ => {}
        }
        to
    }

    /// Browse's ✕, △ and □ on the cursor's slot (B448: L1 / R1 and Select on the sort and filter pills).
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
                "Remove all uploads" => self.wipe_ask(out),
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
                _ => {
                    out.push(Sound(2));
                    if let Err(e) = super::reload(self.add.app.0, out) {
                        self.add.status = e;
                    }
                }
            },
            Slot::Pill(i) => {
                // B471: ✕ steps on (a tap on the pill's left half back: `prev` with it)
                let step = k.ok;
                match v.pills()[i] {
                    // B383: Installed and My uploads are pages of their own
                    Pill::Show(s) if k.ok => return self.mods_show(s, out),
                    Pill::Sort if step => {
                        let at = self.add.b.sort as usize;
                        self.add.b.sort = [Sort::Newest, Sort::Az, Sort::Uploader][if k.prev { (at + 2) % 3 } else { (at + 1) % 3 }];
                    }
                    Pill::Filter if k.hand && self.add.groups() > 1 => self.add.b.filter = (self.add.b.filter + 1) % self.add.groups(),
                    Pill::Filter if step => self.add.filter_step(k.prev),
                    _ => return,
                }
                (self.add.top, self.add.b.tile_at) = (0, 0);
                out.push(Sound(0));
            }
            Slot::Tile(_) if v.packs_page() => self.pack_press(k, out),
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
                        self.open_child(&["Remove"], format!("Remove {}? Reload afterwards to take it off the select screens.", e.meta.name), out);
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

    /// Keeps the cursor's tile and its heading on view (`top`, the first line shown) and notes which way the grid
    /// scrolled (Browse's grids and Upload's alike).
    pub(super) fn browse_scroll(&mut self) {
        let l = lay_of(self);
        let v = &mut self.add;
        let lines = lines(&v.secs(), l.cols);
        if let Some(i) = self.sel.checked_sub(v.acts())
            && let Some(at) = row_line(&lines, i)
        {
            let head = if at > 0 && matches!(lines[at - 1], Line::Head(_)) { at - 1 } else { at };
            v.top = v.top.min(head);
            while v.top < at && v.top + l.fit(&lines, v.top) <= at {
                v.top += 1;
            }
        }
        v.top = v.top.min(l.last_top(&lines));
        if v.top != v.b.seen {
            v.b.down = v.top > v.b.seen;
            v.b.seen = v.top;
        }
    }

    /// The on-screen keyboard's press (it takes them all while up, B441: `osk`): arrows move, ✕ presses the key, □
    /// deletes, ○ closes.
    pub(super) fn osk_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        let Some(mut at) = self.add.b.osk else { return };
        let did = osk::step(&mut at, k, out);
        self.add.b.osk = Some(at);
        let s = &mut self.add.b.search;
        match did {
            Did::Nothing => return,
            Did::Close | Did::Done => self.add.b.osk = None,
            Did::Type(c) => s.push(c),
            Did::Space => s.push(' '),
            Did::Delete => _ = s.pop(),
            Did::Clear => s.clear(),
        }
        if k.ok {
            self.add.top = 0;
            self.add.b.tile_at = 0;
        }
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
            // B441: the iPhone's system keyboard's Return comes as a character
            Key::Character(c) if osk && c.contains(['\n', '\r']) => {
                menu.add.b.osk = None;
            }
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

/// A little rotated bar (the ✓'s, the magnifier's and the spinner's strokes).
pub(super) fn bar(d: &mut Draw, [cx, cy]: [f32; 2], len: f32, thick: f32, angle: f32, rgb: [f32; 3], alpha: f32) {
    d.q(Tex::Solid, [0.0; 4], [cx - len / 2.0, cy - thick / 2.0, len, thick], rgb, alpha);
    if let Some(q) = d.out.last_mut() {
        q.angle = angle;
    }
}

/// A round disc `rgb` with a white ✓ (verified).
fn tick_disc(d: &mut Draw, cx: f32, cy: f32, r: f32, rgb: [f32; 3]) {
    rounded(d, [cx - r, cy - r, 2.0 * r, 2.0 * r], r / PX, rgb, 128.0);
    bar(d, [cx - r * 0.3, cy + r * 0.18], r * 0.6, r * 0.28, 0.8, WHITE, 128.0);
    bar(d, [cx + r * 0.18, cy - r * 0.05], r * 1.05, r * 0.28, -0.95, WHITE, 128.0);
}

/// An install's progress on its tile: the bar's share (None: no bar) and its words.
fn progress(v: &View, e: &Entry) -> Option<(Option<f32>, String)> {
    let (tag, what) = v.jobs.get(&e.key())?;
    Some(match *tag {
        "Queued" => (None, "Queued".into()),
        "Downloading" if what.starts_with("Checking") => (Some(1.0), "Checking...".into()),
        "Downloading" => {
            let pct = what.rsplit('(').next().and_then(|s| s.strip_suffix("%)")).and_then(|s| s.parse::<f32>().ok());
            match pct {
                Some(p) => (Some(p / 100.0), format!("{p:.0}%")),
                None => (Some(0.0), "Downloading...".into()),
            }
        }
        "Error" => (None, "Not installed".into()),
        _ => return None,
    })
}

/// The installed copy's face on the select (a mod with no store picture shows it instead of its initial).
fn face(m: &Menu, e: &Entry) -> Option<(Tex, [f32; 4])> {
    m.add.dir_of(e).and_then(|dir| super::face_of(m, dir))
}

/// An installed court mod's value on the court select (past the disc courts and Random): its tile and column show
/// the select's picture of it (its own `previews/court.png`, else its layout court's).
fn court_at(m: &Menu, dir: &std::path::Path) -> Option<usize> {
    // the menu's dirs keep `./mods`'s `./`, the Mods pages' drop it
    let bare = |p: &std::path::Path| p.strip_prefix(".").unwrap_or(p).to_path_buf();
    m.court_mods.iter().position(|x| bare(&x.dir) == bare(dir)).map(|i| super::super::COURTS + 1 + i)
}

fn court(m: &Menu, e: &Entry) -> Option<usize> {
    court_at(m, m.add.dir_of(e)?)
}

/// What a tile shows: the store's picture (or its loading shimmer), an installed court's select picture, an installed
/// mod's face, or its initial, in `[x, y, w, h]`.
enum Face {
    Thumb(u32, f32),
    Court(usize),
    Select((Tex, [f32; 4])),
    Initial(String, bool),
}

fn draw_face(d: &mut Draw, m: &Menu, f: &Face, [x, y, w, h]: [f32; 4], a: [f32; 3]) {
    match f {
        // the whole picture, as big as it fits (never stretched)
        Face::Thumb(slot, aspect) => {
            let (pw, ph) = if *aspect > w / h { (w, w / aspect) } else { (h * aspect, h) };
            d.q(Tex::Thumb(*slot), [0.0, 0.0, 1.0, 1.0], [x + (w - pw) / 2.0, y + (h - ph) / 2.0, pw, ph], WHITE, 128.0);
        }
        Face::Court(c) => super::super::court_picture(d, m, *c, [x, y, w, h]),
        Face::Select(f) => super::super::face_frame(d, *f, x + w / 2.0, y + h / 2.0, h.min(w * PY / PX) * 0.9, WHITE),
        Face::Initial(c, dim) => d.centred(c, x + w / 2.0, y + h / 2.0 - h * 0.2, h * 0.4, if *dim { look::c(look::RULE) } else { a }),
    }
}

fn initial(name: &str, dim: bool) -> Face {
    Face::Initial(name.chars().take(1).collect::<String>().to_uppercase(), dim)
}

/// A store tile's face, and whether it's still loading.
fn entry_face(m: &Menu, e: &Entry) -> (Face, bool) {
    // (B383: a mod installed by hand has none on the store: the select's drawing)
    let state = if e.rev.starts_with("local:") { Some(Pic::Missing) } else { m.add.b.pics.get(&e.meta.sha256).copied() };
    let loading = matches!(state, None | Some(Pic::Asked));
    let f = match state {
        Some(Pic::Shown(slot, aspect)) => Face::Thumb(slot, aspect),
        // (an installed court: its own picture while the store's loads, the same court.png)
        _ if court(m, e).is_some() => Face::Court(court(m, e).unwrap_or_default()),
        _ => match face(m, e).filter(|_| state == Some(Pic::Missing)) {
            Some(f) => Face::Select(f),
            None => initial(&e.meta.name, loading),
        },
    };
    (f, loading)
}

/// An installed mod's face (Upload's tiles).
fn mod_face(m: &Menu, md: &Mod) -> Face {
    if let Some(c) = court_at(m, &md.dir) {
        return Face::Court(c);
    }
    super::face_of(m, &md.dir).map_or_else(|| initial(&md.name, false), Face::Select)
}

/// One tile `[x, y, w, h]` (`look::tile`: the cursor's white in a dark outline and a ring in the kind's accent `a`),
/// its picture, its name across its foot, the verified ✓ at its top left, `badges` at its top right, an install's
/// `progress` along its foot.
#[allow(clippy::too_many_arguments)]
fn tile(d: &mut Draw, m: &Menu, a: [f32; 3], rect @ [x, y, w, h]: [f32; 4], on: bool, f: &Face, loading: bool, name: &str, verified: bool, badges: &[(&str, [f32; 3])], prog: Option<(Option<f32>, String)>) {
    look::tile(d, rect, a, on);
    let inset = if on { 7.0 } else { 5.0 };
    let pic = [x + inset * PX, y + inset * PY, w - 2.0 * inset * PX, h - 2.0 * inset * PY - 16.0 * PY];
    if loading {
        let k = 60.0 + 30.0 * (m.add.b.tick as f32 * 0.12 + x * 0.02).sin();
        rounded(d, pic, 6.0, look::c(look::RULE), k);
    }
    draw_face(d, m, f, pic, a);
    // the name across the foot
    let size = 11.5 * PY;
    let ny = y + h - inset * PY - 14.0 * PY;
    let roll = on.then_some(m.add.b.roll.0);
    marquee(d, name, [x + inset * PX + 2.0 * PX, ny], size, w - 2.0 * inset * PX - 4.0 * PX, look::c(look::INK), roll);
    if verified {
        let r = 7.0 * PY;
        tick_disc(d, x + inset * PX + r, y + inset * PY + r, r, hex(0x2a5bc4));
    }
    let mut bx = x + w - inset * PX;
    for (b, rgb) in badges {
        bx -= look::badge(d, b, [bx, y + inset * PY], *rgb) + 3.0 * PX;
    }
    if let Some((share, words)) = prog {
        let sy = pic[1] + pic[3] - 16.0 * PY;
        rounded(d, [pic[0], sy, pic[2], 16.0 * PY], 4.0, look::c(look::INK), 110.0);
        if let Some(k) = share {
            rounded(d, [pic[0], sy + 12.0 * PY, pic[2] * k.clamp(0.02, 1.0), 4.0 * PY], 2.0, a, 128.0);
        }
        d.label(&words, pic[0] + pic[2] / 2.0, sy + 1.5 * PY, 10.0 * PY, pic[2] - 6.0 * PX, WHITE);
    }
}

/// B444: still this long at each end of a long title's slide, which goes this fast (px a second).
const ROLL_REST: f32 = 1.2;
const ROLL_SPEED: f32 = 30.0;

/// B444: `s` from `[x, y]` at `size` in `max` px. Too long for it: with `roll` (seconds it has been chosen) it slides
/// left until its end shows, rests, and starts over (clipped to `max`, its height kept); without, it ends in "...".
fn marquee(d: &mut Draw, s: &str, [x, y]: [f32; 2], size: f32, max: f32, rgb: [f32; 3], roll: Option<f32>) {
    let w = d.width(s, size);
    if w <= max {
        d.text(s, x, y, size, rgb);
        return;
    }
    let Some(t) = roll else {
        let cut = ellipsis(d, s, size, max);
        d.text(&cut, x, y, size, rgb);
        return;
    };
    let kept = rolled(d, s, x, y, size, max, rgb, roll_offset(w - max, t));
    d.out.extend(kept);
}

/// How far left a title `over` px too long has slid `t` seconds after it was chosen.
fn roll_offset(over: f32, t: f32) -> f32 {
    let slide = over / ROLL_SPEED;
    ((t % (2.0 * ROLL_REST + slide)) - ROLL_REST).clamp(0.0, slide) * ROLL_SPEED
}

/// `s`'s glyphs from `x - off`, cut to `[x, x + max]` (a glyph across an edge loses the part past it).
#[allow(clippy::too_many_arguments)]
fn rolled(d: &mut Draw, s: &str, x: f32, y: f32, size: f32, max: f32, rgb: [f32; 3], off: f32) -> Vec<super::super::Quad> {
    let start = d.out.len();
    d.text(s, x - off, y, size, rgb);
    let (x0, x1) = (x, x + max);
    d.out
        .drain(start..)
        .filter_map(|mut q| {
            let [qx, _, qw, _] = q.dst;
            let (a, b) = ((x0 - qx).max(0.0), (qx + qw - x1).max(0.0));
            if a + b >= qw {
                return None;
            }
            q.src[0] += q.src[2] * a / qw;
            q.src[2] *= (qw - a - b) / qw;
            q.dst[0] += a;
            q.dst[2] -= a + b;
            Some(q)
        })
        .collect()
}

/// B444: counts the seconds the cursor has stayed put, for [`marquee`].
pub(super) fn roll_time(mut menu: ResMut<Menu>, time: Res<Time>) {
    let sel = menu.sel;
    let r = &mut menu.add.b.roll;
    *r = if r.1 == sel { (r.0 + time.delta_secs(), sel) } else { (0.0, sel) };
}

/// B471: the footer's status: the last action's outcome (a mod list's uploader while there's none), else "2
/// downloading - 1 queued" (nothing waiting: how many are installed); whether downloads are running (a spinner).
fn status(m: &Menu) -> (String, bool) {
    let v = &m.add;
    let said = !v.status.is_empty() && v.child.is_none() && !(v.page == Page::Upload && v.status == RULE);
    if said {
        return (v.status.clone(), false);
    }
    if let Some(b) = v.by.as_ref().filter(|_| v.doing.is_empty() && v.queue.is_empty()) {
        return (format!("{} by {}", mods_kind::name(v.kind), super::who(&b.author)), false);
    }
    let words = match (v.doing.len(), v.queue.len()) {
        (0, 0) => format!("{} installed", v.installed_len()),
        (n, 0) => format!("{n} downloading"),
        (n, q) => format!("{n} downloading - {q} queued"),
    };
    (words, !v.doing.is_empty())
}

/// The downloads' spinner centred on `[cx, cy]`, its strokes in `a`.
fn spinner(d: &mut Draw, v: &View, [cx, cy]: [f32; 2], a: [f32; 3]) {
    for i in 0..8 {
        let ang = i as f32 * std::f32::consts::FRAC_PI_4;
        let lit = (v.b.tick / 4 + i) % 8;
        bar(d, [cx + 5.0 * PX * ang.cos(), cy + 5.0 * PY * ang.sin()], 4.0 * PX, 1.8 * PY, ang, a, 30.0 + 12.0 * lit as f32);
    }
}

/// The search box: a white pill in a dark outline (yellow with the cursor on it), a magnifier, the words typed (or
/// what it searches), "N found".
fn search_box(d: &mut Draw, v: &View, b @ [x, y, w, h]: [f32; 4], on: bool, found: usize) {
    let fill = if on { look::c(look::YELLOW) } else { WHITE };
    look::outlined_pill(d, b, fill, 2.0);
    let (ink, soft) = (look::c(look::INK), look::c(look::MUTED));
    // the magnifier: a ring and its handle
    let (cx, cy) = (x + 15.0 * PX, y + h / 2.0 - 1.0 * PY);
    let r = 5.0 * PY;
    roster::ringed(d, [cx - r, cy - r, 2.0 * r, 2.0 * r], r / PY, 1.6, ink, fill);
    bar(d, [cx + r * 1.0, cy + r * 1.0], 5.0 * PX, 1.8 * PY, std::f32::consts::FRAC_PI_4, ink, 128.0);
    let tx = x + 28.0 * PX;
    let size = 13.0 * PY;
    let ty = y + (h - size) / 2.0;
    if v.b.search.is_empty() {
        let words = ellipsis(d, "Search name, uploader, game", size, x + w - 12.0 * PX - tx);
        d.text(&words, tx, ty, size, soft);
    } else {
        let tail = format!("{found} found");
        let fw = d.width(&tail, 11.0 * PY) + 12.0 * PX;
        let caret = if on && v.b.tick / 30 % 2 == 0 { "_" } else { "" };
        let words = ellipsis(d, &format!("{}{caret}", v.b.search), size, x + w - 10.0 * PX - fw - tx);
        d.text(&words, tx, ty, size, ink);
        d.text(&tail, x + w - fw, y + (h - 11.0 * PY) / 2.0, 11.0 * PY, soft);
    }
}

/// B383: the Upload button: an arrow up and "Upload" on the main menu's grey (yellow with the cursor on it; white
/// while its page is up; B208: faded on a phone, which can't upload).
fn upload_button(d: &mut Draw, b @ [x, y, w, h]: [f32; 4], up: bool, on: bool, phone: bool) {
    let n0 = d.out.len();
    look::outlined_pill(d, b, look::c(if on { look::YELLOW } else if up { look::PAPER } else { look::GREY }), 2.0);
    let ink = if on || up { look::c(look::INK) } else { WHITE };
    let size = 14.0 * PY;
    let tw = d.width("Upload", size);
    let ax = x + (w - tw - 16.0 * PX) / 2.0 + 4.0 * PX;
    let cy = y + h / 2.0;
    bar(d, [ax - 2.4 * PX, cy - 2.0 * PY], 6.5 * PX, 1.8 * PY, -std::f32::consts::FRAC_PI_4, ink, 128.0);
    bar(d, [ax + 2.4 * PX, cy - 2.0 * PY], 6.5 * PX, 1.8 * PY, std::f32::consts::FRAC_PI_4, ink, 128.0);
    d.q(Tex::Solid, [0.0; 4], [ax - 0.9 * PX, cy - 4.5 * PY, 1.8 * PX, 10.5 * PY], ink, 128.0);
    d.text("Upload", ax + 12.0 * PX, y + (h - size) / 2.0, size, ink);
    if phone {
        d.out[n0..].iter_mut().for_each(|q| q.alpha *= 0.5);
    }
}

/// B448: the whole page: the wall, the title band and its "Mods" tab, the Show tabs and the search box in the top
/// bar, then the grid's panel and the chosen mod's (Browse, Installed, My uploads and Upload alike).
pub(super) fn layout(d: &mut Draw, m: &Menu, _hand_x: f32) {
    let v = &m.add;
    let l = lay_of(m);
    let a = look::accent(mods_kind::seat_of(v.kind));
    *ADD_AT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    look::backdrop(d);
    // B471: the band in Characters' red whatever the kind (the panels' strips carry the kind's colour)
    let tab_end = look::top(d, m.grid.view, "Mods", look::accent(0));
    let n0 = d.out.len();
    page(d, m, a, &l, tab_end);
    l.zoom.apply(d, n0);
    let mut add = ADD_AT.lock().unwrap_or_else(|e| e.into_inner());
    *add = add.map(|r| l.zoom.at(r));
    drop(add);
    if v.b.osk.is_some() && v.child.is_none() {
        keyboard(d, v);
    }
    if let Some(r) = v.nophone {
        super::nophone(d, if r == 0 { "Add mod" } else { "Upload my mod" });
    }
    if let Some(a) = v.wipe {
        super::wipe_popup(d, v, a, 0.0);
    }
}

/// The page on the (zoomed) view: everything but the title band, the keyboard and the popups.
fn page(d: &mut Draw, m: &Menu, a: [f32; 3], l: &Lay, tab_end: f32) {
    let v = &m.add;
    // (a kind the store doesn't carry yet: nothing to show or search)
    let grid = v.grid() && !v.empty_kind();
    let slot = grid.then(|| v.slot(m.sel));
    if !v.empty_kind() {
        let (left, ty, right) = l.shows;
        let sx = left.unwrap_or(tab_end);
        let words: Vec<String> = SHOWS.iter().map(|s| match v.show_count(*s) {
            Some(n) => format!("{} {n}", show_name(*s)),
            None => show_name(*s).to_string(),
        }).collect();
        let on = SHOWS.iter().position(|s| *s == v.b.show).filter(|_| v.grid());
        let cursor = match slot {
            Some(Slot::Pill(i)) if i < SHOWS.len() && v.child.is_none() => Some(i),
            _ => None,
        };
        let p0 = 2 + v.words().len();
        look::tabs(d, [sx, ty, right - sx], &words, look::Tabs::Bar, on, cursor, Some((5, 6)), show_ends(v), |i| if grid { Act::Row(p0 + i, OK) } else { Act::Show(i) });
    }
    if grid {
        search_box(d, v, l.search, slot == Some(Slot::Search) && v.child.is_none(), v.tiles().len());
        d.hit(l.search, Act::Row(0, OK));
    }
    look::panel(d, l.card, l.strip[3] / PY, a);
    if let Some(say) = mods_kind::coming(v.kind) {
        strip(d, m, l);
        mods_kind::empty(d, v.kind, l.card);
        detail_text(d, a, l.detail, mods_kind::name(v.kind), say, mods_kind::online(v.kind));
        return;
    }
    strip(d, m, l);
    grid_tiles(d, m, a, l);
    footer(d, m, a, l);
    if v.child.is_some() {
        // B202: the open question takes every input; the page dims under it but the chosen mod's panel
        d.popup(Some(super::super::pointer::press(|k| k.back = true)));
        d.anchor = super::super::Anchor::Span;
        d.q(Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 448.0], look::c(look::INK), 60.0);
        d.anchor = super::super::Anchor::Centre;
    }
    detail(d, m, a, l.detail);
}

/// B471: the grid panel's header strip: the kind tabs on the left, the sort and the filter as white "< value >" pills
/// on the right (a tap: its left half back, its right half on; Upload: Sign in and Upload all), under them when the
/// panel is narrow.
fn strip(d: &mut Draw, m: &Menu, l: &Lay) {
    let v = &m.add;
    let [sx, sy, sw, _] = l.strip;
    let (x, w) = (sx + PAD * PX, sw - 2.0 * PAD * PX);
    let h = PILL * PY;
    let y = sy + (look::STRIP * PY - h) / 2.0;
    let ry = if l.stack { y + h + ROW_GAP * PY } else { y };
    let size = 13.5 * PY;
    let gap = 6.0 * PX;
    let mut rx = x + w;
    if mods_kind::coming(v.kind).is_some() || v.packs_page() {
    } else if v.page == Page::Upload {
        let words = [v.account.as_ref().map_or("Sign in".to_string(), |a| format!("Signed in: {}", super::short(a, 18))), "Upload all".to_string()];
        for (i, word) in words.iter().enumerate().rev() {
            let pw = (d.width(word, size) + 30.0 * PX).min(w * 0.45);
            rx -= pw;
            let b = [rx, ry, pw, h];
            look::word_pill(d, b, m.sel == i && v.child.is_none(), word, size);
            d.hit(b, Act::Row(i, OK));
            rx -= gap;
        }
    } else {
        let p0 = 2 + v.words().len();
        let slot = v.slot(m.sel);
        let sort = SORTS[v.b.sort as usize];
        let filter = v.filter_words();
        let sw = d.width("Uploader", size) + 50.0 * PX;
        let fw = (d.width(&filter, size) + 50.0 * PX).clamp(150.0 * PX, 260.0 * PX).min(w * 0.5);
        rx -= sw + gap + fw;
        let unfiltered = v.b.game.is_none() && v.b.style.is_none() && !v.b.left;
        for (k, (b, word, dim)) in [([rx, ry, sw, h], sort.to_string(), false), ([rx + sw + gap, ry, fw, h], filter, unfiltered)].into_iter().enumerate() {
            let i = SHOWS.len() + k;
            let on = slot == Slot::Pill(i) && v.child.is_none();
            if on {
                look::halo(d, b);
            }
            look::steps(d, b, on, &word, size, dim, [true; 2]);
            d.hit([b[0], b[1], b[2] / 2.0, b[3]], Act::Row(p0 + i, Press { ok: true, prev: true, ..Press::default() }));
            d.hit([b[0] + b[2] / 2.0, b[1], b[2] / 2.0, b[3]], Act::Row(p0 + i, OK));
        }
        rx -= gap;
    }
    let room = if l.stack { w } else { rx - x - gap };
    mods_kind::row(d, [x, y, room], v.kind, v.kind_counts(), &v.kinds_shown());
}

/// The tiles on view under their headings, the scroll bar, or what's missing when there are none.
fn grid_tiles(d: &mut Draw, m: &Menu, a: [f32; 3], l: &Lay) {
    let v = &m.add;
    let secs = v.secs();
    let lines = lines(&secs, l.cols);
    let t0 = v.acts();
    let foot = l.grid[1] + l.grid[3];
    let (heads, tiles) = l.laid(&lines, v.top);
    for (k, rect) in heads {
        look::heading(d, &secs[k].name, secs[k].len, rect);
    }
    let items = v.items();
    let locals = v.locals();
    for (i, hit) in tiles {
        let rect = [hit[0], hit[1], l.tile[0], l.tile[1]];
        let on = m.sel == t0 + i;
        let n0 = d.out.len();
        if v.packs_page() {
            // M5b: a pack's row, with its own taps
            if let Some(p) = items.get(i).and_then(|e| v.pack_of(e)) {
                super::packs::row(d, v, a, rect, on, p, t0 + i);
                roster::cut(d, n0, foot);
            }
            continue;
        }
        if v.page == Page::Upload {
            let Some(md) = locals.get(i) else { continue };
            let badges: Vec<(&str, [f32; 3])> = v.ups.get(&md.dir).map(|u| vec![up_badge(u)]).unwrap_or_default();
            tile(d, m, a, rect, on, &mod_face(m, md), false, &md.name, false, &badges, None);
        } else {
            let Some(e) = items.get(i) else { continue };
            let (f, loading) = entry_face(m, e);
            let badges: Vec<(&str, [f32; 3])> = state_badge(v, e).into_iter().collect();
            tile(d, m, a, rect, on, &f, loading, &e.meta.name, e.verified, &badges, progress(v, e));
        }
        roster::cut(d, n0, foot);
        d.hit(hit, Act::Tile(t0 + i));
    }
    if lines.is_empty() {
        let none = if v.page == Page::Upload {
            "No mods installed yet: install one from the store first."
        } else if v.page == Page::Mine && v.account.is_none() {
            "Sign in to see your uploads: Upload (bottom right), then Sign in."
        } else if v.busy && (v.store.is_none() || v.page == Page::Mine) {
            if v.page == Page::Mine { "Reading your uploads..." } else { "Reading the mod store..." }
        } else if v.entries().is_empty() {
            match v.page {
                Page::Installed if v.packs_page() => "No texture packs installed. Put a pack's folder in mods/, or add its .zip with Add mod.",
                Page::Installed => "No mods installed yet. Install one from Browse, or use Add mod.",
                Page::Mine => "None of your mods are on the store.",
                _ => "Nobody has shared a mod yet.",
            }
        } else {
            "No mod matches: try another filter or search."
        };
        look::para(d, none, [l.grid[0] + 6.0 * PX, l.grid[1] + 10.0 * PY], l.grid[2] - 12.0 * PX, 15.0 * PY, 20.0 * PY, 4, look::c(look::MUTED));
    }
    let fit = l.fit(&lines, v.top);
    look::scroll_bar(d, l.bar, a, fit, lines.len().max(fit), v.top, l.last_top(&lines));
}

/// A store tile's badge: Update, Installed or New (none while it downloads or waits its turn).
fn state_badge(v: &View, e: &Entry) -> Option<(&'static str, [f32; 3])> {
    if matches!(v.jobs.get(&e.key()).map(|j| j.0), Some("Downloading" | "Queued")) {
        return None;
    }
    match v.state(e).or((v.page == Page::Installed).then_some("Installed")) {
        Some("Update") => Some(("Update", hex(BADGE_UPDATE))),
        Some(_) => Some(("Installed", hex(BADGE_INSTALLED))),
        None if v.new_one(e) => Some(("New", hex(BADGE_NEW))),
        None => None,
    }
}

/// B250: an upload's badge.
fn up_badge(u: &Up) -> (&'static str, [f32; 3]) {
    let t = super::up_tag(u);
    let c = match u {
        Up::Done(_) => BADGE_INSTALLED,
        Up::Failed(_) => BADGE_FAILED,
        Up::Uploading => BADGE_UPDATE,
        _ => look::RULE,
    };
    (t, hex(c))
}

/// The footer under a light rule: how the kind behaves online on the left, the status on the right (B471: the
/// downloads moved here from the top bar), then the page's words as grey pills and Upload at the far right (a
/// narrow panel: the words in a row over the pills).
fn footer(d: &mut Draw, m: &Menu, a: [f32; 3], l: &Lay) {
    let v = &m.add;
    let [x, y, w, h] = l.foot;
    let top = l.note[1].min(y);
    d.q(Tex::Solid, [0.0; 4], [x, top - (ROW_GAP / 2.0 + 2.0) * PY, w, 2.0 * PY], look::c(look::RULE), 128.0);
    let size = 12.5 * PY;
    let gap = 6.0 * PX;
    let up = v.page == Page::Upload;
    let grid = v.grid() && !v.empty_kind();
    upload_button(d, l.upload, up, grid && v.slot(m.sel) == Slot::Upload && v.child.is_none(), v.phone);
    // (on Upload: a tap goes back to the grid)
    d.hit(l.upload, if up { Act::Tab(0) } else if grid { Act::Row(1, OK) } else { Act::Tab(2) });
    let mut wx = l.upload[0] - gap;
    let words = if up { Vec::new() } else { v.words() };
    for (i, word) in words.iter().enumerate().rev() {
        let pw = d.width(word, size) + 24.0 * PX;
        wx -= pw;
        let b = [wx, y, pw, h];
        look::word_pill(d, b, m.sel == 2 + i && v.child.is_none(), word, size);
        // (B208: a phone can't pick a file)
        if v.phone && *word == "Add mod" {
            rounded(d, b, h / 2.0 / PY, look::c(look::PAPER), 70.0);
        }
        d.hit(b, Act::Row(2 + i, OK));
        if *word == "Add mod" {
            *ADD_AT.lock().unwrap_or_else(|e| e.into_inner()) = Some(b);
        }
        wx -= gap;
    }
    let (nx, nr, ny, nh) = if l.note == l.foot { (x, wx - 6.0 * PX, y, h) } else { (l.note[0], l.note[0] + l.note[2], l.note[1], l.note[3]) };
    let ty = ny + (nh - size) / 2.0;
    let (said, busy) = status(m);
    let room = (nr - nx).max(0.0);
    let spin = if busy { 16.0 * PX } else { 0.0 };
    let said = ellipsis(d, &said, size, (room * 0.6 - spin).max(0.0));
    let sw = d.width(&said, size);
    d.text(&said, nr - sw, ty, size, look::c(look::INK));
    if busy {
        spinner(d, v, [nr - sw - 10.0 * PX, ny + nh / 2.0], a);
    }
    let note = ellipsis(d, mods_kind::online(v.kind), size, (room - sw - spin - 14.0 * PX).max(0.0));
    d.text(&note, nx + 2.0 * PX, ty, size, look::c(look::MUTED));
}

/// A panel with only words: a name outlined white on its strip in `a`, what it is and a note under it.
fn detail_text(d: &mut Draw, a: [f32; 3], b: [f32; 4], name: &str, about: &str, note: &str) {
    let [sx, sy, sw, sh] = look::panel(d, b, look::STRIP, a);
    let (tx, tw) = (sx + PAD * PX, sw - 2.0 * PAD * PX);
    let size = 22.0 * PY;
    let name = ellipsis(d, name, size, tw);
    look::bold(d, &name, [tx, sy + (sh - size) / 2.0], size, WHITE);
    let ty = look::para(d, about, [tx, sy + sh + 14.0 * PY], tw, 14.0 * PY, 19.0 * PY, 8, look::c(look::INK));
    if !note.is_empty() {
        look::para(d, note, [tx, ty + 10.0 * PY], tw, 12.5 * PY, 17.0 * PY, 4, look::c(look::MUTED));
    }
}

/// What the chosen mod's panel shows.
struct Shown {
    name: String,
    by: String,
    /// The grey line's costumes and hand.
    left: String,
    right: String,
    face: Face,
    loading: bool,
    /// Its badge on the picture (Installed, Update, New; an upload's state).
    badge: Option<(&'static str, [f32; 3])>,
    /// Its play style and grades (a character).
    stats: Option<(usize, Option<[u8; 5]>)>,
    facts: Vec<String>,
    /// The green verified line (None: not reviewed).
    verified: Option<bool>,
}

/// The chosen mod's panel: a header strip in the kind's colour holding its picture, its name outlined white at the
/// strip's bottom left and its badge at the top left; one grey line of uploader, costumes and hand (the play style
/// on its right), the pink stats card, its facts, then the primary button in yellow over two grey ones (an open
/// question: its words in a light box, its buttons in their place). Off a tile: the slot's name and what it does.
fn detail(d: &mut Draw, m: &Menu, a: [f32; 3], b @ [_, _, _, h]: [f32; 4]) {
    let v = &m.add;
    let Some(sh) = shown(m) else {
        let (name, about) = slot_text(m);
        // an open question's words are in its box: the panel only names it
        if let Some(c) = &v.child {
            detail_text(d, a, b, &name, "", "");
            buttons(d, m, b, Some(c));
            return;
        }
        return detail_text(d, a, b, &name, &about, "");
    };
    // the strip: the picture as big as a share of the panel allows
    let strip_h = (h / PY * 0.42).clamp(96.0, 230.0);
    let [sx, sy, sw, shh] = look::panel(d, b, strip_h, a);
    let inner = [sx + 8.0 * PX, sy + 6.0 * PY, sw - 16.0 * PX, shh - 12.0 * PY];
    if v.shot {
        // the bake's 4:5 (M8f9: a court's or umpire's own shape), as big as the box allows
        let aspect = v.shot_aspect.unwrap_or(super::super::previews::CARD_W) * PX / PY;
        let (pw, phh) = if aspect > inner[2] / inner[3] { (inner[2], inner[2] / aspect) } else { (inner[3] * aspect, inner[3]) };
        d.q(Tex::Shelf, [0.0, 0.0, 1.0, 1.0], [inner[0] + (inner[2] - pw) / 2.0, inner[1] + (inner[3] - phh) / 2.0, pw, phh], WHITE, 128.0);
    } else {
        if sh.loading {
            rounded(d, inner, 8.0, WHITE, 40.0);
        }
        draw_face(d, m, &sh.face, inner, WHITE);
    }
    if let Some((s, rgb)) = sh.badge {
        let bw = d.width(s, 10.5 * PY) + 12.0 * PX;
        look::badge(d, s, [sx + 10.0 * PX + bw, sy + 10.0 * PY], rgb);
    }
    // the name (smaller to fit, down to 16 px; longer still, it slides) outlined at the strip's bottom left
    let (tx, tw) = (sx + PAD * PX, sw - 2.0 * PAD * PX);
    let ns = (24.0 * PY * (tw / d.width(&sh.name, 24.0 * PY).max(1.0)).min(1.0)).max(16.0 * PY);
    let roll = v.b.roll.0;
    d.outlined(|d| marquee(d, &sh.name, [tx, sy + shh - ns - 8.0 * PY], ns, tw, WHITE, Some(roll)));
    let mut ty = sy + shh + 2.5 * PY + 10.0 * PY;
    // the grey line: who shared it, costumes and hand; the play style on its right
    let is = 13.0 * PY;
    let line = [sh.by.as_str(), sh.left.as_str(), sh.right.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" - ");
    let style = sh.stats.map(|(st, _)| STYLE_NAMES[st.min(3)]);
    let stw = style.map_or(0.0, |s| d.width(s, is) + 10.0 * PX);
    if !line.is_empty() || style.is_some() {
        d.text(&ellipsis(d, &line, is, tw - stw), tx, ty, is, look::c(look::MUTED));
        if let Some(s) = style {
            let sx = tx + tw - d.width(s, is);
            d.text(s, sx, ty, is, look::c(look::INK));
            d.text(s, sx + 0.6 * PX, ty, is, look::c(look::INK));
        }
        ty += 22.0 * PY;
    }
    // the foot first: the buttons (or the question), then the facts over them; the stats card fills what's between
    let foot = buttons(d, m, b, v.child.as_ref());
    let fs = 12.5 * PY;
    let fgap = 18.0 * PY;
    let nf = sh.facts.len() + usize::from(sh.verified.is_some());
    let fy = foot - 10.0 * PY - fgap * nf as f32;
    let mut ly = fy;
    for f in &sh.facts {
        let f = ellipsis(d, f, fs, tw);
        d.text(&f, tx, ly, fs, look::c(look::INK));
        ly += fgap;
    }
    if let Some(ok) = sh.verified {
        if ok {
            tick_disc(d, tx + 6.0 * PY, ly + fs / 2.0, 6.0 * PY, hex(VERIFIED));
            d.text(&ellipsis(d, "Verified: reviewed and merged into the store", fs, tw - 18.0 * PX), tx + 18.0 * PX, ly, fs, hex(VERIFIED));
        } else {
            d.text(&ellipsis(d, "Not reviewed yet: live on the store as uploaded", fs, tw), tx, ly, fs, look::c(look::MUTED));
        }
    }
    let room = fy - 8.0 * PY - ty;
    if let Some((_, grades)) = sh.stats
        && room >= 70.0 * PY
    {
        let card = [tx, ty, tw, room.min(150.0 * PY)];
        roster::ringed(d, [card[0] - 2.0 * PX, card[1] - 2.0 * PY, card[2] + 4.0 * PX, card[3] + 4.0 * PY], 12.0, 2.0, look::c(look::INK), look::c(look::INK));
        roster::stats(d, grades, card);
    }
}

/// The panel's foot: an open question's box and its buttons (the cursor's yellow), else the page's three buttons (a
/// store mod's Install / More by / Save, an installed one's Upload / Upload all / Sign in on Upload): the first a
/// yellow pill over two grey ones; the top of what it drew.
fn buttons(d: &mut Draw, m: &Menu, b @ [x, y, w, h]: [f32; 4], child: Option<&super::Child>) -> f32 {
    let v = &m.add;
    let (tx, tw) = (x + (RIM + PAD) * PX, w - 2.0 * (RIM + PAD) * PX);
    let bottom = y + h - (RIM + PAD) * PY;
    let size = 15.0 * PY;
    let (ink, soft) = (look::c(look::INK), look::c(look::MUTED));
    // a light box in a light rim
    let boxed = |d: &mut Draw, r: [f32; 4]| roster::ringed(d, r, 10.0, 1.5, look::c(look::TILE_RIM), look::c(look::TILE));
    if let Some(c) = child {
        // B202: the question's words over its buttons (Cancel last)
        let rects = child_rects(c, b);
        let top = rects.first().map_or(bottom, |r| r[1]);
        let ws = 13.5 * PY;
        let words = super::info(m);
        let lines = look::wrap(d, &words, tw - 20.0 * PX, ws, 6).len().max(1);
        let bh = 16.0 * PY + 18.0 * PY * lines as f32;
        let bx = [tx, top - 10.0 * PY - bh, tw, bh];
        boxed(d, bx);
        look::para(d, &words, [tx + 10.0 * PX, bx[1] + 8.0 * PY], tw - 20.0 * PX, ws, 18.0 * PY, lines, ink);
        for (k, (r, word)) in rects.into_iter().zip(&c.opts).enumerate() {
            if k == c.at {
                look::halo(d, r);
            }
            look::word_pill(d, r, k == c.at, word, size);
            d.hit(r, Act::Row(k, OK));
        }
        return bx[1];
    }
    let (bh, sh, gap) = (36.0 * PY, 30.0 * PY, 8.0 * PY);
    let primary = [tx, bottom - sh - gap - bh, tw, bh];
    let half = (tw - gap * PX / PY) / 2.0;
    let seconds = [[tx, bottom - sh, half, sh], [tx + tw - half, bottom - sh, half, sh]];
    let press = |f: fn(&mut Press)| {
        let mut p = Press::default();
        f(&mut p);
        Act::Press(p)
    };
    // the words, each button's pad / key art (the legend's) and its act
    let (words, acts, rule): ([String; 3], [Act; 3], bool) = if v.page == Page::Upload {
        let signed = v.account.is_some();
        (
            ["Upload".into(), "Upload all".into(), if signed { "Sign out".into() } else { "Sign in".into() }],
            [Act::Press(OK), Act::Row(1, OK), Act::Row(0, OK)],
            true,
        )
    } else if v.packs_page() {
        // M5b: a pack's switch and moves
        let Some(r) = v.pack_at(m.sel) else { return primary[1] };
        let first = if r.on { "Turn off" } else { "Turn on" };
        ([first.into(), "Move up".into(), "Move down".into()], [Act::Press(OK), press(|p| p.card = true), press(|p| p.page = true)], false)
    } else {
        let Some(e) = v.hovered(m.sel) else { return primary[1] };
        let mt = &e.meta;
        let installed = v.state(e).or((v.page == Page::Installed).then_some("Installed"));
        // B383: Installed's own mods (one added by hand stays: the game didn't put it in), My uploads' take down
        let by_hand = v.page == Page::Installed && v.local(m.sel).is_some_and(|md| !super::ours(md));
        let first = match (v.jobs.get(&e.key()).map(|j| j.0), installed) {
            (Some("Downloading"), _) => "Downloading...".to_string(),
            (Some("Queued"), _) => "Queued".to_string(),
            _ if v.page == Page::Mine => if e.verified { "Ask to delete" } else { "Take down" }.to_string(),
            (_, Some("Installed")) if by_hand => "Added by hand".to_string(),
            (_, Some("Installed")) => "Remove".to_string(),
            (_, Some(_)) => format!("Update - {}", mb(mt.size)),
            _ => format!("Install - {}", mb(mt.size)),
        };
        let wished = v.b.wish.as_ref().is_some_and(|w| w.contains(&e.key()));
        let more = if mt.author.is_empty() || v.by.is_some() { "More by uploader".to_string() } else { format!("More by {}", mt.author) };
        ([first, more, (if wished { "Unsave" } else { "Save" }).into()], [Act::Press(OK), press(|p| p.card = true), press(|p| p.page = true)], false)
    };
    let mut top = primary[1];
    if rule {
        // the rights rule in a light box over the buttons
        let lines = look::wrap(d, RULE, tw - 20.0 * PX, 12.5 * PY, 6).len().max(1);
        let rh = 14.0 * PY + 17.0 * PY * lines as f32;
        let r = [tx, primary[1] - 10.0 * PY - rh, tw, rh];
        boxed(d, r);
        look::para(d, RULE, [tx + 10.0 * PX, r[1] + 7.0 * PY], tw - 20.0 * PX, 12.5 * PY, 17.0 * PY, lines, soft);
        top = r[1];
    }
    for (k, (r, word)) in [primary, seconds[0], seconds[1]].into_iter().zip(&words).enumerate() {
        look::button(d, r, k == 0);
        let ink = if k == 0 { ink } else { WHITE };
        // the button's own pad / key art, as the legend draws it
        let gh = r[3] - 12.0 * PY;
        let gw = look::glyph(d, 2 + k as u8, [r[0] + 10.0 * PX, r[1] + 6.0 * PY], gh);
        let fs = if k == 0 { size } else { 13.0 * PY };
        d.label(word, r[0] + (r[2] + gw) / 2.0 + 4.0 * PX, r[1] + (r[3] - fs) / 2.0, fs, r[2] - gw - 28.0 * PX, ink);
        d.hit(r, acts[k]);
    }
    top
}

/// B202: the open question's buttons, stacked at the chosen mod's panel's foot (`detail`), Cancel last.
pub(super) fn child_rects(c: &super::Child, [x, y, w, h]: [f32; 4]) -> Vec<[f32; 4]> {
    let n = c.opts.len();
    let (bh, gap) = (32.0 * PY, 8.0 * PY);
    let bottom = y + h - (RIM + PAD) * PY;
    (0..n).map(|k| [x + (RIM + PAD) * PX, bottom - (bh + gap) * (n - k) as f32 + gap, w - 2.0 * (RIM + PAD) * PX, bh]).collect()
}

/// What the chosen mod's panel says of the tile under the cursor (None: the cursor isn't on one).
fn shown(m: &Menu) -> Option<Shown> {
    let v = &m.add;
    let plural = |n: usize, w: &str| format!("{n} {w}{}", if n == 1 { "" } else { "s" });
    let hand = |h: f32| if h < 0.0 { "Left-handed" } else { "Right-handed" }.to_string();
    if v.page == Page::Upload {
        let md = v.local(m.sel)?;
        let player = v.kind == Kind::Character;
        let stats = v.stats.get(&md.dir).map(|&(st, g)| (st, Some(g)));
        let mut facts = vec![if md.source.is_empty() { format!("Folder {}", md.folder()) } else { format!("From {}", md.source) }];
        if player && let Some(c) = CHAR_NAMES.get(md.donor) {
            facts.push(format!("Plays like {c}"));
        }
        // B250: its upload's outcome (the pull request's link, or why not)
        if let Some(u) = v.ups.get(&md.dir) {
            let t = super::up_tag(u);
            facts.push(match u {
                Up::Done(s) | Up::Skipped(s) | Up::Failed(s) => format!("{t}: {}", s.rsplit(' ').next().filter(|w| w.starts_with("https://")).unwrap_or(s)),
                _ => t.into(),
            });
        }
        return Some(Shown {
            name: md.name.clone(),
            by: String::new(),
            left: if player { plural(md.costumes.len(), "costume") } else { String::new() },
            right: if player { hand(md.hand) } else { String::new() },
            face: mod_face(m, md),
            loading: false,
            badge: v.ups.get(&md.dir).map(up_badge),
            stats: stats.filter(|_| player),
            facts,
            verified: None,
        });
    }
    if v.packs_page() {
        // M5b: a texture pack: what it is, where it stands, its folder
        let e = v.hovered(m.sel)?;
        let i = v.pack_of(e)?;
        let r = &v.packs[i];
        return Some(Shown {
            name: r.name.clone(),
            by: if r.author.is_empty() { String::new() } else { format!("by {}", r.author) },
            left: super::packs::count(r.count),
            right: super::packs::size(r.bytes),
            face: initial(&r.name, !r.on),
            loading: false,
            badge: Some(if r.on { ("On", hex(BADGE_INSTALLED)) } else { ("Off", look::c(look::RULE)) }),
            stats: None,
            facts: vec![v.pack_place(i), format!("Folder {}", r.folder), "Where two packs share a texture, the higher wins.".into()],
            verified: None,
        });
    }
    let e = v.hovered(m.sel)?;
    let mt = &e.meta;
    let (face, loading) = entry_face(m, e);
    let like = mt.like.and_then(|c| CHAR_NAMES.get(c as usize));
    let line = [Some(format!("v{}", if mt.version.is_empty() { "1" } else { &mt.version })), (mt.size > 0).then(|| mb(mt.size)), like.map(|c| format!("plays like {c}"))]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" - ");
    let mut facts = vec![if mt.source.is_empty() { "From another game".to_string() } else { format!("From {}", mt.source) }, line];
    if let Some(j) = v.jobs.get(&e.key()) {
        facts.push(j.1.clone());
    }
    Some(Shown {
        name: mt.name.clone(),
        by: if mt.author.is_empty() { String::new() } else { format!("by {}", mt.author) },
        left: mt.costumes.map_or(String::new(), |c| plural(c as usize, "costume")),
        right: mt.hand.map_or(String::new(), |h| hand(f32::from(h))),
        face,
        loading,
        badge: state_badge(v, e),
        stats: mt.style.map(|st| (st as usize, mt.grades)),
        facts,
        verified: Some(e.verified),
    })
}

/// Off a tile: the slot's name and what it does (or the last action's outcome).
fn slot_text(m: &Menu) -> (String, String) {
    let v = &m.add;
    let about = super::info(m);
    let name = match v.page {
        Page::Upload => if m.sel == 0 { if v.account.is_some() { "Account" } else { "Sign in" } } else { "Upload all" }.to_string(),
        _ => match v.slot(m.sel) {
            Slot::Search => "Search".into(),
            Slot::Upload => "Upload".into(),
            Slot::Word(i) => v.words()[i].into(),
            Slot::Pill(i) => match v.pills()[i] {
                Pill::Show(s) => show_name(s).into(),
                Pill::Sort => "Sort".into(),
                Pill::Filter => "Filter".into(),
            },
            Slot::Tile(_) => String::new(),
        },
    };
    (name, about)
}

/// What the cursor's slot does (the chosen mod's column off a tile).
pub(super) fn info(m: &Menu) -> String {
    let v = &m.add;
    match v.slot(m.sel) {
        Slot::Search => "Search by name, uploader or game: press X, then type (on a pad, a keyboard opens on screen).".into(),
        Slot::Upload if v.phone => "Not available in the browser on phones: use the desktop app.".into(),
        Slot::Upload => "Share your installed mods on the store: sign in to Hugging Face once, then upload.".into(),
        Slot::Word(i) => match v.words()[i] {
            "Refresh" if v.page == Page::Mine => "Read your uploads from the store again.".into(),
            "Refresh" => "Read the store's list again.".into(),
            "Add mod" if v.phone => "Not available in the browser on phones: use the desktop app.".into(),
            "Add mod" => "Install a mod from a .zip file on this computer: a character, court, umpire or texture pack.".into(),
            "Remove all" => "Remove every mod the game installed (ones added by hand stay).".into(),
            "Remove all uploads" => "Take every mod you shared off the store (asks first).".into(),
            "Reload" => super::reload_said(v).into(),
            "Download all" => format!("Install every mod by {} you don't have yet.", v.by.as_ref().map_or("", |b| super::who(&b.author))),
            _ => "Back to every mod in the store.".into(),
        },
        Slot::Pill(i) => match v.pills()[i] {
            Pill::Show(Show::All) => "Every mod on the store.".into(),
            Pill::Show(Show::Installed) => format!("The mods on this device ({}): remove them, or add one from a .zip.", v.locals().len()),
            Pill::Show(Show::Updates) => "Installed mods with a newer version on the store.".into(),
            Pill::Show(Show::Mine) if v.account.is_none() => "The mods you've shared on the store. Sign in under Upload (top right) to see them.".into(),
            Pill::Show(Show::Mine) => "The mods you've shared on the store: press one to take it down.".into(),
            Pill::Show(Show::Wish) => "The mods you saved with Save (□ on a mod).".into(),
            Pill::Sort => match v.b.sort {
                Sort::Newest => "The newest uploads first, under their games. Press for A-Z or by uploader.",
                Sort::Az => "By name, under their games A-Z. Press for by uploader or newest.",
                Sort::Uploader => "Grouped by who shared them. Press for newest or A-Z.",
            }
            .into(),
            Pill::Filter if v.groups() > 1 => format!("Showing {}: press for the next, Select for {}.", v.filter_words(), GROUPS[(v.b.filter + 1) % v.groups()].to_lowercase()),
            Pill::Filter => format!("Showing {}: press for the next game.", v.filter_words()),
        },
        Slot::Tile(_) if v.packs_page() => match v.hovered(m.sel).and_then(|e| v.pack_of(e)) {
            Some(i) => format!("{} - {}. Switch it off or on, or move it up or down: the higher pack wins. Reload to see changes.", v.packs[i].name, v.pack_place(i)),
            None => String::new(),
        },
        Slot::Tile(_) => match v.hovered(m.sel) {
            Some(e) if v.jobs.contains_key(&e.key()) => v.jobs[&e.key()].1.clone(),
            Some(e) if v.page == Page::Mine && e.verified => format!("{} is part of the store: press to ask for it to be deleted.", e.meta.name),
            Some(e) if v.page == Page::Mine => format!("{} is live but not reviewed yet: press to take it down.", e.meta.name),
            Some(e) if v.page == Page::Installed && v.state(e) != Some("Update") => match v.local(m.sel) {
                Some(md) if !super::ours(md) => format!("{} was added by hand, not by the game: delete its folder {} to remove it.", md.name, md.dir.display()),
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

/// The on-screen keyboard over the dimmed page (B441: `osk`, the Online menu's name's too).
fn keyboard(d: &mut Draw, v: &View) {
    osk::draw(d, &v.b.search, v.b.osk.unwrap_or(0), 0.0, osk::Look { title: None, done: "Done", full: false });
}

/// The legend's words on Browse.
pub(super) fn legend(m: &Menu) -> Vec<(super::super::legend::Act, &'static str)> {
    use super::super::legend::Act as L;
    let v = &m.add;
    if v.b.osk.is_some() {
        return osk::legend(v.b.osk.unwrap_or(0), Some("Done"));
    }
    let mut l = vec![(L::Move, "Move")];
    match v.slot(m.sel) {
        Slot::Search if v.b.focus => l.extend([(L::Type, "Type"), (L::Ok, "Done")]),
        Slot::Search if !v.pad && !v.phone => l.push((L::Ok, "Type")),
        Slot::Search => l.push((L::Ok, "Keyboard")),
        Slot::Upload => l.push((L::Ok, "Upload")),
        Slot::Pill(i) => match v.pills()[i] {
            Pill::Show(_) => l.push((L::Ok, "Show")),
            Pill::Sort => l.push((L::Ok, "Sort")),
            Pill::Filter if v.groups() > 1 => l.extend([(L::Ok, "Next"), (L::Hand, "Filter")]),
            Pill::Filter => l.push((L::Ok, "Next")),
        },
        Slot::Tile(_) if v.packs_page() => {
            if let Some(r) = v.pack_at(m.sel) {
                l.extend([(L::Ok, if r.on { "Turn off" } else { "Turn on" }), (L::Card, "Move up"), (L::Page, "Move down"), (L::Hand, "Remove")]);
            }
        }
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
    // B471: L1 / R1 the Show tabs
    l.push((L::Costume, "Show"));
    l.extend(v.kind_switch().into_iter().chain([(L::Back, "Back")]));
    l
}

// ---------------------------------------------------------------- pictures

/// The tiles' pictures: fetches what's on view and the next screen (at most `cap` at once; cached ones are read at
/// once), and drops loaded ones far off view past [`KEEP`]. Returns the fetches to start (the caller's, so the
/// tests can see them without the network).
pub(super) fn want(v: &mut View, l: &Lay, cap: usize) -> Vec<Entry> {
    if !v.grid() {
        return Vec::new();
    }
    let tiles: Vec<Entry> = v.tiles().into_iter().cloned().collect();
    let lines = lines(&v.secs(), l.cols);
    let order = plan(&lines, v.top, l.fit(&lines, v.top), v.b.down);
    let asked = v.b.pics.values().filter(|p| **p == Pic::Asked).count();
    let mut go = Vec::new();
    for &i in &order {
        let Some(e) = tiles.get(i) else { continue };
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
    if shown <= KEEP || !v.grid() {
        return Vec::new();
    }
    let tiles = v.tiles();
    let lines = lines(&v.secs(), l.cols);
    let fit = l.fit(&lines, v.top);
    let near: HashSet<String> = plan(&lines, v.top, fit, v.b.down).into_iter().chain(plan(&lines, v.top, fit, !v.b.down)).filter_map(|i| tiles.get(i)).map(|e| e.meta.sha256.clone()).collect();
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

/// The installed mods' faces on the tiles on view (what `previews.rs` bakes first while the page is up).
pub(super) fn faces(m: &Menu) -> Vec<super::super::Who> {
    let v = &m.add;
    let l = lay_of(m);
    let lines = lines(&v.secs(), l.cols);
    let on: Vec<usize> = l.laid(&lines, v.top).1.into_iter().map(|t| t.0).collect();
    let dirs: Vec<&std::path::Path> = if v.page == Page::Upload {
        on.iter().filter_map(|&i| v.locals().get(i)).map(|md| md.dir.as_path()).collect()
    } else {
        let items = v.items();
        on.iter().filter_map(|&i| items.get(i).copied()).filter_map(|e| v.dir_of(e)).collect()
    };
    dirs.into_iter().filter_map(|dir| super::mod_of(m, dir)).map(super::super::Who::Mod).collect()
}

/// The cache folder of the tiles' pictures: `mod-pictures/` beside `mods/`.
pub(super) fn cache_dir() -> std::path::PathBuf {
    super::root().with_file_name("mod-pictures")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B444: a long title rests at its start, slides at `ROLL_SPEED` until its end shows, rests there, starts over.
    #[test]
    fn a_long_title_rests_slides_rests_and_starts_over() {
        let over = 60.0;
        let slide = over / ROLL_SPEED;
        assert_eq!(roll_offset(over, 0.0), 0.0);
        assert_eq!(roll_offset(over, ROLL_REST * 0.9), 0.0);
        assert!((roll_offset(over, ROLL_REST + slide / 2.0) - over / 2.0).abs() < 1e-3);
        assert_eq!(roll_offset(over, ROLL_REST + slide + 0.5), over);
        assert_eq!(roll_offset(over, 2.0 * ROLL_REST + slide + 0.1), 0.0);
    }

    /// B448: the grid's lines: a heading per section, its tiles `cols` a row; ↑↓ keep the column (a short row: its
    /// last), ←→ go on over the rows' ends, off the edge is None; the pictures' plan is the lines on view, then the
    /// next screen the way it scrolled.
    #[test]
    fn lines_and_moves_over_sections() {
        let secs = [Sec { name: "A".into(), start: 0, len: 5 }, Sec { name: "B".into(), start: 5, len: 2 }];
        let l = lines(&secs, 3);
        assert_eq!(l, [Line::Head(0), Line::Row(0, 3), Line::Row(3, 5), Line::Head(1), Line::Row(5, 7)]);
        let (up, down, left, right) = (Press { up: true, ..default() }, Press { down: true, ..default() }, Press { left: true, ..default() }, Press { right: true, ..default() });
        assert_eq!(tile_step(&l, 2, down), Some(4));
        assert_eq!(tile_step(&l, 4, down), Some(6));
        assert_eq!(tile_step(&l, 6, up), Some(4));
        assert_eq!(tile_step(&l, 1, up), None);
        assert_eq!(tile_step(&l, 6, down), None);
        assert_eq!((tile_step(&l, 3, left), tile_step(&l, 4, right), tile_step(&l, 0, left), tile_step(&l, 6, right)), (Some(2), Some(5), None, None));
        assert_eq!(plan(&l, 0, 2, true), [0, 1, 2, 3, 4, 5, 6]);
        assert_eq!(plan(&l, 3, 2, false), [5, 6, 4, 3, 2, 1, 0]);
    }
}
