//! VU1's unlit object flag (batch header +0x30 bit 1, `mdl::Packet::flags`): on court 4 only the net's wire and
//! poles have it, as their draw records in slot 4's RAM do (flag 0x10e/0x20e; every other model 0x..c/0x..d), and
//! they draw at ⌊vc · (ambient + light)⌋ in its GS dump. Skipped without the disc image.

use hst_data::{iso::Iso, mdl, mtl, xb::Archive};

#[test]
fn court_4_unlit_batches() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
    let data = iso.read("COURT/04/HOL01.XB").unwrap();
    let a = Archive::parse(&data).unwrap();
    let read = |n: &str| a.entries.iter().find(|e| e.name.rsplit(['\\', '/']).next().unwrap().eq_ignore_ascii_case(n)).map(|e| a.read(e).unwrap());
    let mut unlit = Vec::new();
    for e in a.entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".mdl")) {
        let stem = e.name[..e.name.len() - 4].rsplit(['\\', '/']).next().unwrap().to_string();
        let m = mdl::parse(&a.read(e).unwrap()).unwrap();
        let Some(mt) = read(&format!("{stem}.MTL")).map(|b| mtl::parse(&b, read(&format!("{stem}.MTI")).as_deref()).unwrap()) else { continue };
        for (mi, pks) in m.materials.iter().enumerate() {
            assert!(pks.iter().all(|p| p.flags & !2 == 0x0c), "{stem} {mi}");
            if pks.iter().any(|p| p.flags & 2 != 0) {
                unlit.push(format!("{stem}:{}", mt.materials[mi].name));
            }
        }
    }
    unlit.sort();
    assert_eq!(unlit, ["netmoto:pole1:pole", "netmoto:pole1:wire", "znet_s1000:pole1:pole", "znet_s1000:pole1:wire"]);
}
