//! The original's surprise pop-ups (`hst_sim::surprise`): the "!" over a computer player caught off guard, and in
//! doubles the sweat drop over the player who lost the point on an error with "..." over their partner, in singles
//! the swirl over that player. Drawn as the timing balloons are (and the balloons are placed here too): a
//! camera-facing quad with its bottom edge 0.5 m over the neck joint (the original's anchor node, `Bip01Neck`),
//! half-width s · max(1, 0.15·z·t) · min(1, 0.3·z·t) for view depth z and t = tan of the half-angle, s 0.3 (the
//! swirl 0.35). Like the head markers they are queued after the whole scene, depth-tested (`markers::LAST`).

use bevy::prelude::*;
use hst_data::{iso::Iso, tim2, xb::Archive};
use hst_sim::serve::{self, Balloon};
use hst_sim::surprise::{self, Pop, Popup};

use super::markers::LAST;
use super::{BalloonView, Figure, Game, Phase};
use crate::character::Rig;
use crate::Args;

/// Live pop-ups, and whether the point was over last tick.
#[derive(Resource, Default)]
struct Pops(Vec<Pop>, bool);

/// Pop-up quad `.0` of the list (its own material).
#[derive(Component)]
struct View(usize, Handle<StandardMaterial>);

/// Textures by `Popup` (bang, sweat, dots, swirl).
#[derive(Resource)]
struct Art([Handle<Image>; 4]);

/// At most a "!" and a sweat drop or "..." per player.
const VIEWS: usize = 8;

pub fn plugin(app: &mut App) {
    app.init_resource::<Pops>()
        .add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, tick.after(super::age_balloons))
        .add_systems(Update, (draw, place_balloons).after(super::balloons).after(crate::character::animate));
}

fn half_width(s: f32, z: f32, t: f32) -> f32 {
    s * (0.15 * z * t).max(1.0) * (0.3 * z * t).min(1.0)
}

/// Local transforms up the hierarchy (joints, figure, game space): this frame's, after the pose is sampled.
type Chain<'w, 's> = Query<'w, 's, (&'static Transform, Option<&'static ChildOf>), (Without<View>, Without<BalloonView>)>;

/// Player `i`'s pop-up anchor in world space: the neck joint, raised 0.5 m.
fn anchor(i: usize, rigs: &Query<(&Figure, &Rig)>, chain: &Chain) -> Option<Vec3> {
    let (_, rig) = rigs.iter().find(|(f, _)| f.0 == i)?;
    let mut e = rig.joints[rig.data.joint("Bip01Neck")?];
    let mut m = bevy::math::Affine3A::IDENTITY;
    loop {
        let (t, up) = chain.get(e).ok()?;
        m = t.compute_affine() * m;
        let Some(up) = up else { break };
        e = up.parent();
    }
    Some(Vec3::from(m.translation) + Vec3::Y * serve::BALLOON_LIFT)
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("EFFCT archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let tex = [Popup::Bang, Popup::Sweat, Popup::Dots, Popup::Swirl].map(|p| {
        let name = p.texture().to_ascii_lowercase();
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&name)).expect("pop-up texture");
        let pic = tim2::decode(&arc.read(e).expect("pop-up bytes")).expect("TIM2").remove(0);
        images.add(Image::new(
            Extent3d { width: pic.width, height: pic.height, depth_or_array_layers: 1 },
            TextureDimension::D2,
            pic.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        ))
    });
    // the bottom edge is the anchor
    let quad = meshes.add(Rectangle::new(1.0, 1.0).mesh().build().translated_by(Vec3::Y * 0.5));
    for i in 0..VIEWS {
        let m = materials.add(StandardMaterial {
            base_color_texture: Some(tex[0].clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            depth_bias: LAST,
            ..default()
        });
        commands.spawn((View(i, m.clone()), Mesh3d(quad.clone()), MeshMaterial3d(m), Transform::default(), Visibility::Hidden));
    }
    commands.insert_resource(Art(tex));
}

