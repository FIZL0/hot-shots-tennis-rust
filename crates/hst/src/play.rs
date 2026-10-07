//! Playable test mode (`--play`): doubles (four players, `--singles` for two) on the ported ball physics, rules,
//! serve placement, contact search and the game's own shot tables. Players 1 and 3 (one team; 1 and 2 in singles) are
//! humans on controllers 1 and 2 (the keyboard also drives player 1); every other slot is a stand-in AI. Players are
//! original stand-in athletes (`figure.rs`); movement tuning, AI and the serve motion are placeholders until those
//! systems are ported (see TODO.md).
//!
//! The view is the original's match camera, behind the −z baseline; the stick moves and aims screen-relative.
//! A red dot marks where the ball will bounce.
//!
//! Keyboard (player 1): WASD move (aim while swinging), J topspin, K slice, I flat, L lob, U drive,
//! J/Space serve, C camera (original / free), arrow keys turn the free camera.
//! Gamepad: left stick or d-pad move/aim, A topspin, B slice, X flat, Y lob, RB drive, A/Start serve,
//! Select camera, right stick turns the free camera.

use bevy::prelude::*;
use hst_data::{exe::ScoreboardTiming, iso::Iso, tim2, xb::Archive};
use hst_sim::ball::{Ball, COURTS, Flight, Material, Shot, V3, rows4};
use hst_sim::camera::{Camera, Scene};
use hst_sim::court;
use hst_sim::flow::{CHANGE_ENDS, Next, PostPoint, serve_placement};
use hst_sim::judge::{BallState, Lines, Rally};
use hst_sim::mesh::World;
use hst_sim::player::{self as loco, Stats};
use hst_sim::score::{Event, Rules, Score};
use hst_sim::shot::{Bounds, Table, launch, lookup};
use hst_sim::serve::{self, Balloon, ServeData, Toss};
use hst_sim::swing::{self, PathPoint, Reach};

use crate::character::{self, CharacterData, Motion};
use crate::{Args, GameSpace, Orbit};

/// The original's default exhibition: one set to 4 games, deuce on.
const SINGLES: Rules = Rules { sets: 1, games: 4, no_deuce: false, one_point_games: false, players: 2 };
const DOUBLES: Rules = Rules { players: 4, ..SINGLES };
/// The umpire's calls by verdict code.
const CALLS: [&str; 7] = ["Point", "Out", "Fault", "Double fault", "Let", "Out", "Illegal hit"];
/// Ball drawn this much larger than its physical size, toon style, so it reads at broadcast distance.
const BALL_DRAW_SCALE: f32 = 2.4;
/// Typical recorded spin per shot kind (rad/frame); the real per-character records are not ported yet.
const KIND_SPIN: [f32; 5] = [2.9671, -2.0944, 0.0, 5.8905, 3.7088];
/// The contact is graded against this frame after the press (the timing table's sweet spot).
const SWEET_FRAME: i32 = serve::SWEET_FRAME;
/// A press stays live this many frames looking for a contact.
const PRESS_FRAMES: u32 = 28;
/// A whiff: the stroke motion reaches its contact pose on this frame and turns into the whiff motion; a new
/// press is taken from 2 frames after that, and the player is free again 30 frames after it, as the original.
const WHIFF_POSE: u32 = 8;
const WHIFF_REPRESS: u32 = WHIFF_POSE + 2;
const WHIFF_FRAMES: u32 = WHIFF_POSE + 30;
/// The game's field of view is the horizontal half-angle of its 4:3 picture; shown, the vertical is this much
/// of it (measured on the court lines against the real game).
const SHOWN_ASPECT: f32 = 0.75;

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
    /// Serve animation progress (0..1) while this player serves.
    serve_anim: Option<f32>,
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
    /// The timing balloon over this player's head and its age in frames.
    balloon: Option<(Balloon, u32)>,
    /// Stand-in AI: the contact frame it waits for before pressing.
    bot_due: Option<usize>,
    /// +1 right-handed, −1 left-handed (the game mirrors left-handers' models).
    hand: f32,
    /// The stroke motion playing (the contact search's swing animation) and its speed to reach the contact pose.
    stroke: Option<(usize, f32)>,
    /// A swing at nothing: its stroke motion and frames since the press.
    whiff: Option<(usize, u32)>,
    /// Movement stats from TParam.csv.
    stats: Stats,
    /// Run, stamina, stand/run motion and facing as the game's play state (`hst_sim::player::Body`).
    body: loco::Body,
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
    /// Serve trajectory tables, kinds 0..3 (topspin, slice, flat, underhand).
    serve_tables: Vec<Table>,
    serve_data: ServeData,
    /// The serve in progress (server's toss and swing).
    serving: Serving,
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
    /// The original's match camera, stepped with the simulation.
    cam: Camera,
    /// The human whose end the camera follows (the original turns round to stay behind its human).
    cam_owner: Option<usize>,
    /// Per player: the character's pelvis forward row per motion at its first frame (the body turn's input).
    pelvis: Vec<Vec<[f32; 2]>>,
}

/// The serve being set up, as the original's server sub-states: standing or walking the baseline, toss
/// (ball in the hand until the toss animation releases it), and the swing locked onto a contact frame.
#[derive(Clone, Copy, Default)]
struct Serving {
    toss: Option<Toss>,
    /// Frames since the toss press (or, before it, since the serve was set up).
    t: u32,
    tossed: bool,
    swing: Option<ServeSwing>,
    whiffed: bool,
    /// Stand-in AI: the contact frame it aims its press at, and its aim.
    bot_due: Option<usize>,
    bot_aim: Vec2,
}

#[derive(Clone, Copy)]
struct ServeSwing {
    left: u32,
    frames: u32,
    offset: i32,
    grade: u8,
    kind: i32,
}

