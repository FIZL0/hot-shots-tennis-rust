//! The umpire's call pop-ups (`AZUMA/INPANE/INPANE.XB0`, `azuma/inpane/mdl`): each model's channels bind to its nodes,
//! morph targets and materials, and its animation ends where the game's settle test waits for it.

use hst_data::{ani, iso::Iso, mdl, mor, mtl, xb::Archive};
use hst_sim::effect::Effect;

#[test]
fn call_models() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
        return eprintln!("ISO absent, skipped");
    };
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").unwrap();
    let arc = Archive::parse(&data).unwrap();
    let get = |name: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&name.to_ascii_lowercase())).map(|e| arc.read(e).unwrap());
    let mut lengths = vec![];
    for s in ["i_let_00", "i_out_00", "i_net_00", "i_fault_00", "i_doublefault_00", "i_coatchange_00"] {
        let m = mdl::parse(&get(&format!("{s}.MDL")).unwrap()).unwrap();
        let a = get(&format!("{s}.ANI")).map_or(ani::Anim { ticks_per_frame: 1, tracks: vec![] }, |d| ani::parse(&d).unwrap());
        let mo = get(&format!("{s}.MOR")).map_or(mor::Tracks { ticks_per_frame: 1, tracks: vec![] }, |d| mor::parse(&d, 1).unwrap());
        let mt = mor::parse(&get(&format!("{s}.MTA")).unwrap(), 1).unwrap();
        let mats = mtl::parse(&get(&format!("{s}.MTL")).unwrap(), get(&format!("{s}.MTI")).as_deref()).unwrap().materials;
        assert!(mo.tracks.iter().all(|t| m.morph_names.contains(&t.name)), "{s} MOR binding");
        assert!(mt.tracks.iter().all(|t| mats.iter().any(|m| m.name == t.name)), "{s} MTA binding");
        let mut e = Effect::new(&m, &a, &mo, &mt, &mats);
        lengths.push(e.clip.length);
        if s == "i_coatchange_00" {
            // MTA keys 0, 480 (frame 6) … 5600: faded in over 6 frames, out at the end
            e.hold = true;
            e.start();
            assert_eq!(e.alphas[0], 0.0);
            (0..6).for_each(|_| e.tick());
            assert_eq!(e.alphas[0], 1.0);
            (0..80).for_each(|_| e.tick());
            assert!(e.live);
            assert_eq!(e.alphas[0], 0.0);
        }
    }
    assert_eq!(lengths, [0.0, 0.0, 0.0, 15.0, 15.0, 70.0]);
}

/// A fault whose voice line has already ended settles when its 15-frame animation does, holds 30, then fades
/// over 128·t/5 to 0.
#[test]
fn call_settles_after_animation() {
    use hst_sim::{flow::PostPoint, judge::Rally, score::Score};
    let t = hst_data::exe::ScoreboardTiming { call_wait: [0, 60, 44, 44, 44, 44], point_wait: 0, game_wait: 0, point_hold: 0.0, game_hold: 0.0, game_rise: 6, game_drop: 10 };
    let (mut score, mut rally) = (Score::new(), Rally::default());
    let mut p = PostPoint::called(None, 2, &t);
    p.set_call_anim(15.0);
    let mut alphas = vec![];
    while let Some(c) = p.call() {
        assert_eq!(c.call, 2);
        alphas.push(c.alpha);
        p.step_with(&mut score, &mut rally, &t, true, false);
    }
    // creation + 14 unsettled, held 1..29 (from the settle tick), fade 5..0
    assert_eq!(alphas.len(), 1 + 14 + 29 + 6);
    assert_eq!(alphas[alphas.len() - 6..], [1.0, 102.0 / 128.0, 76.0 / 128.0, 51.0 / 128.0, 25.0 / 128.0, 0.0]);
}
#[test]
fn tmp_nodes() {
    let mut iso = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")).unwrap();
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").unwrap();
    let arc = Archive::parse(&data).unwrap();
    for e in &arc.entries { if e.name.to_ascii_lowercase().ends_with("_00.mdl") {
        let m = mdl::parse(&arc.read(e).unwrap()).unwrap();
        for (mat, v, _, mo) in m.skinned() { println!("mat {mat} pos {:?}", v.iter().map(|v| v.pos).collect::<Vec<_>>()); for t in mo { println!("  morph {:?}", t); } }
    }}
}
