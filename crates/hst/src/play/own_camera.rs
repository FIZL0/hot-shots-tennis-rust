//! Online, each machine's picture is the original's one-human match camera on its own player (B218): behind that
//! player's end, turned round with it at a change of ends, as a solo game's camera. Every machine counts all the
//! match's humans, so the shared match camera (`Game::cam`, in the state hash: the reaction voices and foot steps
//! test what it sees) keeps its end like a local multi-human game's and frames only player 0's team near it; a
//! player at the other end ran off the top of the picture.
//!
//! The shared camera stays as it is. This machine steps its own copy of it, turned for its own player, on the same
//! scene each tick, and shows that instead whenever the shared camera shows the plain match view (not while a
//! cut-away, replay or the change-ends glide has it). Each human's stick turns with the camera of the machine
//! driving it (`Game::cam_of`, the same on every machine), so the controls stay screen-relative to its own picture.

use bevy::prelude::*;
use hst_sim::camera::{Camera, Scene, View};

use super::Game;
use super::netplay::Netplay;

/// This machine's own match camera.
#[derive(Resource)]
pub(super) struct OwnCam {
    cam: Camera,
    prev: View,
    /// The shared camera's own step this tick: while it still shows that, so does this one.
    expect: View,
    /// The player it follows.
    owner: usize,
    /// The newest tick it stepped (the lockstep's pad log length): a rollback's re-run ticks don't step it again,
    /// though a placement's cut among them is kept for the next new tick.
    tick: usize,
    cut: bool,
}

pub fn plugin(app: &mut App) {
    app.add_systems(FixedUpdate, (link.before(super::control), step.after(super::control).before(super::simulate)));
}

/// Room seat index of player `i` among `n` (as `Pads::slot_of`).
fn seat(i: usize, n: usize) -> usize {
    if n == 4 { [0, 2, 1, 3][i] } else { i }
}

/// Per player, the first player (by number) driven from the same machine, the one its camera follows.
fn cam_of(peers: &[Option<&str>]) -> Vec<usize> {
    let n = peers.len();
    let peer = |i: usize| peers[seat(i, n)];
    (0..n).map(|i| peer(i).and_then(|p| (0..n).find(|&j| peer(j) == Some(p))).unwrap_or(i)).collect()
}

fn link(np: Option<Res<Netplay>>, g: Option<ResMut<Game>>, own: Option<Res<OwnCam>>, mut commands: Commands) {
    let (Some(np), Some(mut g)) = (np, g) else { return };
    let n = g.players.len();
    let peers: Vec<_> =
        (0..n).map(|s| np.room.seats.get(s).cloned().flatten().filter(|x| x.human()).map(|x| x.peer)).collect();
    let peers: Vec<_> = peers.iter().map(Option::as_deref).collect();
    let of = cam_of(&peers);
    if g.cam_of != of {
        g.cam_of = of;
    }
    if own.is_none()
        && let Some(owner) = (0..n).find(|&i| peers[seat(i, n)] == Some(np.me.as_str()))
    {
        let view = g.cam.view;
        commands.insert_resource(OwnCam { cam: Camera::from_view(view), prev: view, expect: view, owner, tick: 0, cut: false });
    }
}

/// Whether player `i`'s stick is turned round: online, its machine's camera looks from the +z end; else the
/// shared camera's.
pub(super) fn flipped(g: &Game, i: usize) -> bool {
    match g.cam_of.get(i) {
        Some(&o) => g.players[o].end < 0.0,
        None => g.cam.view.eye[2] >= 0.0,
    }
}

fn step(g: Option<Res<Game>>, own: Option<ResMut<OwnCam>>, np: Option<Res<Netplay>>) {
    let (Some(g), Some(mut o), Some(np)) = (g, own, np) else { return };
    o.cut |= g.cam_cut;
    let tick = np.lock.log.len();
    if tick <= o.tick {
        return;
    }
    o.tick = tick;
    let players: Vec<_> = g.players.iter().map(|p| p.pos).collect();
    let scene = Scene { players: &players, ball: g.flight.ball.pos };
    let mut shared = g.cam;
    shared.step(&scene);
    o.expect = shared.view;
    let turned = g.players[o.owner].end < 0.0;
    // a new point's placement (the shared camera's cut), or its end changed (cut straight to it)
    let cut = std::mem::take(&mut o.cut) || o.cam.turned != turned;
    if cut {
        o.cam.cut();
        o.cam.turned = turned;
    }
    o.prev = o.cam.view;
    o.cam.step(&scene);
    if cut {
        o.prev = o.cam.view;
    }
}

/// The views the picture blends between this frame: this machine's own camera's while the shared one shows the
/// plain match view, else the shared camera's (`g.prev_view`, `g.cam.view`).
pub(super) fn shown(g: &Game, own: Option<&OwnCam>) -> (View, View) {
    match own {
        Some(o) if g.cam.view == o.expect => (o.prev, o.cam.view),
        _ => (g.prev_view, g.cam.view),
    }
}

#[cfg(test)]
mod tests {
    use super::cam_of;

    #[test]
    fn each_human_follows_its_machines_first_player() {
        // singles: two machines
        assert_eq!(cam_of(&[Some("a"), Some("b")]), [0, 1]);
        // doubles, room seats 0..3 = players 0, 2, 1, 3: a has seats 0 and 3 (players 0 and 3), b seat 1 (player 2),
        // seat 2 (player 1) the computer
        assert_eq!(cam_of(&[Some("a"), Some("b"), None, Some("a")]), [0, 1, 2, 0]);
    }
}
