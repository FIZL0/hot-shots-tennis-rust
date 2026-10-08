//! P1's rally aims (`context/fixtures/aim_singles.bin`, `aim_doubles.bin`; tools/record_aim.py, skipped
//! when missing): every aim `shot::aim` gives from the stick, the hitter and the incoming shot's record is the
//! game's +0x3e90 bit for bit, sweet-spot corner shots and both incoming-slice scales
//! (each fixture ends with AIM_INCOMING=1 aims) included.

use hst_sim::player::pad_dir;
use hst_sim::replay::{Frame, SAMPLE_LIVE};
use hst_sim::shot::{AimStats, Hitter, aim};

const EXTRA: usize = 0x80; // P1 +0x12b0..+0x1320, then the last shot's record (P1 +0x1400 → +0x1b0..+0x1c0)

fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

#[test]
fn human_aims() {
    for (name, players) in [("aim_singles.bin", 2), ("aim_doubles.bin", 4)] {
        let path = format!("{}/../../context/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        let Ok(data) = std::fs::read(&path) else {
            eprintln!("skipping {name}: no recording");
            continue;
        };
        let (mut n, mut corners, mut bad, mut slices) = (0, 0, Vec::new(), [0; 2]); // slices: incoming off-sweet, sweet
        for pair in data.chunks_exact(2 * (SAMPLE_LIVE + EXTRA)) {
            let (pre, cur) = (Frame(&pair[..SAMPLE_LIVE]), Frame(&pair[SAMPLE_LIVE + EXTRA..2 * SAMPLE_LIVE + EXTRA]));
            let x = &pair[SAMPLE_LIVE..SAMPLE_LIVE + EXTRA];
            assert_eq!(cur.global(0x422fa4), players, "{name}: player count");
            let pi = |o: usize| cur.player_f32(0, o).to_bits() as i32;
            let end = f32::from_bits(i32_at(x, 0) as u32);
            let stats = AimStats {
                con: [i32_at(x, 0x54), i32_at(x, 0x58), i32_at(x, 0x5c)],
                body: [i32_at(x, 0x60), i32_at(x, 0x64)],
                rising: i32_at(x, 0x68),
                under: cur.player_f32(0, 0x13b4),
            };
            let branch = pre.player_f32(0, 0x3ec0).to_bits().to_le_bytes()[1];
            let kind = match pi(0x3ee4) {
                0x10 => 4,
                4 => 3,
                8 => 2,
                2 => 1,
                _ => 0,
            };
            let h = Hitter {
                end,
                branch,
                kind,
                offset: pi(0x3fa0),
                body: pi(0x3f50) & 4 != 0,
                from: [cur.player_f32(0, 0x3ef0), cur.player_f32(0, 0x3ef8)],
                height: cur.player_f32(0, 0x3f44),
            };
            // the record of the last shot (the one being struck): the hit routine writes this one after the aim
            let rec = &x[0x70..]; // hitter, branch, kind, sweet byte
            let incoming = (pre.global(0x423060) > 0 && i32_at(rec, 4) < 4 && i32_at(rec, 8) == 1).then_some(rec[12] != 0);
            // the camera sits behind P1: from the +z end the run direction turns by π
            let pad = cur.pad(0);
            let [sx, sz] = pad_dir(pad.buttons, pad.lx, pad.ly, cur.gm()[0x55]);
            let stick = if end < 0.0 { [-sx, -sz] } else { [sx, sz] };
            if let Some(sw) = incoming {
                slices[sw as usize] += 1;
            }
            let got = aim(&h, &stats, stick, players == 4, false, incoming);
            let want = [cur.player_f32(0, 0x3e90), 0.0, cur.player_f32(0, 0x3e98)];
            let sweet = h.offset.abs() < 2 && !h.body && branch != 3;
            if sweet && sx != 0.0 && sx.abs() == sz.abs() {
                corners += 1;
            }
            if got[0].to_bits() != want[0].to_bits() || got[2].to_bits() != want[2].to_bits() {
                bad.push(format!("vsync {}: {h:?} stick {stick:?} incoming {incoming:?}: {got:?} vs {want:?}", cur.vsync()));
            }
            if std::env::var_os("AIM_LIST").is_some() {
                eprintln!("vsync {}: {h:?} stick {stick:?} incoming {incoming:?} (shots {} record {} {} {}) -> {want:?}", cur.vsync(), pre.global(0x423060), i32_at(rec, 4), i32_at(rec, 8), rec[12]);
            }
            n += 1;
        }
        eprintln!("{name}: {n} aims, {corners} sweet full-diagonal, incoming slices {slices:?} (off-sweet, sweet)");
        assert!(bad.is_empty(), "{name}: {} of {n} aims differ:\n{}", bad.len(), bad.join("\n"));
        assert!(n > 0 && corners > 0, "{name}: no sweet corner aims recorded");
        // AIM_INCOMING=1 recordings appended: both `incoming` scales are checked in each mode
        assert!(slices[0] > 0 && slices[1] > 0, "{name}: incoming slices {slices:?} (off-sweet, sweet): both needed");
    }
}

