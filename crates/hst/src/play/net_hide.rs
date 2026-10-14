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
//!
//! The same pass skips any other court object of categories 17–20 the eye is buried in (`buried`): a cut-away
//! camera standing in a flower bed or bush doesn't show it filling the lens.
//!
//! B303: a cut-away follows a player who ended the point off court into the props behind it (Alpine Heights' tables,
//! fences and carved horses), which the test above doesn't hide (thin, partly in front, or no collision mesh), so a
//! tree or prop whose box comes right up in front of the lens is hidden too (`in_front`).

use bevy::prelude::*;
use hst_sim::camera::View;

use super::Game;

/// The court object the close-camera test hides (the net).
#[derive(Component)]
pub struct HideNearCamera;

/// A court object (categories 17–20, its code's third character not '0') the same pass skips while the eye is
/// buried in it: `pos` is its plant's ground position, `prop` its collision object in `Game::world` once found
/// (`None` inside: the model has no collision mesh, so the test never hides it).
#[derive(Component)]
pub struct HideAroundEye {
    pub pos: [f32; 3],
    pub prop: Option<Option<usize>>,
}

/// A tree or prop model with its box in game space (B303's fallback, `in_front`).
#[derive(Component)]
pub struct NearBox {
    pub lo: Vec3,
    pub hi: Vec3,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, hide.after(super::camera)).add_systems(PostUpdate, hide_around_eye.before(bevy::transform::TransformSystems::Propagate));
}

/// The objects the eye test runs on: the pass's categories with the code character the setup checks.
pub fn tested(category: u8, code: [u8; 4]) -> bool {
    (17..=20).contains(&category) && code[2] != b'0'
}

/// Whether the court-object pass skips `o` for this view: a sphere of radius r = tan 20° / tan(half the horizontal
/// fov), clamped to 1..15, centred r ahead of the eye, overlaps the object's bounding sphere, and sweeping it onto
/// that centre from 100 m above and from (3·object radius + 2r) to either side along x all hit the object's mesh,
/// i.e. the eye is looking into the inside of it.
/// ponytail: plain f32 and the ball's sweep (`mesh::Object::sweep`, which drops texel-0 hits); presentation only.
pub fn buried(v: &View, models: &[hst_sim::mesh::Model], o: &hst_sim::mesh::Object) -> bool {
    let r = (20f32.to_radians().tan() / v.fov.tan()).clamp(1.0, 15.0);
    let f = v.rot[2];
    let c = [v.eye[0] + f[0] * r, v.eye[1] + f[1] * r, v.eye[2] + f[2] * r, 1.0];
    let size = o.scale * o.radius;
    let d2: f32 = (0..3).map(|k| (c[k] - o.center[k]).powi(2)).sum();
    if d2 > (r + size).powi(2) {
        return false;
    }
    let side = size * 3.0 + r * 2.0;
    [[0.0, -100.0, 0.0], [-side, 0.0, 0.0], [side, 0.0, 0.0]].iter().all(|off| {
        let start = [c[0] + off[0], c[1] + off[1], c[2] + off[2], 1.0];
        let mut hit = hst_sim::mesh::Hit::NONE;
        o.sweep(models, &mut hit, start, c, r, &[])
    })
}

