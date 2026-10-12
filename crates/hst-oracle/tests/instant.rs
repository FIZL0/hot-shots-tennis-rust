//! The instant replay decision against `hst_sim::instant::decide`, through the oracle. Needs `context/iso/` and
//! `context/ram/s05.bin` (skips without them).

use hst_oracle::{Ee, addr};
use hst_sim::instant::{Point, decide};

fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}
fn uni(seed: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rng(seed) as f32 / u32::MAX as f32)
}
fn int(seed: &mut u64, lo: i32, hi: i32) -> i32 {
    lo + (rng(seed) % (hi - lo + 1) as u32) as i32
}
fn odds(seed: &mut u64, pct: u32) -> bool {
    rng(seed) % 100 < pct
}

/// A coordinate near one of `lines` (within 0.3 m, either side) or anywhere in ±`max`.
fn spot(s: &mut u64, lines: &[f32], max: f32) -> f32 {
    let sign = if odds(s, 50) { 1.0 } else { -1.0 };
    if odds(s, 60) { sign * (lines[int(s, 0, lines.len() as i32 - 1) as usize] + uni(s, -0.3, 0.3)) } else { uni(s, -max, max) }
}

fn point(s: &mut u64) -> Point {
    let hitter = int(s, -1, 3);
    let winner = if odds(s, 60) { hitter } else { int(s, -1, 3) };
    let o = |v: i32| (v >= 0).then_some(v as usize);
    Point {
        players: [1, 2, 2, 4, 4][int(s, 0, 4) as usize],
        awarded: odds(s, 90),
        replaying: odds(s, 10),
        allowed: odds(s, 50),
        cancelled: odds(s, 20),
        recorded: odds(s, 50),
        record_shots: int(s, -1, 3),
        winner: o(winner),
        hitter: o(hitter),
        receiver: int(s, 0, 3) as usize,
        rally: int(s, 0, 4) as u8,
        shots: [0, 1, 1, 2, 2, 3, 5, 9, 10, 11, 14][int(s, 0, 10) as usize],
        faults: [0, 0, 1, 2][int(s, 0, 3) as usize],
        ball_vel: [uni(s, -0.4, 0.4), uni(s, -0.2, 0.2), uni(s, -0.6, 0.6)],
        ball_kind: int(s, 0, 4) as u8,
        ball_sub: int(s, 0, 3),
        ball_state: int(s, 0, 6) as u8,
        ball_bounces: int(s, 0, 3),
        out_side: odds(s, 30),
        bounces: [0, 1].map(|_| [spot(s, &[0.0, 4.115, 5.485], 7.0), spot(s, &[6.4, 11.885], 14.0)]),
        verdict: if odds(s, 80) { 0 } else { int(s, 1, 4) as u8 },
        demo: odds(s, 15),
        match_won: odds(s, 20),
        option: int(s, 0, 2) as u8,
        controller: [0, 1, 0x1f, 0x20, 0x21][int(s, 0, 4) as usize],
        timing: int(s, -3, 3),
        event: int(s, -1, 2),
        ticks: int(s, 0, 1200),
    }
}