/// P1's serve aims (`context/fixtures/aim_serve.bin`, tools/record_aim.py AIM_SERVE=1, the same pairs): the aim
/// `serve::target` gives through the rally aim's core is the game's +0x3e90 bit for bit, and so are the strong
/// toss's mistiming nudges (+0x3f10 sideways, +0x3f18 depth, by the character's skill level) and the centred
/// stick's ±5/±10 (+0x3ed4). The game's coin flips are read back from what they gave.
#[test]
fn human_serve_aims() {
    use hst_sim::serve::{Miss, ServeData, Toss, miss_of, target};
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let (Ok(data), Ok(cnf), Ok(bin)) = (
        std::fs::read(format!("{ctx}/fixtures/aim_serve.bin")),
        std::fs::read(format!("{ctx}/iso/SYSTEM.CNF")),
        std::fs::read(format!("{ctx}/iso/ZZBIN/GAME.BIN")),
    ) else {
        return eprintln!("skipping: no aim_serve.bin recording or extracted disc");
    };
    let rows = hst_data::exe::Game::new(&cnf, &bin).unwrap().serve_miss();
    let (mut n, mut nudged, mut bad) = (0, 0, Vec::new());
    for pair in data.chunks_exact(2 * (SAMPLE_LIVE + EXTRA)) {
        let (pre, cur) = (Frame(&pair[..SAMPLE_LIVE]), Frame(&pair[SAMPLE_LIVE + EXTRA..2 * SAMPLE_LIVE + EXTRA]));
        let x = &pair[SAMPLE_LIVE..SAMPLE_LIVE + EXTRA];
        let pi = |o: usize| cur.player_f32(0, o).to_bits() as i32;
        let (end, character) = (f32::from_bits(i32_at(x, 0) as u32), i32_at(x, 0xc) as usize);
        assert_eq!(miss_of(rows, character), rows[i32_at(x, 0x20) as usize], "character {character}'s skill level");
        let d = ServeData {
            over: [0.0; 3],
            under: [0.0; 3],
            strong_grades: Vec::new(),
            weak_grades: Vec::new(),
            hand_over: [0.0; 4],
            hand_under: [0.0; 4],
            racket_over: [[0.0; 4]; 4],
            racket_under: [[0.0; 4]; 4],
            miss: miss_of(rows, character),
            strong_bias: Vec::new(),
            weak_bias: Vec::new(),
            reach: [0; 2],
            short_miss: 0.0,
            power: 0,
            low_power: 0,
            max_angle: i32_at(x, 0x5c) as f32,
        };
        let toss = match pi(0x3ea0) {
            1 => Toss::Strong,
            4 => Toss::Under,
            _ => Toss::Weak,
        };
        let (offset, grade) = (pi(0x3fa0), (pi(0x3ee8) & 0xff) as u8);
        let server = [cur.player_f32(0, 0x3ef0), 0.0, cur.player_f32(0, 0x3ef8)];
        let pad = cur.pad(0);
        let [sx, sz] = pad_dir(pad.buttons, pad.lx, pad.ly, cur.gm()[0x55]);
        let stick = if end < 0.0 { [-sx, -sz] } else { [sx, sz] };
        let want_miss = Miss { side: cur.player_f32(0, 0x3f10), depth: cur.player_f32(0, 0x3f18), nudge: pi(0x3ed4) };
        let rand = [want_miss.nudge.abs() == 5, want_miss.nudge > 0, want_miss.side < 0.0];
        let side = cur.global(0x423050);
        let (got, miss) = target(&d, toss, offset, grade, server, end, side, cur.global(0x422fa4) == 4, stick, rand);
        let want = [cur.player_f32(0, 0x3e90), 0.0, cur.player_f32(0, 0x3e98)];
        let bits = |v: f32| v.to_bits();
        if bits(got[0]) != bits(want[0]) || bits(got[2]) != bits(want[2]) || bits(miss.side) != bits(want_miss.side) || bits(miss.depth) != bits(want_miss.depth) || miss.nudge != want_miss.nudge {
            bad.push(format!("vsync {}: {toss:?} offset {offset} grade {grade} side {side} stick {stick:?} from {server:?}: {got:?} {miss:?} vs {want:?} {want_miss:?}", cur.vsync()));
        }
        nudged += (want_miss.side != 0.0 || want_miss.depth != 0.0) as usize;
        assert_eq!(pre.player_f32(0, 0x3ec0).to_bits().to_le_bytes()[1], 0, "the serve aim's branch");
        n += 1;
    }
    eprintln!("aim_serve.bin: {n} serve aims, {nudged} with mistiming nudges");
    assert!(bad.is_empty(), "{} of {n} serve aims differ:\n{}", bad.len(), bad.join("\n"));
    assert!(n > 0 && nudged > 0, "no mistimed strong-toss serve aims recorded");
}
