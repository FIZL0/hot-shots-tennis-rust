//! The sun's lens flare (clear or cloudy weather): ghost sprites along the line from the sun through the middle of
//! the picture (`common/flare/flare3.bmp`'s cells), 24 rays round the sun (`Rngs::flare`'s brightness and angle
//! jitter, boosted toward the middle of the picture) and a whiteout when looking straight at it, all added onto the
//! scene. Built each frame as meshes just in front of the match camera, in its tangent space (x right, y up at
//! distance 1), so they follow its field of view at any aspect ratio.
//!
//! ponytail: the game fades it by how much of a 16×8 pixel window round the sun a depth-tested marker still covers
//! (read back from the frame buffer); here it is fully on while the sun is in the picture, so a player or the stands
//! in front of the sun don't dim it.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{Mesh, PrimitiveTopology};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use hst_data::{iso::Iso, xb::Archive};

use super::Game;
use crate::{Args, Orbit};

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, draw.before(TransformSystems::Propagate));
}

/// The flare's elements as the game makes them: position along the sun → middle line, sprite cell (6 = the rays),
/// size (640×448 pixels; the rays' follows the sun's height) and colour (0..255, × the sun's colour / 255).
const ELEMENTS: [(f32, u8, f32, [f32; 3]); 12] = [
    (0.0, 6, 0.0, [128.0, 128.0, 128.0]),
    (0.55, 1, 24.0, [36.0, 36.0, 32.0]),
    (0.70, 2, 72.0, [16.0, 16.0, 19.0]),
    (0.80, 1, 36.0, [16.0, 16.0, 19.0]),
    (0.90, 2, 24.0, [36.0, 32.0, 32.0]),
    (0.95, 0, 12.0, [32.0, 36.0, 32.0]),
    (1.05, 0, 12.0, [32.0, 36.0, 32.0]),
    (1.07, 3, 200.0, [12.0, 10.0, 10.0]),
    (1.10, 1, 36.0, [20.0, 16.0, 16.0]),
    (1.14, 2, 114.0, [20.0, 16.0, 16.0]),
    (1.16, 1, 24.0, [20.0, 16.0, 16.0]),
    (1.18, 2, 48.0, [16.0, 20.0, 16.0]),
];

/// The sun's and the middle's distances from the eye (m).
const SUN: f32 = 32768.0;
const LOOK: f32 = 8192.0;
/// The rays' sizes are in 1/16 pixels of a projection with 410.29 pixels per unit tangent.
const RAY_UNIT: f32 = 16.0 * 410.29;
/// How far in front of the camera the meshes are (m): just past its 0.1 m near plane.
const DEPTH: f32 = 0.11;
/// Bands each ray's Gouraud falloff is drawn in.
const BANDS: usize = 6;

#[derive(Component)]
struct Part(bool);

/// The flare's sprite sheet: the 4-bit grey BMP, top row first.
fn sheet(iso: &mut Iso) -> Option<Image> {
    let data = iso.read("CMN/GAME.XB").ok()?;
    let arc = Archive::parse(&data).ok()?;
    let b = arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("flare3.bmp"))?).ok()?;
    let w32 = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
    let (off, w, h) = (w32(10), w32(18), w32(22));
    if b[28] != 4 {
        return None;
    }
    let stride = (w * 4).div_ceil(32) * 4;
    let mut px = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let row = &b[off + (h - 1 - y) * stride..];
        for x in 0..w {
            let k = (row[x / 2] >> if x % 2 == 0 { 4 } else { 0 } & 15) as usize;
            let g = b[54 + 4 * k];
            px.extend([g, g, g, 255]);
        }
    }
    let mut img = Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    Some(img)
}

/// A colour as the scene's encoded value `v` (0..1) comes out of the patched PBR shader (`hud_gamma`).
fn enc(v: [f32; 3]) -> [f32; 4] {
    let c = Color::srgb(v[0], v[1], v[2]).to_linear();
    [c.red, c.green, c.blue, 1.0]
}

#[derive(Default)]
struct Built {
    pos: Vec<[f32; 3]>,
    col: Vec<[f32; 4]>,
    uv: Vec<[f32; 2]>,
}

impl Built {
    /// A point at tangent (x right, y up).
    fn v(&mut self, t: Vec2, col: [f32; 4], uv: [f32; 2]) {
        self.pos.push([t.x * DEPTH, t.y * DEPTH, -DEPTH]);
        self.col.push(col);
        self.uv.push(uv);
    }

    fn quad(&mut self, lo: Vec2, hi: Vec2, col: [f32; 4], uv0: [f32; 2], uv1: [f32; 2]) {
        let c = [(lo, uv0), (Vec2::new(hi.x, lo.y), [uv1[0], uv0[1]]), (hi, uv1), (Vec2::new(lo.x, hi.y), [uv0[0], uv1[1]])];
        for k in [0, 1, 2, 0, 2, 3] {
            self.v(c[k].0, col, c[k].1);
        }
    }

