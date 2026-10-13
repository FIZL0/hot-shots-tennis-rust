//! The court intro before the first point (`hst_sim::cutaway::Shot::intro`): the court's camera flyover from its
//! camera file, then the match camera holds until the intro's length (`Rngs::intro_ticks`) is up. The match is
//! held still meanwhile, the way the pause screen holds it (its flare ticks were drawn at setup), so the intro
//! never touches the sim or its RNG; ✕ skips the flyover (the rest of the intro then runs `SKIPPED` ticks; ✕ does
//! nothing after the flyover). Over the flyover the court's title plate (`plate`): name, surface and ball ratings
//! top left, "✕ Skip" bottom right, the score panel, balloons and markers hidden till the intro ends. Its last
//! `BANNER` ticks show the "Start the Match!" banner over the match camera (the INPANE model with the "N Sets N
//! Games" pill and "Set 1"). Its sounds
//! (`sounds`): the court's BGM (with `--music`) from the intro's start, running on into the match; the gallery's
//! cheer from its first tick; the system bank's "Start the Match" sound once the cut to the match camera has faded
//! in. Online (B42e) its ticks run in lockstep on every machine (`netplay::intro_tick`, with the match's delay) and
//! a ✕ on any machine skips it on the same tick everywhere, so the match starts together; there `HST_INTRO` and
//! `--shot` don't count (every peer must run it). Off for `--shot` (unless `HST_INTRO=1`), with `HST_INTRO=0` and
//! without a window.

use std::time::Duration;

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::cutaway::{self, Shot};
use hst_sim::rng::{Mt, Rngs};
use hst_sim::sound::{self, Gallery, Reaction};

use super::popups::{CALL_DISTANCE, CALL_FOV_DEG, CALL_LAYER};
use super::widescreen::{self, Anchor};
use super::{Game, SHOWN_ASPECT};
use crate::audio::{GalleryBank, MusicBanks, Sound, SoundBank};
use crate::{Args, Orbit};

#[derive(Resource)]
struct Intro {
    shot: Shot,
    /// The flyover's frames, the intro's whole length in ticks, and the ticks run.
    frames: u32,
    ticks: u32,
    at: u32,
    acc: f32,
    banner: Banner,
    /// The tick the plate gives way to the banner (the flyover's end + 2, or the ✕), and the ticks whose sounds
    /// have played (`None`: not even the start's).
    banner_tick: u32,
    heard: Option<u32>,
    /// Online: this machine's ✕, held till an intro tick takes it.
    press: bool,
    /// The gallery's cheer, on its own copy of the court generator so the match's stays as the setup left it.
    gallery: Gallery,
    rng: Mt,
}

/// The banner's model, sheets (`inpane_start00`–`02`), its UI root and the match's sets and games to win.
struct Banner {
    effect: hst_sim::effect::Effect,
    shown: crate::effects::Shown,
    root: Entity,
    ui: Entity,
    sheets: [Handle<Image>; 3],
    sets: i32,
    games: i32,
}

/// The banner's ticks at the intro's end: its model's 90 frames and the 2 before the match starts (every court's
/// recorded intro length is the camera shot's end plus these).
const BANNER: u32 = 92;
/// The model's scale and its centre on the 640×448 screen.
const BANNER_SCALE: f32 = 3.3;
const BANNER_AT: [f32; 2] = [321.0, 203.0];
const POOL: usize = 16;

impl Intro {
    fn banner_start(&self) -> u32 {
        self.ticks.saturating_sub(BANNER).max(self.frames)
    }
    fn showing(&self) -> bool {
        self.at < self.banner_start()
    }
    /// ✕: the flyover ends here, the intro `SKIPPED` ticks on.
    fn skip(&mut self) {
        self.frames = self.at;
        self.ticks = self.at + SKIPPED;
        self.banner_tick = self.at;
    }
}

/// The intro's ticks left after ✕ skips the flyover (the game's measured gap on courts 4 and 5).
const SKIPPED: u32 = 92;

