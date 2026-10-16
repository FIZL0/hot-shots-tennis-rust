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
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
struct Rec {
    tick: i32,
    hitter: usize,
    /// The ball's class at the hit (6 for a dive, 4 for the △ smash).
    ty: u8,
    /// 0 a plain shot; 3 (6 after a smash) the point's winning shot.
    kind: u8,
    /// Ticks from the hitter's swing to the hit.
    lead: i32,
    /// The ball at the hit, and the hitter's [`At`] frames.
    pos: [f32; 4],
    at: At,
}

/// What the recorder keeps of its subject for the replay's frames: the facing (z) and the place in the subject's
/// body frame of their Spine2 (on the ground), head frame, head bone, Spine2 and Pelvis.
type At = [[[f32; 4]; 2]; 5];

/// Taken at each shot's start: the last hitter and the one before, the subject of the coming record (the player
/// who'll hit it; past the last shot the one who didn't hit last) and its frames, the next record's hitter, and the
/// view from the coming hit's spot along the ball (frame 17).
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct Snap {
    roles: [usize; 2],
    subject: usize,
    at: Option<At>,
    next: usize,
    spot: M4,
}

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct ReplayCamera {
    shots: Vec<CameraShot>,
    pub(super) picks: ReplayPicks,
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
    /// The point's end record (its subject and frames), live and replayed.
    live_tail: Option<(usize, At)>,
    tail: Option<(usize, At)>,
    /// The live point's last bounce (frame 11) and the bounces counted so far.
    bounce: M4,
    bounces: i32,
    /// From the coming hit's spot along the ball (frame 14), and the pick's snapshot.
    spot: M4,
    snap: Option<Snap>,
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
            live_tail: None,
            tail: None,
            bounce: IDENTITY,
            bounces: 0,
            spot: IDENTITY,
            snap: None,
        }
    }

    /// A replayed close line call starts the fast-forwarded replay's first shot over.
    pub(super) fn reset_cut(&mut self) {
        self.picks.serve = 0;
    }

    /// A close-up shot (modes 0x56..0x59) is on: replayed footfalls sound (`sound::step`).
    pub(super) fn close_up(&self) -> bool {
        self.on && self.shot.as_ref().is_some_and(|s| (0x56..=0x59).contains(&s.number))
    }

    fn start(&mut self, n: u8, listed: bool) {
        info!("replay camera {n:#x}");
        self.shot = Some(Shot::new(&self.shots, n, false));
        self.listed = listed;
        self.snap = None; // taken by `show`
    }

    /// The coming record's subject and frames (the game's rule for the end record: the one before the last
    /// hitter, or the next player when they're the same).
    fn coming(&self, i: usize) -> Option<(usize, Option<At>)> {
        match self.recs.get(i) {
            Some(r) => Some((r.hitter, Some(r.at))),
            None => self.tail.map(|(s, at)| (s, Some(at))),
        }
    }
}

/// Upright on the ground under the player, facing the way they face.
fn body(p: &Player) -> M4 {
    let [fx, _, fz, _] = p.body.face.dir;
    [[fz, 0.0, -fx, 0.0], [0.0, 1.0, 0.0, 0.0], [fx, 0.0, fz, 0.0], [p.pos[0], 0.0, p.pos[2], 1.0]]
}

/// Player `i`'s live frames as the game keeps them: body, head frame, head bone (turned like the head frame, without
/// its lift), Spine2 and Pelvis (turned the same way). Without a posed skeleton all are the body.
fn live(g: &Game, q: &Query<(&Figure, &Motion)>, i: usize) -> [M4; 5] {
    let p = &g.players[i];
    let b = body(p);
    let pos = [p.pos[0], p.pos[1], p.pos[2], 1.0];
    let bones = q.iter().find(|(f, _)| f.0 == i).and_then(|(_, m)| {
        let bone = |n| super::cutaway::bone(p, &g.data[i], m.id, m.clock.sampled, pos, n);
        Some((bone("Bip01Head")?, bone("Bip01Spine2")?, bone("Bip01Pelvis")?))
    });
    let Some((h, s, pv)) = bones else { return [b; 5] };
    let head = cutaway::head_frame(&h, p.hand < 0.0);
    let mut bone = head;
    bone[3][1] += 0.2;
    let turn = |m: M4| [m[1], m[0], m[2].map(|v| -v), m[3]];
    [b, head, bone, turn(s), turn(pv)]
}

/// The recorder's [`At`] of player `i` now.
fn at(g: &Game, q: &Query<(&Figure, &Motion)>, i: usize) -> At {
    let [b, head, bone, spine, pelvis] = live(g, q, i);
    let ground = [spine[3][0], 0.0, spine[3][2], 1.0];
    let local = |w: [f32; 4]| {
        let d = [w[0] - b[3][0], w[1] - b[3][1], w[2] - b[3][2]];
        let c = |k: usize| d[0] * b[k][0] + d[1] * b[k][1] + d[2] * b[k][2];
        [c(0), c(1), c(2), 1.0]
    };
    [(spine[2], ground), (head[2], head[3]), (bone[2], bone[3]), (spine[2], spine[3]), (pelvis[2], pelvis[3])]
        .map(|(z, w)| [z, local(w)])
}

