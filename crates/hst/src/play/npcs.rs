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
//! ([`npc::cheerers`]) on the tick after the point, one tick after the original.
//!
//! The walkers stand at their home facing the court centre (each serve placement turns them there) and turn to the
//! middle of the winners' half when they react ([`npc::walker_facing`]).

use std::sync::Arc;

use bevy::prelude::*;
use hst_data::{exe, iso::Iso, layout, xb::Archive};
use hst_sim::npc;

use super::{rand, Game, Phase};
use crate::character::{self, CharacterData, Motion};
use crate::{Args, GameSpace};

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, step.after(character::tick));
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
    let mut npcs = Npcs { umpire: None, walkers: Vec::new(), triggers: Vec::new(), tick: 0, motion: 0, decided: false, near: default() };
    let (umpire, rng) = (g.umpire.clone(), &mut g.rng);
    let mut roll = || {
        rand(rng);
        *rng
    };
    for c in npc::spawn(&list, &plants, &game.npc_roster(n as u32), &walkers, players) {
        match c.kind {
            npc::Kind::Umpire => {
                let Some(data) = load(layout::umpire_files(4)) else { continue };
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
                let w = npc::Walker { slot, mode: 0, counter: 0, anim: 0, frame: 0.0, next: 0.0, speed: 1.0, advancing: true, lens };
                // the serve placement before the first point turns it to the court centre
                let world = npc::walker_facing(c.world[3], 0.0);
                npcs.walkers.push((place(&data, &world, c.scale), w, c.world[3]));
            }
            npc::Kind::Trigger(t) => {
                let Some((model, base, clips)) = game.trigger_model(t) else { continue };
                let Some(data) = load(layout::trigger_files(&model, &base, clips)) else { continue };
                let row = game.trigger(t);
                // ponytail: no path (the creatures that move along one stand at their home); a two-clip creature
                // plays its first
                let len = data.motions.get(&0).map_or(0.0, |c| c.length);
                let mut tr = npc::Trigger { ty: t, anchor: c.world[3], home: c.world, world: c.world, len, ..default() };
                tr.reset(&row, &mut roll);
                npcs.triggers.push((place(&data, &c.world, c.scale), tr, row));
            }
            // ponytail: court 5's own creatures are not drawn (their models are not found yet)
            npc::Kind::Court5(_) => {}
        }
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
) {
    let Some(mut npcs) = npcs else { return };
    let npcs = &mut *npcs;
    let at = |p: [f32; 3]| [p[0], p[1], p[2], 1.0];
    npcs.near.pos = g.players.iter().map(|p| at(p.pos)).chain([at(g.flight.ball.pos)]).collect();
    let (motion, over, players, serve) = (g.umpire.motion, g.umpire.over, g.rules.players as u32, g.phase == Phase::Serve);
    // a decided point turns the walkers to the middle of the winners' half (by the side their first player is on)
    let side = if 0.0 <= g.players.get(g.post_winner as usize).map_or(0.0, |p| p.pos[2]) { 6.4 } else { -6.4 };
    let rng = &mut g.rng;
    let mut roll = || {
        rand(rng);
        *rng
    };
    // a decided point: the walkers react, some cheering
    let cheer = if motion != 0 && npcs.motion == 0 {
        for (e, w, home) in &mut npcs.walkers {
            w.react();
            face(&mut turn, *e, *home, side);
        }
        npcs.decided = true;
        npc::cheerers(npcs.walkers.len(), &mut roll)
    } else {
        [false; 6]
    };
    // the serve after it: a new point
    if npcs.decided && serve {
        npcs.decided = false;
        npcs.tick = 0;
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
    for (k, (e, w, _)) in npcs.walkers.iter_mut().enumerate() {
        w.step(players, cheer[k], npcs.tick, &mut roll);
        if let Ok((_, mut m)) = q.get_mut(*e) {
            show(&mut m, w.anim as usize, w.frame, matches!(w.anim, 3 | 5));
        }
    }
    npcs.tick += 1;
    for (e, t, row) in &mut npcs.triggers {
        // ponytail: their sounds and a hit's message (`struck`) are not played; a type's startled flag stays set
        // until the next point (the original clears it sooner, from code not found)
        t.step_near(row, &mut npcs.near, &mut roll);
        if let Ok((_, mut m)) = q.get_mut(*e) {
            m.clock.sampled = t.frame;
        }
    }
}
