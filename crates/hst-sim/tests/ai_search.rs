//! The AI's contact searches call by call against the original (`tools/record_ai_search.py`): the reach search
//! (ground stroke, volley and body shot) and the tiered search, with the run estimate inside them. Each call is
//! fed the AI generator as it stood, the predicted path, the seen flags and the player's running state, and must
//! leave the game's stand spot, ball, run frames, index, flags and draw count.
//!
//! `context/fixtures/ai_search_s05.bin`: save slot 5, the bot-only doubles match (rows 84, 86 / 85, 89).
//! `context/fixtures/ai_search_singles.bin`: the bot-only singles match of `ai_serve_singles.bin`.

use hst_data::{iso::Iso, xb::Archive};
use hst_sim::ai::{AiParams, PathBall, Runner, Searcher};
use hst_sim::player::{ReachStats, Stats};

const TABLE: u32 = 0x3174c0;
const RECORD: u32 = 0x118;
const MT_SIZE: usize = 0x9c8;
const REC: usize = 0x260;

fn root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

fn table() -> Option<Vec<AiParams>> {
    let mut iso = Iso::open(format!("{}/Hot Shots Tennis (USA).iso", root())).ok()?;
    let data = iso.read("PCDATA/PCDATA.XB").ok()?;
    let arc = Archive::parse(&data).ok()?;
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("aiparam.csv"))?;
    Some(AiParams::table(&arc.read(e).ok()?))
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn i32_at(b: &[u8], o: usize) -> i32 {
    u32_at(b, o) as i32
}

fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_bits(u32_at(b, o))
}

/// The AI generator (MT19937) from its RAM block: state at +4, index at +0x9c4.
#[derive(Clone)]
struct Mt([u32; 624], usize);
impl Mt {
    fn of(b: &[u8]) -> Mt {
        Mt(std::array::from_fn(|k| u32_at(b, 4 + 4 * k)), u32_at(b, 0x9c4) as usize)
    }
    fn next(&mut self) -> u32 {
        if self.1 >= 624 {
            for k in 0..624 {
                let y = (self.0[k] & 0x8000_0000) | (self.0[(k + 1) % 624] & 0x7fff_ffff);
                self.0[k] = self.0[(k + 397) % 624] ^ (y >> 1) ^ if y & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.1 = 0;
        }
        let mut y = self.0[self.1];
        self.1 += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }
}

/// Replays every call of a fixture; returns (reach searches, tiered searches, calls that found the ball).
fn replay(name: &str) -> Option<[usize; 3]> {
    let d = std::fs::read(format!("{}/context/fixtures/{name}", root())).ok()?;
    let table = table()?;
    assert_eq!(&d[..4], b"AISR");
    let mt0 = Mt::of(&d[8..8 + MT_SIZE]);
    let mut o = 8 + MT_SIZE;
    let mut n = [0; 3];
    while o < d.len() {
        let e = &d[o..o + u32_at(&d, o + 4) as usize];
        let x = &d[o + e.len()..][..REC];
        o += e.len() + REC;
        let (tag, before) = (u32_at(e, 0), u32_at(e, 0xc));
        assert_eq!(u32_at(x, 0), tag | 0x100);
        let at = format!("{name} v{} tag {tag}", u32_at(e, 8));
        let arg = |k: usize| u32_at(e, 0x10 + 4 * k);
        let stack = |k: usize| e[0x30 + 4 * k] != 0;
        let row = &table[((u32_at(e, 0x40 + 0xc) - TABLE) / RECORD) as usize];
        let reach = ReachStats {
            base: f32_at(e, 0x84),
            reach: f32_at(e, 0x88),
            stroke_height: f32_at(e, 0x90),
            volley_height: f32_at(e, 0x94),
            ..Default::default()
        };
        let runner = Runner {
            stats: Stats { speed: f32_at(e, 0xe4), agility: i32_at(e, 0xf8), stamina: i32_at(e, 0xe8), ..Default::default() },
            size: i32_at(e, 0xd8),
            side: f32_at(e, 0xc0),
            pos: [f32_at(e, 0x110), f32_at(e, 0x118)],
            stamina: i32_at(e, 0x124),
            tick: i32_at(e, 0x128),
            run: i32_at(e, 0x12c),
            moving: e[0x135],
            players: i32_at(e, 0x140),
            rally: u32_at(e, 0x144) == 3,
            floor: i32_at(e, 0x148),
        };
        let end = u32_at(e, 0x40 + 0x3c) as usize;
        let path: Vec<PathBall> = (0..end)
            .map(|k| {
                let p = REC + 0x30 * k;
                PathBall { pos: [f32_at(e, p), f32_at(e, p + 4), f32_at(e, p + 8)], bounces: i32_at(e, p + 0x20) }
            })
            .collect();
        let s = Searcher {
            row,
            reach: &reach,
            strong: u32_at(e, 0x14c) as u8,
            singles: e[0x40 + 0x38] != 0,
            beside_human: i32_at(e, 0x40 + 0x28) > 0,
            runner,
            depth: f32_at(e, 0x100),
            path: &path,
            first: u32_at(e, 0x40 + 0x30) as usize,
            end,
        };
        let mut mt = mt0.clone();
        (0..before).for_each(|_| _ = mt.next());
        let mut used = 0;
        let mut roll = || {
            used += 1;
            mt.next()
        };
        let from = (arg(1) as i32 >= 0).then_some(arg(1) as usize);
        let mut seen: Vec<bool> = e[0x180..0x260].iter().map(|&b| b != 0).collect();
        let got = if tag == 1 {
            n[0] += 1;
            s.reach_search(from, arg(2) as i32, arg(7) as u8 != 0, stack(0), &mut roll)
        } else {
            n[1] += 1;
            s.tier_search(from, arg(2) as u8, arg(3) as i32, stack(0), stack(1), stack(2), &mut seen, &mut roll)
        };
        drop(roll);
        assert_eq!(used, u32_at(x, 0xc) - before, "{at}: draws");
        assert_eq!(got.is_some(), u32_at(x, 0x3c) == 1, "{at}: found {got:?}");
        let flags: Vec<bool> = x[0x180..0x260].iter().map(|&b| b != 0).collect();
        assert_eq!(seen, flags, "{at}: seen flags");
        if let Some(c) = got {
            n[2] += 1;
            let ball = &e[REC + 0x30 * c.at..];
            assert_eq!(u32_at(x, 0x174) as i32, c.at as i32 - s.first as i32, "{at}: index");
            assert_eq!(&x[0x160..0x170], &ball[..0x10], "{at}: ball");
            assert_eq!(
                [c.stand[0], f32_at(ball, 4), c.stand[1]].map(f32::to_bits),
                [0x150, 0x154, 0x158].map(|k| u32_at(x, k)),
                "{at}: stand {c:?}"
            );
            assert_eq!(i32_at(x, 0x170), c.frames, "{at}: frames");
        }
    }
    Some(n)
}

#[test]
fn doubles_searches_match_the_game() {
    let Some(n) = replay("ai_search_s05.bin") else { return eprintln!("fixture or disc missing, skipped") };
    eprintln!("reach searches, tiered searches, found: {n:?}");
    assert!(n[1] > 0 && n[2] > 0, "{n:?}");
}

#[test]
fn singles_searches_match_the_game() {
    let Some(n) = replay("ai_search_singles.bin") else { return eprintln!("fixture or disc missing, skipped") };
    eprintln!("reach searches, tiered searches, found: {n:?}");
    assert!(n[2] > 0, "{n:?}");
}
