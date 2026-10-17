//! P21a: on the controller assignment (the menu's and the pause menu's, B254) a pad's seat card shakes while that pad
//! moves its right stick, as the original's controller select does with its selector, so a player can tell which
//! controller is which before swapping seats. The menus never read the right stick there, so nothing clashes. Only
//! the card's drawing moves (`Menu::shake`, applied in the Assign layout): no input, nothing of the match's.
//!
//! B564: any button on a seated controller (any key for the keyboard's seat) flashes its card for `GLOW` seconds
//! (`Menu::flash`, drawn by the Assign layout: a gold glow ring and a "this is you" bubble) and rumbles that pad. It
//! only watches: the press still does what it does on the screen, the flash itself moves no focus.
//!
//! A controller (or computer player) that changes seats slides its chip over from the seat it left for `SLIDE` seconds
//! (`Menu::slide`), so a trade reads as one, not a jump.
//!
//! `HST_SHAKE=<seat>` (for `--shot`s): that seat's card is drawn mid-shake; `HST_FLASH=<seat>`: mid-flash.

use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;

use super::{Dev, Menu, Screen};
use crate::play::controls;

/// How far the right stick goes (0..1) before its card shakes.
const PUSH: f32 = 0.5;
/// How long (seconds) a card keeps shaking after its stick is let go.
const LINGER: f32 = 0.3;
/// The shake's widest swing (640×448 units) and its rate (swings per second).
const SWING: f32 = 6.0;
const RATE: f32 = 14.0;

/// How long (seconds) a press flashes its card, and the pad's rumble on it (seconds, strength 0..1).
const GLOW: f32 = 1.0;
const BUZZ: f32 = 0.2;
const BUZZ_STRENGTH: f32 = 0.5;

/// How long (seconds) a moved controller's chip slides to its new seat.
const SLIDE: f32 = 0.3;

/// When each seat's card stops shaking (`Time::elapsed_secs`).
#[derive(Resource, Default)]
struct Shakes([f32; 4]);

/// When each seat's card stops flashing (`Time::elapsed_secs`).
#[derive(Resource, Default)]
struct Flashes([f32; 4]);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Shakes>().init_resource::<Flashes>().add_systems(Update, (shake, flash, slide.after(super::step), columns.after(super::step)).before(super::draw));
}

/// B564: a press on a seated device restarts its card's flash (and rumbles a pad, once per flash: holding a menu's
/// arrows down doesn't keep it buzzing); `Menu::flash` fades from 1 to 0 over `GLOW`.
fn flash(
    mut menu: ResMut<Menu>,
    mut flashes: ResMut<Flashes>,
    pads: Query<(Entity, &Gamepad)>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    time: Res<Time>,
    mut rumble: MessageWriter<GamepadRumbleRequest>,
    mut shot: Local<Option<Option<usize>>>,
    mut seated: Local<Option<[Option<Dev>; 4]>>,
) {
    let now = time.elapsed_secs();
    if menu.screen != Screen::Assign {
        flashes.0 = [0.0; 4];
        *seated = None;
    } else {
        // a device that just joined (wasn't on any seat) flashes its new card too
        if let Some(was) = *seated {
            for p in 0..4 {
                if let Some(d @ (Dev::Keys | Dev::Pad(_))) = menu.seats[p]
                    && !was.contains(&Some(d))
                {
                    flashes.0[p] = now + GLOW;
                }
            }
        }
        *seated = Some(menu.seats);
        let n = menu.players();
        let seat = |d: Dev| menu.seats[..n].iter().position(|s| *s == Some(d));
        for (e, g) in pads.iter().filter(|(_, g)| controls::readable(g)) {
            if g.get_just_pressed().next().is_some()
                && let Some(p) = seat(Dev::Pad(e))
            {
                if flashes.0[p] <= now {
                    rumble.write(GamepadRumbleRequest::Add {
                        gamepad: e,
                        intensity: GamepadRumbleIntensity { strong_motor: BUZZ_STRENGTH, weak_motor: BUZZ_STRENGTH },
                        duration: std::time::Duration::from_secs_f32(BUZZ),
                    });
                }
                flashes.0[p] = now + GLOW;
            }
        }
        if keys.is_some_and(|k| k.get_just_pressed().next().is_some())
            && let Some(p) = seat(Dev::Keys)
        {
            flashes.0[p] = now + GLOW;
        }
    }
    let fixed = *shot.get_or_insert_with(|| std::env::var("HST_FLASH").ok().and_then(|v| v.parse().ok()).filter(|p: &usize| *p < 4));
    let mut want = flashes.0.map(|until| ((until - now) / GLOW).clamp(0.0, 1.0));
    if let Some(p) = fixed.filter(|_| menu.screen == Screen::Assign) {
        want[p] = 1.0;
    }
    if menu.flash != want {
        menu.flash = want;
    }
}

