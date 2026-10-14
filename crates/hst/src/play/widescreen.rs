//! Widescreen: the 3D view widens with the window (`camera` keeps the game's vertical field of view), while the HUD
//! stays on the PS2's 4:3 screen (its 640×448 space), centred, so its art keeps its shape. The score panel is the
//! exception: its halves are pinned to the window's edges (`pin`) so they don't float inside the 4:3 screen, and so
//! are the menus' (`place`): walls and bars span the window, side cards keep to its edges, the rest stays centred.
//! A window taller than 4:3 letterboxes the 4:3 screen top and bottom instead (`vplace`, B232): one scale both ways.
//!
//! B117: the Aspect Ratio setting (`ASPECT`) can fix the picture to 4:3, 16:9, 16:10 or 21:9: everything (3D, HUD,
//! menus) is drawn into the window's centred rect of that ratio (`view`), black bars fill the rest (`hud_gamma`'s
//! composite camera draws only there). All the layouts here take the view's aspect, not the window's.
//!
//! B118: in a two-player phone match (`SIDEWAYS`) held upright, the HUD is laid out on the view turned a quarter turn
//! (`hud_gamma` draws it turned back), so the score and pop-ups read from the phone's side, each player's panel at
//! their own end (1P's at the bottom). `view_size`/`aspect`/`in_view` are the HUD's; the 3D cameras take `scene_aspect`.

use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// B118: a two-player phone match is on (set by the main menu at its start, cleared when the menu is back).
pub static SIDEWAYS: AtomicBool = AtomicBool::new(false);

/// Whether a `size` view's HUD is turned (B118): a two-player phone match in a portrait view.
pub fn turned(size: Vec2) -> bool {
    SIDEWAYS.load(Ordering::Relaxed) && size.x < size.y
}

/// The Aspect Ratio setting's choices (index in `ASPECT`): Auto fills the window (B17's widescreen).
pub const ASPECTS: [&str; 5] = ["Auto", "4:3", "16:9", "16:10", "21:9"];
const RATIOS: [f32; 5] = [0.0, 4.0 / 3.0, 16.0 / 9.0, 16.0 / 10.0, 21.0 / 9.0];
/// The aspect ratio in force (an `ASPECTS` index): the settings' (`main_menu`), 4:3 in the original mode.
pub static ASPECT: AtomicU8 = AtomicU8::new(0);

/// The centred rect of a `size` window the game draws into: all of it at Auto, else the largest rect of the fixed
/// ratio. A portrait window (a phone held upright) keeps all of it, for its own layout (B112).
pub fn view(size: Vec2) -> Rect {
    view_at(size, ASPECT.load(Ordering::Relaxed))
}

fn view_at(size: Vec2, aspect: u8) -> Rect {
    let r = RATIOS.get(aspect as usize).copied().unwrap_or(0.0);
    if r == 0.0 || size.x < size.y {
        return Rect::from_corners(Vec2::ZERO, size);
    }
    let v = if size.x > size.y * r { Vec2::new(size.y * r, size.y) } else { Vec2::new(size.x, size.x / r) };
    Rect::from_center_size(size / 2.0, v)
}

/// The HUD's view size (logical px) in `w`: the view's, turned when `turned`.
pub fn view_size(w: &Window) -> Vec2 {
    let v = view(Vec2::new(w.width(), w.height())).size();
    if turned(v) { v.yx() } else { v }
}

/// The HUD's view width over height.
pub fn aspect(w: &Window) -> f32 {
    let v = view_size(w);
    v.x / v.y.max(1.0)
}

/// The 3D picture's width over height (the view's, never turned).
pub fn scene_aspect(w: &Window) -> f32 {
    let v = view(Vec2::new(w.width(), w.height())).size();
    v.x / v.y.max(1.0)
}

/// A point in a `size` window (or page) → the HUD's view size and the point in it (turned as `hud_gamma` draws it:
/// the HUD's top along the view's right edge).
pub fn in_view(size: Vec2, p: Vec2) -> (Vec2, Vec2) {
    in_view_at(size, p, SIDEWAYS.load(Ordering::Relaxed))
}

fn in_view_at(size: Vec2, p: Vec2, sideways: bool) -> (Vec2, Vec2) {
    let v = view(size);
    let (s, p) = (v.size().max(Vec2::ONE), p - v.min);
    if sideways && s.x < s.y { (s.yx(), Vec2::new(s.y - p.y, p.x)) } else { (s, p) }
}

/// A HUD root on the centred 4:3 screen.
#[derive(Component)]
pub struct Screen43;

/// A HUD root node covering the centred 4:3 screen (kept fitted to the window by `fit`).
pub fn screen_43() -> (Node, Screen43) {
    let node = Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    };
    (node, Screen43)
}

