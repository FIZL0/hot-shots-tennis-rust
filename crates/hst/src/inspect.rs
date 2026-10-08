//! A character's 3D preview, drawn as the original's inspect screen (Data → Items → Characters) draws its model: the
//! menu pose ([`character::POSE`]) held at [`character::pose_frame`] (the screen never advances it), the model
//! yawed π (facing the camera) 8 m in front of it and 1.5 m below its axis, mirrored for left-handers, lit by an
//! ambient 0.47 and two white lights of 0.4 along (1, 0, 1)/√2 and (−0.819152, 0, 0.573576) (game space). The
//! camera sees 30° across the 4:3 frame's 640 pixels, its axis at pixel (520, 176) of 640×448 (the model stands
//! right of the text); a preview shows a crop of that frame ([`CROP`] by default) widened to its rectangle's aspect.
//! R2 plays the win reaction (`gu_set`, 0x2e), L2 the loss (`di_set`, 0x2f) from frame 0 to its end and holds there,
//! △ goes back to the pose; each sets the yaw at once (the loss: 140° for Lola, 135° for Will). Each is a cut (no
//! crossfade), and no root path moves the model; but once Brad (5) or Michael (13) has played either reaction, the
//! model is drawn with `Bip01Pelvis` over its spot ([`pelvis_centre`]), the pose too, until the preview is respawned.
//!
//! B40's select spawns one per slot with [`spawn`] and moves it with [`Preview::rect`]; `--inspect` shows the
//! roster side by side.

use std::f32::consts::PI;
use std::sync::Arc;

use bevy::app::{HierarchyPropagatePlugin, Propagate};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{CameraOutputMode, SubCameraView, Viewport};
use bevy::render::render_resource::BlendState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::motion::Clock;
use hst_sim::pose::{Clip, Skeleton};

use crate::character::{self, CharacterData, Motion, POSE};

/// The part of the original's 640×448 frame a slot shows (x0, y0, x1, y1): the model's column, head to feet.
pub const CROP: Rect = Rect { min: Vec2::new(440.0, 100.0), max: Vec2::new(600.0, 410.0) };

/// The frame's focal lengths in its own pixels (x: 30° across 640, y: VU1's 557.32 per field line × 2) and the
/// camera axis's pixel.
const FOCAL: Vec2 = Vec2::new(1194.256, 1114.64);
const AXIS: Vec2 = Vec2::new(520.0, 176.0);

/// The loss reaction's yaw per character (else π).
fn loss_yaw(n: usize) -> f32 {
    match n {
        4 => 2.443461,
        9 => 2.356194,
        _ => PI,
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins(HierarchyPropagatePlugin::<RenderLayers>::new(PostUpdate)).add_systems(Update, (place, act, centre.after(character::animate)));
}

/// A preview's camera: where it draws (logical window pixels), which part of the original's frame it shows, and
/// its character (`n`: the disc character whose pose timing and yaws it takes, a mod's donor).
#[derive(Component)]
pub struct Preview {
    pub rect: Rect,
    pub crop: Rect,
    pub n: usize,
    pub rig: Entity,
    /// Draw the model with its pelvis over its spot ([`pelvis_centre`]).
    pub centre: bool,
}

/// What the inspect screen's buttons do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Pose,
    Win,
    Loss,
}

