//! The ball hitting a player (`hst_sim::bodyhit`): each rally tick before the point is judged, every player not
//! swinging is tested against the ball with its head and trunk bones as posed; the first hit ends the point and
//! pops up CONK or SMACK at the ball, a camera-facing quad hanging up from its anchor, until the players react.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::bodyhit::{self, Popup, Word};
use hst_sim::player::ReachStats;

use super::{Figure, Game, Phase};
use crate::Args;
use crate::character::Motion;

/// Per player its character's collision size (m); the point's pop-up, and whether it has been made.
#[derive(Resource)]
struct Hits {
    radius: Vec<f32>,
    pop: Option<Popup>,
    made: bool,
}

/// The pop-up quad (its own material) and the two words' textures.
#[derive(Component)]
struct View(Handle<StandardMaterial>, [Handle<Image>; 2]);

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, (detect.before(super::simulate), age.after(super::simulate)))
        .add_systems(Update, draw.after(super::balloons));
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
    let radius = g.chars.iter().map(|&c| ReachStats::from_tparam(&super::tparam(&mut iso, c as usize).join(",")).collision).collect();
    commands.insert_resource(Hits { radius, pop: None, made: false });
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("EFFCT archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let words = [Word::Conk, Word::Smack].map(|w| {
        let name = format!("panel/{}", w.texture());
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(&name)).expect("word in EFFCT");
        super::panel::image(&mut images, &arc.read(e).expect("word bytes"))
    });
    // the bottom edge is the anchor
    let quad = meshes.add(Rectangle::new(1.0, 1.0).mesh().build().translated_by(Vec3::Y * 0.5));
    let m = materials.add(StandardMaterial {
        base_color_texture: Some(words[0].clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((View(m.clone(), words), Mesh3d(quad), MeshMaterial3d(m), Transform::default(), Visibility::Hidden));
}

/// Swinging, diving or whiffing players aren't tested (the game's stroke state).
fn detect(mut g: ResMut<Game>, hits: Res<Hits>, q: Query<(&Figure, &Motion)>) {
    if g.phase == Phase::Serve {
        g.body_hit = None;
    }
    if g.phase != Phase::Rally || g.shots == 0 || g.body_hit.is_some() {
        return;
    }
    // `HST_BODY_HIT=<player>`: that player counts as hit 30 frames into the first rally (tools/record_bodyhit.py forces
    // the same in the original)
    if let Some(i) = std::env::var("HST_BODY_HIT").ok().and_then(|v| v.parse().ok()) {
        if g.since_hit == 30 {
            g.body_hit = Some(i);
        }
        return;
    }
    let ball = g.flight.ball.pos;
    for (f, m) in &q {
        let (p, data) = (&g.players[f.0], &g.data[f.0]);
        if p.swing.is_some() || p.wait_swing.is_some() || p.dive.is_some() || p.whiff.is_some() {
            continue;
        }
        let sk = &data.skeleton;
        let bone = |n: &str| sk.names.iter().position(|b| b == n);
        let (Some(c), Some(head), Some(neck), Some(spine)) = (data.motions.get(&m.id), bone("Bip01Head"), bone("Bip01Neck"), bone("Bip01Spine")) else {
            continue;
        };
        // ponytail: posed from the motion's own clip at its sampled time; mid-crossfade the game's pose is the mix
        let (locals, player) = (c.locals(sk, m.clock.sampled), super::player_matrix(p));
        let at = |n: usize| hst_sim::pose::node_world(sk, &locals, n, &player);
        let point = |n: usize| {
            let r = at(n)[3];
            [r[0], r[1], r[2]]
        };
        if bodyhit::hit(ball, &at(head), point(spine), point(neck), hits.radius[f.0]) {
            info!("ball hit player {}", f.0);
            g.body_hit = Some(f.0 as i32);
            return;
        }
    }
}

/// The pop-up is made the tick the hit is seen and ticks from then; it goes when the players react, and a new
/// point clears it.
/// ponytail: the ball flies on through the player (the game's own course after a body hit isn't ported)
fn age(g: Res<Game>, mut hits: ResMut<Hits>) {
    if g.phase == Phase::Serve || g.post.as_ref().is_some_and(|p| p.reacted) {
        hits.pop = None;
        hits.made &= g.phase != Phase::Serve;
        return;
    }
    if g.body_hit.is_some() && !hits.made {
        hits.made = true;
        hits.pop = Some(Popup::new(g.flight.ball.pos, hst_sim::sound::kmh(g.flight.ball.vel)));
    }
    let practice = g.players.len() < 2;
    if hits.pop.as_mut().is_some_and(|p| !p.tick(practice)) {
        hits.pop = None;
    }
}

fn draw(
    g: Res<Game>,
    hits: Res<Hits>,
    cam: Query<&Transform, (With<crate::Orbit>, Without<View>)>,
    mut q: Query<(&View, &mut Transform, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(cam) = cam.single() else { return };
    for (v, mut tr, mut vis) in &mut q {
        let Some(p) = hits.pop else {
            *vis = Visibility::Hidden;
            continue;
        };
        // game space ↔ world: (x, -y, -z)
        let game = |w: Vec3| [w.x, -w.y, -w.z];
        let world = |p: [f32; 3]| Vec3::new(p[0], -p[1], -p[2]);
        let rows = [game(*cam.right()), game(-*cam.up()), game(*cam.forward())];
        let depth = |at: [f32; 3]| (world(at) - cam.translation).dot(*cam.forward());
        let (at, w, h) = p.quad(rows, depth, g.cam.view.fov.tan());
        tr.translation = world(at);
        tr.rotation = cam.rotation;
        tr.scale = Vec3::new(2.0 * w, 2.0 * h, 1.0);
        if let Some(mut m) = materials.get_mut(&v.0) {
            m.base_color_texture = Some(v.1[(p.word == Word::Smack) as usize].clone());
            m.base_color = Color::srgba(1.0, 1.0, 1.0, p.alpha / 128.0);
        }
        *vis = Visibility::Visible;
    }
}
