//! The court's background figures as the original draws them, from the court's `COURTSET/NN/HOLE01.XB`: the umpire
//! (`uNN`, the menu's umpire number) on her chair, the walking spectators (`g-MM` models, `npcSS` animation sets) driven by
//! [`npc::Walker`], and the trigger creatures with a model (`exe::Game::trigger_model`) driven by [`npc::Trigger`].
//!
//! The umpire faces the court centre. Her motion (0 idle, 1/2 turned toward the point's winner) restarts at frame
//! 0 when it changes and steps one frame a tick while turned or once the match is over (idle stays on frame 0
//! during play); a turn holds its last frame, the idle after the match loops back to frame 16 (14 for umpire 3).
//! Her head follows `Umpire::head` ([`look`]).
//!
//! The points are seen from the game's state: a decided point is the umpire turning (`point_over` sets her
//! motion only then), a new point the next serve after it. The walkers react and the gallery picks who cheers
//! ([`npc::cheerers`]) in the tick the point is decided. At the match's start the walkers cheer and leave marks
//! that the gallery's manager zeroes straight after (court 5 then shows its own); the first serve restarts their
//! idle loops.
//!
//! The court generator's draws come in the game's order each tick: the decided point's cheerers, the gallery's
//! applause and the walkers' reactions; the walkers' step (adding cheer marks); the gallery's manager (its cheer,
//! its marks, its tick); then the trigger creatures and sound emitters in the order they were spawned.
//!
//! The walkers stand at their home facing the court centre (each serve placement turns them there) and turn to the
//! middle of the winners' half when they react ([`npc::walker_facing`]).
//!
//! The trigger creatures hear the match as the original sends it ([`npc::Trigger::hear`]): 0xc entering the change of
//! ends (all reset), 0xe at each serve set-up (`next_point`; after a change of ends only the passing ball resets, and
//! rolls). Each is drawn while active, on and its +0x274 is not 0; the passing ball fades by it.

use std::sync::Arc;

use bevy::prelude::*;
use hst_data::{exe, iso::Iso, layout, xb::Archive};
use hst_sim::{npc, sound};

use super::{Game, Phase};
use crate::character::{self, CharacterData, Motion};
use crate::{Args, GameSpace};

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, step.after(character::tick).before(super::play_sounds))
        .add_systems(Update, (draw_cheers.after(super::camera), draw_crowd, look.after(character::animate)))
        .add_systems(PostUpdate, cull_crowd.before(bevy::transform::TransformSystems::Propagate));
}

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Npcs {
    umpire: Option<(Entity, Anim)>,
    /// Each walker with where it stands (its home; it turns there, `npc::walker_facing`).
    walkers: Vec<(Entity, npc::Walker, [f32; 4])>,
    triggers: Vec<(Entity, npc::Trigger, exe::TriggerRow)>,
    /// The gallery's tick counter (restarts at a new point).
    tick: i32,
    /// The umpire's motion last tick; a decided point since the last serve.
    motion: u8,
    decided: bool,
    /// The players' and ball's positions the creatures startle at, and the types already startled this point.
    near: npc::Near,
    /// Changing ends last tick, and in the serve phase last tick (entering it is the serve set-up, message 0xe).
    ends: bool,
    serving: bool,
    /// The passing ball's materials with their own alpha and blending, faded by its +0x274 (`npc::Trigger::speed`).
    #[serde(skip)]
    fade: Vec<(Handle<StandardMaterial>, f32, AlphaMode)>,
    /// Rigs whose animation carries them far from their bind pose (the passing ball): not culled by its bounds.
    uncull: Vec<Entity>,
    /// The gallery's cheer marks (at a walker's neck when it started cheering); stepped after the walkers.
    cheers: npc::Cheers,
    /// Each walker's last cheer mark (`npc::Cheers::follow`).
    mark: Vec<usize>,
    /// Court 5's own cheer marks about its stands (empty on the other courts).
    marks: Vec<[f32; 4]>,
    /// The trigger creatures and the sound emitters (`Game::emitters`, `None`) in spawn order.
    figures: Vec<Option<usize>>,
    /// The cheer marks' sprites (`npc/clap`) and their sizes by kind (`exe::Game::cheer_sprites`).
    #[serde(skip)]
    sprites: Handle<Mesh>,
    sizes: [[f32; 5]; 2],
    /// The match's first point has been served (the walkers' match-start cheer ends then).
    started: bool,
    /// The match was decided: whether the favoured team won, until the next serve.
    over: Option<bool>,
    /// The scoreboard's pause was over last tick.
    reacted: bool,
    /// Court 5's own creatures (`npc::Crowd`), each with its parts' materials and, per material, its `.UVA` track's
    /// offset at each atlas frame.
    #[serde(with = "crowd_state")]
    crowd: Vec<(npc::Crowd, Vec<(Handle<StandardMaterial>, Vec<[f32; 2]>)>)>,
}

/// The umpire's animation controller: `frame` shown, `next` the one after.
#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct Anim {
    frame: f32,
    next: f32,
}

