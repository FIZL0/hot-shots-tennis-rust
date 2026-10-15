//! Mods (M16, M17). Installed: Add mod picks a mod `.zip` (the browser build: the page's file picker, `web.rs`;
//! desktop: the system's, `modstore::pick`), which is checked whole and kept in `mods/<id>/` (OPFS in the browser,
//! beside the game on desktop; `modfs::install`); ✕ on an installed mod opens Remove / Cancel beside it (B202's child
//! buttons, `child.rs`, for every press that needs a second word).
//! Browse mods shows the mod store as a picture grid (B372, `grid.rs`; `modstore.rs`: Verified = merged on the store's
//! `main`, Installed / Update by the zip's sha256); ✕ downloads, checks and installs one (B339: queued ones download side by side, as many at once as
//! Settings → Online & Mods → Downloads at Once says). Upload my mod signs in with a Hugging Face write token (the
//! token page opens, the player copies one, Paste token pastes it) and then, after the rule and Upload,
//! commits an installed mod to the store as a pull request (`modstore::publish`); Upload all (B250) queues every one, in
//! the background with a status beside each, and a re-upload goes over the player's earlier one. Its My public mods (M19) lists the
//! signed-in player's own uploads (`modstore::mine`); Take down takes one down (`modstore::remove`). Every page is a list on the left and
//! the hovered mod on the right, as the select's cards: its picture, △ card and details. The select's roster is
//! built when the game starts, so Reload brings changes into it. Mouse and touch: a tap on a row chooses it (a Browse tile: a
//! second tap installs); on Add mod the page's own
//! file input lies over the row, so the tap that opens the picker is the browser's own (iOS opens a picker only
//! inside a real tap).

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::super::widescreen;
use super::child::{Child, Got};
use super::mods_kind;
use super::{BROWN, CARD_FILL, CELL_FILL, CHAR_NAMES, CREAM, Draw, Effect, GOLD, INK, Menu, Press, Screen, Tex, WHITE, attribute_card};
use crate::modstore::{self, Entry, Event, Kind, Up};
use crate::mods::Mod;

mod grid;

/// The mods folder: `mods/` beside the disc, as the select's roster reads it (`main_menu.rs setup` sets it; a
/// relative `mods` broke when the game was started from elsewhere, e.g. a desktop shortcut).
static ROOT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

pub(crate) fn set_root(dir: std::path::PathBuf) {
    // `./mods` (a disc path with no folder, the browser's) stays `mods`, the OPFS paths' form
    let _ = ROOT.set(dir.strip_prefix(".").map(Into::into).unwrap_or(dir));
}

pub(crate) fn root() -> &'static Path {
    ROOT.get().map(|p| p.as_path()).unwrap_or(Path::new("mods"))
}
/// The pills above the list on Installed (Add mod, Reload, Remove all) and Upload (Sign in, Upload all): the rows
/// before the mods.
const TOP: usize = 3;
const UP_TOP: usize = 2;
/// The tabs across the title band, in Q/E order.
const PAGES: [Page; 4] = [Page::Browse, Page::Installed, Page::Upload, Page::Mine];
/// A mod card's height and pitch in the list.
const CARD_H: f32 = 40.0;
const CARD_PITCH: f32 = 45.0;

const RULE: &str = "Only upload what you have the right to share: you're responsible for it. It goes live for everyone at once.";

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(super) enum Page {
    #[default]
    Installed,
    Browse,
    Upload,
    /// M19: the signed-in player's uploads.
    Mine,
}

#[derive(Default, Clone, Debug)]
pub(super) struct View {
    pub(super) page: Page,
    /// Installed mods: characters, and (M8f9) court and umpire mods (`mods::read_any`'s headers).
    list: Vec<Mod>,
    courts: Vec<Mod>,
    umpires: Vec<Mod>,
    /// The store's listing (None: not read yet).
    store: Option<Vec<Entry>>,
    /// M19: the player's own uploads (None: not read yet).
    mine: Option<Vec<Entry>>,
    /// B248: the installed store mods by [`Entry::key`] (a pre-B248 install's under `/<id>`): the zip's sha256 and
    /// the folder. A mod added from a zip or by hand isn't one of the store's, though it may share an id.
    have: HashMap<String, (String, PathBuf)>,
    /// The listing or an install is under way.
    busy: bool,
    /// B250: each upload since the game started, by mod folder (uploads run in the background, not `busy`).
    ups: HashMap<PathBuf, Up>,
    /// B202: the open child buttons (Remove, Upload, Sign in / out, Take down) and the row they hang off.
    child: Option<Child>,
    /// What the last action did (the description bar).
    status: String,
    /// The first row drawn.
    top: usize,
    /// The select's play style and △ grades by mod folder, and the disc's path (where the select's pictures are cached).
    stats: HashMap<PathBuf, (usize, [u8; 5])>,
    iso: String,
    /// The signed-in Hugging Face account; a press asked for the token on the clipboard.
    account: Option<String>,
    paste: bool,
    /// The hovered mod's picture is in (`Shelf::shown`); M8f9: a court's or umpire's own width over height (a
    /// character's is the bake's 4:5).
    shot: bool,
    shot_aspect: Option<f32>,
    /// B205: a mod was installed or removed since the game started (the note on the main menu's Mods row), and since
    /// the main menu last asked; the Reload / Later popup is up on the main menu (0 = Reload chosen).
    changed: bool,
    fresh: bool,
    pub(super) ask: Option<usize>,
    /// B255: Browse narrowed to one author's mods (whose list it is, and the row to go back to on the whole list).
    by: Option<By>,
    /// B255: the installs waiting (Download all, or a press while others run), the ones under way (B339: up to
    /// Downloads at Once side by side), and each one's tag by [`Entry::key`] (Queued, Downloading, Installed, Error)
    /// with what to say of it.
    queue: VecDeque<Entry>,
    doing: Vec<Entry>,
    jobs: HashMap<String, (&'static str, String)>,
    /// B208: the web build on a touch device (no file picker, no sign-in): Add mod and Upload my mod are greyed, and
    /// choosing one puts up the "use the desktop app" popup (its row while up).
    phone: bool,
    nophone: Option<usize>,
    /// B445: My uploads' Remove all uploads warning popup while up (its cursor: 0 Remove all, 1 Cancel), and the
    /// uploads still to take down after its Remove all, one at a time, with how many went and how many didn't of all.
    wipe: Option<usize>,
    wiping: VecDeque<Entry>,
    wiped: (usize, usize, usize),
    /// B372: Browse's grid: its search, filters, sort, saved mods and tile pictures.
    b: grid::Browse,
    /// B335: the installs since the queue was last empty: done, all (the status's "3/50").
    batch: (usize, usize),
    /// B373: the kind tab every page lists (`mods_kind.rs`).
    pub(super) kind: Kind,
    /// B383: the installed mods as the grid's tiles (Show → Installed; [`View::relist`]).
    inst: Vec<Entry>,
    /// B383: the last press was a pad's (the search box's ✕ then opens the keyboard on screen).
    pub(super) pad: bool,
}

#[derive(Clone, Debug)]
struct By {
    author: String,
    list: Vec<Entry>,
    from: usize,
}

impl View {
    /// B445: Remove all uploads' next take-down (`busy` while it's out), or, the last one back, what came of them all.
    fn wipe_next(&mut self) {
        let (done, failed, all) = self.wiped;
        let token = modstore::account().map(|a| a.1);
        match (self.wiping.pop_front(), token) {
            (Some(e), Some(token)) => {
                self.busy = true;
                self.status = format!("Removing your uploads: {} ({}/{all})...", e.meta.name, done + failed + 1);
                modstore::remove(&e, token);
            }
            (left, _) => {
                let missed = failed + left.as_ref().map_or(0, |_| 1 + self.wiping.len());
                self.wiping.clear();
                self.wiped = (0, 0, 0);
                self.status = match missed {
                    0 => format!("Removed all {done} of your uploads from the store."),
                    _ if left.is_some() => format!("Signed out: removed {done} of {all} uploads. Sign in again and Remove all uploads for the rest."),
                    _ => format!("Removed {done} of {all} uploads; {missed} couldn't be: Refresh, then Remove all uploads again."),
                };
            }
        }
    }

    /// How many mods the store lists (the Browse tab's count).
    fn entries_len(&self) -> usize {
        self.store.as_ref().map_or(0, Vec::len)
    }

    fn refresh(&mut self) {
        self.list = crate::mods::list(root());
        self.courts = crate::mods::list_kind(root(), &crate::mods::Kind::Court);
        self.umpires = crate::mods::list_kind(root(), &crate::mods::Kind::Umpire);
        self.have = self.list.iter().chain(&self.courts).chain(&self.umpires).filter_map(|m| modstore::stamp(&m.dir).map(|s| (format!("{}/{}", s.author, m.id), (s.sha256, m.dir.clone())))).collect();
        self.child = None;
        self.relist();
    }

    /// B383: the installed mods as tiles (Show → Installed): a store mod's own listing (its picture, its Update), else
    /// an entry made from its `mod.json` and the select's card (`rev` "local:<folder>": no store picture, the
    /// select's drawing instead).
    pub(super) fn relist(&mut self) {
        let store = self.store.as_deref().unwrap_or_default();
        let all = [(&self.list, Kind::Character), (&self.courts, Kind::Court), (&self.umpires, Kind::Umpire)];
        self.inst = all
            .into_iter()
            .flat_map(|(l, k)| l.iter().map(move |m| (m, k)))
            .map(|(m, kind)| {
                let stamp = modstore::stamp(&m.dir);
                if let Some(e) = stamp.as_ref().and_then(|s| store.iter().find(|e| e.meta.author == s.author && e.meta.id == m.id)) {
                    return e.clone();
                }
                let player = kind == Kind::Character;
                let stats = self.stats.get(&m.dir).filter(|_| player);
                let (author, sha256, verified) = stamp.map(|s| (s.author, s.sha256, s.verified)).unwrap_or_default();
                let meta = modstore::Meta {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    source: m.source.clone(),
                    author,
                    version: modstore::version(&m.dir),
                    sha256,
                    style: stats.map(|s| s.0 as u8),
                    grades: stats.map(|s| s.1),
                    hand: player.then_some(if m.hand < 0.0 { -1 } else { 1 }),
                    costumes: player.then_some(m.costumes.len() as u32),
                    like: player.then_some(m.donor as u8),
                    kind,
                    ..Default::default()
                };
                Entry { meta, rev: format!("local:{}", m.dir.display()), verified }
            })
            .collect();
    }

    /// B383: Browse, Installed and My uploads are the picture grid (Show in its rail picks which); Upload is a list.
    pub(super) fn grid(&self) -> bool {
        self.page != Page::Upload
    }

    /// B383: tile `e`'s installed folder (a store mod's install, or an installed mod's own tile).
    fn dir_of<'a>(&'a self, e: &'a Entry) -> Option<&'a Path> {
        self.installed(e).map(|(_, d)| d.as_path()).or_else(|| e.rev.strip_prefix("local:").map(Path::new))
    }

    /// B383: each kind's mods, on the store or installed (by `mods_kind::KINDS`): which tabs the row shows.
    fn kind_totals(&self) -> [usize; 4] {
        mods_kind::KINDS.map(|k| self.store.iter().flatten().filter(|e| e.meta.kind == k).count() + self.of_kind(k).len())
    }

    /// B383: the kind tabs shown (a kind with no mods hidden, the chosen one always).
    pub(super) fn kinds_shown(&self) -> Vec<Kind> {
        mods_kind::shown(self.kind, self.kind_totals())
    }

    /// B383: the legend's kind switch (L2 / R2), while there's another kind to switch to.
    pub(super) fn kind_switch(&self) -> Option<(super::legend::Act, &'static str)> {
        (self.kinds_shown().len() > 1).then_some((super::legend::Act::Kind, "Kind"))
    }

    fn changed(&mut self) {
        self.changed = true;
        self.fresh = true;
    }

    fn entries(&self) -> &[Entry] {
        match (&self.page, &self.by) {
            (Page::Mine, _) => self.mine.as_deref().unwrap_or_default(),
            (Page::Installed, _) => &self.inst,
            (_, Some(b)) => &b.list,
            _ => self.store.as_deref().unwrap_or_default(),
        }
    }

    /// B255: Browse narrowed to `author`'s mods from the store's list (None: the whole list again, on row `sel`).
    fn narrow(&mut self, author: Option<String>, from: usize) {
        self.by = author.map(|a| {
            let list = self.store.iter().flatten().filter(|e| e.meta.author == a && e.meta.kind == self.kind).cloned().collect();
            By { author: a, list, from }
        });
    }

    /// B255: whether Download all leaves `e` out: installed as listed, or installed at the same or a newer version
    /// (its `mod.json`'s), or already queued or under way.
    fn skips(&self, e: &Entry) -> bool {
        let k = e.key();
        self.queue.iter().chain(&self.doing).any(|q| q.key() == k)
            || self.installed(e).is_some_and(|(sha, dir)| *sha == e.meta.sha256 || !modstore::newer(&e.meta.version, &modstore::version(dir)))
    }

    /// B255: queues `e`'s install, starting it if fewer than `at_once` are under way.
    fn enqueue(&mut self, e: Entry, upscaled: bool, at_once: usize) {
        if self.doing.is_empty() && self.queue.is_empty() {
            self.batch = (0, 0);
        }
        self.batch.1 += 1;
        self.jobs.insert(e.key(), ("Queued", format!("{} is waiting its turn.", e.meta.name)));
        self.queue.push_back(e);
        self.next(upscaled, at_once);
    }

    /// B255: starts queued installs while fewer than `at_once` are under way (B339: they download side by side).
    fn next(&mut self, upscaled: bool, at_once: usize) {
        while self.doing.len() < at_once.max(1) {
            let Some(e) = self.queue.pop_front() else { return };
            self.busy = true;
            self.status = format!("{}Downloading {} ({})...", self.count(), e.meta.name, mb(e.meta.size));
            self.jobs.insert(e.key(), ("Downloading", format!("Downloading {}...", e.meta.name)));
            // (the tests' queues fetch nothing)
            #[cfg(not(test))]
            modstore::install(&e, root(), upscaled);
            #[cfg(test)]
            let _ = upscaled;
            self.doing.push(e);
        }
    }

    /// B255: install `key`'s outcome `r` on its tag (B339: whichever of those under way it is; Add mod's pick, key
    /// "", is none of them).
    fn finish(&mut self, key: &str, r: &Result<String, String>) {
        let Some(i) = self.doing.iter().position(|e| e.key() == key) else { return };
        let e = self.doing.remove(i);
        self.batch.0 += 1;
        let job = match r {
            Ok(_) => ("Installed", format!("Installed {}. Reload to play it.", e.meta.name)),
            Err(err) => ("Error", format!("{} not installed: {err}", e.meta.name)),
        };
        self.jobs.insert(e.key(), job);
    }

    /// B335: "3/50 done. " while a batch of installs (Download all) is under way.
    fn count(&self) -> String {
        if self.batch.1 > 1 { format!("{}/{} done. ", self.batch.0, self.batch.1) } else { String::new() }
    }

    /// Whether ←→ move from row `sel` (between pills side by side), for the legend.
    pub(super) fn sideways(&self, sel: usize) -> bool {
        self.child.is_none() && self.nophone.is_none() && self.wipe.is_none() && sel < self.acts() && self.acts() > 1
    }

    /// The page's pills above its list (the first rows): Installed, Add mod and Reload; Browse, Refresh (an author's
    /// list: Download all and All mods); Upload, the account and Upload all; Mine, Refresh.
    fn acts(&self) -> usize {
        if mods_kind::coming(self.kind).is_some() {
            return 0;
        }
        if self.grid() { self.browse_acts() } else { UP_TOP }
    }

    /// The pills, then the mods (installed ones, or the store's with their uploaders' headings).
    fn rows(&self) -> usize {
        self.acts() + if self.grid() { self.items().len() } else { self.locals().len() }
    }

    /// The store's mods under the pills: Browse's tiles (searched, filtered, sorted), My uploads as listed; only
    /// the chosen kind's (B373), none of a kind the game can't install yet.
    fn items(&self) -> Vec<&Entry> {
        if mods_kind::coming(self.kind).is_some() {
            return Vec::new();
        }
        if self.grid() { self.tiles() } else { Vec::new() }
    }

    /// The installed mod on row `sel`, if it's one.
    fn local(&self, sel: usize) -> Option<&Mod> {
        match self.page {
            Page::Upload => self.locals().get(sel.checked_sub(self.acts())?),
            // B383: an Installed tile's mod
            Page::Installed => {
                let dir = self.dir_of(self.hovered(sel)?)?;
                self.locals().iter().find(|m| bare(&m.dir) == bare(dir))
            }
            Page::Browse | Page::Mine => None,
        }
    }

    /// B373: the kind tab is one the store doesn't carry yet (no rows: only the tabs and ○ work).
    pub(super) fn empty_kind(&self) -> bool {
        mods_kind::coming(self.kind).is_some()
    }

    /// B373: the installed mods on the kind tab (M8f9: courts and umpires on theirs).
    fn locals(&self) -> &[Mod] {
        self.of_kind(self.kind)
    }

    /// M8f9: the installed mods of kind `k`.
    fn of_kind(&self, k: Kind) -> &[Mod] {
        match k {
            Kind::Character => &self.list,
            Kind::Court => &self.courts,
            Kind::Umpire => &self.umpires,
            Kind::Textures | Kind::Other => &[],
        }
    }

    /// M8f9: every installed mod, of any kind (the Installed tab's count).
    fn installed_len(&self) -> usize {
        self.list.len() + self.courts.len() + self.umpires.len()
    }

    /// B373: each kind tab's count on this page (by `mods_kind::KINDS`).
    fn kind_counts(&self) -> [usize; 4] {
        mods_kind::KINDS.map(|k| match self.page {
            _ if mods_kind::coming(k).is_some() => 0,
            Page::Installed | Page::Upload => self.of_kind(k).len(),
            Page::Browse | Page::Mine => self.entries().iter().filter(|e| e.meta.kind == k).count(),
        })
    }

    /// Entry `e` installed: its zip's sha256 and folder.
    fn installed(&self, e: &Entry) -> Option<&(String, PathBuf)> {
        self.have.get(&e.key()).or_else(|| self.have.get(&format!("/{}", e.meta.id)))
    }

    /// Entry `e`'s tag: installed as listed, installed but different (an update), or not.
    fn state(&self, e: &Entry) -> Option<&'static str> {
        match self.installed(e) {
            None => None,
            Some((sha, _)) if *sha != e.meta.sha256 => Some("Update"),
            Some(_) => Some("Installed"),
        }
    }

