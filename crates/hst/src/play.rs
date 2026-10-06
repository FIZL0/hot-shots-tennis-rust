//! Playable test mode (`--play`): you (near side) against a simple AI, on the ported ball physics and the game's
//! own shot tables. Players are original stand-in athletes (`figure.rs`); movement tuning, swing timing, AI and
//! the serve are placeholders until those systems are ported (see TODO.md).
//!
//! The stick (or WASD) held at contact aims the shot anywhere in the opponent's court; a red dot marks where
//! the ball will bounce.
//!
//! Keyboard: WASD move (aim while swinging), Shift sprint, J topspin, K slice, I flat, L lob, U drive,
//! J/Space serve, C cycle camera, arrow keys turn the camera in follow/free mode.
//! Gamepad: left stick move/aim, LB sprint, A topspin, B slice, X flat, Y lob, RB drive, A/Start serve,
//! Select cycle camera, right stick turn the camera.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::ball::{Ball, COURTS, Flight, Shot, V3, rows4};
use hst_sim::judge::{BallState, Lines, Rally};
use hst_sim::score::{Event, Rules, Score};
use hst_sim::shot::{Bounds, Table, launch, lookup};

use crate::{Args, GameSpace, Orbit, figure};

/// The original's default exhibition: one set to 4 games, deuce on.
const RULES: Rules = Rules { sets: 1, games: 4, no_deuce: false, one_point_games: false, players: 2 };
const HALF_LENGTH: f32 = 11.885;
/// The umpire's calls by verdict code.
const CALLS: [&str; 7] = ["Point", "Out", "Fault", "Double fault", "Let", "Out", "Illegal hit"];
/// Top running speed in metres per frame (≈ 6 m/s); sprint multiplies it.
const RUN: f32 = 0.1;
const SPRINT: f32 = 1.35;
/// Speed change per frame while accelerating / braking.
const ACCEL: f32 = 0.012;
const BRAKE: f32 = 0.025;
/// A serve swing lasts this many frames.
const SWING_FRAMES: u32 = 30;
/// Serve: frames from toss to contact.
const SERVE_CONTACT: u32 = 18;
/// Ball drawn this much larger than its physical size, toon style, so it reads at broadcast distance.
const BALL_DRAW_SCALE: f32 = 2.4;
/// Typical recorded spin per shot kind (rad/frame); the real per-character records are not ported yet.
const KIND_SPIN: [f32; 5] = [2.9671, -2.0944, 0.0, 5.8905, 3.7088];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Side {
    Near,
    Far,
}

impl Side {
    /// Sign of z on this side of the net (game space).
    fn z(self) -> f32 {
        if self == Side::Near { 1.0 } else { -1.0 }
    }
}

#[derive(Clone, Copy, Default)]
struct Player {
    pos: V3,
    prev: V3,
    vel: Vec2,
    /// Yaw of the figure; 0 faces -z (toward the far court).
    facing: f32,
    stride: f32,
    /// Frames into the current swing.
    swing: Option<u32>,
    swung: bool,
    backhand: bool,
    serving: bool,
    kind: i32,
    aim: Vec2,
    /// A shot press still looking for its contact (frames it stays live).
    pending: Option<u32>,
    contact: Option<Contact>,
    /// Frames of wind-up before contact (sets the animation clock).
    wind: u32,
}