impl Anim {
    fn set(&mut self, t: f32, len: f32) {
        (self.frame, self.next) = (t.clamp(0.0, len), t.clamp(0.0, len));
    }
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mut g: ResMut<Game>,
    root: Query<Entity, With<GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
) {
    let Ok(root) = root.single() else { return };
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let n = g.stage as usize;
    let (Ok(data), Some((list, plants))) = (iso.read(&format!("COURTSET/{n:02}/HOLE01.XB")), crate::court_layout(&mut iso, n)) else {
        return;
    };
    let arc = Archive::parse(&data).expect("court archive");
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
    let game = exe::Game::new(&cnf, &bin).expect("supported disc");
    let mut load = |(stem, anims): (String, Vec<String>)| {
        character::load_npc(&arc, &stem, &anims, &mut meshes, &mut materials, &mut images, &mut bindposes).map(Arc::new)
    };
    let mut place = |data: &Arc<CharacterData>, world: &[[f32; 4]; 4], scale: f32| {
        let e = character::spawn(&mut commands, data, root);
        commands.entity(e).insert(Transform::from_matrix(Mat4::from_cols_array_2d(world)).with_scale(Vec3::splat(scale)));
        e
    };
    let players = g.rules.players as u32;
    let walkers = game.walkers(n as u32);
    let (mut hidden, mut off) = (Vec::new(), Vec::new());
    let mut crowd = Vec::new();
    let mut npcs = Npcs { umpire: None, walkers: Vec::new(), triggers: Vec::new(), tick: 0, motion: 0, decided: false, near: default(), ends: false, serving: false, fade: Vec::new(), uncull: Vec::new(), cheers: npc::Cheers::new(g.stage), mark: Vec::new(), marks: Vec::new(), figures: Vec::new(), sprites: default(), sizes: game.cheer_sprites(), started: false, over: None, reacted: false, crowd: Vec::new() };
    let (umpire, rng) = (g.umpire.clone(), &mut g.rng.court);
    let mut voices = game.deciding_voices().into_iter();
    let mut roll = || rng.next();
    for c in npc::spawn(&list, &plants, &game.npc_roster(n as u32), &walkers, players) {
        match c.kind {
            npc::Kind::Umpire => {
                let Some(data) = load(layout::umpire_files(args.umpire)) else { continue };
                // she faces the court centre: rows (up × forward, up, forward, chair)
                let ([x, z], p) = (umpire.forward, umpire.pos);
                let world = [[z, 0.0, -x, 0.0], [0.0, 1.0, 0.0, 0.0], [x, 0.0, z, 0.0], [p[0], p[1], p[2], 1.0]];
                npcs.umpire = Some((place(&data, &world, 1.0), Anim::default()));
            }
            npc::Kind::Walker(k) => {
                let [model, set, _] = walkers[k as usize];
                let Some(data) = load(layout::walker_files(model, set)) else { continue };
                let lens = std::array::from_fn(|a| data.motions.get(&a).map_or(0.0, |c| c.length));
                let slot = npcs.walkers.len() as u32;
                let mut w = npc::Walker { slot, mode: 0, counter: 0, anim: 0, frame: 0.0, next: 0.0, speed: 1.0, advancing: true, lens, ..Default::default() };
                // the match's start: its cheer, and a mark where it stands
                w.start(players, &mut roll);
                // ponytail: its feet, not its neck (no pose yet); the manager's match start zeroes it straight after
                let mark = npcs.cheers.add([c.world[3][0], c.world[3][1], c.world[3][2]], slot);
                npcs.mark.push(mark);
                // the serve placement before the first point turns it to the court centre
                let world = npc::walker_facing(c.world[3], 0.0);
                w.wander = npc::Wander::new(c.world[3], &[], &[]);
                npcs.walkers.push((place(&data, &world, c.scale), w, c.world[3]));
            }
            npc::Kind::Trigger(t) => {
                if npc::EMITTERS.contains(&t) {
                    npcs.figures.push(None);
                }
                // courts 5's and 9's sound-only creatures (19, 40) have no model but still hear the match
                let Some((model, base, clips)) = game.trigger_model(t) else {
                    if matches!(t, 19 | 40) {
                        let row = game.trigger(t);
                        let mut tr = npc::Trigger { ty: t, anchor: c.world[3], home: c.world, world: c.world, ..default() };
                        tr.reset(&row, &mut roll);
                        npcs.figures.push(Some(npcs.triggers.len()));
                        npcs.triggers.push((Entity::PLACEHOLDER, tr, row));
                    }
                    continue;
                };
                let Some(data) = load(layout::trigger_files(&model, &base, clips)) else { continue };
                let row = game.trigger(t);
                // a two-clip creature plays its first
                let len = data.motions.get(&0).map_or(0.0, |c| c.length);
                let routes = game.ball_routes();
                let (path, radii, start) = (c.path.clone(), c.radii.clone(), c.start);
                let mut tr = npc::Trigger { ty: t, anchor: c.world[3], home: c.world, world: c.world, path, radii, start, len, routes, bounces: game.ball_sounds(), ..default() };
                tr.clip_lens = [0, 1].map(|k| data.motions.get(&k).map_or(0.0, |c| c.length));
                if t == 44 {
                    tr.give_voice(&row, voices.next().unwrap_or((-1, 0)), &mut roll);
                }
                tr.reset(&row, &mut roll);
                let e = place(&data, &c.world, c.scale);
                if t == 15 {
                    hidden.push(e); // the passing ball waits off until a change of ends rolls it
                } else if !(tr.active && tr.on && tr.speed != 0.0) {
                    // off from its spawn (court 6's bear till a deciding-set tiebreak): hidden through the intro too,
                    // which holds the match (and `step`, which draws it) still
                    off.push(e);
                }
                npcs.figures.push(Some(npcs.triggers.len()));
                npcs.triggers.push((e, tr, row));
            }
            npc::Kind::Court5(t) => {
                crowd.push((npc::Crowd::new(&mut roll), t, c.world, c.scale));
            }
        }
    }
    for (c, t, world, scale) in crowd {
        npcs.crowd.push((c, crowd_model(&arc, t, &world, scale, &mut commands, root, &mut meshes, &mut materials, &mut images, &mut bindposes)));
    }
    // the match's start (more than one player): court 5's creatures stand up
    // ponytail: drawn after every creature is made; the original's start message reaches them in its object tree's
    // order among the walkers' and gallery's draws (not recorded)
    if players > 1 {
        npcs.crowd.iter_mut().for_each(|(c, _)| c.cheer(&mut roll));
    }
    // then the manager's match start zeroes the walkers' marks; court 5 shows its own (with more than one player)
    npcs.cheers.clear();
    if g.stage == 5 {
        npcs.marks = game.court5_marks();
        if players > 1 {
            npcs.cheers.court5(&npcs.marks);
        }
    }
    for &e in hidden.iter().chain(&off) {
        commands.entity(e).insert(Visibility::Hidden);
    }
    npcs.uncull = hidden;
    if let Ok(look) = crate::effects::look(&mut iso, "npc/clap", &mut materials, &mut images) {
        npcs.sprites = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
        let view = commands.spawn((Mesh3d(npcs.sprites.clone()), MeshMaterial3d(look), Transform::default(), bevy::camera::visibility::NoFrustumCulling)).id();
        commands.entity(root).add_child(view);
    }
    commands.insert_resource(npcs);
}

