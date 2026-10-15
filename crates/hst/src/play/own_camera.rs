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
    /// B299: whether the shared camera showed the plain match view after the newest tick. Kept from that tick, not
    /// compared each frame: a rollback that re-runs ticks without a new one (catching up after a tab-out, a resync)
    /// leaves the shared camera on corrected scenes this one hasn't stepped, and the picture flipped to it.
    plain: bool,
    fresh: bool,
}

pub fn plugin(app: &mut App) {
    // B315: `plain` reads the view the tick ends on, after the cut-away, replay camera and change-ends glide have
    // written theirs (ordered only after `simulate`, it could see the plain view under a cut-away and hide it)
    let plain = plain.after(super::simulate).after(super::cutaway::step).after(super::replay_camera::step).after(super::change_ends::step);
    app.add_systems(FixedUpdate, (link.before(super::control), step.after(super::control).before(super::simulate), plain))
        // B230: made with the match, so the intro (which holds the fixed ticks) already shows this machine's side
        .add_systems(PostStartup, link.after(super::setup))
        .add_systems(Update, listener);
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
        commands.insert_resource(own_cam(&g, owner));
    }
}

fn own_cam(g: &Game, owner: usize) -> OwnCam {
    let view = g.cam.view;
    let mut cam = Camera::from_view(view);
    cam.turned = g.players[owner].end < 0.0;
    cam.cut();
    OwnCam { cam, prev: view, expect: view, owner, tick: 0, cut: false, plain: true, fresh: false }
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
    let (Some(g), Some(mut o)) = (g, own) else { return };
    o.cut |= g.cam_cut;
    // without a lockstep (a test's own camera) every tick is new
    if let Some(np) = np {
        let tick = np.lock.log.len();
        if tick <= o.tick {
            return;
        }
        o.tick = tick;
    }
    o.fresh = true;
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

/// After the tick `step` stepped: does the shared camera still show the plain match view?
fn plain(g: Option<Res<Game>>, own: Option<ResMut<OwnCam>>) {
    let (Some(g), Some(mut o)) = (g, own) else { return };
    if std::mem::take(&mut o.fresh) {
        o.plain = g.cam.view == o.expect;
    }
}

/// The views the picture blends between this frame: this machine's own camera's while the shared one shows the
/// plain match view, else the shared camera's (`g.prev_view`, `g.cam.view`).
pub(super) fn shown(g: &Game, own: Option<&OwnCam>) -> (View, View) {
    // before the match's first tick (the intro holds them) neither camera has stepped: the view its first step cuts to
    // (B315: the point count restarts at each serve's set-up, whose tick still shows the cut-away; the match's time
    // doesn't)
    if g.instant.ticks == 0 && g.match_stats.frames == 0 {
        let mut c = own.map_or(g.cam, |o| o.cam);
        let players: Vec<_> = g.players.iter().map(|p| p.pos).collect();
        c.step(&Scene { players: &players, ball: g.flight.ball.pos });
        return (c.view, c.view);
    }
    match own {
        Some(o) if o.plain => (o.prev, o.cam.view),
        _ => (g.prev_view, g.cam.view),
    }
}

/// Positional sounds are heard from the end this machine's match camera looks from (`audio::set_turned`).
fn listener(g: Option<Res<Game>>, own: Option<Res<OwnCam>>) {
    crate::audio::set_turned(g.is_some_and(|g| own.map_or(g.cam.turned, |o| o.cam.turned)));
}

#[cfg(test)]
mod tests {
    use super::cam_of;
    use super::super::*;
    use crate::{Args, GameSpace};
    use bevy::mesh::skinning::SkinnedMeshInverseBindposes;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// B315: with this machine's own camera on (online), a post-point cut-away still takes the
    /// picture: from its second tick on (its first only picks the shot) the picture shows the shared camera's
    /// view, the shot's (skipped without the ISO).
    #[test]
    fn a_cut_away_takes_the_own_cameras_picture() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        // SAFETY: only `Pads::slot_of` reads it; every player is a bot
        unsafe { std::env::set_var("HST_AUTOPLAY", "1") };
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, change_ends::plugin, super::plugin));
        app.add_systems(Startup, setup);
        // the match's time, as `match_stats` keeps it
        let time = |mut g: ResMut<Game>| g.match_stats.tick();
        app.add_systems(FixedUpdate, (remember, control, simulate, time, motions, character::tick, played_out, held_ball).chain());
        app.update();
        // following player 1, at the other end from the shared camera's: its picture differs from the shared one
        let own = super::own_cam(app.world().resource::<Game>(), 1);
        app.insert_resource(own);
        let (mut shots, mut was, mut differs) = (0, false, false);
        for _ in 0..60 * 120 {
            app.world_mut().run_schedule(FixedUpdate);
            let w = app.world();
            let (g, own) = (w.resource::<Game>(), w.resource::<super::OwnCam>());
            let showing = w.resource::<cutaway::Director>().showing();
            let (mine, shared) = (super::shown(g, Some(own)), super::shown(g, None));
            if showing && was {
                assert_eq!(mine, shared, "cut-away {shots}: the picture isn't the shot");
            }
            differs |= !showing && mine != shared;
            shots += (showing && !was) as u32;
            was = showing;
            if shots == 3 && !showing {
                break;
            }
        }
        assert!(shots >= 2, "only {shots} cut-aways");
        assert!(differs, "the own camera never showed its own view");
    }

    #[test]
    fn each_human_follows_its_machines_first_player() {
        // singles: two machines
        assert_eq!(cam_of(&[Some("a"), Some("b")]), [0, 1]);
        // doubles, room seats 0..3 = players 0, 2, 1, 3: a has seats 0 and 3 (players 0 and 3), b seat 1 (player 2),
        // seat 2 (player 1) the computer
        assert_eq!(cam_of(&[Some("a"), Some("b"), None, Some("a")]), [0, 1, 2, 0]);
    }
}