/// Spawn character `data` (disc character `n`, or a mod's donor) as a preview on render layer `layer` (one per
/// preview, not 0 or 31) drawing into `rect`; returns its camera.
pub fn spawn(commands: &mut Commands, data: &Arc<CharacterData>, n: usize, hand: f32, layer: usize, rect: Rect) -> Entity {
    let layers = RenderLayers::layer(layer);
    // game space, as `GameSpace`: the GS shader lights in it
    let space = commands.spawn((Transform::from_rotation(Quat::from_rotation_x(PI)), Visibility::default(), Propagate(layers.clone()))).id();
    let rig = character::spawn(commands, data, space);
    let t = character::pose_frame(n, data.motions.get(&POSE).map_or(0.0, |c| c.length));
    commands.entity(rig).insert((Transform { translation: Vec3::new(0.0, 1.5, 8.0), rotation: Quat::from_rotation_y(PI), scale: Vec3::new(hand, 1.0, 1.0) }, held(t), crate::shade::FixedLight(LIGHT)));
    commands
        .spawn((
            Camera3d::default(),
            // the model's alpha is VU1's specular term: copy the view out as it is, not blended over the last camera's
            Camera {
                order: 10 + layer as isize,
                clear_color: ClearColorConfig::None,
                output_mode: CameraOutputMode::Write { blend_state: Some(BlendState::REPLACE), clear_color: ClearColorConfig::None },
                ..default()
            },
            bevy::core_pipeline::tonemapping::Tonemapping::None,
            Projection::Perspective(PerspectiveProjection { fov: 2.0 * (AXIS.y.max(448.0 - AXIS.y) / FOCAL.y).atan(), near: 0.01, far: 3072.0, ..default() }),
            Transform::default(),
            layers,
            Preview { rect, crop: CROP, n, rig, centre: false },
        ))
        .id()
}

/// The inspect screen's light (direction, colour, ambient, second direction, second colour).
const LIGHT: [Vec4; 5] = [
    Vec4::new(std::f32::consts::FRAC_1_SQRT_2, 0.0, std::f32::consts::FRAC_1_SQRT_2, 0.0),
    Vec4::new(0.4, 0.4, 0.4, 1.0),
    Vec4::new(0.47, 0.47, 0.47, 1.0),
    Vec4::new(-0.819152, 0.0, 0.573576, 0.0),
    Vec4::new(0.4, 0.4, 0.4, 1.0),
];

/// The pose held at frame `t`, face and all.
fn held(t: f32) -> Motion {
    let clock = Clock { time: t, sampled: t, speed: 0.0, looping: false, hold: None };
    Motion { id: POSE, clock, prev: t, face: POSE, face_clock: clock, ..default() }
}

/// Do `act` on preview `p`'s character.
pub fn apply(p: &mut Preview, act: Act, rigs: &mut Query<(&mut Motion, &mut Transform)>, lengths: impl Fn(usize) -> f32) {
    let Ok((mut m, mut t)) = rigs.get_mut(p.rig) else { return };
    let (motion, yaw) = match act {
        Act::Pose => (held(character::pose_frame(p.n, lengths(POSE))), PI),
        Act::Win | Act::Loss => {
            let id = if act == Act::Win { 0x2e } else { 0x2f };
            let mut m = *m;
            m.set(id, 1.0, false, None, m.serial);
            p.centre |= p.n == 5 || p.n == 13;
            (m, if act == Act::Win { PI } else { loss_yaw(p.n) })
        }
    };
    *m = motion;
    t.rotation = Quat::from_rotation_y(yaw);
}

/// The model's (x, z) that puts `Bip01Pelvis` over its spot (0, 8) at frame `t` of `clip`, yawed `yaw` and
/// mirrored by `hand`: the original poses the skeleton at the spot, then moves the model by the spot minus the
/// pelvis's world x/z.
pub fn pelvis_centre(sk: &Skeleton, clip: &Clip, t: f32, yaw: f32, hand: f32) -> Option<Vec2> {
    let pelvis = sk.names.iter().position(|n| n == "Bip01Pelvis")?;
    let (s, c) = yaw.sin_cos();
    let player = [[hand * c, 0.0, -hand * s, 0.0], [0.0, 1.0, 0.0, 0.0], [s, 0.0, c, 0.0], [0.0, 1.5, 8.0, 1.0]];
    let w = hst_sim::pose::node_world(sk, &clip.locals(sk, t), pelvis, &player)[3];
    Some(Vec2::new(-w[0], 8.0 + (8.0 - w[2])))
}

/// Centred previews' models moved to [`pelvis_centre`] for the frame drawn (`character::animate`'s time).
fn centre(time: Res<Time<Fixed>>, cams: Query<&Preview>, mut rigs: Query<(&character::Rig, &Motion, &mut Transform)>) {
    for p in cams.iter().filter(|p| p.centre) {
        let Ok((rig, m, mut tf)) = rigs.get_mut(p.rig) else { continue };
        let Some(clip) = rig.data.motions.get(&m.id) else { continue };
        let t = clip.wrap(m.prev + (m.clock.sampled - m.prev) * time.overstep_fraction(), false);
        let (_, yaw, _) = tf.rotation.to_euler(EulerRot::YXZ);
        if let Some(c) = pelvis_centre(&rig.data.skeleton, clip, t, yaw, tf.scale.x) {
            (tf.translation.x, tf.translation.z) = (c.x, c.y);
        }
    }
}