/// Turn walker `e` at `home` toward (0, 0, `z`), keeping its scale.
fn face(turn: &mut Query<&mut Transform, With<character::Rig>>, e: Entity, home: [f32; 4], z: f32) {
    if let Ok(mut t) = turn.get_mut(e) {
        let scale = t.scale;
        *t = Transform::from_matrix(Mat4::from_cols_array_2d(&npc::walker_facing(home, z))).with_scale(scale);
    }
}

/// Walkers back home facing the court centre, nothing moving (a new point, the first serve).
fn home_walkers(walkers: &mut [(Entity, npc::Walker, [f32; 4])], players: &[super::Player]) {
    let near: Vec<[f32; 4]> = players.iter().map(|p| [p.pos[0], p.pos[1], p.pos[2], 1.0]).collect();
    let spots: Vec<[f32; 4]> = walkers.iter().map(|(_, _, h)| *h).collect();
    walkers.iter_mut().for_each(|(_, w, home)| w.wander = npc::Wander::new(*home, &near, &spots));
}

/// Show clip `id` at `frame` (the tick's sampled time; `character::tick` already moved the last one to `prev`).
fn show(m: &mut Motion, id: usize, frame: f32, looping: bool) {
    if m.id != id {
        m.set(id, 0.0, looping, None, 0);
    }
    m.clock.looping = looping;
    m.clock.sampled = frame;
}

/// Pin each trigger creature's clip back on its own frame after a `character::tick` that `step` doesn't follow (the
/// intro's, B180): only `step` moves them, so a ball-struck prop stays standing until a ball hits it.
pub(super) fn hold_triggers(npcs: Option<Res<Npcs>>, mut q: Query<&mut Motion>) {
    for (e, t, _) in npcs.iter().flat_map(|n| &n.triggers) {
        if let Ok(mut m) = q.get_mut(*e) {
            (m.prev, m.clock.sampled) = (t.frame, t.frame);
        }
    }
}

