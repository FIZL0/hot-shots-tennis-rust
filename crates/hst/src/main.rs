//! Viewer / sandbox: `hst <iso> <archive.XB>... [--ball] [--court N] [--radius r] [--shot out.png]`
//! Loads every model in the given disc archives and shows them with an orbit camera
//! (drag: orbit, wheel: zoom). `--shot` saves one frame (after `--shot-at` seconds) and exits, for unattended checks;
//! `--radius r` orbits the origin at distance r instead of framing everything (skyboxes are huge).
//! `--stage NN` loads disc court NN (01..11) with every prop placed from its layout data.
//! `--ball` adds the game ball driven by the ported physics (Space: new shot); `--court` picks the
//! physics surface table (0..11). `--play` is a playable match against a simple AI (see play.rs).
//! `--sound <archive> <bank.hd> <program> <key>` plays one sound of a bank (see audio.rs); a BGM archive
//! (`--sound SND/BGM/BGMM_05.XB data/sound/BGM/Menu/bgmm_05.hd`) plays its music. `--music` turns on the court's
//! BGM in a `--play` match (off by default).
//! Textures come from `mods/textures/` over a PCSX2 pack in `replacements/` over the disc (both beside the ISO;
//! see textures.rs); `hst <iso> --dump-textures` writes every disc texture to `mods/textures/` to edit.

mod audio;
mod character;
mod court_anim;
mod effects;
mod gs;
mod hud_gamma;
mod play;
mod sandbox;
mod shadow;
mod textures;

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
    /// Seconds of game time before the shot (`--shot-at`, default 0.5).
    shot_at: f32,
    radius: Option<f32>,
    ball: bool,
    pub court: usize,
    stage: Option<u32>,
    play: bool,
    pub singles: bool,
    /// Characters on court (player order), `--chars 0,2,1,5`.
    pub chars: Vec<usize>,
    /// Their outfits (costumes 0..9), `--outfits 9,9,4,9`: the model, the panel face and the computer players' AI row.
    pub outfits: Vec<usize>,
    /// Character viewer: `--character N [--motion M]`.
    pub viewer: Option<(usize, usize)>,
    /// Sound audition: archive, bank header, program, key.
    pub sound: Option<(String, String, usize, usize)>,
    /// `--music`: the court's BGM in a match (off by default).
    pub music: bool,
}

#[derive(Component)]
pub struct Orbit {
    pub focus: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}

fn main() {
    let mut a = std::env::args().skip(1);
    let iso = a.next().expect("usage: hst <iso> <archive.XB>... [--shot out.png]");
    let (mut archives, mut shot, mut radius, mut ball, mut court, mut stage, mut play, mut singles, mut chars) = (Vec::new(), None, None, false, 0, None, false, false, Vec::new());
    let (mut viewer_char, mut viewer_motion) = (None, 0);
    // drawing runs uncapped by default; the simulation stays a fixed 60 Hz tick either way
    let mut vsync = false;
    let mut music = false;
    let mut shot_at = 0.5;
    let mut sound = None;
    let mut outfits = Vec::new();
    while let Some(x) = a.next() {
        match x.as_str() {
            "--shot" => shot = a.next(),
            "--shot-at" => shot_at = a.next().and_then(|r| r.parse().ok()).unwrap_or(shot_at),
            "--radius" => radius = a.next().and_then(|r| r.parse().ok()),
            "--ball" => ball = true,
            "--court" => court = a.next().and_then(|r| r.parse().ok()).unwrap_or(0),
            "--stage" => stage = a.next().and_then(|r| r.parse().ok()),
            "--play" => play = true,
            "--singles" => singles = true,
            "--character" => viewer_char = a.next().and_then(|r| r.parse().ok()),
            "--motion" => viewer_motion = a.next().and_then(|r| r.parse().ok()).unwrap_or(0),
            "--vsync" => vsync = true,
            "--music" => music = true,
            "--sound" => {
                let mut n = || a.next().unwrap_or_default();
                let (xb, hd, program, key) = (n(), n(), n(), n());
                sound = Some((xb, hd, program.parse().unwrap_or(0), key.parse().unwrap_or(0)));
            }
            "--dump-textures" => {
                let mut iso_ = Iso::open(&iso).expect("open iso");
                let o = hst_data::texhash::Overrides::scan(std::path::Path::new(&iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()));
                let (n, packed) = o.dump(&mut iso_);
                return println!("wrote {n} textures ({packed} from the PCSX2 pack) to {}", o.root.join("mods/textures").display());
            }
            "--chars" => chars = a.next().map(|s| s.split(',').filter_map(|c| c.trim().parse().ok()).collect()).unwrap_or_default(),
            "--outfits" => outfits = a.next().map(|s| s.split(',').filter_map(|c| c.trim().parse().ok()).collect()).unwrap_or_default(),
            _ => archives.push(x),
        }
    }
    textures::init(&iso);
    let mut app = App::new();
    let present_mode = if vsync { bevy::window::PresentMode::AutoVsync } else { bevy::window::PresentMode::AutoNoVsync };
    app.add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { present_mode, ..default() }), ..default() }));
    app.add_plugins((audio::plugin, gs::plugin, shadow::plugin, court_anim::plugin, textures::plugin, hud_gamma::plugin));
    if play {
        app.add_plugins(play::plugin);
    } else if viewer_char.is_some() {
        app.add_plugins(character::viewer);
    } else if ball {
        app.add_plugins(sandbox::plugin);
    }
    app.insert_resource(Args { iso, archives, shot, shot_at, radius, ball, court, stage, play, singles, chars, outfits, viewer: viewer_char.map(|c| (c, viewer_motion)), sound, music })
        .insert_resource(ClearColor(Color::srgb(0.25, 0.3, 0.35)))
        .add_systems(Startup, load)
        .add_systems(Update, (orbit, auto_shot, clouds))
        .run();
}