#[derive(PartialEq)]
enum Phase {
    Serve,
    Rally,
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
    score: Score,
    rally: Rally,
    /// Shots this rally (1 = the serve).
    shots: i32,
    /// Line tolerance from the game program.
    line_margin: f32,
    message: String,
    rng: u32,
    /// Timing pop-up: text, frames left, grade.
    popup: Option<(&'static str, u32, u8)>,
}

/// Controller state gathered every display frame and consumed by the 60 Hz simulation.
#[derive(Resource, Default)]
struct Pad {
    stick: Vec2,
    sprint: bool,
    /// Latched shot press (kind), so a press between fixed steps is never lost.
    shot: Option<i32>,
    serve: bool,
}

#[derive(Resource, Default, Clone, Copy, PartialEq)]
enum CamMode {
    #[default]
    Follow,
    Broadcast,
    Free,
}

#[derive(Resource, Default)]
struct CamState {
    turn: f32,
    focus: Vec3,
}

#[derive(Component)]
struct Figure(usize);
#[derive(Component)]
struct BallView;
/// Red dot on the court where the ball in flight will first bounce.
#[derive(Component)]
struct LandingMark;
#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct PopupText;

pub fn plugin(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<Pad>()
        .init_resource::<CamMode>()
        .init_resource::<CamState>()
        .add_systems(PostStartup, setup) // after the court's game-space root exists
        .add_systems(Update, (read_input, camera, draw, mark_landing, figure::animate, hud).chain())
        .add_systems(FixedUpdate, (human, ai, simulate, popup).chain());
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

fn line_margin(iso: &str) -> f32 {
    let mut iso = Iso::open(iso).expect("open iso");
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
    hst_data::exe::Game::new(&cnf, &bin).expect("supported disc").line_margin()
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    root: Query<Entity, With<GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut game = Game {
        tables: tables(&args.iso),
        flight: Flight::new(Ball { pos: [0.0; 3], vel: [0.0; 3], spin: 0.0 }, [[0.0; 4]; 4], [[0.0; 4]; 4]),
        shot: Shot::default(),
        prev_ball: [0.0; 3],
        players: [Player::default(); 2],
        phase: Phase::Serve,
        server: Side::Near,
        last_hitter: Side::Near,
        since_hit: 0,
        score: Score::new(),
        rally: Rally::default(),
        shots: 0,
        line_margin: line_margin(&args.iso),
        message: "Your serve".into(),
        rng: 0x2468_ace1,
        popup: None,
    };
    reset_positions(&mut game);
    commands.insert_resource(game);

    let Ok(root) = root.single() else { return };
    for (i, shirt) in [Color::srgb(0.15, 0.4, 0.95), Color::srgb(0.9, 0.3, 0.15)].into_iter().enumerate() {
        let f = figure::spawn(&mut commands, &mut meshes, &mut materials, shirt, root);
        commands.entity(f).insert(Figure(i));
    }
    // toon ball: flat yellow core plus an inverted hull (front faces culled) for the black outline
    let r = 0.033 * BALL_DRAW_SCALE;
    let core = materials.add(StandardMaterial { base_color: Color::srgb(0.95, 1.0, 0.25), unlit: true, ..default() });
    let ink = materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, cull_mode: Some(bevy::render::render_resource::Face::Front), ..default() });
    let b = commands
        .spawn((BallView, Transform::default(), Visibility::default()))
        .with_child((Mesh3d(meshes.add(Sphere::new(r).mesh().ico(4).unwrap())), MeshMaterial3d(core)))
        .with_child((Mesh3d(meshes.add(Sphere::new(r * 1.22).mesh().ico(4).unwrap())), MeshMaterial3d(ink)))
        .id();
    commands.entity(root).add_child(b);
    let mark = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.05, 0.05), unlit: true, ..default() });
    let m = commands
        .spawn((LandingMark, Mesh3d(meshes.add(Cylinder::new(0.14, 0.004))), MeshMaterial3d(mark), Transform::default(), Visibility::Hidden))
        .id();
    commands.entity(root).add_child(m);
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(4.0, 12.0, -6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((ScoreText, Text::new(""), Node { position_type: PositionType::Absolute, top: Val::Px(12.0), left: Val::Px(12.0), ..default() }));
    commands
        .spawn(Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), top: Val::Percent(30.0), justify_content: JustifyContent::Center, ..default() })
        .with_child((PopupText, Text::new(""), TextFont { font_size: FontSize::Px(64.0), ..default() }, TextColor(Color::WHITE), TextShadow::default()));
}

fn reset_positions(g: &mut Game) {
    let sx = if g.score.side == 0 { -1.0 } else { 1.0 }; // deuce / ad court
    for (i, side) in [Side::Near, Side::Far].into_iter().enumerate() {
        let x = if side == g.server { sx * 1.0 * side.z() } else { -sx * 2.5 * side.z() };
        let p = &mut g.players[i];
        *p = Player { pos: [x, 0.0, side.z() * (HALF_LENGTH + 0.3)], facing: if side == Side::Near { 0.0 } else { std::f32::consts::PI }, ..Player::default() };
        p.prev = p.pos;
    }
    let s = g.players[g.server as usize].pos;
    g.flight = Flight::new(Ball { pos: [s[0] + 0.3, -1.0, s[2]], vel: [0.0; 3], spin: 0.0 }, [[0.0; 4]; 4], [[0.0; 4]; 4]);
    g.prev_ball = g.flight.ball.pos;
}