/// Share of the window's width the 4:3 screen covers: all of it at 4:3 or narrower (narrower windows letterbox it
/// top and bottom, `vshare`).
pub(super) fn share(aspect: f32) -> f32 {
    (4.0 / 3.0 / aspect).min(1.0)
}

/// A 640-wide screen rect `x, w` on a full-window node, at the 4:3 screen's scale (`share`): left of the screen's
/// centre it keeps its distance from the window's left edge, right of it from the right edge. Left and width in
/// percent of the window.
pub(super) fn pin(x: f32, w: f32, share: f32) -> (f32, f32) {
    let left = if x + w / 2.0 < 320.0 { x / 6.4 * share } else { 100.0 - (640.0 - x) / 6.4 * share };
    (left, w / 6.4 * share)
}

/// Where a 640-space rect goes on a full-window node (`place`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Anchor {
    /// On the centred 4:3 screen.
    #[default]
    Centre,
    /// Pinned to its side of the window (`pin`).
    Sides,
    /// A wall or bar: its left edge kept from the window's left; it reaches the window's right edge if it ends at
    /// 640, else keeps its right edge from the left too.
    Span,
}

/// A 640-wide screen rect `x, w` placed by `a`: left and width in percent of the window.
pub(super) fn place(x: f32, w: f32, share: f32, a: Anchor) -> (f32, f32) {
    match a {
        Anchor::Span => {
            let right = if x + w >= 640.0 { 100.0 } else { (x + w) / 6.4 * share };
            (x / 6.4 * share, right - x / 6.4 * share)
        }
        Anchor::Sides => pin(x, w, share),
        Anchor::Centre => (50.0 * (1.0 - share) + x / 6.4 * share, w / 6.4 * share),
    }
}

/// Share of the window's height the 4:3 screen covers: all of it at 4:3 or wider. A taller window letterboxes it
/// (B232): the HUD keeps its shape, the 3D view fills the rest.
pub(super) fn vshare(aspect: f32) -> f32 {
    (aspect * 0.75).min(1.0)
}

/// A 448-high screen rect `y, h` placed by `a` on a window of `aspect`: top and height in percent of the window.
/// Same scale as across (`share`), so nothing stretches in a tall window: `Centre` rides the centred 4:3 screen,
/// `Sides`/`Span` keep to the window's top or bottom edge (by which half the rect's centre is in), and a full-height
/// rect (a wall, a dim) fills it.
pub(super) fn vplace(y: f32, h: f32, aspect: f32, a: Anchor) -> (f32, f32) {
    let s = vshare(aspect);
    let height = h / 4.48 * s;
    match a {
        Anchor::Centre => (50.0 * (1.0 - s) + y / 4.48 * s, height),
        _ if y <= 0.0 && y + h >= 448.0 => (0.0, 100.0),
        _ if y + h / 2.0 < 224.0 => (y / 4.48 * s, height),
        _ => (100.0 - (448.0 - y) / 4.48 * s, height),
    }
}

/// The primary window's view's width over height (4:3 without one).
pub(super) fn window_aspect(window: &Query<&Window, With<bevy::window::PrimaryWindow>>) -> f32 {
    window.single().map_or(4.0 / 3.0, aspect)
}

/// Pixels (logical) per 448-space unit: the 4:3 screen's height over 448.
pub(super) fn unit_px(w: &Window) -> f32 {
    let v = view_size(w);
    v.y.min(v.x * 0.75) / 448.0
}

/// `share` of the primary window's view (1 without one).
pub(super) fn window_share(window: &Query<&Window, With<bevy::window::PrimaryWindow>>) -> f32 {
    window.single().map_or(1.0, |w| share(aspect(w)))
}

