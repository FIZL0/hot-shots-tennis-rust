//! The umpire against the slot-5 doubles match recorded by `tools/record_umpire.py` (fixture
//! `context/fixtures/umpire_s05.bin`, not in git; skipped when absent). The port is fed the recorded phases,
//! score and ball each game tick, the score call and game announcement come from `PostPoint`'s own timing, and
//! her state must match the recorded umpire every tick: motion, look, rally flags, the queued call (part, keys,
//! programs) and the word countdown. The voice's end isn't recorded: a word is taken to have ended on the
//! tick the game played the next one.

use hst_sim::flow::PostPoint;
use hst_sim::judge::Rally;
use hst_sim::score::{Event, Score};
use hst_sim::umpire::{Look, Umpire};

const N: usize = 0x1d4;

struct S<'a>(&'a [u8]);

impl S<'_> {
    fn i(&self, o: usize) -> i32 {
        i32::from_le_bytes(self.0[o..o + 4].try_into().unwrap())
    }
    fn f(&self, o: usize) -> f32 {
        f32::from_le_bytes(self.0[o..o + 4].try_into().unwrap())
    }
    fn tick(&self) -> i32 {
        self.i(4)
    }
    fn phase(&self) -> (u8, u8) {
        (self.0[8], self.0[9])
    }
    /// Umpire object field.
    fn u(&self, off: usize) -> usize {
        20 + off - 0xd0
    }
    /// Score globals 0x423040.. and 0x316600..
    fn g(&self, a: usize) -> i32 {
        self.i(260 + a - 0x423040)
    }
    fn r(&self, a: usize) -> u8 {
        self.0[388 + a - 0x316600]
    }
    fn ball(&self) -> [f32; 3] {
        [self.f(436), self.f(440), self.f(444)]
    }
    fn score(&self) -> Score {
        Score {
            points: [self.g(0x423064), self.g(0x423068)],
            server: self.g(0x42304c),
            deuce: self.r(0x316620) != 0,
            deuce_count: self.i(388 + 0x24),
            advantage: self.r(0x316628) != 0,
            tiebreak: self.r(0x31661a) != 0,
            ..Score::new()
        }
    }
    fn event(&self) -> Option<Event> {
        match self.g(0x4230b8) {
            0 => Some(Event::Point),
            1 => Some(Event::Game),
            2 => Some(Event::Set),
            _ => None,
        }
    }
    fn state(&self) -> (i32, [bool; 4], i32, i32, i32, [bool; 3], i32, [i32; 2], [u8; 2]) {
        let b = |o| self.0[self.u(o)] != 0;
        (
            self.i(self.u(0xd0)),
            [b(0xd4), b(0xd5), b(0xd6), b(0xd7)],
            self.i(self.u(0xdc)),
            self.i(self.u(0x170)),
            self.i(self.u(0x17c)),
            [b(0x191), b(0x192), b(0x193)],
            self.i(self.u(0x194)),
            [self.i(self.u(0x198)), self.i(self.u(0x19c))],
            [self.0[self.u(0x1a0)], self.0[self.u(0x1a1)]],
        )
    }
}

fn port(u: &Umpire) -> (i32, [bool; 4], i32, i32, i32, [bool; 3], i32, [i32; 2], [u8; 2]) {
    let look = match u.look {
        Look::Centre => 0,
        Look::Ball => 1,
        Look::Back => 2,
    };
    let [t0, t1] = u.tiebreak;
    (
        u.motion as i32,
        [u.side, t0, t1, u.over],
        u.countdown,
        look,
        u.look_ticks,
        [u.seen, u.rally, u.calling],
        u.part,
        u.keys,
        u.programs,
    )
}