    /// The browse entry under the cursor.
    fn hovered(&self, sel: usize) -> Option<&Entry> {
        if !self.grid() {
            return None;
        }
        self.items().get(sel.checked_sub(self.acts())?).copied()
    }
}

/// A mod folder without the `./` the roster's paths keep (`root` drops it).
fn bare(p: &Path) -> PathBuf {
    p.strip_prefix(".").unwrap_or(p).to_path_buf()
}

/// An uploader's name on their heading (`""`: the store doesn't say).
fn who(author: &str) -> &str {
    if author.is_empty() { "unknown uploader" } else { author }
}

/// Where a move from row `sel` goes (`acts` pills side by side in a row over the list, `n` rows in all): ←→ between
/// the pills, ↓ from them to the first mod, ↑ from the first mod back to them; ↑↓ wrap round the whole page.
fn nav(sel: usize, acts: usize, n: usize, k: Press) -> usize {
    let mods = n > acts;
    if sel < acts {
        if k.left || (k.up && !mods) {
            (sel + acts - 1) % acts
        } else if k.right || (k.down && !mods) {
            (sel + 1) % acts
        } else if k.down {
            acts
        } else if k.up {
            n - 1
        } else {
            sel
        }
    } else if k.down {
        if sel + 1 < n { sel + 1 } else { 0 }
    } else if k.up {
        if sel == acts { 0 } else { sel - 1 }
    } else {
        sel
    }
}

fn mb(n: u64) -> String {
    format!("{} MB", n.div_ceil(1 << 20))
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() > n { format!("{}...", s.chars().take(n - 3).collect::<String>()) } else { s.into() }
}

impl Menu {
    pub(super) fn mods_open(&mut self, out: &mut Vec<Effect>) {
        self.add.status.clear();
        self.add.top = 0;
        self.add.b.show = grid::Show::All;
        self.mods_page(Page::Browse, 0);
        self.go(Screen::Mods, out);
    }

    fn mods_page(&mut self, p: Page, sel: usize) {
        self.add.page = p;
        self.add.child = None;
        self.add.by = None;
        self.add.top = 0;
        self.sel = sel;
        // B383: Show in the rail says which grid it is; the search box waits for a press
        self.add.b.show = match p {
            Page::Installed => grid::Show::Installed,
            Page::Mine => grid::Show::Mine,
            _ if matches!(self.add.b.show, grid::Show::Installed | grid::Show::Mine) => grid::Show::All,
            _ => self.add.b.show,
        };
        (self.add.b.focus, self.add.b.osk, self.add.b.tile_at) = (false, None, 0);
        self.add.status = match p {
            Page::Upload => RULE.into(),
            _ => String::new(),
        };
        if matches!(p, Page::Upload | Page::Mine) {
            self.add.account = modstore::account().map(|a| a.0);
        }
        if p == Page::Mine && self.add.account.is_none() {
            self.add.status = "Sign in to see your uploads: Upload (top right), then Sign in.".into();
        }
        if p == Page::Mine && self.add.mine.is_none() && !self.add.busy {
            self.read_mine();
        }
        if p == Page::Browse && self.add.store.is_none() && !self.add.busy {
            self.add.busy = true;
            self.add.status = "Reading the mod store...".into();
            modstore::list(false);
        }
    }

    /// Page `i` (`PAGES`): B383: the header's Upload button (a tap on it on Upload: back to the grid), the rail's
    /// Show; on a phone Upload and My uploads put up the popup (no sign-in there).
    pub(super) fn mods_tab(&mut self, i: usize, out: &mut Vec<Effect>) {
        if PAGES[i] == self.add.page && self.add.by.is_none() {
            return;
        }
        if self.add.phone && matches!(PAGES[i], Page::Upload | Page::Mine) {
            self.add.nophone = Some(2);
            out.push(Effect::Sound(1));
            return;
        }
        self.mods_page(PAGES[i], 0);
        out.push(Effect::Sound(0));
    }

    /// B373: kind tab `k` (L2 / R2 or a tap): the page lists only that kind, from its first row.
    pub(super) fn mods_kind(&mut self, k: Kind, out: &mut Vec<Effect>) {
        if k == self.add.kind {
            return;
        }
        self.add.kind = k;
        self.add.child = None;
        self.add.by = None;
        self.add.top = 0;
        // B383: the search box (not typing into it) after every switch; another kind's games are another list
        self.sel = 0;
        let b = &mut self.add.b;
        (b.focus, b.tile_at, b.rail_at, b.game) = (false, 0, 0, None);
        out.push(Effect::Sound(0));
    }

    /// B383: Show `s` in the rail: Everything, Updates and Saved mods filter the store's mods, Installed and My
    /// uploads are their own grids; a filter keeps the cursor on its option, another page puts it on the search box.
    fn mods_show(&mut self, s: grid::Show, out: &mut Vec<Effect>) {
        let to = match s {
            grid::Show::Installed => Page::Installed,
            grid::Show::Mine => Page::Mine,
            _ => Page::Browse,
        };
        if self.add.phone && to == Page::Mine {
            self.add.nophone = Some(2);
            return out.push(Effect::Sound(1));
        }
        let page = to != self.add.page;
        if page || self.add.by.is_some() {
            self.mods_page(to, 0);
        }
        self.add.b.show = s;
        if s == grid::Show::Wish {
            self.add.wish();
        }
        (self.add.top, self.add.b.tile_at) = (0, 0);
        if let Some(i) = self.add.rail().iter().position(|o| *o == grid::Opt::Show(s)) {
            // another page (Installed, My uploads, back to the store's) starts on the search box like a kind switch
            self.sel = if page { 0 } else { 2 + self.add.words().len() + i };
            self.add.b.rail_at = i;
        }
        out.push(Effect::Sound(0));
    }

    /// M19: reads the player's uploads again.
    fn read_mine(&mut self) {
        if let Some(name) = self.add.account.clone() {
            self.add.busy = true;
            self.add.status = "Reading your uploads from the store...".into();
            modstore::mine(name);
        }
    }

    /// B221/B202: what the pointer's row moves: the open child's button, else the list's row.
    pub(super) fn mods_cursor(&mut self) -> &mut usize {
        if let Some(w) = &mut self.add.wipe {
            return w;
        }
        match (&mut self.add.child, &mut self.add.b.osk) {
            (Some(c), _) => &mut c.at,
            // B372: Browse's on-screen keyboard's key
            (None, Some(k)) => k,
            (None, None) => &mut self.sel,
        }
    }

    pub(super) fn mods_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        use Effect::Sound;
        // B208: the phone popup takes every press; ✕ or ○ closes it
        if self.add.nophone.is_some() {
            if k.ok || k.back {
                self.add.nophone = None;
                out.push(Sound(1));
            }
            return;
        }
        if self.add.wipe.is_some() {
            return self.wipe_step(k, out);
        }
        let n = self.add.rows();
        // B373: a kind the store doesn't carry yet has no rows: only the tabs and ○ do anything there
        if n == 0 {
            self.sel = 0;
            if k.kind_prev || k.kind_next {
                self.add.child = None;
            } else {
                if k.back {
                    self.mods_back(out);
                }
                return;
            }
        }
        self.sel = self.sel.min(n.max(1) - 1);
        if k.kind_prev || k.kind_next {
            // B373: the kind tabs, round (an open child closes); B383: only the ones shown
            let to = mods_kind::step(self.add.kind, k.kind_prev, &self.add.kinds_shown());
            if to != self.add.kind {
                self.add.child = None;
                self.mods_kind(to, out);
            }
            return;
        }
        // B372: the on-screen keyboard takes every press while up; a press forgets the tile a tap chose
        if self.add.grid() && self.add.b.osk.is_some() {
            return self.osk_step(k, out);
        }
        if k != Press::default() {
            self.add.b.tapped = None;
        }
        if let Some(mut c) = self.add.child.take() {
            match c.step(k, out) {
                Got::Open => self.add.child = Some(c),
                Got::Close => self.add.status = if self.add.page == Page::Upload { RULE.into() } else { String::new() },
                Got::Pick(i) => self.child_pick(c, i, out),
            }
        } else if self.add.grid() {
            if k.back {
                self.mods_back(out);
            } else if self.browse_move(k) {
                if !self.add.busy {
                    self.add.status.clear();
                }
                out.push(Sound(0));
            } else {
                self.browse_press(k, out);
            }
        } else {
            self.list_step(k, n, out);
        }
        // ○ went back to the main menu: its cursor isn't ours
        if self.screen != Screen::Mods {
            return;
        }
        let n = self.add.rows();
        self.sel = self.sel.min(n.max(1) - 1);
        if self.add.grid() && n > 0 {
            // B383: the search box types only while it was pressed and the cursor is on it
            if self.add.slot(self.sel) != grid::Slot::Search {
                self.add.b.focus = false;
            }
            return self.browse_scroll();
        }
        // the mods scroll under the pills; `top` is the first mod drawn
        let shown = shown(&self.grid);
        let v = &mut self.add;
        let acts = v.acts();
        if let Some(i) = self.sel.checked_sub(acts) {
            v.top = v.top.min(i).max((i + 1).saturating_sub(shown));
        }
        v.top = v.top.min((n - acts).saturating_sub(shown));
    }

    /// B202: opens the child buttons `opts` (and Cancel) on the chosen row, saying `status`.
    fn open_child(&mut self, opts: &[&'static str], status: String, out: &mut Vec<Effect>) {
        self.add.child = Some(Child::new(self.sel, opts));
        self.add.status = status;
        out.push(Effect::Sound(0));
    }

    /// The child's button `i` on its row.
    fn child_pick(&mut self, c: Child, i: usize, out: &mut Vec<Effect>) {
        use Effect::Sound;
        let s = c.row;
        match self.add.page {
            // B383: Remove all (a heading word), or Remove on an installed tile
            Page::Installed if matches!(self.add.slot(s), grid::Slot::Word(_)) => self.remove_all(out),
            Page::Installed => self.browse_remove(s, out),
            Page::Upload if s == 0 => {
                let v = &mut self.add;
                v.status = match (&v.account, i) {
                    (Some(_), _) => {
                        modstore::sign_out();
                        v.account = None;
                        v.mine = None;
                        "Signed out. The game no longer keeps your token.".into()
                    }
                    (None, 0) => {
                        modstore::open(modstore::TOKENS);
                        // the child stays, on Paste token
                        v.child = Some(Child { at: 1, ..c });
                        if cfg!(target_arch = "wasm32") {
                            "On the Hugging Face page that opened, create a token with Write access and copy it. Press Ctrl+V here, then choose Paste token.".into()
                        } else {
                            "On the Hugging Face page that opened, create a token with Write access and copy it. Then choose Paste token.".into()
                        }
                    }
                    (None, _) => {
                        v.paste = true;
                        "Checking the token...".into()
                    }
                };
                out.push(Sound(2));
            }
            Page::Upload => {
                let mods = if s == 1 { self.add.locals().to_vec() } else { vec![self.add.locals()[s - UP_TOP].clone()] };
                match modstore::account() {
                    Some((name, token)) => {
                        self.queue(mods, name, token);
                        out.push(Sound(2));
                    }
                    None => {
                        self.add.account = None;
                        self.add.status = "You've been signed out: choose Sign in, then upload again.".into();
                        out.push(Sound(1));
                    }
                }
            }
            Page::Mine => {
                let Some(e) = self.add.hovered(s).cloned() else { return };
                match modstore::account() {
                    Some((_, token)) => {
                        self.add.busy = true;
                        self.add.status = format!("Taking {} off the store...", e.meta.name);
                        modstore::remove(&e, token);
                        out.push(Sound(2));
                    }
                    None => {
                        self.add.account = None;
                        self.add.status = "You've been signed out: sign in again under Upload (top right).".into();
                        out.push(Sound(1));
                    }
                }
            }
            // B372: Remove on an installed tile
            Page::Browse => {
                let _ = i;
                self.browse_remove(s, out);
            }
        }
    }

    /// B255: installs store mod `e` (queued behind any under way), unless it's on its way already.
    fn install(&mut self, e: Entry, out: &mut Vec<Effect>) {
        if self.add.queue.iter().chain(&self.add.doing).any(|q| q.key() == e.key()) {
            self.add.status = format!("{} is on its way already.", e.meta.name);
            return out.push(Effect::Sound(1));
        }
        let (up, n) = (self.settings.0[super::UPSCALED_DOWNLOADS], self.settings.downloads_at_once());
        let queued = self.add.doing.len() >= n;
        let name = e.meta.name.clone();
        self.add.enqueue(e, up, n);
        if queued {
            self.add.status = format!("{name} is queued: it installs after the ones before it.");
        }
        out.push(Effect::Sound(2));
    }

    /// B255: Download all on an author's list or on their heading: queues every one of `list` (`author`'s mods) not
    /// installed at the same or a newer version.
    fn download_all(&mut self, author: &str, list: Vec<Entry>, out: &mut Vec<Effect>) {
        let (up, at_once) = (self.settings.0[super::UPSCALED_DOWNLOADS], self.settings.downloads_at_once());
        let todo: Vec<Entry> = list.iter().filter(|e| !self.add.skips(e)).cloned().collect();
        let (n, skipped) = (todo.len(), list.len() - todo.len());
        for e in todo {
            self.add.enqueue(e, up, at_once);
        }
        let author = who(author);
        self.add.status = match (n, skipped) {
            (0, _) => format!("All of {author}'s mods are installed or queued already."),
            (n, 0) if at_once > 1 && n > 1 => format!("Downloading {n} mods by {author}, {} at a time.", at_once.min(n)),
            (n, 0) => format!("Downloading {n} mods by {author}, one after another."),
            (n, k) => format!("Downloading {n} mods by {author} ({k} installed or queued already)."),
        };
        out.push(Effect::Sound(if n == 0 { 1 } else { 2 }));
    }

    /// ○: an author's list goes back to the whole store's, on the mod it was opened from; Upload to the grid; else
    /// the main menu.
    fn mods_back(&mut self, out: &mut Vec<Effect>) {
        if self.add.page == Page::Upload {
            self.mods_page(Page::Browse, 0);
        } else if let Some(b) = self.add.by.as_ref().filter(|_| self.add.page == Page::Browse) {
            self.sel = b.from;
            self.add.narrow(None, 0);
            self.add.status.clear();
        } else {
            self.screen = Screen::Main;
            self.sel = self.main_at(super::home::Row::Mods);
            // B205: ask once per change; Later leaves the note on the Mods row
            if std::mem::take(&mut self.add.fresh) {
                self.add.ask = Some(0);
            }
        }
        out.push(Effect::Sound(1));
    }

    /// The list's own press (no child open).
    fn list_step(&mut self, k: Press, n: usize, out: &mut Vec<Effect>) {
        use Effect::Sound;
        let to = nav(self.sel, self.add.acts(), n, k);
        if to != self.sel {
            self.sel = to;
            if !matches!(self.add.page, Page::Upload | Page::Mine) && !self.add.busy {
                self.add.status.clear();
            }
            out.push(Sound(0));
        }
        if k.back {
            return self.mods_back(out);
        }
        if !k.ok {
            return;
        }
        let s = self.sel;
        match self.add.page {
            Page::Installed => self.installed_step(s, out),
            // (Browse's presses are `browse_press`)
            Page::Browse => {}
            Page::Upload => {
                if self.add.busy {
                    self.add.status = "Wait for the current upload to finish.".into();
                } else if s == 0 {
                    self.account_step(out);
                } else if self.add.account.is_none() {
                    self.add.status = "Uploading needs a Hugging Face account: choose Sign in first.".into();
                    out.push(Sound(1));
                } else if s == 1 && self.add.locals().is_empty() {
                    self.add.status = "No mods installed to upload.".into();
                    out.push(Sound(1));
                } else if s == 1 {
                    self.open_child(&["Upload all"], format!("{RULE} Upload all {} mods?", self.add.locals().len()), out);
                } else {
                    let m = &self.add.locals()[s - UP_TOP];
                    if matches!(self.add.ups.get(&m.dir), Some(Up::Queued | Up::Uploading)) {
                        self.add.status = format!("{} is uploading already.", m.name);
                    } else {
                        let name = m.name.clone();
                        self.open_child(&["Upload"], format!("{RULE} Upload {name}?"), out);
                    }
                }
            }
            Page::Mine => self.mine_step(s, out),
        }
    }

    /// B250: uploads `mods` in the background as account `name`, one Hub commit each, each marked Queued till its
    /// turn; ones already on their way are left out.
    fn queue(&mut self, mods: Vec<Mod>, name: String, token: String) {
        let v = &mut self.add;
        let mods: Vec<Mod> = mods.into_iter().filter(|m| !matches!(v.ups.get(&m.dir), Some(Up::Queued | Up::Uploading))).collect();
        v.status = match mods.len() {
            0 => "Those are uploading already.".into(),
            1 => format!("Uploading {} in the background...", mods[0].name),
            n => format!("Uploading {n} mods one at a time in the background. You can keep using the menu."),
        };
        let ups = mods
            .into_iter()
            .map(|m| {
                v.ups.insert(m.dir.clone(), Up::Queued);
                let card = super::previews::card(&v.iso, &m);
                let stats = v.stats.get(&m.dir).copied();
                (m, stats, card)
            })
            .collect::<Vec<_>>();
        if !ups.is_empty() {
            modstore::publish(ups, name, token);
        }
    }

    /// The account row: signed out, Open token page / Paste token; signed in, Sign out.
    fn account_step(&mut self, out: &mut Vec<Effect>) {
        match self.add.account.clone() {
            Some(name) => self.open_child(&["Sign out"], format!("Signed in to Hugging Face as {name}. Sign out? The game will forget your token."), out),
            None => self.open_child(
                &["Open token page", "Paste token"],
                "Uploading needs a free Hugging Face token. Open token page, create one with Write access and copy it, then choose Paste token.".into(),
                out,
            ),
        }
    }

    /// M19: My uploads: Take down on one of the player's uploads.
    fn mine_step(&mut self, s: usize, out: &mut Vec<Effect>) {
        use Effect::Sound;
        if self.add.busy {
            self.add.status = "Wait for the store to finish first.".into();
        } else if s == 0 && self.add.account.is_none() {
            self.add.status = "Sign in to see your uploads: Upload (top right), then Sign in.".into();
            out.push(Sound(1));
        } else if s == 0 {
            self.read_mine();
            out.push(Sound(2));
        } else if let Some(e) = self.add.hovered(s).cloned() {
            if e.verified {
                self.open_child(&["Ask to delete"], format!("{} is part of the store now. Ask a maintainer to delete it?", e.meta.name), out);
            } else {
                self.open_child(&["Take down"], format!("Take {} off the store? This closes its pull request.", e.meta.name), out);
            }
        }
    }

    /// B445: Remove all uploads (a word in My uploads' heading): the warning popup, on Cancel, if there's any to remove.
    fn wipe_ask(&mut self, out: &mut Vec<Effect>) {
        use Effect::Sound;
        let n = self.add.mine.as_ref().map_or(0, Vec::len);
        if self.add.busy {
            self.add.status = "Wait for the store to finish first.".into();
            out.push(Sound(1));
        } else if self.add.account.is_none() {
            self.add.status = "Sign in with Upload (top right) to see your uploads.".into();
            out.push(Sound(1));
        } else if n == 0 {
            self.add.status = "None of your mods are on the store.".into();
            out.push(Sound(1));
        } else {
            self.add.wipe = Some(1);
            out.push(Sound(0));
        }
    }

    /// B445: one press while the warning is up (it takes them all): ↑↓←→ move between Remove all and Cancel, ✕
    /// chooses, ○ is Cancel.
    fn wipe_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        use Effect::Sound;
        let Some(a) = self.add.wipe else { return };
        if k.up || k.down || k.left || k.right {
            self.add.wipe = Some(1 - a.min(1));
            out.push(Sound(0));
        } else if k.back || (k.ok && a != 0) {
            self.add.wipe = None;
            out.push(Sound(1));
        } else if k.ok {
            self.add.wipe = None;
            self.wipe_all(out);
        }
    }

    /// B445: takes every one of the player's uploads off the store (`modstore::remove` each, one after another as
    /// each comes back): an open pull request is closed, a merged mod gets its delete asked for.
    fn wipe_all(&mut self, out: &mut Vec<Effect>) {
        let all: VecDeque<Entry> = self.add.mine.clone().unwrap_or_default().into();
        if modstore::account().is_none() {
            self.add.account = None;
            self.add.status = "Signed out: sign in again on the Upload tab.".into();
            return out.push(Effect::Sound(1));
        }
        self.add.wiped = (0, 0, all.len());
        self.add.wiping = all;
        self.add.wipe_next();
        out.push(Effect::Sound(2));
    }

    fn installed_step(&mut self, s: usize, out: &mut Vec<Effect>) {
        use Effect::Sound;
        if s == 0 && self.add.phone {
            self.add.nophone = Some(0);
            out.push(Sound(1));
        } else if s == 0 {
            // a pad's or key's ✕ (a tap lands on the page's input itself)
            #[cfg(target_arch = "wasm32")]
            crate::web::mod_pick();
            // desktop: the system's file picker, once at a time
            #[cfg(not(target_arch = "wasm32"))]
            if !std::mem::replace(&mut self.add.busy, true) {
                modstore::pick(root());
            }
            self.add.status = "Choose a mod .zip file: a character, court or umpire.".into();
            out.push(Sound(2));
        } else if s == 1 {
            if let Err(e) = reload(out) {
                self.add.status = e;
            }
        } else if s == 2 {
            let n = self.add.locals().iter().filter(|m| ours(m)).count();
            if n == 0 {
                self.add.status = "No mods the game installed to remove.".into();
                out.push(Sound(1));
            } else {
                self.open_child(&["Remove all"], format!("Remove all {n} mods the game installed? Mods you added by hand stay. Reload afterwards to update the select screens."), out);
            }
        } else {
            // B383: an installed mod's tile
            let Some(m) = self.add.local(s) else { return };
            let (name, dir) = (m.name.clone(), m.dir.clone());
            if ours(m) {
                self.open_child(&["Remove"], format!("Remove {name}? Reload afterwards to take it off the select screens."), out);
            } else {
                self.add.status = format!("{name} was added by hand: delete {} to remove it.", dir.display());
                out.push(Sound(1));
            }
        }
    }

    /// B335: Remove all: every installed mod the game put in ([`ours`]).
    fn remove_all(&mut self, out: &mut Vec<Effect>) {
        self.add.locals().iter().filter(|m| ours(m)).for_each(delete);
        self.add.refresh();
        self.add.changed();
        self.add.status = "Removed every mod the game installed. Reload to take them off the select screens.".into();
        out.push(Effect::Sound(1));
    }

    /// Removes installed mod `i` (its child's Remove).
    fn remove(&mut self, i: usize, out: &mut Vec<Effect>) {
        let name = self.add.locals()[i].name.clone();
        delete(&self.add.locals()[i]);
        self.add.refresh();
        self.add.changed();
        self.add.status = format!("Removed {name}. Reload to take it off the select screens.");
        out.push(Effect::Sound(1));
    }
}

