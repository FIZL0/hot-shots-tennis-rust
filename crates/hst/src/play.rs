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
//! (original / free), arrow keys turn the free camera, Esc pause (`play/menu.rs`).
//! Gamepad: left stick or d-pad move/aim, A (✕) topspin, B (○) slice, Y (△) lob, A serve, Start pause, Select camera,
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
use hst_sim::rng::{Mt, Rand, Rngs};
use hst_sim::npc;
use hst_sim::params::{self, ShotParams};
use hst_sim::player::{self as loco, Stats};
use hst_sim::pose::{ArmIk, contact_solve};
use hst_sim::score::{Event, Rules, Score};
use hst_sim::serve::{self, Balloon, ServeData, Toss};
use hst_sim::shot::{Bounds, Table, lookup};
use hst_sim::sound;
use hst_sim::swing::{self, PathPoint, Reach};
use hst_sim::umpire::Umpire;

use crate::audio::{
    CourtBank, GalleryBank, MusicBanks, Sound, SoundBank, VoiceBanks, umpire_bank, voice_bank,
};
use crate::character::{self, CharacterData, Motion};
use crate::effects;
use crate::{Args, GameSpace, Orbit};

mod ball_shadow;
mod bodyhit;
mod controls;
mod cutaway;
mod foot_fx;
mod doubles_ai;
mod markers;
mod match_stats;
mod menu;
mod npcs;
mod panel;
mod popups;
mod reaction_voices;
mod surprise;
mod timing;
mod tornado;
mod weather;
mod widescreen;

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
/// Ball drawn this much larger than its physical size, toon style, so it reads at broadcast distance.
/// ponytail: the original draws `ball1.mdl` at scale 1 (ball object matrix in s03–s05); this is the remaster's look.
const BALL_DRAW_SCALE: f32 = 2.4;
/// The contact is graded against this frame after the press (the timing table's sweet spot).
const SWEET_FRAME: i32 = serve::SWEET_FRAME;
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
    /// The server's follow-through after the serve's contact: frames since contact (it stays put, `serve_follow`).
    served: Option<u32>,
    swung: bool,
    backhand: bool,
    /// Serve animation progress (0..1) while this player serves.
    serve_anim: Option<f32>,
    kind: i32,
    aim: Vec2,
    /// The d-pad's held directions with `aim` (`SlotPad::dpad`).
    dpad: u16,
    /// A shot press still looking for its contact (frames it stays live).
    pending: Option<u32>,
    /// The auto-approach's run direction (game x, z) while `pending` counts down to its one search.
    approach: Option<Vec2>,
    contact: Option<Contact>,
    /// Frames of wind-up before contact (sets the animation clock).
    wind: u32,
    /// Distance off centre at this player's last toss: where they serve from next time.
    stance: f32,
    /// Where the player was put for this point (the stand-in AI's home spot).
    home: V3,
    /// The ball where the racket meets it (game space), while a stroke is locked.
    hit_at: Option<V3>,
    /// Where the swing (or dive) started (x, z): the aim's angle is measured from it.
    aim_from: [f32; 2],
    /// The timing balloon over this player's head and its age in frames.
    balloon: Option<(Balloon, u32)>,
    /// The AI was caught off guard this tick (a change-of-pace or fast-ball reaction, a wrong guess): `surprise`.
    surprised: bool,
    /// Stand-in AI: set once it has picked its shot for the coming ball (serving: the contact frame it waits for).
    bot_due: Option<usize>,
    /// Stand-in AI: the shot it last decided to leave to its partner.
    bot_left: i32,
    /// This player's AIParam.csv row when the computer plays it (`hst_sim::ai`).
    ai: hst_sim::ai::AiParams,
    /// Its doubles formation (`hst_sim::position::Team::pick`, 0 staggered, 1 attacking, 2 defensive): picked at
    /// the match's first point, kept for the match.
    formation: u8,
    /// The AI's memory of the opponents' last two shots and last two serves' velocities, whether its team has hit
    /// since the match began (its stroke error is halved until then), and the timing errors it drew last.
    ai_seen: [Option<hst_sim::ai::Seen>; 2],
    ai_serves: [Option<V3>; 2],
    ai_hit: bool,
    ai_timing: hst_sim::ai::Timing,
    /// A doubles bot's place in the formation and its walk back there (`hst_sim::position`), placed afresh every
    /// point; the frames since it last looked again while waiting, and the shot count it last re-picked on.
    ai_form: hst_sim::position::Formation,
    ai_back: hst_sim::position::Return,
    ai_tick: u32,
    ai_shot: i32,
    /// The AI's guess at the serve and where it stood when it drew it (`hst_sim::ai::Guess`), and the frames it
    /// holds before reading the ball: the guess's move frames, or the stuck frames after a wrong guess.
    ai_guess: Option<(hst_sim::ai::Guess, [f32; 2])>,
    /// The side of the ball the AI stands on for the shot it is after (the shot count, true: ball x − reach·side).
    ai_side: Option<(i32, bool)>,
    ai_hold: i32,
    /// A singles bot's centre spot, net dash and walk back (`hst_sim::position::Single`), placed afresh every point,
    /// and the branch of the swing it last had on (its after-hit reads it).
    ai_single: Option<hst_sim::position::Single>,
    ai_branch: Option<swing::Branch>,
    /// A singles bot's net dash off its own serve (`AiParams::serve_aim_singles`): the spot it dashes to.
    ai_serve_dash: Option<[f32; 2]>,
    /// Its serve swing plan while the aim waits for the frame before the contact.
    ai_serve_swing: Option<u8>,
    /// The AI object (`hst_sim::ai::Mind`), kept across points (None until the match's first point); this point's
    /// messages heard so far (0 none, 1 the reset, 2 the point over, 3 the players' reaction); its serve spot (x)
    /// and the frames it still waits there before the toss.
    ai_mind: Option<hst_sim::ai::Mind>,
    ai_heard: u8,
    ai_serve: (f32, i32),
    /// The AI's shot-choice picks drawn with its timing errors (`hst_sim::ai::Picks`); whether that draw takes the
    /// dive chance after its own team's hit (None: a reset, always); the stick and button it chose at its press
    /// (the button after the singles kind lock); its character's strong-toss chance (%) on a second serve.
    ai_picks: hst_sim::ai::Picks,
    ai_dove: Option<bool>,
    ai_press: Option<(Vec2, u8)>,
    ai_second: i32,
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
    /// Serve trajectory tables, kinds 0..3 (topspin, slice, flat, underhand).
    /// Per player: the character's serve trajectory tables with their launch spin, [strong/underhand, weak toss]
    /// by kind (topspin, slice, flat, underhand; the weak toss's `dw1` tables have no underhand).
    serve_tables: Vec<[Vec<(Table, f32, f32)>; 2]>,
    /// Per player: the character's special-shot effects and, when its special slice serve flies by the `up1`
    /// variant, that table with its spin and side angle (see `serve_specials`).
    specials: Vec<(hst_data::exe::ShotEffects, Option<(Table, f32, f32)>)>,
    /// Per player: the character's smash trajectory tables, smash kinds 0 (✕/○) and 1 (△).
    smash_tables: Vec<Vec<Table>>,
    /// Per player: the spin of the character's smash records, smash kinds 0 and 1.
    smash_spins: Vec<[f32; 2]>,
    /// Per player: the character's [stroke, volley] trajectory tables by kind (see `rally_tables`).
    rally_tables: Vec<[Vec<Table>; 2]>,
    /// Per player: the character's stroke and volley shot records (classes 1, 2) by kind, for their spins.
    rally_records: Vec<[[[f32; 13]; 5]; 2]>,
    /// Per player: its character's timing data (see `timing`).
    timing: Vec<timing::Timing>,
    /// The timing error of the stroke or volley being struck, set at its contact.
    timing_error: Option<swing::TimingError>,
    /// The mis-hit roll of the stroke, volley or dive being struck, set at its contact.
    mis_hit: Option<swing::MisHit>,
    /// The last human aim's leftovers for the launch (nudge, short-only, a smash's held depth).
    aim: hst_sim::shot::Aim,
    /// The smash being struck's timing scatter (`timing::smash`), set at its contact.
    smash_scatter: Option<(f32, f32)>,
    /// Per player: the serve (see `serve_data`).
    serve_data: Vec<ServeData>,
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
    /// The ball as it was struck, for the prediction the original held when the ball touched the net.
    hit_flight: Flight,
    /// Court contacts the original's predicted path keeps after the ball touches the net (0 before).
    path_base: i32,
    score: Score,
    rally: Rally,
    /// Shots this rally (1 = the serve).
    shots: i32,
    /// The last shot was on the sweet spot (not a dive, |offset| < 2): its record's flag the next aim reads.
    last_sweet: bool,
    /// The match statistics (`match_stats`).
    match_stats: hst_sim::stats::Stats,
    /// Per player: the character's aim values (see `aim_stats`).
    aim_stats: Vec<hst_sim::shot::AimStats>,
    /// Line tolerance from the game program.
    line_margin: f32,
    /// How far inside the lines a rally shot's aim is pulled, from the game program.
    margins: hst_sim::shot::Margins,
    post: Option<PostPoint>,
    /// A human pressed a face button while the point is over (`post_press`), for the next simulation tick.
    post_press: bool,
    board: ScoreboardTiming,
    /// The stage's collision world (`--stage`, else court 10's) and the material table: court, net, posts, walls.
    world: (World, Vec<Material>),
    court: usize,
    message: String,
    /// The game's generators (`hst_sim::rng`): every draw the app shares with the original comes from one.
    rng: Rngs,
    /// The sound generator as a reseed found it: the hit sparks roll their table afresh from it.
    respark: Option<Mt>,
    /// Sounds due this tick, at a game-space position.
    sounds: Vec<(sound::Play, V3)>,
    /// Swing whooshes and dive thuds waiting to play: ticks left, the player (played at their position) and the sound.
    whooshes: Vec<(u32, usize, sound::Play)>,
    /// The ball's bounce sounds, and whether the last shot was a smash in a game of more than one player.
    bounces: sound::Bounces,
    smashed: bool,
    /// The flight whistle (`sound::FLIGHT`) is on, and how many have started (a new one restarts it).
    whistle: (bool, u32),
    /// What the flight sound plays (whistle or rush), set at each hit that starts one.
    flight_sound: Option<sound::Flight>,
    /// The last hit was a special serve (the AI's reaction adds its special-serve frames).
    special_serve: bool,
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
    /// No serve struck yet this match: the singles server's voice on the coming new point.
    first_serve: bool,
    /// Who has made their match-point serve call this match (`serve_call`).
    serve_called: [bool; 4],
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
    /// The decided point's reaction, for the gallery to take before the walkers step.
    applause: Option<sound::Reaction>,
    /// The court's ambient sound emitters.
    emitters: Vec<npc::Emitter>,
    /// Which players are on a controller (the gallery favours their side).
    humans: Vec<bool>,
    /// The racket impact a shot started this tick.
    hit_effect: Option<effects::Hit>,
    /// Per player its character's smash top and window middle (TParam, m) and the landing markers' state.
    smash_heights: Vec<[f32; 2]>,
    /// Per player the contact search's reach: `reach` with the character's TParam reach and heights and its hand.
    reaches: Vec<Reach>,
    /// The doubles AI's rally state (`doubles_ai`).
    doubles_ai: doubles_ai::Shared,
    marks: Marks,
    /// The finish and Set / Match Point banners (`popups`).
    finish: popups::Finish,
    /// The player the ball hit this point, if any (`bodyhit`).
    body_hit: Option<i32>,
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
    /// The left stick alone (after the pad driver, `pad_stick`).
    stick: Vec2,
    /// D-pad (and direction keys) held, as the pad's bits (`loco::pad_dir`: 0x10 up, 0x20 right, 0x40 down,
    /// 0x80 left): the original runs and aims by them only while the stick sits in its dead square.
    dpad: u16,
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
/// A balloon billboard over player `.0`'s head (its own material).
#[derive(Component)]
struct BalloonView(usize, Handle<StandardMaterial>);
/// The balloon textures by `Balloon` (bunny, turtle, note, sweet).
#[derive(Resource)]
struct BalloonArt([Handle<Image>; 4]);

