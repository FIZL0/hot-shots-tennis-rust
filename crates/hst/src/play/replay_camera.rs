//! The instant replay's cameras (`hst_sim::cutaway::ReplayPicks`): while a replay runs, the match camera gives way
//! to scripted shots picked from the point as it was played. Each live point keeps its shots (tick, hitter, the
//! ball's class, a recorder kind); the replay starts on a tracking shot, cuts to a shot of the serve 10 ticks
//! before it, back to tracking 10 ticks after, to a shot of each marked shot (the point's winning shot) from the
//! hitter's swing, and once the last shot's window is over (or the point is all but over) to a closing shot that
//! holds until the replay ends. No cut-away runs during a replay.

use bevy::prelude::*;
use hst_sim::cutaway::{self, CameraShot, IDENTITY, Look, M4, ReplayCameras, ReplayPicks, Shot};

use super::{Figure, Game, Phase, Player};
use crate::Args;
use crate::character::Motion;

/// A shot of the point, as the game's shot recorder keeps it.
#[derive(Clone, Copy, Debug)]
struct Rec {
    tick: i32,
    hitter: usize,
    /// The ball's class at the hit (6 for a dive, 4 for the △ smash).
    ty: u8,
    /// 0 a plain shot; 3 (6 after a smash) the point's winning shot.
    kind: u8,
    /// Ticks from the hitter's swing to the hit.
    lead: i32,
}

#[derive(Resource, Clone)]
pub(super) struct ReplayCamera {
    shots: Vec<CameraShot>,
    picks: ReplayPicks,
    /// The last point played (its shots and the tick it ended), and the one a replay shows.
    live: Vec<Rec>,
    live_end: i32,
    recs: Vec<Rec>,
    end: i32,
    swing_from: [i32; 4],
    swinging: [bool; 4],
    post: bool,
    /// Hits seen this point, and the last hitter and the one before (the server and receiver before the serve).
    hits: i32,
    roles: [usize; 2],
    /// Replays this match, from −1.
    replay: i32,
    on: bool,
    /// The next shot to come (the recorder's index), its pick tick, the window's end, the closing shot is on.
    next: usize,
    pick_at: i32,
    window: i32,
    listed: bool,
    closed: bool,
    shot: Option<Shot>,
}

impl ReplayCamera {
    pub(super) fn new(shots: Vec<CameraShot>, lists: ReplayCameras) -> Self {
        ReplayCamera {
            shots,
            picks: ReplayPicks::new(lists),
            live: vec![],
            live_end: 0,
            recs: vec![],
            end: 0,
            swing_from: [0; 4],
            swinging: [false; 4],
            post: false,
            hits: 0,
            roles: [0, 1],
            replay: -1,
            on: false,
            next: 0,
            pick_at: 0,
            window: 0,
            listed: false,
            closed: false,
            shot: None,
        }
    }

    /// A replayed close line call starts the fast-forwarded replay's first shot over.
    pub(super) fn reset_cut(&mut self) {
        self.picks.serve = 0;
    }

    fn start(&mut self, n: u8, listed: bool) {
        info!("replay camera {n:#x}");
        self.shot = Some(Shot::new(&self.shots, n, false));
        self.listed = listed;
    }
}

/// Upright at the player's feet, facing the way they face.
fn body(p: &Player) -> M4 {
    let [fx, _, fz, _] = p.body.face.dir;
    [[fz, 0.0, -fx, 0.0], [0.0, 1.0, 0.0, 0.0], [fx, 0.0, fz, 0.0], [p.pos[0], p.pos[1], p.pos[2], 1.0]]
}