/// The "Start the Match" sound's wait after the banner (the 16-frame fade-in of the cut to the match camera, its
/// counter starting 2 higher, then the sound on the first tick it's done; courts 8 and 4).
const BANNER_SOUND: u32 = 17;

/// The system sound bank (slot 9), for the banner's sound (program 0 key 8).
#[derive(Resource)]
struct SystemBank(Option<std::sync::Arc<SoundBank>>);

/// The title plate's sheets (`CourtPreview_00`, the court's name, `info`) and the court's plate row.
#[derive(Resource)]
struct PlateArt([Handle<Image>; 3], [u8; 3]);

#[derive(Component)]
struct PlateSlot(usize);

/// The pill's and words' alpha (128 = opaque) `k` ticks into the banner: up 8 a tick for 15, held 60, down for 15.
fn banner_alpha(k: u32) -> f32 {
    8.0 * match k {
        0..=15 => k,
        16..=75 => 15,
        76..=90 => 90 - k,
        _ => 0,
    } as f32
}

/// The banner's quads: (sheet, src u v w h, dst x y w h, rgb with 128 = 1). The pill's ends stretch to the screen's
/// edges; the digits are the match's sets (best of 2·sets − 1) and games to win, over their white outlines.
fn banner_quads(sets: i32, games: i32) -> Vec<(usize, [f32; 4], [f32; 4], [f32; 3])> {
    let w = [128.0; 3];
    let digit = |n: i32| 112.0 + 24.0 * (n.clamp(1, 6) - 1) as f32;
    let (su, gu) = (digit(2 * sets - 1), digit(games));
    vec![
        (0, [0.0, 0.0, 16.0, 48.0], [0.0, 240.0, 168.0, 48.0], w),
        (0, [16.0, 0.0, 24.0, 48.0], [168.0, 240.0, 24.0, 48.0], w),
        (0, [40.0, 0.0, 16.0, 48.0], [192.0, 240.0, 264.0, 48.0], w),
        (0, [56.0, 0.0, 24.0, 48.0], [456.0, 240.0, 24.0, 48.0], w),
        (0, [80.0, 0.0, 16.0, 48.0], [480.0, 240.0, 168.0, 48.0], w),
        (1, [su, 32.0, 24.0, 32.0], [220.0, 248.0, 24.0, 32.0], w),
        (1, [gu, 32.0, 24.0, 32.0], [324.0, 248.0, 24.0, 32.0], w),
        (1, [su, 0.0, 24.0, 32.0], [220.0, 248.0, 24.0, 32.0], [127.0, 59.0, 59.0]),
        (1, [gu, 0.0, 24.0, 32.0], [324.0, 248.0, 24.0, 32.0], [127.0, 79.0, 18.0]),
        (1, [0.0, 0.0, 80.0, 32.0], [244.0, 248.0, 80.0, 32.0], w),
        (1, [0.0, 32.0, 80.0, 32.0], [348.0, 248.0, 80.0, 32.0], w),
        (2, [0.0, 0.0, 32.0, 32.0], [292.0, 296.0, 32.0, 32.0], w),
        (2, [0.0, 32.0, 24.0, 32.0], [324.0, 296.0, 24.0, 32.0], w),
    ]
}

#[derive(Component)]
struct BannerSlot(usize);

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, start.after(super::setup))
        .add_systems(Update, (draw_plate, sounds))
        .add_systems(PostUpdate, hide_hud.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate))
        .add_systems(RunFixedMainLoop, hold.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop))
        .add_systems(PostUpdate, show.before(crate::backdrops).before(bevy::transform::TransformSystems::Propagate))
        .add_systems(Update, (draw_banner, pose_banner));
}

