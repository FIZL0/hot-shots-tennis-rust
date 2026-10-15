//! Hot Shots Tennis on the interpreter: the user's disc (checked by `hst_data::exe::Game`), RAM dumps from
//! `context/ram/` and the function names in `research/oracle.txt`. Everything game-specific in this crate is here.

use crate::{Ee, RAM};
use std::path::{Path, PathBuf};

/// The repo root (for `context/` and `research/`).
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Function addresses by name from `research/oracle.txt` (`name hexaddr` per line, `#` comments).
pub fn addr(name: &str) -> u32 {
    let t = std::fs::read_to_string(root().join("research/oracle.txt")).expect("research/oracle.txt");
    t.lines()
        .filter_map(|l| l.split('#').next()?.split_once(char::is_whitespace))
        .find(|(n, _)| *n == name)
        .map(|(_, a)| u32::from_str_radix(a.trim().trim_start_matches("0x"), 16).unwrap())
        .unwrap_or_else(|| panic!("{name} not in research/oracle.txt"))
}

/// `None` for a missing disc file or dump, said on stderr so a skipped test shows; with `HST_REQUIRE_DATA=1` (a run
/// that must check against the game) it fails instead of passing quietly.
fn missing<T>(what: &Path) -> Option<T> {
    assert!(std::env::var_os("HST_REQUIRE_DATA").is_none(), "HST_REQUIRE_DATA: no {}", what.display());
    eprintln!("skip: no {}", what.display());
    None
}

/// The boot ELF and the GAME overlay (bytes, load address) from `context/iso/`, the disc checked to be the supported
/// one; `None` without a disc.
pub fn program() -> Option<(Vec<u8>, Vec<u8>, u32)> {
    let iso = root().join("context/iso");
    let read = |f: &str| std::fs::read(iso.join(f)).ok().or_else(|| missing(&iso.join(f)));
    let (cnf, boot, game) = (read("SYSTEM.CNF")?, read("SCUS_976.10")?, read("ZZBIN/GAME.BIN")?);
    hst_data::exe::Game::new(&cnf, &game).expect("supported disc");
    // the overlay header (checked just above) names its own load address
    let load = u32::from_le_bytes(game[8..12].try_into().unwrap());
    Some((boot, game, load))
}

/// The game's machines: `use hst_oracle::HstEe` to get `Ee::game()` and `Ee::from_ram(name)`.
pub trait HstEe: Sized {
    /// The boot program and the GAME overlay as loaded in a match, from `context/iso/`; `None` without a disc.
    fn game() -> Option<Self>;
    /// A 32 MB EE RAM dump (`context/ram/<name>`) as the machine's memory, its GAME overlay checked against the
    /// disc's; `None` when either is missing.
    fn from_ram(name: &str) -> Option<Self>;
}

impl HstEe for Ee {
    fn game() -> Option<Self> {
        let (boot, game, load) = program()?;
        Some(Self::from_program(RAM, &boot, &[(load, &game)]).expect("boot ELF"))
    }

    fn from_ram(name: &str) -> Option<Self> {
        let path = root().join("context/ram").join(name);
        let ram = std::fs::read(&path).ok().or_else(|| missing(&path))?;
        let (_, game, load) = program()?;
        let (at, n) = (load as usize + 0x100, 0x1000); // code, past the overlay header
        assert_eq!(ram[at..at + n], game[0x100..0x100 + n], "{name} holds a different GAME overlay");
        Some(Self::with_ram(ram))
    }
}