fn step(
    mut g: ResMut<Game>,
    npcs: Option<ResMut<Npcs>>,
    mut q: Query<(&character::Rig, &mut Motion)>,
    mut turn: Query<&mut Transform, With<character::Rig>>,
    mut shown: Query<&mut Visibility, With<character::Rig>>,
    mut commands: Commands,
    children: Query<&Children>,
    audio: Option<Res<crate::audio::Sound>>,
    bank: Option<Res<crate::audio::CourtBank>>,
    (ticks, mut once): (Res<super::heard::Heard>, ResMut<super::heard::Played>),
    (mats, mut materials): (Query<&MeshMaterial3d<StandardMaterial>>, ResMut<Assets<StandardMaterial>>),
    globals: Query<&GlobalTransform>,
    args: Res<Args>,
) {
    let umpire_id = args.umpire;
    let Some(mut npcs) = npcs else {
        // no court figures: the gallery and the emitters still step
        let g = &mut *g;
        let rng = &mut g.rng.court;
        let mut roll = || rng.next();
        applaud(&mut g.gallery, &mut g.applause, &mut roll);
        if matches!(crate::weather::now(), 2 | 3) {
            g.gallery.idle();
        } else {
            g.cheers.extend(g.gallery.step(g.stage, g.players.len() as u32, g.gallery_game, &mut roll));
        }
        for k in 0..g.emitters.len() {
            emit(&mut g.emitters[k], &mut g.sounds, &mut roll);
        }
        return;
    };
    let npcs = &mut *npcs;
    for e in std::mem::take(&mut npcs.uncull) {
        for &part in children.get(e).into_iter().flatten() {
            commands.entity(part).insert(bevy::camera::visibility::NoFrustumCulling);
        }
    }
    let at = |p: [f32; 3]| [p[0], p[1], p[2], 1.0];
    npcs.near.pos = g.players.iter().map(|p| at(p.pos)).chain([at(g.flight.ball.pos)]).collect();
    // the ball-hit obstacles' boxes: the players' model matrices (up, facing; the side row's sign doesn't matter to
    // the test) and the ball's
    let player = |p: &super::Player| {
        let d = p.body.face.dir;
        [[-d[2], 0.0, d[0], 0.0], [0.0, 1.0, 0.0, 0.0], d, at(p.pos)]
    };
    let m = g.flight.model;
    let ball = [m[0], m[1], m[2], at(g.flight.ball.pos)];
    // its speed from its last move, kept while it stands
    // (the original's obstacles run before the ball and players move: what they see then is this, after this tick's
    // moves, and their message 0x14's step off them comes before the ball's next own step either way)
    npcs.near.kmh = npc::ball_kmh(g.flight.last);
    npcs.near.boxes = g.players.iter().map(player).chain([ball]).collect();
    let (motion, over, players, serve) = (g.umpire.motion, g.umpire.over, g.rules.players as u32, g.phase == Phase::Serve);
    let ends = matches!(g.phase, Phase::ChangeEnds(_));
    let played = g.score.sets[0] + g.score.sets[1];
    npcs.near.deciding = g.score.tiebreak && played + 1 == 2 * g.rules.sets - 1;
    // a decided point turns the walkers to the middle of the winners' half (by the side their first player is on)
    let side = if 0.0 <= g.players.get(g.post_winner as usize).map_or(0.0, |p| p.pos[2]) { 6.4 } else { -6.4 };
    match_news(&mut npcs.near, &g);
    let g = &mut *g;
    let rng = &mut g.rng.court;
    let mut roll = || rng.next();
    // a decided point: some cheering, the gallery's applause, then the walkers react
    let decided = motion != 0 && npcs.motion == 0;
    let cheer = if decided { npc::cheerers(npcs.walkers.len(), &mut roll) } else { [false; 6] };
    applaud(&mut g.gallery, &mut g.applause, &mut roll);
    // court 5 shows its own marks again after a plain call that won a game or set (short of the match), an ace or a
    // return winner
    if decided && g.stage == 5 {
        let call = g.rally.judge(g.body_hit).call;
        let game = g.gallery_game && !g.score.match_over;
        if npc::court5_again(call, game, g.shots, g.last_hitter & 1 == g.post_winner) {
            npcs.cheers.court5(&npcs.marks);
        }
    }
    if decided && g.score.match_over {
        npcs.over = Some(sound::favoured(&g.humans)[g.post_winner.clamp(0, 1) as usize]);
    }
    let first = !npcs.started;
    // the match's first serve (a new point straight after its start): the idle loops restart one walker per tick
    // (with more than one player), the marks go
    if serve && !npcs.started {
        npcs.started = true;
        npcs.tick = 0;
        npcs.crowd.iter_mut().for_each(|(c, _)| c.reset());
        npcs.cheers.clear();
        npcs.walkers.iter_mut().for_each(|(_, w, _)| w.new_point(players > 1));
        home_walkers(&mut npcs.walkers, &g.players);
    }
    // the match over: the manager resumes, court 5 shows its marks if the favoured team won, an unfavoured win
    // zeroes them
    // ponytail: the match-over phase isn't played (the next match starts at once, its new point zeroing the marks
    // again); the walkers' cheer at it and their match-start cheer for the next match are not replayed
    if serve && let Some(favoured) = npcs.over.take() {
        if players > 1 {
            npcs.cheers.match_over(favoured, &npcs.marks);
        }
        npcs.near.paused = false;
    }
    let mut heard = Vec::new();
    if decided {
        for (e, w, home) in &mut npcs.walkers {
            w.react();
            face(&mut turn, *e, *home, side);
        }
        npcs.decided = true;
        for (_, t, row) in &mut npcs.triggers {
            heard.extend(t.message(row, 6, &mut npcs.near, &mut roll).into_iter().map(|s| (s, t.ty, t.pos3())));
        }
        // court 5's creatures stand up unless the point gets an instant replay
        // ponytail: drawn after the trigger creatures' reactions; the original's order among them follows its object
        // tree (not recorded)
        let event = g.gallery_game || g.score.match_over;
        if npc::crowd_cheers_point(g.instant.decision.replay, event, g.rally.judge(g.body_hit).call) {
            npcs.crowd.iter_mut().for_each(|(c, _)| c.cheer(&mut roll));
        }
    }
    // the scoreboard's pause over (a game, set or match decided)
    let reacted = g.post.as_ref().is_some_and(|p| p.reacted && p.event.is_some());
    if reacted && !npcs.reacted {
        for (_, t, row) in &mut npcs.triggers {
            heard.extend(t.message(row, 7, &mut npcs.near, &mut roll).into_iter().map(|s| (s, t.ty, t.pos3())));
        }
    }
    npcs.reacted = reacted;
    // the serve after it: a new point
    if npcs.decided && serve {
        npcs.decided = false;
        npcs.tick = 0;
        npcs.crowd.iter_mut().for_each(|(c, _)| c.reset());
        npcs.cheers.clear();
        for (e, w, home) in &mut npcs.walkers {
            // the serve placement turns them back to the court centre
            face(&mut turn, *e, *home, 0.0);
            // a new point after a decided one resets the walker (counter 0 with any player count): the cheer loop
            // only ends when the idle loops restart
            w.new_point(true);
        }
        home_walkers(&mut npcs.walkers, &g.players);
    }
    // the serve set-up's message (0xe, every new point and the match's first serve): in the match's state 0 (its
    // start) or 1 (after a change of ends) only the passing ball resets, and only in state 1 its reset rolls; else every
    // creature but type 37
    if serve && !npcs.serving {
        let phase = if first { 0 } else if npcs.ends { 1 } else { 4 };
        (npcs.near.ends, npcs.near.players) = (phase == 1, players);
        for (_, t, row) in &mut npcs.triggers {
            heard.extend(t.hear(row, 0xe, false, phase, &mut npcs.near, &mut roll).into_iter().map(|s| (s, t.ty, t.pos3())));
        }
        npcs.near.ends = false;
    }
    npcs.serving = serve;
    // entering the change of ends (message 0xc): every creature hears the game's end (8) and resets
    if ends && !npcs.ends {
        npcs.crowd.iter_mut().for_each(|(c, _)| c.reset());
        npcs.cheers.clear();
        for (_, t, row) in &mut npcs.triggers {
            heard.extend(t.hear(row, 0xc, false, 4, &mut npcs.near, &mut roll).into_iter().map(|s| (s, t.ty, t.pos3())));
        }
    }
    npcs.ends = ends;
    if let Some((e, a)) = &mut npcs.umpire {
        if let Ok((rig, mut m)) = q.get_mut(*e) {
            let len = rig.data.motions.get(&(motion as usize)).map_or(0.0, |c| c.length);
            if motion != npcs.motion {
                *a = Anim::default();
            }
            if motion != 0 || over {
                a.set(a.next, len);
                a.next = hst_sim::ps2::add(a.next, 1.0);
                // the idle (playing on once the match is over) loops back to frame 16, 14 for umpire 3; a turn holds
                // its last frame
                if motion == 0 && len <= a.frame {
                    a.set(if umpire_id == 3 { 14.0 } else { 16.0 }, len);
                }
            }
            // ponytail: her texture animation (TAF) is not drawn
            show(&mut m, motion as usize, a.frame, false);
        }
    }
    npcs.motion = motion;
    npcs.crowd.iter_mut().for_each(|(c, _)| c.step());
    // the walkers' moves: what they dodge (players, ball, each other) and the court's world they bump into; none in
    // the rain (the original skips their whole tick then)
    // ponytail: rain still runs their animation here, as before
    let mut spots: Vec<[f32; 4]> = npcs.walkers.iter().map(|(_, w, _)| w.wander.world[3]).collect();
    let near: Vec<[f32; 4]> = g.players.iter().map(|p| at(p.pos)).collect();
    let (b, v) = (g.flight.ball.pos, g.flight.ball.vel);
    let raining = matches!(crate::weather::now(), 2 | 3);
    for (k, (e, w, home)) in npcs.walkers.iter_mut().enumerate() {
        let react = w.mode == 1;
        if raining {
            w.step(players, cheer[k], npcs.tick, &mut roll);
        } else {
            let mut around = npc::Around { players: &near, ball: at(b), ball_vel: [v[0], v[1], v[2], 0.0], walkers: &mut spots, world: &g.world.0, side };
            w.step_around(players, cheer[k], npcs.tick, &mut roll, &mut around);
            if let Ok(mut t) = turn.get_mut(*e) {
                let scale = t.scale;
                *t = Transform::from_matrix(Mat4::from_cols_array_2d(&w.wander.world)).with_scale(scale);
            }
        }
        // a walker breaking into its cheer leaves a mark at its neck (as last drawn), and the mark's base follows the
        // neck while it cheers
        let neck = q.get(*e).ok().and_then(|(r, _)| Some(r.joints[r.data.joint("Bip01Neck")?])).and_then(|j| globals.get(j).ok());
        let neck = neck.map_or([home[0], home[1], home[2]], |t| {
            let v = t.translation();
            [v.x, -v.y, -v.z]
        });
        if react && w.anim == 5 {
            npcs.mark[k] = npcs.cheers.add(neck, w.slot);
        }
        if !raining && w.anim == 5 {
            npcs.cheers.follow(npcs.mark[k], neck);
        }
        if let Ok((_, mut m)) = q.get_mut(*e) {
            show(&mut m, w.anim as usize, w.frame, w.looping);
        }
    }
    // the gallery's manager: paused by type 44 in the deciding set, silent in the rain
    if npcs.near.paused || matches!(crate::weather::now(), 2 | 3) {
        g.gallery.idle();
    } else {
        g.cheers.extend(g.gallery.step(g.stage, g.players.len() as u32, g.gallery_game, &mut roll));
        npcs.cheers.step(&mut roll);
        npcs.tick += 1;
    }
    let mut emitters = 0..g.emitters.len();
    let mut flyover = None;
    for &f in &npcs.figures {
        let Some(k) = f else {
            if let Some(k) = emitters.next() {
                emit(&mut g.emitters[k], &mut g.sounds, &mut roll);
            }
            continue;
        };
        let (e, t, row) = &mut npcs.triggers[k];
        // ponytail: a type's startled flag stays set until the next point (the original clears it sooner, from code
        // not found)
        for s in t.step_near(row, &mut npcs.near, &mut roll) {
            if t.ty == 19 {
                heard.push((s, t.ty, t.pos3()));
                continue;
            }
            g.sounds.push((npc::trigger_sound(t.ty, s), t.sound_at()));
        }
        // every creature stands where its tick put it (a startled one runs along its path)
        if let Ok(mut tf) = turn.get_mut(*e) {
            let scale = tf.scale;
            *tf = Transform::from_matrix(Mat4::from_cols_array_2d(&t.world)).with_scale(scale);
        }
        // drawn while active and on with its +0x274 (`speed`) not 0; its draw's countdown (`stage`) is in its tick
        // ponytail: the draw's own off switch (+0x282, cleared by a new match's messages) is never set here: not found
        if let Ok(mut v) = shown.get_mut(*e) {
            v.set_if_neq(if t.active && t.on && t.speed != 0.0 { Visibility::Inherited } else { Visibility::Hidden });
        }
        // the passing ball's +0x274 is also its model's alpha (it fades over its last 10 frames)
        if t.ty == 15 {
            if npcs.fade.is_empty() {
                npcs.fade = children.iter_descendants(*e).filter_map(|d| mats.get(d).ok()).filter_map(|m| materials.get(&m.0).map(|a| (m.0.clone(), a.base_color.alpha(), a.alpha_mode))).collect();
            }
            let a = t.speed.clamp(0.0, 1.0);
            for (h, alpha, mode) in &npcs.fade {
                let (alpha, mode) = if a < 1.0 { (alpha * a, AlphaMode::Blend) } else { (*alpha, *mode) };
                if materials.get(h).is_some_and(|m| m.base_color.alpha() != alpha || m.alpha_mode != mode) {
                    let mut m = materials.get_mut(h).unwrap();
                    (m.alpha_mode, m.base_color) = (mode, m.base_color.with_alpha(alpha));
                }
            }
        }
        // struck by the ball (27–29): message 0x14, the ball's step back off it before its next own step
        // (a struck toss too: the plane step's miss stops it dead, and its bounce has the server toss again);
        // the players, the AI and the path object hear 0x14 in `hear`; the landing markers hide
        if t.struck {
            (g.marks.red, g.marks.smash) = (None, None);
            let (shot, material) = (g.shot, g.world.1[0]);
            g.flight.step_plane(&shot, &hst_sim::ball::COURTS[g.court], material);
            g.doubles_ai.hear(&g.players, 0x14);
        }
        if let Ok((_, mut m)) = q.get_mut(*e) {
            // type 38 plays its second clip after a fast ball
            if t.ty == 38 && m.id != t.alt as usize {
                let looping = m.clock.looping;
                m.set(t.alt as usize, 0.0, looping, None, 0);
            }
            m.clock.sampled = t.frame;
        }
        if t.ty == 6 {
            flyover = Some((t.on, t.volume, t.pos3()));
        }
    }
    let (Some(audio), Some(Some(bank))) = (audio, bank.map(|b| b.0.clone())) else { return };
    let play = |key: i32, volume: i32| sound::Play { slot: 0, program: 7, key: key as u8, volume, speed: 1.0 };
    // each once however often its tick runs (`heard`)
    for (s, ty, at) in heard {
        if !once.first(ticks.tick, ("creature", s, ty, at)) {
            continue;
        }
        let t = npcs.triggers.iter().find(|(_, t, _)| t.ty == ty).map(|(_, t, _)| t);
        match (ty, t) {
            // 19's sweep: from its bearing at its volume
            (19, Some(t)) => _ = audio.play_toward(&bank, play(s, t.volume), t.bearing),
            (6, Some(t)) => once.flyover = Some(audio.play_at(&bank, play(s, t.volume), at)),
            _ => _ = audio.play_at(&bank, play(s, 0x80), at),
        }
    }
    // the flyover's sound follows it, fading with |z|; its reset stops it (as of the last tick heard)
    match (once.flyover, flyover) {
        _ if !once.fresh => {}
        (Some(id), Some((true, volume, at))) => audio.update(id, play(30, volume), at),
        (Some(id), _) => {
            audio.stop(id);
            once.flyover = None;
        }
        _ => {}
    }
}