#[allow(clippy::too_many_arguments)]
fn start(
    mut commands: Commands,
    args: Res<Args>,
    g: Option<Res<Game>>,
    window: Query<&Window>,
    net: Option<ResMut<super::netplay::Netplay>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
) {
    let on = match std::env::var("HST_INTRO").as_deref() {
        _ if net.is_some() => true,
        Ok("0") => false,
        Ok("1") => true,
        _ => args.shot.is_none(),
    };
    let Some(g) = g.filter(|_| on && !window.is_empty()) else { return };
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let n = g.stage as usize;
    let dat = iso.read(&format!("COURT/{n:02}/CMN.XB")).ok().and_then(|d| {
        let arc = Archive::parse(&d).ok()?;
        let e = arc.entries.iter().find(|e| e.name.ends_with(&format!("cam_c{n:02}_00.dat")))?;
        arc.read(e).ok()
    });
    let Some(dat) = dat else { return };
    if let Some(mut net) = net {
        net.start_intro();
    }
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
    let shots = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc").camera_shots();
    let frames = cutaway::intro_frames(&dat);
    let ticks = (Rngs::intro_ticks(n as u32) as u32).max(frames);
    let rng = g.rng.court.clone();
    let bank = SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").map(std::sync::Arc::new);
    commands.insert_resource(SystemBank(bank));
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    // game space (y down) → Bevy, as the call models
    let root = commands.spawn((Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::PI)), Visibility::default())).id();
    let (mut effect, shown) = crate::effects::model(&arc, "azuma/inpane/courtpreview/i_gamestart_00", &mut commands, root, &mut meshes, &mut materials, &mut images, &mut bindposes)
        .expect("i_gamestart_00 in INPANE");
    effect.hold = true;
    // the screen point at the model's distance, on the call camera's 4:3 picture
    let half = CALL_DISTANCE * CALL_FOV_DEG.to_radians().tan();
    let at = Vec3::new((BANNER_AT[0] - 320.0) / 320.0 * half, (BANNER_AT[1] - 224.0) / 224.0 * half * SHOWN_ASPECT, CALL_DISTANCE);
    commands.entity(shown.root).insert(Transform::from_translation(at).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)).with_scale(Vec3::splat(BANNER_SCALE)));
    commands.entity(root).insert_recursive::<Children>(bevy::camera::visibility::RenderLayers::layer(CALL_LAYER));
    let sheets = ["/inpane_start00.tm2", "/inpane_start01.tm2", "/inpane_start02.tm2"].map(|name| {
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name)).unwrap_or_else(|| panic!("{name} in INPANE"));
        super::panel::image(&mut images, &arc.read(e).expect("INPANE bytes"))
    });
    // a full-window root: the pill's ends reach the window's edges (`widescreen::place`)
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    let ui = commands
        .spawn((full, GlobalZIndex(1)))
        .with_children(|p| {
            for i in 0..POOL {
                p.spawn((BannerSlot(i), ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
            }
        })
        .id();
    let banner = Banner { effect, shown, root, ui, sheets, sets: g.rules.sets, games: g.rules.games };
    commands.insert_resource(Intro { shot: Shot::intro(&shots, &dat), frames, ticks, at: 0, acc: 0.0, banner, banner_tick: frames + 2, heard: None, press: false, gallery: Gallery::default(), rng });
    let row = hst_data::exe::court_plates(&iso.read("SCUS_976.10").expect("main program")).expect("supported disc")[n];
    let art = ["/courtpreview_00.tm2".to_string(), format!("/courtpreview_name{n:02}.tm2"), "/2d/info.tm2".into()].map(|name| {
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(&name));
        super::panel::image(&mut images, &arc.read(e.unwrap_or_else(|| panic!("{name} in INPANE"))).expect("INPANE bytes"))
    });
    commands.insert_resource(PlateArt(art, row));
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    commands.spawn((full, GlobalZIndex(1))).with_children(|p| {
        for i in 0..20 {
            p.spawn((PlateSlot(i), ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
        }
    });
}

/// Before the match's ticks each frame: keep them held while the intro runs (the pause screen's closing would
/// restart them), run the intro's own 60 Hz clock (✕, Enter, Space or J skips), and let the match go when it ends.
fn hold(w: &mut World) {
    if !w.contains_resource::<Intro>() {
        return;
    }
    // the match reads no pad through the intro (B125): a ✕ that skips it, or any press while it runs, must not stay
    // latched for the first tick, where it would toss the serve (the pause screen spends its presses the same way)
    if let Some(mut pads) = w.get_resource_mut::<super::Pads>() {
        for s in &mut pads.slots {
            (s.shot, s.serve) = (None, false);
        }
    }
    let paused = super::menu::paused(w);
    let keys = w.resource::<ButtonInput<KeyCode>>();
    let mut skip = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ]);
    let mut q = w.query::<&Gamepad>();
    skip |= q.iter(w).filter(|g| super::controls::readable(g)).any(|g| g.just_pressed(GamepadButton::South));
    let dt = w.resource::<Time<Real>>().delta_secs();
    let online = w.contains_resource::<super::netplay::Netplay>();
    let mut intro = w.remove_resource::<Intro>().unwrap();
    if !online && skip && !paused && intro.showing() {
        intro.skip();
    }
    intro.press |= online && skip && !paused;
    if !paused {
        intro.acc += dt;
        while intro.acc >= 1.0 / 60.0 && intro.at < intro.ticks {
            if online {
                // the tick runs once every seat's ✕ (or not) for it is in, the same on every machine
                let Some(skip) = super::netplay::intro_tick(w, intro.press) else {
                    intro.acc = intro.acc.min(1.0 / 60.0);
                    break;
                };
                intro.press = false;
                if skip && intro.showing() {
                    info!("court intro skipped at its tick {}", intro.at);
                    intro.skip();
                }
            }
            intro.acc -= 1.0 / 60.0;
            intro.at += 1;
            if intro.at <= intro.frames {
                intro.shot.step(Shot::court);
            }
            let start = intro.banner_start();
            if intro.at == start + 1 {
                intro.banner.effect.start();
            } else if intro.at > start + 1 {
                intro.banner.effect.tick();
            }
        }
    }
    w.insert_resource(intro);
    let intro = w.resource::<Intro>();
    let done = intro.at >= intro.ticks;
    let mut fixed = w.resource_mut::<Time<Fixed>>();
    if done {
        fixed.set_timestep_hz(60.0);
        let over = fixed.overstep();
        fixed.discard_overstep(over);
        if let Some(mut np) = w.get_resource_mut::<super::netplay::Netplay>() {
            np.intro = None;
        }
        if let Some(intro) = w.remove_resource::<Intro>() {
            w.entity_mut(intro.banner.root).despawn();
            w.entity_mut(intro.banner.ui).despawn();
        }
    } else {
        fixed.set_timestep(Duration::from_secs(1 << 20));
    }
}

