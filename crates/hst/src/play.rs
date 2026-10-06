//! Playable test mode (`--play`): you (blue, near side) against a simple AI (red), using the ported ball
//! physics and the game's own shot tables. Players are stand-in figures; their movement, swing timing, AI
//! and serve are placeholders until those systems are ported (see TODO.md).
//!
//! Keyboard: WASD move, J topspin, K slice, L lob, I flat, U drive (shot kind 4). Space or any shot serves.
//! Gamepad: left stick move, A topspin, B slice, Y lob, X flat, RB drive.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::ball::{Ball, COURTS, Flight, Shot, V3};
use hst_sim::shot::{Bounds, Table, launch, lookup};

use crate::{Args, GameSpace};

const HALF_LENGTH: f32 = 11.885;
const SINGLES_HALF_WIDTH: f32 = 4.115;
/// Metres per frame at full stick (≈ 6 m/s).
const RUN: f32 = 0.1;
/// How far from the body the racket can take the ball, horizontally.
const REACH: f32 = 1.6;
/// A swing stays live this many frames after the button, so early presses still connect.
const SWING_WINDOW: u32 = 14;
/// Typical recorded spin per shot kind (rad/frame); the real per-character records are not ported yet.
const KIND_SPIN: [f32; 5] = [2.9671, -2.0944, 0.0, 5.8905, 3.7088];

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Near,
    Far,
}

impl Side {
    /// Sign of z on this side of the net (game space).
    fn z(self) -> f32 {
        if self == Side::Near { 1.0 } else { -1.0 }
    }
    fn other(self) -> Side {
        if self == Side::Near { Side::Far } else { Side::Near }
    }
}

#[derive(Clone, Copy)]
struct Player {
    pos: V3,
    prev: V3,
    swing: u32,
    kind: i32,
}

#[derive(PartialEq)]
enum Phase {
    /// Waiting for `server` to serve.
    Serve,
    Rally,
    /// Point over; short pause before the next serve.
    Over(u32),
}

#[derive(Resource)]
struct Game {
    tables: Vec<Table>,
    flight: Flight,
    shot: Shot,
    prev_ball: V3,
    players: [Player; 2],
    phase: Phase,
    server: Side,
    last_hitter: Side,
    since_hit: u32,
    first_bounce_checked: bool,
    score: [u32; 2],
    message: String,
    rng: u32,
}

#[derive(Component)]
struct PlayerView(usize);
#[derive(Component)]
struct BallView;
#[derive(Component)]
struct ScoreText;

pub fn plugin(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_systems(PostStartup, setup) // after the court's game-space root exists
        .add_systems(FixedUpdate, (human, ai, simulate).chain())
        .add_systems(Update, (draw, hud));
}

