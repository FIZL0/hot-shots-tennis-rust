//! Mods in online matches (M2i). A lobby pick of a mod carries its gameplay data ([`Wire`]: donor, hand, TParam row
//! and overrides, AI row and the costume's skeleton), and every peer simulates the mod from that, so the match plays
//! the same whether or not a peer has the mod's looks. The looks are optional: a peer without them asks once
//! (always / never / ask each time, kept in `mods-online/ask.txt` beside the disc); taken, they come peer to peer as
//! one [`pack`] (data files only, size-capped, hash-checked, then §7-checked before use) into `mods-online/<hash>/`;
//! declined, the donor stands in, hue-shifted so it's clearly not the mod.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use hst_sim::pose::Skeleton;
use serde::{Deserialize, Serialize};

use crate::character::CharacterData;
use crate::mods::{self, Mod};

/// The most a mod's looks may weigh on the wire.
pub const CAP: usize = 64 << 20;
/// What a pack may hold: data only.
const KINDS: [&str; 4] = ["glb", "png", "wav", "json"];
const MAX_FILES: usize = 4096;
const MAX_JOINTS: usize = 512;

/// A mod as a lobby pick carries it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wire {
    pub id: String,
    pub name: String,
    pub donor: usize,
    pub hand: f32,
    pub base: usize,
    pub overrides: Vec<(String, String)>,
    pub ai_row: usize,
    /// The picked costume's skeleton, what the simulation poses (rest matrices as f32 bits: exact on any wire).
    pub names: Vec<String>,
    pub parent: Vec<Option<usize>>,
    pub rest: Vec<[u32; 16]>,
    /// The looks: the hash of the mod's [`pack`].
    pub looks: u64,
    /// The owner's folder (never sent).
    #[serde(skip)]
    pub dir: Option<PathBuf>,
}

impl Wire {
    /// Mod `m` in costume `costume` (reads its files: the costume's skeleton and the pack's hash).
    pub fn of(m: &Mod, costume: usize) -> Result<Wire, String> {
        let sk = mods::skeleton(m, costume % m.costumes.len().max(1))?;
        Ok(Wire {
            id: m.id.clone(),
            name: m.name.clone(),
            donor: m.donor,
            hand: m.hand,
            base: m.base,
            overrides: m.overrides.clone(),
            ai_row: m.ai_row,
            names: sk.names,
            parent: sk.parent,
            rest: sk.rest.iter().map(|r| std::array::from_fn(|k| r[k / 4][k % 4].to_bits())).collect(),
            looks: hash(&pack(&m.dir)?),
            dir: Some(m.dir.clone()),
        })
    }

    /// Whether a peer's pick is one this game can play (else it's refused): the ranges `mods::read` allows, real
    /// TParam overrides `mods::read` loads (`mods::override_ok`), and a skeleton that is a tree.
    pub fn check(&self) -> Result<(), String> {
        let bad = |what: &str| Err(format!("mod {:?}: bad {what}", self.id));
        if self.donor > 13 || self.base > 13 || self.ai_row > 13 {
            return bad("donor, params base or AI row");
        }
        if self.hand != 1.0 && self.hand != -1.0 {
            return bad("hand");
        }
        for (k, v) in &self.overrides {
            mods::override_ok(k, v).map_err(|e| format!("mod {:?}: {e}", self.id))?;
        }
        let n = self.names.len();
        if n == 0 || n > MAX_JOINTS || self.parent.len() != n || self.rest.len() != n {
            return bad("skeleton");
        }
        // a tree with parents before children, as `pose::Skeleton` wants
        if self.parent.iter().enumerate().any(|(i, p)| p.is_some_and(|p| p >= i)) {
            return bad("skeleton");
        }
        Ok(())
    }

    pub fn skeleton(&self) -> Skeleton {
        Skeleton {
            names: self.names.clone(),
            parent: self.parent.clone(),
            rest: self.rest.iter().map(|r| std::array::from_fn(|i| std::array::from_fn(|j| f32::from_bits(r[i * 4 + j])))).collect(),
        }
    }
}