pub fn plugin(app: &mut App) {
    app.add_plugins(bodyhit::plugin);
    app.add_plugins(controls::plugin);
    app.add_plugins(cutaway::plugin);
    app.add_plugins(ball_shadow::plugin);
    app.add_plugins(foot_fx::plugin);
    app.add_plugins(markers::plugin);
    app.add_plugins(match_stats::plugin);
    app.add_plugins(menu::plugin);
    app.add_plugins(npcs::plugin);
    app.add_plugins(panel::plugin);
    app.add_plugins(popups::plugin);
    app.add_plugins(reaction_voices::plugin);
    app.add_plugins(surprise::plugin);
    app.add_plugins(tornado::plugin);
    app.add_plugins(weather::plugin);
    app.add_plugins(widescreen::plugin);
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
    app.add_systems(FixedUpdate, flare_tick.after(play_sounds));
}

/// The sun's lens flare's `rand()` draws each tick it shows (clear or cloudy).
fn flare_tick(game: Option<ResMut<Game>>) {
    if let Some(mut g) = game
        && crate::weather::now() < 2
    {
        g.rng.flare_tick();
    }
}

static EFFECTS_SEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);

/// The match's effects seed (`Rngs::setup_rand`), drawn when main.rs sets the match up.
pub fn set_effects_seed(s: u32) {
    EFFECTS_SEED.store(s as u64, std::sync::atomic::Ordering::Relaxed);
}

