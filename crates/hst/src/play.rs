//! Playable test mode (`--play`): doubles (four players, `--singles` for two) on the ported ball physics, rules,
//! serve placement, contact search and the game's own shot tables. Players 1 and 2 (one per team) are humans on
//! controllers 1 and 2 (the keyboard also drives player 1); every other slot is a stand-in AI. Players are
//! original stand-in athletes (`figure.rs`); movement tuning, AI and the serve motion are placeholders until those
//! systems are ported (see TODO.md).
//!
//! The view is the original's match camera, behind the −z baseline; the stick moves and aims screen-relative.
//! A red dot marks where the ball will bounce.
//!
//! Keyboard (player 1): WASD move (aim while swinging), Shift sprint, J topspin, K slice, I flat, L lob, U drive,
//! J/Space serve, C camera (original / free), arrow keys turn the free camera.
//! Gamepad: left stick or d-pad move/aim, LB sprint, A topspin, B slice, X flat, Y lob, RB drive, A/Start serve,
//! Select camera, right stick turns the free camera.

use bevy::prelude::*;
use hst_data::{exe::ScoreboardTiming, iso::Iso, xb::Archive};
use hst_sim::ball::{Ball, COURTS, Flight, Material, Shot, V3, rows4};
use hst_sim::court;
use hst_sim::flow::{CHANGE_ENDS, Next, PostPoint, serve_placement};
use hst_sim::judge::{BallState, Lines, Rally};
use hst_sim::mesh::World;
use hst_sim::score::{Event, Rules, Score};
use hst_sim::shot::{Bounds, Table, launch, lookup};
use hst_sim::swing::{self, PathPoint, Reach};

use crate::{Args, GameSpace, Orbit, figure};

/// The original's default exhibition: one set to 4 games, deuce on.
const SINGLES: Rules = Rules { sets: 1, games: 4, no_deuce: false, one_point_games: false, players: 2 };
const DOUBLES: Rules = Rules { players: 4, ..SINGLES };
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
/// The contact is graded against this frame after the press (the timing table's sweet spot).
const SWEET_FRAME: i32 = 8;
/// A press stays live this many frames looking for a contact.
const PRESS_FRAMES: u32 = 28;
/// The original's serve camera (camera-to-world rows, game space) and field of view: the game stores the
/// horizontal half-angle of its 4:3 picture (vertical = horizontal × 0.75, measured on the court lines).
const SERVE_CAM_EYE: [f32; 3] = [0.0, -11.89, -39.238];
const SERVE_CAM_FORWARD: [f32; 3] = [0.0, 0.292, 0.956];
const SERVE_CAM_FOV: f32 = 0.1745;

#[derive(Clone, Copy, Default)]
struct Player {
    pos: V3,
    prev: V3,
    vel: Vec2,
    /// +1: this player's end is the −z half and it faces +z; −1 the other end.
    end: f32,
    /// Yaw of the figure; 0 faces −z.
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
    /// Distance off centre at this player's last toss: where they serve from next time.
    stance: f32,
    /// Where the player was put for this point (the stand-in AI's home spot).
    home: V3,
    /// The ball where the racket meets it (game space), while a stroke is locked.
    hit_at: Option<V3>,
}

/// A pressed swing locked onto the ball: frames until contact, the game's contact search result, the grade
/// and the body's step into the shot.
#[derive(Clone, Copy)]
struct Contact {
    frames: u32,
    swing: swing::Swing,
    grade: u8,
    offset: i32,
    step: f32,
    step_frames: u32,
}

#[derive(PartialEq)]
enum Phase {
    Serve,
    Rally,
    /// A scored point: the scoreboard's turn (`Game::post`).
    Post,
    /// No point (fault or let): ticks until the serve is taken again.
    Over(u32),
    ChangeEnds(u32),
}