/// Deletes installed mod `m`: from memory, the browser's storage, and (a store download's own folder, `mods/<id>`,
/// `mods/<id>-2`, …; a pack's mods are left to their pack) the disk.
fn delete(m: &Mod) {
    crate::modfs::remove(&m.dir);
    #[cfg(target_arch = "wasm32")]
    crate::web::mod_drop(&m.folder());
    #[cfg(not(target_arch = "wasm32"))]
    if crate::modfs::installed_by_game(root(), &m.dir, &m.id) {
        let _ = std::fs::remove_dir_all(&m.dir);
    }
}

/// Whether installed mod `m` is the game's to delete: on desktop only a store download / Add mod install
/// (`mods/<id>`); a folder put in by hand (a pack, a link to the mods repo, a renamed folder) is left to whoever put
/// it there. Every mod in the browser.
fn ours(m: &Mod) -> bool {
    cfg!(target_arch = "wasm32") || crate::modfs::installed_by_game(root(), &m.dir, &m.id)
}

/// Reload: the browser reloads the page; the desktop game starts itself again with the same arguments and closes (the
/// select's roster, the mods' textures and sounds are all read at startup).
fn reload(out: &mut Vec<Effect>) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = out;
        crate::web::reload();
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let exe = std::env::current_exe().map_err(|e| format!("Couldn't restart: {e}. Restart the game by hand."))?;
        std::process::Command::new(exe)
            .args(std::env::args_os().skip(1))
            .spawn()
            .map_err(|e| format!("Couldn't restart: {e}. Restart the game by hand."))?;
        out.push(Effect::Quit);
        Ok(())
    }
}

/// The Reload / Later popup's pills (640×448): their tops, 40 px apart as the pause dialog's.
const ASK_TOP: f32 = 226.0;

impl Menu {
    /// B205: one press on the main menu while the popup is up (it takes them all): up/down/left/right move between
    /// Reload and Later, ✕ chooses; ○ does nothing (B387: only Later closes it, and Browse then keeps a Reload pill
    /// while the change stands). Returns whether the popup was up.
    pub(super) fn ask_step(&mut self, k: Press, out: &mut Vec<Effect>) -> bool {
        let Some(a) = self.add.ask else { return false };
        if k.up || k.down || k.left || k.right {
            self.add.ask = Some(1 - a);
            out.push(Effect::Sound(0));
        } else if k.ok && a == 0 {
            out.push(Effect::Sound(2));
            if let Err(e) = reload(out) {
                // the desktop couldn't start itself again: say so on the Mods row's note and close
                self.add.status = e;
                self.add.ask = None;
            }
        } else if k.ok {
            self.add.ask = None;
            out.push(Effect::Sound(1));
        }
        true
    }
}

/// B205: mods changed since the start (the main menu's note on its Mods row) and the popup isn't up.
/// B372: the legend on Browse (None: another page, or a child or popup up).
pub(super) fn browse_legend(m: &Menu) -> Option<Vec<(super::legend::Act, &'static str)>> {
    (m.add.grid() && !m.add.empty_kind() && m.add.child.is_none() && m.add.nophone.is_none() && m.add.wipe.is_none()).then(|| grid::legend(m))
}

pub(super) fn reload_due(m: &Menu) -> bool {
    m.add.changed && m.add.ask.is_none()
}

/// The main screen's Mods picture: which installed mods are the store's (their zips' sha256), read once at startup
/// so its Updates and Store facts are known before the Mods page has been opened.
pub(super) fn prime(m: &mut Menu) {
    if m.add.have.is_empty() {
        m.add.have = m.mods.iter().filter_map(|md| modstore::stamp(&md.dir).map(|s| (format!("{}/{}", s.author, md.id), (s.sha256, md.dir.clone())))).collect();
    }
}

/// The installed mods' folders the store has an update for, once it's listed.
pub(super) fn updates(m: &Menu) -> Option<Vec<&Path>> {
    let store = m.add.store.as_ref()?;
    Some(store.iter().filter(|e| m.add.state(e) == Some("Update")).filter_map(|e| m.add.installed(e)).map(|(_, dir)| dir.as_path()).collect())
}

/// How many of the store's mods aren't installed, once it's listed.
pub(super) fn fresh_len(m: &Menu) -> Option<usize> {
    Some(m.add.store.as_ref()?.iter().filter(|e| !m.add.have.contains_key(&e.key())).count())
}

/// The store's mod count, once listed.
pub(super) fn store_len(m: &Menu) -> Option<usize> {
    m.add.store.as_ref().map(Vec::len)
}

impl Menu {
    /// B297: reads the store's listing (cached an hour, `modstore::list`) for the main menu's New in the store
    /// column, once.
    pub(super) fn want_store(&mut self) {
        if self.add.store.is_none() && !self.add.busy {
            self.add.busy = true;
            modstore::list(false);
        }
    }
}

/// B297: the main menu's New in the store column in `[x, y, w, h]` (a wide window): the store's mods not installed
/// here, newest uploads (open pull requests) first, each a chip in its play style's colour with its name and the
/// game it's from.
pub(super) fn news(d: &mut Draw, m: &Menu, [x, y, w, h]: [f32; 4]) {
    d.box3([x, y, w, h], 16.0, [Some(INK), Some(WHITE), Some([128.0; 3])], 118.0);
    let muted = [45.0, 40.0, 52.0];
    d.text("NEW IN THE STORE", x + 12.0, y + 10.0, 12.0, muted);
    let Some(store) = &m.add.store else {
        d.text(if m.add.busy { "Reading the store..." } else { "Store not read yet." }, x + 12.0, y + 32.0, 14.0, muted);
        return;
    };
    let fresh: Vec<&Entry> = store.iter().filter(|e| !e.verified).chain(store.iter().filter(|e| e.verified)).filter(|e| !m.add.have.contains_key(&e.key())).collect();
    if fresh.is_empty() {
        d.text("Nothing new.", x + 12.0, y + 32.0, 14.0, muted);
    }
    let rows = ((h - 40.0) / 46.0) as usize;
    for (k, e) in fresh.iter().take(rows).enumerate() {
        let ry = y + 30.0 + 46.0 * k as f32;
        d.box3([x + 8.0, ry, w - 16.0, 40.0], 10.0, [None, None, Some([122.0, 118.0, 125.0])], 128.0);
        let style = e.meta.style.unwrap_or(0).min(3) as usize;
        d.box3([x + 12.0, ry + 4.0, 32.0, 32.0], 8.0, [Some(INK), None, Some(CARD_FILL[style])], 128.0);
        let initial: String = e.meta.name.chars().take(1).collect();
        d.centred(&initial.to_uppercase(), x + 28.0, ry + 9.0, 22.0, WHITE);
        super::home::fit_text(d, &e.meta.name, x + 52.0, ry + 4.0, 16.0, w - 68.0, INK);
        super::home::fit_text(d, &e.meta.source, x + 52.0, ry + 22.0, 12.0, w - 68.0, muted);
    }
}

/// B205, on the main menu: the note beside the Mods row while mods changed since the start, and the popup over a
/// dimmed menu in the pause dialog's style (`menu.rs`: rounded box, title, a pill per choice, the chosen one bright,
/// the hand beside it).
pub(super) fn ask_layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let Some(a) = m.add.ask else { return };
    use super::super::menu::{CHOSEN, PANEL, UNCHOSEN};
    // the menu's own hand goes; the popup has its own
    if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
        d.out.remove(i);
    }
    // B387: a tap off the popup does nothing, as ○: only Later closes it
    d.popup(None);
    d.anchor = super::Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 80.0);
    d.anchor = super::Anchor::Centre;
    d.box3([160.0, ASK_TOP - 70.0, 320.0, 70.0 + 72.0 + 16.0], 16.0, [Some(INK), Some(WHITE), Some(PANEL)], 128.0);
    d.outlined(|d| d.label("Mods changed.", 320.0, ASK_TOP - 56.0, 22.0, 288.0, WHITE));
    d.outlined(|d| d.label("Reload now to use them?", 320.0, ASK_TOP - 32.0, 22.0, 288.0, WHITE));
    for (k, word) in ["Reload", "Later"].into_iter().enumerate() {
        let y = ASK_TOP + 40.0 * k as f32;
        let on = k == a;
        d.pill_at([176.0, y, 288.0, 32.0], [INK, WHITE, if on { CHOSEN } else { UNCHOSEN }], 128.0);
        d.hit([176.0, y, 288.0, 32.0], super::pointer::Act::Row(k, super::pointer::OK));
        d.label(word, 320.0, y + 6.0, 20.0, 160.0, if on { WHITE } else { UNCHOSEN.map(|c| c * 1.4) });
    }
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [116.0 + hand_x, ASK_TOP + 40.0 * a as f32, 64.0, 32.0], WHITE, 128.0);
    d.said = "Mod changes take effect after a restart. Reload restarts the game now; Later carries on without them.".into();
}

/// The list box and the side panel on the view `g` fits, from under the title band down to the legend strip: the
/// panel right of the list (under it in a portrait window), as Settings' pages (`options::boxes`).
fn boxes(g: &super::Grid) -> ([f32; 4], [f32; 4]) {
    let [l, _, w, _] = g.strip;
    // B383: under the header's row on a narrow view (Upload and the downloads' pill), as the grid's
    let y = grid::lay(g).rail[1];
    let bottom = g.detail[1] + g.detail[3] + 32.0;
    // a portrait view: the detail panel spans the width
    if g.detail[2] >= g.strip[2] {
        let h = ((bottom - y) * 0.55).max(150.0);
        ([l, y, w, h], [l, y + h + 8.0, w, bottom - y - h - 8.0])
    } else {
        let dw = 290.0f32.min(w * 0.47);
        ([l, y, w - dw - 8.0, bottom - y], [l + w - dw, y, dw, bottom - y])
    }
}

/// The pills' row and the first card's top in the list box.
const PILLS_Y: f32 = 10.0;
const CARDS_Y: f32 = 50.0;

/// The mod cards the list shows at once (the rest scroll).
fn shown(g: &super::Grid) -> usize {
    let (list, _) = boxes(g);
    (((list[3] - CARDS_Y - 8.0 + CARD_PITCH - CARD_H) / CARD_PITCH) as usize).max(1)
}

/// Row `r`'s rect: a pill in the row over the list, or a mod's card (None: scrolled off).
fn row_rect(v: &View, list: [f32; 4], shown: usize, r: usize) -> Option<[f32; 4]> {
    let [x, y, w, _] = list;
    let pills = pill_ws(v);
    if r < v.acts() {
        let x = x + 12.0 + pills[..r].iter().map(|pw| pw + 8.0).sum::<f32>();
        return Some([x, y + PILLS_Y, pills[r], 30.0]);
    }
    let k = (r - v.acts()).checked_sub(v.top).filter(|k| *k < shown)?;
    Some([x + 22.0, y + CARDS_Y + CARD_PITCH * k as f32, w - 40.0, CARD_H])
}

/// The pills' widths.
fn pill_ws(v: &View) -> &'static [f32] {
    match v.page {
        Page::Installed => &[100.0, 84.0, 110.0],
        Page::Browse if v.by.is_some() => &[140.0, 110.0],
        Page::Browse | Page::Mine => &[110.0],
        Page::Upload => &[170.0, 120.0],
    }
}

/// The page's pills over its list.
fn pill_words(v: &View) -> Vec<String> {
    match v.page {
        Page::Installed => vec!["Add mod".into(), "Reload".into(), "Remove all".into()],
        Page::Browse if v.by.is_some() => vec!["Download all".into(), "All mods".into()],
        Page::Browse | Page::Mine => vec!["Refresh".into()],
        Page::Upload => vec![v.account.as_ref().map_or("Sign in".into(), |a| format!("Signed in: {}", short(a, 14))), "Upload all".into()],
    }
}

