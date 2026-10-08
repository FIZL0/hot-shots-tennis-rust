//! Every court's background figures have their files: the umpire, each walking spectator and each trigger
//! creature with a model parse as skinned models with their clips. Skips without the ISO.

use hst_data::{ani, exe::Game, iso::Iso, layout, mdl, xb::Archive};

#[test]
fn every_court_figure_loads() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("no ISO, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let (mut models, mut clips) = (0, 0);
    for court in 1..=11u32 {
        let data = iso.read(&format!("COURTSET/{court:02}/HOLE01.XB")).unwrap();
        let arc = Archive::parse(&data).unwrap();
        let find = |name: &str| arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name)).map(|e| arc.read(e).unwrap());
        let mut figures = vec![layout::umpire_files(4)];
        let walkers = game.walkers(court);
        for e in game.npc_roster(court) {
            if (54..60).contains(&e.kind) {
                let w = walkers[e.kind as usize - 54];
                figures.push(layout::walker_files(w[0], w[1]));
            } else if let Some((m, b, n)) = game.trigger_model(e.kind) {
                figures.push(layout::trigger_files(&m.to_ascii_lowercase(), &b.to_ascii_lowercase(), n));
            }
        }
        for (stem, anims) in figures {
            let m = mdl::parse(&find(&format!("{stem}.mdl")).unwrap_or_else(|| panic!("court {court}: {stem}.mdl"))).unwrap();
            assert!(!m.skinned().is_empty() && m.node_count > 0, "court {court}: {stem} not skinned");
            assert!(find(&format!("{stem}.mtl")).is_some(), "court {court}: {stem}.mtl");
            models += 1;
            for a in anims {
                let a = ani::parse(&find(&a).unwrap_or_else(|| panic!("court {court}: {a}"))).unwrap();
                assert_eq!(a.ticks_per_frame, 80);
                clips += 1;
            }
        }
    }
    eprintln!("{models} figure models, {clips} clips");
}