/// What the match-event creatures hear about the point (`npc::Near`): a game or set won short of the match, a named
/// point (an ace, return winner or smash winner by the last hitter's team), the call, winner and shots, the teams
/// with a human, the ball's first bounce and the camera's eye.
// ponytail: a passing or diving winner (the game's fourth named kind) is not told apart: not named
fn match_news(near: &mut npc::Near, g: &Game) {
    let hitter_won = g.last_hitter & 1 == g.post_winner;
    (near.game, near.call, near.winner, near.shots) = (g.gallery_game && !g.score.match_over, g.rally.judge(g.body_hit).call, g.post_winner, g.shots);
    near.named = hitter_won && ((1..=2).contains(&g.shots) || g.smashed);
    near.humans = [false; 2];
    for (i, &h) in g.humans.iter().enumerate() {
        near.humans[i & 1] |= h;
    }
    let (at, material) = g.flight.landing;
    (near.bounces, near.landing, near.in_court) = (g.flight.bounces, [at[0], at[1], at[2], 1.0], material.court && g.body_hit.is_none());
    let e = g.cam.view.eye;
    near.eye = [e[0], e[1], e[2], 1.0];
}

/// The cheer marks' sprites, drawn while the gallery's manager runs and it isn't raining: each faces the camera,
/// pulled 0.5 toward it and hung `sizes[3]` above the mark, `size` either side and `size` tall, at a size that
/// keeps it about the same on screen up close and grows with distance far off; hidden when a metre there would
/// show `sizes[4]` pixels or more (too near the camera).
// ponytail: the original also needs a flag (never set in any dump) to be clear on courts other than 5
fn draw_cheers(npcs: Option<Res<Npcs>>, g: Res<Game>, cam: Query<&Transform, With<crate::Orbit>>, mut meshes: ResMut<Assets<Mesh>>) {
    let (Some(npcs), Ok(cam)) = (npcs, cam.single()) else { return };
    let Some(mut mesh) = meshes.get_mut(&npcs.sprites) else { return };
    let game = |v: Vec3| Vec3::new(v.x, -v.y, -v.z);
    let (right, up, ahead, eye) = (game(cam.rotation * Vec3::X), game(cam.rotation * Vec3::Y), game(cam.rotation * Vec3::NEG_Z), game(cam.translation));
    let t = &npcs.sizes[npcs.cheers.kind.min(1) as usize];
    let tan = g.cam.view.fov.tan();
    let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
    if !npcs.near.paused && !matches!(crate::weather::now(), 2 | 3) {
        for c in npcs.cheers.shown() {
            let mut at = Vec3::from(c.pos) - ahead * 0.5;
            let half = (at - eye).dot(ahead) * tan;
            let size = t[0] * (t[1] * half).max(1.0) * (t[2] * half).min(1.0);
            at.y -= t[3];
            // a metre at the mark spans 240 / (tan · depth) of the picture's 480 lines
            let z = (at - eye).dot(ahead);
            if 0.0 < t[4] && (z <= 0.001 || t[4] <= 240.0 / tan / z) {
                continue;
            }
            let (w, h) = (right * size, up * size);
            let n = pos.len() as u32;
            pos.extend([at - w + h, at + w + h, at - w, at + w].map(|v| v.to_array()));
            uv.extend([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0f32]]);
            colour.extend([[1.0f32; 4]; 4]);
            index.extend([n, n + 1, n + 2, n + 2, n + 1, n + 3]);
        }
    }
    crate::effects::fill(&mut mesh, pos, uv, colour, index);
}