/// After the simulation and the balloons' ageing: raise, drop and age the pop-ups.
fn tick(mut g: ResMut<Game>, mut pops: ResMut<Pops>) {
    let Pops(list, was_over) = &mut *pops;
    let clean = |b: Option<(Balloon, u32)>| matches!(b, Some((Balloon::Note | Balloon::Sweet, _)));
    for i in 0..g.players.len() {
        let surprised = std::mem::take(&mut g.players[i].surprised) && g.humans.get(i) != Some(&true);
        let bang = list.iter().position(|p| p.popup == Popup::Bang && p.player == i);
        let p = &mut g.players[i];
        match bang {
            // a clean hit takes the "!" down
            Some(k) if clean(p.balloon) => _ = list.remove(k),
            // a bad-timing balloon isn't shown while it's up
            Some(_) => p.balloon = None,
            None if surprised && !clean(p.balloon) => {
                p.balloon = None;
                list.push(Pop::new(Popup::Bang, i));
                info!("surprise: \"!\" over player {i}");
            }
            None => {}
        }
    }
    // in doubles a point lost on an error: sweat over the player who made it, "..." over the partner, until the
    // next point is set up
    let over = g.phase == Phase::Post;
    if over && !*was_over {
        let r = &g.rally;
        let (call, n) = (r.judge(None).call, g.players.len());
        let (sweat, swirl) = (surprise::sweat(n, call, r.shots, r.hitter, r.server), surprise::swirl(n, call, r.shots, r.hitter, r.server));
        if let Some((who, partner)) = sweat {
            list.retain(|p| p.player != who && p.player != partner);
            g.players[who].balloon = None;
            g.players[partner].balloon = None;
            list.push(Pop::new(Popup::Sweat, who));
            list.push(Pop::new(Popup::Dots, partner));
            info!("surprise: sweat over player {who}, \"...\" over {partner}");
        }
        // singles: the swirl, taking down that player's "!" and balloon
        if let Some(who) = swirl {
            list.retain(|p| !(p.player == who && p.popup == Popup::Bang));
            g.players[who].balloon = None;
            list.push(Pop::new(Popup::Swirl, who));
            info!("surprise: swirl over player {who}");
        }
    }
    *was_over = over;
    if g.phase == Phase::Serve {
        list.retain(|p| p.popup == Popup::Bang);
    }
    list.retain_mut(|p| p.tick());
}

fn draw(
    g: Res<Game>,
    pops: Res<Pops>,
    art: Res<Art>,
    cam: Query<&Transform, (With<crate::Orbit>, Without<View>, Without<BalloonView>)>,
    rigs: Query<(&Figure, &Rig)>,
    chain: Chain,
    mut q: Query<(&View, &mut Transform, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(cam) = cam.single() else { return };
    let t = g.cam.view.fov.tan();
    for (View(k, m), mut tr, mut vis) in &mut q {
        let Some(pop) = pops.0.get(*k) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let Some(anchor) = anchor(pop.player, &rigs, &chain) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let s = half_width(pop.popup.size(), (anchor - cam.translation).dot(*cam.forward()), t);
        tr.translation = anchor;
        tr.rotation = cam.rotation;
        tr.scale = Vec3::new(2.0 * s, 2.0 * s, 1.0);
        if let Some(mut mat) = materials.get_mut(m) {
            mat.base_color_texture = Some(art.0[pop.popup as usize].clone());
            mat.base_color = Color::srgba(1.0, 1.0, 1.0, pop.alpha as f32 / 128.0);
            // the swirl: one of five 80-px cells side by side
            mat.uv_transform = if pop.popup == Popup::Swirl {
                bevy::math::Affine2::from_scale_angle_translation(Vec2::new(0.2, 1.0), 0.0, Vec2::new(pop.cell as f32 * 0.2, 0.0))
            } else {
                bevy::math::Affine2::IDENTITY
            };
        }
        *vis = Visibility::Visible;
    }
}

/// The timing balloons (`super::balloons` sets their texture, fade and visibility): the same anchor and size.
fn place_balloons(
    g: Res<Game>,
    cam: Query<&Transform, (With<crate::Orbit>, Without<View>, Without<BalloonView>)>,
    rigs: Query<(&Figure, &Rig)>,
    chain: Chain,
    mut q: Query<(&BalloonView, &mut Transform), Without<View>>,
) {
    let Ok(cam) = cam.single() else { return };
    let t = g.cam.view.fov.tan();
    for (view, mut tr) in &mut q {
        let Some(anchor) = anchor(view.0, &rigs, &chain) else { continue };
        let s = half_width(serve::BALLOON_SIZE, (anchor - cam.translation).dot(*cam.forward()), t);
        // the balloon's quad is centred
        tr.translation = anchor + cam.up() * s;
        tr.rotation = cam.rotation;
        tr.scale = Vec3::new(2.0 * s, 2.0 * s, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size() {
        assert_eq!(half_width(0.3, 20.0, 0.2), 0.3);
        assert!((half_width(0.3, 100.0, 0.1) - 0.3 * 1.5).abs() < 1e-6);
        assert!((half_width(0.35, 2.0, 1.0) - 0.35 * 0.6).abs() < 1e-6);
    }
}