/// The effects seed, once the match setup has drawn it.
pub fn effects_seed() -> Option<u32> {
    u32::try_from(EFFECTS_SEED.load(std::sync::atomic::Ordering::Relaxed)).ok()
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

/// Character `c`'s base stroke tables (`strk` 0..5) and volley tables (`voly` 0..5; 0 and 1 sit in the A
/// archive, 2..4 in B).
// ponytail: only the base tables; the up1/dw1/dw2 variants come with the timing/mode pick (P3)
fn rally_tables(iso: &mut Iso, c: usize) -> [Vec<Table>; 2] {
    let mut volleys = character_tables(iso, c, "A", "voly", "", 2);
    volleys.extend((2..5).map(|k| character_table(iso, c, "B", "voly", "", k)));
    [character_tables(iso, c, "A", "strk", "", 5), volleys]
}

/// Character `c`'s one trajectory table `tr_pc<c>_<name><k><suffix>.dat` from `TRAJ<c><ab>.XB`, naming the file
/// when the archive lacks it.
fn character_table(iso: &mut Iso, c: usize, ab: &str, name: &str, suffix: &str, k: usize) -> Table {
    let data = iso
        .read(&format!("TRAJ/TRAJ{c:02}{ab}.XB"))
        .expect("trajectory archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let file = format!("tr_pc{c:02}_{name}{k}{suffix}.dat");
    let e = arc
        .entries
        .iter()
        .find(|e| e.name.to_ascii_lowercase().ends_with(&file))
        .unwrap_or_else(|| panic!("trajectory table {file} not in TRAJ{c:02}{ab}.XB"));
    Table::parse(&arc.read(e).expect("table bytes")).expect("16^3 table")
}

/// Every character's start-up tables (serve, smash, stroke, volley) load from the disc (needs the ISO at the
/// repository root; skipped without it).
#[cfg(test)]
mod every_character_tables {
    use super::*;

    #[test]
    fn load() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
        let Ok(mut iso) = Iso::open(path) else {
            eprintln!("no ISO, skipped");
            return;
        };
        let params = disc(&mut iso, None).3;
        for c in 0..14 {
            character_tables(&mut iso, c, "B", "smsh", "", 2);
            serve_tables(&mut iso, &params, c);
            rally_tables(&mut iso, c);
            timing::load(&mut iso, &params, c);
        }
    }
}

/// A stroke's or volley's spin, first-bounce spin and restitution from the hitter's base shot record
/// (`params::rally_spin`). ponytail: the up1/dw1/dw2 variant records go with the table mode choice (P3).
fn rally_spin(g: &mut Game, who: usize, class: u8, kind: i32, at: V3, target: V3) -> f32 {
    let record = &g.rally_records[who][class as usize - 1][kind as usize];
    let (spin, first, rest) = params::rally_spin(record, class as usize, kind as usize, at, target);
    g.shot.first_bounce_spin = first;
    g.shot.first_bounce_restitution = rest;
    spin
}

/// `rally_spin` from a variant shot record (the timing's table mode).
fn record_spin(g: &mut Game, record: &[f32], class: u8, kind: i32, at: V3, target: V3) -> f32 {
    let (spin, first, rest) = params::rally_spin(record, class as usize, kind as usize, at, target);
    g.shot.first_bounce_spin = first;
    g.shot.first_bounce_restitution = rest;
    spin
}

/// A stroke's (class 1) or volley's (class 2, volleys and dives) launch from player `who`'s character tables (the
/// hitter's own, or on a counter the incoming hitter's).
fn rally_lookup(g: &Game, who: usize, class: u8, kind: i32, at: V3, target: V3) -> hst_sim::shot::Lookup {
    let (volley, k) = (class == 2, kind as usize);
    let bounds = if volley {
        Bounds::volley(kind, at[2])
    } else {
        Bounds::stroke(kind, at[2])
    };
    lookup(&g.rally_tables[who][volley as usize][k], &bounds, at, target)
}

/// A stroke's, volley's or smash's launch off its table lookup: no side angle (a special shot's bend, set by
/// `shot_effect`, only moves a serve's flight frames).
fn rally_launch(class: u8, at: V3, target: V3, l: &hst_sim::shot::Lookup) -> hst_sim::shot::Launch {
    let (at, target) = ([at[0], at[1], at[2], 1.0], [target[0], target[1], target[2], 1.0]);
    hst_sim::shot::launch_turned(class, at, target, l.elevation, l.speed, 0.0, 0.0, 0.0, false, l.frames)
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
/// Clear weather (0): `weather::step` turns the agility to rain's as the games go by.
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

/// A character's aim values from TParam.csv: Strk/Voley/Serv CON (columns 20–22), Body/Vbdy ADJ (23, 24),
/// Rizing ADJ (26) and the underhand limit in cm (62).
fn aim_stats(iso: &mut Iso, n: usize) -> hst_sim::shot::AimStats {
    let row = tparam(iso, n);
    let cell = |i: usize| row[i].split('/').next().unwrap().parse::<i32>().expect("TParam aim value");
    hst_sim::shot::AimStats {
        con: [cell(20), cell(21), cell(22)],
        body: [cell(23), cell(24)],
        rising: cell(26),
        under: hst_sim::ps2::div(cell(62) as f32, 100.0),
    }
}

/// A computer player's AIParam.csv row: character `n` in `outfit`, as an exhibition match picks it (outfits 4 and 9
/// take the hardest block).
fn ai_params(iso: &mut Iso, n: usize, outfit: usize, doubles: bool) -> hst_sim::ai::AiParams {
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
        hst_sim::ai::Choice::new(hst_sim::ai::menu_row(n as u8, outfit as u8) as u32, n as u8, doubles).row;
    hst_sim::ai::AiParams::table(&arc.read(e).expect("AIParam.csv bytes"))[row]
}

/// A character's hand (+1 right, −1 left): right only when TParam.csv's 利き腕 column is exactly 右, as the game's
/// parser. The game keeps it on the model as a left-hand flag, set to (left XOR the player's "switch hand" toggle on
/// character select), and the player's hand follows that flag: Carol (左) picked with the toggle on plays right-handed.
// ponytail: no character select yet, so the toggle is always off; XOR it in here when the menu exists.
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
    // 右 (right) in Shift-JIS
    if row
        .and_then(|r| r.split(|&b| b == b',').nth(6))
        .is_some_and(|c| c.trim_ascii() == [0x89, 0x45])
    {
        1.0
    } else {
        -1.0
    }
}

/// A player's serve: the toss hand and racket from the character's motions, heights from TParam.csv, the rest
/// measured on character 0 (see `ServeData`).
fn serve_data(iso: &mut Iso, data: &CharacterData) -> ServeData {
    let row = tparam_row(iso);
    let cm = |i: usize| -> [f32; 3] {
        let v: Vec<f32> = row[i]
            .split('/')
            .map(|v| v.parse::<f32>().expect("TParam height") / 100.0)
            .collect();
        [v[0], v[1], v[2]]
    };
    let cell = |i: usize| row[i].parse::<i32>().expect("TParam stat");
    // ponytail: timing and depth-bias tables (+0x1644/+0x1774, +0x1554/+0x1684), mistiming error (by
    // skill level 0) and the serve angle (+0x130c) are character 0's measured values; their sources are the
    // character tables (P3/P8)
    let grades = vec![
        0, 0, 4, 4, 2, 2, 2, 2, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 4, 4, 4,
    ];
    let bias = vec![
        -8, -6, -4, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 6, 8, 10, 12, 14, 16,
    ];
    let toss = |k: usize| {
        serve::toss_setup(&data.skeleton, &data.motions[&k], &data.motions[&(k + 2)])
    };
    let ((hand_over, racket_over), (hand_under, racket_under)) = (toss(0x23), toss(0x24));
    ServeData {
        over: cm(65),
        under: cm(66),
        strong_grades: grades.clone(),
        weak_grades: grades,
        hand_over,
        hand_under,
        racket_over,
        racket_under,
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

/// Character `n`'s reach for the contact search: `base` (character 0's measured contact offsets, body-shot widths,
/// grades and dive arm) with the character's own TParam reach centre, reach, ideal heights and smash window as the
/// game parses them, and the player's hand.
fn character_reach(base: &Reach, iso: &mut Iso, n: usize, hand: f32) -> Reach {
    let s = loco::ReachStats::from_tparam(&tparam(iso, n).join(","));
    Reach {
        base: s.base,
        reach: s.reach,
        stroke_height: s.stroke_height,
        volley_height: s.volley_height,
        smash_top: s.smash[0],
        smash_bottom: s.smash[2],
        hand,
        grades: swing::timing(s.after, s.before).0,
        ..base.clone()
    }
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

/// Line margin, rally aim margins, scoreboard timing, the stage's collision world with the material table, the shot parameter table,
/// and the umpire.
#[allow(clippy::type_complexity)]
fn disc(
    iso: &mut Iso,
    stage: Option<u32>,
) -> (
    (f32, hst_sim::shot::Margins),
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
    let (lines, angle) = game.court_margins();
    (
        (game.line_margin(), hst_sim::shot::Margins { lines, angle }),
        game.scoreboard_timing(0, 4),
        (court::world(iso, n), court::materials(&game)),
        params,
        umpire,
    )
}

/// Court `n`'s ambient sound emitters: the trigger creatures that only play sounds, each drawing its first gap.
/// They draw from the court's generator.
fn emitters(iso: &mut Iso, n: usize, players: u32, rng: &mut Mt) -> Vec<npc::Emitter> {
    let Some((list, plants)) = crate::court_layout(iso, n) else {
        return Vec::new();
    };
    let (cnf, bin) = (
        iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"),
        iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"),
    );
    let game = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
    let mut roll = || rng.next();
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

/// Character `c`'s serve tables, spins and side angles (see `Game::serve_tables`): the weak toss serves by the
/// `dw1` variant tables and records (weighted −0.5 on disc, −0.4545 for character 10).
fn serve_tables(iso: &mut Iso, params: &ShotParams, c: usize) -> [Vec<(Table, f32, f32)>; 2] {
    let r = params::record_of(c);
    let dw1 = serve_variant_weights(iso, c, 1);
    let turn = |rec: &[f32]| (params::spin(rec), hst_sim::ps2::mul(rec[10], 0.017453292));
    let base = character_tables(iso, c, "A", "serv", "", 4)
        .into_iter()
        .enumerate()
        .map(|(k, t)| (t, turn(params.record(0, k, r))))
        .map(|(t, (spin, side))| (t, spin, side));
    let weak = character_tables(iso, c, "B", "serv", "_dw1", 3)
        .into_iter()
        .enumerate()
        .map(|(k, t)| (t, turn(&params.variant(0, k, r, dw1[k]))))
        .map(|(t, (spin, side))| (t, spin, side));
    [base.collect(), weak.collect()]
}

/// Character `c`'s serve variant weights by kind for variant table `v` (0 `up1`, 1 `dw1`; 0 where none).
fn serve_variant_weights(iso: &mut Iso, c: usize, v: usize) -> [f32; 4] {
    let (cnf, bin) = (
        iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"),
        iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"),
    );
    let game = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
    let mut w = [0.0; 4];
    for e in game.shot_variants(c).iter().filter(|e| e.class == 0 && e.uses[v] != 0) {
        w[e.kind] = e.weight;
    }
    w
}

/// Character `c`'s shot effects, and its special slice serve's `up1` table, spin and side angle when it has one.
fn serve_specials(iso: &mut Iso, params: &ShotParams, c: usize) -> (hst_data::exe::ShotEffects, Option<(Table, f32, f32)>) {
    let (cnf, bin) = (
        iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"),
        iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"),
    );
    let effects = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc").shot_effects(c);
    let up1 = effects.up1_slice.then(|| {
        let data = iso.read(&format!("TRAJ/TRAJ{c:02}B.XB")).expect("trajectory archive on disc");
        let arc = Archive::parse(&data).expect("xb archive");
        let file = format!("tr_pc{c:02}_serv1_up1.dat");
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&file)).expect("up1 slice serve table");
        let rec = params.variant(0, 1, params::record_of(c), serve_variant_weights(iso, c, 0)[1]);
        let table = Table::parse(&arc.read(e).expect("table bytes")).expect("16^3 table");
        (table, params::spin(&rec), hst_sim::ps2::mul(rec[10], 0.017453292))
    });
    (effects, up1)
}

/// The serve bends (the character bends this kind and the special condition holds): its aim keeps the box
/// margins unscaled.
// ponytail: the kind before the stick turns a topspin flat; the original's order there is unchecked
fn serve_bent(g: &Game, who: usize, toss: Toss, kind: i32, grade: u8, offset: i32) -> bool {
    let bend = g.specials[who].0.bend;
    kind < 2 && bend[kind as usize] != 0.0 && hst_sim::shot::special(0, kind, toss == Toss::Strong, grade, offset)
}

/// The character's special-shot effect on this hit, as launched: (bend, curve, first-bounce turn). `src` is the
/// character whose shot is launched (a counter's is the incoming hitter's), `target` where the ball is sent.
fn shot_effect(g: &Game, who: usize, src: usize, class: u8, kind: i32, (branch, grade, offset): (u8, u8, i32), hit: V3, target: V3) -> (f32, f32, f32) {
    let strong = g.serving.toss == Some(Toss::Strong);
    if !hst_sim::shot::special(class, kind, strong, grade, offset) {
        return (0.0, 0.0, 0.0);
    }
    // a ground stroke or volley played with an odd motion (the other hand's side) bends the other way
    let flip = swing::launch_motion(branch, g.players[who].cmd.id as i32).0;
    hst_sim::shot::effect(&g.specials[src].0, class, kind, hit, target, g.players[who].hand < 0.0, flip)
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    root: Query<Entity, With<GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
    (match_rng, pads, weather): (Option<Res<MatchRng>>, Res<Pads>, Option<Res<crate::weather::Weather>>),
    mut gallery_used: Local<[bool; 4]>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let art = balloon_art(&mut iso, &mut images);
    let ((line_margin, margins), board, world, shot_params, umpire) = disc(&mut iso, args.stage);
    let rules = if args.singles { SINGLES } else { DOUBLES };
    let reach = reach(&mut iso);
    let mut game = Game {
        rules,
        serve_tables: Vec::new(),
        specials: Vec::new(),
        smash_tables: Vec::new(),
        smash_spins: Vec::new(),
        rally_tables: Vec::new(),
        rally_records: Vec::new(),
        timing: Vec::new(),
        timing_error: None,
        mis_hit: None,
        aim: Default::default(),
        smash_scatter: None,
        serve_data: Vec::new(),
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
        hit_flight: Flight::new(Ball { pos: [0.0; 3], vel: [0.0; 3], spin: 0.0 }, [[0.0; 4]; 4], [[0.0; 4]; 4]),
        path_base: 0,
        score: Score::new(),
        rally: Rally::default(),
        shots: 0,
        last_sweet: false,
        match_stats: hst_sim::stats::Stats::new(),
        aim_stats: Vec::new(),
        line_margin,
        margins,
        post: None,
        post_press: false,
        board,
        world,
        court: args.court.min(COURTS.len() - 1),
        message: String::new(),
        rng: match_rng.map_or_else(|| Rngs::new(Rand::default(), Mt::new(1)), |r| r.0.clone()),
        respark: None,
        sounds: Vec::new(),
        whooshes: Vec::new(),
        bounces: default(),
        smashed: false,
        whistle: (false, 0),
        flight_sound: None,
        special_serve: false,
        cam: Camera::new(),
        prev_view: Camera::new().view,
        cam_cut: false,
        cam_owner: Some(0),
        pelvis: vec![vec![[0.0, 1.0]; 48]; rules.players as usize],
        chars: vec![0; rules.players as usize],
        body_hit: None,
        voices: vec![default(); rules.players as usize],
        first_serve: true,
        serve_called: [false; 4],
        data: Vec::new(),
        post_winner: 0,
        umpire,
        umpire_voice: 0,
        gallery: default(),
        errors: 0,
        gallery_game: false,
        cheers: Vec::new(),
        applause: None,
        emitters: Vec::new(),
        humans: Vec::new(),
        jingle: None,
        music_hold: false,
        stage: args.stage.map_or(args.court, |s| s as usize) as u8,
        hit_effect: None,
        smash_heights: Vec::new(),
        reaches: Vec::new(),
        doubles_ai: default(),
        marks: Marks::default(),
        finish: default(),
    };
    reset_positions(&mut game);
    game.finish.new_point(&game.score, &game.rules);
    game.emitters = emitters(
        &mut iso,
        args.stage.map_or(args.court, |s| s as usize),
        rules.players as u32,
        &mut game.rng.court,
    );
    let n = game.players.len();

    let Ok(root) = root.single() else { return };
    // the characters on court, from the disc (each loaded once)
    let mut voices = Vec::new();
    let mut banks = Vec::new();
    let mut loaded: std::collections::HashMap<(usize, usize), std::sync::Arc<CharacterData>> =
        Default::default();
    for i in 0..n {
        // default line-up: player 1 is Carol (character 6), then characters 1, 2, 3
        let c = args.chars.get(i).copied().unwrap_or([6, 1, 2, 3][i]);
        let outfit = args.outfits.get(i).copied().unwrap_or(0);
        let data = match loaded.get(&(c, outfit)) {
            Some(d) => d.clone(),
            None => {
                let d = std::sync::Arc::new(
                    character::load_disc(
                        &mut iso,
                        c,
                        outfit,
                        &mut meshes,
                        &mut materials,
                        &mut images,
                        &mut bindposes,
                    )
                    .unwrap_or_else(|e| panic!("character {c}: {e}")),
                );
                loaded.insert((c, outfit), d.clone());
                d
            }
        };
        game.players[i].hand = character_hand(&mut iso, c);
        game.players[i].stats = character_stats(&mut iso, c);
        game.aim_stats.push(aim_stats(&mut iso, c));
        game.players[i].ai = ai_params(&mut iso, c, outfit, n == 4);
        game.players[i].ai_second = ai_second_toss(&mut iso, c);
        game.players[i].body.stamina = game.players[i].stats.stamina;
        game.pelvis[i] = data.pelvis.clone();
        game.chars[i] = c as i32;
        game.finish.power.push(popups::power(&mut iso, c));
        game.smash_heights.push(smash_heights(&mut iso, c));
        game.reaches.push(character_reach(&game.reach, &mut iso, c, game.players[i].hand));
        game.serve_tables
            .push(serve_tables(&mut iso, &shot_params, c));
        game.specials.push(serve_specials(&mut iso, &shot_params, c));
        game.rally_tables.push(rally_tables(&mut iso, c));
        game.timing.push(timing::load(&mut iso, &shot_params, c));
        game.rally_records.push(std::array::from_fn(|v| {
            std::array::from_fn(|k| shot_params.record(v + 1, k, params::record_of(c)).try_into().unwrap())
        }));
        game.smash_tables.push(character_tables(&mut iso, c, "B", "smsh", "", 2));
        game.smash_spins.push(std::array::from_fn(|k| params::spin(shot_params.record(3, k, params::record_of(c)))));
        // the a/b voice pick: 70/30 at random, players of one character kept apart (`rng::voice_bank`)
        let line_up: Vec<u32> = (0..n).map(|k| args.chars.get(k).copied().unwrap_or([6, 1, 2, 3][k]) as u32).collect();
        let drawn = game.rng.shared.r15() % 100 >= 70;
        banks.push(hst_sim::rng::voice_bank(&line_up, i, &banks, drawn));
        voices.push(voice_bank(&mut iso, c, n, game.stage as u32, banks[i]).map(std::sync::Arc::new));
        // a CPU gets an AI object, and so does a human beside a CPU partner; making one restarts the AI generator
        // ponytail: who is human is taken from the pads at setup
        let cpu = |k: usize| pads.slot_of(k, n).is_none();
        if cpu(i) || n == 4 && cpu(i ^ 2) {
            game.rng.new_ai();
        }
        // the player object places itself
        game.rng.shared.next();
        game.serve_data.push(serve_data(&mut iso, &data));
        serve_character(&mut iso, c, game.aim_stats[i], game.serve_data.last_mut().unwrap());
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
    // the hit-spark table and the gallery bank (slot 6), drawn before the sound manager is made
    // the used banks live as long as the sound manager (the whole run), so a later setup skips the banks played
    let pick = game.rng.setup_gallery(&mut gallery_used);
    let gallery = crate::audio::gallery_bank(&mut iso, game.stage as usize, pick);
    commands.insert_resource(crate::audio::GalleryBank(gallery.map(std::sync::Arc::new)));
    // the match starts (the sound manager's reseed), then its first point
    reseed_sound(&mut game);
    // the intro's lens flare ticks
    if weather.is_some_and(|w| w.today().weather < 2) {
        (0..Rngs::intro_ticks(game.stage as u32)).for_each(|_| game.rng.flare_tick());
    }
    game.rng.new_point();
    game.humans = (0..n).map(|k| pads.slot_of(k, n).is_some()).collect();
    let draws = placement_draws(&mut game, false);
    doubles_ai::formations(&mut game, &draws);
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
    commands.entity(ball).insert(crate::shade::Ball);
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
            g.players[i].formation,
            g.score.swapped,
        );
        let end = at.facing;
        // stamina is full again every point
        let body = loco::Body::new(at.pos, end, &stats);
        // the AI keeps its row and whether its team has hit yet across points
        let ai = (g.players[i].ai, g.players[i].ai_hit);
        let mind = g.players[i].ai_mind;
        let formation = g.players[i].formation;
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
        (g.players[i].ai, g.players[i].ai_hit) = ai;
        g.players[i].ai_mind = mind;
        g.players[i].formation = formation;
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

/// The match's generators as the match setup leaves them (main.rs draws the weather schedule from the shared one).
#[derive(Resource)]
pub struct MatchRng(pub Rngs);

fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut pads: ResMut<Pads>,
    mut cam: ResMut<CamMode>,
    mut cs: ResMut<CamState>,
    time: Res<Time>,
    bind: Res<controls::Bindings>,
    mut held: ResMut<controls::PadSlots>,
) {
    use controls::Action as A;
    let mut now = [SlotPad::default(); 2];
    let dirs = [(A::Up, 0x10), (A::Down, 0x40), (A::Left, 0x80), (A::Right, 0x20)];
    let shots = [(A::Normal, 0), (A::Cut, 1), (A::Lob, 3)];
    for (a, d) in dirs {
        if bind.key_pressed(&keys, a) {
            now[0].dpad |= d;
        }
    }
    now[0].shot = shots
        .into_iter()
        .find(|(a, _)| bind.key_just_pressed(&keys, *a))
        .map(|(_, kind)| kind);
    now[0].serve = bind.key_just_pressed(&keys, A::Serve);
    let mut cycle = bind.key_just_pressed(&keys, A::Camera);
    let mut turn = bind.key_pressed(&keys, A::TurnRight) as i32 as f32
        - bind.key_pressed(&keys, A::TurnLeft) as i32 as f32;
    // controllers keep their slot while connected (`controls::PadSlots`); new ones fill the first free slot
    // ponytail: Steam Input's virtual pads (Valve, 0x28de) mirror real ones; skip them so slot 2 is the second real pad
    let mut list: Vec<_> = gamepads
        .iter()
        .filter(|(_, g)| g.vendor_id() != Some(0x28de))
        .map(|(e, _)| e)
        .collect();
    list.sort();
    held.update(&list);
    for (slot, e) in held.0.iter().enumerate() {
        let Some(Ok((_, g))) = e.map(|e| gamepads.get(e)) else {
            continue;
        };
        let s = &mut now[slot];
        s.stick += pad_stick(g.left_stick());
        for (a, d) in dirs {
            if bind.pad_pressed(g, a) {
                s.dpad |= d;
            }
        }
        s.shot = s.shot.or(shots
            .into_iter()
            .find(|(a, _)| bind.pad_just_pressed(g, *a))
            .map(|(_, k)| k));
        s.serve |= bind.pad_just_pressed(g, A::Serve);
        cycle |= bind.pad_just_pressed(g, A::Camera);
        turn += pad_stick(g.right_stick()).x + bind.pad_pressed(g, A::TurnRight) as i32 as f32
            - bind.pad_pressed(g, A::TurnLeft) as i32 as f32;
    }
    pads.connected = held.connected();
    for (slot, n) in pads.slots.iter_mut().zip(now) {
        slot.stick = n.stick;
        slot.dpad = n.dpad;
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
    let weak = (g.serving.toss == Some(Toss::Weak) && serve::dw1(&g.serve_data[who])) as usize;
    // a slow contact's mis-hit roll; a framed hit lobs to a wild spot
    let miss = g.mis_hit.take().filter(|_| class == 1 || class == 2);
    let framed = miss.filter(|m| m.wild).map(|_| timing::wild(g, who));
    let (kind, target) = framed.map_or((kind, target), |f| (3, f.0));
    // ponytail: the counter's swing-start z is taken at the strike
    let inside_high = kind < 2 && g.players[who].pos[2].abs() < 6.4 && at[1] >= 0.6;
    let power_gap = g.finish.strike(who, branch, kind, offset, g.players.len(), inside_high);
    // a counter launches from the incoming hitter's tables and shot record, as the original
    let src = match power_gap {
        Some(_) if g.last_hitter >= 0 => g.last_hitter as usize,
        _ => who,
    };
    // a rally shot's aim is pulled inside the lines before anything uses it (the timing scatter comes after);
    // ponytail: the smash's deep mode and the powered shots' unscaled margins are not modelled (low, special false)
    let target = if class == 0 {
        target
    } else {
        let doubles = g.rules.players > 2;
        g.margins.inside(class, kind, false, doubles, g.chars[src] as usize, false, at, target)
    };
    let error = g.timing_error.take().or(framed.map(|_| swing::TimingError::default()));
    let timed = error.filter(|_| class == 1 || class == 2).map(|e| {
        timing::launch(g, who, src, class, kind, (branch, grade), e, framed.map(|f| (f.1, f.2)), power_gap.is_some(), at, target)
    });
    let target = timed.as_ref().map_or(target, |t| t.target);
    // a smash's timing scatter, its table looked up from the aim (see `timing::smash`)
    let smash = g.smash_scatter.take().filter(|_| class == 3).map(|(sx, sz)| (target, serve::scatter_along(sx, sz, at, target)));
    let target = smash.map_or(target, |(t, sc)| [hst_sim::ps2::add(t[0], sc[0]), t[1], hst_sim::ps2::add(t[2], sc[2])]);
    let sent = if class == 0 {
        let s = g.serving.scatter;
        [hst_sim::ps2::add(target[0], s[0]), target[1], hst_sim::ps2::add(target[2], s[2])]
    } else {
        target
    };
    let effect = shot_effect(g, who, src, class, kind, (branch, grade, offset), at, sent);
    let launched = if class == 0 {
        // the special slice serve flies by its up1 table
        let (table, spin, side) = match &g.specials[who].1 {
            Some(up1) if kind == 1 && effect.0 != 0.0 => up1,
            _ => &g.serve_tables[who][weak][kind as usize],
        };
        serve::launch(
            table,
            kind == 3,
            hst_sim::ball::Params::default().radius,
            at,
            target,
            g.serving.scatter,
            (*spin, *side, g.players[who].hand < 0.0),
            effect.0,
        )
    } else {
        let l = if let Some((aim, sc)) = smash {
            lookup(&g.smash_tables[who][kind as usize], &Bounds::smash(kind), [hst_sim::ps2::sub(at[0], sc[0]), at[1], hst_sim::ps2::sub(at[2], sc[2])], aim)
        } else if class == 3 {
            lookup(
                &g.smash_tables[who][kind as usize],
                &Bounds::smash(kind),
                at,
                target,
            )
        } else if let Some(t) = &timed {
            t.lookup
        } else {
            rally_lookup(g, src, class, kind, at, target)
        };
        let l = hst_sim::shot::Lookup { elevation: hst_sim::ps2::mul(l.elevation, miss.map_or(1.0, |m| m.scale)), ..l };
        rally_launch(class, at, target, &l)
    };
    let vel = launched.vel;
    g.shot = Shot {
        class,
        kind,
        curve_frames: launched.frames,
        wind: [launched.wind[0], launched.wind[1], launched.wind[2]],
        ..Shot::default()
    };
    (g.shot.bend, g.shot.curve, g.shot.bounce_turn) = effect;
    let side = hst_sim::shot::side_axis(at, sent);
    g.shot.side = [side[0], side[1], side[2]];
    g.shots += 1;
    g.rally.on_hit(
        g.shots,
        who as i32,
        g.score.server,
        g.score.receiver,
        g.flight.contacts,
    );
    let hit = sound::Hit {
        power_gap,
        branch,
        grade,
        offset,
        kind,
        hits: g.shots,
        strong_toss: g.serving.toss == Some(Toss::Strong),
        solo: g.rules.players == 1,
        // the sound generator's bit is drawn only for an unclean stroke (not a smash or drop)
        random_bit: branch != 4 && kind != 4 && !matches!(grade, 1 | 2) && g.rng.sound.bit(),
        framed: framed.is_some(),
        dull: miss.is_some_and(|m| m.dull),
        ..default()
    };
    g.sounds
        .extend(sound::hit_sounds(&hit).into_iter().map(|p| (p, at)));
    let n = g.players.len() as u32;
    // the launch's fresh random (+0x3b9c, read only by the close-up camera's quiet-stroke shout)
    g.rng.shared.next();
    if let Some(program) =
        sound::stroke_shout(&hit, g.chars[who], n, || g.rng.shared.r15() % 100)
    {
        let r = g.rng.shared.r15();
        let shout = g.voices[who].shout(who, program, n, r);
        g.whooshes.push((0, who, shout));
    }
    g.smashed = branch == 4 && g.rules.players > 1;
    g.last_sweet = branch != 3 && offset.abs() < 2;
    match_stats::hit(g, who, branch, grade, offset);
    flight_sound(g, who, &hit, class, vel);
    g.whistle = if g.flight_sound.is_some() {
        (true, g.whistle.1 + 1)
    } else {
        (false, g.whistle.1)
    };
    let spin = match class {
        0 => launched.spin,
        3 => g.smash_spins[who][kind as usize],
        _ => match timed.as_ref().and_then(|t| t.record) {
            Some(r) => record_spin(g, &r, class, kind, at, target),
            None => rally_spin(g, src, class, kind, at, target),
        },
    };
    // the ball launch's two uniforms (kept on the ball; nothing ported reads them)
    (g.rng.shared.next(), g.rng.shared.next());
    g.flight = Flight::new(Ball { pos: at, vel, spin }, launched.frame, launched.frame);
    ai_heard_hit(g, who, branch, vel);
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
    g.hit_flight = g.flight;
    g.path_base = 0;
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
            serve_call(g, i, s.toss.unwrap());
        } else {
            // walk the baseline: a human by the pad's run direction (its dead square, camera flip), a bot by its stick
            let walk = if g.humans.get(i) == Some(&true) {
                pad_run(g, g.players[i].aim, g.players[i].dpad)
            } else {
                stick
            };
            let doubles = g.rules.players > 2;
            let p = &mut g.players[i];
            let (x, m) = loco::serve_walk(pos[0], [walk.x, walk.y], end, g.score.side, doubles, p.hand);
            p.stride += (x - pos[0]).abs() * 9.0;
            p.pos[0] = x;
            set_motion(p, m, 1.0, true, None);
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
            let (hand, apex) = serve::toss_points(&g.serve_data[i], toss, &player_matrix(&g.players[i]));
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
            // a fresh path: last point's net/special count would read the toss as bounced and whiff every swing
            g.path_base = 0;
            g.prev_ball = hand;
            g.serving.tossed = true;
            toss_draws(g);
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
                let grades = g.serve_data[i].grades(toss).to_vec();
                match serve::search(&g.serve_data[i], toss, &predicted_path(g, grades.len())) {
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
                        let r = g.rng.shared.r15();
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
        let coins = || g.rng.shared.bit();
        let d = &g.serve_data[i];
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
        let target = serve::inside(pos[0], hit, target, serve_bent(g, i, toss, sw.kind, sw.grade, sw.offset));
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
        let p = &mut g.players[i];
        p.served = Some(0);
        p.recover = motion::serve_recovery(toss == Toss::Strong, sw.kind);
        // the swing plays on at speed 1 from contact
        set_motion(p, motion::serve_swing(toss == Toss::Under, 1).0, 1.0, false, None);
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

/// A stick axis (−1..1, +1 right/up) as the pad's byte (0x00 full left/up, 0x80 centre, 0xff full right/down;
/// pass −y for a vertical axis).
pub(crate) fn stick_byte(v: f32) -> u8 {
    (128.0 + v.clamp(-1.0, 1.0) * if v < 0.0 { 128.0 } else { 127.0 }).round() as u8
}

/// A stick through the original's pad driver: each axis as its byte, the driver's deadzone
/// (`hst_sim::player::pad_deadzone`), and back. Everything after it (running, aim, the camera turn) reads this, as
/// the game reads the bytes; `pad_run` takes the run/aim dead square from there.
fn pad_stick(v: Vec2) -> Vec2 {
    let axis = |v: f32| {
        let b = hst_sim::player::pad_deadzone(stick_byte(v)) as f32 - 128.0;
        b / if b < 0.0 { 128.0 } else { 127.0 }
    };
    Vec2::new(axis(v.x), -axis(-v.y))
}

/// Whether player `i`'s serve aim comes from a stick (humans) rather than the stand-in AI's pick.
fn pads_aim(g: &Game, i: usize) -> bool {
    g.serving.bot_due.is_none() || i != g.score.server as usize
}

/// The toss's ball launch draws the ball's two shared uniforms, as every launch does (kept on the ball; nothing
/// ported reads them).
fn toss_draws(g: &mut Game) {
    (g.rng.shared.next(), g.rng.shared.next());
}

/// A new point's placement message: one shared draw per player, right after the reseed.
/// In singles the server then voices program 6 (keys 0..1, memory slot 2) on the match's first serve, and after a
/// change of ends (`ends`) half the time.
/// Returns the players' draws, in order (the match sends it to them as they were made); the first point's picks the
/// formations from them (`doubles_ai::formations`).
fn placement_draws(g: &mut Game, ends: bool) -> Vec<u32> {
    if g.first_serve {
        g.serve_called = [false; 4];
    }
    let mut draws = Vec::new();
    for i in 0..g.players.len() {
        draws.push(g.rng.shared.next());
        if g.players.len() == 2
            && i as i32 == g.score.server
            && (g.first_serve || ends && g.rng.shared.r15() % 100 < 50)
            && let Some(play) = g.voices[i].react(i, (6, 0, 1, Some(2)), || g.rng.shared.r15())
        {
            g.whooshes.push((0, i, play));
        }
    }
    g.first_serve = false;
    draws
}

/// A strong toss on the server's team's match point (`Score::match_point`): in doubles the server's partner, in
/// singles the server, calls program 6 key 2 (one shared key draw), once a match.
fn serve_call(g: &mut Game, server: usize, toss: Toss) {
    let n = g.players.len();
    if toss != Toss::Strong || g.score.match_point(&g.rules) != Some(server & 1) {
        return;
    }
    let caller = match n {
        2 => server,
        4 => server ^ 2,
        _ => return,
    };
    if std::mem::replace(&mut g.serve_called[caller], true) {
        return;
    }
    if let Some(play) = g.voices[caller].react(caller, (6, 2, 2, None), || g.rng.shared.r15()) {
        g.whooshes.push((0, caller, play));
    }
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
/// The player's matrix (rows, the spot in row 3): turned to face, mirrored across for a left-hander.
fn player_matrix(p: &Player) -> [[f32; 4]; 4] {
    let [fx, _, fz, _] = p.body.face.dir;
    [
        [p.hand * fz, 0.0, -(p.hand * fx), 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [fx, 0.0, fz, 0.0],
        [p.pos[0], p.pos[1], p.pos[2], 1.0],
    ]
}

fn held_ball(mut g: ResMut<Game>, q: Query<(&Figure, &Motion)>) {
    if g.phase != Phase::Serve || g.serving.tossed {
        return;
    }
    let i = g.score.server as usize;
    let Some((_, m)) = q.iter().find(|(f, _)| f.0 == i) else {
        return;
    };
    let (p, data) = (&g.players[i], &g.data[i]);
    let player = player_matrix(p);
    let rows = [player[0], player[1], player[2]];
    let t = m.clock.sampled;
    let sk = &data.skeleton;
    let at = if m.id == 0x20 {
        data.stance_ball
            .as_ref()
            .map(|b| serve::stance_ball(b.at(t), rows, p.pos))
    } else {
        let finger = sk.names.iter().position(|n| n == "Bip01LFinger21");
        data.motions.get(&m.id).zip(finger).map(|(c, f)| {
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

/// The run direction the original gives a human's stick (`hst_sim::player::pad_dir`): the stick as pad bytes, its
/// dead square, rescale and ×1.2 clip, or the d-pad bits (`dpad`, full length on diagonals) while the stick is
/// dead, on world axes, turned by π when the camera looks from the +z side. ponytail: world axes also under the
/// free camera
fn pad_run(g: &Game, stick: Vec2, dpad: u16) -> Vec2 {
    // 0x00 full left/up, 0x80 centre, 0xff full right/down
    let byte =
        |v: f32| (128.0 + v.clamp(-1.0, 1.0) * if v < 0.0 { 128.0 } else { 127.0 }).round() as u8;
    let phase = if matches!(g.phase, Phase::ChangeEnds(_)) {
        1
    } else {
        3
    };
    let [x, z] = loco::pad_dir(dpad, byte(stick.x), byte(-stick.y), phase);
    if g.cam.view.eye[2] >= 0.0 {
        Vec2::new(-x, -z)
    } else {
        Vec2::new(x, z)
    }
}

/// Analog aim on the court (x, z direction from `pad_run`, or a bot's random pick): sideways spans the court,
/// +z moves the target toward +z (deeper for the −z team, shorter for the other); centred is a deep middle ball.
fn aim_target(g: &mut Game, i: usize, stick: Vec2, branch: u8, kind: i32, offset: i32, body: bool, height: f32) -> V3 {
    let p = &g.players[i];
    let h = hst_sim::shot::Hitter { end: p.end, branch, kind, offset, body, from: p.aim_from, height };
    // the ball being struck was a slice (not a smash) in a rally under way: the angle narrows
    let incoming = (g.shots > 0 && g.shot.class != 3 && g.shot.kind == 1).then_some(g.last_sweet);
    // ponytail: bots aim through this too with a random stick; the original AI's own aim is P11
    g.aim = hst_sim::shot::aim(&h, &g.aim_stats[i], [stick.x, stick.y], g.rules.players == 4, false, incoming, &mut || g.rng.shared.next());
    g.aim.target
}

/// The ball's predicted path for the contact search: this frame's ball, then one step per frame. As the
/// original's, it is stepped against the court only (never the net) and counts court contacts; once the ball
/// has touched the net the original re-seeds it from the ball but keeps the contacts its old prediction had
/// reached (`path_base`), so a net cord in a rally is past every contact search and goes unplayed.
fn predicted_path(g: &Game, frames: usize) -> Vec<PathPoint> {
    let mut f = g.flight;
    f.net = false;
    let mut path = Vec::with_capacity(frames);
    for _ in 0..frames {
        path.push(PathPoint { pos: f.ball.pos, bounces: g.path_base + f.contacts });
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
    let path = predicted_path(g, g.reaches[i].grades.len());
    let s = swing::search(&g.reaches[i], &path, p.pos, p.end, p.kind)?;
    Some(Contact {
        frames: s.frame as u32,
        swing: s,
        grade: g.reaches[i].grades[s.frame],
        offset: s.frame as i32 - SWEET_FRAME,
    })
}

/// The auto-approach a press runs first (`swing::approach`): frames to run and the direction, None when nothing
/// comes within reach in 20 frames of running or the mover blocks the run.
/// ponytail: the app's one reach (character 0's) for everyone, as `find_contact`
fn approach(g: &Game, i: usize) -> Option<(usize, Vec2)> {
    let p = &g.players[i];
    let path = predicted_path(g, g.reach.grades.len() + 20);
    let mate = (g.players.len() == 4).then(|| g.players[i ^ 2].pos);
    let (s, n) = (p.stats, g.players.len() as i32);
    let (mut run, mut stamina, mut tick) = (if p.body.running { p.body.run } else { 0 }, p.body.stamina, p.body.stamina_tick);
    let (end, y) = (p.end, p.pos[1]);
    swing::approach(&g.reach, &path, p.pos, p.end, |at, d| {
        let speed = loco::run_speed(&s, run, stamina, 100);
        let delta = [hst_sim::ps2::mul(d[0], speed), 0.0, hst_sim::ps2::mul(d[1], speed)];
        let to = loco::mover([at[0], y, at[1]], delta, end, mate, false);
        run += 1;
        (stamina, tick) = loco::drain(&s, stamina, tick, n, true, 0);
        let free = to[0] == hst_sim::ps2::add(at[0], delta[0]) && to[2] == hst_sim::ps2::add(at[1], delta[2]);
        free.then_some([to[0], to[2]])
    })
    .map(|(f, d)| (f, Vec2::new(d[0], d[1])))
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
        &g.reaches[i],
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
    if bodyhit::standing(g, i) {
        g.players[i].prev = g.players[i].pos;
        return;
    }
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
        if left > 0 {
            g.players[i].pending = Some(left - 1);
        } else if let Some(c) = find_contact(g, i) {
            let p = &mut g.players[i];
            p.pending = None;
            p.contact = Some(c);
            p.hit_at = Some(c.swing.ball);
            p.aim_from = [p.pos[0], p.pos[2]];
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
            p.aim_from = [p.pos[0], p.pos[2]];
            let thud = sound::dive_thud(crate::weather::now());
            g.whooshes.extend([
                (0, i, thud),
                (sound::dive_echo(g.players.len() as u32), i, thud),
            ]);
            let r = g.rng.shared.r15();
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
            g.players[i].pending = None;
            let quiet = g.players[i].whiff_quiet;
            whiff(g, i, true, quiet);
        }
        if g.players[i].pending.is_none() {
            g.players[i].approach = None;
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
            let target = aim_target(g, i, stick, branch, kind, c.offset, c.swing.body, c.swing.ball[1]);
            // a smash has its own class and kinds: △ (the lob button) smashes kind 1, the others kind 0
            let (class, kind) = if branch == 4 {
                (3, (kind == 3) as i32)
            } else {
                (if branch == 2 { 2 } else { 1 }, kind)
            };
            g.timing_error = (branch < 3).then(|| timing::error(g, i, &c, branch, g.players[i].kind == 3));
            g.smash_scatter = (branch == 4).then(|| timing::smash(g, i, &c, kind == 0));
            g.mis_hit = (1..4).contains(&branch).then(|| timing::mis_hit(g, i, (branch, c.grade, kind), c.swing.forehand));
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
        let (grade, offset) = swing::dive_lock(d.frame);
        let target = aim_target(g, i, stick, 3, kind, offset, false, 0.0);
        g.timing_error = Some(timing::dive_error(g, i, &d, g.players[i].kind == 3));
        g.mis_hit = Some(timing::mis_hit(g, i, (3, grade, kind), true));
        strike(
            g,
            i,
            2,
            kind,
            target,
            (3, grade, offset),
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

/// One frame of the server's follow-through: past its recovery the stick (`stick`; presses are ignored) or the
/// swing's end hands off to standing or running from the next frame. Whether the server is still held this frame.
fn serve_follow(p: &mut Player, stick: bool) -> bool {
    let Some(a) = p.served.map(|a| a + 1) else {
        return false;
    };
    p.prev = p.pos;
    p.served = (!motion::serve_over(a, p.recover, p.played, stick)).then_some(a);
    if p.served.is_none() {
        debug!(
            "serve follow-through over {a} frames after contact ({})",
            if p.played { "played out" } else { "broken off" }
        );
    }
    true
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

/// A shot button press: the auto-approach (`approach`) may first run the player toward the ball's line for a few
/// frames, then the contact is searched once; nothing found dives or swings at nothing, as the original. With no
/// ball for this player to hit the player swings at nothing at once.
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
            let (frames, dir) = approach(g, i).map_or((0, None), |(n, d)| (n as u32, Some(d)));
            g.players[i].pending = Some(frames);
            g.players[i].approach = dir.filter(|_| frames > 0);
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
    let middle = (g.reaches[i].smash_top + g.reaches[i].smash_bottom) / 2.0;
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
            let r = g.rng.shared.r15();
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
                doubles_ai::human(g, i);
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
    g.players[i].dpad = pad.dpad;
    if g.phase == Phase::Serve && g.score.server == i as i32 {
        let press = shot.or(serve_press.then_some(0));
        // the serve aims with the run direction, as the original (its dead square, ×1.2 clip, camera flip)
        let stick = pad_run(g, pad.stick, pad.dpad);
        serve_turn(g, i, stick, press);
        return;
    }
    let moved = pad.stick != Vec2::ZERO || pad.dpad != 0;
    let stick = pad_run(g, pad.stick, pad.dpad) != Vec2::ZERO;
    if serve_follow(&mut g.players[i], stick) {
        return;
    }
    follow_through(&mut g.players[i], shot.is_some() || moved);
    whiff_frame(g, i, moved);
    if let Some(kind) = shot {
        press(g, i, kind);
    }
    // screen-relative: stick right follows the camera's right, stick up its ground-forward; nothing moves the
    // player (or sets its motion) through the swing
    let p = &g.players[i];
    if p.contact.is_none() && p.whiff.is_none() && p.swing.is_none() && p.dive.is_none() {
        let dir = p.approach.unwrap_or_else(|| pad_run(g, pad.stick, pad.dpad));
        locomote(g, i, dir);
    }
    // the stick at the moment of contact aims the shot: the run direction, as the original
    let aim = pad_run(g, pad.stick, pad.dpad);
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

/// A computer player draws its timing errors (`hst_sim::ai`), given the opponents' shots when one just hit.
/// A doubles AI beside a human partner takes a third of the smash base.
fn ai_draw(g: &mut Game, i: usize, shots: Option<hst_sim::ai::Shots>) {
    let (receiver, first) = (g.score.receiver == i as i32, !g.players[i].ai_hit);
    let third = g.players.len() == 4 && g.humans.get(i ^ 2) == Some(&true);
    let rng = &mut g.rng.ai;
    g.players[i].ai_timing = g.players[i].ai.timing(shots.as_ref(), receiver, first, third, &mut || rng.next());
    g.players[i].surprised |= g.players[i].ai_timing.reacted;
    let dive = shots.is_none() && g.players[i].ai_dove.take().unwrap_or(true);
    let kept = g.players[i].ai_picks.dive;
    g.players[i].ai_picks = g.players[i].ai.picks(dive, &mut ai_roll(&mut g.rng));
    // an undrawn dive chance keeps the last one
    g.players[i].ai_picks.dive = g.players[i].ai_picks.dive.or(kept);
    if shots.is_none() {
        // no opponent's hit to react to: no reaction, no guess
        (g.players[i].ai_hold, g.players[i].ai_guess) = (0, None);
    }
}

/// The AI's hit message: every computer player remembers an opponent's shot (or notes its own team's hit) and
/// redraws its timing errors. `branch` is the hitter's swing branch code.
fn ai_heard_hit(g: &mut Game, who: usize, branch: u8, vel: V3) {
    // the AI's shot kinds are the hitter's branch: 0 serve, 1 ground stroke, 2 volley, 3 dive, 4 smash
    let kind = branch as i32;
    let lob = branch != 0 && g.players[who].kind == 3;
    for i in 0..g.players.len() {
        if g.humans.get(i) == Some(&true) {
            continue;
        }
        let again = doubles_ai::heard_hit(g, i);
        ai_heard_shot(g, i, i & 1 == who & 1, again);
        let p = &mut g.players[i];
        let shots = if i & 1 != who & 1 {
            let last = hst_sim::ai::Seen { kind, vel };
            if kind == 0 {
                p.ai_serves = [Some(vel), p.ai_serves[0]];
            }
            p.ai_seen = [Some(last), p.ai_seen[0]];
            Some(hst_sim::ai::Shots { last, before: p.ai_seen[1], serve_before: p.ai_serves[1] })
        } else {
            p.ai_hit = true;
            p.ai_dove = Some(who == i && branch == 3);
            None
        };
        ai_draw(g, i, shots);
        if shots.is_some() {
            ai_draw_guess(g, i, kind, lob);
        }
    }
}

/// A computer player's reaction and guess (ヤマ張り) after an opponent's hit, drawn after its timing errors and
/// shot-choice draws: it stands for the reaction frames (a guess runs for its move frames instead).
fn ai_draw_guess(g: &mut Game, i: usize, kind: i32, lob: bool) {
    let special_serve = g.special_serve;
    let beside_human = g.players.len() == 4 && g.humans.get(i ^ 2) == Some(&true);
    let rng = &mut g.rng.ai;
    let mut roll = || rng.next();
    let p = &mut g.players[i];
    let last = hst_sim::ai::Seen { kind, vel: [0.0; 3] };
    p.ai_hold = p.ai.reaction(&p.ai_timing, &last, lob, special_serve, beside_human, p.pos[2].abs() <= 6.4, &mut roll);
    let guess = p.ai.guess(&p.ai_timing, kind == 0, !beside_human, &mut roll);
    p.ai_guess = guess.map(|q| (q, [p.pos[0], p.pos[2]]));
    if guess.is_some() {
        p.ai_hold = p.ai.guess[1];
    }
}

/// After an opponent's hit the AI stands for its reaction frames (`ai_hold` without a guess). A guessing AI first runs sideways toward the guessed half for the move frames
/// (stopping once there), then judges the guess against its contact point: right, it times the hit within a frame;
/// wrong, it stands still for the stuck frames before it goes for the ball. Some(goal) while it holds.
/// ponytail: the original's run boost after a right guess (×2, ×1.5 with the body-shot roll) is left out until the
/// AI's own approach run (P7f).
fn ai_guessing(g: &mut Game, i: usize, coming: bool, contact: Option<V3>) -> Option<V3> {
    use hst_sim::ai::Verdict;
    let (ad, p) = (g.score.side == 1, g.players[i]);
    if !coming {
        return None;
    }
    if p.ai_hold > 0 {
        g.players[i].ai_hold -= 1;
        let Some((q, _)) = p.ai_guess else { return Some(p.pos) };
        let goal = [q.target_x(p.end, ad), 0.0, p.pos[2]];
        if (goal[0] - p.pos[0]).abs() < 0.1 {
            g.players[i].ai_hold = 0; // there
        }
        return Some(goal);
    }
    let ((q, from), b) = (p.ai_guess?, contact?);
    g.players[i].ai_guess = None;
    match q.verdict(p.end, from, [b[0], b[2]]) {
        Verdict::Right => {
            let e = g.rng.ai.r15() as i32 % 3 - 1;
            let t = &mut g.players[i].ai_timing;
            (t.stroke, t.volley, t.smash) = (e, e, e);
            None
        }
        Verdict::Wrong => {
            g.players[i].surprised = true;
            g.players[i].ai_hold = p.ai.guess[2];
            Some(p.pos)
        }
        Verdict::Neither => None,
    }
}

/// A computer player's run toward `goal` as the game passes it on: the AI's step toward it (none within ⅔ of a
/// step), as stick bytes and back.
fn bot_run(g: &Game, i: usize, goal: V3) -> Vec2 {
    let p = &g.players[i];
    let phase = match g.phase {
        Phase::ChangeEnds(_) => 1,
        Phase::Serve => 2,
        Phase::Rally => 3,
        Phase::Post => 4,
    };
    let body = loco::Body { pos: p.pos, ..p.body };
    let (d, _) = body.toward(&p.stats, [goal[0], goal[2]], g.players.len() as i32, phase, false);
    let [x, z] = loco::stick_dir(loco::bot_stick(d), phase);
    Vec2::new(x, z)
}

/// The AI's stick through its serve's follow-through (`motion::ai_serve_stick`); the return's own frame is the
/// fresh flight's.
/// True once the wait is over: then the rally step's own stick decides (`ai_serve_step`).
fn ai_serve_stick(g: &mut Game, i: usize) -> bool {
    let p = &mut g.players[i];
    let Some(a) = p.served.map(|a| a + 1) else { return false };
    motion::ai_serve_stick(a, p.recover, g.shots, g.flight.frame == 0, &mut p.ai_hold)
}

/// The rally step's stick `dir` ends a computer server's follow-through once its wait is over; none (the step
/// zeroed within ⅔ of a step) keeps it playing out. Whether the server is still held this frame.
/// ponytail: the goal is the stand-in's; the original's sub-state 0 picks a contact chase (keep on, sub-state 1) or a
/// spot helper (keep off) or nothing, which can put the first stick a frame later (B27d)
fn ai_serve_step(g: &mut Game, i: usize, dir: Vec2) -> bool {
    g.players[i].served.is_some() && serve_follow(&mut g.players[i], dir != Vec2::ZERO)
}

/// Stand-in AI for player `i`: serves, runs to the predicted interception (in doubles only the teammate
/// nearer to it; the other goes home), and presses the shot button around the sweet frame with a random timing
/// error, through the same contact search as a human.
fn bot(g: &mut Game, i: usize) {
    if let Some(c) = &g.players[i].contact {
        g.players[i].ai_branch = Some(c.swing.branch);
    }
    let was = g.players[i].ai_mind.map(|m| m.phase);
    let mind = ai_update(g, i);
    // the new-point message's tick runs nothing else, nor does the serve state's entry (the serve routine first
    // runs the tick after)
    if mind.phase == hst_sim::ai::Phase::Start || mind.phase == hst_sim::ai::Phase::Serve && was == Some(hst_sim::ai::Phase::Start) {
        return;
    }
    if mind.phase == hst_sim::ai::Phase::Serve {
        return bot_serve(g, i);
    }
    // past its wait the rally step below decides the break-off (`ai_serve_step`)
    let stick = ai_serve_stick(g, i);
    if !stick && serve_follow(&mut g.players[i], false) || doubles_ai::step(g, i, &mind) {
        return;
    }
    // ponytail: the stand-in AI always wants to move on, so it breaks off at the recovery
    follow_through(&mut g.players[i], true);
    whiff_frame(g, i, true);
    // the press's approach run (set on an earlier frame; the bot's own walk waits while it's pending)
    if let Some(dir) = g.players[i].approach {
        locomote(g, i, dir);
    }
    let busy = g.players[i].contact.is_some()
        || g.players[i].pending.is_some()
        || g.players[i].swing.is_some()
        || g.players[i].whiff.is_some()
        || g.players[i].dive.is_some();
    // the reaction runs down after an opponent's hit, through its own follow-through too
    let opp_hit = g.phase == Phase::Rally && g.last_hitter >= 0 && g.last_hitter & 1 != i as i32 & 1;
    if busy && opp_hit && g.players[i].ai_hold > 0 {
        g.players[i].ai_hold -= 1;
    }
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
        // the original hands the ball over once its reaction has run down, from the second shot on, and calls
        // only while the partner isn't swinging or diving
        if plan.is_some()
            && mine.is_none()
            && g.players.len() == 4
            && g.shots >= 2
            && g.players[i].ai_hold == 0
            && g.players[i].bot_left != g.shots
        {
            g.players[i].bot_left = g.shots;
            if !ai_mate_busy(g, i) && g.rng.ai.r15() % 100 < 25 {
                let call = sound::call_out(i, g.rng.ai.bit());
                g.whooshes.push((0, i, call));
            }
        }
        let guessing = ai_guessing(g, i, opp_hit, mine.map(|(b, _)| b));
        let mine = if guessing.is_some() { None } else { mine };
        let wait = if mine.is_none() {
            guessing.or_else(|| ai_wait(g, i)).or_else(|| ai_wait_singles(g, i))
        } else {
            None
        };
        let stand = mine.map(|(b, _)| ai_stand_x(g, i, b));
        let p = g.players[i];
        let goal = mine.map_or(wait.unwrap_or(p.home), |(b, _)| {
            [
                stand.unwrap_or(b[0]),
                0.0,
                b[2] - p.end * g.reach.ahead,
            ]
        });
        let dir = bot_run(g, i, goal);
        let dir = if mind.active { dir } else { Vec2::ZERO };
        if ai_serve_step(g, i, dir) {
            return;
        }
        locomote(g, i, dir);
        // press when the contact search would lock onto the drawn frame (or later, if it is already past)
        if mine.is_some() {
            if g.players[i].bot_due.is_none() {
                g.players[i].bot_due = Some(SWEET_FRAME as usize);
            }
            // the AI presses once the frames left plus its timing error for this stroke reach the sweet frame
            let t = g.players[i].ai_timing;
            let go = ai_lets_go(g, i);
            if !go && find_contact(g, i).is_some_and(|c| {
                let err = match c.swing.branch {
                    swing::Branch::Ground => t.stroke,
                    swing::Branch::Volley => t.volley,
                    swing::Branch::Smash => t.smash,
                };
                c.frames as i32 + err <= SWEET_FRAME
            }) {
                let kind = ai_press_kind(g, i);
                press(g, i, kind);
                g.players[i].bot_due = None;
            } else if !go && ai_dives(g, i) {
                let kind = ai_press_kind(g, i);
                press(g, i, kind);
                g.players[i].bot_due = None;
            }
        } else {
            g.players[i].bot_due = None;
        }
    }
    advance_stroke(g, i, |g| ai_contact_stick(g, i));
}

/// Whether a computer player dives for its ball: it drew the dive chance (`AiParams::picks`), nothing it can
/// stroke is in reach, and the dive search would find the ball (the original's check runs the same scan as
/// the dive itself, on its own path and frame count, capped at the dive's 16).
/// ponytail: "nothing in reach" is the app's press search and 20-frame approach; the original's own contact
/// search per level (P11i) decides it
fn ai_dives(g: &Game, i: usize) -> bool {
    g.players[i].ai_picks.dive == Some(true)
        && find_contact(g, i).is_none()
        && approach(g, i).is_none()
        && find_dive(g, i).is_some()
}

/// Whether `i`'s partner is mid-stroke or diving (its locomotion state past running): no call-out then.
fn ai_mate_busy(g: &Game, i: usize) -> bool {
    let m = &g.players[i ^ 2];
    m.contact.is_some() || m.pending.is_some() || m.swing.is_some() || m.whiff.is_some() || m.dive.is_some()
}

/// The x a computer player runs to for ball `b`: the side of it the AI's contact search keeps (`hst_sim::ai::stand_side`,
/// the run-round roll drawn on the first search for this shot), a reach away from the ball.
/// ponytail: the stand-in `intercept` finds one contact point, so both sides share it (and their depth: 0 here);
/// the original searches each side's best point of the path, and draws the roll on every search until one is found.
fn ai_stand_x(g: &mut Game, i: usize, b: V3) -> f32 {
    let (p, reach) = (g.players[i], g.reaches[i].reach);
    let minus = match p.ai_side {
        Some((shot, minus)) if shot == g.shots => minus,
        _ => {
            let beside_human = g.players.len() == 4 && g.humans.get(i ^ 2) == Some(&true);
            let rng = &mut g.rng.ai;
            let run = p.ai.run_round(beside_human, &mut || rng.next());
            let width = run.then(|| p.ai.run_round_width(g.players.len() == 2));
            // the strong side is TParam's hand (the game's +0x12dc ignores the select-screen hand toggle)
            let strong = if g.reaches[i].hand >= 0.0 { 1 } else { 2 };
            let spot = Some((1, [b[0], b[2]]));
            let at = [p.pos[0], p.pos[2]];
            let minus = hst_sim::ai::stand_side(spot, spot, at, reach, p.end, 0.0, strong, width) == Some(true);
            g.players[i].ai_side = Some((g.shots, minus));
            minus
        }
    };
    if minus { b[0] - reach * p.end } else { b[0] + reach * p.end }
}

/// A doubles bot's goal while the ball isn't its own (`hst_sim::position`): its formation spot, walked to once
/// the centre roll passes and until it's inside the centre radius (its own position otherwise). It takes its place
/// at the start of a point and looks again when its team hits and every 30 frames after. None in singles (the
/// serve spot stays the goal).
/// ponytail: the original looks from the partner's shot record (where it means to hit) and also when the partner
/// shapes for a volley at the net (`Formation::hold`); this uses the hitter's position and skips the hold.
fn ai_wait(g: &mut Game, i: usize) -> Option<V3> {
    use hst_sim::position::{Cue, Formation, Return, Team};
    if g.players.len() != 4 {
        return None;
    }
    let mate = i ^ 2;
    let human_mate = g.humans.get(mate) == Some(&true);
    let (p, m) = (g.players[i], g.players[mate]);
    // beside a human the row's formation and doubles centre numbers, beside a bot staggered and the singles ones
    let (rate, radius) = if human_mate {
        (p.ai.doubles_center_rate, p.ai.doubles_center_radius)
    } else {
        (p.ai.singles_center_rate, p.ai.singles_center_radius)
    };
    let formation = p.formation;
    let t = Team { side: p.end, formation, lean: Team::lean(formation, !human_mate, p.ai.style, m.ai.style) };
    let at = |q: &Player| [q.pos[0], q.pos[2]];
    let rng = &mut g.rng.ai;
    let mut roll = || rng.next();
    let pl = &mut g.players[i];
    if pl.ai_form.lane == 0 {
        let starter = g.score.server == i as i32 || g.score.receiver == i as i32;
        pl.ai_form = Formation::start(&t, starter, g.score.side == 1);
        pl.ai_back = Return::new(rate, &mut roll);
        pl.ai_shot = g.shots;
    }
    let ours = g.last_hitter >= 0 && g.last_hitter & 1 == i as i32 & 1;
    let hit = std::mem::replace(&mut pl.ai_shot, g.shots) != g.shots;
    pl.ai_tick = if hit { 0 } else { pl.ai_tick + 1 };
    if ours && (hit || pl.ai_tick >= 30) {
        pl.ai_tick = 0;
        let cue = if g.last_hitter == i as i32 { Cue::Me(at(&m)) } else { Cue::Other(at(&g.players[g.last_hitter as usize])) };
        let ball_z = g.flight.ball.pos[2];
        let pl = &mut g.players[i];
        if pl.ai_form.repick(&t, cue, at(&p), ball_z, ours) {
            pl.ai_back = Return::new(rate, &mut roll);
        }
    }
    let pl = &mut g.players[i];
    let spot = pl.ai_form.spot;
    let go = pl.ai_back.step(rate, radius, spot, at(&p), &mut roll);
    Some(if go { [spot[0], 0.0, spot[1]] } else { p.pos })
}

/// A singles bot's goal while the ball isn't its own (`hst_sim::position::Single`): its centre (10 back for a net
/// player, 11 otherwise), walked to once the centre roll passes and until it's inside the centre radius, or its dash
/// spot in at the net. After each of its shots a net player picks the centre from the shot's zone and may dash; a
/// volley or smash (a baseliner: a smash) dashes, so does a net player seeing a smash and one its serve aim sent in
/// (`ai_serve_dash`). The dash is off once the ball passes it on its own side. None in doubles and before the serve
/// is hit. ponytail: the ALL style's coin is drawn once a point (the game re-draws it every few
/// shots) and shot choice 10's deeper dash spot waits for P11's shot choice.
fn ai_wait_singles(g: &mut Game, i: usize) -> Option<V3> {
    use hst_sim::position::{Court, Single};
    if g.players.len() != 2 || g.phase != Phase::Rally {
        return None;
    }
    let (p, o) = (g.players[i], g.players[1 - i]);
    let at = |q: &Player| [q.pos[0], q.pos[2]];
    // the app's AI is always level 3
    let reach = hst_sim::ai::Choice::new(0, 0, false).reach;
    let c = Court { side: p.end, reach, rate: p.ai.singles_center_rate, radius: p.ai.singles_center_radius };
    let (target, ball) = (g.marks.red.unwrap_or(at(&o)), [g.flight.ball.pos[0], g.flight.ball.pos[2]]);
    let rng = &mut g.rng.ai;
    let mut roll = || rng.next();
    let pl = &mut g.players[i];
    let mut s = match pl.ai_single {
        Some(s) => s,
        None => {
            // from no shots seen: the first look takes in the serve (or its own return)
            pl.ai_shot = 0;
            let net = p.ai.style == 1 || p.ai.style == 3 && (roll() >> 16 & 0x7fff) % 100 < 50;
            Single::start(&c, net, &mut roll)
        }
    };
    if std::mem::replace(&mut pl.ai_shot, g.shots) != g.shots {
        let net_row = p.ai.style != 2;
        if g.last_hitter != i as i32 {
            if net_row && p.ai_seen[0].is_some_and(|x| x.kind == 3) {
                s.go_in(&c);
            }
        } else if g.shots == 1 {
            if let Some(d) = p.ai_serve_dash {
                s.dash.get_or_insert(d);
            }
        } else {
            let dash = match p.ai_branch {
                Some(swing::Branch::Smash) => true,
                Some(swing::Branch::Volley) => s.net,
                _ => false,
            };
            s.after_hit(&c, dash, false, target, at(&o), at(&p), &mut roll);
        }
    }
    s.passed(at(&p), ball);
    let go = s.step(&c, at(&p), &mut roll);
    g.players[i].ai_single = Some(s);
    Some(go.map_or(p.pos, |d| [d[0], 0.0, d[1]]))
}

/// The AI's serve state: on entering it a timing draw (ending with the net count), then it walks the baseline to
/// its spot, waits there 60 to 119 frames and tosses; the swing is timed by its serve error (a badly timed strong
/// toss then goes wide or long, as in the original).
/// The serve level picks the spot, the toss and the aim; the swing kind and its stick are drawn as it presses
/// (`hst_sim::aim`), the press waits for the AI's own contact pick (rising for a quick serve).
/// ponytail: the original aims the frame before the contact; the server stands still, so this aims at the press.
fn bot_serve(g: &mut Game, i: usize) {
    if g.serving.bot_due.is_none() {
        let (end, ad, doubles) = (g.players[i].end, g.score.side == 1, g.players.len() == 4);
        let mut roll = ai_roll(&mut g.rng);
        let level0 = g.players[i].ai_picks.serve_level == 0;
        let spot = hst_sim::ai::serve_spot(level0, doubles, end, ad, &mut roll);
        let wait = hst_sim::ai::serve_wait(&mut roll);
        drop(roll);
        g.players[i].ai_serve = (spot, wait);
        g.players[i].ai_serve_swing = None;
        g.serving.bot_due = Some((SWEET_FRAME - g.players[i].ai_timing.serve).max(0) as usize);
    }
    // the aim, the frame before the contact (`serve_turn` hits as `left` runs out)
    if g.players[i].ai_serve_swing.is_some() && g.serving.swing.is_some_and(|sw| sw.left <= 2) {
        ai_serve_aim(g, i);
    }
    let s = g.serving;
    let mut stick = Vec2::ZERO;
    let press = if s.toss.is_none() {
        let (spot, wait) = g.players[i].ai_serve;
        let d = spot - g.players[i].pos[0];
        if d.abs() >= serve::WALK {
            stick.x = d.signum();
            None
        } else if wait > 0 {
            g.players[i].ai_serve.1 -= 1;
            None
        } else {
            Some(ai_toss_kind(g, i))
        }
    } else if let (true, None, false, Some(toss)) = (s.tossed, s.swing, s.whiffed, s.toss) {
        let horizon = g.serve_data[i].grades(toss).len();
        let due = s.bot_due.unwrap_or(SWEET_FRAME as usize);
        let quick = g.players[i].ai_picks.quick_serve;
        serve::ai_search(&g.serve_data[i], toss, quick, &predicted_path(g, horizon))
            .filter(|&k| k <= due)
            .map(|k| {
                let kind = ai_serve_kind(g, i, toss);
                // ponytail: a contact a frame off aims at the press (the app's own search picks the swing's frames)
                if k <= 1 {
                    ai_serve_aim(g, i);
                }
                kind
            })
    } else {
        None
    };
    serve_turn(g, i, stick, press);
}

/// A computer player's aim at its contact (`hst_sim::aim`): the doubles chooser from the four players' spots, the
/// singles rally chooser from its own and its opponent's, as the stick a human would hold (world x, z).
/// The volley and high-ball levels are the AI's picks; the aim and its button are kept on the player (`ai_press`).
/// ponytail: the exhibition level is always 3 (P11a), the mark is the opponent's spot, the reach heights are the contact's own (never low), the singles return of serve
/// and the short-ball flag aren't kept; the kind-1 contact (its own search) isn't told apart from a ground stroke.
fn ai_aim(g: &mut Game, i: usize) -> Vec2 {
    use hst_sim::aim::{Look, Pair};
    let p = g.players[i];
    let at = |j: usize| [g.players[j].pos[0], g.players[j].pos[2]];
    let kind = match (p.dive.is_some(), p.contact.map(|c| c.swing.branch)) {
        (true, _) => 4,
        (_, Some(swing::Branch::Volley)) => 2,
        (_, Some(swing::Branch::Smash)) => 3,
        _ => 0,
    };
    let rng = &mut g.rng;
    let stick = if g.players.len() == 4 {
        let human_mate = g.humans.get(i ^ 2) == Some(&true);
        let l = Pair {
            me: at(i),
            mate: at(i ^ 2),
            opp: [at(i ^ 1), at(i ^ 3)],
            side: p.end,
            singles: false,
            kind,
            volley_level: p.ai_picks.volley_level,
            level: 3,
            formation: p.formation,
            smash_third: false,
        };
        p.ai.pair_aim(&l, &mut ai_roll(rng))
    } else {
        let o = at(i ^ 1);
        let (volley_level, high_level) = (p.ai_picks.volley_level, p.ai_picks.high_level);
        let l = Look { me: at(i), opp: o, side: p.end, singles: true, kind, mark: o, level: 3, volley_level, high_level, ..Default::default() };
        p.ai.rally_aim(&l, &mut false, &mut ai_roll(rng))
    };
    g.players[i].ai_press = Some((Vec2::new(stick.stick[0], stick.stick[2]), hst_sim::aim::button(stick.plan)));
    Vec2::new(stick.stick[0], stick.stick[2])
}

/// The shot button a computer player presses: its aim's plan (`hst_sim::aim::button`), through the kind lock in
/// singles, as the app's kind (✕ topspin 0, ○ slice 1, △ lob 3). The aim is kept for the contact.
fn ai_press_kind(g: &mut Game, i: usize) -> i32 {
    ai_aim(g, i);
    let (stick, mut b) = g.players[i].ai_press.unwrap_or_default();
    if g.players.len() == 2 {
        let ai = g.players[i].ai;
        b = ai.lock_button(b, &mut ai_roll(&mut g.rng));
        g.players[i].ai_press = Some((stick, b));
    }
    button_kind(b)
}

/// A pad button (1 ✕, 2 ○, 4 △) as the app's shot kind.
fn button_kind(b: u8) -> i32 {
    match b {
        2 => 1,
        4 => 3,
        _ => 0,
    }
}

/// The stick a computer player holds at its contact: the one aimed at its press (a fresh aim if it has none),
/// bent by the singles kind lock off a flat or drop shot the button didn't ask for.
fn ai_contact_stick(g: &mut Game, i: usize) -> Vec2 {
    let Some((stick, b)) = g.players[i].ai_press.take() else { return ai_aim(g, i) };
    if g.players.len() != 2 {
        return stick;
    }
    let (ai, end) = (g.players[i].ai, g.players[i].end);
    let s = ai.lock_stick(b, [stick.x, 0.0, stick.y, 0.0], end, &mut ai_roll(&mut g.rng));
    Vec2::new(s[0], s[2])
}

/// A serving computer player's toss (`AiParams::serve_toss`) as the press kind `serve_turn` reads.
fn ai_toss_kind(g: &mut Game, i: usize) -> i32 {
    let p = g.players[i];
    let plan = hst_sim::ai::AiParams::serve_toss(
        g.rally.faults > 0,
        p.ai_second,
        p.ai_picks.serve_level,
        &mut ai_roll(&mut g.rng),
    );
    button_kind(hst_sim::aim::button(plan))
}

/// A serving computer player's swing (`AiParams::serve_swing`), kept for its aim.
fn ai_serve_kind(g: &mut Game, i: usize, toss: Toss) -> i32 {
    let toss = match toss {
        Toss::Strong => 1,
        Toss::Weak => 0,
        Toss::Under => 2,
    };
    let swing = g.players[i].ai.serve_swing(toss, &mut ai_roll(&mut g.rng));
    g.players[i].ai_serve_swing = Some(swing);
    button_kind(hst_sim::aim::button(swing))
}

/// A serving computer player's aim (`serve_aim`, kept as the serve's stick) for its kept swing, and in singles the
/// net dash.
fn ai_serve_aim(g: &mut Game, i: usize) {
    let Some(swing) = g.players[i].ai_serve_swing.take() else { return };
    let p = g.players[i];
    let mut roll = ai_roll(&mut g.rng);
    let ad = g.score.side == 1;
    let (s, dash) = if g.players.len() == 2 {
        // the app's AI is always level 3
        let reach = hst_sim::ai::Choice::new(0, 0, false).reach;
        let (s, dash, spot) =
            p.ai.serve_aim_singles(p.ai_picks.serve_level, swing, ad, p.hand < 0.0, p.end, reach, &mut roll);
        (s, dash.then(|| spot.unwrap_or([0.0, -reach * p.end])))
    } else {
        (p.ai.serve_aim(p.ai_picks.serve_level, swing, ad, p.hand < 0.0, p.end, &mut roll), None)
    };
    drop(roll);
    g.players[i].ai_serve_dash = dash;
    g.serving.bot_aim = Vec2::new(s[0], s[2]);
}

/// A character's strong-toss chance (%) on a second serve, from the game program's per-character table.
fn ai_second_toss(iso: &mut Iso, n: usize) -> i32 {
    let (cnf, bin) = (
        iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"),
        iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"),
    );
    hst_data::exe::Game::new(&cnf, &bin).expect("supported disc").second_toss()[n]
}

/// Whether a computer player leaves the ball: its predicted first bounce lands out by at least the row's line
/// margin (`AiParams::lets_go`; a ball that has bounced is played).
/// ponytail: the original also stops its run there; this one only holds the press.
fn ai_lets_go(g: &Game, i: usize) -> bool {
    let mut f = g.flight;
    if f.bounces > 0 {
        return false;
    }
    for _ in 0..240 {
        f.step(&g.shot, &COURTS[g.court]);
        if f.bounces > 0 {
            let at = [f.ball.pos[0], f.ball.pos[2]];
            return g.players[i].ai.lets_go(at, g.shots <= 1, g.players.len() == 2);
        }
    }
    false
}

/// The AI's draws on its generator.
fn ai_roll(rng: &mut Rngs) -> impl FnMut() -> u32 + '_ {
    move || rng.ai.next()
}

/// The AI object's per-frame update (`hst_sim::ai::Mind`), ahead of the routine `bot` runs for it: the point
/// reset on its first frame (with a timing draw), the state its role gives it, the hand-over to the rally once its
/// serve or return is played, and the point-over messages (a coin flip whether it keeps moving; an ALL-style
/// player's net rate moved by how its pick fared).
/// ponytail: the serve, receive and both rally routines all run `bot`'s stand-in loop until P11e/P11f port them;
/// the mind isn't made afresh for a new match.
fn ai_update(g: &mut Game, i: usize) -> hst_sim::ai::Mind {
    use hst_sim::ai::Phase as Ai;
    let partner_bot = g.players.len() == 2 || g.humans.get(i ^ 2) != Some(&true);
    let (style, heard) = (g.players[i].ai.style, g.players[i].ai_heard);
    let mut m = g.players[i].ai_mind.unwrap_or_default();
    if heard == 0 {
        m.reset(g.players[i].ai_mind.is_none(), partner_bot, &mut ai_roll(&mut g.rng));
        g.players[i].ai_heard = 1;
        ai_draw(g, i, None);
        m.count(&mut ai_roll(&mut g.rng));
        // the new-point message: every AI resets and draws in this tick, its state's entry comes the next
        g.players[i].ai_mind = Some(m);
        return m;
    }
    let p = g.players[i];
    let done = match m.phase {
        Ai::Start => {
            // the serve state's entry draws the timing errors and the count again
            if m.start(g.score.server == i as i32, g.score.receiver == i as i32) == Ai::Serve {
                ai_draw(g, i, None);
                m.count(&mut ai_roll(&mut g.rng));
            }
            false
        }
        Ai::Serve => g.phase != Phase::Serve,
        Ai::Receive => g.shots >= 2 && p.swing.is_none() && p.contact.is_none(),
        Ai::Rally => false,
    };
    if done {
        m.rally(&mut ai_roll(&mut g.rng));
    }
    if g.phase == Phase::Post && heard == 1 {
        m.point_over(&mut ai_roll(&mut g.rng));
        g.players[i].ai_heard = 2;
    }
    if heard == 2 && g.post.as_ref().is_some_and(|p| p.reacted && p.event.is_some()) {
        let won = i as i32 & 1 == g.post_winner;
        m.point_result(style, won, &mut ai_roll(&mut g.rng));
        g.players[i].ai_heard = 3;
    }
    g.players[i].ai_mind = Some(m);
    m
}

/// The AI hears a shot: re-entering the rally (`again`, `doubles_ai::heard_hit`) it looks again at its net pick (a
/// new ball path), and it counts its own team's hits toward the next one.
fn ai_heard_shot(g: &mut Game, i: usize, own: bool, again: bool) {
    let Some(mut m) = g.players[i].ai_mind else { return };
    if again {
        m.rally(&mut ai_roll(&mut g.rng));
    }
    if own {
        m.own_hit();
    }
    g.players[i].ai_mind = Some(m);
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
    // the gallery and the emitters step after the walkers, in `npcs::step` (the game's order)
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
                    reseed_sound(g);
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
                    match_stats::match_over(g);
                    g.score = Score::new();
                    g.first_serve = true;
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
    let (before, touched) = (g.flight.bounces, g.flight.special_contacts);
    g.flight.step_world(&shot, surface, &g.world.0, &g.world.1);
    if touched == 0 && g.flight.special_contacts > 0 {
        g.path_base = g.hit_flight.predicted_contacts(&shot, surface, g.since_hit);
    }
    // bounce sounds stop once the point is decided (the deciding bounce still plays)
    let (n, (at, material)) = (g.flight.bounces, g.flight.landing);
    let bounce = (n != before && n > 0 && g.phase == Phase::Rally).then_some((n, &material));
    if n != before && n == 1 && material.court {
        g.finish.bounce(at, g.flight.ball.vel);
    }
    let smash = g.smashed.then(|| sound::kmh(g.flight.ball.vel));
    g.sounds
        .extend(g.bounces.frame(bounce, smash).into_iter().map(|p| (p, at)));
    g.whistle.0 &= n == 0;
    g.since_hit += 1;
    if g.phase != Phase::Rally || g.shots == 0 {
        return;
    }
    let f = &g.flight;
    // ponytail: no rest detection on steep surfaces yet; the rally timeout counts as at rest
    let view = BallState {
        call: f.call,
        contacts: f.contacts,
        stopped: f.frame > 1800,
        pos: f.ball.pos,
    };
    if !g
        .rally
        .check(&view, g.shots, g.last_hitter, g.score.server, g.body_hit, true)
    {
        return;
    }
    let verdict = g.rally.judge(g.body_hit);
    match_stats::point(g, verdict.call, verdict.winner);
    g.finish.point_over(&g.rally, verdict.call, g.players.len() > 2);
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
    match_stats::scored(g, &before, event, team as usize);
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
        // taken in `npcs::step`, after the cheerers are picked
        g.applause = Some(r);
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
/// ponytail: the app's random numbers stand in for the game's
fn react(g: &mut Game, event: Event) {
    let n = g.players.len() as i32;
    let winner = g.post_winner;
    let mut taken = Vec::new();
    for i in 0..g.players.len() {
        let won = i as i32 & 1 == winner;
        let base = motion::reaction(
            g.body_hit == Some(i as i32),
            n,
            won,
            matches!(event, Event::Game | Event::Set),
            false,
        );
        let id = if n == 4 {
            let draw = |m: u32| g.rng.shared.r15() % m;
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
        (p.whiff, p.pending, p.approach) = (None, None, None);
        // the reaction ends a serve's follow-through
        p.served = None;
        // the ball's target keeps the 0x2b it has played since the hit, in place
        // ponytail: one the reaction finds unfinished isn't snapped to its end (in a match it has long ended by then)
        if bodyhit::standing(g, i) {
            g.players[i].root = None;
            continue;
        }
        let p = &mut g.players[i];
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
    g.finish.new_point(&g.score, &g.rules);
    g.rng.new_point();
    placement_draws(g, !fresh);
    // a point's end leaves for the next point (or match): the sound manager reseeds as on a change of ends
    if fresh {
        reseed_sound(g);
    }
}

/// The sound manager's reseed messages (match start, change of ends, a new point after a point): the hit sparks
/// roll their whole table from the generator as it was (those draws are lost), then it takes a fresh `rand()` seed.
fn reseed_sound(g: &mut Game) {
    g.respark = Some(g.rng.sound.clone());
    g.rng.change_ends();
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
        // the cut-aways come within a metre of the players; Bevy's reverse-Z float depth keeps the layered character
        // models apart at 40 m all the same (a 5 m near plane cut the near ground and players out of the cut-aways)
        p.near = 0.1;
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
        // the shadow: ball_shadow
        if !v.0 {
            t.translation = Vec3::from(g.prev_ball).lerp(Vec3::from(g.flight.ball.pos), a);
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
    if let Some(mut mt) = g.respark.take() {
        sparks.restart(&mut mt);
    }
    sparks.frame(hit, &mut g.rng.sound, &mut commands);
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
                // the baseline walk's motion (`serve_turn`), standing before it
                (None, _) if (0x20..=0x22).contains(&p.cmd.id) => m.play(p.cmd.id as usize, 1.0, true),
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
        // the serve swing plays on (at speed 1) through the server's follow-through
        if p.served.is_some() {
            m.serial = p.cmd.serial;
            m.clock.speed = 1.0;
            m.face_clock.speed = 1.0;
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

/// Each player's balloon: its texture, fade and visibility (`surprise` places it over the neck as its pop-ups).
fn balloons(
    g: Res<Game>,
    art: Res<BalloonArt>,
    time: Res<Time<Fixed>>,
    mut q: Query<(&BalloonView, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let a = time.overstep_fraction();
    for (view, mut vis) in &mut q {
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
            m.depth_bias = markers::LAST;
        }
        *vis = Visibility::Visible;
    }
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

/// The flight sound hit `h` starts (`sound::Flight::start`), and whether it was a special serve for the AI: a strong
/// toss hit within a frame of the sweet one with a topspin by characters 11/12 or a slice by 10 (the bending ones).
fn flight_sound(g: &mut Game, who: usize, h: &sound::Hit, class: u8, vel: V3) {
    let special = hst_sim::shot::special(class, h.kind, h.strong_toss, h.grade, h.offset);
    let c = g.chars[who];
    g.flight_sound = sound::Flight::start(h, c, special, sound::kmh(vel));
    g.special_serve = class == 0 && h.strong_toss && h.offset.abs() < 2 && matches!((h.kind, c), (0, 11 | 12) | (1, 10));
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
    if let (true, 0, true, Some(f)) = (g.whistle.0, whistle.1, whistle.0 != g.whistle.1, g.flight_sound) {
        whistle.1 = sound.play_at(&bank, f.play(ball[1]), ball);
    } else if let (true, Some(f)) = (whistle.1 != 0, &mut g.flight_sound) {
        let (angle, dist) = sound::place(ball);
        let volume = sound::falloff(f.play(ball[1]).volume, dist);
        match f.frame(ball[1], sound::kmh(g.flight.ball.vel), volume) {
            (p, None) => sound.update(whistle.1, p, ball),
            (_, Some(0)) => {
                sound.stop(whistle.1);
                (whistle.1, g.whistle.0) = (0, false);
            }
            (p, Some(q)) => {
                sound.update(whistle.1, p, ball);
                sound.turn(whistle.1, q, angle);
            }
        }
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

/// The serve values `serve_data` leaves at character 0's: the strong toss's mistiming error by the character's
/// skill level (the game program's table) and the serve angle (TParam Serv CON).
fn serve_character(iso: &mut Iso, n: usize, aim: hst_sim::shot::AimStats, d: &mut ServeData) {
    let (cnf, bin) = (
        iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"),
        iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"),
    );
    d.miss = serve::miss_of(hst_data::exe::Game::new(&cnf, &bin).expect("supported disc").serve_miss(), n);
    d.max_angle = aim.con[2] as f32;
}