fn rand(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / (1 << 24) as f32
}

fn read_input(keys: Res<ButtonInput<KeyCode>>, pads: Query<&Gamepad>, mut pad: ResMut<Pad>, mut cam: ResMut<CamMode>, mut cs: ResMut<CamState>, time: Res<Time>) {
    let mut stick = Vec2::ZERO;
    for (k, d) in [(KeyCode::KeyW, Vec2::Y), (KeyCode::KeyS, -Vec2::Y), (KeyCode::KeyA, -Vec2::X), (KeyCode::KeyD, Vec2::X)] {
        if keys.pressed(k) {
            stick += d;
        }
    }
    let mut sprint = keys.pressed(KeyCode::ShiftLeft);
    let mut shot = [(KeyCode::KeyJ, 0), (KeyCode::KeyK, 1), (KeyCode::KeyI, 2), (KeyCode::KeyL, 3), (KeyCode::KeyU, 4)]
        .into_iter()
        .find(|(k, _)| keys.just_pressed(*k))
        .map(|(_, kind)| kind);
    let mut serve = keys.just_pressed(KeyCode::Space);
    let mut cycle = keys.just_pressed(KeyCode::KeyC);
    let mut turn = 0.0;
    if keys.pressed(KeyCode::ArrowLeft) {
        turn -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        turn += 1.0;
    }
    for g in &pads {
        stick += g.left_stick();
        sprint |= g.pressed(GamepadButton::LeftTrigger);
        let buttons = [(GamepadButton::South, 0), (GamepadButton::East, 1), (GamepadButton::West, 2), (GamepadButton::North, 3), (GamepadButton::RightTrigger, 4)];
        shot = shot.or(buttons.into_iter().find(|(b, _)| g.just_pressed(*b)).map(|(_, k)| k));
        serve |= g.just_pressed(GamepadButton::Start);
        cycle |= g.just_pressed(GamepadButton::Select);
        turn += g.right_stick().x;
    }
    pad.stick = stick.clamp_length_max(1.0);
    pad.sprint = sprint;
    if shot.is_some() {
        pad.shot = shot;
    }
    pad.serve |= serve;
    if cycle {
        *cam = match *cam {
            CamMode::Follow => CamMode::Broadcast,
            CamMode::Broadcast => CamMode::Free,
            CamMode::Free => CamMode::Follow,
        };
    }
    cs.turn = (cs.turn + turn * time.delta_secs() * 1.5).clamp(-1.2, 1.2);
}

/// Launch a stroke of `kind` by `who` from the ball's position toward `target`.
fn strike(g: &mut Game, who: Side, kind: i32, target: V3) {
    let at = g.flight.ball.pos;
    let l = lookup(&g.tables[kind as usize], &Bounds::stroke(kind, at[2]), at, target);
    let vel = launch(at, target, l.elevation, l.speed);
    let dir = Vec3::new(vel[0], 0.0, vel[2]).normalize_or(Vec3::Z);
    let side = Vec3::Y.cross(dir).normalize();
    let frame = [side.to_array(), [0.0, 1.0, 0.0], side.cross(Vec3::Y).normalize().to_array()];
    g.shot = Shot { class: 1, kind, curve_frames: l.frames + 1, ..Shot::default() };
    g.shots += 1;
    g.rally.on_hit(g.shots, who as i32, g.score.server, g.score.receiver, g.flight.contacts);
    g.flight = Flight::new(Ball { pos: at, vel, spin: KIND_SPIN[kind as usize] }, rows4(frame), rows4(frame));
    g.flight.lines = Some(Lines { shots: g.shots, doubles: RULES.players > 2, side: g.score.side, hitter_far: who == Side::Near, margin: g.line_margin });
    g.prev_ball = at;
    g.last_hitter = who;
    g.since_hit = 0;
    g.phase = Phase::Rally;
}

/// Placeholder serve: the toss ends overhead and the ball is driven into the diagonal service box.
fn serve(g: &mut Game, who: Side, kind: i32) {
    let p = g.players[who as usize];
    g.flight.ball.pos = [p.pos[0] + 0.25 * who.z(), -2.6, p.pos[2] - 0.2 * who.z()];
    let aim = p.aim.x * 1.2 * who.z();
    let target = [-p.pos[0].signum() * 2.0 - aim, 0.0, -who.z() * 5.2];
    strike(g, who, if kind == 3 { 0 } else { kind }, target);
}