/// The decided point's gallery reaction (`simulate` leaves it for the cheerers to come first).
fn applaud(gallery: &mut sound::Gallery, applause: &mut Option<sound::Reaction>, roll: &mut impl FnMut() -> u32) {
    if let Some(r) = applause.take() {
        gallery.point(r, roll);
    }
}

/// An emitter's step, cueing its sound when due.
fn emit(e: &mut npc::Emitter, sounds: &mut Vec<(sound::Play, [f32; 3])>, roll: &mut impl FnMut() -> u32) {
    if e.step(roll) {
        debug!("emitter type {} sound {}", e.ty, e.row.sound);
        sounds.push((sound::Play { slot: 0, program: 7, key: e.row.sound as u8, volume: 0x40, speed: 1.0 }, e.pos));
    }
}

/// Court 5's creature `t` (`creatureMdl/wim-0N`) placed at `world`: its parts' materials, each with its `.UVA`
/// track's offset at every atlas frame (the track `@mtluva_<material>`, one key a frame).
#[allow(clippy::too_many_arguments)]
fn crowd_model(
    arc: &Archive,
    t: u8,
    world: &[[f32; 4]; 4],
    scale: f32,
    commands: &mut Commands,
    root: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
) -> Vec<(Handle<StandardMaterial>, Vec<[f32; 2]>)> {
    let stem = format!("wim-{t:02}");
    let get = |p: String| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&p)).and_then(|e| arc.read(e).ok());
    let shown = match crate::effects::model(arc, &format!("creaturemdl/{stem}"), commands, root, meshes, materials, images, bindposes) {
        Ok((_, shown)) => shown,
        Err(e) => {
            warn!("court 5 creature {stem}: {e}");
            return Vec::new();
        }
    };
    // (an effect's root starts hidden)
    commands.entity(shown.root).insert((Transform::from_matrix(Mat4::from_cols_array_2d(world)).with_scale(Vec3::splat(scale)), Visibility::Inherited));
    // B283: the end stands' crowds (types 63, 66) are drawn only while a point by them is ahead of the camera
    if let Some(z) = [(2, -49.0), (5, 49.0)].iter().find(|c| c.0 == t).map(|c| c.1) {
        commands.entity(shown.root).insert(CrowdCull([0.0, f32::from_bits(0xc0ff_e76d), z]));
    }
    let names = get(format!("creaturemdl/{stem}.mtl")).and_then(|m| hst_data::mtl::parse(&m, get(format!("creaturemdl/{stem}.mti")).as_deref()).ok());
    let uva = get(format!("creatureani/{stem}.uva")).and_then(|u| hst_data::mor::parse(&u, 4).ok());
    let (Some(names), Some(uva)) = (names, uva) else {
        warn!("court 5 creature {stem}: no materials or .UVA");
        return Vec::new();
    };
    shown
        .materials
        .iter()
        .zip(&names.materials)
        .map(|(h, m)| {
            let track = uva.tracks.iter().find(|tr| tr.name.strip_prefix("@mtluva_") == Some(&m.name));
            // the frame's key: the last at or before it
            let at = |f: i32| track.and_then(|tr| tr.ticks.iter().rposition(|&k| k <= f * uva.ticks_per_frame as i32).map(|k| [tr.values[k][0], tr.values[k][1]]));
            (h.clone(), (0..16).map(|f| at(f).unwrap_or([0.0; 2])).collect())
        })
        .collect()
}

