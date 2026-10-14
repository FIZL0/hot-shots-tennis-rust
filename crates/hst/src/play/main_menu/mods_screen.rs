//! Mods (M16, M17). Installed: Add mod picks a mod `.zip` (the browser build: the page's file picker, `web.rs`;
//! desktop: the system's, `modstore::pick`), which is checked whole and kept in `mods/<id>/` (OPFS in the browser,
//! beside the game on desktop; `modfs::install`); ✕ on an installed mod opens Remove / Cancel beside it (B202's child
//! buttons, `child.rs`, for every press that needs a second word).
//! Browse mods lists the mod store (`modstore.rs`: Verified = merged on the store's `main`, Installed / Update by the
//! zip's sha256); ✕ downloads, checks and installs one. Upload my mod signs in with a Hugging Face write token (the
//! token page opens, the player copies one, Paste token pastes it) and then, after the rule and Upload,
//! commits an installed mod to the store as a pull request (`modstore::publish`). Its My public mods (M19) lists the
//! signed-in player's own uploads (`modstore::mine`); Take down takes one down (`modstore::remove`). Every page is a list on the left and
//! the hovered mod on the right, as the select's cards: its picture, △ card and details. The select's roster is
//! built when the game starts, so Reload brings changes into it. Mouse and touch: a tap on a row chooses it; on Add mod the page's own
//! file input lies over the row, so the tap that opens the picker is the browser's own (iOS opens a picker only
//! inside a real tap).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::super::widescreen;
use super::child::{self, Child, Got};
use super::{CARD_FILL, CHAR_NAMES, Draw, Effect, GOLD, INK, Menu, Press, Screen, Tex, WHITE, attribute_card};
use crate::modstore::{self, Entry, Event};
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
/// Rows drawn at once (the rest scroll), and where: the left half, Browse's tags beside them.
const SHOWN: usize = 7;
const X: f32 = 70.0;
const Y: f32 = 74.0;
const W: f32 = 180.0;
/// The hovered mod's panel on the right half (its picture, △ card, details).
const PANEL: [f32; 4] = [326.0, 60.0, 300.0, 336.0];
const PITCH: f32 = 44.0;
const H: f32 = 38.0;
/// The rows above the installed mods on the main page.
const TOP: usize = 3;

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
    /// B208: the web build on a touch device (no file picker, no sign-in): Add mod and Upload my mod are greyed, and
    /// choosing one puts up the "use the desktop app" popup (its row while up).
    phone: bool,
    nophone: Option<usize>,
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
        if self.page == Page::Mine { &self.mine } else { &self.store }.as_deref().unwrap_or_default()
    }

    /// Installed: Browse mods, Add mod, Upload my mod, the mods, Reload, Back. Browse: the store's mods, Refresh,
    /// Back. Upload: the account, My public mods, the mods, Back. Mine: the player's uploads, Refresh, Back.
    fn rows(&self) -> usize {
        match self.page {
            Page::Installed => TOP + self.list.len() + 2,
            Page::Browse | Page::Mine => self.entries().len() + 2,
            Page::Upload => self.list.len() + 3,
        }
    }

    /// The installed mod on row `sel`, if it's one.
    fn local(&self, sel: usize) -> Option<&Mod> {
        match self.page {
            Page::Installed => self.list.get(sel.checked_sub(TOP)?),
            Page::Upload => self.list.get(sel.checked_sub(2)?),
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
        matches!(self.page, Page::Browse | Page::Mine).then(|| self.entries().get(sel)).flatten()
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
        self.add.page = Page::Installed;
        self.go(Screen::Mods, out);
    }

    fn mods_page(&mut self, p: Page, sel: usize) {
        self.add.page = p;
        self.add.child = None;
        self.add.top = 0;
        self.sel = sel;
        self.add.status = match p {
            Page::Upload => RULE.into(),
            _ => String::new(),
        };
        if p == Page::Upload {
            self.add.account = modstore::account().map(|a| a.0);
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
        } else {
            self.list_step(k, n, out);
        }
        let n = self.add.rows();
        self.sel = self.sel.min(n - 1);
        self.add.top = self.add.top.min(self.sel).max((self.sel + 1).saturating_sub(SHOWN)).min(n.saturating_sub(SHOWN));
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
                let m = self.add.list[s - 2].clone();
                match modstore::account() {
                    Some((_, token)) => {
                        self.add.busy = true;
                        self.add.status = format!("Checking, packing and uploading {}...", m.name);
                        let card = super::previews::card(&self.add.iso, &m);
                        modstore::publish(&m, self.add.stats.get(&m.dir).copied(), card, token);
                        out.push(Sound(2));
                    }
                    None => {
                        self.add.account = None;
                        self.add.status = "Signed out: sign in again on the top row.".into();
                        out.push(Sound(1));
                    }
                }
            }
            Page::Mine => {
                let e = self.add.entries()[s].clone();
                match modstore::account() {
                    Some((_, token)) => {
                        self.add.busy = true;
                        self.add.status = format!("Taking {} off the store...", e.meta.name);
                        modstore::remove(&e, token);
                        out.push(Sound(2));
                    }
                    None => {
                        self.add.account = None;
                        self.add.status = "Signed out: sign in again on the Upload page.".into();
                        out.push(Sound(1));
                    }
                }
            }
            Page::Browse => {}
        }
    }

    /// The list's own press (no child open).
    fn list_step(&mut self, k: Press, n: usize, out: &mut Vec<Effect>) {
        use Effect::Sound;
        if k.up || k.down {
            self.sel = if k.down { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
            if !matches!(self.add.page, Page::Upload | Page::Mine) && !self.add.busy {
                self.add.status.clear();
            }
            out.push(Sound(0));
        }
        let s = self.sel;
        match self.add.page {
            Page::Installed => self.installed_step(k, s, n, out),
            Page::Browse => {
                if k.back || (k.ok && s == n - 1) {
                    self.mods_page(Page::Installed, 0);
                    out.push(Sound(1));
                } else if k.ok && s == n - 2 {
                    if !self.add.busy {
                        self.add.busy = true;
                        self.add.status = "Reading the mod store...".into();
                        modstore::list(true);
                    }
                    out.push(Sound(2));
                } else if k.ok && self.add.busy {
                    self.add.status = "Wait for the store to finish first.".into();
                } else if k.ok {
                    let e = self.add.entries()[s].clone();
                    if self.add.state(&e) == Some("Installed") {
                        self.add.status = format!("{} is installed already.", e.meta.name);
                    } else {
                        self.add.busy = true;
                        self.add.status = format!("Downloading {} ({})...", e.meta.name, mb(e.meta.size));
                        modstore::install(&e, root(), self.settings.0[super::UPSCALED_DOWNLOADS]);
                        out.push(Sound(2));
                    }
                }
            }
            Page::Upload => {
                if k.back || (k.ok && s == n - 1) {
                    self.mods_page(Page::Installed, 2);
                    out.push(Sound(1));
                } else if k.ok && self.add.busy {
                    self.add.status = "Wait for the last one to finish.".into();
                } else if k.ok && s == 0 {
                    self.account_step(out);
                } else if k.ok && self.add.account.is_none() {
                    self.add.status = "Sign in first: the top row.".into();
                    out.push(Sound(1));
                } else if k.ok && s == 1 {
                    self.mods_page(Page::Mine, 0);
                    out.push(Sound(2));
                } else if k.ok {
                    let name = self.add.list[s - 2].name.clone();
                    self.open_child(&["Upload"], format!("{RULE} Upload {name}?"), out);
                }
            }
            Page::Mine => self.mine_step(k, s, n, out),
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

    /// M19: My public mods: Take down on one of the player's uploads.
    fn mine_step(&mut self, k: Press, s: usize, n: usize, out: &mut Vec<Effect>) {
        use Effect::Sound;
        if k.back || (k.ok && s == n - 1) {
            self.mods_page(Page::Upload, 1);
            out.push(Sound(1));
        } else if k.ok && self.add.busy {
            self.add.status = "Wait for the store to finish first.".into();
        } else if k.ok && s == n - 2 {
            self.read_mine();
            out.push(Sound(2));
        } else if k.ok {
            let e = self.add.entries()[s].clone();
            if e.verified {
                self.open_child(&["Ask to delete"], format!("Ask for {} to be deleted from the store (a maintainer merges it)?", e.meta.name), out);
            } else {
                self.open_child(&["Take down"], format!("Take {} off the store (closes its pull request)?", e.meta.name), out);
            }
        }
    }

    fn installed_step(&mut self, k: Press, s: usize, n: usize, out: &mut Vec<Effect>) {
        use Effect::Sound;
        if k.back || (k.ok && s == n - 1) {
            self.screen = Screen::Main;
            self.sel = self.main_at(super::home::Row::Mods);
            // B205: ask once per change; Later leaves the note on the Mods row
            if std::mem::take(&mut self.add.fresh) {
                self.add.ask = Some(0);
            }
            out.push(Sound(1));
        } else if k.ok && s == 0 {
            self.mods_page(Page::Browse, 0);
            out.push(Sound(2));
        } else if k.ok && (s == 1 || s == 2) && self.add.phone {
            self.add.nophone = Some(s);
            out.push(Sound(1));
        } else if k.ok && s == 1 {
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
        } else if k.ok && s == 2 {
            self.mods_page(Page::Upload, 0);
            out.push(Sound(2));
        } else if k.ok && s == n - 2 {
            if let Err(e) = reload(out) {
                self.add.status = e;
            }
        } else if k.ok {
            let i = s - TOP;
            let (id, name, dir) = (&self.add.list[i].id, self.add.list[i].name.clone(), self.add.list[i].dir.clone());
            // desktop: only a store download / Add mod install (`mods/<id>`) is the game's to delete; a folder put in
            // by hand (a pack, a link to the mods repo, a renamed folder) is left to whoever put it there
            if cfg!(not(target_arch = "wasm32")) && !crate::modfs::installed_by_game(root(), &dir, &id) {
                self.add.status = format!("{name} was added by hand: delete {} to remove it.", dir.display());
                out.push(Sound(0));
            } else {
                self.open_child(&["Remove"], format!("Remove {name}? Reload then takes it out of the select."), out);
            }
        }
    }

    /// Removes installed mod `i` (its child's Remove).
    fn remove(&mut self, i: usize, out: &mut Vec<Effect>) {
        let (id, name, dir) = (self.add.list[i].id.clone(), self.add.list[i].name.clone(), self.add.list[i].dir.clone());
        crate::modfs::remove(&dir);
        #[cfg(target_arch = "wasm32")]
        crate::web::mod_drop(&self.add.list[i].folder());
        // a store download's own folder (`mods/<id>`, `mods/<id>-2`, …); a pack's mods are left to their pack
        #[cfg(not(target_arch = "wasm32"))]
        if crate::modfs::installed_by_game(root(), &dir, &id) {
            let _ = std::fs::remove_dir_all(&dir);
        }
        self.add.refresh();
        self.add.changed();
        self.add.status = format!("Removed {name}. Reload to take it out of the select.");
        out.push(Effect::Sound(1));
    }
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

pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let shot = m.add.shot;
    let v = &m.add;
    d.frame(5, 6);
    let label = |r: usize| -> String {
        match v.page {
            Page::Installed => match r {
                0 => "Browse mods".into(),
                1 => "Add mod".into(),
                2 => "Upload my mod".into(),
                r if r < TOP + v.list.len() => short(&v.list[r - TOP].name, 14),
                r if r == TOP + v.list.len() => "Reload".into(),
                _ => "Back".into(),
            },
            Page::Browse | Page::Mine => match v.entries().get(r) {
                Some(e) => short(&e.meta.name, 14),
                None if r == v.entries().len() => "Refresh".into(),
                None => "Back".into(),
            },
            Page::Upload => match r {
                0 if v.account.is_some() => "Signed in".into(),
                0 => "Sign in".into(),
                1 => "My public mods".into(),
                r => v.list.get(r - 2).map_or("Back".into(), |m| short(&m.name, 14)),
            },
        }
    };
    let end = (v.top + SHOWN).min(v.rows());
    let labels: Vec<String> = (v.top..end).map(label).collect();
    let hits = d.hits.len();
    let top = v.top;
    let off = |r: usize| v.phone && v.page == Page::Installed && (r + top == 1 || r + top == 2);
    d.rows_locked(&labels, m.sel - v.top, X, Y, W, hand_x, PITCH, off);
    // the rows shown are `top..`
    for h in &mut d.hits[hits..] {
        if let super::pointer::Act::Row(r, _) = &mut h.act {
            *r += v.top;
        }
    }
    if matches!(v.page, Page::Browse | Page::Mine) {
        for r in v.top..end {
            let Some(e) = v.entries().get(r) else { continue };
            let y = Y + PITCH * (r - v.top) as f32;
            if e.verified {
                d.outlined(|d| d.text("Verified", X + W + 6.0, y + 2.0, 15.0, GOLD));
            }
            if let Some(t) = v.state(e) {
                d.outlined(|d| d.text(t, X + W + 6.0, y + 20.0, 15.0, WHITE));
            }
        }
    }
    if let Some(e) = v.hovered(m.sel) {
        let mt = &e.meta;
        let mut lines = details(&mt.source, &mt.author, mt.hand.map(f32::from), mt.costumes.map(|c| c as usize), mt.like.map(usize::from));
        lines.push((format!("Version {}, {}", if mt.version.is_empty() { "1" } else { &mt.version }, mb(mt.size)), WHITE));
        lines.push(match (e.verified, v.state(e)) {
            (true, Some(t)) => (format!("Verified, {t}"), GOLD),
            (true, None) => ("Verified".into(), GOLD),
            (false, t) => (format!("Not reviewed yet{}", t.map_or(String::new(), |t| format!(", {t}"))), [100.0; 3]),
        });
        panel(d, &mt.name, mt.style.map(usize::from).zip(mt.grades), &lines, shot);
    } else if let Some(md) = v.local(m.sel) {
        let mut lines = details(&md.source, "", Some(md.hand), Some(md.costumes.len()), Some(md.donor));
        lines.push((format!("Folder {}", md.folder()), [100.0; 3]));
        panel(d, &md.name, v.stats.get(&md.dir).copied(), &lines, shot);
    }
    let info = if !v.status.is_empty() {
        v.status.clone()
    } else {
        match v.page {
            Page::Installed => match m.sel {
                0 => "Mods other players shared: install one with a press.".into(),
                1 | 2 if v.phone => "Not available in the browser on phones: use the desktop app.".into(),
                1 => "Add a custom character from a mod .zip (its mod.json and models).".into(),
                2 => "Share one of your mods with everyone (needs a Hugging Face account).".into(),
                s if s < TOP + v.list.len() => {
                    let md = &v.list[s - TOP];
                    format!("{} ({}, {} of {}): press to remove it.", md.name, md.folder(), s - TOP + 1, v.list.len())
                }
                s if s == TOP + v.list.len() => "Reload the game: added mods join the select, removed ones leave it.".into(),
                _ => "Back to the main menu.".into(),
            },
            Page::Browse => match v.entries().get(m.sel) {
                Some(e) => match v.state(e) {
                    Some("Installed") => format!("{} is installed.", e.meta.name),
                    Some(_) => format!("Press to update {}.", e.meta.name),
                    _ => format!("Press to install {}.", e.meta.name),
                },
                None if m.sel == v.entries().len() => "Read the store's list again.".into(),
                None if v.store.as_ref().is_some_and(|s| s.is_empty()) => "Nobody has shared a mod yet.".into(),
                None => "Back to your mods.".into(),
            },
            Page::Upload => match (m.sel, &v.account) {
                (0, Some(name)) => format!("Signed in to Hugging Face as {name}."),
                (0, None) => "Sign in to Hugging Face with a token, once: the game then uploads by itself.".into(),
                (1, _) => "The mods you've put on the store: take one down.".into(),
                _ => RULE.into(),
            },
            Page::Mine => match v.entries().get(m.sel) {
                Some(e) if e.verified => format!("{} is on the store (merged): press to ask for it to be deleted.", e.meta.name),
                Some(e) => format!("{} is live from your pull request: press to take it down.", e.meta.name),
                None if m.sel == v.entries().len() => "Read your uploads from the store again.".into(),
                None if v.mine.as_ref().is_some_and(|s| s.is_empty()) => "None of your mods are on the store.".into(),
                None => "Back to Upload my mod.".into(),
            },
        }
    };
    d.info(&info);
    if let Some(c) = &v.child
        && (v.top..end).contains(&c.row)
    {
        let row = [X, Y + PITCH * (c.row - v.top) as f32, W, H];
        child::draw(d, c, row, &label(c.row), hand_x);
    }
    if let Some(r) = v.nophone {
        nophone(d, if r == 1 { "Add mod" } else { "Upload my mod" });
    }
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

/// A mod's detail lines: where it's from, who made it, its hand, costumes and the disc character it plays like.
fn details(source: &str, author: &str, hand: Option<f32>, costumes: Option<usize>, like: Option<usize>) -> Vec<(String, [f32; 3])> {
    let mut l = Vec::new();
    if !source.is_empty() {
        l.push((format!("From {source}"), WHITE));
    }
    if !author.is_empty() {
        l.push((format!("By {author}"), WHITE));
    }
    match (hand, costumes) {
        (Some(h), Some(c)) => l.push((format!("{}-handed, {c} costume{}", if h < 0.0 { "Left" } else { "Right" }, if c == 1 { "" } else { "s" }), WHITE)),
        (Some(h), None) => l.push((format!("{}-handed", if h < 0.0 { "Left" } else { "Right" }), WHITE)),
        _ => {}
    }
    if let Some(name) = like.and_then(|c| CHAR_NAMES.get(c)) {
        l.push((format!("Plays like {name}"), WHITE));
    }
    l
}

/// The hovered mod on the right half, as the select's card: its picture (`Tex::Shelf`) and △ card side by side, its
/// name, then `lines`.
fn panel(d: &mut Draw, name: &str, stats: Option<(usize, [u8; 5])>, lines: &[(String, [f32; 3])], shot: bool) {
    let [x, y, w, h] = PANEL;
    d.box3([x, y, w, h], 12.0, [Some(INK), Some(WHITE), Some(CARD_FILL[2])], 128.0);
    let pic = [x + 8.0, y + 8.0, 128.0, 160.0];
    d.q(Tex::Solid, [0.0; 4], pic, [10.0; 3], 100.0);
    if shot {
        d.q(Tex::Shelf, [0.0, 0.0, 1.0, 1.0], pic, WHITE, 128.0);
    } else {
        d.centred("No picture", pic[0] + 64.0, pic[1] + 70.0, 16.0, [100.0; 3]);
    }
    match stats {
        Some((style, grades)) => attribute_card(d, 2, style, &grades, x + w - 152.0, y + 4.0),
        None => d.centred("No stats", x + w - 80.0, y + 80.0, 16.0, [100.0; 3]),
    }
    d.outlined(|d| d.label(name, x + w / 2.0, y + 176.0, 26.0, w - 16.0, GOLD));
    for (i, (s, rgb)) in lines.iter().enumerate().take(6) {
        d.label(s, x + w / 2.0, y + 210.0 + 20.0 * i as f32, 17.0, w - 16.0, *rgb);
    }
}

/// `HST_MENU=browsemods` (Browse, its entries made up from the installed mods: Verified, Installed and Update all
/// shown) / `mymods` (My public mods, the same made-up entries, one's Take down open) / `uploadmods` (Upload, one mod's
/// Upload open) / `signinmods` (Upload signed out, Sign in's buttons open) / `addmods` with `HST_CHILD=1` (Installed, a
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
        m.sel = 1;
        if screen == "mymods" {
            (v.page, v.mine, v.account, v.child) = (Page::Mine, v.store.take(), Some("FIZL0".into()), Some(Child::new(1, &["Take down"])));
            v.status = format!("Take {} off the store (closes its pull request)?", v.mine.as_ref().and_then(|l| l.get(1)).map_or("", |e| &e.meta.name));
        }
    } else if screen == "reloadmods" {
        (v.changed, v.ask) = (true, Some(0));
        m.sel = 3;
    } else if screen == "uploadmods" {
        v.page = Page::Upload;
        v.account = Some("FIZL0".into());
        v.child = Some(Child::new(2, &["Upload"]));
        v.status = format!("{RULE} Upload {}?", v.list.first().map_or("", |x| &x.name));
        m.sel = 2;
    } else if screen == "signinmods" {
        v.page = Page::Upload;
        v.child = Some(Child::new(0, &["Open token page", "Paste token"]));
        v.status = "Open token page, make a token (Write) there and copy it, then Paste token.".into();
        m.sel = 0;
    } else if screen == "addmods" && std::env::var("HST_CHILD").is_ok() && !v.list.is_empty() {
        v.child = Some(Child::new(TOP, &["Remove"]));
        v.status = format!("Remove {}? Reload then takes it out of the select.", v.list[0].name);
        m.sel = TOP;
    } else if screen.starts_with("phonemods") {
        v.phone = true;
        m.sel = 1;
        v.nophone = (screen == "phonemods-ask").then_some(1);
    }
}

/// The Add mod row's rectangle in % of the window (left, top, width, height), while it's on screen: where the page
/// puts its file input.
fn add_rect(m: &Menu, aspect: f32) -> Option<[f32; 4]> {
    let (share, v) = (widescreen::share(aspect), widescreen::vshare(aspect));
    (m.screen == Screen::Mods && m.add.page == Page::Installed && m.add.top == 0 && !m.add.phone && m.add.nophone.is_none())
        .then(|| [X * share / 6.4 + 50.0 * (1.0 - share), (Y + PITCH) * v / 4.48 + 50.0 * (1.0 - v), W * share / 6.4, H * v / 4.48])
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
        }
        menu.add.status = s;
    }
    for e in modstore::take() {
        let v = &mut menu.add;
        match e {
            Event::Listing(r) => {
                v.busy = false;
                match r {
                    Ok(list) => {
                        v.status = format!("{} mods in the store.", list.len());
                        v.store = Some(list);
                    }
                    Err(e) => {
                        v.status = e;
                        v.store.get_or_insert_default();
                    }
                }
            }
            Event::Installed(r) => {
                v.busy = false;
                v.refresh();
                if r.is_ok() {
                    v.changed();
                }
                v.status = match r {
                    // the browser's page says when it's saved
                    Ok(_) if cfg!(target_arch = "wasm32") => v.status.clone(),
                    Ok(name) => format!("Installed {name}. Restart the game to play it."),
                    Err(e) => format!("Not installed: {e}"),
                };
            }
            Event::Progress(s) => v.status = s,
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
            Event::Uploaded(r) => {
                v.busy = false;
                v.store = None;
                v.mine = None;
                v.status = r.unwrap_or_else(|e| format!("Not uploaded: {e}"));
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
        let [l, t, wd, h] = add_rect(&menu, widescreen::aspect(w))?;
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

    /// The rows: Browse, Add mod, Upload, each installed mod, Reload, Back; ✕ on a mod opens Remove / Cancel beside
    /// it (B202): ○ and Cancel close it, ↑↓ wrap in it, Remove removes the mod (from memory, and nothing else).
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
        assert!(m.add.status.starts_with("Remove Screen?"));
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
        let r = m.add.child.as_ref().unwrap().rects([X, Y + PITCH * TOP as f32, W, H]);
        assert_eq!(at(&m, X + 10.0, Y + 5.0), None);
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
        m.mods_step(ok(), &mut out);
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
        assert_eq!(m.add.child, None);
        assert!(m.add.status.starts_with("P was added by hand"), "{}", m.add.status);
    }

    /// A long list scrolls to keep the cursor on screen, and taps land on the drawn rows.
    #[test]
    fn scrolls_and_taps() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.list = (0..20).map(|i| a_mod(&format!("m{i}"), &format!("M{i}"))).collect();
        let mut out = Vec::new();
        for _ in 0..10 {
            m.mods_step(Press { down: true, ..default() }, &mut out);
        }
        assert_eq!((m.sel, m.add.top), (10, 4));
        let at = |x: f32, y: f32| super::super::pointer::tests::act_at(&m, x, y);
        assert_eq!(at(X + 10.0, Y + PITCH * 6.0 + 5.0), Some(super::super::pointer::Act::Row(10, super::super::pointer::OK)));
        assert_eq!(at(X + 10.0, Y + PITCH * 7.0 + 5.0), None);
        // the gap between two rows goes to the nearer one
        assert_eq!(at(X + 10.0, Y + H + 1.0), Some(super::super::pointer::Act::Row(4, super::super::pointer::OK)));
        assert!(add_rect(&m, 16.0 / 9.0).is_none());
        (0..10).for_each(|_| m.mods_step(Press { up: true, ..default() }, &mut out));
        assert_eq!((m.sel, m.add.top), (0, 0));
        // 4:3 fills the window: the row's own place
        let r = add_rect(&m, 4.0 / 3.0).unwrap();
        assert!((r[0] - X / 6.4).abs() < 1e-4 && (r[2] - W / 6.4).abs() < 1e-4);
    }

    /// Browse tags each store mod by what's installed (same zip: Installed, another: Update); pressing an installed
    /// one doesn't download it; Back returns to Browse mods. Upload opens Upload / Cancel, the rule shown; Sign in opens
    /// Open token page / Paste token, signed in Sign out.
    #[test]
    fn browse_and_upload_pages() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let e = |id: &str, sha: &str| Entry { meta: Meta { id: id.into(), name: id.into(), sha256: sha.repeat(64), size: 1, ..Default::default() }, rev: "main".into(), verified: true };
        m.add.store = Some(vec![e("a", "1"), e("b", "2"), e("c", "3")]);
        m.add.have = [("/a".to_string(), ("1".repeat(64), "mods/a".into())), ("/b".to_string(), ("9".repeat(64), "mods/b".into()))].into();
        m.add.list = vec![a_mod("a", "A")];
        let mut out = Vec::new();
        // B208: the page opens on Browse mods
        m.mods_open(&mut out);
        assert_eq!(m.sel, 0);
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.page, m.sel), (Page::Browse, 0));
        let tags: Vec<_> = m.add.entries().iter().map(|x| m.add.state(x)).collect();
        assert_eq!(tags, [Some("Installed"), Some("Update"), None]);
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.contains("installed already") && !m.add.busy);
        assert_eq!(super::super::pointer::tests::act_at(&m, X + 10.0, Y + PITCH + 5.0), Some(super::super::pointer::Act::Row(1, super::super::pointer::OK)));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.add.page, m.sel, m.screen), (Page::Installed, 0, Screen::Mods));
        m.sel = 2;
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.page, Page::Upload);
        assert_eq!(m.add.status, RULE);
        // the account row first, then the mods; uploading needs an account
        m.add.account = None;
        m.sel = 2;
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
        m.sel = 3;
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.page, m.sel), (Page::Installed, 2));
    }

    /// B208: on a phone Add mod and Upload my mod are greyed and put up the popup instead (no picker, no page); ✕, ○
    /// or a tap anywhere closes it; Browse mods still opens; the file input stays off.
    #[test]
    fn phone_rows_greyed() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        m.add.phone = true;
        let mut out = Vec::new();
        for (row, close) in [(1, ok()), (2, Press { back: true, ..default() })] {
            m.sel = row;
            m.mods_step(ok(), &mut out);
            assert_eq!((m.add.nophone, m.add.page, m.add.busy), (Some(row), Page::Installed, false));
            // the popup takes the moves
            m.mods_step(Press { down: true, ..default() }, &mut out);
            assert_eq!((m.sel, m.add.nophone), (row, Some(row)));
            m.mods_step(close, &mut out);
            assert_eq!((m.screen, m.add.nophone), (Screen::Mods, None));
        }
        assert!(add_rect(&m, 4.0 / 3.0).is_none());
        use super::super::pointer::{Act, OK, tests::act_at};
        m.add.nophone = Some(1);
        assert_eq!(act_at(&m, 320.0, ASK_TOP), Some(Act::Press(OK)));
        assert_eq!(act_at(&m, 10.0, 300.0), None);
        m.add.nophone = None;
        m.sel = 0;
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.page, Page::Browse);
    }

    /// M19: My public mods needs an account; its entries open Take down (a PR's, it closes) or Ask to delete (a merged
    /// one's), Back returns to its Upload row.
    #[test]
    fn my_public_mods() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let mut out = Vec::new();
        m.mods_page(Page::Upload, 1);
        m.add.account = None;
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.starts_with("Sign in first") && m.add.page == Page::Upload);
        let e = |id: &str, rev: &str| Entry { meta: Meta { id: id.into(), name: id.into(), ..Default::default() }, rev: rev.into(), verified: rev == "main" };
        m.add.mine = Some(vec![e("a", "refs/pr/4"), e("b", "main")]);
        m.add.account = Some("me".into());
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.page, m.sel, m.add.rows(), m.add.busy), (Page::Mine, 0, 4, false));
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.as_ref().is_some_and(|c| c.row == 0 && c.opts[0] == "Take down") && m.add.status.contains("closes its pull request"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        m.mods_step(Press { down: true, ..default() }, &mut out);
        m.mods_step(ok(), &mut out);
        assert!(m.add.child.as_ref().is_some_and(|c| c.row == 1 && c.opts[0] == "Ask to delete") && m.add.status.contains("maintainer"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.add.hovered(m.sel).map(|x| x.meta.id.as_str()), Some("b"));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.add.page, m.sel), (Page::Upload, 1));
    }
}
