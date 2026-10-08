//! The court's background figures as the original draws them, from the court's `COURTSET/NN/HOLE01.XB`: the umpire
//! (Lily, `u04`) on her chair, the walking spectators (`g-MM` models, `npcSS` animation sets) driven by
//! [`npc::Walker`], and the trigger creatures with a model (`exe::Game::trigger_model`) driven by [`npc::Trigger`].
//!
//! The umpire faces the court centre. Her motion (0 idle, 1/2 turned toward the point's winner) restarts at frame
//! 0 when it changes and steps one frame a tick while turned or once the match is over (idle stays on frame 0
//! during play); a turn that reaches its end goes back to frame 16 and plays on from there.
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
        .add_systems(Update, draw_cheers.after(super::camera));
}

#[derive(Resource)]
struct Npcs {
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
    /// Changing ends last tick (court 4's passing ball rolls when it starts).
    ends: bool,
    /// Rigs whose animation carries them far from their bind pose (the passing ball): not culled by its bounds.
    uncull: Vec<Entity>,
    /// The gallery's cheer marks (where a walker started cheering); stepped after the walkers.
    cheers: npc::Cheers,
    /// Court 5's own cheer marks about its stands (empty on the other courts).
    marks: Vec<[f32; 4]>,
    /// The trigger creatures and the sound emitters (`Game::emitters`, `None`) in spawn order.
    figures: Vec<Option<usize>>,
    /// The cheer marks' sprites (`npc/clap`) and their sizes by kind (`exe::Game::cheer_sprites`).
    sprites: Handle<Mesh>,
    sizes: [[f32; 5]; 2],
    /// The match's first point has been served (the walkers' match-start cheer ends then).
    started: bool,
    /// The match was decided: whether the favoured team won, until the next serve.
    over: Option<bool>,
}