/// Analog aim: where in the opponent's court a stick position sends the ball. Sideways spans the singles
/// width, toward the net hits deep, pulling back hits short; centred is a deep middle ball.
fn aim_target(stick: Vec2, hitter: Side) -> V3 {
    let x = stick.x.clamp(-1.0, 1.0) * 3.4;
    let depth = 8.0 + stick.y.clamp(-1.0, 1.0) * 2.8; // 5.2 .. 10.8 m past the net
    // the near player looks toward -z: stick right is -x in game space
    [-x * hitter.z(), 0.0, -hitter.z() * depth]
}

/// Right-hand unit vector of a figure facing `yaw`.
fn right_of(yaw: f32) -> Vec2 {
    Vec2::new(yaw.cos(), -yaw.sin())
}

/// Ball's sideways offset from the player in the player's frame (positive = forehand side).
fn ball_side_at(p: &Player, ball: V3) -> f32 {
    Vec2::new(ball[0] - p.pos[0], ball[2] - p.pos[2]).dot(right_of(p.facing))
}

/// The original's per-player timing grades, indexed by how many frames after the press the ball reaches the
/// contact point: 0 = can't hit, 1 = sweet spot (exactly 8 frames), 2 = early/late, 4 = very early/late.
/// (Character 0's table from a bot-match save; ends vary slightly per character.)
const TIMING_GRADES: [u8; 20] = [0, 0, 4, 4, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 4, 4];
const SWEET_FRAME: i32 = 8;
/// Ground strokes look this many frames ahead for a contact point.
const STROKE_LOOKAHEAD: usize = 28;
/// Contact point sits this far in front of the player; ball height window for strokes is 0..1.25×1.3 m.
const STROKE_AHEAD: f32 = 0.484;
const STROKE_TOP: f32 = 1.25 * 1.3;
/// Reach to that contact point (character 0: 1.1 m).
const STROKE_REACH: f32 = 1.1;

/// What a pressed swing locked onto: frames until contact, the predicted ball there, the grade and offset.
#[derive(Clone, Copy)]
struct Contact {
    frames: u32,
    ball: V3,
    grade: u8,
    offset: i32,
}

/// The original's ground-stroke contact search: scan the predicted path for frames where the ball is on our
/// half (≥ 0.5 m from the net), the timing table allows a hit, it is 0–1.625 m high and within reach of the
/// point in front of us; take the one closest in depth to that point (earliest on ties).
fn find_contact(g: &Game, i: usize, side: Side) -> Option<Contact> {
    if g.phase != Phase::Rally || g.last_hitter == side {
        return None;
    }
    let p = g.players[i];
    let ahead = Vec2::new(p.pos[0], p.pos[2] - side.z() * STROKE_AHEAD);
    let mut f = g.flight;
    let mut best: Option<(Contact, f32)> = None;
    for k in 0..STROKE_LOOKAHEAD {
        if k > 0 {
            f.step(&g.shot, &COURTS[0]);
        }
        if f.bounces > 1 {
            break;
        }
        let b = f.ball.pos;
        let grade = TIMING_GRADES.get(k).copied().unwrap_or(0);
        let height = -b[1];
        // ponytail: the player may also run toward the ball while the swing winds up; the exact allowance is
        // in a branch not decoded yet, so reach grows with the frames available at running speed
        let reach = STROKE_REACH + k as f32 * RUN;
        let d = Vec2::new(b[0], b[2]) - ahead;
        if b[2] * side.z() >= 0.5 && grade != 0 && (0.0..=STROKE_TOP).contains(&height) && d.length() <= reach {
            let depth = (b[2] - ahead.y).abs();
            if best.is_none_or(|(_, bd)| depth < bd) {
                best = Some((Contact { frames: k as u32, ball: b, grade, offset: k as i32 - SWEET_FRAME }, depth));
            }
        }
    }
    best.map(|(c, _)| c)
}

/// Pop-up wording for a timing result (offset < 0: the ball arrived sooner than the sweet frame, you were late).
fn timing_word(c: &Contact) -> &'static str {
    match c.offset {
        0 => "SWEET SPOT",
        o if o < 0 => "SLOW",
        _ => "QUICK",
    }
}

