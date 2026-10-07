//! Every hit-effect model in `AZUMA/C_EFF/EFFCT.XB0` parses: models, materials, hierarchical `.ANI` byte-exact
//! to the end, `.MOR`/`.MTA`/`.UVA` key tracks. Skips without the ISO.

use hst_data::{ani, iso::Iso, mdl, mor, mtl, xb::Archive};

#[test]
fn every_effect_parses() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").unwrap();
    let arc = Archive::parse(&data).unwrap();
    let get = |name: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&name.to_ascii_lowercase())).map(|e| arc.read(e).unwrap());
    let mut n = 0;
    for e in &arc.entries {
        let lower = e.name.to_ascii_lowercase();
        let d = arc.read(e).unwrap();
        let ok = |r: Result<(), hst_data::xb::Error>| r.unwrap_or_else(|err| panic!("{}: {err}", e.name));
        match lower.rsplit_once('.').map(|x| x.1) {
            Some("ani") => {
                let a = ani::parse(&d).unwrap_or_else(|err| panic!("{}: {err}", e.name));
                assert_eq!(a.ticks_per_frame, 80, "{}", e.name);
            }
            Some("mor") => ok(mor::parse(&d, 1).map(drop)),
            Some("mta") => ok(mor::parse(&d, 1).map(drop)),
            Some("uva") => ok(mor::parse(&d, 4).map(drop)),
            Some("mdl") => {
                ok(mdl::parse(&d).map(drop));
                let stem = e.name[..e.name.len() - 4].replace('\\', "/");
                let m = mtl::parse(&get(&format!("{stem}.MTL")).unwrap(), get(&format!("{stem}.MTI")).as_deref()).unwrap();
                assert!(m.materials.iter().all(|m| !m.name.is_empty()), "{}", e.name);
            }
            _ => continue,
        }
        n += 1;
    }
    eprintln!("{n} effect files");
    assert!(n > 90);
    let top = ani::parse(&get("yumoto/impact_top_a.ANI").unwrap()).unwrap();
    assert_eq!(top.tracks.iter().filter(|t| t.parent == Some(0)).count(), 8);
    let m = mtl::parse(&get("yumoto/impact_top_a.MTL").unwrap(), get("yumoto/impact_top_a.MTI").as_deref()).unwrap();
    assert!(m.materials.iter().any(|m| m.blend() == mtl::Blend::Add));
}
