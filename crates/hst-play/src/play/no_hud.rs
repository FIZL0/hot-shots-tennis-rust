//! M98: a clean picture for screenshots (ours, not the original's): with the HUD off (`--no-hud`, or F10 toggles it
//! when launched with `--hud-key`)
//! the match draws no UI at all (score panel, pop-ups, hints, name tags, the pause and result screens) and none of
//! the pieces in the scene (timing balloons, the 1P..4P/COM markers, the umpire's calls and the GAME START/SET banners); the court, players, ball and effects draw as
//! ever. Drawing only: nothing here reaches the sim.

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// The HUD is off (`--no-hud`; F10 flips it when [`KEY`] is on).
pub static OFF: AtomicBool = AtomicBool::new(false);
/// F10 toggles the HUD (`--hud-key`, or a scripted F10 in `HST_KEYS`): off by default, so a stray F10 can't hide it
/// in a normal game (B505).
pub static KEY: AtomicBool = AtomicBool::new(false);

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, (toggle, hide, hide_calls).chain().before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate));
}

fn toggle(keys: Option<Res<ButtonInput<KeyCode>>>) {
    if KEY.load(Ordering::Relaxed) && keys.is_some_and(|k| k.just_pressed(KeyCode::F10)) {
        OFF.fetch_xor(true, Ordering::Relaxed);
    }
}

/// Every UI root and every head piece hidden after the HUD's own systems have set them, each frame while it's off.
/// A root's own setting is kept (the last one before it was hidden) and put back when the HUD comes back on.
#[allow(clippy::type_complexity)]
fn hide(
    mut ui: Query<(Entity, &mut Visibility), (With<Node>, Without<ChildOf>)>,
    mut heads: Query<&mut Visibility, (Or<(With<BalloonView>, With<markers::Marker>, With<popups::OnCallCamera>)>, Without<Node>)>,
    mut kept: Local<bevy::platform::collections::HashMap<Entity, Visibility>>,
    game: Option<Res<Game>>,
) {
    // only in a match: the menus always show
    if OFF.load(Ordering::Relaxed) && game.is_some() {
        for (e, mut v) in &mut ui {
            if *v != Visibility::Hidden {
                kept.insert(e, *v);
                *v = Visibility::Hidden;
            }
        }
        for mut v in &mut heads {
            v.set_if_neq(Visibility::Hidden);
        }
    } else {
        for (e, was) in kept.drain() {
            if let Ok((_, mut v)) = ui.get_mut(e) {
                *v = was;
            }
        }
    }
}

/// The umpire's calls: their roots are set shown each frame (`effects::pose`), which a hidden parent doesn't override.
fn hide_calls(calls: Option<Res<popups::Calls>>, game: Option<Res<Game>>, mut q: Query<&mut Visibility, Without<Node>>) {
    if let Some(calls) = calls
        && OFF.load(Ordering::Relaxed)
        && game.is_some()
    {
        for e in calls.roots() {
            if let Ok(mut v) = q.get_mut(e) {
                v.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B505: without `--hud-key`, F10 leaves the HUD on.
    #[test]
    fn f10_does_nothing_without_the_hud_key() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>().add_systems(Update, toggle);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::F10);
        app.update();
        assert!(!KEY.load(Ordering::Relaxed) && !OFF.load(Ordering::Relaxed));
    }
}
