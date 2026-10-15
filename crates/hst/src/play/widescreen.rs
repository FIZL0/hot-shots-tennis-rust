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
//! B314: in the browser the window is the canvas, which starts below the screen's top safe area (an iPhone's Dynamic
//! Island, `web/index.html`), so the view here is already the shorter rect under it: HUD and menus never reach it.
//!
//! M106: the native phone apps (iOS, Android) draw edge to edge, so there the view is fitted inside the window's safe
//! area (`SAFE`, read by `follow_safe_area`): at Auto it is the whole safe rect, a fixed ratio is the largest rect of it
//! centred in the safe rect; the notch, cutout and home-indicator strips stay black like the bars. The touch pad keeps
//! to the whole window (`touch_pad`), so neither the safe area nor the Aspect Ratio moves it. `HST_SAFE=l,t,r,b`
//! (logical px) fakes the insets anywhere, for `--shot`s.
//!
//! B421: there the Aspect Ratio also offers Stretch (`STRETCH`, `aspect_choices`): the view is the whole window, notch,
//! cutout and home-indicator strips included, so the picture fills the screen (HUD and menus may reach under the island
//! then). Nothing is distorted: the 3D view widens to it and the HUD keeps its 4:3 screen, as at Auto. The touch pad
//! still keeps its buttons clear of the safe area.

use bevy::prelude::*;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

/// The Aspect Ratio setting's choices (index in `ASPECT`): Auto fills the window (B17's widescreen).
/// Stretch (B421) is the phone apps' only (`aspect_choices`).
pub const ASPECTS: [&str; 6] = ["Auto", "4:3", "16:9", "16:10", "21:9", "Stretch"];
const RATIOS: [f32; 5] = [0.0, 4.0 / 3.0, 16.0 / 9.0, 16.0 / 10.0, 21.0 / 9.0];
/// B421: the `ASPECTS` index of Stretch: the whole window, the safe area ignored.
pub const STRETCH: u8 = 5;

/// How many of `ASPECTS` the setting offers: Stretch only where there is a safe area to ignore (the phone apps, or
/// `HST_SAFE`); elsewhere Auto already fills the window.
pub fn aspect_choices() -> usize {
    if cfg!(any(target_os = "ios", target_os = "android")) || fake_safe().is_some() { ASPECTS.len() } else { STRETCH as usize }
}
/// The aspect ratio in force (an `ASPECTS` index): the settings' (`main_menu`), 4:3 in the original mode.
pub static ASPECT: AtomicU8 = AtomicU8::new(0);

/// The centred rect of a `size` window the game draws into: all of it at Auto, else the largest rect of the fixed
/// ratio. A portrait window (a phone held upright) keeps all of it, for its own layout (B112).
pub fn view(size: Vec2) -> Rect {
    view_at(size, ASPECT.load(Ordering::Relaxed))
}

fn view_at(size: Vec2, aspect: u8) -> Rect {
    view_in(size, aspect, SAFE.each_ref().map(|a| f32::from_bits(a.load(Ordering::Relaxed))))
}

/// `view_at` with the safe-area insets `safe` (left, top, right, bottom, as shares of the window's width and height).
fn view_in(size: Vec2, aspect: u8, [l, t, r, b]: [f32; 4]) -> Rect {
    if aspect == STRETCH {
        return Rect::from_corners(Vec2::ZERO, size);
    }
    let safe = Rect::from_corners(Vec2::new(l * size.x, t * size.y), Vec2::new(size.x * (1.0 - r), size.y * (1.0 - b)));
    let ratio = RATIOS.get(aspect as usize).copied().unwrap_or(0.0);
    let s = safe.size();
    if ratio == 0.0 || size.x < size.y || s.x <= 0.0 || s.y <= 0.0 {
        return if s.x > 0.0 && s.y > 0.0 { safe } else { Rect::from_corners(Vec2::ZERO, size) };
    }
    let v = if s.x > s.y * ratio { Vec2::new(s.y * ratio, s.y) } else { Vec2::new(s.x, s.x / ratio) };
    Rect::from_center_size(safe.center(), v)
}

