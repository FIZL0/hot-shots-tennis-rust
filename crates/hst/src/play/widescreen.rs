//! Widescreen: the 3D view widens with the window (`camera` keeps the game's vertical field of view), while the HUD
//! stays on the PS2's 4:3 screen (its 640×448 space), centred, so its art keeps its shape.

use bevy::prelude::*;

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

/// Share of the window's width the 4:3 screen covers: all of it at 4:3 or narrower (narrower windows stretch it
/// vertically, as the 3D view widens vertically there).
fn share(aspect: f32) -> f32 {
    (4.0 / 3.0 / aspect).min(1.0)
}

fn fit(window: Query<&Window>, mut q: Query<&mut Node, With<Screen43>>) {
    let Ok(w) = window.single() else { return };
    let s = share(w.width() / w.height().max(1.0));
    for mut n in &mut q {
        let (left, width) = (Val::Percent(50.0 * (1.0 - s)), Val::Percent(100.0 * s));
        if n.left != left || n.width != width {
            n.left = left;
            n.width = width;
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