/// What a card shows: its face, name, the line under it, its chips (words, colour) and play style (its fill).
struct Card {
    face: Option<(Tex, [f32; 4])>,
    name: String,
    sub: String,
    chips: Vec<(&'static str, [f32; 3])>,
    style: Option<usize>,
}

/// The chips' words' colours (on white, the lobby's): Verified and Uploaded green, Update orange, Error red.
const GREEN: [f32; 3] = [30.0, 61.0, 15.0];
const ORANGE: [f32; 3] = [90.0, 48.0, 3.0];
const RED: [f32; 3] = [100.0, 15.0, 15.0];
const GREY: [f32; 3] = [55.0; 3];

/// Installed mod `md`'s face in the select's roster (built at startup: one added since has none yet); M8f9/B400:
/// an umpire mod's head from its `previews/umpire.png`, as the umpire select's card has it.
fn face_of(m: &Menu, dir: &Path) -> Option<(Tex, [f32; 4])> {
    mod_of(m, dir).map(|j| super::cell_face(Some(super::Who::Mod(j)))).or_else(|| super::umpire_mods::face_of(m, dir))
}

/// Installed mod `dir`'s place in the select's roster.
fn mod_of(m: &Menu, dir: &Path) -> Option<usize> {
    // the roster's dirs keep `./mods`'s `./`, the page's (`root`) drop it
    let bare = |p: &Path| p.strip_prefix(".").unwrap_or(p).to_path_buf();
    m.mods.iter().position(|x| bare(&x.dir) == bare(dir))
}

/// The faces of the installed mods the list shows now, top first: what `previews.rs` bakes and reads first while
/// this page is up, so its cards fill in without a select visit.
pub(super) fn faces(m: &Menu) -> Vec<super::Who> {
    let v = &m.add;
    if v.grid() {
        return grid::faces(m);
    }
    let shown = shown(&m.grid);
    (v.acts() + v.top..v.rows().min(v.acts() + v.top + shown))
        .filter_map(|r| match v.local(r) {
            Some(md) => mod_of(m, &md.dir),
            None => v.hovered(r).and_then(|e| v.installed(e)).and_then(|(_, dir)| mod_of(m, dir)),
        })
        .map(super::Who::Mod)
        .collect()
}

fn card_of(m: &Menu, r: usize) -> Option<Card> {
    let v = &m.add;
    if let Some(md) = v.local(r) {
        let style = v.stats.get(&md.dir).map(|s| s.0);
        let mut sub = if md.source.is_empty() { format!("Folder {}", md.folder()) } else { format!("From {}", md.source) };
        if let Some(s) = style {
            sub = format!("{sub} - {}", super::STYLE_NAMES[s.min(3)]);
        }
        let chips = match v.ups.get(&md.dir).filter(|_| v.page == Page::Upload) {
            Some(u) => vec![up_tag(u)],
            None => modstore::stamp(&md.dir).map(|_| vec![("From store", GREY)]).unwrap_or_default(),
        };
        return Some(Card { face: face_of(m, &md.dir), name: md.name.clone(), sub, chips, style });
    }
    let e = v.hovered(r)?;
    let mt = &e.meta;
    // (who uploaded it is its heading's, or the author's list's title)
    let sub = if mt.source.is_empty() { String::new() } else { format!("From {}", mt.source) };
    let mut chips = Vec::new();
    if e.verified {
        chips.push(("Verified", GREEN));
    }
    match v.jobs.get(&e.key()).map(|j| j.0).or_else(|| v.state(e)) {
        Some(t @ "Update") => chips.push((t, ORANGE)),
        Some(t @ "Error") => chips.push((t, RED)),
        Some(t) => chips.push((t, GREY)),
        None => {}
    }
    let face = v.installed(e).and_then(|(_, dir)| face_of(m, dir));
    Some(Card { face, name: mt.name.clone(), sub, chips, style: mt.style.map(usize::from) })
}

/// A mod's card (the lobby's seat rows, `seat_row`): filled its play style's colour, gold-ringed when chosen, its
/// face (or its initial), name and line, its chips at the right end.
fn card(d: &mut Draw, [x, y, w, h]: [f32; 4], c: &Card, on: bool) {
    let fill = c.style.map_or([56.0, 56.0, 67.0], |s| CARD_FILL[s.min(3)]);
    d.box3([x, y, w, h], 12.0, [Some(INK), Some(if on { GOLD } else { WHITE }), Some(fill)], 128.0);
    let fx = x + h / 2.0 + 2.0;
    match c.face {
        Some(f) => super::face_frame(d, f, fx, y + h / 2.0, h - 6.0, WHITE),
        None => {
            let r = (h - 6.0) / 2.0;
            for (src, dst) in super::super::menu_art::pill_quads(0, [fx - r, y + h / 2.0 - r, 2.0 * r, 2.0 * r]) {
                d.q(Tex::Pill, src, dst, [40.0; 3], 128.0);
            }
            let initial: String = c.name.chars().take(1).collect();
            d.centred(&initial.to_uppercase(), fx, y + h / 2.0 - 10.0, 20.0, WHITE);
        }
    }
    let mut cx = x + w - 6.0;
    for (words, rgb) in c.chips.iter().rev() {
        let cw = d.width(words, 12.0) + 16.0;
        cx -= cw;
        d.pill_at([cx, y + (h - 18.0) / 2.0, cw, 18.0], [[60.0; 3], WHITE, WHITE], 128.0);
        d.label(words, cx + cw / 2.0, y + (h - 12.0) / 2.0, 12.0, cw - 6.0, *rgb);
        cx -= 4.0;
    }
    let (tx, tw) = (fx + h / 2.0 + 4.0, cx - fx - h / 2.0 - 8.0);
    // too long for the room left by the chips: smaller, never squeezed
    let fit = |d: &Draw, s: &str, size: f32| size * (tw / d.width(s, size).max(1.0)).min(1.0);
    let (ns, ss) = (fit(d, &c.name, 17.0), fit(d, &c.sub, 11.0));
    super::left(d, &c.name, tx, y + 3.0 + (17.0 - ns) / 2.0, ns, tw, WHITE);
    super::left(d, &c.sub, tx, y + 22.0 + (11.0 - ss) / 2.0, ss, tw, [118.0; 3]);
}


pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let v = &m.add;
    d.frame(5, 6);
    if v.grid() && !v.empty_kind() {
        // B372: the picture grid (B383: Browse, Installed and My uploads; it draws the header too)
        grid::layout(d, m, hand_x);
        if v.child.is_some() {
            side_panel(d, m, grid::lay(&m.grid).detail, hand_x);
        }
        if let Some(r) = v.nophone {
            nophone(d, if r == 0 { "Add mod" } else { "Upload my mod" });
        }
        if let Some(a) = v.wipe {
            wipe_popup(d, v, a, hand_x);
        }
        return;
    }
    // B383: the header (the kinds, Upload, the downloads), then Upload's list or a kind still to come
    grid::header(d, m, &grid::lay(&m.grid));
    let (list, side) = boxes(&m.grid);
    for b in [list, side] {
        d.box3(b, 14.0, [Some(INK), Some(WHITE), Some(CREAM)], 128.0);
    }
    let shown = shown(&m.grid);
    let words = pill_words(v);
    let pills = pill_ws(v);
    let mut hand = None;
    for r in 0..v.rows() {
        let Some(rect) = row_rect(v, list, shown, r) else { continue };
        let on = r == m.sel;
        if r < v.acts() {
            // B208: Add mod can't pick a file on a phone
            let off = v.phone && v.page == Page::Installed && r == 0;
            super::side_button(d, rect, &words[r], on && !off, false);
            if off {
                d.box3(rect, 15.0, [None, None, Some([40.0; 3])], 80.0);
            }
        } else if let Some(c) = card_of(m, r) {
            card(d, rect, &c, on);
        }
        d.hit(rect, super::pointer::Act::Row(r, super::pointer::OK));
        if on {
            hand = Some(rect);
        }
    }
    if let Some(b) = v.by.as_ref().filter(|_| v.page == Page::Browse) {
        // B255: whose mods these are, beside the pills
        let x = list[0] + 12.0 + pills.iter().map(|w| w + 8.0).sum::<f32>() + 4.0;
        super::left(d, &format!("Mods by {}", b.author), x, list[1] + PILLS_Y + 7.0, 16.0, list[0] + list[2] - x - 12.0, BROWN);
    }
    let mods = v.rows() - v.acts();
    if mods_kind::coming(v.kind).is_some() {
        mods_kind::empty(d, v.kind, list);
    } else if mods == 0 {
        let none = match v.page {
            Page::Installed | Page::Upload => "No mods installed yet: install one from the store first.",
            Page::Browse if v.busy => "Reading the mod store...",
            Page::Browse => "Nobody has shared a mod yet.",
            Page::Mine if v.account.is_none() => "Sign in to see your uploads: Upload (top right), then Sign in.",
            Page::Mine if v.busy => "Reading your uploads...",
            Page::Mine => "None of your mods are on the store.",
        };
        super::options::para(d, none, list[0] + 22.0, list[1] + CARDS_Y + 4.0, list[2] - 44.0, 15.0, 19.0, super::options::MUTED);
    } else if mods > shown {
        // the scroll bar
        let (ty, th) = (list[1] + CARDS_Y, CARD_PITCH * shown as f32 - (CARD_PITCH - CARD_H));
        let tx = list[0] + list[2] - 12.0;
        d.box3([tx, ty, 5.0, th], 2.5, [None, None, Some([110.0, 104.0, 100.0])], 128.0);
        let (k, at) = (shown as f32 / mods as f32, v.top as f32 / mods as f32);
        d.box3([tx, ty + th * at, 5.0, (th * k).max(10.0)], 2.5, [None, None, Some([70.0, 60.0, 64.0])], 128.0);
    }
    if let Some(r) = hand {
        d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [r[0] - 18.0 + hand_x, r[1] + r[3] / 2.0 - 9.0, 36.0, 18.0], WHITE, 128.0);
    }
    side_panel(d, m, side, hand_x);
    if let Some(r) = v.nophone {
        nophone(d, if r == 0 { "Add mod" } else { "Upload my mod" });
    }
}

/// What the chosen row does, or the last action's outcome.
fn info(m: &Menu) -> String {
    let v = &m.add;
    if let Some(say) = mods_kind::coming(v.kind) {
        return say.into();
    }
    if !v.status.is_empty() {
        return v.status.clone();
    }
    match (v.page, m.sel) {
        // B383: the grids say it themselves
        (Page::Browse | Page::Installed | Page::Mine, _) => grid::info(m),
        (Page::Upload, 0) => match &v.account {
            Some(name) => format!("Signed in to Hugging Face as {name}."),
            None => "Sign in to Hugging Face once with a token. After that, the game uploads for you.".into(),
        },
        (Page::Upload, 1) => "Upload every installed mod in the background. Mods already on the store, unchanged, are skipped.".into(),
        (Page::Upload, _) => RULE.into(),
    }
}

/// The side panel: the chosen mod's picture and △ card, its name, a line of who and where, its facts as chips and
/// what a press does; on a pill, its name and what it does. An open child's buttons stack at its foot (the rest
/// dims and takes no input).
fn side_panel(d: &mut Draw, m: &Menu, side: [f32; 4], hand_x: f32) {
    let v = &m.add;
    let [x, y, w, h] = side;
    let (tx, tw) = (x + 14.0, w - 28.0);
    let mut ty = y + 12.0;
    let wide = w > 2.0 * h;
    let mut chips: Vec<String> = Vec::new();
    let mut name = String::new();
    let mut meta = String::new();
    let mut stats = None;
    if let Some(e) = v.hovered(m.sel) {
        let mt = &e.meta;
        name = mt.name.clone();
        meta = [
            (!mt.author.is_empty()).then(|| format!("by {}", mt.author)),
            Some(format!("v{}", if mt.version.is_empty() { "1" } else { &mt.version })),
            Some(mb(mt.size)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" - ");
        chips = facts(&mt.source, mt.hand.map(f32::from), mt.costumes.map(|c| c as usize), mt.like.map(usize::from));
        chips.push(if e.verified { "Verified".into() } else { "Not reviewed yet".into() });
        stats = mt.style.map(usize::from).zip(mt.grades);
    } else if let Some(md) = v.local(m.sel) {
        name = md.name.clone();
        meta = format!("Folder {}", md.folder());
        chips = if v.kind == Kind::Character { facts(&md.source, Some(md.hand), Some(md.costumes.len()), Some(md.donor)) } else { facts(&md.source, None, None, None) };
        stats = v.stats.get(&md.dir).copied();
        // B250: its upload's outcome (the pull request's link, or why not)
        if let Some(u) = v.ups.get(&md.dir).filter(|_| v.page == Page::Upload) {
            let t = up_tag(u).0;
            meta = match u {
                Up::Done(s) | Up::Skipped(s) | Up::Failed(s) => format!("{t}: {}", s.rsplit(' ').next().filter(|w| w.starts_with("https://")).unwrap_or(s)),
                _ => t.into(),
            };
        }
    }
    let child = v.child.as_ref();
    let foot = child.map_or(0.0, |c| 34.0 * c.opts.len() as f32 + 8.0);
    if !name.is_empty() && child.is_none() {
        // the picture (as the select's) and the △ card beside it; in a wide short panel, text right of them
        let ph = 160.0f32.min(h - 24.0);
        let aspect = v.shot_aspect.unwrap_or(super::previews::CARD_W);
        let pic = [x + 12.0, ty, (ph * aspect).min(w - 174.0).max(60.0), ph];
        d.box3(pic, 8.0, [None, None, Some([122.0, 108.0, 115.0])], 128.0);
        if v.shot {
            // B402: the bake's 4:5 on screen (M8f9: a court's or umpire's own shape), as big as the box allows
            let pw = pic[2].min(ph * aspect);
            let phh = pw / aspect;
            d.q(Tex::Shelf, [0.0, 0.0, 1.0, 1.0], [pic[0] + (pic[2] - pw) / 2.0, pic[1] + (ph - phh) / 2.0, pw, phh], WHITE, 128.0);
        } else {
            d.centred("No picture", pic[0] + pic[2] / 2.0, pic[1] + ph / 2.0 - 7.0, 13.0, super::options::MUTED);
        }
        let ax = pic[0] + pic[2] + 6.0;
        let card = !wide || w >= 560.0;
        match stats.filter(|_| card) {
            Some((style, grades)) if ph >= 150.0 => attribute_card(d, 2, style, &grades, [ax, ty - 4.0], 1.0),
            Some((style, _)) => super::left(d, super::STYLE_NAMES[style.min(3)], ax, ty + 4.0, 13.0, 144.0, BROWN),
            None => d.text("No stats", ax + 40.0, ty + ph / 2.0 - 7.0, 13.0, super::options::MUTED),
        }
        if wide {
            ty = y + 12.0;
            let x2 = if card { ax + 156.0 } else { ax + 8.0 };
            return side_text(d, m, [x2, ty, x + w - 14.0 - x2, y + h - 12.0 - foot], &name, &meta, &chips);
        }
        ty += ph + 8.0;
    }
    if let Some(c) = child {
        // B202: the child's buttons (Cancel last) over the panel's foot, the rest dimmed
        if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
            d.out.remove(i);
        }
        d.popup(Some(super::pointer::press(|k| k.back = true)));
        d.anchor = super::Anchor::Span;
        d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 50.0);
        d.anchor = super::Anchor::Centre;
        d.box3(side, 14.0, [Some(INK), Some(WHITE), Some(CREAM)], 128.0);
        let what = if name.is_empty() { pill_words(v).get(c.row).cloned().unwrap_or_default() } else { name.clone() };
        let ty = super::options::para(d, &what, tx, y + 12.0, tw, 22.0, 26.0, BROWN) + 4.0;
        super::options::para(d, &info(m), tx, ty, tw, 14.0, 18.0, BROWN);
        for (k, (r, word)) in child_rects(c, side).into_iter().zip(&c.opts).enumerate() {
            super::side_button(d, r, word, k == c.at, false);
            d.hit(r, super::pointer::Act::Row(k, super::pointer::OK));
            if k == c.at {
                d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [r[0] - 18.0 + hand_x, r[1] + r[3] / 2.0 - 9.0, 36.0, 18.0], WHITE, 128.0);
            }
        }
        return;
    }
    if name.is_empty() && mods_kind::coming(v.kind).is_some() {
        // B373: a kind the store doesn't carry yet: what it is and how it behaves online
        name = mods_kind::name(v.kind).into();
        meta = mods_kind::online(v.kind).into();
    } else if name.is_empty() {
        // a pill: its name and what it does
        name = pill_words(v).get(m.sel).cloned().unwrap_or_default();
    }
    side_text(d, m, [tx, ty, tw, y + h - 12.0 - ty], &name, &meta, &chips);
}

/// B202: the open child's buttons, a column at the side panel's foot.
fn child_rects(c: &Child, [x, y, w, h]: [f32; 4]) -> Vec<[f32; 4]> {
    let n = c.opts.len();
    (0..n).map(|k| [x + 22.0, y + h - 12.0 - 34.0 * (n - k) as f32, w - 36.0, 30.0]).collect()
}

/// The side panel's words in `[x, y, w, h]`: the name, the who-and-where line, the facts as chips, what a press does.
fn side_text(d: &mut Draw, m: &Menu, [x, y, w, h]: [f32; 4], name: &str, meta: &str, chips: &[String]) {
    use super::options::{MUTED, para};
    let mut y = y;
    super::left(d, name, x, y, 22.0, w, BROWN);
    y += 26.0;
    if !meta.is_empty() {
        super::left(d, meta, x, y, 12.0, w, MUTED);
        y += 18.0;
    }
    let mut cx = x;
    for c in chips {
        let cw = (d.width(c, 12.0) + 14.0).min(w);
        if cx + cw > x + w && cx > x {
            (cx, y) = (x, y + 22.0);
        }
        d.pill_at([cx, y, cw, 18.0], [INK, [90.0; 3], CELL_FILL], 128.0);
        d.label(c, cx + cw / 2.0, y + 3.0, 12.0, cw - 8.0, BROWN);
        cx += cw + 4.0;
    }
    if !chips.is_empty() {
        y += 26.0;
    }
    let _ = h;
    para(d, &info(m), x, y, w, 12.0, 16.0, MUTED);
}

/// B250: an upload's chip, and its words' colour.
fn up_tag(u: &Up) -> (&'static str, [f32; 3]) {
    match u {
        Up::Queued => ("Queued", GREY),
        Up::Uploading => ("Uploading", GREY),
        Up::Done(_) => ("Uploaded", GREEN),
        Up::Skipped(_) => ("Skipped", GREY),
        Up::Failed(_) => ("Failed", RED),
    }
}

/// A mod's facts as chips: where it's from, its hand, costumes and the disc character it plays like.
fn facts(source: &str, hand: Option<f32>, costumes: Option<usize>, like: Option<usize>) -> Vec<String> {
    let mut l = Vec::new();
    if !source.is_empty() {
        l.push(format!("From {source}"));
    }
    if let Some(h) = hand {
        l.push(format!("{}-handed", if h < 0.0 { "Left" } else { "Right" }));
    }
    if let Some(c) = costumes {
        l.push(format!("{c} costume{}", if c == 1 { "" } else { "s" }));
    }
    if let Some(name) = like.and_then(|c| CHAR_NAMES.get(c)) {
        l.push(format!("Plays like {name}"));
    }
    l
}

/// B445: Remove all uploads' warning over a dimmed page, in B205's popup style: what goes, that it can't be undone,
/// and Remove all / Cancel pills (Cancel chosen first), the hand beside the chosen one; a tap off it is ○ (Cancel).
fn wipe_popup(d: &mut Draw, v: &View, a: usize, hand_x: f32) {
    use super::super::menu::{CHOSEN, PANEL, UNCHOSEN};
    use super::pointer::{Act, OK, press};
    if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
        d.out.remove(i);
    }
    d.popup(Some(press(|k| k.back = true)));
    d.anchor = super::Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 80.0);
    d.anchor = super::Anchor::Centre;
    let mine = v.mine.as_deref().unwrap_or_default();
    let n = mine.len();
    let b = [130.0, ASK_TOP - 104.0, 380.0, 104.0 + 72.0 + 16.0];
    d.box3(b, 16.0, [Some(INK), Some(WHITE), Some(PANEL)], 128.0);
    d.hit(b, Act::Press(Press::default()));
    let plural = if n == 1 { "" } else { "s" };
    d.outlined(|d| d.label("Warning", 320.0, ASK_TOP - 92.0, 22.0, 348.0, GOLD));
    d.outlined(|d| d.label(&format!("Remove all {n} of your upload{plural}?"), 320.0, ASK_TOP - 66.0, 20.0, 348.0, WHITE));
    // a merged one isn't the uploader's to close: its delete is asked for (`modstore::remove`)
    let merged = mine.iter().filter(|e| e.verified).count();
    let gone = match merged {
        0 => "They leave the store for everyone.".to_string(),
        m => format!("They leave the store ({m} merged: once a maintainer agrees)."),
    };
    d.outlined(|d| d.label(&gone, 320.0, ASK_TOP - 42.0, 16.0, 348.0, WHITE));
    d.outlined(|d| d.label("This can't be undone.", 320.0, ASK_TOP - 22.0, 16.0, 348.0, WHITE));
    for (k, word) in ["Remove all", "Cancel"].into_iter().enumerate() {
        let y = ASK_TOP + 40.0 * k as f32;
        let on = k == a;
        let fill = match (on, k) {
            (true, 0) => [150.0, 40.0, 36.0],
            (true, _) => CHOSEN,
            _ => UNCHOSEN,
        };
        d.pill_at([176.0, y, 288.0, 32.0], [INK, WHITE, fill], 128.0);
        d.hit([176.0, y, 288.0, 32.0], Act::Row(k, OK));
        d.label(word, 320.0, y + 6.0, 20.0, 160.0, if on { WHITE } else { UNCHOSEN.map(|c| c * 1.4) });
    }
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [116.0 + hand_x, ASK_TOP + 40.0 * a as f32, 64.0, 32.0], WHITE, 128.0);
}