/// Court `n`'s entry list and hole-01 placement records.
fn court_layout(iso: &mut Iso, n: usize) -> Option<(Vec<layout::Entry>, Vec<layout::Placement>)> {
    let find = |data: &[u8], suffix: &str| {
        let arc = Archive::parse(data).ok()?;
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix))?;
        arc.read(e).ok()
    };
    let (cmn, hol) = (iso.read(&format!("COURT/{n:02}/CMN.XB")).ok()?, iso.read(&format!("COURT/{n:02}/HOL01.XB")).ok()?);
    let list = layout::entries(&String::from_utf8_lossy(&find(&cmn, &format!("entry_c{n:02}.txt"))?));
    Some((list, layout::plants(&find(&hol, &format!("plant_c{n:02}_h01_0.dat"))?).ok()?))
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
    mut gs_materials: ResMut<Assets<gs::GsMaterial>>,
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
    let spawn = |commands: &mut Commands, parts: &[(Handle<Mesh>, Handle<gs::GsMaterial>)], t: Transform| {
        let e = commands.spawn((t, Visibility::default())).id();
        for (m, mat) in parts {
            commands.entity(e).with_child((Mesh3d(m.clone()), MeshMaterial3d(mat.clone())));
        }
        commands.entity(root).add_child(e);
        e
    };
    let add = |parts: Vec<(Mesh, Vec<gs::GsMaterial>)>, meshes: &mut Assets<Mesh>, materials: &mut Assets<gs::GsMaterial>| {
        let mut out = Vec::new();
        for (m, mats) in parts {
            let m = meshes.add(m);
            out.extend(mats.into_iter().map(|mat| (m.clone(), materials.add(mat))));
        }
        out
    };
    if let Some(n) = args.stage {
        let dir = format!("COURT/{n:02}");
        let mut library = std::collections::HashMap::new();
        let mut anims = std::collections::HashMap::new();
        // the game's season is 1 in singles (hole archive SSN1/HOL01.XB, which differs by the net), 0 in doubles
        let season = args.singles as usize;
        let mut sky_colour = std::collections::HashMap::new();
        for name in ["CMN.XB", "GRD01.XB", if args.singles { "SSN1/HOL01.XB" } else { "HOL01.XB" }] {
            let data = iso.read(&format!("{dir}/{name}")).expect("court archive on disc");
            for_models(&data, |n| n.contains("_sky"), |stem, model, mats| {
                sky_colour.insert(stem, sky_top(&model, &mats));
            });
            for (stem, parts, anim) in gs_models_anim(&data, |n| !skip(n), &mut images) {
                let tags: Vec<_> = parts.iter().flat_map(|(_, g, t)| std::iter::repeat_n(*t, g.len())).collect();
                let parts = add(parts.into_iter().map(|(m, g, _)| (m, g)).collect(), &mut meshes, &mut gs_materials);
                anims.insert(stem.clone(), (anim, tags));
                library.insert(stem, parts);
            }
        }
        // the season's fog on every court material (FGE batches use it)
        let envir = iso.read(&format!("{dir}/CMN.XB")).ok().and_then(|d| {
            let arc = Archive::parse(&d).ok()?;
            arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("envir_c{n:02}.dat")))?).ok()
        });
        if let Some((fog, colour)) = envir.as_ref().and_then(|e| gs::court_fog(e, season)) {
            for (_, m) in gs_materials.iter_mut() {
                (m.uniform.fog, m.uniform.fog_color) = (fog, colour);
            }
        }
        let (list, plants) = court_layout(&mut iso, n as usize).expect("court layout");
        // ground, skies and clouds stand at the origin; props are placed from the plant records
        // the entry list names every hole variant; this layout is hole 01
        let this_hole = |e: &&layout::Entry| e.dir != "hole" || e.stem.contains("_h01");
        // the hole's ground model is the one shadow receiver
        let sun = shadow::Sun::read(&mut iso, n as usize);
        // the game loads every in-season sky but draws only the first; its colour also clears the screen
        let mut sky = None;
        for e in list.iter().filter(|e| matches!(e.dir.as_str(), "hole" | "bg")).filter(this_hole).filter(|e| layout::in_season(&e.stem, season)) {
            if e.stem.contains("_sky") {
                if sky.is_some() {
                    continue;
                }
                sky = sky_colour.get(&e.stem).copied();
            }
            if let Some(parts) = library.get(&e.stem) {
                spawn(&mut commands, parts, Transform::default());
                for (_, m) in parts.iter().filter(|_| e.dir == "hole") {
                    gs_materials.get_mut(m).unwrap().uniform.shadow = sun.map_or(0.0, |s| s.darken);
                }
                // the hole model's own .UVA/.MTA play from the start (water, waterfalls)
                if let Some((anim, tags)) = anims.remove(&e.stem).filter(|(a, _)| e.dir == "hole" && !a.is_empty()) {
                    let parts = parts.iter().zip(tags).map(|((_, m), t)| (m.clone(), t)).collect();
                    commands.spawn(court_anim::Playing { anim, parts });
                }
            }
        }
        if let Some([r, g, b]) = envir.as_ref().zip(sky).and_then(|(e, c)| gs::court_clear(e, season, c)) {
            commands.insert_resource(ClearColor(Color::srgb_u8(r, g, b)));
        }
        if let Some(sun) = sun {
            commands.insert_resource(sun);
        }
        for p in &plants {
            let Some(e) = layout::resolve(&list, p, season) else { continue };
            if !matches!(e.dir.as_str(), "tree" | "prop" | "structure" | "billboard") {
                continue; // creatures/gallery are animated NPCs, dmy are markers
            }
            let Some(parts) = library.get(&e.stem) else { continue };
            let s = if p.scale > 0.0 { p.scale } else { 1.0 };
            let t = Transform::from_translation(Vec3::from(p.pos)).with_rotation(Quat::from_rotation_y(p.yaw)).with_scale(Vec3::splat(s));
            let e = spawn(&mut commands, parts, t);
            if p.code[3] != b'0' && (17..=19).contains(&p.category) {
                commands.entity(e).insert(shadow::Caster);
            }
        }
        // clouds: singles only (the game makes them in doubles too but neither moves nor draws them)
        if args.singles {
            let read = |iso: &mut Iso, xb: &str, suffix: &str| {
                let data = iso.read(&format!("{dir}/{xb}")).ok()?;
                let arc = Archive::parse(&data).ok()?;
                arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix))?).ok()
            };
            let envir = read(&mut iso, "CMN.XB", &format!("envir_c{n:02}.dat"));
            let hole = read(&mut iso, "GRD01.XB", &format!("envir_c{n:02}_h01.dat"));
            let count = envir.zip(hole).and_then(|(e, h)| layout::cloud_count(&e, &h, 1)).unwrap_or(20);
            let models: Vec<_> = list.iter().filter(|e| e.dir == "cloud" && layout::in_season(&e.stem, season)).filter_map(|e| library.get(&e.stem)).collect();
            if !models.is_empty() {
                let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
                let (directions, speed) = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc").wind(n);
                let mut rng = 0x2545_f491u32 ^ n;
                let mut r = || {
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    (rng >> 8) as f32 / (1 << 24) as f32
                };
                let degrees = directions[((r() * directions.len() as f32) as usize).min(directions.len() - 1)];
                let list = hst_sim::clouds::spawn(count, models.len(), &mut r);
                for (i, k) in list.iter().enumerate() {
                    let rot = Quat::from_rotation_x(std::f32::consts::PI) * Quat::from_rotation_y(k.yaw);
                    let bases = models[k.model].iter().map(|(_, m)| gs_materials.get(m).map_or(1.0, |m| m.uniform.color.w)).collect();
                    let e = commands.spawn((CloudView(i, bases), Transform::from_rotation(rot).with_scale(Vec3::splat(40.0)), Visibility::default())).id();
                    for (m, mat) in models[k.model] {
                        // its own material: the fade scales its alpha
                        let mut mat = gs_materials.get(mat).expect("cloud material").clone();
                        mat.key.modulate = true;
                        commands.entity(e).with_child((Mesh3d(m.clone()), MeshMaterial3d(gs_materials.add(mat))));
                    }
                    commands.entity(root).add_child(e);
                }
                commands.insert_resource(Clouds { list, degrees, speed, ticks: 0.0 });
            }
        }
        // background figures where the game makes them; stand-in capsules (walkers grey, trigger creatures
        // green, court 5's own blue) until their models and motion are ported. The umpire is P13's.
        let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
        let game = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
        let figure = meshes.add(Capsule3d::new(0.25, 1.1));
        let players = if args.singles { 2 } else { 4 };
        for npc in hst_sim::npc::spawn(&list, &plants, &game.npc_roster(n), &game.walkers(n), players) {
            let colour = match npc.kind {
                hst_sim::npc::Kind::Walker(_) => Color::srgb(0.6, 0.6, 0.6),
                hst_sim::npc::Kind::Trigger(_) => Color::srgb(0.3, 0.7, 0.3),
                hst_sim::npc::Kind::Court5(_) => Color::srgb(0.3, 0.4, 0.8),
                hst_sim::npc::Kind::Umpire => continue,
            };
            let [x, y, z, _] = npc.world[3];
            // game space is Y-down: the capsule's centre sits 0.8 m above the feet
            let t = Transform::from_matrix(Mat4::from_cols_array_2d(&npc.world)).with_translation(Vec3::new(x, y - 0.8, z));
            let e = commands.spawn((t, Visibility::default(), Mesh3d(figure.clone()), MeshMaterial3d(materials.add(colour)))).id();
            commands.entity(root).add_child(e);
        }
        bounds = (Vec3::splat(-20.0), Vec3::splat(20.0));
    }
    for path in &args.archives {
        let data = iso.read(path).expect("archive on disc");
        for (_, parts) in gs_models(&data, |n| !skip(n), &mut images) {
            for (mesh, _) in &parts {
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    for v in p {
                        let p = Vec3::new(v[0], -v[1], -v[2]);
                        bounds = (bounds.0.min(p), bounds.1.max(p));
                    }
                }
            }
            let parts = add(parts, &mut meshes, &mut gs_materials);
            spawn(&mut commands, &parts, Transform::default());
        }
    }
    if args.ball && !args.play {
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
        // the PS2 writes its colours as they are
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Projection::Perspective(PerspectiveProjection { far: 20_000.0, ..default() }),
        Transform::default(),
        // play mode: broadcast view from behind the near (your) baseline
        if args.play { Orbit { focus: Vec3::new(0.0, 0.0, 2.0), radius: 24.0, yaw: std::f32::consts::PI, pitch: -0.38 } } else { Orbit { focus, radius, yaw: 0.6, pitch: -0.4 } },
    ));
}

