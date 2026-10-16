//! The doubles AI's receive call by call against the original (`tools/record_ai_rally.py`, tag 1): each call is
//! fed the AI, its player, the match, the predicted path and the AI generator as they stood, and must leave the
//! AI's 0x280 bytes, the run target or stick, the button, the return, the path copy, the seen flags, the
//! ball-lost log and the draw count as the game did.
//!
//! `context/fixtures/ai_rally_s05.bin`, `ai_rally_s05_all.bin`: save slot 5, the bot-only doubles match.
//! `ai_rally_singles*.bin`: the singles routines (`HST_SINGLES=1`), `bots_singles` in slot 8 (`_rows`: rows 45 level 3 and 16
//! level 1, both NET, set with `HST_AI=0:45:3,1:16:1`; `_spec`: rows 13 BASE and 2 NET, level 3; `_base`: rows 51 level 3 and
//! 5 level 0, both BASE); the AI fields 0x234..0x260 sit 0x10 lower there. `ai_rally_s05_weak.bin`: slot 5 with level-0
//! doubles rows (`HST_AI=0:89:0,1:84:0,2:86:0,3:97:0`).

use hst_data::{iso::Iso, xb::Archive};
use hst_sim::ai::{AiParams, Mind, Play};
use hst_sim::player::{ReachStats, Stats};
use hst_sim::position::Return;
use hst_sim::rally::{Ball, Body, Human, Out, PathCopy, Rally, Shot, World, SEEN};

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

/// A doubles AI offset in the singles AI: its receive fields (0x234..0x260) sit 0x10 lower.
fn shift(o: usize, singles: bool) -> usize {
    if singles && (0x234..0x260).contains(&o) { o - 0x10 } else { o }
}

/// The walk back (wait, going, there) and the net pick (left, rate, net): 0x260.. and 0x26c.. in doubles,
/// 0x250.. and 0x258.. in singles.
fn back_mind(singles: bool) -> (usize, usize) {
    if singles { (0x250, 0x258) } else { (0x260, 0x26c) }
}

fn rally(e: &[u8], singles: bool) -> Rally {
    let at = |o| shift(o, singles);
    let (bk, md) = back_mind(singles);
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
        leads: [i32_at(a, at(0x234)), i32_at(a, at(0x238)), i32_at(a, at(0x23c))],
        dive: a[at(0x245)] != 0,
        wait: i32_at(a, at(0x248)),
        body: a[at(0x24c)] != 0,
        low: a[at(0x24d)] != 0,
        push: a[at(0x24e)] != 0,
        guess: a[at(0x24f)],
        from: [f32_at(a, at(0x250)), f32_at(a, at(0x258))],
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
        said: None,
        lean: i32_at(a, 0xe0),
        back: Return { wait: i32_at(a, bk), going: a[bk + 4] != 0, there: a[bk + 5] != 0 },
        lane: a[0x266],
        front: a[0x267] != 0,
        forward: a[0x268] != 0,
        mind: Mind { net_left: i32_at(a, md), net_rate: i32_at(a, md + 4), net: a[md + 8] != 0, ..Mind::default() },
        defer: a[0x275] != 0,
        stash: quad(e, 0x730),
        reach: f32_at(a, 0x14),
        dash: a[0x256] != 0 && singles,
        last: quad(a, 0x130),
        return_level: a[0x5e],
        short: a[0x257] != 0 && singles,
        high_level: a[0x5f],
        opp_kind: i32_at(a, 0x124),
    }
}

/// A flag byte: kept as the game left it (any non-zero) unless the port flipped it.
fn flag(a: &mut [u8], o: usize, v: bool) {
    if (a[o] != 0) != v {
        a[o] = v as u8;
    }
}

