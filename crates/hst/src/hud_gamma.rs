//! Scene and HUD blended the way the PS2's GS blends: on the gamma-encoded values, not in linear light.
//!
//! The 3D cameras draw HDR (float, nothing encoded) into `scene` (raw `Rgba8Unorm`), and every 3D shader outputs
//! encoded colour: `gs.wgsl` as it computes it, Bevy's PBR shader (`StandardMaterial`) through a patched
//! `main_pass_post_lighting_processing` that encodes its result; the clear colour follows `ClearColor` encoded. So
//! translucent and `@add` draws blend on encoded values as the GS does (grass blue 0x33 + 0x4c is 0x7f, not the
//! 0x98 a linear-light add gives).
//! The UI draws on its own camera into `hud` (cleared transparent, so it holds the UI premultiplied: colour·a, a),
//! and a last camera puts `enc(hud / a)·a + scene·(1 − a)` on the window. Bevy blends the UI into an sRGB target in
//! linear light, which makes translucent pieces read stronger (the serve panel's 35 % strip ≈ 0.45 over the court
//! instead of ≈ 0.3).
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
        .add_systems(Update, encode_pbr_output)
        .add_systems(PostUpdate, (retarget, clear, follow_window).chain().before(bevy::camera::CameraUpdateSystems));
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
    let target = |f| Image::new_target_texture(1, 1, f, None);
    let (scene, hud) = (images.add(target(TextureFormat::Rgba8Unorm)), images.add(target(TextureFormat::Rgba8UnormSrgb)));
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

/// A 3D camera drawing into the scene image.
#[derive(Component)]
struct Scene;

/// Every 3D camera drawing to the window draws into the scene image instead, HDR so nothing re-encodes its values.
fn retarget(mut commands: Commands, t: Res<Targets>, mut q: Query<(Entity, &mut RenderTarget), Added<Camera3d>>) {
    for (e, mut target) in &mut q {
        if matches!(*target, RenderTarget::Window(_)) {
            *target = RenderTarget::Image(ImageRenderTarget { handle: t.scene.clone(), scale_factor: 1.0 });
            commands.entity(e).insert((Scene, bevy::camera::Hdr));
        }
    }
}

/// The scene cameras clear to `ClearColor`'s encoded values.
fn clear(colour: Res<ClearColor>, mut q: Query<&mut Camera, With<Scene>>, added: Query<(), Added<Scene>>) {
    if !colour.is_changed() && added.is_empty() {
        return;
    }
    let c = colour.0.to_srgba();
    for mut cam in &mut q {
        cam.clear_color = ClearColorConfig::Custom(Color::linear_rgba(c.red, c.green, c.blue, c.alpha));
    }
}

/// Bevy's PBR fragment (every `StandardMaterial`) ends with the encoded colour, as `gs.wgsl` does.
fn encode_pbr_output(mut events: MessageReader<AssetEvent<Shader>>, mut shaders: ResMut<Assets<Shader>>) {
    const AT: &str = "#ifdef PREMULTIPLY_ALPHA\n    output_color = premultiply_alpha(pbr_input.material.flags, output_color);\n#endif\n    return output_color;";
    const ENC: &str = "    output_color = vec4(select(1.055 * pow(output_color.rgb, vec3(1.0 / 2.4)) - 0.055, output_color.rgb * 12.92, output_color.rgb <= vec3(0.0031308)), output_color.a);\n";
    for e in events.read() {
        let AssetEvent::Added { id } = e else { continue };
        let Some(shader) = shaders.get(*id) else { continue };
        if !shader.path.ends_with("bevy_pbr/render/pbr_functions.wgsl") {
            continue;
        }
        let bevy::shader::Source::Wgsl(src) = &shader.source else { continue };
        assert_eq!(src.matches(AT).count(), 1, "Bevy's pbr_functions.wgsl changed: re-find where its forward output ends");
        let patched = src.replace(AT, &format!("{ENC}{AT}"));
        shaders.get_mut(*id).unwrap().source = bevy::shader::Source::Wgsl(patched.into());
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