/// Every model in an archive whose lower-cased name ends in `.mdl` and passes `keep`, keyed by its
/// lower-case file stem, as one Bevy mesh + material per original material.
fn models(data: &[u8], keep: impl Fn(&str) -> bool, images: &mut Assets<Image>) -> Vec<(String, Vec<(Mesh, StandardMaterial)>)> {
    let mut out = Vec::new();
    for_models(data, keep, |model_stem, model, mats| {
        let tex: Vec<Handle<Image>> = mats.textures.iter().map(|t| textures::add_mtl(images, image(t), t)).collect();
        let mut parts = Vec::new();
        {
            for (mi, packets) in model.materials.iter().enumerate() {
                let Some(mesh) = mesh(packets.iter()) else { continue };
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
        }
        out.push((model_stem, parts));
    });
    out
}

/// Every model in an archive whose lower-cased name ends in `.mdl` and passes `keep`, with its lower-case file
/// stem and parsed MTL.
fn for_models(data: &[u8], keep: impl Fn(&str) -> bool, mut f: impl FnMut(String, mdl::Model, mtl::Mtl)) {
    let arc = Archive::parse(data).expect("xb archive");
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
        f(stem.rsplit(['\\', '/']).next().unwrap_or(stem).to_ascii_lowercase(), model, mats);
    }
}

