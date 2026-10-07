//! The AI tuning table read from the disc's AIParam.csv must equal the game's own copy in RAM (`context/ram/s05.bin`,
//! slot 5: a doubles bot match), and each bot's row and level must be the ones the game gave it.

use hst_data::{iso::Iso, xb::Archive};
use hst_sim::ai::{AiParams, Choice, ROWS, menu_row};

const TABLE: usize = 0x3174c0;
const RECORD: usize = 0x118;

fn load() -> Option<(Vec<u8>, Vec<u8>)> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let ram = std::fs::read(format!("{root}/context/ram/s05.bin")).ok()?;
    let mut iso = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")).ok()?;
    let data = iso.read("PCDATA/PCDATA.XB").ok()?;
    let arc = Archive::parse(&data).ok()?;
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("aiparam.csv"))?;
    Some((ram, arc.read(e).ok()?))
}

fn word(ram: &[u8], a: usize) -> usize {
    u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize
}

#[test]
fn table_equals_ram() {
    let Some((ram, csv)) = load() else { return eprintln!("RAM dump or disc missing, skipped") };
    let table = AiParams::table(&csv);
    for (r, a) in table.iter().enumerate() {
        let want = &ram[TABLE + r * RECORD..TABLE + (r + 1) * RECORD];
        assert_eq!(a.bytes(), want, "row {r}: {a:?}");
    }
    assert_eq!(table.len(), ROWS);
}

#[test]
fn bots_get_their_rows() {
    let Some((ram, _)) = load() else { return eprintln!("RAM dump or disc missing, skipped") };
    let gm = word(&ram, 0x422f80);
    for i in 0..4 {
        // the menu's player slot: character, outfit, …, setup word at +8
        let slot = 0x2ef7f4 + 12 * i;
        let (character, outfit) = (ram[slot], ram[slot + 1]);
        let setup = word(&ram, slot + 8) as u32;
        assert_eq!(setup & 0xff, menu_row(character, outfit) as u32, "player {i} setup row");
        // the AI object (player +0x80): row pointer, level, strategy byte, reach
        let ai = word(&ram, word(&ram, gm + 0xa8 + 4 * i) + 0x80);
        let c = Choice::new(setup, character, true);
        assert_eq!(TABLE + c.row * RECORD, word(&ram, ai + 12), "player {i} row");
        assert_eq!(c.level, ram[ai + 16], "player {i} level");
        assert_eq!(c.strategy, ram[ai + 17], "player {i} strategy");
        assert_eq!(c.reach.to_bits(), word(&ram, ai + 20) as u32, "player {i} reach");
    }
}

#[test]
fn rows_by_outfit_and_mode() {
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map(|o| menu_row(6, o)), [6, 6, 6, 6, 48, 20, 20, 20, 20, 48]);
    assert_eq!(Choice::new(13, 13, false).row, 13);
    assert_eq!(Choice::new(13, 13, true).row, 97);
    // a forced level puts the character in that block; rows clamp to the singles half first
    assert_eq!(Choice::new(0x200 | 50, 4, false), Choice { row: 18, level: 1, strategy: 0, reach: 2.85 });
    assert_eq!(Choice::new(160, 0, true).row, 167);
}
