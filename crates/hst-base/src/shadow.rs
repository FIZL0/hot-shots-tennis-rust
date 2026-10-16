//! Court shadows the way the game casts them on a day match: one sun per court, a light-space projection of each
//! caster onto the hole's ground model, which it darkens by a fixed amount.
//!
//! - Sun: `envir_cNN.dat` (CMN.XB) row k (k = time of day) holds `u32 count, f32 start, f32 end, f32 tilt` at
//!   0x10 + k·0x30; `envir_cNN_h01.dat` (GRD01.XB) the court azimuth in degrees at 0x58. With hour h (1 at a match
//!   start), m = Rx(−(start + (h−1)/17·(end − start))) · Rz(tilt) · Ry(−π/2 − azimuth); the light goes along m's
//!   third row (game space, sun → ground), its elevation raised to at least 50° for the shadows
//!   (`hst_sim::shade::sun`, in the game's floats).
//! - Strength: S = ⌊0.xx·255⌋ from `envir_cNN.dat` 0x410 + k·0x10; the shadow texel is (S·255)>>8 and the ground
//!   under it is multiplied by 1 − that/128 (GS ALPHA (0 − Cd)·A + Cd, in gamma space).
//! - Casters: the players (and rackets), and the trees/props/structures whose placement code byte 3 is not `'0'`.
//!   Only the hole's ground model receives. The ball has its own shadow model.
//!
//! - Weather scales S (`hst_sim::weather::shadow`, applied by `weather`); in rain the players also get a blob.
//!
//! ponytail: time of day 0 and hour 1; add them with the match settings that pick them (P17e).

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};

/// Placed props that cast shadows (on the plant record's spawned root).
#[derive(Component)]
pub struct Caster;

/// B351: a caster that stops casting at Shadows Low (on the root of a small [`Caster`] prop or of an NPC rig other
/// than the umpire): each caster is another draw in the shadow pass, too many for a phone. The players, the umpire
/// and the props whose shadow reaches the court ([`reaches_court`]: the net, the umpire's stand, the trees and
/// buildings by it) keep theirs; the ball has its own shadow model either way.
#[derive(Component)]
pub struct Minor;

/// A mesh under a [`Minor`] root that would cast: `NotShadowCaster` while the shadows are Low (`low`).
#[derive(Component)]
struct Dropped;

/// B351: whether a [`Caster`] prop's shadow can fall on the court or its run-off (game space, metres): its placed
/// bounding box `lo`..`hi` swept along the sun `dir` (sun → ground) down to the ground (y 0) overlaps the
/// doubles court's 11.9 × 5.5 m half-extents plus 3 m behind the baselines and 2 m beside the sidelines. At Shadows
/// Low only those keep casting: 4 to 37 prop draws a court instead of 21 to 144 (all 11 courts' plant records); the
/// rest land where the match camera hardly looks.
pub fn reaches_court(lo: Vec3, hi: Vec3, dir: Vec3) -> bool {
    let (mut a, mut b) = (Vec2::MAX, Vec2::MIN);
    for i in 0..8 {
        let p = Vec3::new([lo.x, hi.x][i & 1], [lo.y, hi.y][i >> 1 & 1], [lo.z, hi.z][i >> 2]);
        // the corner itself and where the sun takes it on the ground (above it: y < 0, Y down)
        let g = if dir.y > 1e-3 { p - dir * (p.y / dir.y) } else { p };
        for q in [p, g] {
            (a, b) = (a.min(q.xz()), b.max(q.xz()));
        }
    }
    let court = Vec2::new(5.5 + 2.0, 11.9 + 3.0);
    a.cmple(court).all() && b.cmpge(-court).all()
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct Sun {
    /// Shadow direction in game space (Y down), sun → ground: `light` raised to at least 50°.
    pub dir: Vec3,
    /// The sun as VU1 lights the court with it (game space, sun → ground).
    pub light: Vec3,
    /// The court's shadow strength row value (0..1; `strength` turns it into S).
    pub strength: f32,
    /// The light axes the shade map's casters are seen along (`hst_sim::shade::sun_axes` of `dir`).
    pub axes: [[f32; 4]; 3],
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (casters, low.after(casters), aim));
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
        let (light, dir) = hst_sim::shade::sun(f(&envir, row + 4)?, f(&envir, row + 8)?, f(&envir, row + 12)?, f(&hole, 0x58)?, 1);
        let strength = f(&envir, 0x410 + k * 0x10)?;
        let v = |d: [f32; 4]| Vec3::new(d[0], d[1], d[2]);
        Some(Sun { dir: v(dir), light: v(light), strength, axes: hst_sim::shade::sun_axes(dir) })
    }
}

#[cfg(test)]
fn darken(strength: f32) -> f32 {
    darken_s((strength * 255.0) as u32)
}

/// The shadow's strength S (0..255) before the weather scales it: ⌊row·255⌋, or ⌊intensity·128⌋ (the court light's,
/// weather-scaled) when the row is 0 (court 7's first time of day).
pub fn strength(row: f32, intensity: f32) -> u32 {
    if row == 0.0 { (intensity * 128.0) as u32 } else { (row * 255.0) as u32 }
}

/// How much a shadow of strength S takes off the ground colour.
pub fn darken_s(s: u32) -> f32 {
    ((s * 255) >> 8) as f32 / 128.0
}

