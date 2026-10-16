//! Texture replacement packs (M5a; the mods repo's `standard/HST-TEXTURE-PACK-STANDARD.md`): a `mod.json` of `kind`
//! `textures` beside `textures/**/<anything>-<key>.png` files, the same key-named, straight-alpha PNGs as
//! `mods/texture-replacements/` (`hst <iso> --dump-textures` writes their sources to `mods/textures-src/`), any size
//! or content. Installed packs stack: the first in the order wins per texture, then `texture-replacements/`, then the
//! disc (`textures.rs`). The order and the packs switched off are saved in `settings.txt` (`texture_packs`,
//! `texture_packs_off`, by folder under `mods/`); a pack in neither is on, above the listed ones (a new install
//! lands on top). Looks only: a lobby never syncs them, and with the Texture Replacements setting off (or the
//! Original look) none is read.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use bevy::prelude::*;

use crate::mods::Kind;

/// A texture pack's manifest and files, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Pack {
    pub dir: PathBuf,
    pub id: String,
    pub name: String,
    /// Who made it, as written (empty when unset).
    pub author: String,
    /// key -> its file.
    pub textures: HashMap<String, PathBuf>,
}

/// Whether `k` is a texture key (`texhash::GsTex::key`: 16 lowercase hex digits).
fn is_key(k: &str) -> bool {
    k.len() == 16 && k.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The key of a pack file name `<anything>-<key>.png` (case-blind extension).
pub fn key_of(name: &str) -> Option<&str> {
    let stem = name.len().checked_sub(4).filter(|&n| name.is_char_boundary(n) && name[n..].eq_ignore_ascii_case(".png")).map(|n| &name[..n])?;
    stem.rsplit_once('-').map(|(_, k)| k).filter(|k| is_key(k))
}

/// Reads and checks `dir/mod.json` as a texture pack and finds its textures under `dir/textures/`.
pub fn read(dir: &Path) -> Result<Pack, String> {
    let at = dir.join("mod.json");
    let err = |m: String| format!("{}: {m}", at.display());
    let text = crate::modfs::read_to_string(&at).map_err(|e| err(e.to_string()))?;
    let j: serde_json::Value = serde_json::from_str(&text).map_err(|e| err(format!("not JSON: {e}")))?;
    if Kind::of(&j) != Kind::Textures {
        return Err(err(format!("`kind` is {}: not a texture pack", Kind::of(&j))));
    }
    if j["standard"].as_u64() != Some(1) {
        return Err(err(format!("`standard` is {}, this game reads standard 1", j["standard"])));
    }
    // shown as plain text, length-capped (M16)
    let text = |k: &str| j[k].as_str().filter(|t| (1..=64).contains(&t.chars().count())).map(str::to_string).ok_or_else(|| err(format!("`{k}` must be a string of 1 to 64 characters")));
    let mut textures = HashMap::new();
    walk(&dir.join("textures"), &mut textures);
    if textures.is_empty() {
        return Err(err("no textures: a pack holds `textures/**/<name>-<key>.png` files (`--dump-textures` names)".into()));
    }
    Ok(Pack { dir: dir.to_path_buf(), id: text("id")?, name: text("name")?, author: j["author"].as_str().unwrap_or("").chars().take(64).collect(), textures })
}

/// Every key-named PNG under `d`, sorted by path; the first file of a key wins.
fn walk(d: &Path, out: &mut HashMap<String, PathBuf>) {
    for p in crate::modfs::read_dir(d) {
        match p.file_name().and_then(|n| n.to_str()).and_then(key_of) {
            Some(k) if crate::modfs::is_file(&p) => {
                out.entry(k.to_string()).or_insert(p);
            }
            _ if !crate::modfs::is_file(&p) => walk(&p, out),
            _ => {}
        }
    }
}

/// Every texture pack under `root` (`mods::dirs`' order); a broken one is left out with its reason logged.
pub fn list(root: &Path) -> Vec<Pack> {
    crate::mods::dirs(root).iter().filter(|d| Kind::of_dir(d) == Kind::Textures).filter_map(|d| read(d).map_err(|e| warn!("{e}")).ok()).collect()
}

/// The packs' order and the ones switched off, by [`folder`] (settings.txt's `texture_packs` / `texture_packs_off`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Order {
    /// Top (wins) first.
    pub order: Vec<String>,
    pub off: Vec<String>,
}

static ORDER: Mutex<Order> = Mutex::new(Order { order: Vec::new(), off: Vec::new() });

/// The saved order.
pub fn order() -> Order {
    ORDER.lock().unwrap().clone()
}

pub fn set_order(o: Order) {
    *ORDER.lock().unwrap() = o;
}

/// A pack's name in the [`Order`]: its folder under `root`, `/`-separated (`mypack`, `hst-mods/mypack`).
pub fn folder(root: &Path, dir: &Path) -> String {
    crate::modfs::key(dir.strip_prefix(root).unwrap_or(dir))
}

/// Takes a settings.txt line `k = v` that is the pack order's (true), else leaves it (false).
pub fn setting(k: &str, v: &str) -> bool {
    let names = || v.split(',').map(str::trim).filter(|n| !n.is_empty() && n.split('/').all(crate::modfs::plain)).map(str::to_string).collect();
    match k.trim() {
        "texture_packs" => ORDER.lock().unwrap().order = names(),
        "texture_packs_off" => ORDER.lock().unwrap().off = names(),
        _ => return false,
    }
    true
}