/// Character 0's stroke tables (kinds 0..4) straight from the disc.
fn tables(iso: &str) -> Vec<Table> {
    let mut iso = Iso::open(iso).expect("open iso");
    let data = iso.read("TRAJ/TRAJ00A.XB").expect("trajectory archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    (0..5)
        .map(|k| {
            let name = format!("tr_pc00_strk{k}.dat");
            let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&name)).expect("stroke table");
            Table::parse(&arc.read(e).expect("table bytes")).expect("16^3 table")
        })
        .collect()
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    root: Query<Entity, With<GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let idle = Player { pos: [0.0; 3], prev: [0.0; 3], swing: 0, kind: 0 };
    let mut game = Game {
        tables: tables(&args.iso),
        flight: Flight::new(Ball { pos: [0.0; 3], vel: [0.0; 3], spin: 0.0 }, [[0.0; 3]; 3], [[0.0; 3]; 3]),
        shot: Shot::default(),
        prev_ball: [0.0; 3],
        players: [idle; 2],
        phase: Phase::Serve,
        server: Side::Near,
        last_hitter: Side::Near,
        since_hit: 0,
        first_bounce_checked: false,
        score: [0; 2],
        message: "Your serve: press J or Space".into(),
        rng: 0x2468_ace1,
    };
    reset_positions(&mut game);
    commands.insert_resource(game);

    // stand-in figures: body, head and racket in each player's colour (game space is Y-down)
    let body = meshes.add(Capsule3d::new(0.22, 1.1));
    let head = meshes.add(Sphere::new(0.14));
    let racket = meshes.add(Cuboid::new(0.05, 0.6, 0.3));
    let Ok(root) = root.single() else { return };
    for (i, color) in [Color::srgb(0.2, 0.45, 1.0), Color::srgb(0.95, 0.25, 0.2)].into_iter().enumerate() {
        let mat = materials.add(StandardMaterial { base_color: color, ..default() });
        let p = commands
            .spawn((PlayerView(i), Transform::default(), Visibility::default()))
            .with_children(|c| {
                c.spawn((Mesh3d(body.clone()), MeshMaterial3d(mat.clone()), Transform::from_xyz(0.0, -0.8, 0.0)));
                c.spawn((Mesh3d(head.clone()), MeshMaterial3d(mat.clone()), Transform::from_xyz(0.0, -1.6, 0.0)));
                c.spawn((Mesh3d(racket.clone()), MeshMaterial3d(mat.clone()), Transform::from_xyz(0.45, -1.0, 0.0)));
            })
            .id();
        commands.entity(root).add_child(p);
    }
    let ball = meshes.add(Sphere::new(0.033));
    let ball_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 1.0, 0.2), unlit: true, ..default() });
    let b = commands.spawn((BallView, Mesh3d(ball), MeshMaterial3d(ball_mat), Transform::default())).id();
    commands.entity(root).add_child(b);
    commands.spawn((DirectionalLight { illuminance: 8000.0, ..default() }, Transform::from_xyz(4.0, 10.0, -6.0).looking_at(Vec3::ZERO, Vec3::Y)));
    commands.spawn((
        ScoreText,
        Text::new(""),
        Node { position_type: PositionType::Absolute, top: Val::Px(12.0), left: Val::Px(12.0), ..default() },
    ));
}

fn reset_positions(g: &mut Game) {
    let sx = if (g.score[0] + g.score[1]) % 2 == 0 { -1.0 } else { 1.0 }; // deuce/ad court alternates
    for (i, side) in [Side::Near, Side::Far].into_iter().enumerate() {
        let x = if side == g.server { sx * 1.0 * side.z() } else { -sx * 2.5 * side.z() };
        g.players[i].pos = [x, 0.0, side.z() * (HALF_LENGTH + 0.3)];
        g.players[i].prev = g.players[i].pos;
        g.players[i].swing = 0;
    }
    let s = g.players[g.server as usize].pos;
    g.flight = Flight::new(Ball { pos: [s[0] + 0.3, -1.0, s[2]], vel: [0.0; 3], spin: 0.0 }, [[0.0; 3]; 3], [[0.0; 3]; 3]);
    g.prev_ball = g.flight.ball.pos;
}

fn rand(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / (1 << 24) as f32
}

/// Launch a stroke of `kind` by `who` from the ball's position toward `target`.
fn strike(g: &mut Game, who: Side, kind: i32, target: V3) {
    let at = g.flight.ball.pos;
    let l = lookup(&g.tables[kind as usize], &Bounds::stroke(kind, at[2]), at, target);
    let vel = launch(at, target, l.elevation, l.speed);
    let dir = Vec3::new(vel[0], 0.0, vel[2]).normalize_or(Vec3::Z);
    let side = Vec3::Y.cross(dir).normalize();
    let frame = [side.to_array(), [0.0, 1.0, 0.0], side.cross(Vec3::Y).normalize().to_array()];
    debug!("strike by {} kind {kind} from {at:?} to {target:?}: elev {:.3} speed {:.3} frames {}", who as usize, l.elevation, l.speed, l.frames);
    g.shot = Shot { class: 1, kind, curve_frames: l.frames + 1, ..Shot::default() };
    g.flight = Flight::new(Ball { pos: at, vel, spin: KIND_SPIN[kind as usize] }, frame, frame);
    g.prev_ball = at;
    g.last_hitter = who;
    g.since_hit = 0;
    g.first_bounce_checked = false;
    g.phase = Phase::Rally;
}

/// Placeholder serve: hit from overhead height into the diagonal service box.
fn serve(g: &mut Game, who: Side, kind: i32) {
    let p = g.players[who as usize].pos;
    g.flight.ball.pos = [p[0] + 0.3 * who.z(), -2.6, p[2]];
    let target = [-p[0].signum() * 2.0, 0.0, -who.z() * 5.2];
    strike(g, who, if kind == 3 { 0 } else { kind }, target);
}