/// The match's mod for a seat: gameplay from the wire; looks from `looks` (a folder with the same pack), else a
/// stand-in (no costumes: `play::mod_match` draws the donor hue-shifted).
pub fn for_match(w: &Wire, looks: Option<&Path>) -> Mod {
    let mut m = looks.and_then(|d| mods::read(d).map_err(|e| warn!("{e}")).ok()).unwrap_or_else(|| Mod {
        dir: PathBuf::new(),
        id: w.id.clone(),
        name: w.name.clone(),
        source: String::new(),
        costumes: Vec::new(),
        donor: 0,
        hand: 1.0,
        base: 0,
        overrides: Vec::new(),
        ai_row: 0,
        texture_face: false,
        no_face: true,
        voice: String::new(),
        motions: None,
        costume_of: None,
        skeleton: None,
        upscale: true,
    });
    (m.donor, m.hand, m.base, m.overrides, m.ai_row) = (w.donor, w.hand, w.base, w.overrides.clone(), w.ai_row);
    m.skeleton = Some(Arc::new(w.skeleton()));
    m
}

/// 64-bit FNV-1a.
pub fn hash(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325, |h, &x| (h ^ x as u64).wrapping_mul(0x100_0000_01b3))
}

/// A mod folder's data files (sorted, `/`-separated, upscaled textures left out) as one blob: per file the path's
/// length (u32 LE), the path, the size (u64 LE), the bytes.
pub fn pack(dir: &Path) -> Result<Vec<u8>, String> {
    // B285: through `modfs`, so the browser's mods (held in memory, not on a file system) pack too
    fn walk(root: &Path, d: &Path, out: &mut Vec<String>) {
        for p in crate::modfs::read_dir(d) {
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            if crate::modfs::is_file(&p) {
                if allowed(&rel) {
                    out.push(rel);
                }
            } else if rel != "textures/upscaled" {
                // ponytail: the optional upscaled set stays home (standard §3); the receiver draws the mod's own textures
                walk(root, &p, out);
            }
        }
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files);
    if files.is_empty() {
        return Err(format!("{}: no mod files", dir.display()));
    }
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let b = crate::modfs::read(&dir.join(&f)).map_err(|e| format!("{f}: {e}"))?;
        out.extend((f.len() as u32).to_le_bytes());
        out.extend(f.as_bytes());
        out.extend((b.len() as u64).to_le_bytes());
        out.extend(b);
        if out.len() > CAP {
            return Err(format!("{}: over {} MB", dir.display(), CAP >> 20));
        }
    }
    Ok(out)
}

/// A relative path of a data file: no `..`, no root, a data extension.
fn allowed(rel: &str) -> bool {
    let p = Path::new(rel);
    p.components().all(|c| matches!(c, Component::Normal(_)))
        && p.extension().and_then(|e| e.to_str()).is_some_and(|e| KINDS.contains(&e.to_ascii_lowercase().as_str()))
}

/// A pack's files, checked: size, hash, paths and count.
fn files(pack: &[u8], looks: u64) -> Result<Vec<(String, &[u8])>, String> {
    if pack.len() > CAP || hash(pack) != looks {
        return Err("the mod's files didn't arrive whole".into());
    }
    let mut out = Vec::new();
    let mut rest = pack;
    fn take<'a>(rest: &mut &'a [u8], n: usize) -> Result<&'a [u8], String> {
        if rest.len() < n {
            return Err("a cut-off mod pack".into());
        }
        let (b, r) = rest.split_at(n);
        *rest = r;
        Ok(b)
    }
    while !rest.is_empty() {
        let n = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap()) as usize;
        let name = String::from_utf8(take(&mut rest, n)?.to_vec()).map_err(|_| "a file name that isn't text")?;
        let size = u64::from_le_bytes(take(&mut rest, 8)?.try_into().unwrap());
        let b = take(&mut rest, usize::try_from(size).map_err(|_| "a cut-off mod pack")?)?;
        if !allowed(&name) || out.len() >= MAX_FILES {
            return Err(format!("{name:?} isn't a mod data file"));
        }
        out.push((name, b));
    }
    Ok(out)
}