/// The AI's bytes with the port's fields written over the entry's.
fn bytes(e: &[u8], r: &Rally, singles: bool) -> Vec<u8> {
    let at = |o| shift(o, singles);
    let (bk, md) = back_mind(singles);
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
    w(at(0x248), r.wait as u32);
    for k in 0..4 {
        w(0xc0 + 4 * k, r.spot[k].to_bits());
    }
    w(0xd0, r.next as u32);
    w(0xd8, r.tick as u32);
    w(bk, r.back.wait as u32);
    w(md, r.mind.net_left as u32);
    a[0x56] = r.state;
    flag(&mut a, 0x98, r.fresh);
    a[0x9a] = r.kind;
    a[0xb2] = r.plan;
    flag(&mut a, at(0x24e), r.push);
    a[at(0x24f)] = r.guess;
    a[0x57] = r.sub;
    flag(&mut a, 0x99, r.chase);
    flag(&mut a, 0xbc, r.held);
    flag(&mut a, 0xd4, r.follow);
    flag(&mut a, 0xd5, r.middle);
    flag(&mut a, 0xd6, r.ours);
    flag(&mut a, 0xdc, r.voice);
    flag(&mut a, bk + 4, r.back.going);
    flag(&mut a, bk + 5, r.back.there);
    flag(&mut a, md + 8, r.mind.net);
    if singles {
        flag(&mut a, 0x256, r.dash);
        flag(&mut a, 0x257, r.short);
    } else {
        a[0x266] = r.lane;
        flag(&mut a, 0x267, r.front);
        flag(&mut a, 0x268, r.forward);
        flag(&mut a, 0x275, r.defer);
    }
    a
}

/// The AI's bytes with every field `rally` reads written back from `r` (`bytes` writes only the ones the
/// routines were first checked on): a port write the game doesn't make, or a game write the port misses, shows.
fn bytes_all(e: &[u8], r: &Rally, singles: bool) -> Vec<u8> {
    let at = |o| shift(o, singles);
    let mut a = bytes(e, r, singles);
    let mut w = |o: usize, v: u32| a[o..o + 4].copy_from_slice(&v.to_le_bytes());
    w(0x28, r.mate as u32);
    w(0x30, r.first as u32);
    w(0x34, r.window as u32);
    for (k, v) in r.leads.iter().enumerate() {
        w(at(0x234 + 4 * k), *v as u32);
    }
    w(at(0x250), r.from[0].to_bits());
    w(at(0x258), r.from[1].to_bits());
    w(0x18, r.rate as u32);
    w(0x1c, r.radius.to_bits());
    w(0xe0, r.lean as u32);
    w(back_mind(singles).1 + 4, r.mind.net_rate as u32);
    w(0x14, r.reach.to_bits());
    for k in 0..4 {
        w(0x130 + 4 * k, r.last[k].to_bits());
    }
    w(0x124, r.opp_kind as u32);
    a[0x10] = r.level;
    a[0x5d] = r.volley_level;
    a[0x5e] = r.return_level;
    a[0x5f] = r.high_level;
    flag(&mut a, 0x38, r.singles);
    flag(&mut a, 0xb0, r.hold);
    flag(&mut a, at(0x245), r.dive);
    flag(&mut a, at(0x24c), r.body);
    flag(&mut a, at(0x24d), r.low);
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
            under_min: f32_at(e, 0x30c),
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
        target: quad(e, 0x4f0),
        opp_lefty: e[0x521] != 0,
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
        path_from: None,
        records: std::array::from_fn(|k| {
            let o = 0x580 + 0x30 * k;
            Shot { stamp: i32_at(e, o), n: i32_at(e, o + 4), kind: i32_at(e, o + 8), pos: quad(e, o + 0x10), vec: quad(e, o + 0x20) }
        }),
        path_mode: e[0x56d],
        path_gap: f32_at(e, 0x570),
    }
}

