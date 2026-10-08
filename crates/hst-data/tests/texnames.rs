//! PCSX2 replacement names computed from the disc match the user's pack (`mods/texture-replacements/`, dumped by PCSX2) for
//! court 05 and the HUD: every pack file whose texels belong to one of their textures is found by its full name.
//! Skips without the ISO or the pack.

use hst_data::{iso::Iso, texhash};
use std::collections::HashSet;

#[test]
fn court_and_hud_match_the_pack() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(mut iso) = Iso::open(format!("{root}Hot Shots Tennis (USA).iso")) else {
        return eprintln!("no ISO, skipped");
    };
    let Ok(dir) = std::fs::read_dir(format!("{root}mods/texture-replacements")) else {
        return eprintln!("no mods/texture-replacements/, skipped");
    };
    let pack: HashSet<String> = dir
        .filter_map(|e| {
            e.ok()?
                .file_name()
                .into_string()
                .ok()?
                .strip_suffix(".png")
                .map(str::to_owned)
        })
        .collect();
    let texels: HashSet<&str> = pack.iter().map(|n| n.split('-').next().unwrap()).collect();
    let (mut court, mut hud, mut missed) = (0, 0, vec![]);
    texhash::disc_textures(&mut iso, |t| {
        let is_hud = t
            .path
            .starts_with("AZUMA/INPANE/INPANE/data/azuma/inpane/mtl/");
        if !(t.path.starts_with("COURT/05/") || is_hud) {
            return;
        }
        if t.names.iter().any(|n| pack.contains(n)) {
            *if is_hud { &mut hud } else { &mut court } += 1;
        } else if t
            .names
            .iter()
            .any(|n| texels.contains(n.split('-').next().unwrap()))
        {
            missed.push(t.path);
        }
    });
    eprintln!("court 05: {court}, HUD: {hud} textures in the pack");
    // the one court-05 texture the game re-palettes at runtime: same texels, another CLUT
    missed.retain(|p| !p.ends_with("hole/wim_h01_s1111.22"));
    assert!(
        missed.is_empty(),
        "texels in the pack but the name differs: {missed:?}"
    );
    assert!(court >= 160 && hud >= 43, "court 05 {court}, HUD {hud}");
}
