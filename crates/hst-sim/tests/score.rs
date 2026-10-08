//! Score, rotation and ends. Unit cases from the original's rules, plus a replay of every point of the user's
//! round1 recording (fixture `context/fixtures/round1.bin` from `tools/record_p2m2.py`, not in git; skipped when
//! absent): each point's winner is read from the recorded score and fed through `Score`, which must then match
//! the recorded points, games, sets, rotation, ends, server, receiver and court side exactly.

use hst_sim::replay::{Frame, frames};
use hst_sim::score::{Event, Rules, Score};

const SINGLES: Rules = Rules { sets: 1, games: 4, no_deuce: false, one_point_games: false, players: 2 };
const DOUBLES: Rules = Rules { players: 4, ..SINGLES };

fn win(s: &mut Score, r: &Rules, team: usize, n: usize) -> Option<Event> {
    let mut e = None;
    for _ in 0..n {
        e = s.point(r, team);
        if e == Some(Event::Game) {
            s.new_game();
        }
        s.note_tiebreak_start();
        s.change_ends();
        s.next_point(r);
    }
    e
}

#[test]
fn game_deuce_advantage() {
    let mut s = Score::new();
    assert_eq!(win(&mut s, &SINGLES, 0, 3), Some(Event::Point));
    assert_eq!(win(&mut s, &SINGLES, 1, 3), Some(Event::Point));
    assert!(s.deuce && s.points == [3, 3] && s.deuce_count == 1);
    assert!(!s.game_point(&SINGLES, 0));
    s.point(&SINGLES, 0);
    assert!(s.advantage && s.points == [4, 3] && s.game_point(&SINGLES, 0));
    s.point(&SINGLES, 1);
    assert!(s.deuce && s.points == [3, 3] && s.deuce_count == 2);
    s.point(&SINGLES, 1);
    assert_eq!(s.point(&SINGLES, 1), Some(Event::Game));
    assert_eq!((s.points, s.games, s.rotation, s.deuce_count), ([3, 5], [0, 1], 1, 0));
}

#[test]
fn no_deuce_is_sudden_death() {
    let r = Rules { no_deuce: true, ..SINGLES };
    let mut s = Score::new();
    win(&mut s, &r, 0, 3);
    win(&mut s, &r, 1, 3);
    assert!(!s.deuce);
    assert_eq!(s.point(&r, 1), Some(Event::Game));
}

#[test]
fn tiebreak_and_set() {
    let r = SINGLES;
    let mut s = Score::new();
    for _ in 0..3 {
        win(&mut s, &r, 0, 4);
        win(&mut s, &r, 1, 4);
    }
    assert_eq!(win(&mut s, &r, 0, 4), Some(Event::Game)); // 4-3
    assert_eq!(win(&mut s, &r, 1, 4), Some(Event::Game)); // 4-4 → tiebreak
    // The tiebreak sets the ad court; the next-point toggle brings the first serve back to the deuce court.
    assert!(s.tiebreak && s.tiebreak_count == 1);
    assert_eq!(s.tiebreak_start, Some((8, s.swapped)));
    let start = s.tiebreak_start.unwrap();
    assert_eq!((s.server, s.side), (0, 0));
    for _ in 0..6 {
        assert_eq!(win(&mut s, &r, 0, 1), Some(Event::TiebreakPoint));
        assert_eq!(win(&mut s, &r, 1, 1), Some(Event::TiebreakPoint));
    }
    assert_eq!(win(&mut s, &r, 0, 1), Some(Event::TiebreakPoint)); // 7-6
    assert!(s.game_point(&r, 0));
    assert_eq!(win(&mut s, &r, 1, 1), Some(Event::TiebreakPoint)); // 7-7 deuce
    assert!(s.deuce);
    assert_eq!(win(&mut s, &r, 1, 1), Some(Event::TiebreakPoint)); // advantage
    assert_eq!(win(&mut s, &r, 1, 1), Some(Event::Set));
    assert!(s.match_over && s.sets == [0, 1] && !s.tiebreak);
    // The result board's history: the tiebreak set counts as 4-5 (the original's 7-6 for a 6-game set).
    assert_eq!((s.set, s.set_games), (1, [[4, 0, 0, 0, 0], [5, 0, 0, 0, 0]]));
    // Ends restored to the tiebreak start, then changed again: 9 games played.
    assert_eq!((s.rotation, s.swapped), (start.0 + 1, !start.1));
}