/// Only players (skinned rigs and what they carry) and [`Caster`] props cast.
fn casters(mut commands: Commands, new: Query<Entity, Added<Mesh3d>>, parents: Query<&ChildOf>, cast: Query<(), Or<(With<Caster>, With<crate::character::Rig>)>>, minor: Query<(), With<Minor>>) {
    for e in &new {
        if !parents.iter_ancestors(e).any(|a| cast.contains(a)) {
            commands.entity(e).try_insert(NotShadowCaster);
        } else if parents.iter_ancestors(e).any(|a| minor.contains(a)) {
            commands.entity(e).try_insert(Dropped);
        }
    }
}

/// Shadows Low: the [`Minor`] casters' meshes don't cast; Off and High give them back (Off draws none anyway, and
/// switching on again needs no rebuild). B343: so does Low with Fewer Shadows off (`Graphics::fewer_shadows`).
fn low(mut commands: Commands, g: Option<Res<crate::graphics::Graphics>>, all: Query<Entity, With<Dropped>>, new: Query<Entity, Added<Dropped>>) {
    let on = g.as_ref().is_some_and(|g| g.shadows == crate::graphics::Shadows::Low && g.fewer_shadows);
    let changed = g.as_ref().is_some_and(|g| g.is_changed());
    for e in if changed { all.iter().collect::<Vec<_>>() } else { new.iter().collect() } {
        if on {
            commands.entity(e).try_insert(NotShadowCaster);
        } else {
            commands.entity(e).try_remove::<NotShadowCaster>();
        }
    }
}

/// Point the match's shadow light along the court's sun.
/// (Its cascades follow the shadow quality setting, `graphics::lights`.)
fn aim(sun: Option<Res<Sun>>, mut lights: Query<&mut Transform, Added<DirectionalLight>>) {
    let Some(sun) = sun else { return };
    for mut t in &mut lights {
        // game (x, y, z) is Bevy (x, −y, −z)
        let d = Vec3::new(sun.dir.x, -sun.dir.y, -sun.dir.z);
        *t = Transform::default().looking_to(d, Vec3::Y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_keeps_the_shadows_on_the_court() {
        let sun = Vec3::new(-0.5877856, 0.7694209, 0.2499991);
        let b = |lo: [f32; 3], hi: [f32; 3]| reaches_court(Vec3::from(lo), Vec3::from(hi), sun);
        // the net, the umpire's stand beside it (game Y down: above the ground is y < 0)
        assert!(b([-6.6, -1.1, -0.1], [6.6, 0.0, 0.1]) && b([-7.7, -2.3, -0.3], [-6.2, 0.0, 0.3]));
        // a 15 m tree 6 m past the sideline: its shadow falls across the court, a bench there doesn't reach
        assert!(b([10.0, -15.0, -2.0], [14.0, 0.0, 2.0]) && !b([10.0, -0.8, -2.0], [11.0, 0.0, 2.0]));
        // a spare net 17 m off to the side, a clubhouse far behind a baseline
        assert!(!b([16.9, -1.1, -6.6], [17.1, 0.0, 6.6]) && !b([-20.0, -6.0, -50.0], [-14.0, 0.0, -40.0]));
    }

    #[test]
    fn sun_matches_game_ram() {
        // court 10 and court 04 at match start, as the game left them in memory
        let sun = |s, e, t, a| hst_sim::shade::sun(s, e, t, a, 1);
        let (c10, shade10) = sun(0.9424778, 0.0, -0.31415924, 1e-4);
        assert!(Vec3::from_slice(&c10).distance(Vec3::new(-0.5877856, 0.7694209, 0.2499991)) < 1e-5, "{c10:?}");
        let (c04, _) = sun(1.9024088, 2.7925267, -0.034906585, 224.7169);
        assert!(Vec3::from_slice(&c04).distance(Vec3::new(-0.20812754, 0.94494265, -0.25252026)) < 1e-5, "{c04:?}");
        // court 10's sun is just above the shades' 50°; a low one is raised, heading kept
        assert!(Vec3::from_slice(&shade10).distance(Vec3::from_slice(&c10)) < 1e-5);
        let (raw, low) = sun(0.2, 0.2, 0.0, 0.0);
        assert!((low[1].asin().to_degrees() - 50.0).abs() < 1e-3 && low[2].abs() < 1e-6 && low[0] * raw[0] > 0.0, "{low:?}");
        // shadow texel 0x2c on court 10 (0.18), 0x4b on court 04 (0.3)
        assert_eq!(darken(0.18) * 128.0, 44.0);
        assert_eq!(darken(0.29999998) * 128.0, 75.0);
        // PINE court 10 weather 3: intensity 0x3ee8f5c0 (0.455) → S 58 when the row is 0
        assert_eq!((strength(0.0, f32::from_bits(0x3ee8_f5c0)), strength(0.18, 0.455)), (58, 45));
        // PINE slot 5 (court 10) switched to weather 3: texel 0x0a; with the row poked to 0, 0x0d
        let texel = |row| darken_s(hst_sim::weather::shadow(strength(row, f32::from_bits(0x3ee8_f5c0)), 3)) * 128.0;
        assert_eq!((texel(0.18), texel(0.0)), (10.0, 13.0));
    }
}