/// The pack order's settings.txt lines (none while nothing is ordered or off).
pub fn settings_text() -> String {
    let o = ORDER.lock().unwrap();
    let line = |k: &str, v: &[String]| if v.is_empty() { String::new() } else { format!("{k} = {}\n", v.join(", ")) };
    line("texture_packs", &o.order) + &line("texture_packs_off", &o.off)
}

/// `packs` in the order `o` gives them (top first): the ones it doesn't name first, in their own order, then its
/// listed ones; those it has off left out.
pub fn arrange(root: &Path, packs: Vec<Pack>, o: &Order) -> Vec<Pack> {
    let at = |p: &Pack| o.order.iter().position(|n| *n == folder(root, &p.dir)).map_or(0, |i| i + 1);
    let mut on: Vec<Pack> = packs.into_iter().filter(|p| !o.off.contains(&folder(root, &p.dir))).collect();
    on.sort_by_key(at);
    on
}

/// The packs switched on under `root`, top first ([`list`] in the saved [`order`]).
pub fn active(root: &Path) -> Vec<Pack> {
    arrange(root, list(root), &order())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hst_data::texhash::write_png;

    fn pack(root: &Path, dir: &str, id: &str, files: &[(&str, [u8; 4])]) -> PathBuf {
        let d = root.join(dir);
        std::fs::create_dir_all(d.join("textures")).unwrap();
        std::fs::write(d.join("mod.json"), format!(r#"{{"standard": 1, "kind": "textures", "id": "{id}", "name": "Pack {id}"}}"#)).unwrap();
        for (f, px) in files {
            let p = d.join("textures").join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            write_png(&p, 1, 1, px).unwrap();
        }
        d
    }

    #[test]
    fn keys_from_dump_names() {
        assert_eq!(key_of("ballrolling.0-f90743e09d9b06f2.png"), Some("f90743e09d9b06f2"));
        assert_eq!(key_of("a-b-F90743E09D9B06F2.PNG"), None);
        assert_eq!(key_of("a-f90743e09d9b06f2.PNG"), Some("f90743e09d9b06f2"));
        for bad in ["f90743e09d9b06f2.png", "a-f90743e09d9b06f.png", "a-f90743e09d9b06f2.webp", "a-g90743e09d9b06f2.png", "é.png"] {
            assert_eq!(key_of(bad), None, "{bad}");
        }
    }

    /// A pack reads its key-named files anywhere under `textures/`, is a known kind off the character roster, and
    /// one without textures or with a bad manifest is refused.
    #[test]
    fn reads_a_pack() {
        let root = std::env::temp_dir().join(format!("hst-texpack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let d = pack(&root, "a", "pack_a", &[("COURT/x.0-0000000000000001.png", [1; 4]), ("y.1-0000000000000002.png", [2; 4]), ("notes.png", [0; 4])]);
        let p = read(&d).unwrap();
        assert_eq!((p.id.as_str(), p.name.as_str()), ("pack_a", "Pack pack_a"));
        let mut keys: Vec<&str> = p.textures.keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(keys, ["0000000000000001", "0000000000000002"]);
        assert_eq!(Kind::of_dir(&d), Kind::Textures);
        assert!(crate::mods::list(&root).is_empty());
        assert_eq!(crate::mods::read_any(&d).unwrap().id, "pack_a");
        let e = pack(&root, "empty", "pack_e", &[]);
        assert!(read(&e).unwrap_err().contains("no textures"));
        std::fs::write(d.join("mod.json"), r#"{"standard": 2, "kind": "textures", "id": "x", "name": "X"}"#).unwrap();
        assert!(read(&d).unwrap_err().contains("standard"));
        std::fs::remove_dir_all(&root).ok();
    }

    /// Unnamed packs go on top, the named ones follow in their order, the off ones drop; settings.txt round-trips.
    #[test]
    fn order_and_settings() {
        let root = std::env::temp_dir().join(format!("hst-texorder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for n in ["a", "b", "c", "d"] {
            pack(&root, n, n, &[("x-0000000000000001.png", [0; 4])]);
        }
        let ids = |o: &Order| arrange(&root, list(&root), o).into_iter().map(|p| p.id).collect::<Vec<_>>();
        assert_eq!(ids(&Order::default()), ["a", "b", "c", "d"]);
        let o = Order { order: vec!["c".into(), "a".into(), "gone".into()], off: vec!["b".into()] };
        assert_eq!(ids(&o), ["d", "c", "a"]);
        let saved = order();
        set_order(o.clone());
        let text = settings_text();
        assert_eq!(text, "texture_packs = c, a, gone\ntexture_packs_off = b\n");
        set_order(Order::default());
        for (k, v) in text.lines().filter_map(|l| l.split_once('=')) {
            assert!(setting(k, v));
        }
        assert_eq!(order(), o);
        assert!(!setting("music", "on"));
        assert!(setting("texture_packs", " ../x, ok , /abs"));
        assert_eq!(order().order, ["ok"]);
        set_order(saved);
        std::fs::remove_dir_all(&root).ok();
    }
}
