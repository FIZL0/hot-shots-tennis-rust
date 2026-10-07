//! A singles bot match (`context/fixtures/ai_pos_singles.bin`, tools/record_ai_pos.py from a COM-vs-COM singles
//! save state; skipped when missing): each point starts at the style's centre, every net player's after-hit spot
//! and dash spot is one `Single::after_hit` gives, a dash is switched on only by the paths ported, and a bot stops
//! walking back just as it gets inside the centre radius.

use hst_sim::position::{Court, Return, Single};

const AI: usize = 0x280;
const PLAYER: usize = 0x10 + AI;
const N: usize = 2;
const BASE: usize = 4 + 0x18 + 0x9d0 + 0x40 + 0xc0;
const FRAME: usize = BASE + N * PLAYER + N * 0x30;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

struct Frame<'a>(&'a [u8]);
impl<'a> Frame<'a> {
    fn pos(&self, p: usize) -> [f32; 2] {
        let o = BASE + p * PLAYER;
        [f(self.0, o), f(self.0, o + 8)]
    }
    fn ai(&self, p: usize) -> &'a [u8] {
        let o = BASE + p * PLAYER + 0x10;
        &self.0[o..o + AI]
    }
    /// The player's shot target.
    fn target(&self, p: usize) -> [f32; 2] {
        let o = BASE + N * PLAYER + p * 0x30;
        [f(self.0, o), f(self.0, o + 8)]
    }
    fn spot(&self, p: usize) -> [f32; 2] {
        [f(self.ai(p), 0xc0), f(self.ai(p), 0xc8)]
    }
    fn dash(&self, p: usize) -> Option<[f32; 2]> {
        let a = self.ai(p);
        (a[0x256] != 0).then(|| [f(a, 0x70), f(a, 0x78)])
    }
}

#[test]
fn singles_matches_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(d) = std::fs::read(format!("{root}/context/fixtures/ai_pos_singles.bin")) else {
        return eprintln!("ai_pos_singles.bin missing, skipped");
    };
    assert_eq!(u32::from_le_bytes(d[..4].try_into().unwrap()), N as u32 | 0x100);
    let frames: Vec<Frame> = d[4 + 32 * N + 8..].chunks_exact(FRAME).map(Frame).collect();
    let (mut starts, mut afters, mut dashes, mut stops) = (0, 0, 0, 0);
    for w in frames.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let v = i32::from_le_bytes(b.0[..4].try_into().unwrap());
        for p in 0..N {
            let (x, y) = (a.ai(p), b.ai(p));
            // the side sign flips at the change of ends: +1 plays at −z
            let side = if b.pos(p)[1] < 0.0 { 1.0 } else { -1.0 };
            let c = Court { side, reach: f(y, 0x14), rate: 100, radius: f(y, 0x1c) };
            // ponytail: the style isn't recorded; a net player is the one whose point-start centre is 10 back
            let net = b.spot(p)[1].abs() == 10.0 || y[0x54] == 3 && a.spot(p)[1].abs() != 11.0;
            if y[0x54] != 3 && x[0x54] == 3 {
                assert_eq!(Single::start(&c, net, &mut || 0).spot, b.spot(p), "vsync {v} player {p} start");
                starts += 1;
            }
            if x[0x255] == 0 && y[0x255] == 1 {
                let mut r = Return { wait: 0, going: true, there: false };
                assert!(!r.step(100, c.radius, b.spot(p), b.pos(p), &mut || 0) && r.there, "vsync {v} arrival");
                stops += 1;
            }
            if x[0x54] == 3 && x[0x57] == 3 && y[0x57] == 0 && net {
                // the opponent's zone may roll the MT: try both ends
                let ok = [0, u32::MAX].iter().any(|&m| {
                    [a, b].iter().any(|fr| {
                        let mut s = Single { net, spot: a.spot(p), dash: a.dash(p), back: Return::default() };
                        s.after_hit(&c, false, x[0xb2] == 10, b.target(p), fr.pos(1 - p), fr.pos(p), &mut || m);
                        s.spot == b.spot(p) && s.dash == b.dash(p)
                    })
                });
                assert!(ok, "vsync {v} player {p}: {:?} {:?} -> {:?} {:?}", a.spot(p), a.dash(p), b.spot(p), b.dash(p));
                afters += 1;
            }
            if x[0x256] == 0 && y[0x256] == 1 {
                // contact on a volley or smash (net style), on a smash (baseline style), or after its own shot
                let contact = y[0x57] == 3 && x[0x57] != 3 && (y[0x9a] == 3 || net && y[0x9a] == 2);
                assert!(contact || x[0x57] == 3 && y[0x57] == 0, "vsync {v} player {p}: dash from elsewhere");
                dashes += 1;
            }
        }
    }
    eprintln!("{starts} starts, {afters} after-hits, {dashes} dashes, {stops} arrivals");
    assert!(starts >= 4 && afters >= 10 && dashes >= 4 && stops >= 10);
}
