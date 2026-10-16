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
use super::{CARD_FILL, Draw, Effect, GOLD, INK, Menu, Press, Screen, Tex, WHITE};
use crate::modstore::{self, Entry, Event, Kind, Up};
use crate::mods::Mod;

mod grid;
mod packs;

/// B471: which of L1 / R1 (the Show tabs) do nothing on `v`: the legend dims their hint.
pub(super) fn show_ends(v: &View) -> [bool; 2] {
    grid::show_ends(v)
}

// The mods folder, in hst-base (`web.rs` reads it too)
pub(crate) use hst_base::modfs::{root, set_root};

/// The pills above the list on Installed (Add mod, Reload, Remove all) and Upload (Sign in, Upload all): the rows
/// before the mods.
const TOP: usize = 3;
const UP_TOP: usize = 2;
/// The tabs across the title band, in Q/E order.
const PAGES: [Page; 4] = [Page::Browse, Page::Installed, Page::Upload, Page::Mine];

/// M114: how long the footer says "Uploaded N mods · waiting for review" before it goes back to the count installed.
const UPPED_SECS: f32 = 4.0;
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
    /// M5b: the texture packs (`mods::read_any`'s headers), and as Installed's list has them: in their order, on or
    /// off (`packs.rs`).
    textures: Vec<Mod>,
    packs: Vec<packs::Row>,
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
    /// M114: how many went up since none were last under way, and the seconds left of the footer's "Uploaded N mods
    /// · waiting for review" once the last of them is done.
    upped: (usize, f32),
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
    /// B420: the iOS / Android app, which can't start itself again: Reload closes it instead (the player opens it
    /// again) and the popup asks to close.
    app: PhoneApp,
}

/// B420: whether this is the phone apps' build (`View::app`); `HST_APP=1` makes a desktop `--shot` show theirs.
#[derive(Clone, Copy, Debug)]
struct PhoneApp(bool);