/// Who sits on a seat, for telling where they moved: a device, a computer player, or nobody.
fn occupants(m: &Menu) -> [Option<Option<Dev>>; 4] {
    std::array::from_fn(|p| if m.seats[p].is_some() { Some(m.seats[p]) } else if m.bots[p] { Some(None) } else { None })
}

/// Each seat whose occupant came from another seat this frame slides its chip in from that seat's card: an offset
/// from there to here easing out to nothing over `SLIDE`.
fn slide(mut menu: ResMut<Menu>, time: Res<Time<Real>>, mut was: Local<Option<[Option<Option<Dev>>; 4]>>, mut moves: Local<[Option<([f32; 2], f32)>; 4]>) {
    let now = time.elapsed_secs();
    let on = menu.screen == Screen::Assign;
    let new = occupants(&menu);
    if let Some(old) = was.filter(|_| on) && old != new {
        let cards = super::assign::cards(menu.grid.view, menu.players());
        for q in 0..cards.len() {
            let Some(o) = new[q].filter(|o| old[q] != Some(*o)) else { continue };
            if let Some(p) = (0..cards.len()).find(|&p| p != q && old[p] == Some(o) && new[p] != Some(o)) {
                moves[q] = Some(([cards[p][0] - cards[q][0], cards[p][1] - cards[q][1]], now));
            }
        }
    }
    *was = Some(new);
    if !on {
        *moves = [None; 4];
    }
    let want = moves.map(|m| eased(m, now));
    if menu.slide != want {
        menu.slide = want;
    }
}

/// Who each player column on the select / lobby shows: online, the room's seat (its machine and pick); locally the
/// seat's device (a computer player's column has none).
fn column_ids(m: &Menu) -> [Option<String>; 4] {
    std::array::from_fn(|p| match m.net.room.as_ref().filter(|_| m.online()) {
        Some(r) => r.seats.get(p).cloned().flatten().map(|s| format!("{}#{:?}", s.peer, s.pick)),
        None => m.seats[p].filter(|d| matches!(d, Dev::Keys | Dev::Pad(_))).map(|d| format!("{d:?}")),
    })
}

/// A player who changes sides on the select or the online lobby (□ / Tab, the host's swap) slides their whole column
/// over from the one they left for `SLIDE`; two columns that trade places (a human and a computer one too) both do.
fn columns(mut menu: ResMut<Menu>, time: Res<Time<Real>>, mut was: Local<Option<[Option<String>; 4]>>, mut moves: Local<[Option<([f32; 2], f32)>; 4]>) {
    let now = time.elapsed_secs();
    let on = matches!(menu.screen, Screen::Chars | Screen::Lobby);
    let new = column_ids(&menu);
    if let Some(old) = was.as_ref().filter(|_| on)
        && *old != new
    {
        let n = menu.players();
        let c = menu.columns();
        let at = |p: usize| c.parts(p).card;
        let changed: Vec<usize> = (0..n).filter(|&p| old[p] != new[p]).collect();
        let mut from = [None; 4];
        if let [p, q] = changed[..]
            && old[p] == new[q]
            && old[q] == new[p]
        {
            (from[p], from[q]) = (Some(q), Some(p));
        } else {
            for &q in &changed {
                from[q] = new[q].as_ref().and_then(|id| (0..n).find(|&p| p != q && old[p].as_ref() == Some(id)));
            }
        }
        for q in 0..n {
            if let Some(p) = from[q] {
                let (a, b) = (at(p), at(q));
                moves[q] = Some(([a[0] - b[0], a[1] - b[1]], now));
            }
        }
    }
    *was = Some(new);
    if !on {
        *moves = [None; 4];
    }
    let want = moves.map(|m| eased(m, now));
    if menu.cols != want {
        menu.cols = want;
    }
}

/// A move's offset at `now`: all of it at the start, none after `SLIDE`, eased both ends (smoothstep).
fn eased(m: Option<([f32; 2], f32)>, now: f32) -> [f32; 2] {
    match m {
        Some((d, t)) if now - t < SLIDE => {
            let u = (now - t) / SLIDE;
            let k = 1.0 - u * u * (3.0 - 2.0 * u);
            [d[0] * k, d[1] * k]
        }
        _ => [0.0; 2],
    }
}