/// The umpire's animation controller: `frame` shown, `next` the one after.
#[derive(Default)]
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
    let mut hidden = Vec::new();
    let mut npcs = Npcs { umpire: None, walkers: Vec::new(), triggers: Vec::new(), tick: 0, motion: 0, decided: false, near: default(), ends: false, uncull: Vec::new(), cheers: npc::Cheers::new(g.stage), marks: Vec::new(), figures: Vec::new(), sprites: default(), sizes: game.cheer_sprites(), started: false, over: None };
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
                let mut w = npc::Walker { slot, mode: 0, counter: 0, anim: 0, frame: 0.0, next: 0.0, speed: 1.0, advancing: true, lens };
                // the match's start: its cheer, and a mark where it stands
                w.start(players, &mut roll);
                npcs.cheers.add([c.world[3][0], c.world[3][1], c.world[3][2]], slot);
                // the serve placement before the first point turns it to the court centre
                let world = npc::walker_facing(c.world[3], 0.0);
                npcs.walkers.push((place(&data, &world, c.scale), w, c.world[3]));
            }
            npc::Kind::Trigger(t) => {
                if npc::EMITTERS.contains(&t) {
                    npcs.figures.push(None);
                }
                let Some((model, base, clips)) = game.trigger_model(t) else { continue };
                let Some(data) = load(layout::trigger_files(&model, &base, clips)) else { continue };
                let row = game.trigger(t);
                // ponytail: no path (the creatures that move along one stand at their home); a two-clip creature
                // plays its first
                let len = data.motions.get(&0).map_or(0.0, |c| c.length);
                let routes = game.ball_routes();
                let mut tr = npc::Trigger { ty: t, anchor: c.world[3], home: c.world, world: c.world, len, routes, ..default() };
                if t == 44 {
                    tr.give_voice(&row, voices.next().unwrap_or((-1, 0)), &mut roll);
                }
                tr.reset(&row, &mut roll);
                let e = place(&data, &c.world, c.scale);
                if t == 15 {
                    hidden.push(e); // the passing ball waits off until a change of ends rolls it
                }
                npcs.figures.push(Some(npcs.triggers.len()));
                npcs.triggers.push((e, tr, row));
            }
            // ponytail: court 5's own creatures are not drawn (their models are not found yet)
            npc::Kind::Court5(_) => {}
        }
    }
    // then the manager's match start zeroes the walkers' marks; court 5 shows its own (with more than one player)
    npcs.cheers.clear();
    if g.stage == 5 {
        npcs.marks = game.court5_marks();
        if players > 1 {
            npcs.cheers.court5(&npcs.marks);
        }
    }
    for &e in &hidden {
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

/// Show clip `id` at `frame` (the tick's sampled time; `character::tick` already moved the last one to `prev`).
fn show(m: &mut Motion, id: usize, frame: f32, looping: bool) {
    if m.id != id {
        m.set(id, 0.0, looping, None, 0);
    }
    m.clock.looping = looping;
    m.clock.sampled = frame;
}

fn step(
    mut g: ResMut<Game>,
    npcs: Option<ResMut<Npcs>>,
    mut q: Query<(&character::Rig, &mut Motion)>,
    mut turn: Query<&mut Transform, With<character::Rig>>,
    mut shown: Query<&mut Visibility, With<character::Rig>>,
    mut commands: Commands,
    children: Query<&Children>,
) {
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
    let (motion, over, players, serve) = (g.umpire.motion, g.umpire.over, g.rules.players as u32, g.phase == Phase::Serve);
    let ends = matches!(g.phase, Phase::ChangeEnds(_));
    let played = g.score.sets[0] + g.score.sets[1];
    npcs.near.deciding = g.score.tiebreak && played + 1 == 2 * g.rules.sets - 1;
    // a decided point turns the walkers to the middle of the winners' half (by the side their first player is on)
    let side = if 0.0 <= g.players.get(g.post_winner as usize).map_or(0.0, |p| p.pos[2]) { 6.4 } else { -6.4 };
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
    // the match's first serve (a new point straight after its start): the idle loops restart one walker per tick
    // (with more than one player), the marks go
    if serve && !npcs.started {
        npcs.started = true;
        npcs.tick = 0;
        npcs.cheers.clear();
        npcs.walkers.iter_mut().for_each(|(_, w, _)| w.new_point(players > 1));
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
    if decided {
        for (e, w, home) in &mut npcs.walkers {
            w.react();
            face(&mut turn, *e, *home, side);
        }
        npcs.decided = true;
    }
    // the serve after it: a new point
    if npcs.decided && serve {
        npcs.decided = false;
        npcs.tick = 0;
        npcs.cheers.clear();
        for (e, w, home) in &mut npcs.walkers {
            // the serve placement turns them back to the court centre
            face(&mut turn, *e, *home, 0.0);
            // the match's first point (no stagger) comes before any decided one
            w.new_point(players >= 3);
        }
        for (_, t, row) in &mut npcs.triggers {
            t.reset_near(row, &mut npcs.near, &mut roll);
        }
    }
    // the change of ends (the match's state 1): its reset of the passing ball (type 15) is the one that rolls
    if ends && !npcs.ends {
        (npcs.near.ends, npcs.near.players) = (true, players);
        npcs.cheers.clear();
        for (_, t, row) in npcs.triggers.iter_mut().filter(|(_, t, _)| t.ty == 15) {
            t.reset_near(row, &mut npcs.near, &mut roll);
        }
        npcs.near.ends = false;
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
                if motion != 0 && len <= a.frame {
                    a.set(16.0, len);
                }
            }
            // ponytail: her head's look-at (toward `Umpire::head`) and the texture animation are not drawn
            show(&mut m, motion as usize, a.frame, false);
        }
    }
    npcs.motion = motion;
    for (k, (e, w, home)) in npcs.walkers.iter_mut().enumerate() {
        let react = w.mode == 1;
        w.step(players, cheer[k], npcs.tick, &mut roll);
        // a walker breaking into its cheer leaves a mark where it stands
        if react && w.anim == 5 {
            npcs.cheers.add([home[0], home[1], home[2]], w.slot);
        }
        if let Ok((_, mut m)) = q.get_mut(*e) {
            show(&mut m, w.anim as usize, w.frame, matches!(w.anim, 3 | 5));
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
    for &f in &npcs.figures {
        let Some(k) = f else {
            if let Some(k) = emitters.next() {
                emit(&mut g.emitters[k], &mut g.sounds, &mut roll);
            }
            continue;
        };
        let (e, t, row) = &mut npcs.triggers[k];
        // ponytail: a type's startled flag stays set until the next point (the original clears it sooner, from code
        // not found); of message 0x14's other listeners the landing markers don't hear it
        for s in t.step_near(row, &mut npcs.near, &mut roll) {
            let at = [t.world[3][0], t.world[3][1], t.world[3][2]];
            g.sounds.push((sound::Play { slot: 0, program: 7, key: s as u8, volume: 0x40, speed: 1.0 }, at));
        }
        // struck by the ball (27–29): message 0x14, the ball's step back off it before its next own step
        // ponytail: a struck toss doesn't match (the original stops it dead; tracked in PLAN)
        if t.struck {
            let (shot, material) = (g.shot, g.world.1[0]);
            g.flight.step_plane(&shot, &hst_sim::ball::COURTS[g.court], material);
            g.doubles_ai.hear(&g.players, 0x14);
        }
        if let Ok((_, mut m)) = q.get_mut(*e) {
            m.clock.sampled = t.frame;
        }
        // the passing ball: drawn while on with its fade (+0x274) above 0, from where its route starts
        // ponytail: the fade's alpha over its last 10 frames is not drawn; the other types' `on` is not used to hide them
        // type 44 stands turned to the court centre in the deciding set
        if t.ty == 44 {
            if let Ok(mut tf) = turn.get_mut(*e) {
                let scale = tf.scale;
                *tf = Transform::from_matrix(Mat4::from_cols_array_2d(&t.world)).with_scale(scale);
            }
        }
        if t.ty == 15 {
            if let Ok(mut v) = shown.get_mut(*e) {
                *v = if t.active && t.on && t.speed != 0.0 { Visibility::Inherited } else { Visibility::Hidden };
            }
            if let Ok(mut tf) = turn.get_mut(*e) {
                let scale = tf.scale;
                *tf = Transform::from_matrix(Mat4::from_cols_array_2d(&t.world)).with_scale(scale);
            }
        }
    }
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
