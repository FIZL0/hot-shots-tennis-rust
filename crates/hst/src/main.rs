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
//! Textures come from `mods/texture-replacements/` (key-named files over a PCSX2 pack's) over the disc (beside the
//! ISO; see textures.rs); `hst <iso> --dump-textures` writes every disc texture to `mods/textures-src/` to edit or
//! upscale into `mods/texture-replacements/`.

mod audio;
mod character;
mod court_anim;
mod effects;
mod gs;
mod mods;
mod hud_gamma;
mod noise;
mod play;
mod sandbox;
mod shade;
mod shadow;
mod textures;
mod weather;

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
    let mut viewer_mod = None;
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
            "--mod" => (viewer_mod, viewer_char) = (a.next(), viewer_char.or(Some(0))),
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
                let n = o.dump(&mut iso_);
                return println!("wrote {n} disc textures to {} (upscale them into mods/texture-replacements)", o.root.join("mods/textures-src").display());
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
    app.add_plugins((audio::plugin, gs::plugin, shadow::plugin, court_anim::plugin, textures::plugin, hud_gamma::plugin, weather::plugin));
    app.add_plugins(shade::plugin);
    app.add_plugins(noise::plugin);
    if play {
        app.add_plugins(play::plugin);
    } else if viewer_char.is_some() {
        app.add_plugins(character::viewer).insert_resource(character::ViewerMod(viewer_mod));
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

/// How many clouds court `n`'s setup places (`layout::cloud_count`; singles' environment row 1, doubles' 0).
// ponytail: the rain weathers' count (an exe constant) isn't read: the clear and cloudy rule only
fn cloud_count(iso: &mut Iso, n: u32, envir: Option<&[u8]>, singles: bool) -> usize {
    let hole = iso.read(&format!("COURT/{n:02}/GRD01.XB")).ok().and_then(|d| {
        let arc = Archive::parse(&d).ok()?;
        arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("envir_c{n:02}_h01.dat")))?).ok()
    });
    envir.zip(hole).and_then(|(e, h)| layout::cloud_count(e, &h, singles as usize)).unwrap_or(20)
}

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
        let (mut hole_box, mut model_tris) = (std::collections::HashMap::new(), std::collections::HashMap::new());
        let mut model_shapes = std::collections::HashMap::new();
        for name in ["CMN.XB", "GRD01.XB", if args.singles { "SSN1/HOL01.XB" } else { "HOL01.XB" }] {
            let data = iso.read(&format!("{dir}/{name}")).expect("court archive on disc");
            for_models(&data, |n| n.contains("_sky"), |stem, model, mats| {
                sky_colour.insert(stem, sky_top(&model, &mats));
            });
            // the sun-shade map: the hole model's box frames it, the casters' triangles shade it
            for_models(&data, |n| !skip(n), |stem, model, mats| {
                hole_box.insert(stem.clone(), shade::packet_box(&model));
                model_shapes.insert(stem.clone(), hst_sim::shade::Shape::new(&model, &mats));
                let tris: Vec<[[f32; 3]; 3]> = model.materials.iter().flatten().flat_map(|pk| pk.triangles.iter().map(|t| t.map(|i| pk.vertices[i as usize].pos))).collect();
                model_tris.insert(stem, tris);
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
        // the fogs, light, clear colour and ground shadow follow the weather (`weather::apply`)
        let sun = shadow::Sun::read(&mut iso, n as usize);
        let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
        let exe = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
        let today = {
            let o = exe.weather_odds(n);
            let odds = hst_sim::weather::Odds { cloudy: o[0], cloudy_len: [o[1], o[2]], rain: o[3], rain_len: [o[4], o[5]], no_border: o[7] != 0, heavy: o[8] != 0 };
            let ((directions, speed), (chance, kind)) = (exe.wind(n), exe.gusts(n));
            let wind = hst_sim::weather::Wind { chance, kind, directions, speed };
            // a match: 4 games, 1 set (`play` keeps `Weather::game` on the games played)
            let players = if !args.play { 1 } else if args.singles { 2 } else { 4 };
            // the game seeds its MT19937 with a `rand()` output when it sets the match up; `HST_WEATHER_SEED` gives it
            // directly (slot 5's was 0x28c7c4a1)
            // the menus' `rand()` calls before the match are the character select's flame (`rng::MenuFlame`), drawing
            // every frame it's up; the app has no such menu yet, so a clock-picked count stands in for them
            let mut r = hst_sim::weather::Rand::default();
            let seed = std::env::var("HST_WEATHER_SEED").ok().and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok()).unwrap_or_else(|| {
                let skip = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_micros() % 65536);
                (0..skip).for_each(|_| _ = r.next());
                r.next()
            });
            let mut mt = hst_sim::weather::Mt::new(seed);
            let schedule = hst_sim::weather::schedule(&odds, &wind, 4, 1, players, || mt.next());
            // the match goes on drawing from the same generators (with a given seed, `rand()` is taken as at boot)
            let mut rngs = hst_sim::rng::Rngs::new(r, mt);
            // the setup's own `rand()` calls: the lens flare, the sound manager, the clouds and the effects seed
            let effects = rngs.setup_rand(cloud_count(&mut iso, n, envir.as_deref(), args.singles));
            play::set_effects_seed(effects);
            commands.insert_resource(play::MatchRng(rngs));
            let fixed = std::env::var("HST_WEATHER").ok().and_then(|w| w.parse().ok());
            let w = weather::Weather { schedule, game: 0, fixed };
            let today = w.today();
            commands.insert_resource(w);
            today
        };
        let mut look = weather::CourtLook {
            envir: envir.clone().unwrap_or_default(),
            season,
            sky: None,
            looks: exe.weather_looks(),
            players: exe.player_light(n),
            bg: Default::default(),
            skies: Default::default(),
            holes: Default::default(),
            acc: Vec::new(),
            clo: Vec::new(),
            last: None,
        };
        let (list, plants) = court_layout(&mut iso, n as usize).expect("court layout");
        // the court load shifts the hole model's vertex colours in HSL by the court's and the hole's envir
        let hole_hsl = envir.as_deref().zip(iso.read(&format!("{dir}/GRD01.XB")).ok().and_then(|d| {
            let arc = Archive::parse(&d).ok()?;
            arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("envir_c{n:02}_h01.dat")))?).ok()
        })).and_then(|(c, h)| gs::hole_hsl(c, &h));
        // ground, skies and clouds stand at the origin; props are placed from the plant records
        // the entry list names every hole variant; this layout is hole 01
        let this_hole = |e: &&layout::Entry| e.dir != "hole" || e.stem.contains("_h01");
        // the hole's ground model is the one shadow receiver
        // the game loads every in-season sky but draws only the first; its colour also clears the screen
        let mut sky = None;
        let (mut shade_frame, mut shade_casters) = (None, Vec::new());
        for e in list.iter().filter(|e| matches!(e.dir.as_str(), "hole" | "bg")).filter(this_hole).filter(|e| layout::in_season(&e.stem, season)) {
            if e.stem.contains("_sky") {
                if sky.is_some() {
                    continue;
                }
                sky = sky_colour.get(&e.stem).copied();
            }
            // the sun-shade map frames the hole model (court 7 has none)
            if let Some(&(lo, hi)) = hole_box.get(&e.stem).filter(|_| e.dir == "hole" && n != 7) {
                shade_frame = Some((hst_sim::shade::Frame::new(lo, hi, 1.0, [0.0; 3]), e.stem.clone()));
            }
            if let Some(parts) = library.get(&e.stem) {
                if let Some(hsl) = hole_hsl.filter(|_| e.dir == "hole") {
                    shift_colours(parts, &mut meshes, hsl);
                }
                let id = spawn(&mut commands, parts, Transform::default());
                let set = if e.dir == "hole" { &mut look.holes } else if e.stem.contains("_sky") { &mut look.skies } else { &mut look.bg };
                set.extend(parts.iter().map(|(_, m)| m.id()));
                if e.stem.contains("_acc") {
                    look.acc.push(id);
                } else if e.stem.contains("_clo") {
                    look.clo.push(id);
                }
                // the hole model's own .UVA/.MTA play from the start (water, waterfalls)
                if let Some((anim, tags)) = anims.remove(&e.stem).filter(|(a, _)| e.dir == "hole" && !a.is_empty()) {
                    let parts = parts.iter().zip(tags).map(|((_, m), t)| (m.clone(), t)).collect();
                    commands.spawn(court_anim::Playing { anim, parts });
                }
            }
        }
        look.sky = sky;
        if envir.is_some() {
            commands.insert_resource(look);
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
            let shape = model_shapes.get(&e.stem).cloned().unwrap_or_default();
            let e = spawn(&mut commands, parts, t);
            if p.code[3] != b'0' && (17..=19).contains(&p.category) {
                commands.entity(e).insert(shadow::Caster);
                let axes = [Vec3::X, Vec3::Y, Vec3::Z].map(|a| (t.rotation * a * t.scale).to_array());
                shade_casters.push(hst_sim::shade::Caster { axes, pos: p.pos, tris: shape.tris, cut: shape.cut });
            }
        }
        if let (Some((frame, hole)), Some(sun)) = (shade_frame, sun) {
            commands.insert_resource(shade::build(frame, sun.dir, &shade_casters, &model_tris.remove(&hole).unwrap_or_default(), hst_sim::court::world(&mut iso, n)));
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
                let models = models.into_iter().cloned().collect();
                // the layout is made when the weather settles (`clouds`)
                commands.insert_resource(Clouds { list: Vec::new(), degrees: today.degrees, speed: today.speed, ticks: 0.0, count, models, root, layout: None, rng: 0x2545_f491u32 ^ n });
            }
        }
        // the background figures are `play::npcs`'s
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
                if let (Some(m), Some(t), Some(h)) = (mat, mat.and_then(|m| m.texture), &texture) {
                    let mut raw = image(&mats.textures[t]);
                    raw.texture_descriptor.format = TextureFormat::Rgba8Unorm;
                    let mut draws = gs::GsMaterial::for_batch(m, packets.first().map_or(0x10, |p| p.prim), Some(textures::add_mtl(images, raw, &mats.textures[t])));
                    draws.iter_mut().for_each(|g| g.uniform.lod_k = model.lod_k.get(mi).copied().unwrap_or(0.0));
                    shade::BALL_GS.lock().unwrap().insert(h.id(), draws);
                }
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
            let key = |pi: usize| (packets[pi].prim & 0x70, anim.uv_track(mi, pi), packets[pi].uv_swap, packets[pi].flags & 2);
            let mut keys: Vec<_> = (0..packets.len()).map(|pi| (key(pi), pi)).collect();
            keys.sort();
            keys.dedup_by_key(|k| k.0);
            for (k, first) in keys {
                let Some(mesh) = mesh((0..packets.len()).filter(|&pi| key(pi) == k).map(|pi| &packets[pi])) else { continue };
                let mut draws = gs::GsMaterial::for_batch(mat, k.0, texture.clone());
                draws.iter_mut().for_each(|g| (g.uniform.lod_k, g.uniform.unlit) = (model.lod_k.get(mi).copied().unwrap_or(0.0), (k.3 != 0) as u8 as f32));
                parts.push((mesh, draws, (mi, first)));
            }
        }
        out.push((stem, parts, anim));
    });
    out
}

