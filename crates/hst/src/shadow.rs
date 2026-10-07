//! Court shadows the way the game casts them on a day match: one sun per court, a light-space projection of each
//! caster onto the hole's ground model, which it darkens by a fixed amount.
//!
//! - Sun: `envir_cNN.dat` (CMN.XB) row k (k = time of day) holds `u32 count, f32 start, f32 end, f32 tilt` at
//!   0x10 + k·0x30; `envir_cNN_h01.dat` (GRD01.XB) the court azimuth in degrees at 0x58. With hour h (1 at a match
//!   start), m = Rx(−(start + (h−1)/17·(end − start))) · Rz(tilt) · Ry(−π/2 − azimuth); the light goes along m's
//!   third row (game space, sun → ground), its elevation raised to at least 50° for the shadows.
//! - Strength: S = ⌊0.xx·255⌋ from `envir_cNN.dat` 0x410 + k·0x10; the shadow texel is (S·255)>>8 and the ground
//!   under it is multiplied by 1 − that/128 (GS ALPHA (0 − Cd)·A + Cd, in gamma space).
//! - Casters: the players (and rackets), and the trees/props/structures whose placement code byte 3 is not `'0'`.
//!   Only the hole's ground model receives. The ball has its own shadow model.
//!
//! ponytail: time of day 0, hour 1 and no weather (rain/snow scale S by 0.75/0.5/0.25 and swap in the blob
//! shadow `shadow.tm2`); add them with the match settings that pick them (P17e).

use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};

/// Placed props that cast shadows (on the plant record's spawned root).
#[derive(Component)]
pub struct Caster;

#[derive(Resource, Clone, Copy, Debug)]
pub struct Sun {
    /// Light direction in game space (Y down), sun → ground.
    pub dir: Vec3,
    /// How much a shadow takes off the ground colour (0..1).
    pub darken: f32,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (casters, aim));
}

impl Sun {
    pub fn read(iso: &mut Iso, n: usize) -> Option<Sun> {
        let find = |iso: &mut Iso, xb: &str, suffix: &str| {
            let data = iso.read(&format!("COURT/{n:02}/{xb}")).ok()?;
            let arc = Archive::parse(&data).ok()?;
            let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix))?;
            arc.read(e).ok()
        };
        let envir = find(iso, "CMN.XB", &format!("envir_c{n:02}.dat"))?;
        let hole = find(iso, "GRD01.XB", &format!("envir_c{n:02}_h01.dat"))?;
        let f = |d: &[u8], o: usize| Some(f32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?));
        let k = 0;
        let row = 0x10 + k * 0x30;
        let dir = sun_dir(f(&envir, row + 4)?, f(&envir, row + 8)?, f(&envir, row + 12)?, f(&hole, 0x58)?, 1.0);
        Some(Sun { dir: clamp_elevation(dir, 50f32.to_radians()), darken: darken(f(&envir, 0x410 + k * 0x10)?) })
    }
}

/// The game's sun direction (game space, unit), before the shadows' elevation clamp.
fn sun_dir(start: f32, end: f32, tilt: f32, azimuth_deg: f32, hour: f32) -> Vec3 {
    use std::f32::consts::PI;
    let rot = |ax: usize, a: f32| {
        let (s, c) = a.sin_cos();
        let (i, j) = [(1, 2), (2, 0), (0, 1)][ax];
        let mut m = [[0.0f32; 3]; 3];
        (0..3).for_each(|d| m[d][d] = 1.0);
        (m[i][i], m[i][j], m[j][i], m[j][j]) = (c, s, -s, c);
        m
    };
    let mul = |a: [[f32; 3]; 3], b: [[f32; 3]; 3]| std::array::from_fn::<_, 3, _>(|r| std::array::from_fn::<_, 3, _>(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum::<f32>()));
    let pitch = -((hour - 1.0) / 17.0 * (end - start) + start);
    let mut yaw = -PI / 2.0 - azimuth_deg.to_radians();
    if yaw > PI {
        yaw -= 2.0 * PI;
    } else if yaw < -PI {
        yaw += 2.0 * PI;
    }
    let m = mul(mul(rot(0, pitch), rot(2, tilt)), rot(1, yaw));
    Vec3::from(m[2]).normalize()
}

/// Raise `d`'s elevation (angle below the horizon, +y down) to at least `min`, keeping its heading.
fn clamp_elevation(d: Vec3, min: f32) -> Vec3 {
    let heading = d.x.atan2(d.z);
    let elev = d.y.atan2(d.x * heading.sin() + d.z * heading.cos()).max(min);
    Vec3::new(elev.cos() * heading.sin(), elev.sin(), elev.cos() * heading.cos())
}

fn darken(strength: f32) -> f32 {
    let s = (strength * 255.0) as u32;
    ((s * 255) >> 8) as f32 / 128.0
}

/// Only players (skinned rigs and what they carry) and [`Caster`] props cast.
fn casters(mut commands: Commands, new: Query<Entity, Added<Mesh3d>>, parents: Query<&ChildOf>, cast: Query<(), Or<(With<Caster>, With<crate::character::Rig>)>>) {
    for e in &new {
        if !parents.iter_ancestors(e).any(|a| cast.contains(a)) {
            commands.entity(e).insert(NotShadowCaster);
        }
    }
}

/// Point the match's shadow light along the court's sun.
fn aim(mut commands: Commands, sun: Option<Res<Sun>>, mut lights: Query<(Entity, &mut Transform), Added<DirectionalLight>>) {
    let Some(sun) = sun else { return };
    for (e, mut t) in &mut lights {
        // game (x, y, z) is Bevy (x, −y, −z)
        let d = Vec3::new(sun.dir.x, -sun.dir.y, -sun.dir.z);
        *t = Transform::default().looking_to(d, Vec3::Y);
        commands.entity(e).insert(CascadeShadowConfigBuilder { first_cascade_far_bound: 30.0, maximum_distance: 120.0, ..default() }.build());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_matches_game_ram() {
        // court 10 and court 04 at match start, as the game left them in memory
        let c10 = sun_dir(0.9424778, 0.0, -0.31415924, 1e-4, 1.0);
        assert!(c10.distance(Vec3::new(-0.5877856, 0.7694209, 0.2499991)) < 1e-5, "{c10}");
        let c04 = sun_dir(1.9024088, 2.7925267, -0.034906585, 224.7169, 1.0);
        assert!(c04.distance(Vec3::new(-0.20812754, 0.94494265, -0.25252026)) < 1e-5, "{c04}");
        // court 10's sun is just above the shadows' 50°; a low one is raised, heading kept
        assert!(clamp_elevation(c10, 50f32.to_radians()).distance(c10) < 1e-5);
        let low = clamp_elevation(Vec3::new(1.0, 0.1, 0.0).normalize(), 50f32.to_radians());
        assert!((low.y.asin().to_degrees() - 50.0).abs() < 1e-3 && low.z.abs() < 1e-6 && low.x > 0.0);
        // shadow texel 0x2c on court 10 (0.18), 0x4b on court 04 (0.3)
        assert_eq!(darken(0.18) * 128.0, 44.0);
        assert_eq!(darken(0.29999998) * 128.0, 75.0);
    }
}
