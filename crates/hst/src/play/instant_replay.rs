//! The instant replay's decision at each decided point (`hst_sim::instant::decide`), fed from the port's state.
//!
//! The run (`run`, the serve phase's entry, message 0xe): each serve set-up keeps the whole match state
//! (`snapshot::save`; the original's copies of its match globals, rally block and RNG seed) and the tick's pads from
//! then on. At the next serve set-up with a replay pending it puts that state back and plays the logged pads again:
//! the point, its post-point phase and the serve set-up replay as they went, the scoreboard awards the point onto the
//! score as it was before it (so it isn't scored twice) and every generator draws what it drew, so the match goes on
//! exactly as without the replay, only later. Its pacing is `replay_pace`; its cameras (P0b4d4) and the buttons that skip or ask
//! for one (P0b4d5) come later.

use super::{Game, snapshot, tick_input};
use bevy::prelude::*;
use hst_sim::instant::{self, Decision, Point};
use hst_sim::score::Event;
use std::collections::VecDeque;

/// The decision's own state across a point.
#[derive(Clone, Debug, Default)]
pub(super) struct Instant {
    /// Match-handler ticks since the serve was set up (gm+0x36c).
    pub ticks: i32,
    /// Per player: the last swing's timing offset (frames off perfect, signed), as the shot recorder keeps it.
    pub timing: [i32; 4],
    /// The ball mark of the point's last first bounce on the court: the landing spot and its tail, the spot pushed
    /// along the ball's ground speed × 0.8 (x, z). The line-call test reads these two.
    pub mark: [[f32; 2]; 2],
    /// The last point's decision (the recorder's flags carry into the next point's set-up).
    pub decision: Decision,
    /// The serve was set up this tick (`run` takes it).
    pub set_up: bool,
    /// A replay is playing (gm+0x344): its point makes no decision.
    pub replaying: bool,
    /// Replays shown this match (gm+0x358), and those a button asked for (gm+0x354, P0b4d5).
    pub replays: i32,
    pub asked: i32,
}

/// The run's state outside `Game`: the match as the last serve set-up left it and the pads since.
#[derive(Resource, Default)]
pub(super) struct Recorder {
    serve: Option<snapshot::Snapshot>,
    pads: Vec<[hst_sim::replay::Pad; 4]>,
    /// Pads an outside replay source still had queued when ours took over (given back after ours).
    outside: Option<VecDeque<[hst_sim::replay::Pad; 4]>>,
    /// The match as the serve set-up that started the last replay left it (the test compares the replay's end).
    #[cfg(test)]
    back: Option<Game>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Recorder>().add_systems(FixedUpdate, run.after(super::play_sounds));
}

/// End of each match tick: log the pads; at a serve set-up keep the state, or with a replay pending put the point's
/// serve back and queue its pads.
pub(super) fn run(w: &mut World) {
    let Some(g) = w.get_resource::<Game>() else { return };
    let (set_up, pending) = (g.instant.set_up, g.instant.decision.replay && !g.instant.replaying);
    let now = w.get_resource::<tick_input::TickPads>().map_or([tick_input::IDLE; 4], |t| t.now());
    w.resource_mut::<Game>().instant.set_up = false;
    w.init_resource::<Recorder>();
    let first = w.resource::<Recorder>().serve.is_none();
    if pending && set_up && !first {
        start(w);
    } else if set_up || first {
        // the match's first tick has no set-up through `next_point`: its serve is kept from there
        w.resource_mut::<Game>().instant.replaying = false;
        end_queue(w);
        let s = snapshot::save(w);
        let mut r = w.resource_mut::<Recorder>();
        (r.serve, r.pads) = (Some(s), vec![]);
    } else {
        w.resource_mut::<Recorder>().pads.push(now);
    }
}

/// The serve phase's entry with a replay pending: back to the point's serve, counted, its pads queued.
fn start(w: &mut World) {
    let g = w.resource::<Game>();
    let (replays, asked) = (g.instant.replays + 1, g.instant.asked);
    #[cfg(test)]
    let back = g.clone();
    let mut r = w.resource_mut::<Recorder>();
    #[cfg(test)]
    {
        r.back = Some(back);
    }
    let s = r.serve.take().expect("kept at the serve");
    let queue: VecDeque<_> = std::mem::take(&mut r.pads).into();
    snapshot::restore(w, &s);
    w.resource_mut::<Recorder>().serve = Some(s);
    let i = &mut w.resource_mut::<Game>().instant;
    (i.replays, i.asked, i.replaying) = (replays, asked, true);
    bevy::log::info!("instant replay {replays}: {} ticks", queue.len());
    // an outside source (a recording, later the network) goes on after the replay
    let outside = w.remove_resource::<tick_input::Replay>().map(|o| o.0);
    w.resource_mut::<Recorder>().outside = Some(outside.unwrap_or_default());
    w.insert_resource(tick_input::Replay(queue));
}

/// The replay's own pads are spent at its serve set-up: an outside source's queue comes back, else the devices.
fn end_queue(w: &mut World) {
    let Some(outside) = w.resource_mut::<Recorder>().outside.take() else { return };
    w.remove_resource::<tick_input::Replay>();
    if !outside.is_empty() {
        w.insert_resource(tick_input::Replay(outside));
    }
}

/// The serve's set-up (message 0xe without a replay pending): the match block from +0x340 is cleared (allowed,
/// cancelled, the tick count), so are `recorded` and the ball mark.
pub(super) fn new_point(g: &mut Game) {
    let i = &mut g.instant;
    (i.ticks, i.mark) = (0, [[0.0; 2]; 2]);
    i.decision.allowed = false;
    i.decision.recorded = false;
    i.set_up = true;
}

