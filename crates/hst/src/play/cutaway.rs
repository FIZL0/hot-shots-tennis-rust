//! The post-point cut-away (`hst_sim::cutaway`): when the players start their reactions to a point, a scripted
//! shot of the player who decided it takes over the match camera until the next point.

use bevy::prelude::*;
use hst_data::iso::Iso;
use hst_sim::cutaway::{self, CameraShot, Frames, M4, PickInput, Shot};
use hst_sim::pose::node_world;
use hst_sim::score::Event;

use super::{Figure, Game, Phase, Player, player_matrix};
use crate::Args;
use crate::character::{CharacterData, Motion};

#[derive(Resource)]
struct Director {
    shots: Vec<CameraShot>,
    lists: [Vec<u8>; 3],
    /// Each list's own counter.
    counters: [usize; 3],
    /// The side of the orbit flips on every cut-away.
    mirror: bool,
    /// The last hitter and the one before (the server and receiver before any hit).
    roles: [usize; 2],
    shot: Option<(Shot, Frames, usize)>,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(FixedUpdate, step.after(super::simulate).before(super::start_effects));
}

fn setup(mut commands: Commands, args: Res<Args>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
    let game = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
    commands.insert_resource(Director {
        shots: game.camera_shots(),
        lists: game.cutaway_lists(),
        counters: [0; 3],
        mirror: true,
        roles: [0, 1],
        shot: None,
    });
}

/// Bone `name`'s world matrix with the player's motion `id` at time `t` (clamped to its length), the player
/// standing at `pos`.
fn bone(p: &Player, data: &CharacterData, id: usize, t: f32, pos: [f32; 4], name: &str) -> Option<M4> {
    let sk = &data.skeleton;
    let c = data.motions.get(&id)?;
    let n = sk.names.iter().position(|s| s == name)?;
    let mut m = player_matrix(p);
    m[3] = pos;
    Some(node_world(sk, &c.locals(sk, t.min(c.length)), n, &m))
}

/// Player `i`'s head and Spine2 as their reaction will leave them `t` frames in: a team reaction (and `gu_set`)
/// carries the player along its root path.
fn predicted(g: &Game, i: usize, t: f32) -> Option<(M4, M4)> {
    let (p, data) = (&g.players[i], &g.data[i]);
    let r = p.root?;
    let len = data.motions.get(&r.motion)?.length;
    let pos = match data.paths.get(&r.motion) {
        Some(path) if r.motion == 0x2e || r.motion >= 0x30 => {
            let [fx, _, fz, _] = p.body.face.dir;
            let rows = [[p.hand * fz, 0.0, -(p.hand * fx), 0.0], [0.0, 1.0, 0.0, 0.0], [fx, 0.0, fz, 0.0]];
            hst_sim::motion::reaction_root(path.at(t.min(len)), r.motion >= 0x30, g.chars[i], rows, r.base, r.base)
        }
        _ => [p.pos[0], p.pos[1], p.pos[2], 1.0],
    };
    Some((bone(p, data, r.motion, t, pos, "Bip01Head")?, bone(p, data, r.motion, t, pos, "Bip01Spine2")?))
}

fn step(mut g: ResMut<Game>, mut d: ResMut<Director>, q: Query<(&Figure, &Motion)>) {
    if g.last_hitter >= 0 && g.last_hitter as usize != d.roles[0] {
        d.roles = [g.last_hitter as usize, d.roles[0]];
    }
    let event = g.post.as_ref().filter(|p| p.reacted).and_then(|p| p.event);
    let (true, Some(event)) = (g.phase == Phase::Post, event) else {
        if g.phase == Phase::Serve && g.last_hitter < 0 {
            d.roles = [g.score.server as usize, g.score.receiver as usize];
        }
        d.shot = None;
        return;
    };
    let d = &mut *d;
    if d.shot.is_none() {
        let game_end = matches!(event, Event::Game | Event::Set);
        let [a, b] = d.roles;
        let winner = |i: usize| i as i32 & 1 == g.post_winner & 1;
        let swap = if game_end { !winner(a) } else { g.last_hitter >= 0 && a as i32 & 1 != g.last_hitter & 1 };
        let (shown, other) = if swap { (b, a) } else { (a, b) };
        // ponytail: 117 frames in, as every rally point recorded; the game adds 93 or 99 instead of 97 to its 20 under
        // two flags not traced yet
        let t = if game_end { 378.0 } else { 117.0 };
        let (Some(s), Some(o)) = (predicted(&g, shown, t), predicted(&g, other, t)) else {
            return;
        };
        let p = &g.players[shown];
        let frames = Frames {
            corner: cutaway::corner_frame([p.pos[0], 0.0, p.pos[2], 1.0]),
            shown: [cutaway::subject_frames(&s.0, &s.1), cutaway::subject_frames(&o.0, &o.1)],
            head: cutaway::IDENTITY,
        };
        let n = cutaway::pick(
            &d.lists,
            &mut d.counters,
            &PickInput {
                team: p.root.is_some_and(|r| r.motion >= 0x30),
                game_end,
                lost: !winner(shown),
                low: frames.shown[0][2][3][1] > -0.8,
                replays: 0,
                character: g.chars[shown],
            },
        );
        d.mirror = !d.mirror;
        info!("cut-away shot {n:#x} on player {shown} (other {other}), mirror {}", d.mirror);
        d.shot = Some((Shot::new(&d.shots, n, d.mirror), frames, shown));
    }
    let (shot, frames, shown) = d.shot.as_mut().expect("started");
    // the shown player's live head
    if let Some((_, m)) = q.iter().find(|(f, _)| f.0 == *shown) {
        let p = &g.players[*shown];
        if let Some(h) = bone(p, &g.data[*shown], m.id, m.clock.sampled, [p.pos[0], p.pos[1], p.pos[2], 1.0], "Bip01Head") {
            frames.head = cutaway::head_frame(&h, p.hand < 0.0);
        }
    }
    let first = shot.fov == 0.0;
    let f = *frames;
    shot.step(|i| f.get(i));
    g.cam.view = shot.view();
    if first {
        g.prev_view = g.cam.view; // a cut doesn't blend
    }
}