/// A sky's colour where the game reads it: the top-most vertex (smallest game y, the last of equals)
/// colour × its material colour / 128, in 0..255.
fn sky_top(model: &mdl::Model, mats: &mtl::Mtl) -> [f32; 3] {
    let mut top = (f32::MAX, [0.0; 3]);
    for (mi, pk) in model.materials.iter().enumerate() {
        let m = mats.materials.get(mi).map_or([1.0; 4], |m| m.color);
        for v in pk.iter().flat_map(|p| &p.vertices) {
            if v.pos[1] <= top.0 {
                top = (v.pos[1], std::array::from_fn(|i| v.color[i] as f32 * m[i]));
            }
        }
    }
    top.1
}

/// One triangle-list mesh of these packets; vertex colours stay in PS2 units (0x80 = 1.0).
fn mesh<'a>(packets: impl Iterator<Item = &'a mdl::Packet>) -> Option<Mesh> {
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
    if idx.is_empty() {
        return None;
    }
    Some(
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
            .with_inserted_indices(Indices::U32(idx)),
    )
}

/// Like [`models`], drawn as the GS draws them ([`gs`]): one mesh per material and batch PRIM, its texture raw and
/// wrapped per the MDL's material header.
fn gs_models(data: &[u8], keep: impl Fn(&str) -> bool, images: &mut Assets<Image>) -> Vec<(String, Vec<(Mesh, Vec<gs::GsMaterial>)>)> {
    gs_models_anim(data, keep, images).into_iter().map(|(stem, parts, _)| (stem, parts.into_iter().map(|(m, g, _)| (m, g)).collect())).collect()
}

