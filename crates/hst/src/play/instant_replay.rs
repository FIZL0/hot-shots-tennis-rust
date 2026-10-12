//! The instant replay's decision at each decided point (`hst_sim::instant::decide`), fed from the port's state.
//! The replay run itself (P0b4d2), its pacing (P0b4d3), cameras (P0b4d4) and the buttons that cancel or ask for
//! one (P0b4d5) come later; this keeps the decision for them.

use super::Game;
use hst_sim::instant::{self, Decision, Point};
use hst_sim::score::Event;

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
}

/// The serve's set-up (message 0xe without a replay pending): the match block from +0x340 is cleared (allowed,
/// cancelled, the tick count), so are `recorded` and the ball mark.
pub(super) fn new_point(g: &mut Game) {
    let i = &mut g.instant;
    (i.ticks, i.mark) = (0, [[0.0; 2]; 2]);
    i.decision.allowed = false;
    i.decision.recorded = false;
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
        replaying: false,
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
mod tests {
    use super::super::*;
    use super::Decision;
    use crate::{Args, GameSpace};

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// Every decided point's (shots, match won, decision) of a bot match, 1P flagged human when `human`.
    fn points(human: bool) -> Vec<(i32, bool, Decision)> {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 2, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: true, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        // ponytail: the bot plays 1P; flagging it human only opens the decision's human branches
        app.world_mut().resource_mut::<Game>().humans[0] = human;
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
}