#[test]
fn rotation_receivers_and_ends() {
    let r = DOUBLES;
    let mut s = Score::new();
    s.point(&r, 0);
    s.change_ends();
    s.next_point(&r);
    assert_eq!((s.server, s.receiver, s.side), (0, 3, 1));
    s.point(&r, 0);
    s.change_ends();
    s.next_point(&r);
    assert_eq!((s.server, s.receiver, s.side), (0, 1, 0));
    s.point(&r, 0);
    s.point(&r, 0); // game 1-0: server 1, ends change
    assert!(s.change_ends());
    s.next_point(&r);
    s.new_game();
    assert_eq!((s.server, s.receiver, s.side), (1, 0, 0));
    s.second_serve = true; // first-serve fault: nothing moves
    s.next_point(&r);
    assert_eq!((s.server, s.receiver, s.side), (1, 0, 0));
}

fn rd(f: Frame, addr: usize) -> i32 {
    f.global(addr)
}

fn snapshot(f: Frame) -> ([i32; 2], [i32; 2], [i32; 2]) {
    f.score()
}

#[test]
fn round1_replay() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/round1.bin")) else { return eprintln!("round1.bin absent, skipped") };
    let frames = frames(&data);
    if frames.is_empty() {
        return eprintln!("round1.bin empty, skipped");
    }
    let f0 = frames[0];
    let r = Rules { sets: rd(f0, 0x423048), games: rd(f0, 0x423044), no_deuce: f0.global_u8(0x4230bc) != 0, one_point_games: false, players: rd(f0, 0x422fa4) };
    let (points, games, sets) = snapshot(f0);
    let mut s = Score { points, games, sets, rotation: rd(f0, 0x4230b0), swapped: f0.global_u8(0x4230b4) != 0, games_played: rd(f0, 0x4230c0), server: rd(f0, 0x42304c), side: rd(f0, 0x423050), receiver: rd(f0, 0x423054), ..Score::new() };
    let mut scored = 0;
    // Instant replays restore the point's saved state and play it again: score states already seen are skipped.
    let mut seen = vec![snapshot(f0)];
    for w in frames.windows(2) {
        let (prev, cur) = (snapshot(w[0]), snapshot(w[1]));
        if prev == cur || seen.contains(&cur) {
            continue;
        }
        seen.push(cur);
        let model = (s.points, s.games, s.sets);
        if cur.0 == [0, 0] && (cur.1 == model.1 || cur.1 == [0, 0]) && model.0 != [0, 0] {
            if cur.1 == [0, 0] { s.new_set() } else { s.new_game() }
            assert_eq!((s.points, s.games, s.sets), cur, "reset at vsync {}", w[1].vsync());
            continue;
        }
        let who = rd(w[0], 0x42304c);
        assert_eq!((s.server, s.receiver, s.side), (who, rd(w[0], 0x423054), rd(w[0], 0x423050)), "serve before point {scored}");
        let winner = (0..2).find(|&t| {
            let mut c = s.clone();
            c.point(&r, t);
            (c.points, c.games, c.sets) == cur
        });
        let Some(t) = winner else { panic!("point {scored}: no winner turns {model:?} into {cur:?}") };
        s.point(&r, t);
        s.note_tiebreak_start();
        s.change_ends();
        s.next_point(&r);
        scored += 1;
    }
    eprintln!("round1: {scored} points replayed");
    assert!(scored > 0);
}