/// [`gs_models`] with each model's `.UVA`/`.MTA` ([`court_anim`]): meshes are further split per UV track, and each
/// part carries the (material, packet) its animation is read from.
fn gs_models_anim(
    data: &[u8],
    keep: impl Fn(&str) -> bool,
    images: &mut Assets<Image>,
) -> Vec<(String, Vec<(Mesh, Vec<gs::GsMaterial>, (usize, usize))>, hst_sim::court_anim::CourtAnim)> {
    let arc = Archive::parse(data).expect("xb archive");
    let tracks = |stem: &str, ext: &str, width| {
        let suffix = format!("{stem}.{ext}");
        let e = arc.entries.iter().find(|e| e.name.rsplit(['\\', '/']).next().is_some_and(|n| n.eq_ignore_ascii_case(&suffix)))?;
        hst_data::mor::parse(&arc.read(e).ok()?, width).ok()
    };
    let mut out = Vec::new();
    for_models(data, keep, |stem, model, mats| {
        let anim = hst_sim::court_anim::CourtAnim::new(&model, tracks(&stem, "uva", 4).as_ref(), tracks(&stem, "mta", 1).as_ref(), &mats.materials);
        let mut parts = Vec::new();
        for (mi, packets) in model.materials.iter().enumerate() {
            let Some(mat) = mats.materials.get(mi) else { continue };
            let wrap = model.wrap.get(mi).copied().unwrap_or([0, 0]);
            let texture = mat.texture.map(|t| {
                let mut img = image(&mats.textures[t]);
                img.texture_descriptor.format = TextureFormat::Rgba8Unorm;
                // the mip levels TEX1 MXL lets the GS use; gs.wgsl picks one from the view depth
                let t = &mats.textures[t];
                if !t.mips.is_empty() && !t.rgba.is_empty() {
                    img.texture_descriptor.mip_level_count = 1 + t.mips.len() as u32;
                    img.data.as_mut().unwrap().extend(t.mips.iter().flatten());
                }
                // GS CLAMP: 0 repeat; 1 clamp; 2 region clamp over the whole texture (as the game sets it) = clamp
                let mode = |w: u8| if w == 0 { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
                img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: mode(wrap[0]),
                    address_mode_v: mode(wrap[1]),
                    ..ImageSamplerDescriptor::linear()
                });
                textures::add_mtl(images, img, t)
            });
            let key = |pi: usize| (packets[pi].prim & 0x70, anim.uv_track(mi, pi), packets[pi].uv_swap);
            let mut keys: Vec<_> = (0..packets.len()).map(|pi| (key(pi), pi)).collect();
            keys.sort();
            keys.dedup_by_key(|k| k.0);
            for (k, first) in keys {
                let Some(mesh) = mesh((0..packets.len()).filter(|&pi| key(pi) == k).map(|pi| &packets[pi])) else { continue };
                let mut draws = gs::GsMaterial::for_batch(mat, k.0, texture.clone());
                draws.iter_mut().for_each(|g| g.uniform.lod_k = model.lod_k.get(mi).copied().unwrap_or(0.0));
                parts.push((mesh, draws, (mi, first)));
            }
        }
        out.push((stem, parts, anim));
    });
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