/// A received pack into `cache/<looks>/`, read and §7-checked; on any failure nothing is kept.
pub fn unpack(pack: &[u8], looks: u64, cache: &Path) -> Result<Mod, String> {
    let files = files(pack, looks)?;
    let dir = cache.join(format!("{looks:016x}"));
    let gone = || {
        let _ = std::fs::remove_dir_all(&dir);
        crate::modfs::remove(&dir);
    };
    gone();
    let got = (|| {
        for (name, b) in &files {
            let p = dir.join(name);
            // the browser keeps them in memory for the session (`modfs::write`)
            if !cfg!(target_arch = "wasm32") {
                std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
            }
            crate::modfs::write(&p, b.to_vec())?;
        }
        let m = mods::read(&dir)?;
        mods::check(&m)?;
        Ok(m)
    })();
    if got.is_err() {
        gone();
    }
    got
}

/// Where received mods and the choice live: `mods-online/` beside the disc (not under `mods/`, so they don't join
/// the custom roster).
pub fn cache(iso: &str) -> PathBuf {
    Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("mods-online")
}

/// A folder here with the looks `w` names: a received copy, or one of `roster` with the same id whose pack hashes the
/// same.
pub fn find(w: &Wire, roster: &[Mod], cache: &Path) -> Option<PathBuf> {
    let got = cache.join(format!("{:016x}", w.looks));
    if crate::modfs::is_file(&got.join("mod.json")) {
        return Some(got);
    }
    roster.iter().filter(|m| m.id == w.id).find(|m| pack(&m.dir).is_ok_and(|p| hash(&p) == w.looks)).map(|m| m.dir.clone())
}

/// What to do with a remote mod's looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    Always,
    Never,
    Each,
}

impl Ask {
    const WORDS: [(Ask, &str); 3] = [(Ask::Always, "always"), (Ask::Never, "never"), (Ask::Each, "ask")];

    /// `word`'s choice (as `word()` writes it in settings.txt).
    pub fn parse(word: &str) -> Option<Ask> {
        Ask::WORDS.iter().find(|(_, w)| word.trim() == *w).map(|(a, _)| *a)
    }

    pub fn word(self) -> &'static str {
        Ask::WORDS.iter().find(|(a, _)| *a == self).unwrap().1
    }

    /// The choice an older build saved in `cache` (before B289 moved it into settings.txt), `None` if none.
    pub fn load(cache: &Path) -> Option<Ask> {
        Ask::parse(&std::fs::read_to_string(cache.join("ask.txt")).ok()?)
    }
}