/// The umpire's verdict at every point the game decided in round1 (sub-phase gm+0x56 = 4 is set on the
/// frame the point-over check fires): fed the recorded hits and ball state, `Rally` must also see the point
/// over, and its verdict must match the recorded outcome (no point when the game marks one, else the team
/// whose score moved). The recorded ball is the path predictor; its call is final by the decision frame.
#[test]
fn round1_verdicts() {
    use hst_sim::judge::{BallState, Call, Rally};
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/round1.bin")) else { return eprintln!("round1.bin absent, skipped") };
    let frames = frames(&data);
    let gm = |s: Frame, o: usize| s.gm()[o];
    let ball = |s: Frame| -> BallState {
        let b = s.ball();
        let f = |o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        BallState {
            call: [Call::None, Call::In, Call::Out, Call::Net, Call::NetIn, Call::NetOut][b[0xa5] as usize],
            contacts: i32::from_le_bytes(b[0x228..0x22c].try_into().unwrap()),
            stopped: b[0xa4] == 3,
            pos: [f(0xe0), f(0xe4), f(0xe8)],
        }
    };
    let mut rally = Rally::default();
    let (mut judged, mut reseeded, mut skipped, mut calls) = (0, false, 0, [0; 7]);
    let frame_no = |s: Frame| i32::from_le_bytes(s.ball()[0xac..0xb0].try_into().unwrap());
    for (k, w) in frames.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        // A scored point ends the serve's faults (the scoreboard clears them as it shows the score).
        if snapshot(b) != snapshot(a) {
            rally.faults = 0;
        }
        let shots = rd(b, 0x423060);
        if shots > rd(a, 0x423060) {
            rally.on_hit(shots, rd(b, 0x423058), rd(b, 0x42304c), rd(b, 0x423054), ball(a).contacts);
            reseeded = false;
        } else if gm(b, 0x55) == 3 && shots > 0 && frame_no(b) < frame_no(a) {
            // The predictor restarted from the live ball without a hit: the live ball met the net, which the
            // predictor ignores. The live ball (gm+0x88) is not in this capture, so its call is unknown.
            reseeded = true;
        }
        if !(gm(b, 0x56) == 4 && gm(a, 0x56) != 4 && gm(b, 0x55) == 3) {
            continue;
        }
        let body = Some(rd(b, 0x42305c)).filter(|&p| p >= 0);
        if reseeded && body.is_none() {
            eprintln!("vsync {}: net point skipped (shots {shots}, game marks {})", b.vsync(), if rd(frames[(k + 2).min(frames.len() - 1)], 0x4230b8) == -1 { "no point" } else { "a point" });
            skipped += 1;
            rally.next_point();
            rally.new_point();
            continue;
        }
        assert!(rally.check(&ball(b), shots, rd(b, 0x423058), rd(b, 0x42304c), body, true), "vsync {}: game ended the point, port did not", b.vsync());
        let v = rally.judge(body);
        // outcome: the next score change, or none if the serve comes again
        let before = snapshot(b);
        let after = frames[k + 1..].iter().map(|&s| snapshot(s)).find(|s| *s != before);
        let no_point = rd(frames[(k + 2).min(frames.len() - 1)], 0x4230b8) == -1;
        match v.winner {
            None => assert!(no_point, "vsync {}: port {v:?}, game scored", b.vsync()),
            Some(t) => {
                let Some(after) = after else { break };
                let moved = (0..2).find(|&i| after.0[i] > before.0[i] || after.1[i] > before.1[i]);
                assert_eq!(moved, Some(t), "vsync {}: port {v:?}, score {before:?} -> {after:?}", b.vsync());
            }
        }
        calls[v.call as usize] += 1;
        rally.next_point();
        rally.new_point();
        judged += 1;
    }
    eprintln!("round1: {judged} verdicts (by call {calls:?}), {skipped} net points skipped (no live ball in the capture)");
    assert!(judged > 0);
}