/// A court 5 crowd drawn only while this game-space point passes the game's view test (`crowd_shown`).
#[derive(Component)]
struct CrowdCull([f32; 3]);

/// The game's draw test for a point (types 63/66): more than a millimetre ahead of the camera and not so near that
/// `focal` (240 / tan of the horizontal half-angle) over its depth reaches 1000.
fn crowd_shown(depth: f32, focal: f32) -> bool {
    0.001 < depth && focal / depth < 1000.0
}

/// Runs on the drawn camera (any mode) once it has moved, like `net_hide::hide_around_eye`.
// ponytail: depth and focal from the drawn camera in f32, not the game's VU0 view matrix (rendering only)
fn cull_crowd(camera: Query<(&Transform, &Projection), With<crate::Orbit>>, window: Query<&Window>, mut q: Query<(&CrowdCull, &mut Visibility)>) {
    let Ok((t, Projection::Perspective(p))) = camera.single() else { return };
    let aspect = window.single().map_or(16.0 / 9.0, crate::play::widescreen::aspect);
    let focal = 240.0 * 0.75f32.max(1.0 / aspect) / (p.fov / 2.0).tan();
    for (c, mut v) in &mut q {
        // game space is Bevy's turned a half-turn about X
        let at = Vec3::new(c.0[0], -c.0[1], -c.0[2]);
        let shown = crowd_shown((at - t.translation).dot(*t.forward()), focal);
        v.set_if_neq(if shown { Visibility::Inherited } else { Visibility::Hidden });
    }
}

