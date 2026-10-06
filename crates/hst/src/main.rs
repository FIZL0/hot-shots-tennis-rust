//! Viewer / sandbox: `hst <iso> <archive.XB>... [--ball] [--court N] [--radius r] [--shot out.png]`
//! Loads every model in the given disc archives and shows them with an orbit camera
//! (drag: orbit, wheel: zoom). `--shot` saves one frame and exits, for unattended checks;
//! `--radius r` orbits the origin at distance r instead of framing everything (skyboxes are huge).
//! `--stage NN` loads disc court NN (01..11) with every prop placed from its layout data.
//! `--ball` adds the game ball driven by the ported physics (Space: new shot); `--court` picks the
//! physics surface table (0..11).

mod sandbox;

use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use hst_data::{iso::Iso, layout, mdl, mtl, xb::Archive};

#[derive(Resource)]
pub struct Args {
    pub iso: String,
    archives: Vec<String>,
    shot: Option<String>,
    radius: Option<f32>,
    ball: bool,
    pub court: usize,
    stage: Option<u32>,
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
    let (mut archives, mut shot, mut radius, mut ball, mut court, mut stage) = (Vec::new(), None, None, false, 0, None);
    while let Some(x) = a.next() {
        match x.as_str() {
            "--shot" => shot = a.next(),
            "--radius" => radius = a.next().and_then(|r| r.parse().ok()),
            "--ball" => ball = true,
            "--court" => court = a.next().and_then(|r| r.parse().ok()).unwrap_or(0),
            "--stage" => stage = a.next().and_then(|r| r.parse().ok()),
            _ => archives.push(x),
        }
    }
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    if ball {
        app.add_plugins(sandbox::plugin);
    }
    app.insert_resource(Args { iso, archives, shot, radius, ball, court, stage })
        .insert_resource(ClearColor(Color::srgb(0.25, 0.3, 0.35)))
        .add_systems(Startup, load)
        .add_systems(Update, (orbit, auto_shot))
        .run();
}

/// Parent for everything in game space: game space is Y-down, a half-turn about X maps it to Bevy's
/// Y-up without mirroring.
#[derive(Component)]
pub struct GameSpace;