/// The intro's sounds, each tick's once (the original's, recorded on courts 8, 4 and 5): the BGM starts with the
/// intro (the match's ticks leave it running), the gallery cheers from tick 1 as after a point (program 10 from
/// stand to stand, but not in the rain), and the banner's sound plays `BANNER_SOUND` ticks after the banner.
fn sounds(intro: Option<ResMut<Intro>>, g: Option<Res<Game>>, sound: Option<Res<Sound>>, music: Option<Res<MusicBanks>>, gallery: Option<Res<GalleryBank>>, system: Option<Res<SystemBank>>) {
    let (Some(mut intro), Some(g), Some(sound)) = (intro, g, sound) else { return };
    let intro = &mut *intro;
    if intro.heard.is_none() {
        if let Some((bank, mid)) = music.as_ref().and_then(|m| m.bgm.as_ref()) {
            sound.music(bank, mid, 55);
        }
        intro.heard = Some(0);
    }
    let cheers = gallery.and_then(|b| b.0.clone());
    while intro.heard.is_some_and(|t| t < intro.at) {
        let t = intro.heard.unwrap() + 1;
        intro.heard = Some(t);
        let roll = &mut || intro.rng.next();
        if t == 1 {
            intro.gallery.point(Reaction { cheer: true, event: None, chain: false }, roll);
        }
        if !matches!(crate::weather::now(), 2 | 3) {
            for (p, angle) in intro.gallery.step(g.stage, g.players.len() as u32, false, roll) {
                if let Some(b) = &cheers {
                    sound.play_toward(b, p, angle);
                }
            }
        }
        if let (true, Some(Some(b))) = (t == intro.banner_tick + BANNER_SOUND, system.as_ref().map(|s| &s.0)) {
            sound.play_centre(b, sound::Play { slot: 9, program: 0, key: 8, volume: 0x80, speed: 1.0 });
        }
    }
}