#[test]
fn umpire_s05() {
    use hst_data::exe;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| format!("{root}/context/fixtures"));
    let (Ok(data), Ok(cnf), Ok(bin)) = (
        std::fs::read(format!("{dir}/umpire_s05.bin")),
        std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")),
        std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")),
    ) else {
        return eprintln!("umpire_s05.bin or disc files absent, skipped");
    };
    let game = exe::Game::new(&cnf, &bin).unwrap();
    let timing = game.scoreboard_timing();
    // the last sample of each game tick (a sample taken mid-tick shows the tick half done)
    let mut ss: Vec<S> = data.chunks_exact(N).map(S).collect();
    ss.dedup_by(|b, a| a.tick() == b.tick() && {
        std::mem::swap(a, b);
        true
    });
    let s0 = &ss[0];
    let (lang, ids) = (s0.0[460], (s0.0[454], s0.0[455]));
    assert_eq!(s0.phase().1, 0, "starts at the match entry");
    // ponytail: court unknown in this recording (only the deuce-again word on court 5 depends on it)
    let mut u = Umpire::new([0.0; 3], s0.0[s0.u(0xd4)] != 0, 0, game.umpire_words(lang, ids.0, ids.1), s0.ball());
    // left from the point before the save state
    u.look_ticks = s0.i(s0.u(0x17c));
    u.look_step = [s0.f(s0.u(0x180)), s0.f(s0.u(0x184)), s0.f(s0.u(0x188))];
    let mut pp: Option<(PostPoint, Score, Rally)> = None;
    let mut called = false;
    let (mut ticks, mut calls, mut plays, mut left) = (0, 0, 0, i32::MAX);
    for k in 1..ss.len() {
        let (a, s) = (&ss[k - 1], &ss[k]);
        let (was, now) = (a.state(), s.state());
        // the game played the call's last word between the samples
        let next_word = was.5[2] && !now.5[2];
        for t in a.tick() + 1..=s.tick() {
            let last = t == s.tick();
            // phase messages come before her update, the scoreboard's after
            if last && s.phase().1 != a.phase().1 {
                match s.phase().1 {
                    1 => u.start(true, a.ball()),
                    2 => u.serve(!matches!(s.phase().0, 0 | 1), s.0[16] != 0, a.ball()),
                    3 => u.rally(),
                    4 => {
                        u.point_over(s.event(), s.g(0x4230a8), s.g(0x4230b4) & 0xff != 0, s.0[254]);
                        // ponytail: a called point's longer scoreboard pause is P0b4c: there the recorded show starts the call
                        called = !matches!(s.0[254], 0 | 6);
                        pp = s.event().filter(|_| !called).map(|e| (PostPoint::new(e), Score::new(), Rally::default()));
                    }
                    5 => u.match_over(),
                    _ => {}
                }
            }
            let before = u.countdown;
            u.step(!(last && next_word), a.ball()); // she runs before the ball moves
            if last && next_word && was.6 == 1 {
                left = left.min(before - 1); // > 0: the word ended before its countdown
            }
            plays += u.voice.is_some() as i32;
            if let Some((p, score, rally)) = &mut pp {
                let (paused, showing) = (p.paused(), p.showing());
                p.step(score, rally, &timing);
                if paused && !p.paused() {
                    u.call_score(&s.score(), s.event(), s.g(0x4230a8));
                    calls += 1;
                }
                if p.showing() && !showing {
                    u.announce(&s.score(), p.event, s.g(0x4230a8));
                    calls += 1;
                }
            }
            if last && called && s.0[246] != 0 && a.0[246] == 0 {
                u.call_score(&s.score(), s.event(), s.g(0x4230a8));
                calls += 1;
                // the rest of the point-over phase from there
                let mut p = (PostPoint::new(s.event().unwrap()), Score::new(), Rally::default());
                while p.0.paused() {
                    p.0.step(&mut p.1, &mut p.2, &timing);
                }
                pp = Some(p);
            }
        }
        assert_eq!(port(&u), now, "tick {} (phase {:?})", s.tick(), s.phase());
        let step = [s.f(s.u(0x180)), s.f(s.u(0x184)), s.f(s.u(0x188))];
        assert!((0..3).all(|k| (u.look_step[k] - step[k]).abs() < 1e-4), "tick {}: look step {:?} vs {step:?}", s.tick(), u.look_step);
        ticks += 1;
    }
    eprintln!("umpire_s05: {ticks} ticks, {calls} queued calls, {plays} voices, next words {left}+ ticks before the countdown");
    assert!(calls > 20 && left > 0, "every second word waits for the first to end");
}

/// Her chair on court 10 where the recording's game put her (umpire +0xa0, read live from slot 5).
#[test]
fn chair_c10() {
    let Ok(mut iso) = hst_data::iso::Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("no ISO, skipped");
    };
    let at = hst_sim::court::umpire_chair(&mut iso, 10).unwrap();
    let live = [6.515_601_2, -1.771_156_1, -0.010_923_3];
    assert!((0..3).all(|k| (at[k] - live[k]).abs() < 1e-5), "{at:?}");
}