fn can_hit(g: &Game, i: usize, side: Side) -> bool {
    let (b, p) = (g.flight.ball, g.players[i].pos);
    let dx = b.pos[0] - p[0];
    let dz = b.pos[2] - p[2];
    g.phase == Phase::Rally
        && g.last_hitter != side
        && g.since_hit > 8
        && b.pos[2] * side.z() > 0.0
        && (-2.4..-0.15).contains(&b.pos[1])
        && (dx * dx + dz * dz).sqrt() < REACH
}

fn human(mut g: ResMut<Game>, keys: Res<ButtonInput<KeyCode>>, pads: Query<&Gamepad>) {
    // HST_AUTOPLAY: the bot plays your side too (unattended soak tests)
    if std::env::var_os("HST_AUTOPLAY").is_some() {
        bot(&mut g, Side::Near);
        return;
    }
    let mut stick = Vec2::ZERO;
    for (k, d) in [(KeyCode::KeyW, Vec2::Y), (KeyCode::KeyS, -Vec2::Y), (KeyCode::KeyA, -Vec2::X), (KeyCode::KeyD, Vec2::X)] {
        if keys.pressed(k) {
            stick += d;
        }
    }
    let mut shot = [(KeyCode::KeyJ, 0), (KeyCode::KeyK, 1), (KeyCode::KeyI, 2), (KeyCode::KeyL, 3), (KeyCode::KeyU, 4)]
        .into_iter()
        .find(|(k, _)| keys.just_pressed(*k))
        .map(|(_, kind)| kind);
    for pad in &pads {
        stick += pad.left_stick();
        let buttons = [
            (GamepadButton::South, 0),
            (GamepadButton::East, 1),
            (GamepadButton::West, 2),
            (GamepadButton::North, 3),
            (GamepadButton::RightTrigger, 4),
        ];
        shot = shot.or(buttons.into_iter().find(|(b, _)| pad.just_pressed(*b)).map(|(_, k)| k));
    }
    let stick = stick.clamp_length_max(1.0);
    if g.phase == Phase::Serve && g.server == Side::Near {
        if let Some(kind) = shot.or(keys.just_pressed(KeyCode::Space).then_some(0)) {
            serve(&mut g, Side::Near, kind);
        }
        return;
    }
    // camera looks from behind the near baseline toward -z: screen right is -x, up is toward the net
    let p = &mut g.players[0];
    p.prev = p.pos;
    p.pos[0] = (p.pos[0] - stick.x * RUN).clamp(-8.0, 8.0);
    p.pos[2] = (p.pos[2] - stick.y * RUN).clamp(0.4, 17.0);
    if let Some(kind) = shot {
        p.swing = SWING_WINDOW;
        p.kind = kind;
    }
    if p.swing > 0 {
        p.swing -= 1;
        if can_hit(&g, 0, Side::Near) {
            let kind = g.players[0].kind;
            let target = [-stick.x * 3.3, 0.0, -(if stick.y < -0.3 { 6.5 } else { 9.0 })];
            strike(&mut g, Side::Near, kind, target);
            g.players[0].swing = 0;
        }
    }
}

/// Where the ball will be hittable on `side`: step a copy of the flight until it is waist-high after its bounce.
fn intercept(g: &Game, side: Side) -> Option<V3> {
    let mut f = g.flight;
    f.net = true;
    for _ in 0..240 {
        f.step(&g.shot, &COURTS[0]);
        let b = f.ball;
        if b.pos[2] * side.z() > 0.0 && f.bounces >= 1 && b.vel[1] > 0.0 && (-1.3..-0.5).contains(&b.pos[1]) {
            return Some(b.pos);
        }
        if f.bounces >= 2 {
            return None;
        }
    }
    None
}

fn ai(mut g: ResMut<Game>) {
    bot(&mut g, Side::Far);
}

