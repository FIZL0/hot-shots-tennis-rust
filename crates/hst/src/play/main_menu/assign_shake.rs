//! P21a: on the controller assignment (the menu's and the pause menu's, B254) a pad's seat card shakes while that pad
//! moves its right stick, as the original's controller select does with its selector, so a player can tell which
//! controller is which before swapping seats. The menus never read the right stick there, so nothing clashes. Only
//! the card's drawing moves (`Menu::shake`, applied in the Assign layout): no input, nothing of the match's.
//!
//! `HST_SHAKE=<seat>` (for `--shot`s): that seat's card is drawn mid-shake.

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

/// When each seat's card stops shaking (`Time::elapsed_secs`).
#[derive(Resource, Default)]
struct Shakes([f32; 4]);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Shakes>().add_systems(Update, shake.before(super::draw));
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
}
