//! The doubles AI's receive call by call against the original (`tools/record_ai_rally.py`, tag 1): each call is
//! fed the AI, its player, the match, the predicted path and the AI generator as they stood, and must leave the
//! AI's 0x280 bytes, the run target or stick, the button, the return, the path copy, the seen flags, the
//! ball-lost log and the draw count as the game did.
//!
//! `context/fixtures/ai_rally_s05.bin`, `ai_rally_s05_all.bin`: save slot 5, the bot-only doubles match.

use hst_data::{iso::Iso, xb::Archive};
use hst_sim::ai::AiParams;
use hst_sim::player::{ReachStats, Stats};
use hst_sim::position::Return;
use hst_sim::rally::{Ball, Body, Out, PathCopy, Rally, Shot, World, SEEN};

const TABLE: u32 = 0x3174c0;
const RECORD: u32 = 0x118;
const MT_SIZE: usize = 0x9c8;
const REC: usize = 0x780;

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

fn quad(b: &[u8], o: usize) -> [f32; 4] {
    std::array::from_fn(|k| f32_at(b, o + 4 * k))
}

fn balls(b: &[u8], n: usize) -> Vec<Ball> {
    (0..n)
        .map(|k| {
            let p = &b[REC + 0x30 * k..];
            Ball { pos: quad(p, 0), vel: quad(p, 0x10), bounces: i32_at(p, 0x20) }
        })
        .collect()
}

const A: usize = 0x80;

fn rally(e: &[u8]) -> Rally {
    let a = &e[A..A + 0x280];
    Rally {
        level: a[0x10],
        mate: i32_at(a, 0x28),
        first: i32_at(a, 0x30),
        window: i32_at(a, 0x34),
        singles: a[0x38] != 0,
        len: i32_at(a, 0x3c),
        state: a[0x56],
        stand: quad(a, 0x70),
        ball: quad(a, 0x80),
        frames: i32_at(a, 0x90),
        index: i32_at(a, 0x94),
        fresh: a[0x98] != 0,
        kind: a[0x9a],
        stick: quad(a, 0xa0),
        hold: a[0xb0] != 0,
        plan: a[0xb2],
        swing: i32_at(a, 0xb4),
        lead: i32_at(a, 0xb8),
        leads: [i32_at(a, 0x234), i32_at(a, 0x238), i32_at(a, 0x23c)],
        dive: a[0x245] != 0,
        wait: i32_at(a, 0x248),
        body: a[0x24c] != 0,
        low: a[0x24d] != 0,
        push: a[0x24e] != 0,
        guess: a[0x24f],
        from: [f32_at(a, 0x250), f32_at(a, 0x258)],
        sub: a[0x57],
        volley_level: a[0x5d],
        rate: i32_at(a, 0x18),
        radius: f32_at(a, 0x1c),
        chase: a[0x99] != 0,
        held: a[0xbc] != 0,
        spot: quad(a, 0xc0),
        next: i32_at(a, 0xd0),
        follow: a[0xd4] != 0,
        middle: a[0xd5] != 0,
        ours: a[0xd6] != 0,
        tick: i32_at(a, 0xd8),
        voice: a[0xdc] != 0,
        lean: i32_at(a, 0xe0),
        back: Return { wait: i32_at(a, 0x260), going: a[0x264] != 0, there: a[0x265] != 0 },
        lane: a[0x266],
        front: a[0x267] != 0,
        forward: a[0x268] != 0,
        rounds: i32_at(a, 0x26c),
        defer_rate: i32_at(a, 0x270),
        defer_roll: a[0x274] != 0,
        defer: a[0x275] != 0,
        stash: quad(e, 0x730),
    }
}

/// A flag byte: kept as the game left it (any non-zero) unless the port flipped it.
fn flag(a: &mut [u8], o: usize, v: bool) {
    if (a[o] != 0) != v {
        a[o] = v as u8;
    }
}

