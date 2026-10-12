//! The court intro before the first point (`hst_sim::cutaway::Shot::intro`): the court's camera flyover from its
//! camera file, then the match camera holds until the intro's length (`Rngs::intro_ticks`) is up. The match is
//! held still meanwhile, the way the pause screen holds it (its flare ticks were drawn at setup), so the intro
//! never touches the sim or its RNG; ✕ skips it. Off in online matches (both peers would have to skip together),
//! for `--shot` (unless `HST_INTRO=1`), with `HST_INTRO=0` and without a window.

use std::time::Duration;

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::cutaway::{self, Shot};
use hst_sim::rng::Rngs;

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
}

impl Intro {
    fn showing(&self) -> bool {
        self.at < self.frames
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, start.after(super::setup))
        .add_systems(RunFixedMainLoop, hold.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop))
        .add_systems(PostUpdate, show.before(crate::backdrops).before(bevy::transform::TransformSystems::Propagate));
}

fn start(mut commands: Commands, args: Res<Args>, g: Option<Res<Game>>, window: Query<&Window>, net: Option<Res<super::netplay::Netplay>>) {
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
    commands.insert_resource(Intro { shot: Shot::intro(&shots, &dat), frames, ticks, at: 0, acc: 0.0 });
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