/// The sky's clouds and the wind they drift on (`--stage` in singles).
#[derive(Resource)]
struct Clouds {
    list: Vec<hst_sim::clouds::Cloud>,
    degrees: f32,
    speed: f32,
    /// 60 Hz ticks owed.
    ticks: f32,
}

#[derive(Component)]
/// Index into `Clouds::list` and each part's own material alpha.
struct CloudView(usize, Vec<f32>);

/// Drifts the clouds at 60 Hz; each stands at its place relative to the camera and fades out near the edge.
fn clouds(
    time: Res<Time>,
    mut sky: Option<ResMut<Clouds>>,
    camera: Query<&GlobalTransform, With<Orbit>>,
    mut views: Query<(&CloudView, &mut Transform, &Children)>,
    parts: Query<&MeshMaterial3d<gs::GsMaterial>>,
    mut materials: ResMut<Assets<gs::GsMaterial>>,
) {
    let (Some(sky), Ok(camera)) = (sky.as_mut(), camera.single()) else { return };
    sky.ticks += time.delta_secs() * 60.0;
    while sky.ticks >= 1.0 {
        sky.ticks -= 1.0;
        let (d, s) = (sky.degrees, sky.speed);
        hst_sim::clouds::tick(&mut sky.list, d, s);
    }
    // game space is Bevy's turned a half-turn about X
    let eye = camera.translation() * Vec3::new(1.0, -1.0, -1.0);
    for (v, mut t, children) in &mut views {
        let k = &sky.list[v.0];
        t.translation = Vec3::from(k.pos) + eye;
        let fade = hst_sim::clouds::fade(k);
        for (c, base) in children.iter().zip(&v.1) {
            // ponytail: the fade scales the material alpha; the game writes it to the model (+0x5c) whose exact use
            // in the draw is unconfirmed
            if let Some(mut m) = parts.get(c).ok().and_then(|m| materials.get_mut(&m.0)) {
                m.uniform.color.w = base * fade;
            }
        }
    }
}

fn auto_shot(mut commands: Commands, args: Res<Args>, time: Res<Time>, mut taken: Local<bool>, mut exit: MessageWriter<AppExit>) {
    let Some(path) = &args.shot else { return };
    if !*taken && time.elapsed_secs() >= args.shot_at {
        *taken = true;
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
    }
    // the save is asynchronous: wait for the file
    if *taken && std::path::Path::new(path).exists() {
        exit.write(AppExit::Success);
    }
}