#[test]
fn fault_then_double_fault() {
    use hst_sim::judge::{BallState, Call, Rally};
    let out = BallState { call: Call::Out, contacts: 1, stopped: false, pos: [0.0; 3] };
    let mut r = Rally::default();
    r.on_hit(1, 0, 0, 1, 0);
    assert!(r.check(&out, 1, 0, 0, None, true));
    assert_eq!((r.judge(None).call, r.judge(None).winner, r.faults), (2, None, 1));
    r.next_point();
    r.new_point();
    assert_eq!(r.faults, 1, "second serve keeps the fault");
    r.on_hit(1, 0, 0, 1, 0);
    assert!(r.check(&out, 1, 0, 0, None, true));
    assert_eq!((r.judge(None).call, r.judge(None).winner), (3, Some(1)));
    r.next_point();
    assert_eq!(r.faults, 0);
}

/// The rally block 0x3165f0 as `Rally` models it.
fn rally_block(f: Frame) -> hst_sim::judge::Rally {
    use hst_sim::judge::{Call, Rally};
    let call = |v: u8| [Call::None, Call::In, Call::Out, Call::Net, Call::NetIn, Call::NetOut][v as usize];
    let r = |a| f.rally(a);
    let u = |a| f.rally_u8(a) != 0;
    Rally {
        call: call(f.rally_u8(0x3165f0)),
        shots: r(0x3165f4),
        hitter: r(0x3165f8),
        server: r(0x3165fc),
        illegal_hit: u(0x316604),
        faults: r(0x316608),
        let_: u(0x31660c),
        serve_call: call(f.rally_u8(0x316610)),
        checked_shots: r(0x316614),
        return_bounced: u(0x316618),
        hit_on_serve: u(0x316629),
        lose: u(0x31662a),
        serve_bounced: u(0x31662b),
    }
}

/// Every frame of the slot-5 bot match recorded with the live ball (`context/fixtures/match_s05.bin`, from
/// `tools/record_p2m2.py 5 …`, not in git): the hit check and the point-over check run on the frames the game runs
/// them (a shot counted; every rally frame until the decision), fed the live ball, and the rally block must match
/// the recording after every frame. The point-start resets (`new_point`, `next_point`, faults cleared by a scored
/// point) are applied where the recording shows them — their timing is the post-point flow (P0b4). Each decision's
/// verdict must match the outcome: no point when the game marks one, else the team whose score moved.
/// Same for the one-human doubles match `1p3goodcpus.bin` (court 11, Carol on port 0 with three CPUs).
#[test]
fn match_s05_rally_block() {
    rally_block_replay("match_s05", 30);
    rally_block_replay("1p3goodcpus", 5);
}