#[derive(Resource)]
struct Game {
    rules: Rules,
    tables: Vec<Table>,
    reach: Reach,
    flight: Flight,
    shot: Shot,
    prev_ball: V3,
    players: Vec<Player>,
    phase: Phase,
    /// Player who hit last (−1: none yet).
    last_hitter: i32,
    since_hit: u32,
    score: Score,
    rally: Rally,
    /// Shots this rally (1 = the serve).
    shots: i32,
    /// Line tolerance from the game program.
    line_margin: f32,
    post: Option<PostPoint>,
    board: ScoreboardTiming,
    /// The stage's collision world and material table (`--stage`); without one the ball meets a flat court and net.
    world: Option<(World, Vec<Material>)>,
    court: usize,
    message: String,
    rng: u32,
    /// Timing pop-up: text, frames left, grade.
    popup: Option<(&'static str, u32, u8)>,
}

/// One controller slot's state, gathered every display frame and consumed by the 60 Hz simulation.
#[derive(Clone, Copy, Default)]
struct SlotPad {
    stick: Vec2,
    sprint: bool,
    /// Latched shot press (kind), so a press between fixed steps is never lost.
    shot: Option<i32>,
    serve: bool,
}

/// Controller slots 1 and 2 (keyboard and the first gamepad drive slot 1). Slot `i` controls player `i`.
#[derive(Resource, Default)]
struct Pads {
    slots: [SlotPad; 2],
    /// Gamepads connected.
    connected: usize,
}

impl Pads {
    /// Which slot controls player `i`, if any (slot 2 only while a second gamepad is connected).
    fn slot_of(&self, i: usize) -> Option<usize> {
        if std::env::var_os("HST_AUTOPLAY").is_some() {
            return None;
        }
        match i {
            0 => Some(0),
            1 if self.connected >= 2 => Some(1),
            _ => None,
        }
    }
}

#[derive(Resource, Default, Clone, Copy, PartialEq)]
enum CamMode {
    #[default]
    Original,
    Free,
}

#[derive(Resource, Default)]
struct CamState {
    turn: f32,
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
        .init_resource::<Pads>()
        .init_resource::<CamMode>()
        .init_resource::<CamState>()
        .add_systems(PostStartup, setup) // after the court's game-space root exists
        .add_systems(Update, (read_input, camera, draw, mark_landing, figure::animate, hud).chain())
        .add_systems(FixedUpdate, (control, simulate, popup).chain());
}

/// Character 0's stroke tables (kinds 0..4) straight from the disc.
fn tables(iso: &mut Iso) -> Vec<Table> {
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

/// Character 0's reach for the contact search: TParam.csv from the disc (reach base, reach, ideal stroke and
/// volley heights, smash window, handedness).
fn reach(iso: &mut Iso) -> Reach {
    let data = iso.read("PCDATA/PCDATA.XB").expect("character archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).expect("TParam.csv");
    let csv = arc.read(e).expect("TParam.csv bytes");
    // header cells hold quoted line breaks; character rows are plain and start with their number
    let row = csv.split(|&b| b == b'\n').find(|l| l.starts_with(b"0,")).expect("character 0 row");
    let cols: Vec<&[u8]> = row.split(|&b| b == b',').collect();
    let num = |i: usize| std::str::from_utf8(cols[i]).ok().and_then(|s| s.trim().parse::<f32>().ok()).expect("TParam number");
    let pair = |i: usize, k: usize| -> f32 {
        let s = std::str::from_utf8(cols[i]).expect("TParam cell");
        s.split('/').nth(k).and_then(|v| v.trim().parse().ok()).expect("TParam slash cell")
    };
    let right = cols[6].starts_with(&[0x89, 0x45]); // 右 (right) in Shift-JIS
    Reach {
        base: num(55),
        reach: (num(56) + num(57)) / 100.0,
        stroke_height: pair(63, 0) / 100.0,
        volley_height: pair(63, 1) / 100.0,
        smash_top: pair(64, 0) / 100.0,
        smash_bottom: pair(64, 2) / 100.0,
        // ponytail: measured on character 0's swing animations at their contact frame (the game derives them
        // from the skeleton at load); read them from the motion data with P8
        ahead: 0.484,
        smash_ahead: 0.3578,
        body_low: 0.5535,
        body_high: 0.5828,
        // ponytail: character 0's timing grades (player +0x1510, 28-frame horizon); their source is P3
        grades: vec![0, 0, 4, 4, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
        hand: if right { 1.0 } else { -1.0 },
    }
}

/// Line margin, scoreboard timing, and the stage's collision world with the material table.
fn disc(iso: &mut Iso, stage: Option<u32>) -> (f32, ScoreboardTiming, Option<(World, Vec<Material>)>) {
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
    let game = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
    (game.line_margin(), game.scoreboard_timing(), stage.map(|n| (court::world(iso, n), court::materials(&game))))
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    root: Query<Entity, With<GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let (line_margin, board, world) = disc(&mut iso, args.stage);
    let rules = if args.singles { SINGLES } else { DOUBLES };
    let mut game = Game {
        rules,
        tables: tables(&mut iso),
        reach: reach(&mut iso),
        flight: Flight::new(Ball { pos: [0.0; 3], vel: [0.0; 3], spin: 0.0 }, [[0.0; 4]; 4], [[0.0; 4]; 4]),
        shot: Shot::default(),
        prev_ball: [0.0; 3],
        players: vec![Player { stance: 3.0, ..Player::default() }; rules.players as usize],
        phase: Phase::Serve,
        last_hitter: -1,
        since_hit: 0,
        score: Score::new(),
        rally: Rally::default(),
        shots: 0,
        line_margin,
        post: None,
        board,
        world,
        court: args.court.min(COURTS.len() - 1),
        message: String::new(),
        rng: 0x2468_ace1,
        popup: None,
    };
    reset_positions(&mut game);
    let n = game.players.len();
    commands.insert_resource(game);

    let Ok(root) = root.single() else { return };
    let shirts = [Color::srgb(0.15, 0.4, 0.95), Color::srgb(0.9, 0.3, 0.15), Color::srgb(0.2, 0.75, 0.9), Color::srgb(0.95, 0.65, 0.1)];
    for (i, shirt) in shirts.into_iter().enumerate().take(n) {
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

/// Put every player where the original does for the next serve (`serve_placement`) and the ball in the server's hand.
fn reset_positions(g: &mut Game) {
    for i in 0..g.players.len() {
        let stance = g.players[i].stance;
        let at = serve_placement(i as i32, &g.score, g.rally.faults, stance, 0, g.score.swapped);
        let end = at.facing;
        g.players[i] = Player { pos: at.pos, prev: at.pos, end, facing: base_yaw(end), stance, home: at.pos, ..Player::default() };
    }
    let s = &g.players[g.score.server as usize];
    let ball = [s.pos[0] + 0.3, -1.0, s.pos[2]];
    g.flight = Flight::new(Ball { pos: ball, vel: [0.0; 3], spin: 0.0 }, [[0.0; 4]; 4], [[0.0; 4]; 4]);
    g.prev_ball = ball;
}

/// Figure yaw squarely facing the other end (yaw 0 faces −z).
fn base_yaw(end: f32) -> f32 {
    if end > 0.0 { std::f32::consts::PI } else { 0.0 }
}

fn rand(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / (1 << 24) as f32
}

fn read_input(keys: Res<ButtonInput<KeyCode>>, gamepads: Query<(Entity, &Gamepad)>, mut pads: ResMut<Pads>, mut cam: ResMut<CamMode>, mut cs: ResMut<CamState>, time: Res<Time>) {
    let mut now = [SlotPad::default(); 2];
    for (k, d) in [(KeyCode::KeyW, Vec2::Y), (KeyCode::KeyS, -Vec2::Y), (KeyCode::KeyA, -Vec2::X), (KeyCode::KeyD, Vec2::X)] {
        if keys.pressed(k) {
            now[0].stick += d;
        }
    }
    now[0].sprint = keys.pressed(KeyCode::ShiftLeft);
    now[0].shot = [(KeyCode::KeyJ, 0), (KeyCode::KeyK, 1), (KeyCode::KeyI, 2), (KeyCode::KeyL, 3), (KeyCode::KeyU, 4)]
        .into_iter()
        .find(|(k, _)| keys.just_pressed(*k))
        .map(|(_, kind)| kind);
    now[0].serve = keys.just_pressed(KeyCode::Space);
    let mut cycle = keys.just_pressed(KeyCode::KeyC);
    let mut turn = keys.pressed(KeyCode::ArrowRight) as i32 as f32 - keys.pressed(KeyCode::ArrowLeft) as i32 as f32;
    // controllers in connection order: the first is slot 1, the second slot 2
    let mut list: Vec<_> = gamepads.iter().collect();
    list.sort_by_key(|(e, _)| *e);
    for (slot, (_, g)) in list.iter().take(2).enumerate() {
        let s = &mut now[slot];
        s.stick += g.left_stick() + g.dpad();
        s.sprint |= g.pressed(GamepadButton::LeftTrigger);
        let buttons = [(GamepadButton::South, 0), (GamepadButton::East, 1), (GamepadButton::West, 2), (GamepadButton::North, 3), (GamepadButton::RightTrigger, 4)];
        s.shot = s.shot.or(buttons.into_iter().find(|(b, _)| g.just_pressed(*b)).map(|(_, k)| k));
        s.serve |= g.just_pressed(GamepadButton::Start);
        cycle |= g.just_pressed(GamepadButton::Select);
        turn += g.right_stick().x;
    }
    pads.connected = list.len();
    for (slot, n) in pads.slots.iter_mut().zip(now) {
        slot.stick = n.stick.clamp_length_max(1.0);
        slot.sprint = n.sprint;
        if n.shot.is_some() {
            slot.shot = n.shot;
        }
        slot.serve |= n.serve;
    }
    if cycle {
        *cam = if *cam == CamMode::Original { CamMode::Free } else { CamMode::Original };
    }
    cs.turn = turn * time.delta_secs() * 1.5;
}

/// Launch a stroke of `kind` by player `who` from the ball's position toward `target`.
fn strike(g: &mut Game, who: usize, kind: i32, target: V3) {
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
    let hitter_far = g.players[who].end < 0.0;
    g.flight.lines = Some(Lines { shots: g.shots, doubles: g.rules.players > 2, side: g.score.side, hitter_far, margin: g.line_margin });
    g.prev_ball = at;
    g.last_hitter = who as i32;
    g.since_hit = 0;
    g.phase = Phase::Rally;
}

/// Placeholder serve: the toss ends overhead and the ball is driven into the diagonal service box.
fn serve(g: &mut Game, who: usize, kind: i32) {
    let p = g.players[who];
    g.flight.ball.pos = [p.pos[0] - 0.25 * p.end, -2.6, p.pos[2] + 0.2 * p.end];
    let target = [-p.pos[0].signum() * 2.0 + p.aim.x * 1.2, 0.0, p.end * 5.2];
    strike(g, who, if kind == 3 { 0 } else { kind }, target);
}

/// Analog aim, screen-relative (the camera looks toward +z): sideways spans the court, stick up moves the target
/// toward +z (deeper for the near team, shorter for the far one); centred is a deep middle ball.
fn aim_target(g: &Game, stick: Vec2, end: f32) -> V3 {
    let width = if g.rules.players > 2 { 4.4 } else { 3.4 };
    let x = stick.x.clamp(-1.0, 1.0) * width;
    [x, 0.0, end * 8.0 + stick.y.clamp(-1.0, 1.0) * 2.8] // 5.2 .. 10.8 m past the net
}

/// The ball's predicted path for the contact search: this frame's ball, then one step per frame.
fn predicted_path(g: &Game, frames: usize) -> Vec<PathPoint> {
    let mut f = g.flight;
    let mut path = Vec::with_capacity(frames);
    for _ in 0..frames {
        path.push(PathPoint { pos: f.ball.pos, bounces: f.bounces });
        f.step(&g.shot, &COURTS[g.court]);
    }
    path
}

/// The original's contact search for player `i` (smash, volley, ground stroke), only while the other team's
/// ball is in play.
fn find_contact(g: &Game, i: usize) -> Option<Contact> {
    if g.phase != Phase::Rally || g.last_hitter < 0 || g.last_hitter & 1 == i as i32 & 1 {
        return None;
    }
    let p = &g.players[i];
    let path = predicted_path(g, g.reach.grades.len());
    let s = swing::search(&g.reach, &path, p.pos, p.end, p.kind)?;
    // ponytail: the body's sideways step comes from the original's arm IK on the character skeleton (0.04–0.29 m
    // toward the ball in recordings); a fixed 0.1 m stands in until the skeleton is read (P8)
    let step = if s.body { 0.0 } else { 0.1 * (s.ball[0] - p.pos[0]).signum() };
    Some(Contact {
        frames: s.frame as u32,
        swing: s,
        grade: g.reach.grades[s.frame],
        offset: s.frame as i32 - SWEET_FRAME,
        step,
        step_frames: swing::step_frames(s.frame) as u32,
    })
}

/// Pop-up wording for a timing result (offset < 0: the ball arrived sooner than the sweet frame, you were late).
fn timing_word(c: &Contact) -> &'static str {
    match c.offset {
        0 => "SWEET SPOT",
        o if o < 0 => "SLOW",
        _ => "QUICK",
    }
}

/// Run toward a desired velocity with acceleration limits, staying on this player's half.
fn locomote(p: &mut Player, want: Vec2) {
    let rate = if want.length() > p.vel.length() { ACCEL } else { BRAKE };
    p.vel += (want - p.vel).clamp_length_max(rate);
    p.prev = p.pos;
    p.pos[0] = (p.pos[0] + p.vel.x).clamp(-9.0, 9.0);
    p.pos[2] = p.pos[2] + p.vel.y;
    p.pos[2] = if p.end > 0.0 { p.pos[2].clamp(-17.0, -0.4) } else { p.pos[2].clamp(0.4, 17.0) };
    p.stride += p.vel.length() * 9.0;
    // face the other end, leaning toward a sideways run
    let lean = (p.vel.x * p.end).clamp(-0.1, 0.1) * 4.0;
    p.facing += (base_yaw(p.end) + lean - p.facing) * 0.2;
}

/// One frame of a player's stroke: a pending press keeps searching for a contact (pressing early grades
/// QUICK, late SLOW). Once locked the body squares up to the net and stands, taking its small step into the
/// shot over the last frames before contact, as the original; the racket meets the ball on the chosen frame.
fn advance_stroke(g: &mut Game, i: usize, aim: impl Fn(&mut Game) -> V3) -> Option<Contact> {
    let mut struck = None;
    if let Some(left) = g.players[i].pending {
        if let Some(c) = find_contact(g, i) {
            let p = &mut g.players[i];
            p.pending = None;
            p.contact = Some(c);
            p.hit_at = Some(c.swing.ball);
            p.wind = c.frames.max(1);
            p.backhand = !c.swing.forehand;
            p.swing = Some(0);
            p.swung = false;
            p.vel = Vec2::ZERO;
            p.facing = base_yaw(p.end);
        } else {
            g.players[i].pending = left.checked_sub(1);
        }
    }
    if let Some(c) = g.players[i].contact {
        let p = &mut g.players[i];
        p.prev = p.pos;
        if c.frames < c.step_frames {
            p.pos[0] += c.step / c.step_frames as f32;
        }
        if c.frames == 0 {
            let kind = g.players[i].kind;
            let target = aim(g);
            strike(g, i, kind, target);
            debug!("player {i} {:?} {} anim {:#x}: {} (offset {}, grade {})", c.swing.branch, if c.swing.forehand { "forehand" } else { "backhand" }, c.swing.anim, timing_word(&c), c.offset, c.grade);
            let p = &mut g.players[i];
            p.contact = None;
            p.swung = true;
            struck = Some(c);
        } else {
            p.contact = Some(Contact { frames: c.frames - 1, ..c });
        }
    }
    // animation clock: contact at 45% of the swing, follow-through over the remaining frames
    let p = &mut g.players[i];
    if let Some(f) = p.swing {
        p.swing = (f + 1 < p.wind + 16).then_some(f + 1);
        if p.swing.is_none() {
            p.hit_at = None;
        }
    }
    struck
}

fn swing_progress(p: &Player) -> Option<f32> {
    let f = p.swing? as f32;
    let w = p.wind.max(1) as f32;
    Some(if f <= w { 0.45 * f / w } else { 0.45 + 0.55 * ((f - w) / 16.0).min(1.0) })
}

/// Serve: toss, then contact at SERVE_CONTACT.
fn advance_serve(g: &mut Game, i: usize) {
    let Some(f) = g.players[i].swing else { return };
    if g.phase == Phase::Serve {
        let (p, end) = (g.players[i].pos, g.players[i].end);
        let t = f as f32 / SERVE_CONTACT as f32;
        g.flight.ball.pos = [p[0] - 0.25 * end, -1.3 - 1.3 * (t * std::f32::consts::FRAC_PI_2).sin(), p[2] + 0.2 * end];
        g.prev_ball = g.flight.ball.pos;
        if f == SERVE_CONTACT {
            let kind = g.players[i].kind;
            serve(g, i, kind);
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
        *p = Player { swing: Some(0), wind: SERVE_CONTACT, serving: true, kind, stance: p.pos[0].abs(), ..*p };
    }
}

/// A shot button press: remembered for a while and checked every frame until a contact is found.
fn press(g: &mut Game, i: usize, kind: i32) {
    let p = &mut g.players[i];
    if p.contact.is_none() && p.swing.is_none() {
        p.pending = Some(PRESS_FRAMES);
        p.kind = kind;
    }
}

/// Every player's turn this frame: humans from their controller slot, the rest from the stand-in AI.
fn control(mut g: ResMut<Game>, mut pads: ResMut<Pads>) {
    let g = &mut *g;
    for i in 0..g.players.len() {
        match pads.slot_of(i) {
            Some(s) => {
                let pad = &mut pads.slots[s];
                let (shot, serve_press) = (pad.shot.take(), std::mem::take(&mut pad.serve));
                human(g, i, pad, shot, serve_press);
            }
            None => bot(g, i),
        }
    }
    let server = g.score.server as usize;
    if g.phase == Phase::Serve && g.message.is_empty() {
        g.message = match pads.slot_of(server) {
            Some(s) => format!("Player {} serve: J/Space (A/Start)", s + 1),
            None => "Serving".into(),
        };
    }
}

fn human(g: &mut Game, i: usize, pad: &SlotPad, shot: Option<i32>, serve_press: bool) {
    g.players[i].aim = pad.stick;
    if g.phase == Phase::Serve && g.score.server == i as i32 {
        if shot.is_some() || serve_press {
            start_serve(g, i, shot.unwrap_or(0));
        }
        advance_serve(g, i);
        return;
    }
    if g.players[i].serving {
        advance_serve(g, i);
    }
    if let Some(kind) = shot {
        press(g, i, kind);
    }
    if g.players[i].contact.is_none() {
        // screen-relative: the camera looks toward +z, so stick right is +x and stick up is +z
        let mut want = pad.stick * RUN * if pad.sprint { SPRINT } else { 1.0 };
        if g.players[i].swing.is_some() {
            want *= 0.25;
        }
        locomote(&mut g.players[i], want);
    }
    // the stick at the moment of contact aims the shot
    let (aim, end) = (pad.stick, g.players[i].end);
    if let Some(c) = advance_stroke(g, i, move |g| aim_target(g, aim, end)) {
        g.popup = Some((timing_word(&c), 50, c.grade));
    }
}

/// Where the ball will be hittable on player `i`'s half: step a copy of the flight until it is waist-high after
/// its bounce.
fn intercept(g: &Game, i: usize) -> Option<(V3, u32)> {
    let half = -g.players[i].end;
    let mut f = g.flight;
    for n in 0..240 {
        f.step(&g.shot, &COURTS[g.court]);
        let b = f.ball;
        if b.pos[2] * half > 0.0 && f.bounces >= 1 && b.vel[1] > 0.0 && (-1.3..-0.5).contains(&b.pos[1]) {
            return Some((b.pos, n));
        }
        if f.bounces >= 2 {
            return None;
        }
    }
    None
}

/// Timing error of the stand-in AI in frames (AIParam's "normal shot deviation" for character 0 is 9).
const BOT_TIMING_ERROR: f32 = 9.0;

/// Stand-in AI for player `i`: serves, runs to the predicted interception (in doubles only the teammate
/// nearer to it; the other goes home), and presses the shot button around the sweet frame with a random timing
/// error, through the same contact search as a human.
fn bot(g: &mut Game, i: usize) {
    if g.phase == Phase::Serve && g.score.server == i as i32 {
        if g.players[i].swing.is_none() {
            let kind = if rand(&mut g.rng) < 0.5 { 0 } else { 2 };
            start_serve(g, i, kind);
        }
        advance_serve(g, i);
        return;
    }
    if g.players[i].serving {
        advance_serve(g, i);
    }
    let busy = g.players[i].contact.is_some() || g.players[i].pending.is_some() || g.players[i].swing.is_some();
    if !busy {
        let plan = match g.phase {
            Phase::Rally if g.last_hitter >= 0 && g.last_hitter & 1 != i as i32 & 1 => intercept(g, i),
            _ => None,
        };
        // ponytail: who takes the ball in doubles is the nearer teammate; the original's partner logic is P0c/P11
        let mine = plan.filter(|(b, _)| {
            let d = |j: usize| Vec2::new(b[0] - g.players[j].pos[0], b[2] - g.players[j].pos[2]).length();
            let mate = i ^ 2;
            mate >= g.players.len() || d(i) <= d(mate)
        });
        let p = g.players[i];
        let goal = mine.map_or(p.home, |(b, _)| [b[0] - 1.1 * (b[0] - p.pos[0]).signum(), 0.0, b[2] - p.end * g.reach.ahead]);
        let d = Vec2::new(goal[0] - p.pos[0], goal[2] - p.pos[2]);
        let want = if d.length() < 0.05 { Vec2::ZERO } else { d.clamp_length_max(RUN) };
        locomote(&mut g.players[i], want);
        // press when the ball is due about SWEET_FRAME frames out, give or take the timing error
        if let Some((_, frames)) = mine {
            let due = SWEET_FRAME as f32 + (rand(&mut g.rng) * 2.0 - 1.0) * BOT_TIMING_ERROR * 0.1;
            if frames as f32 <= due {
                let r = rand(&mut g.rng);
                let kind = if r < 0.6 { 0 } else if r < 0.8 { 1 } else if r < 0.9 { 2 } else { 3 };
                press(g, i, kind);
            }
        }
    }
    let end = g.players[i].end;
    advance_stroke(g, i, move |g| {
        let stick = Vec2::new(rand(&mut g.rng) * 1.8 - 0.9, rand(&mut g.rng) * 1.6 - 0.8);
        aim_target(g, stick, end)
    });
}

fn simulate(mut g: ResMut<Game>) {
    match g.phase {
        Phase::Serve => return,
        Phase::ChangeEnds(0) => return next_point(&mut g),
        Phase::ChangeEnds(n) => return g.phase = Phase::ChangeEnds(n - 1),
        Phase::Over(0) => {
            g.score.second_serve = g.rally.faults == 1;
            g.score.let_ = g.rally.let_;
            g.score.change_ends();
            return next_point(&mut g);
        }
        Phase::Over(n) => g.phase = Phase::Over(n - 1),
        Phase::Post => {
            let g = &mut *g;
            let mut post = g.post.take().expect("post-point state");
            match post.step(&mut g.score, &mut g.rally, &g.board) {
                None => g.post = Some(post),
                Some(Next::Serve) => return next_point(g),
                Some(Next::ChangeEnds) => return g.phase = Phase::ChangeEnds(CHANGE_ENDS),
                Some(Next::MatchOver) => {
                    g.score = Score::new();
                    g.players.iter_mut().for_each(|p| p.stance = 3.0);
                    return next_point(g);
                }
            }
        }
        Phase::Rally => {}
    }
    let g = &mut *g;
    g.prev_ball = g.flight.ball.pos;
    let (shot, surface) = (g.shot, &COURTS[g.court]);
    g.flight.in_play = g.shots > 0;
    match &g.world {
        Some((world, materials)) => g.flight.step_world(&shot, surface, world, materials),
        None => g.flight.step(&shot, surface),
    }
    g.since_hit += 1;
    if g.phase != Phase::Rally {
        return;
    }
    let f = &g.flight;
    // ponytail: no ball body hits and no rest detection on steep surfaces yet; the rally timeout counts as at rest
    let view = BallState { call: f.call, contacts: f.contacts, stopped: f.frame > 1800, pos: f.ball.pos };
    if !g.rally.check(&view, g.shots, g.last_hitter, g.score.server, None, true) {
        return;
    }
    let verdict = g.rally.judge(None);
    let why = CALLS[verdict.call as usize];
    let Some(team) = verdict.winner else {
        g.message = if verdict.call == 2 { "Fault - second serve".into() } else { format!("{why} - serve again") };
        info!("no point: {why} after {} frames", g.since_hit);
        g.phase = Phase::Over(90); // ponytail: the umpire call sets this delay (call sprite + voice line, P0b4c)
        return;
    };
    let event = g.score.point(&g.rules, team as usize);
    g.score.note_tiebreak_start();
    let who = format!("team {}", team + 1);
    g.message = match event {
        Some(Event::Set) if g.score.match_over => format!("{why} - match to {who}"),
        Some(Event::Set) => format!("{why} - set to {who}"),
        Some(Event::Game) => format!("{why} - game to {who}"),
        _ => format!("{why} - point to {who}"),
    };
    info!("point: {who} ({why}) after {} frames, {event:?} {:?}", g.since_hit, g.score);
    g.post = Some(PostPoint::new(event.expect("match in progress")));
    g.phase = Phase::Post;
}

/// Leave the point: next server, receiver and side, and set up the serve.
fn next_point(g: &mut Game) {
    g.score.next_point(&g.rules);
    g.rally.next_point();
    g.rally.new_point();
    g.shots = 0;
    g.last_hitter = -1;
    g.phase = Phase::Serve;
    g.message.clear();
    reset_positions(g);
}

/// Orbit rig parameters that put the eye at `eye` looking along `forward` (game space).
fn orbit_from(eye: [f32; 3], forward: [f32; 3]) -> (Vec3, f32, f32) {
    // game space → Bevy: (x, -y, -z)
    let back = -Vec3::new(forward[0], -forward[1], -forward[2]).normalize();
    (Vec3::new(eye[0], -eye[1], -eye[2]), (-back.y).asin(), back.x.atan2(back.z))
}

/// The original's match camera; free mode leaves the mouse/right stick in charge.
fn camera(mode: Res<CamMode>, cs: Res<CamState>, mut q: Query<(&mut Orbit, &mut Projection)>) {
    let Ok((mut o, mut proj)) = q.single_mut() else { return };
    if *mode == CamMode::Free {
        o.yaw += cs.turn;
        return;
    }
    // ponytail: the serve camera's resting pose; the rally camera's dolly and fit are being ported (P16)
    let (eye, pitch, yaw) = orbit_from(SERVE_CAM_EYE, SERVE_CAM_FORWARD);
    o.radius = 40.0;
    o.pitch = pitch;
    o.yaw = yaw;
    o.focus = eye - Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0) * Vec3::Z * o.radius;
    if let Projection::Perspective(p) = &mut *proj {
        p.fov = 2.0 * (SERVE_CAM_FOV.tan() * 0.75).atan();
    }
}

/// First bounce point of the ball in flight, found by stepping a copy with the real physics.
fn landing(g: &Game) -> Option<V3> {
    if g.phase != Phase::Rally || g.flight.bounces > 0 {
        return None;
    }
    let mut f = g.flight;
    for _ in 0..240 {
        f.step(&g.shot, &COURTS[g.court]);
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
        // the racket reaches for the contact ball, in the figure's own frame
        pose.reach = p.hit_at.map(|b| t.rotation.inverse() * (Vec3::from(b) - t.translation));
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

fn hud(g: Res<Game>, pads: Res<Pads>, mode: Res<CamMode>, mut q: Query<&mut Text, With<ScoreText>>) {
    let who = |i: usize| pads.slot_of(i).map_or("CPU".to_string(), |s| format!("P{}", s + 1));
    let team = |t: usize| (t..g.players.len()).step_by(2).map(who).collect::<Vec<_>>().join("+");
    for mut t in &mut q {
        t.0 = format!(
            "Team 1 ({}) {}  -  {} ({}) Team 2\n{}\ncontrollers: {} · camera: {} (C / Select)\nmove WASD/stick/d-pad · sprint Shift/LB · J/A topspin · K/B slice · I/X flat · L/Y lob · U/RB drive",
            team(0),
            score_line(&g.score, 0),
            score_line(&g.score, 1),
            team(1),
            g.message,
            pads.connected,
            if *mode == CamMode::Original { "original" } else { "free" }
        );
    }
}

/// Games and points for one team, tennis style (points 0/15/30/40/Ad; tiebreak points as numbers).
fn score_line(s: &Score, team: usize) -> String {
    let p = s.points[team];
    let pts = if s.tiebreak { p.to_string() } else { ["0", "15", "30", "40", "Ad"].get(p as usize).unwrap_or(&"Ad").to_string() };
    format!("{} | {pts}", s.games[team])
}