/// Each preview's viewport and crop for its rectangle; a preview camera clears nothing (`hud_gamma` gives every scene
/// camera its clear colour, which would wipe the others).
fn place(window: Query<&Window, With<PrimaryWindow>>, mut cams: Query<(&Preview, &mut Camera)>) {
    let Ok(w) = window.single() else { return };
    let s = w.scale_factor();
    for (p, mut cam) in &mut cams {
        cam.clear_color = ClearColorConfig::None;
        let (pos, size) = ((p.rect.min * s).as_uvec2(), (p.rect.size() * s).max(Vec2::ONE).as_uvec2());
        cam.viewport = Some(Viewport { physical_position: pos, physical_size: size, ..default() });
        cam.sub_camera_view = Some(sub_view(p.crop, size.as_vec2()));
    }
}

/// The sub-view of the camera's full frustum (symmetric about the axis pixel, in display-square units) that shows
/// `crop` of the original's frame widened (or heightened) to `size`'s aspect, about its centre.
fn sub_view(crop: Rect, size: Vec2) -> SubCameraView {
    // a frame pixel's width / height on a 4:3 screen
    let px = FOCAL.x / FOCAL.y;
    let half = Vec2::new(AXIS.x.max(640.0 - AXIS.x), AXIS.y.max(448.0 - AXIS.y));
    let k = 16.0;
    let full = Vec2::new(2.0 * half.x / px, 2.0 * half.y) * k;
    let (mut c, mut e) = (Vec2::new(crop.min.x / px, crop.min.y), Vec2::new(crop.size().x / px, crop.size().y));
    let want = size.x / size.y;
    if e.x / e.y < want {
        c.x -= (e.y * want - e.x) / 2.0;
        e.x = e.y * want;
    } else {
        c.y -= (e.x / want - e.y) / 2.0;
        e.y = e.x / want;
    }
    let origin = Vec2::new(AXIS.x / px - half.x / px, AXIS.y - half.y);
    SubCameraView { full_size: full.as_uvec2(), offset: (c - origin) * k, size: (e * k).as_uvec2() }
}

/// The inspect screen's buttons on any pad (R2, L2, △), and 1/2/3 on the keyboard, for every preview.
fn act(
    pads: Query<&Gamepad>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cams: Query<&mut Preview>,
    mut rigs: Query<(&mut Motion, &mut Transform)>,
    data: Query<&character::Rig>,
) {
    let pressed = |b: GamepadButton, k: KeyCode| keys.just_pressed(k) || pads.iter().any(|p| p.just_pressed(b));
    let act = if pressed(GamepadButton::RightTrigger2, KeyCode::Digit1) {
        Act::Win
    } else if pressed(GamepadButton::LeftTrigger2, KeyCode::Digit2) {
        Act::Loss
    } else if pressed(GamepadButton::North, KeyCode::Digit3) {
        Act::Pose
    } else {
        return;
    };
    for mut p in &mut cams {
        let Ok(rig) = data.get(p.rig) else { continue };
        let d = rig.data.clone();
        apply(&mut p, act, &mut rigs, |id| d.motions.get(&id).map_or(0.0, |c| c.length));
    }
}

/// Disc character `n`'s hand (+1 right, −1 left) from TParam.csv's 利き腕 column (右 = right), as `play.rs` reads it.
pub fn disc_hand(iso: &mut Iso, n: usize) -> f32 {
    let row = iso.read("PCDATA/PCDATA.XB").ok().and_then(|d| {
        let arc = Archive::parse(&d).ok()?;
        let csv = arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv"))?).ok()?;
        let tag = format!("{n},");
        csv.split(|&b| b == b'\n').find(|l| l.starts_with(tag.as_bytes())).map(<[u8]>::to_vec)
    });
    // 右 in Shift-JIS
    if row.is_some_and(|r| r.split(|&b| b == b',').any(|c| c.trim_ascii() == [0x89, 0x45])) { 1.0 } else { -1.0 }
}