fn rally_block_replay(name: &str, min: usize) {
    use hst_sim::judge::{BallState, Call};
    use hst_sim::replay::frames_live;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/{name}.bin")) else { return eprintln!("{name}.bin absent, skipped") };
    let frames = frames_live(&data);
    let ball = |s: Frame| -> BallState {
        let b = s.live_ball();
        let f = |o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        BallState {
            call: [Call::None, Call::In, Call::Out, Call::Net, Call::NetIn, Call::NetOut][b[0xa5] as usize],
            contacts: i32::from_le_bytes(b[0x228..0x22c].try_into().unwrap()),
            stopped: b[0xa4] == 3,
            pos: [f(0xe0), f(0xe4), f(0xe8)],
        }
    };
    let mut rally = rally_block(frames[0]);
    let (mut saved, mut replaying, mut replays) = (rally, false, 0);
    let (mut decisions, mut calls, mut resets) = (0, [0; 7], 0);
    for (k, w) in frames.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        assert_eq!(b.vsync(), a.vsync() + 1, "frame missing after vsync {}", a.vsync());
        if replaying {
            // the instant replay re-runs the point at its own pace (gm+0x50 ticks ≠ vsyncs); it was judged live
            replaying = !(a.gm()[0x55] == 4 && b.gm()[0x55] != 4);
            rally = rally_block(b);
            continue;
        }
        let (gm55, gm56) = (b.gm()[0x55], a.gm()[0x56]);
        let in_rally = gm55 == 3 && gm56 != 4;
        let shots = rd(b, 0x423060);
        if in_rally && shots > rd(a, 0x423060) {
            rally.on_hit(shots, rd(b, 0x423058), rd(b, 0x42304c), rd(b, 0x423054), ball(a).contacts);
        }
        if in_rally {
            let body = Some(rd(a, 0x42305c)).filter(|&p| p >= 0);
            let over = rally.check(&ball(a), shots, rd(b, 0x423058), rd(b, 0x42304c), body, true);
            assert_eq!(over, b.gm()[0x56] == 4, "vsync {}: point-over check", b.vsync());
            if over {
                let v = rally.judge(body);
                let before = snapshot(b);
                let no_point = frames[k + 1..].iter().take(4).any(|&s| rd(s, 0x4230b8) == -1);
                match v.winner {
                    None => assert!(no_point, "vsync {}: port {v:?}, game scored", b.vsync()),
                    Some(t) => {
                        if let Some(after) = frames[k + 1..].iter().map(|&s| snapshot(s)).find(|s| *s != before) {
                            let moved = (0..2).find(|&i| after.0[i] > before.0[i] || after.1[i] > before.1[i]);
                            assert_eq!(moved, Some(t), "vsync {}: port {v:?}, score {before:?} -> {after:?}", b.vsync());
                        }
                    }
                }
                calls[v.call as usize] += 1;
                decisions += 1;
            }
        }
        let want = rally_block(b);
        if rally != want {
            // a point-start reset: some combination of the three, in the game's order
            let fixed = (0..8).find_map(|m| {
                let mut r = rally;
                if m & 1 != 0 { r.faults = 0 }
                if m & 2 != 0 { r.next_point() }
                if m & 4 != 0 { r.new_point() }
                (r == want).then_some((r, m & 4 != 0))
            });
            // or an instant replay: the block saved at the point's start comes back (0x316640 → 0x3165f0)
            let Some((r, start)) = fixed.or((saved == want).then_some((saved, false))) else {
                panic!("vsync {}: rally block\n port {rally:?}\n game {want:?}", b.vsync())
            };
            if start {
                saved = r;
            }
            if fixed.is_none() {
                replaying = true;
                replays += 1;
            }
            rally = r;
            resets += 1;
        }
    }
    eprintln!("{name}: {} frames, {decisions} decisions (by call {calls:?}), {resets} resets, {replays} replays", frames.len());
    assert!(decisions >= min);
}

fn score_at(f: Frame) -> Score {
    let (points, games, sets) = snapshot(f);
    let u = |a| f.global_u8(a) != 0;
    Score {
        points,
        games,
        sets,
        deuce: f.rally_u8(0x316620) != 0,
        advantage: f.rally_u8(0x316628) != 0,
        tiebreak: f.rally_u8(0x31661a) != 0,
        tiebreak_count: f.rally(0x31661c),
        rotation: rd(f, 0x4230b0),
        swapped: u(0x4230b4),
        game_changed: u(0x4230ac),
        games_played: rd(f, 0x4230c0),
        match_over: u(0x4230be),
        server: rd(f, 0x42304c),
        side: rd(f, 0x423050),
        receiver: rd(f, 0x423054),
        ..Score::new()
    }
}

fn tick(f: Frame) -> u32 {
    u32::from_le_bytes(f.gm()[0x58..0x5c].try_into().unwrap())
}