/// The flyover's view, rolled, over the match camera (game space → Bevy: (x, −y, −z)).
fn show(intro: Option<Res<Intro>>, mut camera: Query<(&mut Transform, &mut Projection), With<Orbit>>, window: Query<&Window>) {
    let Some(intro) = intro.filter(|i| i.showing()) else { return };
    let Ok((mut t, mut proj)) = camera.single_mut() else { return };
    let v = intro.shot.rolled();
    let b = |p: [f32; 3]| Vec3::new(p[0], -p[1], -p[2]);
    *t = Transform::from_translation(b(v.eye)).looking_to(b(v.rot[2]), -b(v.rot[1]));
    let aspect = window.single().map_or(4.0 / 3.0, |w| w.width() / w.height().max(1.0));
    if let Projection::Perspective(p) = &mut *proj {
        p.fov = 2.0 * (v.fov.tan() * SHOWN_ASPECT.max(1.0 / aspect)).atan();
    }
}

/// One plate quad: sheet (0 `CourtPreview_00`, 1 the name, 2 `info`), source `u v w h`, 640×448 rect `x y w h`,
/// tint and alpha (128 = 1), and how it is placed on a wider window.
type Quad = (usize, [f32; 4], [f32; 4], [u8; 4], Anchor);

/// The surface label's tint per surface (grass, clay, hard).
const SURFACE: [[u8; 3]; 3] = [[41, 120, 25], [114, 72, 31], [26, 100, 127]];

/// The title plate on intro frame `f` (drawn from 1) for a court's plate row (surface, ball ratings): a dark bar
/// across the top at half alpha, the name fading in over 15 frames, its underline, the tinted surface label and the
/// two rows of 4 balls (bright up to the rating, dark after); once the name is in, "✕ Skip" bottom right, its glow
/// blinking every 15 frames. The plate keeps its distance from the left edge as a whole, the bar spans the window
/// and "✕ Skip" keeps to the right edge.
fn plate([surface, a, b]: [u8; 3], f: u32) -> Vec<Quad> {
    let fade = 16u32.saturating_sub(f) as i32;
    let alpha = (128 - fade * 128 / 15).max(0) as u8;
    let (on, l) = ([128, 128, 128, 128], Anchor::Span);
    let tint = SURFACE.get(surface as usize).map_or(on, |c| [c[0], c[1], c[2], 128]);
    let mut q: Vec<Quad> = vec![
        (0, [176.0, 48.0, 64.0, 80.0], [0.0, 0.0, 640.0, 80.0], [128, 128, 128, 64], l),
        (1, [0.0, 0.0, 512.0, 32.0], [32.0, 20.0, 512.0, 32.0], [128, 128, 128, alpha], l),
        (0, [0.0, 69.0, 24.0, 2.0], [16.0, 46.0, 24.0, 2.0], on, l),
        (0, [24.0, 69.0, 24.0, 2.0], [40.0, 46.0, 440.0, 2.0], on, l),
        (0, [48.0, 69.0, 24.0, 2.0], [480.0, 46.0, 24.0, 2.0], on, l),
        (0, [0.0, surface as f32 * 20.0, 135.0, 20.0], [32.0, 52.0, 135.0, 20.0], tint, l),
    ];
    for (row, rating, x) in [(0.0, a, 192.0), (20.0, b, 352.0)] {
        q.push((0, [136.0, row, 48.0, 20.0], [x, 52.0, 48.0, 20.0], on, l));
        for i in 0..4u8 {
            let u = if i < rating { 184.0 } else { 208.0 };
            q.push((0, [u, 0.0, 24.0, 21.0], [x + 48.0 + 24.0 * i as f32, 52.0, 24.0, 21.0], on, l));
        }
    }
    if f >= 16 {
        if (f - 16) / 15 % 2 == 1 {
            q.push((2, [144.0, 160.0, 32.0, 32.0], [512.0, 400.0, 32.0, 32.0], on, Anchor::Sides));
        }
        q.push((2, [224.0, 96.0, 32.0, 32.0], [512.0, 400.0, 32.0, 32.0], on, Anchor::Sides));
        q.push((0, [0.0, 72.0, 80.0, 32.0], [544.0, 400.0, 80.0, 32.0], on, Anchor::Sides));
    }
    q
}

