//! A disc pack (`Iso::write_pack`) opens like the ISO and reads every packed file byte for byte, its hash is the same
//! on every import, and it is far smaller than the image. Skips without the ISO.

use hst_data::iso::Iso;

const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

#[test]
fn pack_reads_like_the_disc() {
    let Ok(mut iso) = Iso::open(ISO) else { return eprintln!("no ISO, skipped") };
    let dir = std::env::temp_dir().join(format!("hst-pack-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("disc.pack");
    let hash = iso.write_pack(&out, |_, _| {}).unwrap();
    let mut pack = Iso::open(&out).unwrap();
    assert_eq!(pack.pack_hash, Some(hash));
    assert!(pack.files.contains_key("ZZBIN/GAME.BIN") && !pack.files.contains_key("A/OPEN.PSS"));
    let names: Vec<String> = pack.files.keys().cloned().collect();
    for n in &names {
        assert_eq!(pack.read(&n.to_lowercase()).unwrap(), iso.read(n).unwrap(), "{n}");
    }
    let (p, i) = (std::fs::metadata(&out).unwrap().len(), std::fs::metadata(ISO).unwrap().len());
    assert!(p * 5 < i, "pack {p} bytes vs ISO {i}");
    // the same disc always gives the same pack (M2e: peers compare this hash)
    assert_eq!(iso.write_pack(&out, |_, _| {}).unwrap(), hash);
    // the browser's path: written into any writer, mounted from memory, opened by its mount path
    let mut mem = std::io::Cursor::new(Vec::new());
    assert_eq!(iso.write_pack_to(&mut mem, |_, _| {}).unwrap(), hash);
    assert_eq!(mem.get_ref(), &std::fs::read(&out).unwrap());
    hst_data::iso::mount("mounted.pack", mem.into_inner().into());
    let mut m = Iso::open("mounted.pack").unwrap();
    assert_eq!((m.pack_hash, m.read("zzbin/game.bin").unwrap()), (Some(hash), iso.read("ZZBIN/GAME.BIN").unwrap()));
    std::fs::remove_file(&out).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}

#[test]
fn rejects_other_files() {
    let dir = std::env::temp_dir().join(format!("hst-pack-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("not.iso");
    std::fs::write(&f, vec![0u8; 40 * 2048]).unwrap();
    assert!(Iso::open(&f).is_err());
    std::fs::remove_file(&f).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}
