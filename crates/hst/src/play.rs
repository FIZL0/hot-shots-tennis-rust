//! Playable test mode (`--play`): doubles (four players, `--singles` for two) on the ported ball physics, rules,
//! serve placement, contact search and the game's own shot tables. Players 1 and 3 (one team; 1 and 2 in singles) are
//! humans on controllers 1 and 2 (the keyboard also drives player 1); every other slot is a stand-in AI. Players are
//! original stand-in athletes (`figure.rs`); movement tuning, AI and the serve motion are placeholders until those
//! systems are ported (see TODO.md).
//!
//! The view is the original's match camera, behind the −z baseline; the stick moves and aims screen-relative.
//! A red dot marks where the ball will bounce.
//!
//! Keyboard (player 1): WASD move (aim while swinging), J topspin, K slice, L lob, J/Space serve, C camera
//! (original / free), arrow keys turn the free camera.
//! Gamepad: left stick or d-pad move/aim, A (✕) topspin, B (○) slice, Y (△) lob, A/Start serve, Select camera,
//! right stick turns the free camera. As the original, there is no flat or drop button: the stick toward the
//! net at contact makes a topspin flat, pulled back makes a slice a drop shot (`shot::stick_kind`).

use bevy::prelude::*;
use hst_data::{exe::ScoreboardTiming, iso::Iso, tim2, xb::Archive};
use hst_sim::ball::{Ball, COURTS, Flight, Material, Shot, V3, rows4};
use hst_sim::camera::{Camera, Scene, View};
use hst_sim::court;
use hst_sim::effect::{PathEntry, SmashSearch};
use hst_sim::flow::{CHANGE_ENDS, Next, PostPoint, serve_placement};
use hst_sim::judge::{BallState, Lines, Rally};
use hst_sim::mesh::World;
use hst_sim::motion;
use hst_sim::npc;
use hst_sim::params::{self, ShotParams};
use hst_sim::player::{self as loco, Stats};
use hst_sim::pose::{ArmIk, contact_solve};
use hst_sim::score::{Event, Rules, Score};
use hst_sim::serve::{self, Balloon, ServeData, Toss};
use hst_sim::shot::{Bounds, Table, launch, lookup};
use hst_sim::sound;
use hst_sim::swing::{self, PathPoint, Reach};
use hst_sim::umpire::Umpire;

use crate::audio::{
    CourtBank, GalleryBank, MusicBanks, Sound, SoundBank, VoiceBanks, umpire_bank, voice_bank,
};
use crate::character::{self, CharacterData, Motion};
use crate::effects;
use crate::{Args, GameSpace, Orbit};

mod panel;

/// The original's default exhibition: one set to 4 games, deuce on.
const SINGLES: Rules = Rules {
    sets: 1,
    games: 4,
    no_deuce: false,
    one_point_games: false,
    players: 2,
};
const DOUBLES: Rules = Rules {
    players: 4,
    ..SINGLES
};
/// The umpire's calls by verdict code.
const CALLS: [&str; 7] = [
    "Point",
    "Out",
    "Fault",
    "Double fault",
    "Let",
    "Out",
    "Illegal hit",
];
/// Ball (and shadow) drawn this much larger than its physical size, toon style, so it reads at broadcast distance.
/// ponytail: the original draws `ball1.mdl` at scale 1 (ball object matrix in s03–s05); this is the remaster's look.
const BALL_DRAW_SCALE: f32 = 2.4;
/// Typical recorded spin per stroke kind (rad/frame); the real per-character records are not ported yet (serves use them).
const KIND_SPIN: [f32; 5] = [2.9671, -2.0944, 0.0, 5.8905, 3.7088];
/// The contact is graded against this frame after the press (the timing table's sweet spot).
const SWEET_FRAME: i32 = serve::SWEET_FRAME;
/// A press stays live this many frames looking for a contact.
const PRESS_FRAMES: u32 = 28;
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
    /// `facing` one tick earlier (drawing blends the two).
    prev_facing: f32,
    stride: f32,
    /// Frames into the current swing; it lasts through the follow-through.
    swing: Option<u32>,
    /// The follow-through: frames since contact, frames before a stick or press may break it off, and whether the
    /// motion has played to its end (as of the last tick).
    after: Option<u32>,
    recover: u32,
    played: bool,
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
    /// Stand-in AI: the shot it last decided to leave to its partner.
    bot_left: i32,
    /// This player's AIParam.csv row when the computer plays it (`hst_sim::ai`).
    ai: hst_sim::ai::AiParams,
    /// +1 right-handed, −1 left-handed (the game mirrors left-handers' models).
    hand: f32,
    /// The motion the game's code last set (number, speed, loop, start frame), restarted on every set.
    cmd: Cmd,
    /// A swing waiting for 8 frames before contact while the body turns, and a soft follow-through due next frame.
    wait_swing: Option<i32>,
    follow: Option<i32>,
    /// A swing at nothing (`motion::Whiff`), and a pending press's whiff to come is a re-press (no shout).
    whiff: Option<motion::Whiff>,
    whiff_quiet: bool,
    /// The player's last swing ended in its miss motion and no swing has started since: the next whiff is quiet.
    /// Cleared by a stroke press or a serve swing, not by the point's end (the game's re-press window outlives it).
    missed: bool,
    /// Movement stats from TParam.csv.
    stats: Stats,
    /// Run, stamina, stand/run motion and facing as the game's play state (`hst_sim::player::Body`).
    body: loco::Body,
    /// The post-point reaction carrying the player along its root path, if any.
    root: Option<Root>,
    /// The contact IK after a stroke's solve (the body's step into the shot, the arm's turn toward the ball),
    /// and this frame's arm turn and weight for drawing.
    ik: Option<ArmIk>,
    arm: Option<([[f32; 4]; 4], f32)>,
    /// A dive under way (the press found no stroke while running).
    dive: Option<swing::Dive>,
}

/// A reaction's root motion: its motion number, the spot it started from and the accumulated spot (game space,
/// w carried as the game's), and the frames it has run.
#[derive(Clone, Copy, Default)]
struct Root {
    motion: usize,
    base: [f32; 4],
    acc: [f32; 4],
    t: f32,
}

/// A motion set by the game's code (the motion setter): number, speed, looping, crossfade hold.
#[derive(Clone, Copy, Default)]
struct Cmd {
    id: usize,
    speed: f32,
    looping: bool,
    hold: Option<i32>,
    serial: u32,
}

/// The game's motion setter: a looping motion already playing keeps going, anything else restarts.
fn set_motion(p: &mut Player, id: i32, speed: f32, looping: bool, hold: Option<i32>) {
    if looping && p.cmd.serial > 0 && p.cmd.id == id as usize {
        return;
    }
    p.cmd = Cmd {
        id: id as usize,
        speed,
        looping,
        hold,
        serial: p.cmd.serial + 1,
    };
}

/// A pressed swing locked onto the ball: frames until contact, the game's contact search result, the grade
/// and the body's step into the shot.
#[derive(Clone, Copy)]
struct Contact {
    frames: u32,
    swing: swing::Swing,
    grade: u8,
    offset: i32,
}

#[derive(PartialEq)]
enum Phase {
    Serve,
    Rally,
    /// The point is over (or a fault or let called): the umpire's call and the scoreboard's turn (`Game::post`).
    Post,
    ChangeEnds(u32),
}

#[derive(Resource)]
struct Game {
    rules: Rules,
    tables: Vec<Table>,
    /// Serve trajectory tables, kinds 0..3 (topspin, slice, flat, underhand).
    /// Per player: the character's serve trajectory tables with their launch spin, [strong/underhand, weak toss]
    /// by kind (topspin, slice, flat, underhand; the weak toss's `dw1` tables have no underhand).
    serve_tables: Vec<[Vec<(Table, f32)>; 2]>,
    /// Smash trajectory tables, smash kinds 0 (✕/○) and 1 (△).
    smash_tables: Vec<Table>,
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
    /// A human pressed a face button while the point is over (`post_press`), for the next simulation tick.
    post_press: bool,
    board: ScoreboardTiming,
    /// The stage's collision world (`--stage`, else court 10's) and the material table: court, net, posts, walls.
    world: (World, Vec<Material>),
    court: usize,
    message: String,
    rng: u32,
    /// Sounds due this tick, at a game-space position.
    sounds: Vec<(sound::Play, V3)>,
    /// Swing whooshes and dive thuds waiting to play: ticks left, the player (played at their position) and the sound.
    whooshes: Vec<(u32, usize, sound::Play)>,
    /// The ball's bounce sounds, and whether the last shot was a smash in a game of more than one player.
    bounces: sound::Bounces,
    smashed: bool,
    /// The flight whistle (`sound::FLIGHT`) is on, and how many have started (a new one restarts it).
    whistle: (bool, u32),
    /// The original's match camera, stepped with the simulation.
    cam: Camera,
    /// Its view one tick earlier (drawing blends the two); `cam_cut` makes the next step start from the new view.
    prev_view: View,
    cam_cut: bool,
    /// The human whose end the camera follows (the original turns round to stay behind its human).
    cam_owner: Option<usize>,
    /// Per player: the character's pelvis forward row per motion at its first frame (the body turn's input).
    pelvis: Vec<Vec<[f32; 2]>>,
    /// Each player's character number and data.
    chars: Vec<i32>,
    /// Each player's last shouts.
    voices: Vec<sound::Voice>,
    data: Vec<std::sync::Arc<CharacterData>>,
    /// The team that won the last point.
    post_winner: i32,
    /// The chair umpire, and her last voice's play id (0 none).
    umpire: Umpire,
    umpire_voice: u64,
    /// The court number (the gallery's banks and keys).
    stage: u8,
    /// The gallery, the errors in a row it has seen, whether the last point won a game, and its plays due (with
    /// their bearings).
    gallery: sound::Gallery,
    errors: u32,
    gallery_game: bool,
    cheers: Vec<(sound::Play, i32)>,
    /// The court's ambient sound emitters.
    emitters: Vec<npc::Emitter>,
    /// Which players are on a controller (the gallery favours their side).
    humans: Vec<bool>,
    /// The racket impact a shot started this tick.
    hit_effect: Option<effects::Hit>,
    /// Per player its character's smash top and window middle (TParam, m) and the landing markers' state.
    smash_heights: Vec<[f32; 2]>,
    marks: Marks,
    /// The jingle due (slot 8 key: 0 change ends, 1 game, 2 set, 3 match won by a player's side, 4 lost), and whether the BGM
    /// stays faded until the next point.
    jingle: Option<u8>,
    music_hold: bool,
}

/// The landing markers, as the original: the red one at the shot's aim (from the serve return on), the yellow one
/// where a receiving human can smash the ball, searched along a predicted path grown 15 entries a frame.
#[derive(Default)]
struct Marks {
    red: Option<[f32; 2]>,
    /// The smash search, its path so far (y up) and the predictor's flight at the path's end.
    smash: Option<(SmashSearch, Vec<PathEntry>, Flight)>,
    /// The search found its first point this frame (the yellow marker's play starts).
    start: bool,
    /// The path was built by a strike this frame: the original searches it but grows it only from the next frame.
    fresh: bool,
}

const PATH_MAX: usize = 600;

