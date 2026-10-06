//! Score, rotation and ends. Unit cases from the original's rules, plus a replay of every point of the user's
//! round1 recording (fixture `context/fixtures/round1.bin` from `tools/record_p2m2.py`, not in git; skipped when
//! absent): each point's winner is read from the recorded score and fed through `Score`, which must then match
//! the recorded points, games, sets, rotation, ends, server, receiver and court side exactly.

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

const BASE: usize = 4 + 0x90; // sample = u32 vsync + pad block, then the globals at 0x422f80
const SAMPLE: usize = BASE + 0x180 + 0x100 + 0x290 + 4 * (0x200 + 0x400);

fn rd(s: &[u8], addr: usize) -> i32 {
    let o = BASE + addr - 0x422f80;
    i32::from_le_bytes(s[o..o + 4].try_into().unwrap())
}

fn snapshot(f: &[u8]) -> ([i32; 2], [i32; 2], [i32; 2]) {
    ([rd(f, 0x423064), rd(f, 0x423068)], [rd(f, 0x42306c), rd(f, 0x423070)], [rd(f, 0x423074), rd(f, 0x423078)])
}

#[test]
fn round1_replay() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/round1.bin")) else { return eprintln!("round1.bin absent, skipped") };
    if data.len() < SAMPLE {
        return eprintln!("round1.bin empty, skipped");
    }
    let frames: Vec<&[u8]> = data.chunks_exact(SAMPLE).collect();
    let f0 = frames[0];
    let r = Rules { sets: rd(f0, 0x423048), games: rd(f0, 0x423044), no_deuce: f0[BASE + 0x4230bc - 0x422f80] != 0, one_point_games: false, players: rd(f0, 0x422fa4) };
    let (points, games, sets) = snapshot(f0);
    let mut s = Score { points, games, sets, rotation: rd(f0, 0x4230b0), swapped: f0[BASE + 0x4230b4 - 0x422f80] != 0, games_played: rd(f0, 0x4230c0), server: rd(f0, 0x42304c), side: rd(f0, 0x423050), receiver: rd(f0, 0x423054), ..Score::new() };
    let mut scored = 0;
    for w in frames.windows(2) {
        let (prev, cur) = (snapshot(w[0]), snapshot(w[1]));
        if prev == cur {
            continue;
        }
        let model = (s.points, s.games, s.sets);
        if cur.0 == [0, 0] && (cur.1 == model.1 || cur.1 == [0, 0]) && model.0 != [0, 0] {
            if cur.1 == [0, 0] { s.new_set() } else { s.new_game() }
            assert_eq!((s.points, s.games, s.sets), cur, "reset at vsync {}", u32::from_le_bytes(w[1][..4].try_into().unwrap()));
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