/// The roster `--inspect` shows: (disc character, costume) pairs and mod folders.
#[derive(Resource)]
pub struct Roster(pub Vec<(usize, usize)>, pub Vec<String>);

/// `--inspect`: every roster entry's preview side by side across the window.
pub fn viewer(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_plugins(plugin)
        .add_systems(PostStartup, spawn_roster)
        // the inspect screen's panel pink, so blended edges read as there
        .add_systems(Startup, |mut c: ResMut<ClearColor>| c.0 = Color::srgb_u8(215, 135, 135))
        .add_systems(FixedUpdate, character::tick)
        .add_systems(Update, (character::animate, lay_out));
}

#[allow(clippy::too_many_arguments)]
fn spawn_roster(
    mut commands: Commands,
    args: Res<crate::Args>,
    roster: Res<Roster>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let mut layer = 1;
    for &(n, costume) in &roster.0 {
        match character::load_disc(&mut iso, n, costume, &mut meshes, &mut materials, &mut images, &mut bindposes) {
            Ok(d) => {
                let hand = disc_hand(&mut iso, n);
                spawn(&mut commands, &Arc::new(d), n, hand, layer, Rect::default());
                layer += 1;
            }
            Err(e) => warn!("character {n} costume {costume}: {e}"),
        }
    }
    for dir in &roster.1 {
        match crate::mods::read(dir.as_ref()).and_then(|m| Ok((crate::mods::load(&mut iso, &m, 0, &mut meshes, &mut materials, &mut images, &mut bindposes)?, m))) {
            Ok((d, m)) => {
                spawn(&mut commands, &Arc::new(d), m.donor, m.hand, layer, Rect::default());
                layer += 1;
            }
            Err(e) => warn!("{e}"),
        }
    }
}

/// The previews in one row across the window.
fn lay_out(window: Query<&Window, With<PrimaryWindow>>, mut cams: Query<(&Camera, &mut Preview)>) {
    let Ok(w) = window.single() else { return };
    let mut cams: Vec<_> = cams.iter_mut().collect();
    cams.sort_by_key(|(c, _)| c.order);
    let size = Vec2::new(w.width() / cams.len().max(1) as f32, w.height());
    for (k, (_, mut p)) in cams.into_iter().enumerate() {
        p.rect = Rect::from_corners(Vec2::new(size.x * k as f32, 0.0), Vec2::new(size.x * (k + 1) as f32, size.y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hst_data::mdl;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// Brad's model x/z with the pelvis over the spot, against the original's root matrix (PCSX2, inspect screen,
    /// research/b40b3_probe.py: context/b40b3/c5_r2.tsv): the held pose after △, then gu_set (R2) as it plays.
    #[test]
    fn pelvis_centre_matches_the_game() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        let data = iso.read("PC/PC05C00.XB").unwrap();
        let arc = Archive::parse(&data).unwrap();
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("_c00.mdl")).unwrap();
        let model = mdl::parse(&arc.read(e).unwrap()).unwrap();
        let sk = Skeleton { names: model.node_names.clone(), parent: model.node_parent.clone(), rest: model.node_local.clone() };
        let m = character::disc_motions(&mut iso, 5, &sk, |_| None).unwrap();
        // (motion, frame drawn, root x, root z); the game's motion clock reads one frame on (it ticks after the draw)
        let game = [
            (POSE, 66.0, -0.00028384244, 8.001564),
            (0x2e, 0.0, -2.0148352e-06, 7.8515825),
            (0x2e, 12.0, -2.0148425e-06, 7.7896185),
            (0x2e, 15.0, -2.1597882e-06, 7.8467035),
            (0x2e, 59.0, 4.2081205e-09, 7.888503),
            (0x2e, 120.0, -1.2543314e-11, 7.960661),
        ];
        for (id, t, x, z) in game {
            let c = pelvis_centre(&sk, &m.motions[&id], t, PI, 1.0).unwrap();
            assert!((c.x - x).abs() < 1e-5 && (c.y - z).abs() < 1e-5, "motion {id:#x} frame {t}: {c} vs ({x}, {z})");
        }
    }
}