/// One recorded call (entry `e`, exit `x`) through the port: the AI state it leaves, and what differs from the
/// game's exit (the AI bytes `bytes` writes back, the stick, button, shot records, return, path copy, seen flags,
/// ball-lost log, draw count).
fn call(name: &str, e: &[u8], x: &[u8], mt0: &Mt, row: &AiParams, tag: u32, singles: bool) -> (Rally, Vec<String>) {
    let before = u32_at(e, 0xc);
    let mut r = rally(e, singles);
    if name == "ai_human_vpad_b.bin" && tag == 2 && (9760..=9764).contains(&u32_at(e, 8)) {
        // recorded before the recorder took the stand scratch last in its entry stub: a vsync during the stub
        // zeroed it after it was taken (as on m3c v6843 / m3d v7900, P11k8), so the game read 0 here
        r.stash = [0.0; 4];
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
    let a = &e[A..];
    let mut h = Human { found: a[0x40] != 0, next: i32_at(a, 0x44), kind: a[0x48], min: i32_at(a, 0x4c), volley: a[0x50] != 0 };
    let ret = if tag == 1 {
        r.receive(row, &mut b, &w, &mut c, &mut seen, &mut out, &mut roll)
    } else if tag == 4 {
        let claimed = i32_at(e, 0x74c) == b.team;
        r.human_record(&mut h, row, &b, &mut w, &mut c, &mut seen, claimed, &mut roll);
        false
    } else {
        if singles {
            r.singles_rally(tag == 2, row, &mut b, &w, &mut c, &mut seen, &mut out, &mut roll);
        } else {
            r.rally(tag == 2, row, &mut b, &mut w, &mut c, &mut seen, &mut out, &mut roll);
        }
        false
    };
    drop(roll);
    let mut errs = Vec::new();
    if used != u32_at(x, 0xc) - before {
        errs.push(format!("draws {used} vs {}", u32_at(x, 0xc) - before));
    }
    let mut got = bytes(e, &r, singles);
    if tag == 4 {
        got[0x44..0x48].copy_from_slice(&h.next.to_le_bytes());
        got[0x4c..0x50].copy_from_slice(&h.min.to_le_bytes());
        flag(&mut got, 0x40, h.found);
        got[0x48] = h.kind;
    }
    let want = &x[A..A + 0x280];
    let diff: Vec<String> = (0..0x280 / 4)
        .filter(|k| got[4 * k..4 * k + 4] != want[4 * k..4 * k + 4])
        .map(|k| format!("+{:#x} {:08x} vs {:08x}", 4 * k, u32_at(&got, 4 * k), u32_at(want, 4 * k)))
        .collect();
    if !diff.is_empty() {
        errs.push(format!("ai {}", diff.join(", ")));
    }
    // the human record takes no stick or button (their record slots hold whatever was there)
    if tag != 4 && out.stick.map(f32::to_bits) != quad(x, 0x20).map(f32::to_bits) {
        errs.push(format!("stick {:?} vs {:?}", out.stick, quad(x, 0x20)));
    }
    let button = out.button.unwrap_or(u32_at(e, 0x30));
    if tag != 4 && button != u32_at(x, 0x30) {
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
    (r, errs)
}

/// Replays every call with `tag` (1 receive, 2 NET, 3 BASE) of a fixture; returns (calls, per substate on entry).
fn replay(name: &str, tag: u32) -> Option<(usize, [usize; 4])> {
    let singles = name.contains("singles");
    let d = std::fs::read(format!("{}/context/fixtures/{name}", root())).ok()?;
    let mut table = table()?;
    if name.contains("let_go") {
        // recorded with every CPU row's let-go margin poked to -3 m (`HST_MARGIN=-3`)
        table.iter_mut().for_each(|r| r.line_margin = -3.0);
    }
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
        assert_eq!(u32_at(x, 0), u32_at(e, 0) | 0x100);
        if u32_at(e, 0) != tag {
            continue;
        }
        n += 1;
        let at = format!("{name} v{} #{n}", u32_at(e, 8));
        let row = &table[((u32_at(e, A + 0xc) - TABLE) / RECORD) as usize];
        // the dispatcher: receive in state 2, NET/BASE by style in state 3
        let want = match e[A + 0x54] {
            _ if tag == 4 => 4,
            2 => 1,
            3 => match (Mind { net: e[A + back_mind(singles).1 + 8] != 0, ..Mind::default() }).play(row.style) {
                Play::Net => 2,
                Play::Base => 3,
            },
            s => panic!("{name}: routine {} called in state {s}", u32_at(e, 0)),
        };
        assert_eq!(u32_at(e, 0), want, "{name}: dispatch");
        let r = rally(e, singles);
        let state = if tag == 1 { r.state } else { r.sub };
        if let Some(s) = states.get_mut(state as usize) {
            *s += 1;
        }
        let (_, errs) = call(name, e, x, &mt0, row, tag, singles);
        if !errs.is_empty() {
            fails += 1;
            if fails <= 15 {
                eprintln!("{at} state {state} kind {} guess {}: {}", e[A + 0x9a], e[A + shift(0x24f, singles)], errs.join("; "));
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
    for name in ["ai_rally_s05_fix.bin", "ai_rally_s05_all.bin", "ai_rally_s05_long.bin", "ai_rally_s05_slow.bin", "ai_rally_s05_slow2.bin", "ai_rally_s05_weak.bin"] {
        let Some(n) = replay(name, 1) else { return eprintln!("fixture or disc missing, skipped") };
        eprintln!("{name}: calls, per substate {n:?}");
        assert!(n.1.iter().all(|&k| k > 0), "{n:?}");
    }
}

/// P11k4: the receive branches only a human's serves reach, slot 3 with P1 serving from the pad (`HST_DRIVE=1
/// HST_SERVE=cross HST_SERVE_Y=1 HST_SERVE_WALK=0`: short sliced serves to the receiver's sideline; journal
/// 2026-10-07-p11-ai 23). Every call replays bit for bit, and each fixture must run its branch, read from the game's
/// own exits: `_wide`/`_wide_b` the out-of-court aims (plans 11 and 10, the receiver past the doubles sideline as it
/// aims; `_b` waits 47 polls before its first serve, `HST_SERVE_DELAY`, for another roll);
/// `_last` the last-entry pick (players at 0.3 speed so the search misses, and the path object stopped growing past
/// z 9 as at its 600-entry cap, which no point lasts long enough to reach: copies of 1..=11 entries); `_smash` a
/// smash find (the receivers' stroke height 0.01 and smash window 0.8..1.8 m poked into their character records,
/// which the records carry, as no serve kicks up to the real 1.9 m).
#[test]
fn vpad_serves_receive_matches_the_game() {
    let (mut plans, mut short, mut smash) = ([0; 2], 0, 0);
    for name in ["ai_rally_vpad_wide.bin", "ai_rally_vpad_wide_b.bin", "ai_rally_vpad_last.bin", "ai_rally_vpad_smash.bin"] {
        let Some(n) = replay(name, 1) else { return eprintln!("{name} or disc missing, skipped") };
        eprintln!("{name}: calls, per substate: {n:?}");
        let d = std::fs::read(format!("{}/context/fixtures/{name}", root())).unwrap();
        let mut counts = [0; 4];
        for (_, _, e, x) in calls(&d, &[1]) {
            let (from, to, plan) = (e[A + 0x56], x[A + 0x56], x[A + 0xb2]);
            if from == 2 && to == 3 && (10..=11).contains(&plan) {
                counts[(plan - 10) as usize] += 1;
            }
            if from == 0 && to == 0 && (1..=11).contains(&i32_at(x, A + 0x3c)) {
                counts[2] += 1;
            }
            if from == 0 && to == 1 && x[A + 0x9a] == 3 {
                counts[3] += 1;
            }
        }
        match name {
            _ if name.contains("wide") => plans = [plans[0] + counts[0], plans[1] + counts[1]],
            "ai_rally_vpad_last.bin" => short = counts[2],
            _ => smash = counts[3],
        }
    }
    eprintln!("plans 10/11 {plans:?}, short copies {short}, smash finds {smash}");
    assert!(plans[0] > 0 && plans[1] > 0 && short > 0 && smash > 0);
}

/// What a doubles NET/BASE call ran, read from its entry `e` and exit `x` (P11k5): the not-found walk on a short
/// landing (by its target: 0 landing + 0.3, 1 the after-bounce descent, 2 entry 11, 3 the last entry; the chase flag
/// kept), the not-found tail's repick from the partner's record, the give-way defer set, the substate-3 wait
/// countdown, and beside a human (+0x264 > 0) which of give-way's comparisons decided.
#[derive(Debug, Default)]
struct NetBase {
    short: [usize; 4],
    chase_kept: usize,
    repick: [usize; 2],
    defer: [usize; 2],
    wait: [usize; 2],
    give_way: [usize; 5],
}

impl NetBase {
    fn add(&mut self, e: &[u8], x: &[u8]) {
        let (r, after) = (rally(e, false), rally(x, false));
        let (b, w) = (body(e), world(e));
        let human = (r.mate > 0) as usize;
        let sign = |v: f32| if v > 0.0 { 1 } else if v < 0.0 { -1 } else { 0 };
        if r.sub == 3 && after.wait == r.wait - 1 {
            self.wait[human] += 1;
        }
        if !r.defer && after.defer {
            self.defer[human] += 1;
        }
        // give-way runs on the frames in substate 1 until it defers; on the frame the substate reaches 1 the AI's
        // own record is the one it has just written (the exit's), the human partner's is never touched here
        if r.mate > 0 && !r.defer && (r.sub == 1 || after.sub == 1) {
            let mine = if r.sub == 1 { w.records[b.team as usize] } else { world(x).records[b.team as usize] };
            self.give_way[give_way_test(&b, &mine, &w.records[(b.team ^ 2) as usize])] += 1;
        }
        // past the early returns, the copy taken and nothing found: the tail ends by aiming the next search at the
        // copy's last entry, or repicks from the partner's record and follows it
        let tail = r.sub == 0 && after.sub == 0 && !(r.hold || r.held || r.wait > 0 || r.follow || r.ours)
            && w.shots >= 2 && !after.ours && after.len > 0;
        if !tail {
            return;
        }
        let copy = balls(x, after.len as usize);
        let mate = w.records[(b.team ^ 2) as usize];
        let early = mate.n >= 0 && (!r.fresh || (r.front && sign(b.pos[2]) == sign(w.ball[2])));
        if after.follow && !after.middle && !early {
            self.repick[human] += 1;
        }
        if after.follow || after.next != after.len - 1 || copy[copy.len() - 1].bounces >= 2 {
            return;
        }
        let other = |p: &[f32; 4]| sign(b.side) != sign(p[2]);
        let descent = (|| {
            let mut up = false;
            for c in &copy {
                up |= c.bounces == 1 && !(c.vel[1] <= 0.0);
                if up && c.bounces == 1 && c.vel[1] < 0.0 {
                    return other(&c.pos);
                }
            }
            false
        })();
        let at = if copy[0].bounces == 0 {
            0
        } else if descent {
            1
        } else if copy.len() > 11 {
            2
        } else if other(&copy[copy.len() - 1].pos) {
            3
        } else {
            return;
        };
        self.short[at] += 1;
        if after.chase == r.chase {
            self.chase_kept += 1;
        }
    }
}

/// Which of give-way's comparisons beside a human decides (0 its own run under 2.5 and well ahead of the partner, 1
/// the partner's run under 2, 2 its own under 2.5, 3 the frames within 30, 4 the frames or no partner shot), as
/// `Rally::give_way` reads the two shot records.
fn give_way_test(b: &Body, mine: &Shot, theirs: &Shot) -> usize {
    use hst_sim::ps2::F;
    let n = |s: &Shot| if s.n >= 0 { s.n + s.stamp } else { s.n };
    if n(theirs) < 0 {
        return 4;
    }
    let run = |s: &Shot| {
        let (dz, dx) = ((F(s.vec[2]) - s.pos[2]).0, (F(s.vec[0]) - s.pos[0]).0);
        (F(dz) * dz).madd(dx, dx).sqrt().0
    };
    let (f4, f3) = (run(mine), run(theirs));
    if f4 < 2.5 && (F(1.0) + b.pos[2].abs()).0 < b.mate[2].abs() {
        0
    } else if f3 < 2.0 {
        1
    } else if f4 < 2.5 {
        2
    } else if n(theirs).wrapping_sub(n(mine)).wrapping_abs() < 30 {
        3
    } else {
        4
    }
}

/// P11k5: the doubles NET/BASE branches only a human side reaches, slot 3 with P1 driven by the pad (`HST_DRIVE=1`,
/// journal 2026-10-07-p11-ai 24). Every call of the new fixtures replays bit for bit; the branches are read from the
/// game's own exits across them and the earlier pad-driven ones (`ai_human_*`, replayed in
/// `human_record_matches_the_game`). `_lob`: P1 lobs (`HST_BTN=triangle`); `_fast`: the fast row 119 on every CPU
/// (`HST_AI=1:119:3,2:119:3,3:119:3`); `_base_lob`: lobs against row 121 everywhere; `_slice`: slices (`HST_BTN=cross`) against
/// the quickest-reacting rows (`HST_AI=1:119:3,2:121:3,3:119:3`: the human's partner reacts in 2..4 frames, before
/// the path reaches its second bounce, so it walks to the short landing); `_window10`/`_window14`: every CPU's path
/// window poked to 10/14 entries in rallies (`HST_WINDOW`, the game's is 180), so copies end before the second
/// bounce after the ball has bounced too, the walk's later targets (no unpoked copy is ever that short: the path
/// grows 15 entries a frame and reaches the second bounce before any reaction ends).
#[test]
fn vpad_rallies_net_and_base_match_the_game() {
    let new = ["ai_rally_vpad_lob.bin", "ai_rally_vpad_fast.bin", "ai_rally_vpad_base_lob.bin", "ai_rally_vpad_slice.bin", "ai_rally_vpad_window10.bin", "ai_rally_vpad_window14.bin"];
    for name in new {
        for (tag, what) in [(1, "receive"), (2, "NET"), (3, "BASE")] {
            let Some(n) = replay(name, tag) else { return eprintln!("{name} or disc missing, skipped") };
            eprintln!("{name} {what}: calls, per substate {n:?}");
        }
    }
    let old = ["ai_human_1p.bin", "ai_human_1p_b.bin", "ai_human_1p_c.bin", "ai_human_vpad.bin", "ai_human_vpad_b.bin", "ai_human_vpad_c.bin"];
    let (mut unpoked, mut all) = (NetBase::default(), NetBase::default());
    for name in new.iter().chain(&old) {
        let Ok(d) = std::fs::read(format!("{}/context/fixtures/{name}", root())) else { return eprintln!("{name} missing, skipped") };
        for (_, _, e, x) in calls(&d, &[2, 3]) {
            all.add(e, x);
            if !name.contains("window") {
                unpoked.add(e, x);
            }
        }
    }
    eprintln!("unpoked {unpoked:?}\nall {all:?}");
    assert!(unpoked.short[0] > 0 && all.short.iter().all(|&k| k > 0) && all.chase_kept > 0, "{all:?}");
    assert!(unpoked.repick.iter().all(|&k| k > 0) && unpoked.defer[1] > 0 && unpoked.wait.iter().all(|&k| k > 0));
    // the partner's run under 2 m (the human standing near the landing as the AI weighs it) came up once, in a
    // poked-window match; the window doesn't feed give-way's comparisons
    assert!([0, 2, 3, 4].iter().all(|&k| unpoked.give_way[k] > 0) && all.give_way[1] > 0, "{all:?}");
}

#[test]
fn net_and_base_match_the_game() {
    for (tag, what) in [(2, "NET"), (3, "BASE")] {
        for name in ["ai_net_s05.bin", "ai_rally_s05_weak.bin"] {
            let Some(n) = replay(name, tag) else { return eprintln!("fixture or disc missing, skipped") };
            eprintln!("{name} {what}: calls, per substate {n:?}");
        }
    }
}

/// B34: a human player's shot record (tag 4) in a 1P doubles match (`context/recordings/1p3goodcpus2.p2m2`
/// played to a save at vsync 7930, 8380 and 8960, then hooked with the human idle: `context/b34/run2.sh`), and its
/// CPU partner beside it (tags 1-3).
#[test]
fn human_record_matches_the_game() {
    // P11k7: ai_human_vpad.bin is slot 3 with P1 driven by the pad (`context/p11k7/drive.sh 3 1800`; _b slot 4, 3000; _c the timed pad script `context/p11k7/run.sh 1800 out context/p11k7/c_pad.txt`)
    for name in ["ai_human_1p.bin", "ai_human_1p_b.bin", "ai_human_1p_c.bin", "ai_human_vpad.bin", "ai_human_vpad_b.bin", "ai_human_vpad_c.bin"] {
        for (tag, what) in [(4, "human"), (1, "receive"), (2, "NET"), (3, "BASE")] {
            let Some(n) = replay(name, tag) else { return eprintln!("fixture or disc missing, skipped") };
            eprintln!("{name} {what}: calls, per substate {n:?}");
        }
    }
}

#[test]
fn singles_receive_matches_the_game() {
    for name in ["ai_rally_singles.bin", "ai_rally_singles_long.bin", "ai_rally_singles_rows.bin", "ai_rally_singles_spec.bin", "ai_rally_singles_base.bin"] {
        let Some(n) = replay(name, 1) else { return eprintln!("fixture or disc missing, skipped") };
        eprintln!("{name}: calls, per substate {n:?}");
    }
}

#[test]
fn singles_net_and_base_match_the_game() {
    for name in ["ai_rally_singles_net.bin", "ai_rally_singles_rows.bin", "ai_rally_singles_spec.bin", "ai_rally_singles_base.bin"] {
        for (tag, what) in [(2, "NET"), (3, "BASE")] {
            let Some(n) = replay(name, tag) else { return eprintln!("fixture or disc missing, skipped") };
            eprintln!("{name} {what}: calls, per substate {n:?}");
        }
    }
}

/// B57: CPUs let a ball that will land out go by. `ai_let_go_s05.bin` is slot 5 with the line-call margin pulled 2 m
/// inside every line (`HST_LINE=-2`, so near-line landings are called out) and every CPU row's let-go margin at -3 m
/// (`HST_MARGIN=-3`), so out calls whose distance to the nearest true line is negative still pass the check: the BASE
/// CPU in substate 2 sets its hold and stands, as the game did.
#[test]
fn cpus_let_out_balls_go() {
    let name = "ai_let_go_s05.bin";
    for tag in [1, 2, 3] {
        let Some(n) = replay(name, tag) else { return eprintln!("fixture or disc missing, skipped") };
        eprintln!("{name} tag {tag}: calls, per substate {n:?}");
    }
    let d = std::fs::read(format!("{}/context/fixtures/{name}", root())).unwrap();
    let (mut o, mut holds) = (8 + MT_SIZE, 0);
    while o < d.len() {
        let e = &d[o..o + u32_at(&d, o + 4) as usize];
        let x = &d[o + e.len()..o + e.len() + u32_at(&d, o + e.len() + 4) as usize];
        o += e.len() + x.len();
        holds += (u32_at(e, 0) != 1 && e[A + 0xbc] == 0 && x[A + 0xbc] == 1 && e[0x56d] == 2) as usize;
    }
    assert!(holds >= 2, "{holds} let-go holds");
}

/// The match sample(s) a call's tick lies in: every `match_s05.bin` sample showing the call's game frame, with the
/// one before and the one after (a sample is a vsync snapshot; a frame can span two, and players earlier in the
/// update order have already moved when a later one's AI runs).
fn tick_window<'a>(m: &[hst_sim::replay::Frame<'a>], e: &[u8]) -> &'a [u8] {
    let (v, frame) = (u32_at(e, 8) as usize, i32_at(e, 0x34));
    let first = m[0].vsync() as usize;
    let near = v.saturating_sub(first + 6)..(v + 6 - first).min(m.len());
    let shows: Vec<usize> = near.filter(|&i| i32_at(m[i].gm(), 0x58) == frame).collect();
    assert!(!shows.is_empty(), "v{v}: game frame {frame} not in match_s05.bin near its vsync");
    let (lo, hi) = (shows[0] - 1, shows[shows.len() - 1] + 1);
    // the samples are contiguous in the file
    let s = m[lo].0.as_ptr_range().start;
    let len = m[hi].0.as_ptr_range().end as usize - s as usize;
    // SAFETY: m's frames are consecutive slices of one buffer
    unsafe { std::slice::from_raw_parts(s, len) }
}

/// The calls of a fixture with one of `tags`: (vsync, file order, tag, entry, exit).
fn calls<'a>(d: &'a [u8], tags: &[u32]) -> Vec<(u32, usize, &'a [u8], &'a [u8])> {
    let (mut o, mut v) = (8 + MT_SIZE, Vec::new());
    while o < d.len() {
        let e = &d[o..o + u32_at(d, o + 4) as usize];
        o += e.len();
        let x = &d[o..o + u32_at(d, o + 4) as usize];
        o += x.len();
        if tags.contains(&u32_at(e, 0)) {
            v.push((u32_at(e, 8), v.len(), e, x));
        }
    }
    v
}

