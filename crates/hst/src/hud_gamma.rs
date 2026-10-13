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
//! While a match runs (`Persist`) the scene also keeps 25 % of the last frame shown, as the game's last draw does:
//! a full-screen sprite of the other frame buffer at FIX 0x20 over the new frame (its fields shifted by half a line
//! to line up, which a progressive frame needs not). Two images take turns as this frame's mix and last frame's.
//! Instead of it, while the camera looks within ~37° of the sun in clear or cloudy weather, the game blurs the frame
//! by half a texel twice (`glare`), more the closer to the sun and the higher the camera looks.
//!
//! ponytail: UI pieces overlapping each other still blend in linear among themselves; only their sum goes over the
//! court in gamma. Fine while translucent HUD pieces don't stack; else draw the UI with raw (non-sRGB) values.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{CameraProjection, ImageRenderTarget, RenderTarget};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureFormat};
use bevy::shader::ShaderRef;
use bevy::window::PrimaryWindow;

pub fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "hud_gamma.wgsl");
    bevy::asset::embedded_asset!(app, "persist.wgsl");
    app.add_plugins((UiMaterialPlugin::<Composite>::default(), UiMaterialPlugin::<Mix>::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, encode_pbr_output)
        .add_systems(PostUpdate, (retarget, clear, follow_window, persist).chain().before(bevy::camera::CameraUpdateSystems))
        .add_systems(PostUpdate, pixel_aspect.after(bevy::camera::CameraUpdateSystems));
}

#[derive(Resource)]
struct Targets {
    scene: Handle<Image>,
    hud: Handle<Image>,
    composite: Handle<Composite>,
    /// The persistence's two images, cameras and mixes: frame parity `i` draws `mixes[i]` into `last[i]` with
    /// cameras[i], from the scene and `last[1 - i]`.
    last: [Handle<Image>; 2],
    cameras: [Entity; 2],
    mixes: [Handle<Mix>; 2],
}

/// Present while a match runs (the game's match scene turns its frame persistence on, menus have none).
#[derive(Resource)]
pub struct Persist;

#[derive(AsBindGroup, Asset, TypePath, Clone)]
struct Mix {
    #[texture(0)]
    #[sampler(3)]
    scene: Handle<Image>,
    #[texture(1)]
    last: Handle<Image>,
    /// x: how much of the last frame; y: the sun glare's FIX (0 = none), zw: its texel in uv (vec4 for WebGL's
    /// 16-byte uniforms)
    #[uniform(2)]
    weight: Vec4,
}

impl UiMaterial for Mix {
    fn fragment_shader() -> ShaderRef {
        "embedded://hst/persist.wgsl".into()
    }
}

#[derive(AsBindGroup, Asset, TypePath, Clone)]
struct Composite {
    #[texture(0)]
    #[sampler(2)]
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

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut materials: ResMut<Assets<Composite>>, mut mixes: ResMut<Assets<Mix>>) {
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
    // HDR like the scene cameras, so the mix lands in the image unencoded
    let last = [(); 2].map(|_| images.add(target(TextureFormat::Rgba8Unorm)));
    let mixes = [0, 1].map(|i| mixes.add(Mix { scene: scene.clone(), last: last[1 - i].clone(), weight: Vec4::ZERO }));
    let cameras = [0, 1].map(|i| {
        let cam = commands
            .spawn((
                Camera2d,
                Camera { order: 48 + i as isize, is_active: false, ..default() },
                RenderTarget::Image(ImageRenderTarget { handle: last[i].clone(), scale_factor: 1.0 }),
                bevy::camera::Hdr,
                bevy::core_pipeline::tonemapping::DebandDither::Disabled,
                Msaa::Off,
                RenderLayers::layer(LAYER - 1),
            ))
            .id();
        commands.spawn((
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            MaterialNode(mixes[i].clone()),
            UiTargetCamera(cam),
        ));
        cam
    });
    commands.insert_resource(Targets { scene, hud, composite, last, cameras, mixes });
}

/// A 3D camera drawing into the scene image.
#[derive(Component)]
pub struct Scene;

/// Every 3D camera drawing to the window draws into the scene image instead, HDR so nothing re-encodes its values.
fn retarget(mut commands: Commands, t: Res<Targets>, mut q: Query<(Entity, &mut RenderTarget), Added<Camera3d>>) {
    for (e, mut target) in &mut q {
        if matches!(*target, RenderTarget::Window(_)) {
            *target = RenderTarget::Image(ImageRenderTarget { handle: t.scene.clone(), scale_factor: 1.0 });
            commands.entity(e).try_insert((Scene, bevy::camera::Hdr));
        }
    }
}

