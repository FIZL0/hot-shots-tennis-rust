//! B221: mouse and touch on every menu screen. `layout` records a hit rect beside what it draws (`Draw::hit`); the
//! draw keeps the last frame's (`Hits`), and `pointer` turns the mouse and fingers into acts on them (`Pending`),
//! which `super::step` plays through `Menu::point` as the pointer's device (player 1's, else the keyboard's).
//!
//! Mouse: hovering a row or face moves the cursor there, a click chooses it (as ✕), the right button is Back, the
//! wheel steps up and down. Touch: a tap selects and chooses at once; a drag steps the cursor every `DRAG` px (lists
//! scroll with it). A tap or click outside a popup is its Back (`Draw::popup`). Hit rects are placed as their quads are
//! (`widescreen::place`), so they follow the art at any aspect; a press within `SLOP` of one still lands on it.

use std::sync::atomic::{AtomicBool, Ordering};

use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

use super::super::widescreen::{self, Anchor};
use super::{Dev, Effect, Menu, Press, Screen, Who, GRID};

/// What a hit rect does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Act {
    /// Row `i` of the screen's list: hovering selects it, a tap selects it and presses `k` (usually ✕).
    Row(usize, Press),
    /// A character select face: the disc grid's cell, or the custom page's mod.
    Cell(usize),
    /// A court tile or umpire card in the confirm screen's picker: a tap hovers it, a tap on the hovered one keeps it.
    Pick(usize),
    /// A button: a tap is this press.
    Press(Press),
    /// B238: a seat's ✕: a tap takes its device off it.
    Drop(usize),
}

/// ✕.
pub(super) const OK: Press = Press { up: false, down: false, left: false, right: false, ok: true, back: false, card: false, prev: false, next: false, page: false, hand: false };