/// Run toward a desired velocity with acceleration limits.
fn locomote(p: &mut Player, want: Vec2, side: Side) {
    let rate = if want.length() > p.vel.length() { ACCEL } else { BRAKE };
    p.vel += (want - p.vel).clamp_length_max(rate);
    p.prev = p.pos;
    p.pos[0] = (p.pos[0] + p.vel.x).clamp(-8.0, 8.0);
    p.pos[2] = p.pos[2] + p.vel.y;
    p.pos[2] = if side == Side::Near { p.pos[2].clamp(0.4, 17.0) } else { p.pos[2].clamp(-17.0, -0.4) };
    p.stride += p.vel.length() * 9.0;
    // face the opposite court, leaning toward a sideways run
    let base = if side == Side::Near { 0.0 } else { std::f32::consts::PI };
    let lean = (p.vel.x * side.z()).clamp(-0.1, 0.1) * 4.0;
    p.facing += (base - lean - p.facing) * 0.2;
}

/// Where the player should stand for a locked contact: ball on the racket side, contact point in front.
fn contact_stance(p: &Player, c: &Contact, side: Side) -> Vec2 {
    let hand = if ball_side_at(p, c.ball) >= 0.0 { 1.0 } else { -1.0 };
    Vec2::new(c.ball[0], c.ball[2] + side.z() * STROKE_AHEAD) - right_of(p.facing) * 0.6 * hand
}

/// One frame of a player's stroke: a pending press keeps searching for a contact (pressing early grades
/// QUICK, late SLOW), a locked contact steers the player into position and winds up so the racket meets the
/// ball exactly on the chosen frame, then the follow-through plays out.
fn advance_stroke(g: &mut Game, i: usize, side: Side, stick: Vec2, aim: impl Fn(&mut Game) -> V3) -> Option<Contact> {
    let mut struck = None;
    if let Some(left) = g.players[i].pending {
        if let Some(c) = find_contact(g, i, side) {
            let p = &mut g.players[i];
            p.pending = None;
            p.contact = Some(c);
            p.wind = c.frames.max(1);
            p.backhand = ball_side_at(p, c.ball) < 0.0;
            p.swing = Some(0);
            p.swung = false;
        } else {
            g.players[i].pending = left.checked_sub(1);
        }
    }
    if let Some(c) = g.players[i].contact {
        let p = &mut g.players[i];
        let want = (contact_stance(p, &c, side) - Vec2::new(p.pos[0], p.pos[2])) / (c.frames.max(1) as f32);
        locomote(p, want.clamp_length_max(RUN * 1.4) + stick * 0.0, side);
        if c.frames == 0 {
            let kind = g.players[i].kind;
            let target = aim(g);
            strike(g, side, kind, target);
            debug!("{side:?} stroke kind {kind}: {} (offset {}, grade {})", timing_word(&c), c.offset, c.grade);
            g.players[i].contact = None;
            g.players[i].swung = true;
            struck = Some(c);
        } else {
            g.players[i].contact = Some(Contact { frames: c.frames - 1, ..c });
        }
    }
    // animation clock: contact at 45% of the swing, follow-through over the remaining frames
    let p = &mut g.players[i];
    if let Some(f) = p.swing {
        p.swing = (f + 1 < p.wind + 16).then_some(f + 1);
    }
    struck
}

fn swing_progress(p: &Player) -> Option<f32> {
    let f = p.swing? as f32;
    let w = p.wind.max(1) as f32;
    Some(if f <= w { 0.45 * f / w } else { 0.45 + 0.55 * ((f - w) / 16.0).min(1.0) })
}

/// Serve: toss, then contact at SERVE_CONTACT.
fn advance_serve(g: &mut Game, i: usize, side: Side) {
    let Some(f) = g.players[i].swing else { return };
    if g.phase == Phase::Serve {
        let p = g.players[i].pos;
        let t = f as f32 / SERVE_CONTACT as f32;
        g.flight.ball.pos = [p[0] + 0.25 * side.z(), -1.3 - 1.3 * (t * std::f32::consts::FRAC_PI_2).sin(), p[2] - 0.2 * side.z()];
        g.prev_ball = g.flight.ball.pos;
        if f == SERVE_CONTACT {
            let kind = g.players[i].kind;
            serve(g, side, kind);
        }
    }
    let p = &mut g.players[i];
    p.swing = (f + 1 < SWING_FRAMES).then_some(f + 1);
    if p.swing.is_none() {
        p.serving = false;
    }
}