/// The AI's bytes with the port's fields written over the entry's.
fn bytes(e: &[u8], r: &Rally) -> Vec<u8> {
    let mut a = e[A..A + 0x280].to_vec();
    let mut w = |o: usize, v: u32| a[o..o + 4].copy_from_slice(&v.to_le_bytes());
    w(0x3c, r.len as u32);
    for k in 0..4 {
        w(0x70 + 4 * k, r.stand[k].to_bits());
        w(0x80 + 4 * k, r.ball[k].to_bits());
        w(0xa0 + 4 * k, r.stick[k].to_bits());
    }
    w(0x90, r.frames as u32);
    w(0x94, r.index as u32);
    w(0xb4, r.swing as u32);
    w(0xb8, r.lead as u32);
    w(0x248, r.wait as u32);
    for k in 0..4 {
        w(0xc0 + 4 * k, r.spot[k].to_bits());
    }
    w(0xd0, r.next as u32);
    w(0xd8, r.tick as u32);
    w(0x260, r.back.wait as u32);
    w(0x26c, r.rounds as u32);
    a[0x56] = r.state;
    flag(&mut a, 0x98, r.fresh);
    a[0x9a] = r.kind;
    a[0xb2] = r.plan;
    flag(&mut a, 0x24e, r.push);
    a[0x24f] = r.guess;
    a[0x57] = r.sub;
    flag(&mut a, 0x99, r.chase);
    flag(&mut a, 0xbc, r.held);
    flag(&mut a, 0xd4, r.follow);
    flag(&mut a, 0xd5, r.middle);
    flag(&mut a, 0xd6, r.ours);
    flag(&mut a, 0xdc, r.voice);
    flag(&mut a, 0x264, r.back.going);
    flag(&mut a, 0x265, r.back.there);
    a[0x266] = r.lane;
    flag(&mut a, 0x267, r.front);
    flag(&mut a, 0x268, r.forward);
    flag(&mut a, 0x274, r.defer_roll);
    flag(&mut a, 0x275, r.defer);
    a
}

/// The shot records (0x580..0x640) with the port's written over the entry's.
fn records(e: &[u8], w: &World) -> Vec<u8> {
    let mut a = e[0x580..0x640].to_vec();
    for (k, r) in w.records.iter().enumerate() {
        let mut put = |o: usize, v: u32| a[0x30 * k + o..0x30 * k + o + 4].copy_from_slice(&v.to_le_bytes());
        put(0, r.stamp as u32);
        put(4, r.n as u32);
        put(8, r.kind as u32);
        for j in 0..4 {
            put(0x10 + 4 * j, r.pos[j].to_bits());
            put(0x20 + 4 * j, r.vec[j].to_bits());
        }
    }
    a
}

fn body(e: &[u8]) -> Body {
    let n = (u32_at(e, 0x3b4) as usize).min(50);
    Body {
        pos: quad(e, 0x490),
        facing: quad(e, 0x480),
        vel: [f32_at(e, 0x4b0), f32_at(e, 0x4b8)],
        side: f32_at(e, 0x340),
        hand: f32_at(e, 0x344),
        team: i32_at(e, 0x348),
        size: i32_at(e, 0x358),
        stats: Stats { speed: f32_at(e, 0x364), agility: i32_at(e, 0x378), stamina: i32_at(e, 0x368), ..Default::default() },
        stamina: i32_at(e, 0x4a4),
        tick: i32_at(e, 0x4a8),
        run: i32_at(e, 0x4ac),
        moving: e[0x4e5],
        depth: f32_at(e, 0x390),
        smash_off: [f32_at(e, 0x3a0), f32_at(e, 0x3a8)],
        control: i32_at(e, 0x380),
        formation: e[0x384],
        swing: i32_at(e, 0x4c4),
        stroke: e[0x4d5],
        lost: i32_at(e, 0x3b0),
        lost_log: (0..n).map(|k| i32_at(e, 0x3b8 + 4 * k)).collect(),
        reach: ReachStats {
            base: f32_at(e, 0x304),
            reach: f32_at(e, 0x308),
            stroke_height: f32_at(e, 0x310),
            volley_height: f32_at(e, 0x314),
            smash: [f32_at(e, 0x318), f32_at(e, 0x31c), f32_at(e, 0x320)],
            ..Default::default()
        },
        strong: e[0x704],
        mate: quad(e, 0x4f0),
        opp: [quad(e, 0x500), quad(e, 0x510)],
        mate_voice: e[0x725],
        mark: e[0x4d6],
    }
}

