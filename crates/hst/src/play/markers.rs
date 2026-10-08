//! The original's player markers: before each serve a "1P".."4P"/"COM" tag with a pointer under it hangs over each
//! player's head (`azuma/panel/i_playerinfo` in `AZUMA/C_EFF/EFFCT.XB0`, 40×32 cells), tinted with the player's
//! panel colour. They come on when the point is set up and go the moment the server starts the toss (back if the
//! toss is caught), solid (no fade), only in games of more than one player.
//!
//! Each is a camera-facing quad whose bottom edge (the pointer's tip) is at the player's ground position raised to
//! the character's reach centre (TParam column 55) + 0.65 m. Its half-width is 0.3 · max(1, 0.15·z·t) · min(1,
//! 0.2·z·t) for view depth z and t = tan of the horizontal half-angle, which keeps it a near-constant size on screen;
//! its height is 0.84 of its width.
//!
//! The original queues them in a draw bucket after the whole 3D scene, Z-tested but not Z-written: the court and
//! the players' bodies in front still cover them (sunk into the court they clip at the ground,
//! `research/marker_depth.py`), but translucent scenery never draws over them. Bevy sorts blended meshes by centre
//! distance, so a big blended scenery mesh nearer by centre would draw over the marker; `LAST` sorts it after them.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};

use super::panel::{self, Colours};
use super::{Game, Pads, Phase};
use crate::Args;

/// Added to the marker's sort distance (metres) so it draws after every other blended mesh. Bevy also applies it
/// as a constant GPU depth bias (in depth-buffer units: ~0.1 % of the view depth, a few cm at court range).
pub(super) const LAST: f32 = 1e4;

/// One player's marker quad (its own material: tint and cell).
#[derive(Component)]
struct Marker(usize, Handle<StandardMaterial>);

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, setup.after(super::setup))
        .add_systems(Update, draw.after(super::balloons));
}

/// Whether the markers show: from the serve's set-up until the toss, with more than one player.
fn shown(g: &Game) -> bool {
    g.players.len() > 1 && g.phase == Phase::Serve && g.serving.toss.is_none()
}

/// The marker's half-width at view depth `z` with `t` = tan(horizontal half-angle).
fn half_width(z: f32, t: f32) -> f32 {
    0.3 * (0.15 * z * t).max(1.0) * (0.2 * z * t).min(1.0)
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    g: Res<Game>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("EFFCT archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc
        .entries
        .iter()
        .find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with("panel/i_playerinfo.tm2"))
        .expect("i_playerinfo in EFFCT");
    let tex = panel::image(&mut images, &arc.read(e).expect("i_playerinfo bytes"));
    // the bottom edge is the anchor
    let quad = meshes.add(Rectangle::new(1.0, 1.0).mesh().build().translated_by(Vec3::Y * 0.5));
    for i in 0..g.players.len() {
        let m = materials.add(StandardMaterial {
            base_color_texture: Some(tex.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            depth_bias: LAST,
            ..default()
        });
        commands.spawn((Marker(i, m.clone()), Mesh3d(quad.clone()), MeshMaterial3d(m), Transform::default(), Visibility::Hidden));
    }
}

fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    colours: Option<Res<Colours>>,
    time: Res<Time<Fixed>>,
    cam: Query<&Transform, (With<crate::Orbit>, Without<Marker>)>,
    mut q: Query<(&Marker, &mut Transform, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (Ok(cam), Some(colours)) = (cam.single(), colours) else { return };
    let on = shown(&g);
    let n = g.players.len();
    let a = time.overstep_fraction();
    let t = g.cam.view.fov.tan();
    for (Marker(i, m), mut tr, mut vis) in &mut q {
        if !on {
            *vis = Visibility::Hidden;
            continue;
        }
        let p = &g.players[*i];
        let at = Vec3::from(p.prev).lerp(Vec3::from(p.pos), a);
        // game space → world: (x, -y, -z), and the anchor's height is absolute
        let anchor = Vec3::new(at.x, g.reaches[*i].base + 0.65, -at.z);
        let z = (anchor - cam.translation).dot(*cam.forward());
        let s = half_width(z, t);
        tr.translation = anchor;
        tr.rotation = cam.rotation;
        tr.scale = Vec3::new(2.0 * s, 2.0 * 0.84 * s, 1.0);
        if let Some(mut mat) = materials.get_mut(m) {
            let slot = pads.slot_of(*i, n).unwrap_or(4);
            // cell `slot` of five: u from slot·0.156 to (slot+1)·0.156, the full height
            mat.uv_transform = bevy::math::Affine2::from_scale_angle_translation(Vec2::new(0.156, 1.0), 0.0, Vec2::new(slot as f32 * 0.156, 0.0));
            let [r, gg, b] = colours.0[*i].map(|c| c as f32 / 128.0);
            mat.base_color = Color::srgb(r, gg, b);
        }
        *vis = Visibility::Visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constant on-screen size far out (z·t ≥ 1/0.15), constant world size in the middle band, then shrinking
    /// with the view near the camera.
    #[test]
    fn size() {
        assert!((half_width(100.0, 0.1) - 0.3 * 1.5).abs() < 1e-6);
        assert_eq!(half_width(5.5, 1.0), 0.3);
        assert!((half_width(2.0, 1.0) - 0.3 * 0.4).abs() < 1e-6);
    }
}
