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
    /// A swing locked onto the ball since the last hit (the game's per-player search records, not a dive's or a miss's).
    latched: bool,
    /// Last frame's locked swings: per player, then the serve's.
    locked: Vec<bool>,
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
    // the GS adds the stored (gamma-encoded) texel values: raw texels, as gs.rs's (sRGB-decoded, the aura's grey
    // texture added a fraction of its glow)
    // ponytail: added in linear light (the scene target is sRGB), not on gamma values as the GS; B31
    for h in &view.materials {
        if let Some(mut img) = materials.get(h).and_then(|m| m.base_color_texture.clone()).and_then(|t| images.get_mut(&t)) {
            img.texture_descriptor.format = bevy::render::render_resource::TextureFormat::Rgba8Unorm;
        }
    }
    commands.insert_resource(Wind {
        tornado: Tornado::default(),
        fade,
        uv: CourtAnim::new(&mdl, Some(&uva), None, &mtl.materials),
        colours: mtl.materials.iter().map(|m| m.color).collect(),
        view,
        latched: false,
        locked: Vec::new(),
    });
}

/// One game frame: a swing locking onto the ball latches it, and the shot leaving the racket then starts it (the
/// game's ball-away flag is the contact frame; before `start_effects` takes the hit), then it rides the ball. A dive
/// or a missed swing posts no record, so a dive's return starts none unless an earlier lock is still latched.
// ponytail: the latch clears once the point is over; the game clears it at its point reset, no contact comes between
fn tick(fx: Option<ResMut<Wind>>, g: Res<Game>) {
    let Some(mut fx) = fx else { return };
    let fx = &mut *fx;
    let b = &g.flight.ball;
    let v4 = |v: [f32; 3], w: f32| [v[0], v[1], v[2], w];
    let locked: Vec<bool> = g.players.iter().map(|p| p.contact.is_some()).chain([g.serving.swing.is_some()]).collect();
    fx.latched |= locked.iter().zip(fx.locked.iter().chain(std::iter::repeat(&false))).any(|(&now, &was)| now && !was);
    fx.locked = locked;
    if !matches!(g.phase, super::Phase::Serve | super::Phase::Rally) {
        fx.latched = false;
    }
    if g.hit_effect.is_some() && std::mem::take(&mut fx.latched) {
        fx.tornado.start(v4(b.vel, 0.0), fx.fade);
        fx.uv.restart(fx.tornado.uv_speed());
        debug!("tornado {} at speed {}", if fx.tornado.on { "on" } else { "too slow" }, fx.tornado.speed);
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