/// B208: the phone popup over a dimmed page, in B205's popup style: the row's name, why, and an OK pill; a tap
/// anywhere closes it.
fn nophone(d: &mut Draw, row: &str) {
    use super::super::menu::{CHOSEN, PANEL};
    use super::pointer::{Act, OK, press};
    if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
        d.out.remove(i);
    }
    d.popup(Some(press(|k| k.back = true)));
    d.anchor = super::Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 80.0);
    d.anchor = super::Anchor::Centre;
    let b = [140.0, ASK_TOP - 70.0, 360.0, 70.0 + 56.0 + 16.0];
    d.box3(b, 16.0, [Some(INK), Some(WHITE), Some(PANEL)], 128.0);
    d.hit(b, Act::Press(OK));
    d.outlined(|d| d.label(row, 320.0, ASK_TOP - 58.0, 22.0, 328.0, GOLD));
    d.outlined(|d| d.label("Not available in the browser on phones:", 320.0, ASK_TOP - 34.0, 20.0, 328.0, WHITE));
    d.outlined(|d| d.label("use the desktop app.", 320.0, ASK_TOP - 12.0, 20.0, 328.0, WHITE));
    d.pill_at([176.0, ASK_TOP + 20.0, 288.0, 32.0], [INK, WHITE, CHOSEN], 128.0);
    d.label("OK", 320.0, ASK_TOP + 26.0, 20.0, 160.0, WHITE);
    let why = if row == "Add mod" { "picks a .zip from your files" } else { "signs in to Hugging Face" };
    d.said = format!("{row} {why}, which the browser on phones can't do: use the desktop app.");
}

/// `HST_MENU=browsemods` (Browse, its entries made up from the installed mods: Verified, Installed and Update all
/// shown, someone's folded; with `HST_CHILD=1` narrowed to FIZL0's, B255's Download all under way) / `mymods` (My public mods, the same made-up entries, one's Take down open; `HST_CHILD=wipe`: B445's Remove all uploads warning) / `uploadmods` (Upload, Upload
/// all open, the first mods tagged with each upload state; `HST_CHILD=0`: none open, the first mod hovered) / `signinmods` (Upload signed out, Sign in's buttons open) / `addmods` with `HST_CHILD=1` (Installed, a
/// mod's Remove open; B383: the Installed grid) / `reloadmods` (the main menu, B205's popup up): for `--shot`s.
pub(super) fn sample(m: &mut Menu, screen: &str) {
    let v = &mut m.add;
    v.refresh();
    // B373: `HST_KIND=1..3` opens the Courts, Umpires or Texture packs tab
    v.kind = std::env::var("HST_KIND").ok().and_then(|k| k.parse::<usize>().ok()).and_then(|i| mods_kind::KINDS.get(i).copied()).unwrap_or_default();
    if screen == "browsemods" || screen == "mymods" {
        v.page = Page::Browse;
        // B387: `HST_RELOAD=1`: mods changed (Later chosen), the heading's Reload shown
        v.changed = std::env::var_os("HST_RELOAD").is_some();
        // M8f9: made up from the chosen kind's installed mods (a court or umpire with its kind, no character facts)
        let (src, kind) = (v.locals().to_vec(), v.kind);
        let player = kind == Kind::Character;
        v.store = Some(
            src.iter().take(if screen == "browsemods" { 60 } else { 9 }).enumerate().map(|(i, md)| {
                let meta = modstore::Meta {
                    id: md.id.clone(),
                    name: md.name.clone(),
                    source: md.source.clone(),
                    author: ["FIZL0", "someone", ""][i % 3].into(),
                    size: (i as u64 + 1) * 7 << 20,
                    sha256: if screen == "browsemods" { format!("{i:064x}") } else { format!("{:064}", i % 2) },
                    style: (player && i % 3 != 2).then_some((i % 4) as u8),
                    grades: (player && i % 3 != 2).then_some([5, 4, 3, 2, 1].map(|g: u8| 1 + (g + i as u8) % 5)),
                    hand: player.then_some(if md.hand < 0.0 { -1 } else { 1 }),
                    costumes: player.then_some(md.costumes.len() as u32),
                    like: player.then_some(md.donor as u8),
                    kind,
                    ..Default::default()
                };
                Entry { meta, rev: if i % 3 == 0 { "main".into() } else { format!("refs/pr/{}", 100 - i) }, verified: i % 3 == 0 }
            }).collect(),
        );
        v.have = v.store.iter().flatten().zip(&src).take(4).map(|(e, md)| (e.key(), (format!("{:064}", 0), md.dir.clone()))).collect();
        m.sel = 2;
        // B372: the grid: the first four installed (the second an update), two downloading and one queued, no
        // network for the made-up mods' pictures; `HST_CHILD=search` searches "ma", `=keys` has the on-screen keyboard up
        let child = std::env::var("HST_CHILD").ok();
        if screen == "browsemods" {
            if let Some((sha, _)) = v.store.as_ref().and_then(|l| l.get(1)).and_then(|e| v.have.get_mut(&e.key())) {
                *sha = "old".into();
            }
            for (k, (e, (t, s))) in v.store.iter().flatten().skip(6).zip([("Downloading", "Downloading {} (62%)"), ("Downloading", "Checking {}..."), ("Queued", "{} is waiting its turn.")]).enumerate() {
                v.jobs.insert(e.key(), (t, s.replace("{}", &e.meta.name)));
                if k < 2 { v.doing.push(e.clone()) } else { v.queue.push_back(e.clone()) }
            }
            v.b.offline = true;
            v.b.wish = Some(v.store.iter().flatten().skip(3).step_by(7).map(Entry::key).collect());
            m.sel = v.browse_acts() + 1;
            // B444: `HST_LONG=<secs>`: the first two mods' titles too long for their tiles, the chosen one's slid that far
            if let Some(t) = std::env::var("HST_LONG").ok().and_then(|t| t.parse::<f32>().ok()) {
                for e in v.store.iter_mut().flatten().take(2) {
                    e.meta.name = format!("{} of the Extremely Long Title Edition", e.meta.name);
                    e.meta.source = "Mario Tennis: Power Tour Deluxe Collection".into();
                }
                v.b.roll = (t, m.sel);
            }
            match child.as_deref() {
                Some("search") => (v.b.search, m.sel) = ("ma".into(), 0),
                Some("keys") => (v.b.search, v.b.osk) = ("ma".into(), Some(12)),
                _ => {}
            }
        }
        if screen == "browsemods" && child.is_some_and(|c| c != "search" && c != "keys") {
            // B255: FIZL0's mods, Download all under way: one each of Installed, Downloading, Queued, Error
            v.narrow(Some("FIZL0".into()), 0);
            let tags = [("Installed", "Installed {}."), ("Downloading", "Downloading {}..."), ("Queued", "{} is waiting its turn."), ("Error", "{} not installed: the download doesn't match its sha256")];
            v.jobs = v.entries().iter().zip(tags).map(|(e, (t, s))| (e.key(), (t, s.replace("{}", &e.meta.name)))).collect();
            m.sel = v.browse_acts();
            v.batch = (1, 3);
            v.status = format!("{}Downloading {}...", v.count(), v.entries()[1].meta.name);
        }
        if screen == "mymods" {
            (v.page, v.mine, v.account, v.b.show, v.b.offline) = (Page::Mine, v.store.take(), Some("FIZL0".into()), grid::Show::Mine, true);
            m.sel = v.browse_acts() + 1;
            v.child = Some(Child::new(m.sel, &["Take down"]));
            v.status = format!("Take {} off the store? This closes its pull request.", v.hovered(m.sel).map_or("", |e| &e.meta.name));
            // B445: `HST_CHILD=wipe`: Remove all uploads' warning up instead (Cancel chosen)
            if std::env::var("HST_CHILD").is_ok_and(|c| c == "wipe") {
                v.child = None;
                v.wipe = Some(1);
                m.sel = 2 + v.words().iter().position(|w| *w == "Remove all uploads").unwrap_or(0);
            }
        }
    } else if screen == "reloadmods" {
        (v.changed, v.ask) = (true, Some(0));
        m.sel = 3;
    } else if screen == "uploadmods" {
        v.page = Page::Upload;
        v.account = Some("FIZL0".into());
        v.child = Some(Child::new(1, &["Upload all"]));
        v.status = format!("{RULE} Upload all {} mods?", v.list.len());
        let ups = [
            Up::Done("https://huggingface.co/datasets/FIZL0/opentennis-mods/discussions/12".into()),
            Up::Skipped("this copy is on the store already".into()),
            Up::Uploading,
            Up::Queued,
            Up::Failed("413: too big".into()),
        ];
        v.ups = v.list.iter().zip(ups).map(|(md, u)| (md.dir.clone(), u)).collect();
        m.sel = 1;
        // `HST_CHILD=0`: none open, the first mod hovered (its upload's line in its panel)
        if std::env::var("HST_CHILD").is_ok_and(|c| c == "0") {
            (v.child, v.status, m.sel) = (None, RULE.into(), UP_TOP);
        }
    } else if screen == "signinmods" {
        v.page = Page::Upload;
        v.child = Some(Child::new(0, &["Open token page", "Paste token"]));
        v.status = "Uploading needs a free Hugging Face token. Open token page, create one with Write access and copy it, then choose Paste token.".into();
        m.sel = 0;
    } else if screen == "addmods" {
        // B383: the Installed grid (Show → Installed), the first mod chosen; `HST_CHILD=1`: its Remove open
        (v.b.show, v.b.offline) = (grid::Show::Installed, true);
        v.relist();
        m.sel = v.browse_acts();
        if std::env::var("HST_CHILD").is_ok() && v.hovered(m.sel).is_some() {
            v.child = Some(Child::new(m.sel, &["Remove"]));
            v.status = format!("Remove {}? Reload afterwards to take it off the select screens.", v.hovered(m.sel).map_or("", |e| &e.meta.name));
        }
    } else if screen.starts_with("phonemods") {
        v.phone = true;
        m.sel = 0;
        v.nophone = (screen == "phonemods-ask").then_some(0);
    }
    // B373: a kind tab with nothing on it has no rows to open buttons on
    if v.empty_kind() {
        (v.child, m.sel) = (None, 0);
    }
}

/// The Add mod row's rectangle in % of the window (left, top, width, height), while it's on screen: where the page
/// puts its file input.
fn add_rect(m: &Menu, aspect: f32, k: f32) -> Option<[f32; 4]> {
    let (share, v) = widescreen::menu_shares(aspect, k);
    // B383: the Installed grid's Add mod word, where it was last drawn
    let [x, y, w, h] = (*grid::ADD_AT.lock().unwrap_or_else(|e| e.into_inner()))?;
    (m.screen == Screen::Mods && m.add.page == Page::Installed && !m.add.empty_kind() && !m.add.phone && m.add.nophone.is_none())
        .then(|| [x * share / 6.4 + 50.0 * (1.0 - share), y * v / 4.48 + 50.0 * (1.0 - v), w * share / 6.4, h * v / 4.48])
}

/// The store's previews (by mod id; None: it has none, or it's on its way), and the hovered one's (`Tex::Shelf`).
#[derive(Resource, Default)]
pub(super) struct Shelf {
    shots: HashMap<String, Option<Handle<Image>>>,
    pub shown: Option<Handle<Image>>,
    /// B372: Browse's tile pictures by slot (`grid::Pic::Shown`; None: free).
    pub thumbs: Vec<Option<Handle<Image>>>,
}

/// The list read on coming to the screen; the page's file input follows the Add mod row; an import's, a store
/// listing's, a download's or an upload's outcome comes back as the status; the hovered store mod's preview is
/// fetched once.
#[allow(clippy::too_many_arguments)]
fn page(
    mut menu: ResMut<Menu>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut shelf: ResMut<Shelf>,
    mut images: ResMut<Assets<Image>>,
    (art, args): (Option<Res<super::Art>>, Option<Res<crate::Args>>),
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    mut was: Local<Option<[f32; 4]>>,
    mut on: Local<bool>,
    (mut card, mut baked): (ResMut<super::previews::ModCard>, Local<u32>),
) {
    if (menu.screen == Screen::Mods) != *on {
        *on = !*on;
        // B334: it opens on Browse (whose Installed tags need it too; the other pages keep it); a `sample` screen
        // opens on its own page, already read, or with its child open
        if *on && matches!(menu.add.page, Page::Browse | Page::Installed) && menu.add.child.is_none() && !menu.add.b.offline {
            menu.add.refresh();
        }
        // B208: a touch device stays one (`web/touch.js` never forgets a touch)
        menu.add.phone |= *on && super::touch_page();
        // the select's cards for the mods it has (its roster, built at startup)
        if *on && let Some(art) = &art {
            let stats = menu.mods.iter().zip(&art.2.mods).map(|(m, s)| (m.dir.clone(), *s)).collect();
            menu.add.stats = stats;
            // (B383: the Installed grid's cards)
            menu.add.relist();
        }
        if let Some(a) = &args {
            menu.add.iso = a.iso.clone();
        }
    }
    if std::mem::take(&mut menu.add.paste) {
        // ponytail: desktop clipboards answer at once; the browser's is the last Ctrl+V on the page
        #[cfg(not(target_arch = "wasm32"))]
        let text = clipboard.as_mut().and_then(|c| c.fetch_text().poll_result()).and_then(Result::ok);
        #[cfg(target_arch = "wasm32")]
        let text = {
            let _ = &mut clipboard;
            crate::web::pasted()
        };
        match text.as_deref().and_then(modstore::token_in) {
            Some(t) => {
                menu.add.busy = true;
                modstore::sign_in(t);
            }
            None => {
                let mut c = Child::new(0, &["Open token page", "Paste token"]);
                c.at = 1;
                menu.add.child = Some(c);
                menu.add.status = "No token on the clipboard (tokens start with hf_). Copy yours, then choose Paste token again.".into();
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = crate::web::mod_status_take() {
        menu.add.refresh();
        // the page's word for a picked mod saved (index.html)
        if s.starts_with("Added ") {
            menu.add.changed();
            shelf.shots.retain(|k, _| !k.starts_with("local/"));
        }
        menu.add.status = format!("{}{s}", menu.add.count());
    }
    let (up, at_once) = (menu.settings.0[super::UPSCALED_DOWNLOADS], menu.settings.downloads_at_once());
    for e in modstore::take() {
        let v = &mut menu.add;
        match e {
            Event::Listing(r) => {
                v.busy = false;
                match r {
                    Ok(list) => {
                        v.status = format!("{} mods in the store.", list.len());
                        v.store = Some(list);
                        v.relist();
                        if let Some(b) = v.by.take() {
                            v.narrow(Some(b.author), b.from);
                        }
                    }
                    Err(e) => {
                        v.status = e;
                        v.store.get_or_insert_default();
                    }
                }
            }
            Event::Installed(key, r) => {
                v.finish(&key, &r);
                v.refresh();
                if r.is_ok() {
                    v.changed();
                    // B259: an update's new picture, not the old one read before it
                    shelf.shots.retain(|k, _| !k.starts_with("local/"));
                }
                v.status = match r {
                    // the browser's page says when it's saved
                    Ok(_) if cfg!(target_arch = "wasm32") => v.status.clone(),
                    Ok(name) => format!("{}Installed {name}. Reload to play it.", v.count()),
                    Err(e) => format!("{}Not installed: {e}", v.count()),
                };
                v.busy = !v.doing.is_empty();
                v.next(up, at_once);
            }
            Event::Progress(key, s) => {
                if let Some(j) = v.jobs.get_mut(&key) {
                    j.1 = s.clone();
                }
                v.status = format!("{}{s}", v.count());
            }
            Event::Mine(r) => {
                v.busy = false;
                match r {
                    Ok(list) => {
                        v.status = format!("{} of your mods on the store.", list.len());
                        v.mine = Some(list);
                    }
                    Err(e) => {
                        v.status = e;
                        v.mine.get_or_insert_default();
                    }
                }
            }
            Event::Removed(id, r) => {
                v.busy = false;
                if r.is_ok() {
                    // ponytail: a merged mod stays listed on the store until its delete PR is merged; this hides it till the next read
                    v.mine.get_or_insert_default().retain(|e| e.meta.id != id);
                    v.store = None;
                }
                let ok = r.is_ok();
                v.status = r.unwrap_or_else(|e| format!("Not removed: {e}"));
                // B445: Remove all uploads goes on to the next
                if v.wiped.2 > 0 {
                    if ok { v.wiped.0 += 1 } else { v.wiped.1 += 1 }
                    v.wipe_next();
                }
            }
            Event::Upload(dir, u) => {
                match &u {
                    Up::Done(s) => {
                        v.store = None;
                        v.mine = None;
                        v.status = s.clone();
                    }
                    Up::Skipped(s) => v.status = s.clone(),
                    Up::Failed(e) => v.status = format!("Not uploaded: {e}"),
                    _ => {}
                }
                v.ups.insert(dir, u);
            }
            Event::Signed(r) => {
                v.busy = false;
                v.status = match r {
                    Ok(name) => {
                        let s = format!("Signed in as {name}: pick a mod to upload.");
                        v.mine = None;
                        v.account = Some(name);
                        s
                    }
                    Err(e) => format!("Not signed in: {e}"),
                };
            }
            Event::Thumb(sha, png) => {
                let png = png.or_else(|| installed_card(&menu, &sha));
                let v = &mut menu.add;
                let pic = png.and_then(|b| thumb_image(&b)).map(|(i, a)| (shelf.slot(images.add(i)), a));
                grid::got(v, sha, pic);
            }
            Event::Preview(id, png) => {
                let img = png.and_then(|b| {
                    Image::from_buffer(&b, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), RenderAssetUsages::RENDER_WORLD).ok()
                });
                shelf.shots.insert(id, img.map(|i| images.add(i)));
            }
        }
    }
    let on = menu.screen == Screen::Mods;
    if on && menu.add.grid() {
        browse_pictures(&mut menu, &mut shelf, &mut images);
    }
    // only its own rows: every other screen's cursor is `sel` too (past row 4 it snapped back: Settings, the lobby)
    if on {
        let n = menu.add.rows();
        menu.sel = menu.sel.min(n - 1);
    }
    let png = |b: Vec<u8>| Image::from_buffer(&b, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), RenderAssetUsages::RENDER_WORLD).ok();
    // B229: a drawing came back: the pictures that weren't there are read again
    if card.baked != *baked {
        *baked = card.baked;
        shelf.shots.retain(|k, h| !k.starts_with("local/") || h.is_some());
    }
    // an installed store mod shows the installed one's picture
    let local = menu.add.hovered(menu.sel).and_then(|e| menu.add.installed(e)).and_then(|(_, dir)| menu.add.locals().iter().find(|m| m.dir == *dir));
    let local = local.or_else(|| menu.add.local(menu.sel)).filter(|_| on);
    card.want = None;
    shelf.shown = if let Some(m) = local {
        // an installed mod's: the select's picture of it, read once; none yet: drawn (B229)
        let key = format!("local/{}", m.dir.display());
        let h = match shelf.shots.get(&key) {
            Some(h) => h.clone(),
            None => {
                // M8f9: a court's or umpire's own picture (the select bakes characters only)
                let pic = if menu.add.kind == Kind::Character { super::previews::card(&menu.add.iso, m) } else { own_picture(&m.dir, menu.add.kind) };
                let h = pic.and_then(png).map(|i| images.add(i));
                shelf.shots.insert(key, h.clone());
                h
            }
        };
        if h.is_none() && menu.add.kind == Kind::Character {
            card.want = Some(m.clone());
        }
        h
    } else if let Some(e) = menu.add.hovered(menu.sel).filter(|_| on) {
        // a store mod's preview, fetched once
        shelf.shots.get(&e.key()).cloned().unwrap_or_else(|| {
            modstore::preview(e);
            shelf.shots.insert(e.key(), None);
            None
        })
    } else {
        None
    };
    menu.add.shot = shelf.shown.is_some();
    menu.add.shot_aspect = shelf.shown.as_ref().filter(|_| menu.add.kind != Kind::Character).and_then(|h| images.get(h)).map(|i| i.width() as f32 / i.height().max(1) as f32);
    let rect = window.single().ok().and_then(|w| {
        // the view's % → the window's (B117: a fixed aspect ratio's view sits inside the window)
        let size = Vec2::new(w.width(), w.height()).max(Vec2::ONE);
        let v = widescreen::view(size);
        let [l, t, wd, h] = add_rect(&menu, widescreen::aspect(w), widescreen::menu_scale())?;
        let (o, k) = (v.min / size * 100.0, v.size() / size);
        Some([o.x + l * k.x, o.y + t * k.y, wd * k.x, h * k.y])
    });
    if rect != *was {
        *was = rect;
        #[cfg(target_arch = "wasm32")]
        crate::web::mod_picker(rect);
    }
}

impl Shelf {
    /// A free tile picture slot for `h`.
    fn slot(&mut self, h: Handle<Image>) -> u32 {
        match self.thumbs.iter().position(Option::is_none) {
            Some(i) => {
                self.thumbs[i] = Some(h);
                i as u32
            }
            None => {
                self.thumbs.push(Some(h));
                self.thumbs.len() as u32 - 1
            }
        }
    }
}

/// M8f9: an installed court's or umpire's own picture (the standard's `previews/court.png` / `previews/umpire.png`).
fn own_picture(dir: &Path, kind: Kind) -> Option<Vec<u8>> {
    let f = match kind {
        Kind::Court => "previews/court.png",
        Kind::Umpire => "previews/umpire.png",
        _ => return None,
    };
    crate::modfs::read(&dir.join(f)).ok()
}

/// A store mod with no picture of its own that's installed: the select's picture of it (drawn when it has none).
fn installed_card(m: &Menu, sha: &str) -> Option<Vec<u8>> {
    let v = &m.add;
    let e = v.entries().iter().find(|e| e.meta.sha256 == sha)?;
    let dir = &v.installed(e)?.1;
    let md = v.list.iter().find(|md| md.dir == *dir)?;
    super::previews::card(&v.iso, md)
}

/// A tile picture's PNG as an image, and its width over its height.
fn thumb_image(png: &[u8]) -> Option<(Image, f32)> {
    let i = Image::from_buffer(png, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), RenderAssetUsages::RENDER_WORLD).ok()?;
    let s = i.size();
    Some((i, s.x as f32 / s.y.max(1) as f32))
}

