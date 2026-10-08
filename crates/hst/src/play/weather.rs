//! The match's weather moves on a game at a time: the original steps its weather and wind counters after each
//! game and applies them when the next point is set up. Rain makes running slower to pick up (`Stats::new`'s
//! agility ×1.5). In rain each player also casts a round blob (`shadow.tm2`, 1.2 m, vertex colour
//! (128, 128, 128, 77)) just above the ground between the feet, on top of the projected shadow.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};

use super::{panel, Figure, Game, Phase};
use crate::character::Rig;
use crate::weather::Weather;
use crate::Args;

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, setup.after(super::setup)).add_systems(FixedUpdate, step).add_systems(Update, blobs);
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
