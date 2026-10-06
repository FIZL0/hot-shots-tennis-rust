//! Ball sandbox: two invisible baseline hitters rally using the game's own shot tables and ported physics,
//! simulated at a fixed 60 Hz and drawn at display rate by interpolating the last two frames.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::ball::{Ball, COURTS, Flight, Shot, V3};
use hst_sim::shot::{Bounds, Table, launch, lookup};

use crate::Args;

#[derive(Component)]
pub struct BallView;

#[derive(Resource)]
struct Sim {
    flight: Flight,
    shot: Shot,
    prev: V3,
    rng: u32,
    stroke: Table,
    /// Frames since the last hit; hitters wait a moment so they don't hit the ball twice.
    since_hit: u32,
}

pub fn plugin(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, simulate)
        .add_systems(Update, (restart, draw));
}

/// Character 0's regular ground stroke table, straight from the disc.
fn stroke_table(iso: &str) -> Table {
    let mut iso = Iso::open(iso).expect("open iso");
    let data = iso.read("TRAJ/TRAJ00A.XB").expect("trajectory archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tr_pc00_strk0.dat")).expect("stroke table");
    Table::parse(&arc.read(e).expect("table bytes")).expect("16^3 table")
}

fn setup(mut commands: Commands, args: Res<Args>) {
    let mut sim = Sim {
        flight: Flight::new(Ball { pos: [0.0; 3], vel: [0.0; 3], spin: 0.0 }, [[0.0; 3]; 3], [[0.0; 3]; 3]),
        shot: Shot::default(),
        prev: [0.0; 3],
        rng: 0x1234_5678,
        stroke: stroke_table(&args.iso),
        since_hit: 0,
    };
    hit(&mut sim, [1.0, -1.0, 11.0]);
    commands.insert_resource(sim);
}

fn rand(state: &mut u32) -> f32 {
    // xorshift; the game's own MT19937 comes with the AI port
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / (1 << 24) as f32
}

/// A regular stroke from `at` to a random spot in the opposite singles court.
fn hit(sim: &mut Sim, at: V3) {
    let far = if at[2] > 0.0 { -1.0 } else { 1.0 };
    let target = [(rand(&mut sim.rng) - 0.5) * 7.0, 0.0, far * (5.0 + rand(&mut sim.rng) * 5.5)];
    let l = lookup(&sim.stroke, &Bounds::stroke(0, at[2]), at, target);
    let vel = launch(at, target, l.elevation, l.speed);
    let dir = Vec3::new(vel[0], 0.0, vel[2]).normalize();
    let side = Vec3::Y.cross(dir).normalize();
    let frame = [side.to_array(), [0.0, 1.0, 0.0], side.cross(Vec3::Y).normalize().to_array()];
    sim.shot = Shot { class: 1, kind: 0, curve_frames: l.frames + 1, ..Shot::default() };
    // ponytail: topspin of a typical recorded stroke; the per-character spin record comes with the shot params port
    sim.flight = Flight::new(Ball { pos: at, vel, spin: 2.9671 }, frame, frame);
    sim.prev = at;
    sim.since_hit = 0;
}

fn simulate(mut sim: ResMut<Sim>, args: Res<Args>) {
    sim.prev = sim.flight.ball.pos;
    let court = COURTS[args.court.min(COURTS.len() - 1)];
    let shot = sim.shot;
    sim.flight.step(&shot, &court);
    sim.since_hit += 1;
    // a hitter takes the ball after its bounce, falling through waist height, behind the service line
    let b = sim.flight.ball;
    if sim.since_hit > 20 && sim.flight.bounces == 1 && b.vel[1] > 0.0 && (-1.2..-0.6).contains(&b.pos[1]) && b.pos[2].abs() > 6.4 {
        info!("hit at {:?} after {} frames", b.pos.map(|c| (c * 100.0).round() / 100.0), sim.since_hit);
        hit(&mut sim, b.pos);
    }
}

fn restart(keys: Res<ButtonInput<KeyCode>>, mut sim: ResMut<Sim>) {
    let dead = sim.flight.frame > 600 || sim.flight.rolling || sim.flight.bounces >= 2 || sim.flight.special_contacts > 0;
    if keys.just_pressed(KeyCode::Space) || dead {
        let f = &sim.flight;
        info!("rally over: bounces {} net {} rolling {} at {:?}", f.bounces, f.special_contacts, f.rolling, f.ball.pos.map(|c| (c * 100.0).round() / 100.0));
        let x = (rand(&mut sim.rng) - 0.5) * 4.0;
        hit(&mut sim, [x, -1.0, 11.5]);
    }
}

fn draw(sim: Res<Sim>, time: Res<Time<Fixed>>, mut q: Query<&mut Transform, With<BallView>>) {
    let p = Vec3::from(sim.prev).lerp(Vec3::from(sim.flight.ball.pos), time.overstep_fraction());
    for mut t in &mut q {
        t.translation = p;
    }
}