/// A stand-in's look: every texture of its parts and racket with its colour channels rotated (a 120° hue turn), so
/// skin and clothes come out in colours no disc character has.
pub fn tint(data: &CharacterData, materials: &Assets<StandardMaterial>, images: &mut Assets<Image>) {
    use bevy::render::render_resource::TextureFormat;
    let mut ids: Vec<AssetId<Image>> = data.parts.iter().map(|p| &p.1).chain(data.racket.iter().map(|r| &r.1)).filter_map(|h| materials.get(h)?.base_color_texture.as_ref().map(|t| t.id())).collect();
    ids.extend(data.gs.values().flatten().filter_map(|g| g.texture.as_ref().map(|t| t.id())));
    ids.sort();
    ids.dedup();
    for id in ids {
        let Some(mut img) = images.get_mut(id) else { continue };
        if !matches!(img.texture_descriptor.format, TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb) {
            continue;
        }
        if let Some(d) = img.data.as_mut() {
            d.chunks_exact_mut(4).for_each(|p| p[..3].rotate_left(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire() -> Wire {
        Wire {
            id: "x".into(),
            name: "X".into(),
            donor: 3,
            hand: -1.0,
            base: 3,
            overrides: vec![("Strk POW".into(), "6".into())],
            ai_row: 3,
            names: vec!["a".into(), "b".into()],
            parent: vec![None, Some(0)],
            rest: vec![[0.5f32.to_bits(); 16], [(-0.1f32).to_bits(); 16]],
            looks: 1,
            dir: None,
        }
    }

    #[test]
    fn wire_round_trips_exactly_and_bad_picks_are_refused() {
        let w = wire();
        let back: Wire = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!(back, w);
        assert_eq!(back.skeleton().rest[1][3][2], -0.1);
        assert_eq!(w.check(), Ok(()));
        let bad = |w: Wire| assert!(w.check().is_err(), "{w:?}");
        bad(Wire { donor: 14, ..wire() });
        bad(Wire { hand: 0.5, ..wire() });
        bad(Wire { overrides: vec![("nope".into(), "1".into())], ..wire() });
        bad(Wire { overrides: vec![("Strk POW".into(), "1,2".into())], ..wire() });
        // B275: TParam's text cells (Get a Grip / Out of Bounds mods set their play style) pass, as `mods::read` loads them
        assert_eq!(Wire { overrides: vec![("タイプ".into(), "オール".into()), ("Strk POW".into(), "1e-3".into())], ..wire() }.check(), Ok(()));
        bad(Wire { parent: vec![Some(1), Some(0)], ..wire() });
        bad(Wire { parent: vec![None, Some(9)], ..wire() });
        bad(Wire { rest: vec![], ..wire() });
    }

    #[test]
    fn stand_in_plays_the_wire_and_looks_are_left_out() {
        let m = for_match(&wire(), None);
        assert!(m.costumes.is_empty());
        assert_eq!((m.donor, m.hand, m.base, m.ai_row), (3, -1.0, 3, 3));
        assert_eq!(crate::handshake::gameplay_hash(&m), crate::handshake::gameplay_hash(&for_match(&wire(), None)));
        assert_eq!(m.skeleton.unwrap().names, ["a", "b"]);
    }

    #[test]
    fn pack_unpack_checks_what_arrives() {
        let tmp = std::env::temp_dir().join(format!("hst-m2i-{}", std::process::id()));
        let src = tmp.join("src");
        std::fs::create_dir_all(src.join("voice")).unwrap();
        std::fs::create_dir_all(src.join("textures/upscaled")).unwrap();
        std::fs::write(src.join("mod.json"), "{}").unwrap();
        std::fs::write(src.join("voice/0_0.wav"), [1, 2, 3]).unwrap();
        std::fs::write(src.join("run.sh"), "rm -rf /").unwrap();
        std::fs::write(src.join("textures/upscaled/a.png"), [0; 9]).unwrap();
        let p = pack(&src).unwrap();
        let names: Vec<_> = files(&p, hash(&p)).unwrap().into_iter().map(|f| f.0).collect();
        assert_eq!(names, ["mod.json", "voice/0_0.wav"]);
        // B285: the browser's mods are in memory (`modfs`), not on disk: the same pack
        let mem = tmp.join("mem");
        for f in ["mod.json", "voice/0_0.wav", "run.sh", "textures/upscaled/a.png"] {
            crate::modfs::add(&mem.join(f), std::fs::read(src.join(f)).unwrap().into());
        }
        assert_eq!(pack(&mem).unwrap(), p);
        // a changed byte, a path out of the folder, a cut-off pack: refused
        let mut q = p.clone();
        *q.last_mut().unwrap() ^= 1;
        assert!(files(&q, hash(&p)).is_err());
        let evil = [&4u32.to_le_bytes()[..], b"../x", &1u64.to_le_bytes(), b"a"].concat();
        assert!(files(&evil, hash(&evil)).is_err());
        assert!(files(&p[..p.len() - 1], hash(&p[..p.len() - 1])).is_err());
        // `{}` isn't a mod: nothing stays in the cache
        assert!(unpack(&p, hash(&p), &tmp.join("cache")).is_err());
        assert!(!tmp.join("cache").join(format!("{:016x}", hash(&p))).exists());
        std::fs::write(tmp.join("ask.txt"), "never").unwrap();
        assert_eq!(Ask::load(&tmp), Some(Ask::Never));
        assert!(Ask::WORDS.iter().all(|(a, _)| Ask::parse(a.word()) == Some(*a)));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