/// M106: the window's safe-area insets (left, top, right, bottom) as shares of its width and height, f32 bits: zero
/// but in the native phone apps (or with `HST_SAFE`), whose windows reach under the notch and the home indicator.
static SAFE: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];

/// The safe-area insets in logical px of a `size` window (the touch pad's layout keeps clear of them too).
pub(crate) fn safe_insets(size: Vec2) -> [f32; 4] {
    let s = SAFE.each_ref().map(|a| f32::from_bits(a.load(Ordering::Relaxed)));
    [s[0] * size.x, s[1] * size.y, s[2] * size.x, s[3] * size.y]
}

/// `HST_SAFE=l,t,r,b`: fixed insets in logical px, instead of the phone's.
fn fake_safe() -> Option<[f32; 4]> {
    static FAKE: std::sync::OnceLock<Option<[f32; 4]>> = std::sync::OnceLock::new();
    *FAKE.get_or_init(|| {
        let v: Vec<f32> = std::env::var("HST_SAFE").ok()?.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        <[f32; 4]>::try_from(v).ok()
    })
}

/// Keeps `SAFE` to the window's safe area: the phone's own (twice a second, it changes as the phone turns), or
/// `HST_SAFE`'s.
fn follow_safe_area(window: Query<(&Window, Option<&bevy::window::RawHandleWrapper>), With<bevy::window::PrimaryWindow>>, mut frame: Local<u32>) {
    let Ok((w, handle)) = window.single() else { return };
    *frame += 1;
    if *frame % 30 != 1 {
        return;
    }
    let _ = handle;
    #[allow(unused_mut)]
    let mut px = fake_safe();
    #[cfg(target_os = "ios")]
    if px.is_none()
        && let Some(h) = handle
    {
        px = Some(crate::ios::safe_area(h));
    }
    #[cfg(target_os = "android")]
    if px.is_none() {
        px = crate::android::safe_area().map(|p| p.map(|v| v / w.scale_factor()));
    }
    let Some([l, t, r, b]) = px else { return };
    let (ww, wh) = (w.width().max(1.0), w.height().max(1.0));
    let shares = [l / ww, t / wh, r / ww, b / wh].map(|v| v.clamp(0.0, 0.45));
    let mut changed = false;
    for (a, v) in SAFE.iter().zip(shares) {
        changed |= a.swap(v.to_bits(), Ordering::Relaxed) != v.to_bits();
    }
    if changed {
        info!("safe area {:?} in a {}×{} window", [l, t, r, b], w.width(), w.height());
    }
}

/// The HUD's view size (logical px) in `w`.
pub fn view_size(w: &Window) -> Vec2 {
    view(Vec2::new(w.width(), w.height())).size()
}

/// The HUD's view width over height.
pub fn aspect(w: &Window) -> f32 {
    let v = view_size(w);
    v.x / v.y.max(1.0)
}

/// The 3D picture's width over height (the view's).
pub fn scene_aspect(w: &Window) -> f32 {
    aspect(w)
}