fn start_serve(g: &mut Game, i: usize, kind: i32) {
    let p = &mut g.players[i];
    if p.swing.is_none() {
        *p = Player { swing: Some(0), wind: SERVE_CONTACT, serving: true, kind, ..*p };
    }
}

/// A shot button press: remembered for a while and checked every frame until a contact is found.
fn press(g: &mut Game, i: usize, kind: i32) {
    let p = &mut g.players[i];
    if p.contact.is_none() && p.swing.is_none() {
        p.pending = Some(STROKE_LOOKAHEAD as u32);
        p.kind = kind;
    }
}

fn human(mut g: ResMut<Game>, mut pad: ResMut<Pad>) {
    // HST_AUTOPLAY: the bot plays your side too (unattended soak tests)
    if std::env::var_os("HST_AUTOPLAY").is_some() {
        bot(&mut g, Side::Near);
        return;
    }
    let shot = pad.shot.take();
    let serve_press = std::mem::take(&mut pad.serve);
    g.players[0].aim = pad.stick;
    if g.phase == Phase::Serve && g.server == Side::Near {
        g.message = "Your serve: J/Space (A/Start)".into();
        if shot.is_some() || serve_press {
            start_serve(&mut g, 0, shot.unwrap_or(0));
        }
        advance_serve(&mut g, 0, Side::Near);
        return;
    }
    if g.players[0].serving {
        advance_serve(&mut g, 0, Side::Near);
    }
    if let Some(kind) = shot {
        press(&mut g, 0, kind);
    }
    if g.players[0].contact.is_none() {
        // camera sits behind the near baseline looking toward -z: stick right = -x, stick up = toward the net
        let mut want = Vec2::new(-pad.stick.x, -pad.stick.y) * RUN * if pad.sprint { SPRINT } else { 1.0 };
        if g.players[0].swing.is_some() {
            want *= 0.25;
        }
        locomote(&mut g.players[0], want, Side::Near);
    }
    // the stick at the moment of contact aims the shot
    let aim = pad.stick;
    if let Some(c) = advance_stroke(&mut g, 0, Side::Near, pad.stick, move |_| aim_target(aim, Side::Near)) {
        g.popup = Some((timing_word(&c), 50, c.grade));
    }
}

