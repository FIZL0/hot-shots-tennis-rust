//! The court intro before the first point (`hst_sim::cutaway::Shot::intro`): the court's camera flyover from its
//! camera file, then the match camera holds until the intro's length (`Rngs::intro_ticks`) is up. The match is
//! held still meanwhile, the way the pause screen holds it (its flare ticks were drawn at setup), so the intro
//! never touches the sim or its RNG; ✕ skips it. Its last `BANNER` ticks show the "Start the Match!" banner over
//! the match camera (the INPANE model with the "N Sets N Games" pill and "Set 1"). Off in online matches (both peers would have to skip together),
//! for `--shot` (unless `HST_INTRO=1`), with `HST_INTRO=0` and without a window.

use std::time::Duration;

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::cutaway::{self, Shot};
use hst_sim::rng::Rngs;

use super::popups::{CALL_DISTANCE, CALL_FOV_DEG, CALL_LAYER};
use super::widescreen::{self, Anchor};
use super::{Game, SHOWN_ASPECT};
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
}

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
    net: Option<Res<super::netplay::Netplay>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
) {
    let on = match std::env::var("HST_INTRO").as_deref() {
        Ok("0") => false,
        Ok("1") => true,
        _ => args.shot.is_none(),
    };
    let Some(g) = g.filter(|_| on && net.is_none() && !window.is_empty()) else { return };
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let n = g.stage as usize;
    let dat = iso.read(&format!("COURT/{n:02}/CMN.XB")).ok().and_then(|d| {
        let arc = Archive::parse(&d).ok()?;
        let e = arc.entries.iter().find(|e| e.name.ends_with(&format!("cam_c{n:02}_00.dat")))?;
        arc.read(e).ok()
    });
    let Some(dat) = dat else { return };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
    let shots = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc").camera_shots();
    let frames = cutaway::intro_frames(&dat);
    let ticks = (Rngs::intro_ticks(n as u32) as u32).max(frames);
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
    commands.insert_resource(Intro { shot: Shot::intro(&shots, &dat), frames, ticks, at: 0, acc: 0.0, banner });
}

/// Before the match's ticks each frame: keep them held while the intro runs (the pause screen's closing would
/// restart them), run the intro's own 60 Hz clock (✕, Enter, Space or J skips), and let the match go when it ends.
fn hold(w: &mut World) {
    if !w.contains_resource::<Intro>() {
        return;
    }
    let paused = super::menu::paused(w);
    let keys = w.resource::<ButtonInput<KeyCode>>();
    let mut skip = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ]);
    let mut q = w.query::<&Gamepad>();
    skip |= q.iter(w).filter(|g| super::controls::readable(g)).any(|g| g.just_pressed(GamepadButton::South));
    let dt = w.resource::<Time<Real>>().delta_secs();
    let mut intro = w.resource_mut::<Intro>();
    if skip && !paused {
        intro.at = intro.ticks;
    }
    if !paused {
        intro.acc += dt;
        while intro.acc >= 1.0 / 60.0 && intro.at < intro.ticks {
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
    let done = intro.at >= intro.ticks;
    let mut fixed = w.resource_mut::<Time<Fixed>>();
    if done {
        fixed.set_timestep_hz(60.0);
        let over = fixed.overstep();
        fixed.discard_overstep(over);
        if let Some(intro) = w.remove_resource::<Intro>() {
            w.entity_mut(intro.banner.root).despawn();
            w.entity_mut(intro.banner.ui).despawn();
        }
    } else {
        fixed.set_timestep(Duration::from_secs(1 << 20));
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

#[cfg(test)]
mod tests {
    use super::*;

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
