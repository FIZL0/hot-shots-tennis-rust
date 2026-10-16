//! B524: the splash's sharp court backdrops (ours), baked during a match on the court it already has loaded: never a
//! court loaded just for this (a first use in Safari freezes on it). Once a match, a moment into a point's end (the
//! umpire's call; the last point's too, at the match end), if this court has no backdrop for this window yet, one
//! extra camera draws the court from a fixed spot ([`view`], the disc's court photo's kind of angle: low behind a
//! baseline, the net across the bottom and the court's buildings and trees above) into an image of the window's
//! shape at most ~1080 lines ([`size`]), read back and kept as a PNG beside the settings (`store::write_backdrop`;
//! the browser's card store). Only the court's own meshes are on its layer (`COURT_LAYER`: the court load's and a court
//! mod's models, the clouds), so no player, ball, effect or HUD is in it; the players' shadows on the ground can be.
//! A backdrop of another size (the window grew or shrank by a lot, or changed shape) is drawn again.
//!
//! Only reads the match: no generator draw, nothing of the simulation or its timing changes; a rainy or snowy match
//! doesn't bake (its sky and darkness would stay on the splash).
//!
//! `HST_BACKDROP=off` never bakes; `HST_BACKDROP=now` bakes `HST_BACKDROP_AT` seconds (default 2) into any match,
//! whatever the phase and the weather, and under `--shot` too (by default a `--shot` run doesn't write one);
//! `HST_BACKDROP=on` bakes as a match does (a point's end, a fair sky, no fitting backdrop yet), `--shot` or not;
//! `HST_BACKDROP_VIEW=x,y,z,yaw,pitch[,fov]` (Bevy space, radians) tries another camera.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::tasks::{IoTaskPool, Task};
use bevy::window::PrimaryWindow;
use std::sync::{Arc, Mutex};

use super::{Game, Phase};
use crate::{Args, COURT_LAYER, Orbit, store};

/// Seconds into a point's end before the bake.
const INTO_POST: f32 = 0.6;
/// Frames the bake camera draws before its picture is read (a new view's first frame can lack a mesh or a shadow).
const DRAWS: u32 = 3;
/// Frames to wait at most for the pipelines and the read-back before giving up for this match.
const GIVE_UP: u32 = 240;
/// The most lines a backdrop has, and columns (a very wide window).
const MAX: [u32; 2] = [2560, 1080];

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Bake>().add_systems(Last, bake);
}

#[derive(Default)]
enum Step {
    #[default]
    Waiting,
    /// The camera drawing (frames so far).
    Drawing { cam: Entity, image: Handle<Image>, size: UVec2, frames: u32, reading: Option<Entity>, got: Arc<Mutex<Option<Vec<u8>>>> },
    /// The PNG encoding and writing.
    Saving(Task<()>),
    Done,
}

#[derive(Resource, Default)]
struct Bake {
    step: Step,
    /// Seconds in the current point's end (0 outside one).
    post: f32,
    /// Seconds into the match (`HST_BACKDROP=now`).
    t: f32,
}

/// The match's court's backdrop key: the court mod drawn, else the disc court `--stage` draws.
fn key(args: &Args, court_mod: Option<&crate::mods_court::MatchCourt>) -> Option<String> {
    match court_mod {
        Some(c) => Some(store::backdrop_key(None, Some(&c.0.id))),
        None => Some(store::backdrop_key(Some(args.stage?.checked_sub(1)? as usize), None)),
    }
}

/// The backdrop's size for a window of `w` × `h` physical pixels: its shape, at most [`MAX`], columns a multiple of 64
/// (the read-back's rows then need no padding).
pub(crate) fn size(w: u32, h: u32) -> UVec2 {
    let (w, h) = (w.max(1) as f32, h.max(1) as f32);
    let k = (MAX[0] as f32 / w).min(MAX[1] as f32 / h).min(1.0);
    let cols = ((w * k / 64.0).round() as u32).max(1) * 64;
    UVec2::new(cols, ((h * k).round() as u32).max(1))
}

/// Whether a backdrop `had` (its PNG's size) is fit for `want`: within a fifth in lines and a tenth in shape.
pub(crate) fn fits(had: [u32; 2], want: UVec2) -> bool {
    let (ha, wa) = (had[0] as f32 / had[1].max(1) as f32, want.x as f32 / want.y.max(1) as f32);
    (had[1] as f32 / want.y as f32 - 1.0).abs() <= 0.2 && (ha / wa - 1.0).abs() <= 0.1
}