/// One controller slot's state, gathered every display frame and consumed by the 60 Hz simulation.
#[derive(Clone, Copy, Default)]
struct SlotPad {
    stick: Vec2,
    /// Latched shot press (kind), so a press between fixed steps is never lost.
    shot: Option<i32>,
    serve: bool,
}

/// Controller slots 1 and 2 (keyboard and the first gamepad drive slot 1). See `slot_of` for which player each drives.
#[derive(Resource, Default)]
struct Pads {
    slots: [SlotPad; 2],
    /// Gamepads connected.
    connected: usize,
}

impl Pads {
    /// Which slot controls player `i` of `n`, if any (slot 2 only while a second gamepad is connected). In
    /// doubles slot 2 is player 1's partner (player 3, teams are {1,3} vs {2,4}); in singles the opponent.
    fn slot_of(&self, i: usize, n: usize) -> Option<usize> {
        if std::env::var_os("HST_AUTOPLAY").is_some() {
            return None;
        }
        match i {
            0 => Some(0),
            _ if i == n / 2 && self.connected >= 2 => Some(1),
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
/// A balloon billboard over player `.0`'s head (its own material).
#[derive(Component)]
struct BalloonView(usize, Handle<StandardMaterial>);
/// The balloon textures by `Balloon` (bunny, turtle, note, sweet).
#[derive(Resource)]
struct BalloonArt([Handle<Image>; 4]);

pub fn plugin(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<Pads>()
        .init_resource::<CamMode>()
        .init_resource::<CamState>()
        .add_systems(PostStartup, setup) // after the court's game-space root exists
        .add_systems(Update, (read_input, camera, draw, balloons, mark_landing, character::animate, hud).chain())
        .add_systems(FixedUpdate, (control, simulate, age_balloons, motions, character::tick).chain());
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

/// Character 0's serve tables (`serv0..3`).
fn serve_tables(iso: &mut Iso) -> Vec<Table> {
    let data = iso.read("TRAJ/TRAJ00A.XB").expect("trajectory archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    (0..4)
        .map(|k| {
            let name = format!("tr_pc00_serv{k}.dat");
            let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&name)).expect("serve table");
            Table::parse(&arc.read(e).expect("table bytes")).expect("16^3 table")
        })
        .collect()
}

/// TParam.csv's row for character 0 as cells (header cells hold quoted line breaks; character rows are plain).
fn tparam_row(iso: &mut Iso) -> Vec<String> {
    tparam(iso, 0)
}

/// TParam.csv's row for character `n` as cells.
fn tparam(iso: &mut Iso, n: usize) -> Vec<String> {
    let data = iso.read("PCDATA/PCDATA.XB").expect("character archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).expect("TParam.csv");
    let csv = arc.read(e).expect("TParam.csv bytes");
    let tag = format!("{n},");
    let row = csv.split(|&b| b == b'\n').find(|l| l.starts_with(tag.as_bytes())).expect("character row");
    row.split(|&b| b == b',').map(|c| String::from_utf8_lossy(c).trim().to_string()).collect()
}

/// A character's movement stats from TParam.csv (SPE, Agili, STA, dive/backhand/smash stamina costs).
/// ponytail: clear weather (0); the match's weather table (rain/snow slow the acceleration) comes with P17/P21
fn character_stats(iso: &mut Iso, n: usize) -> Stats {
    let row = tparam(iso, n);
    let cell = |i: usize| row[i].parse::<i32>().expect("TParam stat");
    let costs: Vec<i32> = row[42].split('/').map(|v| v.parse().expect("TParam stamina cost")).collect();
    Stats::new(cell(40), cell(43), cell(41), [costs[0], costs[1], costs[2]], 0)
}

/// A character's hand from TParam.csv (+1 right, −1 left).
fn character_hand(iso: &mut Iso, n: usize) -> f32 {
    let data = iso.read("PCDATA/PCDATA.XB").expect("character archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv")).expect("TParam.csv");
    let csv = arc.read(e).expect("TParam.csv bytes");
    let tag = format!("{n},");
    let row = csv.split(|&b| b == b'\n').find(|l| l.starts_with(tag.as_bytes()));
    // 左 (left) in Shift-JIS
    if row.and_then(|r| r.split(|&b| b == b',').nth(6)).is_some_and(|c| c.starts_with(&[0x8d, 0xb6])) { -1.0 } else { 1.0 }
}

/// Character 0's serve: heights from TParam.csv, the rest measured on character 0 (see `ServeData`).
fn serve_data(iso: &mut Iso) -> ServeData {
    let row = tparam_row(iso);
    let cm = |i: usize| -> [f32; 3] {
        let v: Vec<f32> = row[i].split('/').map(|v| v.parse::<f32>().expect("TParam height") / 100.0).collect();
        [v[0], v[1], v[2]]
    };
    // ponytail: timing tables (+0x1644/+0x1774), toss hand/apex drift (toss animation), mistiming error
    // (0x3fc760, skill level 0) and the serve angle (+0x130c) are character 0's measured values; their sources
    // are the motion data and the character tables (P3/P8)
    let grades = vec![0, 0, 4, 4, 2, 2, 2, 2, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 4, 4, 4];
    ServeData {
        over: cm(65),
        under: cm(66),
        strong_grades: grades.clone(),
        weak_grades: grades,
        hand_over: [0.144, -1.546, 0.12],
        hand_under: [0.016, -0.774, 0.271],
        apex_drift_over: [-0.133, 0.15],
        apex_drift_under: [0.184, 0.033],
        miss: [50, 30, 100],
        max_angle: 22.0,
    }
}

/// The timing balloons from the disc (`AZUMA/C_EFF/EFFCT.XB0`).
fn balloon_art(iso: &mut Iso, images: &mut Assets<Image>) -> [Handle<Image>; 4] {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("effect archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    [Balloon::Bunny, Balloon::Turtle, Balloon::Note, Balloon::Sweet].map(|b| {
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&b.texture().to_ascii_lowercase())).expect("balloon texture");
        let pic = tim2::decode(&arc.read(e).expect("balloon bytes")).expect("TIM2").remove(0);
        images.add(Image::new(
            Extent3d { width: pic.width, height: pic.height, depth_or_array_layers: 1 },
            TextureDimension::D2,
            pic.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        ))
    })
}

