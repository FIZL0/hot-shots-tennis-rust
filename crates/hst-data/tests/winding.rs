//! Drawn triangles face the way the game's VU1 culls them: the same rule the collision walk orients by (the
//! third vertex's UV w, checked bit-exact in `collision.rs`). Court 1's ground and props (its school fence is
//! one-sided and must vanish from behind, B7). Skipped without the disc image.

use hst_data::iso::Iso;
use hst_data::mdl;
use hst_data::xb::Archive;

#[test]
fn draw_winding_matches_collision() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        eprintln!("disc missing, skipped");
        return;
    };
    let mut checked = 0;
    for arc in ["GRD01.XB", "HOL01.XB"] {
        let data = iso.read(&format!("COURT/01/{arc}")).unwrap();
        let a = Archive::parse(&data).unwrap();
        for e in a.entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".mdl")) {
            let Ok(m) = mdl::parse(&a.read(e).unwrap()) else { continue };
            let key = |p: [f32; 3]| p.map(f32::to_bits);
            // every drawn triangle in its three rotations, by vertex position
            let mut drawn = std::collections::HashSet::new();
            for pk in m.materials.iter().flatten() {
                for t in &pk.triangles {
                    let p = t.map(|i| key(pk.vertices[i as usize].pos));
                    for r in 0..3 {
                        drawn.insert([p[r], p[(r + 1) % 3], p[(r + 2) % 3]]);
                    }
                }
            }
            for t in &m.collision {
                let p = t.pos.map(|v| key([v[0], v[1], v[2]]));
                let (p, q) = if 0.0 <= t.uv[2][3] { (p, [p[2], p[1], p[0]]) } else { ([p[2], p[1], p[0]], p) };
                // degenerate or doubled-back geometry can hold both orders; only the reverse alone is wrong
                assert!(drawn.contains(&p) || !drawn.contains(&q), "{}: triangle drawn facing the wrong way", e.name);
                checked += drawn.contains(&p) as usize;
            }
        }
    }
    assert!(checked > 1000, "only {checked} triangles matched");
}
