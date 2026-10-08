//! The user's replacements in `mods/texture-replacements/` (upscaled `--dump-textures` output, `<path>-<key>.png`)
//! find their disc textures: every key-named file's key is a disc texture's key.
//! Skips without the ISO or with no key-named files.

use hst_data::{iso::Iso, texhash};
use std::collections::HashSet;

#[test]
fn replacements_match_disc_keys() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(mut iso) = Iso::open(format!("{root}Hot Shots Tennis (USA).iso")) else {
        return eprintln!("no ISO, skipped");
    };
    let mods = texhash::Overrides::scan_mods(root.as_ref());
    // a PCSX2 pack in the same folder parses to short keys; only ours are checked
    let ours: Vec<&String> = mods.keys().filter(|k| k.len() >= 12).collect();
    if ours.is_empty() {
        return eprintln!("no key-named files in mods/texture-replacements/, skipped");
    }
    let mut disc = HashSet::new();
    texhash::disc_textures(&mut iso, |t| {
        disc.insert(t.key);
    });
    let unknown: Vec<_> = ours.iter().filter(|k| !disc.contains(**k)).collect();
    eprintln!("{} key-named replacements, {} unknown", ours.len(), unknown.len());
    assert!(unknown.is_empty(), "no disc texture has these keys: {unknown:?}");
}