fn balloon_index(b: Balloon) -> usize {
    match b {
        Balloon::Bunny => 0,
        Balloon::Turtle => 1,
        Balloon::Note => 2,
        Balloon::Sweet => 3,
    }
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
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let art = balloon_art(&mut iso, &mut images);
    let (line_margin, board, world) = disc(&mut iso, args.stage);
    let rules = if args.singles { SINGLES } else { DOUBLES };
    let mut game = Game {
        rules,
        tables: tables(&mut iso),
        serve_tables: serve_tables(&mut iso),
        serve_data: serve_data(&mut iso),
        serving: Serving::default(),
        reach: reach(&mut iso),
        flight: Flight::new(Ball { pos: [0.0; 3], vel: [0.0; 3], spin: 0.0 }, [[0.0; 4]; 4], [[0.0; 4]; 4]),
        shot: Shot::default(),
        prev_ball: [0.0; 3],
        players: vec![Player { stance: 3.0, hand: 1.0, ..Player::default() }; rules.players as usize],
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
        cam: Camera::new(),
        cam_owner: Some(0),
        pelvis: vec![vec![[0.0, 1.0]; 48]; rules.players as usize],
    };
    reset_positions(&mut game);
    let n = game.players.len();

    let Ok(root) = root.single() else { return };
    // the characters on court, from the disc (each loaded once)
    let mut loaded: std::collections::HashMap<usize, std::sync::Arc<CharacterData>> = Default::default();
    for i in 0..n {
        // default line-up: player 1 is Carol (character 6), then characters 1, 2, 3
        let c = args.chars.get(i).copied().unwrap_or([6, 1, 2, 3][i]);
        let data = match loaded.get(&c) {
            Some(d) => d.clone(),
            None => {
                let d = std::sync::Arc::new(character::load_disc(&mut iso, c, 0, &mut meshes, &mut materials, &mut images, &mut bindposes).unwrap_or_else(|e| panic!("character {c}: {e}")));
                loaded.insert(c, d.clone());
                d
            }
        };
        game.players[i].hand = character_hand(&mut iso, c);
        game.players[i].stats = character_stats(&mut iso, c);
        game.players[i].body.stamina = game.players[i].stats.stamina;
        game.pelvis[i] = data.pelvis.clone();
        let f = character::spawn(&mut commands, &data, root);
        commands.entity(f).insert(Figure(i));
    }
    commands.insert_resource(game);
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
    // one balloon billboard per player (world space, turned to the camera every frame)
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    for i in 0..n {
        let m = materials.add(StandardMaterial { base_color_texture: Some(art[0].clone()), unlit: true, alpha_mode: AlphaMode::Blend, double_sided: true, cull_mode: None, ..default() });
        commands.spawn((BalloonView(i, m.clone()), Mesh3d(quad.clone()), MeshMaterial3d(m), Transform::default(), Visibility::Hidden));
    }
    commands.insert_resource(BalloonArt(art));
}

