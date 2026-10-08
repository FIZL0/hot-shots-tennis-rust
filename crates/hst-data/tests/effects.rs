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

/// Court 10's wind (slot 5 blew from 315° at 2) and cloud count (9 in doubles as in RAM, 11 in singles).
#[test]
fn court10_wind_and_clouds() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = hst_data::exe::Game::new(&cnf, &bin).unwrap();
    assert_eq!(game.wind(10), (vec![315.0, 0.0, 45.0], 2.0));
    // weathers (P17i): court 10 gusts 2% kind 2, cloudy 5% for 2..3 games, no rain; court 11 rains heavy; court 7 calm
    assert_eq!(game.gusts(10), (2, 2));
    assert_eq!(game.weather_odds(10), [5, 2, 3, 0, 1, 3, 0, 1, 0]);
    assert_eq!(game.weather_odds(11), [20, 2, 3, 30, 2, 4, 0, 0, 1]);
    assert_eq!(game.wind(7), (vec![0.0], 0.0));
    assert_eq!(game.weather_looks()[3], [1.0, 0.7, 0.6, 40.0, 150.0, 0.65]);
    let mut find = |xb: &str, suffix: &str| {
        let data = iso.read(xb).unwrap();
        let arc = Archive::parse(&data).unwrap();
        arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix)).unwrap()).unwrap()
    };
    let (envir, hole) = (find("COURT/10/CMN.XB", "envir_c10.dat"), find("COURT/10/GRD01.XB", "envir_c10_h01.dat"));
    assert_eq!(hst_data::layout::cloud_count(&envir, &hole, 0), Some(9));
    assert_eq!(hst_data::layout::cloud_count(&envir, &hole, 1), Some(11));
}