/// Generated points written into the slot-5 dump's match, ball, recorder and globals, the original's decision
/// called on its match object. The scoreboard's point award it calls first is stubbed to return `awarded`
/// (that award is `hst_sim::score`'s, checked by its own tests; its event is an input here).
#[test]
fn decides_like_the_game() {
    let Some(mut ee) = Ee::from_ram("s05.bin") else { return eprintln!("skip: no context/iso or context/ram/s05.bin") };
    let r32 = |ee: &Ee, a: u32| ee.read(a, 4) as u32;
    let gm = r32(&ee, 0x42_2f80);
    let (ball, rec, cut) = (r32(&ee, gm + 0x88), r32(&ee, gm + 0xb8), r32(&ee, gm + 0xbc));
    let (rally_blk, judge, path) = (r32(&ee, 0x31_65c0), r32(&ee, 0x42_d6c0), r32(&ee, r32(&ee, 0x43_b1d0) + 0x71c));
    let award = addr("score_award");
    let (at, n) = (addr("instant_replay"), 100_000);
    let (mut seed, mut bad, mut replays) = (0x9e37_79b9_7f4a_7c15u64, vec![], 0);
    for _ in 0..n {
        let p = point(&mut seed);
        // stub: jr ra; addiu v0, zero, awarded
        ee.write(award, 4, 0x03e0_0008);
        ee.write(award + 4, 4, 0x2402_0000 | p.awarded as u128);
        let (f, b, i) = (|v: f32| v.to_bits() as u128, |v: bool| v as u128, |v: i32| v as u32 as u128);
        let h = p.hitter.map_or(-1, |h| h as i32);
        let hk = h.max(0) as u32;
        for (a, n, v) in [
            (0x42_2fa4, 4, i(p.players)),
            (gm + 0x344, 1, b(p.replaying)),
            (gm + 0x346, 1, b(p.allowed)),
            (gm + 0x347, 1, b(p.cancelled)),
            (gm + 0x32f, 1, b(p.recorded)),
            (gm + 0x36c, 4, i(p.ticks)),
            (gm + 0x32e, 1, 0),
            (gm + 0x350, 4, 0),
            (gm + 0x35c, 1, 0),
            (gm + 0x376, 1, 0xff),
            (rec + 0xd0a0, 4, i(p.record_shots)),
            (rec + 0x10f34 + 4 * hk, 4, i(p.timing)),
            (cut + 0x84, 4, 0x1234),
            (ball + 0x150, 4, f(p.ball_vel[0])),
            (ball + 0x154, 4, f(p.ball_vel[1])),
            (ball + 0x158, 4, f(p.ball_vel[2])),
            (ball + 0x58, 1, p.ball_kind as u128),
            (ball + 0x5c, 4, i(p.ball_sub)),
            (ball + 0xa5, 1, p.ball_state as u128),
            (ball + 0x228, 4, i(p.ball_bounces)),
            (rally_blk + 0x578, 4, i(p.winner.map_or(-1, |w| w as i32))),
            (0x42_3058, 4, i(h)),
            (0x42_3054, 4, p.receiver as u128),
            (0x31_65f0, 1, p.rally as u128),
            (0x42_3060, 4, i(p.shots)),
            (0x31_6608, 4, i(p.faults)),
            (0x42_305c, 4, i(if p.out_side { 1 } else { -1 })),
            (path + 0xfb0, 4, f(p.bounces[0][0])),
            (path + 0xfb8, 4, f(p.bounces[0][1])),
            (path + 0xfa0, 4, f(p.bounces[1][0])),
            (path + 0xfa8, 4, f(p.bounces[1][1])),
            (judge + 0x426, 1, p.verdict as u128),
            (0x2e_f870, 4, b(p.demo)),
            (0x42_30be, 1, b(p.match_won)),
            (0x2e_f2b6, 1, p.option as u128),
            (0x42_2fc8 + 4 * hk, 4, i(p.controller)),
            (0x42_30b8, 4, i(p.event)),
        ] {
            ee.write(a, n, v);
        }
        ee.call(at, &[gm as u64], &[], 100_000);
        let d = decide(&p);
        let game = (
            ee.read(gm + 0x32e, 1) == 1 && ee.read(gm + 0x350, 4) == 1 && ee.read(gm + 0x35c, 1) == 1,
            ee.read(cut + 0x84, 4) == 0,
            ee.read(gm + 0x346, 1) == 1,
            ee.read(gm + 0x32f, 1) == 1,
            ee.read(gm + 0x376, 1) == 0,
        );
        let untouched = !game.0 && (ee.read(gm + 0x32e, 1) | ee.read(gm + 0x350, 4) | ee.read(gm + 0x35c, 1)) == 0;
        assert!(game.0 || untouched, "partial replay flags for {p:?}");
        replays += d.replay as usize;
        if game != (d.replay, d.reset_cut, d.allowed, d.recorded, p.players >= 2) {
            bad.push(format!("{p:?}\n  game {game:?}, port {d:?}"));
        }
    }
    eprintln!("{replays} of {n} points replay");
    assert!(replays > n / 50, "too few replays ({replays}) to say much");
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(8)].join("\n"));
}