/// The recorded frames rebuilt on the subject's live body: upright along each recorded facing.
fn rebuilt(at: &At, b: &M4) -> [M4; 5] {
    at.map(|[z, l]| cutaway::facing(z, std::array::from_fn(|i| b[3][i] + l[0] * b[0][i] + l[1] * b[1][i] + l[2] * b[2][i])))
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
            let b = g.flight.ball.pos;
            rc.live.push(Rec { tick: t, hitter: h, ty, kind: 0, lead, pos: [b[0], b[1], b[2], 1.0], at: at(&g, &q, h) });
        }
    }
    if !g.instant.replaying {
        // the point's last bounce, turned to face from the last hitter's side to the other
        if g.flight.bounces < rc.bounces {
            rc.bounces = 0;
        }
        if g.flight.bounces > rc.bounces {
            rc.bounces = g.flight.bounces;
            let (v, l) = (g.flight.ball.vel, g.flight.landing.0);
            let m = cutaway::facing([v[0], 0.0, v[2], 0.0], [l[0], 0.0, l[2], 1.0]);
            let [a, b] = rc.roles.map(|i| g.players[i].pos);
            let away = m[2][0] * (b[0] - a[0]) + m[2][2] * (b[2] - a[2]) < 0.0;
            rc.bounce = if away { [m[0].map(|v| -v), m[1], m[2].map(|v| -v), m[3]] } else { m };
        }
    }
    let post = g.phase == Phase::Post;
    if post && !rc.post && !g.instant.replaying {
        rc.live_end = t;
        let n = g.players.len();
        rc.live_tail = match rc.live.as_slice() {
            [.., x, y] if x.hitter != y.hitter => Some(x.hitter),
            [.., y] => Some((y.hitter + 1) % n),
            [] => None,
        }
        .map(|s| (s, at(&g, &q, s)));
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
        (rc.recs, rc.end, rc.tail) = (rc.live.clone(), rc.live_end, rc.live_tail);
        rc.spot = rc.recs.first().map_or(IDENTITY, |r| cutaway::facing([0.0; 4], r.pos));
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
            if let Some(r) = rc.recs.get(rc.next) {
                let v = g.flight.ball.vel;
                rc.spot = cutaway::facing([v[0], v[1], v[2], 0.0], r.pos);
            }
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

/// Steps the shot and shows it, on the game's reference frames: the last hitter and the one before (live and as
/// at the shot's start), the coming record's subject (its frames as recorded at the hit, carried on their live body)
/// and the next hitter, the ball, the point's last bounce and court spots.
fn show(g: &mut Game, rc: &mut ReplayCamera, q: &Query<(&Figure, &Motion)>) {
    if rc.shot.is_none() {
        return;
    }
    let snap = rc.snap.unwrap_or_else(|| {
        let (subject, at) = rc.coming(rc.next).unwrap_or((rc.roles[0], None));
        let next = rc.recs.get(rc.next + 1).map_or(subject, |r| r.hitter);
        Snap { roles: rc.roles, subject, at, next, spot: rc.spot }
    });
    rc.snap = Some(snap);
    let [a, b] = rc.roles.map(|i| live(g, q, i));
    let [sa, sb] = snap.roles.map(|i| live(g, q, i));
    let (subject, next) = (live(g, q, snap.subject), live(g, q, snap.next));
    let (cur, cur_at) = rc.coming(rc.next).unwrap_or((snap.subject, None));
    let current = cur_at.map(|at| rebuilt(&at, &live(g, q, cur)[0]));
    let recorded = snap.at.map(|at| rebuilt(&at, &subject[0]));
    let v = g.flight.ball.vel;
    let p = g.flight.ball.pos;
    let ball = cutaway::facing([v[0], v[1], v[2], 0.0], [p[0], p[1], p[2], 1.0]);
    let side = if a[0][3][2] < 0.0 { -6.4 } else { 6.4 };
    let frame = |i: u8| -> M4 {
        let row = |f: [M4; 5]| f[(i as usize - 19) / 16];
        match i {
            // the far court half's spot, turned round (frame 4 mirrored)
            5 => [[-1.0, 0.0, 0.0, 0.0], IDENTITY[1], [0.0, 0.0, -1.0, 0.0], [0.0, 0.0, side, 1.0]],
            // the net's top at the court centre
            7 => [IDENTITY[0], IDENTITY[1], IDENTITY[2], [0.0, -0.91, 0.0, 1.0]],
            8 | 9 | 10 | 12 => ball,
            11 => rc.bounce,
            14 => rc.spot,
            17 => snap.spot,
            19 | 35 | 51 | 67 | 83 => row(a),
            20 | 36 | 52 | 68 | 84 => row(b),
            21 | 37 | 53 | 69 | 85 => row(sa),
            22 | 38 | 54 | 70 | 86 => row(sb),
            23 | 39 | 55 | 71 | 87 => current.map_or(IDENTITY, |f| f[(i as usize - 23) / 16]),
            24 | 40 | 56 | 72 | 88 => recorded.map_or(IDENTITY, |f| f[(i as usize - 24) / 16]),
            25 | 41 | 57 | 73 | 89 => subject[(i as usize - 25) / 16],
            26 | 42 | 58 | 74 | 90 => next[(i as usize - 26) / 16],
            // the court centre: 1, 113, 114 and 120 (the original's replays leave them at identity) and the rest
            _ => IDENTITY,
        }
    };
    let shot = rc.shot.as_mut().unwrap();
    let first = shot.fov == 0.0;
    shot.step(frame);
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
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 1, pads: None });
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

impl super::snapshot::Wire for ReplayCamera {}
