//! The AI's dispatcher (`hst_sim::rally`): a computer player runs the ported receive while it receives and the NET
//! or BASE rally routine (by its style, an ALL-style player by its net pick) while it rallies, in place of the
//! stand-in intercept; doubles and singles have their own routines. The routines think y-up; the app is y-down.

use super::*;
use hst_sim::ai::{style, Guess, Phase as Ai, Play};
use hst_sim::rally::{guess, msg, phase, record, step, Ball as PathBall, Body, Human, Out, CPU, PATH_MAX, PathCopy, PathFrom, Rally, SEEN, Shot, World};

/// The doubles AI state the players share (the path copy and the shot records) and each one's own.
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct Shared {
    /// Bumped once a tick (when a player steps a second time): the path copy's stamp; who stepped since.
    frame: i32,
    stepped: u8,
    path: PathCopy,
    seen: Vec<bool>,
    records: [Shot; 4],
    me: [Mine; 4],
    /// The human players' record keepers.
    humans: [Human; 4],
    /// The path object's claim: the player whose swing last found its contact (−1 none), and the tick it did.
    claim: (i32, i32),
    /// The last ball path stepped (`step`) and what it was stepped from.
    built: Built,
    /// The path's line gap (B57): the distance to the nearest line of the last landing that went from no call to
    /// out; kept across paths and points as the original's.
    gap: f32,
}

#[cfg(test)]
static NO_CACHE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Tests step the path per player again, to compare.
fn no_cache() -> bool {
    #[cfg(test)]
    return NO_CACHE.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(not(test))]
    false
}

/// The predicted ball path from now (or, for a held ball, what to step it from), and (B57) the path's mode and gap.
fn path(g: &mut Game) -> (Vec<PathBall>, Option<PathFrom>) {
    let mut start = g.flight;
    start.net = false;
    // P38: a ball without line calls (held, tossed) keeps its call and makes no gap: only the entries need its steps,
    // and only on the frames a copy is taken
    if start.lines.is_none() && !no_cache() {
        let b = &mut g.doubles_ai.built;
        (b.0, b.2, b.3) = (Vec::new(), start.call as u8, None);
        return (Vec::new(), Some(PathFrom { flight: start, shot: g.shot, court: g.court, base: g.path_base }));
    }
    // the path is the same for every player stepping on the same ball (P23: 180 steps each, the doubles AI's
    // whole cost); keyed on everything it's stepped from, bit for bit (bincode writes each float's bits)
    let key = Steps::key(&start, &g.shot, g.path_base, g.court);
    if g.doubles_ai.built.0 != key || no_cache() {
        let s = &mut g.doubles_ai.built.4;
        // P38: the ball moved one step along last tick's path: the same path one step on, so one new step instead
        // of PATH_MAX
        if s.next != key || no_cache() {
            *s = Steps::new(start, &g.shot, g.court);
        } else {
            s.advance(&g.shot, g.court);
        }
        s.next = Steps::key(&s.flights[1], &g.shot, g.path_base, g.court);
        let (path, mode, gap) = s.path(g.path_base);
        if no_cache() {
            assert_eq!(format!("{:?}", (&path, mode, gap)), format!("{:?}", predict(g.flight, &g.shot, g.court, g.path_base)));
        }
        (g.doubles_ai.built.0, g.doubles_ai.built.1, g.doubles_ai.built.2, g.doubles_ai.built.3) = (key, path, mode, gap);
    }
    if let Some(gap) = g.doubles_ai.built.3 {
        g.doubles_ai.gap = gap;
    }
    (g.doubles_ai.built.1.clone(), None)
}

/// The flights a path is stepped through (`predict`'s, kept): `flights[k]` is the ball `k` steps on, `gaps[k]` the
/// gap step `k` → `k + 1` takes if it calls the ball out.
#[derive(Default, Clone)]
struct Steps {
    flights: std::collections::VecDeque<Flight>,
    gaps: std::collections::VecDeque<Option<f32>>,
    /// The key of the path stepped from `flights[1]`.
    next: Vec<u8>,
}

impl Steps {
    fn key(f: &Flight, shot: &hst_sim::ball::Shot, base: i32, court: usize) -> Vec<u8> {
        bincode::serialize(&(f, shot, base, court)).expect("flight key")
    }

