//! The slot-5 doubles bots' formation (`context/fixtures/ai_pos_s05.bin`, tools/record_ai_pos.py; skipped when
//! missing): every change of a bot's lane, front flag or spot must be one `Formation::start`, `repick` or `hold`
//! gives from what the frame shows, and a bot must stop walking back just as it gets inside the centre radius.

use hst_sim::position::{Cue, Formation, Return, Team};

const AI: usize = 0x280;
const PLAYER: usize = 0x10 + AI;
const FRAME: usize = 4 + 0x18 + 0x9d0 + 0x40 + 0xc0 + 4 * PLAYER;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

struct Frame<'a>(&'a [u8]);
impl<'a> Frame<'a> {
    fn global(&self, a: usize) -> i32 {
        i(self.0, 4 + a - 0x423048)
    }
    fn ball(&self) -> [f32; 3] {
        let o = 4 + 0x18 + 0x9d0 + 0x30;
        [f(self.0, o), f(self.0, o + 4), f(self.0, o + 8)]
    }
    /// A player's last shot record: its position.
    fn shot(&self, p: usize) -> [f32; 2] {
        let o = 4 + 0x18 + 0x9d0 + 0x40 + 0x30 * p + 0x20;
        [f(self.0, o), f(self.0, o + 8)]
    }
    fn pos(&self, p: usize) -> [f32; 2] {
        let o = FRAME - 4 * PLAYER + p * PLAYER;
        [f(self.0, o), f(self.0, o + 8)]
    }
    fn ai(&self, p: usize) -> &'a [u8] {
        let o = FRAME - 4 * PLAYER + p * PLAYER + 0x10;
        &self.0[o..o + AI]
    }
    fn formation(&self, p: usize) -> Formation {
        let a = self.ai(p);
        Formation { lane: a[0x266], front: a[0x267] != 0, spot: [f(a, 0xc0), f(a, 0xc8)] }
    }
}

#[test]
fn formation_matches_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(d) = std::fs::read(format!("{root}/context/fixtures/ai_pos_s05.bin")) else {
        return eprintln!("ai_pos_s05.bin missing, skipped");
    };
    // header: count, then per player its address, AI address, 0x12b0..0x12c0 and 0x13f0..0x13f8; ball, shot table
    let players: Vec<(f32, usize, u8)> =
        (0..4).map(|p| 4 + 32 * p).map(|o| (f(&d, o + 8), i(&d, o + 16) as usize, d[o + 28])).collect();
    let frames: Vec<Frame> = d[4 + 32 * 4 + 8..].chunks_exact(FRAME).map(Frame).collect();
    let (mut starts, mut repicks, mut holds, mut stops) = (0, 0, 0, 0);
    for w in frames.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let last = b.global(0x423058);
        for (p, &(side, idx, formation)) in players.iter().enumerate() {
            let (was, now) = (a.formation(p), b.formation(p));
            let ai = b.ai(p);
            let partner = idx ^ 2;
            let t = Team { side, formation, lean: i(ai, 0xe0) };
            let our_hit = last >= 0 && (last & 1) as usize == idx & 1;
            if a.ai(p)[0x265] == 0 && ai[0x265] == 1 {
                // arrived: this frame's position is inside the radius, last frame's wasn't
                let (rate, radius) = (i(ai, 0x18), f(ai, 0x1c));
                let mut r = Return { wait: 0, going: true, there: false };
                assert!(!r.step(rate, radius, now.spot, b.pos(p), &mut || 0) && r.there, "vsync {}", i(b.0, 0));
                stops += 1;
            }
            let v = i(b.0, 0);
            if ai[0x54] == 0 {
                let starter = idx as i32 == b.global(0x42304c) || idx as i32 == b.global(0x423054);
                assert_eq!(Formation::start(&t, starter, b.global(0x423050) != 0), now, "vsync {v} player {p} start");
                starts += 1;
                continue;
            }
            if was == now {
                continue;
            }
            let mut cues = vec![Cue::Middle, Cue::Me(a.pos(partner)), Cue::Me(b.pos(partner))];
            for fr in [a, b] {
                cues.extend([Cue::Other(fr.pos(partner)), Cue::Other(fr.shot(partner)), Cue::Other(fr.shot(idx))]);
            }
            cues.push(Cue::Other([f(ai, 0x100), f(ai, 0x108)]));
            let found = cues.iter().any(|&c| {
                [a, b].iter().any(|fr| {
                    [a.ball(), b.ball()].iter().any(|ball| {
                        let mut g = was;
                        g.repick(&t, c, fr.pos(p), ball[2], our_hit);
                        g == now
                    })
                })
            });
            let held = [a, b].iter().any(|fr| {
                [a.ball(), b.ball()].iter().any(|ball| {
                    let mut g = was;
                    g.hold(fr.pos(p), ball[0]);
                    g == now
                })
            });
            assert!(found || held, "vsync {v} player {p}: {was:?} -> {now:?}");
            if found { repicks += 1 } else { holds += 1 }
        }
    }
    eprintln!("{starts} starts, {repicks} re-picks, {holds} holds, {stops} arrivals");
    assert!(starts >= 8 && repicks >= 30 && holds >= 3 && stops >= 30);
}
