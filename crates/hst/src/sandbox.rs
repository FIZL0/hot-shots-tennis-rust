//! Ball sandbox: the ported ball physics at a fixed 60 Hz, drawn at display rate by interpolating
//! between the last two simulated frames.

use bevy::prelude::*;
use hst_sim::ball::{Ball, COURTS, Flight, Shot, V3};

use crate::Args;

#[derive(Component)]
pub struct BallView;

#[derive(Resource)]
struct Sim {
    flight: Flight,
    shot: Shot,
    prev: V3,
    seed: u32,
}

pub fn plugin(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(Sim { flight: launch(1), shot: Shot::default(), prev: [0.0; 3], seed: 1 })
        .add_systems(FixedUpdate, simulate)
        .add_systems(Update, (relaunch, draw));
}

/// Placeholder serve from behind the near baseline toward the far court.
// ponytail: stand-in launch until the game's shot creation is ported; keeps the sandbox watchable.
fn launch(seed: u32) -> Flight {
    let r = |k: u32| ((seed.wrapping_mul(2654435761).rotate_left(k * 7) >> 8) as f32 / (1 << 24) as f32) - 0.5;
    let dir = Vec3::new(r(1) * 0.25, 0.0, -1.0).normalize();
    let speed = 0.42 + r(2) * 0.1; // metres per frame (~25 m/s)
    let vel = [dir.x * speed, -0.06 + r(3) * 0.04, dir.z * speed];
    let axis = Vec3::Y;
    let side = axis.cross(dir).normalize();
    let fwd = side.cross(axis).normalize();
    let frame = [side.to_array(), axis.to_array(), fwd.to_array()];
    let ball = Ball { pos: [r(4) * 4.0, -1.2, 11.9], vel, spin: 2.97 + r(5) * 2.0 };
    Flight::new(ball, frame, frame)
}

fn simulate(mut sim: ResMut<Sim>, args: Res<Args>) {
    let Sim { flight, shot, prev, .. } = &mut *sim;
    *prev = flight.ball.pos;
    flight.step(shot, &COURTS[args.court.min(COURTS.len() - 1)]);
}

fn relaunch(keys: Res<ButtonInput<KeyCode>>, mut sim: ResMut<Sim>) {
    let done = sim.flight.frame > 600 || (sim.flight.rolling && len(sim.flight.ball.vel) < 1e-3);
    if keys.just_pressed(KeyCode::Space) || done {
        sim.seed += 1;
        sim.flight = launch(sim.seed);
        sim.prev = sim.flight.ball.pos;
    }
}

fn len(v: V3) -> f32 {
    Vec3::from(v).length()
}

fn draw(sim: Res<Sim>, time: Res<Time<Fixed>>, mut q: Query<&mut Transform, With<BallView>>) {
    let a = time.overstep_fraction();
    let p = Vec3::from(sim.prev).lerp(Vec3::from(sim.flight.ball.pos), a);
    for mut t in &mut q {
        t.translation = p;
    }
}
