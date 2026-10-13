//! Mods (M16, the browser build's main menu): Add mod picks a mod `.zip` (the page's file picker, `web.rs`), which is
//! checked whole and kept in OPFS `mods/<id>/` (`modfs::install`); ✕ twice on an installed mod removes it. The select's
//! roster is built when the game starts, so Reload brings the changes into it. Mouse and touch: a tap on a row chooses
//! it; on Add mod the page's own file input lies over the row, so the tap that opens the picker is the browser's own
//! (iOS opens a picker only inside a real tap).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::path::Path;

use super::super::widescreen;
use super::{Draw, Effect, Menu, Press, Screen};

/// The mods folder: `mods/` beside the disc pack, as the select's roster reads it (`main_menu.rs setup`).
pub(crate) const ROOT: &str = "mods";
/// Rows drawn at once (the rest scroll), and where.
const SHOWN: usize = 7;
const X: f32 = 170.0;
const Y: f32 = 74.0;
const W: f32 = 300.0;
const PITCH: f32 = 44.0;
const H: f32 = 38.0;

#[derive(Default, Clone, Debug)]
pub(super) struct View {
    /// Installed mods: (id, name).
    list: Vec<(String, String)>,
    /// The mod a first ✕ armed for removal.
    armed: Option<usize>,
    /// What the last import or removal did (the description bar).
    status: String,
    /// The first row drawn.
    top: usize,
}

impl View {
    fn refresh(&mut self) {
        self.list = crate::mods::list(Path::new(ROOT)).into_iter().map(|m| (m.id, m.name)).collect();
        self.armed = None;
    }

    /// Add mod, the mods, Reload, Back.
    fn rows(&self) -> usize {
        self.list.len() + 3
    }
}

impl Menu {
    pub(super) fn mods_open(&mut self, out: &mut Vec<Effect>) {
        self.add.status.clear();
        self.add.top = 0;
        self.go(Screen::Mods, out);
    }

    pub(super) fn mods_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        use Effect::Sound;
        let n = self.add.rows();
        self.sel = self.sel.min(n - 1);
        if k.up || k.down {
            self.sel = if k.down { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
            self.add.armed = None;
            self.add.status.clear();
            out.push(Sound(0));
        }
        let s = self.sel;
        if k.back || (k.ok && s == n - 1) {
            self.screen = Screen::Main;
            self.sel = 3;
            out.push(Sound(1));
            return;
        } else if k.ok && s == 0 {
            // a pad's or key's ✕ (a tap lands on the page's input itself)
            #[cfg(target_arch = "wasm32")]
            crate::web::mod_pick();
            self.add.status = "Pick a mod .zip (a character from the mods repo).".into();
            out.push(Sound(2));
        } else if k.ok && s == n - 2 {
            #[cfg(target_arch = "wasm32")]
            crate::web::reload();
        } else if k.ok {
            let i = s - 1;
            let (id, name) = self.add.list[i].clone();
            if self.add.armed == Some(i) {
                crate::modfs::remove(&Path::new(ROOT).join(&id));
                #[cfg(target_arch = "wasm32")]
                crate::web::mod_drop(&id);
                self.add.refresh();
                self.add.status = format!("Removed {name}. Reload to take it out of the select.");
                out.push(Sound(1));
            } else {
                self.add.armed = Some(i);
                self.add.status = format!("Press again to remove {name}.");
                out.push(Sound(0));
            }
        }
        let n = self.add.rows();
        self.sel = self.sel.min(n - 1);
        self.add.top = self.add.top.min(self.sel).max((self.sel + 1).saturating_sub(SHOWN)).min(n.saturating_sub(SHOWN));
    }
}

pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let v = &m.add;
    d.frame(5, 6);
    let label = |r: usize| -> String {
        match r {
            0 => "Add mod".into(),
            r if r <= v.list.len() => {
                let name = &v.list[r - 1].1;
                if name.chars().count() > 20 { format!("{}...", name.chars().take(17).collect::<String>()) } else { name.clone() }
            }
            r if r == v.list.len() + 1 => "Reload".into(),
            _ => "Back".into(),
        }
    };
    let end = (v.top + SHOWN).min(v.rows());
    let labels: Vec<String> = (v.top..end).map(label).collect();
    d.rows_at(&labels, m.sel - v.top, X, Y, W, hand_x, PITCH);
    let info = if !v.status.is_empty() {
        v.status.clone()
    } else {
        match m.sel {
            0 => "Add a custom character from a mod .zip (its mod.json and models).".into(),
            s if s <= v.list.len() => format!("{} ({}, {} of {}): press twice to remove it.", v.list[s - 1].1, v.list[s - 1].0, s, v.list.len()),
            s if s == v.list.len() + 1 => "Reload the game: added mods join the select, removed ones leave it.".into(),
            _ => "Back to the main menu.".into(),
        }
    };
    d.info(&info);
}

/// The row under `p` (640×448), as `layout` draws them.
fn row_at(v: &View, p: Vec2) -> Option<usize> {
    let k = ((p.y - Y) / PITCH).floor();
    let r = v.top + k as usize;
    (p.x >= X - 60.0 && p.x < X + W && k >= 0.0 && (k as usize) < SHOWN && r < v.rows() && p.y - Y - k * PITCH < H).then_some(r)
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
    (m.screen == Screen::Mods && m.add.top == 0).then(|| [X * share / 6.4 + 50.0 * (1.0 - share), Y / 4.48, W * share / 6.4, H / 4.48])
}

/// The list read on coming to the screen; the page's file input follows the Add mod row; an import's outcome comes
/// back as the status.
fn page(mut menu: ResMut<Menu>, window: Query<&Window, With<PrimaryWindow>>, mut was: Local<Option<[f32; 4]>>, mut on: Local<bool>) {
    if (menu.screen == Screen::Mods) != *on {
        *on = !*on;
        if *on {
            menu.add.refresh();
        }
    }
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = crate::web::mod_status_take() {
        menu.add.refresh();
        menu.add.status = s;
        let n = menu.add.rows();
        menu.sel = menu.sel.min(n - 1);
    }
    let rect = window.single().ok().and_then(|w| add_rect(&menu, w.width() / w.height().max(1.0)));
    if rect != *was {
        *was = rect;
        #[cfg(target_arch = "wasm32")]
        crate::web::mod_picker(rect);
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (pointer.before(super::step), page.before(super::draw)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok() -> Press {
        Press { ok: true, ..default() }
    }

    /// The rows: Add mod, each installed mod, Reload, Back; ✕ twice removes a mod (from memory, and nothing else).
    #[test]
    fn removes_on_the_second_press() {
        let mut m = Menu::default();
        crate::modfs::add(&Path::new(ROOT).join("t_screen/mod.json"), Vec::new().into());
        m.add.list = vec![("t_screen".into(), "Screen".into()), ("b".into(), "B".into())];
        m.screen = Screen::Mods;
        m.sel = 1;
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
        m.add.list = (0..20).map(|i| (format!("m{i}"), format!("M{i}"))).collect();
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
}