/// Simple stand-in opponent for `side`: serves at once, runs to the predicted interception and returns.
fn bot(g: &mut Game, side: Side) {
    let i = side as usize;
    if g.phase == Phase::Serve && g.server == side {
        if side == Side::Far {
            g.message = "Opponent serving".into();
        }
        let kind = if rand(&mut g.rng) < 0.5 { 0 } else { 2 };
        serve(g, side, kind);
        return;
    }
    let goal = match g.phase {
        Phase::Rally if g.last_hitter != side => intercept(g, side).map(|p| [p[0] - 0.45 * side.z(), 0.0, p[2] + 0.3 * side.z()]),
        _ => None,
    }
    .unwrap_or([0.0, 0.0, side.z() * (HALF_LENGTH + 0.5)]);
    let p = &mut g.players[i];
    p.prev = p.pos;
    let d = Vec2::new(goal[0] - p.pos[0], goal[2] - p.pos[2]);
    let step = d.clamp_length_max(RUN * 0.9);
    p.pos[0] += step.x;
    p.pos[2] += step.y;
    if can_hit(g, i, side) {
        let r = rand(&mut g.rng);
        let kind = if r < 0.6 { 0 } else if r < 0.8 { 1 } else if r < 0.9 { 2 } else { 3 };
        let target = [(rand(&mut g.rng) - 0.5) * 6.0, 0.0, -side.z() * (6.0 + rand(&mut g.rng) * 4.0)];
        strike(g, side, kind, target);
    }
}

fn simulate(mut g: ResMut<Game>, args: Res<Args>) {
    match g.phase {
        Phase::Serve => return,
        Phase::Over(0) => {
            g.server = g.server.other();
            g.phase = Phase::Serve;
            reset_positions(&mut g);
            if g.server == Side::Near {
                g.message = "Your serve: press J or Space".into();
            }
            return;
        }
        Phase::Over(n) => {
            g.phase = Phase::Over(n - 1);
        }
        Phase::Rally => {}
    }
    g.prev_ball = g.flight.ball.pos;
    let shot = g.shot;
    g.flight.step(&shot, &COURTS[args.court.min(COURTS.len() - 1)]);
    g.since_hit += 1;
    if g.phase != Phase::Rally {
        return;
    }
    let (b, hitter) = (g.flight.ball, g.last_hitter);
    let receiver = hitter.other();
    let mut winner = None;
    if g.flight.special_contacts > 0 && b.pos[2] * hitter.z() > 0.0 && g.since_hit > 30 {
        winner = Some((receiver, "Net"));
    } else if g.flight.bounces >= 1 && !g.first_bounce_checked {
        g.first_bounce_checked = true;
        let inside = b.pos[0].abs() <= SINGLES_HALF_WIDTH + 0.033 && b.pos[2] * receiver.z() > 0.0 && b.pos[2].abs() <= HALF_LENGTH + 0.033;
        if !inside {
            winner = Some((receiver, "Out"));
        }
    } else if g.flight.bounces >= 2 || g.flight.rolling {
        winner = Some((hitter, "Winner"));
    } else if g.since_hit > 400 {
        winner = Some((hitter, "Winner"));
    }
    if let Some((side, why)) = winner {
        g.score[side as usize] += 1;
        g.message = format!("{why} - point to {}", if side == Side::Near { "you" } else { "opponent" });
        info!("point: {} ({why}) after {} frames, score {:?}", if side == Side::Near { "near" } else { "far" }, g.since_hit, g.score);
        g.phase = Phase::Over(90);
    }
}

fn draw(g: Res<Game>, time: Res<Time<Fixed>>, mut players: Query<(&PlayerView, &mut Transform)>, mut ball: Query<&mut Transform, (With<BallView>, Without<PlayerView>)>) {
    let a = time.overstep_fraction();
    for (v, mut t) in &mut players {
        let p = g.players[v.0];
        t.translation = Vec3::from(p.prev).lerp(Vec3::from(p.pos), a);
        t.rotation = if v.0 == 0 { Quat::from_rotation_y(std::f32::consts::PI) } else { Quat::IDENTITY };
    }
    for mut t in &mut ball {
        t.translation = Vec3::from(g.prev_ball).lerp(Vec3::from(g.flight.ball.pos), a);
    }
}

fn hud(g: Res<Game>, mut q: Query<&mut Text, With<ScoreText>>) {
    for mut t in &mut q {
        t.0 = format!("You {}  -  {} Opponent\n{}\nWASD move  J topspin  K slice  I flat  L lob  U drive", g.score[0], g.score[1], g.message);
    }
}
