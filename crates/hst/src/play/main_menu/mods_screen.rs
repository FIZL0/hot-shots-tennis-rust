//! Mods (M16, M17). Installed: Add mod picks a mod `.zip` (the browser build: the page's file picker, `web.rs`;
//! desktop: the system's, `modstore::pick`), which is checked whole and kept in `mods/<id>/` (OPFS in the browser,
//! beside the game on desktop; `modfs::install`); ✕ on an installed mod opens Remove / Cancel beside it (B202's child
//! buttons, `child.rs`, for every press that needs a second word).
//! Browse mods lists the mod store (`modstore.rs`: Verified = merged on the store's `main`, Installed / Update by the
//! zip's sha256); ✕ downloads, checks and installs one. Upload my mod signs in with a Hugging Face write token (the
//! token page opens, the player copies one, Paste token pastes it) and then, after the rule and Upload,
//! commits an installed mod to the store as a pull request (`modstore::publish`); Upload all (B250) queues every one, in
//! the background with a status beside each, and a re-upload goes over the player's earlier one. Its My public mods (M19) lists the
//! signed-in player's own uploads (`modstore::mine`); Take down takes one down (`modstore::remove`). Every page is a list on the left and
//! the hovered mod on the right, as the select's cards: its picture, △ card and details. The select's roster is
//! built when the game starts, so Reload brings changes into it. Mouse and touch: a tap on a row chooses it; on Add mod the page's own
//! file input lies over the row, so the tap that opens the picker is the browser's own (iOS opens a picker only
//! inside a real tap).

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::super::widescreen;
use super::child::{Child, Got};
use super::{BROWN, CARD_FILL, CELL_FILL, CHAR_NAMES, CREAM, Draw, Effect, GOLD, INK, Menu, Press, Screen, Tex, WHITE, attribute_card};
use crate::modstore::{self, Entry, Event, Up};
use crate::mods::Mod;

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

/// B255: Browse's child buttons on a store mod.
const INSTALL: &str = "Install";
const BY: &str = "By author";

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
    /// Installed mods.
    list: Vec<Mod>,
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
    /// The hovered mod's picture is in (`Shelf::shown`).
    shot: bool,
    /// B205: a mod was installed or removed since the game started (the note on the main menu's Mods row), and since
    /// the main menu last asked; the Reload / Later popup is up on the main menu (0 = Reload chosen).
    changed: bool,
    fresh: bool,
    pub(super) ask: Option<usize>,
    /// B255: Browse narrowed to one author's mods (whose list it is, and the row to go back to on the whole list).
    by: Option<By>,
    /// B255: the installs waiting (Download all, or a press while one runs), the one under way, and each one's tag
    /// by [`Entry::key`] (Queued, Downloading, Installed, Error) with what to say of it.
    queue: VecDeque<Entry>,
    doing: Option<Entry>,
    jobs: HashMap<String, (&'static str, String)>,
    /// B208: the web build on a touch device (no file picker, no sign-in): Add mod and Upload my mod are greyed, and
    /// choosing one puts up the "use the desktop app" popup (its row while up).
    phone: bool,
    nophone: Option<usize>,
    /// B335: the uploaders whose mods Browse hides under their heading (←, → shows them again).
    folded: HashSet<String>,
    /// B335: the installs since the queue was last empty: done, all (the status's "3/50").
    batch: (usize, usize),
}