/// Put every player where the original does for the next serve (`serve_placement`) and the ball in the server's hand.
fn reset_positions(g: &mut Game) {
    for i in 0..g.players.len() {
        let (stance, hand, stats) = (g.players[i].stance, g.players[i].hand, g.players[i].stats);
        let at = serve_placement(i as i32, &g.score, g.rally.faults, stance, 0, g.score.swapped);
        let end = at.facing;
        // stamina is full again every point
        let body = loco::Body::new(at.pos, end, &stats);
        g.players[i] = Player { pos: at.pos, prev: at.pos, end, facing: base_yaw(end), stance, hand, home: at.pos, stats, body, ..Player::default() };
    }
    g.cam.turned = g.cam_owner.is_some_and(|i| g.players[i].pos[2] > 0.0);
    g.cam.cut();
    g.serving = Serving::default();
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
    now[0].shot = [(KeyCode::KeyJ, 0), (KeyCode::KeyK, 1), (KeyCode::KeyI, 2), (KeyCode::KeyL, 3), (KeyCode::KeyU, 4)]
        .into_iter()
        .find(|(k, _)| keys.just_pressed(*k))
        .map(|(_, kind)| kind);
    now[0].serve = keys.just_pressed(KeyCode::Space);
    let mut cycle = keys.just_pressed(KeyCode::KeyC);
    let mut turn = keys.pressed(KeyCode::ArrowRight) as i32 as f32 - keys.pressed(KeyCode::ArrowLeft) as i32 as f32;
    // controllers in connection order: the first is slot 1, the second slot 2
    // ponytail: Steam Input's virtual pads (Valve, 0x28de) mirror real ones; skip them so slot 2 is the second real pad
    let mut list: Vec<_> = gamepads.iter().filter(|(_, g)| g.vendor_id() != Some(0x28de)).collect();
    list.sort_by_key(|(e, _)| *e);
    for (slot, (_, g)) in list.iter().take(2).enumerate() {
        let s = &mut now[slot];
        s.stick += deadzone(g.left_stick()) + g.dpad();
        let buttons = [(GamepadButton::South, 0), (GamepadButton::East, 1), (GamepadButton::West, 2), (GamepadButton::North, 3), (GamepadButton::RightTrigger, 4)];
        s.shot = s.shot.or(buttons.into_iter().find(|(b, _)| g.just_pressed(*b)).map(|(_, k)| k));
        s.serve |= g.just_pressed(GamepadButton::Start);
        cycle |= g.just_pressed(GamepadButton::Select);
        turn += deadzone(g.right_stick()).x;
    }
    pads.connected = list.len();
    for (slot, n) in pads.slots.iter_mut().zip(now) {
        slot.stick = n.stick.clamp_length_max(1.0);
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

/// Launch a stroke (class 1) or serve (class 0) of `kind` by player `who` from the ball's position toward
/// `target`, along the game's trajectory tables.
fn strike(g: &mut Game, who: usize, class: u8, kind: i32, target: V3) {
    let at = g.flight.ball.pos;
    let l = if class == 0 {
        lookup(&g.serve_tables[kind as usize], &Bounds::serve(kind == 3, hst_sim::ball::Params::default().radius), at, target)
    } else {
        lookup(&g.tables[kind as usize], &Bounds::stroke(kind, at[2]), at, target)
    };
    let vel = launch(at, target, l.elevation, l.speed);
    let dir = Vec3::new(vel[0], 0.0, vel[2]).normalize_or(Vec3::Z);
    let side = Vec3::Y.cross(dir).normalize();
    let frame = [side.to_array(), [0.0, 1.0, 0.0], side.cross(Vec3::Y).normalize().to_array()];
    g.shot = Shot { class, kind, curve_frames: l.frames + 1, ..Shot::default() };
    g.shots += 1;
    g.rally.on_hit(g.shots, who as i32, g.score.server, g.score.receiver, g.flight.contacts);
    let spin = if class == 0 { KIND_SPIN[[0, 1, 2, 3][kind as usize]] } else { KIND_SPIN[kind as usize] };
    g.flight = Flight::new(Ball { pos: at, vel, spin }, rows4(frame), rows4(frame));
    let hitter_far = g.players[who].end < 0.0;
    g.flight.lines = Some(Lines { shots: g.shots, doubles: g.rules.players > 2, side: g.score.side, hitter_far, margin: g.line_margin });
    g.prev_ball = at;
    g.last_hitter = who as i32;
    g.since_hit = 0;
    g.phase = Phase::Rally;
}

/// The server's turn while the serve is set up, as the original: before the toss the server stands or walks
/// along the baseline (stick mostly sideways); a shot button tosses (topspin: strong toss, lob: underhand,
/// others: weak toss) and the ball leaves the hand on the toss animation's release frame; a press while it is
/// in the air (topspin/slice, or lob for an underhand toss) locks the swing onto the contact frame the serve
/// search finds — the timing grade there decides the balloon and, for a strong toss, how far the aim is thrown
/// off. No contact in reach is a whiff; a toss that lands is simply tossed again.
fn serve_turn(g: &mut Game, i: usize, stick: Vec2, press: Option<i32>) {
    let (pos, end) = (g.players[i].pos, g.players[i].end);
    let mut s = g.serving;
    let Some(toss) = s.toss else {
        if let Some(k) = press {
            s.toss = Some(match k {
                0 => Toss::Strong,
                3 => Toss::Under,
                _ => Toss::Weak,
            });
            s.t = 0;
            g.players[i].stance = pos[0].abs();
        } else if stick.x.abs() > stick.y.abs() {
            // walk the baseline, between the centre mark's side and the sideline
            let court = if g.score.side == 0 { 1.0 } else { -1.0 };
            let far = end * court * if g.rules.players > 2 { serve::WALK_MAX_DOUBLES } else { serve::WALK_MAX_SINGLES };
            let x = pos[0] + serve::WALK * stick.x.signum();
            let p = &mut g.players[i];
            p.prev = p.pos;
            p.pos[0] = if far > 0.0 { x.clamp(serve::WALK_MIN, far) } else { x.clamp(far, -serve::WALK_MIN) };
            p.stride += serve::WALK * 9.0;
        }
        s.t += 1;
        g.serving = s;
        hold_ball(g, i, Toss::Strong);
        g.players[i].serve_anim = Some(0.0);
        return;
    };
    s.t += 1;
    if !s.tossed {
        g.serving = s;
        if s.t < serve::TOSS_RELEASE {
            hold_ball(g, i, toss);
        } else {
            let (hand, apex) = serve::toss_points(&g.serve_data, toss, pos, end);
            g.flight = Flight::new(Ball { pos: hand, vel: serve::toss_velocity(hand, apex), spin: 0.1 }, rows4([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]), rows4([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]));
            g.shot = Shot::default();
            g.prev_ball = hand;
            g.serving.tossed = true;
        }
        g.players[i].serve_anim = Some(0.35 * (s.t as f32 / serve::TOSS_RELEASE as f32).min(1.0));
        return;
    }
    match s.swing {
        None if !s.whiffed => {
            let allowed = match (toss, press) {
                (Toss::Under, Some(3)) => true,
                (Toss::Strong | Toss::Weak, Some(0 | 1)) => true,
                _ => false,
            };
            if allowed {
                let grades = g.serve_data.grades(toss).to_vec();
                match serve::search(&g.serve_data, toss, &predicted_path(g, grades.len())) {
                    Some(k) => {
                        let kind = if toss == Toss::Under { 3 } else { press.unwrap_or(0) };
                        s.swing = Some(ServeSwing { left: k as u32, frames: k as u32, offset: k as i32 - SWEET_FRAME, grade: grades[k], kind });
                    }
                    None => s.whiffed = true,
                }
            }
        }
        Some(mut sw) => {
            sw.left = sw.left.saturating_sub(1);
            s.swing = Some(sw);
        }
        None => {}
    }
    g.serving = s;
    // swing animation: wind up to contact, follow through
    g.players[i].serve_anim = Some(match s.swing {
        Some(sw) => 0.35 + 0.3 * (1.0 - sw.left as f32 / sw.frames.max(1) as f32),
        None if s.whiffed => 0.9,
        None => 0.35,
    });
    if let Some(sw) = s.swing.filter(|sw| sw.left == 0) {
        let aim = if pads_aim(g, i) { stick } else { g.serving.bot_aim };
        let rand_bit = rand(&mut g.rng) < 0.5;
        let d = &g.serve_data;
        let target = serve::target(d, toss, sw.offset, sw.grade, pos, end, g.score.side, g.rules.players > 2, [aim.x, aim.y], rand_bit);
        debug!("player {i} serve {toss:?} kind {}: offset {} grade {} -> {target:?}", sw.kind, sw.offset, sw.grade);
        g.players[i].balloon = serve::balloon(sw.grade, sw.offset, false).map(|b| (b, 0));
        g.players[i].serve_anim = None;
        g.players[i].stance = pos[0].abs();
        strike(g, i, 0, sw.kind, target);
        g.serving = Serving::default();
        return;
    }
    // a toss that comes down untouched (or after a whiff) is tossed again
    if g.flight.bounces > 0 {
        g.serving = Serving { t: 0, ..Serving::default() };
        hold_ball(g, i, Toss::Strong);
    }
}

/// Radial stick deadzone: worn or off-centre sticks rest a little off zero and would creep the player along.
fn deadzone(v: Vec2) -> Vec2 {
    const DEADZONE: f32 = 0.2; // ponytail: fixed; make it a setting if some pads need more
    if v.length() < DEADZONE { Vec2::ZERO } else { v }
}

/// Whether player `i`'s serve aim comes from a stick (humans) rather than the stand-in AI's pick.
fn pads_aim(g: &Game, i: usize) -> bool {
    g.serving.bot_due.is_none() || i != g.score.server as usize
}

/// The ball rests in the server's tossing hand.
fn hold_ball(g: &mut Game, i: usize, toss: Toss) {
    let (pos, end) = (g.players[i].pos, g.players[i].end);
    let (hand, _) = serve::toss_points(&g.serve_data, toss, pos, end);
    g.flight = Flight::new(Ball { pos: hand, vel: [0.0; 3], spin: 0.0 }, [[0.0; 4]; 4], [[0.0; 4]; 4]);
    g.prev_ball = hand;
}

/// A stick direction on screen as a direction on the court (x, z): right along the camera's right, up along its
/// forward.
fn screen(g: &Game, stick: Vec2) -> Vec2 {
    let r = g.cam.view.rot;
    let right = Vec2::new(r[0][0], r[0][2]).normalize_or(Vec2::X);
    let up = Vec2::new(r[2][0], r[2][2]).normalize_or(Vec2::Y);
    right * stick.x + up * stick.y
}

/// Analog aim on the court (x, z direction from `screen`, or a bot's random pick): sideways spans the court,
/// +z moves the target toward +z (deeper for the −z team, shorter for the other); centred is a deep middle ball.
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

/// Run in direction `dir` (game-space x, z; any length) or stand, as the original's play state
/// (`hst_sim::player::Body`): full speed in the stick's direction growing by 30 % over the character's agility
/// frames, stamina drained while running in a rally, the body turning toward the run 22.5° a frame, the run motion
/// by direction (dash past half the agility) or the ready stance, the doubles partner kept 1 m away and the court
/// bounds of the game's mover.
fn locomote(g: &mut Game, i: usize, dir: Vec2) {
    let n = g.players.len();
    let sc = loco::Scene {
        players: n as i32,
        phase: match g.phase {
            Phase::ChangeEnds(_) => 1,
            Phase::Serve => 2,
            Phase::Rally => 3,
            Phase::Post | Phase::Over(_) => 4,
        },
        last_hitter: g.last_hitter,
        team: i as i32,
        forward: g.players[i].end,
        hand: g.players[i].hand,
        // the partner, already moved this frame if it updates first
        mate: (n == 4).then(|| g.players[i ^ 2].pos),
        ball: g.flight.ball.pos,
        ball_dir: g.flight.ball.vel,
        short: false,
    };
    let Game { players, pelvis, .. } = g;
    let p = &mut players[i];
    p.prev = p.pos;
    p.body.pos = p.pos;
    p.body.step(&p.stats, [dir.x, dir.y], &sc, &pelvis[i]);
    p.pos = p.body.pos;
    p.vel = if p.body.running { Vec2::new(p.body.vel[0], p.body.vel[2]) } else { Vec2::ZERO };
    p.stride += p.vel.length() * 9.0;
    p.facing = yaw(p.body.face.dir);
}

/// Figure yaw (0 faces −z) of a game-space facing direction.
fn yaw(d: [f32; 4]) -> f32 {
    (-d[0]).atan2(-d[2])
}

/// A stroke squares the body up to the other end (the game's stroke mode sets facing and target forward).
fn square_up(p: &mut Player) {
    let dir = [0.0, 0.0, p.end, 0.0];
    p.body.target = dir;
    p.body.face = loco::Facing { dir, way: -1, ..loco::Facing::default() };
    p.body.running = false;
    p.facing = base_yaw(p.end);
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
            p.stroke = Some((c.swing.anim as usize, serve::SWEET_FRAME as f32 / c.frames.max(1) as f32));
            p.backhand = !c.swing.forehand;
            p.swing = Some(0);
            p.swung = false;
            p.vel = Vec2::ZERO;
            square_up(p);
        } else {
            g.players[i].pending = left.checked_sub(1);
            if left == 0 {
                whiff(g, i);
            }
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
            strike(g, i, 1, kind, target);
            debug!("player {i} {:?} {} anim {:#x}: {} (offset {}, grade {})", c.swing.branch, if c.swing.forehand { "forehand" } else { "backhand" }, c.swing.anim, timing_word(&c), c.offset, c.grade);
            let rally = g.phase == Phase::Rally && g.players.len() > 1;
            let p = &mut g.players[i];
            if rally {
                let branch = match c.swing.branch {
                    swing::Branch::Ground => 1,
                    swing::Branch::Volley => 2,
                    swing::Branch::Smash => 4,
                };
                p.body.stamina = loco::stroke_stamina(&p.stats, p.body.stamina, branch, c.swing.forehand, 0);
            }
            p.contact = None;
            p.swung = true;
            p.balloon = serve::balloon(c.grade, c.offset, false).map(|b| (b, 0));
            struck = Some(c);
        } else {
            p.contact = Some(Contact { frames: c.frames - 1, ..c });
        }
    }
    // animation clock: contact at 45% of the swing, follow-through over the remaining frames
    let p = &mut g.players[i];
    if let Some((anim, f)) = p.whiff {
        p.prev = p.pos;
        p.whiff = (f + 1 < WHIFF_FRAMES).then_some((anim, f + 1));
    }
    if let Some(f) = p.swing {
        p.swing = (f + 1 < p.wind + 16).then_some(f + 1);
        if p.swing.is_none() {
            p.hit_at = None;
            p.stroke = None;
        }
    }
    struck
}

