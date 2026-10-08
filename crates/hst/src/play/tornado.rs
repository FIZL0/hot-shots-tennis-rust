//! The ball's wind tornado (`hst_sim::tornado`): `wind2/tatumakiball` from `AZUMA/C_EFF/EFFCT.XB0`, started by a
//! shot leaving the racket at 90 km/h or more, riding the ball scaled up by the sim, its `.UVA` scrolling each
//! part's texture at the launch speed + 1.5 frames a frame, its materials fading with the sim's alpha.

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use hst_data::{iso::Iso, mdl, mor, mtl, xb::Archive};
use hst_sim::{court_anim::CourtAnim, tornado::Tornado};

use super::Game;
use crate::effects::{model, Shown};
use crate::Args;

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, tick.before(super::start_effects).after(super::simulate))
        .add_systems(Update, draw);
}

#[derive(Resource)]
struct Wind {
    tornado: Tornado,
    fade: i32,
    uv: CourtAnim,
    /// Per material its MTL colour.
    colours: Vec<[f32; 4]>,
    view: Shown,
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    root: Query<Entity, With<crate::GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let Ok(root) = root.single() else { return };
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
    let fade = hst_data::exe::Game::new(&cnf, &bin).expect("game program").tornado_fade();
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("effect archive");
    let arc = Archive::parse(&data).expect("xb archive");
    let s = "wind2/tatumakiball";
    let get = |ext: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&format!("{s}.{ext}"))).and_then(|e| arc.read(e).ok());
    let mdl = mdl::parse(&get("mdl").expect("tornado mdl")).expect("tornado mdl");
    let mtl = mtl::parse(&get("mtl").expect("tornado mtl"), get("mti").as_deref()).expect("tornado mtl");
    let uva = mor::parse(&get("uva").expect("tornado uva"), 4).expect("tornado uva");
    let (_, view) = model(&arc, s, &mut commands, root, &mut meshes, &mut materials, &mut images, &mut bindposes).expect("tornado model");
    commands.insert_resource(Wind {
        tornado: Tornado::default(),
        fade,
        uv: CourtAnim::new(&mdl, Some(&uva), None, &mtl.materials),
        colours: mtl.materials.iter().map(|m| m.color).collect(),
        view,
    });
}

/// One game frame: a shot leaving the racket starts it (before `start_effects` takes the hit), then it rides the ball.
// ponytail: starts on the frame the shot leaves; the game latches the hit and starts it once it flags the ball away
// (4 frames later on a serve)
fn tick(fx: Option<ResMut<Wind>>, g: Res<Game>) {
    let Some(mut fx) = fx else { return };
    let fx = &mut *fx;
    let b = &g.flight.ball;
    let v4 = |v: [f32; 3], w: f32| [v[0], v[1], v[2], w];
    if g.hit_effect.is_some() {
        fx.tornado.start(v4(b.vel, 0.0), fx.fade);
        fx.uv.restart(fx.tornado.uv_speed());
    }
    fx.tornado.tick(g.flight.bounces, v4(b.pos, 1.0), v4(b.vel, 0.0), fx.fade);
    if fx.tornado.on {
        fx.uv.tick();
    }
}

fn draw(fx: Option<Res<Wind>>, mut q: Query<(&mut Transform, &mut Visibility)>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let Some(fx) = fx else { return };
    let t = &fx.tornado;
    if let Ok((mut tr, mut v)) = q.get_mut(fx.view.root) {
        *v = if t.on { Visibility::Visible } else { Visibility::Hidden };
        *tr = Transform::from_matrix(Mat4::from_cols_array_2d(&t.m)).with_scale(Vec3::splat(t.t));
    }
    if !t.on {
        return;
    }
    for (k, h) in fx.view.materials.iter().enumerate() {
        let Some(mut m) = materials.get_mut(h) else { continue };
        let [r, g, b, a] = fx.colours[k];
        m.base_color = Color::linear_rgba(r, g, b, a * t.opacity());
        m.uv_transform = bevy::math::Affine2::from_translation(Vec2::from(fx.uv.uv_offset(k, 0)));
    }
}