/// A row under Browse's pills: an uploader's heading (who, how many of their mods) or one of their mods.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Item<'a> {
    Head(&'a str, usize),
    Mod(&'a Entry),
}

#[derive(Clone, Debug)]
struct By {
    author: String,
    list: Vec<Entry>,
    from: usize,
}

impl View {
    fn refresh(&mut self) {
        self.list = crate::mods::list(root());
        self.have = self.list.iter().filter_map(|m| modstore::stamp(&m.dir).map(|s| (format!("{}/{}", s.author, m.id), (s.sha256, m.dir.clone())))).collect();
        self.child = None;
    }

    fn changed(&mut self) {
        self.changed = true;
        self.fresh = true;
    }

    fn entries(&self) -> &[Entry] {
        match (&self.page, &self.by) {
            (Page::Mine, _) => self.mine.as_deref().unwrap_or_default(),
            (_, Some(b)) => &b.list,
            _ => self.store.as_deref().unwrap_or_default(),
        }
    }

    /// B255: Browse narrowed to `author`'s mods from the store's list (None: the whole list again, on row `sel`).
    fn narrow(&mut self, author: Option<String>, from: usize) {
        self.by = author.map(|a| {
            let list = self.store.iter().flatten().filter(|e| e.meta.author == a).cloned().collect();
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

    /// B255: queues `e`'s install, starting it if nothing else is under way.
    fn enqueue(&mut self, e: Entry, upscaled: bool) {
        if self.doing.is_none() && self.queue.is_empty() {
            self.batch = (0, 0);
        }
        self.batch.1 += 1;
        self.jobs.insert(e.key(), ("Queued", format!("{} is waiting its turn.", e.meta.name)));
        self.queue.push_back(e);
        self.next(upscaled);
    }

    /// B255: starts the next queued install when none is under way.
    fn next(&mut self, upscaled: bool) {
        if self.doing.is_some() {
            return;
        }
        let Some(e) = self.queue.pop_front() else { return };
        self.busy = true;
        self.status = format!("{}Downloading {} ({})...", self.count(), e.meta.name, mb(e.meta.size));
        self.jobs.insert(e.key(), ("Downloading", format!("Downloading {}...", e.meta.name)));
        modstore::install(&e, root(), upscaled);
        self.doing = Some(e);
    }

    /// B335: "3/50 done. " while a batch of installs (Download all) is under way.
    fn count(&self) -> String {
        if self.batch.1 > 1 { format!("{}/{} done. ", self.batch.0, self.batch.1) } else { String::new() }
    }

    /// B335: the uploader row `sel` folds or unfolds on Browse's whole list (its heading, or one of their mods), and
    /// whether they're folded.
    pub(super) fn fold_at(&self, sel: usize) -> Option<(String, bool)> {
        if self.page != Page::Browse || self.by.is_some() {
            return None;
        }
        let a = self.head(sel).map(|h| h.0).or_else(|| self.hovered(sel).map(|e| e.meta.author.as_str()))?;
        Some((a.to_string(), self.folded.contains(a)))
    }

    /// Whether ←→ move from row `sel` (between pills side by side), for the legend.
    pub(super) fn sideways(&self, sel: usize) -> bool {
        self.child.is_none() && self.nophone.is_none() && sel < self.acts() && self.acts() > 1
    }

    /// The page's pills above its list (the first rows): Installed, Add mod and Reload; Browse, Refresh (an author's
    /// list: Download all and All mods); Upload, the account and Upload all; Mine, Refresh.
    fn acts(&self) -> usize {
        match self.page {
            Page::Installed => TOP,
            Page::Upload => UP_TOP,
            Page::Browse if self.by.is_some() => 2,
            Page::Browse | Page::Mine => 1,
        }
    }

    /// The pills, then the mods (installed ones, or the store's with their uploaders' headings).
    fn rows(&self) -> usize {
        self.acts()
            + match self.page {
                Page::Installed | Page::Upload => self.list.len(),
                Page::Browse | Page::Mine => self.items().len(),
            }
    }

    /// The store's rows under the pills: on Browse's whole list, each uploader's heading then their mods (uploaders
    /// in the store's order, the unknown one last); an author's list and My uploads, just the mods.
    fn items(&self) -> Vec<Item<'_>> {
        let all = self.entries();
        if self.page != Page::Browse || self.by.is_some() {
            return all.iter().map(Item::Mod).collect();
        }
        let mut who: Vec<&str> = Vec::new();
        for e in all {
            if !who.contains(&e.meta.author.as_str()) {
                who.push(&e.meta.author);
            }
        }
        if let Some(i) = who.iter().position(|a| a.is_empty()) {
            let a = who.remove(i);
            who.push(a);
        }
        let mut out = Vec::with_capacity(all.len() + who.len());
        for a in who {
            let theirs: Vec<&Entry> = all.iter().filter(|e| e.meta.author == a).collect();
            out.push(Item::Head(a, theirs.len()));
            if !self.folded.contains(a) {
                out.extend(theirs.into_iter().map(Item::Mod));
            }
        }
        out
    }

    /// The uploader whose heading is row `sel` (empty: unknown), and how many mods they have.
    fn head(&self, sel: usize) -> Option<(&str, usize)> {
        match self.items().get(sel.checked_sub(self.acts())?) {
            Some(Item::Head(a, n)) => Some((*a, *n)),
            _ => None,
        }
    }

    /// The installed mod on row `sel`, if it's one.
    fn local(&self, sel: usize) -> Option<&Mod> {
        match self.page {
            Page::Installed | Page::Upload => self.list.get(sel.checked_sub(self.acts())?),
            Page::Browse | Page::Mine => None,
        }
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
        if !matches!(self.page, Page::Browse | Page::Mine) {
            return None;
        }
        match self.items().get(sel.checked_sub(self.acts())?) {
            Some(Item::Mod(e)) => Some(*e),
            _ => None,
        }
    }
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
        self.mods_page(Page::Browse, 0);
        self.go(Screen::Mods, out);
    }

    fn mods_page(&mut self, p: Page, sel: usize) {
        self.add.page = p;
        self.add.child = None;
        self.add.by = None;
        self.add.top = 0;
        self.sel = sel;
        self.add.status = match p {
            Page::Upload => RULE.into(),
            _ => String::new(),
        };
        if matches!(p, Page::Upload | Page::Mine) {
            self.add.account = modstore::account().map(|a| a.0);
        }
        if p == Page::Mine && self.add.account.is_none() {
            self.add.status = "Sign in on the Upload tab to see your uploads.".into();
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

    /// The tab `i` (`PAGES`), from Q/E, L1/R1 or a tap; on a phone Upload and My uploads put up the popup (no
    /// sign-in there).
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
        match &mut self.add.child {
            Some(c) => &mut c.at,
            None => &mut self.sel,
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
        let n = self.add.rows();
        self.sel = self.sel.min(n - 1);
        if let Some(mut c) = self.add.child.take() {
            match c.step(k, out) {
                Got::Open => self.add.child = Some(c),
                Got::Close => self.add.status = if self.add.page == Page::Upload { RULE.into() } else { String::new() },
                Got::Pick(i) => self.child_pick(c, i, out),
            }
        } else if k.prev || k.next {
            // the tabs, round; a phone skips the ones it can't use
            let at = PAGES.iter().position(|p| *p == self.add.page).unwrap_or(0);
            let tabs = if self.add.phone { 2 } else { PAGES.len() };
            let i = if k.next { (at + 1) % tabs } else { (at + tabs - 1) % tabs };
            self.mods_tab(i, out);
        } else {
            self.list_step(k, n, out);
        }
        // ○ went back to the main menu: its cursor isn't ours
        if self.screen != Screen::Mods {
            return;
        }
        let n = self.add.rows();
        self.sel = self.sel.min(n - 1);
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
            Page::Installed if s < TOP => self.remove_all(out),
            Page::Installed => self.remove(s - TOP, out),
            Page::Upload if s == 0 => {
                let v = &mut self.add;
                v.status = match (&v.account, i) {
                    (Some(_), _) => {
                        modstore::sign_out();
                        v.account = None;
                        v.mine = None;
                        "Signed out: the token is forgotten.".into()
                    }
                    (None, 0) => {
                        modstore::open(modstore::TOKENS);
                        // the child stays, on Paste token
                        v.child = Some(Child { at: 1, ..c });
                        if cfg!(target_arch = "wasm32") {
                            "On the page that opened, make a token (Write), copy it, paste it here (Ctrl+V), then Paste token.".into()
                        } else {
                            "On the page that opened, make a token (Write) and copy it, then Paste token.".into()
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
                let mods = if s == 1 { self.add.list.clone() } else { vec![self.add.list[s - UP_TOP].clone()] };
                match modstore::account() {
                    Some((name, token)) => {
                        self.queue(mods, name, token);
                        out.push(Sound(2));
                    }
                    None => {
                        self.add.account = None;
                        self.add.status = "Signed out: sign in again (Sign in, top left).".into();
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
                        self.add.status = "Signed out: sign in again on the Upload tab.".into();
                        out.push(Sound(1));
                    }
                }
            }
            Page::Browse => {
                let Some(e) = self.add.hovered(s).cloned() else { return };
                if c.opts[i] == BY {
                    self.add.narrow(Some(e.meta.author.clone()), s);
                    self.sel = self.add.acts();
                    self.add.top = 0;
                    self.add.status = format!("{} mods by {}.", self.add.entries().len(), e.meta.author);
                    out.push(Sound(2));
                } else {
                    self.install(e, out);
                }
            }
        }
    }

    /// B255: installs store mod `e` (queued behind any under way).
    fn install(&mut self, e: Entry, out: &mut Vec<Effect>) {
        let up = self.settings.0[super::UPSCALED_DOWNLOADS];
        let queued = self.add.doing.is_some();
        let name = e.meta.name.clone();
        self.add.enqueue(e, up);
        if queued {
            self.add.status = format!("{name} is queued: it installs after the ones before it.");
        }
        out.push(Effect::Sound(2));
    }

    /// B255: Download all on an author's list or on their heading: queues every one of `list` (`author`'s mods) not
    /// installed at the same or a newer version.
    fn download_all(&mut self, author: &str, list: Vec<Entry>, out: &mut Vec<Effect>) {
        let up = self.settings.0[super::UPSCALED_DOWNLOADS];
        let todo: Vec<Entry> = list.iter().filter(|e| !self.add.skips(e)).cloned().collect();
        let (n, skipped) = (todo.len(), list.len() - todo.len());
        for e in todo {
            self.add.enqueue(e, up);
        }
        let author = who(author);
        self.add.status = match (n, skipped) {
            (0, _) => format!("All of {author}'s mods are installed or queued already."),
            (n, 0) => format!("Downloading {n} mods by {author}, one after another."),
            (n, k) => format!("Downloading {n} mods by {author} ({k} installed or queued already)."),
        };
        out.push(Effect::Sound(if n == 0 { 1 } else { 2 }));
    }

    /// ○: an author's list goes back to the whole store's, on the mod it was opened from; else the main menu.
    fn mods_back(&mut self, out: &mut Vec<Effect>) {
        if let Some(b) = self.add.by.as_ref().filter(|_| self.add.page == Page::Browse) {
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
        // B335: ← folds an uploader's mods under their heading (from it or one of theirs), → shows them again
        if (k.left || k.right) && let Some((a, folded)) = self.add.fold_at(self.sel) {
            if k.left && !folded {
                self.add.folded.insert(a.clone());
                self.sel = self.add.acts() + self.add.items().iter().position(|it| matches!(it, Item::Head(h, _) if *h == a)).unwrap_or(0);
                out.push(Sound(0));
            } else if k.right && folded {
                self.add.folded.remove(&a);
                out.push(Sound(0));
            }
            return;
        }
        if !k.ok {
            return;
        }
        let s = self.sel;
        match self.add.page {
            Page::Installed => self.installed_step(s, out),
            Page::Browse => {
                if s == 1 && self.add.by.is_some() {
                    self.mods_back(out);
                } else if let Some(b) = self.add.by.clone().filter(|_| s == 0) {
                    self.download_all(&b.author, b.list, out);
                } else if s == 0 {
                    if !self.add.busy {
                        self.add.busy = true;
                        self.add.status = "Reading the mod store...".into();
                        modstore::list(true);
                    }
                    out.push(Sound(2));
                } else if let Some((a, _)) = self.add.head(s) {
                    // an uploader's heading: Download all of theirs
                    let a = a.to_string();
                    let list = self.add.entries().iter().filter(|e| e.meta.author == a).cloned().collect();
                    self.download_all(&a, list, out);
                } else if self.add.busy && self.add.doing.is_none() {
                    self.add.status = "Wait for the store to finish first.".into();
                } else if let Some(e) = self.add.hovered(s).cloned() {
                    let installed = self.add.state(&e) == Some("Installed");
                    if self.add.by.is_none() && !e.meta.author.is_empty() {
                        // B255: install it, or list its author's mods
                        let opts: &[&'static str] = if installed { &[BY] } else { &[INSTALL, BY] };
                        let what = if installed { format!("{} is installed already", e.meta.name) } else { format!("Install {}", e.meta.name) };
                        self.open_child(opts, format!("{what}. By author lists every mod {} shared.", e.meta.author), out);
                    } else if installed {
                        self.add.status = format!("{} is installed already.", e.meta.name);
                    } else {
                        self.install(e, out);
                    }
                }
            }
            Page::Upload => {
                if self.add.busy {
                    self.add.status = "Wait for the last one to finish.".into();
                } else if s == 0 {
                    self.account_step(out);
                } else if self.add.account.is_none() {
                    self.add.status = "Sign in first: Sign in, top left.".into();
                    out.push(Sound(1));
                } else if s == 1 && self.add.list.is_empty() {
                    self.add.status = "No mods installed to upload.".into();
                    out.push(Sound(1));
                } else if s == 1 {
                    self.open_child(&["Upload all"], format!("{RULE} Upload all {} mods?", self.add.list.len()), out);
                } else {
                    let m = &self.add.list[s - UP_TOP];
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
            1 => format!("Checking, packing and uploading {} (in the background)...", mods[0].name),
            n => format!("Uploading {n} mods one after another in the background: the menu stays yours."),
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
            Some(name) => self.open_child(&["Sign out"], format!("Signed in as {name}. Sign out (the game forgets the token)?"), out),
            None => self.open_child(
                &["Open token page", "Paste token"],
                "Open token page, make a token (Write) there and copy it, then Paste token.".into(),
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
            self.add.status = "Sign in on the Upload tab to see your uploads.".into();
            out.push(Sound(1));
        } else if s == 0 {
            self.read_mine();
            out.push(Sound(2));
        } else if let Some(e) = self.add.hovered(s).cloned() {
            if e.verified {
                self.open_child(&["Ask to delete"], format!("Ask for {} to be deleted from the store (a maintainer merges it)?", e.meta.name), out);
            } else {
                self.open_child(&["Take down"], format!("Take {} off the store (closes its pull request)?", e.meta.name), out);
            }
        }
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
            self.add.status = "Pick a mod .zip (a character from the mods repo).".into();
            out.push(Sound(2));
        } else if s == 1 {
            if let Err(e) = reload(out) {
                self.add.status = e;
            }
        } else if s == 2 {
            let n = self.add.list.iter().filter(|m| ours(m)).count();
            if n == 0 {
                self.add.status = "No mods the game installed to remove.".into();
                out.push(Sound(1));
            } else {
                self.open_child(&["Remove all"], format!("Remove all {n} mods the game installed (Reload then takes them out of the select)?"), out);
            }
        } else {
            let m = &self.add.list[s - TOP];
            let (name, dir) = (m.name.clone(), m.dir.clone());
            if ours(m) {
                self.open_child(&["Remove"], format!("Remove {name} (Reload then takes it out of the select)?"), out);
            } else {
                self.add.status = format!("{name} was added by hand: delete {} to remove it.", dir.display());
                out.push(Sound(1));
            }
        }
    }

    /// B335: Remove all: every installed mod the game put in ([`ours`]).
    fn remove_all(&mut self, out: &mut Vec<Effect>) {
        self.add.list.iter().filter(|m| ours(m)).for_each(delete);
        self.add.refresh();
        self.add.changed();
        self.add.status = "Removed every mod the game installed. Reload to take them out of the select.".into();
        out.push(Effect::Sound(1));
    }

    /// Removes installed mod `i` (its child's Remove).
    fn remove(&mut self, i: usize, out: &mut Vec<Effect>) {
        let name = self.add.list[i].name.clone();
        delete(&self.add.list[i]);
        self.add.refresh();
        self.add.changed();
        self.add.status = format!("Removed {name}. Reload to take it out of the select.");
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
    /// Reload and Later, ✕ chooses, ○ is Later. Returns whether the popup was up.
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
        } else if k.ok || k.back {
            self.add.ask = None;
            out.push(Effect::Sound(1));
        }
        true
    }
}

/// B205: mods changed since the start (the main menu's note on its Mods row) and the popup isn't up.
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
    // B221: a tap off the popup is Later
    d.popup(Some(super::pointer::Act::Row(1, super::pointer::OK)));
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
    d.said = "Mods aren't loaded while the game runs: Reload restarts it with them; Later keeps playing as it is.".into();
}

/// The list box and the side panel on the view `g` fits, from under the title band down to the legend strip: the
/// panel right of the list (under it in a portrait window), as Settings' pages (`options::boxes`).
fn boxes(g: &super::Grid) -> ([f32; 4], [f32; 4]) {
    let [l, y, w, _] = g.strip;
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

/// Installed mod `md`'s face in the select's roster (built at startup: one added since has none yet).
fn face_of(m: &Menu, dir: &Path) -> Option<(Tex, [f32; 4])> {
    mod_of(m, dir).map(|j| super::cell_face(Some(super::Who::Mod(j))))
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
            None => modstore::stamp(&md.dir).map(|_| vec![("Store", GREY)]).unwrap_or_default(),
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

/// An uploader's heading over their mods: "BY <name> · N mods" with a thin line to its "Download all N" chip (gold
/// while chosen: a press downloads them all).
/// (B335: "+" before it while their mods are folded away.)
fn heading(d: &mut Draw, [x, y, w, h]: [f32; 4], author: &str, n: usize, on: bool, folded: bool) {
    use super::options::MUTED;
    let chip = format!("Download all {n}");
    let cw = d.width(&chip, 12.0) + 20.0;
    let cx = x + w - cw - 6.0;
    let ty = y + h - 22.0;
    let title = format!("{}BY {} - {n} mod{}", if folded { "+ " } else { "" }, who(author).to_uppercase(), if n == 1 { "" } else { "s" });
    let size = 14.0 * ((cx - x - 30.0) / d.width(&title, 14.0).max(1.0)).min(1.0);
    super::left(d, &title, x + 4.0, ty + (14.0 - size) / 2.0 + 2.0, size, cx - x - 30.0, if on { BROWN } else { MUTED });
    let lx = x + 12.0 + d.width(&title, size);
    if cx - 8.0 > lx {
        d.box3([lx, ty + 9.0, cx - 8.0 - lx, 1.5], 0.75, [None, None, Some(if on { BROWN } else { [100.0, 92.0, 90.0] })], 128.0);
    }
    let fill = if on { [128.0, 110.0, 40.0] } else { [56.0, 56.0, 67.0] };
    d.pill_at([cx, ty, cw, 20.0], [INK, if on { WHITE } else { [100.0; 3] }, fill], 128.0);
    d.label(&chip, cx + cw / 2.0, ty + 4.0, 12.0, cw - 8.0, WHITE);
}

pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let v = &m.add;
    d.frame(5, 6);
    let tabs = ["Browse".into(), format!("Installed {}", v.list.len()), "Upload".into(), "My uploads".into()];
    let words: Vec<&str> = tabs.iter().map(String::as_str).collect();
    super::options::tab_row(d, m, 5, &words, PAGES.iter().position(|p| *p == v.page));
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
        } else if let Some((a, n)) = v.head(r) {
            heading(d, rect, a, n, on, v.folded.contains(a));
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
    if mods == 0 {
        let none = match v.page {
            Page::Installed | Page::Upload => "No mods installed yet: Add mod, or the Browse tab.",
            Page::Browse if v.busy => "Reading the mod store...",
            Page::Browse => "Nobody has shared a mod yet.",
            Page::Mine if v.account.is_none() => "Sign in on the Upload tab to see your uploads.",
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
    if !v.status.is_empty() {
        return v.status.clone();
    }
    match (v.page, m.sel) {
        (Page::Installed, 0) if v.phone => "Not available in the browser on phones: use the desktop app.".into(),
        (Page::Installed, 0) => "Add a custom character from a mod .zip (its mod.json and models).".into(),
        (Page::Installed, 1) => "Reload the game: added mods join the select, removed ones leave it.".into(),
        (Page::Installed, s) => v.local(s).map_or(String::new(), |md| format!("{} ({}, {} of {}): press to remove it.", md.name, md.folder(), s - TOP + 1, v.list.len())),
        (Page::Browse, 0) if let Some(b) = &v.by => format!("Install every mod by {} you don't have yet.", b.author),
        (Page::Browse, 0) => "Read the store's list again.".into(),
        (Page::Browse, 1) if v.by.is_some() => "Back to every mod in the store.".into(),
        (Page::Browse, s) if let Some((a, n)) = v.head(s) => format!("{n} mod{} by {}: press to download every one you don't have yet.", if n == 1 { "" } else { "s" }, who(a)),
        (Page::Browse, s) => match v.hovered(s) {
            Some(e) if v.jobs.contains_key(&e.key()) => v.jobs[&e.key()].1.clone(),
            Some(e) => match v.state(e) {
                Some("Installed") => format!("{} is installed.", e.meta.name),
                Some(_) => format!("Press to update {}.", e.meta.name),
                _ => format!("Press to install {}.", e.meta.name),
            },
            None => String::new(),
        },
        (Page::Upload, 0) => match &v.account {
            Some(name) => format!("Signed in to Hugging Face as {name}."),
            None => "Sign in to Hugging Face with a token, once: the game then uploads by itself.".into(),
        },
        (Page::Upload, 1) => "Upload every installed mod, in the background (ones already on the store as they are are skipped).".into(),
        (Page::Upload, _) => RULE.into(),
        (Page::Mine, 0) => "Read your uploads from the store again.".into(),
        (Page::Mine, s) => match v.hovered(s) {
            Some(e) if e.verified => format!("{} is on the store (merged): press to ask for it to be deleted.", e.meta.name),
            Some(e) => format!("{} is live from your pull request: press to take it down.", e.meta.name),
            None => String::new(),
        },
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
        chips = facts(&md.source, Some(md.hand), Some(md.costumes.len()), Some(md.donor));
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
        let pic = [x + 12.0, ty, (ph * 0.8).min(w - 174.0).max(60.0), ph];
        d.box3(pic, 8.0, [None, None, Some([122.0, 108.0, 115.0])], 128.0);
        if v.shot {
            d.q(Tex::Shelf, [0.0, 0.0, 1.0, 1.0], pic, WHITE, 128.0);
        } else {
            d.centred("No picture", pic[0] + pic[2] / 2.0, pic[1] + ph / 2.0 - 7.0, 13.0, super::options::MUTED);
        }
        let ax = pic[0] + pic[2] + 6.0;
        let card = !wide || w >= 560.0;
        match stats.filter(|_| card) {
            Some((style, grades)) if ph >= 150.0 => attribute_card(d, 2, style, &grades, ax, ty - 4.0),
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
    if let Some((a, n)) = v.head(m.sel) {
        // an uploader's heading
        name = format!("By {}", who(a));
        meta = format!("{n} mod{} in the store", if n == 1 { "" } else { "s" });
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
        Up::Failed(_) => ("Error", RED),
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
/// shown, someone's folded; with `HST_CHILD=1` narrowed to FIZL0's, B255's Download all under way) / `mymods` (My public mods, the same made-up entries, one's Take down open) / `uploadmods` (Upload, Upload
/// all open, the first mods tagged with each upload state; `HST_CHILD=0`: none open, the first mod hovered) / `signinmods` (Upload signed out, Sign in's buttons open) / `addmods` with `HST_CHILD=1` (Installed, a
/// mod's Remove open) / `reloadmods` (the main menu, B205's popup up): for `--shot`s.
pub(super) fn sample(m: &mut Menu, screen: &str) {
    let v = &mut m.add;
    v.refresh();
    if screen == "browsemods" || screen == "mymods" {
        v.page = Page::Browse;
        v.store = Some(
            v.list.iter().take(9).enumerate().map(|(i, md)| {
                let meta = modstore::Meta {
                    id: md.id.clone(),
                    name: md.name.clone(),
                    source: md.source.clone(),
                    author: ["FIZL0", "someone", ""][i % 3].into(),
                    size: (i as u64 + 1) * 7 << 20,
                    sha256: format!("{:064}", i % 2),
                    style: (i % 3 != 2).then_some((i % 4) as u8),
                    grades: (i % 3 != 2).then_some([5, 4, 3, 2, 1].map(|g: u8| 1 + (g + i as u8) % 5)),
                    hand: Some(if md.hand < 0.0 { -1 } else { 1 }),
                    costumes: Some(md.costumes.len() as u32),
                    like: Some(md.donor as u8),
                    ..Default::default()
                };
                Entry { meta, rev: if i % 3 == 0 { "main".into() } else { format!("refs/pr/{i}") }, verified: i % 3 == 0 }
            }).collect(),
        );
        v.have = v.store.iter().flatten().zip(&v.list).take(4).map(|(e, md)| (e.key(), (format!("{:064}", 0), md.dir.clone()))).collect();
        m.sel = 2;
        // B335: someone's mods folded under their heading
        v.folded.insert("someone".into());
        if screen == "browsemods" && std::env::var("HST_CHILD").is_ok() {
            // B255: FIZL0's mods, Download all under way: one each of Installed, Downloading, Queued, Error
            v.narrow(Some("FIZL0".into()), 0);
            let tags = [("Installed", "Installed {}."), ("Downloading", "Downloading {}..."), ("Queued", "{} is waiting its turn."), ("Error", "{} not installed: the download doesn't match its sha256")];
            v.jobs = v.entries().iter().zip(tags).map(|(e, (t, s))| (e.key(), (t, s.replace("{}", &e.meta.name)))).collect();
            m.sel = 0;
            v.batch = (1, 3);
            v.status = format!("{}Downloading {}...", v.count(), v.entries()[1].meta.name);
        }
        if screen == "mymods" {
            (v.page, v.mine, v.account, v.child) = (Page::Mine, v.store.take(), Some("FIZL0".into()), Some(Child::new(2, &["Take down"])));
            v.status = format!("Take {} off the store (closes its pull request)?", v.mine.as_ref().and_then(|l| l.get(1)).map_or("", |e| &e.meta.name));
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
        v.status = "Open token page, make a token (Write) there and copy it, then Paste token.".into();
        m.sel = 0;
    } else if screen == "addmods" && std::env::var("HST_CHILD").is_ok() && !v.list.is_empty() {
        v.child = Some(Child::new(TOP, &["Remove"]));
        v.status = format!("Remove {} (Reload then takes it out of the select)?", v.list[0].name);
        m.sel = TOP;
    } else if screen.starts_with("phonemods") {
        v.phone = true;
        m.sel = 0;
        v.nophone = (screen == "phonemods-ask").then_some(0);
    }
}

/// The Add mod row's rectangle in % of the window (left, top, width, height), while it's on screen: where the page
/// puts its file input.
fn add_rect(m: &Menu, aspect: f32, k: f32) -> Option<[f32; 4]> {
    let (share, v) = widescreen::menu_shares(aspect, k);
    let g = super::Grid::fit(aspect);
    let [x, y, w, h] = row_rect(&m.add, boxes(&g).0, shown(&g), 0)?;
    (m.screen == Screen::Mods && m.add.page == Page::Installed && !m.add.phone && m.add.nophone.is_none())
        .then(|| [x * share / 6.4 + 50.0 * (1.0 - share), y * v / 4.48 + 50.0 * (1.0 - v), w * share / 6.4, h * v / 4.48])
}

/// The store's previews (by mod id; None: it has none, or it's on its way), and the hovered one's (`Tex::Shelf`).
#[derive(Resource, Default)]
pub(super) struct Shelf {
    shots: HashMap<String, Option<Handle<Image>>>,
    pub shown: Option<Handle<Image>>,
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
        // (a `sample` screen opens on its own page, already read, or with its child open)
        if *on && menu.add.page == Page::Installed && menu.add.child.is_none() {
            menu.add.refresh();
        }
        // B208: a touch device stays one (`web/touch.js` never forgets a touch)
        menu.add.phone |= *on && super::touch_page();
        // the select's cards for the mods it has (its roster, built at startup)
        if *on && let Some(art) = &art {
            let stats = menu.mods.iter().zip(&art.2.mods).map(|(m, s)| (m.dir.clone(), *s)).collect();
            menu.add.stats = stats;
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
                menu.add.status = "No token (hf_...) was copied: copy it, then Paste token.".into();
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
    let up = menu.settings.0[super::UPSCALED_DOWNLOADS];
    for e in modstore::take() {
        let v = &mut menu.add;
        match e {
            Event::Listing(r) => {
                v.busy = false;
                match r {
                    Ok(list) => {
                        v.status = format!("{} mods in the store.", list.len());
                        v.store = Some(list);
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
            Event::Installed(r) => {
                v.busy = false;
                // B255: the queued install's tag, then the next one
                if let Some(e) = v.doing.take() {
                    v.batch.0 += 1;
                    let job = match &r {
                        Ok(_) => ("Installed", format!("Installed {}. Restart the game to play it.", e.meta.name)),
                        Err(err) => ("Error", format!("{} not installed: {err}", e.meta.name)),
                    };
                    v.jobs.insert(e.key(), job);
                }
                v.refresh();
                if r.is_ok() {
                    v.changed();
                    // B259: an update's new picture, not the old one read before it
                    shelf.shots.retain(|k, _| !k.starts_with("local/"));
                }
                v.status = match r {
                    // the browser's page says when it's saved
                    Ok(_) if cfg!(target_arch = "wasm32") => v.status.clone(),
                    Ok(name) => format!("Installed {name}. Restart the game to play it."),
                    Err(e) => format!("Not installed: {e}"),
                };
                v.next(up);
            }
            Event::Progress(s) => v.status = format!("{}{s}", v.count()),
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
                v.status = r.unwrap_or_else(|e| format!("Not removed: {e}"));
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
            Event::Preview(id, png) => {
                let img = png.and_then(|b| {
                    Image::from_buffer(&b, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), RenderAssetUsages::RENDER_WORLD).ok()
                });
                shelf.shots.insert(id, img.map(|i| images.add(i)));
            }
        }
    }
    let on = menu.screen == Screen::Mods;
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
    let local = menu.add.hovered(menu.sel).and_then(|e| menu.add.installed(e)).and_then(|(_, dir)| menu.add.list.iter().find(|m| m.dir == *dir));
    let local = local.or_else(|| menu.add.local(menu.sel)).filter(|_| on);
    card.want = None;
    shelf.shown = if let Some(m) = local {
        // an installed mod's: the select's picture of it, read once; none yet: drawn (B229)
        let key = format!("local/{}", m.dir.display());
        let h = match shelf.shots.get(&key) {
            Some(h) => h.clone(),
            None => {
                let h = super::previews::card(&menu.add.iso, m).and_then(png).map(|i| images.add(i));
                shelf.shots.insert(key, h.clone());
                h
            }
        };
        if h.is_none() {
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

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Shelf>().add_systems(Update, page.before(super::draw));
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

    /// The page keeps its cursor within its own rows only: on Graphics, row 7 stays row 7 (it snapped back to 4).
    #[test]
    fn other_screens_keep_their_cursor() {
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

    /// The rows: Add mod, Reload, each installed mod; ✕ on a mod opens Remove / Cancel in the side panel (B202): ○ and Cancel close it, ↑↓ wrap in it, Remove removes the mod (from memory, and nothing else).
    #[test]
    fn removes_from_the_child() {
        let mut m = Menu::default();
        crate::modfs::add(&root().join("t_screen/mod.json"), Vec::new().into());
        m.add.list = vec![a_mod("t_screen", "Screen"), a_mod("b", "B")];
        m.screen = Screen::Mods;
        m.sel = TOP;
        let mut out = Vec::new();
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.child, Some(Child { row: TOP, at: 0, opts: vec!["Remove", "Cancel"] }));
        assert!(m.add.status.starts_with("Remove Screen"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.add.child.is_none(), m.screen, m.sel), (true, Screen::Mods, TOP));
        m.mods_step(ok(), &mut out);
        m.mods_step(Press { up: true, ..default() }, &mut out);
        assert_eq!((m.sel, m.add.child.as_ref().map(|c| c.at)), (TOP, Some(1)));
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.is_none() && crate::modfs::is_file(&root().join("t_screen/mod.json")));
        m.mods_step(ok(), &mut out);
        // the pointer: only the child's buttons, a tap elsewhere is Back
        let at = |m: &Menu, x: f32, y: f32| super::super::pointer::tests::act_at(m, x, y);
        let r = child_rects(m.add.child.as_ref().unwrap(), boxes(&m.grid).1);
        let p = rect(&m, 0);
        assert_eq!(at(&m, p[0] + 10.0, p[1] + 5.0), None);
        assert_eq!(at(&m, r[1][0] + 10.0, r[1][1] + 5.0), Some(super::super::pointer::Act::Row(1, super::super::pointer::OK)));
        assert_eq!(super::super::pointer::tests::hits(&m).1, Some(super::super::pointer::press(|k| k.back = true)));
        m.point(super::super::Dev::Keys, super::super::pointer::Act::Row(1, super::super::pointer::OK), false);
        assert_eq!((m.sel, m.add.child.as_ref().map(|c| c.at)), (TOP, Some(1)));
        m.point(super::super::Dev::Keys, super::super::pointer::Act::Row(0, super::super::pointer::OK), true);
        assert!(!crate::modfs::is_file(&root().join("t_screen/mod.json")));
        assert!(m.add.status.starts_with("Removed Screen"));
        m.sel = 0;
        m.mods_step(Press { up: true, ..default() }, &mut out);
        assert_eq!(m.sel, m.add.rows() - 1);
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.screen, m.sel), (Screen::Main, m.main_at(super::super::home::Row::Mods)));
        // B205: back on the main menu the popup asks (Reload chosen) and takes the presses; Later closes it, the note stays
        assert_eq!(m.add.ask, Some(0));
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
    }

    /// A mod in a folder put in by hand (here a pack's) isn't removed: the press says where to delete it.
    #[test]
    fn hand_added_mods_stay() {
        let mut m = Menu { screen: Screen::Mods, sel: TOP, ..default() };
        m.add.list = vec![Mod { dir: root().join("pack/p"), ..a_mod("p", "P") }];
        let mut out = Vec::new();
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.is_none() && m.add.status.contains("added by hand"), "{}", m.add.status);
        // nor by Remove all
        m.sel = 2;
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.is_none() && m.add.status.starts_with("No mods"), "{}", m.add.status);
    }

    /// B335: Remove all (Installed's third pill) asks, then removes every mod the game installed.
    #[test]
    fn remove_all_mods() {
        let mut m = Menu { screen: Screen::Mods, sel: 2, ..default() };
        for id in ["t_all1", "t_all2"] {
            crate::modfs::add(&root().join(id).join("mod.json"), Vec::new().into());
        }
        m.add.list = vec![a_mod("t_all1", "A"), a_mod("t_all2", "B")];
        let mut out = Vec::new();
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.child.as_ref().map(|c| c.opts.clone()), Some(vec!["Remove all", "Cancel"]));
        m.mods_step(ok(), &mut out);
        assert!(["t_all1", "t_all2"].iter().all(|id| !crate::modfs::is_file(&root().join(id).join("mod.json"))));
        assert!(m.add.status.starts_with("Removed every mod") && m.add.changed, "{}", m.add.status);
    }

    /// Browse groups the store's mods under their uploaders' headings (the store's order, the unknown one last); a
    /// heading's press queues all of that uploader's mods. ←→ move between pills side by side, ↓ into the list, ↑
    /// back out.
    #[test]
    fn headings_and_side_by_side() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let e = |id: &str, who: &str| Entry {
            meta: Meta { id: id.into(), name: id.into(), author: who.into(), sha256: id.repeat(32), size: 1, ..Default::default() },
            rev: "main".into(),
            verified: true,
        };
        m.add.store = Some(vec![e("x1", ""), e("b1", "bob"), e("a1", "ann"), e("b2", "bob")]);
        m.mods_page(Page::Browse, 0);
        let rows: Vec<_> = m.add.items().iter().map(|i| match i {
            Item::Head(a, n) => format!("{a}:{n}"),
            Item::Mod(e) => e.meta.id.clone(),
        }).collect();
        assert_eq!(rows, ["bob:2", "b1", "b2", "ann:1", "a1", ":1", "x1"]);
        assert_eq!((m.add.rows(), m.add.head(4), m.add.hovered(5).map(|x| x.meta.id.as_str())), (8, Some(("ann", 1)), Some("a1")));
        // (something under way: the heading only queues)
        m.add.doing = Some(e("zz", "zed"));
        let mut out = Vec::new();
        m.sel = 1;
        m.mods_step(ok(), &mut out);
        let ids: Vec<_> = m.add.queue.iter().map(|x| x.meta.id.clone()).collect();
        assert_eq!(ids, ["b1", "b2"]);
        assert!(m.add.status.starts_with("Downloading 2 mods by bob"), "{}", m.add.status);
        m.sel = 6;
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.contains("by unknown uploader"), "{}", m.add.status);
        // B335: the batch's count; ← on one of bob's mods folds them under his heading, → shows them again
        assert_eq!(m.add.count(), "0/3 done. ");
        let (l, r) = (Press { left: true, ..default() }, Press { right: true, ..default() });
        m.sel = 3;
        m.mods_step(l, &mut out);
        assert_eq!((m.sel, m.add.rows(), m.add.fold_at(1)), (1, 6, Some(("bob".into(), true))));
        m.mods_step(r, &mut out);
        assert_eq!((m.sel, m.add.rows()), (1, 8));
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

    /// A long list scrolls to keep the cursor on screen, and taps land on the drawn rows.
    #[test]
    fn scrolls_and_taps() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.list = (0..20).map(|i| a_mod(&format!("m{i}"), &format!("M{i}"))).collect();
        let mut out = Vec::new();
        for _ in 0..9 {
            m.mods_step(Press { down: true, ..default() }, &mut out);
        }
        // the cursor's mod (the 9th) is the last card shown
        let n = shown(&m.grid);
        assert!(n < 9);
        assert_eq!((m.sel, m.add.top), (TOP + 8, 9 - n));
        let at = |x: f32, y: f32| super::super::pointer::tests::act_at(&m, x, y);
        let r = rect(&m, TOP + 8);
        assert_eq!(at(r[0] + 10.0, r[1] + 5.0), Some(super::super::pointer::Act::Row(TOP + 8, super::super::pointer::OK)));
        assert!(row_rect(&m.add, boxes(&m.grid).0, n, TOP + 9).is_none());
        // the gap between two cards goes to the nearer one
        let first = rect(&m, TOP + m.add.top);
        assert_eq!(at(first[0] + 10.0, first[1] + CARD_H + 1.0), Some(super::super::pointer::Act::Row(TOP + m.add.top, super::super::pointer::OK)));
        (0..9).for_each(|_| m.mods_step(Press { up: true, ..default() }, &mut out));
        assert_eq!((m.sel, m.add.top), (0, 0));
        // 4:3 fills the window: Add mod's own place
        let (r, a) = (add_rect(&m, 4.0 / 3.0, 1.0).unwrap(), rect(&m, 0));
        assert!((r[0] - a[0] / 6.4).abs() < 1e-4 && (r[2] - a[2] / 6.4).abs() < 1e-4);
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
        // Browse first (Refresh, then the entries)
        assert_eq!((m.add.page, m.sel, m.add.rows()), (Page::Browse, 0, 5));
        let tags: Vec<_> = m.add.entries().iter().map(|x| m.add.state(x)).collect();
        assert_eq!(tags, [Some("Installed"), Some("Update"), None]);
        m.sel = 2;
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.contains("installed already") && !m.add.busy);
        let r = rect(&m, 2);
        assert_eq!(super::super::pointer::tests::act_at(&m, r[0] + 10.0, r[1] + 5.0), Some(super::super::pointer::Act::Row(2, super::super::pointer::OK)));
        // the tab row's Upload tab
        m.point(super::super::Dev::Keys, super::super::pointer::Act::Tab(2), true);
        assert_eq!(m.add.page, Page::Upload);
        assert_eq!(m.add.status, RULE);
        // the account row first, then the mods; uploading needs an account
        m.add.account = None;
        m.sel = UP_TOP;
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.starts_with("Sign in first") && m.add.child.is_none());
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
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.screen, Screen::Main);
    }

    /// B250: Upload all opens Upload all / Cancel; it (and a mod's own Upload) queues the mods in the background
    /// without making the page busy, each tagged; one on its way isn't queued again; the outcomes come back per mod.
    #[test]
    fn upload_all() {
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
        m.mods_step(Press { next: true, ..default() }, &mut out);
        assert_eq!(m.add.page, Page::Mine);
        m.mods_step(Press { prev: true, ..default() }, &mut out);
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

    /// B208: on a phone Add mod is greyed and it and the Upload tab put up the popup instead (no picker, no page); ✕,
    /// ○ or a tap anywhere closes it; Q/E skip Upload and My uploads; Browse still opens; the file input stays off.
    #[test]
    fn phone_rows_greyed() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.phone = true;
        let mut out = Vec::new();
        for (row, close) in [(0, ok()), (2, Press { back: true, ..default() })] {
            m.sel = 0;
            if row == 0 {
                m.mods_step(ok(), &mut out);
            } else {
                m.mods_tab(2, &mut out);
            }
            assert_eq!((m.add.nophone, m.add.page, m.add.busy), (Some(row), Page::Installed, false));
            // the popup takes the moves
            m.mods_step(Press { down: true, ..default() }, &mut out);
            assert_eq!((m.sel, m.add.nophone), (0, Some(row)));
            m.mods_step(close, &mut out);
            assert_eq!((m.screen, m.add.nophone), (Screen::Mods, None));
        }
        assert!(add_rect(&m, 4.0 / 3.0, 1.0).is_none());
        use super::super::pointer::{Act, OK, tests::act_at};
        m.add.nophone = Some(1);
        assert_eq!(act_at(&m, 320.0, ASK_TOP), Some(Act::Press(OK)));
        assert_eq!(act_at(&m, 10.0, 300.0), None);
        m.add.nophone = None;
        m.add.store = Some(Vec::new());
        m.mods_step(Press { prev: true, ..default() }, &mut out);
        assert_eq!(m.add.page, Page::Browse);
        m.mods_step(Press { next: true, ..default() }, &mut out);
        assert_eq!((m.add.page, m.add.nophone), (Page::Installed, None));
    }

    /// M19: My public mods needs an account; its entries open Take down (a PR's, it closes) or Ask to delete (a merged
    /// one's) under the Refresh pill; Back returns to the main menu.
    #[test]
    fn my_public_mods() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let mut out = Vec::new();
        let e = |id: &str, rev: &str| Entry { meta: Meta { id: id.into(), name: id.into(), ..Default::default() }, rev: rev.into(), verified: rev == "main" };
        m.add.mine = Some(vec![e("a", "refs/pr/4"), e("b", "main")]);
        m.mods_tab(3, &mut out);
        m.add.account = Some("me".into());
        assert_eq!((m.add.page, m.sel, m.add.rows(), m.add.busy), (Page::Mine, 0, 3, false));
        m.mods_step(Press { down: true, ..default() }, &mut out);
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.as_ref().is_some_and(|c| c.row == 1 && c.opts[0] == "Take down") && m.add.status.contains("closes its pull request"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        m.mods_step(Press { down: true, ..default() }, &mut out);
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.as_ref().is_some_and(|c| c.row == 2 && c.opts[0] == "Ask to delete") && m.add.status.contains("maintainer"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.add.hovered(m.sel).map(|x| x.meta.id.as_str()), Some("b"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.screen, Screen::Main);
    }

    /// B255: a store mod's press opens Install / By author; By author lists only that author's mods with Download all,
    /// which queues each one not installed at the same or a newer version (tagged Queued) and leaves the rest; Back
    /// returns to the whole list on the mod it came from.
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
        m.mods_page(Page::Browse, 5);
        let mut out = Vec::new();
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.child.as_ref().map(|c| c.opts.clone()), Some(vec![INSTALL, BY, "Cancel"]));
        m.mods_step(Press { down: true, ..default() }, &mut out);
        m.mods_step(ok(), &mut out);
        let ids = |l: &[Entry]| l.iter().map(|x| x.meta.id.clone()).collect::<Vec<_>>();
        assert_eq!((ids(m.add.entries()), m.sel, m.add.rows()), (vec!["a1".to_string(), "a2".into(), "a3".into(), "a4".into()], 2, 6));
        assert_eq!(m.add.status, "4 mods by ann.");
        // an installed one's press offers By author only; in the author's list it says so
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.contains("installed already") && m.add.child.is_none());
        // (something under way: Download all only queues, nothing is fetched here)
        let busy = e("zz", "zed", "1");
        m.add.doing = Some(busy.clone());
        // Download all, the first pill
        m.sel = 0;
        m.mods_step(ok(), &mut out);
        assert_eq!(ids(&Vec::from(m.add.queue.clone())), ["a3"]);
        assert_eq!(m.add.jobs.get("ann/a3").map(|j| j.0), Some("Queued"));
        assert!(m.add.status.starts_with("Downloading 1 mods by ann (3 installed"), "{}", m.add.status);
        // again: nothing left to queue
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.starts_with("All of ann's mods") && m.add.queue.len() == 1);
        // a press on a mod outside Download all queues it too
        m.add.have.remove("ann/a1");
        m.sel = 2;
        m.mods_step(ok(), &mut out);
        assert_eq!(ids(&Vec::from(m.add.queue.clone())), ["a3", "a1"]);
        assert!(m.add.status.contains("is queued"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.add.page, m.sel, m.add.entries().len()), (Page::Browse, 5, 5));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.screen, Screen::Main);
        assert!(modstore::newer("1.10", "1.9") && !modstore::newer("1.0", "1") && !modstore::newer("", "1") && modstore::newer("2", ""));
    }
}