/// The point-over phase entered at `frames[0]`, run through `PostPoint` fed the recorded score and rally block
/// at the decision: faults, points and games must match every game tick (gm+0x58), and the next phase must be
/// asked for on the game's tick. `settle`: the tick the umpire's call line ends (`None`: it never does, the
/// countdown settles the call). A face-button press on port 0 is a human's press. Err: what differed.
fn post_point(frames: &[Frame], t: &hst_data::exe::ScoreboardTiming, settle: Option<u32>) -> Result<u32, String> {
    use hst_sim::flow::{Next, PostPoint};
    let f = frames[0];
    let mut score = score_at(f);
    let mut rally = rally_block(f);
    let body = Some(rd(f, 0x42305c)).filter(|&p| p >= 0);
    let event = match rd(f, 0x4230b8) {
        0 if score.tiebreak => Some(Event::TiebreakPoint),
        0 => Some(Event::Point),
        1 => Some(Event::Game),
        2 => Some(Event::Set),
        _ => None,
    };
    let mut pp = PostPoint::called(event, rally.judge(body).call, t);
    let (mut asked, mut prev) = (None, f);
    for &s in frames {
        let press = s.pad(0).buttons & !prev.pad(0).buttons & 0xf000 != 0;
        prev = s;
        while pp.tick < tick(s) {
            let idle = settle.is_some_and(|v| pp.tick + 1 >= v);
            // a press is seen on the tick it is sampled
            if let Some(n) = pp.step_with(&mut score, &mut rally, t, idle, press && pp.tick + 1 == tick(s)) {
                asked.get_or_insert((n, pp.tick));
            }
        }
        if s.gm()[0x56] != 0xff {
            let want = match s.gm()[0x56] {
                1 => Next::ChangeEnds,
                2 => Next::Serve,
                _ => Next::MatchOver,
            };
            return if asked == Some((want, tick(s))) { Ok(tick(s)) } else { Err(format!("{event:?}: asked {asked:?}, game {want:?} at {}", tick(s))) };
        }
        if asked.is_some() {
            return Err(format!("{event:?}: port asked early ({asked:?}), game at tick {}", tick(s)));
        }
        let want = (s.rally(0x316608), snapshot(s).0, snapshot(s).1);
        if (rally.faults, score.points, score.games) != want {
            return Err(format!("{event:?} tick {}: port {:?}, game {want:?}", tick(s), (rally.faults, score.points, score.games)));
        }
    }
    Err("recording ends mid-phase".into())
}

/// Every point-over phase of a recording (from the phase's entry; the match-over point and its instant replay
/// left out, P0b4c/P0b4d): those without a call must match as recorded; a called one must match for one tick
/// where her call line ends (not recorded: searched from 1 to the call's countdown), or on the countdown alone (0). Returns (phases, called
/// phases' line ends by call).
fn post_points(frames: &[Frame], t: &hst_data::exe::ScoreboardTiming) -> (usize, Vec<(u8, u32)>) {
    let (mut checked, mut lines) = (0, Vec::new());
    for k in 1..frames.len() {
        let (a, f) = (frames[k - 1], frames[k]);
        if !(f.gm()[0x55] == 4 && a.gm()[0x55] != 4) || score_at(f).match_over {
            continue;
        }
        let call = rally_block(f).judge(Some(rd(f, 0x42305c)).filter(|&p| p >= 0)).call;
        if call == 0 || call == 6 {
            post_point(&frames[k..], t, None).unwrap_or_else(|e| panic!("vsync {}: {e}", f.vsync()));
        } else {
            // a line that outlasts the countdown: the call runs on its countdown (pushed as line end 0)
            let line = (1..=t.call_wait[call as usize] as u32).find(|&v| post_point(&frames[k..], t, Some(v)).is_ok());
            let line = line.or_else(|| post_point(&frames[k..], t, None).is_ok().then_some(0));
            let line = line.unwrap_or_else(|| panic!("vsync {}: call {call}: {}", f.vsync(), post_point(&frames[k..], t, None).unwrap_err()));
            lines.push((call, line));
        }
        checked += 1;
    }
    (checked, lines)
}

/// The scoreboard timing for the region's language and the match's umpire; `ee`: a save state's EE RAM to read them
/// from (the language at 0x2ef110, the menu's umpire at 0x2ef7de), else 0 and 4 (English, umpire 4: slots 3 and 5).
fn disc_timing(ee: Option<&[u8]>) -> Option<hst_data::exe::ScoreboardTiming> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let cnf = std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")).ok()?;
    let bin = std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")).ok()?;
    let (language, umpire) = ee.map_or((0, 4), |ram| (ram[0x2ef110], ram[0x2ef7de]));
    Some(hst_data::exe::Game::new(&cnf, &bin).unwrap().scoreboard_timing(language, umpire))
}

