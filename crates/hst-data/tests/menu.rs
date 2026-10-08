//! The MENU overlay's character grades, as the original's character select shows them (Ashley, Jun and Kaito
//! checked on screen; the rest read from the same table). Skips without the ISO.

use hst_data::{exe::Menu, iso::Iso};

#[test]
fn character_grades() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("no ISO, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/MENU.BIN").unwrap());
    let g = Menu::new(&cnf, &bin).unwrap().grades();
    let letters = |c: usize| g[c].map(|v| b"ABCDEF"[5 - v as usize] as char).iter().collect::<String>();
    assert_eq!(letters(0), "EFEAD"); // Ashley
    assert_eq!(letters(2), "EFCBC"); // Jun
    assert_eq!(letters(3), "DDCCC"); // Kaito
    assert_eq!(letters(13), "DDDCC"); // Suzuki
}