fn load(
    mut commands: Commands,
    args: Res<Args>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let root = commands
        .spawn((GameSpace, Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::PI)), Visibility::default()))
        .id();
    let mut bounds = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    // HST_ONLY / HST_SKIP: substring filters on model paths, for bisecting what draws what
    let only = std::env::var("HST_ONLY").ok();
    let not = std::env::var("HST_SKIP").ok();
    let skip = move |n: &str| {
        n.contains("debug")
            || n.contains("holeend")
            || only.as_deref().is_some_and(|o| !n.contains(o))
            || not.as_deref().is_some_and(|o| n.contains(o))
    };
    let spawn = |commands: &mut Commands, parts: &[(Handle<Mesh>, Handle<StandardMaterial>)], t: Transform| {
        let e = commands.spawn((t, Visibility::default())).id();
        for (m, mat) in parts {
            commands.entity(e).with_child((Mesh3d(m.clone()), MeshMaterial3d(mat.clone())));
        }
        commands.entity(root).add_child(e);
    };
    let add = |parts: Vec<(Mesh, StandardMaterial)>, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>| {
        parts.into_iter().map(|(m, mat)| (meshes.add(m), materials.add(mat))).collect::<Vec<_>>()
    };
    if let Some(n) = args.stage {
        let dir = format!("COURT/{n:02}");
        let mut library = std::collections::HashMap::new();
        for name in ["CMN.XB", "GRD01.XB", "HOL01.XB"] {
            let data = iso.read(&format!("{dir}/{name}")).expect("court archive on disc");
            for (stem, parts) in models(&data, |n| !skip(n), &mut images) {
                library.insert(stem, add(parts, &mut meshes, &mut materials));
            }
        }
        let find = |data: &[u8], suffix: &str| {
            let arc = Archive::parse(data).ok()?;
            let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix))?;
            arc.read(e).ok()
        };
        let cmn = iso.read(&format!("{dir}/CMN.XB")).expect("cmn archive");
        let hol = iso.read(&format!("{dir}/HOL01.XB")).expect("hole archive");
        let list = layout::entries(&String::from_utf8_lossy(&find(&cmn, &format!("entry_c{n:02}.txt")).expect("entry list")));
        let plants = layout::plants(&find(&hol, &format!("plant_c{n:02}_h01_0.dat")).expect("plant file")).expect("plant records");
        // ground, skies and clouds stand at the origin; props are placed from the plant records
        // the entry list names every hole variant; this layout is hole 01
        let this_hole = |e: &&layout::Entry| e.dir != "hole" || e.stem.contains("_h01");
        // ponytail: clouds are positioned by special-category records (14?) not yet mapped; left out
        // instead of piling them on the court at the origin
        for e in list.iter().filter(|e| matches!(e.dir.as_str(), "hole" | "bg")).filter(this_hole) {
            if let Some(parts) = library.get(&e.stem) {
                spawn(&mut commands, parts, Transform::default());
            }
        }
        for p in &plants {
            let Some(e) = layout::resolve(&list, p, 0) else { continue };
            if !matches!(e.dir.as_str(), "tree" | "prop" | "structure" | "billboard") {
                continue; // creatures/gallery are animated NPCs, dmy are markers
            }
            let Some(parts) = library.get(&e.stem) else { continue };
            let s = if p.scale > 0.0 { p.scale } else { 1.0 };
            let t = Transform::from_translation(Vec3::from(p.pos)).with_rotation(Quat::from_rotation_y(p.yaw)).with_scale(Vec3::splat(s));
            spawn(&mut commands, parts, t);
        }
        bounds = (Vec3::splat(-20.0), Vec3::splat(20.0));
    }
    for path in &args.archives {
        let data = iso.read(path).expect("archive on disc");
        for (_, parts) in models(&data, |n| !skip(n), &mut images) {
            for (mesh, _) in &parts {
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    for v in p {
                        let p = Vec3::new(v[0], -v[1], -v[2]);
                        bounds = (bounds.0.min(p), bounds.1.max(p));
                    }
                }
            }
            let parts = add(parts, &mut meshes, &mut materials);
            spawn(&mut commands, &parts, Transform::default());
        }
    }
    if args.ball {
        let data = iso.read("CMN/GAME.XB").expect("ball archive on disc");
        let parts: Vec<_> = models(&data, |n| n.ends_with("ball1.mdl"), &mut images)
            .into_iter()
            .flat_map(|(_, p)| p)
            .map(|(mesh, material)| (Mesh3d(meshes.add(mesh)), MeshMaterial3d(materials.add(material))))
            .collect();
        let ball = commands.spawn((sandbox::BallView, Transform::default(), Visibility::default())).id();
        for part in parts {
            commands.entity(ball).with_child(part);
        }
        commands.entity(root).add_child(ball);
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

/// Every model in an archive whose lower-cased name ends in `.mdl` and passes `keep`, keyed by its
/// lower-case file stem, as one Bevy mesh + material per original material.
fn models(data: &[u8], keep: impl Fn(&str) -> bool, images: &mut Assets<Image>) -> Vec<(String, Vec<(Mesh, StandardMaterial)>)> {
    let mut out = Vec::new();
    let arc = Archive::parse(data).expect("xb archive");
    {
        for e in arc.entries.iter().filter(|e| { let n = e.name.to_ascii_lowercase(); n.ends_with(".mdl") && keep(&n) }) {
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
            let model_stem = stem.rsplit(['\\', '/']).next().unwrap_or(stem).to_ascii_lowercase();
            let mut parts = Vec::new();
            for (mi, packets) in model.materials.iter().enumerate() {
                let (mut pos, mut nrm, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![], vec![]);
                for pk in packets {
                    let base = pos.len() as u32;
                    for v in &pk.vertices {
                        pos.push(v.pos);
                        nrm.push(v.normal);
                        uv.push(v.uv);
                        col.push(v.color.map(|c| c as f32 / 128.0)); // PS2 modulate: 0x80 = 1.0
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
                parts.push((
                    mesh,
                    StandardMaterial {
                        base_color: Color::linear_rgba(r, g, b, a),
                        base_color_texture: texture,
                        unlit: true,
                        double_sided: true,
                        cull_mode: None,
                        alpha_mode: AlphaMode::Mask(0.5),
                        ..default()
                    },
                ));
            }
            out.push((model_stem, parts));
        }
    }
    out
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