fn world(e: &[u8]) -> World {
    let tail = (u32_at(e, 4) as usize - REC) / 0x30;
    World {
        frame: i32_at(e, 0x34),
        phase: e[0x38],
        players: i32_at(e, 0x3c),
        court: i32_at(e, 0x50),
        receiver: i32_at(e, 0x54),
        hitter: i32_at(e, 0x58),
        shots: i32_at(e, 0x60),
        ball: quad(e, 0x540),
        hits: [i32_at(e, 0x554), i32_at(e, 0x558)],
        gravity: f32_at(e, 0x710),
        drag: f32_at(e, 0x714),
        floor: i32_at(e, 0x78),
        path: balls(e, tail),
        records: std::array::from_fn(|k| {
            let o = 0x580 + 0x30 * k;
            Shot { stamp: i32_at(e, o), n: i32_at(e, o + 4), kind: i32_at(e, o + 8), pos: quad(e, o + 0x10), vec: quad(e, o + 0x20) }
        }),
        path_mode: e[0x56d],
        path_gap: f32_at(e, 0x570),
    }
}

/// Replays every call with `tag` (1 receive, 2 NET, 3 BASE) of a fixture; returns (calls, per substate on entry).
fn replay(name: &str, tag: u32) -> Option<(usize, [usize; 4])> {
    let d = std::fs::read(format!("{}/context/fixtures/{name}", root())).ok()?;
    let table = table()?;
    assert_eq!(&d[..4], b"AIRL");
    let mt0 = Mt::of(&d[8..8 + MT_SIZE]);
    let mut o = 8 + MT_SIZE;
    let (mut n, mut states) = (0, [0; 4]);
    let mut fails = 0;
    while o < d.len() {
        let e = &d[o..o + u32_at(&d, o + 4) as usize];
        o += e.len();
        let x = &d[o..o + u32_at(&d, o + 4) as usize];
        o += x.len();
        let before = u32_at(e, 0xc);
        assert_eq!(u32_at(x, 0), u32_at(e, 0) | 0x100);
        if u32_at(e, 0) != tag {
            continue;
        }
        n += 1;
        let at = format!("{name} v{} #{n}", u32_at(e, 8));
        let row = &table[((u32_at(e, A + 0xc) - TABLE) / RECORD) as usize];
        let mut r = rally(e);
        let state = if tag == 1 { r.state } else { r.sub };
        if let Some(s) = states.get_mut(state as usize) {
            *s += 1;
        }
        let mut b = body(e);
        let mut w = world(e);
        // The copy's entries aren't recorded on entry: taken this frame they are the path's first `count`; an older
        // copy is only ever replaced, so its count is all that shows.
        let count = i32_at(e, 0x74).max(0) as usize;
        let mut copy = w.path[..count.min(w.path.len())].to_vec();
        copy.resize(count, Ball::default());
        let mut c = PathCopy { stamp: i32_at(e, 0x70), balls: copy };
        let mut seen: Vec<bool> = e[0x640..0x640 + SEEN].iter().map(|&v| v != 0).collect();
        let mut out = Out { stick: quad(e, 0x20), button: None };
        let mut mt = mt0.clone();
        (0..before).for_each(|_| _ = mt.next());
        let mut used = 0;
        let mut roll = || {
            used += 1;
            mt.next()
        };
        let ret = if tag == 1 {
            r.receive(row, &mut b, &w, &mut c, &mut seen, &mut out, &mut roll)
        } else {
            r.rally(tag == 2, row, &mut b, &mut w, &mut c, &mut seen, &mut out, &mut roll);
            false
        };
        drop(roll);
        let mut errs = Vec::new();
        if used != u32_at(x, 0xc) - before {
            errs.push(format!("draws {used} vs {}", u32_at(x, 0xc) - before));
        }
        let (got, want) = (bytes(e, &r), &x[A..A + 0x280]);
        let diff: Vec<String> = (0..0x280 / 4)
            .filter(|k| got[4 * k..4 * k + 4] != want[4 * k..4 * k + 4])
            .map(|k| format!("+{:#x} {:08x} vs {:08x}", 4 * k, u32_at(&got, 4 * k), u32_at(want, 4 * k)))
            .collect();
        if !diff.is_empty() {
            errs.push(format!("ai {}", diff.join(", ")));
        }
        if out.stick.map(f32::to_bits) != quad(x, 0x20).map(f32::to_bits) {
            errs.push(format!("stick {:?} vs {:?}", out.stick, quad(x, 0x20)));
        }
        let button = out.button.unwrap_or(u32_at(e, 0x30));
        if button != u32_at(x, 0x30) {
            errs.push(format!("button {button} vs {}", u32_at(x, 0x30)));
        }
        if tag != 1 && records(e, &w) != x[0x580..0x640] {
            let (got, want) = (records(e, &w), &x[0x580..0x640]);
            let diff: Vec<String> = (0..0xc0 / 4)
                .filter(|k| got[4 * k..4 * k + 4] != want[4 * k..4 * k + 4])
                .map(|k| format!("+{:#x} {:08x} vs {:08x}", 4 * k, u32_at(&got, 4 * k), u32_at(want, 4 * k)))
                .collect();
            errs.push(format!("records {}", diff.join(", ")));
        }
        // NET and BASE leave v0 as whatever their last call returned
        if tag == 1 && ret as u32 != u32_at(x, 0x1c) {
            errs.push(format!("return {ret} vs {}", u32_at(x, 0x1c)));
        }
        if (c.stamp, c.balls.len() as i32) != (i32_at(x, 0x70), i32_at(x, 0x74)) {
            errs.push(format!("copy {} {} vs {} {}", c.stamp, c.balls.len(), i32_at(x, 0x70), i32_at(x, 0x74)));
        }
        let len = (r.len.max(0) as usize).min(c.balls.len());
        if i32_at(e, 0x70) != w.frame && c.stamp == w.frame && c.balls[..len] != balls(x, (u32_at(x, 4) as usize - REC) / 0x30)[..] {
            errs.push("path copy".into());
        }
        if seen != x[0x640..0x640 + SEEN].iter().map(|&v| v != 0).collect::<Vec<_>>() {
            errs.push("seen".into());
        }
        let lost = body(x);
        if (b.lost, &b.lost_log) != (lost.lost, &lost.lost_log) {
            errs.push(format!("lost {} {:?} vs {} {:?}", b.lost, b.lost_log, lost.lost, lost.lost_log));
        }
        if !errs.is_empty() {
            fails += 1;
            if fails <= 15 {
                eprintln!("{at} state {state} kind {} guess {}: {}", e[A + 0x9a], e[A + 0x24f], errs.join("; "));
            }
        }
    }
    assert_eq!(fails, 0, "{name}: {fails} of {n} calls differ");
    Some((n, states))
}

#[test]
fn receive_matches_the_game() {
    let Some(n) = replay("ai_rally_s05.bin", 1) else { return eprintln!("fixture or disc missing, skipped") };
    eprintln!("calls, per substate: {n:?}");
}

#[test]
fn receive_matches_the_game_long() {
    for name in ["ai_rally_s05_fix.bin", "ai_rally_s05_all.bin", "ai_rally_s05_long.bin", "ai_rally_s05_slow.bin", "ai_rally_s05_slow2.bin"] {
        let Some(n) = replay(name, 1) else { return eprintln!("fixture or disc missing, skipped") };
        eprintln!("{name}: calls, per substate {n:?}");
        assert!(n.1.iter().all(|&k| k > 0), "{n:?}");
    }
}

#[test]
fn net_and_base_match_the_game() {
    for (tag, what) in [(2, "NET"), (3, "BASE")] {
        let Some(n) = replay("ai_net_s05.bin", tag) else { return eprintln!("fixture or disc missing, skipped") };
        eprintln!("{what}: calls, per substate {n:?}");
    }
}