/// T2: the rally AI's state replayed tick by tick through a stretch of the slot-5 doubles match (vsync 7592..10612),
/// every receive, NET and BASE call of all four CPUs, each tick tied to `match_s05.bin`. The receive calls come from
/// `ai_rally_s05_all.bin`, NET and BASE from `ai_net_s05.bin`: the same match (slot 5 is bot-only), recorded twice
/// because `_all` predates the recorder's NET/BASE regions (P11k2) and is only good for the receive.
///
/// Unlike `replay`, which seeds every call afresh from the recording and compares the fields it was written for, each
/// CPU's AI block here is the port's own from its previous call; only bytes the game changed between the two calls
/// (outside the routines: the dispatcher, the point flow) are taken from the recording. After every call the whole
/// 0x280-byte block, as far as `Rally` models it (`bytes_all`), must be the game's. Each call's world is checked to be
/// the match's at that tick: its game frame is in `match_s05.bin`, and the score/rotation globals, the phase and all
/// four players' positions it read equal a sample of that tick bit for bit.
#[test]
fn rally_state_tick_by_tick_on_match_s05() {
    let dir = format!("{}/context/fixtures", root());
    let read = |f: &str| std::fs::read(format!("{dir}/{f}")).ok();
    let (Some(rd), Some(nd), Some(md), Some(table)) = (read("ai_rally_s05_all.bin"), read("ai_net_s05.bin"), read("match_s05.bin"), table()) else {
        return eprintln!("skip: ai_rally_s05_all.bin, ai_net_s05.bin, match_s05.bin or the disc missing");
    };
    let m = hst_sim::replay::frames_live(&md);
    let sample = hst_sim::replay::SAMPLE_LIVE;
    let mts = [Mt::of(&rd[8..8 + MT_SIZE]), Mt::of(&nd[8..8 + MT_SIZE])];
    let (rc, nc) = (calls(&rd, &[1]), calls(&nd, &[2, 3]));
    let end = nc.last().unwrap().0;
    let mut ticks: Vec<(u32, usize, usize, &[u8], &[u8])> = rc.into_iter().filter(|c| c.0 <= end).map(|(v, k, e, x)| (v, 0, k, e, x)).collect();
    ticks.extend(nc.into_iter().map(|(v, k, e, x)| (v, 1, k, e, x)));
    ticks.sort_by_key(|t| (t.0, t.1, t.2));
    // per AI object: the game's exit block of its last call, and the port's
    let mut last: std::collections::HashMap<u32, (Vec<u8>, Vec<u8>)> = Default::default();
    let (mut n, mut carried, mut outside, mut fails, mut tags) = (0, 0, 0, 0, [0usize; 4]);
    for (_, src, _, e, x) in ticks {
        let (tag, ai) = (u32_at(e, 0), u32_at(e, 0x10));
        n += 1;
        tags[tag as usize] += 1;
        let at = format!("v{} #{n} ai {ai:#x} tag {tag}", u32_at(e, 8));
        let mut errs = Vec::new();

        // the tick is the match's
        let win = tick_window(&m, e);
        let samples: Vec<hst_sim::replay::Frame> = win.chunks_exact(sample).map(hst_sim::replay::Frame).collect();
        let globals: Vec<i32> = (0..12).map(|k| i32_at(e, 0x40 + 4 * k)).collect();
        if !samples.iter().any(|f| f.gm()[0x55] == e[0x38] && (0..12).all(|k| f.global(0x423040 + 4 * k) == globals[k])) {
            errs.push(format!("globals {globals:?} / phase {} in no sample of the tick", e[0x38]));
        }
        for (who, q) in [("own", 0x490), ("partner", 0x4f0), ("opponent", 0x500), ("opponent", 0x510)] {
            let p: [u32; 3] = std::array::from_fn(|k| u32_at(e, q + 4 * k));
            let seen = samples.iter().any(|f| (0..4).any(|i| (0..3).all(|k| f.player_f32(i, 0x3d70 + 4 * k).to_bits() == p[k])));
            if !seen {
                errs.push(format!("{who} position {:?} in no sample of the tick", p.map(f32::from_bits)));
            }
        }

        // the AI block: the port's own, with what the game changed between calls
        let mut entry = e.to_vec();
        if let Some((game, port)) = last.get(&ai) {
            carried += 1;
            for k in 0..0x280 {
                if e[A + k] == game[k] {
                    entry[A + k] = port[k];
                } else {
                    outside += 1;
                }
            }
        }
        let row = &table[((u32_at(e, A + 0xc) - TABLE) / RECORD) as usize];
        let (r, call_errs) = call("", &entry, x, &mts[src], row, tag, false);
        errs.extend(call_errs);
        let got = bytes_all(&entry, &r, false);
        let want = &x[A..A + 0x280];
        let diff: Vec<String> = (0..0x280 / 4)
            .filter(|k| got[4 * k..4 * k + 4] != want[4 * k..4 * k + 4])
            .map(|k| format!("+{:#x} {:08x} vs {:08x}", 4 * k, u32_at(&got, 4 * k), u32_at(want, 4 * k)))
            .collect();
        if !diff.is_empty() {
            errs.push(format!("block {}", diff.join(", ")));
        }
        last.insert(ai, (want.to_vec(), got));
        if !errs.is_empty() {
            fails += 1;
            if fails <= 15 {
                eprintln!("{at}: {}", errs.join("; "));
            }
        }
    }
    eprintln!("{n} calls (receive / NET / BASE {:?}), {carried} on carried state, {outside} bytes changed between calls", &tags[1..]);
    assert_eq!(last.len(), 4, "every CPU's calls");
    assert!(tags[1..].iter().all(|&k| k > 500), "{tags:?}");
    assert_eq!(fails, 0, "{fails} of {n} ticks differ");
}