/// The bake camera: eye and turn (Bevy space; yaw about +y, 0 looking along −z; pitch up positive) and vertical
/// field of view. Every court's centre is the origin, its length along z: 1.4 m up inside the −z half, off to the
/// +x side, looking across the net at the far end, the umpire's chair at the net's right end and turned a little up,
/// as the disc's court-00 photo is taken (by eye; every court checked at it).
fn view() -> (Vec3, f32, f32, f32) {
    if let Ok(v) = std::env::var("HST_BACKDROP_VIEW") {
        let f: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        if f.len() >= 5 {
            return (Vec3::new(f[0], f[1], f[2]), f[3], f[4], f.get(5).copied().unwrap_or(0.9));
        }
    }
    (Vec3::new(3.0, 1.4, -9.0), 2.72, 0.2, 0.7)
}

#[allow(clippy::too_many_arguments)]
fn bake(
    mut commands: Commands,
    mut bake: ResMut<Bake>,
    g: Option<Res<Game>>,
    args: Option<Res<Args>>,
    court_mod: Option<Res<crate::mods_court::MatchCourt>>,
    images: Option<ResMut<Assets<Image>>>,
    window: Query<&Window, With<PrimaryWindow>>,
    main: Query<(&Msaa, &Transform), With<Orbit>>,
    colour: Option<Res<ClearColor>>,
    compiling: Option<Res<crate::mode::Compiling>>,
    time: Res<Time>,
    mut cams: Query<&mut Camera, With<Backdrop>>,
) {
    let (Some(g), Some(args), Some(mut images)) = (g, args, images) else { return };
    let mode = std::env::var("HST_BACKDROP").unwrap_or_default();
    let b = &mut *bake;
    match &mut b.step {
        Step::Done => {}
        Step::Waiting => {
            let now = mode == "now";
            if mode == "off" || (args.shot.is_some() && !now && mode != "on") {
                b.step = Step::Done;
                return;
            }
            b.t += time.delta_secs();
            b.post = if g.phase == Phase::Post { b.post + time.delta_secs() } else { 0.0 };
            let at = std::env::var("HST_BACKDROP_AT").ok().and_then(|s| s.parse().ok()).unwrap_or(2.0);
            let ready = if now { b.t >= at } else { b.post >= INTO_POST && crate::weather::now() < 2 };
            if !ready {
                return;
            }
            let (Ok(w), Some(key)) = (window.single(), key(&args, court_mod.as_deref())) else {
                b.step = Step::Done;
                return;
            };
            let want = size(w.physical_width(), w.physical_height());
            if !now && store::backdrop_size(&args.iso, &key).is_some_and(|had| fits(had, want)) {
                b.step = Step::Done;
                return;
            }
            let mut target = Image::new_target_texture(want.x, want.y, TextureFormat::Rgba8Unorm, None);
            target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
            target.data = None;
            let image = images.add(target);
            let (eye, yaw, pitch, fov) = view();
            // the scene cameras' clear (`hud_gamma::clear`): the encoded colour as it is
            let c = colour.map_or(Color::BLACK, |c| c.0).to_srgba();
            let cam = commands
                .spawn((
                    Camera3d::default(),
                    Camera { order: -20, clear_color: ClearColorConfig::Custom(Color::linear_rgba(c.red, c.green, c.blue, c.alpha)), ..default() },
                    RenderTarget::Image(ImageRenderTarget { handle: image.clone(), scale_factor: 1.0 }),
                    // as the scene camera draws (`hud_gamma::retarget`), so its pipelines are the ones already built
                    bevy::core_pipeline::tonemapping::Tonemapping::None,
                    bevy::core_pipeline::tonemapping::DebandDither::Disabled,
                    main.iter().next().map(|m| *m.0).unwrap_or_default(),
                    Projection::Perspective(PerspectiveProjection { fov, near: 0.1, far: 20_000.0, ..default() }),
                    Transform::from_translation(eye).with_rotation(Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)),
                    RenderLayers::layer(COURT_LAYER),
                    // from the start (`graphics::direct_draws` adds it a frame late): a view that switches to direct
                    // draws after its first frame never draws its opaque and alpha-tested meshes
                    bevy::render::view::NoIndirectDrawing,
                    Backdrop,
                ))
                .id();
            info!("baking the splash backdrop {key} at {}×{} (the match camera at {:?})", want.x, want.y, main.iter().next().map(|m| m.1));
            b.step = Step::Drawing { cam, image, size: want, frames: 0, reading: None, got: default() };
        }
        Step::Drawing { cam, image, size, frames, reading, got } => {
            *frames += 1;
            let data = got.lock().unwrap().take();
            if let Some(data) = data {
                commands.entity(*cam).despawn();
                if let Some(r) = reading.take() {
                    commands.entity(r).despawn();
                }
                images.remove(image.id());
                let (Some(key), iso, size) = (key(&args, court_mod.as_deref()), args.iso.clone(), *size) else {
                    b.step = Step::Done;
                    return;
                };
                b.step = Step::Saving(IoTaskPool::get().spawn(async move { save(&iso, &key, &data, size) }));
            } else if *frames > GIVE_UP {
                warn!("splash backdrop: no picture after {GIVE_UP} frames");
                commands.entity(*cam).despawn();
                if let Some(r) = reading.take() {
                    commands.entity(r).despawn();
                }
                b.step = Step::Done;
            } else if *frames >= DRAWS && reading.is_none() && !compiling.is_some_and(|c| c.busy()) {
                // drawn: the camera stops, its picture stays in the image for the read
                if let Ok(mut c) = cams.get_mut(*cam) {
                    c.is_active = false;
                }
                let got = got.clone();
                let r = commands
                    .spawn(Readback::texture(image.clone()))
                    .observe(move |ev: On<ReadbackComplete>, mut commands: Commands| {
                        got.lock().unwrap().get_or_insert_with(|| ev.data.clone());
                        commands.entity(ev.entity).try_remove::<Readback>();
                    })
                    .id();
                *reading = Some(r);
            }
        }
        Step::Saving(task) => {
            if task.is_finished() {
                b.step = Step::Done;
            }
        }
    }
}

