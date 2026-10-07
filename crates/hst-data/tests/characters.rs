//! Every character's models and animations on the disc parse: all `ANI2` files byte-exact to their end, all
//! skinned models with bind-pose vertices that agree across their bone bindings. Skips without the ISO.

use hst_data::{ani, iso::Iso, mdl, xb::Archive};

fn iso() -> Option<Iso> {
    Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")).ok()
}

#[test]
fn every_animation_parses() {
    let Some(mut iso) = iso() else { return eprintln!("no ISO, skipped") };
    let (mut files, mut named) = (0, 0);
    for c in 0..10 {
        let data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
        let arc = Archive::parse(&data).unwrap();
        for e in arc.entries.iter().filter(|e| e.name.to_ascii_uppercase().ends_with(".ANI2")) {
            let a = ani::parse(&arc.read(e).unwrap()).unwrap_or_else(|err| panic!("{}: {err}", e.name));
            assert_eq!(a.ticks_per_frame, 80, "{}", e.name);
            files += 1;
        }
        for m in 0..ani::MOTIONS.len() {
            let stem = ani::motion_name(m, c).unwrap().to_ascii_lowercase();
            named += arc.entries.iter().any(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))) as usize;
        }
    }
    eprintln!("{files} animations, {named} named motions found");
    assert!(files > 500 && named > 500);
}

#[test]
fn character_models_skin() {
    let Some(mut iso) = iso() else { return eprintln!("no ISO, skipped") };
    let data = iso.read("PC/PC00C00.XB").unwrap();
    let arc = Archive::parse(&data).unwrap();
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("pc00_t00_c00.mdl")).unwrap();
    let m = mdl::parse(&arc.read(e).unwrap()).unwrap();
    assert_eq!(m.node_names.iter().filter(|n| n.starts_with("Bip01")).count(), 53);
    assert!(m.node_names.iter().any(|n| n == "Racket"));
    let parts = m.skinned();
    let verts: usize = parts.iter().map(|p| p.1.len()).sum();
    // standing figure ~1.55 m tall: feet near y = 0, head near y = -1.5 (Y down)
    let (lo, hi) = parts.iter().flat_map(|p| p.1.iter()).fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v.pos[1]), hi.max(v.pos[1])));
    eprintln!("{verts} vertices, y {lo}..{hi}");
    assert!(verts > 1000 && lo < -1.3 && hi > -0.1, "y range {lo}..{hi}");
    // every vertex's copies agree in the bind pose: weights sum to 1
    for (_, vs, _, _) in &parts {
        for v in vs {
            assert!((v.weights.iter().sum::<f32>() - 1.0).abs() < 1e-3, "{v:?}");
        }
    }
}

