//! P1's rally aims (`context/fixtures/aim_singles.bin`, `aim_doubles.bin`; tools/record_aim.py, skipped
//! when missing): every aim `shot::aim` gives from the stick, the hitter and the incoming shot's record is the
//! game's +0x3e90 bit for bit, sweet-spot corner shots included.

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
        let (mut n, mut corners, mut bad) = (0, 0, Vec::new());
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
        eprintln!("{name}: {n} aims, {corners} sweet full-diagonal");
        assert!(bad.is_empty(), "{name}: {} of {n} aims differ:\n{}", bad.len(), bad.join("\n"));
        assert!(n > 0 && corners > 0, "{name}: no sweet corner aims recorded");
    }
}