/// The predictor's path entry for `f` (the game stores it y up).
fn path_entry(f: &Flight) -> PathEntry {
    let (p, v) = (f.ball.pos, f.ball.vel);
    PathEntry {
        pos: [p[0], -p[1], p[2]],
        vel: [v[0], -v[1], v[2]],
        bounces: f.bounces,
    }
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
    /// The serve's error off its aim (mistimed swing, contact off the ideal height), set at contact.
    scatter: V3,
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
/// The ball model, or (`true`) its shadow.
#[derive(Component)]
struct BallView(bool);
#[derive(Component)]
struct ScoreText;
/// A balloon billboard over player `.0`'s head (its own material).
#[derive(Component)]
struct BalloonView(usize, Handle<StandardMaterial>);
/// The balloon textures by `Balloon` (bunny, turtle, note, sweet).
#[derive(Resource)]
struct BalloonArt([Handle<Image>; 4]);

pub fn plugin(app: &mut App) {
    app.add_plugins(panel::plugin);
    app.insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<Pads>()
        .init_resource::<CamMode>()
        .init_resource::<CamState>()
        .add_systems(PostStartup, setup) // after the court's game-space root exists
        .add_systems(
            Update,
            (
                read_input,
                camera,
                draw,
                effects::draw,
                effects::draw_sparks,
                effects::draw_trails,
                effects::draw_flight,
                effects::draw_bounce,
                effects::draw_marks,
                balloons,
                character::animate,
                hud,
            )
                .chain(),
        )
        .add_systems(
            FixedUpdate,
            (
                remember,
                effects::tick,
                control,
                simulate,
                start_effects,
                age_balloons,
                motions,
                character::tick,
                effects::tick_trails,
                played_out,
                held_ball,
                play_sounds,
            )
                .chain(),
        );
}

/// Character 0's trajectory tables `tr_pc00_<name><k>.dat`, k in 0..n, from one of its archives on the disc.
fn tables(iso: &mut Iso, archive: &str, name: &str, n: usize) -> Vec<Table> {
    character_tables(iso, 0, archive, name, "", n)
}

/// Character `c`'s trajectory tables `tr_pc<c>_<name><k><suffix>.dat`, k in 0..n, from its archive `TRAJ<c><ab>.XB`.
fn character_tables(
    iso: &mut Iso,
    c: usize,
    ab: &str,
    name: &str,
    suffix: &str,
    n: usize,
) -> Vec<Table> {
    let data = iso
        .read(&format!("TRAJ/TRAJ{c:02}{ab}.XB"))
        .expect("trajectory archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    (0..n)
        .map(|k| {
            let file = format!("tr_pc{c:02}_{name}{k}{suffix}.dat");
            let e = arc
                .entries
                .iter()
                .find(|e| e.name.to_ascii_lowercase().ends_with(&file))
                .expect("trajectory table");
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
    let data = iso
        .read("PCDATA/PCDATA.XB")
        .expect("character archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc
        .entries
        .iter()
        .find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv"))
        .expect("TParam.csv");
    let csv = arc.read(e).expect("TParam.csv bytes");
    let tag = format!("{n},");
    let row = csv
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(tag.as_bytes()))
        .expect("character row");
    row.split(|&b| b == b',')
        .map(|c| String::from_utf8_lossy(c).trim().to_string())
        .collect()
}

/// A character's movement stats from TParam.csv (SPE, Agili, STA, dive/backhand/smash stamina costs).
/// ponytail: clear weather (0); the match's weather table (rain/snow slow the acceleration) comes with P17/P21
fn character_stats(iso: &mut Iso, n: usize) -> Stats {
    let row = tparam(iso, n);
    let cell = |i: usize| row[i].parse::<i32>().expect("TParam stat");
    let costs: Vec<i32> = row[42]
        .split('/')
        .map(|v| v.parse().expect("TParam stamina cost"))
        .collect();
    Stats::new(
        cell(40),
        cell(43),
        cell(41),
        [costs[0], costs[1], costs[2]],
        0,
    )
}

/// A computer player's AIParam.csv row: character `n` in outfit 0, as an exhibition match picks it.
fn ai_params(iso: &mut Iso, n: usize, doubles: bool) -> hst_sim::ai::AiParams {
    let data = iso
        .read("PCDATA/PCDATA.XB")
        .expect("character archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc
        .entries
        .iter()
        .find(|e| e.name.to_ascii_lowercase().ends_with("aiparam.csv"))
        .expect("AIParam.csv");
    let row =
        hst_sim::ai::Choice::new(hst_sim::ai::menu_row(n as u8, 0) as u32, n as u8, doubles).row;
    hst_sim::ai::AiParams::table(&arc.read(e).expect("AIParam.csv bytes"))[row]
}

/// A character's hand from TParam.csv (+1 right, −1 left).
fn character_hand(iso: &mut Iso, n: usize) -> f32 {
    let data = iso
        .read("PCDATA/PCDATA.XB")
        .expect("character archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc
        .entries
        .iter()
        .find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv"))
        .expect("TParam.csv");
    let csv = arc.read(e).expect("TParam.csv bytes");
    let tag = format!("{n},");
    let row = csv
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(tag.as_bytes()));
    // 左 (left) in Shift-JIS
    if row
        .and_then(|r| r.split(|&b| b == b',').nth(6))
        .is_some_and(|c| c.starts_with(&[0x8d, 0xb6]))
    {
        -1.0
    } else {
        1.0
    }
}

/// Character 0's serve: heights from TParam.csv, the rest measured on character 0 (see `ServeData`).
fn serve_data(iso: &mut Iso) -> ServeData {
    let row = tparam_row(iso);
    let cm = |i: usize| -> [f32; 3] {
        let v: Vec<f32> = row[i]
            .split('/')
            .map(|v| v.parse::<f32>().expect("TParam height") / 100.0)
            .collect();
        [v[0], v[1], v[2]]
    };
    let cell = |i: usize| row[i].parse::<i32>().expect("TParam stat");
    // ponytail: timing and depth-bias tables (+0x1644/+0x1774, +0x1554/+0x1684), toss hand/apex drift (toss
    // animation), mistiming error (0x3fc760, skill level 0) and the serve angle (+0x130c) are character 0's
    // measured values; their sources are the motion data and the character tables (P3/P8)
    let grades = vec![
        0, 0, 4, 4, 2, 2, 2, 2, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 4, 4, 4,
    ];
    let bias = vec![
        -8, -6, -4, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 6, 8, 10, 12, 14, 16,
    ];
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
        strong_bias: bias.clone(),
        weak_bias: bias,
        reach: [cell(34), cell(35)],
        short_miss: row[54].parse().expect("TParam serve short-miss factor"),
        power: cell(12),
        low_power: cell(50),
        max_angle: 22.0,
    }
}

/// The timing balloons from the disc (`AZUMA/C_EFF/EFFCT.XB0`).
fn balloon_art(iso: &mut Iso, images: &mut Assets<Image>) -> [Handle<Image>; 4] {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let data = iso
        .read("AZUMA/C_EFF/EFFCT.XB0")
        .expect("effect archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    [
        Balloon::Bunny,
        Balloon::Turtle,
        Balloon::Note,
        Balloon::Sweet,
    ]
    .map(|b| {
        let e = arc
            .entries
            .iter()
            .find(|e| {
                e.name
                    .to_ascii_lowercase()
                    .ends_with(&b.texture().to_ascii_lowercase())
            })
            .expect("balloon texture");
        let pic = tim2::decode(&arc.read(e).expect("balloon bytes"))
            .expect("TIM2")
            .remove(0);
        images.add(Image::new(
            Extent3d {
                width: pic.width,
                height: pic.height,
                depth_or_array_layers: 1,
            },
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
/// …and the middle of its smash window (m).
fn reach(iso: &mut Iso) -> Reach {
    let data = iso
        .read("PCDATA/PCDATA.XB")
        .expect("character archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc
        .entries
        .iter()
        .find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv"))
        .expect("TParam.csv");
    let csv = arc.read(e).expect("TParam.csv bytes");
    // header cells hold quoted line breaks; character rows are plain and start with their number
    let row = csv
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(b"0,"))
        .expect("character 0 row");
    let cols: Vec<&[u8]> = row.split(|&b| b == b',').collect();
    let num = |i: usize| {
        std::str::from_utf8(cols[i])
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .expect("TParam number")
    };
    let pair = |i: usize, k: usize| -> f32 {
        let s = std::str::from_utf8(cols[i]).expect("TParam cell");
        s.split('/')
            .nth(k)
            .and_then(|v| v.trim().parse().ok())
            .expect("TParam slash cell")
    };
    let right = cols[6].starts_with(&[0x89, 0x45]); // 右 (right) in Shift-JIS
    let reach = Reach {
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
        grades: vec![
            0, 0, 4, 4, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        ],
        hand: if right { 1.0 } else { -1.0 },
        // ponytail: character 0's arm joints in the receive pose (shoulder, racket tip 0.7 m along the hand
        // joint); per-character from the skeleton with P8
        shoulder: [0.08548, -0.83622, 0.20118],
        tip: [0.006694, -0.646701, 1.220628],
    };
    reach
}

/// Character `n`'s smash top and the middle of its smash window (TParam column 64, m): the smash marker's heights.
fn smash_heights(iso: &mut Iso, n: usize) -> [f32; 2] {
    let data = iso
        .read("PCDATA/PCDATA.XB")
        .expect("character archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc
        .entries
        .iter()
        .find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv"))
        .expect("TParam.csv");
    let csv = arc.read(e).expect("TParam.csv bytes");
    let tag = format!("{n},");
    let row = csv
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(tag.as_bytes()))
        .expect("character row");
    let cell = row.split(|&b| b == b',').nth(64).expect("TParam smash cell");
    let v: Vec<f32> = std::str::from_utf8(cell)
        .expect("TParam cell")
        .split('/')
        .map(|v| v.trim().parse::<f32>().expect("TParam number") / 100.0)
        .collect();
    [v[0], v[1]]
}

/// Line margin, scoreboard timing, the stage's collision world with the material table, the shot parameter table,
/// and the umpire.
#[allow(clippy::type_complexity)]
fn disc(
    iso: &mut Iso,
    stage: Option<u32>,
) -> (
    f32,
    ScoreboardTiming,
    (World, Vec<Material>),
    ShotParams,
    Umpire,
) {
    let (cnf, bin) = (
        iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"),
        iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"),
    );
    let game = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
    let src = game.shot_params();
    let params = ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
    // ponytail: umpire 4 (Lily), voice set a, English; off a stage her chair and the collision world (net, posts,
    // fences) are court 10's
    let n = stage.unwrap_or(10);
    let chair = court::umpire_chair(iso, n).unwrap_or([6.5156, -1.7712, -0.0109]);
    let umpire = Umpire::new(
        chair,
        game.umpire_side(n as usize),
        n as u8,
        game.umpire_words(0, 4, 0),
        [0.0; 3],
    );
    (
        game.line_margin(),
        game.scoreboard_timing(0, 4),
        (court::world(iso, n), court::materials(&game)),
        params,
        umpire,
    )
}

/// Court `n`'s ambient sound emitters: the trigger creatures that only play sounds, each drawing its first gap.
// ponytail: they draw from our rng, not the game's shared MT (not ported to the app)
fn emitters(iso: &mut Iso, n: usize, players: u32, rng: &mut u32) -> Vec<npc::Emitter> {
    let Some((list, plants)) = crate::court_layout(iso, n) else {
        return Vec::new();
    };
    let (cnf, bin) = (
        iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"),
        iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"),
    );
    let game = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
    let mut roll = || {
        rand(rng);
        *rng
    };
    npc::spawn(
        &list,
        &plants,
        &game.npc_roster(n as u32),
        &game.walkers(n as u32),
        players,
    )
    .into_iter()
    .filter_map(|c| match c.kind {
        npc::Kind::Trigger(t) if npc::EMITTERS.contains(&t) => {
            let [x, y, z, _] = c.world[3];
            Some(npc::Emitter::new(t, game.emitter(t), [x, y, z], &mut roll))
        }
        _ => None,
    })
    .collect()
}

/// Character `c`'s serve tables and spins (see `Game::serve_tables`): the weak toss serves by the `dw1`
/// variant tables and records (every character's serve variants are weighted −0.5 on disc).
fn serve_tables(iso: &mut Iso, params: &ShotParams, c: usize) -> [Vec<(Table, f32)>; 2] {
    let r = params::record_of(c);
    let base = character_tables(iso, c, "A", "serv", "", 4)
        .into_iter()
        .enumerate()
        .map(|(k, t)| (t, params::spin(params.record(0, k, r))));
    let weak = character_tables(iso, c, "B", "serv", "_dw1", 3)
        .into_iter()
        .enumerate()
        .map(|(k, t)| (t, params::spin(&params.variant(0, k, r, -0.5))));
    [base.collect(), weak.collect()]
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
    let (line_margin, board, world, shot_params, umpire) = disc(&mut iso, args.stage);
    let rules = if args.singles { SINGLES } else { DOUBLES };
    let reach = reach(&mut iso);
    let mut game = Game {
        rules,
        tables: tables(&mut iso, "A", "strk", 5),
        serve_tables: Vec::new(),
        smash_tables: tables(&mut iso, "B", "smsh", 2),
        serve_data: serve_data(&mut iso),
        serving: Serving::default(),
        reach,
        flight: Flight::new(
            Ball {
                pos: [0.0; 3],
                vel: [0.0; 3],
                spin: 0.0,
            },
            [[0.0; 4]; 4],
            [[0.0; 4]; 4],
        ),
        shot: Shot::default(),
        prev_ball: [0.0; 3],
        players: vec![
            Player {
                stance: 3.0,
                hand: 1.0,
                ..Player::default()
            };
            rules.players as usize
        ],
        phase: Phase::Serve,
        last_hitter: -1,
        since_hit: 0,
        score: Score::new(),
        rally: Rally::default(),
        shots: 0,
        line_margin,
        post: None,
        post_press: false,
        board,
        world,
        court: args.court.min(COURTS.len() - 1),
        message: String::new(),
        rng: 0x2468_ace1,
        sounds: Vec::new(),
        whooshes: Vec::new(),
        bounces: default(),
        smashed: false,
        whistle: (false, 0),
        cam: Camera::new(),
        prev_view: Camera::new().view,
        cam_cut: false,
        cam_owner: Some(0),
        pelvis: vec![vec![[0.0, 1.0]; 48]; rules.players as usize],
        chars: vec![0; rules.players as usize],
        voices: vec![default(); rules.players as usize],
        data: Vec::new(),
        post_winner: 0,
        umpire,
        umpire_voice: 0,
        gallery: default(),
        errors: 0,
        gallery_game: false,
        cheers: Vec::new(),
        emitters: Vec::new(),
        humans: Vec::new(),
        jingle: None,
        music_hold: false,
        stage: args.stage.map_or(args.court, |s| s as usize) as u8,
        hit_effect: None,
        smash_heights: Vec::new(),
        marks: Marks::default(),
    };
    reset_positions(&mut game);
    game.emitters = emitters(
        &mut iso,
        args.stage.map_or(args.court, |s| s as usize),
        rules.players as u32,
        &mut game.rng,
    );
    let n = game.players.len();

    let Ok(root) = root.single() else { return };
    // the characters on court, from the disc (each loaded once)
    let mut voices = Vec::new();
    let mut loaded: std::collections::HashMap<usize, std::sync::Arc<CharacterData>> =
        Default::default();
    for i in 0..n {
        // default line-up: player 1 is Carol (character 6), then characters 1, 2, 3
        let c = args.chars.get(i).copied().unwrap_or([6, 1, 2, 3][i]);
        let data = match loaded.get(&c) {
            Some(d) => d.clone(),
            None => {
                let d = std::sync::Arc::new(
                    character::load_disc(
                        &mut iso,
                        c,
                        0,
                        &mut meshes,
                        &mut materials,
                        &mut images,
                        &mut bindposes,
                    )
                    .unwrap_or_else(|e| panic!("character {c}: {e}")),
                );
                loaded.insert(c, d.clone());
                d
            }
        };
        game.players[i].hand = character_hand(&mut iso, c);
        game.players[i].stats = character_stats(&mut iso, c);
        game.players[i].ai = ai_params(&mut iso, c, n == 4);
        game.players[i].body.stamina = game.players[i].stats.stamina;
        game.pelvis[i] = data.pelvis.clone();
        game.chars[i] = c as i32;
        game.smash_heights.push(smash_heights(&mut iso, c));
        game.serve_tables
            .push(serve_tables(&mut iso, &shot_params, c));
        // ponytail: the a/b voice pick is 70/30 at random; the game's rules for two players of one character are left out
        voices.push(voice_bank(&mut iso, c, n, rand(&mut game.rng) < 0.3).map(std::sync::Arc::new));
        game.data.push(data.clone());
        let f = character::spawn(&mut commands, &data, root);
        commands
            .entity(f)
            .insert((Figure(i), effects::SwingTrail::default()));
    }
    // her voice bank in slot 5 (`VoiceBanks` holds slots 1..)
    voices.resize(4, None);
    voices.push(umpire_bank(&mut iso, 4, 0).map(std::sync::Arc::new));
    // the jingles (slot 8: `jig_00` in a match) and, with `--music`, the court's BGM (`bgmg_NN`; `bgmg_14` off the
    // 11 courts)
    let stage = game.stage as usize;
    let bgm = if (1..=11).contains(&stage) {
        format!("bgmg_{stage:02}")
    } else {
        "bgmg_14".into()
    };
    commands.insert_resource(MusicBanks {
        jingles: SoundBank::load(&mut iso, "SND/JGL/JIG_00.XB", "data/sound/JINGLE/jig_00.hd")
            .map(std::sync::Arc::new),
        bgm: args
            .music
            .then(|| SoundBank::music(&mut iso, "Court", &bgm))
            .flatten()
            .map(|(b, mid)| (std::sync::Arc::new(b), mid)),
    });
    // ponytail: a stand-in figure for her (a capsule on the chair) until the umpire model is drawn
    let chair = game.umpire.pos;
    let coat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.85, 0.7),
        ..default()
    });
    let stand_in = commands
        .spawn((
            Mesh3d(meshes.add(Capsule3d::new(0.2, 0.8))),
            MeshMaterial3d(coat),
            Transform::from_xyz(chair[0], chair[1] - 0.6, chair[2]),
        ))
        .id();
    commands.entity(root).add_child(stand_in);
    commands.insert_resource(game);
    commands.insert_resource(VoiceBanks(voices));
    let impacts = effects::load(
        &mut iso,
        &mut commands,
        root,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut bindposes,
    )
    .expect("hit effects");
    commands.insert_resource(impacts);
    let sparks = effects::load_sparks(
        &mut iso,
        &mut commands,
        root,
        &mut meshes,
        &mut materials,
        &mut images,
    )
    .expect("hit sparks");
    commands.insert_resource(sparks);
    let trails = effects::load_trails(
        &mut iso,
        &mut commands,
        root,
        &mut meshes,
        &mut materials,
        &mut images,
    )
    .expect("swing trails");
    commands.insert_resource(trails);
    let flight = effects::load_flight(
        &mut iso,
        &mut commands,
        root,
        &mut meshes,
        &mut materials,
        &mut images,
    )
    .expect("ball flight");
    commands.insert_resource(flight);
    let stage = args.stage.map_or(args.court, |s| s as usize);
    let marks = effects::load_marks(
        &mut iso,
        &mut commands,
        root,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut bindposes,
    )
    .expect("landing markers");
    commands.insert_resource(marks);
    let bounce = effects::load_bounce(
        &mut iso,
        stage,
        &mut commands,
        root,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut bindposes,
    )
    .expect("bounce effects");
    commands.insert_resource(bounce);
    // the game's ball (`ball1.mdl`, radius 0.0325) and its shadow (`ballshadow.mdl`), drawn large, the ball with an
    // inverted hull (front faces culled) for the black outline
    let game_xb = iso.read("CMN/GAME.XB").expect("ball archive on disc");
    let mut part = |name: &str, view| {
        let e = commands
            .spawn((
                view,
                Transform::from_scale(Vec3::splat(BALL_DRAW_SCALE)),
                Visibility::default(),
            ))
            .id();
        for (_, parts) in crate::models(&game_xb, |n| n.ends_with(name), &mut images) {
            for (mesh, material) in parts {
                commands.entity(e).with_child((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(material)),
                ));
            }
        }
        commands.entity(root).add_child(e);
        e
    };
    let ball = part("ball1.mdl", BallView(false));
    part("ballshadow.mdl", BallView(true));
    let ink = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        cull_mode: Some(bevy::render::render_resource::Face::Front),
        ..default()
    });
    commands.entity(ball).with_child((
        Mesh3d(meshes.add(Sphere::new(0.0325 * 1.22).mesh().ico(4).unwrap())),
        MeshMaterial3d(ink),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 12.0, -6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        ScoreText,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
    // one balloon billboard per player (world space, turned to the camera every frame)
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    for i in 0..n {
        let m = materials.add(StandardMaterial {
            base_color_texture: Some(art[0].clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            BalloonView(i, m.clone()),
            Mesh3d(quad.clone()),
            MeshMaterial3d(m),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    commands.insert_resource(BalloonArt(art));
}

/// Put every player where the original does for the next serve (`serve_placement`) and the ball in the server's hand.
fn reset_positions(g: &mut Game) {
    for i in 0..g.players.len() {
        let (stance, hand, stats) = (g.players[i].stance, g.players[i].hand, g.players[i].stats);
        let at = serve_placement(
            i as i32,
            &g.score,
            g.rally.faults,
            stance,
            0,
            g.score.swapped,
        );
        let end = at.facing;
        // stamina is full again every point
        let body = loco::Body::new(at.pos, end, &stats);
        g.players[i] = Player {
            pos: at.pos,
            prev: at.pos,
            end,
            facing: base_yaw(end),
            prev_facing: base_yaw(end),
            stance,
            hand,
            home: at.pos,
            stats,
            body,
            ..Player::default()
        };
    }
    // with more than one human the camera keeps its end through changes of ends; only solo games turn it
    if g.humans.iter().filter(|&&h| h).count() <= 1 {
        g.cam.turned = g.cam_owner.is_some_and(|i| g.players[i].pos[2] > 0.0);
    }
    g.cam.cut();
    g.cam_cut = true;
    g.serving = Serving::default();
    let s = &g.players[g.score.server as usize];
    let ball = [s.pos[0] + 0.3, -1.0, s.pos[2]];
    g.flight = Flight::new(
        Ball {
            pos: ball,
            vel: [0.0; 3],
            spin: 0.0,
        },
        [[0.0; 4]; 4],
        [[0.0; 4]; 4],
    );
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

fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut pads: ResMut<Pads>,
    mut cam: ResMut<CamMode>,
    mut cs: ResMut<CamState>,
    time: Res<Time>,
) {
    let mut now = [SlotPad::default(); 2];
    for (k, d) in [
        (KeyCode::KeyW, Vec2::Y),
        (KeyCode::KeyS, -Vec2::Y),
        (KeyCode::KeyA, -Vec2::X),
        (KeyCode::KeyD, Vec2::X),
    ] {
        if keys.pressed(k) {
            now[0].stick += d;
        }
    }
    now[0].shot = [(KeyCode::KeyJ, 0), (KeyCode::KeyK, 1), (KeyCode::KeyL, 3)]
        .into_iter()
        .find(|(k, _)| keys.just_pressed(*k))
        .map(|(_, kind)| kind);
    now[0].serve = keys.just_pressed(KeyCode::Space);
    let mut cycle = keys.just_pressed(KeyCode::KeyC);
    let mut turn = keys.pressed(KeyCode::ArrowRight) as i32 as f32
        - keys.pressed(KeyCode::ArrowLeft) as i32 as f32;
    // controllers in connection order: the first is slot 1, the second slot 2
    // ponytail: Steam Input's virtual pads (Valve, 0x28de) mirror real ones; skip them so slot 2 is the second real pad
    let mut list: Vec<_> = gamepads
        .iter()
        .filter(|(_, g)| g.vendor_id() != Some(0x28de))
        .collect();
    list.sort_by_key(|(e, _)| *e);
    for (slot, (_, g)) in list.iter().take(2).enumerate() {
        let s = &mut now[slot];
        s.stick += deadzone(g.left_stick()) + g.dpad();
        let buttons = [
            (GamepadButton::South, 0),
            (GamepadButton::East, 1),
            (GamepadButton::North, 3),
        ];
        s.shot = s.shot.or(buttons
            .into_iter()
            .find(|(b, _)| g.just_pressed(*b))
            .map(|(_, k)| k));
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
        *cam = if *cam == CamMode::Original {
            CamMode::Free
        } else {
            CamMode::Original
        };
    }
    cs.turn = turn * time.delta_secs() * 1.5;
}

/// Launch a serve (class 0), stroke (class 1) or smash (class 3) of `kind` by player `who` from the ball's position toward
/// `target`, along the game's trajectory tables.
/// `branch`, `grade` and `offset` are the swing's (branch code, timing grade and offset) for the hit sounds.
fn strike(
    g: &mut Game,
    who: usize,
    class: u8,
    kind: i32,
    target: V3,
    (branch, grade, offset): (u8, u8, i32),
) {
    // one hit per team per ball, as the original: only the other team's ball can be struck (see `theirs`)
    debug_assert!(
        class == 0 || g.last_hitter < 0 || g.last_hitter & 1 != who as i32 & 1,
        "player {who} struck its own team's ball"
    );
    let at = g.flight.ball.pos;
    let weak = (g.serving.toss == Some(Toss::Weak) && serve::dw1(&g.serve_data)) as usize;
    let (vel, frames) = if class == 0 {
        let (table, _) = &g.serve_tables[who][weak][kind as usize];
        serve::launch(
            table,
            kind == 3,
            hst_sim::ball::Params::default().radius,
            at,
            target,
            g.serving.scatter,
        )
    } else {
        let l = if class == 3 {
            lookup(
                &g.smash_tables[kind as usize],
                &Bounds::smash(kind),
                at,
                target,
            )
        } else {
            lookup(
                &g.tables[kind as usize],
                &Bounds::stroke(kind, at[2]),
                at,
                target,
            )
        };
        (launch(at, target, l.elevation, l.speed), l.frames)
    };
    let dir = Vec3::new(vel[0], 0.0, vel[2]).normalize_or(Vec3::Z);
    let side = Vec3::Y.cross(dir).normalize();
    let frame = [
        side.to_array(),
        [0.0, 1.0, 0.0],
        side.cross(Vec3::Y).normalize().to_array(),
    ];
    g.shot = Shot {
        class,
        kind,
        curve_frames: frames + 1,
        ..Shot::default()
    };
    g.shots += 1;
    g.rally.on_hit(
        g.shots,
        who as i32,
        g.score.server,
        g.score.receiver,
        g.flight.contacts,
    );
    // ponytail: framed/dull mis-hits and the power gap are not modelled (P3), so their sounds never play
    let hit = sound::Hit {
        branch,
        grade,
        offset,
        kind,
        hits: g.shots,
        strong_toss: g.serving.toss == Some(Toss::Strong),
        solo: g.rules.players == 1,
        random_bit: rand(&mut g.rng) < 0.5,
        ..default()
    };
    g.sounds
        .extend(sound::hit_sounds(&hit).into_iter().map(|p| (p, at)));
    let n = g.players.len() as u32;
    if let Some(program) =
        sound::stroke_shout(&hit, g.chars[who], n, || (rand(&mut g.rng) * 100.0) as u32)
    {
        let r = (rand(&mut g.rng) * 32768.0) as u32;
        let shout = g.voices[who].shout(who, program, n, r);
        g.whooshes.push((0, who, shout));
    }
    g.smashed = branch == 4 && g.rules.players > 1;
    g.whistle = if kind == 3 {
        (true, g.whistle.1 + 1)
    } else {
        (false, g.whistle.1)
    };
    // every recorded smash spins 5°, △ smashes too (lob_smash_s05)
    let spin = match class {
        0 => g.serve_tables[who][weak][kind as usize].1,
        3 => 5f32.to_radians(),
        _ => KIND_SPIN[kind as usize],
    };
    g.flight = Flight::new(Ball { pos: at, vel, spin }, rows4(frame), rows4(frame));
    // ponytail: practice's (one player) side pick between candidates and its red-marker timeout aren't ported
    g.marks.red = (g.rules.players == 1 || g.shots > 1).then_some([target[0], target[2]]);
    // each receiving human with its own character's heights; not on the serve (the game sets the search up from
    // the serve return on, or in practice)
    let heights: Vec<[f32; 2]> = (0..g.players.len())
        .filter(|&i| i & 1 != who & 1 && g.humans.get(i) == Some(&true))
        .map(|i| g.smash_heights[i])
        .collect();
    g.marks.smash = (!heights.is_empty() && (g.rules.players == 1 || g.shots > 1)).then(|| {
        let mut f = g.flight;
        let mut path = vec![path_entry(&f)];
        for _ in 0..14 {
            f.step(&g.shot, &COURTS[g.court]);
            path.push(path_entry(&f));
        }
        (SmashSearch::new(&heights), path, f)
    });
    g.marks.fresh = true;
    g.hit_effect = Some(effects::Hit {
        kind,
        smash: class == 3,
        pos: at,
        vel,
    });
    let hitter_far = g.players[who].end < 0.0;
    g.flight.lines = Some(Lines {
        shots: g.shots,
        doubles: g.rules.players > 2,
        side: g.score.side,
        hitter_far,
        margin: g.line_margin,
    });
    g.prev_ball = at;
    g.last_hitter = who as i32;
    g.since_hit = 0;
    g.phase = Phase::Rally;
    g.umpire.rally();
}

/// The server's turn while the serve is set up, as the original: before the toss the server stands or walks
/// along the baseline (stick mostly sideways); a shot button tosses (topspin: strong toss, lob: underhand,
/// others: weak toss) and the ball leaves the hand on the toss animation's release frame; a press while it is
/// in the air (topspin/slice, or lob for an underhand toss) locks the swing onto the contact frame the serve
/// search finds — the timing grade there decides the balloon and, for a strong toss, how far the aim is thrown
/// off. No contact in reach is a whiff; a toss that lands is simply tossed again.
fn serve_turn(g: &mut Game, i: usize, stick: Vec2, press: Option<i32>) {
    let (pos, end) = (g.players[i].pos, g.players[i].end);
    // standing still unless this tick walks
    g.players[i].prev = pos;
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
            let far = end
                * court
                * if g.rules.players > 2 {
                    serve::WALK_MAX_DOUBLES
                } else {
                    serve::WALK_MAX_SINGLES
                };
            let x = pos[0] + serve::WALK * stick.x.signum();
            let p = &mut g.players[i];
            p.pos[0] = if far > 0.0 {
                x.clamp(serve::WALK_MIN, far)
            } else {
                x.clamp(far, -serve::WALK_MIN)
            };
            p.stride += serve::WALK * 9.0;
        }
        s.t += 1;
        g.serving = s;
        hold_ball(g);
        g.players[i].serve_anim = Some(0.0);
        return;
    };
    s.t += 1;
    if !s.tossed {
        g.serving = s;
        if s.t < serve::TOSS_RELEASE {
            hold_ball(g);
        } else {
            let (hand, apex) = serve::toss_points(&g.serve_data, toss, pos, end);
            g.flight = Flight::new(
                Ball {
                    pos: hand,
                    vel: serve::toss_velocity(hand, apex),
                    spin: 0.1,
                },
                rows4([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
                rows4([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
            );
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
                g.players[i].missed = false;
                let grades = g.serve_data.grades(toss).to_vec();
                match serve::search(&g.serve_data, toss, &predicted_path(g, grades.len())) {
                    Some(k) => {
                        let kind = if toss == Toss::Under {
                            3
                        } else {
                            press.unwrap_or(0)
                        };
                        s.swing = Some(ServeSwing {
                            left: k as u32,
                            frames: k as u32,
                            offset: k as i32 - SWEET_FRAME,
                            grade: grades[k],
                            kind,
                        });
                        whoosh(g, i, 0, kind);
                    }
                    None => {
                        // a missed serve swing shouts with its miss motion, never muted by an earlier whiff
                        s.whiffed = true;
                        let r = (rand(&mut g.rng) * 32768.0) as u32;
                        let shout =
                            g.voices[i].shout(i, sound::WHIFF_SHOUT, g.players.len() as u32, r);
                        g.whooshes.push((0, i, shout));
                    }
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
        let aim = if pads_aim(g, i) {
            stick
        } else {
            g.serving.bot_aim
        };
        let coins = [(); 3].map(|_| rand(&mut g.rng) < 0.5);
        let d = &g.serve_data;
        let (target, miss) = serve::target(
            d,
            toss,
            sw.offset,
            sw.grade,
            pos,
            end,
            g.score.side,
            g.rules.players > 2,
            [aim.x, aim.y],
            coins,
        );
        let hit = g.flight.ball.pos;
        let error = serve::depth_error(
            d,
            toss,
            (sw.offset + SWEET_FRAME) as usize,
            sw.grade,
            -hit[1],
        );
        g.serving.scatter = serve::scatter(d, toss, miss, error, hit, target);
        // the stick toward the net turns a topspin serve flat
        let sw = ServeSwing {
            kind: hst_sim::shot::stick_kind(0, sw.kind, [aim.x, aim.y], end),
            ..sw
        };
        debug!(
            "player {i} serve {toss:?} kind {}: offset {} grade {} -> {target:?}",
            sw.kind, sw.offset, sw.grade
        );
        g.players[i].balloon = serve::balloon(sw.grade, sw.offset, false).map(|b| (b, 0));
        g.players[i].serve_anim = None;
        g.players[i].stance = pos[0].abs();
        strike(g, i, 0, sw.kind, target, (0, sw.grade, sw.offset));
        g.serving = Serving::default();
        return;
    }
    // a toss that comes down untouched (or after a whiff) is tossed again
    if g.flight.bounces > 0 {
        g.serving = Serving {
            t: 0,
            ..Serving::default()
        };
        hold_ball(g);
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

/// The ball is held: still, and placed on the server's motion each tick (`held_ball`).
fn hold_ball(g: &mut Game) {
    g.flight = Flight::new(
        Ball {
            pos: g.flight.ball.pos,
            vel: [0.0; 3],
            spin: 0.0,
        },
        [[0.0; 4]; 4],
        [[0.0; 4]; 4],
    );
}

/// Where the server holds the ball before the toss leaves the hand, as the original: in the stance (0x20) on
/// the stance's ball track (`*_serve_ad00_ball`), on the walk and through the toss at the left hand's
/// `Bip01LFinger21`, both at the motion's sampled time.
/// ponytail: posed from the motion's own clip; the original's pose mid-crossfade is the mixed one.
fn held_ball(mut g: ResMut<Game>, q: Query<(&Figure, &Motion)>) {
    if g.phase != Phase::Serve || g.serving.tossed {
        return;
    }
    let i = g.score.server as usize;
    let Some((_, m)) = q.iter().find(|(f, _)| f.0 == i) else {
        return;
    };
    let (p, data) = (&g.players[i], &g.data[i]);
    // the player's matrix: a turn, never mirrored (left-handers too)
    let [fx, _, fz, _] = p.body.face.dir;
    let rows = [
        [fz, 0.0, -fx, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [fx, 0.0, fz, 0.0],
    ];
    let t = m.clock.sampled;
    let sk = &data.skeleton;
    let at = if m.id == 0x20 {
        data.stance_ball
            .as_ref()
            .map(|b| serve::stance_ball(b.at(t), rows, p.pos))
    } else {
        let finger = sk.names.iter().position(|n| n == "Bip01LFinger21");
        data.motions.get(&m.id).zip(finger).map(|(c, f)| {
            let player = [
                rows[0],
                rows[1],
                rows[2],
                [p.pos[0], p.pos[1], p.pos[2], 1.0],
            ];
            let [x, y, z, _] = hst_sim::vu0::transform(
                &hst_sim::pose::node_world(sk, &c.locals(sk, t), f, &player),
                serve::HAND_BALL,
            );
            [x, y, z]
        })
    };
    if let Some(at) = at {
        g.prev_ball = g.flight.ball.pos;
        g.flight.ball.pos = at;
    }
}

/// A stick direction on screen as a direction on the court (x, z): right along the camera's right, up along its
/// forward.
fn screen(g: &Game, stick: Vec2) -> Vec2 {
    let r = g.cam.view.rot;
    let right = Vec2::new(r[0][0], r[0][2]).normalize_or(Vec2::X);
    let up = Vec2::new(r[2][0], r[2][2]).normalize_or(Vec2::Y);
    right * stick.x + up * stick.y
}

/// The run direction the original gives a human's stick (`hst_sim::player::pad_dir`): the stick as pad bytes, its
/// dead square, rescale and ×1.2 clip, on world axes, turned by π when the camera looks from the +z side.
/// ponytail: the app's stick already merges d-pad and keys into one vector, so the d-pad's own (faster diagonal)
/// path isn't taken; world axes also under the free camera
fn pad_run(g: &Game, stick: Vec2) -> Vec2 {
    // 0x00 full left/up, 0x80 centre, 0xff full right/down
    let byte =
        |v: f32| (128.0 + v.clamp(-1.0, 1.0) * if v < 0.0 { 128.0 } else { 127.0 }).round() as u8;
    let phase = if matches!(g.phase, Phase::ChangeEnds(_)) {
        1
    } else {
        3
    };
    let [x, z] = loco::pad_dir(0, byte(stick.x), byte(-stick.y), phase);
    if g.cam.view.eye[2] >= 0.0 {
        Vec2::new(-x, -z)
    } else {
        Vec2::new(x, z)
    }
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
        path.push(PathPoint {
            pos: f.ball.pos,
            bounces: f.bounces,
        });
        f.step(&g.shot, &COURTS[g.court]);
    }
    path
}

/// Whether `i`'s ball is the other team's, in play, and not already taken by `i`'s partner: the original never
/// has both teammates locked onto a ball (match_s05, lob_smash_s05), so a teammate's lock leaves `i` nothing to
/// hit, not a second stroke that would lose the point as an illegal hit.
fn theirs(g: &Game, i: usize) -> bool {
    let mate = g.players.get(i ^ 2).filter(|_| g.players.len() == 4);
    g.phase == Phase::Rally
        && g.last_hitter >= 0
        && g.last_hitter & 1 != i as i32 & 1
        && mate.is_none_or(|m| m.contact.is_none() && m.dive.is_none())
}

/// The original's contact search for player `i` (smash, volley, ground stroke), only while the other team's
/// ball is in play.
fn find_contact(g: &Game, i: usize) -> Option<Contact> {
    if !theirs(g, i) {
        return None;
    }
    let p = &g.players[i];
    let path = predicted_path(g, g.reach.grades.len());
    let s = swing::search(&g.reach, &path, p.pos, p.end, p.kind)?;
    Some(Contact {
        frames: s.frame as u32,
        swing: s,
        grade: g.reach.grades[s.frame],
        offset: s.frame as i32 - SWEET_FRAME,
    })
}

/// The search's last branch: a running player whose press finds no stroke dives toward the ball.
fn find_dive(g: &Game, i: usize) -> Option<swing::Dive> {
    let p = &g.players[i];
    // ponytail: this frame's run stands in for the game's previous-frame locomotion state
    if !theirs(g, i) || !p.body.running {
        return None;
    }
    let path = predicted_path(g, swing::DIVE_HORIZON);
    let f = p.body.face.dir;
    swing::dive(
        &g.reach,
        &path,
        p.pos,
        p.end,
        [f[0], f[2]],
        [p.body.vel[0], p.body.vel[2]],
    )
}

/// The game's contact-search branch number (+0x3ec1).
fn branch_code(b: swing::Branch) -> u8 {
    match b {
        swing::Branch::Ground => 1,
        swing::Branch::Volley => 2,
        swing::Branch::Smash => 4,
    }
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
            Phase::Post => 4,
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
    // reacting to the point: the reaction plays, nobody runs
    if g.post.as_ref().is_some_and(|p| p.reacted) {
        let (p, data) = (&mut g.players[i], &g.data[i]);
        p.prev = p.pos;
        if let Some(r) = p.root.as_mut()
            && let Some(path) = data.paths.get(&r.motion)
        {
            // the motion's time, held at its end
            let t =
                r.t.min(data.motions.get(&r.motion).map_or(0.0, |c| c.length));
            r.t += 1.0;
            let [fx, _, fz, _] = p.body.face.dir;
            let rows = [
                [p.hand * fz, 0.0, -(p.hand * fx), 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [fx, 0.0, fz, 0.0],
            ];
            let acc = motion::reaction_root(
                path.at(t),
                r.motion >= 0x30,
                g.chars[i],
                rows,
                r.base,
                r.acc,
            );
            // ponytail: moved straight by the change; the game's mover (0x34a960) also checks collisions
            p.pos[0] += acc[0] - r.acc[0];
            p.pos[2] += acc[2] - r.acc[2];
            r.acc = acc;
            if std::env::var("HST_DBG").is_ok() {
                eprintln!("DBG root p{i} m{:#x} t{} pos {:?}", r.motion, r.t, p.pos);
            }
        }
        return;
    }
    let Game {
        players, pelvis, ..
    } = g;
    let p = &mut players[i];
    p.prev = p.pos;
    p.body.pos = p.pos;
    p.body.step(&p.stats, [dir.x, dir.y], &sc, &pelvis[i]);
    p.pos = p.body.pos;
    p.vel = if p.body.running {
        Vec2::new(p.body.vel[0], p.body.vel[2])
    } else {
        Vec2::ZERO
    };
    p.stride += p.vel.length() * 9.0;
    p.facing = yaw(p.body.face.dir);
    let m = p.body.motion;
    set_motion(p, m, 1.0, true, None);
}

/// Figure yaw (0 faces −z) of a game-space facing direction.
fn yaw(d: [f32; 4]) -> f32 {
    (-d[0]).atan2(-d[2])
}

/// A stroke squares the body up to the other end (the game's stroke mode sets facing and target forward).
fn square_up(p: &mut Player) {
    let dir = [0.0, 0.0, p.end, 0.0];
    p.body.target = dir;
    p.body.face = loco::Facing {
        dir,
        way: -1,
        ..loco::Facing::default()
    };
    p.body.running = false;
    p.facing = base_yaw(p.end);
}

/// One frame of a player's stroke: a pending press keeps searching for a contact (pressing early grades
/// QUICK, late SLOW). Once locked the body squares up to the net and stands, taking its small step into the
/// shot over the last frames before contact, as the original; the racket meets the ball on the chosen frame.
fn advance_stroke(g: &mut Game, i: usize, aim: impl Fn(&mut Game) -> Vec2) -> Option<Contact> {
    let mut struck = None;
    if let Some(f) = g.players[i].follow.take() {
        set_motion(
            &mut g.players[i],
            f,
            1.0,
            false,
            Some(motion::SOFT_FOLLOW_HOLD),
        );
    }
    if let Some(left) = g.players[i].pending {
        if let Some(c) = find_contact(g, i) {
            let p = &mut g.players[i];
            p.pending = None;
            p.contact = Some(c);
            p.hit_at = Some(c.swing.ball);
            p.wind = c.frames.max(1);
            let (m, speed, wait) = motion::stroke_start(
                branch_code(c.swing.branch),
                c.frames as i32,
                c.swing.anim as i32,
            );
            set_motion(p, m, speed, wait.is_some(), None);
            p.wait_swing = wait;
            p.backhand = !c.swing.forehand;
            p.swing = Some(0);
            p.swung = false;
            p.vel = Vec2::ZERO;
            square_up(p);
            let kind = p.kind;
            whoosh(g, i, branch_code(c.swing.branch), kind);
            // the arm reaches for the ball: strokes 0x10–0x19 and volleys 0x1a/0x1b (higher motions: the game indexes
            // past its table; serves step 0, so no IK)
            let a = c.swing.anim as usize;
            if let (Some(t), true) = (
                g.data.get(i).and_then(|d| d.arm.clone()),
                (0x10..0x1c).contains(&a),
            ) {
                let p = &mut g.players[i];
                let b = c.swing.ball;
                let pos = [p.pos[0], p.pos[1], p.pos[2], 1.0];
                // the side scale is +1 on the −z end (`end`); the second size factor is 1 in normal play
                let solve =
                    contact_solve(&t, a - 0x10, [b[0], -b[1], b[2], 1.0], pos, [p.end, 1.0]);
                p.ik = Some(ArmIk::new(solve, c.frames as i32, pos));
            }
        } else if let Some(d) = find_dive(g, i) {
            let p = &mut g.players[i];
            debug!(
                "player {i} dives {} frames toward the ball ({})",
                d.frame,
                if d.contact {
                    "reaching it"
                } else {
                    "short of it"
                }
            );
            p.pending = None;
            p.dive = Some(d);
            // ponytail: clear weather (see Stats), so the thud's key is 0
            let thud = sound::dive_thud(0);
            g.whooshes.extend([
                (0, i, thud),
                (sound::dive_echo(g.players.len() as u32), i, thud),
            ]);
            let r = (rand(&mut g.rng) * 32768.0) as u32;
            let shout = g.voices[i].shout(i, sound::DIVE_SHOUT, g.players.len() as u32, r);
            g.whooshes.push((0, i, shout));
            let p = &mut g.players[i];
            let dir = [d.dir[0], 0.0, d.dir[1], 0.0];
            p.body.target = dir;
            p.body.face = loco::Facing {
                dir,
                ..loco::Facing::default()
            };
            p.body.running = false;
            p.facing = yaw(dir);
            p.vel = Vec2::ZERO;
            set_motion(p, 0x1e, 1.0, false, None);
        } else {
            g.players[i].pending = left.checked_sub(1);
            if left == 0 {
                let quiet = g.players[i].whiff_quiet;
                whiff(g, i, true, quiet);
            }
        }
    }
    dive_frame(g, i, &aim);
    // the contact IK's frame: the body steps into the shot through the mover, the arm turns by its weight
    let mate = (g.players.len() == 4).then(|| g.players[i ^ 2].pos);
    let p = &mut g.players[i];
    p.arm = None;
    if let Some(ik) = p.ik.as_mut() {
        let mut pos = [p.pos[0], p.pos[1], p.pos[2], 1.0];
        let end = p.end;
        // ponytail: the mover's partner check is skipped for the server in the original (game +0x1fc); kept here
        let (w, unit) = ik.tick(&mut pos, |at, d| {
            let to = loco::mover([at[0], at[1], at[2]], [d[0], d[1], d[2]], end, mate, false);
            let free =
                to[0] == hst_sim::ps2::add(at[0], d[0]) && to[2] == hst_sim::ps2::add(at[2], d[2]);
            ([to[0], to[1], to[2], at[3]], free)
        });
        p.arm = Some((ik.solve.quats, w));
        if !ik.on {
            p.ik = None;
        }
        // the swing reaches its contact pose at speed 1
        if unit {
            p.cmd.speed = 1.0;
        }
        p.prev = p.pos;
        p.pos = [pos[0], pos[1], pos[2]];
    }
    if let Some(c) = g.players[i].contact {
        let p = &mut g.players[i];
        if p.ik.is_none() {
            p.prev = p.pos;
        }
        if c.frames == 0 {
            let (stick, end) = (aim(g), g.players[i].end);
            let branch = branch_code(c.swing.branch);
            let kind =
                hst_sim::shot::stick_kind(branch, g.players[i].kind, [stick.x, stick.y], end);
            let target = aim_target(g, stick, end);
            // a smash has its own class and kinds: △ (the lob button) smashes kind 1, the others kind 0
            let (class, kind) = if branch == 4 {
                (3, (kind == 3) as i32)
            } else {
                (1, kind)
            };
            strike(
                g,
                i,
                class,
                kind,
                target,
                (branch_code(c.swing.branch), c.grade, c.offset),
            );
            debug!(
                "player {i} {:?} {} anim {:#x}: {} (offset {}, grade {})",
                c.swing.branch,
                if c.swing.forehand {
                    "forehand"
                } else {
                    "backhand"
                },
                c.swing.anim,
                timing_word(&c),
                c.offset,
                c.grade
            );
            let rally = g.phase == Phase::Rally && g.players.len() > 1;
            let v = g.flight.ball.vel;
            let p = &mut g.players[i];
            let branch = branch_code(c.swing.branch);
            if rally {
                p.body.stamina =
                    loco::stroke_stamina(&p.stats, p.body.stamina, branch, c.swing.forehand, 0);
            }
            // a slow ball off a ground stroke: the soft follow-through from next frame (`forehand` is already
            // mirrored for left-handers)
            p.follow = motion::soft_follow(
                branch,
                c.swing.anim as i32,
                v,
                if c.swing.forehand { 1 } else { 2 },
                1.0,
            );
            p.contact = None;
            p.swung = true;
            p.after = Some(0);
            p.recover = motion::recovery(branch, kind);
            p.balloon = serve::balloon(c.grade, c.offset, false).map(|b| (b, 0));
            struck = Some(c);
        } else {
            p.contact = Some(Contact {
                frames: c.frames - 1,
                ..c
            });
            // the swing waiting behind the body's turn starts 8 frames before contact
            if c.frames as i32 - 1 == motion::SWING_LEAD {
                if let Some(anim) = p.wait_swing.take() {
                    set_motion(p, anim, 1.0, false, None);
                }
            }
        }
    }
    // animation clock: contact at 45% of the swing, follow-through over the remaining frames
    let p = &mut g.players[i];
    if let Some(f) = p.swing {
        p.swing = Some(f + 1);
    }
    struck
}

/// One frame of a dive: the slide and the receive motion's root path through the mover (its motion held on its
/// 4th frame through the slide); a dive that reaches the ball volleys it on its frame.
fn dive_frame(g: &mut Game, i: usize, aim: &impl Fn(&mut Game) -> Vec2) {
    let Some(mut d) = g.players[i].dive else {
        return;
    };
    let mate = (g.players.len() == 4).then(|| g.players[i ^ 2].pos);
    let Game { players, data, .. } = g;
    let (p, root) = (
        &mut players[i],
        data.get(i).and_then(|d| d.paths.get(&0x1e)),
    );
    let n = d.tick;
    p.cmd.speed = if (4..d.frame.saturating_sub(1)).contains(&n) {
        0.0
    } else {
        1.0
    };
    p.prev = p.pos;
    let (pos, end) = (p.pos, p.end);
    let at = d.step(
        [pos[0], pos[2]],
        |t| root.map_or(0.0, |r| r.at(t)[2]),
        |m| {
            let q = loco::mover(pos, [m[0], 0.0, m[1]], end, mate, false);
            [q[0], q[2]]
        },
    );
    p.dive = at.map(|_| d);
    if let Some(q) = at {
        (p.pos[0], p.pos[2]) = (q[0], q[1]);
    }
    if d.contact && n == d.frame {
        let stick = aim(g);
        let kind = hst_sim::shot::stick_kind(3, g.players[i].kind, [stick.x, stick.y], end);
        let target = aim_target(g, stick, end);
        let offset = d.frame as i32 - SWEET_FRAME;
        strike(
            g,
            i,
            2,
            kind,
            target,
            (3, if offset.abs() < 2 { 2 } else { 4 }, offset),
        );
        if g.phase == Phase::Rally && g.players.len() > 1 {
            let p = &mut g.players[i];
            p.body.stamina = loco::stroke_stamina(&p.stats, p.body.stamina, 3, true, 0);
        }
    }
}

/// One frame of the follow-through after contact: past its recovery a stick or press (`input`) breaks it off, else
/// it plays to the end of its motion; either way the swing is over and the player stands, runs or swings again.
fn follow_through(p: &mut Player, input: bool) {
    let Some(a) = p.after.map(|a| a + 1) else {
        return;
    };
    p.after = Some(a);
    p.prev = p.pos;
    if motion::follow_over(a, p.recover, p.played, input) {
        debug!(
            "follow-through of motion {:#x} over {a} frames after contact ({})",
            p.cmd.id,
            if p.played { "played out" } else { "broken off" }
        );
        (p.after, p.swing, p.hit_at) = (None, None, None);
    }
}

/// Whether each player's motion has played to its end, as of this tick (the game's end-of-motion test).
fn played_out(mut g: ResMut<Game>, q: Query<(&Figure, &Motion, &character::Rig)>) {
    for (f, m, rig) in &q {
        if let Some(p) = g.players.get_mut(f.0) {
            p.played = m.serial == p.cmd.serial
                && rig
                    .data
                    .motions
                    .get(&m.id)
                    .is_some_and(|c| m.clock.done(c.length));
        }
    }
}

/// A shot button press: remembered for a while and checked every frame until a contact is found. With no
/// ball for this player to hit (or none found in time) the player swings at nothing, as the original.
/// ponytail: the original holds an early press only while its auto-approach (P7) finds a reachable ball ahead;
/// the fixed PRESS_FRAMES window stands in, so an early whiff comes up to 28 frames late.
/// A whiffing player takes a new press only past its re-press lock (quietly) or its recovery; nobody's press counts
/// from the point's reactions until the next serve. A press after the dead ball but before the reactions still
/// searches the last hitter's ball, so it whiffs with its miss motion.
fn press(g: &mut Game, i: usize, kind: i32) {
    if input_off(g) {
        return;
    }
    let p = &mut g.players[i];
    let Some(quiet) = p.whiff.map_or(Some(false), |w| w.press()) else {
        return;
    };
    if p.contact.is_none() && p.swing.is_none() && p.pending.is_none() && p.dive.is_none() {
        let quiet = quiet || p.missed;
        p.missed = false;
        p.whiff = None;
        p.kind = kind;
        p.whiff_quiet = quiet;
        let theirs = g.last_hitter >= 0 && g.last_hitter & 1 != i as i32 & 1;
        if theirs && g.phase == Phase::Rally {
            g.players[i].pending = Some(PRESS_FRAMES);
        } else {
            whiff(g, i, theirs, quiet);
        }
    }
}

/// Whether presses are off: from the point's reactions until the next serve.
fn input_off(g: &Game) -> bool {
    matches!(g.phase, Phase::ChangeEnds(_)) || g.post.as_ref().is_some_and(|p| p.reacted)
}

/// Swing at nothing. With no ball for the player (`!miss`) it is the pressed shot type's ground stroke; else a
/// smash when the ball is close and rises over the smash window's middle, or the ground stroke on the side the
/// ball passes (the side of the ball's line the player stands on). At full speed, squared up to the net.
fn whiff(g: &mut Game, i: usize, miss: bool, quiet: bool) {
    let path = if miss {
        predicted_path(g, g.reach.grades.len())
    } else {
        Vec::new()
    };
    // ponytail: the middle height is TParam's middle smash cell, the window's midpoint for every character seen
    let middle = (g.reach.smash_top + g.reach.smash_bottom) / 2.0;
    let (b, p) = (g.flight.ball, &mut g.players[i]);
    let base = match p.kind {
        1 => 0x12,
        3 => 0x14,
        _ => 0x10,
    };
    let d = [p.pos[0] - b.pos[0], p.pos[2] - b.pos[2]];
    let right = d[1] * b.vel[0] - d[0] * b.vel[2] > 0.0;
    let other = if right { p.hand >= 0.0 } else { p.hand < 0.0 };
    let anim = if !miss {
        base
    } else if swing::smash_whiff(&path, b.pos, p.pos, middle) {
        0x1f
    } else {
        base + other as i32
    };
    p.whiff = Some(motion::Whiff::new(anim, miss, quiet));
    set_motion(p, anim, 1.0, false, None);
    p.vel = Vec2::ZERO;
    p.facing = base_yaw(p.end);
}

/// One frame of a whiff, before this frame's press: at the contact pose the swing turns into its miss motion
/// with a shout, and once recovered a stick (`stick`) or the motion's end frees the player.
fn whiff_frame(g: &mut Game, i: usize, stick: bool) {
    let p = &mut g.players[i];
    let Some(mut w) = p.whiff else { return };
    p.prev = p.pos;
    if let Some(m) = w.step() {
        set_motion(p, m, 1.0, false, None);
        p.missed = true;
        if !w.quiet {
            let r = (rand(&mut g.rng) * 32768.0) as u32;
            let shout = g.voices[i].shout(i, sound::WHIFF_SHOUT, g.players.len() as u32, r);
            g.whooshes.push((0, i, shout));
        }
    }
    let p = &mut g.players[i];
    p.whiff = (!w.over(stick, p.played)).then_some(w);
}

/// Every player's turn this frame: humans from their controller slot, the rest from the stand-in AI.
fn control(mut g: ResMut<Game>, mut pads: ResMut<Pads>) {
    let g = &mut *g;
    g.humans = (0..g.players.len())
        .map(|i| pads.slot_of(i, g.players.len()).is_some())
        .collect();
    for i in 0..g.players.len() {
        match pads.slot_of(i, g.players.len()) {
            Some(s) => {
                let pad = &mut pads.slots[s];
                let (shot, serve_press) = (pad.shot.take(), std::mem::take(&mut pad.serve));
                post_press(g, shot.is_some() || serve_press);
                human(g, i, pad, shot, serve_press);
            }
            None => bot(g, i),
        }
    }
    g.cam_owner = (0..g.players.len()).find(|&i| pads.slot_of(i, g.players.len()).is_some());
    let server = g.score.server as usize;
    if g.phase == Phase::Serve && g.message.is_empty() {
        g.message = match pads.slot_of(server, g.players.len()) {
            Some(s) => format!(
                "Player {} serve: walk the baseline, toss with J/A (strong), K/B (weak) or L/Y (underhand), hit at the top with J/A or K/B (L/Y underhand)",
                s + 1
            ),
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
    follow_through(&mut g.players[i], shot.is_some() || pad.stick != Vec2::ZERO);
    whiff_frame(g, i, pad.stick != Vec2::ZERO);
    if let Some(kind) = shot {
        press(g, i, kind);
    }
    // screen-relative: stick right follows the camera's right, stick up its ground-forward; nothing moves the
    // player (or sets its motion) through the swing
    let p = &g.players[i];
    if p.contact.is_none() && p.whiff.is_none() && p.swing.is_none() && p.dive.is_none() {
        let dir = pad_run(g, pad.stick);
        locomote(g, i, dir);
    }
    // the stick at the moment of contact aims the shot
    let aim = screen(g, pad.stick);
    advance_stroke(g, i, move |_| aim);
}

/// Where the ball will be hittable on player `i`'s half: step a copy of the flight until it is waist-high after
/// its bounce.
fn intercept(g: &Game, i: usize) -> Option<(V3, u32)> {
    let half = -g.players[i].end;
    let mut f = g.flight;
    for n in 0..240 {
        f.step(&g.shot, &COURTS[g.court]);
        let b = f.ball;
        if b.pos[2] * half > 0.0
            && f.bounces >= 1
            && b.vel[1] > 0.0
            && (-1.3..-0.5).contains(&b.pos[1])
        {
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
const BOT_STROKE_TIMING: [(i32, u32); 17] = [
    (-6, 52),
    (-5, 16),
    (-4, 4),
    (-3, 4),
    (-2, 4),
    (-1, 9),
    (0, 7),
    (1, 4),
    (2, 9),
    (3, 4),
    (4, 4),
    (5, 3),
    (6, 4),
    (7, 1),
    (9, 2),
    (10, 2),
    (15, 2),
];
const BOT_SERVE_TIMING: [(i32, u32); 7] = [
    (-6, 9),
    (-5, 2),
    (-4, 1),
    (-3, 1),
    (-2, 1),
    (-1, 1),
    (0, 20),
];

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
    // ponytail: the stand-in AI always wants to move on, so it breaks off at the recovery
    follow_through(&mut g.players[i], true);
    whiff_frame(g, i, true);
    let busy = g.players[i].contact.is_some()
        || g.players[i].pending.is_some()
        || g.players[i].swing.is_some()
        || g.players[i].whiff.is_some()
        || g.players[i].dive.is_some();
    if !busy {
        let plan = match g.phase {
            Phase::Rally if g.last_hitter >= 0 && g.last_hitter & 1 != i as i32 & 1 => {
                intercept(g, i)
            }
            _ => None,
        };
        // ponytail: in doubles each teammate takes the balls on its side of the court (the nearer by distance left
        // the one at the net standing all rally); the original's partner logic is P0c/P11
        let mine = plan.filter(|(b, _)| {
            let d = |j: usize| (b[0] - g.players[j].pos[0]).abs();
            let mate = i ^ 2;
            mate >= g.players.len() || d(i) < d(mate) || (d(i) == d(mate) && i < mate)
        });
        // ponytail: the original decides a few frames into the shot (its AI, P11); this calls at once
        if plan.is_some()
            && mine.is_none()
            && g.players.len() == 4
            && g.shots >= 2
            && g.players[i].bot_left != g.shots
        {
            g.players[i].bot_left = g.shots;
            if rand(&mut g.rng) < 0.25 {
                let call = sound::call_out(i, rand(&mut g.rng) < 0.5);
                g.whooshes.push((0, i, call));
            }
        }
        let p = g.players[i];
        let goal = mine.map_or(p.home, |(b, _)| {
            [
                b[0] - 1.1 * (b[0] - p.pos[0]).signum(),
                0.0,
                b[2] - p.end * g.reach.ahead,
            ]
        });
        let d = Vec2::new(goal[0] - p.pos[0], goal[2] - p.pos[2]);
        // ponytail: stops within one stride of the goal; the AI's own approach is P11
        let dir = if d.length() < 0.1 { Vec2::ZERO } else { d };
        locomote(g, i, dir);
        // press when the contact search would lock onto the drawn frame (or later, if it is already past)
        if mine.is_some() {
            if g.players[i].bot_due.is_none() {
                let r = rand(&mut g.rng);
                g.players[i].kind = if r < 0.7 {
                    0
                } else if r < 0.9 {
                    1
                } else {
                    3
                };
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
    advance_stroke(g, i, |g| {
        Vec2::new(rand(&mut g.rng) * 1.8 - 0.9, rand(&mut g.rng) * 1.6 - 0.8)
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
        serve::search(&g.serve_data, Toss::Strong, &predicted_path(g, horizon))
            .filter(|&k| k <= due)
            .map(|_| 0)
    } else {
        None
    };
    serve_turn(g, i, Vec2::ZERO, press);
}

/// What drawing blends from: facing and camera as the last tick left them.
fn remember(mut g: ResMut<Game>) {
    g.prev_view = g.cam.view;
    for p in &mut g.players {
        p.prev_facing = p.facing;
    }
}

fn simulate(mut g: ResMut<Game>) {
    let g2 = &mut *g;
    // ponytail: she steps first, so this tick's phase messages reach her a tick later than in the original
    let playing = g2.umpire_voice != 0;
    g2.umpire.step(playing, g2.flight.ball.pos);
    let rng = &mut g2.rng;
    let cheers = g2.gallery.step(
        g2.stage,
        g2.players.len() as u32,
        g2.gallery_game,
        &mut || {
            rand(rng);
            *rng
        },
    );
    g2.cheers.extend(cheers);
    for e in &mut g2.emitters {
        if e.step(&mut || {
            rand(rng);
            *rng
        }) {
            debug!("emitter type {} sound {}", e.ty, e.row.sound);
            g2.sounds.push((
                sound::Play {
                    slot: 0,
                    program: 7,
                    key: e.row.sound as u8,
                    volume: 0x40,
                    speed: 1.0,
                },
                e.pos,
            ));
        }
    }
    let players: Vec<V3> = g2.players.iter().map(|p| p.pos).collect();
    g2.cam.step(&Scene {
        players: &players,
        ball: g2.flight.ball.pos,
    });
    if std::mem::take(&mut g2.cam_cut) {
        g2.prev_view = g2.cam.view; // a cut doesn't blend
    }
    match g.phase {
        // only the toss flies while the serve is set up
        Phase::Serve if !g.serving.tossed => return,
        Phase::Serve => {}
        Phase::ChangeEnds(0) => return next_point(&mut g, false),
        Phase::ChangeEnds(n) => return g.phase = Phase::ChangeEnds(n - 1),
        Phase::Post => {
            let g = &mut *g;
            let mut post = g.post.take().expect("post-point state");
            let (reacted, paused, showing) = (post.reacted, post.paused(), post.showing());
            // her call line has ended (it started the tick the point was decided)
            let idle = post.tick > 0 && g.umpire_voice == 0;
            let press = std::mem::take(&mut g.post_press);
            let step = post.step_with(&mut g.score, &mut g.rally, &g.board, idle, press);
            if let Some(event) = post.event {
                if paused && !post.paused() {
                    g.umpire.call_score(&g.score, Some(event), g.post_winner);
                }
                if post.showing() && !showing {
                    g.umpire.announce(&g.score, event, g.post_winner);
                }
                if post.reacted && !reacted {
                    react(g, event);
                }
            }
            match step {
                None => g.post = Some(post),
                Some(Next::Serve) => return next_point(g, true),
                Some(Next::ChangeEnds) => {
                    g.gallery.hush();
                    g.umpire.start(true, g.flight.ball.pos);
                    // the change-ends tune, cued as the phase is entered
                    g.jingle = Some(0);
                    return g.phase = Phase::ChangeEnds(CHANGE_ENDS);
                }
                Some(Next::MatchOver) => {
                    // ponytail: the match-over phase isn't played: she announces it as the next match starts
                    g.umpire.call_match();
                    g.umpire.match_over();
                    g.score = Score::new();
                    g.players.iter_mut().for_each(|p| p.stance = 3.0);
                    return next_point(g, true);
                }
            }
        }
        Phase::Rally => {}
    }
    let g = &mut *g;
    g.prev_ball = g.flight.ball.pos;
    let (shot, surface) = (g.shot, &COURTS[g.court]);
    g.flight.in_play = g.shots > 0;
    let before = g.flight.bounces;
    g.flight.step_world(&shot, surface, &g.world.0, &g.world.1);
    // bounce sounds stop once the point is decided (the deciding bounce still plays)
    let (n, (at, material)) = (g.flight.bounces, g.flight.landing);
    let bounce = (n != before && n > 0 && g.phase == Phase::Rally).then_some((n, &material));
    let smash = g.smashed.then(|| sound::kmh(g.flight.ball.vel));
    g.sounds
        .extend(g.bounces.frame(bounce, smash).into_iter().map(|p| (p, at)));
    g.whistle.0 &= n == 0;
    g.since_hit += 1;
    if g.phase != Phase::Rally || g.shots == 0 {
        return;
    }
    let f = &g.flight;
    // ponytail: no ball body hits and no rest detection on steep surfaces yet; the rally timeout counts as at rest
    let view = BallState {
        call: f.call,
        contacts: f.contacts,
        stopped: f.frame > 1800,
        pos: f.ball.pos,
    };
    if !g
        .rally
        .check(&view, g.shots, g.last_hitter, g.score.server, None, true)
    {
        return;
    }
    let verdict = g.rally.judge(None);
    let why = CALLS[verdict.call as usize];
    let Some(team) = verdict.winner else {
        g.umpire
            .point_over(None, 0, g.score.swapped, verdict.call as u8);
        g.message = if verdict.call == 2 {
            "Fault - second serve".into()
        } else {
            format!("{why} - serve again")
        };
        info!("no point: {why} after {} frames", g.since_hit);
        g.post = Some(PostPoint::called(None, verdict.call as u8, &g.board));
        g.phase = Phase::Post;
        return;
    };
    g.post_winner = team as i32;
    let before = g.score.clone();
    let event = g.score.point(&g.rules, team as usize);
    g.umpire
        .point_over(event, team as i32, g.score.swapped, verdict.call as u8);
    g.score.note_tiebreak_start();
    g.gallery_game = matches!(event, Some(Event::Game | Event::Set));
    // ponytail: played at the verdict; the game's director cues it a little later in the post-point sequence
    g.jingle = match event {
        Some(Event::Set) if g.score.match_over => Some(
            if g.humans
                .iter()
                .enumerate()
                .any(|(i, &h)| h && i & 1 == team as usize)
            {
                3
            } else {
                4
            },
        ),
        Some(Event::Set) => Some(2),
        Some(Event::Game) => Some(1),
        _ => None,
    };
    let applause = if g.gallery_game {
        sound::favoured(&g.humans)[team as usize]
    } else {
        g.smashed && g.last_hitter & 1 == team as i32
    };
    if let Some(r) = sound::reaction(verdict.call, g.gallery_game, applause, &mut g.errors) {
        let rng = &mut g.rng;
        g.gallery.point(r, &mut || {
            rand(rng);
            *rng
        });
    }
    let who = format!("team {}", team + 1);
    g.message = match event {
        Some(Event::Set) if g.score.match_over => format!("{why} - match to {who}"),
        Some(Event::Set) => format!("{why} - set to {who}"),
        Some(Event::Game) => format!("{why} - game to {who}"),
        _ => format!("{why} - point to {who}"),
    };
    info!(
        "point: {who} ({why}) after {} frames, {event:?} {:?}",
        g.since_hit, g.score
    );
    let mut post = PostPoint::called(
        Some(event.expect("match in progress")),
        verdict.call as u8,
        &g.board,
    );
    post.before = Some(before);
    g.post = Some(post);
    g.phase = Phase::Post;
}

/// Every player's reaction to the point (`hst_sim::motion::reaction`): winners `gu`, losers `di` (`_set` when the
/// point ends a game), in doubles sometimes a team reaction instead, each player's own pick.
/// ponytail: the app's random numbers stand in for the game's; ball body hits (`re_ball`) aren't simulated yet
fn react(g: &mut Game, event: Event) {
    let n = g.players.len() as i32;
    let winner = g.post_winner;
    let mut taken = Vec::new();
    for i in 0..g.players.len() {
        let won = i as i32 & 1 == winner;
        let base = motion::reaction(
            false,
            n,
            won,
            matches!(event, Event::Game | Event::Set),
            false,
        );
        let id = if n == 4 {
            let draw = |m: u32| (rand(&mut g.rng) * m as f32) as u32;
            let id = motion::team_reaction(base, g.chars[i], &taken, draw);
            if id >= 0x30 {
                taken.push(id - 0x30);
            }
            id
        } else {
            base
        };
        let p = &mut g.players[i];
        p.vel = Vec2::ZERO;
        (p.whiff, p.pending) = (None, None);
        set_motion(p, id, 1.0, false, None);
        let spot = [p.pos[0], p.pos[1], p.pos[2], 1.0];
        p.root = Some(Root {
            motion: id as usize,
            base: spot,
            acc: spot,
            t: 0.0,
        });
    }
}

/// Leave the point: next server, receiver and side, and set up the serve. `fresh`: not after a change of ends.
fn next_point(g: &mut Game, fresh: bool) {
    g.umpire.serve(fresh, false, g.flight.ball.pos);
    g.music_hold = false;
    g.gallery.hush();
    for e in &mut g.emitters {
        e.reset(None);
    }
    g.score.next_point(&g.rules);
    g.rally.next_point();
    g.rally.new_point();
    g.shots = 0;
    g.last_hitter = -1;
    g.marks = Marks::default();
    g.phase = Phase::Serve;
    g.message.clear();
    reset_positions(g);
}

/// Orbit rig parameters that put the eye at `eye` looking along `forward` (game space).
fn orbit_from(eye: [f32; 3], forward: [f32; 3]) -> (Vec3, f32, f32) {
    // game space → Bevy: (x, -y, -z)
    let back = -Vec3::new(forward[0], -forward[1], -forward[2]).normalize();
    (
        Vec3::new(eye[0], -eye[1], -eye[2]),
        (-back.y).asin(),
        back.x.atan2(back.z),
    )
}

/// The original's match camera (`hst_sim::camera`); free mode leaves the mouse/right stick in charge.
fn camera(
    g: Res<Game>,
    time: Res<Time<Fixed>>,
    mode: Res<CamMode>,
    cs: Res<CamState>,
    window: Query<&Window>,
    mut q: Query<(&mut Orbit, &mut Projection)>,
) {
    let Ok((mut o, mut proj)) = q.single_mut() else {
        return;
    };
    if *mode == CamMode::Free {
        o.yaw += cs.turn;
        return;
    }
    // between the last two ticks' views
    let (a, v0, v) = (time.overstep_fraction(), g.prev_view, g.cam.view);
    let mix = |x: V3, y: V3| Vec3::from(x).lerp(Vec3::from(y), a).to_array();
    let (eye, pitch, yaw) = orbit_from(mix(v0.eye, v.eye), mix(v0.rot[2], v.rot[2]));
    let fov = v0.fov + (v.fov - v0.fov) * a;
    o.radius = 40.0;
    o.pitch = pitch;
    o.yaw = yaw;
    o.focus = eye - Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0) * Vec3::Z * o.radius;
    if let Projection::Perspective(p) = &mut *proj {
        // never show less than the game's 4:3 picture: windows narrower than 4:3 widen vertically
        let aspect = window
            .single()
            .map_or(4.0 / 3.0, |w| w.width() / w.height().max(1.0));
        p.fov = 2.0 * (fov.tan() * SHOWN_ASPECT.max(1.0 / aspect)).atan();
        // the camera stays ~40 m out: a far near plane keeps depth precision for the layered character models
        p.near = 5.0;
    }
}

/// One frame of the smash search: grow the predicted path by 15 entries (until it has bounced twice; not on the
/// launch frame, N4a) and search it; the yellow marker goes once the live ball bounces.
fn smash_frame(g: &mut Game) {
    g.marks.start = false;
    let fresh = std::mem::take(&mut g.marks.fresh);
    if g.flight.bounces > 0 {
        g.marks.smash = None;
    }
    let (shot, court) = (g.shot, &COURTS[g.court]);
    let Some((search, path, f)) = &mut g.marks.smash else {
        return;
    };
    if !fresh && path.last().is_some_and(|e| e.bounces < 2) {
        for _ in 0..15.min(PATH_MAX - path.len()) {
            f.step(&shot, court);
            path.push(path_entry(f));
        }
    }
    g.marks.start = search.search(path);
}

fn draw(
    g: Res<Game>,
    time: Res<Time<Fixed>>,
    mut figures: Query<(&Figure, &mut Transform)>,
    mut ball: Query<(&BallView, &mut Transform), Without<Figure>>,
) {
    let a = time.overstep_fraction();
    for (f, mut t) in &mut figures {
        let p = g.players[f.0];
        t.translation = Vec3::from(p.prev).lerp(Vec3::from(p.pos), a);
        // the models face their local +z; `facing` is the stand-in yaw (0 = facing −z); left-handers mirrored
        let turn = |y: f32| Quat::from_rotation_y(y - std::f32::consts::PI);
        t.rotation = turn(p.prev_facing).slerp(turn(p.facing), a);
        t.scale = Vec3::new(p.hand, 1.0, 1.0);
    }
    for (v, mut t) in &mut ball {
        t.translation = Vec3::from(g.prev_ball).lerp(Vec3::from(g.flight.ball.pos), a);
        // ponytail: the shadow lies on the flat court (y 0); the stage's floor height under the ball if it shows
        if v.0 {
            t.translation.y = 0.0;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn start_effects(
    mut g: ResMut<Game>,
    mut fx: ResMut<effects::Impacts>,
    mut sparks: ResMut<effects::HitSparks>,
    mut flight: ResMut<effects::BallFlight>,
    mut bounce: ResMut<effects::BallBounce>,
    mut marks: ResMut<effects::LandingMarks>,
    mut transforms: Query<&mut Transform>,
    mut commands: Commands,
) {
    smash_frame(&mut g);
    marks.red_at = g.marks.red;
    marks.smash_at = g.marks.smash.as_ref().and_then(|(s, _, _)| s.at());
    marks.tick(g.marks.start);
    let hit = g.hit_effect.take();
    if let Some(h) = hit {
        fx.start(h, &mut transforms);
    }
    sparks.frame(hit, &mut commands);
    let dead = !matches!(g.phase, Phase::Serve | Phase::Rally);
    flight.frame(
        hit,
        dead,
        g.flight.ball.pos,
        g.flight.ball.vel,
        &mut commands,
    );
    let f = &g.flight;
    let smash = g.smashed && sound::kmh(f.ball.vel) >= 85.0;
    // ponytail: marks age while a point is on (the game's own on/off messages are not traced)
    bounce.frame(
        f.bounces,
        f.landing.0,
        f.ball.vel,
        f.landing.1.court,
        smash,
        !dead,
        &mut transforms,
    );
}

/// Which of the game's motions each player plays (by its motion number): the serve's stance, baseline walk,
/// toss and swing; a locked stroke's swing (timed so its contact pose, frame 8, meets the ball); otherwise the
/// stand/run motion the game picks (`locomote`).
fn motions(g: Res<Game>, mut q: Query<(&Figure, &mut Motion)>) {
    for (f, mut m) in &mut q {
        let i = f.0;
        let p = &g.players[i];
        m.arm = p.arm;
        let serving = g.phase == Phase::Serve && g.score.server == i as i32;
        if serving {
            let s = g.serving;
            let under = s.toss == Some(Toss::Under);
            match (s.toss, s.swing) {
                (None, _) if p.pos != p.prev => m.play(
                    motion::serve_walk(p.pos[0] - p.prev[0], p.end, p.hand) as usize,
                    1.0,
                    true,
                ),
                (None, _) => m.play(0x20, 1.0, true),
                (Some(_), Some(sw)) => {
                    let (id, speed) = motion::serve_swing(under, sw.frames as i32);
                    // the speed is set once, at the swing's start
                    let speed = if m.id == id as usize {
                        m.clock.speed
                    } else {
                        speed
                    };
                    m.play(id as usize, speed, false);
                }
                (Some(_), None) if s.whiffed => m.play(
                    motion::whiff(if under { 0x26 } else { 0x25 }).unwrap_or(0x29) as usize,
                    1.0,
                    false,
                ),
                (Some(_), None) => m.play(motion::serve_toss(under) as usize, 1.0, false),
            }
            continue;
        }
        // a finished serve swing plays out before running
        if (m.id == 0x25 || m.id == 0x26) && m.clock.time < 30.0 {
            continue;
        }
        let c = p.cmd;
        if c.serial != m.serial {
            m.set(c.id, c.speed, c.looping, c.hold, c.serial);
        }
        m.clock.speed = c.speed;
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
        let Some(alpha) = serve::balloon_alpha(age) else {
            continue;
        };
        // fades blend from the last tick's alpha (none before the first)
        let before = age
            .checked_sub(1)
            .and_then(serve::balloon_alpha)
            .unwrap_or(0.0);
        let alpha = before + (alpha - before) * a;
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

fn hud(
    g: Res<Game>,
    pads: Res<Pads>,
    mode: Res<CamMode>,
    mut q: Query<&mut Text, With<ScoreText>>,
) {
    let who = |i: usize| {
        pads.slot_of(i, g.players.len())
            .map_or("CPU".to_string(), |s| format!("P{}", s + 1))
    };
    let team = |t: usize| {
        (t..g.players.len())
            .step_by(2)
            .map(who)
            .collect::<Vec<_>>()
            .join("+")
    };
    let board = board_score(&g);
    for mut t in &mut q {
        t.0 = format!(
            "Team 1 ({}) {}  -  {} ({}) Team 2\n{}\ncontrollers: {} · camera: {} (C / Select)\nmove WASD/stick/d-pad · J/A topspin · K/B slice · L/Y lob · stick forward: flat, back + slice: drop",
            team(0),
            score_line(board, 0),
            score_line(board, 1),
            team(1),
            g.message,
            pads.connected,
            if *mode == CamMode::Original {
                "original"
            } else {
                "free"
            }
        );
    }
}

/// Games and points for one team, tennis style (points 0/15/30/40/Ad; tiebreak points as numbers).
fn score_line(s: &Score, team: usize) -> String {
    let p = s.points[team];
    let pts = if s.tiebreak {
        p.to_string()
    } else {
        ["0", "15", "30", "40", "Ad"]
            .get(p as usize)
            .unwrap_or(&"Ad")
            .to_string()
    };
    format!("{} | {pts}", s.games[team])
}

/// A swing starts: its whoosh, if the original plays one (`sound::swing_sound`), now or a few ticks on.
fn whoosh(g: &mut Game, i: usize, branch: u8, kind: i32) {
    if let Some(wait) = sound::swing_sound(branch, kind, g.rally.faults) {
        g.whooshes.push((wait, i, sound::SWING));
    }
}

/// The BGM's level as the game's director sets it each frame: full (55) until a game, set or change-ends jingle,
/// then down to silence over 60 frames, held there while the jingle plays and until the next point, then back up
/// over 60 frames.
#[derive(Default)]
struct Bgm {
    started: bool,
    volume: f32,
    fading: bool,
    jingle: u64,
}

impl Bgm {
    const FULL: f32 = 55.0;

    fn step(&mut self, g: &mut Game, sound: &Sound, music: &MusicBanks) {
        if !self.started {
            self.started = true;
            if let Some((bank, mid)) = &music.bgm {
                sound.music(bank, mid, Self::FULL as u32);
            }
        }
        if let (Some(key), Some(b)) = (g.jingle.take(), &music.jingles) {
            sound.stop(self.jingle);
            self.jingle = sound.play_centre(
                b,
                sound::Play {
                    slot: 8,
                    program: 0,
                    key,
                    volume: 0x80,
                    speed: 1.0,
                },
            );
            // ponytail: the match-end jingles (3, 4) leave the BGM as it is, as the game's match end does
            if key < 3 {
                // the change-ends tune fades the BGM without a hold of its own: after a game the game jingle's
                // hold lasts to the next serve; in a tiebreak the BGM comes back once the tune ends
                self.fading = true;
                g.music_hold |= key > 0;
            }
        }
        if self.jingle != 0 && !sound.playing(self.jingle) {
            self.jingle = 0;
        }
        let step = Self::FULL / 60.0;
        self.volume = if !self.fading {
            Self::FULL
        } else if self.jingle == 0 && !g.music_hold {
            self.fading = self.volume + step < Self::FULL;
            (self.volume + step).clamp(0.0, Self::FULL)
        } else {
            (self.volume - step).max(0.0)
        };
        sound.music_volume(self.volume as u32);
    }
}

/// Plays the sounds due on the court's bank.
/// The flight whistle follows the ball: started at the hit, re-placed and re-pitched each tick, stopped at the bounce.
fn play_sounds(
    mut g: ResMut<Game>,
    sound: Option<Res<Sound>>,
    bank: Option<Res<CourtBank>>,
    voices: Option<Res<VoiceBanks>>,
    gallery: Option<Res<GalleryBank>>,
    music: Option<Res<MusicBanks>>,
    mut whistle: Local<(u32, u64)>,
    mut bgm: Local<Bgm>,
) {
    let g = &mut *g;
    if let (Some(sound), Some(music)) = (&sound, &music) {
        bgm.step(g, sound, music);
    }
    for w in &mut g.whooshes {
        w.0 = w.0.saturating_sub(1);
        if w.0 == 0 {
            debug!("player {} sound {:?}", w.1, w.2);
            g.sounds.push((w.2, g.players[w.1].pos));
        }
    }
    g.whooshes.retain(|w| w.0 > 0);
    let due = std::mem::take(&mut g.sounds);
    let cheers = std::mem::take(&mut g.cheers);
    if let (Some(sound), Some(Some(b))) = (&sound, gallery.map(|b| b.0.clone())) {
        for (p, angle) in cheers {
            sound.play_toward(&b, p, angle);
        }
    }
    let (Some(sound), Some(Some(bank))) = (sound, bank.map(|b| b.0.clone())) else {
        return;
    };
    // slot 0 the court, 1.. the players' voices (the umpire's voice, slot 5, is played below);
    // ponytail: slot 9 (framed hits) is a character bank not loaded yet
    for (p, at) in due {
        let b = if p.slot == 0 {
            Some(&bank)
        } else {
            voices
                .as_ref()
                .and_then(|v| v.0.get(p.slot as usize - 1)?.as_ref())
        };
        if let Some(b) = b {
            sound.play_at(b, p, at);
        }
    }
    let ball = g.flight.ball.pos;
    if whistle.0 != g.whistle.1 || !g.whistle.0 {
        sound.stop(whistle.1);
        whistle.1 = 0;
    }
    if g.whistle.0 && whistle.1 == 0 && whistle.0 != g.whistle.1 {
        whistle.1 = sound.play_at(
            &bank,
            sound::Play {
                speed: sound::flight_speed(ball[1], 0.0, 10.0),
                ..sound::FLIGHT
            },
            ball,
        );
    } else if whistle.1 != 0 {
        sound.update(
            whistle.1,
            sound::Play {
                speed: sound::flight_speed(ball[1], 0.3, 10.0),
                ..sound::FLIGHT
            },
            ball,
        );
    }
    whistle.0 = g.whistle.1;
    // her voice (slot 5, non-positional): a new word stops the last
    if let (Some((program, key)), Some(b)) = (
        g.umpire.voice.take(),
        voices.as_ref().and_then(|v| v.0.get(4).cloned().flatten()),
    ) {
        sound.stop(g.umpire_voice);
        g.umpire_voice = sound.play_centre(
            &b,
            sound::Play {
                slot: 5,
                program,
                key: key as u8,
                volume: 0x80,
                speed: 1.0,
            },
        );
    }
    if g.umpire_voice != 0 && !sound.playing(g.umpire_voice) {
        g.umpire_voice = 0;
    }
}

/// A human's face-button press while the point is over: it ends the phase once the new score has settled.
fn post_press(g: &mut Game, pressed: bool) {
    g.post_press |= pressed && g.phase == Phase::Post;
}

/// The score the scoreboard shows: the one before the point until the score show brings in the new one.
fn board_score(g: &Game) -> &Score {
    g.post
        .as_ref()
        .filter(|p| !p.shown)
        .and_then(|p| p.before.as_ref())
        .unwrap_or(&g.score)
}
