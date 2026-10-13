//! Widescreen: the 3D view widens with the window (`camera` keeps the game's vertical field of view), while the HUD
//! stays on the PS2's 4:3 screen (its 640×448 space), centred, so its art keeps its shape. The score panel is the
//! exception: its halves are pinned to the window's edges (`pin`) so they don't float inside the 4:3 screen, and so
//! are the menus' (`place`): walls and bars span the window, side cards keep to its edges, the rest stays centred.
//! A window taller than 4:3 letterboxes the 4:3 screen top and bottom instead (`vplace`, B232): one scale both ways.

use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// Widescreen on (the default); off (`--4x3`, the settings menu) the window shows only the centre 4:3 picture, black
/// either side, with the score panel pinned to the 4:3 screen's edges. The 3D view keeps its vertical field of view
/// whatever the width, so its centre 4:3 is exactly the 4:3 picture.
pub static WIDE: AtomicBool = AtomicBool::new(true);

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
    pin_in(x, w, share, WIDE.load(Ordering::Relaxed))
}

fn pin_in(x: f32, w: f32, share: f32, wide: bool) -> (f32, f32) {
    if !wide {
        return (50.0 * (1.0 - share) + x / 6.4 * share, w / 6.4 * share);
    }
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

/// A 640-wide screen rect `x, w` placed by `a`: left and width in percent of the window. Widescreen off, all of them
/// sit on the centred 4:3 screen.
pub(super) fn place(x: f32, w: f32, share: f32, a: Anchor) -> (f32, f32) {
    place_in(x, w, share, a, WIDE.load(Ordering::Relaxed))
}

fn place_in(x: f32, w: f32, share: f32, a: Anchor, wide: bool) -> (f32, f32) {
    match a {
        Anchor::Span if wide => {
            let right = if x + w >= 640.0 { 100.0 } else { (x + w) / 6.4 * share };
            (x / 6.4 * share, right - x / 6.4 * share)
        }
        Anchor::Sides => pin_in(x, w, share, wide),
        _ => pin_in(x, w, share, false),
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
/// rect (a wall, a dim) fills it. Widescreen off, all of them sit on the centred 4:3 screen.
pub(super) fn vplace(y: f32, h: f32, aspect: f32, a: Anchor) -> (f32, f32) {
    vplace_in(y, h, aspect, a, WIDE.load(Ordering::Relaxed))
}

fn vplace_in(y: f32, h: f32, aspect: f32, a: Anchor, wide: bool) -> (f32, f32) {
    let s = vshare(aspect);
    let height = h / 4.48 * s;
    match a {
        _ if !wide => (50.0 * (1.0 - s) + y / 4.48 * s, height),
        Anchor::Centre => (50.0 * (1.0 - s) + y / 4.48 * s, height),
        _ if y <= 0.0 && y + h >= 448.0 => (0.0, 100.0),
        _ if y + h / 2.0 < 224.0 => (y / 4.48 * s, height),
        _ => (100.0 - (448.0 - y) / 4.48 * s, height),
    }
}

/// The primary window's width over height (4:3 without one).
pub(super) fn window_aspect(window: &Query<&Window, With<bevy::window::PrimaryWindow>>) -> f32 {
    window.single().map_or(4.0 / 3.0, |w| w.width() / w.height().max(1.0))
}

/// Pixels (logical) per 448-space unit: the 4:3 screen's height over 448.
pub(super) fn unit_px(w: &Window) -> f32 {
    w.height().min(w.width() * 0.75) / 448.0
}

/// `share` of the primary window (1 without one).
pub(super) fn window_share(window: &Query<&Window, With<bevy::window::PrimaryWindow>>) -> f32 {
    window.single().map_or(1.0, |w| share(w.width() / w.height().max(1.0)))
}

fn fit(window: Query<&Window>, mut q: Query<&mut Node, With<Screen43>>) {
    let Ok(w) = window.single() else { return };
    let a = w.width() / w.height().max(1.0);
    let (s, v) = (share(a), vshare(a));
    for mut n in &mut q {
        let (left, width) = (Val::Percent(50.0 * (1.0 - s)), Val::Percent(100.0 * s));
        let (top, height) = (Val::Percent(50.0 * (1.0 - v)), Val::Percent(100.0 * v));
        if n.left != left || n.width != width || n.top != top || n.height != height {
            (n.left, n.width, n.top, n.height) = (left, width, top, height);
        }
    }
}

/// The black bars either side of the 4:3 picture with widescreen off.
#[derive(Component)]
struct Bar(bool);

fn bars(mut commands: Commands, window: Query<&Window>, mut q: Query<(&Bar, &mut Node)>, mut spawned: Local<bool>) {
    let wide = WIDE.load(Ordering::Relaxed);
    if wide && !*spawned {
        return;
    }
    if !std::mem::replace(&mut *spawned, true) {
        for right in [false, true] {
            commands.spawn((Bar(right), Node { position_type: PositionType::Absolute, height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::BLACK), GlobalZIndex(i32::MAX)));
        }
    }
    let Ok(w) = window.single() else { return };
    // turned back on mid-match (the pause menu's settings): no bars
    let side = Val::Percent(if wide { 0.0 } else { 50.0 * (1.0 - share(w.width() / w.height().max(1.0))) });
    for (Bar(right), mut n) in &mut q {
        n.width = side;
        if *right {
            n.right = Val::Px(0.0);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, (fit, bars).before(bevy::ui::UiSystems::Layout));
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
fn pinned_inside_43_when_narrow() {
    let (l, w) = pin_in(560.0, 80.0, 0.75, false);
    assert!((l - (12.5 + 87.5 * 0.75)).abs() < 1e-4 && (w - 9.375).abs() < 1e-4);
}

#[test]
fn placed_rects() {
    // 16:9: a full-width wall fills the window, a bar's left piece keeps to the left, its last piece reaches the right
    assert_eq!(place_in(0.0, 640.0, 0.75, Anchor::Span, true), (0.0, 100.0));
    assert_eq!(place_in(0.0, 512.0, 0.75, Anchor::Span, true), (0.0, 60.0));
    assert_eq!(place_in(512.0, 128.0, 0.75, Anchor::Span, true), (60.0, 40.0));
    // centred content stays on the 4:3 screen
    assert_eq!(place_in(320.0, 64.0, 0.75, Anchor::Centre, true), (50.0, 7.5));
    // widescreen off: everything on the 4:3 screen
    assert_eq!(place_in(0.0, 640.0, 0.75, Anchor::Span, false), (12.5, 75.0));
}

#[test]
fn tall_window_letterboxes() {
    // 3:4 (a phone upright): the 4:3 screen is 9/16 of the height, centred; nothing stretches
    let a = 3.0 / 4.0;
    assert!((vshare(a) - 0.5625).abs() < 1e-6);
    assert_eq!(vshare(16.0 / 9.0), 1.0);
    let (t, h) = vplace_in(0.0, 448.0, a, Anchor::Centre, true);
    assert!((t - 21.875).abs() < 1e-4 && (h - 56.25).abs() < 1e-4);
    // a wall fills, the top bar keeps to the top, the bottom bar to the bottom
    assert_eq!(vplace_in(0.0, 448.0, a, Anchor::Span, true), (0.0, 100.0));
    assert_eq!(vplace_in(0.0, 14.0, a, Anchor::Span, true).0, 0.0);
    let (t, h) = vplace_in(404.0, 32.0, a, Anchor::Span, true);
    assert!((t + h - (100.0 - 12.0 / 4.48 * 0.5625)).abs() < 1e-4);
    // wide: as before
    assert_eq!(vplace_in(44.8, 44.8, 16.0 / 9.0, Anchor::Sides, true), (10.0, 10.0));
}