/// The bake camera (`HST_SCENE_CENSUS` and the like tell it apart).
#[derive(Component)]
pub struct Backdrop;

/// The read-back (rows padded to 256 bytes) as an opaque PNG.
fn save(iso: &str, key: &str, data: &[u8], size: UVec2) {
    let rgba = unpad(data, size);
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new_with_quality(&mut png, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::Adaptive);
    let r = image::ImageEncoder::write_image(encoder, &rgba, size.x, size.y, image::ExtendedColorType::Rgb8).map_err(|e| e.to_string());
    match r.and_then(|()| store::write_backdrop(iso, key, png).map_err(|e| e.to_string())) {
        Ok(()) => info!("baked the splash backdrop {key}"),
        Err(e) => warn!("splash backdrop {key}: {e}"),
    }
}

/// `size`'s RGB bytes from a read-back of RGBA rows each padded to a multiple of 256 bytes.
fn unpad(data: &[u8], size: UVec2) -> Vec<u8> {
    let row = (size.x as usize * 4).div_ceil(256) * 256;
    let mut out = Vec::with_capacity(size.x as usize * size.y as usize * 3);
    for y in 0..size.y as usize {
        let Some(r) = data.get(y * row..y * row + size.x as usize * 4) else { break };
        r.as_chunks::<4>().0.iter().for_each(|p| out.extend_from_slice(&p[..3]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_cap_at_1080_and_refit() {
        assert_eq!(size(1920, 1080), UVec2::new(1920, 1080));
        assert_eq!(size(3840, 2160), UVec2::new(1920, 1080));
        assert_eq!(size(1280, 720), UVec2::new(1280, 720));
        let wide = size(5120, 1440);
        assert!(wide.x <= 2560 + 32 && wide.y <= 1080 && wide.x.is_multiple_of(64), "{wide}");
        assert_eq!(size(1000, 1000).x % 64, 0);
        assert!(fits([1920, 1080], size(1920, 1080)));
        assert!(fits([1920, 1080], size(1760, 990)));
        assert!(!fits([1280, 720], size(1920, 1080)), "a much bigger window bakes again");
        assert!(!fits([1920, 1080], size(1080, 1920)), "so does another shape");
    }

    #[test]
    fn unpads_rows() {
        let size = UVec2::new(3, 2);
        let mut data = vec![0u8; 512];
        data[..12].copy_from_slice(&[1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255]);
        data[256..268].copy_from_slice(&[10, 11, 12, 255, 13, 14, 15, 255, 16, 17, 18, 255]);
        assert_eq!(unpad(&data, size), (1..=18).collect::<Vec<u8>>());
    }

    #[test]
    fn keys() {
        assert_eq!(store::backdrop_key(Some(3), None), "court03");
        assert_eq!(store::backdrop_key(None, Some("gag c/03")), "mod-gag_c_03");
        assert_eq!(store::png_size(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\x07\x80\0\0\x048"), Some([1920, 1080]));
    }
}