pub(super) fn step(mut g: ResMut<Game>, mut rc: ResMut<ReplayCamera>, args: Res<Args>, q: Query<(&Figure, &Motion)>) {
    let rc = &mut *rc;
    let t = g.instant.ticks;
    for (i, p) in g.players.iter().enumerate() {
        let s = p.swing.is_some();
        if s && !rc.swinging[i] {
            rc.swing_from[i] = t;
        }
        rc.swinging[i] = s;
    }
    if g.shots < rc.hits {
        // a new point (or the replay back at its serve)
        rc.hits = 0;
        rc.roles = [g.score.server as usize, g.score.receiver as usize];
    }
    if g.shots > rc.hits {
        rc.hits = g.shots;
        rc.roles = [g.last_hitter as usize, if g.shots == 1 { rc.roles[1] } else { rc.roles[0] }];
        if !g.instant.replaying {
            if g.shots == 1 {
                rc.live.clear();
            }
            let h = g.last_hitter as usize;
            let ty = match (g.shot.class, g.shot.kind) {
                _ if g.players[h].dive.is_some() => 6,
                (3, 1) => 4,
                (c, _) => c,
            };
            let lead = if g.shots == 1 { 0 } else { t - rc.swing_from[h] };
            // ponytail: a human's strong shots are kind 1 in the game (from a character stat and the ball's speed);
            // every shot is plain here, as every bot shot is
            rc.live.push(Rec { tick: t, hitter: h, ty, kind: 0, lead });
        }
    }
    let post = g.phase == Phase::Post;
    if post && !rc.post && !g.instant.replaying {
        rc.live_end = t;
        // the point's winning shot (not an ace) is marked
        let won = |r: &Rec| r.hitter as i32 & 1 == g.post_winner & 1;
        if let [_, .., last] = rc.live.as_mut_slice()
            && won(last)
        {
            last.kind = if last.ty == 3 { 6 } else { 3 };
        }
    }
    rc.post = post;

    // ponytail: the game's closing shot holds into its match-over phase; the port starts the next match instead
    // (`Next::MatchOver`), so the match camera takes over where the replay ends
    if !g.instant.replaying {
        if rc.on {
            rc.on = false;
            rc.shot = None;
            g.prev_view = g.cam.view; // a cut doesn't blend
        }
        return;
    }
    let look = |i: usize| Look { costume: args.outfits.get(i).copied().unwrap_or(0) as i32, character: g.chars[i] };
    if !rc.on {
        rc.on = true;
        rc.replay += 1;
        // the last point played is the one replayed
        (rc.recs, rc.end) = (rc.live.clone(), rc.live_end);
        (rc.next, rc.window, rc.closed) = (0, 0, false);
        if g.instant.decision.reset_cut {
            rc.reset_cut();
        }
        let n = rc.picks.tracking(rc.replay, g.players.len());
        rc.start(n, false);
        rc.pick_at = rc.recs.first().map_or(0, |r| r.tick + rc.picks.lists.windows[0].0 as i32);
    }
    if g.instant.replaying {
        // the hit just replayed: the next recorded shot's pick is set from its kind and the hitter's swing
        if rc.next < rc.hits as usize {
            rc.next = rc.hits as usize;
            rc.pick_at = match rc.recs.get(rc.next) {
                Some(r) if r.kind != 0 => {
                    if r.lead != 0 && matches!(r.kind, 1 | 2 | 3 | 6) {
                        r.tick - r.lead
                    } else {
                        r.tick + rc.picks.lists.windows[r.kind as usize].0 as i32
                    }
                }
                Some(_) => rc.pick_at,
                None => 0,
            };
        }
        if let Some(r) = rc.recs.get(rc.next).copied()
            && rc.pick_at != 0
            && t == rc.pick_at
        {
            rc.window = r.tick + rc.picks.lists.windows[r.kind as usize].1 as i32;
            let tracking_fixed = rc.shot.as_ref().is_some_and(|s| (0xe..=0x10).contains(&s.number));
            if (r.ty == 0 || r.kind != 0) && (!tracking_fixed || matches!(r.kind, 3 | 6)) {
                let n = rc.picks.shot(r.ty, look(r.hitter));
                rc.start(n, true);
            }
        }
        if rc.listed && !rc.closed {
            let near_end = rc.end - 4;
            let last = rc.next >= rc.recs.len();
            if last && (t == rc.window || (near_end < rc.window && near_end < t)) {
                rc.closed = true;
                let [a, b] = rc.roles;
                let n = rc.picks.end(look(a), look(b));
                rc.start(n, true);
            } else if t == rc.window {
                let n = rc.picks.tracking(rc.replay, g.players.len());
                rc.start(n, false);
            }
        }
    }
    show(&mut g, rc, &q);
}