fn fit(window: Query<&Window>, mut q: Query<&mut Node, With<Screen43>>) {
    let Ok(w) = window.single() else { return };
    let a = aspect(w);
    let (s, v) = (share(a), vshare(a));
    for mut n in &mut q {
        let (left, width) = (Val::Percent(50.0 * (1.0 - s)), Val::Percent(100.0 * s));
        let (top, height) = (Val::Percent(50.0 * (1.0 - v)), Val::Percent(100.0 * v));
        if n.left != left || n.width != width || n.top != top || n.height != height {
            (n.left, n.width, n.top, n.height) = (left, width, top, height);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, fit.before(bevy::ui::UiSystems::Layout));
}

#[test]
fn share_of_width() {
    assert_eq!(share(4.0 / 3.0), 1.0);
    assert_eq!(share(1.0), 1.0);
    assert!((share(16.0 / 9.0) - 0.75).abs() < 1e-6);
}

#[test]
fn pinned_to_edges() {
    // 4:3: the plain 640 screen
    assert_eq!(pin(16.0, 64.0, 1.0), (2.5, 10.0));
    assert_eq!(pin(560.0, 80.0, 1.0), (87.5, 12.5));
    // 16:9: left rects from the left edge, right rects end at the same distance from the right edge
    assert_eq!(pin(0.0, 64.0, 0.75), (0.0, 7.5));
    assert_eq!(pin(16.0, 64.0, 0.75), (1.875, 7.5));
    let (l, w) = pin(560.0, 80.0, 0.75);
    assert!((l + w - 100.0).abs() < 1e-4 && (w - 9.375).abs() < 1e-4);
}

#[test]
fn fixed_ratio_views() {
    let v = |w: f32, h: f32, a| view_at(Vec2::new(w, h), a);
    // Auto, and a portrait window: all of it
    assert_eq!(v(2400.0, 1080.0, 0), Rect::new(0.0, 0.0, 2400.0, 1080.0));
    assert_eq!(v(1080.0, 2400.0, 2), Rect::new(0.0, 0.0, 1080.0, 2400.0));
    // a 20:9 phone at 16:9: bars at the sides
    assert_eq!(v(2400.0, 1080.0, 2), Rect::new(240.0, 0.0, 2160.0, 1080.0));
    // 21:9 on a 16:9 window: bars top and bottom
    let r = v(1920.0, 1080.0, 4);
    assert!((r.width() - 1920.0).abs() < 1e-3 && (r.height() - 1920.0 * 9.0 / 21.0).abs() < 1e-3 && (r.center().y - 540.0).abs() < 1e-3);
    // 4:3 and 16:10 on 16:9
    assert_eq!(v(1920.0, 1080.0, 1), Rect::new(240.0, 0.0, 1680.0, 1080.0));
    assert_eq!(v(1920.0, 1080.0, 3), Rect::new(96.0, 0.0, 1824.0, 1080.0));
}

#[test]
fn placed_rects() {
    // 16:9: a full-width wall fills the window, a bar's left piece keeps to the left, its last piece reaches the right
    assert_eq!(place(0.0, 640.0, 0.75, Anchor::Span), (0.0, 100.0));
    assert_eq!(place(0.0, 512.0, 0.75, Anchor::Span), (0.0, 60.0));
    assert_eq!(place(512.0, 128.0, 0.75, Anchor::Span), (60.0, 40.0));
    // centred content stays on the 4:3 screen
    assert_eq!(place(320.0, 64.0, 0.75, Anchor::Centre), (50.0, 7.5));
}

#[test]
fn sideways_hud() {
    let in_view = |s, p| in_view_at(s, p, true);
    // a portrait view turns a quarter left: the HUD's top-left is the view's bottom-left, its bottom-left (1P's
    // panel) the view's bottom-right, so each player's panel is at their own end
    let (s, p) = in_view(Vec2::new(400.0, 800.0), Vec2::new(0.0, 800.0));
    assert_eq!((s, p), (Vec2::new(800.0, 400.0), Vec2::ZERO));
    assert_eq!(in_view(Vec2::new(400.0, 800.0), Vec2::new(400.0, 800.0)).1, Vec2::new(0.0, 400.0));
    // landscape doesn't
    assert_eq!(in_view(Vec2::new(800.0, 400.0), Vec2::new(10.0, 20.0)).1, Vec2::new(10.0, 20.0));
}

#[test]
fn tall_window_letterboxes() {
    // 3:4 (a phone upright): the 4:3 screen is 9/16 of the height, centred; nothing stretches
    let a = 3.0 / 4.0;
    assert!((vshare(a) - 0.5625).abs() < 1e-6);
    assert_eq!(vshare(16.0 / 9.0), 1.0);
    let (t, h) = vplace(0.0, 448.0, a, Anchor::Centre);
    assert!((t - 21.875).abs() < 1e-4 && (h - 56.25).abs() < 1e-4);
    // a wall fills, the top bar keeps to the top, the bottom bar to the bottom
    assert_eq!(vplace(0.0, 448.0, a, Anchor::Span), (0.0, 100.0));
    assert_eq!(vplace(0.0, 14.0, a, Anchor::Span).0, 0.0);
    let (t, h) = vplace(404.0, 32.0, a, Anchor::Span);
    assert!((t + h - (100.0 - 12.0 / 4.48 * 0.5625)).abs() < 1e-4);
    // wide: as before
    assert_eq!(vplace(44.8, 44.8, 16.0 / 9.0, Anchor::Sides), (10.0, 10.0));
}