impl Default for PhoneApp {
    fn default() -> Self {
        PhoneApp(cfg!(any(target_os = "ios", target_os = "android")))
    }
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
        self.textures = crate::mods::list_kind(root(), &crate::mods::Kind::Textures);
        self.packs = packs::read(root());
        self.have = self.list.iter().chain(&self.courts).chain(&self.umpires).chain(&self.textures).filter_map(|m| modstore::stamp(&m.dir).map(|s| (format!("{}/{}", s.author, m.id), (s.sha256, m.dir.clone())))).collect();
        self.child = None;
        self.relist();
    }

    /// B383: the installed mods as tiles (Show → Installed): a store mod's own listing (its picture, its Update), else
    /// an entry made from its `mod.json` and the select's card (`rev` "local:<folder>": no store picture, the
    /// select's drawing instead).
    pub(super) fn relist(&mut self) {
        let store = self.store.as_deref().unwrap_or_default();
        let all = [(&self.list, Kind::Character), (&self.courts, Kind::Court), (&self.umpires, Kind::Umpire), (&self.textures, Kind::Textures)];
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

    /// B250: upload `dir`'s news `u`, beside the mod (M114: and in the footer's count; the last one done starts its
    /// "Uploaded N mods").
    fn uploaded(&mut self, dir: PathBuf, u: Up) {
        match &u {
            Up::Done(_) => {
                self.store = None;
                self.mine = None;
                self.upped.0 += 1;
            }
            Up::Skipped(s) => self.status = s.clone(),
            Up::Failed(e) => self.status = format!("Not uploaded: {e}"),
            _ => {}
        }
        let end = !matches!(u, Up::Queued | Up::Uploading);
        self.ups.insert(dir, u);
        if end && self.uploads().1 == 0 && self.upped.0 > 0 {
            self.upped.1 = UPPED_SECS;
        }
    }

    /// M114: the mod uploading now (its name) and how many uploads are queued or under way.
    fn uploads(&self) -> (Option<String>, usize) {
        let n = self.ups.values().filter(|u| matches!(u, Up::Queued | Up::Uploading)).count();
        let now = self.ups.iter().find(|(_, u)| **u == Up::Uploading).map(|(dir, _)| {
            let m = self.list.iter().chain(&self.courts).chain(&self.umpires).find(|m| m.dir == *dir);
            m.map(|m| m.name.clone()).unwrap_or_else(|| dir.file_name().unwrap_or_default().to_string_lossy().into_owned())
        });
        (now, n)
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
            Kind::Textures => &self.textures,
            Kind::Other => &[],
        }
    }

    /// M8f9: every installed mod, of any kind (the Installed tab's count).
    fn installed_len(&self) -> usize {
        self.list.len() + self.courts.len() + self.umpires.len() + self.textures.len()
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

    /// B425: player `p`'s rail ends in the Get more chip: 1P's only (another local player mustn't open the store by
    /// accident), on a local select, while no mod of any kind (character, court, umpire) is installed.
    pub(super) fn store_chip(&self, p: usize) -> bool {
        p == 0 && self.screen == Screen::Chars && !self.online() && !self.select_online() && self.mods.is_empty() && self.court_mods.is_empty() && self.umpire_mods.is_empty()
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
        (b.focus, b.tile_at, b.pill_at, b.game, b.filter) = (false, 0, 0, None, 0);
        out.push(Effect::Sound(0));
    }

    /// B383: Show `s`: Browse, Updates and Saved mods filter the store's mods, Installed and My uploads are their
    /// own grids.
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
        // B448: the cursor stays on the pill (every grid page has the pills; another page's words may be more)
        if let Some(i) = self.add.pills().iter().position(|p| *p == grid::Pill::Show(s)) {
            self.sel = 2 + self.add.words().len() + i;
            self.add.b.pill_at = i;
        }
        out.push(Effect::Sound(0));
    }

    /// B471: the Show tab before (`back`) or after the current one, if there is one (Upload: the current one's).
    pub(super) fn show_step(&mut self, back: bool, out: &mut Vec<Effect>) {
        let [first, last] = grid::show_ends(&self.add);
        if back && first || !back && last {
            return;
        }
        let at = grid::SHOWS.iter().position(|s| *s == self.add.b.show).unwrap_or(0);
        self.mods_show(grid::SHOWS[if back { at - 1 } else { at + 1 }], out);
    }

    /// B471: a tap on Show tab `i` (`grid::SHOWS`), from any page.
    pub(super) fn show_tap(&mut self, i: usize, out: &mut Vec<Effect>) {
        if let Some(s) = grid::SHOWS.get(i)
            && (*s != self.add.b.show || !self.add.grid() || self.add.by.is_some())
        {
            self.mods_show(*s, out);
        }
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
            // B373: the kind tabs (an open child closes), stopping at the ends (B462); B383: only the ones shown
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
        // B471: L1 / R1 walk the Show tabs in the top bar from anywhere on the page (a tap's ✕ on the sort's left half
        // carries `prev` too), stopping at the ends as the kinds do
        if (k.prev || k.next) && !k.ok && self.add.child.is_none() {
            return self.show_step(k.prev, out);
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
        // B383: the search box types only while it was pressed and the cursor is on it
        if self.add.grid() && n > 0 && self.add.slot(self.sel) != grid::Slot::Search {
            self.add.b.focus = false;
        }
        // B448: Upload's tiles scroll as the grids' do
        self.browse_scroll();
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
        let _ = n;
        let to = self.upload_move(k);
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
        if mods.is_empty() {
            v.status = "Those are uploading already.".into();
        } else {
            // M114: the footer's status says how they go (on every Mods page)
            v.status = if v.page == Page::Upload { RULE.into() } else { String::new() };
            if v.uploads().1 == 0 {
                v.upped = (0, 0.0);
            }
        }
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
            self.add.status = "Choose a mod .zip file: a character, court, umpire or texture pack.".into();
            out.push(Sound(2));
        } else if s == 1 {
            if let Err(e) = reload(self.add.app.0, out) {
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
/// select's roster, the mods' textures and sounds are all read at startup). B420: the iOS / Android app (`app`) can't
/// start itself, so it only closes, at once (an `AppExit` doesn't end an iOS app); the player opens it again. Mods and
/// settings are on disk already.
fn reload(app: bool, out: &mut Vec<Effect>) -> Result<(), String> {
    if app {
        std::process::exit(0);
    }
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
            if let Err(e) = reload(self.add.app.0, out) {
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

/// B471: whether L1 / R1 walk the Show tabs now (Upload's legend says so too): no question or popup up.
pub(super) fn show_keys(m: &Menu) -> bool {
    m.add.child.is_none() && m.add.nophone.is_none() && m.add.wipe.is_none() && !m.add.empty_kind()
}

/// B420: the popup's question and its two pills: the phone apps can only close.
fn ask_words(v: &View) -> (&'static str, [&'static str; 2], &'static str) {
    if v.app.0 {
        (
            "Close the game to use them?",
            ["Close game", "Later"],
            "Mod changes take effect when the game starts, and the app can't restart itself. Close game shuts it now: open it again to play with them. Later carries on without them.",
        )
    } else {
        ("Reload now to use them?", ["Reload", "Later"], "Mod changes take effect after a restart. Reload restarts the game now; Later carries on without them.")
    }
}

/// B420: what the Mods page's Reload pill does, for its description.
pub(super) fn reload_said(v: &View) -> &'static str {
    if v.app.0 {
        "Close the game so your mod changes take effect: open it again and new mods join the select screens, removed ones leave."
    } else {
        "Restart the game so your mod changes take effect: new mods join the select screens, removed ones leave."
    }
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
    let (ask, words, said) = ask_words(&m.add);
    d.outlined(|d| d.label(ask, 320.0, ASK_TOP - 32.0, 22.0, 288.0, WHITE));
    for (k, word) in words.into_iter().enumerate() {
        let y = ASK_TOP + 40.0 * k as f32;
        let on = k == a;
        d.pill_at([176.0, y, 288.0, 32.0], [INK, WHITE, if on { CHOSEN } else { UNCHOSEN }], 128.0);
        d.hit([176.0, y, 288.0, 32.0], super::pointer::Act::Row(k, super::pointer::OK));
        d.label(word, 320.0, y + 6.0, 20.0, 160.0, if on { WHITE } else { UNCHOSEN.map(|c| c * 1.4) });
    }
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [116.0 + hand_x, ASK_TOP + 40.0 * a as f32, 64.0, 32.0], WHITE, 128.0);
    d.said = said.into();
}

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

/// The faces of the installed mods on the tiles on view, top first: what `previews.rs` bakes and reads first while
/// this page is up, so its tiles fill in without a select visit.
pub(super) fn faces(m: &Menu) -> Vec<super::Who> {
    grid::faces(m)
}

/// B448: the page (`grid.rs` draws every page in the select's look).
pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    grid::layout(d, m, hand_x);
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

/// B250: an upload's tag.
fn up_tag(u: &Up) -> &'static str {
    match u {
        Up::Queued => "Queued",
        Up::Uploading => "Uploading",
        Up::Done(_) => "Uploaded",
        Up::Skipped(_) => "Skipped",
        Up::Failed(_) => "Failed",
    }
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
/// mod's Remove open; B383: the Installed grid) / `reloadmods` (the main menu, B205's popup up; `HST_APP=1`: the phone apps' Close game one): for `--shot`s.
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
        // B420: `HST_APP=1`: the phone apps' popup (Close game)
        if std::env::var_os("HST_APP").is_some() {
            v.app = PhoneApp(true);
        }
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
    // M114: `HST_UPS=1`: the first installed mod uploading, the next two queued (`=2`: and two downloads under way);
    // `=done`: three just went up
    let v = &mut m.add;
    if let Ok(n) = std::env::var("HST_UPS") {
        if n == "done" {
            v.upped = (3, UPPED_SECS);
        } else {
            v.ups = v.list.iter().take(3).enumerate().map(|(i, md)| (md.dir.clone(), if i == 0 { Up::Uploading } else { Up::Queued })).collect();
            if n == "2" {
                v.doing = v.entries().iter().take(2).cloned().collect();
            }
        }
        v.status = if v.page == Page::Upload { RULE.into() } else { String::new() };
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
    time: Res<Time>,
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
    // M114: the footer's "Uploaded N mods" runs out
    if menu.add.upped.1 > 0.0 {
        menu.add.upped.1 = (menu.add.upped.1 - time.delta_secs()).max(0.0);
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
            Event::Upload(dir, u) => v.uploaded(dir, u),
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
            // M116: the store busy, the last listing kept up
            Event::Note(s) => menu.add.status = s,
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
    let l = grid::lay_of(menu);
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

    /// Held by the tests that draw the grid and read where Add mod was last drawn (`grid::ADD_AT`) or draw a lot.
    static DRAWN: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
            upscale: true,
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
        let r = grid::child_rects(m.add.child.as_ref().unwrap(), grid::lay(&m.grid).detail);
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
        assert_eq!(m.add.words(), ["Add mod", "Reload", "Remove all"]);
        m.sel = word(&m, "Remove all");
        let mut out = Vec::new();
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.child.as_ref().map(|c| c.opts.clone()), Some(vec!["Remove all", "Cancel"]));
        m.mods_step(ok(), &mut out);
        assert!(["t_all1", "t_all2"].iter().all(|id| !crate::modfs::is_file(&root().join(id).join("mod.json"))));
        assert!(m.add.status.starts_with("Removed every mod") && m.add.changed, "{}", m.add.status);
    }

    /// B372: Browse's grid: Newest (pull requests newest first, merged ones after), A-Z and Uploader sorts, B448: under
    /// a heading per game (per uploader by Uploader); the search matches names, uploaders and games in any case; the
    /// filter pill's Select picks Game, Plays like or Hand and ✕ / L1 / R1 step its value; ↑↓ go row to row keeping
    /// the column, ↑ off the top to the pills, ↑ again the search box, ↓ back where they were.
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
        assert_eq!(names(&m), ["Alma", "Zed", "Carol", "Kent", "Mia"]);
        let heads = |m: &Menu| m.add.secs().iter().map(|s| (s.name.clone(), s.len)).collect::<Vec<_>>();
        assert_eq!(heads(&m), [("Fore!".into(), 2), ("Get a Grip".into(), 2), ("Tekken".into(), 1)]);
        m.add.b.sort = grid::Sort::Az;
        assert_eq!(names(&m), ["Alma", "Zed", "Carol", "Kent", "Mia"]);
        m.add.b.sort = grid::Sort::Uploader;
        assert_eq!(names(&m), ["Carol", "Mia", "Kent", "Zed", "Alma"]);
        assert_eq!(heads(&m), [("ann".into(), 2), ("bob".into(), 1), ("cal".into(), 1), ("Unknown uploader".into(), 1)]);
        m.add.b.search = "GRIP".into();
        assert_eq!(names(&m), ["Carol", "Kent"]);
        m.add.b.search = "an".into();
        assert_eq!(names(&m), ["Carol", "Mia"]);
        m.add.b.search.clear();
        // the Game filter's options: games by how many mods (one each: Other only past six)
        let games = m.add.options(0);
        let label = |o: &grid::Opt| format!("{o:?}");
        assert!(label(&games[1]).contains("Fore!") && label(&games[2]).contains("Get a Grip") && label(&games[3]).contains("Tekken"), "{games:?}");
        assert_eq!(games.len(), 4);
        // the filter pill: Select to Hand, ✕ Left-handed; Select round to Game, ✕ Fore! (both apply); a tap on its left half back
        let mut out = Vec::new();
        let p0 = 2 + m.add.words().len();
        m.sel = p0 + 6;
        let hand = Press { hand: true, ..default() };
        m.mods_step(hand, &mut out);
        m.mods_step(hand, &mut out);
        m.mods_step(ok(), &mut out);
        assert_eq!((names(&m), m.add.filter_words()), (vec!["Carol".to_string(), "Zed".into()], "Hand: Left-handed".to_string()));
        m.mods_step(hand, &mut out);
        m.mods_step(ok(), &mut out);
        assert_eq!((names(&m), m.add.filter_words()), (vec!["Zed".to_string()], "Game: Fore! +1".to_string()));
        m.mods_step(Press { ok: true, prev: true, ..default() }, &mut out);
        assert_eq!(names(&m), ["Carol", "Zed"]);
        assert_eq!(m.sel, p0 + 6);
        m.add.b.left = false;
        // Show's Updates pill: the installed mod with another zip
        m.sel = p0 + 2;
        m.mods_step(ok(), &mut out);
        assert_eq!((names(&m), m.sel), (vec!["Kent".to_string()], p0 + 2));
        m.sel = p0;
        m.mods_step(ok(), &mut out);
        assert_eq!(names(&m).len(), 5);
        let t0 = m.add.browse_acts();
        // more tiles a row on a 16:9 view than on 4:3
        let cols = grid::lay(&m.grid).cols;
        assert!(grid::lay(&super::super::Grid::fit(16.0 / 9.0)).cols > cols && cols >= 2, "{cols}");
        // moves, by Uploader (ann: Carol, Mia; bob: Kent; cal: Zed; Alma): ← → along, ↓ the next heading's row, off
        // the foot the footer's words, ↑ back to the tile, ↑ off the top the strip's sort and filter (B471), ↑ from
        // the filter the search box
        let (l, r, u, d) = (Press { left: true, ..default() }, Press { right: true, ..default() }, Press { up: true, ..default() }, Press { down: true, ..default() });
        let mut go = |m: &mut Menu, k: Press| {
            m.mods_step(k, &mut Vec::new());
            m.sel
        };
        m.sel = t0 + 1;
        assert_eq!([go(&mut m, l), go(&mut m, l), go(&mut m, r), go(&mut m, d), go(&mut m, d), go(&mut m, d)], [t0, t0, t0 + 1, t0 + 2, t0 + 3, t0 + 4]);
        assert_eq!([go(&mut m, d), go(&mut m, u)], [2, t0 + 4]);
        m.sel = t0 + 1;
        assert_eq!([go(&mut m, u), go(&mut m, r), go(&mut m, u), go(&mut m, d), go(&mut m, d)], [p0 + 5, p0 + 6, 0, p0 + 6, t0 + 1]);
        // B471: the search box at the top bar's right: ← the last Show tab, → stays; the Show tabs' ↓ the sort;
        // Upload in the footer, ← the footer's last word
        m.sel = 0;
        assert_eq!([go(&mut m, r), go(&mut m, l), go(&mut m, r), go(&mut m, d)], [0, p0 + 4, 0, p0 + 6]);
        m.sel = p0 + 1;
        assert_eq!(go(&mut m, d), p0 + 5);
        m.sel = 1;
        assert_eq!(go(&mut m, l), 2 + m.add.words().len() - 1);
        // the sort pill: ✕ the next, a tap on its left half the one before
        m.add.b.sort = grid::Sort::Newest;
        m.sel = p0 + 5;
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.b.sort, names(&m)[0].as_str()), (grid::Sort::Az, "Alma"));
        m.mods_step(Press { ok: true, prev: true, ..default() }, &mut out);
        assert_eq!(m.add.b.sort, grid::Sort::Newest);
        // the legend says what a tile's buttons do; one kind of mod: no kind switch (B383)
        m.sel = t0 + names(&m).iter().position(|n| n == "Kent").unwrap();
        let l = grid::legend(&m).into_iter().map(|x| x.1).collect::<Vec<_>>();
        assert_eq!(l, ["Move", "Update", "More by", "Save", "Show", "Back"]);
        // Upload: Sign in and Upload all side by side, then the mods' tiles (two: one row)
        m.mods_page(Page::Upload, 0);
        m.add.list = vec![a_mod("a", "A"), a_mod("b", "B")];
        let (l, r, u, d) = (Press { left: true, ..default() }, Press { right: true, ..default() }, Press { up: true, ..default() }, Press { down: true, ..default() });
        assert_eq!([go(&mut m, r), go(&mut m, r), go(&mut m, l), go(&mut m, d), go(&mut m, r), go(&mut m, d), go(&mut m, u), go(&mut m, u), go(&mut m, d)], [1, 1, 0, UP_TOP, UP_TOP + 1, UP_TOP + 1, 0, 0, UP_TOP + 1]);
        // no mods under the pills: ↓ stays
        m.add.list.clear();
        m.sel = 1;
        assert_eq!(go(&mut m, d), 1);
    }

    /// B448: Upload's tiles scroll as the grids' do, keeping the cursor's row (and its heading) on screen, and taps
    /// land on the drawn tiles; the browser's file input sits on Installed's Add mod word.
    #[test]
    fn scrolls_and_taps() {
        let _one = DRAWN.lock().unwrap_or_else(|e| e.into_inner());
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.list = (0..40).map(|i| a_mod(&format!("m{i}"), &format!("M{i}"))).collect();
        m.mods_page(Page::Upload, 0);
        let mut out = Vec::new();
        let lay = grid::lay(&m.grid);
        let down = |m: &mut Menu, n: usize| (0..n).for_each(|_| m.mods_step(Press { down: true, ..default() }, &mut Vec::new()));
        down(&mut m, 9);
        let i = m.sel - UP_TOP;
        assert!(i >= lay.cols * 4, "{i}");
        let lines = grid::lines(&m.add.secs(), lay.cols);
        let (_, tiles) = lay.laid(&lines, m.add.top);
        assert!(m.add.top > 0);
        let r = tiles.iter().find(|t| t.0 == i).expect("the cursor's tile on view").1;
        let at = |m: &Menu, x: f32, y: f32| super::super::pointer::tests::act_at(m, x, y);
        assert_eq!(at(&m, r[0] + 10.0, r[1] + 5.0), Some(super::super::pointer::Act::Tile(m.sel)));
        // to the last row and back up: the heading shows again at the top
        down(&mut m, 20);
        assert_eq!(m.add.top, lay.last_top(&lines));
        (0..20).for_each(|_| m.mods_step(Press { up: true, ..default() }, &mut out));
        assert_eq!((m.sel, m.add.top), (0, 0));
        // B383: the header's Upload button is lit here; a tap on it goes back to the grid
        assert!(add_rect(&m, 4.0 / 3.0, 1.0).is_none());
        let u = lay.upload;
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
        let l = grid::lay(&m.grid);
        let c = l.laid(&grid::lines(&m.add.secs(), l.cols), 0).1[2].1;
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
        // (M114: the footer's status says it)
        assert!(!m.add.busy && m.add.status == RULE);
        assert_eq!(grid::status(&m), ("2 uploading".to_string(), true));
        assert!(m.add.list.iter().all(|x| m.add.ups.get(&x.dir) == Some(&Up::Queued)));
        m.sel = UP_TOP;
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.is_none() && m.add.status == "A is uploading already.");
        m.queue(m.add.list.clone(), "me".into(), "hf_test".into());
        assert_eq!(m.add.status, "Those are uploading already.");
        // the menu stays usable meanwhile: My uploads and back
        m.add.mine = Some(Vec::new());
        m.mods_show(grid::Show::Mine, &mut out);
        assert_eq!((m.add.page, m.add.slot(m.sel)), (Page::Mine, grid::Slot::Pill(3)));
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

    /// M114: uploads in the footer's status on any page, over the last action's words, after the downloads when
    /// both run; then "Uploaded N mods · waiting for review" till it runs out, then the count installed again.
    #[test]
    fn upload_status() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.mods_page(Page::Browse, 0);
        m.add.list = vec![a_mod("t_st_a", "A"), a_mod("t_st_b", "B"), a_mod("t_st_c", "C")];
        let st = |m: &Menu| grid::status(m);
        m.add.status.clear();
        assert_eq!(st(&m), ("3 installed".into(), false));
        let dirs: Vec<_> = m.add.list.iter().map(|x| x.dir.clone()).collect();
        for d in &dirs {
            m.add.ups.insert(d.clone(), Up::Queued);
        }
        m.add.ups.insert(dirs[0].clone(), Up::Uploading);
        m.add.status = "Something said.".into();
        assert_eq!(st(&m), ("Uploading A · 2 queued".into(), true));
        m.add.ups.insert(dirs[1].clone(), Up::Done(String::new()));
        assert_eq!(st(&m), ("Uploading A · 1 queued".into(), true));
        m.add.ups.insert(dirs[2].clone(), Up::Done(String::new()));
        assert_eq!(st(&m), ("Uploading A".into(), true));
        // downloads first
        let e = Entry { meta: Default::default(), rev: "main".into(), verified: true };
        m.add.doing = vec![e.clone(), e.clone()];
        assert_eq!(st(&m), ("2 downloading · 1 uploading".into(), true));
        m.add.queue.push_back(e);
        assert_eq!(st(&m), ("2 downloading · 1 queued · 1 uploading".into(), true));
        (m.add.doing, m.add.queue) = (Vec::new(), Default::default());
        // the last one up starts the few seconds
        (m.add.upped, m.add.status) = ((2, 0.0), String::new());
        m.add.uploaded(dirs[0].clone(), Up::Done("A is up.".into()));
        assert_eq!((m.add.upped.0, m.add.status.as_str()), (3, ""));
        assert_eq!(st(&m), ("Uploaded 3 mods · waiting for review".into(), false));
        m.add.upped.1 = 0.0;
        assert_eq!(st(&m), ("3 installed".into(), false));
        // a failure stays said once the uploads are over (`queue` starts a new count)
        m.add.upped = (0, 0.0);
        m.add.ups.insert(dirs[0].clone(), Up::Uploading);
        m.add.uploaded(dirs[0].clone(), Up::Failed("no network".into()));
        assert_eq!(st(&m), ("Not uploaded: no network".into(), false));
    }

    /// B208: on a phone Add mod (B383: Installed's word) and the header's Upload are greyed and put up the popup
    /// instead (no picker, no page), as does Show → My uploads; ✕, ○ or a tap anywhere closes it; the other Shows
    /// still open; the file input stays off.
    /// B420: the phone apps can't restart, so the popup asks to close the game (Later still closes only the popup) and
    /// the Reload pill says it closes; the desktop's keeps Reload.
    #[test]
    fn app_popup_closes_the_game() {
        let mut m = Menu { screen: Screen::Main, ..default() };
        assert_eq!(ask_words(&m.add).1, ["Reload", "Later"]);
        assert!(reload_said(&m.add).starts_with("Restart"));
        m.add.app = PhoneApp(true);
        let (ask, words, said) = ask_words(&m.add);
        assert_eq!((ask, words), ("Close the game to use them?", ["Close game", "Later"]));
        assert!(said.contains("can't restart") && reload_said(&m.add).starts_with("Close the game"));
        (m.add.changed, m.add.ask) = (true, Some(0));
        let mut out = Vec::new();
        assert!(m.ask_step(Press { down: true, ..default() }, &mut out));
        assert!(m.ask_step(ok(), &mut out));
        assert_eq!((m.add.ask, m.add.changed), (None, true));
        assert_eq!(out, [Effect::Sound(0), Effect::Sound(1)]);
    }

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
        assert_eq!((m.add.page, m.add.b.show, m.sel, m.add.rows(), m.add.busy), (Page::Mine, grid::Show::Mine, t0 - 4, t0 + 2, false));
        assert_eq!(m.add.words(), ["Refresh", "Remove all uploads"]);
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
        assert_eq!(m.add.words(), ["Download all", "All mods"]);
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
        let _one = DRAWN.lock().unwrap_or_else(|e| e.into_inner());
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
        // B383: the one tab row: the kinds with mods (M5b: the store's pack brings Texture packs')
        assert_eq!(m.add.kinds_shown(), [Kind::Character, Kind::Court, Kind::Umpire, Kind::Textures]);
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
                assert_eq!(m.add.kind_counts(), [2, 1, 1, 1]);
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
                assert_eq!(m.add.groups(), 1);
                assert!(m.add.options(0).iter().all(|o| !matches!(o, grid::Opt::Style(_) | grid::Opt::Left(_))));
            }
            // and Umpires
            m.mods_step(r2, &mut out);
            assert_eq!(m.add.kind, Kind::Umpire);
            match page {
                Page::Installed => assert_eq!(names(&m), ["gag_ump00", "gag_ump01"]),
                Page::Upload => assert_eq!(m.add.local(m.add.acts() + 1).unwrap().id, "gag_ump01"),
                _ => assert_eq!(names(&m), ["ump"], "{page:?}"),
            }
            // M5b: and Texture packs (none installed here: the store's only)
            m.mods_step(r2, &mut out);
            assert_eq!(m.add.kind, Kind::Textures);
            assert!(!m.add.empty_kind());
            match page {
                Page::Installed => assert!(names(&m).is_empty()),
                Page::Upload => assert_eq!(m.add.rows(), m.add.acts()),
                _ => assert_eq!(names(&m), ["pack"], "{page:?}"),
            }
            // B462: R2 on the last tab stays, L2 back; a tap on Characters' tab
            m.mods_step(r2, &mut out);
            assert_eq!(m.add.kind, Kind::Textures, "{page:?}: R2 doesn't wrap");
            m.mods_step(l2, &mut out);
            m.mods_step(l2, &mut out);
            assert_eq!(m.add.kind, Kind::Court);
            m.add.kind = Kind::Court;
            let tab = super::super::pointer::tests::hits(&m).0.into_iter().find(|h| h.act == super::super::pointer::Act::Kind(0)).unwrap().rect;
            assert_eq!(super::super::pointer::tests::act_at(&m, tab[0] + 4.0, tab[1] + 4.0), Some(super::super::pointer::Act::Kind(0)));
            m.point(super::super::Dev::Keys, super::super::pointer::Act::Kind(0), true);
            assert_eq!(m.add.kind, Kind::Character);
        }
        // Download all by an uploader leaves the other kinds out
        m.add.page = Page::Browse;
        m.add.kind = Kind::Character;
        m.add.narrow(Some("ann".into()), 0);
        assert_eq!(m.add.entries().iter().map(|e| e.meta.id.as_str()).collect::<Vec<_>>(), ["c1", "c2"]);
        let acts = super::super::legend::acts(&m, super::super::Dev::Keys);
        assert!(acts.iter().any(|(a, l)| *a == super::super::legend::Act::Kind && l == "Kind"));
        // ○ from Texture packs leaves for the main menu
        m.add.by = None;
        m.add.kind = Kind::Textures;
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.screen, Screen::Main);
    }

    /// M5b: Installed → Texture packs lists the packs in their order (not sorted), each row with its name, author, count
    /// and size; ✕ switches the cursor's one, △ / □ move it (the cursor follows; at an end it stays, with a no), each
    /// saved at once; a tap on a row's switch or arrows does the same; the legend and the panel name them; any aspect.
    #[test]
    fn texture_packs_list() {
        let _one = DRAWN.lock().unwrap_or_else(|e| e.into_inner());
        use super::super::pointer::{Act, OK, tests::{act_at, hits}};
        let was = crate::mods_textures::order();
        let row = |id: &str, name: &str, on: bool| packs::Row { dir: root().join(id), folder: id.into(), name: name.into(), author: "ann".into(), count: 3, bytes: 3 << 20, on };
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.textures = vec![a_mod("hi", "Zed HD"), a_mod("lo", "Alpha HD"), a_mod("ui", "Menu HD")];
        m.add.packs = vec![row("hi", "Zed HD", true), row("lo", "Alpha HD", false), row("ui", "Menu HD", true)];
        installed(&mut m);
        m.add.kind = Kind::Textures;
        m.add.relist();
        let first = m.add.browse_acts();
        let names = |m: &Menu| m.add.tiles().iter().map(|e| e.meta.id.clone()).collect::<Vec<_>>();
        assert_eq!(names(&m), ["hi", "lo", "ui"], "in pack order, not A-Z");
        assert_eq!(grid::lay_of(&m).cols, 1);
        // no sort or filter: ↑ from the top row is the Show tabs' Installed, ↓ back, ↓ the next row
        assert_eq!(m.add.pills().len(), grid::SHOWS.len());
        m.sel = first;
        m.mods_step(Press { up: true, ..default() }, &mut Vec::new());
        assert_eq!(m.add.slot(m.sel), grid::Slot::Pill(grid::SHOWS.iter().position(|s| *s == grid::Show::Installed).unwrap()));
        m.mods_step(Press { down: true, ..default() }, &mut Vec::new());
        assert_eq!(m.sel, first);
        m.mods_step(Press { down: true, ..default() }, &mut Vec::new());
        assert_eq!(m.sel, first + 1);
        m.sel = first;
        assert_eq!(m.add.pack_at(m.sel).unwrap().name, "Zed HD");
        assert_eq!(m.add.pack_place(0), "On: 1st of 2 in use");
        assert_eq!(m.add.pack_place(1), "Off: not used");
        assert_eq!((packs::count(1), packs::count(320), packs::size(1500), packs::size(3 << 20)), ("1 texture".into(), "320 textures".into(), "2 KB".into(), "3.0 MB".into()));
        // the legend names the pack's keys
        let acts = super::super::legend::acts(&m, super::super::Dev::Keys);
        for w in ["Turn off", "Move up", "Move down", "Remove"] {
            assert!(acts.iter().any(|(_, l)| l == w), "{w}: {acts:?}");
        }
        // ✕ on the second (off) turns it on, saved
        let mut out = Vec::new();
        m.sel = first + 1;
        m.mods_step(ok(), &mut out);
        assert!(m.add.packs[1].on && out.contains(&Effect::SaveSettings), "{out:?}");
        assert!(m.add.status.contains("Alpha HD is on: 2nd of 3"), "{}", m.add.status);
        // △ moves it up, the cursor with it; △ again at the top stays
        out.clear();
        m.mods_step(Press { card: true, ..default() }, &mut out);
        assert_eq!(names(&m), ["lo", "hi", "ui"]);
        assert_eq!((m.sel, out.contains(&Effect::SaveSettings)), (first, true));
        out.clear();
        m.mods_step(Press { card: true, ..default() }, &mut out);
        assert_eq!((names(&m), out), (vec!["lo".to_string(), "hi".into(), "ui".into()], vec![Effect::Sound(1)]));
        // □ moves it down
        m.mods_step(Press { page: true, ..default() }, &mut Vec::new());
        assert_eq!((names(&m), m.sel), (vec!["hi".to_string(), "lo".into(), "ui".into()], first + 1));
        // the saved order is the list's
        let rows: Vec<_> = m.add.packs.iter().map(|r| (r.folder.clone(), r.on)).collect();
        assert_eq!(crate::mods_textures::order_of(&rows).order, ["hi", "lo", "ui"]);
        // taps: the third row's switch turns it off, its ▲ moves it up; any aspect, all on screen
        for aspect in [4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 9.0 / 16.0] {
            m.grid = super::super::Grid::fit(aspect);
            let hs = hits(&m).0;
            let slot = first + 2;
            let of = |k: Press| hs.iter().find(|h| h.act == Act::Row(slot, k)).unwrap_or_else(|| panic!("{aspect} {k:?}")).rect;
            let (share, vshare) = crate::widescreen::menu_shares(aspect, 1.0);
            for k in [OK, Press { card: true, ..default() }, Press { page: true, ..default() }, Press::default()] {
                let h = hs.iter().find(|h| h.act == Act::Row(slot, k)).unwrap();
                // on screen (in % of the window), as wide as tall in mockup px whatever the window
                let (x, w) = crate::widescreen::place(h.rect[0], h.rect[2], share, h.anchor);
                let (y, hh) = crate::widescreen::vplace_in(h.rect[1], h.rect[3], vshare, h.anchor);
                assert!(x >= 0.0 && y >= 0.0 && x + w <= 100.0 + 1e-3 && y + hh <= 100.0 + 1e-3, "{aspect} {k:?} {:?}", [x, y, w, hh]);
                assert!(of(k)[2] > 0.0);
            }
        }
        m.grid = super::super::Grid::fit(4.0 / 3.0);
        let hs = hits(&m).0;
        let slot = first + 2;
        let r = hs.iter().find(|h| h.act == Act::Row(slot, OK)).unwrap().rect;
        let a = act_at(&m, r[0] + r[2] / 2.0, r[1] + r[3] / 2.0).unwrap();
        assert_eq!(a, Act::Row(slot, OK));
        m.point(super::super::Dev::Keys, a, true);
        assert!(!m.add.packs[2].on);
        let r = hs.iter().find(|h| h.act == Act::Row(slot, Press { card: true, ..default() })).unwrap().rect;
        let a = act_at(&m, r[0] + r[2] / 2.0, r[1] + r[3] / 2.0).unwrap();
        m.point(super::super::Dev::Keys, a, true);
        assert_eq!((names(&m), m.sel), (vec!["hi".to_string(), "ui".into(), "lo".into()], first + 1));
        crate::mods_textures::set_order(was);
    }

    /// B372: the pictures asked for: every tile on view first, then one more screen the way the grid scrolled (nearest
    /// row first), at most `cap` fetching at once; one back (`got`) frees a turn; far-off loaded ones go past KEEP.
    #[test]
    fn pictures_on_view_then_ahead() {
        let mut m = Menu { screen: Screen::Mods, grid: super::super::Grid::fit(16.0 / 9.0), ..default() };
        let e = |i: usize| Entry { meta: Meta { id: format!("m{i:03}"), name: format!("M{i:03}"), sha256: format!("{i:064x}"), size: 1, ..Default::default() }, rev: "main".into(), verified: true };
        m.add.store = Some((0..200).map(e).collect());
        m.mods_page(Page::Browse, 0);
        m.add.b.sort = grid::Sort::Az;
        let l = grid::lay(&m.grid);
        let lines = grid::lines(&m.add.secs(), l.cols);
        let first = grid::plan(&lines, 0, l.fit(&lines, 0), false);
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
        assert_eq!(n, first.len());
        // a scroll down asks for the screen after; loaded ones far away go once past KEEP
        m.add.top = l.fit(&lines, 0);
        m.add.b.down = true;
        let go = grid::want(&mut m.add, &l, 1000);
        let next = grid::plan(&lines, m.add.top, l.fit(&lines, m.add.top), true);
        assert_eq!(go.len(), next.iter().filter(|i| !first.contains(i)).count());
        assert_eq!(go[0].meta.name, format!("M{:03}", first.len()));
        let on_view = first.len() / 2;
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