    /// `start` and the PATH_MAX steps from it.
    fn new(start: Flight, shot: &hst_sim::ball::Shot, court: usize) -> Self {
        let mut s = Steps { flights: [start].into(), ..Default::default() };
        for _ in 0..PATH_MAX {
            s.push(shot, court);
        }
        s
    }

    fn push(&mut self, shot: &hst_sim::ball::Shot, court: usize) {
        let mut f = *self.flights.back().expect("a start");
        let was = f.call;
        f.step(shot, &COURTS[court]);
        let out = was == hst_sim::judge::Call::None && f.call == hst_sim::judge::Call::Out;
        self.gaps.push_back(out.then_some(f.line_distance));
        self.flights.push_back(f);
    }

    /// Starts one step on.
    fn advance(&mut self, shot: &hst_sim::ball::Shot, court: usize) {
        self.flights.pop_front();
        self.gaps.pop_front();
        self.push(shot, court);
    }

    /// The path as `predict` gives it: the first PATH_MAX balls, the last flight's call, the last out call's gap.
    fn path(&self, base: i32) -> (Vec<PathBall>, u8, Option<f32>) {
        let path = self.flights.iter().take(PATH_MAX).map(|f| PathBall::of(f, base));
        let mode = self.flights.back().expect("a start").call as u8;
        (path.collect(), mode, self.gaps.iter().rev().find_map(|g| *g))
    }
}

/// The path stepped from `f` against the court (no net), its mode (the far end's line call) and, B57, the gap the
/// original takes on a step that calls a not-yet-landed ball out (in, net or net-out first leave the old gap).
fn predict(mut f: Flight, shot: &hst_sim::ball::Shot, court: usize, base: i32) -> (Vec<PathBall>, u8, Option<f32>) {
    let mut path = Vec::with_capacity(PATH_MAX);
    f.net = false;
    let mut gap = None;
    for _ in 0..PATH_MAX {
        let e = path_entry(&f);
        let [x, y, z] = e.pos;
        let [u, v, s] = e.vel;
        path.push(PathBall { pos: [x, y, z, 0.0], vel: [u, v, s, 0.0], bounces: base + f.contacts });
        let was = f.call;
        f.step(shot, &COURTS[court]);
        if was == hst_sim::judge::Call::None && f.call == hst_sim::judge::Call::Out {
            gap = Some(f.line_distance);
        }
    }
    (path, f.call as u8, gap)
}

/// A cache, so not part of the state (`snapshot::state_hash` prints `Game`); the flights it was stepped through (P38)
/// aren't saved either.
#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct Built(Vec<u8>, Vec<PathBall>, u8, Option<f32>, #[serde(skip)] Steps);

