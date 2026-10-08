//! HUD blended the way the PS2's GS blends it: on the gamma-encoded values, not in linear light.
//!
//! The 3D cameras draw into `scene`, the UI draws on its own camera into `hud` (cleared transparent, so it holds the
//! UI premultiplied: colour·a, a), and a last camera puts `enc(hud / a)·a + enc(scene)·(1 − a)` on the window.
//! Bevy blends the UI into an sRGB target in linear light, which makes translucent pieces read stronger (the serve
//! panel's 35 % strip ≈ 0.45 over the court instead of ≈ 0.3).
//!
//! ponytail: UI pieces overlapping each other still blend in linear among themselves; only their sum goes over the
//! court in gamma. Fine while translucent HUD pieces don't stack; else draw the UI with raw (non-sRGB) values.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureFormat};
use bevy::shader::ShaderRef;
use bevy::window::PrimaryWindow;

pub fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "hud_gamma.wgsl");
    app.add_plugins(UiMaterialPlugin::<Composite>::default())
        .add_systems(Startup, setup)
        .add_systems(PostUpdate, (retarget, follow_window).before(bevy::camera::CameraUpdateSystems));
}

#[derive(Resource)]
struct Targets {
    scene: Handle<Image>,
    hud: Handle<Image>,
    composite: Handle<Composite>,
}

#[derive(AsBindGroup, Asset, TypePath, Clone)]
struct Composite {
    #[texture(0)]
    scene: Handle<Image>,
    #[texture(1)]
    hud: Handle<Image>,
}

impl UiMaterial for Composite {
    fn fragment_shader() -> ShaderRef {
        "embedded://hst/hud_gamma.wgsl".into()
    }
}

/// The composite camera's layer, so no sprite or mesh of the game's ends up on it.
const LAYER: usize = 31;

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut materials: ResMut<Assets<Composite>>) {
    let target = || Image::new_target_texture(1, 1, TextureFormat::Rgba8UnormSrgb, None);
    let (scene, hud) = (images.add(target()), images.add(target()));
    commands.spawn((
        Camera2d,
        Camera { order: 50, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        RenderTarget::Image(ImageRenderTarget { handle: hud.clone(), scale_factor: 1.0 }),
        IsDefaultUiCamera,
        Msaa::Off,
    ));
    let composite = materials.add(Composite { scene: scene.clone(), hud: hud.clone() });
    let out = commands
        .spawn((Camera2d, Camera { order: 51, ..default() }, Msaa::Off, RenderLayers::layer(LAYER)))
        .id();
    commands.spawn((
        Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        MaterialNode(composite.clone()),
        UiTargetCamera(out),
    ));
    commands.insert_resource(Targets { scene, hud, composite });
}

/// Every 3D camera drawing to the window draws into the scene image instead.
fn retarget(t: Res<Targets>, mut q: Query<&mut RenderTarget, Added<Camera3d>>) {
    for mut target in &mut q {
        if matches!(*target, RenderTarget::Window(_)) {
            *target = RenderTarget::Image(ImageRenderTarget { handle: t.scene.clone(), scale_factor: 1.0 });
        }
    }
}

/// Both images at the window's physical size and scale factor, as the window target was.
fn follow_window(
    t: Res<Targets>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<Composite>>,
    mut targets: Query<&mut RenderTarget>,
    mut stale: Local<u8>,
) {
    let Ok(w) = window.single() else { return };
    let size = Extent3d { width: w.physical_width().max(1), height: w.physical_height().max(1), depth_or_array_layers: 1 };
    let scale = w.scale_factor();
    for h in [&t.scene, &t.hud] {
        if images.get(h).is_some_and(|i| i.texture_descriptor.size != size) {
            images.get_mut(h).unwrap().resize(size);
            *stale = 2;
        }
    }
    // the material's bind group holds the old texture views until it is prepared again, which can happen before the
    // resized images are: again the frame after
    if *stale > 0 {
        *stale -= 1;
        materials.get_mut(&t.composite).map(|m| m.into_inner());
    }
    for mut target in &mut targets {
        if let RenderTarget::Image(i) = &*target
            && (i.handle == t.scene || i.handle == t.hud)
            && i.scale_factor != scale
        {
            let handle = i.handle.clone();
            *target = RenderTarget::Image(ImageRenderTarget { handle, scale_factor: scale });
        }
    }
}
