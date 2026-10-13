//! Mods (M16, M17). Installed: Add mod picks a mod `.zip` (the browser build: the page's file picker, `web.rs`;
//! desktop: the system's, `modstore::pick`), which is checked whole and kept in `mods/<id>/` (OPFS in the browser,
//! beside the game on desktop; `modfs::install`); ✕ twice on an installed mod removes it.
//! Browse mods lists the mod store (`modstore.rs`: Verified = merged on the store's `main`, Installed / Update by the
//! zip's sha256) with the hovered mod's preview; ✕ downloads, checks and installs one. Upload my mod shows the rule,
//! then packages an installed mod and opens the store's upload page. The select's roster is built when the game
//! starts, so Reload brings changes into it. Mouse and touch: a tap on a row chooses it; on Add mod the page's own
//! file input lies over the row, so the tap that opens the picker is the browser's own (iOS opens a picker only
//! inside a real tap).

use std::collections::HashMap;
use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::super::widescreen;
use super::{Draw, Effect, GOLD, Menu, Press, Screen, Tex, WHITE};
use crate::modstore::{self, Entry, Event};
use crate::mods::Mod;

/// The mods folder: `mods/` beside the disc pack, as the select's roster reads it (`main_menu.rs setup`).
pub(crate) const ROOT: &str = "mods";
/// Rows drawn at once (the rest scroll), and where.
const SHOWN: usize = 7;
const X: f32 = 170.0;
const Y: f32 = 74.0;
const W: f32 = 300.0;
/// Browse's narrower rows: its tags beside them, the hovered mod's preview right of those.
const BX: f32 = 140.0;
const BW: f32 = 250.0;
const SHOT: [f32; 4] = [492.0, 80.0, 128.0, 160.0];
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
}

#[derive(Default, Clone, Debug)]
pub(super) struct View {
    page: Page,
    /// Installed mods.
    list: Vec<Mod>,
    /// The store's listing (None: not read yet), and each entry's sha256 if it's installed.
    store: Option<Vec<Entry>>,
    have: HashMap<String, Option<String>>,
    /// The listing or an install is under way.
    busy: bool,
    /// The row a first ✕ armed (removal, or upload).
    armed: Option<usize>,
    /// What the last action did (the description bar).
    status: String,
    /// The first row drawn.
    top: usize,
}

impl View {
    fn refresh(&mut self) {
        self.list = crate::mods::list(Path::new(ROOT));
        self.have = self.list.iter().map(|m| (m.id.clone(), modstore::stamp(&m.dir).map(|s| s.sha256))).collect();
        self.armed = None;
    }

    fn entries(&self) -> &[Entry] {
        self.store.as_deref().unwrap_or_default()
    }

    /// Installed: Add mod, Browse mods, Upload my mod, the mods, Reload, Back. Browse: the store's mods, Refresh,
    /// Back. Upload: the mods, Back.
    fn rows(&self) -> usize {
        match self.page {
            Page::Installed => TOP + self.list.len() + 2,
            Page::Browse => self.entries().len() + 2,
            Page::Upload => self.list.len() + 1,
        }
    }