    /// Empty: one zero-area triangle, since Bevy's mesh allocator logs an error for a mesh with no vertices (B149).
    fn mesh(mut self) -> Mesh {
        if self.pos.is_empty() {
            (self.pos, self.col, self.uv) = (vec![[0.0; 3]; 3], vec![[0.0; 4]; 3], vec![[0.0; 2]; 3]);
        }
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.col)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
    }
}

/// Spawn the two meshes (sprites, rays and whiteout) under the match camera once, then rebuild them each frame.
#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    args: Option<Res<Args>>,
    g: Option<Res<Game>>,
    sun: Option<Res<crate::shadow::Sun>>,
    cam: Query<(Entity, &Projection, &Transform), With<Orbit>>,
    mut parts: Query<(&Part, &Mesh3d, &mut Visibility)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut spawned: Local<bool>,
) {
    let (Some(g), Some(sun), Ok((camera, Projection::Perspective(p), tf))) = (g, sun, cam.single()) else { return };
    if !*spawned {
        *spawned = true;
        let Some(texture) = args.and_then(|a| Iso::open(&a.iso).ok()).and_then(|mut iso| sheet(&mut iso)) else { return };
        let texture = images.add(texture);
        for textured in [true, false] {
            let m = materials.add(StandardMaterial {
                base_color_texture: textured.then(|| texture.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Add,
                fog_enabled: false,
                cull_mode: None,
                double_sided: true,
                ..default()
            });
            let part = commands
                .spawn((Part(textured), Mesh3d(meshes.add(Built::default().mesh())), MeshMaterial3d(m), Transform::default(), Visibility::Hidden, NoFrustumCulling, NotShadowCaster, NotShadowReceiver))
                .id();
            commands.entity(camera).add_child(part);
        }
        return;
    }
    let (sprites, rays) = build(&g.rng.flare, &sun, p, tf.rotation);
    let on = crate::weather::now() < 2;
    let mut built = [Some(rays), Some(sprites)];
    for (part, mesh, mut vis) in &mut parts {
        let Some(b) = built[part.0 as usize].take() else { continue };
        *vis = if on && !b.pos.is_empty() { Visibility::Inherited } else { Visibility::Hidden };
        // an empty one stays as it was, hidden: Bevy's mesh allocator logs a use-after-free for a mesh with no
        // vertices, twice a frame while the sun is out of the picture (B163)
        if b.pos.is_empty() {
            continue;
        }
        // hidden, it waits for the next frame it's on: each `get_mut` re-uploads the mesh (B357)
        if !on {
            continue;
        }
        if let Some(mut m) = meshes.get_mut(&mesh.0) {
            *m = b.mesh();
        }
    }
}

/// The sprites' and the rays' (with the whiteout) meshes for the camera turned by `rot` (Bevy space) with
/// projection `p`; empty while the sun isn't in the picture.
fn build(f: &hst_sim::rng::Flare, sun: &crate::shadow::Sun, p: &PerspectiveProjection, rot: Quat) -> (Built, Built) {
    let (mut sprites, mut rays) = (Built::default(), Built::default());
    // the sun's direction (game space, y down) in the camera's view space (Bevy: y up, looking along −z)
    let s = -sun.light.normalize();
    let el = (-s.y).clamp(-1.0, 1.0).asin();
    // `HST_FLARE_SUN=x,y,z`: the flare's sun along another direction (game space; its height stays the court's), for
    // `--shot` checks against the original with its flare's sun poked (research/p3f5_flare_hunt.py)
    let s = std::env::var("HST_FLARE_SUN")
        .ok()
        .map(|e| e.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect::<Vec<_>>())
        .filter(|v| v.len() == 3)
        .map_or(s, |v| Vec3::new(v[0], v[1], v[2]).normalize());
    let sv = rot.inverse() * Vec3::new(s.x, -s.y, -s.z);
    let fr = el / FRAC_PI_2;
    // the sun's colour: from (192, 128, 0) low to (240, 224, 208) overhead, in 256ths of its height
    let k = (el * 162.974_66) as i32;
    let rgb = [(192, 240), (128, 224), (0, 208)].map(|(lo, hi)| (lo + ((k * (hi - lo)) >> 8)) as f32);
    let ray_size = fr * (150.0 - 500.0) + 500.0;
    // ponytail: the boost's scale for the court's flare mode as slot 5's match has it
    let boost_scale = 133.333_33;
    let strength = 0.15 + 0.85 * fr;
    let (sun_p, look_p) = (SUN * sv, Vec3::new(0.0, 0.0, -LOOK));
    let d = sun_p - look_p;
    let c = -d.z / d.length();
    let (tan_v, aspect) = ((p.fov / 2.0).tan(), p.aspect_ratio);
    if c < 0.01 || sun_p.z >= 0.0 {
        return (sprites, rays);
    }
    let ts = sun_p.truncate() / -sun_p.z;
    if ts.x.abs() > tan_v * aspect || ts.y.abs() > tan_v {
        return (sprites, rays);
    }
    let fade = 1.0;
    let b1 = ((c * 192.0) as i32).min(128);
    let b2 = ((c * 192.0 * strength) as i32).min(128);
    // the game's 4:3 picture in tangents: 640 columns, 448 lines
    let tan_h = tan_v / 0.75f32.max(1.0 / aspect);
    let (col, line) = (2.0 * tan_h / 640.0, 1.5 * tan_h / 448.0);

    // the sprites: additive, texel × colour / 128 × alpha / 128
    let a = (b2.min(68) as f32 / 68.0).sqrt() * fade;
    for &(t, cell, size, ergb) in &ELEMENTS[1..] {
        let at = (1.0 - t) * sun_p + t * look_p;
        if at.z >= 0.0 {
            continue;
        }
        let size = size * (1.0 - (1.0 - c) * t);
        let (mid, half) = (at.truncate() / -at.z, Vec2::new(size / 2.0 * col, size / 2.0 * line));
        let (u0, u1) = (cell as f32 * 42.0 / 256.0, (cell as f32 * 42.0 + 42.0) / 256.0);
        let v = [0, 1, 2].map(|i| rgb[i] * ergb[i] / 255.0 / 128.0 * a);
        sprites.quad(mid - half, mid + half, enc(v), [u0, 42.0 / 64.0], [u1, 0.0]);
    }

    // the rays: a fan of kites round the sun, longest toward the middle of the picture the farther the sun is from it
    let p5 = (ray_size * 16.0) as i32;
    let (dx, dy) = (ts.x / col, -ts.y / line / 2.0);
    let r = dx.hypot(dy).min(400.0);
    let mut th = dy.atan2(dx) + PI;
    if th >= 2.0 * PI {
        th -= 2.0 * PI;
    }
    let n = f.bright.len() as i32;
    let i0 = (th * n as f32 / (2.0 * PI)) as i32;
    let centre = rgb.map(|x| x / 255.0 / 255.0 * ((b1 + b2) / 2) as f32 * fade);
    let cap = 16384.0 * 410.29 * tan_h / 320.0;
    let div = 10.0 * (0.2 + 0.8 * fr);
    let dir = |a: f32| Vec2::new(a.cos(), -a.sin()) / RAY_UNIT;
    for (j, (&bright, &angle)) in f.bright.iter().zip(&f.angle).enumerate() {
        let mut diff = j as i32 - i0;
        if diff > n / 2 {
            diff -= n;
        } else if diff < -n / 2 {
            diff += n;
        }
        let near = n / 2 - diff.abs();
        let boost = 1.0 + if near > 0 { near as f32 * r / (boost_scale * (n / 2) as f32) } else { 0.0 };
        let len = (fade * (p5 >> 2) as f32 * bright * boost).min(cap);
        let w = ((p5 >> 3) as f32).min(len / div);
        let (side0, tip, side1) = (ts + w * dir(angle + FRAC_PI_2), ts + len * dir(angle), ts + w * dir(angle - FRAC_PI_2));
        // Gouraud from the centre's colour to black at the other two corners, on the encoded values as the GS shades:
        // in bands (Bevy interpolates the linear values the shader encodes)
        for (a, b) in [(side0, tip), (tip, side1)] {
            for k in 0..BANDS {
                let (l0, l1) = (k as f32 / BANDS as f32, (k + 1) as f32 / BANDS as f32);
                let (c0, c1) = (enc(centre.map(|x| x * (1.0 - l0))), enc(centre.map(|x| x * (1.0 - l1))));
                let at = |q: Vec2, l: f32| ts + (q - ts) * l;
                for (q, col) in [(at(a, l0), c0), (at(a, l1), c1), (at(b, l1), c1), (at(a, l0), c0), (at(b, l1), c1), (at(b, l0), c0)] {
                    rays.v(q, col, [0.0, 0.0]);
                }
            }
        }
    }

    // looking almost straight at the sun: the whole picture washes over with its colour
    if c * 4096.0 > 4096.0 - 24.0 * 16.0 {
        let aw = ((strength * 192.0) as i32).min(255) as f32 * (c * 4096.0 - 3712.0) / 384.0 / 2.0;
        let edge = Vec2::new(tan_v * aspect, tan_v) * 1.01;
        rays.quad(-edge, edge, enc(rgb.map(|x| x / 255.0 * aw / 128.0)), [0.0, 0.0], [0.0, 0.0]);
    }
    (sprites, rays)
}