/// The point-over phase of every point of the slot-5 bot match (P0b4, P12a): scored points, the umpire's
/// calls (fault, double fault, out after net) chained or not into the score show, faults cleared, the score reset
/// for a new game and the next phase (serve or change ends) asked on the game's own ticks (gm+0x58); every
/// change-ends phase lasts `CHANGE_ENDS`. Same for `1p3goodcpus.bin` (umpire 2), where a human's press ends the
/// phase once the score has settled. The umpire (her call countdowns) comes from each match's save-state RAM.
#[test]
fn match_s05_post_point() {
    post_point_replay("match_s05", "slot5_ee", 30, 1);
    post_point_replay("1p3goodcpus", "1p3goodcpus_ee", 4, 0);
}

fn post_point_replay(name: &str, ee: &str, min: usize, min_changes: usize) {
    use hst_sim::flow::CHANGE_ENDS;
    use hst_sim::replay::frames_live;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(ram)) = (std::fs::read(format!("{dir}/{name}.bin")), std::fs::read(format!("{dir}/{ee}.bin"))) else {
        return eprintln!("{name}.bin or {ee}.bin absent, skipped");
    };
    let Some(timing) = disc_timing(Some(&ram)) else { return eprintln!("disc files absent, skipped") };
    let frames = frames_live(&data);
    let mut changes = 0;
    for k in 1..frames.len() {
        let (a, f) = (frames[k - 1], frames[k]);
        if f.gm()[0x55] == 1 && a.gm()[0x55] != 1 {
            let end = frames[k..].iter().find(|s| s.gm()[0x56] != 0xff).unwrap();
            assert_eq!((end.gm()[0x56], tick(*end)), (2, CHANGE_ENDS + 1), "vsync {}: change ends", f.vsync());
            changes += 1;
        }
    }
    let (checked, lines) = post_points(&frames, &timing);
    eprintln!("{name}: {checked} point-over phases (call line ends {lines:?}), {changes} change-ends phases");
    assert!(checked > min && !lines.is_empty() && changes >= min_changes);
}

/// The point-over phases of a match with a human (`new_recording.bin`, slot 3): a face-button press ends the
/// phase once the new score has settled, earlier presses do nothing.
#[test]
fn human_post_point() {
    use hst_sim::replay::frames_live;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Some(timing)) = (std::fs::read(format!("{dir}/new_recording.bin")), disc_timing(None)) else {
        return eprintln!("new_recording.bin or disc files absent, skipped");
    };
    let (checked, lines) = post_points(&frames_live(&data), &timing);
    eprintln!("new_recording: {checked} point-over phases (call line ends {lines:?})");
    assert!(checked >= 8);
}

/// Every serve set-up of the slot-5 doubles match: all four players placed exactly where the game put them
/// (position and facing) on entering the serve or change-ends phase. A serve entry right after change ends
/// keeps the change-ends placement. Same for `1p3goodcpus.bin`.
#[test]
fn match_s05_serve_placement() {
    serve_placement_replay("match_s05", 37);
    serve_placement_replay("1p3goodcpus", 5);
}

fn serve_placement_replay(name: &str, setups: usize) {
    use hst_sim::flow::serve_placement;
    use hst_sim::replay::frames_live;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/{name}.bin")) else {
        return eprintln!("{name}.bin absent, skipped");
    };
    let frames = frames_live(&data);
    let mut placed = 0;
    for k in 1..frames.len() {
        let (a, f) = (frames[k - 1], frames[k]);
        let phase = f.gm()[0x55];
        if phase == a.gm()[0x55] || !(phase == 1 || phase == 2 && a.gm()[0x55] != 1) {
            continue;
        }
        // a serve entry that loads can be sampled mid-frame (1p3goodcpus: gm+0x58 still 0, nobody placed, then
        // 2): the set-up is checked on the first sample past the entry tick
        let f = frames[k..].iter().copied().find(|&s| tick(s) > 0).unwrap();
        let s = Score { server: rd(f, 0x42304c), side: rd(f, 0x423050), receiver: rd(f, 0x423054), ..Score::new() };
        let first_point = f.global_u8(0x423040) != 0;
        let swapped = (f.global_u8(0x4230b4) != 0) != (f.global_u8(0x422f99) != 0 && rd(f, 0x422fa4) != 1);
        for i in 0..4 {
            let stance = if first_point { 3.0 } else { a.player_f32(i, 0x140c) };
            let formation = f.player_f32(i, 0x13f4).to_bits() as u8; // the byte at +0x13f4
            let p = serve_placement(i as i32, &s, f.rally(0x316608), stance, formation, swapped);
            let got = (f.player_pos(i), f.player_f32(i, 0x3d68));
            assert_eq!((p.pos.map(f32::to_bits), p.facing.to_bits()), (got.0.map(f32::to_bits), got.1.to_bits()), "vsync {} player {i}: {p:?} vs {got:?}", f.vsync());
        }
        placed += 1;
    }
    assert_eq!(placed, setups, "{name}: serve set-ups in the match");
}