/// A point in a `size` window (or page) → the HUD's view size and the point in it.
pub fn in_view(size: Vec2, p: Vec2) -> (Vec2, Vec2) {
    let v = view(size);
    (v.size().max(Vec2::ONE), p - v.min)
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

/// Settings › Display › Menu Size (`main_menu`, f32 bits): a factor on the menus' uniform scale, so their whole
/// layout shrinks or grows together on the view (the extra room goes around it); the match's HUD doesn't take it.
pub static MENU_SCALE: AtomicU32 = AtomicU32::new(0x3f80_0000);

/// The Menu Size factor in force.
pub(super) fn menu_scale() -> f32 {
    f32::from_bits(MENU_SCALE.load(Ordering::Relaxed))
}

/// The menus' `share` and `vshare` on a view `aspect` wide at Menu Size `k`: the 640×448 screen's share of the view's
/// width and height (over 1 when it's bigger than the view).
pub(super) fn menu_shares(aspect: f32, k: f32) -> (f32, f32) {
    (share(aspect) * k, vshare(aspect) * k)
}

/// The menus' `menu_shares` on the primary window's view at the Menu Size in force ((1, 1) without one).
pub(super) fn window_menu_shares(window: &Query<&Window, With<bevy::window::PrimaryWindow>>) -> (f32, f32) {
    window.single().map_or((1.0, 1.0), |w| menu_shares(aspect(w), menu_scale()))
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
    vplace_in(y, h, vshare(aspect), a)
}

/// `vplace` with the 448-high screen covering `s` of the view's height (the menus' `menu_shares`).
pub(super) fn vplace_in(y: f32, h: f32, s: f32, a: Anchor) -> (f32, f32) {
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
    if cfg!(any(target_os = "ios", target_os = "android")) || fake_safe().is_some() {
        app.add_systems(First, follow_safe_area);
    }
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
fn views_in_the_safe_area() {
    // M106: an iPhone 15 Pro on its side, Dynamic Island left, home indicator below (852×393 pt, 59/0/59/21)
    let (size, safe) = (Vec2::new(852.0, 393.0), [59.0 / 852.0, 0.0, 59.0 / 852.0, 21.0 / 393.0]);
    let close = |a: Rect, b: Rect| (a.min - b.min).abs().max_element() < 1e-3 && (a.max - b.max).abs().max_element() < 1e-3;
    // Auto: the whole safe rect
    assert!(close(view_in(size, 0, safe), Rect::new(59.0, 0.0, 793.0, 372.0)));
    // 16:9 and 4:3: the largest rect of the ratio, centred in the safe rect, never past it
    for a in [1, 2, 3, 4] {
        let v = view_in(size, a, safe);
        assert!(v.min.x >= 59.0 - 1e-3 && v.max.x <= 793.0 + 1e-3 && v.min.y >= -1e-3 && v.max.y <= 372.0 + 1e-3, "{a}");
        assert!((v.width() / v.height() - RATIOS[a as usize]).abs() < 1e-3 && (v.center() - Vec2::new(426.0, 186.0)).length() < 1e-3, "{a}");
    }
    // an Android phone's cutout on one side only: the view starts past it
    let v = view_in(Vec2::new(2400.0, 1080.0), 0, [0.04, 0.0, 0.0, 0.0]);
    assert_eq!(v, Rect::new(96.0, 0.0, 2400.0, 1080.0));
    // no insets: as before
    assert_eq!(view_in(size, 0, [0.0; 4]), Rect::new(0.0, 0.0, 852.0, 393.0));
    // B421: Stretch fills the whole window, notch and home indicator included, upright too
    assert_eq!(view_in(size, STRETCH, safe), Rect::new(0.0, 0.0, 852.0, 393.0));
    assert_eq!(view_in(Vec2::new(393.0, 852.0), STRETCH, [0.0, 59.0 / 852.0, 0.0, 34.0 / 852.0]), Rect::new(0.0, 0.0, 393.0, 852.0));
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

#[test]
fn legend_strip_under_the_description_bar() {
    // B325: the menus' description bar (404..436) and the legend strip (436..448) stay stacked at any aspect
    for a in [0.4, 9.0 / 16.0, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
        let (bt, bh) = vplace(404.0, 32.0, a, Anchor::Span);
        let (lt, lh) = vplace(436.0, 12.0, a, Anchor::Span);
        assert!((bt + bh - lt).abs() < 1e-3 && (lt + lh - 100.0).abs() < 1e-3, "{a}");
    }
}

/// Places a HUD quad (percent of the view); only a real change marks the node, since a changed `Node` relayouts the
/// whole UI tree (B328: every frame otherwise).
pub(crate) fn put(node: &mut Mut<Node>, left: f32, top: f32, width: f32, height: f32) {
    let placed = Node { left: Val::Percent(left), top: Val::Percent(top), width: Val::Percent(width), height: Val::Percent(height), ..(**node).clone() };
    node.set_if_neq(placed);
}
