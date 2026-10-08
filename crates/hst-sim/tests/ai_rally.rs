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
    let singles = name.contains("singles");
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
        let mut r = rally(e, singles);
        if name == "ai_human_vpad_b.bin" && tag == 2 && (9760..=9764).contains(&u32_at(e, 8)) {
            // recorded before the recorder took the stand scratch last in its entry stub: a vsync during the stub
            // zeroed it after it was taken (as on m3c v6843 / m3d v7900, P11k8), so the game read 0 here
            r.stash = [0.0; 4];
        }
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