/// The match-point check behind the players' serve call (`Score::match_point`): game points that are and aren't
/// for the match, team 0's plain game point hiding team 1's match point, a deciding-set tiebreak.
#[test]
fn match_points() {
    let r = Rules { sets: 2, games: 6, ..DOUBLES };
    let s = Score { points: [3, 0], games: [5, 4], sets: [1, 0], ..Score::new() };
    assert_eq!(s.match_point(&r), Some(0));
    assert_eq!(Score { sets: [0, 0], ..s.clone() }.match_point(&r), None);
    assert_eq!(Score { games: [4, 4], ..s.clone() }.match_point(&r), None);
    assert_eq!(Score { points: [3, 3], deuce: true, ..s.clone() }.match_point(&r), None);
    // deuce off: 40-40 is a game point for both
    let sudden = Score { points: [3, 3], games: [0, 5], sets: [0, 1], ..Score::new() };
    assert_eq!(sudden.match_point(&Rules { no_deuce: true, ..r }), None);
    assert_eq!(Score { points: [2, 3], ..sudden }.match_point(&r), Some(1));
    let tb = Score { tiebreak: true, points: [6, 2], games: [6, 6], sets: [1, 1], set: 2, ..Score::new() };
    assert_eq!(tb.match_point(&r), Some(0));
}

/// Live toss recordings (`research/p3d4_serve_call.py`, context/p3d4, skipped when absent): slot 5's doubles serve
/// with the server's team put at match point, left as is, the receivers at match point; slot 8's singles serve at
/// match point. On a strong toss (kind 1) at the server's team's match point the server's partner (doubles) or the
/// server (singles) sets its call flag and the shared generator draws once more; otherwise nothing.
#[test]
fn serve_calls_like_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/p3d4");
    let mut seen = 0;
    for name in ["call_s05", "call_s05_asis", "call_s05_other", "call_singles"] {
        let Ok(d) = std::fs::read(format!("{root}/{name}.bin")) else { continue };
        let u = |k: usize| u32::from_le_bytes(d[4 + 4 * k..8 + 4 * k].try_into().unwrap()) as i32;
        let (players, server, kind) = (u(0), u(1) as usize, u(2));
        let s = Score {
            points: [u(3), u(4)],
            games: [u(5), u(6)],
            sets: [u(7), u(8)],
            tiebreak: u(13) != 0,
            deuce: u(14) != 0,
            advantage: u(15) != 0,
            ..Score::new()
        };
        let r = Rules { sets: u(10), games: u(11), players, ..SINGLES };
        let caller = if players == 4 { server ^ 2 } else { server };
        let calls = kind == 1 && s.match_point(&r) == Some(server & 1);
        let flips: Vec<usize> = (0..4).filter(|&k| u(16 + k) == 0 && u(20 + k) != 0).collect();
        assert_eq!(flips, if calls { vec![caller] } else { vec![] }, "{name}");
        // the new point's placements (one per player, the singles first serve's key) and the call's key
        let draws = u(24);
        let first = (players == 2) as i32;
        assert_eq!(draws, players + first + calls as i32, "{name}");
        seen += 1;
    }
    if seen == 0 {
        eprintln!("context/p3d4 missing, skipped");
    }
}