pub(super) fn press(f: impl FnOnce(&mut Press)) -> Act {
    let mut k = Press::default();
    f(&mut k);
    Act::Press(k)
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Hit {
    pub rect: [f32; 4],
    pub anchor: Anchor,
    pub act: Act,
}

/// The last drawn screen's hit rects (later drawn on top) and what a tap outside them does.
#[derive(Resource, Default)]
pub(super) struct Hits(pub Vec<Hit>, pub Option<Act>);

/// Acts waiting for `super::step` (tapped or only hovered).
#[derive(Resource, Default)]
pub(in crate::play) struct Pending(pub Vec<(Act, bool)>);

/// How far (640×448 px) a press may miss a rect and still hit it: farther for a finger.
const SLOP: f32 = 4.0;
const SLOP_TOUCH: f32 = 12.0;
/// A drag's step (448-high px).
const DRAG: f32 = 40.0;

/// The act under `p` (fractions of the window, 0..1) on a window of aspect `aspect`: the rect it's in (the last drawn
/// on top), else the nearest within `slop`.
pub(super) fn hit_at(hits: &[Hit], p: Vec2, aspect: f32, slop: f32) -> Option<Act> {
    let share = widescreen::share(aspect);
    let mut best: Option<(f32, Act)> = None;
    for h in hits.iter().rev() {
        let [x, y, w, hh] = h.rect;
        let (left, width) = widescreen::place(x, w, share, h.anchor);
        let (top, height) = widescreen::vplace(y, hh, aspect, h.anchor);
        // to 640×448 units at the 4:3 screen's scale
        let (to_x, to_y) = (6.4 / share, 4.48 / widescreen::vshare(aspect));
        let (x0, x1) = (left * to_x, (left + width) * to_x);
        let (y0, y1) = (top * to_y, (top + height) * to_y);
        let (px, py) = (p.x * 100.0 * to_x, p.y * 100.0 * to_y);
        let dx = (x0 - px).max(px - x1).max(0.0);
        let dy = (y0 - py).max(py - y1).max(0.0);
        let d = dx.hypot(dy);
        if d == 0.0 {
            return Some(h.act);
        }
        if d <= slop && best.is_none_or(|b| d < b.0) {
            best = Some((d, h.act));
        }
    }
    best.map(|b| b.1)
}

impl Menu {
    /// Row `i` chosen by the pointer on this screen; whether the cursor moved.
    fn choose(&mut self, i: usize) -> bool {
        let was = match self.screen {
            Screen::Main if self.add.ask.is_some() => self.add.ask.replace(i),
            Screen::Mode => Some(std::mem::replace(&mut self.doubles, i == 1) as usize),
            Screen::Mods => Some(std::mem::replace(self.mods_cursor(), i)),
            Screen::Lobby => {
                if i != self.sel {
                    self.net.status.clear();
                }
                Some(std::mem::replace(&mut self.sel, i))
            }
            _ => Some(std::mem::replace(&mut self.sel, i)),
        };
        was != Some(i)
    }

    /// A pointer act from `dev`: hovered (`tap` false) or tapped / clicked.
    pub(super) fn point(&mut self, dev: Dev, act: Act, tap: bool) -> Vec<Effect> {
        let mut out = Vec::new();
        match act {
            Act::Press(k) if tap => out = self.step(dev, k),
            Act::Press(_) => {}
            Act::Row(i, k) => {
                // a row on a screen the press can't reach (the confirm screen's rows while another device drives)
                if self.choose(i) {
                    out.push(Effect::Sound(0));
                }
                if tap {
                    out.extend(self.step(dev, k));
                }
            }
            Act::Cell(i) => {
                let Some(p) = self.driven(dev) else { return out };
                let me = self.players[p];
                if me.ready {
                    return out;
                }
                // B217: the Random card
                if i == super::CHARS && !self.custom {
                    if !me.random {
                        self.players[p].random = true;
                        out.push(Effect::Sound(0));
                    }
                    if tap {
                        out.extend(self.step(dev, OK));
                    }
                    return out;
                }
                let (w, at) = if self.custom { (Who::Mod(i), me.pick) } else { (Who::Disc(GRID[i].0), me.cursor) };
                if at != i || me.modded != self.custom || me.random {
                    // B66: a face with every costume taken can't be picked
                    if self.next_free(p, w, 0, false).is_none() {
                        if tap {
                            out.push(Effect::Sound(1));
                        }
                        return out;
                    }
                    let me = &mut self.players[p];
                    if self.custom { me.pick = i } else { me.cursor = i }
                    me.random = false;
                    (me.modded, me.costume) = (self.custom, 0);
                    self.skip_taken();
                    out.push(Effect::Sound(0));
                }
                if tap {
                    out.extend(self.step(dev, OK));
                }
            }
            Act::Pick(v) if tap => {
                let court = self.sel == 1;
                if v == if court { self.court } else { self.umpire as usize } {
                    out = self.step(dev, OK);
                } else {
                    if court { self.court = v } else { self.umpire = v as u8 }
                    out.push(Effect::Sound(0));
                }
            }
            Act::Pick(_) => {}
            Act::Drop(p) if tap && self.drop_seat(p) => out.push(Effect::Sound(1)),
            Act::Drop(_) => {}
        }
        out
    }
}

/// The mouse pointer is hidden and ignored (hover and clicks) since a key or a pad button was pressed, until the mouse
/// moves or clicks again: a resting mouse never pulls the keyboard's or pad's cursor back to the row under it.
pub(in crate::play) static MOUSE_IDLE: AtomicBool = AtomicBool::new(false);

/// Keeps `MOUSE_IDLE` and the window's pointer visibility: any key or pad button hides it, mouse motion (more than
/// a resting mouse's jitter) or a click brings it back.
pub(in crate::play) fn mouse_idle(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let moved = motion.read().map(|m| m.delta).sum::<Vec2>().length() > 2.0 || mouse.get_just_pressed().next().is_some();
    let pressed = keys.get_just_pressed().next().is_some() || pads.iter().any(|g| g.get_just_pressed().next().is_some() || g.left_stick().length() > 0.5);
    let idle = if moved { false } else if pressed { true } else { MOUSE_IDLE.load(Ordering::Relaxed) };
    MOUSE_IDLE.store(idle, Ordering::Relaxed);
    for mut c in &mut cursor {
        if c.visible == idle {
            c.visible = !idle;
        }
    }
}

/// Where the mouse was, the finger down (its id, where it went down, where the last drag step was, whether it
/// dragged), the wheel's part-notches and when a finger last touched (the browser's mouse events copied from a touch
/// are dropped).
#[derive(Default)]
pub(super) struct State {
    mouse: Option<Vec2>,
    /// B233: the row or face the mouse is over: hovering selects only on moving onto a new one, so a resting mouse
    /// that reports a jitter (sub-pixel moves, a compositor re-sending it, a pad's touchpad) can't pull the pad's or
    /// keyboard's cursor back every frame.
    hovered: Option<Act>,
    finger: Option<(u64, Vec2, Vec2, bool)>,
    wheel: f32,
    touched: f32,
}

/// The mouse, fingers and wheel → `Pending`.
#[allow(clippy::too_many_arguments)]
pub(in crate::play) fn pointer(
    window: Query<&Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut wheel: MessageReader<MouseWheel>,
    hits: Res<Hits>,
    time: Res<Time>,
    mut pending: ResMut<Pending>,
    mut st: Local<State>,
) {
    let Ok(win) = window.single() else { return };
    #[allow(unused_mut)]
    let mut size = Vec2::new(win.width().max(1.0), win.height().max(1.0));
    // the browser's touches and mouse come in the page's CSS px, which the canvas (the whole page) needn't match: a
    // phone's backing store can be smaller than CSS × devicePixelRatio (seen at half in Chrome's phone emulation)
    #[cfg(target_arch = "wasm32")]
    if let Some(css) = crate::web::page_size() {
        size = css;
    }
    // B117: measured in the view (a fixed aspect ratio's centred rect)
    let off = widescreen::view(size).min;
    let size = widescreen::in_view(size, off).0;
    let aspect = size.x / size.y;
    let at = |p: Vec2, slop: f32| hit_at(&hits.0, (p - off) / size, aspect, slop);
    let mut out = |a: Option<Act>, tap: bool| pending.0.extend(a.map(|a| (a, tap)));
    let now = time.elapsed_secs();
    let hover = |a: Option<Act>| a.filter(|a| matches!(a, Act::Row(..) | Act::Cell(_)));
    // touch
    for t in touches.iter_just_pressed() {
        if st.finger.is_none() {
            st.finger = Some((t.id(), t.position(), t.position(), false));
            out(hover(at(t.position(), SLOP_TOUCH)), false);
        }
    }
    if let Some((id, start, last, dragged)) = st.finger.as_mut() {
        if let Some(t) = touches.get_pressed(*id) {
            let d = (t.position() - *last) / size * 448.0;
            let step = |v: f32| if v.abs() >= DRAG { v.signum() as i32 } else { 0 };
            let (sx, sy) = (step(d.x * size.x / size.y), step(d.y));
            if sx != 0 || sy != 0 {
                *dragged = true;
                *last = t.position();
                // the finger pulls the list: up moves the cursor down
                out(Some(press(|k| (k.down, k.up, k.right, k.left) = (sy < 0, sy > 0, sx < 0, sx > 0))), true);
            }
        } else if touches.iter_just_released().chain(touches.iter_just_canceled()).any(|t| t.id() == *id) {
            if !*dragged && touches.iter_just_released().any(|t| t.id() == *id) {
                let a = at(*start, SLOP_TOUCH);
                out(a.or(hits.1), true);
            }
            st.finger = None;
        } else {
            st.finger = None;
        }
    }
    if touches.iter().next().is_some() || touches.iter_just_released().next().is_some() {
        st.touched = now;
    }
    // mouse (not the browser's copy of a touch)
    if now - st.touched > 0.6 && MOUSE_IDLE.load(Ordering::Relaxed) {
        // hidden: still follow what's under it, so waking it (a compositor's stray motion as the pointer hides or shows)
        // doesn't pull the pad's or keyboard's cursor back to the row it rests on; only moving onto another row does
        if let Some(c) = win.cursor_position() {
            (st.mouse, st.hovered) = (Some(c), hover(at(c, SLOP)));
        }
    } else if now - st.touched > 0.6 {
        let c = win.cursor_position();
        // no position (the pointer left, or the compositor's leave / enter as it hides): what it was over stays, so
        // coming back to the same row doesn't select it again
        if c.is_some() && c != st.mouse {
            st.mouse = c;
            let a = c.and_then(|c| hover(at(c, SLOP)));
            if a != st.hovered {
                st.hovered = a;
                out(a, false);
            }
        }
        if mouse.just_pressed(MouseButton::Left)
            && let Some(c) = c
        {
            out(at(c, SLOP).or(hits.1), true);
        }
        if mouse.just_pressed(MouseButton::Right) {
            out(Some(press(|k| k.back = true)), true);
        }
    }
    for e in wheel.read() {
        st.wheel += if e.unit == MouseScrollUnit::Line { e.y } else { e.y / 40.0 };
    }
    while st.wheel.abs() >= 1.0 {
        let up = st.wheel > 0.0;
        st.wheel -= st.wheel.signum();
        out(Some(press(|k| (k.up, k.down) = (up, !up))), true);
    }
}

/// The pointer's device: player 1's while it's a connected pad, else the keyboard (which a touch-only phone also is).
pub(super) fn device(m: &Menu, pads: &[Entity]) -> Dev {
    match m.seats[0] {
        Some(Dev::Pad(e)) if pads.contains(&e) => Dev::Pad(e),
        _ => Dev::Keys,
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::{Bindings, Text, drawn};
    use super::*;

    fn text() -> Text {
        Text { msg: Vec::new(), glyphs: default(), grades: [[0; 5]; 14], style: [0; 14], hands: [1.0; 14], ranks: Vec::new(), mods: Vec::new() }
    }

    /// The screen's hits as drawn, and what a press outside them does.
    pub(in crate::play::main_menu) fn hits(m: &Menu) -> (Vec<Hit>, Option<Act>) {
        let t = text();
        let d = drawn(m, &t, &Bindings::default(), &[], 0.0);
        (d.hits, d.outside)
    }

    /// The act under the 640×448 point (x, y) on a 4:3 window.
    pub(in crate::play::main_menu) fn act_at(m: &Menu, x: f32, y: f32) -> Option<Act> {
        hit_at(&hits(m).0, Vec2::new(x / 640.0, y / 448.0), 4.0 / 3.0, SLOP)
    }

    /// A tap at (x, y), or outside: what `pointer` sends `step`.
    fn tap(m: &mut Menu, x: f32, y: f32) -> Vec<Effect> {
        let (h, outside) = hits(m);
        let a = hit_at(&h, Vec2::new(x / 640.0, y / 448.0), 4.0 / 3.0, SLOP_TOUCH).or(outside).unwrap_or_else(|| panic!("nothing at {x},{y} on {:?}", m.screen));
        m.point(Dev::Keys, a, true)
    }

    /// The task's test: main menu → character select → the match starting, by taps alone.
    #[test]
    fn taps_from_main_menu_to_match() {
        let mut m = Menu::default();
        tap(&mut m, 150.0, 69.0); // Local (B297: the left list)
        assert_eq!(m.screen, Screen::Mode);
        tap(&mut m, 320.0, 240.0); // Doubles' bar: changes the mode and goes on
        assert_eq!((m.screen, m.doubles), (Screen::Assign, true));
        tap(&mut m, 20.0, 10.0); // the title tab: Back
        assert_eq!(m.screen, Screen::Mode);
        tap(&mut m, 320.0, 170.0); // Singles
        assert_eq!((m.screen, m.doubles), (Screen::Assign, false));
        tap(&mut m, 468.0, 188.0); // B216: 2P's + COM
        assert!(m.bots[1]);
        tap(&mut m, 320.0, 118.0); // 1P's card
        assert_eq!(m.screen, Screen::Chars);
        // a face: 1P moves there and is ready; then the face for the computer's pick
        let (_, x, y) = GRID[5];
        tap(&mut m, x, y);
        assert!(m.players[0].ready && m.players[0].cursor == 5);
        let (_, x, y) = GRID[9];
        tap(&mut m, x + 20.0, y - 20.0);
        assert!(m.players[1].ready && m.players[1].cursor == 9);
        assert_eq!(m.screen, Screen::Confirm);
        // the court picker: a tile hovers it, a tap off puts it back, the centre keeps it
        tap(&mut m, 120.0, 340.0); // Select Court
        assert_eq!(m.picking, Some(m.court));
        let was = m.court;
        tap(&mut m, 320.0 + 112.0, 366.0);
        assert_ne!(m.court, was);
        tap(&mut m, 320.0, 100.0);
        assert_eq!((m.picking, m.court), (None, was));
        tap(&mut m, 120.0, 340.0);
        tap(&mut m, 320.0 + 112.0, 366.0);
        tap(&mut m, 320.0, 366.0);
        assert!(m.picking.is_none() && m.court != was);
        let fx = tap(&mut m, 190.0, 305.0); // Start the match!
        assert!(fx.contains(&Effect::Launch), "{fx:?}");
    }

    /// The mouse: hovering moves the cursor (no press), the click then chooses; graphics values step both ways.
    #[test]
    fn hover_then_click_and_arrows() {
        let mut m = Menu::default();
        // B297: the list's Settings pill (Local, Online, Mods, Settings from y 50, 46 apart, x 64..264 at 4:3)
        let a = act_at(&m, 150.0, 50.0 + 46.0 * 3.0 + 19.0).unwrap();
        assert_eq!(m.point(Dev::Keys, a, false), [Effect::Sound(0)]);
        assert_eq!((m.screen, m.sel), (Screen::Main, 3));
        m.point(Dev::Keys, a, true);
        assert_eq!(m.screen, Screen::Settings);
        // Settings → Graphics → its first value's arrows
        use super::super::options::{Opt, PITCH, TOP};
        tap(&mut m, 270.0, TOP + PITCH + 19.0);
        assert_eq!(m.screen, Screen::Graphics);
        let v = m.settings.value(Opt::Msaa);
        let y = TOP + PITCH * 3.0 + 19.0;
        tap(&mut m, 600.0, y);
        assert_eq!(m.sel, 3);
        assert_ne!(m.settings.value(Opt::Msaa), v);
        tap(&mut m, 440.0, y);
        assert_eq!(m.settings.value(Opt::Msaa), v);
        // Controls: a row starts a rebind
        let mut c = Menu { screen: Screen::Controls, ..default() };
        tap(&mut c, 200.0, 62.0 + 21.0 * 2.0 + 10.0);
        assert_eq!(c.rebinding, Some(2));
    }

    /// Hit rects follow their quads' anchors: on 16:9 the 2P card (x 480..624, Sides) is pinned 16 px off the window's
    /// right edge, the grid (Centre) stays on the middle 4:3.
    #[test]
    fn wide_window_hits() {
        let m = Menu { screen: Screen::Chars, seats: [Some(Dev::Keys), None, None, None], ..default() };
        let h = hits(&m).0;
        let at = |x: f32, y: f32| hit_at(&h, Vec2::new(x, y / 448.0), 16.0 / 9.0, SLOP);
        let right = 1.0 - 16.0 / 640.0 * 0.75;
        assert_eq!(at(right - 0.01, 150.0), Some(press(|k| k.ok = true)));
        assert_eq!(at(right + 0.01, 150.0), None);
        let (_, x, y) = GRID[0];
        assert_eq!(at(0.5 + (x - 320.0) / 640.0 * 0.75, y), Some(Act::Cell(0)));
    }

    /// Popups: a tap off the mods' Reload / Later popup is Later; the lobby's topics close on Cancel.
    #[test]
    fn taps_outside_popups() {
        let mut m = Menu::default();
        m.add.ask = Some(0);
        assert_eq!(hits(&m).1, Some(Act::Row(1, OK)));
        let fx = tap(&mut m, 40.0, 400.0);
        assert_eq!((m.add.ask, m.screen), (None, Screen::Main));
        assert_eq!(fx.last(), Some(&Effect::Sound(1)));
    }

    /// The system itself: a window, a finger down and up on a row, a drag, a click off everything.
    #[test]
    fn touches_and_clicks_reach_pending() {
        use bevy::input::touch::{TouchInput, TouchPhase};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::input::InputPlugin));
        app.init_resource::<Pending>().add_systems(Update, pointer);
        let win = app.world_mut().spawn((Window { resolution: (800, 448).into(), ..default() }, PrimaryWindow)).id();
        // one row in the middle of the 4:3 screen; a press outside is Back
        app.insert_resource(Hits(vec![Hit { rect: [220.0, 100.0, 200.0, 40.0], anchor: Anchor::Centre, act: Act::Row(3, OK) }], Some(press(|k| k.back = true))));
        app.update();
        let touch = |app: &mut App, phase, x: f32, y: f32| {
            app.world_mut().write_message(TouchInput { phase, position: Vec2::new(x, y), window: win, force: None, id: 7 });
            app.update();
            std::mem::take(&mut app.world_mut().resource_mut::<Pending>().0)
        };
        // 4:3 screen 597 px wide, 101 px in: the row's middle at x 400, y 120
        assert_eq!(touch(&mut app, TouchPhase::Started, 400.0, 120.0), [(Act::Row(3, OK), false)]);
        assert_eq!(touch(&mut app, TouchPhase::Ended, 400.0, 120.0), [(Act::Row(3, OK), true)]);
        assert_eq!(touch(&mut app, TouchPhase::Started, 400.0, 300.0), []);
        assert_eq!(touch(&mut app, TouchPhase::Ended, 400.0, 300.0), [(press(|k| k.back = true), true)]);
        // a drag up 50 px: one step down, no tap
        touch(&mut app, TouchPhase::Started, 400.0, 300.0);
        assert_eq!(touch(&mut app, TouchPhase::Moved, 400.0, 250.0), [(press(|k| k.down = true), true)]);
        assert_eq!(touch(&mut app, TouchPhase::Ended, 400.0, 250.0), []);
        // a finger down and up within one frame still taps
        app.world_mut().write_message(TouchInput { phase: TouchPhase::Started, position: Vec2::new(400.0, 120.0), window: win, force: None, id: 8 });
        app.world_mut().write_message(TouchInput { phase: TouchPhase::Ended, position: Vec2::new(400.0, 120.0), window: win, force: None, id: 8 });
        app.update();
        assert!(app.world().resource::<Pending>().0.contains(&(Act::Row(3, OK), true)), "{:?}", app.world().resource::<Pending>().0);
    }

    /// B233: a resting mouse that jitters (sub-pixel moves every frame) over Copy Link, or over E on the name
    /// entry, doesn't pull the pad's cursor back: → from Copy Link stays on Leave while the code fills in, and the
    /// pad walks the whole entry grid.
    #[test]
    fn resting_mouse_keeps_the_pads_cursor() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::input::InputPlugin));
        app.init_resource::<Pending>().init_resource::<Hits>().add_systems(Update, pointer);
        // the mouse counts 0.6 s after the last touch: past that from the first frame
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(250)));
        (0..4).for_each(|_| app.update());
        // 4:3, so a 640×448 point (x, y) is at (x, y × 480 / 448)
        let win = app.world_mut().spawn((Window { resolution: (640, 480).into(), ..default() }, PrimaryWindow)).id();
        // one frame with the mouse at `at` (+ a jitter) over `m`'s hits, its acts played as the menu's step does
        let frame = |app: &mut App, m: &mut Menu, at: Vec2, n: usize| {
            let (h, outside) = hits(m);
            app.insert_resource(Hits(h, outside));
            let p = Vec2::new(at.x + 0.25 * (n % 2) as f32, (at.y + 0.25 * (n % 3) as f32) * 480.0 / 448.0);
            app.world_mut().get_mut::<Window>(win).unwrap().set_cursor_position(Some(p));
            app.update();
            for (a, tap) in std::mem::take(&mut app.world_mut().resource_mut::<Pending>().0) {
                m.point(Dev::Keys, a, tap);
            }
        };
        let centre = |m: &Menu, i: usize| {
            let r = hits(m).0.into_iter().find(|h| h.act == Act::Row(i, OK)).unwrap().rect;
            Vec2::new(r[0] + r[2] / 2.0, r[1] + r[3] / 2.0)
        };
        let right = Press { right: true, ..default() };
        // the host's lobby: the mouse moved onto Copy Link selects it, then the pad's → to Leave holds
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        super::super::online::sample(&mut m);
        (m.net.hosting, m.sel) = (true, 0);
        // seats 1–3, Start Characters Court, Copy Link (6) Leave (7)
        let copy = 6;
        let at = centre(&m, copy);
        frame(&mut app, &mut m, at, 0);
        assert_eq!(m.sel, copy, "hovering selects");
        m.step(Dev::Keys, right);
        assert_eq!(m.sel, copy + 1);
        for n in 1..30 {
            if n == 10 {
                m.net.code = "filled in".into();
            }
            frame(&mut app, &mut m, at, n);
            assert_eq!(m.sel, copy + 1, "frame {n}");
        }
        // the name entry with the mouse resting on E: the pad walks every key and button
        let mut m = Menu { screen: Screen::Code, sel: 4, net: super::super::online::View { naming: true, ..default() }, ..default() };
        let at = centre(&m, 4);
        // the name's 40 keys, then Delete Clear Save Back
        let keys = 40;
        let mut seen = std::collections::BTreeSet::new();
        for n in 0..(keys + 4) * 2 {
            frame(&mut app, &mut m, at, n);
            seen.insert(m.sel);
            let row_end = if m.sel < keys { m.sel % 8 == 7 } else { m.sel == keys + 3 };
            // → along the row (it wraps), then ↓ at its end
            m.step(Dev::Keys, right);
            if row_end {
                m.step(Dev::Keys, Press { down: true, ..default() });
            }
        }
        assert_eq!(seen, (0..keys + 4).collect(), "every key and button reached");
        // the host's lobby, the mouse resting on 2P's seat: ↑ hides it (a key press) and reaches Singles / Doubles; the
        // compositor's leave / enter and a stray motion waking it don't pull the cursor back to the seat
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        super::super::online::sample(&mut m);
        (m.net.hosting, m.sel) = (true, 0);
        let at = centre(&m, 0);
        frame(&mut app, &mut m, at, 0);
        assert_eq!(m.sel, 0);
        MOUSE_IDLE.store(true, Ordering::Relaxed);
        m.step(Dev::Keys, Press { up: true, ..default() });
        let mode = m.sel;
        assert_ne!(mode, 0);
        for n in 1..4 {
            frame(&mut app, &mut m, at, n);
        }
        MOUSE_IDLE.store(false, Ordering::Relaxed);
        app.world_mut().get_mut::<Window>(win).unwrap().set_cursor_position(None);
        app.update();
        for n in 4..10 {
            frame(&mut app, &mut m, at, n);
            assert_eq!(m.sel, mode, "frame {n}");
        }
        // moving onto another row still hovers it
        let other = centre(&m, 1);
        frame(&mut app, &mut m, other, 0);
        assert_eq!(m.sel, 1);
    }

}