/// Whether the box `lo..hi` comes within r/2 of the lens' line out to 3r ahead of the eye (r as `buried`, at least 1):
/// the model would fill most of the view.
/// ponytail: by eye, not the original's (its cut-aways have no obstacle check, so it shows these too); sampled along
/// the line, a box thinner than the step between samples can slip through.
pub fn in_front(v: &View, lo: Vec3, hi: Vec3) -> bool {
    let r = (20f32.to_radians().tan() / v.fov.tan()).clamp(1.0, 15.0);
    let (e, f) = (Vec3::from(v.eye), Vec3::from(v.rot[2]));
    (0..=12).any(|i| {
        let p = e + f * (3.0 * r * i as f32 / 12.0);
        p.clamp(lo, hi).distance(p) <= r / 2.0
    })
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

fn hide(g: Res<Game>, own: Option<Res<super::own_camera::OwnCam>>, d: Option<Res<super::cutaway::Director>>, mut q: Query<&mut Visibility, With<HideNearCamera>>) {
    let vis = if hidden(&super::own_camera::shown(&g, own.as_deref()).1, d.is_some_and(|d| d.showing())) { Visibility::Hidden } else { Visibility::Inherited };
    for mut v in &mut q {
        v.set_if_neq(vis);
    }
}

/// Runs on the drawn camera (any mode: match, cut-away, replay, own camera, `HST_EYE`) once it has moved.
fn hide_around_eye(g: Option<Res<Game>>, camera: Query<(&Transform, &Projection), With<crate::Orbit>>, window: Query<&Window>, mut q: Query<(Option<&mut HideAroundEye>, Option<&NearBox>, &mut Visibility), Or<(With<HideAroundEye>, With<NearBox>)>>) {
    let (Some(g), Ok((t, proj))) = (g, camera.single()) else { return };
    let Projection::Perspective(p) = proj else { return };
    // game space is Bevy's turned a half-turn about X; the horizontal fov as `fixed_eye` sets the vertical one
    let aspect = window.single().map_or(16.0 / 9.0, crate::play::widescreen::aspect);
    let (e, f) = (t.translation * Vec3::new(1.0, -1.0, -1.0), *t.forward() * Vec3::new(1.0, -1.0, -1.0));
    let view = View { rot: [[0.0; 3], [0.0; 3], f.to_array()], eye: e.to_array(), fov: ((p.fov / 2.0).tan() / 0.75f32.max(1.0 / aspect)).atan() };
    let world = &g.world.0;
    for (h, near, mut v) in &mut q {
        let hidden = h.is_some_and(|mut h| {
            let pos = h.pos;
            // the prop whose placement takes the plant's ground position to its model origin
            let prop = *h.prop.get_or_insert_with(|| {
                world.props.iter().position(|o| {
                    let m = hst_sim::vu0::transform(&o.to_model, [pos[0], pos[1], pos[2], 1.0]);
                    m[0].abs() + m[1].abs() + m[2].abs() < 1e-3
                })
            });
            prop.is_some_and(|i| buried(&view, &world.models, &world.props[i]))
        }) || near.is_some_and(|b| in_front(&view, b.lo, b.hi));
        v.set_if_neq(if hidden { Visibility::Hidden } else { Visibility::Inherited });
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

    /// Cut-away views the port's own bots reached on Alpine Heights (disc court 6) and Wild Green (8), with the eye
    /// in a flower bed (91) and a red flower bush (44): hidden; the match camera hides nothing in frame.
    #[test]
    fn buried_props() {
        let Ok(mut iso) = hst_data::iso::Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
        let match_cam = view([0.0, -11.89, -39.238], [0.0, 0.292, 0.956], 20.0);
        for (court, prop, eye, fwd, t) in [
            (6, 91, [-2.4966, -1.63983, -16.0944], [0.162865, 0.036274, 0.985981], 0.0436609f32),
            (8, 44, [5.77494, -0.114015, -19.7535], [-0.273811, -0.108348, 0.955661], 0.176327),
        ] {
            let w = hst_sim::court::world(&mut iso, court);
            let v = View { rot: [[0.0; 3], [0.0; 3], fwd], eye, fov: t.atan() };
            assert!(buried(&v, &w.models, &w.props[prop]), "court {court} prop {prop}");
            let gone: Vec<_> = (0..w.props.len()).filter(|&i| buried(&match_cam, &w.models, &w.props[i])).collect();
            // Wild Green's match camera stands in the forked tree behind the baseline (out of frame)
            assert_eq!(gone, if court == 8 { vec![54] } else { vec![] }, "court {court} match camera");
        }
        assert!(tested(17, [0, 0, 0, 0]) && tested(20, *b"ab1\0") && !tested(18, *b"ab0\0") && !tested(16, [0; 4]));
    }

    /// B303: a table 1.5 m ahead of a cut-away's lens is hidden, one 5 m ahead or off to the side isn't; the match
    /// camera's narrower view reaches further.
    #[test]
    fn props_in_front() {
        let cut = view([0.0, -1.0, 0.0], [0.0, 0.0, 1.0], 51.4);
        let table = |z: f32, x: f32| (Vec3::new(x - 0.4, -0.8, z - 0.4), Vec3::new(x + 0.4, 0.0, z + 0.4));
        let hit = |v: &View, (lo, hi): (Vec3, Vec3)| in_front(v, lo, hi);
        assert!(hit(&cut, table(1.5, 0.0)) && hit(&cut, table(2.0, 0.6)));
        assert!(!hit(&cut, table(5.0, 0.0)) && !hit(&cut, table(1.5, 2.0)) && !hit(&cut, table(-1.0, 0.0)));
        assert!(hit(&view([0.0, -1.0, 0.0], [0.0, 0.0, 1.0], 20.0), table(5.0, 0.0)));
    }
}
