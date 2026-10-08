//! The AI's dispatcher (`hst_sim::rally`): a computer player runs the ported receive while it receives and the NET
//! or BASE rally routine (by its style, an ALL-style player by its net pick) while it rallies, in place of the
//! stand-in intercept; doubles and singles have their own routines. The routines think y-up; the app is y-down.

use super::*;
use hst_sim::ai::{Guess, Phase as Ai, Play};
use hst_sim::rally::{Ball as PathBall, Body, Human, Out, PATH_MAX, PathCopy, Rally, SEEN, Shot, World};

/// The doubles AI state the players share (the path copy and the shot records) and each one's own.
#[derive(Default)]
pub(super) struct Shared {
    /// Bumped once a tick (when a player steps a second time): the path copy's stamp; who stepped since.
    frame: i32,
    stepped: u8,
    path: PathCopy,
    seen: Vec<bool>,
    records: [Shot; 4],
    me: [Mine; 4],
    /// The human players' record keepers, and the shot count the records were last reset on.
    humans: [Human; 4],
    heard: Option<i32>,
}

#[derive(Default)]
struct Mine {
    rally: Rally,
    lost: i32,
    lost_log: Vec<i32>,
    /// The AI phase and shot count it last stepped in.
    phase: Option<Ai>,
    shots: i32,
    /// The re-entry's return-to-centre draw, made in the hit message (`heard_hit`).
    early: Option<u32>,
}

fn up(v: V3) -> [f32; 4] {
    [v[0], -v[1], v[2], 0.0]
}

/// One frame of computer player `i` in the receive or rally: false (nothing done) outside them and between ends.
pub(super) fn step(g: &mut Game, i: usize, mind: &hst_sim::ai::Mind) -> bool {
    let phase = match g.phase {
        Phase::Serve => 2,
        Phase::Rally => 3,
        Phase::Post => 4,
        Phase::ChangeEnds(_) => return false,
    };
    let singles = g.players.len() == 2;
    if !matches!(g.players.len(), 2 | 4) || !matches!(mind.phase, Ai::Receive | Ai::Rally) {
        return false;
    }
    if !singles {
        heard_shot(g);
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
        control: 0x21,
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
        strong: if r.hand >= 0.0 { 1 } else { 2 },
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
        ball: up(g.flight.ball.pos),
        // ponytail: the app settles a hit at once (both counters equal), the path's mode and line gap are the
        // plain path's (P11k6)
        hits: [g.shots; 2],
        gravity: hst_sim::ball::Params::default().gravity,
        drag: hst_sim::ball::Params::default().drag,
        floor: 0,
        path: Vec::new(),
        records: g.doubles_ai.records,
        path_mode: 0,
        path_gap: 0.0,
    };
    if g.doubles_ai.path.stamp != w.frame {
        let mut f = g.flight;
        f.net = false;
        for _ in 0..PATH_MAX {
            let e = path_entry(&f);
            let [x, y, z] = e.pos;
            let [u, v, s] = e.vel;
            w.path.push(PathBall { pos: [x, y, z, 0.0], vel: [u, v, s, 0.0], bounces: g.path_base + f.contacts });
            f.step(&g.shot, &COURTS[g.court]);
        }
    }
    let row = p.ai;
    let me = &mut g.doubles_ai.me[i];
    let x = &mut me.rally;
    (x.level, x.mate, x.first, x.window, x.singles) = (3, human_mate as i32, 2, PATH_MAX as i32, singles);
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
    if new_shot {
        // the hit message's timing draw: the reaction (or the guess walk) and the guess
        x.wait = p.ai_hold;
        (x.guess, x.from) = match p.ai_guess {
            Some((Guess::Wide, at)) => (1, at),
            Some((Guess::Other, at)) => (2, at),
            None => (0, x.from),
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
        Ai::Receive if entered || (new_shot && x.state != 3) => x.enter_receive(&mut roll),
        // singles re-enters on the new path, not on the hit
        Ai::Rally if entered || (new_shot && (singles || g.shots > 1) && x.sub != 3) => {
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

/// The hit message to a computer player `i` that rallies: its state re-enters substate 0 (as `step` does on the new
/// shot), which rolls the return to the centre first, ahead of the net repick and the timing draw. The roll is drawn
/// here and kept for `step`'s re-entry. Whether it re-enters.
pub(super) fn heard_hit(g: &mut Game, i: usize) -> bool {
    let singles = g.players.len() == 2;
    let rally = g.players[i].ai_mind.is_some_and(|m| m.phase == Ai::Rally);
    let me = &mut g.doubles_ai.me[i];
    if !(matches!(g.players.len(), 2 | 4) && rally && (singles || g.shots > 1) && me.rally.sub != 3) {
        return false;
    }
    me.early = Some(g.rng.ai.next());
    true
}

/// The ball's next landing on the app's flight (y-up), for the singles routines' shot target (the player's +0x3e90).
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

/// The strike, new path and point messages to the AI objects: the shot records back to unset (n = −1) and the human
/// players' searches back to the path's start. Run once per shot count, by whichever player steps first.
// ponytail: keyed on the shot count changing, not the game's messages (the new path without a hit, P11k7); the
// claim slot (path object +0x1fc) isn't kept (P11k7)
fn heard_shot(g: &mut Game) {
    let d = &mut g.doubles_ai;
    if d.heard == Some(g.shots) {
        return;
    }
    d.heard = Some(g.shots);
    for (k, p) in g.players.iter().enumerate().take(4) {
        d.records[k].n = -1;
        d.humans[k].reset(p.pos[2]);
    }
}

/// A human player `i` in doubles: its AI object keeps its shot record (where it will play the opponents' shot), so
/// a computer partner gives way to it or follows it.
pub(super) fn human(g: &mut Game, i: usize) {
    let phase = match g.phase {
        Phase::Serve => 2,
        Phase::Rally => 3,
        Phase::Post => 4,
        Phase::ChangeEnds(_) => return,
    };
    if g.players.len() != 4 {
        return;
    }
    heard_shot(g);
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
        control: 0x21,
        reach: loco::ReachStats {
            base: r.base,
            reach: r.reach,
            stroke_height: r.stroke_height,
            volley_height: r.volley_height,
            smash: [r.smash_top, g.smash_heights[i][1], r.smash_bottom],
            ..Default::default()
        },
        strong: if r.hand >= 0.0 { 1 } else { 2 },
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
        ball: up(g.flight.ball.pos),
        hits: [g.shots; 2],
        gravity: hst_sim::ball::Params::default().gravity,
        drag: hst_sim::ball::Params::default().drag,
        records: g.doubles_ai.records,
        ..Default::default()
    };
    if g.doubles_ai.path.stamp != w.frame {
        let mut f = g.flight;
        f.net = false;
        for _ in 0..PATH_MAX {
            let e = path_entry(&f);
            let [x, y, z] = e.pos;
            let [u, v, s] = e.vel;
            w.path.push(PathBall { pos: [x, y, z, 0.0], vel: [u, v, s, 0.0], bounces: g.path_base + f.contacts });
            f.step(&g.shot, &COURTS[g.court]);
        }
    }
    let row = p.ai;
    let d = &mut g.doubles_ai;
    d.seen.resize(SEEN, false);
    let x = &mut d.me[i].rally;
    // its AI object beside a human: level 3, +0x28 = 2
    (x.level, x.mate, x.first, x.window, x.singles) = (3, 2, 2, PATH_MAX as i32, false);
    let rng = &mut g.rng.ai;
    x.human_record(&mut d.humans[i], &row, &b, &mut w, &mut d.path, &mut d.seen, false, &mut || rng.next());
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
