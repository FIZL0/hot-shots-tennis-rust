//! The match's weather moves on a game at a time: the original steps its weather and wind counters after each
//! game and applies them when the next point is set up. Rain makes running slower to pick up (`Stats::new`'s
//! agility ×1.5). In rain each player also casts a round blob (`shadow.tm2`, 1.2 m, vertex colour
//! (128, 128, 128, 77)) just above the ground between the feet, on top of the projected shadow.

mod rain;

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};

use super::{panel, Figure, Game, Phase};
use crate::character::Rig;
use crate::weather::Weather;
use crate::Args;
use hst_sim::flow::CHANGE_ENDS;

pub fn plugin(app: &mut App) {
    app.add_plugins(rain::plugin).add_systems(PostStartup, setup.after(super::setup)).add_systems(FixedUpdate, step).add_systems(Update, (blobs, fade.before(crate::weather::apply)));
}

/// A player's rain blob.
#[derive(Component)]
struct Blob(usize);

fn setup(mut commands: Commands, args: Res<Args>, g: Res<Game>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("PCDATA/PCCG0.XB").expect("PCCG0 archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let Some(e) = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with("other/shadow.tm2")) else { return };
    let tex = panel::image(&mut images, &arc.read(e).expect("shadow.tm2 bytes"));
    let quad = meshes.add(Plane3d::default().mesh().size(1.2, 1.2));
    // GS alpha: texture 252 × vertex 77 / 128 ≈ 1.18 at the centre, so the blob's core is fully dark.
    let m = materials.add(StandardMaterial { base_color: Color::WHITE, base_color_texture: Some(tex), unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
    for i in 0..g.players.len() {
        commands.spawn((Blob(i), Mesh3d(quad.clone()), MeshMaterial3d(m.clone()), Transform::from_xyz(0.0, -100.0, 0.0), Visibility::Inherited));
    }
}

/// Between the feet (`Bip01RFoot`, `Bip01LFoot`), 0.0075 above the ground (Bevy space: game y is down).
fn blobs(w: Option<Res<Weather>>, figures: Query<(&Figure, &Rig)>, joints: Query<&GlobalTransform>, mut blobs: Query<(&Blob, &mut Transform, &mut Visibility)>) {
    let rain = w.is_some_and(|w| hst_sim::weather::rain(w.today().weather));
    for (b, mut t, mut v) in &mut blobs {
        let feet = figures.iter().find(|(f, _)| f.0 == b.0).and_then(|(_, r)| {
            let at = |n: &str| r.data.joints.iter().position(|j| j.name == n).and_then(|i| joints.get(r.joints[i]).ok()).map(|g| g.translation());
            Some((at("Bip01RFoot")? + at("Bip01LFoot")?) * 0.5)
        });
        match feet.filter(|_| rain) {
            Some(p) => {
                t.translation = Vec3::new(p.x, 0.0075, p.z);
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
}

/// Each player's clear-weather agility, taken at the first tick.
fn step(mut g: ResMut<Game>, w: Option<ResMut<Weather>>, mut clear: Local<Vec<i32>>) {
    let Some(mut w) = w else { return };
    if g.phase == Phase::Serve && w.game != g.score.games_played as usize {
        w.game = g.score.games_played as usize;
    }
    if clear.len() != g.players.len() {
        *clear = g.players.iter().map(|p| p.stats.agility).collect();
    }
    let rain = hst_sim::weather::rain(w.today().weather);
    for (p, a) in g.players.iter_mut().zip(clear.iter()) {
        p.stats.agility = if rain { a * 150 / 100 } else { *a };
    }
}

/// B360 (user 2026-10-15): the original switches the court's look in one frame when the next game's weather lands
/// on the first point set-up after a change of ends (B377), which shows as the whole picture brightening or
/// darkening. The picture fades into the new game's look over the change of ends instead and holds it until the
/// weather lands. Only the look: `Weather` and the sim keep the original's timing.
fn fade(g: Res<Game>, w: Option<Res<Weather>>, f: Option<ResMut<crate::weather::Fade>>) {
    let (Some(w), Some(mut f)) = (w, f) else { return };
    let game = g.score.games_played as usize;
    let mix = match g.phase {
        Phase::ChangeEnds(n) => Some((Weather { game, ..w.clone() }.today().weather, weight(n))),
        // the change of ends is over but its point hasn't been set up yet
        _ if w.game != game => f.0.map(|(to, _)| (to, 1.0)),
        _ => None,
    };
    f.set_if_neq(crate::weather::Fade(mix));
}

/// How far the look has faded at change-ends tick `n` (counting down to 0): eased in and out, 0 on the phase's
/// first tick, 1 on its last.
fn weight(n: u32) -> f32 {
    let t = 1.0 - n.min(CHANGE_ENDS) as f32 / CHANGE_ENDS as f32;
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fade_runs_over_the_change_of_ends() {
        assert_eq!(weight(CHANGE_ENDS), 0.0);
        assert_eq!(weight(0), 1.0);
        assert_eq!(weight(CHANGE_ENDS / 2), 0.5);
        assert!((0..CHANGE_ENDS).all(|n| weight(n) > weight(n + 1)));
    }
}