/// [`gs::hsl_shift`] on these parts' vertex colours (PS2 units, 0x80 = 1.0), each mesh once.
fn shift_colours(parts: &[(Handle<Mesh>, Handle<gs::GsMaterial>)], meshes: &mut Assets<Mesh>, hsl: [i32; 3]) {
    let mut done = std::collections::HashSet::new();
    for (m, _) in parts.iter().filter(|(m, _)| done.insert(m.id())) {
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(col)) = meshes.get_mut(m).and_then(|m| m.into_inner().attribute_mut(Mesh::ATTRIBUTE_COLOR)) else { continue };
        for c in col {
            let rgb = gs::hsl_shift(std::array::from_fn(|i| (c[i] * 128.0) as u8), hsl);
            c[..3].copy_from_slice(&rgb.map(|x| x as f32 / 128.0));
        }
    }
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
    /// The court's cloud count, its cloud models and where they hang.
    count: usize,
    models: Vec<Vec<(Handle<Mesh>, Handle<gs::GsMaterial>)>>,
    root: Entity,
    /// The layout drawn (`hst_sim::weather::cloud_layout`), and the layout's random source.
    layout: Option<(f32, f32, f32, Option<usize>)>,
    rng: u32,
}

#[derive(Component)]
/// Index into `Clouds::list` and each part's own material colour.
struct CloudView(usize, Vec<Vec4>);