/// Court 5's creatures' atlas frames on their materials.
// ponytail: the original sets the frame at the start of the creature's tick (the frame drawn lags its state by a
// tick); left out (rendering only)
fn draw_crowd(npcs: Option<Res<Npcs>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let Some(npcs) = npcs else { return };
    for (c, parts) in &npcs.crowd {
        for (h, uv) in parts {
            let off = Vec2::from(uv[c.frame.clamp(0, 15) as usize]);
            if materials.get(h).is_some_and(|m| m.uv_transform.translation != off) {
                materials.get_mut(h).unwrap().uv_transform = bevy::math::Affine2::from_translation(off);
            }
        }
    }
}

/// A resync's `Npcs` (`snapshot::encode`): the court 5 creatures' state without their materials.
mod crowd_state {
    use super::{Handle, StandardMaterial, npc::Crowd};
    use serde::{Deserialize, Deserializer, Serializer};

    type Parts = Vec<(Handle<StandardMaterial>, Vec<[f32; 2]>)>;

    pub fn serialize<S: Serializer>(c: &[(Crowd, Parts)], s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(c.iter().map(|c| &c.0))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<(Crowd, Parts)>, D::Error> {
        Ok(Vec::<Crowd>::deserialize(d)?.into_iter().map(|c| (c, Vec::new())).collect())
    }
}

// and this machine's own entities and handles
impl super::snapshot::Wire for Npcs {
    fn local(&mut self, l: &Self) {
        if let (Some(u), Some(l)) = (&mut self.umpire, &l.umpire) {
            u.0 = l.0;
        }
        self.walkers.iter_mut().zip(&l.walkers).for_each(|(w, l)| w.0 = l.0);
        self.triggers.iter_mut().zip(&l.triggers).for_each(|(t, l)| t.0 = l.0);
        self.crowd.iter_mut().zip(&l.crowd).for_each(|(c, l)| c.1 = l.1.clone());
        (self.fade, self.uncull, self.sprites) = (l.fade.clone(), l.uncull.clone(), l.sprites.clone());
    }
}

/// The umpire's head turned toward `Umpire::head`, as the game's look-at does it: the target in the frame of
/// `Bip01Head` at its rest local under its posed parent, 0.7 of its turn about the head's x axis (atan2(y, −z), within
/// ±70°) and of its tilt about y (atan2(−x, √(y²+z²)), −25°..35°), applied before the rest local (the motion's own
/// head keys are dropped). Checked against the game's stored head local by `research/b78_head.py`.
fn look(g: Res<Game>, npcs: Option<Res<Npcs>>, rigs: Query<(&character::Rig, &Transform)>, mut joints: Query<&mut Transform, Without<character::Rig>>) {
    let Some((e, _)) = npcs.as_ref().and_then(|n| n.umpire.as_ref()) else { return };
    let Ok((rig, at)) = rigs.get(*e) else { return };
    let Some(head) = rig.data.joint("Bip01Head") else { return };
    let rest = rig.data.joints[head].rest;
    let mut m = rest.to_matrix();
    let mut j = rig.data.joints[head].parent;
    while let Some(k) = j {
        let Ok(tf) = joints.get(rig.joints[k]) else { return };
        m = tf.to_matrix() * m;
        j = rig.data.joints[k].parent;
    }
    let p = (at.to_matrix() * m).inverse().transform_point3(Vec3::from(g.umpire.head));
    let yaw = (0.7 * p.y.atan2(-p.z)).clamp(-70f32.to_radians(), 70f32.to_radians());
    let tilt = (0.7 * (-p.x).atan2(p.y.hypot(p.z))).clamp(-25f32.to_radians(), 35f32.to_radians());
    if let Ok(mut tf) = joints.get_mut(rig.joints[head]) {
        *tf = rest.with_rotation(rest.rotation * Quat::from_rotation_x(yaw) * Quat::from_rotation_y(tilt));
    }
}

#[cfg(test)]
mod b283_tests {
    #[test]
    fn crowd_shown_skips_behind_and_too_near() {
        assert!(super::crowd_shown(30.0, 700.0));
        assert!(!super::crowd_shown(-30.0, 700.0));
        assert!(!super::crowd_shown(0.0005, 700.0));
        assert!(!super::crowd_shown(0.5, 700.0));
    }
}