/// A shot button press: remembered for a while and checked every frame until a contact is found. With no
/// ball for this player to hit (or none found in time) the player swings at nothing, as the original.
/// ponytail: the original holds an early press only while its auto-approach (P7) finds a reachable ball ahead;
/// the fixed PRESS_FRAMES window stands in, so an early whiff comes up to 28 frames late.
fn press(g: &mut Game, i: usize, kind: i32) {
    let p = &mut g.players[i];
    let free = p.whiff.is_none_or(|(_, f)| f >= WHIFF_REPRESS);
    if p.contact.is_none() && p.swing.is_none() && p.pending.is_none() && free {
        p.whiff = None;
        p.kind = kind;
        let theirs = g.phase == Phase::Rally && g.last_hitter >= 0 && g.last_hitter & 1 != i as i32 & 1;
        if theirs {
            g.players[i].pending = Some(PRESS_FRAMES);
        } else {
            whiff(g, i);
        }
    }
}

/// Swing at nothing: the ground stroke of the pressed shot type, on the side the ball passes (the side of the
/// ball's line the player stands on), at full speed.
fn whiff(g: &mut Game, i: usize) {
    let (b, p) = (g.flight.ball, &mut g.players[i]);
    let base = match p.kind {
        1 => 0x12,
        3 => 0x14,
        _ => 0x10,
    };
    let d = [p.pos[0] - b.pos[0], p.pos[2] - b.pos[2]];
    let right = d[1] * b.vel[0] - d[0] * b.vel[2] > 0.0;
    let other = if right { p.hand >= 0.0 } else { p.hand < 0.0 };
    p.whiff = Some((base + other as usize, 0));
    p.vel = Vec2::ZERO;
    p.facing = base_yaw(p.end);
}

