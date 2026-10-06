//! The shot parameter table built from the disc must equal the game's own runtime table (captured from a
//! save state's RAM: `context/fixtures/shot_params_ram.bin`). Needs the extracted disc under `context/iso`.

use hst_data::exe::Game;
use hst_sim::params::ShotParams;

#[test]
fn built_table_equals_the_games_runtime_table() {
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let (Ok(cnf), Ok(bin), Ok(ram)) = (
        std::fs::read(format!("{ctx}/iso/SYSTEM.CNF")),
        std::fs::read(format!("{ctx}/iso/ZZBIN/GAME.BIN")),
        std::fs::read(format!("{ctx}/fixtures/shot_params_ram.bin")),
    ) else {
        eprintln!("fixture missing, skipped");
        return;
    };
    let src = Game::new(&cnf, &bin).unwrap().shot_params();
    let built = ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
    let want: Vec<f32> = ram.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
    let mut checked = 0;
    for class in 0..4 {
        for kind in 0..src.kinds[class] as usize {
            for r in 0..hst_sim::params::RECORDS {
                let got = built.record(class, kind, r);
                let o = class * 0x451 + kind * 0xdd + r * 0xd;
                for f in 0..13 {
                    assert_eq!(got[f].to_bits(), want[o + f].to_bits(), "class {class} kind {kind} record {r} field {f}: {} vs {}", got[f], want[o + f]);
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, (4 + 5 + 5 + 2) * 17);
}