/// Drifts the clouds at 60 Hz on the game's wind; each stands at its place relative to the camera, fades out near
/// the edge and darkens in rain. Heavy rain remakes them low and many.
fn clouds(
    mut commands: Commands,
    time: Res<Time>,
    mut sky: Option<ResMut<Clouds>>,
    weather: Option<Res<weather::Weather>>,
    camera: Query<&GlobalTransform, With<Orbit>>,
    mut views: Query<(Entity, &CloudView, &mut Transform, &Children)>,
    parts: Query<&MeshMaterial3d<gs::GsMaterial>>,
    mut materials: ResMut<Assets<gs::GsMaterial>>,
) {
    let (Some(sky), Ok(camera)) = (sky.as_mut(), camera.single()) else { return };
    let today = weather.map(|w| w.today()).unwrap_or_default();
    (sky.degrees, sky.speed) = (today.degrees, today.speed);
    let layout = hst_sim::weather::cloud_layout(today.weather);
    if sky.layout != Some(layout) {
        sky.layout = Some(layout);
        for (e, ..) in &views {
            commands.entity(e).despawn();
        }
        let (_, y0, y1, count) = layout;
        let mut rng = sky.rng;
        let mut r = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            (rng >> 8) as f32 / (1 << 24) as f32
        };
        sky.list = hst_sim::clouds::spawn(count.unwrap_or(sky.count), sky.models.len(), [y0 * 40.0, y1 * 40.0], &mut r);
        sky.rng = rng;
        for (i, k) in sky.list.iter().enumerate() {
            let rot = Quat::from_rotation_x(std::f32::consts::PI) * Quat::from_rotation_y(k.yaw);
            let bases = sky.models[k.model].iter().map(|(_, m)| materials.get(m).map_or(Vec4::ONE, |m| m.uniform.color)).collect();
            let e = commands.spawn((CloudView(i, bases), Transform::from_rotation(rot).with_scale(Vec3::splat(40.0)), Visibility::default())).id();
            for (m, mat) in &sky.models[k.model] {
                // its own material: the fade scales its alpha; clouds draw without fog
                let mut mat = materials.get(mat).expect("cloud material").clone();
                mat.key.modulate = true;
                mat.uniform.fog = gs::NO_FOG;
                // the game builds each cloud with object flags 3: alpha and VU1's unlit path
                mat.uniform.unlit = 1.0;
                // and sets its TEST to GEQUAL 0x60, FB_ONLY on fail (mode 25's split at 0x70 moved down)
                mat.key.test = match mat.key.test {
                    gs::Test::Ge70 => gs::Test::Ge60,
                    gs::Test::Lt70 => gs::Test::Lt60,
                    t => t,
                };
                commands.entity(e).with_child((Mesh3d(m.clone()), MeshMaterial3d(materials.add(mat))));
            }
            commands.entity(sky.root).add_child(e);
        }
        return;
    }
    sky.ticks += time.delta_secs() * 60.0;
    while sky.ticks >= 1.0 {
        sky.ticks -= 1.0;
        let (d, s) = (sky.degrees, sky.speed);
        hst_sim::clouds::tick(&mut sky.list, d, s);
    }
    // game space is Bevy's turned a half-turn about X
    let eye = camera.translation() * Vec3::new(1.0, -1.0, -1.0);
    let tint = hst_sim::weather::cloud_tint(today.weather);
    for (_, v, mut t, children) in &mut views {
        let k = &sky.list[v.0];
        t.translation = Vec3::from(k.pos) + eye;
        let fade = hst_sim::clouds::fade(k);
        for (c, base) in children.iter().zip(&v.1) {
            // VU1 (flag bit 0): A = vc.a·mat.a·fade, highlight 0; the weather tint scales the light colour, the same
            // product (court 10 GS dump, research/p17s7_cloud_gs.py)
            if let Some(mut m) = parts.get(c).ok().and_then(|m| materials.get_mut(&m.0)) {
                m.uniform.color = (base.truncate() * tint).extend(base.w * fade);
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