/// Every player's turn this frame: humans from their controller slot, the rest from the stand-in AI.
fn control(mut g: ResMut<Game>, mut pads: ResMut<Pads>) {
    let g = &mut *g;
    for i in 0..g.players.len() {
        match pads.slot_of(i, g.players.len()) {
            Some(s) => {
                let pad = &mut pads.slots[s];
                let (shot, serve_press) = (pad.shot.take(), std::mem::take(&mut pad.serve));
                human(g, i, pad, shot, serve_press);
            }
            None => bot(g, i),
        }
    }
    g.cam_owner = (0..g.players.len()).find(|&i| pads.slot_of(i, g.players.len()).is_some());
    let server = g.score.server as usize;
    if g.phase == Phase::Serve && g.message.is_empty() {
        g.message = match pads.slot_of(server, g.players.len()) {
            Some(s) => format!("Player {} serve: walk the baseline, toss with J/A (strong), K/B (weak) or L/Y (underhand), hit at the top with J/A or K/B (L/Y underhand)", s + 1),
            None => "Serving".into(),
        };
    }
}

fn human(g: &mut Game, i: usize, pad: &SlotPad, shot: Option<i32>, serve_press: bool) {
    g.players[i].aim = pad.stick;
    if g.phase == Phase::Serve && g.score.server == i as i32 {
        let press = shot.or(serve_press.then_some(0));
        let stick = screen(g, pad.stick);
        serve_turn(g, i, stick, press);
        return;
    }
    if let Some(kind) = shot {
        press(g, i, kind);
    }
    if g.players[i].contact.is_none() && g.players[i].whiff.is_none() {
        // screen-relative: stick right follows the camera's right, stick up its ground-forward; no running
        // through the follow-through
        let dir = if g.players[i].swing.is_some() { Vec2::ZERO } else { screen(g, pad.stick) };
        locomote(g, i, dir);
    }
    // the stick at the moment of contact aims the shot
    let (aim, end) = (screen(g, pad.stick), g.players[i].end);
    advance_stroke(g, i, move |g| aim_target(g, aim, end));
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

/// How early/late the original's bots press (contact frame − sweet frame, with counts), measured over every
/// stroke and serve of a recorded bot match (slot 5). The stand-in AI draws its timing from these.
/// ponytail: the AI's own timing logic (AIParam, P11) replaces this.
const BOT_STROKE_TIMING: [(i32, u32); 17] =
    [(-6, 52), (-5, 16), (-4, 4), (-3, 4), (-2, 4), (-1, 9), (0, 7), (1, 4), (2, 9), (3, 4), (4, 4), (5, 3), (6, 4), (7, 1), (9, 2), (10, 2), (15, 2)];
const BOT_SERVE_TIMING: [(i32, u32); 7] = [(-6, 9), (-5, 2), (-4, 1), (-3, 1), (-2, 1), (-1, 1), (0, 20)];

/// A contact frame drawn from a recorded timing distribution.
fn draw_due(rng: &mut u32, table: &[(i32, u32)]) -> usize {
    let total: u32 = table.iter().map(|&(_, n)| n).sum();
    let mut r = (rand(rng) * total as f32) as u32;
    for &(off, n) in table {
        if r < n {
            return (SWEET_FRAME + off).max(0) as usize;
        }
        r -= n;
    }
    SWEET_FRAME as usize
}

/// Stand-in AI for player `i`: serves, runs to the predicted interception (in doubles only the teammate
/// nearer to it; the other goes home), and presses the shot button around the sweet frame with a random timing
/// error, through the same contact search as a human.
fn bot(g: &mut Game, i: usize) {
    if g.phase == Phase::Serve && g.score.server == i as i32 {
        return bot_serve(g, i);
    }
    let busy = g.players[i].contact.is_some() || g.players[i].pending.is_some() || g.players[i].swing.is_some() || g.players[i].whiff.is_some();
    if !busy {
        let plan = match g.phase {
            Phase::Rally if g.last_hitter >= 0 && g.last_hitter & 1 != i as i32 & 1 => intercept(g, i),
            _ => None,
        };
        // ponytail: in doubles each teammate takes the balls on its side of the court (the nearer by distance left
        // the one at the net standing all rally); the original's partner logic is P0c/P11
        let mine = plan.filter(|(b, _)| {
            let d = |j: usize| (b[0] - g.players[j].pos[0]).abs();
            let mate = i ^ 2;
            mate >= g.players.len() || d(i) < d(mate) || (d(i) == d(mate) && i < mate)
        });
        let p = g.players[i];
        let goal = mine.map_or(p.home, |(b, _)| [b[0] - 1.1 * (b[0] - p.pos[0]).signum(), 0.0, b[2] - p.end * g.reach.ahead]);
        let d = Vec2::new(goal[0] - p.pos[0], goal[2] - p.pos[2]);
        // ponytail: stops within one stride of the goal; the AI's own approach is P11
        let dir = if d.length() < 0.1 { Vec2::ZERO } else { d };
        locomote(g, i, dir);
        // press when the contact search would lock onto the drawn frame (or later, if it is already past)
        if mine.is_some() {
            if g.players[i].bot_due.is_none() {
                let r = rand(&mut g.rng);
                g.players[i].kind = if r < 0.6 { 0 } else if r < 0.8 { 1 } else if r < 0.9 { 2 } else { 3 };
                g.players[i].bot_due = Some(draw_due(&mut g.rng, &BOT_STROKE_TIMING));
            }
            let due = g.players[i].bot_due.unwrap_or(SWEET_FRAME as usize);
            if find_contact(g, i).is_some_and(|c| c.frames as usize <= due) {
                let kind = g.players[i].kind;
                press(g, i, kind);
                g.players[i].bot_due = None;
            }
        } else {
            g.players[i].bot_due = None;
        }
    }
    let end = g.players[i].end;
    advance_stroke(g, i, move |g| {
        let stick = Vec2::new(rand(&mut g.rng) * 1.8 - 0.9, rand(&mut g.rng) * 1.6 - 0.8);
        aim_target(g, stick, end)
    });
}

/// The stand-in AI's serve: strong toss, swing timed from the recorded bots' serve timing (a badly timed
/// strong toss then goes wide or long, as in the original). ponytail: the AI's serve aim is P11.
fn bot_serve(g: &mut Game, i: usize) {
    if g.serving.bot_due.is_none() {
        g.serving.bot_due = Some(draw_due(&mut g.rng, &BOT_SERVE_TIMING));
        g.serving.bot_aim = Vec2::new(rand(&mut g.rng) * 2.0 - 1.0, rand(&mut g.rng) * 2.0 - 1.0);
    }
    let s = g.serving;
    let press = if s.toss.is_none() {
        (s.t >= 40).then_some(0)
    } else if s.tossed && s.swing.is_none() && !s.whiffed {
        let horizon = g.serve_data.grades(Toss::Strong).len();
        let due = s.bot_due.unwrap_or(SWEET_FRAME as usize);
        serve::search(&g.serve_data, Toss::Strong, &predicted_path(g, horizon)).filter(|&k| k <= due).map(|_| 0)
    } else {
        None
    };
    serve_turn(g, i, Vec2::ZERO, press);
}

fn simulate(mut g: ResMut<Game>) {
    let g2 = &mut *g;
    let players: Vec<V3> = g2.players.iter().map(|p| p.pos).collect();
    g2.cam.step(&Scene { players: &players, ball: g2.flight.ball.pos });
    match g.phase {
        // only the toss flies while the serve is set up
        Phase::Serve if !g.serving.tossed => return,
        Phase::Serve => {}
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
    if g.phase != Phase::Rally || g.shots == 0 {
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

/// The original's match camera (`hst_sim::camera`); free mode leaves the mouse/right stick in charge.
fn camera(g: Res<Game>, mode: Res<CamMode>, cs: Res<CamState>, window: Query<&Window>, mut q: Query<(&mut Orbit, &mut Projection)>) {
    let Ok((mut o, mut proj)) = q.single_mut() else { return };
    if *mode == CamMode::Free {
        o.yaw += cs.turn;
        return;
    }
    let v = g.cam.view;
    let (eye, pitch, yaw) = orbit_from(v.eye, v.rot[2]);
    o.radius = 40.0;
    o.pitch = pitch;
    o.yaw = yaw;
    o.focus = eye - Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0) * Vec3::Z * o.radius;
    if let Projection::Perspective(p) = &mut *proj {
        // never show less than the game's 4:3 picture: windows narrower than 4:3 widen vertically
        let aspect = window.single().map_or(4.0 / 3.0, |w| w.width() / w.height().max(1.0));
        p.fov = 2.0 * (v.fov.tan() * SHOWN_ASPECT.max(1.0 / aspect)).atan();
        // the camera stays ~40 m out: a far near plane keeps depth precision for the layered character models
        p.near = 5.0;
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

fn draw(g: Res<Game>, time: Res<Time<Fixed>>, mut figures: Query<(&Figure, &mut Transform)>, mut ball: Query<&mut Transform, (With<BallView>, Without<Figure>)>) {
    let a = time.overstep_fraction();
    for (f, mut t) in &mut figures {
        let p = g.players[f.0];
        t.translation = Vec3::from(p.prev).lerp(Vec3::from(p.pos), a);
        // the models face their local +z; `facing` is the stand-in yaw (0 = facing −z); left-handers mirrored
        t.rotation = Quat::from_rotation_y(p.facing - std::f32::consts::PI);
        t.scale = Vec3::new(p.hand, 1.0, 1.0);
    }
    for mut t in &mut ball {
        t.translation = Vec3::from(g.prev_ball).lerp(Vec3::from(g.flight.ball.pos), a);
    }
}

/// Which of the game's motions each player plays (by its motion number): the serve's stance, baseline walk,
/// toss and swing; a locked stroke's swing (timed so its contact pose, frame 8, meets the ball); otherwise the
/// stand/run motion the game picks (`locomote`).
fn motions(g: Res<Game>, mut q: Query<(&Figure, &mut Motion)>) {
    for (f, mut m) in &mut q {
        let i = f.0;
        let p = &g.players[i];
        let serving = g.phase == Phase::Serve && g.score.server == i as i32;
        if serving {
            let s = g.serving;
            let under = s.toss == Some(Toss::Under);
            match (s.toss, s.swing) {
                (None, _) if p.pos != p.prev => {
                    let right = (p.pos[0] - p.prev[0]) * p.end * p.hand < 0.0;
                    m.play(if right { 0x22 } else { 0x21 }, 1.0, true);
                }
                (None, _) => m.play(0x20, 1.0, true),
                (Some(_), Some(sw)) => {
                    let id = if under { 0x26 } else { 0x25 };
                    let speed = if m.id == id { m.speed } else { serve::SWEET_FRAME as f32 / sw.frames.max(1) as f32 };
                    let past = m.id == id && m.time >= serve::SWEET_FRAME as f32;
                    m.play(id, if past { 1.0 } else { speed }, false);
                }
                (Some(_), None) if s.whiffed => m.play(if under { 0x2a } else { 0x29 }, 1.0, false),
                (Some(_), None) => m.play(if under { 0x24 } else { 0x23 }, 1.0, false),
            }
            continue;
        }
        // a whiff's stroke turns into the whiff motion at the contact pose (forehand-side strokes 0x27, the others 0x28)
        if let Some((anim, f)) = p.whiff {
            m.play(if f < WHIFF_POSE { anim } else if anim % 2 == 0 { 0x27 } else { 0x28 }, 1.0, false);
            continue;
        }
        if let Some((anim, speed)) = p.stroke {
            let past = m.id == anim && m.time >= serve::SWEET_FRAME as f32;
            m.play(anim, if past { 1.0 } else { speed }, false);
            continue;
        }
        // a finished serve swing plays out before running
        if (m.id == 0x25 || m.id == 0x26) && m.time < 30.0 {
            continue;
        }
        m.play(p.body.motion as usize, 1.0, true);
    }
}

/// Balloons live their fade-in, hold and fade-out, in simulation frames.
fn age_balloons(mut g: ResMut<Game>) {
    for p in &mut g.players {
        if let Some((b, age)) = p.balloon {
            p.balloon = serve::balloon_alpha(age + 1).map(|_| (b, age + 1));
        }
    }
}

/// Each player's balloon: a billboard facing the camera, its tail just above the head.
fn balloons(
    g: Res<Game>,
    art: Res<BalloonArt>,
    time: Res<Time<Fixed>>,
    cam: Query<&Transform, (With<Camera3d>, Without<BalloonView>)>,
    mut q: Query<(&BalloonView, &mut Transform, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(cam) = cam.single() else { return };
    let a = time.overstep_fraction();
    for (view, mut t, mut vis) in &mut q {
        let p = &g.players[view.0];
        let Some((b, age)) = p.balloon else {
            *vis = Visibility::Hidden;
            continue;
        };
        let Some(alpha) = serve::balloon_alpha(age) else { continue };
        if let Some(mut m) = materials.get_mut(&view.1) {
            m.base_color_texture = Some(art.0[balloon_index(b)].clone());
            m.base_color = Color::srgba(1.0, 1.0, 1.0, alpha);
        }
        let at = Vec3::from(p.prev).lerp(Vec3::from(p.pos), a);
        // game space → world: (x, -y, -z); the figure's head centre stands 1.77 m up
        let size = 2.0 * serve::BALLOON_SIZE;
        let up = 1.77 + serve::BALLOON_LIFT + size * 0.5;
        t.translation = Vec3::new(at.x, up, -at.z);
        t.rotation = cam.rotation;
        t.scale = Vec3::splat(size);
        *vis = Visibility::Visible;
    }
}

fn hud(g: Res<Game>, pads: Res<Pads>, mode: Res<CamMode>, mut q: Query<&mut Text, With<ScoreText>>) {
    let who = |i: usize| pads.slot_of(i, g.players.len()).map_or("CPU".to_string(), |s| format!("P{}", s + 1));
    let team = |t: usize| (t..g.players.len()).step_by(2).map(who).collect::<Vec<_>>().join("+");
    for mut t in &mut q {
        t.0 = format!(
            "Team 1 ({}) {}  -  {} ({}) Team 2\n{}\ncontrollers: {} · camera: {} (C / Select)\nmove WASD/stick/d-pad · J/A topspin · K/B slice · I/X flat · L/Y lob · U/RB drive",
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
