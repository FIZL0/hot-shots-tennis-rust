//! The post-point cut-away (`hst_sim::cutaway`): when the players start their reactions to a point, a scripted
//! shot of the player who decided it takes over the match camera until the next point.

use bevy::prelude::*;
use hst_data::iso::Iso;
use hst_sim::cutaway::{self, CameraShot, Frames, M4, PickInput, Shot};
use hst_sim::motion::id;
use hst_sim::pose::node_world;
use hst_sim::score::Event;

use super::{Figure, Game, Phase, Player, player_matrix};
use crate::Args;
use crate::character::{CharacterData, Motion};

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Director {
    pub(super) shots: Vec<CameraShot>,
    lists: [Vec<u8>; 3],
    /// Each list's own counter.
    counters: [usize; 3],
    /// The side of the orbit flips on every cut-away.
    mirror: bool,
    /// The last hitter and the one before (the server and receiver before any hit).
    roles: [usize; 2],
    shot: Option<(Shot, Frames, usize)>,
    /// The serve's first tick has shown the shot.
    held: bool,
}

impl Director {
    /// A cut-away shot is on screen.
    pub(super) fn showing(&self) -> bool {
        self.shot.is_some()
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(FixedUpdate, (step, super::replay_camera::step).chain().after(super::simulate).before(super::start_effects));
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
        held: false,
    });
    commands.insert_resource(super::replay_camera::ReplayCamera::new(game.camera_shots(), game.replay_cameras()));
}

/// Bone `name`'s world matrix with the player's motion `id` as drawn at time `t` (clamped to its length), the player
/// standing at `pos`. Drawn (B502): a mod's own clip for the motion (its reactions 0x2c..0x2f can face up to half a
/// turn from the donor's the game plays), else the gameplay clip, so a shot frames the body on screen. Online, a mod's
/// gameplay clip (B556, `CharacterData::framed`): the camera decides the match's draws (`reaction_voices`), and a peer
/// without the mod's looks draws its donor.
pub(super) fn bone(p: &Player, data: &CharacterData, id: usize, t: f32, pos: [f32; 4], name: &str) -> Option<M4> {
    let sk = &data.skeleton;
    let c = data.framed(id)?;
    let n = sk.names.iter().position(|s| s == name)?;
    let mut m = player_matrix(p);
    m[3] = pos;
    Some(node_world(sk, &c.locals(sk, t.min(c.length)), n, &m))
}

/// The time on the clock of the clip framed for `m`: a mod's own clip runs on `Motion::drawn`, else the gameplay clock.
pub(super) fn drawn_time(data: &CharacterData, m: &Motion) -> f32 {
    if !data.frame_gameplay && data.visual.contains_key(&m.id) { m.drawn.sampled } else { m.clock.sampled }
}

/// Player `i`'s head and Spine2 as their reaction will leave them `t` frames in (as drawn: a mod's own clip starts
/// with the gameplay one and runs at its speed, so `t` frames in on either clock): a team reaction (and `gu_set`)
/// carries the player along its root path. One without a reaction (a body-hit player standing) holds `motion`, its
/// current one, where it stands.
fn predicted(g: &Game, i: usize, t: f32, motion: Option<usize>) -> Option<(M4, M4)> {
    let (p, data) = (&g.players[i], &g.data[i]);
    let Some(r) = p.root else {
        let (id, pos) = (motion?, [p.pos[0], p.pos[1], p.pos[2], 1.0]);
        return Some((bone(p, data, id, t, pos, "Bip01Head")?, bone(p, data, id, t, pos, "Bip01Spine2")?));
    };
    let len = data.motions.get(&r.motion)?.length;
    let pos = match data.paths.get(&r.motion) {
        Some(path) if r.motion == id::WON_GAME as usize || r.motion >= id::TEAM as usize => {
            let [fx, _, fz, _] = p.body.face.dir;
            let rows = [[p.hand * fz, 0.0, -(p.hand * fx), 0.0], [0.0, 1.0, 0.0, 0.0], [fx, 0.0, fz, 0.0]];
            cutaway::reaction_spot(path.at(t.min(len)), r.motion, g.chars[i], rows, r.base)
        }
        _ => [p.pos[0], p.pos[1], p.pos[2], 1.0],
    };
    Some((bone(p, data, r.motion, t, pos, "Bip01Head")?, bone(p, data, r.motion, t, pos, "Bip01Spine2")?))
}