    /// Entry `e`'s tag: installed as listed, installed but different (an update), or not.
    fn state(&self, e: &Entry) -> Option<&'static str> {
        match self.have.get(&e.meta.id) {
            None => None,
            Some(Some(sha)) if *sha != e.meta.sha256 => Some("Update"),
            Some(_) => Some("Installed"),
        }
    }

    fn geom(&self) -> (f32, f32) {
        if self.page == Page::Browse { (BX, BW) } else { (X, W) }
    }

    /// The browse entry under the cursor.
    fn hovered(&self, sel: usize) -> Option<&Entry> {
        (self.page == Page::Browse).then(|| self.entries().get(sel)).flatten()
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
        self.add.armed = None;
        self.add.top = 0;
        self.sel = sel;
        self.add.status = match p {
            Page::Upload => RULE.into(),
            _ => String::new(),
        };
        if p == Page::Browse && self.add.store.is_none() && !self.add.busy {
            self.add.busy = true;
            self.add.status = "Reading the mod store...".into();
            modstore::list(false);
        }
    }

    pub(super) fn mods_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        use Effect::Sound;
        let n = self.add.rows();
        self.sel = self.sel.min(n - 1);
        if k.up || k.down {
            self.sel = if k.down { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
            self.add.armed = None;
            if self.add.page != Page::Upload && !self.add.busy {
                self.add.status.clear();
            }
            out.push(Sound(0));
        }
        let s = self.sel;
        match self.add.page {
            Page::Installed => self.installed_step(k, s, n, out),
            Page::Browse => {
                if k.back || (k.ok && s == n - 1) {
                    self.mods_page(Page::Installed, 1);
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
                        modstore::install(&e, Path::new(ROOT));
                        out.push(Sound(2));
                    }
                }
            }
            Page::Upload => {
                if k.back || (k.ok && s == n - 1) {
                    self.mods_page(Page::Installed, 2);
                    out.push(Sound(1));
                } else if k.ok && self.add.busy {
                    self.add.status = "Wait for the last upload to be packed.".into();
                } else if k.ok && self.add.armed == Some(s) {
                    let m = self.add.list[s].clone();
                    self.add.busy = true;
                    self.add.armed = None;
                    self.add.status = format!("Checking and packing {}...", m.name);
                    modstore::upload(&m);
                    out.push(Sound(2));
                } else if k.ok {
                    self.add.armed = Some(s);
                    self.add.status = format!("{RULE} Press again to pack {}.", self.add.list[s].name);
                    out.push(Sound(0));
                }
            }
        }
        let n = self.add.rows();
        self.sel = self.sel.min(n - 1);
        self.add.top = self.add.top.min(self.sel).max((self.sel + 1).saturating_sub(SHOWN)).min(n.saturating_sub(SHOWN));
    }

    fn installed_step(&mut self, k: Press, s: usize, n: usize, out: &mut Vec<Effect>) {
        use Effect::Sound;
        if k.back || (k.ok && s == n - 1) {
            self.screen = Screen::Main;
            self.sel = 3;
            out.push(Sound(1));
        } else if k.ok && s == 0 {
            // a pad's or key's ✕ (a tap lands on the page's input itself)
            #[cfg(target_arch = "wasm32")]
            crate::web::mod_pick();
            // desktop: the system's file picker, once at a time
            #[cfg(not(target_arch = "wasm32"))]
            if !std::mem::replace(&mut self.add.busy, true) {
                modstore::pick(Path::new(ROOT));
            }
            self.add.status = "Pick a mod .zip (a character from the mods repo).".into();
            out.push(Sound(2));
        } else if k.ok && s == 1 {
            self.mods_page(Page::Browse, 0);
            out.push(Sound(2));
        } else if k.ok && s == 2 {
            self.mods_page(Page::Upload, 0);
            out.push(Sound(2));
        } else if k.ok && s == n - 2 {
            #[cfg(target_arch = "wasm32")]
            crate::web::reload();
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.add.status = "Restart the game to bring the changes into the select.".into();
            }
        } else if k.ok {
            let i = s - TOP;
            let (id, name) = (self.add.list[i].id.clone(), self.add.list[i].name.clone());
            if self.add.armed == Some(i) {
                let dir = self.add.list[i].dir.clone();
                crate::modfs::remove(&dir);
                #[cfg(target_arch = "wasm32")]
                crate::web::mod_drop(&id);
                // a store download's own folder (`mods/<id>`); a pack's mods are left to their pack
                #[cfg(not(target_arch = "wasm32"))]
                if dir == Path::new(ROOT).join(&id) {
                    let _ = std::fs::remove_dir_all(&dir);
                }
                self.add.refresh();
                self.add.status = format!("Removed {name}. Reload to take it out of the select.");
                out.push(Sound(1));
            } else {
                self.add.armed = Some(i);
                self.add.status = format!("Press again to remove {name}.");
                out.push(Sound(0));
            }
        }
    }
}

pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let v = &m.add;
    d.frame(5, 6);
    let (x, w) = v.geom();
    let label = |r: usize| -> String {
        match v.page {
            Page::Installed => match r {
                0 => "Add mod".into(),
                1 => "Browse mods".into(),
                2 => "Upload my mod".into(),
                r if r < TOP + v.list.len() => short(&v.list[r - TOP].name, 20),
                r if r == TOP + v.list.len() => "Reload".into(),
                _ => "Back".into(),
            },
            Page::Browse => match v.entries().get(r) {
                Some(e) => short(&e.meta.name, 16),
                None if r == v.entries().len() => "Refresh".into(),
                None => "Back".into(),
            },
            Page::Upload => v.list.get(r).map_or("Back".into(), |m| short(&m.name, 20)),
        }
    };
    let end = (v.top + SHOWN).min(v.rows());
    let labels: Vec<String> = (v.top..end).map(label).collect();
    d.rows_at(&labels, m.sel - v.top, x, Y, w, hand_x, PITCH);
    if v.page == Page::Browse {
        for r in v.top..end {
            let Some(e) = v.entries().get(r) else { continue };
            let y = Y + PITCH * (r - v.top) as f32;
            if e.verified {
                d.outlined(|d| d.text("Verified", x + w + 6.0, y + 2.0, 16.0, GOLD));
            }
            if let Some(t) = v.state(e) {
                d.outlined(|d| d.text(t, x + w + 6.0, y + 20.0, 16.0, WHITE));
            }
        }
        if v.hovered(m.sel).is_some() {
            d.q(Tex::Shelf, [0.0, 0.0, 1.0, 1.0], SHOT, WHITE, 128.0);
        }
    }
    let info = if !v.status.is_empty() {
        v.status.clone()
    } else {
        match v.page {
            Page::Installed => match m.sel {
                0 => "Add a custom character from a mod .zip (its mod.json and models).".into(),
                1 => "Mods other players shared: install one with a press.".into(),
                2 => "Share one of your mods with everyone (needs a Hugging Face account).".into(),
                s if s < TOP + v.list.len() => {
                    let md = &v.list[s - TOP];
                    format!("{} ({}, {} of {}): press twice to remove it.", md.name, md.id, s - TOP + 1, v.list.len())
                }
                s if s == TOP + v.list.len() => "Reload the game: added mods join the select, removed ones leave it.".into(),
                _ => "Back to the main menu.".into(),
            },
            Page::Browse => match v.entries().get(m.sel) {
                Some(e) => {
                    let by = if e.meta.author.is_empty() { String::new() } else { format!(", by {}", e.meta.author) };
                    let from = if e.meta.source.is_empty() { String::new() } else { format!(" from {}", e.meta.source) };
                    let what = match v.state(e) {
                        Some("Installed") => "installed",
                        Some(_) => "press to update",
                        _ => "press to install",
                    };
                    format!("{}{from}{by}, {}{}: {what}.", e.meta.name, mb(e.meta.size), if e.verified { ", Verified" } else { "" })
                }
                None if m.sel == v.entries().len() => "Read the store's list again.".into(),
                None if v.store.as_ref().is_some_and(|s| s.is_empty()) => "Nobody has shared a mod yet.".into(),
                None => "Back to your mods.".into(),
            },
            Page::Upload => RULE.into(),
        }
    };
    d.info(&info);
}

/// `HST_MENU=browsemods` (Browse, its entries made up from the installed mods: Verified, Installed and Update all
/// shown) / `uploadmods` (Upload, one mod armed): for `--shot`s.
pub(super) fn sample(m: &mut Menu, screen: &str) {
    let v = &mut m.add;
    v.refresh();
    if screen == "browsemods" {
        v.page = Page::Browse;
        v.store = Some(
            v.list.iter().take(9).enumerate().map(|(i, md)| {
                let meta = modstore::Meta { id: md.id.clone(), name: md.name.clone(), source: md.source.clone(), author: ["FIZL0", "someone", ""][i % 3].into(), size: (i as u64 + 1) * 7 << 20, sha256: format!("{:064}", i % 2), ..Default::default() };
                Entry { meta, rev: if i % 3 == 0 { "main".into() } else { format!("refs/pr/{i}") }, verified: i % 3 == 0 }
            }).collect(),
        );
        v.have = v.list.iter().take(4).enumerate().map(|(i, md)| (md.id.clone(), (i != 2).then(|| format!("{:064}", 0)))).collect();
        m.sel = 1;
    } else if screen == "uploadmods" {
        v.page = Page::Upload;
        v.armed = Some(0);
        v.status = format!("{RULE} Press again to pack {}.", v.list.first().map_or("", |x| &x.name));
    }
}

