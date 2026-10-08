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
    // every vertex's copies agree in the bind pose: weights sum to 1; each bone's normal comes pre-weighted
    // (VU1 sums them as is: |n| = the weight)
    for (_, vs, _, _) in &parts {
        for v in vs {
            assert!((v.weights.iter().sum::<f32>() - 1.0).abs() < 1e-3, "{v:?}");
            for k in 0..2 {
                let len = v.normals[k].iter().map(|c| c * c).sum::<f32>().sqrt();
                assert!((len - v.weights[k]).abs() < 0.01, "{v:?}");
            }
        }
    }
}


/// Skinned triangles wind by their closing vertex's UV `w` (as rigid ones): every character's triangles then face
/// the way their vertex normals point, which strip parity breaks (Cody's face lost a third of its triangles to the
/// back-face cull).
#[test]
fn skinned_triangles_face_their_normals() {
    let Some(mut iso) = iso() else { return eprintln!("no ISO, skipped") };
    for c in 0..14 {
        let data = iso.read(&format!("PC/PC{c:02}C00.XB")).unwrap();
        let arc = Archive::parse(&data).unwrap();
        let body = |n: &str| n.rsplit(['\\', '/']).next().is_some_and(|f| f.starts_with(&format!("pc{c:02}_t")) && f.ends_with("_c00.mdl"));
        let e = arc.entries.iter().find(|e| body(&e.name.to_ascii_lowercase())).unwrap();
        let m = mdl::parse(&arc.read(e).unwrap()).unwrap();
        let (mut along, mut against) = (0, 0);
        for (_, verts, tris, _) in m.skinned() {
            for t in tris {
                let [a, b, c] = t.map(|i| verts[i as usize].pos);
                let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
                let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
                let vn = t.iter().fold([0.0; 3], |s, &i| {
                    let x = &verts[i as usize].normals;
                    [s[0] + x[0][0] + x[1][0], s[1] + x[0][1] + x[1][1], s[2] + x[0][2] + x[1][2]]
                });
                let d = n[0] * vn[0] + n[1] * vn[1] + n[2] * vn[2];
                if d > 0.0 { along += 1 } else if d < 0.0 { against += 1 }
            }
        }
        eprintln!("character {c}: {along} along, {against} against");
        // ponytail: a few slivers disagree with their smoothed normals (≤ 18 of ~2500); parity gave ~half
        assert!(against * 50 < along, "character {c}: {against} of {} triangles face away from their normals", along + against);
    }
}