/// B372: Browse's tile pictures: the ones on view and the next screen asked for (cached ones read at once, the rest
/// fetched a few at a time: one while installs run, else Downloads at Once, at least two), far-off ones dropped.
fn browse_pictures(menu: &mut Menu, shelf: &mut Shelf, images: &mut Assets<Image>) {
    menu.add.b.tick = menu.add.b.tick.wrapping_add(1);
    if menu.add.store.is_none() {
        return;
    }
    let l = grid::lay(&menu.grid);
    for k in grid::evict(&mut menu.add, &l) {
        if let Some(h) = shelf.thumbs.get_mut(k as usize).and_then(Option::take) {
            images.remove(&h);
        }
    }
    let cap = if menu.add.doing.is_empty() { menu.settings.downloads_at_once().max(2) } else { 1 };
    let dir = grid::cache_dir();
    // cached pictures don't wait for a turn: up to a few rounds a frame
    for _ in 0..8 {
        let go = grid::want(&mut menu.add, &l, cap);
        let mut read = false;
        for e in go {
            let cached = if menu.add.b.offline { Some(None) } else { modstore::cached_thumb(&dir, &e.meta.sha256) };
            match cached {
                Some(png) => {
                    let png = png.or_else(|| installed_card(menu, &e.meta.sha256));
                    let pic = png.and_then(|b| thumb_image(&b)).map(|(i, a)| (shelf.slot(images.add(i)), a));
                    grid::got(&mut menu.add, e.meta.sha256.clone(), pic);
                    read = true;
                }
                None => modstore::thumb(&e, &dir),
            }
        }
        if !read {
            break;
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Shelf>().add_systems(Update, (page.before(super::draw), grid::typing.before(super::step), grid::roll_time.before(super::draw)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modstore::Meta;

    fn ok() -> Press {
        Press { ok: true, ..default() }
    }

    /// Row `r`'s drawn rect (a pill, or a card while on screen).
    fn rect(m: &Menu, r: usize) -> [f32; 4] {
        row_rect(&m.add, boxes(&m.grid).0, shown(&m.grid), r).unwrap()
    }

    /// B383: the Installed grid (Show → Installed) on the mods put in `list`; its first tile's cursor slot.
    fn installed(m: &mut Menu) -> usize {
        m.mods_page(Page::Installed, 0);
        m.add.relist();
        m.add.browse_acts()
    }

    /// B383: the heading's word `w`'s cursor slot.
    fn word(m: &Menu, w: &str) -> usize {
        2 + m.add.words().iter().position(|x| *x == w).unwrap()
    }

    /// Held by the tests that run `page`: it takes every store event, one test's outcomes included.
    static PAGE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// The page keeps its cursor within its own rows only: on Graphics, row 7 stays row 7 (it snapped back to 4).
    #[test]
    fn other_screens_keep_their_cursor() {
        let _one = PAGE.lock().unwrap_or_else(|e| e.into_inner());
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>().init_resource::<Shelf>().init_resource::<super::super::previews::ModCard>();
        app.insert_resource(Menu { screen: Screen::Graphics, sel: 7, ..default() });
        app.add_systems(Update, page);
        app.update();
        assert_eq!(app.world().resource::<Menu>().sel, 7);
    }

    fn a_mod(id: &str, name: &str) -> Mod {
        Mod {
            dir: root().join(id),
            id: id.into(),
            name: name.into(),
            source: String::new(),
            costumes: Vec::new(),
            donor: 0,
            hand: 1.0,
            base: 0,
            overrides: Vec::new(),
            ai_row: 0,
            texture_face: false,
            no_face: false,
            voice: String::new(),
            motions: None,
            costume_of: None,
            skeleton: None,
        }
    }

    /// B383: Installed is a grid of the installed mods (Show → Installed); ✕ on one opens Remove / Cancel in the
    /// side panel (B202): ○ and Cancel close it, ↑↓ wrap in it, Remove removes the mod (from memory, and nothing else).
    #[test]
    fn removes_from_the_child() {
        let mut m = Menu::default();
        crate::modfs::add(&root().join("t_screen/mod.json"), Vec::new().into());
        m.add.list = vec![a_mod("t_screen", "Screen"), a_mod("b", "B")];
        m.screen = Screen::Mods;
        let t0 = installed(&mut m);
        assert_eq!((m.add.b.show, m.add.rows()), (grid::Show::Installed, t0 + 2));
        m.sel = t0;
        let mut out = Vec::new();
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.child, Some(Child { row: t0, at: 0, opts: vec!["Remove", "Cancel"] }));
        assert!(m.add.status.starts_with("Remove Screen"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.add.child.is_none(), m.screen, m.sel), (true, Screen::Mods, t0));
        m.mods_step(ok(), &mut out);
        m.mods_step(Press { up: true, ..default() }, &mut out);
        assert_eq!((m.sel, m.add.child.as_ref().map(|c| c.at)), (t0, Some(1)));
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.is_none() && crate::modfs::is_file(&root().join("t_screen/mod.json")));
        m.mods_step(ok(), &mut out);
        // the pointer: only the child's buttons, a tap elsewhere is Back
        let at = |m: &Menu, x: f32, y: f32| super::super::pointer::tests::act_at(m, x, y);
        let r = child_rects(m.add.child.as_ref().unwrap(), grid::lay(&m.grid).detail);
        let p = grid::lay(&m.grid).search;
        assert_eq!(at(&m, p[0] + 10.0, p[1] + 5.0), None);
        assert_eq!(at(&m, r[1][0] + 10.0, r[1][1] + 5.0), Some(super::super::pointer::Act::Row(1, super::super::pointer::OK)));
        assert_eq!(super::super::pointer::tests::hits(&m).1, Some(super::super::pointer::press(|k| k.back = true)));
        m.point(super::super::Dev::Keys, super::super::pointer::Act::Row(1, super::super::pointer::OK), false);
        assert_eq!((m.sel, m.add.child.as_ref().map(|c| c.at)), (t0, Some(1)));
        m.point(super::super::Dev::Keys, super::super::pointer::Act::Row(0, super::super::pointer::OK), true);
        assert!(!crate::modfs::is_file(&root().join("t_screen/mod.json")));
        assert!(m.add.status.starts_with("Removed Screen"));
        // the header's search box is the top: ↑ stays
        m.sel = 0;
        m.mods_step(Press { up: true, ..default() }, &mut out);
        assert_eq!(m.sel, 0);
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.screen, m.sel), (Screen::Main, m.main_at(super::super::home::Row::Mods)));
        // B205: back on the main menu the popup asks (Reload chosen) and takes the presses; Later closes it, the note stays
        assert_eq!(m.add.ask, Some(0));
        // B387: ○ doesn't close it, only Later does
        let out = m.step(super::super::Dev::Keys, Press { back: true, ..default() });
        assert_eq!((m.add.ask, m.screen), (Some(0), Screen::Main));
        assert!(out.is_empty(), "{out:?}");
        let out = m.step(super::super::Dev::Keys, Press { down: true, ..default() });
        assert_eq!((m.add.ask, m.sel), (Some(1), m.main_at(super::super::home::Row::Mods)));
        assert_eq!(out, [Effect::Sound(0)]);
        let out = m.step(super::super::Dev::Keys, ok());
        assert_eq!((m.add.ask, m.screen, m.add.changed), (None, Screen::Main, true));
        assert_eq!(out, [Effect::Sound(1)]);
        // no new change: no second ask
        m.step(super::super::Dev::Keys, ok());
        assert_eq!(m.screen, Screen::Mods);
        m.mods_step(Press { back: true, ..default() }, &mut Vec::new());
        assert_eq!((m.screen, m.add.ask), (Screen::Main, None));
        // B387: after Later, Browse (where Mods opens) keeps a Reload in its heading, as Installed's pill
        m.step(super::super::Dev::Keys, ok());
        assert_eq!((m.screen, m.add.page), (Screen::Mods, Page::Browse));
        assert_eq!(m.add.words().last(), Some(&"Reload"));
        m.add.changed = false;
        assert!(!m.add.words().contains(&"Reload"));
    }

    /// A mod in a folder put in by hand (here a pack's) isn't removed: the press says where to delete it.
    #[test]
    fn hand_added_mods_stay() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.list = vec![Mod { dir: root().join("pack/p"), ..a_mod("p", "P") }];
        m.sel = installed(&mut m);
        let mut out = Vec::new();
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.is_none() && m.add.status.contains("added by hand"), "{}", m.add.status);
        // nor by Remove all
        m.sel = word(&m, "Remove all");
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.is_none() && m.add.status.starts_with("No mods"), "{}", m.add.status);
    }

    /// B335: Remove all (B383: a word in Installed's heading) asks, then removes every mod the game installed.
    #[test]
    fn remove_all_mods() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        for id in ["t_all1", "t_all2"] {
            crate::modfs::add(&root().join(id).join("mod.json"), Vec::new().into());
        }
        m.add.list = vec![a_mod("t_all1", "A"), a_mod("t_all2", "B")];
        installed(&mut m);
        assert_eq!(m.add.words(), ["Newest", "A-Z", "Uploader", "Add mod", "Reload", "Remove all"]);
        m.sel = word(&m, "Remove all");
        let mut out = Vec::new();
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.child.as_ref().map(|c| c.opts.clone()), Some(vec!["Remove all", "Cancel"]));
        m.mods_step(ok(), &mut out);
        assert!(["t_all1", "t_all2"].iter().all(|id| !crate::modfs::is_file(&root().join(id).join("mod.json"))));
        assert!(m.add.status.starts_with("Removed every mod") && m.add.changed, "{}", m.add.status);
    }

    /// B372: Browse's grid: Newest (pull requests newest first, merged ones after), A-Z and Uploader sorts; the search
    /// matches names, uploaders and games in any case; each rail option counts what it would leave (the other groups'
    /// filters applied) and filters the tiles; ← from the grid's first column goes into the rail and → back to the
    /// tile it left, ↑ from the top row to the sort word, ↑ again the search box.
    #[test]
    fn grid_sorts_filters_and_moves() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let e = |id: &str, who: &str, src: &str, rev: &str, hand: i8, style: u8| Entry {
            meta: Meta { id: id.into(), name: id.into(), author: who.into(), source: src.into(), sha256: id.repeat(32), size: 1, hand: Some(hand), style: Some(style), ..Default::default() },
            rev: rev.into(),
            verified: rev == "main",
        };
        m.add.store = Some(vec![
            e("Kent", "bob", "Get a Grip", "main", 1, 0),
            e("Carol", "ann", "Get a Grip", "refs/pr/7", -1, 1),
            e("Alma", "", "Fore!", "refs/pr/12", 1, 2),
            e("Zed", "cal", "Fore!", "main", -1, 1),
            e("Mia", "ann", "Tekken", "refs/pr/3", 1, 3),
        ]);
        m.add.have = [("bob/Kent".to_string(), ("old".to_string(), "mods/kent".into()))].into();
        m.mods_page(Page::Browse, 0);
        let names = |m: &Menu| m.add.tiles().iter().map(|e| e.meta.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&m), ["Alma", "Carol", "Mia", "Kent", "Zed"]);
        m.add.b.sort = grid::Sort::Az;
        assert_eq!(names(&m), ["Alma", "Carol", "Kent", "Mia", "Zed"]);
        m.add.b.sort = grid::Sort::Uploader;
        assert_eq!(names(&m), ["Carol", "Mia", "Kent", "Zed", "Alma"]);
        m.add.b.search = "GRIP".into();
        assert_eq!(names(&m), ["Carol", "Kent"]);
        m.add.b.search = "an".into();
        assert_eq!(names(&m), ["Carol", "Mia"]);
        m.add.b.search.clear();
        // the rail: games by how many mods (one each: Other only past six), Left-handed, Updates
        let rail = m.add.rail();
        let label = |o: &grid::Opt| format!("{o:?}");
        assert!(label(&rail[1]).contains("Fore!") && label(&rail[2]).contains("Get a Grip") && label(&rail[3]).contains("Tekken"), "{rail:?}");
        let left = rail.iter().position(|o| *o == grid::Opt::Left(true)).unwrap();
        let upd = rail.iter().position(|o| *o == grid::Opt::Show(grid::Show::Updates)).unwrap();
        assert_eq!((m.add.count_of(&rail[1]), m.add.count_of(&rail[left]), m.add.count_of(&rail[upd])), (2, 2, 1));
        let r0 = 2 + m.add.words().len();
        let mut out = Vec::new();
        m.sel = r0 + left;
        m.mods_step(ok(), &mut out);
        assert_eq!(names(&m), ["Carol", "Zed"]);
        // the Game counts now count left-handers only: Fore! has one
        assert_eq!(m.add.count_of(&rail[1]), 1);
        m.sel = r0 + 1;
        m.mods_step(ok(), &mut out);
        assert_eq!(names(&m), ["Zed"]);
        m.add.b.game = None;
        m.add.b.left = false;
        // moves: the second tile ← → the first, ← into the rail (on the option left last), → back to the tile
        let t0 = m.add.browse_acts();
        // five a row on a 16:9 view, three on 4:3
        assert_eq!(grid::lay(&super::super::Grid::fit(16.0 / 9.0)).cols, 5);
        let cols = grid::lay(&m.grid).cols;
        assert_eq!(cols, 3);
        let (l, r, u, d) = (Press { left: true, ..default() }, Press { right: true, ..default() }, Press { up: true, ..default() }, Press { down: true, ..default() });
        m.sel = t0 + 1;
        m.mods_step(l, &mut out);
        assert_eq!(m.sel, t0);
        m.mods_step(l, &mut out);
        assert_eq!(m.sel, r0);
        m.mods_step(d, &mut out);
        m.mods_step(r, &mut out);
        assert_eq!(m.sel, t0);
        // ↑ to the chosen sort's word (Uploader), ↑ to the search box, ↓ back to the sort word, ↓ into the grid
        m.mods_step(u, &mut out);
        assert_eq!(m.sel, 2 + 2);
        m.mods_step(u, &mut out);
        assert_eq!(m.sel, 0);
        m.mods_step(d, &mut out);
        m.mods_step(d, &mut out);
        assert_eq!(m.sel, t0);
        // B383: → from the search box to Upload, ↓ from it to the heading's last word, ← back
        m.sel = 0;
        m.mods_step(r, &mut out);
        assert_eq!((m.sel, m.add.slot(1)), (1, grid::Slot::Upload));
        m.mods_step(d, &mut out);
        assert_eq!(m.sel, 2 + m.add.words().len() - 1);
        m.sel = 1;
        m.mods_step(l, &mut out);
        assert_eq!(m.sel, 0);
        // a sort word's press sorts
        m.sel = 3;
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.b.sort, names(&m)[0].as_str()), (grid::Sort::Az, "Alma"));
        // the legend says what a tile's buttons do; one kind of mod: no kind switch (B383)
        m.sel = t0 + 2;
        let l = grid::legend(&m).into_iter().map(|x| x.1).collect::<Vec<_>>();
        assert_eq!(l, ["Move", "Update", "More by", "Save", "Back"]);
        // Upload: Sign in and Upload all side by side, then the mods
        m.mods_page(Page::Upload, 0);
        m.add.list = vec![a_mod("a", "A"), a_mod("b", "B")];
        let mut go = |k: Press| {
            m.mods_step(k, &mut Vec::new());
            m.sel
        };
        let (l, r, u, d) = (Press { left: true, ..default() }, Press { right: true, ..default() }, Press { up: true, ..default() }, Press { down: true, ..default() });
        assert_eq!([go(r), go(r), go(l), go(d), go(d), go(u), go(u), go(u)], [1, 0, 1, UP_TOP, UP_TOP + 1, UP_TOP, 0, UP_TOP + 1]);
        // no mods under the pills: ↑↓ go between them
        assert_eq!((nav(0, 2, 2, d), nav(1, 2, 2, d), nav(0, 2, 2, u)), (1, 0, 1));
    }

    /// Upload's long list scrolls to keep the cursor on screen, and taps land on the drawn rows; the browser's file
    /// input sits on Installed's Add mod word.
    #[test]
    fn scrolls_and_taps() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.list = (0..20).map(|i| a_mod(&format!("m{i}"), &format!("M{i}"))).collect();
        m.mods_page(Page::Upload, 0);
        let mut out = Vec::new();
        for _ in 0..9 {
            m.mods_step(Press { down: true, ..default() }, &mut out);
        }
        // the cursor's mod (the 9th) is the last card shown
        let n = shown(&m.grid);
        assert!(n < 9);
        assert_eq!((m.sel, m.add.top), (UP_TOP + 8, 9 - n));
        let at = |m: &Menu, x: f32, y: f32| super::super::pointer::tests::act_at(m, x, y);
        let r = rect(&m, UP_TOP + 8);
        assert_eq!(at(&m, r[0] + 10.0, r[1] + 5.0), Some(super::super::pointer::Act::Row(UP_TOP + 8, super::super::pointer::OK)));
        assert!(row_rect(&m.add, boxes(&m.grid).0, n, UP_TOP + 9).is_none());
        // the gap between two cards goes to the nearer one
        let first = rect(&m, UP_TOP + m.add.top);
        assert_eq!(at(&m, first[0] + 10.0, first[1] + CARD_H + 1.0), Some(super::super::pointer::Act::Row(UP_TOP + m.add.top, super::super::pointer::OK)));
        (0..9).for_each(|_| m.mods_step(Press { up: true, ..default() }, &mut out));
        assert_eq!((m.sel, m.add.top), (0, 0));
        // B383: the header's Upload button is lit here; a tap on it goes back to the grid
        assert!(add_rect(&m, 4.0 / 3.0, 1.0).is_none());
        let u = grid::lay(&m.grid).upload;
        assert_eq!(at(&m, u[0] + 10.0, u[1] + 5.0), Some(super::super::pointer::Act::Tab(0)));
        // 4:3 fills the window: Add mod's own place (as last drawn)
        installed(&mut m);
        let w = word(&m, "Add mod");
        let a = super::super::pointer::tests::hits(&m).0.into_iter().find(|h| h.act == super::super::pointer::Act::Row(w, super::super::pointer::OK)).unwrap().rect;
        let r = add_rect(&m, 4.0 / 3.0, 1.0).unwrap();
        assert!((r[0] - a[0] / 6.4).abs() < 1e-4 && (r[2] - a[2] / 6.4).abs() < 1e-4, "{r:?} {a:?}");
    }

    /// Browse tags each store mod by what's installed (same zip: Installed, another: Update); pressing an installed
    /// one doesn't download it; Back returns to the main menu. Upload opens Upload / Cancel, the rule shown; Sign in
    /// opens Open token page / Paste token, signed in Sign out.
    #[test]
    fn browse_and_upload_pages() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let e = |id: &str, sha: &str| Entry { meta: Meta { id: id.into(), name: id.into(), sha256: sha.repeat(64), size: 1, ..Default::default() }, rev: "main".into(), verified: true };
        m.add.store = Some(vec![e("a", "1"), e("b", "2"), e("c", "3")]);
        m.add.have = [("/a".to_string(), ("1".repeat(64), "mods/a".into())), ("/b".to_string(), ("9".repeat(64), "mods/b".into()))].into();
        m.add.list = vec![a_mod("a", "A")];
        let mut out = Vec::new();
        m.mods_open(&mut out);
        // Browse first (the search box, words and rail, then the tiles)
        let t0 = m.add.browse_acts();
        assert_eq!((m.add.page, m.sel, m.add.rows()), (Page::Browse, 0, t0 + 3));
        let tags: Vec<_> = m.add.entries().iter().map(|x| m.add.state(x)).collect();
        assert_eq!(tags, [Some("Installed"), Some("Update"), None]);
        // an installed tile's press offers Remove, not a download
        m.sel = t0;
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.child.as_ref().map(|c| c.opts.clone()), m.add.busy), (Some(vec!["Remove", "Cancel"]), false));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert!(m.add.child.is_none());
        // the pointer: a tap on a tile chooses it, a tap on it again presses ✕ (the third tile: it queues)
        use super::super::pointer::{Act, tests::act_at};
        let c = grid::lay(&m.grid).cell(2);
        assert_eq!(act_at(&m, c[0] + 10.0, c[1] + 10.0), Some(Act::Tile(t0 + 2)));
        (m.settings.8, m.add.doing) = (1, vec![e("zz", "z")]);
        m.point(super::super::Dev::Keys, Act::Tile(t0 + 2), true);
        assert_eq!((m.sel, m.add.queue.len()), (t0 + 2, 0));
        m.point(super::super::Dev::Keys, Act::Tile(t0 + 2), true);
        assert_eq!(m.add.queue.iter().map(|x| x.meta.id.as_str()).collect::<Vec<_>>(), ["c"]);
        // the mouse moving onto another tile chooses it; the next click there is a first tap
        m.point(super::super::Dev::Keys, Act::Tile(t0 + 1), false);
        assert_eq!((m.sel, m.add.b.tapped), (t0 + 1, None));
        (m.add.doing, m.add.queue) = (Vec::new(), Default::default());
        // the tab row's Upload tab
        m.point(super::super::Dev::Keys, super::super::pointer::Act::Tab(2), true);
        assert_eq!(m.add.page, Page::Upload);
        assert_eq!(m.add.status, RULE);
        // the account row first, then the mods; uploading needs an account
        m.add.account = None;
        m.sel = UP_TOP;
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.starts_with("Uploading needs a Hugging Face account") && m.add.child.is_none());
        m.add.account = Some("me".into());
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.starts_with(RULE) && m.add.status.ends_with("Upload A?") && !m.add.busy);
        assert_eq!(m.add.child.as_ref().map(|c| c.opts.clone()), Some(vec!["Upload", "Cancel"]));
        assert_eq!(m.add.local(m.sel).map(|x| x.id.as_str()), Some("a"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.add.page, m.add.status.as_str()), (Page::Upload, RULE));
        m.sel = 0;
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.child.as_ref().map(|c| c.opts.clone()), Some(vec!["Sign out", "Cancel"]));
        m.mods_step(Press { down: true, ..default() }, &mut out);
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.child.is_none(), m.add.account.as_deref()), (true, Some("me")));
        m.add.account = None;
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.child.as_ref().map(|c| c.opts.clone()), Some(vec!["Open token page", "Paste token", "Cancel"]));
        // (Open token page would open a browser here) Paste token closes it and reads the clipboard
        m.mods_step(Press { down: true, ..default() }, &mut out);
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.is_none() && m.add.paste && m.add.status.starts_with("Checking"));
        m.add.paste = false;
        // B383: ○ on Upload goes back to the grid, ○ there to the main menu
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.screen, m.add.page, m.sel), (Screen::Mods, Page::Browse, 0));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.screen, Screen::Main);
    }

    /// B339: Download all starts as many installs as Downloads at Once says and queues the rest; each one's outcome
    /// (by its key) ends that one and starts the next; Add mod's pick, finishing meanwhile, ends none of them.
    #[test]
    fn downloads_side_by_side() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.settings.8 = 3;
        let e = |id: &str| Entry { meta: Meta { id: id.into(), name: id.to_uppercase(), author: "ann".into(), sha256: id.repeat(32), size: 1, ..Default::default() }, rev: "main".into(), verified: true };
        let list: Vec<Entry> = ["a1", "a2", "a3", "a4", "a5"].map(e).into();
        let mut out = Vec::new();
        m.download_all("ann", list, &mut out);
        let ids = |l: Vec<&Entry>| l.iter().map(|x| x.meta.id.clone()).collect::<Vec<_>>();
        assert_eq!((ids(m.add.doing.iter().collect()), ids(m.add.queue.iter().collect())), (vec!["a1".to_string(), "a2".into(), "a3".into()], vec!["a4".to_string(), "a5".into()]));
        assert!(m.add.status.ends_with("3 at a time."), "{}", m.add.status);
        assert_eq!(m.add.jobs.get("ann/a4").map(|j| j.0), Some("Queued"));
        assert_eq!(m.add.jobs.get("ann/a2").map(|j| j.0), Some("Downloading"));
        m.add.finish("", &Ok("Picked".into()));
        assert_eq!((m.add.doing.len(), m.add.batch), (3, (0, 5)));
        m.add.finish("ann/a2", &Err("the store said 404".into()));
        m.add.next(false, m.settings.downloads_at_once());
        assert_eq!(ids(m.add.doing.iter().collect()), ["a1", "a3", "a4"]);
        assert_eq!((m.add.jobs["ann/a2"].0, m.add.count().as_str()), ("Error", "1/5 done. "));
        // a mod under way isn't queued again; one at a time leaves the rest waiting
        m.install(e("a3"), &mut out);
        assert_eq!((m.add.queue.len(), m.add.status.as_str()), (1, "A3 is on its way already."));
        m.settings.8 = 1;
        for k in ["ann/a1", "ann/a3"] {
            m.add.finish(k, &Ok(String::new()));
        }
        m.add.next(false, m.settings.downloads_at_once());
        assert_eq!((ids(m.add.doing.iter().collect()), m.add.queue.len()), (vec!["a4".to_string()], 1));
    }

    /// B250: Upload all opens Upload all / Cancel; it (and a mod's own Upload) queues the mods in the background
    /// without making the page busy, each tagged; one on its way isn't queued again; the outcomes come back per mod.
    #[test]
    fn upload_all() {
        // (its uploads report from `queue` on)
        let _one = PAGE.lock().unwrap_or_else(|e| e.into_inner());
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let mut out = Vec::new();
        m.mods_page(Page::Upload, 1);
        m.add.account = Some("me".into());
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.starts_with("No mods") && m.add.child.is_none());
        // (no such folders: each fails packing, before any network)
        m.add.list = vec![a_mod("t_up_a", "A"), a_mod("t_up_b", "B")];
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.child.as_ref().map(|c| (c.row, c.opts.clone())), Some((1, vec!["Upload all", "Cancel"])));
        assert!(m.add.status.ends_with("Upload all 2 mods?"));
        // ✕ goes through the account on disk; signed in, the same as `queue`
        m.add.child = None;
        m.queue(m.add.list.clone(), "me".into(), "hf_test".into());
        assert!(!m.add.busy && m.add.status.contains("2 mods"));
        assert!(m.add.list.iter().all(|x| m.add.ups.get(&x.dir) == Some(&Up::Queued)));
        m.sel = UP_TOP;
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.is_none() && m.add.status == "A is uploading already.");
        m.queue(m.add.list.clone(), "me".into(), "hf_test".into());
        assert_eq!(m.add.status, "Those are uploading already.");
        // the menu stays usable meanwhile: My uploads and back
        m.add.mine = Some(Vec::new());
        m.mods_show(grid::Show::Mine, &mut out);
        assert_eq!((m.add.page, m.sel), (Page::Mine, 0));
        m.mods_tab(2, &mut out);
        assert_eq!(m.add.page, Page::Upload);
        super::super::pointer::tests::hits(&m);
        // each mod's outcome lands beside it (here both fail packing), the last one said in the status bar
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>().init_resource::<Shelf>().init_resource::<super::super::previews::ModCard>();
        app.insert_resource(m);
        app.add_systems(Update, page);
        // ponytail: up to 30 s for the packing threads, which run late when the whole suite shares the CPU
        for _ in 0..3000 {
            app.update();
            let v = &app.world().resource::<Menu>().add;
            if v.list.iter().all(|x| matches!(v.ups.get(&x.dir), Some(Up::Failed(_)))) {
                return assert!(v.status.starts_with("Not uploaded:"), "{}", v.status);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("no outcomes: {:?}", app.world().resource::<Menu>().add.ups);
    }

    /// B208: on a phone Add mod (B383: Installed's word) and the header's Upload are greyed and put up the popup
    /// instead (no picker, no page), as does Show → My uploads; ✕, ○ or a tap anywhere closes it; the other Shows
    /// still open; the file input stays off.
    #[test]
    fn phone_rows_greyed() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.phone = true;
        m.add.store = Some(Vec::new());
        installed(&mut m);
        let mut out = Vec::new();
        for (row, close) in [(0, ok()), (2, Press { back: true, ..default() }), (2, ok())] {
            m.sel = if row == 0 { word(&m, "Add mod") } else { 1 };
            if row == 2 && close.ok {
                m.mods_show(grid::Show::Mine, &mut out);
            } else {
                m.mods_step(ok(), &mut out);
            }
            assert_eq!((m.add.nophone, m.add.page, m.add.busy), (Some(row), Page::Installed, false));
            // the popup takes the moves
            let at = m.sel;
            m.mods_step(Press { down: true, ..default() }, &mut out);
            assert_eq!((m.sel, m.add.nophone), (at, Some(row)));
            m.mods_step(close, &mut out);
            assert_eq!((m.screen, m.add.nophone), (Screen::Mods, None));
        }
        assert!(add_rect(&m, 4.0 / 3.0, 1.0).is_none());
        use super::super::pointer::{Act, OK, tests::act_at};
        m.add.nophone = Some(2);
        assert_eq!(act_at(&m, 320.0, ASK_TOP), Some(Act::Press(OK)));
        assert_eq!(act_at(&m, 10.0, 300.0), None);
        m.add.nophone = None;
        m.mods_show(grid::Show::All, &mut out);
        assert_eq!((m.add.page, m.add.nophone), (Page::Browse, None));
        m.mods_show(grid::Show::Installed, &mut out);
        assert_eq!((m.add.page, m.add.nophone), (Page::Installed, None));
    }

    /// M19: My public mods needs an account; B383: it's a grid (Show → My uploads) whose tiles open Take down (a
    /// PR's, it closes) or Ask to delete (a merged one's) in the side panel; Back returns to the main menu.
    #[test]
    fn my_public_mods() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let mut out = Vec::new();
        let e = |id: &str, rev: &str| Entry { meta: Meta { id: id.into(), name: id.into(), ..Default::default() }, rev: rev.into(), verified: rev == "main" };
        m.add.mine = Some(vec![e("a", "refs/pr/4"), e("b", "main")]);
        m.add.store = Some(Vec::new());
        m.mods_show(grid::Show::Mine, &mut out);
        m.add.account = Some("me".into());
        let t0 = m.add.browse_acts();
        assert_eq!((m.add.page, m.add.b.show, m.sel, m.add.rows(), m.add.busy), (Page::Mine, grid::Show::Mine, 0, t0 + 2, false));
        assert_eq!(m.add.words(), ["Newest", "A-Z", "Uploader", "Refresh", "Remove all uploads"]);
        m.sel = t0;
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.as_ref().is_some_and(|c| c.row == t0 && c.opts[0] == "Take down") && m.add.status.contains("closes its pull request"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        m.mods_step(Press { right: true, ..default() }, &mut out);
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.as_ref().is_some_and(|c| c.row == t0 + 1 && c.opts[0] == "Ask to delete") && m.add.status.contains("maintainer"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.add.hovered(m.sel).map(|x| x.meta.id.as_str()), Some("b"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.screen, Screen::Main);
    }

    /// B445: Remove all uploads (a word in My uploads' heading) puts up the warning on Cancel: it takes every press, ○
    /// and Cancel close it with nothing removed, ↓ then ✕ on Remove all starts the take-downs (here signed out:
    /// nothing goes, it says so); none to remove or signed out, it says why instead of asking.
    #[test]
    fn remove_all_uploads_asks_first() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let mut out = Vec::new();
        let e = |id: &str, rev: &str| Entry { meta: Meta { id: id.into(), name: id.into(), ..Default::default() }, rev: rev.into(), verified: rev == "main" };
        m.add.mine = Some(vec![e("a", "refs/pr/4"), e("b", "main")]);
        m.add.store = Some(Vec::new());
        m.mods_show(grid::Show::Mine, &mut out);
        // signed out: no popup
        m.sel = word(&m, "Remove all uploads");
        m.mods_step(ok(), &mut out);
        assert!(m.add.wipe.is_none() && m.add.status.starts_with("Sign in"), "{}", m.add.status);
        m.add.account = Some("me".into());
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.wipe, m.add.child.is_none(), browse_legend(&m).is_none()), (Some(1), true, true));
        // the popup takes the moves and the kind tabs; ✕ on Cancel closes it
        let at = m.sel;
        m.mods_step(Press { kind_next: true, ..default() }, &mut out);
        m.mods_step(Press { right: true, ..default() }, &mut out);
        assert_eq!((m.add.wipe, m.sel, m.add.kind), (Some(0), at, Kind::Character));
        m.mods_step(Press { up: true, ..default() }, &mut out);
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.wipe, m.add.busy, m.add.mine.as_ref().map(Vec::len)), (None, false, Some(2)));
        // ○ closes it too
        m.mods_step(ok(), &mut out);
        *m.mods_cursor() = 0;
        assert_eq!(m.add.wipe, Some(0));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.add.wipe, m.screen), (None, Screen::Mods));
        // Remove all with no token kept: nothing taken down, signed out (never pressed with a real token kept here:
        // that would take this machine's account's uploads down)
        if modstore::account().is_none() {
            m.mods_step(ok(), &mut out);
            m.mods_step(Press { down: true, ..default() }, &mut out);
            m.mods_step(ok(), &mut out);
            assert_eq!((m.add.wipe, m.add.busy, m.add.account.as_deref()), (None, false, None));
            assert!(m.add.status.starts_with("Signed out"), "{}", m.add.status);
        }
        // the last one back says what came of them all
        (m.add.wiping, m.add.wiped) = (VecDeque::new(), (1, 1, 2));
        m.add.wipe_next();
        assert_eq!((m.add.wiped, m.add.status.as_str()), ((0, 0, 0), "Removed 1 of 2 uploads; 1 couldn't be: Refresh, then Remove all uploads again."));
        // none on the store: no popup
        m.add.account = Some("me".into());
        m.add.mine = Some(Vec::new());
        m.mods_step(ok(), &mut out);
        assert!(m.add.wipe.is_none() && m.add.status.starts_with("None of your mods"), "{}", m.add.status);
    }

    /// B255/B372: △ on a tile lists only that uploader's mods with Download all, which queues each one not installed at
    /// the same or a newer version (tagged Queued) and leaves the rest; Back returns to the whole grid on the tile it
    /// came from.
    #[test]
    fn browse_by_author() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let e = |id: &str, who: &str, ver: &str| Entry {
            meta: Meta { id: id.into(), name: id.to_uppercase(), author: who.into(), version: ver.into(), sha256: id.repeat(32), size: 1, ..Default::default() },
            rev: "main".into(),
            verified: true,
        };
        m.add.store = Some(vec![e("a1", "ann", "1"), e("b1", "bob", "1"), e("a2", "ann", "2"), e("a3", "ann", "1.1"), e("a4", "ann", "3")]);
        // a1 installed as listed; a2 at its version (2.0), a3 at an older one, a4 at a newer one (another zip each)
        let mut have = vec![("ann/a1".to_string(), ("a1".repeat(32), PathBuf::from("mods/a1")))];
        for (id, ver) in [("a2", "2.0"), ("a3", "1.0"), ("a4", "4")] {
            let dir = root().join(format!("t_b255_{id}"));
            crate::modfs::add(&dir.join("mod.json"), format!(r#"{{"version":"{ver}"}}"#).into_bytes().into());
            have.push((format!("ann/{id}"), ("x".into(), dir)));
        }
        m.add.have = have.into_iter().collect();
        // △ on A3's tile (A-Z: A1 A2 A3 A4 B1) lists every mod of ann's
        m.mods_page(Page::Browse, 0);
        m.add.b.sort = grid::Sort::Az;
        let from = m.add.browse_acts() + 2;
        m.sel = from;
        let mut out = Vec::new();
        m.mods_step(Press { card: true, ..default() }, &mut out);
        let ids = |l: &[Entry]| l.iter().map(|x| x.meta.id.clone()).collect::<Vec<_>>();
        let t0 = m.add.browse_acts();
        assert_eq!((ids(m.add.entries()), m.sel, m.add.rows()), (vec!["a1".to_string(), "a2".into(), "a3".into(), "a4".into()], t0, t0 + 4));
        assert_eq!(m.add.status, "4 mods by ann.");
        assert_eq!(m.add.words(), ["Newest", "A-Z", "Uploader", "Download all", "All mods"]);
        // △ again in the author's list says so; □ saves one (its Show filter then lists it)
        m.mods_step(Press { card: true, ..default() }, &mut out);
        assert!(m.add.status.starts_with("These are all ann's"), "{}", m.add.status);
        m.mods_step(Press { page: true, ..default() }, &mut out);
        m.add.b.show = grid::Show::Wish;
        assert_eq!(ids(&m.add.tiles().into_iter().cloned().collect::<Vec<_>>()), ["a1"]);
        m.add.b.show = grid::Show::All;
        // (something under way: Download all only queues, nothing is fetched here)
        let busy = e("zz", "zed", "1");
        (m.settings.8, m.add.doing) = (1, vec![busy.clone()]);
        // Download all, the heading's fourth word
        m.sel = word(&m, "Download all");
        m.mods_step(ok(), &mut out);
        assert_eq!(ids(&Vec::from(m.add.queue.clone())), ["a3"]);
        assert_eq!(m.add.jobs.get("ann/a3").map(|j| j.0), Some("Queued"));
        assert!(m.add.status.starts_with("Downloading 1 mods by ann (3 installed"), "{}", m.add.status);
        // again: nothing left to queue
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.starts_with("All of ann's mods") && m.add.queue.len() == 1);
        // a press on a mod outside Download all queues it too
        m.add.have.remove("ann/a1");
        m.sel = t0;
        m.mods_step(ok(), &mut out);
        assert_eq!(ids(&Vec::from(m.add.queue.clone())), ["a3", "a1"]);
        assert!(m.add.status.contains("is queued"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.add.page, m.sel, m.add.entries().len()), (Page::Browse, from, 5));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.screen, Screen::Main);
        assert!(modstore::newer("1.10", "1.9") && !modstore::newer("1.0", "1") && !modstore::newer("", "1") && modstore::newer("2", ""));
    }

    /// B373: the kind tabs filter every page (a court in the store shows on Courts only, never on Characters), L2 /
    /// R2 and a tap switch them on every page, and a kind the store doesn't carry yet is an empty page naming its task
    /// where only the tabs and ○ do anything; the legend lists L2 / R2. M8f9: courts and umpires are carried: their
    /// tabs list the store's and the installed ones of their kind (counted per tab), and Browse's rail drops the
    /// character-only filters there.
    #[test]
    fn kind_tabs_filter_every_page() {
        let e = |id: &str, kind: Kind| Entry { meta: Meta { id: id.into(), name: id.into(), author: "ann".into(), sha256: "1".repeat(64), size: 1, kind, ..Default::default() }, rev: "main".into(), verified: true };
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.list = vec![a_mod("a", "A"), a_mod("b", "B")];
        m.add.courts = vec![a_mod("gag_c01", "Court 01")];
        m.add.umpires = vec![a_mod("gag_ump00", "Umpire 00"), a_mod("gag_ump01", "Umpire 01")];
        m.add.store = Some(vec![e("c1", Kind::Character), e("court", Kind::Court), e("c2", Kind::Character), e("pack", Kind::Textures), e("new", Kind::Other), e("ump", Kind::Umpire)]);
        m.add.mine = m.add.store.clone();
        let mut out = Vec::new();
        let ids = |m: &Menu| m.add.items().iter().map(|e| e.meta.id.clone()).collect::<Vec<_>>();
        let r2 = Press { kind_next: true, ..default() };
        let l2 = Press { kind_prev: true, ..default() };
        // B383: the one tab row: the kinds with mods (a pack in the store isn't installable yet: no Texture packs tab)
        assert_eq!(m.add.kinds_shown(), [Kind::Character, Kind::Court, Kind::Umpire]);
        for page in PAGES {
            m.mods_page(page, 0);
            m.add.relist();
            m.add.kind = Kind::Character;
            let local = page == Page::Upload;
            let names = |m: &Menu| if local { m.add.locals().iter().map(|x| x.id.clone()).collect() } else { ids(m) };
            if page == Page::Installed {
                assert_eq!(names(&m), ["a", "b"]);
                assert_eq!(m.add.kind_counts(), [2, 1, 2, 0]);
            } else if local {
                assert_eq!(m.add.rows() - m.add.acts(), 2, "{page:?}");
                assert_eq!(m.add.kind_counts(), [2, 1, 2, 0]);
            } else {
                assert_eq!(names(&m), ["c1", "c2"], "{page:?}");
                assert_eq!(m.add.kind_counts(), [2, 1, 1, 0], "a pack in the store isn't installable yet");
            }
            // R2 to Courts: the court only, installed or in the store; on the search box, not typing
            m.sel = 3;
            m.mods_step(r2, &mut out);
            assert_eq!((m.add.kind, m.sel, m.add.b.focus), (Kind::Court, 0, false), "{page:?}");
            assert!(!m.add.empty_kind());
            match page {
                Page::Installed => assert_eq!(names(&m), ["gag_c01"]),
                Page::Upload => assert_eq!(m.add.local(m.add.acts()).unwrap().id, "gag_c01"),
                _ => assert_eq!(names(&m), ["court"], "{page:?}"),
            }
            if page == Page::Browse {
                assert!(m.add.rail().iter().all(|o| !matches!(o, grid::Opt::Style(_) | grid::Opt::Left(_))));
            }
            // and Umpires
            m.mods_step(r2, &mut out);
            assert_eq!(m.add.kind, Kind::Umpire);
            match page {
                Page::Installed => assert_eq!(names(&m), ["gag_ump00", "gag_ump01"]),
                Page::Upload => assert_eq!(m.add.local(m.add.acts() + 1).unwrap().id, "gag_ump01"),
                _ => assert_eq!(names(&m), ["ump"], "{page:?}"),
            }
            // R2 round to Characters (Texture packs is hidden), L2 back; a tap on Characters' tab
            m.mods_step(r2, &mut out);
            assert_eq!(m.add.kind, Kind::Character);
            m.mods_step(l2, &mut out);
            assert_eq!(m.add.kind, Kind::Umpire);
            m.add.kind = Kind::Court;
            let tab = super::super::pointer::tests::hits(&m).0.into_iter().find(|h| h.act == super::super::pointer::Act::Kind(0)).unwrap().rect;
            assert_eq!(super::super::pointer::tests::act_at(&m, tab[0] + 4.0, tab[1] + 4.0), Some(super::super::pointer::Act::Kind(0)));
            m.point(super::super::Dev::Keys, super::super::pointer::Act::Kind(0), true);
            assert_eq!(m.add.kind, Kind::Character);
            // Texture packs (if it's ever chosen): nothing to choose, the empty state's words name M5
            m.add.kind = Kind::Textures;
            m.sel = 0;
            assert_eq!(m.add.rows(), 0, "{page:?}");
            assert!(info(&m).contains("mods/texture-replacements/"));
            for k in [ok(), Press { down: true, ..default() }, Press { card: true, ..default() }] {
                m.mods_step(k, &mut out);
                assert_eq!((m.screen, m.add.kind, m.sel), (Screen::Mods, Kind::Textures, 0));
            }
            assert_eq!(m.add.kinds_shown(), [Kind::Character, Kind::Court, Kind::Umpire, Kind::Textures]);
        }
        // Download all by an uploader leaves the other kinds out
        m.add.page = Page::Browse;
        m.add.kind = Kind::Character;
        m.add.narrow(Some("ann".into()), 0);
        assert_eq!(m.add.entries().iter().map(|e| e.meta.id.as_str()).collect::<Vec<_>>(), ["c1", "c2"]);
        let acts = super::super::legend::acts(&m, super::super::Dev::Keys);
        assert!(acts.iter().any(|(a, l)| *a == super::super::legend::Act::Kind && l == "Kind"));
        // ○ from an empty kind leaves for the main menu; its legend has no moves or ✕
        m.add.by = None;
        m.add.kind = Kind::Textures;
        let acts = super::super::legend::acts(&m, super::super::Dev::Keys);
        assert_eq!(acts.iter().map(|a| a.1.as_str()).collect::<Vec<_>>(), ["Kind", "Back"]);
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.screen, Screen::Main);
    }

    /// B372: the pictures asked for: every tile on view first, then one more screen the way the grid scrolled (nearest
    /// row first), at most `cap` fetching at once; one back (`got`) frees a turn; far-off loaded ones go past KEEP.
    #[test]
    fn pictures_on_view_then_ahead() {
        assert_eq!(grid::plan(30, 5, 0, 2, true), (0..20).collect::<Vec<_>>());
        assert_eq!(grid::plan(23, 5, 2, 2, true), (10..23).collect::<Vec<_>>());
        // scrolled up: the screen above, its lowest row first
        let up = grid::plan(40, 5, 4, 2, false);
        assert_eq!(&up[..10], (20..30).collect::<Vec<_>>());
        assert_eq!(&up[10..], [15, 16, 17, 18, 19, 10, 11, 12, 13, 14]);
        let mut m = Menu { screen: Screen::Mods, grid: super::super::Grid::fit(16.0 / 9.0), ..default() };
        let e = |i: usize| Entry { meta: Meta { id: format!("m{i:03}"), name: format!("M{i:03}"), sha256: format!("{i:064x}"), size: 1, ..Default::default() }, rev: "main".into(), verified: true };
        m.add.store = Some((0..200).map(e).collect());
        m.mods_page(Page::Browse, 0);
        m.add.b.sort = grid::Sort::Az;
        let l = grid::lay(&m.grid);
        let on_view = l.cols * l.rows;
        let go = grid::want(&mut m.add, &l, 3);
        assert_eq!(go.iter().map(|e| e.meta.name.as_str()).collect::<Vec<_>>(), ["M000", "M001", "M002"]);
        assert!(grid::want(&mut m.add, &l, 3).is_empty(), "three on their way: no more");
        grid::got(&mut m.add, go[0].meta.sha256.clone(), None);
        assert_eq!(grid::want(&mut m.add, &l, 3).iter().map(|e| e.meta.name.as_str()).collect::<Vec<_>>(), ["M003"]);
        // every one on view and the next screen gets asked for, then nothing more
        let mut n = 4;
        loop {
            let go = grid::want(&mut m.add, &l, 1000);
            if go.is_empty() {
                break;
            }
            n += go.len();
        }
        assert_eq!(n, 2 * on_view);
        // a scroll down asks for the screen after; loaded ones far away go once past KEEP
        m.add.top = l.rows;
        m.add.b.down = true;
        let go = grid::want(&mut m.add, &l, 1000);
        assert_eq!(go.len(), on_view);
        assert_eq!(go[0].meta.name, format!("M{:03}", 2 * on_view));
        let shas: Vec<String> = m.add.b.pics.keys().cloned().collect();
        for (k, sha) in shas.into_iter().enumerate() {
            grid::got(&mut m.add, sha, Some((k as u32, 1.0)));
        }
        let free = grid::evict(&mut m.add, &l);
        let shown = m.add.b.pics.values().filter(|p| matches!(p, grid::Pic::Shown(..))).count();
        assert!(shown <= 160 && (3 * on_view <= 160 || !free.is_empty()), "{shown} {}", free.len());
    }

    /// B372: the tile pictures' cache is keyed by the zip's sha256: a picture kept is read back, "none" (an empty file)
    /// reads as none, an unknown zip isn't cached; `shrink` makes an old upload's preview small once.
    #[test]
    fn pictures_cache_by_sha256() {
        let dir = std::env::temp_dir().join(format!("hst-b372-{}", std::process::id()));
        let (a, b, c) = ("ab".repeat(32), "cd".repeat(32), "ef".repeat(32));
        let mut big = Vec::new();
        image::DynamicImage::new_rgba8(600, 400).write_to(&mut std::io::Cursor::new(&mut big), image::ImageFormat::Png).unwrap();
        let small = modstore::shrink(&big, modstore::THUMB).unwrap();
        let img = image::load_from_memory(&small).unwrap();
        assert_eq!((img.width(), img.height()), (256, 171));
        modstore::keep_thumb(&dir, &a, Some(&small));
        modstore::keep_thumb(&dir, &b, None);
        assert_eq!(modstore::thumb_file(&dir, &a), dir.join(format!("{a}.png")));
        assert_eq!(modstore::cached_thumb(&dir, &a), Some(Some(small)));
        assert_eq!(modstore::cached_thumb(&dir, &b), Some(None));
        assert_eq!(modstore::cached_thumb(&dir, &c), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// B372: ✕ on the search box opens the on-screen keyboard: arrows move round it, ✕ types its key, □ deletes,
    /// Done (or ○) closes it; the tiles follow the search.
    #[test]
    fn search_with_the_keyboard() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let e = |n: &str| Entry { meta: Meta { id: n.into(), name: n.into(), sha256: n.repeat(64), size: 1, ..Default::default() }, rev: "main".into(), verified: true };
        m.add.store = Some(vec![e("Kent"), e("Carol"), e("Kim")]);
        m.mods_page(Page::Browse, 0);
        let mut out = Vec::new();
        let p = |f: fn(&mut Press)| {
            let mut k = Press::default();
            f(&mut k);
            k
        };
        // B383: the box never takes the keys by itself; from the keyboard ✕ types into it in place, and leaving it
        // (or ✕ again) stops
        assert_eq!((m.sel, m.add.b.focus, m.add.b.osk), (0, false, None));
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.b.focus, m.add.b.osk), (true, None));
        m.mods_step(ok(), &mut out);
        assert!(!m.add.b.focus);
        m.mods_step(ok(), &mut out);
        m.mods_step(p(|k| k.down = true), &mut out);
        assert_eq!((m.sel != 0, m.add.b.focus), (true, false));
        m.sel = 0;
        // a pad's ✕ opens the on-screen keyboard
        m.add.pad = true;
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.b.osk, Some(0));
        // K is the second row's first key; then I (row one, column nine)
        m.mods_step(p(|k| k.down = true), &mut out);
        m.mods_step(ok(), &mut out);
        m.mods_step(p(|k| k.up = true), &mut out);
        m.mods_step(p(|k| k.left = true), &mut out);
        m.mods_step(p(|k| k.left = true), &mut out);
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.b.search, "ki");
        assert_eq!(m.add.tiles().iter().map(|e| e.meta.name.as_str()).collect::<Vec<_>>(), ["Kim"]);
        m.mods_step(p(|k| k.page = true), &mut out);
        assert_eq!(m.add.tiles().len(), 2);
        // ↑ from the top row: the wide keys (Done is the last)
        m.mods_step(p(|k| k.up = true), &mut out);
        assert_eq!(m.add.b.osk, Some(43));
        assert_eq!(*m.mods_cursor(), 43);
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.b.osk, m.sel, m.add.b.search.as_str()), (None, 0, "k"));
        // □ on the box clears it
        m.mods_step(p(|k| k.page = true), &mut out);
        assert!(m.add.b.search.is_empty());
    }
}