/// Steps the shot and shows it, the reference frames from the players (the last hitter and the one before) and the
/// ball as they are now.
fn show(g: &mut Game, rc: &mut ReplayCamera, q: &Query<(&Figure, &Motion)>) {
    let Some(shot) = rc.shot.as_mut() else { return };
    let bones = |i: usize| {
        let p = &g.players[i];
        let pos = [p.pos[0], p.pos[1], p.pos[2], 1.0];
        let (_, m) = q.iter().find(|(f, _)| f.0 == i)?;
        let b = |n| super::cutaway::bone(p, &g.data[i], m.id, m.clock.sampled, pos, n);
        Some((b("Bip01Head")?, b("Bip01Spine2")?))
    };
    let [a, b] = rc.roles;
    let (ha, hb) = (bones(a), bones(b));
    let head = |i: usize, h: &Option<(M4, M4)>| h.map_or(body(&g.players[i]), |h| h.0);
    let head_frame = |i: usize, h: &Option<(M4, M4)>| cutaway::head_frame(&head(i, h), g.players[i].hand < 0.0);
    let chest = |i: usize, h: &Option<(M4, M4)>| h.map_or(body(&g.players[i]), |h| cutaway::subject_frames(&h.0, &h.1)[3]);
    let ball = g.flight.ball.pos;
    let ball = [IDENTITY[0], IDENTITY[1], IDENTITY[2], [ball[0], ball[1], ball[2], 1.0]];
    // ponytail: the game's frame table (role bodies 19–22, head frames 35–38, head bones 51–54, the recorded shot's
    // frames, court spots) is read off its users by eye: frames it fills from the shot recorder stand in as the
    // hitter's chest and the ball
    let frames: Vec<(u8, M4)> = vec![
        (17, body(&g.players[a])),
        (19, body(&g.players[a])),
        (21, body(&g.players[a])),
        (57, body(&g.players[a])),
        (113, body(&g.players[a])),
        (20, body(&g.players[b])),
        (22, body(&g.players[b])),
        (114, body(&g.players[b])),
        (35, head_frame(a, &ha)),
        (37, head_frame(a, &ha)),
        (36, head_frame(b, &hb)),
        (38, head_frame(b, &hb)),
        (51, head(a, &ha)),
        (53, head(a, &ha)),
        (52, head(b, &hb)),
        (54, head(b, &hb)),
        (56, chest(b, &hb)),
        (88, chest(b, &hb)),
        (11, ball),
        (12, ball),
        (112, ball),
        (120, ball),
    ];
    let first = shot.fov == 0.0;
    shot.step(|i| frames.iter().find(|f| f.0 == i).map_or(IDENTITY, |f| f.1));
    g.cam.view = shot.view();
    if first {
        g.prev_view = g.cam.view; // a cut doesn't blend
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::ReplayCamera;
    use crate::{Args, GameSpace};

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// The bot match's match-ending point replays on the tracking shot, the serve's shot, the winning shot's and a
    /// closing shot that holds to its end, with no cut-away during or after it.
    #[test]
    fn the_match_point_replay_cuts_and_holds_its_closing_shot() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 2, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: true, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.init_resource::<instant_replay::Recorder>();
        app.add_plugins(cutaway::plugin);
        app.add_systems(FixedUpdate, (remember, tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball, instant_replay::run).chain());
        app.update();
        let (mut cams, mut after, mut over) = (vec![], 0, false);
        for _ in 0..60 * 60 * 30 {
            app.world_mut().run_schedule(FixedUpdate);
            let (g, rc) = (app.world().resource::<Game>(), app.world().resource::<ReplayCamera>());
            let n = rc.shot.as_ref().map(|s| s.number);
            if g.instant.replaying {
                assert!(!app.world().resource::<cutaway::Director>().showing(), "a cut-away during the replay");
                over |= g.score.match_over;
                if cams.last() != Some(&n) {
                    cams.push(n);
                }
            } else if !cams.is_empty() {
                assert!(over, "the replayed point didn't end the match");
                assert_eq!(n, None, "the replay camera outlived the replay");
                assert!(!app.world().resource::<cutaway::Director>().showing(), "a cut-away after the replay");
                after += 1;
                if after == 120 {
                    break;
                }
            }
        }
        eprintln!("replay cameras {:x?}", cams);
        let lists = &app.world().resource::<ReplayCamera>().picks.lists;
        assert_eq!(after, 120, "no replay ran to match over");
        assert_eq!(cams[0], Some(lists.tracking[0][0]), "didn't start on the first tracking shot");
        assert!(cams.contains(&Some(0x1d)), "no serve shot");
        assert!(lists.end.iter().flatten().any(|&e| Some(e) == *cams.last().unwrap()), "didn't close on an end shot");
    }
}
