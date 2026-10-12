//! The net isn't drawn when the camera is right at it looking past it, as the original's court-object pass.
//!
//! The game marks one court object for this: the category-17 plant placed within 1 m of the court's centre (the
//! net). Every frame, before drawing it, it skips it when all of these hold for the view (game space, y down):
//! - the eye is less than 7 m from the court's long axis (|x| < 7·k),
//! - the eye's distance to the net plane, along the view's heading, is under 2 m during a post-point cut-away and
//!   0.8 m otherwise (|z| / cos heading < d·k),
//! - the net top (0.91 m up) seen from the eye stands higher than a line 0.8 × the vertical half-angle below the
//!   view's pitch, the half-angle's tangent being 0.7 × the horizontal one's,
//!
//! k = tan 20° / tan(half the horizontal fov), at least 1 (a zoomed-in view widens the zone).
//! Checked frame by frame against the game's own "not drawn" flag over slot 5's matches (`research/b74d_check.py`).

use bevy::prelude::*;
use hst_sim::camera::View;

use super::Game;

/// The court object the close-camera test hides (the net).
#[derive(Component)]
pub struct HideNearCamera;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, hide.after(super::camera));
}

/// The plant the game picks: category 17 and its ground position within 1 m of the centre.
pub fn picked(category: u8, pos: [f32; 3]) -> bool {
    category == 17 && pos[0].hypot(pos[2]) < 1.0
}

/// Whether the game skips the net's draw for this view.
pub fn hidden(v: &View, cutaway: bool) -> bool {
    let (e, f) = (v.eye, v.rot[2]);
    let k = (20f32.to_radians().tan() / v.fov.tan()).max(1.0);
    let flat = f[0].hypot(f[2]);
    if e[0].abs() >= 7.0 * k || e[2].abs() / (f[2].abs() / flat) >= if cutaway { 2.0 } else { 0.8 } * k {
        return false;
    }
    let top = ((0.91 + e[1]) / e[2].abs()).atan();
    let low = (-f[1]).atan2(flat) - (v.fov.tan() * 0.7).atan() * 0.8;
    low < top
}

fn hide(g: Res<Game>, d: Option<Res<super::cutaway::Director>>, mut q: Query<&mut Visibility, With<HideNearCamera>>) {
    let vis = if hidden(&g.cam.view, d.is_some_and(|d| d.showing())) { Visibility::Hidden } else { Visibility::Inherited };
    for mut v in &mut q {
        v.set_if_neq(vis);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(eye: [f32; 3], fwd: [f32; 3], fov_deg: f32) -> View {
        View { rot: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], fwd], eye, fov: fov_deg.to_radians() / 2.0 }
    }

    /// Views read from the game (slot 5, court 10) with its flag: the close cut-away hides it, the match camera
    /// doesn't.
    #[test]
    fn slot5_cutaway() {
        assert!(hidden(&view([-2.926, -0.517, -0.329], [-0.207, -0.128, 0.97], 51.41), true));
        assert!(hidden(&view([-3.247, -0.5165, -0.3792], [-0.099, -0.128, 0.987], 51.41), true));
        assert!(!hidden(&view([0.0, -11.89, -39.238], [0.0, 0.292, 0.956], 20.0), false));
        // match over, the camera 2.6 m past the net: drawn (the cut-away distance would hide it)
        let end = view([-0.001, -0.807, 2.611], [0.0, 0.019, 1.0], 30.93);
        assert!(!hidden(&end, false) && hidden(&end, true));
    }

    #[test]
    fn picks_the_net() {
        assert!(picked(17, [0.0, 0.0, 0.0]));
        assert!(!picked(17, [0.0, 0.0, 5.0]));
        assert!(!picked(18, [0.0, 0.0, 0.0]));
    }
}