pub(super) fn step(mut g: ResMut<Game>, mut d: ResMut<Director>, q: Query<(&Figure, &Motion)>) {
    // a replay runs on its own cameras (`replay_camera`); after the match-ending one its closing shot holds
    if g.instant.replaying {
        d.shot = None;
        return;
    }
    if g.last_hitter >= 0 && g.last_hitter as usize != d.roles[0] {
        d.roles = [g.last_hitter as usize, d.roles[0]];
    }
    let event = g.post.as_ref().filter(|p| p.reacted).and_then(|p| p.event);
    let (true, Some(event)) = (g.phase == Phase::Post, event) else {
        if g.phase == Phase::Serve && g.last_hitter < 0 {
            d.roles = [g.score.server as usize, g.score.receiver as usize];
        }
        // the game's camera takes the switch back a tick late: the serve's first tick still shows the shot
        if g.phase == Phase::Serve && !d.held && d.shot.as_ref().is_some_and(|s| s.0.fov != 0.0) {
            d.held = true;
            return show(&mut g, &mut d, &q);
        }
        if d.shot.take().is_some() {
            g.prev_view = g.cam.view; // a cut doesn't blend
        }
        d.held = false;
        return;
    };
    let d = &mut *d;
    if d.shot.is_none() {
        let game_end = matches!(event, Event::Game | Event::Set);
        let winner = |i: usize| i as i32 & 1 == g.post_winner & 1;
        let [shown, other] = cutaway::roles(d.roles, g.last_hitter, game_end, g.post_winner, g.instant.replays, |i| g.players[i].dive.is_some());
        // ponytail: 117 frames in, as every rally point recorded; the game adds 93 or 99 instead of 97 to its 20 under
        // two flags not traced yet
        let t = if game_end { 378.0 } else { 117.0 };
        let motion = |i: usize| q.iter().find(|(f, _)| f.0 == i).map(|(_, m)| m.id);
        let (Some(s), Some(o)) = (predicted(&g, shown, t, motion(shown)), predicted(&g, other, t, motion(other))) else {
            return;
        };
        let p = &g.players[shown];
        let frames = Frames {
            corner: cutaway::corner_frame([p.pos[0], 0.0, p.pos[2], 1.0]),
            shown: [cutaway::subject_frames(&s.0, &s.1), cutaway::subject_frames(&o.0, &o.1)],
            head: cutaway::IDENTITY,
            spot: [p.pos[0], p.pos[1], p.pos[2], 1.0],
        };
        let n = cutaway::pick(
            &d.lists,
            &mut d.counters,
            &PickInput {
                team: p.root.is_some_and(|r| r.motion >= id::TEAM as usize),
                game_end,
                lost: !winner(shown),
                low: frames.shown[0][2][3][1] > -0.8,
                replays: g.instant.replays,
                character: g.chars[shown],
            },
        );
        d.mirror = !d.mirror;
        info!("cut-away shot {n:#x} on player {shown} (other {other}), mirror {}", d.mirror);
        d.shot = Some((Shot::new(&d.shots, n, d.mirror), frames, shown));
        // picked the tick the reactions start, as the game; its camera switches to it (and steps it) the next
        return;
    }
    show(&mut g, d, &q);
}

/// Steps the shot and shows it.
fn show(g: &mut Game, d: &mut Director, q: &Query<(&Figure, &Motion)>) {
    let (shot, frames, shown) = d.shot.as_mut().expect("started");
    // the shown player's live head
    if let Some((_, m)) = q.iter().find(|(f, _)| f.0 == *shown) {
        let p = &g.players[*shown];
        frames.spot = [p.pos[0], p.pos[1], p.pos[2], 1.0];
        if let Some(h) = bone(p, &g.data[*shown], m.id, drawn_time(&g.data[*shown], m), [p.pos[0], p.pos[1], p.pos[2], 1.0], "Bip01Head") {
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

impl super::snapshot::Wire for Director {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// B502: a cut-away frames the clip drawn: a stock player's gameplay reaction as before, a mod's own reaction
    /// (0x2c..0x2f, up to half a turn from the donor's) on its own clock.
    #[test]
    fn frames_follow_the_drawn_clip() {
        let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
        let (mut a, mut b, mut c, mut d) = (Assets::default(), Assets::default(), Assets::default(), Assets::default());
        let mut data = character::load_disc(&mut iso, 6, 0, &mut a, &mut b, &mut c, &mut d).unwrap();
        let mut p = Player { hand: 1.0, ..default() };
        p.body.face.dir = [0.6, 0.0, 0.8, 0.0];
        let pos = [1.0, 0.0, 2.0, 1.0];
        let mut m = Motion::default();
        m.set(0x2c, 1.0, false, None, 0);
        (m.clock.sampled, m.drawn.sampled) = (5.0, 9.0);
        let gameplay = |id: usize, t: f32| {
            let (sk, c) = (&data.skeleton, &data.motions[&id]);
            let mut w = player_matrix(&p);
            w[3] = pos;
            node_world(sk, &c.locals(sk, t.min(c.length)), sk.names.iter().position(|s| s == "Bip01Head").unwrap(), &w)
        };
        let (stock, own) = (gameplay(0x2c, 5.0), gameplay(0x10, 9.0));
        assert_eq!(drawn_time(&data, &m), 5.0, "stock clock changed");
        assert_eq!(bone(&p, &data, 0x2c, drawn_time(&data, &m), pos, "Bip01Head").unwrap(), stock, "stock frames changed");
        data.visual.insert(0x2c, data.motions[&0x10].clone());
        assert_eq!(drawn_time(&data, &m), 9.0, "not the drawn clock");
        assert_eq!(bone(&p, &data, 0x2c, drawn_time(&data, &m), pos, "Bip01Head").unwrap(), own, "not the drawn clip");
    }
}