/// The scene cameras clear to `ClearColor`'s encoded values.
fn clear(colour: Res<ClearColor>, mut q: Query<&mut Camera, With<Scene>>, added: Query<(), Added<Scene>>) {
    if !colour.is_changed() && added.is_empty() {
        return;
    }
    let c = colour.0.to_srgba();
    // an overlay drawing over the court (`ClearColorConfig::None`) keeps not clearing
    for mut cam in q.iter_mut().filter(|c| !matches!(c.clear_color, ClearColorConfig::None)) {
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

/// Both images at the window's physical size and scale factor, as the window target was; the scene's times the
/// render scale setting (the composite upscales it). The original mode (`Graphics::original`) draws the scene at 448
/// lines and 640 columns per 4:3 width (the PS2's non-square pixels, `pixel_aspect`) and the HUD at 448 lines, both
/// scaled up unfiltered.
fn follow_window(
    t: Res<Targets>,
    g: Res<crate::graphics::Graphics>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<Composite>>,
    mut mixes: ResMut<Assets<Mix>>,
    mut targets: Query<&mut RenderTarget>,
    mut stale: Local<u8>,
) {
    let Ok(w) = window.single() else { return };
    let (pw, ph) = (w.physical_width() as f32, w.physical_height() as f32);
    let size = |x: f32, y: f32| Extent3d { width: (x.round() as u32).max(1), height: (y.round() as u32).max(1), depth_or_array_layers: 1 };
    let ((scene, ks), (hud, kh)) = if g.original {
        let k = 448.0 / ph.max(1.0);
        ((size(480.0 * pw / ph.max(1.0), 448.0), k), (size(pw * k, 448.0), k))
    } else {
        let k = g.scale as f32 / 100.0;
        ((size((pw * k).floor(), (ph * k).floor()), k), (size(pw, ph), 1.0))
    };
    let sampler = if g.original { ImageSampler::nearest() } else { ImageSampler::Default };
    for (h, size) in [(&t.scene, scene), (&t.hud, hud), (&t.last[0], scene), (&t.last[1], scene)] {
        if images.get(h).is_some_and(|i| i.texture_descriptor.size != size) {
            images.get_mut(h).unwrap().resize(size);
            *stale = 2;
        }
    }
    // the composite samples whichever of these it shows
    for h in [&t.scene, &t.last[0], &t.last[1]] {
        if images.get(h).is_some_and(|i| i.sampler != sampler) {
            images.get_mut(h).unwrap().sampler = sampler.clone();
            *stale = 2;
        }
    }
    // the material's bind group holds the old texture views until it is prepared again, which can happen before the
    // resized images are: again the frame after
    if *stale > 0 {
        *stale -= 1;
        materials.get_mut(&t.composite).map(|m| m.into_inner());
        for m in &t.mixes {
            mixes.get_mut(m).map(|m| m.into_inner());
        }
    }
    for mut target in &mut targets {
        let Some(scale) = (match &*target {
            RenderTarget::Image(i) if i.handle == t.scene => Some(w.scale_factor() * ks),
            RenderTarget::Image(i) if i.handle == t.hud => Some(w.scale_factor() * kh),
            _ => None,
        }) else {
            continue;
        };
        if let RenderTarget::Image(i) = &*target
            && i.scale_factor != scale
        {
            let handle = i.handle.clone();
            *target = RenderTarget::Image(ImageRenderTarget { handle, scale_factor: scale });
        }
    }
}

/// Each frame in a match: this parity's camera mixes the last frame into the scene, and the composite shows that.
/// The game mixes 25 % per 60 Hz frame; at a higher frame rate more per frame, so the trail fades as fast.
#[allow(clippy::too_many_arguments)]
fn persist(
    on: Option<Res<Persist>>,
    time: Res<Time<Real>>,
    t: Res<Targets>,
    sun: Option<Res<crate::shadow::Sun>>,
    view: Query<(&Projection, &Transform), With<crate::Orbit>>,
    images: Res<Assets<Image>>,
    mut cams: Query<&mut Camera>,
    mut mixes: ResMut<Assets<Mix>>,
    mut materials: ResMut<Assets<Composite>>,
    mut parity: Local<usize>,
) {
    *parity ^= 1;
    let i = *parity;
    for (k, &e) in t.cameras.iter().enumerate() {
        if let Ok(mut c) = cams.get_mut(e)
            && c.is_active != (on.is_some() && k == i)
        {
            c.is_active = !c.is_active;
        }
    }
    let shown = if on.is_some() {
        if let Some(mut m) = mixes.get_mut(&t.mixes[i]) {
            let fix = match (sun, view.single()) {
                (Some(sun), Ok((Projection::Perspective(p), tf))) if crate::weather::now() < 2 => glare(sun.light, p, tf.rotation),
                _ => 0.0,
            };
            // the game's field texel: 640 columns per 4:3 width, 224 lines
            let size = images.get(&t.scene).map_or(UVec2::ONE, |i| i.size()).as_vec2();
            let texel = Vec2::new(size.y * 4.0 / 3.0 / 640.0 / size.x, 1.0 / 224.0);
            let w = if fix > 0.0 { 0.0 } else { 0.25f32.powf((time.delta_secs() * 60.0).clamp(0.25, 1.0)) };
            m.weight = Vec4::new(w, fix, texel.x, texel.y);
        }
        &t.last[i]
    } else {
        &t.scene
    };
    if materials.get(&t.composite).is_some_and(|c| c.scene != *shown) {
        materials.get_mut(&t.composite).unwrap().scene = shown.clone();
    }
}

/// The sun glare's blend weight (FIX / 128, 0 = off) for the match camera turned by `rot` (Bevy space) with
/// projection `p` and the sun lighting along `light` (game space, y down, sun → ground): on while the view is within
/// acos 0.8 of the sun, ((cos − 0.8)/0.2·0.5 + seen·0.2) · how far it looks up (clamped to 0..0.8)/0.8 · 128/0.7.
/// ponytail: `seen` is the lens flare's read-back of how much of the sun shows (0..1); taken as 1 while the sun is in
/// the picture, as `play/flare.rs` does, so a player in front of the sun doesn't weaken it.
fn glare(light: Vec3, p: &PerspectiveProjection, rot: Quat) -> f32 {
    let s = -light.normalize();
    let sun = Vec3::new(s.x, -s.y, -s.z);
    let fwd = rot * Vec3::NEG_Z;
    let c = fwd.dot(sun);
    if c <= 0.8 {
        return 0.0;
    }
    let sv = rot.inverse() * sun;
    let tan = (p.fov / 2.0).tan();
    let seen = (sv.x / -sv.z).abs() <= tan * p.aspect_ratio && (sv.y / -sv.z).abs() <= tan;
    let fix = ((c - 0.8) / 0.2 * 0.5 + if seen { 0.2 } else { 0.0 }) * (fwd.y.clamp(0.0, 0.8) / 0.8) * 128.0 / 0.7;
    (fix as u8) as f32 / 128.0
}

/// In the original mode the scene image's pixels aren't square (640 per 4:3 width): project for the window's aspect,
/// not the image's. Bevy recomputes `clip_from_view` from the image's aspect whenever the projection changes, so this
/// runs after it every frame. The frustum stays the image's, a little wider than the view: it only culls less.
fn pixel_aspect(g: Res<crate::graphics::Graphics>, window: Query<&Window, With<PrimaryWindow>>, mut q: Query<(&mut Camera, &Projection), With<Scene>>) {
    let Ok(w) = window.single() else { return };
    if !g.original {
        return;
    }
    let aspect = w.width() / w.height().max(1.0);
    for (mut cam, proj) in &mut q {
        if let Projection::Perspective(p) = proj {
            let clip = PerspectiveProjection { aspect_ratio: aspect, ..p.clone() }.get_clip_from_view();
            if cam.computed.clip_from_view != clip {
                cam.computed.clip_from_view = clip;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glare_strength() {
        let p = PerspectiveProjection { fov: 0.8, aspect_ratio: 4.0 / 3.0, ..default() };
        // sun 40° up, straight ahead (game space: −z ahead of a camera looking along Bevy −z, y down)
        let (e, look) = (40f32.to_radians(), |pitch: f32| Quat::from_rotation_x(pitch.to_radians()));
        let light = -Vec3::new(0.0, -e.sin(), e.cos());
        // looking straight at it: cos 1, sun in the picture, looking up sin 40° → ((0.5 + 0.2)·0.643/0.8·128/0.7) = 102
        assert_eq!(glare(light, &p, look(40.0)), 102.0 / 128.0);
        // looking level: 40° off (cos 0.766 < 0.8) → off; looking down → off
        assert_eq!(glare(light, &p, look(0.0)), 0.0);
        assert_eq!(glare(light, &p, look(-20.0)), 0.0);
    }
}
