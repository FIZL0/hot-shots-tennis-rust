//! Model viewer: `hst <iso> <archive.XB>... [--shot out.png]`
//! Loads every model in the given disc archives and shows them with an orbit camera
//! (drag: orbit, wheel: zoom). `--shot` saves one frame and exits, for unattended checks;
//! `--radius r` orbits the origin at distance r instead of framing everything (skyboxes are huge).

use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use hst_data::{iso::Iso, mdl, mtl, xb::Archive};

#[derive(Resource)]
struct Args {
    iso: String,
    archives: Vec<String>,
    shot: Option<String>,
    radius: Option<f32>,
}

#[derive(Component)]
struct Orbit {
    focus: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
}

fn main() {
    let mut a = std::env::args().skip(1);
    let iso = a.next().expect("usage: hst <iso> <archive.XB>... [--shot out.png]");
    let (mut archives, mut shot, mut radius) = (Vec::new(), None, None);
    while let Some(x) = a.next() {
        match x.as_str() {
            "--shot" => shot = a.next(),
            "--radius" => radius = a.next().and_then(|r| r.parse().ok()),
            _ => archives.push(x),
        }
    }
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Args { iso, archives, shot, radius })
        .insert_resource(ClearColor(Color::srgb(0.25, 0.3, 0.35)))
        .add_systems(Startup, load)
        .add_systems(Update, (orbit, auto_shot))
        .run();
}

fn load(
    mut commands: Commands,
    args: Res<Args>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    // Game space is Y-down; a half-turn about X maps it to Bevy's Y-up without mirroring.
    let root = commands.spawn((Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::PI)), Visibility::default())).id();
    let mut bounds = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for path in &args.archives {
        let data = iso.read(path).expect("archive on disc");
        let arc = Archive::parse(&data).expect("xb archive");
        for e in arc.entries.iter().filter(|e| { let n = e.name.to_ascii_lowercase(); n.ends_with(".mdl") && !n.contains("debug") }) {
            let stem = &e.name[..e.name.len() - 4];
            let sib = |ext: &str| arc.find(&format!("{stem}.{ext}")).and_then(|x| arc.read(x).ok());
            let (Ok(model), Some(mtl_bytes)) = (mdl::parse(&arc.read(e).unwrap()), sib("MTL")) else {
                warn!("skipping {}", e.name);
                continue;
            };
            let mats = match mtl::parse(&mtl_bytes, sib("MTI").as_deref()) {
                Ok(m) => m,
                Err(err) => { warn!("{}: {err}", e.name); continue }
            };
            let tex: Vec<Handle<Image>> = mats.textures.iter().map(|t| images.add(image(t))).collect();
            for (mi, packets) in model.materials.iter().enumerate() {
                let (mut pos, mut nrm, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![], vec![]);
                for pk in packets {
                    let base = pos.len() as u32;
                    for v in &pk.vertices {
                        pos.push(v.pos);
                        nrm.push(v.normal);
                        uv.push(v.uv);
                        col.push(v.color.map(|c| c as f32 / 128.0)); // PS2 modulate: 0x80 = 1.0
                        let p = Vec3::new(v.pos[0], -v.pos[1], -v.pos[2]);
                        bounds = (bounds.0.min(p), bounds.1.max(p));
                    }
                    idx.extend(pk.triangles.iter().flatten().map(|i| i + base));
                }
                if idx.is_empty() { continue }
                let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
                    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
                    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
                    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
                    .with_inserted_indices(Indices::U32(idx));
                let mat = mats.materials.get(mi);
                let texture = mat.and_then(|m| m.texture).map(|t| tex[t].clone());
                let [r, g, b, a] = mat.map_or([1.0; 4], |m| m.color);
                commands.entity(root).with_child((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::linear_rgba(r, g, b, a),
                        base_color_texture: texture,
                        unlit: true,
                        double_sided: true,
                        cull_mode: None,
                        alpha_mode: AlphaMode::Mask(0.5),
                        ..default()
                    })),
                ));
            }
        }
    }
    let (focus, radius) = if bounds.0.x <= bounds.1.x {
        ((bounds.0 + bounds.1) / 2.0, (bounds.1 - bounds.0).length().max(1.0))
    } else {
        (Vec3::ZERO, 10.0)
    };
    info!("bounds {bounds:?}");
    let (focus, radius) = args.radius.map_or((focus, radius), |r| (Vec3::ZERO, r));
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection { far: 20_000.0, ..default() }),
        Transform::default(),
        Orbit { focus, radius, yaw: 0.6, pitch: -0.4 },
    ));
}

fn image(t: &mtl::Texture) -> Image {
    let mut img = Image::new(
        Extent3d { width: t.width.max(1), height: t.height.max(1), depth_or_array_layers: 1 },
        TextureDimension::D2,
        if t.rgba.is_empty() { vec![255; 4 * (t.width.max(1) * t.height.max(1)) as usize] } else { t.rgba.clone() },
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // PS2 UVs tile freely; Bevy's default sampler clamps
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    img
}

fn orbit(
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut q: Query<(&mut Orbit, &mut Transform)>,
) {
    for (mut o, mut t) in &mut q {
        if buttons.pressed(MouseButton::Left) {
            o.yaw -= motion.delta.x * 0.005;
            o.pitch = (o.pitch - motion.delta.y * 0.005).clamp(-1.5, 1.5);
        }
        o.radius *= 1.0 - scroll.delta.y * 0.1;
        let rot = Quat::from_euler(EulerRot::YXZ, o.yaw, o.pitch, 0.0);
        *t = Transform::from_translation(o.focus + rot * Vec3::Z * o.radius).looking_at(o.focus, Vec3::Y);
    }
}

fn auto_shot(mut commands: Commands, args: Res<Args>, mut frame: Local<u32>, mut exit: MessageWriter<AppExit>) {
    let Some(path) = &args.shot else { return };
    *frame += 1;
    if *frame == 30 {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
    }
    if *frame == 45 {
        exit.write(AppExit::Success);
    }
}
