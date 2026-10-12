//! The court intro before the first point (`hst_sim::cutaway::Shot::intro`): the court's camera flyover from its
//! camera file, then the match camera holds until the intro's length (`Rngs::intro_ticks`) is up. The match is
//! held still meanwhile, the way the pause screen holds it (its flare ticks were drawn at setup), so the intro
//! never touches the sim or its RNG; ✕ skips the flyover (the rest of the intro then runs `SKIPPED` ticks; ✕ does
//! nothing after the flyover). Over the flyover the court's title plate (`plate`): name, surface and ball ratings
//! top left, "✕ Skip" bottom right, the score panel, balloons and markers hidden till the intro ends. Its sounds
//! (`sounds`): the court's BGM (with `--music`) from the intro's start, running on into the match; the gallery's
//! cheer from its first tick; the system bank's "Start the Match" sound once the cut to the match camera has faded
//! in. Off in online matches (both peers would have to skip together),
//! for `--shot` (unless `HST_INTRO=1`), with `HST_INTRO=0` and without a window.

use std::time::Duration;

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::cutaway::{self, Shot};
use hst_sim::rng::{Mt, Rngs};
use hst_sim::sound::{self, Gallery, Reaction};

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
    /// The tick the plate gives way to the banner (the flyover's end + 2, or the ✕), and the ticks whose sounds
    /// have played (`None`: not even the start's).
    banner: u32,
    heard: Option<u32>,
    /// The gallery's cheer, on its own copy of the court generator so the match's stays as the setup left it.
    gallery: Gallery,
    rng: Mt,
}

impl Intro {
    fn showing(&self) -> bool {
        self.at < self.frames
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

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, start.after(super::setup))
        .add_systems(Update, (draw_plate, sounds))
        .add_systems(PostUpdate, hide_hud.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate))
        .add_systems(RunFixedMainLoop, hold.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop))
        .add_systems(PostUpdate, show.before(crate::backdrops).before(bevy::transform::TransformSystems::Propagate));
}

fn start(mut commands: Commands, mut images: ResMut<Assets<Image>>, args: Res<Args>, g: Option<Res<Game>>, window: Query<&Window>, net: Option<Res<super::netplay::Netplay>>) {
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
    let rng = g.rng.court.clone();
    commands.insert_resource(Intro { shot: Shot::intro(&shots, &dat), frames, ticks, at: 0, acc: 0.0, banner: frames + 2, heard: None, gallery: Gallery::default(), rng });
    let bank = SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").map(std::sync::Arc::new);
    commands.insert_resource(SystemBank(bank));
    let row = hst_data::exe::court_plates(&iso.read("SCUS_976.10").expect("main program")).expect("supported disc")[n];
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
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
    let paused = super::menu::paused(w);
    let keys = w.resource::<ButtonInput<KeyCode>>();
    let mut skip = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ]);
    let mut q = w.query::<&Gamepad>();
    skip |= q.iter(w).filter(|g| super::controls::readable(g)).any(|g| g.just_pressed(GamepadButton::South));
    let dt = w.resource::<Time<Real>>().delta_secs();
    let mut intro = w.resource_mut::<Intro>();
    if skip && !paused && intro.showing() {
        intro.frames = intro.at;
        intro.ticks = intro.at + SKIPPED;
        intro.banner = intro.at;
    }
    if !paused {
        intro.acc += dt;
        while intro.acc >= 1.0 / 60.0 && intro.at < intro.ticks {
            intro.acc -= 1.0 / 60.0;
            intro.at += 1;
            if intro.at <= intro.frames {
                intro.shot.step(Shot::court);
            }
        }
    }
    let done = intro.at >= intro.ticks;
    let mut fixed = w.resource_mut::<Time<Fixed>>();
    if done {
        fixed.set_timestep_hz(60.0);
        let over = fixed.overstep();
        fixed.discard_overstep(over);
        w.remove_resource::<Intro>();
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
        if let (true, Some(Some(b))) = (t == intro.banner + BANNER_SOUND, system.as_ref().map(|s| &s.0)) {
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
}