/// A first bounce on the court during the rally leaves the ball mark the decision reads.
pub(super) fn bounce(g: &mut Game, at: [f32; 3], vel: [f32; 3]) {
    // ponytail: the game's unit(v_xz)·|v_xz|·0.8 in host floats, not ps2 ops (presentation only)
    g.instant.mark = [[at[0], at[2]], [at[0] + vel[0] * 0.8, at[2] + vel[2] * 0.8]];
}

/// The point message, after the scoreboard awarded the point to `team` with `event`, on the judge's `call`.
pub(super) fn point(g: &mut Game, team: usize, event: Option<Event>, call: u8) {
    let hitter = usize::try_from(g.last_hitter).ok();
    let p = Point {
        players: g.rules.players as i32,
        awarded: event.is_some(),
        replaying: g.instant.replaying,
        allowed: g.instant.decision.allowed,
        // ponytail: the rally's cancel (over 1799 ticks / the long-rally option) is P0b4d5's
        cancelled: false,
        recorded: g.instant.decision.recorded,
        // ponytail: no shot recorder yet (P0b4d2); it holds every shot of the rally
        record_shots: g.shots,
        winner: hitter.filter(|&h| h & 1 == team),
        hitter,
        receiver: g.score.receiver as usize,
        rally: g.rally.call as u8,
        shots: g.shots,
        faults: g.rally.faults,
        ball_vel: g.hit_flight.ball.vel,
        ball_kind: g.shot.class,
        ball_sub: g.shot.kind,
        ball_state: g.flight.call as u8,
        ball_bounces: g.flight.contacts,
        out_side: g.body_hit.is_some(),
        bounces: g.instant.mark,
        verdict: call,
        demo: false,
        match_won: g.score.match_over,
        // ponytail: the original's option byte (1 in every save state); the app has no option for it yet
        option: 1,
        controller: if hitter.is_some_and(|h| g.humans.get(h).copied().unwrap_or(false)) { 0 } else { 0x20 },
        timing: hitter.map_or(0, |h| g.instant.timing[h]),
        event: match event {
            Some(Event::Game) => 1,
            Some(Event::Set) => 2,
            _ => 0,
        },
        ticks: g.instant.ticks,
    };
    let d = instant::decide(&p);
    if d.replay {
        bevy::log::info!("instant replay: {} shots, class {} kind {}, event {}, {:.0} km/h", p.shots, p.ball_kind, p.ball_sub, p.event, instant::speed_kmh(p.ball_vel));
    }
    g.instant.decision = d;
}

/// A bot-only singles match through the app, headless: the original replays a bot match's point only when it wins
/// the match (and the recorder holds its shots). Needs the ISO at the repository root (skipped without).
#[cfg(test)]
pub(super) mod tests {
    use super::super::*;
    use super::{Decision, Recorder};
    use crate::{Args, GameSpace};

    pub(in crate::play) const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    pub(in crate::play) fn app(human: bool) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 2, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: true, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.init_resource::<Recorder>();
        app.add_systems(FixedUpdate, (remember, tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball, super::run).chain());
        app.update();
        // ponytail: the bot plays 1P; flagging it human only opens the decision's human branches
        app.world_mut().resource_mut::<Game>().humans[0] = human;
        app
    }

    /// Every decided point's (shots, match won, decision) of a bot match, 1P flagged human when `human`.
    fn points(human: bool) -> Vec<(i32, bool, Decision)> {
        let mut app = app(human);
        let (mut out, mut was_post) = (vec![], false);
        for _ in 0..60 * 60 * 30 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            let post = g.phase == Phase::Post;
            if post && !was_post && g.post.as_ref().is_some_and(|p| p.event.is_some()) {
                out.push((g.shots, g.score.match_over, g.instant.decision));
                if g.score.match_over {
                    break;
                }
            }
            was_post = post;
        }
        out
    }

    #[test]
    fn a_bot_match_replays_its_match_point_only() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        for human in [false, true] {
            let pts = points(human);
            for (i, (shots, won, d)) in pts.iter().enumerate() {
                eprintln!("human {human} point {i}: {shots} shots, match won {won}, {d:?}");
            }
            let (_, won, last) = *pts.last().expect("no point decided");
            assert!(won && last.replay, "the match-ending point didn't replay");
            if !human {
                assert_eq!(pts.iter().filter(|p| p.2.replay).count(), 1, "a bot point other than match point replayed");
            }
        }
    }

    /// The match-ending point replays at the next serve set-up: back at its serve with the score before it, played
    /// again from its pads, it ends at the same serve set-up with the whole match state as it was without the replay
    /// (score awarded once, every generator where it was), counted once.
    #[test]
    fn the_replay_ends_where_the_point_left_the_match() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = app(false);
        let (mut seen, mut over_seen) = (false, false);
        for _ in 0..60 * 60 * 30 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            if g.instant.replaying && !seen {
                seen = true;
                let back = app.world().resource::<Recorder>().back.as_ref().unwrap();
                assert_eq!(g.instant.replays, 1);
                // back at the point's serve: the score before the match ended
                assert!(back.score.games != g.score.games || back.score.sets != g.score.sets, "the score wasn't put back");
            }
            over_seen |= seen && g.score.match_over;
            if seen && !g.instant.replaying {
                assert!(over_seen, "the replay didn't decide the match again");
                let mut back = app.world().resource::<Recorder>().back.clone().unwrap();
                back.instant = g.instant.clone();
                assert_eq!(format!("{back:?}"), format!("{g:?}"), "the replay left the match elsewhere");
                assert_eq!(g.instant.replays, 1);
                return;
            }
        }
        panic!("no replay ran (seen {seen})");
    }
}