impl std::fmt::Debug for Built {
    fn fmt(&self, _: &mut std::fmt::Formatter) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Mine {
    rally: Rally,
    lost: i32,
    lost_log: Vec<i32>,
    /// The AI phase and shot count it last stepped in.
    phase: Option<Ai>,
    shots: i32,
    /// The re-entry's return-to-centre draw, made in the hit message (`heard_hit`).
    early: Option<u32>,
    /// A creature struck the ball or a new path began (`msg::STRIKE`, `msg::NEW_PATH`) since its last step: back to the start of
    /// its state.
    struck: bool,
    /// Singles: the point set-up ran (cleared by the serve message).
    #[serde(default)]
    set_up: bool,
}

/// The ball as the AI reads it from the ball object: y down, as the app (unlike the path, which is stored y up).
fn ball(g: &Game) -> [f32; 4] {
    let [x, y, z] = g.flight.ball.pos;
    [x, y, z, 1.0]
}

fn up(v: V3) -> [f32; 4] {
    [v[0], -v[1], v[2], 0.0]
}

/// One frame of computer player `i` in the receive or rally: false (nothing done) outside them and between ends.
pub(super) fn step(g: &mut Game, i: usize, mind: &hst_sim::ai::Mind) -> bool {
    let phase = match g.phase {
        Phase::Serve => phase::SERVE,
        Phase::Rally => phase::RALLY,
        Phase::Post => phase::POINT_OVER,
        Phase::ChangeEnds(_) => return false,
    };
    let singles = g.players.len() == 2;
    if !matches!(g.players.len(), 2 | 4) || !matches!(mind.phase, Ai::Receive | Ai::Rally) {
        return false;
    }
    let d = &mut g.doubles_ai;
    if d.stepped & 1 << i != 0 {
        (d.frame, d.stepped) = (d.frame + 1, 0);
    }
    d.stepped |= 1 << i;
    // the stand-in's preamble: the recovery, the whiff, the press's approach run
    follow_through(&mut g.players[i], true);
    whiff_frame(g, i, true);
    if let Some(dir) = g.players[i].approach {
        locomote(g, i, dir);
    }
    let p = g.players[i];
    let busy = p.contact.is_some() || p.pending.is_some() || p.swing.is_some() || p.whiff.is_some() || p.dive.is_some();
    // singles: no partner (its own spot), one opponent
    let mate = if singles { i } else { i ^ 2 };
    let human_mate = !singles && g.humans.get(mate) == Some(&true);
    let opp = if singles { [i ^ 1; 2] } else { [(i & 1) ^ 1, ((i & 1) ^ 1) + 2] };
    let r = &g.reaches[i];
    let mut b = Body {
        pos: up(p.pos),
        facing: [p.body.face.dir[0], -p.body.face.dir[1], p.body.face.dir[2], p.body.face.dir[3]],
        vel: [p.body.vel[0], p.body.vel[2]],
        side: p.end,
        hand: p.hand,
        team: i as i32,
        size: 100,
        stats: p.stats,
        stamina: p.body.stamina,
        tick: p.body.stamina_tick,
        run: p.body.run,
        moving: p.body.running as u8,
        // ponytail: character 0's contact depth and smash stand offset (x measured on the recording); per character
        // from the skeleton (P11k6)
        depth: r.ahead,
        smash_off: [-0.067, r.smash_ahead],
        control: CPU,
        formation: p.formation,
        swing: p.contact.map_or(-1, |c| c.frames as i32),
        stroke: busy as u8,
        lost: g.doubles_ai.me[i].lost,
        lost_log: std::mem::take(&mut g.doubles_ai.me[i].lost_log),
        reach: loco::ReachStats {
            base: r.base,
            reach: r.reach,
            stroke_height: r.stroke_height,
            volley_height: r.volley_height,
            smash: [r.smash_top, g.smash_heights[i][1], r.smash_bottom],
            ..Default::default()
        },
        strong: if g.strong_hand[i] >= 0.0 { hst_sim::ai::strong::MINUS } else { hst_sim::ai::strong::PLUS },
        mate: up(g.players[mate].pos),
        opp: opp.map(|j| up(g.players[j].pos)),
        mate_voice: 0,
        mark: 0,
        target: if singles { landing(g) } else { [0.0; 4] },
        opp_lefty: g.players[opp[0]].hand < 0.0,
    };
    let mut w = World {
        frame: g.doubles_ai.frame,
        phase,
        players: 4,
        court: g.score.side,
        receiver: g.score.receiver,
        hitter: g.last_hitter,
        shots: g.shots,
        ball: ball(g),
        hits: [g.flight.bounces, g.flight.contacts],
        gravity: hst_sim::ball::Params::default().gravity,
        drag: hst_sim::ball::Params::default().drag,
        floor: 0,
        path: Vec::new(),
        path_from: None,
        records: g.doubles_ai.records,
        path_mode: 0,
        path_gap: 0.0,
    };
    // B57: a CPU lets a ball that will land out by its row's margin go by (the NET and BASE routines' hold)
    (w.path_mode, w.path_gap) = (g.doubles_ai.built.2, g.doubles_ai.gap);
    if g.doubles_ai.path.stamp != w.frame {
        (w.path, w.path_from) = path(g);
    }
    let row = p.ai;
    let me = &mut g.doubles_ai.me[i];
    if singles && !std::mem::replace(&mut me.set_up, true) {
        // B15: the singles AI's point set-up: the spot it walks back to (the middle, 10 back for the NET style, else
        // 11), no dash, no short ball (hold and held clear on the serve message, `heard`)
        let z = if row.style == style::NET { -10.0 } else { -11.0 };
        (me.rally.spot[0], me.rally.spot[2]) = (0.0, hst_sim::ps2::mul(p.end, z));
        (me.rally.dash, me.rally.short) = (false, false);
    }
    let x = &mut me.rally;
    (x.level, x.mate, x.first, x.window, x.singles) = (3, human_mate as i32, 2, PATH_MAX as i32, singles);
    // B15: the net reach (the dashes' depth) its level gives it; unset it was 0 and every dash ran into the net
    x.reach = hst_sim::ai::Choice::new(0, 0, false).reach;
    (x.rate, x.radius) = if singles {
        (row.singles_center_rate, row.singles_center_radius)
    } else {
        (row.doubles_center_rate, row.doubles_center_radius)
    };
    x.high_level = p.ai_picks.high_level;
    x.lean = hst_sim::position::Team::lean(b.formation, !human_mate, row.style, g.players[mate].ai.style);
    let t = p.ai_timing;
    x.leads = [t.stroke, t.volley, t.smash];
    (x.dive, x.body, x.low, x.volley_level) =
        (p.ai_picks.dive == Some(true), p.ai_picks.body, p.ai_picks.low, p.ai_picks.volley_level);
    // the app's Mind keeps the net pick (its point result, its repick on each hit); the routines' own repicks write back
    (x.mind.net, x.mind.net_rate, x.mind.net_left) = (mind.net, mind.net_rate, mind.net_left);
    let (entered, new_shot) = (me.phase != Some(mind.phase), me.shots != g.shots);
    (me.phase, me.shots) = (Some(mind.phase), g.shots);
    let struck = std::mem::take(&mut me.struck);
    if new_shot {
        // the hit message's timing draw: the reaction (or the guess walk) and the guess
        x.wait = p.ai_hold;
        (x.guess, x.from) = match p.ai_guess {
            Some((Guess::Wide, at)) => (guess::WIDE, at),
            Some((Guess::Other, at)) => (guess::OTHER, at),
            None => (guess::NONE, x.from),
        };
        // the singles hit message on an opponent's shot: its kind and spot (ponytail: the hitter's position for the
        // shot's own spot, P11l6), and the dash after a dive
        if singles && g.last_hitter >= 0 && g.last_hitter as usize & 1 != i & 1 {
            if let Some(seen) = p.ai_seen[0] {
                x.opp_kind = seen.kind;
                x.last = b.opp[0];
                x.heard_hit(seen.kind, row.style, &b);
            }
        }
    }
    let mut early = me.early.take();
    let rng = &mut g.rng.ai;
    let mut roll = || early.take().unwrap_or_else(|| rng.next());
    // the dispatcher's state setter on entering a state, and the strike/hit messages: back to the start of its
    // state (the app already drew the net repick those messages make, so theirs is dropped)
    let kept = x.mind;
    match mind.phase {
        Ai::Receive if entered || ((new_shot || struck) && x.state != step::SWING) => x.enter_receive(&mut roll),
        // singles re-enters on the new path, not on the hit
        Ai::Rally if entered || (((new_shot && (singles || g.shots > 1)) || struck) && x.sub != step::SWING) => {
            x.enter_rally(&b, &mut w, &mut roll);
            x.mind = kept;
        }
        _ => {}
    }
    // ponytail: the caller's stick quad starts zeroed each frame
    let mut out = Out::default();
    let seen = &mut g.doubles_ai.seen;
    seen.resize(SEEN, false);
    let c = &mut g.doubles_ai.path;
    let mut received = false;
    if mind.phase == Ai::Receive {
        received = x.receive(&row, &mut b, &w, c, seen, &mut out, &mut roll);
    } else {
        let net = x.mind.play(row.style) == Play::Net;
        if singles {
            x.singles_rally(net, &row, &mut b, &w, c, seen, &mut out, &mut roll);
        } else {
            x.rally(net, &row, &mut b, &mut w, c, seen, &mut out, &mut roll);
        }
    }
    if singles {
        // the singles dispatcher's kind lock on the routine's button (its stick lock waits for the contact,
        // `ai_contact_stick`)
        out.button = out.button.map(|bt| row.lock_button(bt as u8, &mut roll) as u32);
    }
    if let Some(bit) = x.said.take() {
        g.whooshes.push((0, i, hst_sim::sound::call_out(i, bit)));
    }
    let (net, net_rate, net_left, aim) = (x.mind.net, x.mind.net_rate, x.mind.net_left, x.stick);
    (me.lost, me.lost_log) = (b.lost, b.lost_log);
    g.doubles_ai.records = w.records;
    let mut m = *mind;
    (m.net, m.net_rate, m.net_left) = (net, net_rate, net_left);
    if received {
        m.rally(&mut ai_roll(&mut g.rng));
    }
    g.players[i].ai_mind = Some(m);
    if !busy {
        let dir = if mind.active { [out.stick[0], out.stick[2]] } else { [0.0; 2] };
        let [x, z] = loco::stick_dir(loco::bot_stick(dir), phase);
        // past its wait the stick breaks off a serve's follow-through (`ai_serve_step`)
        if ai_serve_step(g, i, Vec2::new(x, z)) {
            return true;
        }
        locomote(g, i, Vec2::new(x, z));
    }
    if let Some(button) = out.button {
        g.players[i].ai_press = Some((Vec2::new(aim[0], aim[2]), button as u8));
        press(g, i, button_kind(button as u8));
    }
    advance_stroke(g, i, |g| ai_contact_stick(g, i));
    true
}

/// The hit message to a computer player `i` that rallies: its state re-enters its wait substate (as `step` does on the new
/// shot), which rolls the return to the centre first, ahead of the net repick and the timing draw. The roll is drawn
/// here and kept for `step`'s re-entry. Whether it re-enters.
pub(super) fn heard_hit(g: &mut Game, i: usize) -> bool {
    let singles = g.players.len() == 2;
    let rally = g.players[i].ai_mind.is_some_and(|m| m.phase == Ai::Rally);
    let me = &mut g.doubles_ai.me[i];
    if !(matches!(g.players.len(), 2 | 4) && rally && (singles || g.shots > 1) && me.rally.sub != step::SWING) {
        return false;
    }
    me.early = Some(g.rng.ai.next());
    true
}

/// The ball's next landing on the app's flight (y-up), for the singles routines' shot target (`Body::target`).
// ponytail: where its shot lands, not the target it aimed at (scatter and timing errors included); P11l6
fn landing(g: &Game) -> [f32; 4] {
    let mut f = g.flight;
    f.net = false;
    let n = f.contacts;
    for _ in 0..PATH_MAX {
        if f.contacts != n {
            break;
        }
        f.step(&g.shot, &COURTS[g.court]);
    }
    up(f.ball.pos)
}

/// A match message to the AI side (`msg`: `rally::msg::*`), sent where the original sends it, in its order within
/// the tick.
pub(super) fn heard(g: &mut Game, msg: u8) {
    g.doubles_ai.hear(&g.players, msg);
    for m in &mut g.doubles_ai.me {
        let x = &mut m.rally;
        match msg {
            // a serve that touched the net: the AI stands still for the rest of it
            msg::NEW_PATH if g.shots == 1 => x.hold = true,
            // ponytail: the original clears both as it sets the point up; the serve message comes there
            msg::SERVE => (x.hold, x.held, m.set_up) = (false, false, false),
            _ => {}
        }
        if msg == msg::HIT || msg == msg::NEW_PATH && g.phase == Phase::Rally {
            x.held = false;
        }
    }
}

impl Shared {
    /// `heard`: the path object drops its claim (not on the reaction) and every shot record (`record::CLEARED`); on the strike, hit, new
    /// path and serve each AI object's base handler sends its human search back to the path's start (by `players`'
    /// z) and marks the shared path copy stale.
    pub(super) fn hear(&mut self, players: &[Player], msg: u8) {
        if msg != msg::REACTION {
            self.claim.0 = -1;
        }
        self.records.iter_mut().for_each(|r| r.n = record::CLEARED);
        if matches!(msg, msg::STRIKE | msg::NEW_PATH) {
            // only an AI in its receive or rally hears it
            for (m, p) in self.me.iter_mut().zip(players) {
                m.struck = p.ai_mind.is_some_and(|x| matches!(x.phase, Ai::Receive | Ai::Rally));
            }
        }
        if matches!(msg, msg::STRIKE | msg::HIT | msg::NEW_PATH | msg::SERVE) {
            self.path = PathCopy { stamp: -1, balls: Vec::new() };
            for (h, p) in self.humans.iter_mut().zip(players) {
                h.reset(p.pos[2]);
            }
        }
    }
}

/// Player `i`'s swing found its contact (the stroke, volley or smash search; not a dive): it claims the ball unless
/// another player claimed it earlier this tick.
pub(super) fn claim(g: &mut Game, i: usize) {
    let d = &mut g.doubles_ai;
    if d.claim.0 < 0 || d.claim.1 < d.frame {
        d.claim = (i as i32, d.frame);
    }
}

/// A human player `i` in doubles: its AI object keeps its shot record (where it will play the opponents' shot), so
/// a computer partner gives way to it or follows it.
pub(super) fn human(g: &mut Game, i: usize) {
    let phase = match g.phase {
        Phase::Serve => phase::SERVE,
        Phase::Rally => phase::RALLY,
        Phase::Post => phase::POINT_OVER,
        Phase::ChangeEnds(_) => return,
    };
    // not while it serves (to its serve's end) or reacts to the point
    let serving = i as i32 == g.score.server && (g.phase == Phase::Serve || g.players[i].served.is_some());
    if g.players.len() != 4 || serving || input_off(g) {
        return;
    }
    let d = &mut g.doubles_ai;
    if d.stepped & 1 << i != 0 {
        (d.frame, d.stepped) = (d.frame + 1, 0);
    }
    d.stepped |= 1 << i;
    let p = g.players[i];
    let r = &g.reaches[i];
    let b = Body {
        pos: up(p.pos),
        facing: [p.body.face.dir[0], -p.body.face.dir[1], p.body.face.dir[2], p.body.face.dir[3]],
        vel: [p.body.vel[0], p.body.vel[2]],
        side: p.end,
        hand: p.hand,
        team: i as i32,
        size: 100,
        stats: p.stats,
        stamina: p.body.stamina,
        tick: p.body.stamina_tick,
        run: p.body.run,
        moving: p.body.running as u8,
        depth: r.ahead,
        smash_off: [-0.067, r.smash_ahead],
        control: CPU,
        reach: loco::ReachStats {
            base: r.base,
            reach: r.reach,
            stroke_height: r.stroke_height,
            volley_height: r.volley_height,
            smash: [r.smash_top, g.smash_heights[i][1], r.smash_bottom],
            ..Default::default()
        },
        strong: if g.strong_hand[i] >= 0.0 { hst_sim::ai::strong::MINUS } else { hst_sim::ai::strong::PLUS },
        mate: up(g.players[i ^ 2].pos),
        ..Default::default()
    };
    let mut w = World {
        frame: g.doubles_ai.frame,
        phase,
        players: 4,
        court: g.score.side,
        receiver: g.score.receiver,
        hitter: g.last_hitter,
        shots: g.shots,
        ball: ball(g),
        hits: [g.flight.bounces, g.flight.contacts],
        gravity: hst_sim::ball::Params::default().gravity,
        drag: hst_sim::ball::Params::default().drag,
        records: g.doubles_ai.records,
        ..Default::default()
    };
    if g.doubles_ai.path.stamp != w.frame {
        (w.path, w.path_from) = path(g);
    }
    let row = p.ai;
    let d = &mut g.doubles_ai;
    d.seen.resize(SEEN, false);
    let x = &mut d.me[i].rally;
    // its AI object beside a human: level 3, `mate` 2
    (x.level, x.mate, x.first, x.window, x.singles) = (3, 2, 2, PATH_MAX as i32, false);
    let rng = &mut g.rng.ai;
    let claimed = d.claim.0 == i as i32;
    x.human_record(&mut d.humans[i], &row, &b, &mut w, &mut d.path, &mut d.seen, claimed, &mut || rng.next());
    d.records = w.records;
}

/// The match's first point: each doubles player's formation from its placement draw (`Team::pick`; players 2 and 3
/// take their partner's), and everyone placed again for the serve with it (the set-up placed them before the rows
/// were loaded).
pub(super) fn formations(g: &mut Game, draws: &[u32]) {
    use hst_sim::position::Team;
    if g.players.len() != 4 {
        return;
    }
    let human = |g: &Game, k: usize| g.humans.get(k) == Some(&true);
    for i in 0..4 {
        let m = i ^ 2;
        g.players[i].formation = if i < 2 {
            let rows = (g.players[i].ai.formation, g.players[m].ai.formation);
            // the menus' setup words have no third byte (slots 3 and 5 read 0)
            Team::pick((human(g, i), human(g, m)), rows, 0, draws[i])
        } else {
            g.players[m].formation
        };
    }
    for i in 0..4 {
        let p = &g.players[i];
        let at = serve_placement(i as i32, &g.score, g.rally.faults, p.stance, p.formation, g.score.swapped);
        let p = &mut g.players[i];
        (p.pos, p.prev, p.home, p.body.pos) = (at.pos, at.pos, at.pos, at.pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Args, GameSpace};

    /// B57: a rally ball landing 1 m past the baseline gives the path mode 2 (out) and gap 1 m; one landing in, mode
    /// 1 and no gap; a ball already called keeps the old gap.
    #[test]
    fn path_mode_and_gap() {
        let id = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        let lines = hst_sim::judge::Lines { shots: 3, doubles: true, side: 0, hitter_far: false, margin: 0.0 };
        let shot = hst_sim::ball::Shot { params: hst_sim::ball::Params::default(), ..Default::default() };
        let landing = |vz: f32| {
            let mut f = Flight::new(Ball { pos: [0.0, -1.0, 0.5], vel: [0.0, 0.0, vz], spin: 0.0 }, id, id);
            f.lines = Some(lines);
            let mut p = f;
            p.net = false;
            for _ in 0..PATH_MAX {
                if p.contacts == 0 {
                    p.step(&shot, &COURTS[1]);
                }
            }
            (predict(f, &shot, 1, 0), p.landing.0[2])
        };
        let (out, z) = landing(0.55);
        assert!(z > 11.885, "{z}");
        assert_eq!(out.1, 2);
        assert!((out.2.unwrap() - (z - 11.885)).abs() < 0.05, "{:?} {z}", out.2);
        let ((_, mode, gap), z) = landing(0.3);
        assert!(z < 11.885, "{z}");
        assert_eq!((mode, gap), (1, None));
    }

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// The shared path (P23) plays a 4-bot doubles match tick for tick as stepping it per player did.
    #[test]
    fn path_cache_keeps_the_match() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let hashes = |cache: bool| {
            NO_CACHE.store(!cache, std::sync::atomic::Ordering::Relaxed);
            // every player a bot
            crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default()));
            app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
            app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: false, chars: vec![0, 1, 2, 3], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
            app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
            app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
            app.add_systems(Startup, setup);
            app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
            app.update();
            let mut v = Vec::new();
            for _ in 0..60 * 60 {
                app.world_mut().run_schedule(FixedUpdate);
                v.push(snapshot::state_hash(app.world_mut()));
            }
            (v, app.world().resource::<Game>().shots)
        };
        let (with, _) = hashes(true);
        let (without, shots) = hashes(false);
        assert!(shots > 0);
        assert_eq!(with.iter().zip(&without).position(|(a, b)| a != b), None);
    }

    /// B15: singles bots get the point set-up's walk-back spot (the middle, 10 or 11 back) and their level's net reach
    /// (3.2), so after their own shot they go back to the baseline instead of running into the net, where the
    /// opponent's next ball went past them.
    #[test]
    fn singles_bots_go_back_to_the_baseline() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        // every player a bot
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![0, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let (mut stepped, mut pinned, mut rallies) = (0, 0, 0);
        for _ in 0..60 * 60 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            if g.phase != Phase::Rally {
                continue;
            }
            rallies += 1;
            for (i, p) in g.players.iter().enumerate() {
                let me = &g.doubles_ai.me[i];
                if me.set_up {
                    stepped += 1;
                    let z = if p.ai.style == style::NET { -10.0 } else { -11.0 };
                    assert_eq!((me.rally.spot[0], me.rally.spot[2]), (0.0, p.end * z), "p{i}");
                    assert_eq!(me.rally.reach, 3.2, "p{i}");
                }
                // its own ball on the way: never up against the net (the court stops a player 1.5 from it)
                if g.last_hitter == i as i32 && p.pos[2].abs() < 1.6 {
                    pinned += 1;
                }
            }
        }
        assert!(rallies > 0 && stepped > 0, "{rallies} {stepped}");
        assert_eq!(pinned, 0);
    }
}
