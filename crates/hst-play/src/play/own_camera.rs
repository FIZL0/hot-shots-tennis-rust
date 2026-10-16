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
//!
//! B486: the change-ends glide is this machine's own too. The shared camera's glides to the shared camera's end, behind
//! the player there: shown on the machine whose player had moved to the other end, it put that player at the far end
//! and the picture snapped round to the own camera when it ended. Each machine glides to its own camera's view of the
//! new set-up, round the player at its end, the same move as a solo game's.
//!
//! B534: a local game whose humans are all on one team (2P doubles against two computers) is the same case: the
//! shared camera keeps its end, as with more than one human, so after a change of ends the humans played from the far
//! end. This machine's camera follows its first human there too, and every human's stick turns with it.

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
    /// B486: through a change of ends, the views of this machine's own glide the picture blends between this tick.
    glide: Option<(View, View)>,
}

pub fn plugin(app: &mut App) {
    // B315: `plain` reads the view the tick ends on, after the cut-away, replay camera and change-ends glide have
    // written theirs (ordered only after `simulate`, it could see the plain view under a cut-away and hide it).
    // B376: and after `start_effects`, which every system that moves the shared camera runs before (the host's
    // Steam Deck showed no cut-away)
    let plain = plain.after(super::simulate).after(super::cutaway::step).after(super::replay_camera::step).after(super::change_ends::step).after(super::start_effects);
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

fn link(np: Option<Res<Netplay>>, g: Option<ResMut<Game>>, own: Option<Res<OwnCam>>, pads: Option<Res<super::Pads>>, mut commands: Commands) {
    let Some(mut g) = g else { return };
    let Some(np) = np else {
        if own.is_none()
            && let Some(pads) = pads
            && let Some((owner, of)) = local(&g, &pads)
        {
            g.cam_of = of;
            commands.insert_resource(own_cam(&g, owner));
        }
        return;
    };
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

/// B534: offline, with two or more humans all at one end, the first of them (whose end this machine's camera
/// follows) and each player's camera (`Game::cam_of`: the humans that one's); else `None` (one human: the shared
/// camera turns with it; humans on both teams: it keeps its end).
fn local(g: &Game, pads: &super::Pads) -> Option<(usize, Vec<usize>)> {
    let n = g.players.len();
    let humans: Vec<usize> = (0..n).filter(|&i| pads.slot_of(i, n).is_some()).collect();
    let &owner = humans.first()?;
    let side = |i: usize| g.players[i].end < 0.0;
    if humans.len() < 2 || humans.iter().any(|&i| side(i) != side(owner)) {
        return None;
    }
    Some((owner, (0..n).map(|i| if humans.contains(&i) { owner } else { i }).collect()))
}

fn own_cam(g: &Game, owner: usize) -> OwnCam {
    let view = g.cam.view;
    let mut cam = Camera::from_view(view);
    cam.turned = g.players[owner].end < 0.0;
    cam.cut();
    OwnCam { cam, prev: view, expect: view, owner, tick: 0, cut: false, plain: true, fresh: false, glide: None }
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
/// B486: and through a change of ends, this machine's glide at the tick it ends on.
fn plain(g: Option<Res<Game>>, own: Option<ResMut<OwnCam>>, d: Option<Res<super::cutaway::Director>>) {
    let (Some(g), Some(mut o)) = (g, own) else { return };
    if std::mem::take(&mut o.fresh) {
        o.plain = g.cam.view == o.expect;
    }
    o.glide = glide(&g, o.owner, d.as_deref());
}

/// The own camera's change-ends glide at the tick `g` ends on (previous and current view): a function of the phase's
/// count and the new set-up only, so a rollback's re-run ticks come out the same as the first run.
fn glide(g: &Game, owner: usize, d: Option<&super::cutaway::Director>) -> Option<(View, View)> {
    use super::change_ends::{at, start};
    let super::Phase::ChangeEnds(n) = g.phase else { return None };
    // the end view: this machine's match camera cut to the new set-up at its player's end
    let mut cam = Camera::new();
    cam.turned = g.players[owner].end < 0.0;
    cam.cut();
    let players: Vec<_> = g.players.iter().map(|p| p.pos).collect();
    cam.step(&Scene { players: &players, ball: g.flight.ball.pos });
    let to = cam.view;
    let (from, pivot) = start(g, &to, d, to);
    let prev = if n >= hst_sim::flow::CHANGE_ENDS { from } else { at(&from, &to, pivot, n + 1) };
    Some((prev, at(&from, &to, pivot, n)))
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
        Some(OwnCam { glide: Some(v), .. }) => *v,
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
    use bevy::prelude::*;
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
        // every player a bot
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
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

    /// B486: following the player the shared camera's change-ends glide isn't behind, the picture glides round that
    /// player to its own camera's view at its new end and the serve's set-up carries on from there, with no cut
    /// (it showed the shared glide, the player at the far end, and snapped round when it ended; skipped without the ISO).
    #[test]
    fn the_change_of_ends_glides_behind_the_own_player() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, change_ends::plugin, super::plugin));
        app.add_systems(Startup, setup);
        let time = |mut g: ResMut<Game>| g.match_stats.tick();
        app.add_systems(FixedUpdate, (remember, control, simulate, time, motions, character::tick, played_out, held_ball).chain());
        app.update();
        // no pad drives a player, so the shared camera keeps its end (as online): after the change player 1 is at it and
        // its glide is behind player 1; this machine follows player 0
        let own = super::own_cam(app.world().resource::<Game>(), 0);
        app.insert_resource(own);
        let (mut views, mut after, mut differs) = (Vec::<(u32, hst_sim::camera::View, hst_sim::camera::View)>::new(), None, false);
        for _ in 0..60 * 600 {
            app.world_mut().run_schedule(FixedUpdate);
            let w = app.world();
            let (g, own) = (w.resource::<Game>(), w.resource::<super::OwnCam>());
            let (prev, now) = super::shown(g, Some(own));
            if let Phase::ChangeEnds(n) = g.phase {
                let z = g.players[0].pos[2];
                if n < hst_sim::flow::CHANGE_ENDS {
                    assert!(now.eye[2] * z > 0.0 && now.eye[2].abs() > 11.885, "tick {n}: eye {:?} not behind player 0 at {z}", now.eye);
                }
                differs |= (prev, now) != super::shown(g, None);
                views.push((n, prev, now));
            } else if !views.is_empty() {
                after = Some((prev, now));
                break;
            }
        }
        assert_eq!(views.len() as u32, hst_sim::flow::CHANGE_ENDS + 1, "a whole change-ends phase");
        assert!(differs, "player 0 wasn't at the other end from the shared camera's glide");
        for w in views.windows(2) {
            assert_eq!(w[1].1, w[0].2, "tick {} starts where {} ended", w[1].0, w[0].0);
        }
        let last = views.last().unwrap().2;
        let (prev, now) = after.expect("the serve's first tick");
        let d = |a: &hst_sim::camera::View, b: &hst_sim::camera::View| Vec3::from(a.eye).distance(Vec3::from(b.eye));
        assert!(d(&prev, &last) < 0.05 && d(&now, &last) < 0.05, "the serve's set-up cut from {last:?} to {prev:?} → {now:?}");
    }

    /// B534: a local 2P doubles game against two computers (players 0 and 2, room seats 0 and 1: one team) follows
    /// player 0 with this machine's camera and turns both humans' sticks with it; through two changes of ends, glides
    /// included, the picture stays behind the humans while the shared camera keeps its end. 2P against each other or
    /// one human: no own camera (skipped without the ISO).
    #[test]
    fn local_humans_on_one_team_stay_near_through_changes_of_ends() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        // the players are bots (no one presses anything); who'd be human is asked of the pads below
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: false, chars: vec![6, 1, 2, 3], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, change_ends::plugin, super::plugin));
        app.add_systems(Startup, setup);
        let time = |mut g: ResMut<Game>| g.match_stats.tick();
        app.add_systems(FixedUpdate, (remember, control, simulate, time, motions, character::tick, played_out, held_ball).chain());
        app.update();
        assert!(!app.world().contains_resource::<super::OwnCam>(), "all bots: no own camera");
        let pads = |connected, seats| Pads { slots: default(), connected, seats, autoplay: false };
        let g = app.world().resource::<Game>();
        assert_eq!(super::local(g, &pads(2, None)), Some((0, vec![0, 1, 0, 3])), "2P: players 0 and 2, one team");
        assert_eq!(super::local(g, &pads(1, None)), None, "one human: the shared camera turns");
        assert_eq!(super::local(g, &pads(4, None)), None, "humans on both teams");
        assert_eq!(super::local(g, &pads(0, Some([true, false, true, false]))), None, "1P and 3P: players 0 and 1, both teams");
        // the match's link makes the camera (the pads then go back to bots)
        app.world_mut().insert_resource(pads(2, None));
        bevy::ecs::system::RunSystemOnce::run_system_once(app.world_mut(), super::link).unwrap();
        app.world_mut().insert_resource(Pads::default());
        assert_eq!(app.world().resource::<super::OwnCam>().owner, 0, "following player 0");
        assert_eq!(app.world().resource::<Game>().cam_of, [0, 1, 0, 3]);
        let (mut changes, mut gliding, mut far, mut ticks) = (0, false, false, 0);
        for _ in 0..60 * 1200 {
            app.world_mut().run_schedule(FixedUpdate);
            let w = app.world();
            let (g, own) = (w.resource::<Game>(), w.resource::<super::OwnCam>());
            let showing = w.resource::<cutaway::Director>().showing();
            let (_, now) = super::shown(g, Some(own));
            let z = g.players[0].pos[2];
            assert_eq!(super::flipped(g, 0), super::flipped(g, 2), "the partners' sticks turn together");
            match g.phase {
                Phase::ChangeEnds(n) => {
                    gliding = true;
                    if n < hst_sim::flow::CHANGE_ENDS {
                        assert!(now.eye[2] * z > 0.0 && now.eye[2].abs() > 11.885, "change {changes} tick {n}: eye {:?} not behind player 0 at {z}", now.eye);
                    }
                }
                _ if gliding => {
                    gliding = false;
                    changes += 1;
                    if changes == 2 {
                        break;
                    }
                }
                _ if own.plain && !showing && g.instant.ticks > 0 => {
                    ticks += 1;
                    assert!(now.eye[2] * z > 0.0, "after {changes} changes: eye {:?}, player 0 at {z}", now.eye);
                    // the humans' sticks turn with the picture (the eye at +z: turned)
                    assert_eq!(super::flipped(g, 0), now.eye[2] > 0.0, "player 0's stick, eye {:?}", now.eye);
                    far |= g.cam.view.eye[2] * z < 0.0;
                }
                _ => {}
            }
        }
        assert_eq!(changes, 2, "two changes of ends");
        assert!(ticks > 0 && far, "the shared camera never had the humans at the far end ({ticks} plain ticks)");
    }

    /// B376: deciding whether the picture shows this machine's camera waits for every system that moves the shared
    /// one off the plain view; unordered, the executor could run it first and hide the cut-aways online.
    #[test]
    fn the_own_camera_waits_for_the_cutaway_and_replay_cameras() {
        let mut app = App::new();
        app.add_plugins((super::super::cutaway::plugin, super::super::change_ends::plugin, super::plugin));
        app.world_mut().schedule_scope(FixedUpdate, |w, s| s.initialize(w).unwrap());
        let s = app.get_schedule(FixedUpdate).unwrap();
        let names: std::collections::HashMap<_, _> = s.systems().unwrap().map(|(k, sys)| (k, sys.name().to_string())).collect();
        let pairs = s.graph().conflicting_systems().iter().map(|(a, b, _)| format!("{} / {}", names[a], names[b]));
        let bad: Vec<_> = pairs.filter(|n| n.contains("own_camera::plain")).collect();
        assert!(bad.is_empty(), "unordered with the own camera's check: {bad:?}");
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