fn draw_plate(
    intro: Option<Res<Intro>>,
    art: Option<Res<PlateArt>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q: Query<(&PlateSlot, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let Some(art) = art else { return };
    let quads = intro.filter(|i| i.showing() && i.at > 0).map_or(vec![], |i| plate(art.1, i.at));
    let share = widescreen::window_share(&window);
    for (PlateSlot(i), mut img, mut node, mut vis) in &mut q {
        let Some(&(sheet, [u, v, w, h], [x, y, dw, dh], [r, g, b, a], anchor)) = quads.get(*i) else {
            *vis = Visibility::Hidden;
            continue;
        };
        img.image = art.0[sheet].clone();
        img.rect = Some(Rect::new(u, v, u + w, v + h));
        img.color = Color::srgba(r as f32 / 128.0, g as f32 / 128.0, b as f32 / 128.0, a as f32 / 128.0);
        let (left, width) = widescreen::place(x, dw, share, anchor);
        node.left = Val::Percent(left);
        node.top = Val::Percent(y / 4.48);
        node.width = Val::Percent(width);
        node.height = Val::Percent(dh / 4.48);
        *vis = Visibility::Inherited;
    }
}

/// The banner's model on the call camera, posed each frame while the banner runs.
fn pose_banner(
    intro: Option<Res<Intro>>,
    mut q: Query<(&mut Visibility, &mut bevy::mesh::morph::MorphWeights)>,
    mut joints: Query<&mut Transform, Without<crate::Orbit>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(intro) = intro else { return };
    crate::effects::pose(&intro.banner.effect, &intro.banner.shown, &mut q, &mut joints, &mut materials);
}

/// The pill, digits and words over the match camera while the banner runs; the pill's middle on the centred 4:3
/// screen, its two outer line pieces stretched out to the window's edges.
fn draw_banner(
    intro: Option<Res<Intro>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q: Query<(&BannerSlot, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let Some(intro) = intro else { return };
    let b = &intro.banner;
    let start = intro.banner_start();
    let alpha = if intro.at > start { banner_alpha(intro.at - start - 1) } else { 0.0 };
    let quads = banner_quads(b.sets, b.games);
    let share = widescreen::window_share(&window);
    let edge = |x: f32| widescreen::place(x, 0.0, share, Anchor::Sides).0;
    for (BannerSlot(i), mut img, mut node, mut vis) in &mut q {
        let Some(&(sheet, [u, v, w, h], [x, y, dw, dh], rgb)) = quads.get(*i).filter(|_| alpha > 0.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        img.image = b.sheets[sheet].clone();
        img.rect = Some(Rect::new(u, v, u + w, v + h));
        img.color = Color::srgba(rgb[0] / 128.0, rgb[1] / 128.0, rgb[2] / 128.0, alpha / 128.0);
        let (left, width) = match *i {
            0 => (edge(0.0), widescreen::place(x + dw, 0.0, share, Anchor::Centre).0 - edge(0.0)),
            4 => {
                let l = widescreen::place(x, 0.0, share, Anchor::Centre).0;
                (l, edge(640.0) - l)
            }
            _ => widescreen::place(x, dw, share, Anchor::Centre),
        };
        node.left = Val::Percent(left);
        node.top = Val::Percent(y / 4.48);
        node.width = Val::Percent(width);
        node.height = Val::Percent(dh / 4.48);
        *vis = Visibility::Inherited;
    }
}

/// The score panel, the timing balloons and the player markers stay off while the intro runs.
fn hide_hud(
    intro: Option<Res<Intro>>,
    mut q: Query<&mut Visibility, Or<(With<super::panel::Slot>, With<super::BalloonView>, With<super::markers::Marker>)>>,
) {
    if intro.is_some() {
        for mut v in &mut q {
            *v = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Court 8 (grass, 2 and 4 balls) against the recorded name alpha per frame and the blink's toggles.
    #[test]
    fn court_8_plate() {
        let alphas: Vec<u8> = (1..=16).map(|f| plate([0, 2, 4], f)[1].3[3]).collect();
        assert_eq!(alphas, [0, 9, 18, 26, 35, 43, 52, 60, 69, 77, 86, 94, 103, 111, 120, 128]);
        let q = plate([0, 2, 4], 1);
        let bright = |x0: f32| q.iter().filter(|q| q.2[0] >= x0 && q.2[0] < x0 + 96.0 && q.1[0] == 184.0).count();
        assert_eq!((bright(240.0), bright(400.0)), (2, 4));
        assert_eq!(q[5].3, [41, 120, 25, 128]);
        let skip = |f| plate([0, 2, 4], f).iter().filter(|q| q.0 == 2).count();
        assert_eq!([skip(15), skip(16), skip(30), skip(31), skip(45), skip(46)], [0, 1, 1, 2, 2, 1]);
        assert!(plate([0, 2, 4], 31).len() <= 20);
    }

    /// 16:9: the bar spans the window, the plate keeps its 4:3 distance from the left edge (the right-hand balls
    /// too), "Skip" keeps its 16 px from the right edge.
    #[test]
    fn pinned_16_9() {
        if !widescreen::WIDE.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let q = plate([0, 2, 4], 40);
        let at = |q: &Quad| widescreen::place(q.2[0], q.2[2], 0.75, q.4);
        assert_eq!(at(&q[0]), (0.0, 100.0));
        let ball = q.iter().find(|q| q.2[0] == 472.0).unwrap();
        assert!((at(ball).0 - 472.0 / 6.4 * 0.75).abs() < 1e-4);
        let (l, w) = at(q.last().unwrap());
        assert!((100.0 - l - w - 16.0 / 6.4 * 0.75).abs() < 1e-4);
    }

    #[test]
    fn banner_fade() {
        let a: Vec<f32> = (0..92).map(banner_alpha).collect();
        assert_eq!((a[0], a[1], a[15], a[75], a[76], a[89], a[90]), (0.0, 8.0, 120.0, 120.0, 112.0, 8.0, 0.0));
        assert!(a[15..=75].iter().all(|&x| x == 120.0));
    }

    #[test]
    fn banner_digits() {
        // one set to 4 games: "1 Sets 4 Games"; best of 3 sets to 6: "3 Sets 6 Games"
        for (sets, games, su, gu) in [(1, 4, 112.0, 184.0), (2, 6, 160.0, 232.0)] {
            let q = banner_quads(sets, games);
            assert_eq!((q[7].1[0], q[8].1[0]), (su, gu));
        }
        // the pill spans the screen and past it (648)
        let q = banner_quads(1, 4);
        assert_eq!(q[..5].iter().map(|q| q.2[2]).sum::<f32>(), 648.0);
    }
}