/// The row under `p` (640×448), as `layout` draws them.
fn row_at(v: &View, p: Vec2) -> Option<usize> {
    let (x, w) = v.geom();
    let k = ((p.y - Y) / PITCH).floor();
    let r = v.top + k as usize;
    (p.x >= x - 60.0 && p.x < x + w && k >= 0.0 && (k as usize) < SHOWN && r < v.rows() && p.y - Y - k * PITCH < H).then_some(r)
}

/// A click or tap on a row chooses it (as ✕ would), the right button is Back.
fn pointer(
    window: Query<&Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut menu: ResMut<Menu>,
    mut click: ResMut<super::pause::Click>,
) {
    if menu.screen != Screen::Mods {
        return;
    }
    let (at, back) = super::pause::pointer_press(&window, &mouse, &touches);
    if let Some(r) = at.and_then(|p| row_at(&menu.add, p)) {
        if r != menu.sel {
            menu.add.armed = None;
        }
        menu.sel = r;
        click.0 = Some(Press { ok: true, ..default() });
    } else if back {
        click.0 = Some(Press { back: true, ..default() });
    }
}

/// The Add mod row's rectangle in % of the window (left, top, width, height), while it's on screen: where the page
/// puts its file input.
fn add_rect(m: &Menu, aspect: f32) -> Option<[f32; 4]> {
    let share = widescreen::share(aspect);
    (m.screen == Screen::Mods && m.add.page == Page::Installed && m.add.top == 0).then(|| [X * share / 6.4 + 50.0 * (1.0 - share), Y / 4.48, W * share / 6.4, H / 4.48])
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
fn page(
    mut menu: ResMut<Menu>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut shelf: ResMut<Shelf>,
    mut images: ResMut<Assets<Image>>,
    mut was: Local<Option<[f32; 4]>>,
    mut on: Local<bool>,
) {
    if (menu.screen == Screen::Mods) != *on {
        *on = !*on;
        // (a `sample` screen opens on its own page, already read)
        if *on && menu.add.page == Page::Installed {
            menu.add.refresh();
        }
    }
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = crate::web::mod_status_take() {
        menu.add.refresh();
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
                v.status = match r {
                    // the browser's page says when it's saved
                    Ok(_) if cfg!(target_arch = "wasm32") => v.status.clone(),
                    Ok(name) => format!("Installed {name}. Restart the game to play it."),
                    Err(e) => format!("Not installed: {e}"),
                };
            }
            Event::Packaged(r) => {
                v.busy = false;
                v.status = r.unwrap_or_else(|e| format!("Can't upload it: {e}"));
            }
            Event::Preview(id, png) => {
                let img = png.and_then(|b| {
                    Image::from_buffer(&b, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), RenderAssetUsages::RENDER_WORLD).ok()
                });
                shelf.shots.insert(id, img.map(|i| images.add(i)));
            }
        }
    }
    let n = menu.add.rows();
    menu.sel = menu.sel.min(n - 1);
    let hovered = if menu.screen == Screen::Mods { menu.add.hovered(menu.sel).cloned() } else { None };
    shelf.shown = hovered.as_ref().and_then(|e| match shelf.shots.get(&e.meta.id) {
        Some(h) => h.clone(),
        None => {
            modstore::preview(e);
            None
        }
    });
    if let Some(e) = hovered {
        shelf.shots.entry(e.meta.id).or_default();
    }
    let rect = window.single().ok().and_then(|w| add_rect(&menu, w.width() / w.height().max(1.0)));
    if rect != *was {
        *was = rect;
        #[cfg(target_arch = "wasm32")]
        crate::web::mod_picker(rect);
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Shelf>().add_systems(Update, (pointer.before(super::step), page.before(super::draw)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modstore::Meta;

    fn ok() -> Press {
        Press { ok: true, ..default() }
    }

    fn a_mod(id: &str, name: &str) -> Mod {
        Mod {
            dir: Path::new(ROOT).join(id),
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

    /// The rows: Add mod, Browse, Upload, each installed mod, Reload, Back; ✕ twice removes a mod (from memory, and
    /// nothing else).
    #[test]
    fn removes_on_the_second_press() {
        let mut m = Menu::default();
        crate::modfs::add(&Path::new(ROOT).join("t_screen/mod.json"), Vec::new().into());
        m.add.list = vec![a_mod("t_screen", "Screen"), a_mod("b", "B")];
        m.screen = Screen::Mods;
        m.sel = TOP;
        let mut out = Vec::new();
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.armed, Some(0));
        assert!(m.add.status.contains("again"));
        assert!(crate::modfs::is_file(&Path::new(ROOT).join("t_screen/mod.json")));
        m.mods_step(ok(), &mut out);
        assert!(!crate::modfs::is_file(&Path::new(ROOT).join("t_screen/mod.json")));
        assert!(m.add.status.starts_with("Removed Screen"));
        m.sel = 0;
        m.mods_step(Press { up: true, ..default() }, &mut out);
        assert_eq!(m.sel, m.add.rows() - 1);
        m.mods_step(ok(), &mut out);
        assert_eq!((m.screen, m.sel), (Screen::Main, 3));
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
        assert_eq!(row_at(&m.add, Vec2::new(X + 10.0, Y + PITCH * 6.0 + 5.0)), Some(10));
        assert_eq!(row_at(&m.add, Vec2::new(X + 10.0, Y + PITCH * 7.0 + 5.0)), None);
        assert_eq!(row_at(&m.add, Vec2::new(X + 10.0, Y + H + 2.0)), None);
        assert!(add_rect(&m, 16.0 / 9.0).is_none());
        (0..10).for_each(|_| m.mods_step(Press { up: true, ..default() }, &mut out));
        assert_eq!((m.sel, m.add.top), (0, 0));
        // 4:3 fills the window: the row's own place
        let r = add_rect(&m, 4.0 / 3.0).unwrap();
        assert!((r[0] - X / 6.4).abs() < 1e-4 && (r[2] - W / 6.4).abs() < 1e-4);
    }

    /// Browse tags each store mod by what's installed (same zip: Installed, another: Update); pressing an installed
    /// one doesn't download it; Back returns to Browse mods. Upload asks twice, the rule shown both times.
    #[test]
    fn browse_and_upload_pages() {
        let mut m = Menu { screen: Screen::Mods, ..default() };
        let e = |id: &str, sha: &str| Entry { meta: Meta { id: id.into(), name: id.into(), sha256: sha.repeat(64), size: 1, ..Default::default() }, rev: "main".into(), verified: true };
        m.add.store = Some(vec![e("a", "1"), e("b", "2"), e("c", "3")]);
        m.add.have = [("a".to_string(), Some("1".repeat(64))), ("b".to_string(), Some("9".repeat(64)))].into();
        m.add.list = vec![a_mod("a", "A")];
        let mut out = Vec::new();
        m.sel = 1;
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.page, m.sel), (Page::Browse, 0));
        let tags: Vec<_> = m.add.entries().iter().map(|x| m.add.state(x)).collect();
        assert_eq!(tags, [Some("Installed"), Some("Update"), None]);
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.contains("installed already") && !m.add.busy);
        assert_eq!(row_at(&m.add, Vec2::new(BX + 10.0, Y + PITCH + 5.0)), Some(1));
        m.mods_step(Press { back: true, ..default() }, &mut out);
        assert_eq!((m.add.page, m.sel, m.screen), (Page::Installed, 1, Screen::Mods));
        m.sel = 2;
        m.mods_step(ok(), &mut out);
        assert_eq!(m.add.page, Page::Upload);
        assert_eq!(m.add.status, RULE);
        m.mods_step(ok(), &mut out);
        assert!(m.add.status.starts_with(RULE) && m.add.status.ends_with("pack A.") && !m.add.busy);
        m.sel = 1;
        m.mods_step(ok(), &mut out);
        assert_eq!((m.add.page, m.sel), (Page::Installed, 2));
    }
}