/// Card `p`'s sideways offset at `now`, shaking until `until`: a swing that dies out over its last `LINGER`.
fn offset(now: f32, until: f32) -> f32 {
    let left = until - now;
    if left <= 0.0 {
        return 0.0;
    }
    SWING * (left / LINGER).min(1.0) * (now * RATE * std::f32::consts::TAU).sin()
}

fn shake(mut menu: ResMut<Menu>, mut shakes: ResMut<Shakes>, pads: Query<(Entity, &Gamepad)>, time: Res<Time>, mut shot: Local<Option<Option<usize>>>) {
    let now = time.elapsed_secs();
    if menu.screen != Screen::Assign {
        shakes.0 = [0.0; 4];
    } else {
        for (e, g) in pads.iter().filter(|(_, g)| controls::readable(g)) {
            if g.right_stick().length() >= PUSH
                && let Some(p) = menu.seats.iter().position(|s| *s == Some(Dev::Pad(e)))
            {
                shakes.0[p] = now + LINGER;
            }
        }
    }
    let fixed = *shot.get_or_insert_with(|| std::env::var("HST_SHAKE").ok().and_then(|v| v.parse().ok()).filter(|p: &usize| *p < 4));
    let mut want = shakes.0.map(|until| offset(now, until));
    if let Some(p) = fixed.filter(|_| menu.screen == Screen::Assign) {
        want[p] = SWING;
    }
    // written only when it moves, so the menu isn't marked changed every frame
    if menu.shake != want {
        menu.shake = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A seated pad's right stick shakes its own card (not the keyboard's, not an unseated pad's), and only on the
    /// assignment screen; let go, it settles within `LINGER`.
    #[test]
    fn a_pads_right_stick_shakes_its_card() {
        let mut app = App::new();
        app.init_resource::<Shakes>().init_resource::<Time>();
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        let loose = app.world_mut().spawn(Gamepad::default()).id();
        app.insert_resource(Menu { screen: Screen::Assign, doubles: true, seats: [Some(Dev::Keys), None, Some(Dev::Pad(pad)), None], ..default() });
        app.add_systems(Update, shake);
        let stick = |app: &mut App, e: Entity, x: f32, y: f32| {
            let mut g = app.world_mut().get_mut::<Gamepad>(e).unwrap();
            g.analog_mut().set(GamepadAxis::RightStickX, x);
            g.analog_mut().set(GamepadAxis::RightStickY, y);
        };
        // at elapsed 0 the swing's sine is 0: step the clock to a quarter swing
        let at = |app: &mut App, t: f32| {
            let mut time = app.world_mut().resource_mut::<Time>();
            let now = time.elapsed_secs();
            time.advance_by(std::time::Duration::from_secs_f32(t - now));
            app.update();
            app.world().resource::<Menu>().shake
        };
        let quarter = 1.0 / (4.0 * RATE);
        stick(&mut app, loose, 1.0, 0.0);
        assert_eq!(at(&mut app, quarter), [0.0; 4], "an unseated pad has no card");
        stick(&mut app, loose, 0.0, 0.0);
        stick(&mut app, pad, 0.0, 0.3);
        assert_eq!(at(&mut app, quarter + 1.0), [0.0; 4], "a nudge isn't a move");
        stick(&mut app, pad, 0.0, -0.9);
        let s = at(&mut app, quarter + 2.0);
        assert!((s[2] - SWING).abs() < 0.01 && s[0] == 0.0 && s[1] == 0.0 && s[3] == 0.0, "{s:?}");
        stick(&mut app, pad, 0.0, 0.0);
        assert!(at(&mut app, quarter + 2.0 + 2.0 / RATE)[2].abs() > 0.5, "still settling");
        assert_eq!(at(&mut app, quarter + 3.0), [0.0; 4], "settled");
        stick(&mut app, pad, 1.0, 1.0);
        app.world_mut().resource_mut::<Menu>().screen = Screen::Chars;
        assert_eq!(at(&mut app, quarter + 4.0), [0.0; 4], "the select's right stick is its emotes, B463");
    }

    /// B564: any button on a seated pad (any key for the keyboard's seat) flashes that card alone, fading over `GLOW`,
    /// rumbles the pad once per flash, and moves nothing else of the menu; an unseated pad flashes nothing.
    #[test]
    fn a_press_flashes_its_card_and_rumbles_its_pad() {
        let mut app = App::new();
        app.init_resource::<Flashes>().init_resource::<Time>().init_resource::<ButtonInput<KeyCode>>().add_message::<GamepadRumbleRequest>();
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        let loose = app.world_mut().spawn(Gamepad::default()).id();
        app.insert_resource(Menu { screen: Screen::Assign, doubles: true, seats: [Some(Dev::Keys), None, Some(Dev::Pad(pad)), None], ..default() });
        app.add_systems(Update, flash);
        let press = |app: &mut App, e: Entity, b: GamepadButton| {
            let mut g = app.world_mut().get_mut::<Gamepad>(e).unwrap();
            g.digital_mut().clear();
            g.digital_mut().press(b);
        };
        let release = |app: &mut App, e: Entity| app.world_mut().get_mut::<Gamepad>(e).unwrap().digital_mut().reset_all();
        let at = |app: &mut App, t: f32| {
            let mut time = app.world_mut().resource_mut::<Time>();
            let now = time.elapsed_secs();
            time.advance_by(std::time::Duration::from_secs_f32(t - now));
            app.update();
            let buzzes: Vec<Entity> = app.world_mut().resource_mut::<Messages<GamepadRumbleRequest>>().drain().map(|r| r.gamepad()).collect();
            (app.world().resource::<Menu>().flash, buzzes)
        };
        let before = (app.world().resource::<Menu>().sel, app.world().resource::<Menu>().held);
        press(&mut app, loose, GamepadButton::South);
        assert_eq!(at(&mut app, 1.0), ([0.0; 4], vec![]), "an unseated pad has no card");
        release(&mut app, loose);
        press(&mut app, pad, GamepadButton::East);
        assert_eq!(at(&mut app, 2.0), ([0.0, 0.0, 1.0, 0.0], vec![pad]));
        release(&mut app, pad);
        let (f, b) = at(&mut app, 2.0 + GLOW / 2.0);
        assert!((f[2] - 0.5).abs() < 0.01 && b.is_empty(), "{f:?}");
        press(&mut app, pad, GamepadButton::DPadUp);
        assert_eq!(at(&mut app, 2.0 + GLOW * 0.6), ([0.0, 0.0, 1.0, 0.0], vec![]), "a press mid-flash restarts it, no second buzz");
        release(&mut app, pad);
        assert_eq!(at(&mut app, 4.0).0, [0.0; 4], "faded");
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
        assert_eq!(at(&mut app, 5.0), ([1.0, 0.0, 0.0, 0.0], vec![]), "the keyboard's seat");
        let m = app.world().resource::<Menu>();
        assert_eq!((m.sel, m.held), before, "the flash moves no focus");
        app.world_mut().resource_mut::<Menu>().screen = Screen::Chars;
        assert_eq!(at(&mut app, 5.1).0, [0.0; 4], "only on the assignment");
    }

    /// A controller carried to another seat slides its chip over from the seat it left, easing to nothing in `SLIDE`.
    #[test]
    fn a_moved_controller_slides_over() {
        let mut app = App::new();
        app.init_resource::<Time<Real>>();
        app.insert_resource(Menu { screen: Screen::Assign, doubles: true, grid: super::super::Grid::fit(16.0 / 9.0), seats: [Some(Dev::Keys), None, None, None], bots: [false, true, false, false], ..default() });
        app.add_systems(Update, slide);
        app.update();
        assert_eq!(app.world().resource::<Menu>().slide, [[0.0; 2]; 4]);
        app.world_mut().resource_mut::<Menu>().bots.swap(1, 2);
        app.update();
        let s = app.world().resource::<Menu>().slide;
        assert!(s[2][0] < -10.0 && s[1] == [0.0; 2], "{s:?}");
    }

    /// A player switching sides on the select slides their column over from the old one, and the computer column they
    /// traded with slides the other way.
    #[test]
    fn a_side_switch_slides_both_columns() {
        let mut app = App::new();
        app.init_resource::<Time<Real>>();
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.insert_resource(Menu { screen: Screen::Chars, doubles: true, grid: super::super::Grid::fit(16.0 / 9.0), seats: [Some(Dev::Keys), Some(Dev::Pad(pad)), None, None], ..default() });
        app.add_systems(Update, columns);
        app.update();
        app.world_mut().resource_mut::<Menu>().seats.swap(1, 2);
        app.update();
        let c = app.world().resource::<Menu>().cols;
        assert!(c[2][0] < -10.0 && c[1][0] > 10.0 && c[0] == [0.0; 2] && c[3] == [0.0; 2], "{c:?}");
    }
}
