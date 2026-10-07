//! Court 10's mip levels (MTL texture header MXL) and per-material LOD bias K (MDL material header), against
//! the values the game's draw records hold in RAM. Skipped without the disc image.

use hst_data::{iso::Iso, mdl, mtl, xb::Archive};

#[test]
fn court_10_mips_and_lod_k() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
    let (mut ks, mut mipped) = (Vec::new(), 0);
    for arc in ["CMN.XB", "GRD01.XB", "HOL01.XB"] {
        let data = iso.read(&format!("COURT/10/{arc}")).unwrap();
        let a = Archive::parse(&data).unwrap();
        let read = |n: &str| a.entries.iter().find(|e| e.name.eq_ignore_ascii_case(n)).map(|e| a.read(e).unwrap());
        for e in a.entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".mdl")) {
            let stem = &e.name[..e.name.len() - 4];
            let m = mdl::parse(&a.read(e).unwrap()).unwrap();
            ks.extend(m.lod_k);
            let mt = mtl::parse(&read(&format!("{stem}.MTL")).unwrap(), read(&format!("{stem}.MTI")).as_deref()).unwrap();
            for t in &mt.textures {
                assert_eq!(t.rgba.len() as u32, t.width * t.height * 4);
                for (k, l) in t.mips.iter().enumerate() {
                    assert_eq!(l.len() as u32, (t.width >> (k + 1)) * (t.height >> (k + 1)) * 4, "{stem}: level {}", k + 1);
                }
                mipped += !t.mips.is_empty() as usize;
            }
        }
    }
    assert!(mipped > 0, "no mipmapped textures");
    // clouds, background, jungle as the draw records hold them
    for k in [-6.0625, -11.4375, -15.25] {
        assert!(ks.contains(&k), "K {k} not found");
    }
}