/// Where the ball will be hittable on `side`: step a copy of the flight until it is waist-high after its bounce.
fn intercept(g: &Game, side: Side) -> Option<(V3, u32)> {
    let mut f = g.flight;
    for n in 0..240 {
        f.step(&g.shot, &COURTS[0]);
        let b = f.ball;
        if b.pos[2] * side.z() > 0.0 && f.bounces >= 1 && b.vel[1] > 0.0 && (-1.3..-0.5).contains(&b.pos[1]) {
            return Some((b.pos, n));
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

/// Timing error of the stand-in AI in frames (AIParam's "normal shot deviation" for character 0 is 9).
const BOT_TIMING_ERROR: f32 = 9.0;

/// Stand-in opponent for `side`: serves, runs to the predicted interception, and presses the shot button
/// around the sweet frame with a random timing error, through the same contact search as the human.
fn bot(g: &mut Game, side: Side) {
    let i = side as usize;
    if g.phase == Phase::Serve && g.server == side {
        if side == Side::Far {
            g.message = "Opponent serving".into();
        }
        if g.players[i].swing.is_none() {
            let kind = if rand(&mut g.rng) < 0.5 { 0 } else { 2 };
            start_serve(g, i, kind);
        }
        advance_serve(g, i, side);
        return;
    }
    if g.players[i].serving {
        advance_serve(g, i, side);
    }
    let busy = g.players[i].contact.is_some() || g.players[i].pending.is_some() || g.players[i].swing.is_some();
    if !busy {
        let plan = match g.phase {
            Phase::Rally if g.last_hitter != side => intercept(g, side),
            _ => None,
        };
        let goal = plan.map_or([0.0, 0.0, side.z() * (HALF_LENGTH + 0.5)], |(p, _)| {
            let r = right_of(g.players[i].facing);
            [p[0] - r.x * 0.6, 0.0, p[2] - r.y * 0.6 + STROKE_AHEAD * side.z()]
        });
        let d = Vec2::new(goal[0] - g.players[i].pos[0], goal[2] - g.players[i].pos[2]);
        let want = if d.length() < 0.05 { Vec2::ZERO } else { d.clamp_length_max(RUN) };
        locomote(&mut g.players[i], want, side);
        // press when the ball is due about SWEET_FRAME frames out, give or take the timing error
        if let Some((_, frames)) = plan {
            let due = SWEET_FRAME as f32 + (rand(&mut g.rng) * 2.0 - 1.0) * BOT_TIMING_ERROR * 0.1;
            if frames as f32 <= due {
                let r = rand(&mut g.rng);
                let kind = if r < 0.6 { 0 } else if r < 0.8 { 1 } else if r < 0.9 { 2 } else { 3 };
                press(g, i, kind);
            }
        }
    }
    advance_stroke(g, i, side, Vec2::ZERO, move |g| {
        let stick = Vec2::new(rand(&mut g.rng) * 1.8 - 0.9, rand(&mut g.rng) * 1.6 - 0.8);
        aim_target(stick, side)
    });
}

fn simulate(mut g: ResMut<Game>, args: Res<Args>) {
    match g.phase {
        Phase::Serve => return,
        Phase::Over(0) => {
            g.score.second_serve = g.rally.faults == 1;
            g.score.let_ = g.rally.let_;
            g.score.change_ends(); // ponytail: ends are tracked, not yet swapped on court (P0b serve flow)
            g.score.next_point(&RULES);
            g.rally.next_point();
            g.rally.new_point();
            g.shots = 0;
            g.server = if g.score.server % 2 == 0 { Side::Near } else { Side::Far };
            g.phase = Phase::Serve;
            reset_positions(&mut g);
            return;
        }
        Phase::Over(n) => g.phase = Phase::Over(n - 1),
        Phase::Rally => {}
    }
    g.prev_ball = g.flight.ball.pos;
    let shot = g.shot;
    g.flight.step(&shot, &COURTS[args.court.min(COURTS.len() - 1)]);
    g.since_hit += 1;
    if g.phase != Phase::Rally {
        return;
    }
    let f = &g.flight;
    // ponytail: no ball body hits and no rest detection on steep surfaces yet; the rally timeout counts as at rest
    let view = BallState { call: f.call, contacts: f.contacts, stopped: f.frame > 1800, pos: f.ball.pos };
    let (shots, hitter, server) = (g.shots, g.last_hitter as i32, g.score.server);
    if !g.rally.check(&view, shots, hitter, server, None, true) {
        return;
    }
    let verdict = g.rally.judge(None);
    let why = CALLS[verdict.call as usize];
    let Some(team) = verdict.winner else {
        g.message = if verdict.call == 2 { "Fault - second serve".into() } else { format!("{why} - serve again") };
        info!("no point: {why} after {} frames", g.since_hit);
        g.phase = Phase::Over(90);
        return;
    };
    // a scored point ends the serve's faults (the scoreboard clears them as it shows the score)
    g.rally.faults = 0;
    let side = if team == 0 { Side::Near } else { Side::Far };
    let event = g.score.point(&RULES, side as usize);
    match event {
        Some(Event::Game) => g.score.new_game(),
        Some(Event::Set) => g.score.new_set(),
        _ => {}
    }
    g.score.note_tiebreak_start();
    let who = if side == Side::Near { "you" } else { "opponent" };
    g.message = match event {
        Some(Event::Set) if g.score.match_over => format!("{why} - match to {who}"),
        Some(Event::Set) => format!("{why} - set to {who}"),
        Some(Event::Game) => format!("{why} - game to {who}"),
        _ => format!("{why} - point to {who}"),
    };
    info!("point: {} ({why}) after {} frames, {event:?} {:?}", if side == Side::Near { "near" } else { "far" }, g.since_hit, g.score);
    if g.score.match_over {
        g.score = Score::new();
    }
    g.phase = Phase::Over(90);
}

/// Camera modes drive the orbit rig: follow sits behind you, broadcast is the classic high baseline view,
/// free leaves the mouse/right stick in charge.
fn camera(g: Res<Game>, mode: Res<CamMode>, mut cs: ResMut<CamState>, time: Res<Time>, mut q: Query<&mut Orbit>) {
    let Ok(mut o) = q.single_mut() else { return };
    let me = g.players[0].pos;
    // game space → Bevy: (x, -y, -z)
    let (focus, radius, pitch, yaw) = match *mode {
        CamMode::Follow => (Vec3::new(me[0] * 0.6, 1.2, -me[2] + 6.0), 10.5, -0.3, std::f32::consts::PI + cs.turn),
        CamMode::Broadcast => (Vec3::new(0.0, 0.0, 2.0), 24.0, -0.38, std::f32::consts::PI),
        CamMode::Free => {
            o.yaw += cs.turn * time.delta_secs();
            return;
        }
    };
    let k = 1.0 - (-6.0 * time.delta_secs()).exp();
    cs.focus = if cs.focus == Vec3::ZERO { focus } else { cs.focus.lerp(focus, k) };
    o.focus = cs.focus;
    o.radius += (radius - o.radius) * k;
    o.pitch += (pitch - o.pitch) * k;
    o.yaw += (yaw - o.yaw) * k;
}

/// First bounce point of the ball in flight, found by stepping a copy with the real physics.
fn landing(g: &Game) -> Option<V3> {
    if g.phase != Phase::Rally || g.flight.bounces > 0 {
        return None;
    }
    let mut f = g.flight;
    for _ in 0..240 {
        f.step(&g.shot, &COURTS[0]);
        if f.special_contacts > 0 {
            return None;
        }
        if f.bounces > 0 {
            return Some(f.ball.pos);
        }
    }
    None
}

fn mark_landing(g: Res<Game>, mut q: Query<(&mut Transform, &mut Visibility), With<LandingMark>>) {
    let Ok((mut t, mut v)) = q.single_mut() else { return };
    match landing(&g) {
        Some(p) => {
            t.translation = Vec3::new(p[0], -0.01, p[2]); // just above the court surface (Y-down)
            *v = Visibility::Inherited;
        }
        None => *v = Visibility::Hidden,
    }
}

fn draw(g: Res<Game>, time: Res<Time<Fixed>>, mut figures: Query<(&Figure, &mut Transform, &mut figure::Pose)>, mut ball: Query<&mut Transform, (With<BallView>, Without<Figure>)>) {
    let a = time.overstep_fraction();
    for (f, mut t, mut pose) in &mut figures {
        let p = g.players[f.0];
        t.translation = Vec3::from(p.prev).lerp(Vec3::from(p.pos), a);
        t.rotation = Quat::from_rotation_y(p.facing);
        pose.speed = p.vel.length();
        pose.stride = p.stride;
        pose.swing = if p.serving { p.swing.map(|s| s as f32 / SWING_FRAMES as f32) } else { swing_progress(&p) };
        pose.backhand = p.backhand;
        pose.serving = p.serving;
    }
    for mut t in &mut ball {
        t.translation = Vec3::from(g.prev_ball).lerp(Vec3::from(g.flight.ball.pos), a);
    }
}

fn popup(mut g: ResMut<Game>, mut q: Query<(&mut Text, &mut TextColor), With<PopupText>>) {
    if let Some((_, n, _)) = &mut g.popup {
        *n = n.saturating_sub(1);
    }
    let shown = g.popup.filter(|(_, n, _)| *n > 0);
    for (mut t, mut c) in &mut q {
        match shown {
            Some((word, n, _)) => {
                t.0 = word.to_string();
                let a = (n as f32 / 15.0).min(1.0);
                c.0 = if word == "SWEET SPOT" { Color::srgba(1.0, 0.82, 0.15, a) } else { Color::srgba(1.0, 1.0, 1.0, a) };
            }
            None => t.0.clear(),
        }
    }
}

fn hud(g: Res<Game>, mode: Res<CamMode>, mut q: Query<&mut Text, With<ScoreText>>) {
    for mut t in &mut q {
        t.0 = format!(
            "You {}  -  {} Opponent\n{}\ncamera: {} (C / Select)\nmove WASD/stick · sprint Shift/LB · J/A topspin · K/B slice · I/X flat · L/Y lob · U/RB drive",
            score_line(&g.score, 0),
            score_line(&g.score, 1),
            g.message,
            match *mode {
                CamMode::Follow => "follow",
                CamMode::Broadcast => "broadcast",
                CamMode::Free => "free",
            }
        );
    }
}

/// Games and points for one team, tennis style (points 0/15/30/40/Ad; tiebreak points as numbers).
fn score_line(s: &Score, team: usize) -> String {
    let p = s.points[team];
    let pts = if s.tiebreak { p.to_string() } else { ["0", "15", "30", "40", "Ad"].get(p as usize).unwrap_or(&"Ad").to_string() };
    format!("{} | {pts}", s.games[team])
}
