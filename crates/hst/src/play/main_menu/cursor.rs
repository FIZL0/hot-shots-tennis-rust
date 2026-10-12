//! The menus' pointing hand (B40e1): the original's menu overlay draws `MENU00B.XB0`'s `menu/3d/cursor` model, a
//! skinned 3D hand playing `cursor.ani` (its fingers and wrist wag in place; the hand doesn't slide), not the pause
//! menu's swinging `hsm_yubi` sprite. Drawn here by its own camera into an image that the layout's `Tex::Cursor`
//! quads show, so the hand stays a quad of the 640×448 layout at any aspect ratio.

use bevy::app::Propagate;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget, ScalingMode};
use bevy::mesh::morph::MorphWeights;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::effect::Effect;

use crate::Args;
use crate::effects::Shown;

/// Its render layer (the select's previews use 1–4 and 6).
const LAYER: usize = 8;
/// The image: the quad's 64×32 at 4×.
const SIZE: UVec2 = UVec2::new(256, 128);
/// The box the image shows (x, y centre after the root's turn; width; the height is half of it): the hand and
/// fingers of `cursor.ani`, fingertip right as the original's.
const VIEW: [f32; 3] = [-0.09, -0.02, 0.3];

#[derive(Resource)]
pub(super) struct Cursor {
    pub(super) image: Handle<Image>,
    effect: Effect,
    shown: Shown,
    acc: f32,
}

pub(super) fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let Some(arc) = Iso::open(&args.iso).ok().and_then(|mut iso| iso.read("MENU/MENU00B.XB0").ok()) else { return };
    let Ok(arc) = Archive::parse(&arc) else { return };
    let layers = RenderLayers::layer(LAYER);
    // game space (y down) → Bevy, seen from the side the original shows: fingertip right, back of the hand front
    let root = commands.spawn((Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::PI)), Visibility::default(), Propagate(layers.clone()))).id();
    let Ok((mut effect, shown)) = crate::effects::model(&arc, "menu/3d/cursor", &mut commands, root, &mut meshes, &mut materials, &mut images, &mut bindposes) else {
        commands.entity(root).despawn();
        return;
    };
    // the original lights it (VU1): lit here by its own light, grey on the shadowed side
    for m in &shown.materials {
        if let Some(mut m) = materials.get_mut(m) {
            m.unlit = false;
            m.perceptual_roughness = 0.8;
        }
    }
    commands.spawn((DirectionalLight { illuminance: 4000.0, ..default() }, Transform::from_xyz(-1.0, 2.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y), layers.clone()));
    // no MOR: without this the effect ends on its first frame
    effect.looping = true;
    effect.start();
    let image = images.add(Image::new_target_texture(SIZE.x, SIZE.y, bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb, None));
    let [x, y, w] = VIEW;
    commands.spawn((
        Camera3d::default(),
        Camera { order: -1, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        RenderTarget::Image(ImageRenderTarget { handle: image.clone(), scale_factor: 1.0 }),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Msaa::Off,
        Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::Fixed { width: w, height: w / 2.0 }, ..OrthographicProjection::default_3d() }),
        Transform::from_xyz(x, y, 10.0),
        AmbientLight { brightness: 250.0, ..default() },
        layers,
    ));
    commands.insert_resource(Cursor { image, effect, shown, acc: 0.0 });
}

/// The clip at 60 Hz, from its start again at its end.
pub(super) fn tick(
    cursor: Option<ResMut<Cursor>>,
    time: Res<Time>,
    mut q: Query<(&mut Visibility, &mut MorphWeights)>,
    mut joints: Query<&mut Transform>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(mut cursor) = cursor else { return };
    let c = &mut *cursor;
    c.acc += time.delta_secs() * 60.0;
    while c.acc >= 1.0 {
        c.acc -= 1.0;
        c.effect.tick();
        if c.effect.times()[0] >= c.effect.clip.length {
            c.effect.start();
        }
    }
    crate::effects::pose(&c.effect, &c.shown, &mut q, &mut joints, &mut materials);
}
